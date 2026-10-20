"""Agent budget state machine tests (SPEC.md 9.2, Annex A.2, CONFORMANCE F.9)."""

import threading

import pytest

from adc.budget import AgentBudget, BudgetState
from adc.errors import AdcError

SLICE_TOKENS = 420
LIMIT = 10
LIST_ITEM_TOKENS_P95 = 480
PROJECTED = SLICE_TOKENS + LIMIT * LIST_ITEM_TOKENS_P95  # SPEC 9.2 formula


def test_estimating_projection_formula():
    # SPEC 9.2: c_tokens = slice.tokens + (limit * list_item_tokens_p95).
    budget = AgentBudget(token_max=10_000)
    projected = budget.project(SLICE_TOKENS, LIMIT, LIST_ITEM_TOKENS_P95)
    assert projected == float(PROJECTED)
    assert budget.state is BudgetState.ESTIMATING


def test_admit_reduces_remaining_balance():
    budget = AgentBudget(token_max=1000)
    hold = budget.admit("fetch-1", 300.0)
    assert budget.token_rem == 700.0
    assert budget.state is BudgetState.COMMITTED
    assert budget.holds["fetch-1"] is hold


def test_admit_over_budget_raises_budget_exceeded():
    # SPEC 9.3: exhaustion is 402 BUDGET_EXCEEDED with
    # details.current_balance and details.required_balance.
    budget = AgentBudget(token_max=1000)
    budget.admit("fetch-1", 700.0)
    with pytest.raises(AdcError) as exc:
        budget.admit("fetch-2", 400.0)
    error = exc.value
    assert error.code == "BUDGET_EXCEEDED"
    assert error.status == 402
    assert error.details["current_balance"] == 300.0
    assert error.details["required_balance"] == 400.0


def test_release_reservation_restores_balance_exactly():
    budget = AgentBudget(token_max=1000)
    before = budget.token_rem
    budget.admit("fetch-1", 250.0)
    budget.release_reservation("fetch-1")
    assert budget.token_rem == before
    assert "fetch-1" not in budget.holds


def test_reconcile_applies_actual_usage():
    # SPEC 9.2 RECONCILING: B_token_rem += projected - usage.
    budget = AgentBudget(token_max=1000)
    budget.admit("fetch-1", 300.0)
    budget.reconcile("fetch-1", 250.0)
    assert budget.token_rem == 750.0  # 1000 - 250 actual usage
    assert budget.state is BudgetState.IDLE
    assert budget.holds == {}


def test_reconcile_unknown_reservation_is_internal_error():
    budget = AgentBudget(token_max=1000)
    with pytest.raises(AdcError) as exc:
        budget.reconcile("nope", 10.0)
    assert exc.value.code == "INTERNAL_ERROR"


def test_retryable_failure_releases_and_returns_to_estimating():
    # SPEC 9.2: on network error / 429 / 503, release the reservation and
    # return to ESTIMATING.
    budget = AgentBudget(token_max=1000)
    budget.admit("fetch-1", 400.0)
    state = budget.handle_retryable_failure("fetch-1")
    assert state is BudgetState.ESTIMATING
    assert budget.token_rem == 1000.0
    assert budget.holds == {}


def test_overrun_marks_halted_overrun():
    # CONFORMANCE F.9.3: an actual spend above the ceiling halts with
    # HALTED_OVERRUN.
    budget = AgentBudget(token_max=1000)
    budget.admit("fetch-1", 1000.0)
    budget.reconcile("fetch-1", 1100.0)
    assert budget.state is BudgetState.HALTED_OVERRUN
    assert budget.token_rem == 0.0


def test_concurrent_admits_never_oversubscribe():
    # SPEC 9.2: COMMITTED atomically reserves projected costs (mutex).
    # CONFORMANCE F.9.1: exactly the affordable subset may succeed.
    budget = AgentBudget(token_max=1000)
    cost = 120.0
    admitted = []
    rejected = []
    lock = threading.Lock()
    barrier = threading.Barrier(20)

    def worker(index):
        barrier.wait()
        try:
            budget.admit("fetch-%d" % index, cost)
            with lock:
                admitted.append(index)
        except AdcError as exc:
            assert exc.code == "BUDGET_EXCEEDED"
            with lock:
                rejected.append(index)

    threads = [
        threading.Thread(target=worker, args=(i,)) for i in range(20)
    ]
    for thread in threads:
        thread.start()
    for thread in threads:
        thread.join(timeout=30)

    assert len(admitted) + len(rejected) == 20
    # floor(1000 / 120) == 8 affordable reservations, never more.
    assert len(admitted) == 8
    assert budget.token_rem >= 0
    held = sum(res.tokens for res in budget.holds.values())
    assert held == len(admitted) * cost
    # CONFORMANCE F.9.4: reserved + remaining == total budget.
    assert held + budget.token_rem == budget.token_max


def test_snapshot_reports_state_and_balances():
    budget = AgentBudget(token_max=500, fiat_max=5.0)
    snap = budget.snapshot()
    assert snap["state"] == "IDLE"
    assert snap["token_rem"] == 500.0
    assert snap["fiat_rem"] == 5.0
    assert snap["active_reservations"] == 0


def test_reset_restores_idle_state():
    budget = AgentBudget(token_max=1000)
    budget.admit("fetch-1", 100.0)
    budget.reset()
    assert budget.state is BudgetState.IDLE
    assert budget.token_rem == 1000.0
    assert budget.holds == {}


def test_negative_budget_ceilings_rejected():
    with pytest.raises(ValueError):
        AgentBudget(token_max=-1)
