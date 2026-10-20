"""Filter parsing and evaluation (SPEC.md 6.4.1 GET, 6.4.5 POST, Annex B.2).

One operator vocabulary serves both shapes:

* **GET** is a flat list of field conditions built from query parameters.
* **POST** is a recursive boolean tree (``and`` / ``or`` / ``not``) of the
  same field conditions.

Both validate against the passport's declared ``filters`` before anything is
executed, and both enforce ``limits.max_filter_conditions`` and
``limits.max_filter_depth``.
"""

from __future__ import annotations

import re
from dataclasses import dataclass, field
from typing import Any, Iterable, Mapping, Sequence

from .errors import AdcError

__all__ = [
    "FilterSpec",
    "OPS",
    "LIST_OPS",
    "GETFilter",
    "PostFilterNode",
    "conditions_from_query",
    "conditions_match",
    "coerce_value",
    "filter_specs",
    "parse_post_filter",
    "post_filter_match",
]

OPS = (
    "eq",
    "neq",
    "lt",
    "lte",
    "gt",
    "gte",
    "in",
    "nin",
    "contains",
    "contains_all",
    "contains_any",
    "contains_none",
    "prefix",
    "exists",
)

#: Operators whose value is a repeated/list parameter, never a comma list.
LIST_OPS = ("in", "nin", "contains_all", "contains_any", "contains_none")

FIELD_TYPES = ("string", "number", "boolean", "date", "enum")

_FILTER_KEY_RE = re.compile(r"^filter\[(?P<name>[A-Za-z0-9_.]+)\](?P<rest>\[([a-z_]+)\])?$")
_FIELD_NAME_RE = re.compile(r"^[A-Za-z0-9_.]+$")


@dataclass(frozen=True)
class FilterSpec:
    """A field descriptor declared in a passport ``filters`` entry."""

    name: str
    type: str
    ops: tuple[str, ...] = ("eq",)
    enum: tuple[Any, ...] | None = None
    unit: str | None = None
    minimum: float | None = None
    maximum: float | None = None

    @classmethod
    def from_json(cls, raw: Any) -> "FilterSpec":
        if isinstance(raw, str):
            # v0.1-compatible plain string form: eq only.
            return cls(name=raw, type="string", ops=("eq",))
        if not isinstance(raw, Mapping):
            raise AdcError(
                "FILTER_INVALID",
                f"filter descriptor must be a string or object, got {type(raw).__name__}",
            )
        name = raw.get("name")
        if not isinstance(name, str) or not _FIELD_NAME_RE.match(name):
            raise AdcError("FILTER_INVALID", f"bad filter name: {name!r}")
        ftype = raw.get("type", "string")
        if ftype not in FIELD_TYPES:
            raise AdcError(
                "FILTER_INVALID",
                f"filter {name!r} declares unknown type {ftype!r}",
                details={"field": name, "allowed": list(FIELD_TYPES)},
            )
        ops = raw.get("ops", ["eq"])
        if not isinstance(ops, (list, tuple)) or not ops:
            raise AdcError(
                "FILTER_INVALID",
                f"filter {name!r} must declare a non-empty ops list",
                details={"field": name},
            )
        for op in ops:
            if op not in OPS:
                raise AdcError(
                    "FILTER_INVALID",
                    f"filter {name!r} declares unknown op {op!r}",
                    details={"field": name, "allowed": list(OPS)},
                )
        enum = raw.get("enum")
        return cls(
            name=name,
            type=ftype,
            ops=tuple(ops),
            enum=tuple(enum) if isinstance(enum, (list, tuple)) else None,
            unit=raw.get("unit"),
            minimum=_num(raw.get("min")),
            maximum=_num(raw.get("max")),
        )

    def allows(self, op: str) -> bool:
        return op in self.ops


def _num(value: Any) -> float | None:
    if isinstance(value, bool) or value is None:
        return None
    if isinstance(value, (int, float)):
        return float(value)
    return None


def filter_specs(raw: Iterable[Any] | None) -> dict[str, FilterSpec]:
    """Build ``{name: FilterSpec}`` from a passport ``filters`` member."""
    out: dict[str, FilterSpec] = {}
    for entry in raw or []:
        spec = FilterSpec.from_json(entry)
        if spec.name in out:
            raise AdcError(
                "FILTER_INVALID",
                f"duplicate filter descriptor for {spec.name!r}",
            )
        out[spec.name] = spec
    return out


def coerce_value(spec: FilterSpec, value: Any) -> Any:
    """Coerce a wire value to the declared field type, or raise FILTER_INVALID."""
    if spec.type == "number":
        if isinstance(value, bool):
            raise AdcError(
                "FILTER_INVALID", f"{spec.name}: expected number, got boolean"
            )
        if isinstance(value, (int, float)):
            return float(value)
        try:
            return float(str(value))
        except (TypeError, ValueError):
            raise AdcError(
                "FILTER_INVALID", f"{spec.name}: {value!r} is not a number"
            ) from None
    if spec.type == "boolean":
        if isinstance(value, bool):
            return value
        low = str(value).lower()
        if low in ("true", "false"):
            return low == "true"
        raise AdcError(
            "FILTER_INVALID", f"{spec.name}: {value!r} is not a boolean"
        )
    if spec.type == "enum":
        if spec.enum is not None and value not in spec.enum:
            raise AdcError(
                "FILTER_INVALID",
                f"{spec.name}: {value!r} is not one of the declared enum values",
                details={"field": spec.name, "allowed": list(spec.enum)},
            )
        return value
    return str(value)


