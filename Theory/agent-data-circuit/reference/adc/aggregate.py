"""Aggregate computation and k-anonymity (SPEC.md 6.9, 8.4, 8.5).

Metric expressions follow Annex B.3:

    metric-expr = metric-func "(" [ field-name ] ")" [ ":" alias ]

Group privacy is enforced here rather than trusted to the caller: groups
smaller than ``min_group_size`` are suppressed, and when any group is
suppressed the response must either omit ``totals`` or mark them
``totals_visible_only: true`` so suppressed data can never be folded in.
"""

from __future__ import annotations

import math
import re
import statistics
from dataclasses import dataclass, field
from typing import Any, Iterable, Mapping, Sequence

from .errors import AdcError

__all__ = [
    "METRIC_RE",
    "SUPPORTED_FUNCS",
    "OPTIONAL_FUNCS",
    "Metric",
    "parse_metric",
    "parse_metrics",
    "parse_order_by",
    "parse_having",
    "compute_aggregate",
]

#: Functions every Commerce implementation must support.
SUPPORTED_FUNCS = ("count", "sum", "avg", "min", "max")
#: Functions that must be advertised in ``aggregations.metrics`` to be used.
OPTIONAL_FUNCS = ("distinct", "p50", "p90", "p95", "p99", "median")

METRIC_RE = re.compile(
    r"^(?P<func>[a-z][a-z0-9]*)\((?P<field>[A-Za-z0-9_.]*)\)(?::(?P<alias>[A-Za-z0-9_]+))?$"
)
_ALIAS_RE = re.compile(r"^[A-Za-z0-9_]+$")


@dataclass(frozen=True)
class Metric:
    func: str
    field_name: str | None
    alias: str

    @property
    def expr(self) -> str:
        inner = self.field_name or ""
        return f"{self.func}({inner})"

    @property
    def requires_field(self) -> bool:
        return self.func != "count"

    @classmethod
    def from_json(cls, raw: Any) -> "Metric":
        if isinstance(raw, str):
            return parse_metric(raw)
        if isinstance(raw, Mapping):
            expr = raw.get("expr")
            if not isinstance(expr, str):
                raise AdcError(
                    "INVALID_AGGREGATION", "metric.expr must be a string"
                )
            metric = parse_metric(expr)
            alias = raw.get("alias", metric.alias)
            if not isinstance(alias, str) or not _ALIAS_RE.match(alias):
                raise AdcError(
                    "INVALID_AGGREGATION", f"bad metric alias: {alias!r}"
                )
            return cls(func=metric.func, field_name=metric.field_name, alias=alias)
        raise AdcError(
            "INVALID_AGGREGATION",
            f"metric must be a string or object, got {type(raw).__name__}",
        )


def parse_metric(expr: str) -> Metric:
    m = METRIC_RE.match(expr.strip()) if isinstance(expr, str) else None
    if not m:
        raise AdcError(
            "INVALID_AGGREGATION",
            f"metric does not match the Annex B.3 grammar: {expr!r}",
            details={"expected": "func(field)[:alias]"},
        )
    func = m.group("func")
    field_name = m.group("field") or None
    known = SUPPORTED_FUNCS + OPTIONAL_FUNCS
    if func not in known:
        raise AdcError(
            "INVALID_AGGREGATION",
            f"unsupported metric function {func!r}",
            details={"unsupported": [func], "allowed": list(known)},
        )
    if func == "count":
        # count() is legal; count(field) counts non-null values.
        pass
    elif not field_name:
        raise AdcError(
            "INVALID_AGGREGATION", f"{func}() requires a field argument"
        )
    alias = m.group("alias") or f"{func}_{field_name or 'all'}"
    return Metric(func=func, field_name=field_name, alias=alias)


