"""MCP stdio bridge exposing Agent Data Circuit endpoints as MCP tools.

Newline-delimited JSON-RPC 2.0 over stdin/stdout, protocol version 2024-11-05.
Each tool is a thin HTTP client against an ADC server: it builds the query
string or JSON body of SPEC.md section 6 and returns the response JSON as the
tool result. Non-2xx responses are surfaced as RFC 9457 Problem Details with
isError set. Argument validation reuses reference/adc (errors, anchors,
pathing, filters, aggregate). Standard library only, Python 3.11+.
Run as: python reference/mcp-tools/adc_mcp.py
"""

from __future__ import annotations

import json
import re
import sys
import traceback
import urllib.error
import urllib.parse
import urllib.request
import uuid
from pathlib import Path

_REFERENCE_DIR = str(Path(__file__).resolve().parents[1])
if _REFERENCE_DIR not in sys.path:
    sys.path.insert(0, _REFERENCE_DIR)

from adc.aggregate import parse_group_by, parse_having, parse_metrics, parse_order_by
from adc.anchors import AnchorError, normalize_origin, parse_wire_anchor, strip_namespace
from adc.errors import ERROR_BASE_URI, ERROR_TABLE, AdcError
from adc.filters import LIST_OPS, OPS
from adc.pathing import canonicalize_path

PROTOCOL_VERSION = "2024-11-05"
SERVER_NAME = "adc-mcp"
SERVER_VERSION = "0.1.0"
HTTP_TIMEOUT = 20.0
ACCEPT_HEADER = "application/adc+json, application/json"

_FIELD_RE = re.compile(r"^[A-Za-z0-9_.]+$")
_AI_VAR_TOKEN = re.compile(r"^[A-Za-z0-9_-]{1,16}$")

_STATUS_FALLBACK = {
    400: "INVALID_BODY",
    401: "AUTH_REQUIRED",
    403: "FORBIDDEN",
    404: "ANCHOR_NOT_FOUND",
    409: "GENERATION_STALE",
    410: "ANCHOR_EXPIRED",
    413: "PAYLOAD_TOO_LARGE",
    415: "UNSUPPORTED_MEDIA_TYPE",
    422: "AGGREGATION_TOO_BROAD",
    429: "RATE_LIMIT",
    500: "INTERNAL_ERROR",
    501: "NOT_IMPLEMENTED",
    502: "BAD_GATEWAY",
    503: "HYDRATION_FAILED",
    504: "BACKEND_TIMEOUT",
}


class ToolHttpError(Exception):
    """A non-2xx HTTP response whose body should be read as Problem Details."""

    def __init__(self, status: int, body: bytes, url: str) -> None:
        self.status = status
        self.body = body
        self.url = url
        super().__init__(f"HTTP {status} from {url}")


# ---------------------------------------------------------------------------
# Logging and JSON-RPC frames
# ---------------------------------------------------------------------------


def _log(message: str) -> None:
    try:
        sys.stderr.write(message.rstrip() + "\n")
        sys.stderr.flush()
    except Exception:
        pass


def _emit(frame: dict) -> None:
    line = json.dumps(frame, ensure_ascii=True) + "\n"
    stream = getattr(sys.stdout, "buffer", None)
    if stream is not None:
        stream.write(line.encode("utf-8"))
        stream.flush()
    else:
        sys.stdout.write(line)
        sys.stdout.flush()


def _ok(request_id, result: dict) -> dict:
    return {"jsonrpc": "2.0", "id": request_id, "result": result}


def _err(request_id, code: int, message: str) -> dict:
    return {"jsonrpc": "2.0", "id": request_id, "error": {"code": code, "message": message}}


def _initialize_result() -> dict:
    return {
        "protocolVersion": PROTOCOL_VERSION,
        "capabilities": {"tools": {}},
        "serverInfo": {"name": SERVER_NAME, "version": SERVER_VERSION},
    }


# ---------------------------------------------------------------------------
# Problem Details helpers
# ---------------------------------------------------------------------------


