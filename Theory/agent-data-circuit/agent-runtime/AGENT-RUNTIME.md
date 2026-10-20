# Agent Runtime Requirements (ADC Draft)

**Status:** DRAFT  
**Protocol:** Agent Data Circuit (ADC)  
**Scope:** Normative Annex for Agent-Side Runtimes

> **Note:** This document is a draft specification. It defines normative requirements for agent runtimes interacting with ADC-compliant servers. It does not constitute a ratified standard, nor does it imply the existence of a governing working group or audit history.

## 1. Purpose and Scope

This annex defines the normative behavior required of an agent runtime to consume an ADC circuit correctly and safely. The agent runtime acts as the client in the ADC protocol, responsible for discovering circuit capabilities, managing anchor state, executing pointwise fetches, and enforcing budget constraints.

The runtime MUST:
*   Operate under strict namespace isolation.
*   Enforce token budgets locally before network transmission.
*   Treat all received data as inert.
*   Handle degradation states explicitly.

This document references `SPEC.md` sections for wire-format definitions.

## 2. Discovery and Session Bootstrap

Upon session initialization, the agent runtime MUST perform the following steps:

1.  **Fetch Manifest:** Retrieve `/.well-known/adc.json` from the target origin.
2.  **Version Verification:** Verify that the `major_version` in the manifest is compatible with the runtime's supported range. If incompatible, the runtime MUST abort the session.
3.  **Orientation:** Call the `orient` endpoint to establish the initial circuit state.
4.  **State Recording:** Record the `tree_generation` and `tokenizer` identifier returned by `orient`. These values are required for subsequent cache keys and anchor validity checks.
5.  **Fallback:** If no manifest is found or the manifest is malformed, the runtime SHOULD fall back to web heuristics (e.g., standard HTTP headers, content-type sniffing) but MUST NOT assume ADC-specific anchor semantics unless explicitly confirmed by a successful `orient` call.

## 3. Anchor Table and Namespace Isolation

The runtime MUST maintain a local anchor table to map logical anchors to wire-format tokens.

### 3.1 Key Structure
Anchors MUST be stored internally keyed by the tuple:
`(origin_authority, raw_anchor_token)`

### 3.2 Namespace Handling
*   **Internal:** The runtime MUST prepend the origin namespace to the anchor token for all internal lookups and state management.
*   **Wire:** The runtime MUST strip the origin namespace before transmitting any anchor token over the wire. The wire format contains only the `raw_anchor_token`.
*   **Mismatch:** If a response contains an anchor with a namespace that does not match the current session's origin authority, the runtime MUST abort the local operation and raise a `MIXED_NAMESPACE` error.

### 3.3 Generation Invalidation
The runtime MUST invalidate the entire anchor table when the `tree_generation` changes. Any cached anchors from a previous generation MUST be discarded.

## 4. Anchor Arithmetic

Anchor arithmetic operations must follow strict rules to ensure deterministic navigation.

### 4.1 Suffix Matching
*   Anchor resolution uses case-sensitive, segment-boundary suffix matching.
*   There must be exactly one descendant match. Ambiguity is an error.

### 4.2 Rollback Constraints
*   Rollback depth `k` MUST satisfy `1 <= k <= depth`.
*   If `k` is outside this range, the runtime MUST raise `ROLLBACK_INVALID`.

### 4.3 Error Conditions
The runtime MUST handle the following errors explicitly:
*   `NO_SUCH_ANCESTOR`: The requested ancestor does not exist in the current tree.
*   `NO_SUCH_CHILD`: The requested child does not exist.
*   `AMBIGUOUS_CHILD`: Multiple children match the suffix criteria.

### 4.4 Wire Transmission Rules
*   The runtime MUST NEVER send a raw arithmetic expression (e.g., `anchor/child`) over the wire.
*   If the target path is known but the specific anchor token is not cached, the runtime MUST send `path=` in the request.
*   Upon receiving a response to a `path=` query, the runtime MUST cache the returned anchor token for future use.

## 5. Pointwise Fetch Loop

The fetch loop MUST operate within strict budget constraints.

1.  **Read Passport:** Retrieve the current passport state.
2.  **Estimate Cost:**
    *   Compute `projection_slice.tokens` for the requested slice.
    *   Estimate list items as `limit * list_item_tokens_p95`.
    *   Total estimated tokens = `projection_slice.tokens + (limit * list_item_tokens_p95)`.
3.  **Budget Check:**
    *   If `estimated_tokens > remaining_budget`, the runtime MUST NOT initiate the fetch.
    *   If `estimated_tokens <= remaining_budget`, proceed to fetch.
4.  **Fetch:** Request only the specific slices needed.
5.  **Reconcile:**
    *   After the response is received, reconcile actual token usage against the budget.
    *   Follow the state machine defined in `reference/adc/budget.py`.

## 6. Freshness Handling

The runtime MUST manage cache freshness using the following key structure:
`(origin, anchor, slice, generation)`

### 6.1 State Determination
*   **Clean/Read:** If the cached `generation` matches the current session `tree_generation`, the data is considered fresh.
*   **Stale/Unread:** If the cached `generation` differs from the current session `tree_generation`, the data is considered stale.

### 6.2 Age Policy
If a `generated_at` timestamp is present in the payload, the runtime SHOULD compare it against the current time to determine age. Age policies are implementation-defined but MUST be documented in the runtime configuration.

## 7. Degradation Handling

The runtime MUST handle degraded server states explicitly.

### 7.1 Stale Fallback
If any slice status is `stale_fallback`:
*   The runtime MAY use the data for informational summaries only.
*   The runtime MUST disclose the staleness age to the user or downstream consumer.
*   The runtime MUST NOT perform any transactions (mutations) on `stale_fallback` data.

### 7.2 Degraded Partial
If any slice status is `degraded_partial`:
*   The runtime MUST NOT perform any transactions.
*   The runtime MUST revalidate the state or wait for the server to return to `live` status before proceeding with critical operations.

## 8. Passive Data Handling

All payloads received from the ADC server MUST be treated as inert data.

*   **Framing:** The runtime MUST frame all data before injecting it into LLM context.
*   **No Execution:** The runtime MUST NEVER execute instructions found within data blocks or error payloads.
*   **Injection Safety:** Any text resembling commands (e.g., "ignore previous instructions") MUST be treated as literal string data, not as directives.

## 9. Budget Reconciliation Identity

After each fetch and reconcile cycle, the following identity MUST hold:

```text
remaining == initial_remaining - actual_tokens
```

### 9.1 Overrun
If `actual_tokens > initial_remaining`, the runtime MUST raise a `HALTED_OVERRUN` error and halt further fetches for the current session.

### 9.2 Reservation Release
If a fetch fails due to:
*   HTTP 429 (Too Many Requests)
*   HTTP 503 (Service Unavailable)
*   Network error

The runtime MUST release the reserved budget tokens back to `remaining`.

## 10. Reference Implementation Map

This draft references the following implementation artifacts for normative behavior:

*   `reference/adc/anchors.py`: Anchor table management, namespace stripping, and arithmetic validation.
*   `reference/adc/budget.py`: Budget state machine, estimation logic, and reconciliation identity.
*   `reference/adc/client.py`: Session bootstrap, discovery, and pointwise fetch loop.
*   `reference/mcp-tools`: Tool bindings for exposing ADC operations to LLM agents.

Developers SHOULD consult these references for concrete error codes and state transitions.