def parse_metrics(raw: Any) -> list[Metric]:
    """Parse GET ``metrics=a,b`` or a POST ``metrics`` array."""
    if raw is None:
        raise AdcError("INVALID_AGGREGATION", "metrics is required")
    if isinstance(raw, str):
        parts = [p.strip() for p in raw.split(",") if p.strip()]
        if not parts:
            raise AdcError("INVALID_AGGREGATION", "metrics is required")
        metrics = [parse_metric(p) for p in parts]
    elif isinstance(raw, (list, tuple)):
        metrics = [Metric.from_json(p) for p in raw]
    else:
        raise AdcError(
            "INVALID_AGGREGATION",
            f"metrics must be a string or array, got {type(raw).__name__}",
        )
    aliases = [m.alias for m in metrics]
    if len(set(aliases)) != len(aliases):
        dupes = sorted({a for a in aliases if aliases.count(a) > 1})
        raise AdcError(
            "INVALID_AGGREGATION",
            f"duplicate metric alias(es): {', '.join(dupes)}",
            details={"duplicates": dupes},
        )
    return metrics


def parse_group_by(raw: Any) -> list[str]:
    if raw is None or raw == "":
        return []
    if isinstance(raw, str):
        fields = [p.strip() for p in raw.split(",") if p.strip()]
    elif isinstance(raw, (list, tuple)):
        fields = [str(p) for p in raw]
    else:
        raise AdcError(
            "INVALID_AGGREGATION_TARGET", "group_by must be a string or array"
        )
    for f in fields:
        if not _ALIAS_RE.match(f):
            raise AdcError(
                "INVALID_AGGREGATION_TARGET", f"bad group_by field: {f!r}"
            )
    return fields


def parse_order_by(raw: Any) -> list[tuple[str, bool]]:
    """Return ``[(field, descending), ...]``."""
    if raw is None or raw == "":
        return []
    if isinstance(raw, str):
        parts = [p.strip() for p in raw.split(",") if p.strip()]
    elif isinstance(raw, (list, tuple)):
        parts = [str(p) for p in raw]
    else:
        raise AdcError("INVALID_AGGREGATION", "order_by must be a string or array")
    out: list[tuple[str, bool]] = []
    for part in parts:
        if part.startswith("-"):
            out.append((part[1:], True))
        elif part.startswith("+"):
            out.append((part[1:], False))
        else:
            out.append((part, False))
    return out


def parse_having(raw: Any) -> list[tuple[str, str, Any]]:
    """Normalize ``having`` into ``[(alias, op, value), ...]``."""
    if raw is None or raw == "":
        return []
    items: list[Any]
    if isinstance(raw, str):
        items = [raw]
    elif isinstance(raw, (list, tuple)):
        items = list(raw)
    else:
        raise AdcError("INVALID_AGGREGATION", "having must be a string or array")

    out: list[tuple[str, str, Any]] = []
    for item in items:
        if isinstance(item, str):
            parts = item.split(":")
            if len(parts) != 3:
                raise AdcError(
                    "INVALID_AGGREGATION",
                    f"having must be alias:op:value, got {item!r}",
                )
            alias, op, value = parts
            out.append((alias, op, _auto(value)))
        elif isinstance(item, Mapping):
            alias = item.get("alias")
            op = item.get("op", "gt")
            if not isinstance(alias, str):
                raise AdcError("INVALID_AGGREGATION", "having.alias must be a string")
            out.append((alias, str(op), item.get("value")))
        else:
            raise AdcError("INVALID_AGGREGATION", "bad having entry")
    return out


def _auto(text: str) -> Any:
    if text in ("true", "false"):
        return text == "true"
    try:
        return int(text)
    except ValueError:
        pass
    try:
        return float(text)
    except ValueError:
        return text


# --------------------------------------------------------------------------
# Computation
# --------------------------------------------------------------------------


@dataclass
class AggConfig:
    metrics: list[Metric]
    group_by: list[str] = field(default_factory=list)
    having: list[tuple[str, str, Any]] = field(default_factory=list)
    order_by: list[tuple[str, bool]] = field(default_factory=list)
    limit: int = 10
    min_group_size: int = 5
    approximate: bool = False


def _aggregate(func: str, values: Sequence[Any]) -> Any:
    if func == "count":
        return len(values)
    nums = [float(v) for v in values if v is not None]
    if func == "sum":
        return _num(round(sum(nums), 6))
    if func == "avg":
        return _num(round(sum(nums) / len(nums), 6)) if nums else None
    if func == "min":
        return _num(min(nums)) if nums else None
    if func == "max":
        return _num(max(nums)) if nums else None
    if func == "distinct":
        return len({json_key(v) for v in values})
    if func == "median":
        return _num(statistics.median(nums)) if nums else None
    if func in ("p50", "p90", "p95", "p99"):
        if not nums:
            return None
        pct = {"p50": 50, "p90": 90, "p95": 95, "p99": 99}[func]
        return _num(_percentile(nums, pct))
    raise AdcError("INVALID_AGGREGATION", f"unhandled metric function {func!r}")


