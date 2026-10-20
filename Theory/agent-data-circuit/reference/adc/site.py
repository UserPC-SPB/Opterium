"""Static ADC site loader and structural validator.

A site is a directory:

    manifest.json          served as /.well-known/adc.json
    tree.json              {"tree_generation", "nodes": [{name, path, anchor, kind, ...}]}
    nodes/<aN>.json        {"passport": {...}, "slices": {name: payload}}
    ai.json                optional /AI/ cold entry map (AI-ENTRY.md)

The loader is the single place that decides whether a directory is a valid
circuit. The HTTP server and the ``adc validate`` CLI both call
:meth:`Site.load` / :func:`check_site`, so a site the CLI accepts is a site
the server can serve.
"""

from __future__ import annotations

import hashlib
import json
import math
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Iterable, Mapping

from .aggregate import parse_metric, parse_group_by
from .anchors import AnchorTable
from .errors import AdcError
from .filters import FilterSpec, filter_specs
from .pathing import canonicalize_path, is_prefix_of, parent_path, path_depth
from .tokens import count_tokens

__all__ = [
    "AI_KEYS",
    "canonical_bytes",
    "payload_stats",
    "Slice",
    "Node",
    "Site",
    "check_site",
]

# Key vocabulary of the /AI/ cold entry (AI-ENTRY.md section 5). A site may
# declare a subset; declaring anything outside this list is a configuration
# error, so the vocabulary cannot drift from the note.
AI_KEYS = (
    "text",
    "meta",
    "image",
    "transcript",
    "chapters",
    "comments",
    "price",
    "stock",
)


def canonical_bytes(payload: Any) -> bytes:
    """RFC 8785-flavoured canonical JSON: sorted keys, no whitespace, UTF-8."""
    return json.dumps(
        payload, sort_keys=True, separators=(",", ":"), ensure_ascii=False
    ).encode("utf-8")


def payload_stats(payload: Any, tokenizer: str) -> tuple[int, int, str]:
    """Return ``(tokens, bytes, generation)`` for a slice payload."""
    data = canonical_bytes(payload)
    text = data.decode("utf-8")
    tokens = count_tokens(text, tokenizer)
    generation = "V1:" + hashlib.sha256(data).hexdigest()[:8]
    return tokens, len(data), generation


@dataclass
class Slice:
    name: str
    tokens: int
    bytes: int
    generation: str
    generated_at: str | None = None
    dynamic: bool = False
    provenance: str = "publisher"
    moderated: bool | None = None
    default: bool = False
    price: float | None = None
    currency: str | None = None
    unit: str | None = None
    degradation: dict[str, Any] | None = None
    payload: Any = None

    @classmethod
    def from_json(cls, name: str, raw: Mapping[str, Any], payload: Any) -> "Slice":
        if not isinstance(raw, Mapping):
            raise AdcError("INVALID_BODY", f"slice descriptor {name!r} must be an object")
        return cls(
            name=name,
            tokens=int(raw.get("tokens", -1)),
            bytes=int(raw.get("bytes", -1)),
            generation=str(raw.get("generation", "")),
            generated_at=raw.get("generated_at"),
            dynamic=bool(raw.get("dynamic", False)),
            provenance=str(raw.get("provenance", "publisher")),
            moderated=raw.get("moderated"),
            default=bool(raw.get("default", False)),
            price=_maybe_float(raw.get("price")),
            currency=raw.get("currency"),
            unit=raw.get("unit"),
            degradation=raw.get("degradation"),
            payload=payload,
        )

    @property
    def hydration_mode(self) -> str:
        if self.degradation and isinstance(self.degradation, Mapping):
            return str(self.degradation.get("mode", "fail_closed"))
        return "fail_closed"

    @property
    def max_staleness_seconds(self) -> int | None:
        if self.degradation and isinstance(self.degradation, Mapping):
            value = self.degradation.get("max_staleness_seconds")
            return int(value) if value is not None else None
        return None


def _maybe_float(value: Any) -> float | None:
    if value is None or isinstance(value, bool):
        return None
    try:
        return float(value)
    except (TypeError, ValueError):
        return None


