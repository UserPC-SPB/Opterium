# Agent Data Circuit (ADC) Protocol Draft: Conformance

**Status:** DRAFT
**Implementation Status:** No production implementations exist. This document defines normative requirements for the Agent Data Circuit (ADC) protocol.

This document specifies the conformance levels, requirements matrix, and normative test vectors for the ADC protocol. It is intended for implementers, validators, and auditors.

## 1. Conformance Levels

Conformance is cumulative. A system declaring a higher level implies conformance to all lower levels.

| Level | Name | Description |
| :--- | :--- | :--- |
| 1 | Core | Mandatory baseline. Includes manifest, navigation, passports, fetch, and error handling. |
| 2 | Search | Adds search index, grep, and JSON pointer capabilities. |
| 3 | Dynamic | Adds dynamic slices, hydration maps, and degradation handling. |
| 4 | Commerce | Adds billing invariants, POST fetch, aggregates, and privacy-safe totals. |
| 5 | Trust | Adds signing (JWS/JWKS), SLA assertions, and advanced privacy suppression. |

**Rule:** A system MUST declare `Core` explicitly. Declaring `Search` implies `Core`. Declaring `Dynamic` implies `Core` and `Search`, and so on.

## 2. Requirements Matrix

The following matrix defines the normative requirements for each capability across conformance levels.

- **MUST**: The feature is mandatory for conformance at this level.
- **SHOULD**: The feature is recommended but not strictly mandatory for basic conformance (subject to specific context).
- **-**: Not required at this level.

| Capability | Core | Search | Dynamic | Commerce | Trust |
| :--- | :---: | :---: | :---: | :---: | :---: |
| Manifest at `/.well-known/adc.json` | MUST | MUST | MUST | MUST | MUST |
| Navigation tree | MUST | MUST | MUST | MUST | MUST |
| Passports | MUST | MUST | MUST | MUST | MUST |
| GET fetch | MUST | MUST | MUST | MUST | MUST |
| Tokenizer declaration | MUST | MUST | MUST | MUST | MUST |
| RFC 9457 problem details | MUST | MUST | MUST | MUST | MUST |
| Anchor invariant enforcement | MUST | MUST | MUST | MUST | MUST |
| URI fragment namespacing | MUST | MUST | MUST | MUST | MUST |
| Search index | - | MUST | MUST | MUST | MUST |
| Grep with JSON pointers | - | MUST | MUST | MUST | MUST |
| Dynamic slices + per-slice hydration map | - | - | MUST | MUST | MUST |
| Explicit degradation handling | - | - | MUST | MUST | MUST |
| Billing degradation invariants | - | - | - | MUST | MUST |
| POST fetch | - | - | - | MUST | MUST |
| Aggregate endpoint | - | - | - | MUST | MUST |
| Privacy-safe totals | - | - | - | MUST | MUST |
| Estimate (MUST at Commerce) | - | - | - | MUST | MUST |
| k-anonymity group suppression | - | - | - | - | MUST |
| JWS single-item + collection signing | - | - | - | - | MUST |
| JWKS with retention | - | - | - | - | MUST |
| SLA assertion | - | - | - | - | MUST |

## 3. Conformance Test Vectors

The following normative test vectors (F.1–F.14) define the specific behaviors an implementation must exhibit to claim conformance. These are requirements, not results of passed experiments. Implementations must be able to execute these vectors to verify correctness.

### F.1 Anchor Arithmetic
1. **Rollback beyond depth**: Attempting to rollback an anchor to a depth greater than its current depth MUST result in an error.
2. **Rollback to root**: Rolling back to the root anchor MUST succeed and return the root anchor state.
3. **Rollback zero invalid**: A rollback request with a delta of zero MUST be rejected as invalid.
4. **Suffix ambiguity**: If a suffix resolves to multiple anchors, the request MUST fail with an ambiguity error.

### F.2 k-Anonymity Differencing
Given groups with sizes A=10, B=2, and k=5:
1. Group A (size 10) MUST be emitted in the response.
2. Group B (size 2) MUST be suppressed because size < k.
3. The response MUST include `suppressed_groups=1`.
4. Totals MUST reflect only visible groups (or be omitted if no groups are visible).

### F.4 Generation Invalidation
1. **Tree generation change**: If the tree generation changes between request and processing, the response MUST be `409 Conflict` with `GENERATION_STALE` and `details.current` reflecting the new generation.
2. **Anchor moved**: If the anchor has been moved, the response MUST be `409 Conflict` with `ANCHOR_MOVED` and `details.replaced_by` indicating the new location.
3. **Anchor expired**: If the anchor has expired, the response MUST be `410 Gone`.
4. **Cursor expired**: If the cursor has expired, the response MUST be `410 Gone`.

### F.5 Filter Grammar
1. **GET `in` operator**: Repeated query parameters MUST be interpreted as the `in` operator.
2. **`contains_all`**: The `contains_all` operator MUST support checking if a set contains all specified values.
3. **POST boolean logic**: POST requests MUST support `and`, `or`, and `not` operators.
4. **Unknown field**: Filtering on an unknown field MUST result in `400 Bad Request` with `FILTER_INVALID` and `details.unknown_fields`.
5. **Unsupported op**: Using an unsupported operator MUST result in `400 Bad Request` with `details.unsupported_ops`.

