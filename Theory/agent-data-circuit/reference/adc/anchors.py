"""Anchors: issuance, wire parsing, and agent-local arithmetic.

Three distinct grammars live here (SPEC.md 5.3, Annex B.1):

1. **Wire grammar** ``raw-anchor``: ``[a10]-3/beta``. This is the only form
   that ever appears in an HTTP query or body.
2. **Agent expression grammar**: a namespaced anchor optionally bound to a
   local alias, e.g. ``https://shop.example.com#[a10]=headphones``. The
   ``=alias`` tail is local-only and MUST NOT be sent on the wire.
3. **Namespaced anchor**: ``<origin-authority>#<raw-anchor>``, enforced by the
   runtime so an anchor minted for one origin is never dispatched to another.

Issuance is idempotent within a tree generation: the reference site assigns
anchors by lexicographic rank of the canonical path, so the same path always
yields the same anchor across ``tree``, ``passport`` and ``fetch``.
"""

from __future__ import annotations

import re
from dataclasses import dataclass
from typing import Iterable, Mapping

from .errors import AdcError
from .pathing import canonicalize_path, is_prefix_of, path_depth, path_segments

__all__ = [
    "ANCHOR_RE",
    "AnchorError",
    "AnchorTable",
    "NamespacedAnchor",
    "parse_namespaced_anchor",
    "parse_wire_anchor",
    "resolve_suffix",
    "rollback_to",
    "strip_namespace",
    "validate_namespace",
    "normalize_origin",
]

ANCHOR_RE = re.compile(r"^\[a(?P<id>\d+)\](?:-(?P<rollback>\d+))?(?:/(?P<suffix>.+))?$")
_NS_RE = re.compile(r"^(?P<ns>.+?)#(?P<anchor>\[a\d+\].*)$")
_ORIGIN_RE = re.compile(r"^(?P<scheme>https?)://(?P<host>[^:/?#]+)(?::(?P<port>\d+))?$")


class AnchorError(Exception):
    """An agent-local anchor failure. Never emitted over HTTP."""

    def __init__(self, code: str, detail: str) -> None:
        self.code = code
        self.detail = detail
        super().__init__(f"{code}: {detail}")


@dataclass(frozen=True)
class ParsedAnchor:
    anchor_id: int
    rollback: int | None
    suffix: str | None
    raw: str


def parse_wire_anchor(token: str) -> ParsedAnchor:
    """Parse a wire anchor, raising ``ANCHOR_INVALID`` on malformed input."""
    if not isinstance(token, str):
        raise AdcError("ANCHOR_INVALID", "anchor must be a string")
    m = ANCHOR_RE.match(token)
    if not m:
        raise AdcError(
            "ANCHOR_INVALID",
            f"anchor does not match the wire grammar: {token!r}",
            details={"expected": "[a<digits>] with optional -k rollback and /suffix"},
        )
    rb = m.group("rollback")
    if rb is not None and int(rb) < 1:
        raise AdcError("ANCHOR_INVALID", "rollback depth k must be >= 1")
    return ParsedAnchor(
        anchor_id=int(m.group("id")),
        rollback=int(rb) if rb is not None else None,
        suffix=m.group("suffix"),
        raw=token,
    )


def strip_namespace(token: str) -> str:
    """Return the raw wire anchor, dropping any ``origin#`` prefix."""
    m = _NS_RE.match(token)
    return m.group("anchor") if m else token


def parse_namespaced_anchor(token: str) -> tuple[str, str]:
    """Split ``origin#raw`` into its parts."""
    m = _NS_RE.match(token)
    if not m:
        raise AnchorError("NAMESPACE_INVALID", f"not a namespaced anchor: {token!r}")
    return m.group("ns"), m.group("anchor")


def validate_namespace(token: str, expected_origin: str) -> str:
    """Ensure a namespaced token belongs to ``expected_origin``.

    Returns the raw wire anchor. Raises :class:`AnchorError`
    (``NAMESPACE_MISMATCH``) when the origins differ.
    """
    m = _NS_RE.match(token)
    if not m:
        return token
    if normalize_origin(m.group("ns")) != normalize_origin(expected_origin):
        raise AnchorError(
            "NAMESPACE_MISMATCH",
            f"anchor namespace {m.group('ns')!r} does not match origin {expected_origin!r}",
        )
    return m.group("anchor")


def normalize_origin(origin: str) -> str:
    """Lowercase scheme/host, punycode the host, elide default ports (RFC 3986)."""
    m = _ORIGIN_RE.match(origin.strip())
    if not m:
        raise AnchorError(
            "NAMESPACE_INVALID", f"not a valid origin authority: {origin!r}"
        )
    scheme = m.group("scheme").lower()
    host = m.group("host").lower()
    if not host.startswith("xn--"):
        try:
            host = host.encode("idna").decode("ascii")
        except (UnicodeError, UnicodeDecodeError):
            pass
    port = m.group("port")
    if port is not None and (
        (scheme == "http" and port == "80") or (scheme == "https" and port == "443")
    ):
        port = None
    authority = host if port is None else f"{host}:{port}"
    return f"{scheme}://{authority}"