@dataclass
class Node:
    path: str
    name: str
    anchor: str
    kind: str
    has_children: bool
    updated_at: str | None
    count: int | None = None
    passport: dict[str, Any] = field(default_factory=dict)
    slices: dict[str, Slice] = field(default_factory=dict)
    children: list[str] = field(default_factory=list)

    @property
    def depth(self) -> int:
        return path_depth(self.path)

    @property
    def default_slice(self) -> Slice:
        for slice_ in self.slices.values():
            if slice_.default:
                return slice_
        raise AdcError(
            "INVALID_BODY", f"node {self.path!r} declares no default slice"
        )

    def slice(self, name: str) -> Slice:
        try:
            return self.slices[name]
        except KeyError:
            raise AdcError(
                "SLICE_NOT_FOUND",
                f"node {self.path!r} has no slice {name!r}",
                details={"available": sorted(self.slices)},
            ) from None

    @property
    def filter_specs(self) -> dict[str, FilterSpec]:
        return filter_specs(self.passport.get("filters"))

    @property
    def item_records(self) -> list[dict[str, Any]]:
        """List rows of a collection ``summary`` payload."""
        payload = self.default_slice.payload or {}
        if isinstance(payload, Mapping) and isinstance(payload.get("items"), list):
            return list(payload["items"])
        return []

    @property
    def fields(self) -> dict[str, Any]:
        payload = self.default_slice.payload or {}
        if isinstance(payload, Mapping) and isinstance(payload.get("fields"), Mapping):
            return dict(payload["fields"])
        return {}


