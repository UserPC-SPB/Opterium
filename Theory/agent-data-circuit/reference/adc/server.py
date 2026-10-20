"""Reference ADC HTTP server (stdlib only).

Serves a directory-format site (see :mod:`adc.site`) over the endpoints in
SPEC.md section 6. Every handler walks the normative validation order of
section 6.10 and returns RFC 9457 Problem Details on failure.

Degraded dynamic slices are simulated for testing: set ``ADC_FAIL_SLICES``
to a comma-separated list of slice names to force hydration failure, and
``ADC_STALE_SECONDS`` to control the reported age of a stale fallback.
"""

from __future__ import annotations

import base64
import hashlib
import hmac
import json
import os
import re
import threading
import time
from datetime import datetime, timezone
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from typing import Any, Callable, Iterable, Mapping
from urllib.parse import parse_qs, unquote, urlsplit

from .aggregate import (
    AggConfig,
    compute_aggregate,
    parse_group_by,
    parse_having,
    parse_metrics,
    parse_order_by,
)
from .errors import AdcError
from .filters import conditions_from_query, filter_specs, parse_post_filter, conditions_match, post_filter_match
from .pathing import canonicalize_path
from .site import Site, canonical_bytes, payload_stats
from .tokens import bytes_for, count_tokens

__all__ = ["AdcHandler", "create_server", "serve", "run"]

ADC_MEDIA = "application/adc+json; charset=utf-8"
PROBLEM_MEDIA = "application/problem+json; charset=utf-8"
AI_MEDIA = "application/json; charset=utf-8"
MAX_URI = 8192

# /AI/<var> - the cold entry variable syntax (AI-ENTRY.md section 4)
_AI_VAR_RE = re.compile(r"^/AI\[([A-Za-z0-9_-]{1,16})\]$")
_AI_LIMIT_RE = re.compile(r"[1-9][0-9]{0,5}")
_AI_FIELD_RE = re.compile(r"[A-Za-z0-9_]{1,64}")


class AiParamError(Exception):
    """A cold entry parameter the tunnel cannot honour (AI-ENTRY.md 11)."""

    def __init__(self, parameter: str, detail: str) -> None:
        self.parameter = parameter
        self.detail = detail
        super().__init__(detail)


_ORDER_RE = re.compile(r"^[-+]?[A-Za-z0-9_.]+$")


def _now() -> str:
    return datetime.now(timezone.utc).replace(microsecond=0).isoformat().replace("+00:00", "Z")


# --------------------------------------------------------------------------
# Cursor (opaque, bound to the query and the tree generation)
# --------------------------------------------------------------------------


class CursorTool:
    """Issues and checks opaque cursors bound to a query fingerprint.

    The payload holds only an offset and a digest. Totals are never encoded,
    so a cursor cannot leak how many rows match (SPEC 6.4.4).
    """

    def __init__(self, secret: bytes | None = None) -> None:
        self.secret = secret or os.urandom(32)

    def _digest(self, binding: str) -> str:
        return hmac.new(self.secret, binding.encode("utf-8"), hashlib.sha256).hexdigest()[:16]

    def encode(self, binding: str, offset: int) -> str:
        payload = {"o": offset, "b": self._digest(binding)}
        raw = json.dumps(payload, separators=(",", ":")).encode("utf-8")
        return base64.urlsafe_b64encode(raw).decode("ascii").rstrip("=")

    def decode(self, token: str, binding: str) -> int:
        try:
            padding = "=" * (-len(token) % 4)
            payload = json.loads(base64.urlsafe_b64decode(token + padding))
            offset = int(payload["o"])
            digest = str(payload["b"])
        except Exception:
            raise AdcError("CURSOR_INVALID", "cursor is malformed") from None
        if not hmac.compare_digest(digest, self._digest(binding)):
            raise AdcError(
                "CURSOR_INVALID",
                "cursor does not match this query",
                details={"hint": "cursor must be reused with an identical query"},
            )
        if offset < 0:
            raise AdcError("CURSOR_INVALID", "cursor offset is negative")
        return offset


# --------------------------------------------------------------------------
# Rate limiting
# --------------------------------------------------------------------------


class RateLimiter:
    """Fixed-window counter with per-method weights (SPEC 6.11)."""

    def __init__(self, requests: int, window_seconds: int, weights: Mapping[str, int]) -> None:
        self.requests = max(1, requests)
        self.window = max(1, window_seconds)
        self.weights = dict(weights)
        self._lock = threading.Lock()
        self._count = 0
        self._window_start = time.time()

    def status(self) -> tuple[int, int, int]:
        """Window state without consuming quota (SPEC 6.11).

        Used by error responses: reporting ``Remaining: 0`` on a plain 400
        would make an agent throttle itself after a malformed request.
        """
        with self._lock:
            if time.time() - self._window_start >= self.window:
                self._count = 0
                self._window_start = time.time()
            remaining = max(0, self.requests - self._count)
            reset = int(self._remaining_window()) + 1
            return self.requests, remaining, reset

    def _remaining_window(self) -> float:
        elapsed = time.time() - self._window_start
        return max(0.0, self.window - elapsed)

    def check(self, method_key: str, weight: int | None = None) -> tuple[int, int, int]:
        weight = weight if weight is not None else self.weights.get(method_key, 1)
        with self._lock:
            if time.time() - self._window_start >= self.window:
                self._count = 0
                self._window_start = time.time()
            if self._count + weight > self.requests:
                retry_after = int(self._remaining_window()) + 1
                raise AdcError(
                    "RATE_LIMIT",
                    "rate limit exceeded for this window",
                    retry_after=retry_after,
                    details={"scope_weight": weight, "limit": self.requests},
                )
            self._count += weight
            remaining = max(0, self.requests - self._count)
            reset = int(self._remaining_window()) + 1
            return self.requests, remaining, reset


# --------------------------------------------------------------------------
# Server
# --------------------------------------------------------------------------


