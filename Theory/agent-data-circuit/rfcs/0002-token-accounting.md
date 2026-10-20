# RFC-0002: Token Accounting Requires a Declared Tokenizer

- Status: Draft
- Version: ADC 0.1
- Created: 2026-09-25
- Requires: ADC 0.1

## Problem
ADC declares costs in tokens everywhere: passport `list_item_tokens` and `list_item_tokens_p95`,
slice `tokens`, `estimate.estimated_tokens`, `usage.tokens`
([SPEC.md 4.2](../SPEC.md#42-manifest)). A token count is meaningless without the algorithm that
produced it: the same bytes count near 1 token per 4 bytes under one tokenizer and near 1 per
byte (or worse) under another, so two parties computing "cost" with different vocabularies are
adding incompatible units.
The damage is concrete. The budget FSM admits a request iff `c_tokens <= B_token_rem` where
`c_tokens = slice.tokens + (limit x list_item_tokens_p95)`
([SPEC.md 9.2](../SPEC.md#92-agent-budget-state-machine-with-concurrency)). If the publisher
counts with `o200k_base` and the agent budgets with `cl100k_base`, the scalar `B_token_rem`
mixes units and admission becomes arithmetic nonsense; reconciliation
(`B_token_rem += c_tokens_projected - usage.tokens`) then reports phantom overruns or hides
real ones. Nothing also stops a publisher from declaring a cheap per-endpoint tokenizer to
make hostile payloads look affordable, and ADC has no checkable claim for "I counted with the
algorithm you declared", so an honest mistake and cost laundering look identical.

## Decision
### D1. One tokenizer per circuit, declared in the manifest
The manifest field `tokenizer` is the single tokenizer for the whole circuit; `orient` echoes
it ([SPEC.md 6.1](../SPEC.md#61-orient-get-adcorient)). Every `tokens`, `estimated_tokens`,
`list_item_tokens`, `list_item_tokens_p95`, and `usage.tokens` field MUST be computed with it
([SPEC.md 4.2](../SPEC.md#42-manifest), SPEC.md 1). Per-slice, per-endpoint, and
request-selected tokenizers are forbidden: budget arithmetic compares these numbers across
requests against one scalar budget, and estimate-to-usage equality
([CONFORMANCE.md F.12](../CONFORMANCE.md#f12-tokenizer)) is checkable only inside one unit system.

### D2. Vocabulary
- `cl100k_base` - OpenAI GPT-4 family; normative reference tokenizer.
- `o200k_base` - OpenAI GPT-4o family.
- `utf8_bytes_div_4` - reference definition, computable by anyone with no dependencies:
  `tokens = ceil(bytes / 4)` where `bytes` is the UTF-8 byte length of the serialized text
  ([reference/adc/tokens.py](../reference/adc/tokens.py)).
- `<absolute-uri>` - escape hatch for a custom tokenizer with a publicly documented
  algorithm; a URI most agents cannot evaluate, so its counts are structurally unverifiable (D4).
The bundled example circuit uses `utf8_bytes_div_4` precisely because the reference
implementation can verify it byte-for-byte.

### D3. Disagreement between publisher and agent
1. **The wire wins for reconciliation.** The agent MUST budget, admit, and reconcile against
   the publisher's declared numbers; it MUST NOT recompute with its own tokenizer and
   substitute them ([SPEC.md 6.4.7](../SPEC.md#647-usage-and-reconciliation)).
2. **Unevaluable tokenizer demotes confidence.** If the agent cannot run the declared
   tokenizer (missing `tiktoken`, custom URI), every count from that origin is locally
   unverifiable: never admitted as exact, always subject to the safety margin (D4), with
   `bytes` as the cross-check floor - `bytes` is tokenizer-independent and always comparable.
3. **Deterministic mismatches mark the origin non-conforming.** Under `utf8_bytes_div_4`,
   `usage.tokens` MUST equal `ceil(bytes / 4)`; under exact confidence, `estimated_tokens`
   MUST equal `usage.tokens` ([CONFORMANCE.md F.12](../CONFORMANCE.md#f12-tokenizer)). Either is
   plain integer arithmetic; the agent flags the origin for the session and treats its future
   estimates as `sample`. ADC 0.1 defines no wire error for this (see limits).

### D4. `confidence: exact | bounded | sample`, and the conservatism rule
`estimate` reports `confidence` ([SPEC.md 6.5](../SPEC.md#65-estimate-get-adcestimate)); agents
MUST treat non-exact estimates conservatively. This RFC pins the vocabulary:
- **`exact`** - `estimated_tokens` will equal `usage.tokens` for the same unmodified request.
  Allowed only when the count is deterministic, the tokenizer is exactly evaluable
  server-side, and nothing is sampled.
- **`bounded`** - the estimate equals the normative projection
  `slice.tokens + (limit x list_item_tokens_p95)` (SPEC.md 9.2). It bounds the projection,
  not reality: tail items above p95 can overrun, absorbed by `HALTED_OVERRUN`.
- **`sample`** - derived from sampling or heuristics; neither equality nor projection guarantee.
`EVALUATING` MUST admit only against `exact` or `bounded` numbers, or against a `sample`
number multiplied by an implementation-defined safety factor >= 1. Understatement is a
budget-integrity defect, so publishers MUST downgrade `confidence` rather than round a
projection down, and MUST NOT emit `exact` for a projection formula.

## Alternatives considered
**A. Per-slice or per-endpoint tokenizer.** Rejected: it makes `B_token_rem` an
incommensurable sum, lets a publisher launder cost by moving a payload under a lenient
tokenizer, and breaks the F.12 estimate/usage identity.
**B. Agent-declared tokenizer (request parameter).** Rejected: publishers would tokenize
every payload once per vocabulary (CPU DoS surface), responses would need `Vary` on the
tokenizer (killing the cache contract, SPEC.md 6.9.5), and the sides could still disagree
after the fact.
**C. Publish only `bytes`, drop token counts.** Partly adopted: `bytes` stays in every
payload as the tokenizer-independent floor and `utf8_bytes_div_4` is the dependency-free
reference. Rejected as the sole unit: agent budgets are denominated in tokens, so byte-only
budgets would replace the FSM in SPEC.md 9.2.
**D. Emit `tokens_by_tokenizer` maps everywhere.** Rejected for 0.1: it multiplies passport
size, invites unbounded vocabulary growth, and no working group exists to curate it. Unknown
member handling leaves the door open later.

## Consequences (positive)
- The budget FSM operates in one unit; admission and reconciliation are arithmetic an
  auditor can re-run ([ECONOMICS.md 3](../ECONOMICS.md#3-agent-budget-state-machine)).
- `utf8_bytes_div_4` gives every implementation a zero-dependency conformance oracle
  ([CONFORMANCE.md F.12](../CONFORMANCE.md#f12-tokenizer)).
- Disagreement is detectable client-side: identity and exact-confidence checks are integer
  comparisons, not opinions.
- `confidence` becomes an actionable contract instead of an adjective.

## Consequences (negative / known limits)
- `cl100k_base` and `o200k_base` counts are unverifiable without an optional dependency;
  agents must apply margin rather than trust (`reference/adc/tokens.py` flags this itself).
- `utf8_bytes_div_4` guarantees internal consistency, not real model cost: on CJK or
  symbol-dense text it mispredicts GPT-style counts badly; cross-vendor comparability stays
  open ([ECONOMICS.md 7](../ECONOMICS.md#7-tokenizer-caveat)).
- A publisher counting with a different algorithm is caught only post hoc, only by agents
  that reconcile, and has no wire error to be reported with (D3.3) - a 0.1 gap.
- `<absolute-uri>` tokenizers are third-party-unverifiable by construction: ADC defines no
  checksum or test-vector format for them.
- One tokenizer cannot be honestly shared by agents with wildly different vocabularies;
  margin, not truth, papers over that.

## Test vectors
```json
[
  {
    "case": "utf8_bytes_div_4 identity on a fetch response",
    "given": "manifest.tokenizer == utf8_bytes_div_4",
    "response_excerpt": { "anchor": "[a1042]", "slice": "summary", "usage": { "tokens": 420, "bytes": 1680, "items": 1, "cost": { "amount": 0.0, "currency": "USD", "unit": "call" } } },
    "assert": "usage.tokens == ceil(usage.bytes / 4) == ceil(1680 / 4) == 420; any other value violates CONFORMANCE F.12"
  },
  {
    "case": "exact confidence implies estimate equals usage",
    "estimate_excerpt": { "anchor": "[a10]", "count": 412, "count_exact": true, "items_returned": 20, "estimated_tokens": 10020, "estimated_bytes": 40080, "confidence": "exact" },
    "same_request_fetch_usage": { "tokens": 10020, "bytes": 40080, "items": 20 },
    "assert": "estimated_tokens == usage.tokens while confidence == exact and the request is unchanged"
  },
  {
    "case": "bounded estimate is exactly the SPEC 9.2 projection",
    "passport_excerpt": { "list_item_tokens": 160, "list_item_tokens_p95": 480, "slices": [ { "name": "summary", "tokens": 420, "bytes": 1680 } ] },
    "request": { "anchor": "[a10]", "limit": 20 },
    "estimate_excerpt": { "estimated_tokens": 10020, "confidence": "bounded" },
    "assert": "420 + (20 * 480) == 10020; the agent admits iff 10020 <= B_token_rem, then reconciles with actual usage.tokens"
  },
  {
    "case": "sample confidence must not be admitted as-is",
    "estimate_excerpt": { "anchor": "[a10]", "estimated_tokens": 8600, "confidence": "sample" },
    "agent_action": "reserve ceil(8600 * safety_factor) with safety_factor >= 1 (implementation-defined), or refuse admission; never treat 8600 as exact"
  },
  {
    "case": "publisher/agent tokenizer disagreement is detectable",
    "given": "manifest.tokenizer == utf8_bytes_div_4",
    "response_excerpt": { "usage": { "tokens": 3210, "bytes": 12800, "items": 20 } },
    "assert": "ceil(12800 / 4) == 3200 != 3210: flag a tokenizer-conformance violation, demote this origin's estimates to sample, keep reconciling against the wire value"
  }
]
```

## References
- SPEC.md [4.2](../SPEC.md#42-manifest), [6.4.7](../SPEC.md#647-usage-and-reconciliation), [6.5](../SPEC.md#65-estimate-get-adcestimate), [9.2](../SPEC.md#92-agent-budget-state-machine-with-concurrency)
- [CONFORMANCE.md F.12](../CONFORMANCE.md#f12-tokenizer), [ECONOMICS.md 7](../ECONOMICS.md#7-tokenizer-caveat)
- `reference/adc/tokens.py` (identity tokenizer, honest fallback)
- [rfcs/0003-k-anonymous-aggregates.md](0003-k-anonymous-aggregates.md), [rfcs/0004-degradation-contract.md](0004-degradation-contract.md)