def _problem_from_http(error: ToolHttpError) -> dict:
    text = error.body.decode("utf-8", "replace") if error.body else ""
    data = None
    if text.strip():
        try:
            data = json.loads(text)
        except ValueError:
            data = None
    source = data if isinstance(data, dict) else {}
    detail = source.get("detail")
    if not isinstance(detail, str) or not detail:
        detail = text.strip()[:500] or f"HTTP {error.status} from {error.url}"
    status = source.get("status")
    if not isinstance(status, int):
        status = error.status
    retryable = source.get("retryable")
    if not isinstance(retryable, bool):
        code = str(source.get("type", "")).rsplit("/", 1)[-1]
        if code in ERROR_TABLE:
            retryable = ERROR_TABLE[code][1]
        elif error.status in _STATUS_FALLBACK:
            retryable = ERROR_TABLE[_STATUS_FALLBACK[error.status]][1]
        else:
            retryable = error.status == 429 or error.status >= 500
    ordered = {
        "type": source.get("type") or "about:blank",
        "title": source.get("title") or f"HTTP {error.status}",
        "status": status,
        "detail": detail,
        "retryable": retryable,
    }
    for key, value in source.items():
        if key not in ordered:
            ordered[key] = value
    return ordered


def _tool_error(problem: dict, tool: str) -> dict:
    detail = problem.get("detail")
    if not isinstance(detail, str) or not detail:
        detail = str(problem.get("title") or "tool call failed")
        problem["detail"] = detail
    problem["detail"] = f"{tool}: {detail}"
    text = json.dumps(problem, ensure_ascii=True)
    return {"content": [{"type": "text", "text": text}], "isError": True}


def _result_text(payload) -> str:
    if isinstance(payload, str):
        return payload
    return json.dumps(payload, ensure_ascii=True)


# ---------------------------------------------------------------------------
# HTTP client
# ---------------------------------------------------------------------------


def _reason_text(reason) -> str:
    if isinstance(reason, OSError):
        return f"{type(reason).__name__}({reason.errno})" if reason.errno else type(reason).__name__
    return str(reason)


def _is_timeout(reason) -> bool:
    return isinstance(reason, TimeoutError) or "timed out" in str(reason).lower()


def _request(url: str, params=None, body=None):
    if params:
        url = url + ("&" if "?" in url else "?") + urllib.parse.urlencode(
            params, quote_via=urllib.parse.quote
        )
    data = None
    headers = {"Accept": ACCEPT_HEADER, "User-Agent": f"{SERVER_NAME}/{SERVER_VERSION}"}
    if body is not None:
        data = json.dumps(body, ensure_ascii=True).encode("utf-8")
        headers["Content-Type"] = "application/adc+json"
        headers["Idempotency-Key"] = str(uuid.uuid4())
    request = urllib.request.Request(
        url,
        data=data,
        headers=headers,
        method="POST" if body is not None else "GET",
    )
    try:
        with urllib.request.urlopen(request, timeout=HTTP_TIMEOUT) as response:
            return response.read()
    except urllib.error.HTTPError as exc:
        raise ToolHttpError(exc.code, exc.read(), url) from None
    except urllib.error.URLError as exc:
        reason = getattr(exc, "reason", exc)
        code = "BACKEND_TIMEOUT" if _is_timeout(reason) else "BAD_GATEWAY"
        raise AdcError(code, f"cannot reach {url}: {_reason_text(reason)}") from None
    except OSError as exc:
        code = "BACKEND_TIMEOUT" if _is_timeout(exc) else "BAD_GATEWAY"
        raise AdcError(code, f"cannot reach {url}: {_reason_text(exc)}") from None


def _read(url: str, params=None, body=None):
    raw = _request(url, params=params, body=body)
    text = raw.decode("utf-8", "replace") if raw else ""
    if not text.strip():
        return {}
    try:
        return json.loads(text)
    except ValueError:
        return text


_MANIFEST_ENDPOINTS: dict[str, dict[str, str]] = {}


def _load_manifest_endpoints(root: str) -> dict[str, str]:
    url = urllib.parse.urljoin(root + "/", ".well-known/adc.json")
    try:
        raw = _request(url)
    except (AdcError, ToolHttpError) as exc:
        _log(f"{SERVER_NAME}: no manifest at {url} ({exc}); using default endpoints")
        return {}
    try:
        data = json.loads(raw.decode("utf-8", "replace"))
    except ValueError:
        return {}
    if not isinstance(data, dict):
        return {}
    endpoints = data.get("endpoints")
    if not isinstance(endpoints, dict):
        return {}
    return {
        str(key): str(value)
        for key, value in endpoints.items()
        if isinstance(key, str) and isinstance(value, str) and value
    }