def _percentile(sorted_nums: list[float], pct: float) -> float:
    if len(sorted_nums) == 1:
        return sorted_nums[0]
    rank = (pct / 100.0) * (len(sorted_nums) - 1)
    lo = math.floor(rank)
    hi = math.ceil(rank)
    if lo == hi:
        return sorted_nums[int(rank)]
    frac = rank - lo
    return sorted_nums[lo] * (1 - frac) + sorted_nums[hi] * frac


def _num(value: Any) -> Any:
    """Return int when the float is integral, else float (clean JSON output)."""
    if isinstance(value, float) and value.is_integer():
        return int(value)
    return value


def json_key(value: Any) -> str:
    if isinstance(value, bool):
        return "true" if value else "false"
    if value is None:
        return ""
    return str(value)


def _compare_values(left: Any, right: Any) -> int:
    if isinstance(left, bool) or isinstance(right, bool):
        left, right = int(bool(left)), int(bool(right))
    if isinstance(left, (int, float)) and isinstance(right, (int, float)):
        return (left > right) - (left < right)
    left, right = str(left), str(right)
    return (left > right) - (left < right)


def compute_aggregate(
    records: Iterable[Mapping[str, Any]],
    config: AggConfig,
) -> dict[str, Any]:
    """Run the aggregate and apply the privacy invariants of SPEC.md 8.4/8.5."""
    if config.approximate:
        # The reference server always computes exactly. Advertising a lower
        # fidelity than we actually produced would be dishonest, so an
        # approximate request is answered with exact numbers and flagged as
        # such (the response carries approximate: false).
        pass

    rows = list(records)
    total_matched = len(rows)

    if not config.group_by:
        values: dict[str, Any] = {}
        for metric in config.metrics:
            if metric.requires_field:
                field_values = [r.get(metric.field_name) for r in rows]
                if metric.func in ("count",) :
                    values[metric.alias] = sum(1 for v in field_values if v is not None)
                else:
                    values[metric.alias] = _aggregate(metric.func, [v for v in field_values if v is not None])
            else:
                values[metric.alias] = _aggregate("count", rows)
        for alias, op, expected in config.having:
            if alias not in values:
                raise AdcError(
                    "INVALID_AGGREGATION", f"having references unknown alias {alias!r}"
                )
            if not _having_pass(values[alias], op, expected):
                return {
                    "values": {},
                    "totals_visible_only": False,
                    "suppressed_groups": 0,
                    "partial": False,
                    "coverage": 0.0,
                }
        return {
            "values": values,
            "totals_visible_only": False,
            "suppressed_groups": 0,
            "partial": False,
            "coverage": 1.0,
        }

    # Grouped.
    buckets: dict[tuple, list[Mapping[str, Any]]] = {}
    order_of_keys: list[tuple] = []
    for row in rows:
        key = tuple(json_key(row.get(f)) for f in config.group_by)
        if key not in buckets:
            buckets[key] = []
            order_of_keys.append(key)
        buckets[key].append(row)

    visible: list[dict[str, Any]] = []
    visible_sizes: list[int] = []
    suppressed = 0
    for key in sorted(order_of_keys):
        group_rows = buckets[key]
        if len(group_rows) < config.min_group_size:
            suppressed += 1
            continue
        values_out: dict[str, Any] = {}
        for metric in config.metrics:
            if metric.requires_field:
                field_values = [r.get(metric.field_name) for r in group_rows]
                if metric.func == "count":
                    values_out[metric.alias] = sum(
                        1 for v in field_values if v is not None
                    )
                else:
                    values_out[metric.alias] = _aggregate(
                        metric.func, [v for v in field_values if v is not None]
                    )
            else:
                values_out[metric.alias] = _aggregate("count", group_rows)
        if any(
            not _having_pass(values_out.get(a), op, v) for a, op, v in config.having
        ):
            continue
        key_obj = {f: _decode(k) for f, k in zip(config.group_by, key)}
        visible.append({"key": key_obj, "values": values_out})
        visible_sizes.append(len(group_rows))

    if config.order_by:
        pairs = list(zip(visible, visible_sizes))
        pairs = _stable_multi_sort(pairs, config.order_by)
        visible = [p[0] for p in pairs]
        visible_sizes = [p[1] for p in pairs]

    truncated = len(visible) > config.limit
    if truncated:
        visible = visible[: config.limit]
        visible_sizes = visible_sizes[: config.limit]

    result: dict[str, Any] = {
        "groups": visible,
        "suppressed_groups": suppressed,
        "partial": suppressed > 0,
        "coverage": round(sum(visible_sizes) / total_matched, 6)
        if total_matched
        else 0.0,
    }

    totals = _totals(visible, config.metrics)
    if totals is not None:
        # SPEC 8.4: when any group was suppressed the totals must be computed
        # over the visible groups only, and say so. Omitting them entirely is
        # the other permitted option; emitting them unflagged is not.
        result["totals"] = totals
        result["totals_visible_only"] = suppressed > 0

    result["truncated"] = truncated
    return result