### F.6 Aggregate Privacy
1. **Metric aliases**: Aggregates MUST support aliasing metrics.
2. **Having clause**: Aggregates MUST support `having` clauses for post-aggregation filtering.
3. **Ordering tie-break**: Ordering MUST include a deterministic tie-breaker.
4. **Min group size floor**: If a group size falls below the configured floor, the aggregation MUST reject the request with `details.privacy_floor`.
5. **Approximate false**: If `approximate=false` is specified, strict calculation MUST be enforced.
6. **Ungrouped shape**: If no groups are specified, the response MUST return top-level values and no `groups` array.

### F.7 Fetch Polymorphism
1. **Collection shape**: MUST include `parent_anchor`, `items`, `cursor`, and `truncated`. MUST NOT include top-level entity fields.
2. **Entity shape**: MUST include top-level `anchor`, `path`, `slice`, `generation`, and `fields`. MUST NOT include `items` or `parent_anchor`.
3. **Default slice**: If no slice is specified, the default slice MUST be used.
4. **Limit cap**: The `limit` parameter MUST be capped at the server-defined maximum.
5. **Path on GET**: The `path` parameter MUST be accepted on GET requests.
6. **Order by deterministic**: The `order_by` parameter MUST produce deterministic results.

### F.8 Hydration, Degradation, and Billing
1. **Per-slice map**: The hydration map MUST define outcomes for each slice (e.g., `fresh`, `stale`).
2. **Staleness enforcement**: Stale slices MUST be handled according to the degradation policy.
3. **Billing zero on degraded**: If a slice is degraded, the billing cost for that slice MUST be zero.
4. **Partial omit**: If a slice is omitted due to degradation, the response MUST reflect this via `partial_omit`.
5. **Fail closed**: If the system cannot serve the request safely, it MUST return `503 Service Unavailable` with a `Retry-After` header.

### F.9 Budget FSM
1. **Concurrent reservation**: In a race condition, exactly one concurrent reservation MUST succeed.
2. **Release on failure**: If a request fails, the reserved budget MUST be released.
3. **Overrun**: If the budget is exceeded, the state MUST transition to `HALTED_OVERRUN`.
4. **Reconciliation identity**: The sum of reserved and used budgets MUST equal the total budget minus released amounts.

### F.10 Caching Invariants
1. **Billed GET private**: Billed GET responses MUST include `Cache-Control: private, no-store`.
2. **Dynamic no-store**: Dynamic content MUST include `Cache-Control: no-store`.
3. **CDN copy zero cost**: Cached copies served from CDN MUST have zero billing cost.
4. **Vary headers**: Responses MUST include appropriate `Vary` headers for conditional caching.
5. **POST no-store no ETag**: POST responses MUST include `Cache-Control: no-store` and MUST NOT include an `ETag`.

### F.11 Error Model
1. **RFC 9457 shape**: All errors MUST conform to RFC 9457 Problem Details.
2. **Status mapping**: Status codes MUST map correctly to error types.
3. **402 details**: Payment Required responses MUST include specific details.
4. **429 Retry-After**: Too Many Requests responses MUST include a `Retry-After` header.
5. **Uniform 401 denial**: Unauthorized requests MUST return a uniform `401 Unauthorized` response.

### F.12 Tokenizer
1. **Manifest requirement**: If tokens are declared in the manifest, a tokenizer MUST be specified.
2. **UTF-8 identity**: `usage.tokens` MUST equal `ceil(utf8_bytes / 4)` for the identity tokenizer.
3. **Estimate equality**: If confidence is exact, `estimate` MUST equal `usage`.

### F.13 Grep Pointers and ReDoS
1. **Pointer correctness**: Grep results MUST return correct JSON pointers.
2. **Regex gating**: Regular expressions MUST be gated to prevent catastrophic backtracking.
3. **Linear-time requirement**: Grep operations MUST complete in linear time relative to input size.

### F.14 Namespacing
1. **Collision**: Colliding namespace names MUST be rejected.
2. **Wire stripping**: Namespace prefixes MUST be stripped from the wire format.
3. **Mixed namespace**: Mixing namespaces in a single request MUST fail locally with `MIXED_NAMESPACE`.

## 4. How to Claim Conformance

To claim conformance to the ADC protocol:

1. **Run the validator**: Execute the bundled reference implementation and test suite against the specified vectors (F.1–F.14).
2. **Publish the manifest**: Include the conformance level and results in the `/.well-known/adc.json` manifest.
3. **State independence**: Clearly state that independent implementations are required before any standardization claim can be made.

**Standing Rule:**
At least two independent conforming implementations MUST pass the same test vectors before a conformance claim is considered valid for standardization purposes.

**Note:** The bundled reference implementation and test suite exist to execute these vectors. No vectors are claimed to currently pass in this draft. This document defines the requirements; verification is the responsibility of the implementer.