class AdcHandler(BaseHTTPRequestHandler):
    server_version = "adc-reference/0.1"
    protocol_version = "HTTP/1.1"

    # injected by create_server
    site: Site
    cursor: CursorTool
    limiter: RateLimiter
    fail_slices: set[str]
    stale_seconds: int
    max_post_bytes: int

    # -- plumbing --------------------------------------------------------

    def log_message(self, fmt: str, *args: Any) -> None:
        if os.environ.get("ADC_QUIET") != "1":
            super().log_message(fmt, *args)

    @property
    def generation_header(self) -> str:
        return self.site.tree_generation

    def _send(
        self,
        status: int,
        body: bytes,
        content_type: str,
        *,
        extra: Mapping[str, str] | None = None,
    ) -> None:
        self.send_response(status)
        self.send_header("Content-Type", content_type)
        self.send_header("Content-Length", str(len(body)))
        self.send_header("ADC-Tree-Generation", self.generation_header)
        self.send_header("Vary", "Accept, Accept-Language, Authorization")
        for key, value in (extra or {}).items():
            self.send_header(key, value)
        self.end_headers()
        if self.command != "HEAD":
            self.wfile.write(body)

    def _json(
        self,
        status: int,
        payload: Any,
        *,
        extra: Mapping[str, str] | None = None,
        media: str = ADC_MEDIA,
    ) -> None:
        body = json.dumps(payload, ensure_ascii=False, separators=(",", ":")).encode("utf-8")
        self._send(status, body, media, extra=extra)

    def _problem(self, error: AdcError) -> None:
        problem = error.to_problem()
        problem["instance"] = problem.get("instance") or self.path
        problem["tree_generation"] = self.site.tree_generation
        headers: dict[str, str] = {"Cache-Control": "no-store"}
        if error.retry_after is not None:
            headers["Retry-After"] = str(error.retry_after)
        limit, remaining, reset = self.limiter.status()
        headers["RateLimit-Limit"] = str(limit)
        headers["RateLimit-Remaining"] = str(remaining)
        headers["RateLimit-Reset"] = str(reset)
        self._json(error.status or 500, problem, extra=headers, media=PROBLEM_MEDIA)

    def _rate_headers(self, triple: tuple[int, int, int]) -> dict[str, str]:
        limit, remaining, reset = triple
        return {
            "RateLimit-Limit": str(limit),
            "RateLimit-Remaining": str(remaining),
            "RateLimit-Reset": str(reset),
        }

    # -- request entry points --------------------------------------------

    def do_GET(self) -> None:  # noqa: N802
        self._dispatch("GET")

    def do_HEAD(self) -> None:  # noqa: N802
        self._dispatch("GET")

    def do_POST(self) -> None:  # noqa: N802
        self._dispatch("POST")

    def do_PUT(self) -> None:  # noqa: N802
        self._reject_method()

    def do_DELETE(self) -> None:  # noqa: N802
        self._reject_method()

    def _reject_method(self) -> None:
        self._problem(AdcError("NOT_IMPLEMENTED", f"{self.command} is not part of ADC"))

    def _dispatch(self, method: str) -> None:
        try:
            parts = urlsplit(self.path)
            if len(self.path) > MAX_URI:
                raise AdcError("PATH_INVALID", "request URI is too long")
            path = parts.path
            query = parse_qs(parts.query, keep_blank_values=True)
            flat = {k: v for k, v in query.items()}

            # /AI/ is the cold entry: a separate protocol layered on top of
            # the circuit, never part of ADC conformance (AI-ENTRY.md).
            cold = unquote(path)
            if cold in ("/AI", "/AI/"):
                return self._serve_ai_map(method)
            if cold.startswith("/AI["):
                return self._serve_ai_var(method, cold, flat)

            if path in ("/.well-known/adc.json", "/adc/manifest"):
                return self._serve_manifest()
            if path == "/adc/orient":
                return self._serve_orient(method, flat)
            if path == "/adc/tree":
                return self._serve_tree(method, flat)
            if path == "/adc/passport":
                return self._serve_passport(method, flat)
            if path == "/adc/fetch":
                return self._serve_fetch(method, flat)
            if path == "/adc/estimate":
                return self._serve_estimate(method, flat)
            if path == "/adc/aggregate":
                return self._serve_aggregate(method, flat)
            if path in ("/adc/search", "/adc/grep", "/adc/verify"):
                raise AdcError(
                    "NOT_IMPLEMENTED",
                    f"{path} belongs to a conformance tier this circuit does not declare",
                    details={"conformance": self.site.manifest.get("conformance", [])},
                )
            raise AdcError("ANCHOR_NOT_FOUND", f"no such endpoint: {path}")
        except AdcError as exc:
            self._problem(exc)
        except (BrokenPipeError, ConnectionResetError):
            return
        except Exception as exc:  # pragma: no cover - defensive
            self._problem(AdcError("INTERNAL_ERROR", f"{type(exc).__name__}: {exc}"))

    def _read_body(self) -> bytes:
        length = int(self.headers.get("Content-Length") or 0)
        if length > self.max_post_bytes:
            raise AdcError(
                "PAYLOAD_TOO_LARGE",
                f"body of {length} bytes exceeds limits.max_post_bytes "
                f"{self.max_post_bytes}",
            )
        return self.rfile.read(length) if length else b""

    def _require_media(self) -> None:
        media = (self.headers.get("Content-Type") or "").split(";")[0].strip().lower()
        if media not in ("application/adc+json", "application/json"):
            raise AdcError(
                "UNSUPPORTED_MEDIA_TYPE",
                "POST bodies must be application/adc+json or application/json",
                details={"received": media or None},
            )

    def _single(self, params: Mapping[str, list[str]], name: str) -> str | None:
        values = params.get(name)
        if not values:
            return None
        return values[0]

    # -- endpoints -------------------------------------------------------

    def _serve_manifest(self) -> None:
        etag = 'W"' + hashlib.sha256(self.site.tree_generation.encode()).hexdigest()[:16] + '"'
        self._json(
            200,
            self.site.manifest,
            extra={
                "Cache-Control": "public, max-age=300",
                "ETag": etag,
                "ADC-Cost-Amount": "0",
            },
        )

    def _serve_orient(self, method: str, params: Mapping[str, list[str]]) -> None:
        rate = self.limiter.check("orient")
        payload = {
            "adc": self.site.manifest.get("adc"),
            "root": self.site.root,
            "tree_generation": self.site.tree_generation,
            "tokenizer": self.site.tokenizer,
            "conformance": self.site.manifest.get("conformance", []),
            "endpoints": self.site.endpoints,
            "namespace": self.site.namespace,
        }
        etag = 'W"' + hashlib.sha256(self.site.tree_generation.encode()).hexdigest()[:16] + '"'
        self._json(
            200,
            payload,
            extra={
                **self._rate_headers(rate),
                "Cache-Control": "public, max-age=3600",
                "ETag": etag,
            },
        )

    def _serve_tree(self, method: str, params: Mapping[str, list[str]]) -> None:
        rate = self.limiter.check("tree")
        limits = self.site.limits
        defaults = self.site.defaults
        max_depth = int(limits.get("max_depth", 4))
        max_limit = int(limits.get("max_limit", 200))
        page_size = int(defaults.get("page_size", 20))

        depth = _int_param(params, "depth", 1, minimum=1, maximum=max_depth)
        limit = _int_param(params, "limit", page_size, minimum=1, maximum=max_limit)

        anchor = self._single(params, "anchor")
        path = self._single(params, "path")
        if anchor and path:
            raise AdcError("ANCHOR_INVALID", "supply exactly one of anchor or path")
        origin = anchor or path or "/"

        supplied_generation = self._single(params, "tree_generation")
        if supplied_generation and supplied_generation != self.site.tree_generation:
            raise AdcError(
                "GENERATION_STALE",
                "requested tree_generation is not current",
                details={"current": self.site.tree_generation},
            )

        node = self.site.resolve(anchor=anchor, path=None if anchor else origin)
        rows = self.site.subordinates(node.path, depth)
        ordered = sorted(rows, key=lambda n: n.path)

        cursor_tool = self.cursor
        binding = f"tree|{node.path}|{depth}|{limit}|{self.site.tree_generation}"
        offset_param = self._single(params, "cursor")
        offset = cursor_tool.decode(offset_param, binding) if offset_param else 0

        window = ordered[offset : offset + limit]
        next_offset = offset + len(window)
        has_more = next_offset < len(ordered)
        cursor_out = cursor_tool.encode(binding, next_offset) if has_more else None

        payload = {
            "tree_generation": self.site.tree_generation,
            "path": node.path,
            "depth": depth,
            "nodes": [
                _tree_entry(child) for child in window
            ],
            "cursor": cursor_out,
            "truncated": has_more,
        }
        self._json(200, payload, extra=self._rate_headers(rate))

    def _serve_passport(self, method: str, params: Mapping[str, list[str]]) -> None:
        rate = self.limiter.check("passport")
        anchor = self._single(params, "anchor")
        path = self._single(params, "path")
        node = self.site.resolve(anchor=anchor, path=path)

        supplied_generation = self._single(params, "tree_generation")
        if supplied_generation and supplied_generation != self.site.tree_generation:
            raise AdcError(
                "GENERATION_STALE",
                "requested tree_generation is not current",
                details={"current": self.site.tree_generation},
            )

        doc = dict(node.passport)
        doc["tree_generation"] = self.site.tree_generation
        doc["path"] = node.path
        doc["anchor"] = node.anchor
        doc["kind"] = node.kind
        etag = '"' + hashlib.sha256(canonical_bytes(doc)).hexdigest()[:16] + '"'
        self._json(200, doc, extra={**self._rate_headers(rate), "ETag": etag})

    # -- fetch -----------------------------------------------------------

    def _serve_fetch(self, method: str, params: Mapping[str, list[str]]) -> None:
        if method == "POST":
            return self._serve_fetch_post()
        rate = self.limiter.check("fetch")
        spec = self._plan_get(params, kind="fetch")
        status, payload, headers = self._execute(spec)
        headers.update(self._rate_headers(rate))
        self._json(status, payload, extra=headers)

    def _serve_fetch_post(self) -> None:
        rate = self.limiter.check("post_fetch")
        self._require_media()
        body = self._read_body()
        try:
            data = json.loads(body.decode("utf-8")) if body else {}
        except (ValueError, UnicodeDecodeError) as exc:
            raise AdcError("INVALID_BODY", f"body is not valid JSON: {exc}") from None
        if not isinstance(data, dict):
            raise AdcError("INVALID_BODY", "body must be a JSON object")

        # 4.10 step 1/4: shape then schema
        spec = self._plan_post(data, kind="fetch")
        status, payload, headers = self._execute(spec)
        headers.update(self._rate_headers(rate))
        headers["Cache-Control"] = "no-store, private"
        headers.pop("ETag", None)
        self._json(status, payload, extra=headers)

    # -- estimate --------------------------------------------------------

    def _serve_estimate(self, method: str, params: Mapping[str, list[str]]) -> None:
        rate = self.limiter.check("estimate")
        spec = self._plan_get(params, kind="estimate")
        payload = self._build_estimate(spec)
        self._json(200, payload, extra=self._rate_headers(rate))

    # -- aggregate -------------------------------------------------------

    def _serve_aggregate(self, method: str, params: Mapping[str, list[str]]) -> None:
        if method == "POST":
            return self._serve_aggregate_post()
        rate = self.limiter.check("aggregate")
        spec = self._plan_get(params, kind="aggregate")
        payload = self._build_aggregate(spec)
        etag = '"' + hashlib.sha256(canonical_bytes(payload)).hexdigest()[:16] + '"'
        self._json(200, payload, extra={**self._rate_headers(rate), "ETag": etag})

    def _serve_aggregate_post(self) -> None:
        rate = self.limiter.check("aggregate")
        self._require_media()
        body = self._read_body()
        try:
            data = json.loads(body.decode("utf-8")) if body else {}
        except (ValueError, UnicodeDecodeError) as exc:
            raise AdcError("INVALID_BODY", f"body is not valid JSON: {exc}") from None
        if not isinstance(data, dict):
            raise AdcError("INVALID_BODY", "body must be a JSON object")
        spec = self._plan_post(data, kind="aggregate")
        payload = self._build_aggregate(spec)
        headers = self._rate_headers(rate)
        headers["Cache-Control"] = "no-store, private"
        self._json(200, payload, extra=headers)

    # -- cold entry (AI-ENTRY.md) ---------------------------------------

    def _serve_ai_map(self, method: str) -> None:
        if self.site.ai is None:
            raise AdcError("ANCHOR_NOT_FOUND", "no such endpoint: /AI/")
        if method != "GET":
            raise AdcError(
                "NOT_IMPLEMENTED",
                f"{method} is not part of the /AI/ cold entry",
            )
        self._json(
            200,
            self.site.ai,
            media=AI_MEDIA,
            extra={"Cache-Control": "public, max-age=3600"},
        )

    def _serve_ai_var(
        self,
        method: str,
        path: str,
        params: Mapping[str, list[str]],
    ) -> None:
        entry = self.site.ai
        if entry is None:
            raise AdcError("ANCHOR_NOT_FOUND", f"no such endpoint: {path}")
        if method != "GET":
            raise AdcError(
                "NOT_IMPLEMENTED",
                f"{method} is not part of the /AI/ cold entry",
            )
        variables: dict[str, str] = dict(entry.get("vars") or {})
        match = _AI_VAR_RE.match(path)
        variable = match.group(1) if match else None
        if variable is None or variable not in variables:
            self._ai_error(404, "unknown_variable", list(variables))
            return
        try:
            node = self.site.resolve(path=variables[variable])
        except AdcError:
            # the generation no longer carries this target: the var is unknown
            self._ai_error(404, "unknown_variable", list(variables))
            return
        available = self._ai_keys(node)
        key = self._single(params, "get") or "text"
        if key not in available:
            self._ai_error(400, "key_unavailable", available)
            return
        try:
            payload = self._ai_payload(node, key, params)
        except AiParamError as exc:
            self._ai_error(
                400,
                "invalid_parameter",
                extra={"parameter": exc.parameter, "detail": exc.detail},
            )
            return
        self._json(
            200,
            payload,
            media=AI_MEDIA,
            extra={"Cache-Control": "no-store"},
        )

    def _ai_error(
        self,
        status: int,
        code: str,
        available: Iterable[str] | None = None,
        extra: Mapping[str, Any] | None = None,
    ) -> None:
        body: dict[str, Any] = {"error": code}
        if available is not None:
            body["available"] = list(available)
        if extra:
            body.update(extra)
        self._json(
            status,
            body,
            media=AI_MEDIA,
            extra={"Cache-Control": "no-store"},
        )

    def _ai_keys(self, node: Any) -> list[str]:
        """Keys this node can actually produce, in the site's declared order."""
        entry = self.site.ai or {}
        allowed = ["text", "meta"]
        if node.kind != "collection":
            fields = node.fields
            if "price" in fields:
                allowed.append("price")
            if "stock" in fields:
                allowed.append("stock")
        return [key for key in (entry.get("keys") or []) if key in allowed]

    def _ai_limit(self, params: Mapping[str, list[str]]) -> int | None:
        raw = self._single(params, "limit")
        if raw is None:
            return None
        candidate = raw.strip()
        if not _AI_LIMIT_RE.fullmatch(candidate):
            raise AiParamError("limit", "must be an integer between 1 and 999999")
        return int(candidate)

    def _ai_fields(self, params: Mapping[str, list[str]]) -> list[str] | None:
        raw = self._single(params, "fields")
        if raw is None:
            return None
        names = [part.strip() for part in raw.split(",") if part.strip()]
        if not names:
            raise AiParamError("fields", "must name at least one field")
        for name in names:
            if not _AI_FIELD_RE.fullmatch(name):
                raise AiParamError("fields", f"invalid field name {name!r}")
        return list(dict.fromkeys(names))

    def _ai_payload(
        self,
        node: Any,
        key: str,
        params: Mapping[str, list[str]],
    ) -> dict[str, Any]:
        limit = self._ai_limit(params)
        wanted = self._ai_fields(params)
        if key != "text":
            if wanted is not None:
                raise AiParamError("fields", "'fields' only applies to get=text")
            if limit is not None:
                raise AiParamError("limit", "'limit' only applies to get=text")
        payload: dict[str, Any] = {"get": key, "path": node.path, "name": node.name}
        if key == "meta":
            payload["kind"] = node.kind
            payload["anchor"] = node.anchor
            payload["updated_at"] = node.updated_at
            if node.count is not None:
                payload["count"] = node.count
            return payload
        if key == "price":
            payload["price"] = node.fields.get("price")
            payload["currency"] = node.fields.get("currency")
            return payload
        if key == "stock":
            payload["stock"] = node.fields.get("stock")
            return payload
        if node.kind == "collection" and wanted is not None:
            records = node.item_records
            if limit is not None:
                records = records[:limit]
            rows = []
            for record in records:
                source = record
                if isinstance(record, Mapping):
                    inner = record.get("fields")
                    if isinstance(inner, Mapping):
                        source = inner
                rows.append(
                    {name: source[name] for name in wanted if name in source}
                )
            payload["fields"] = wanted
            payload["items"] = rows
            payload["count"] = len(rows)
            return payload
        if node.kind == "collection":
            names = [self.site.nodes[child].name for child in node.children]
            listed = names if limit is None else names[:limit]
            payload["text"] = "%s (%d): %s" % (
                node.name,
                len(names),
                ", ".join(listed),
            )
            return payload
        fields = node.fields
        if wanted is not None:
            fields = {name: fields[name] for name in wanted if name in fields}
        payload["text"] = "\n".join(
            "%s: %s" % (name, _ai_scalar(value)) for name, value in fields.items()
        )
        return payload

    # -- request planning (validation order steps 4..7) -------------------

    def _plan_get(self, params: Mapping[str, list[str]], kind: str) -> "Plan":
        anchor = self._single(params, "anchor")
        path = self._single(params, "path")
        data: dict[str, Any] = {
            "anchor": anchor,
            "path": path,
            "slice": self._single(params, "slice"),
            "limit": self._single(params, "limit"),
            "cursor": self._single(params, "cursor"),
            "fields": self._single(params, "fields"),
            "order_by": self._single(params, "order_by"),
            "tree_generation": self._single(params, "tree_generation"),
            "min_group_size": self._single(params, "min_group_size"),
            "approximate": self._single(params, "approximate"),
            "filter_query": {k: v for k, v in params.items() if k.startswith("filter[")},
        }
        if kind == "aggregate":
            data["metrics"] = self._single(params, "metrics")
            data["group_by"] = self._single(params, "group_by")
            data["having"] = params.get("having")
        if kind == "fetch":
            data["estimate_only"] = False
        return self._plan(data, kind=kind)

    def _plan_post(self, data: Mapping[str, Any], kind: str) -> "Plan":
        if kind == "fetch":
            size = len(json.dumps(data).encode("utf-8"))
            limits = self.site.limits
            if size > int(limits.get("max_post_bytes", 65536)):
                raise AdcError("PAYLOAD_TOO_LARGE", "parsed body exceeds limits.max_post_bytes")
        plan_data = dict(data)
        plan_data["filter_query"] = None
        plan_data["filter_body"] = data.get("filter")
        return self._plan(plan_data, kind=kind)

    def _plan(self, data: Mapping[str, Any], kind: str) -> "Plan":
        # step 4 schema already done by callers for POST; step 5 resolve
        anchor = data.get("anchor")
        path = data.get("path")
        if anchor and path:
            raise AdcError("ANCHOR_INVALID", "supply exactly one of anchor or path")

        supplied_generation = data.get("tree_generation")
        try:
            node = self.site.resolve(anchor=anchor, path=path)
        except AdcError as exc:
            # SPEC 6.10 / 5.3.3 precedence: an unresolvable anchor combined with
            # an explicitly stale tree_generation means the caller's anchor
            # table is out of date, not that the node never existed.
            if (
                exc.code == "ANCHOR_NOT_FOUND"
                and supplied_generation
                and supplied_generation != self.site.tree_generation
            ):
                raise AdcError(
                    "GENERATION_STALE",
                    "requested tree_generation is not current",
                    details={"current": self.site.tree_generation},
                ) from None
            raise

        slice_name = data.get("slice")
        if slice_name is None:
            slice_ = node.default_slice
        else:
            slice_ = node.slice(str(slice_name))

        limits = self.site.limits
        defaults = self.site.defaults
        max_limit = int(limits.get("max_limit", 200))
        page_size = int(defaults.get("page_size", 20))
        limit_raw = data.get("limit")
        limit = (
            _coerce_int(str(limit_raw), "limit", 1, max_limit)
            if limit_raw is not None
            else page_size
        )
        limit = max(1, min(limit, max_limit))

        # step 6: filter / metric / group-by validation
        specs = node.filter_specs
        conditions = []
        post_filter = None
        if data.get("filter_body") is not None:
            post_filter = parse_post_filter(
                data.get("filter_body"),
                specs,
                max_depth=int(limits.get("max_filter_depth", 4)),
                max_conditions=int(limits.get("max_filter_conditions", 32)),
            )
        elif data.get("filter_query"):
            conditions = conditions_from_query(
                data["filter_query"],
                specs,
                max_conditions=int(limits.get("max_filter_conditions", 32)),
            )

        metrics = []
        group_by: list[str] = []
        having: list[tuple[str, str, Any]] = []
        order_by: list[tuple[str, bool]] = []
        min_group_size = None
        approximate = False

        if kind == "aggregate":
            aggregations = node.passport.get("aggregations") or {}
            if not aggregations.get("supported"):
                raise AdcError(
                    "AGGREGATE_UNSUPPORTED",
                    f"node {node.path!r} does not support aggregation",
                )
            declared_metrics = [str(m) for m in aggregations.get("metrics", [])]
            declared_groups = [str(g) for g in aggregations.get("group_by", [])]
            metrics = parse_metrics(data.get("metrics"))
            for metric in metrics:
                if metric.expr not in declared_metrics:
                    raise AdcError(
                        "METRIC_UNSUPPORTED",
                        f"metric {metric.expr!r} is not declared for this node",
                        details={"unsupported": [metric.expr], "declared": declared_metrics},
                    )
            group_by = parse_group_by(data.get("group_by"))
            for field_name in group_by:
                if field_name not in declared_groups:
                    raise AdcError(
                        "GROUP_BY_UNSUPPORTED",
                        f"group_by field {field_name!r} is not declared for this node",
                        details={"unsupported": [field_name], "declared": declared_groups},
                    )
            max_group_by = int(limits.get("max_aggregate_group_by", 3))
            if len(group_by) > max_group_by:
                raise AdcError(
                    "INVALID_AGGREGATION_TARGET",
                    f"group_by exceeds limits.max_aggregate_group_by ({max_group_by})",
                )
            having = parse_having(data.get("having"))
            min_group_default = int(aggregations.get("min_group_size_default", 5))
            min_group_raw = data.get("min_group_size")
            if min_group_raw is None:
                min_group_size = min_group_default
            else:
                requested = int(min_group_raw)
                if requested < min_group_default:
                    # SPEC 8.5: lowering the floor must be refused, never
                    # silently clamped, because the caller would then believe
                    # the stricter privacy guarantee was in force.
                    raise AdcError(
                        "FILTER_INVALID",
                        f"min_group_size {requested} is below the passport "
                        f"privacy floor {min_group_default}",
                        details={
                            "privacy_floor": min_group_default,
                            "requested": requested,
                        },
                    )
                min_group_size = requested
            approximate = _as_bool(data.get("approximate"), "approximate")

        if data.get("order_by"):
            parsed_order = (
                data["order_by"]
                if isinstance(data["order_by"], list)
                else str(data["order_by"])
            )
            order_by = parse_order_by(parsed_order)
            for name, _ in order_by:
                if not _ORDER_RE.match(name):
                    raise AdcError("INVALID_BODY", f"bad order_by field {name!r}")

        # step 7: generation pre-validation, before execution and hydration
        # (`supplied_generation` was captured during resolution above)
        if supplied_generation and supplied_generation != self.site.tree_generation:
            raise AdcError(
                "GENERATION_STALE",
                "requested tree_generation is not current",
                details={"current": self.site.tree_generation},
            )
        slice_generations = data.get("generation")
        if isinstance(slice_generations, Mapping):
            stale = {
                name: {"expected": gen, "current": node.slices[name].generation}
                for name, gen in slice_generations.items()
                if name in node.slices and node.slices[name].generation != gen
            }
            if stale:
                raise AdcError(
                    "GENERATION_STALE",
                    "one or more slice generations are stale",
                    details={"current": self.site.tree_generation, "stale": stale},
                )

        return Plan(
            site=self.site,
            node=node,
            slice=slice_,
            kind=kind,
            limit=limit,
            cursor_raw=data.get("cursor"),
            fields=data.get("fields"),
            conditions=conditions,
            post_filter=post_filter,
            metrics=metrics,
            group_by=group_by,
            having=having,
            order_by=order_by,
            min_group_size=min_group_size if min_group_size is not None else 5,
            approximate=approximate,
            estimate_only=bool(data.get("estimate_only")),
            params=data,
        )

    # -- execution -------------------------------------------------------

    def _execute(self, plan: "Plan") -> tuple[int, dict[str, Any], dict[str, str]]:
        if plan.estimate_only:
            return 200, self._build_estimate(plan), {"Cache-Control": "no-store"}

        node = plan.node
        headers: dict[str, str] = {}

        if node.kind == "collection":
            payload, headers = self._execute_collection(plan)
        else:
            payload, headers = self._execute_entity(plan)
        return 200, payload, headers

    def _execute_entity(self, plan: "Plan") -> tuple[dict[str, Any], dict[str, str]]:
        node = plan.node
        slice_ = plan.slice
        payload_value = slice_.payload or {}
        fields = dict(payload_value.get("fields") or {}) if isinstance(payload_value, Mapping) else {}
        fields = _project(plan.fields, fields)

        hydration = self._hydrate(slice_)
        if hydration is not None and hydration.get("status") == "degraded_partial":
            fields = {}

        body: dict[str, Any] = {
            "tree_generation": self.site.tree_generation,
            "anchor": node.anchor,
            "path": node.path,
            "slice": slice_.name,
            "generation": slice_.generation,
            "fields": fields,
        }
        if hydration is not None:
            body["hydration"] = {slice_.name: hydration}

        tokens, byte_len, _ = payload_stats({"fields": fields}, self.site.tokenizer)
        body["usage"] = self._usage(tokens, byte_len, 1, slice_, hydrated=hydration)
        headers = self._cache_headers(slice_, body["usage"])
        return body, headers

    def _execute_collection(self, plan: "Plan") -> tuple[dict[str, Any], dict[str, str]]:
        node = plan.node
        slice_ = plan.slice
        records = self.site.records(node, slice_.name)

        if plan.conditions:
            records = [r for r in records if conditions_match(plan.conditions, r)]
        if plan.post_filter is not None:
            records = [r for r in records if post_filter_match(plan.post_filter, r)]
        if plan.order_by:
            records = _sort_records(records, plan.order_by)

        binding = (
            f"fetch|{node.anchor}|{slice_.name}|{plan.limit}|"
            f"{plan.conditions!r}|{plan.post_filter!r}|{plan.order_by!r}|"
            f"{self.site.tree_generation}"
        )
        offset = self.cursor.decode(plan.cursor_raw, binding) if plan.cursor_raw else 0
        window = records[offset : offset + plan.limit]
        next_offset = offset + len(window)
        has_more = next_offset < len(records)

        items = []
        hydration_status: dict[str, Any] = {}
        for row in window:
            item = {
                "anchor": row.get("anchor"),
                "path": row.get("path"),
                "generation": row.get("generation"),
                "fields": _project(plan.fields, {k: v for k, v in row.items() if k not in ("anchor", "path", "generation")}),
            }
            hydration = self._hydrate(slice_)
            if hydration is not None:
                item["hydration"] = {slice_.name: hydration}
                hydration_status = hydration
            items.append(item)

        body: dict[str, Any] = {
            "tree_generation": self.site.tree_generation,
            "parent_anchor": node.anchor,
            "slice": slice_.name,
            "items": items,
            "cursor": self.cursor.encode(binding, next_offset) if has_more else None,
            "truncated": has_more,
        }
        payload_tokens, payload_bytes, _ = payload_stats({"items": items}, self.site.tokenizer)
        body["usage"] = self._usage(
            payload_tokens, payload_bytes, len(items), slice_,
            hydrated=hydration_status or None,
        )
        headers = self._cache_headers(slice_, body["usage"])
        return body, headers

    # -- estimate --------------------------------------------------------

    def _build_estimate(self, plan: "Plan") -> dict[str, Any]:
        node = plan.node
        slice_ = plan.slice

        if node.kind == "collection":
            records = self.site.records(node, slice_.name)
            if plan.conditions:
                records = [r for r in records if conditions_match(plan.conditions, r)]
            if plan.post_filter is not None:
                records = [r for r in records if post_filter_match(plan.post_filter, r)]
            count = len(records)
            items_returned = min(plan.limit, count)
            list_item_tokens = int(node.passport.get("list_item_tokens", 0)) or _mean_tokens(records)
            list_tokens = items_returned * list_item_tokens
            slice_tokens = 0 if slice_.name == "summary" else slice_.tokens
            breakdown = [
                {"part": "list", "items": items_returned, "tokens": list_tokens},
                {"part": "slice", "slice": slice_.name, "tokens": slice_tokens},
            ]
        else:
            count = 1
            items_returned = 1
            list_tokens = 0
            slice_tokens = slice_.tokens
            breakdown = [
                {"part": "slice", "slice": slice_.name, "tokens": slice_tokens},
            ]

        estimated = int(list_tokens + slice_tokens)
        price = slice_.price or 0.0
        result = {
            "anchor": node.anchor,
            "path": node.path,
            "count": count,
            "count_exact": True,
            "items_returned": items_returned,
            "estimated_tokens": estimated,
            "estimated_bytes": estimated * 4,
            "breakdown": breakdown,
            "price": {
                "amount": price,
                "currency": slice_.currency or self.site.defaults.get("currency", "USD"),
                "unit": slice_.unit or "call",
            },
            "generation": {slice_.name: slice_.generation},
            # Declared per-row averages, not a per-byte serialization, so the
            # number is bounded rather than exact.
            "confidence": "bounded" if node.kind == "collection" else "exact",
            "tree_generation": self.site.tree_generation,
        }
        if slice_.dynamic:
            result["dynamic"] = True
        return result

    # -- aggregate -------------------------------------------------------

    def _build_aggregate(self, plan: "Plan") -> dict[str, Any]:
        node = plan.node
        records = self.site.records(node, plan.slice.name)
        if plan.conditions:
            records = [r for r in records if conditions_match(plan.conditions, r)]
        if plan.post_filter is not None:
            records = [r for r in records if post_filter_match(plan.post_filter, r)]

        config = AggConfig(
            metrics=plan.metrics,
            group_by=plan.group_by,
            having=plan.having,
            order_by=plan.order_by,
            limit=min(plan.limit, int(self.site.limits.get("max_aggregate_groups", 1000))),
            min_group_size=plan.min_group_size,
            approximate=plan.approximate,
        )
        result = compute_aggregate(records, config)

        source_generations = sorted(
            {s.generation for n in self.site.nodes.values() for s in n.slices.values()}
        )[:1]
        fingerprint = {
            "tree_generation": self.site.tree_generation,
            "anchor": node.anchor,
            "filter": [repr(c) for c in plan.conditions] if plan.conditions else [],
            "post_filter": repr(plan.post_filter),
            "metrics": [m.expr for m in plan.metrics],
            "group_by": plan.group_by,
            "source": source_generations,
        }
        generation = "AGG:" + hashlib.sha256(canonical_bytes(fingerprint)).hexdigest()[:8]

        payload: dict[str, Any] = {
            "tree_generation": self.site.tree_generation,
            "anchor": node.anchor,
            "path": node.path,
            "group_by": plan.group_by,
            "metrics": [{"expr": m.expr, "alias": m.alias} for m in plan.metrics],
            "cursor": None,
            "truncated": bool(result.get("truncated", False)),
            "approximate": False,
            "suppressed_groups": int(result.get("suppressed_groups", 0)),
            "partial": bool(result.get("partial", False)),
            "computed_at": _now(),
            "generation": generation,
        }
        if plan.group_by:
            payload["groups"] = result.get("groups", [])
        else:
            payload["values"] = result.get("values", {})
        # totals and its privacy flag travel together or not at all.
        if result.get("totals") is not None:
            payload["totals"] = result["totals"]
            payload["totals_visible_only"] = bool(result.get("totals_visible_only", False))
        payload["coverage"] = result.get("coverage", 1.0)

        text = json.dumps(payload, ensure_ascii=False, separators=(",", ":"))
        payload["usage"] = {
            "tokens": count_tokens(text, self.site.tokenizer),
            "bytes": bytes_for(text),
            "items": 0,
            "cost": {"amount": 0.0, "currency": self.site.defaults.get("currency", "USD"), "unit": "call"},
        }
        return payload

    # -- hydration, cost, caching ---------------------------------------

    def _hydrate(self, slice_: Any) -> dict[str, Any] | None:
        if not slice_.dynamic:
            return None
        hydrated_at = _now()
        if slice_.name not in self.fail_slices:
            return {
                "status": "live",
                "hydrated_at": hydrated_at,
                "staleness_seconds": 0,
            }

        mode = slice_.hydration_mode
        staleness = self.stale_seconds
        max_staleness = slice_.max_staleness_seconds

        if mode == "stale_fallback" and (
            max_staleness is None or staleness <= max_staleness
        ):
            return {
                "status": "stale_fallback",
                "hydrated_at": hydrated_at,
                "staleness_seconds": staleness,
                "max_staleness_seconds": max_staleness,
                "failure_reason": "simulated_upstream_timeout",
                "revalidation_required": True,
            }
        if mode == "partial_omit":
            return {
                "status": "degraded_partial",
                "hydrated_at": hydrated_at,
                "staleness_seconds": staleness,
                "failure_reason": "simulated_upstream_timeout",
            }
        # fail_closed, or a stale_fallback that aged past its budget (SPEC 7.3)
        raise AdcError(
            "HYDRATION_FAILED",
            f"dynamic slice {slice_.name!r} could not be hydrated",
            details={
                "slice": slice_.name,
                "mode": mode,
                "staleness_seconds": staleness,
                "max_staleness_seconds": max_staleness,
            },
        )

    def _usage(
        self,
        tokens: int,
        byte_len: int,
        items: int,
        slice_: Any,
        hydrated: Mapping[str, Any] | None = None,
    ) -> dict[str, Any]:
        degraded = bool(
            hydrated and hydrated.get("status") in (
                "stale_fallback", "degraded_partial", "synthetic_stub"
            )
        )
        price = 0.0 if degraded else float(slice_.price or 0.0)
        return {
            "tokens": tokens,
            "bytes": byte_len,
            "items": items,
            "cost": {
                "amount": price,
                "currency": slice_.currency or self.site.defaults.get("currency", "USD"),
                "unit": slice_.unit or "call",
            },
        }

    def _cache_headers(self, slice_: Any, usage: Mapping[str, Any]) -> dict[str, str]:
        amount = float(usage.get("cost", {}).get("amount", 0.0))
        headers: dict[str, str] = {"ADC-Cost-Amount": _fmt(amount)}
        if amount > 0:
            headers["ADC-Cost-Currency"] = str(usage["cost"].get("currency", "USD"))
            headers["ADC-Cost-Unit"] = str(usage["cost"].get("unit", "call"))
        if slice_.dynamic or amount > 0:
            headers["Cache-Control"] = "private, no-store"
        else:
            headers["Cache-Control"] = "public, max-age=60"
        return headers