def _totals(visible: Sequence[Mapping[str, Any]], metrics: Sequence[Metric]) -> dict[str, Any] | None:
    if not visible:
        return None
    # Totals are recomputed across the *visible* groups only.
    totals: dict[str, Any] = {}
    for metric in metrics:
        collected = [g["values"].get(metric.alias) for g in visible]
        collected = [c for c in collected if c is not None]
        if not collected:
            totals[metric.alias] = None
            continue
        if metric.func == "count":
            totals[metric.alias] = sum(int(c) for c in collected)
        elif metric.func in ("sum",):
            totals[metric.alias] = _num(round(sum(float(c) for c in collected), 6))
        elif metric.func in ("avg",):
            totals[metric.alias] = _num(round(sum(float(c) for c in collected) / len(collected), 6))
        elif metric.func == "min":
            totals[metric.alias] = _num(min(float(c) for c in collected))
        elif metric.func == "max":
            totals[metric.alias] = _num(max(float(c) for c in collected))
        else:
            totals[metric.alias] = None
    return totals


def _decode(token: str) -> Any:
    if token == "":
        return None
    if token == "true":
        return True
    if token == "false":
        return False
    try:
        if "." not in token:
            return int(token)
        return float(token)
    except ValueError:
        return token


def _sort_rank(value: Any) -> int:
    if value is None:
        return 0
    if isinstance(value, (int, float)) and not isinstance(value, bool):
        return 1
    return 2


def _order_key(entry: Mapping[str, Any], order_by: Sequence[tuple[str, bool]]) -> tuple:
    """Primary key only; used with :func:`_stable_multi_sort` for mixed direction."""
    if not order_by:
        return ()
    name, _ = order_by[0]
    store = entry["key"] if name in entry["key"] else entry["values"]
    value = store.get(name)
    return (_sort_rank(value), value if value is not None else "")


def _stable_multi_sort(
    entries: list[dict[str, Any]], order_by: Sequence[tuple[str, bool]]
) -> list[dict[str, Any]]:
    """Sort by multiple keys with independent directions (stable, last key first)."""
    result = list(entries)
    for name, desc in reversed(order_by):
        result.sort(
            key=lambda e, n=name: (
                _sort_rank(e["key"].get(n, e["values"].get(n))),
                e["key"].get(n, e["values"].get(n)) if e["key"].get(n, e["values"].get(n)) is not None else "",
            ),
            reverse=desc,
        )
    return result


def _having_pass(value: Any, op: str, expected: Any) -> bool:
    if op in ("eq", "=", "=="):
        return value == expected
    if op in ("ne", "neq", "!="):
        return value != expected
    if value is None:
        return False
    if op == "lt":
        return value < expected
    if op == "lte":
        return value <= expected
    if op == "gt":
        return value > expected
    if op == "gte":
        return value >= expected
    raise AdcError("INVALID_AGGREGATION", f"unknown having operator {op!r}")