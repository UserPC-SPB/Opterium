# RFC-0001: Stateless Anchors Across Tree Restructuring

- Status: Draft
- Version: ADC 0.1
- Created: 2026-09-25
- Requires: ADC 0.1

## Problem
[SPEC.md 5.3.1](../SPEC.md#531-idempotent-issuance) defines anchor issuance as a pure function
`anchor(tree_generation, path) -> [aN]`: the same path yields the same anchor across `tree`,
`passport`, and `fetch`. P6 promises anchors map 1:1 to paths "for the life of a tree
generation". [SPEC.md 5.3.3](../SPEC.md#533-lifecycle) then promises a grace period
(RECOMMENDED >= 7 days) answered with `409 ANCHOR_MOVED`. The three demands collide at
restructuring time:
1. Never bumping `tree_generation` keeps old anchors resolving to wrong or deleted nodes;
   cache keys ([SPEC.md 7.1](../SPEC.md#71-freshness-generations)) and cursor binding
   ([SPEC.md 6.4.4](../SPEC.md#644-pagination)) depend on generation changes and rot too.
2. Bumping and forgetting is legal but hostile: every stored anchor dies at once and the
   agent learns nothing about where the node went.
3. A durable `anchor -> path` row per node makes issuance stateful: replicas or
   static-export edges can disagree about what `[a10]` means.
The specification must fix which part is a pure function, which bounded state is permitted,
and which error separates "your context is stale" from "this anchor is dead" - otherwise
`409 GENERATION_STALE` and `410 ANCHOR_EXPIRED` are indistinguishable to the budget FSM (SPEC.md 9.2).

## Decision
### D1. Issuance reads no storage
Within a generation, `[aN]` is computed from `(tree_generation, path)` alone; the reference
assigns anchors by lexicographic rank of the canonical path (`reference/adc/anchors.py`,
`AnchorTable.from_paths`). Implementations MUST preserve replica-independent derivation,
agreement of `tree`, `passport`, and `fetch` on every path
([SPEC.md 5.3.1](../SPEC.md#531-idempotent-issuance)), and full recomputability from the path
set, so no issuance state exists to lose.
### D2. Restructuring changes the generation, never the mapping
`tree_generation` MUST change whenever issuance output would change for any path, and MAY
stay constant across content-only updates ([SPEC.md 5.5](../SPEC.md#55-tree-generation)).
Stability across restructuring is therefore (a) strict stability within a generation (P6)
plus (b) explicit migration across generations (D4). A publisher that restructures without
bumping the generation is non-conforming.
### D3. Error precedence: `409 GENERATION_STALE` vs `410 ANCHOR_EXPIRED`
| Caller supplied `tree_generation` | Anchor state | Status | Code | Retryable |
|---|---|---|---|---|
| yes, not current | not evaluated | 409 | `GENERATION_STALE` + `details.current` | yes |
| no / current | resolves | 200 | - | - |
| no | retired, alias in grace | 409 | `ANCHOR_MOVED` + `replaced_by`, `path` | yes |
| no | retired, alias expired | 410 | `ANCHOR_EXPIRED` | no |
| no | never issued / not authorized | 404 | `ANCHOR_NOT_FOUND` | no |

- `409 GENERATION_STALE` is about the caller's context: refresh from `orient`/`tree` and
  retry; the budget FSM releases the reservation first.
- `410 ANCHOR_EXPIRED` is about the world: refresh cannot revive it; re-navigate via `tree`.
- `404 ANCHOR_NOT_FOUND` stays uniform: never distinguish "never existed" from "not
  authorized" (SPEC.md 5.3.3, SPEC.md 8.3).

Ordering clarification (reading of [SPEC.md 6.10](../SPEC.md#610-validation-order-normative-for-all-methods)):
resolution is step 5, generation pre-validation step 7; read literally, a stale-generation
request could fail 404/410 at step 5 first, making `409 GENERATION_STALE` unreachable for
unknown anchors. Resolved here: an anchor has no meaning outside a generation, so when
resolution *fails* a supplied `tree_generation` is compared before 404/410 is reported and
the 409 branch fires for unknown anchors. When resolution *succeeds*, the request continues
through step 6 (filters/metrics) and is rejected with 409 at step 7. Both paths guarantee a
stale generation never yields a 200, while step 6 keeps its documented precedence over
step 7 - a bad filter is still reported to an agent holding a stale cache.

### D4. Alias and grace-period migration
On a move the publisher MAY retain `old_anchor -> {replaced_by, path}`: inside the grace
period answer `409 ANCHOR_MOVED` with `details.replaced_by` and `details.path`; after expiry,
or on deletion, answer `410 ANCHOR_EXPIRED`. The grace period is RECOMMENDED, not REQUIRED,
so publishers MAY answer 410 immediately and agents MUST handle both. The alias map is the
only permitted memory: bounded by removed nodes, never per-client, mandatory to expire, no
secrets ([SECURITY.md 5](../SECURITY.md#5-anchor-security)). Issuance never reads it (D1); only
retirement does.

### D5. Agent-side obligations
- On a new `ADC-Tree-Generation`, invalidate the local anchor table
  ([SPEC.md 5.3](../SPEC.md#53-anchors-generations-lifecycle)).
- Send `tree_generation` on POST `fetch` ([SPEC.md 6.4.5](../SPEC.md#645-heavy-filter-fetch-post-adcfetch))
  so a stale context yields the retryable 409 instead of a wrong answer.
- Tokens are unique only within a generation (SPEC.md 1) and rank issuance makes reuse
  systematic: `[a7]` in G+1 may be a different path than `[a7]` in G. Fetch responses echo
  `path`; on disagreement with the local table, treat it as a generation change (invalidate,
  re-orient) even when the header looks familiar.

## Alternatives considered
**A. Server-side anchor registry (durable row per node).** Rejected: (1) a single-writer
mutable store to mint anchors breaks replica and static-export agreement; (2) every
restructure adds rows and garbage collection becomes unowned protocol surface; (3) registry
loss destroys addressing while D1 recomputes from the path set; (4) per-request lookup is a
mutable existence oracle that strains the uniform-404 rule and moves `tree`/`passport` off
shared caches; (5) it does not remove the alias map D4 needs. The line ADC draws: issuance
reads nothing; only retirement may read a bounded, expiring alias map.

**B. Content-addressed anchors (hash of path or entity id).** Rejected: a path change changes
the hash, so restructuring stability is unsolved; the token embeds a path preimage; Annex B.1
fixes `anchor-id = "a" 1*DIGIT`.

**C. Never increment `tree_generation`.** Rejected: contradicts
[SPEC.md 5.3](../SPEC.md#53-anchors-generations-lifecycle), disables generation-keyed cache and
cursor invalidation, leaves deleted nodes addressable forever.

**D. Standalone migration document (`/.well-known/adc-anchors.json`).** Deferred: duplicates
what `details.replaced_by` already transports, adds a discovery surface, pays off only if
grace mappings outgrow an error body.

## Consequences (positive)
- `tree` and `passport` are reproducible from (generation, path set): static hosting,
  replicas, edge caching with `ETag: W"<tree_generation>"`
  ([SPEC.md 6.1](../SPEC.md#61-orient-get-adcorient)).
- The retryable split maps onto the budget FSM: 409 -> release, refresh, retry; 410 ->
  re-plan, no retry.
- Migration needs no permanent issuance state, only an expiring alias map.
- Agents get a machine-actionable refresh-vs-renavigate split, no prose parsing.

## Consequences (negative / known limits)
- Cross-generation token reuse is not rejected on the wire: a stale agent that omits
  `tree_generation` can silently address a different node; only D5's echoed-`path` comparison
  detects it.
- Dense rank issuance discloses catalog size: probing distinguishes `[a5000]` from `[a5001]`,
  leaking node count N even under uniform 404s. Accepted for 0.1; hiding it needs non-dense
  tokens, which Annex B.1 does not offer.
- The grace period is unenforceable (RECOMMENDED), so the same dead anchor yields 409 at one
  publisher and 410 at another.
- Alias retention is still state: rebuilding from scratch loses migration history and old
  anchors degrade to 410.
- No cross-origin migration: namespaced anchors die with their origin (SPEC.md 5.6), by
  design.

## Test vectors
Response bodies are asserted as field subsets; error bodies are `application/problem+json` (SPEC.md 6.12).

```json
[
  {
    "case": "issuance is idempotent across endpoints",
    "request": "GET /adc/tree?path=/electronics/audio&depth=1", "expect_status": 200,
    "expect_body": { "tree_generation": "2026-10-01T12:00:00Z", "nodes": [ { "name": "headphones", "path": "/electronics/audio/headphones", "anchor": "[a10]", "kind": "collection" } ] },
    "assert": "GET /adc/passport?path=/electronics/audio/headphones returns tree_generation 2026-10-01T12:00:00Z and anchor [a10]; /adc/fetch with the same path echoes [a10]"
  },
  {
    "case": "explicit stale tree_generation short-circuits before anchor liveness",
    "request": "GET /adc/fetch?anchor=%5Ba10%5D&tree_generation=2026-09-01T00:00:00Z", "expect_status": 409,
    "expect_body": { "type": "https://agentdatacircuit.org/errors/GENERATION_STALE", "status": 409, "retryable": true, "tree_generation": "2026-10-01T12:00:00Z", "details": { "current": "2026-10-01T12:00:00Z" } },
    "assert": "no 404/410 even though the caller's generation is retired; agent refreshes and retries"
  },
  {
    "case": "moved node inside the grace period",
    "request": "GET /adc/fetch?anchor=%5Ba1042%5D", "expect_status": 409,
    "expect_body": { "type": "https://agentdatacircuit.org/errors/ANCHOR_MOVED", "status": 409, "retryable": true, "details": { "replaced_by": "[a2087]", "path": "/electronics/audio/headphones/sony-wh1000xm5" } },
    "assert": "agent re-issues against details.replaced_by within the same generation"
  },
  {
    "case": "retired anchor after grace expiry",
    "request": "GET /adc/fetch?anchor=%5Ba1042%5D", "expect_status": 410,
    "expect_body": { "type": "https://agentdatacircuit.org/errors/ANCHOR_EXPIRED", "status": 410, "retryable": false, "details": { "path": "/electronics/audio/headphones/sony-wh1000xm5" } },
    "assert": "agent does not retry; it invalidates its table and re-navigates via /adc/tree"
  }
]
```

## References
- SPEC.md [5.3](../SPEC.md#53-anchors-generations-lifecycle), [5.3.1](../SPEC.md#531-idempotent-issuance), [5.3.3](../SPEC.md#533-lifecycle), [5.5](../SPEC.md#55-tree-generation)
- SPEC.md [6.10](../SPEC.md#610-validation-order-normative-for-all-methods), [CONFORMANCE.md F.4](../CONFORMANCE.md#f4-generation-invalidation)
- [SECURITY.md 5](../SECURITY.md#5-anchor-security), [agent-runtime/AGENT-RUNTIME.md 3.3](../agent-runtime/AGENT-RUNTIME.md#33-generation-invalidation)
- `reference/adc/anchors.py` (rank-based issuance, `AnchorTable`)