# --------------------------------------------------------------------------
# Plan + helpers
# --------------------------------------------------------------------------


class Plan:
    __slots__ = (
        "site", "node", "slice", "kind", "limit", "cursor_raw", "fields",
        "conditions", "post_filter", "metrics", "group_by", "having",
        "order_by", "min_group_size", "approximate", "estimate_only", "params",
    )

    def __init__(self, **kwargs: Any) -> None:
        for key in self.__slots__:
            setattr(self, key, kwargs.get(key))


def _ai_scalar(value: Any) -> str:
    if isinstance(value, list):
        return ", ".join(str(item) for item in value)
    return str(value)


def _tree_entry(node: Any) -> dict[str, Any]:
    entry: dict[str, Any] = {
        "name": node.name,
        "path": node.path,
        "anchor": node.anchor,
        "kind": node.kind,
        "has_children": node.has_children,
    }
    if node.kind == "collection" and node.count is not None:
        entry["count"] = node.count
    if node.updated_at:
        entry["updated_at"] = node.updated_at
    return entry


def _int_param(
    params: Mapping[str, list[str]],
    name: str,
    default: int,
    minimum: int,
    maximum: int,
) -> int:
    raw = params.get(name)
    if not raw:
        return default
    return _coerce_int(raw[0], name, minimum, maximum)