def _endpoint(root: str, name: str) -> str:
    table = _MANIFEST_ENDPOINTS.get(root)
    if table is None:
        table = _load_manifest_endpoints(root)
        _MANIFEST_ENDPOINTS[root] = table
    declared = table.get(name) or f"/adc/{name}"
    if declared.startswith(("http://", "https://")):
        return declared
    return urllib.parse.urljoin(root + "/", declared.lstrip("/"))


# ---------------------------------------------------------------------------
# Local argument parsing (all failures become Problem Details, never crashes)
# ---------------------------------------------------------------------------


def _require_base(args: dict, name: str = "root") -> str:
    root = args.get(name)
    if not isinstance(root, str) or not root.strip():
        raise AdcError("INVALID_BODY", f"missing required argument '{name}'")
    root = root.strip()
    parts = urllib.parse.urlsplit(root)
    if parts.scheme not in ("http", "https") or not parts.netloc:
        raise AdcError(
            "INVALID_BODY",
            f"'{name}' must be an absolute http(s) URL, got {root!r}",
        )
    if parts.query or parts.fragment:
        raise AdcError("INVALID_BODY", f"'{name}' must not carry a query or fragment")
    return root.rstrip("/")


def _require_root(args: dict) -> str:
    return _require_base(args, "root")


def _string(args: dict, name: str):
    value = args.get(name)
    if value is None:
        return None
    if not isinstance(value, str):
        raise AdcError("INVALID_BODY", f"'{name}' must be a string")
    return value


def _integer(args: dict, name: str):
    value = args.get(name)
    if value is None:
        return None
    if isinstance(value, bool) or not isinstance(value, int):
        raise AdcError("INVALID_BODY", f"'{name}' must be an integer")
    if value < 1:
        raise AdcError("INVALID_BODY", f"'{name}' must be >= 1")
    return value


def _path(args: dict):
    value = args.get("path")
    if value is None:
        return None
    if not isinstance(value, str):
        raise AdcError("INVALID_BODY", "'path' must be a string")
    try:
        return canonicalize_path(value)
    except AdcError as exc:
        raise AdcError(exc.code, exc.detail, details=dict(exc.details)) from None


def _anchor(args: dict, root: str):
    value = args.get("anchor")
    if value is None:
        return None
    if not isinstance(value, str) or not value.strip():
        raise AdcError("INVALID_BODY", "'anchor' must be a non-empty string")
    token = value.strip()
    if "#" in token:
        namespace = token.split("#", 1)[0]
        try:
            root_origin = normalize_origin(
                f"{urllib.parse.urlsplit(root).scheme}://{urllib.parse.urlsplit(root).netloc}"
            )
            anchor_origin = normalize_origin(namespace)
        except AnchorError as exc:
            raise AdcError("ANCHOR_INVALID", exc.detail) from None
        if anchor_origin != root_origin:
            raise AdcError(
                "ANCHOR_INVALID",
                f"anchor namespace {namespace!r} does not belong to root origin {root_origin!r}",
            )
        token = strip_namespace(token)
    parse_wire_anchor(token)
    return token


def _selector(args: dict, root: str) -> dict:
    anchor = _anchor(args, root)
    path = _path(args)
    if anchor is not None and path is not None:
        raise AdcError("ANCHOR_INVALID", "supply exactly one of anchor or path")
    if anchor is not None:
        return {"anchor": anchor}
    if path is not None:
        return {"path": path}
    return {}


def _scalar_text(value):
    if value is None:
        return "null"
    if isinstance(value, bool):
        return "true" if value else "false"
    if isinstance(value, str):
        return value
    if isinstance(value, (int, float)):
        return str(value)
    return None