def _as_list(value: Any) -> list[Any]:
    return value if isinstance(value, list) else [value]


@dataclass(frozen=True)
class GETFilter:
    """A single validated GET filter condition."""

    name: str
    op: str
    value: Any

    def matches(self, record: Mapping[str, Any]) -> bool:
        return _compare(self.name, self.op, self.value, record)


def conditions_from_query(
    params: Mapping[str, Sequence[str]],
    specs: Mapping[str, FilterSpec],
    *,
    max_conditions: int = 32,
) -> list[GETFilter]:
    """Parse ``filter[field]`` / ``filter[field][op]`` query parameters.

    Repeated parameters carry list values (SPEC.md 6.4.1); comma-separated
    lists are never used. Multiple conditions on one field are ANDed by
    returning one :class:`GETFilter` per occurrence.
    """
    conditions: list[GETFilter] = []
    allowed = sorted(specs)

    for key, raw_values in params.items():
        if not key.startswith("filter["):
            continue
        m = _FILTER_KEY_RE.match(key)
        if not m:
            raise AdcError(
                "FILTER_INVALID",
                f"malformed filter parameter: {key!r}",
                details={"allowed": allowed},
            )
        name = m.group("name")
        spec = specs.get(name)
        if spec is None:
            raise AdcError(
                "FILTER_INVALID",
                f"unknown filter field: {name!r}",
                details={"unknown_fields": [name], "allowed": allowed},
            )
        rest = m.group("rest")
        if rest:
            op = rest[1:-1]
            if op not in OPS:
                raise AdcError(
                    "FILTER_INVALID",
                    f"unknown filter operator {op!r} on field {name!r}",
                    details={"unsupported_ops": [op], "allowed": list(OPS)},
                )
            if not spec.allows(op):
                raise AdcError(
                    "FILTER_INVALID",
                    f"operator {op!r} is not declared for field {name!r}",
                    details={"unsupported_ops": [op], "field": name, "declared": list(spec.ops)},
                )
        else:
            op = "eq"

        if op == "exists":
            raw_value = raw_values[0]
            if str(raw_value).lower() not in ("true", "false"):
                raise AdcError(
                    "FILTER_INVALID", f"{name}: exists takes true or false"
                )
            value: Any = str(raw_value).lower() == "true"
        elif op in LIST_OPS:
            value = [coerce_value(spec, v) for v in raw_values]
        else:
            if len(raw_values) > 1:
                raise AdcError(
                    "FILTER_INVALID",
                    f"{name}: operator {op!r} takes a single value; use {LIST_OPS[0]} for lists",
                )
            value = coerce_value(spec, raw_values[0])

        conditions.append(GETFilter(name=name, op=op, value=value))
        if len(conditions) > max_conditions:
            raise AdcError(
                "FILTER_INVALID",
                f"more than {max_conditions} filter conditions",
                details={"limit": max_conditions},
            )

    return conditions


def conditions_match(conditions: Sequence[GETFilter], record: Mapping[str, Any]) -> bool:
    return all(c.matches(record) for c in conditions)


# --------------------------------------------------------------------------
# POST boolean tree
# --------------------------------------------------------------------------


@dataclass
class PostFilterNode:
    kind: str  # "and" | "or" | "not" | "cond"
    children: list["PostFilterNode"] = field(default_factory=list)
    name: str | None = None
    op: str | None = None
    value: Any = None

    def matches(self, record: Mapping[str, Any]) -> bool:
        if self.kind == "and":
            return all(c.matches(record) for c in self.children)
        if self.kind == "or":
            return any(c.matches(record) for c in self.children)
        if self.kind == "not":
            return not self.children[0].matches(record)
        return _compare(self.name or "", self.op or "eq", self.value, record)

    @property
    def condition_count(self) -> int:
        if self.kind == "cond":
            return 1
        return sum(c.condition_count for c in self.children)

    @property
    def depth(self) -> int:
        if self.kind == "cond" or not self.children:
            return 1
        return 1 + max(c.depth for c in self.children)