def _coerce_int(raw: str, name: str, minimum: int, maximum: int) -> int:
    try:
        value = int(str(raw))
    except (TypeError, ValueError):
        raise AdcError("INVALID_BODY", f"{name} must be an integer") from None
    if value < minimum or value > maximum:
        raise AdcError(
            "INVALID_BODY",
            f"{name} must be between {minimum} and {maximum}, got {value}",
            details={"param": name, "min": minimum, "max": maximum},
        )
    return value


def _as_bool(value: Any, name: str) -> bool:
    if value is None or value is False:
        return False
    if value is True:
        return True
    text = str(value).lower()
    if text in ("true", "1", "yes"):
        return True
    if text in ("false", "0", "no"):
        return False
    raise AdcError("INVALID_BODY", f"{name} must be true or false")


def _fmt(value: float) -> str:
    return f"{value:g}"


def _project(spec: str | None, record: Mapping[str, Any]) -> dict[str, Any]:
    """Dot-path projection. ``anchor`` and ``generation`` are always kept."""
    if not spec:
        return dict(record)
    wanted = [part.strip() for part in str(spec).split(",") if part.strip()]
    out: dict[str, Any] = {}
    for dotted in wanted:
        source: Any = record
        ok = True
        for segment in dotted.split("."):
            if isinstance(source, Mapping) and segment in source:
                source = source[segment]
            else:
                ok = False
                break
        if not ok:
            continue
        target = out
        segments = dotted.split(".")
        for segment in segments[:-1]:
            nxt = target.setdefault(segment, {})
            if not isinstance(nxt, dict):
                ok = False
                break
            target = nxt
        if ok:
            target[segments[-1]] = source
    return out