def _append_value(pairs: list, key: str, op: str, value, field: str) -> bool:
    if op == "exists":
        if isinstance(value, bool):
            flag = value
        elif isinstance(value, str) and value.lower() in ("true", "false"):
            flag = value.lower() == "true"
        else:
            return False
        pairs.append((key, "true" if flag else "false"))
        return True
    if op in LIST_OPS:
        if not isinstance(value, list):
            return False
        for item in value:
            text = _scalar_text(item)
            if text is None:
                return False
            pairs.append((key, text))
        return True
    text = _scalar_text(value)
    if text is None:
        return False
    pairs.append((key, text))
    return True


def _filter_pairs(flt):
    """GET query pairs for a filter, or None when the POST profile is required."""
    if not isinstance(flt, dict):
        raise AdcError("FILTER_INVALID", "'filter' must be an object")
    pairs: list[tuple[str, str]] = []
    for field, condition in flt.items():
        if not isinstance(field, str) or not _FIELD_RE.match(field):
            raise AdcError("FILTER_INVALID", f"invalid filter field name: {field!r}")
        if field in ("and", "or", "not"):
            return None
        if isinstance(condition, dict):
            if not condition:
                return None
            for op, value in condition.items():
                if op not in OPS:
                    raise AdcError(
                        "FILTER_INVALID",
                        f"unknown filter operator {op!r} on field {field!r}",
                        details={"unsupported_ops": [op], "allowed": list(OPS)},
                    )
                if not _append_value(pairs, f"filter[{field}][{op}]", op, value, field):
                    return None
        elif not _append_value(pairs, f"filter[{field}]", "eq", condition, field):
            return None
    return pairs


def _fields_arg(value):
    if value is None:
        return None
    if isinstance(value, str):
        return value or None
    if isinstance(value, list):
        return ",".join(str(item) for item in value) or None
    raise AdcError("INVALID_BODY", "'fields' must be a string or array")


def _order_by_arg(args: dict):
    value = args.get("order_by")
    if value is None:
        return None
    parsed = parse_order_by(value)
    return [("-" if descending else "") + name for name, descending in parsed] or None


def _group_by_arg(args: dict):
    value = args.get("group_by")
    if value is None:
        return None
    return parse_group_by(value) or None


def _having_arg(args: dict):
    value = args.get("having")
    if value is None:
        return None
    return parse_having(value) or None


def _having_text(item) -> str:
    alias, op, value = item
    text = _scalar_text(value)
    if text is None:
        text = json.dumps(value, ensure_ascii=True)
    return f"{alias}:{op}:{text}"


def _metrics_arg(args: dict):
    raw = args.get("metrics")
    if raw is None or raw == [] or (isinstance(raw, str) and not raw.strip()):
        raise AdcError("INVALID_AGGREGATION", "missing required argument 'metrics'")
    if isinstance(raw, str):
        text = raw
    elif isinstance(raw, list):
        parts = []
        for item in raw:
            if isinstance(item, str):
                parts.append(item)
            elif isinstance(item, dict):
                expr = item.get("expr")
                if not isinstance(expr, str):
                    raise AdcError("INVALID_AGGREGATION", "metric 'expr' must be a string")
                alias = item.get("alias")
                if alias is None:
                    parts.append(expr)
                elif isinstance(alias, str):
                    parts.append(f"{expr}:{alias}")
                else:
                    raise AdcError("INVALID_AGGREGATION", "metric 'alias' must be a string")
            else:
                raise AdcError("INVALID_AGGREGATION", "metrics entries must be strings or objects")
        text = ",".join(parts)
        if not text:
            raise AdcError("INVALID_AGGREGATION", "missing required argument 'metrics'")
    else:
        raise AdcError("INVALID_AGGREGATION", "'metrics' must be a string or array")
    return text, parse_metrics(text)


# ---------------------------------------------------------------------------
# Tool handlers
# ---------------------------------------------------------------------------


def _orient(args: dict):
    root = _require_root(args)
    return _read(_endpoint(root, "orient"))


def _tree(args: dict):
    root = _require_root(args)
    params: list[tuple[str, str]] = []
    path = _path(args)
    if path is not None:
        params.append(("path", path))
    for name in ("depth", "limit"):
        value = _integer(args, name)
        if value is not None:
            params.append((name, str(value)))
    cursor = _string(args, "cursor")
    if cursor is not None:
        params.append(("cursor", cursor))
    return _read(_endpoint(root, "tree"), params)


