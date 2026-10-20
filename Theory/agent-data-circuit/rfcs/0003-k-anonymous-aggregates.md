# RFC-0003: K-Anonymous Aggregates and What They Still Leak

- Status: Draft
- Version: ADC 0.1
- Created: 2026-09-25
- Requires: ADC 0.1

## Problem
`aggregate` exists so agents stop downloading whole catalogs
([SPEC.md 6.9](../SPEC.md#69-aggregate--commerce-tier)), but server-side reduction concentrates
individual records into small groups: narrow the filter to one buyer and `avg(price)` is that
buyer's price. Even a suppressed group leaks through the envelope around it -
`Value_suppressed = Totals - sum(Visible)` ([SPEC.md 8.5](../SPEC.md#85-privacy-invariants-for-totals)).
ADC 0.1 has rules ([SPEC.md 8.4](../SPEC.md#84-privacy-in-aggregates-k-anonymity),
[SECURITY.md 4](../SECURITY.md#4-privacy-for-aggregates)) but leaves real ambiguity:
1. What `min_group_size` floors, and what happens when a request tries to lower it.
2. Whether `totals` after suppression is a convenience or a leak, and what `coverage` and
   `partial` mean - a naive `coverage` re-derives the suppressed mass by itself.
3. Whether `approximate` can drift: a publisher unable to compute exactly might quietly
   return an approximation.
4. Whether k-anonymity holds across two queries. It does not, and pretending otherwise would
   be the real defect.

## Decision
### D1. `min_group_size` is a publisher-enforced floor
The passport declares `aggregations.min_group_size_default` (k). A request MAY raise
`min_group_size`; a request below the floor MUST be rejected with `400 FILTER_INVALID` and
`details.privacy_floor` ([SPEC.md 8.5](../SPEC.md#85-privacy-invariants-for-totals)). The floor
is evaluated at step 6 of [SPEC.md 6.10](../SPEC.md#610-validation-order-normative-for-all-methods),
before execution and before billing, so a violating query costs the caller nothing but the request.
### D2. Suppression rule
For every group G with `0 < count(G) < min_group_size`: G is omitted from `groups`, and
`suppressed_groups` increments once per suppressed group
([SPEC.md 8.4](../SPEC.md#84-privacy-in-aggregates-k-anonymity)). Extremum masking: when
`count(G) < 2 x min_group_size`, exact `min`/`max` over G MUST NOT be emitted; the alias is
set to `null` and the same `values` object carries `"metric_suppressed": true`.
Ungrouped aggregates are one group of all matched records (reading of
[SPEC.md 6.9.4](../SPEC.md#694-response), which is silent here): if
`0 < matched_count < min_group_size`, the server MUST return `"values": {}` with
`"suppressed_groups": 1`, `"partial": true`, and no `totals`, rather than a single-record
average. `reference/adc/aggregate.py` already returns this shape; this RFC generalizes it to privacy.

### D3. Totals: recompute over visible groups, or omit
When `suppressed_groups > 0`, exactly one of: recompute `totals` from the visible groups
only and set `"totals_visible_only": true`, or omit `totals` entirely. Folding suppressed
rows into `totals` is forbidden ([SPEC.md 6.9.4](../SPEC.md#694-response)). When
`suppressed_groups == 0`, `totals_visible_only` MUST be `false` and totals are honest
population totals. The flag exists so an agent can never mistake a partial sum for a full
one without reading a boolean.
### D4. `coverage` and `partial`
- `partial: true` iff `suppressed_groups > 0`. Truncation by `limit` is reported separately
  as `truncated: true`. `partial` is qualitative and leaks no magnitude.
- `coverage` is the fraction of matched records represented in `groups` (post-suppression,
  post-`limit`), as implemented in `reference/adc/aggregate.py`. **When
  `suppressed_groups > 0`, `coverage` MUST be omitted.** With visible group counts known,
  `coverage` yields `matched_count = sum(visible_counts) / coverage`, re-deriving the
  suppressed record mass - differencing by another name. When nothing was suppressed,
  `coverage` MAY be present: collection cardinality is already disclosed by passport `count`
  ([SPEC.md 6.3](../SPEC.md#63-passport-get-adcpassport)), so it adds no new secret.

### D5. `approximate: false` is a hard contract
If exact computation is impossible (cardinality bound, backend failure), the server MUST
fail with `422 AGGREGATION_TOO_BROAD` or `503 HYDRATION_FAILED` and MUST NOT return
`approximate: true` ([SPEC.md 6.9.4](../SPEC.md#694-response)). The response `approximate`
field MUST echo the mode actually used: a 200 carrying `approximate: true` for a request
that set `false` is a conformance violation, and the agent MUST discard that response
rather than consume it.
### D6. Residual: cross-query differencing is NOT closed
ADC closes differencing **inside one response** (D3, D4). It does not close it **across
responses**: querying with filter F and then with F plus one condition differences out
suppressed groups exactly as `totals` would have. Available mitigations are partial -
`aggregate` costs 5 rate-limit units (SPEC.md 4.2), `limits.max_aggregate_groups` caps
breadth, `min_group_size` is enforced per request. Not available: per-session query
accounting (would require per-client server state, which P7 forbids), differential-privacy
noise, or any proof that a publisher suppressed at all
([SECURITY.md 4](../SECURITY.md#4-privacy-for-aggregates) admits this). **ADC 0.1 does not
fully close differencing.** Agents must not read k-anonymity as an anonymity guarantee;
publishers needing one must add their own session policy or refuse `aggregate` entirely.

## Alternatives considered
**A. Differential-privacy noise.** Rejected for 0.1: it needs an epsilon governance process
nobody exists to run, contradicts the exactness contract in D5, and interacts badly with
integer `count`. Revisit only as opt-in `approximate: true`.
**B. Always omit `totals`.** Rejected: with `suppressed_groups == 0` totals are the point of
an aggregate and leak nothing beyond passport `count`; omission would tax correct behavior.
**C. Round metric values to coarse buckets.** Rejected as a default: it silently corrupts
`sum`/`avg` semantics with no flag an agent can trust. Could live inside an explicit
`approximate: true` response later.
**D. Cross-query suppression memory in the protocol.** Rejected: it needs per-client server
state (P7) and turns `aggregate` into a session protocol with no deployment story. Recorded
in D6 as a known non-goal.

## Consequences (positive)
- Small groups cannot be named by the response; the F.2 vector
  ([CONFORMANCE.md F.2](../CONFORMANCE.md#f2-k-anonymity-differencing)) passes by construction.
- `totals_visible_only` makes the one safe recomputation machine-detectable.
- Omitting `coverage` under suppression closes the side channel the naive definition would
  open, without losing anything when suppression is zero.
- `422 AGGREGATION_TOO_BROAD` preserves the meaning of `approximate: false` instead of
  letting it decay into advisory.
- The differencing limit is written down rather than implied, so reviewers can argue with
  the actual claim.

## Consequences (negative / known limits)
- Differencing across queries remains open (D6); k is per-request, not per-session.
- `suppressed_groups` still discloses how many small groups exist; `partial` discloses that
  something was withheld. Structure leaks, records do not (within one response).
- The ungrouped-aggregate reading in D2 is an interpretation: SPEC 6.9.4 does not state it,
  and a literalist publisher could ship single-record ungrouped averages today.
- `metric_suppressed` placement (inside `values`) is chosen here; SPEC 8.4 requires the flag
  but not its location.
- D4 changes `coverage` from "present in the example response" to "conditionally present";
  agents MUST treat absence as "not computable without leaking", not as an error.
- Compliance is unverifiable: no cryptographic proof of suppression exists in ADC, so
  privacy rests on publisher honesty ([SECURITY.md 4](../SECURITY.md#4-privacy-for-aggregates)).

## Test vectors
```json
[
  {
    "case": "F.2 suppression with visible-only totals",
    "request": "GET /adc/aggregate?anchor=%5Ba10%5D&metrics=count():count,avg(price):avg_price&group_by=brand&min_group_size=5",
    "given_group_sizes": { "sony": 10, "rare-brand": 2 },
    "expect_status": 200,
    "expect_body": { "groups": [ { "key": { "brand": "sony" }, "values": { "count": 10, "avg_price": 348.2 } } ], "totals": { "count": 10, "avg_price": 348.2 }, "totals_visible_only": true, "approximate": false, "suppressed_groups": 1, "partial": true, "truncated": false },
    "forbid": [ "coverage", "any group with key brand=rare-brand" ],
    "assert": "totals.count == sum of visible group counts only; suppressed rows never fold into totals (SPEC 8.5)"
  },
  {
    "case": "request below the passport privacy floor",
    "passport_excerpt": { "aggregations": { "supported": true, "min_group_size_default": 5 } },
    "request": "GET /adc/aggregate?anchor=%5Ba10%5D&metrics=count():count&group_by=brand&min_group_size=2",
    "expect_status": 400,
    "expect_body": { "type": "https://agentdatacircuit.org/errors/FILTER_INVALID", "status": 400, "retryable": false, "details": { "privacy_floor": 5 } },
    "assert": "server rejects before execution and before billing (SPEC 6.10 step 6)"
  },
  {
    "case": "approximate=false must fail, never degrade",
    "request_body": { "anchor": "[a10]", "metrics": [ { "expr": "count()", "alias": "count" } ], "group_by": [ "model" ], "limit": 1000, "approximate": false },
    "condition": "estimated group cardinality exceeds limits.max_aggregate_groups",
    "expect_status": 422,
    "expect_body": { "type": "https://agentdatacircuit.org/errors/AGGREGATION_TOO_BROAD", "status": 422, "retryable": false, "details": { "estimated_groups": 4213 } },
    "assert": "a 200 with approximate:true for this request is a conformance violation the agent MUST discard"
  },
  {
    "case": "extremum masking below 2k",
    "request": "GET /adc/aggregate?anchor=%5Ba10%5D&metrics=count():count,max(price):max_price&group_by=model&min_group_size=5",
    "given_group_size": 8,
    "expect_status": 200,
    "expect_body": { "groups": [ { "key": { "model": "WH-1000XM5" }, "values": { "count": 8, "max_price": null, "metric_suppressed": true } } ], "suppressed_groups": 0, "partial": false },
    "assert": "8 < 2 * 5, so the exact max is withheld even though the group itself is visible"
  }
]
```

## References
- SPEC.md [6.3](../SPEC.md#63-passport-get-adcpassport), [6.9.4](../SPEC.md#694-response), [8.4](../SPEC.md#84-privacy-in-aggregates-k-anonymity), [8.5](../SPEC.md#85-privacy-invariants-for-totals), [6.10](../SPEC.md#610-validation-order-normative-for-all-methods)
- [CONFORMANCE.md F.2](../CONFORMANCE.md#f2-k-anonymity-differencing), [F.6](../CONFORMANCE.md#f6-aggregate-privacy)
- [SECURITY.md 4](../SECURITY.md#4-privacy-for-aggregates), `reference/adc/aggregate.py` (suppression, totals, coverage)
- [rfcs/0002-token-accounting.md](0002-token-accounting.md)