def _sort_records(records: list[dict[str, Any]], order_by: list[tuple[str, bool]]) -> list[dict[str, Any]]:
    result = list(records)
    for name, desc in reversed(order_by):
        result.sort(
            key=lambda row, n=name: _sort_key(row.get(n)),
            reverse=desc,
        )
    return result


def _sort_key(value: Any) -> tuple:
    if value is None:
        return (0, "")
    if isinstance(value, bool):
        return (1, int(value))
    if isinstance(value, (int, float)):
        return (1, value)
    return (2, str(value))


def _mean_tokens(records: Iterable[Mapping[str, Any]]) -> int:
    rows = list(records)
    if not rows:
        return 0
    total = sum(payload_stats({"fields": dict(r)}, "utf8_bytes_div_4")[0] for r in rows)
    return max(1, total // len(rows))


def create_server(
    site_dir: str,
    host: str = "127.0.0.1",
    port: int = 8080,
    *,
    fail_slices: Iterable[str] | None = None,
    stale_seconds: int | None = None,
    secret: bytes | None = None,
) -> ThreadingHTTPServer:
    """Build a ready-to-serve :class:`ThreadingHTTPServer`."""
    site = Site.load(site_dir)
    limits = site.limits
    rate_limits = site.manifest.get("rate_limits") or []
    scope = rate_limits[0] if rate_limits else {}
    window_seconds = _parse_window(str(scope.get("window", "PT1M")))
    requests = int(scope.get("requests", 60))
    if os.environ.get("ADC_NO_RATE_LIMIT") == "1":
        requests = 10**9
    limiter = RateLimiter(
        requests=requests,
        window_seconds=window_seconds,
        weights=scope.get("weights") or {},
    )

    if fail_slices is None:
        env_value = os.environ.get("ADC_FAIL_SLICES", "")
        fail_slices = [s.strip() for s in env_value.split(",") if s.strip()]
    if stale_seconds is None:
        stale_seconds = int(os.environ.get("ADC_STALE_SECONDS", "612"))

    handler = type(
        "BoundAdcHandler",
        (AdcHandler,),
        {
            "site": site,
            "cursor": CursorTool(secret=secret),
            "limiter": limiter,
            "fail_slices": set(fail_slices),
            "stale_seconds": stale_seconds,
            "max_post_bytes": int(limits.get("max_post_bytes", 65536)),
        },
    )
    server = ThreadingHTTPServer((host, port), handler)
    server.daemon_threads = True
    return server


def _parse_window(value: str) -> int:
    """Parse an ISO 8601 duration such as ``PT1M`` into seconds."""
    match = re.fullmatch(
        r"P(?:(\d+)D)?(?:T(?:(\d+)H)?(?:(\d+)M)?(?:(\d+)S)?)?", value
    )
    if not match:
        return 60
    days, hours, minutes, seconds = (int(g) if g else 0 for g in match.groups())
    total = days * 86400 + hours * 3600 + minutes * 60 + seconds
    return total or 60


def serve(site_dir: str, host: str = "127.0.0.1", port: int = 8080) -> None:
    server = create_server(site_dir, host, port)
    print(f"ADC reference server on http://{host}:{port} (site: {site_dir})")
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        print("\nshutting down")
    finally:
        server.server_close()


def run() -> None:  # pragma: no cover - CLI shim
    import argparse

    parser = argparse.ArgumentParser(description="ADC reference server")
    parser.add_argument("site", help="path to a site directory")
    parser.add_argument("--host", default="127.0.0.1")
    parser.add_argument("--port", type=int, default=8080)
    args = parser.parse_args()
    serve(args.site, args.host, args.port)


if __name__ == "__main__":  # pragma: no cover
    run()