# RFC-0004: The Degradation Contract

- Status: Draft
- Version: ADC 0.1
- Created: 2026-09-25
- Requires: ADC 0.1

## Problem
Dynamic slices hydrate live backends on demand
([SPEC.md 7.2](SPEC.md#72-dynamic-slice-hydration)), and live backends fail. When they do, a
publisher has three bad options: serve stale data silently (P13 forbids presenting stale data
as live), serve nothing and lose the whole circuit (P10 forbids backend failure from
invalidating static slices), or serve stale data and charge a live hydration fee that never
happened (P14 forbids charging dynamic fees on degraded responses).
Without a contract the failure modes are worse than the outage: an agent books inventory from
a `stale_fallback` price, or a publisher bills a hydration fee over 10-minute-old data, and
the payload says nothing about which happened. Agents also lack a rule for when degraded data
may be read versus refused ([SPEC.md 7.3.3](SPEC.md#733-agent-transactional-safety)) and lack
an enforcement hook if the publisher charges anyway
([SPEC.md 7.3.4](SPEC.md#734-billing-invariants-for-degraded-responses)).

## Decision
### D1. Three declared modes, executed on failure
Each dynamic slice declares `degradation: { mode, max_staleness_seconds }` in the passport
([SPEC.md 6.3](SPEC.md#63-passport-get-adcpassport)). On dependency failure, timeout, or SLA
breach ([SPEC.md 7.3](SPEC.md#73-degradation-semantics)):
- **`stale_fallback`** - HTTP 200 with the most recent cached representation, allowed only
  while `staleness_seconds <= max_staleness_seconds`. The `hydration` object MUST declare
  `status: "stale_fallback"`, `hydrated_at`, `staleness_seconds`, `failure_reason`, and
  `revalidation_required: true`. Beyond the budget the publisher MUST use `fail_closed` or
  `partial_omit` ([SPEC.md 6.4.3](SPEC.md#643-per-slice-hydration)).
- **`partial_omit`** - HTTP 200, every static slice intact, the dynamic slice's data omitted,
  `hydration[slice].status = "degraded_partial"`.
- **`fail_closed`** - HTTP 503 with `HYDRATION_FAILED` and a `Retry-After` header
  ([SPEC.md 6.12](SPEC.md#612-errors-rfc-9457-problem-details)); mandatory when the slice is
  critical or the staleness budget is exceeded.
No fourth option: synthesizing plausible values under any status outside the declared
vocabulary is a P13 violation.

### D2. Hydration vocabulary and agent treatment
`hydration.status` vocabulary: `live`, `cached`, `stale_fallback`, `degraded_partial`,
`synthetic_stub` ([SPEC.md 7.3.2](SPEC.md#732-the-hydration-object)).
- `live` and `cached` are non-degraded: data is current within its generation and the
  transactional gate is not engaged.
- `stale_fallback` and `degraded_partial` are degraded (D3).
- `synthetic_stub` is fabricated placeholder data, strictly more dangerous than stale data,
  so this RFC extends the transactional gate to it. (SPEC 7.3.3 names only `stale_fallback`
  and `degraded_partial`; see limits.)
- Unknown status values MUST be treated as `stale_fallback` (SPEC.md Annex C), so a
  forward-declared status fails safe.

### D3. Transactional gate
An agent MUST NOT initiate a write, payment, binding reservation, or any external side
effect on data whose hydration is `stale_fallback`, `degraded_partial`, or `synthetic_stub`
([SPEC.md 7.3.3](SPEC.md#733-agent-transactional-safety),
[agent-runtime/AGENT-RUNTIME.md 7](agent-runtime/AGENT-RUNTIME.md#7-degradation-handling)).
Required instead: re-request with a precondition (an If-Match-style guard on the slice
`generation`) or wait for a `live` cycle before acting. Read-only informational use of
`stale_fallback` is allowed provided the agent discloses the staleness age to its consumer (A6).

### D4. Billing invariant and the agent-side veto
For any response where `hydration.status` is `stale_fallback`, `degraded_partial`, or
`synthetic_stub`:
- The publisher MUST set `ADC-Cost-Amount: 0.0` and `usage.cost.amount: 0.0`
  ([SPEC.md 7.3.4](SPEC.md#734-billing-invariants-for-degraded-responses)).
- The agent MUST reject any non-zero charge with `DEGRADATION_BILLING_VIOLATION` and refuse
  to reconcile it. The code is agent-local: `reference/adc/errors.py` gives it status 0, and
  publishers MUST NOT emit it over HTTP.
- Refusal means: the fiat charge is not applied (effective cost 0 for this response), the
  reservation is released per SPEC.md 9.2, and the origin is flagged non-conforming. Token
  reconciliation still proceeds - context was genuinely consumed even when hydration failed,
  and `B_token_rem` must track reality.
- On `fail_closed` (503) there is no payload and no `usage`; the budget FSM releases the
  reservation and returns to `ESTIMATING` or `HALTED` as for any other 503 (SPEC.md 9.2).

## Alternatives considered
**A. Serve stale data under HTTP caching headers only (`Age`, `Warning`).** Rejected: HTTP
caches hide origin state, and `Age` cannot say why data is old or whether the backend is
down. ADC puts the truth in a per-slice `hydration` object agents must parse.
**B. Fail every dynamic request during an outage (`fail_closed` only).** Rejected by P10:
static slices and unaffected dynamic slices must survive. `fail_closed` stays the backstop
for critical slices and blown staleness budgets, not the default.
**C. Charge a discounted fee on degraded responses instead of zero.** Rejected: it creates a
standing incentive to declare failure (cheaper serving) or hide it (higher revenue), and
[NON-GOALS.md](NON-GOALS.md) rules out the settlement machinery that could enforce refunds.
Zeroing the charge at the response is enforceable by the only party that can act
unilaterally: the agent, at D4.
**D. Publisher-initiated refund/dispute flow.** Rejected for 0.1: it needs an identity and
settlement layer ADC explicitly does not own, and `DEGRADATION_BILLING_VIOLATION` has no
wire channel to report it with.

## Consequences (positive)
- Stale data cannot pass as live: every degraded path carries an explicit `hydration` status,
  and agents gate transactions on it (D3).
- The billing asymmetry removes the incentive to mask backend failure - serve degraded data
  for free or fail closed, but never both badly and paid.
- Static and dynamic stay decoupled (P10): one dead dependency does not darken the circuit.
- The agent gets a hard, auditable rejection code instead of a heuristic.

## Consequences (negative / known limits)
- `hydration` is self-reported: a publisher can label stale data `live`. 0.1 has no liveness
  proof (Trust-tier signatures attest integrity, not freshness).
- Gating `synthetic_stub` is this RFC's reading; SPEC 7.3.3 enumerates only `stale_fallback`
  and `degraded_partial`, while SPEC 7.3.4's billing set includes `synthetic_stub`.
- Billing ambiguity: `hydration.status = "cached"` is absent from the zero-cost set of SPEC
  7.3.4, so a publisher-origin cache hit on a dynamic slice may still be billed. This RFC
  keeps the literal three-status set; only CDN hits are forced to zero (SPEC.md 6.4.6).
- `DEGRADATION_BILLING_VIOLATION` cannot be reported to the publisher; repeated violations
  cause only local origin flagging.
- Agents must carry age-disclosure and precondition machinery for A6 and D3 - complexity
  read-only agents pay even when they never transact.
- `fail_closed` during an outage produces 503 storms; correctness depends on agents honouring
  `Retry-After` ([SPEC.md 6.11](SPEC.md#611-rate-limiting)).

## Test vectors
Note: the illustrative hydration example in [SPEC.md 6.4.3](SPEC.md#643-per-slice-hydration)
shows `staleness_seconds: 1023` against `max_staleness_seconds: 900` while labelling the
status `stale_fallback`, which the normative sentence below it forbids. The vectors below use
a compliant pair and add an explicit violation case.
```json
[
  {
    "case": "stale_fallback inside the staleness budget",
    "passport_excerpt": { "slices": [ { "name": "logistics", "dynamic": true, "price": 0.0005, "currency": "USD", "unit": "call", "degradation": { "mode": "stale_fallback", "max_staleness_seconds": 900 } } ] },
    "expect_status": 200,
    "expect_headers": { "ADC-Cost-Amount": "0.0", "Cache-Control": "no-store" },
    "expect_body": { "hydration": { "logistics": { "status": "stale_fallback", "hydrated_at": "2026-10-01T15:30:00Z", "staleness_seconds": 612, "max_staleness_seconds": 900, "failure_reason": "upstream_timeout", "revalidation_required": true } }, "usage": { "tokens": 280, "bytes": 1120, "items": 1, "cost": { "amount": 0.0, "currency": "USD", "unit": "call" } } },
    "assert": "612 <= 900 so stale_fallback is legal; cost.amount == 0.0; agent may summarize with disclosed age but MUST NOT transact"
  },
  {
    "case": "staleness beyond budget cannot be served as stale_fallback",
    "given": { "mode": "stale_fallback", "max_staleness_seconds": 900, "available_cache_age_seconds": 1023 },
    "expect_one_of": [
      { "status": 503, "body": { "type": "https://agentdatacircuit.org/errors/HYDRATION_FAILED", "status": 503, "retryable": true }, "headers": { "Retry-After": "30" } },
      { "status": 200, "body": { "hydration": { "logistics": { "status": "degraded_partial" } } } }
    ],
    "forbid": { "hydration.logistics.status": "stale_fallback" }
  },
  {
    "case": "partial_omit keeps static data and zeroes the charge",
    "expect_status": 200,
    "expect_headers": { "ADC-Cost-Amount": "0.0" },
    "expect_body": { "anchor": "[a1042]", "slice": "summary", "fields": { "brand": "Sony", "model": "WH-1000XM5", "price": 348.0 }, "hydration": { "logistics": { "status": "degraded_partial", "failure_reason": "backend_unavailable" } }, "usage": { "tokens": 420, "bytes": 1680, "items": 1, "cost": { "amount": 0.0, "currency": "USD", "unit": "call" } } },
    "assert": "static fields present, dynamic slice data absent, agent MUST NOT transact"
  },
  {
    "case": "agent rejects a charge on a degraded response",
    "publisher_response_excerpt": { "hydration": { "logistics": { "status": "stale_fallback", "staleness_seconds": 612, "max_staleness_seconds": 900 } }, "usage": { "tokens": 280, "cost": { "amount": 0.0005, "currency": "USD", "unit": "call" } } },
    "expect_agent_local": { "code": "DEGRADATION_BILLING_VIOLATION", "status": 0, "retryable": false, "action": "refuse to reconcile the fiat charge, treat effective cost as 0.0, release the reservation, flag the origin non-conforming; token reconciliation still applies usage.tokens" },
    "assert": "the violation is never sent to the publisher as an HTTP error (reference/adc/errors.py: agent-side only)"
  }
]
```

## References
- SPEC.md [7.3](SPEC.md#73-degradation-semantics), [7.3.2](SPEC.md#732-the-hydration-object), [7.3.3](SPEC.md#733-agent-transactional-safety), [7.3.4](SPEC.md#734-billing-invariants-for-degraded-responses)
- SPEC.md [6.4.3](SPEC.md#643-per-slice-hydration), [6.4.6](SPEC.md#646-billing-and-caching-invariants), [9.2](SPEC.md#92-agent-budget-state-machine-with-concurrency)
- [agent-runtime/AGENT-RUNTIME.md 7](agent-runtime/AGENT-RUNTIME.md#7-degradation-handling), [CONFORMANCE.md F.8](CONFORMANCE.md#f8-hydration-degradation-and-billing), [ECONOMICS.md 5](ECONOMICS.md#5-micro-billing)
- `reference/adc/errors.py` (`DEGRADATION_BILLING_VIOLATION`, status 0)
- [rfcs/0001-stateless-anchors.md](rfcs/0001-stateless-anchors.md), [rfcs/0002-token-accounting.md](rfcs/0002-token-accounting.md)