def _passport(args: dict):
    root = _require_root(args)
    selector = _selector(args, root)
    return _read(_endpoint(root, "passport"), list(selector.items()))


def _fetch(args: dict):
    root = _require_root(args)
    selector = _selector(args, root)
    endpoint = _endpoint(root, "fetch")
    slice_name = _string(args, "slice")
    limit = _integer(args, "limit")
    cursor = _string(args, "cursor")
    fields = _fields_arg(args.get("fields"))
    order_by = _order_by_arg(args)
    flt = args.get("filter")
    pairs = _filter_pairs(flt) if flt is not None else None
    if flt is not None and pairs is None:
        body = dict(selector)
        if slice_name is not None:
            body["slice"] = slice_name
        if limit is not None:
            body["limit"] = limit
        if cursor is not None:
            body["cursor"] = cursor
        if fields is not None:
            body["fields"] = fields.split(",")
        if order_by is not None:
            body["order_by"] = order_by
        body["filter"] = flt
        return _read(endpoint, body=body)

    params = list(selector.items())
    if slice_name is not None:
        params.append(("slice", slice_name))
    if limit is not None:
        params.append(("limit", str(limit)))
    if cursor is not None:
        params.append(("cursor", cursor))
    if fields is not None:
        params.append(("fields", fields))
    if order_by is not None:
        params.append(("order_by", ",".join(order_by)))
    return _read(endpoint, params + (pairs or []))


def _aggregate(args: dict):
    root = _require_root(args)
    selector = _selector(args, root)
    endpoint = _endpoint(root, "aggregate")
    metrics_text, metrics = _metrics_arg(args)
    group_by = _group_by_arg(args)
    having = _having_arg(args)
    order_by = _order_by_arg(args)
    limit = _integer(args, "limit")
    min_group_size = _integer(args, "min_group_size")
    flt = args.get("filter")
    pairs = _filter_pairs(flt) if flt is not None else None

    if flt is not None and pairs is None:
        body = dict(selector)
        body["metrics"] = [{"expr": m.expr, "alias": m.alias} for m in metrics]
        body["filter"] = flt
        if group_by is not None:
            body["group_by"] = group_by
        if having:
            body["having"] = [
                {"alias": alias, "op": op, "value": value} for alias, op, value in having
            ]
        if order_by is not None:
            body["order_by"] = order_by
        if limit is not None:
            body["limit"] = limit
        if min_group_size is not None:
            body["min_group_size"] = min_group_size
        return _read(endpoint, body=body)

    params = list(selector.items())
    params.append(("metrics", metrics_text))
    if group_by is not None:
        params.append(("group_by", ",".join(group_by)))
    for item in having or []:
        params.append(("having", _having_text(item)))
    if order_by is not None:
        params.append(("order_by", ",".join(order_by)))
    if limit is not None:
        params.append(("limit", str(limit)))
    if min_group_size is not None:
        params.append(("min_group_size", str(min_group_size)))
    return _read(endpoint, params + (pairs or []))


def _cold_entry(args: dict):
    """AI-ENTRY.md: probe /AI/, or fetch one variable through it."""
    origin = _require_base(args, "origin")
    var = _string(args, "var")
    get = _string(args, "get")
    if var is None:
        return _read(origin + "/AI/")
    if not _AI_VAR_TOKEN.match(var):
        raise AdcError(
            "INVALID_BODY",
            f"'var' must be a token from the map, got {var!r}",
        )
    params = [("get", get)] if get is not None else None
    return _read(origin + "/AI[" + var + "]", params)


# ---------------------------------------------------------------------------
# Tool registry
# ---------------------------------------------------------------------------

_ROOT_DESCRIPTION = "Base URL of the ADC server, e.g. http://127.0.0.1:8080"
_SELECTOR_NOTE = "Exactly one of anchor or path may be sent; sending both is an error."
_FILTER_NOTE = (
    "Filter object. Flat field conditions use the GET grammar "
    "(filter[field][op]=value); and/or/not trees use the POST profile."
)


def _string_prop(description: str) -> dict:
    return {"type": "string", "description": description}


def _int_prop(description: str) -> dict:
    return {"type": "integer", "description": description, "minimum": 1}


