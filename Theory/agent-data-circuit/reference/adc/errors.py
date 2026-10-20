"""RFC 9457 Problem Details for ADC errors (SPEC.md 6.12).

Every error a conforming publisher returns is a Problem Details document with
``Content-Type: application/problem+json``. This module defines the canonical
error codes, their HTTP status, whether they are retryable, and a typed
exception the server layer can raise and serialize.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any

ERROR_BASE_URI = "https://agentdatacircuit.org/errors"

# code -> (http status, retryable, human title)
ERROR_TABLE: dict[str, tuple[int, bool, str]] = {
    "INVALID_BODY": (400, False, "Invalid Body"),
    "FILTER_INVALID": (400, False, "Invalid Filter"),
    "INVALID_AGGREGATION": (400, False, "Invalid Aggregation"),
    "INVALID_AGGREGATION_TARGET": (400, False, "Invalid Aggregation Target"),
    "METRIC_UNSUPPORTED": (400, False, "Metric Unsupported"),
    "GROUP_BY_UNSUPPORTED": (400, False, "Group By Unsupported"),
    "AGGREGATE_UNSUPPORTED": (400, False, "Aggregate Unsupported"),
    "PATH_INVALID": (400, False, "Invalid Path"),
    "CURSOR_INVALID": (400, False, "Invalid Cursor"),
    "ANCHOR_INVALID": (400, False, "Invalid Anchor"),
    "AUTH_REQUIRED": (401, False, "Authentication Required"),
    "BUDGET_EXCEEDED": (402, False, "Budget Exceeded"),
    "FORBIDDEN": (403, False, "Forbidden"),
    "ANCHOR_NOT_FOUND": (404, False, "Anchor Not Found"),
    "SLICE_NOT_FOUND": (404, False, "Slice Not Found"),
    "GENERATION_STALE": (409, True, "Generation Stale"),
    "ANCHOR_MOVED": (409, True, "Anchor Moved"),
    "ANCHOR_EXPIRED": (410, False, "Anchor Expired"),
    "CURSOR_EXPIRED": (410, True, "Cursor Expired"),
    "PAYLOAD_TOO_LARGE": (413, False, "Payload Too Large"),
    "UNSUPPORTED_MEDIA_TYPE": (415, False, "Unsupported Media Type"),
    "AGGREGATION_TOO_BROAD": (422, False, "Aggregation Too Broad"),
    "RATE_LIMIT": (429, True, "Rate Limit Exceeded"),
    "INTERNAL_ERROR": (500, True, "Internal Error"),
    "NOT_IMPLEMENTED": (501, False, "Not Implemented"),
    "BAD_GATEWAY": (502, True, "Bad Gateway"),
    "HYDRATION_FAILED": (503, True, "Hydration Failed"),
    "BACKEND_TIMEOUT": (504, True, "Backend Timeout"),
    # Agent-side (never emitted by a publisher over HTTP):
    "DEGRADATION_BILLING_VIOLATION": (0, False, "Degradation Billing Violation"),
}


@dataclass
class AdcError(Exception):
    """A protocol error carrying enough context to build a Problem Details body."""

    code: str
    detail: str = ""
    status: int | None = None
    retryable: bool | None = None
    details: dict[str, Any] = field(default_factory=dict)
    instance: str | None = None
    tree_generation: str | None = None
    request_id: str | None = None
    retry_after: int | None = None

    def __post_init__(self) -> None:
        if self.code not in ERROR_TABLE:
            raise ValueError(f"unknown ADC error code: {self.code!r}")
        status, retryable, _ = ERROR_TABLE[self.code]
        if self.status is None:
            self.status = status
        if self.retryable is None:
            self.retryable = retryable
        # RFC 9110 / CONFORMANCE F.8.5: a 503 (and any 429) must tell the
        # agent when to come back, otherwise retry policy is guesswork.
        if self.retry_after is None and self.status in (429, 503):
            self.retry_after = 30
        super().__init__(self.detail or self.code)

    @property
    def title(self) -> str:
        return ERROR_TABLE[self.code][2]

    def to_problem(self) -> dict[str, Any]:
        body: dict[str, Any] = {
            "type": f"{ERROR_BASE_URI}/{self.code}",
            "title": self.title,
            "status": self.status,
            "detail": self.detail,
        }
        if self.instance is not None:
            body["instance"] = self.instance
        if self.tree_generation is not None:
            body["tree_generation"] = self.tree_generation
        body["retryable"] = self.retryable
        if self.details:
            body["details"] = self.details
        else:
            body["details"] = {}
        if self.request_id is not None:
            body["request_id"] = self.request_id
        if self.retry_after is not None:
            body["retry_after"] = self.retry_after
        return body