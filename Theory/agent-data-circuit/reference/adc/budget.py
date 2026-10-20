"""Agent budget state machine (SPEC.md 9.2, Annex A.2).

    IDLE -> ESTIMATING -> EVALUATING -> COMMITTED -> EXECUTING -> RECONCILING
                                     \\-> HALTED

Reservation is atomic under a lock: two concurrent fetches can never both
admit against the same remaining budget. Every failure path must call
``release_reservation()`` so a dropped connection cannot leak a hold.
"""

from __future__ import annotations

import threading
from dataclasses import dataclass, field
from enum import Enum
from typing import Any

from .errors import AdcError

__all__ = ["BudgetState", "Reservation", "AgentBudget"]


class BudgetState(str, Enum):
    IDLE = "IDLE"
    ESTIMATING = "ESTIMATING"
    EVALUATING = "EVALUATING"
    COMMITTED = "COMMITTED"
    EXECUTING = "EXECUTING"
    RECONCILING = "RECONCILING"
    HALTED = "HALTED"
    HALTED_OVERRUN = "HALTED_OVERRUN"


@dataclass
class Reservation:
    key: str
    tokens: float
    fiat: float
    state: BudgetState = BudgetState.COMMITTED


@dataclass
class AgentBudget:
    """Token and fiat ceilings with reserve/reconcile semantics."""

    token_max: float
    fiat_max: float = 0.0
    token_rem: float = field(init=False)
    fiat_rem: float = field(init=False)
    state: BudgetState = BudgetState.IDLE
    _lock: threading.Lock = field(default_factory=threading.Lock, repr=False)
    _holds: dict[str, Reservation] = field(default_factory=dict, repr=False)

    def __post_init__(self) -> None:
        if self.token_max < 0 or self.fiat_max < 0:
            raise ValueError("budget ceilings must be non-negative")
        self.token_rem = float(self.token_max)
        self.fiat_rem = float(self.fiat_max)

    # -- state transitions ------------------------------------------------

    def project(self, slice_tokens: int, limit: int, list_item_tokens_p95: int) -> float:
        """SPEC 9.2 projection: ``slice.tokens + limit * list_item_tokens_p95``."""
        self.state = BudgetState.ESTIMATING
        return float(slice_tokens) + float(limit) * float(list_item_tokens_p95)

    def admit(self, key: str, c_tokens: float, c_fiat: float = 0.0) -> Reservation:
        """EVALUATING + COMMITTED: atomically reserve projected cost.

        Raises ``402 BUDGET_EXCEEDED`` when the projection does not fit.
        """
        with self._lock:
            self.state = BudgetState.EVALUATING
            if key in self._holds:
                raise AdcError(
                    "INTERNAL_ERROR", f"duplicate reservation key {key!r}"
                )
            if c_tokens > self.token_rem or c_fiat > self.fiat_rem:
                raise AdcError(
                    "BUDGET_EXCEEDED",
                    "projected cost exceeds the remaining budget",
                    details={
                        "current_balance": self.token_rem,
                        "required_balance": c_tokens,
                        "currency": "tokens",
                    },
                )
            hold = Reservation(key=key, tokens=c_tokens, fiat=c_fiat)
            self._holds[key] = hold
            self.token_rem -= c_tokens
            self.fiat_rem -= c_fiat
            self.state = BudgetState.COMMITTED
            return hold

    def executing(self) -> None:
        self.state = BudgetState.EXECUTING

    def reconcile(self, key: str, usage_tokens: float, usage_fiat: float = 0.0) -> None:
        """Release the hold and charge actual usage (SPEC 9.2).

        ``B_token_rem += projected - actual``; an actual spend above the
        ceiling is only possible when the ceiling shrank concurrently, and is
        reported as ``HALTED_OVERRUN``.
        """
        with self._lock:
            hold = self._holds.pop(key, None)
            if hold is None:
                raise AdcError("INTERNAL_ERROR", f"unknown reservation {key!r}")
            self.token_rem += hold.tokens - usage_tokens
            self.fiat_rem += hold.fiat - usage_fiat
            self.state = BudgetState.RECONCILING
            if self.token_rem < 0 or self.fiat_rem < 0:
                self.token_rem = max(self.token_rem, 0.0)
                self.fiat_rem = max(self.fiat_rem, 0.0)
                self.state = BudgetState.HALTED_OVERRUN
                return
            self.state = BudgetState.IDLE

    def release_reservation(self, key: str) -> None:
        """Failure path: drop the hold without charging anything."""
        with self._lock:
            hold = self._holds.pop(key, None)
            if hold is not None:
                self.token_rem += hold.tokens
                self.fiat_rem += hold.fiat
            if self._holds:
                self.state = BudgetState.ESTIMATING
            else:
                self.state = BudgetState.IDLE

    def handle_retryable_failure(self, key: str) -> BudgetState:
        """Network error / 429 / 503: release and return to ESTIMATING."""
        self.release_reservation(key)
        self.state = BudgetState.ESTIMATING
        return self.state

    def halt(self) -> BudgetState:
        self.state = BudgetState.HALTED
        return self.state

    # -- introspection ----------------------------------------------------

    @property
    def holds(self) -> dict[str, Reservation]:
        return dict(self._holds)

    def snapshot(self) -> dict[str, Any]:
        return {
            "state": self.state.value,
            "token_max": self.token_max,
            "token_rem": self.token_rem,
            "fiat_max": self.fiat_max,
            "fiat_rem": self.fiat_rem,
            "active_reservations": len(self._holds),
        }

    def reset(self) -> None:
        with self._lock:
            self._holds.clear()
            self.token_rem = float(self.token_max)
            self.fiat_rem = float(self.fiat_max)
            self.state = BudgetState.IDLE