def _multi_prop(description: str) -> dict:
    return {"type": ["string", "array"], "description": description}


def _schema(properties: dict, required: list) -> dict:
    return {"type": "object", "properties": properties, "required": required}


_ANNOTATIONS = {"readOnlyHint": True, "idempotentHint": True, "openWorldHint": True}

TOOLS: dict[str, dict] = {
    "adc_orient": {
        "description": (
            "Discover an ADC circuit: returns the ADC version, declared root, "
            "tree generation, tokenizer, conformance tiers and endpoint links "
            "(SPEC.md 6.1)."
        ),
        "inputSchema": _schema({"root": _string_prop(_ROOT_DESCRIPTION)}, ["root"]),
        "handler": _orient,
        "annotations": _ANNOTATIONS,
    },
    "adc_tree": {
        "description": (
            "List child nodes of an ADC navigation tree position with depth, "
            "limit and cursor paging (SPEC.md 6.2). Defaults to path /."
        ),
        "inputSchema": _schema(
            {
                "root": _string_prop(_ROOT_DESCRIPTION),
                "path": _string_prop("Absolute path relative to root, default /."),
                "depth": _int_prop("Levels to descend, 1..limits.max_depth."),
                "limit": _int_prop("Maximum number of nodes to return."),
                "cursor": _string_prop("Opaque cursor from a previous tree response."),
            },
            ["root"],
        ),
        "handler": _tree,
        "annotations": _ANNOTATIONS,
    },
    "adc_passport": {
        "description": (
            "Return node metadata for one anchor or path: token and byte costs, "
            "filters, slices, generations and aggregation support "
            f"(SPEC.md 6.3). {_SELECTOR_NOTE}"
        ),
        "inputSchema": _schema(
            {
                "root": _string_prop(_ROOT_DESCRIPTION),
                "anchor": _string_prop("Wire anchor such as [a10]."),
                "path": _string_prop("Absolute path of the node."),
            },
            ["root"],
        ),
        "handler": _passport,
        "annotations": _ANNOTATIONS,
    },
    "adc_fetch": {
        "description": (
            "Fetch one entity or document block, or one page of collection "
            "items, from an ADC server with slice, filter, field projection "
            f"and ordering (SPEC.md 6.4). {_SELECTOR_NOTE} {_FILTER_NOTE}"
        ),
        "inputSchema": _schema(
            {
                "root": _string_prop(_ROOT_DESCRIPTION),
                "anchor": _string_prop("Wire anchor such as [a10]."),
                "path": _string_prop("Absolute path of the node."),
                "slice": _string_prop("Slice name, default is the passport default slice."),
                "limit": _int_prop("Maximum number of items to return."),
                "cursor": _string_prop("Opaque cursor from a previous fetch response."),
                "filter": {
                    "type": "object",
                    "description": _FILTER_NOTE,
                },
                "fields": _multi_prop(
                    "Field projection: comma-separated dot paths, or an array of them."
                ),
                "order_by": _multi_prop(
                    "Ordering: comma-separated fields, or an array; prefix with - to sort descending."
                ),
            },
            ["root"],
        ),
        "handler": _fetch,
        "annotations": _ANNOTATIONS,
    },
    "adc_aggregate": {
        "description": (
            "Run a server-side aggregation over a collection without "
            "serializing entities; metrics is required and k-anonymity is "
            f"enforced by the publisher (SPEC.md 6.9). {_SELECTOR_NOTE} {_FILTER_NOTE}"
        ),
        "inputSchema": _schema(
            {
                "root": _string_prop(_ROOT_DESCRIPTION),
                "anchor": _string_prop("Wire anchor such as [a10]."),
                "path": _string_prop("Absolute path of the node."),
                "metrics": _multi_prop(
                    "Required metric expressions, e.g. count(), avg(price):avg_price."
                ),
                "group_by": _multi_prop("Grouping fields, comma-separated or an array."),
                "filter": {
                    "type": "object",
                    "description": _FILTER_NOTE,
                },
                "limit": _int_prop("Maximum number of groups to return, default 10."),
                "min_group_size": _int_prop(
                    "Privacy floor: groups smaller than this are suppressed."
                ),
                "having": _multi_prop(
                    "Group filter as alias:op:value, one string or an array of them."
                ),
                "order_by": _multi_prop(
                    "Ordering over groups, comma-separated or an array; prefix with - to sort descending."
                ),
            },
            ["root", "metrics"],
        ),
        "handler": _aggregate,
        "annotations": _ANNOTATIONS,
    },
    "adc_cold_entry": {
        "description": (
            "Probe a site's /AI/ cold entry (AI-ENTRY.md). Without 'var' it "
            "returns the tunnel map (vars and keys); with 'var' it returns "
            "one typed fragment, selecting what to produce with 'get'. The "
            "cold entry is optional and independent of ADC conformance."
        ),
        "inputSchema": _schema(
            {
                "origin": _string_prop(
                    "Origin of the site offering /AI/, e.g. http://127.0.0.1:8080"
                ),
                "var": _string_prop(
                    "Variable name from the tunnel map, e.g. h1. Omit to fetch the map."
                ),
                "get": _string_prop(
                    "Key to produce: text, meta, image, transcript, chapters, "
                    "comments, price or stock. Defaults to text."
                ),
            },
            ["origin"],
        ),
        "handler": _cold_entry,
        "annotations": _ANNOTATIONS,
    },
}