def parse_post_filter(
    node: Any,
    specs: Mapping[str, FilterSpec],
    *,
    max_depth: int = 4,
    max_conditions: int = 32,
) -> PostFilterNode | None:
    """Validate a POST ``filter`` body into an evaluable tree.

    Returns ``None`` when ``node`` is ``None`` (no filtering). Raises
    ``FILTER_INVALID`` for schema violations, depth overruns, condition
    overruns, unknown fields, and undeclared operators.

    The ``max_depth`` / ``max_conditions`` bounds are normative DoS limits
    (SPEC 8.2) and are enforced on the whole tree, not per nesting level.
    """
    parsed = _parse_post_filter(node, specs)
    if parsed is None:
        return None
    if parsed.depth > max_depth:
        raise AdcError(
            "FILTER_INVALID",
            f"filter nesting depth {parsed.depth} exceeds limits.max_filter_depth {max_depth}",
            details={"depth": parsed.depth, "limit": max_depth},
        )
    if parsed.condition_count > max_conditions:
        raise AdcError(
            "FILTER_INVALID",
            f"{parsed.condition_count} filter conditions exceed "
            f"limits.max_filter_conditions {max_conditions}",
            details={"conditions": parsed.condition_count, "limit": max_conditions},
        )
    return parsed


def _parse_post_filter(
    node: Any,
    specs: Mapping[str, FilterSpec],
) -> PostFilterNode | None:
    if node is None:
        return None
    if not isinstance(node, Mapping):
        raise AdcError(
            "FILTER_INVALID",
            f"filter must be an object, got {type(node).__name__}",
        )
    allowed = sorted(specs)

    if len(node) != 1:
        raise AdcError(
            "FILTER_INVALID",
            "each filter node must hold exactly one key (and/or/not/field)",
            details={"keys": sorted(node), "allowed": allowed},
        )
    key, value = next(iter(node.items()))

    if key in ("and", "or"):
        if not isinstance(value, list) or not value:
            raise AdcError(
                "FILTER_INVALID", f"{key!r} requires a non-empty array"
            )
        parsed = [_parse_post_filter(v, specs) for v in value]
        return PostFilterNode(kind=key, children=[p for p in parsed if p])

    if key == "not":
        child = _parse_post_filter(value, specs)
        if child is None:
            raise AdcError("FILTER_INVALID", "'not' requires a nested filter")
        return PostFilterNode(kind="not", children=[child])

    # field condition
    spec = specs.get(key)
    if spec is None:
        raise AdcError(
            "FILTER_INVALID",
            f"unknown filter field: {key!r}",
            details={"unknown_fields": [key], "allowed": allowed},
        )
    if isinstance(value, Mapping):
        if len(value) != 1:
            raise AdcError(
                "FILTER_INVALID",
                f"condition for {key!r} must hold exactly one operator",
                details={"field": key, "allowed": list(spec.ops)},
            )
        op, operand = next(iter(value.items()))
        if op not in OPS:
            raise AdcError(
                "FILTER_INVALID",
                f"unknown filter operator {op!r} on field {key!r}",
                details={"unsupported_ops": [op], "allowed": list(OPS)},
            )
        if not spec.allows(op):
            raise AdcError(
                "FILTER_INVALID",
                f"operator {op!r} is not declared for field {key!r}",
                details={"unsupported_ops": [op], "field": key, "declared": list(spec.ops)},
            )
    else:
        op, operand = "eq", value
        if not spec.allows("eq"):
            raise AdcError(
                "FILTER_INVALID",
                f"field {key!r} does not declare the implicit 'eq' operator",
                details={"field": key, "declared": list(spec.ops)},
            )

    if op == "exists":
        if not isinstance(operand, bool):
            raise AdcError("FILTER_INVALID", f"{key}: exists takes a boolean")
        cooked: Any = operand
    elif op in LIST_OPS:
        if not isinstance(operand, list):
            raise AdcError(
                "FILTER_INVALID", f"{key}: {op} requires an array value"
            )
        cooked = [coerce_value(spec, v) for v in operand]
    else:
        cooked = coerce_value(spec, operand)

    return PostFilterNode(kind="cond", name=key, op=op, value=cooked)


def post_filter_match(
    root: PostFilterNode | None, record: Mapping[str, Any]
) -> bool:
    if root is None:
        return True
    return root.matches(record)


def _compare(name: str, op: str, expected: Any, record: Mapping[str, Any]) -> bool:
    """Apply one comparison, treating a missing field as ``None``."""
    if op == "exists":
        present = name in record and record[name] is not None
        return present if expected else not present

    actual = record.get(name)
    if actual is None:
        return op == "neq" and expected is not None

    if op == "eq":
        return actual == expected
    if op == "neq":
        return actual != expected
    if op == "prefix":
        return str(actual).startswith(str(expected))
    if op == "contains":
        return str(expected) in str(actual) if not isinstance(actual, list) else expected in actual
    if op == "in":
        return actual in expected
    if op == "nin":
        return actual not in expected
    if op == "contains_all":
        return all(x in actual for x in expected)
    if op == "contains_any":
        return any(x in actual for x in expected)
    if op == "contains_none":
        return not any(x in actual for x in expected)

    if op in ("lt", "lte", "gt", "gte"):
        try:
            left = float(actual)
            right = float(expected)
        except (TypeError, ValueError):
            left, right = str(actual), str(expected)
        if op == "lt":
            return left < right
        if op == "lte":
            return left <= right
        if op == "gt":
            return left > right
        return left >= right

    raise AdcError("FILTER_INVALID", f"unhandled operator {op!r}")