class Site:
    """An in-memory, validated ADC circuit."""

    def __init__(self, directory: Path) -> None:
        self.directory = Path(directory)
        self.manifest: dict[str, Any] = {}
        self.nodes: dict[str, Node] = {}
        self.anchors: AnchorTable | None = None
        self.ai: dict[str, Any] | None = None

    # -- construction ----------------------------------------------------

    @classmethod
    def load(cls, directory: str | Path) -> "Site":
        site = cls(Path(directory))
        site._load_manifest()
        site._load_tree()
        site._load_nodes()
        site._link_children()
        site._load_ai()
        return site

    @property
    def tree_generation(self) -> str:
        return str(self.manifest.get("tree_generation", ""))

    @property
    def tokenizer(self) -> str:
        return str(self.manifest.get("tokenizer", "utf8_bytes_div_4"))

    @property
    def limits(self) -> dict[str, Any]:
        raw = self.manifest.get("limits")
        return dict(raw) if isinstance(raw, Mapping) else {}

    @property
    def defaults(self) -> dict[str, Any]:
        raw = self.manifest.get("defaults")
        return dict(raw) if isinstance(raw, Mapping) else {}

    @property
    def endpoints(self) -> dict[str, str]:
        raw = self.manifest.get("endpoints")
        return dict(raw) if isinstance(raw, Mapping) else {}

    @property
    def namespace(self) -> str:
        return str(self.manifest.get("namespace", ""))

    @property
    def root(self) -> str:
        return str(self.manifest.get("root", ""))

    def _read_json(self, path: Path) -> Any:
        if not path.exists():
            raise AdcError("NOT_IMPLEMENTED", f"missing site file: {path.name}")
        with path.open("r", encoding="utf-8") as handle:
            return json.load(handle)

    def _load_manifest(self) -> None:
        raw = self._read_json(self.directory / "manifest.json")
        if not isinstance(raw, dict):
            raise AdcError("INVALID_BODY", "manifest.json must be a JSON object")
        for required in ("adc", "root", "tree_generation", "tokenizer"):
            if required not in raw:
                raise AdcError(
                    "INVALID_BODY", f"manifest is missing required key {required!r}"
                )
        self.manifest = raw

    def _load_tree(self) -> None:
        raw = self._read_json(self.directory / "tree.json")
        if not isinstance(raw, dict) or not isinstance(raw.get("nodes"), list):
            raise AdcError("INVALID_BODY", "tree.json must be an object with a nodes array")
        generation = raw.get("tree_generation")
        if generation != self.tree_generation:
            raise AdcError(
                "GENERATION_STALE",
                "tree.json tree_generation does not match manifest.json",
                details={"manifest": self.tree_generation, "tree": generation},
            )
        entries = raw["nodes"]
        for entry in entries:
            if not isinstance(entry, dict):
                raise AdcError("INVALID_BODY", "every tree node must be an object")
            path = canonicalize_path(str(entry.get("path", "")))
            kind = entry.get("kind")
            if kind not in ("collection", "entity", "document"):
                raise AdcError(
                    "INVALID_BODY", f"node {path!r} has unknown kind {kind!r}"
                )
            if path in self.nodes:
                raise AdcError("PATH_INVALID", f"duplicate node path {path!r}")
            self.nodes[path] = Node(
                path=path,
                name=str(entry.get("name", "")),
                anchor=str(entry.get("anchor", "")),
                kind=str(kind),
                has_children=bool(entry.get("has_children", False)),
                updated_at=entry.get("updated_at"),
                count=int(entry["count"]) if entry.get("count") is not None else None,
            )
        # Issuance must be reproducible (SPEC 5.3.1).
        expected = AnchorTable.from_paths(self.tree_generation, list(self.nodes))
        self.anchors = expected
        for path, node in self.nodes.items():
            issued = expected.anchor_for(path)
            if node.anchor != issued:
                raise AdcError(
                    "ANCHOR_INVALID",
                    f"node {path!r} declares {node.anchor!r} but issuance yields {issued!r}",
                    details={"path": path, "declared": node.anchor, "issued": issued},
                )

    def _load_nodes(self) -> None:
        nodes_dir = self.directory / "nodes"
        if not nodes_dir.is_dir():
            raise AdcError("NOT_IMPLEMENTED", "site has no nodes/ directory")
        for path, node in self.nodes.items():
            anchor_id = node.anchor.strip("[]")[1:]
            payload = self._read_json(nodes_dir / f"a{anchor_id}.json")
            if not isinstance(payload, dict):
                raise AdcError("INVALID_BODY", f"{node.anchor} payload must be an object")
            passport = payload.get("passport")
            slices_raw = payload.get("slices")
            if not isinstance(passport, dict) or not isinstance(slices_raw, dict):
                raise AdcError(
                    "INVALID_BODY", f"{node.anchor} must carry passport and slices objects"
                )
            node.passport = passport
            for name, descriptor in slices_raw.items():
                if name not in passport.get("slices", []):
                    # descriptor list in passport may be list of objects
                    pass
                declared = _find_slice_descriptor(passport.get("slices"), name)
                slice_payload = _payload_of(descriptor)
                node.slices[name] = Slice.from_json(name, declared or {}, slice_payload)

    def _link_children(self) -> None:
        for path in self.nodes:
            parent = parent_path(path)
            if parent is not None and parent in self.nodes and parent != path:
                self.nodes[parent].children.append(path)
        for node in self.nodes.values():
            node.children.sort()

    def _load_ai(self) -> None:
        """Load the optional ``/AI/`` cold entry map (AI-ENTRY.md).

        A site directory without ``ai.json`` simply publishes no cold entry;
        the probe then 404s and agents fall back to ordinary parsing.
        """
        path = self.directory / "ai.json"
        if not path.exists():
            return
        raw = self._read_json(path)
        if not isinstance(raw, dict):
            raise AdcError("INVALID_BODY", "ai.json must be a JSON object")
        if not isinstance(raw.get("ai"), str) or not raw.get("ai"):
            raise AdcError("INVALID_BODY", "ai.json must carry an \"ai\" version string")
        variables = raw.get("vars")
        keys = raw.get("keys")
        if not isinstance(variables, dict):
            raise AdcError("INVALID_BODY", "ai.json must carry a 'vars' object")
        if not isinstance(keys, list):
            raise AdcError("INVALID_BODY", "ai.json must carry a 'keys' array")
        for name, target in variables.items():
            if not isinstance(name, str) or not isinstance(target, str):
                raise AdcError("INVALID_BODY", "ai.json vars must map strings to strings")
            if not name:
                raise AdcError("INVALID_BODY", "ai.json var names must be non-empty")
        if not all(isinstance(key, str) and key for key in keys):
            raise AdcError("INVALID_BODY", "ai.json keys must be non-empty strings")
        self.ai = raw

    # -- accessors -------------------------------------------------------

    def resolve(self, *, anchor: str | None = None, path: str | None = None) -> Node:
        if (anchor is None) == (path is None):
            raise AdcError("ANCHOR_INVALID", "supply exactly one of anchor or path")
        if anchor is not None:
            assert self.anchors is not None
            target = self.anchors.path_for(anchor)
        else:
            target = canonicalize_path(str(path))
        try:
            return self.nodes[target]
        except KeyError:
            raise AdcError(
                "ANCHOR_NOT_FOUND",
                f"no node at {target!r}",
                tree_generation=self.tree_generation,
                details={"path": target},
            ) from None

    def subordinates(self, path: str, depth: int = 1) -> list[Node]:
        """Nodes at 1..``depth`` levels below ``path``, deterministic order."""
        wanted: list[Node] = []
        frontier = [path]
        for _ in range(max(1, depth)):
            next_frontier: list[str] = []
            for current in frontier:
                for child in self.nodes[current].children:
                    wanted.append(self.nodes[child])
                    next_frontier.append(child)
            frontier = next_frontier
        return wanted

    def records(self, node: Node, slice_name: str | None = None) -> list[dict[str, Any]]:
        """Flatten a node into filterable records.

        Collections expand to one record per child, annotated with
        ``anchor``/``path``/``generation``. Entities yield their own fields.
        """
        slice_ = node.slice(slice_name) if slice_name else node.default_slice
        payload = slice_.payload
        if node.kind == "collection" and isinstance(payload, Mapping):
            rows: list[dict[str, Any]] = []
            for item in payload.get("items", []):
                if not isinstance(item, Mapping):
                    continue
                child_path = str(item.get("path", ""))
                child = self.nodes.get(child_path)
                if child is None:
                    continue
                record = dict(item.get("fields") or {})
                record["anchor"] = child.anchor
                record["path"] = child.path
                record["generation"] = child.slices.get(slice_.name).generation if slice_.name in child.slices else child.default_slice.generation
                rows.append(record)
            return rows
        if isinstance(payload, Mapping):
            record = dict(payload.get("fields") or {})
            record["anchor"] = node.anchor
            record["path"] = node.path
            record["generation"] = slice_.generation
            return [record]
        return []