@dataclass(frozen=True)
class NamespacedAnchor:
    origin: str
    anchor: str
    alias: str | None = None

    @classmethod
    def parse(cls, token: str) -> "NamespacedAnchor":
        """Parse ``origin#raw-anchor[=alias]`` (agent-local grammar, Annex B.1)."""
        hash_at = token.find("#")
        if hash_at <= 0:
            raise AnchorError("NAMESPACE_INVALID", f"not a namespaced anchor: {token!r}")
        origin = token[:hash_at]
        rest = token[hash_at + 1 :]
        alias: str | None = None
        if "=" in rest:
            rest, alias = rest.split("=", 1)
            if not alias or not re.fullmatch(r"[A-Za-z0-9_-]+", alias):
                raise AnchorError("ALIAS_INVALID", f"bad alias: {alias!r}")
        parse_wire_anchor(rest)
        return cls(origin=origin, anchor=rest, alias=alias)

    def to_wire(self) -> str:
        """The form sent on the wire: namespace and alias stripped."""
        return self.anchor

    def to_expression(self) -> str:
        base = f"{self.origin}#{self.anchor}"
        return f"{base}={self.alias}" if self.alias else base


def rollback_to(anchor: str, k: int, depths: Mapping[str, int]) -> str:
    """Agent-local ``[aN]-k`` against a ``{anchor: depth}`` table.

    Raises :class:`AnchorError` (``ROLLBACK_INVALID`` / ``NO_SUCH_ANCESTOR``);
    never emits a wire request.
    """
    if k < 1:
        raise AnchorError("ROLLBACK_INVALID", f"rollback depth must be >= 1, got {k}")
    parsed = parse_wire_anchor(anchor)
    depth = depths.get(anchor)
    if depth is None:
        raise AnchorError("NO_SUCH_ANCESTOR", f"unknown anchor {anchor!r}")
    if k > depth:
        raise AnchorError(
            "NO_SUCH_ANCESTOR",
            f"rollback {k} exceeds depth {depth} for {anchor!r}",
        )
    return f"[a{parsed.anchor_id}]-{k}"


def resolve_suffix(
    anchor_path: str, suffix: str, all_paths: Iterable[str]
) -> str:
    """Resolve ``[aN]/suffix`` against the known path set.

    The suffix is a run of segments beginning at the first child of
    ``anchor_path``, matched case-sensitively on segment boundaries.

    * zero matches -> ``NO_SUCH_CHILD``
    * more than one -> ``AMBIGUOUS_CHILD``

    Servers are never asked to disambiguate.
    """
    if not suffix:
        raise AnchorError("NO_SUCH_CHILD", "empty suffix")
    if "/" in suffix and any(
        seg in ("", ".", "..") for seg in suffix.split("/")
    ):
        raise AnchorError("NO_SUCH_CHILD", f"malformed suffix: {suffix!r}")

    base_segments = path_segments(anchor_path)
    want = suffix.split("/")
    results = [
        p
        for p in all_paths
        if is_prefix_of(anchor_path, p) and p != anchor_path
        and path_segments(p)[len(base_segments) : len(base_segments) + len(want)] == want
        and len(path_segments(p)) >= len(base_segments) + len(want)
    ]
    if not results:
        raise AnchorError(
            "NO_SUCH_CHILD",
            f"no descendant of {anchor_path!r} matches suffix {suffix!r}",
        )
    if len(results) > 1:
        raise AnchorError(
            "AMBIGUOUS_CHILD",
            f"suffix {suffix!r} matches {len(results)} descendants of {anchor_path!r}",
            # details are for local logging, not the wire
        )
    return results[0]


class AnchorTable:
    """Bidirectional, generation-bound path <-> anchor table."""

    def __init__(self, tree_generation: str) -> None:
        self.tree_generation = tree_generation
        self._by_anchor: dict[str, str] = {}
        self._by_path: dict[str, str] = {}

    @classmethod
    def from_paths(cls, tree_generation: str, paths: Iterable[str]) -> "AnchorTable":
        table = cls(tree_generation)
        for rank, path in enumerate(sorted({canonicalize_path(p) for p in paths}), 1):
            anchor = f"[a{rank}]"
            table._by_anchor[anchor] = path
            table._by_path[path] = anchor
        return table

    def anchor_for(self, path: str) -> str:
        path = canonicalize_path(path)
        try:
            return self._by_path[path]
        except KeyError:
            raise AdcError(
                "ANCHOR_NOT_FOUND",
                f"no node at path {path!r} in generation {self.tree_generation}",
                tree_generation=self.tree_generation,
                details={"path": path},
            ) from None

    def path_for(self, anchor: str) -> str:
        parsed = parse_wire_anchor(anchor)
        if parsed.rollback is not None or parsed.suffix is not None:
            raise AdcError(
                "ANCHOR_INVALID",
                "rollback and suffix are agent-local; the server resolves bare anchors only",
            )
        key = f"[a{parsed.anchor_id}]"
        try:
            return self._by_anchor[key]
        except KeyError:
            raise AdcError(
                "ANCHOR_NOT_FOUND",
                f"anchor {key} is not issued in generation {self.tree_generation}",
                tree_generation=self.tree_generation,
            ) from None

    def resolve(self, *, anchor: str | None, path: str | None) -> tuple[str, str]:
        """Return ``(anchor, path)`` from exactly one of the two inputs."""
        if (anchor is None) == (path is None):
            raise AdcError(
                "ANCHOR_INVALID", "supply exactly one of anchor or path"
            )
        if anchor is not None:
            p = self.path_for(anchor)
            return self.anchor_for(p), p
        p = canonicalize_path(path)  # type: ignore[arg-type]
        return self.anchor_for(p), p

    def contains_anchor(self, anchor: str) -> bool:
        return anchor in self._by_anchor

    def contains_path(self, path: str) -> bool:
        return path in self._by_path

    def __len__(self) -> int:
        return len(self._by_anchor)

    def snapshot(self) -> dict[str, str]:
        return dict(self._by_path)

    def depth_of(self, anchor: str) -> int:
        return path_depth(self.path_for(anchor))