def _tool_defs() -> list:
    return [
        {
            "name": name,
            "description": tool["description"],
            "inputSchema": tool["inputSchema"],
            "annotations": tool["annotations"],
        }
        for name, tool in TOOLS.items()
    ]


# ---------------------------------------------------------------------------
# JSON-RPC dispatch
# ---------------------------------------------------------------------------


def _tools_call(request_id, params) -> dict:
    if not isinstance(params, dict):
        return _err(request_id, -32602, "Invalid params: 'params' must be an object")
    name = params.get("name")
    if not isinstance(name, str) or name not in TOOLS:
        return _err(request_id, -32602, f"Unknown tool: {name}")
    arguments = params.get("arguments")
    if arguments is None:
        arguments = {}
    tool = TOOLS[name]
    if not isinstance(arguments, dict):
        problem = AdcError("INVALID_BODY", "'arguments' must be an object").to_problem()
        return _ok(request_id, _tool_error(problem, name))
    try:
        payload = tool["handler"](arguments)
    except AdcError as exc:
        return _ok(request_id, _tool_error(exc.to_problem(), name))
    except ToolHttpError as exc:
        return _ok(request_id, _tool_error(_problem_from_http(exc), name))
    except Exception as exc:
        _log(traceback.format_exc())
        problem = AdcError("INTERNAL_ERROR", f"{type(exc).__name__}: {exc}").to_problem()
        return _ok(request_id, _tool_error(problem, name))
    return _ok(request_id, {"content": [{"type": "text", "text": _result_text(payload)}]})


def _dispatch(message):
    if not isinstance(message, dict):
        return _err(None, -32600, "Invalid Request: expected a JSON object")
    method = message.get("method")
    has_id = "id" in message
    request_id = message.get("id")
    if not isinstance(method, str):
        if not has_id:
            return None
        return _err(request_id, -32600, "Invalid Request: missing or invalid 'method'")
    if not has_id or method.startswith("notifications/"):
        return None
    if method == "initialize":
        return _ok(request_id, _initialize_result())
    if method == "ping":
        return _ok(request_id, {})
    if method == "tools/list":
        return _ok(request_id, {"tools": _tool_defs()})
    if method == "tools/call":
        return _tools_call(request_id, message.get("params"))
    return _err(request_id, -32601, f"Method not found: {method}")


def main() -> None:
    stream = getattr(sys.stdin, "buffer", sys.stdin)
    for line in stream:
        if isinstance(line, bytes):
            line = line.decode("utf-8", "replace")
        line = line.strip()
        if not line:
            continue
        try:
            message = json.loads(line)
        except ValueError:
            _emit(_err(None, -32700, "Parse error: line is not valid JSON"))
            continue
        try:
            response = _dispatch(message)
        except Exception:
            _log(traceback.format_exc())
            request_id = message.get("id") if isinstance(message, dict) else None
            response = None if request_id is None else _err(
                request_id, -32603, "Internal error"
            )
        if response is not None:
            _emit(response)


if __name__ == "__main__":
    main()