def _find_slice_descriptor(raw: Any, name: str) -> dict[str, Any] | None:
    if isinstance(raw, Mapping):
        value = raw.get(name)
        return dict(value) if isinstance(value, Mapping) else None
    if isinstance(raw, list):
        for entry in raw:
            if isinstance(entry, Mapping) and entry.get("name") == name:
                return dict(entry)
    return None


def _payload_of(descriptor: Any) -> Any:
    if isinstance(descriptor, Mapping):
        for key in ("items", "fields"):
            if key in descriptor:
                return descriptor
        return descriptor
    return descriptor


# --------------------------------------------------------------------------
# Structural validation
# --------------------------------------------------------------------------


def _check_ai_entry(site: Site) -> list[str]:
    """Validate the optional ``/AI/`` map against the circuit and AI-ENTRY.md."""
    problems: list[str] = []
    entry = site.ai
    if entry is None:
        return problems
    for name, target in (entry.get("vars") or {}).items():
        try:
            resolved = canonicalize_path(str(target))
        except AdcError as exc:
            problems.append(f"ai.json var {name!r}: {exc.detail}")
            continue
        if resolved not in site.nodes:
            problems.append(
                f"ai.json var {name!r} points at unknown path {resolved!r}"
            )
    for key in entry.get("keys") or []:
        if key not in AI_KEYS:
            problems.append(f"ai.json declares unknown key {key!r}")
    return problems


def check_site(directory: str | Path) -> list[str]:
    """Return a list of problems. Empty list means the site is conforming."""
    problems: list[str] = []
    try:
        site = Site.load(directory)
    except AdcError as exc:
        return [f"{exc.code}: {exc.detail}"]
    except (json.JSONDecodeError, OSError) as exc:
        return [f"INVALID_BODY: {exc}"]

    manifest = site.manifest
    problems.extend(_check_ai_entry(site))
    if manifest.get("tokenizer") == "utf8_bytes_div_4":
        pass
    elif str(manifest.get("tokenizer")) not in ("cl100k_base", "o200k_base"):
        tokenizer = str(manifest.get("tokenizer", ""))
        if "://" not in tokenizer:
            problems.append(f"manifest tokenizer {tokenizer!r} is not a known vocabulary")

    limits = site.limits
    max_depth = int(limits.get("max_depth", 4))
    max_nodes = int(limits.get("max_tree_nodes", 500))
    if len(site.nodes) > max_nodes:
        problems.append(
            f"tree declares {len(site.nodes)} nodes but limits.max_tree_nodes is {max_nodes}"
        )

    for path, node in sorted(site.nodes.items()):
        if path_depth(path) > max_depth:
            problems.append(f"{path}: depth {path_depth(path)} exceeds limits.max_depth {max_depth}")
        if not node.name:
            problems.append(f"{path}: empty node name")
        if node.kind == "collection":
            if node.count is None:
                problems.append(f"{path}: collection is missing count")
            elif node.count != len(node.item_records):
                problems.append(
                    f"{path}: count {node.count} != {len(node.item_records)} summary items"
                )
        elif node.count is not None:
            problems.append(f"{path}: {node.kind} must not declare count")

        # passport must agree with the tree entry
        for key, expected in (("path", path), ("anchor", node.anchor), ("kind", node.kind)):
            if node.passport.get(key) != expected:
                problems.append(
                    f"{path}: passport.{key}={node.passport.get(key)!r} disagrees with tree {expected!r}"
                )

        # filters must parse
        try:
            node.filter_specs
        except AdcError as exc:
            problems.append(f"{path}: {exc.code}: {exc.detail}")

        # aggregations
        aggregations = node.passport.get("aggregations")
        if isinstance(aggregations, dict) and aggregations.get("supported"):
            for expr in aggregations.get("metrics", []):
                try:
                    parse_metric(str(expr))
                except AdcError as exc:
                    problems.append(f"{path}: aggregation metric {expr!r}: {exc.detail}")
            try:
                parse_group_by(aggregations.get("group_by"))
            except AdcError as exc:
                problems.append(f"{path}: aggregation group_by: {exc.detail}")

        # slices
        if not node.slices:
            problems.append(f"{path}: node declares no slices")
        defaults = [s for s in node.slices.values() if s.default]
        if len(defaults) != 1:
            problems.append(f"{path}: expected exactly one default slice, found {len(defaults)}")
        for name, slice_ in node.slices.items():
            if not slice_.generation:
                problems.append(f"{path}/{name}: slice is missing generation")
            if slice_.payload is None:
                problems.append(f"{path}/{name}: slice has no payload")
                continue
            tokens, byte_len, generation = payload_stats(slice_.payload, site.tokenizer)
            if slice_.tokens != tokens:
                problems.append(
                    f"{path}/{name}: declared tokens {slice_.tokens} != computed {tokens}"
                )
            if slice_.bytes != byte_len:
                problems.append(f"{path}/{name}: declared bytes {slice_.bytes} != computed {byte_len}")
            if slice_.generation != generation:
                problems.append(
                    f"{path}/{name}: declared generation {slice_.generation} != computed {generation}"
                )
            if slice_.dynamic and slice_.hydration_mode not in (
                "stale_fallback",
                "partial_omit",
                "fail_closed",
            ):
                problems.append(
                    f"{path}/{name}: dynamic slice declares unknown degradation mode {slice_.hydration_mode!r}"
                )
            if slice_.provenance not in ("publisher", "user_generated", "third_party"):
                problems.append(f"{path}/{name}: unknown provenance {slice_.provenance!r}")

        # children linkage
        expected_children = [p for p in site.nodes if parent_path(p) == path]
        if node.has_children != bool(expected_children):
            problems.append(
                f"{path}: has_children={node.has_children} disagrees with actual children"
            )

        # collection item integrity
        for item in node.item_records:
            child_path = str(item.get("path", ""))
            child = site.nodes.get(child_path)
            if child is None:
                problems.append(f"{path}: summary item points at unknown path {child_path!r}")
                continue
            if parent_path(child_path) != path:
                problems.append(f"{path}: summary item {child_path!r} is not a direct child")

    return problems