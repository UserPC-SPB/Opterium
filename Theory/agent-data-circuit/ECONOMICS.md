# Agent Data Circuit (ADC): Economic Model

This document defines the economic mechanics for the Agent Data Circuit (ADC) protocol draft. It outlines how cost is declared, how agent budgets are managed, and how micro-billing operates. Note that ADC is an unreviewed draft with no production implementations. The economic incentives described here are theoretical mechanics, not proven market outcomes.

## 1. Precedence Model

Cost and data constraints in ADC follow a strict hierarchy of precedence. When multiple sources define a parameter, the highest-precedence source wins.

1.  **Manifest Policy**: The dataset manifest defines global constraints (e.g., total allowed limits, feature availability). This is the highest authority.
2.  **Passport Metadata**: The resource passport defines per-resource metadata (e.g., `list_item_tokens`, `list_item_bytes`). This overrides manifest defaults for specific resources.
3.  **Slice Override**: The specific slice definition can override certain parameters (e.g., dynamic pricing, degradation settings) but cannot exceed manifest-imposed hard limits.

**Precedence Rules:**
*   **Rule 1**: Manifest hard limits (e.g., max `limit` parameter) cannot be overridden by Passport or Slice.
*   **Rule 2**: Passport cost estimates (e.g., `list_item_tokens_p95`) are used for budget projection unless the Slice explicitly declares a different static cost.
*   **Rule 3**: Slice dynamic pricing parameters override Passport static costs only if the Manifest permits dynamic pricing for that resource type.

## 2. Cost Declaration Locations

Cost information is distributed across three layers of the protocol. All token counts use the tokenizer declared in the Manifest. Supported tokenizers are: `cl100k_base`, `o200k_base`, `utf8_bytes_div_4`, and `absolute-uri` (custom).

### Manifest
*   `limits`: Maximum allowed `limit` parameter for list endpoints.
*   `features`: Boolean flags indicating supported features (e.g., `micro_billing`, `dynamic_pricing`).

### Passport
*   `list_item_tokens`: Average token count per item.
*   `list_item_bytes`: Average byte count per item.
*   `list_item_tokens_p95`: 95th percentile token count per item, used for conservative budget projections.

### Slice
*   `tokens`: Estimated total tokens for the entire slice.
*   `bytes`: Estimated total bytes for the entire slice.
*   `price`: Numeric price value.
*   `currency`: ISO 4217 currency code.
*   `unit`: Billing unit (`per_call`, `per_item`, `per_megabyte`).
*   `dynamic`: Boolean indicating if price fluctuates.
*   `degradation.max_staleness_seconds`: Maximum acceptable age for degraded responses.

## 3. Agent Budget State Machine

Agents manage their budget using a finite state machine to prevent overrun and ensure atomicity.

### States
*   `IDLE`: No active request.
*   `ESTIMATING`: Calculating projected cost based on Passport metadata.
*   `EVALUATING`: Comparing projection against current budget.
*   `COMMITTED`: Budget reserved; request prepared.
*   `EXECUTING`: HTTP request in flight.
*   `RECONCILING`: Comparing actual cost (from response headers) against reserved budget.
*   `HALTED`: Budget insufficient; request aborted.
*   `HALTED_OVERRUN`: Actual cost exceeded reserved budget; session terminated.

### Projection Formula
The projected token cost is calculated as:

```
c_tokens = slice.tokens + (limit * list_item_tokens_p95)
```

### Admission Condition
The request is admitted only if:

```
c_tokens <= current_budget_remaining
```

### Atomic Reservation
If admitted, the agent atomically reserves `c_tokens` from its budget before sending the HTTP request. This prevents concurrent requests from double-spending the same budget.

### Reconciliation Formula
After receiving the response, the agent reconciles the budget:

```
budget_remaining = budget_remaining - actual_tokens
```

If `actual_tokens > c_tokens`, the state transitions to `HALTED_OVERRUN`.

### State Diagram

```
      +-------+   start    +------------+
      |       |----------->|            |
      |  IDLE |             | ESTIMATING |
      |       |<------------|            |
      +-------+   fail      +------------+
          |                    |
          | pass               | pass
          v                    v
      +-------+             +------------+
      |       |             |            |
      |COMMIT |             | EVALUATING |
      |       |<------------|            |
      +-------+   fail      +------------+
          |
          | reserve
          v
      +-------+
      |       |
      |EXECUT |
      |       |
      +-------+
          |
          | response
          v
      +-------+
      |       |
      |RECONC |
      +-------+
          |
          +--> [OK] --> IDLE
          |
          +--> [OVER] --> HALTED_OVERRUN
```

### Worked Example

Consider an agent with a remaining budget of 50,000 tokens. The agent requests a slice with the following parameters:
*   `slice.tokens`: 420
*   `limit`: 20
*   `list_item_tokens_p95`: 480

**Step 1: Projection**
```
c_tokens = 420 + (20 * 480)
c_tokens = 420 + 9600
c_tokens = 10020
```

**Step 2: Admission**
```
10020 <= 50000
```
Admission passes. Agent reserves 10,020 tokens.

**Step 3: Execution**
The agent sends the request. The server returns a response with `ADC-Cost-Actual-Tokens: 9300`.

**Step 4: Reconciliation**
```
budget_remaining = 50000 - 9300
budget_remaining = 40700
```

Since 9300 <= 10020, the state transitions back to `IDLE` with 40,700 tokens remaining.

```text
Budget State Trace:
1. IDLE: balance=50000
2. ESTIMATING: projection=10020
3. EVALUATING: 10020 <= 50000 (Pass)
4. COMMITTED: reserved=10020, balance=39980 (virtual)
5. EXECUTING: request sent
6. RECONCILING: actual=9300
7. IDLE: balance=40700
```

## 4. Why Cost Signals Change Agent Behavior

Explicit cost signals allow agents to optimize for efficiency rather than relying on "download and see" strategies.

*   **Choose a Cheaper Slice**: If a slice is priced high, the agent may select an alternative slice with lower cost.
*   **Narrow a Filter**: The agent may apply stricter filters to reduce the number of items returned, lowering the projected `c_tokens`.
*   **Lower Limit**: The agent may reduce the `limit` parameter to fit within budget constraints.
*   **Call Aggregate**: Instead of fetching a full collection, the agent may call an `aggregate` endpoint to get summary statistics, which typically has a much lower `slice.tokens` value.

This contrasts with traditional APIs where agents must fetch data to determine its size and cost, leading to wasted bandwidth and compute.

## 5. Micro-Billing

ADC supports micro-billing through specific response headers and error codes.

### Billing Units
*   `per_call`: Fixed cost per API call.
*   `per_item`: Cost multiplied by the number of items returned.
*   `per_megabyte`: Cost multiplied by the data size in MB.

### Required Response Headers
*   `ADC-Cost-Amount`: Numeric cost amount.
*   `ADC-Cost-Currency`: ISO 4217 currency code.
*   `ADC-Cost-Unit`: Billing unit used.

### 402 BUDGET_EXCEEDED
If the agent's budget is insufficient, the server may return `402 Payment Required` with the body:
```json
{
  "error": "BUDGET_EXCEEDED",
  "current_balance": 1000,
  "required_balance": 1500
}
```

### Degraded Responses
Degraded responses (e.g., stale data) are free. The `ADC-Cost-Amount` header will be `0`.

## 6. Caching Economics

Caching behavior is tied to cost and data freshness.

*   **Dynamic Slices**: Must send `Cache-Control: no-store`. Dynamic pricing prevents safe caching.
*   **Static Slices**: May use `Cache-Control: public` only if the cost is zero. If cost is non-zero, caching is discouraged to avoid repeated billing disputes.
*   **CDN Cache Hits**: If a request is served from a CDN cache, the reported cost is zero.
*   **GET Aggregate**: Aggregate endpoints are cacheable. The `ETag` should be derived from the dataset generation timestamp to ensure invalidation when data changes.

## 7. Tokenizer Caveat

Token counts in ADC are approximate proxies for real model cost. They are not exact measures of compute consumption.

*   **Custom Tokenizers**: Publishers may declare a custom tokenizer via an `absolute-uri`.
*   **Comparability**: Cross-vendor comparability is not guaranteed. A token count from `cl100k_base` may not equal a token count from a custom model tokenizer.
*   **Open Question**: How to standardize tokenizer output for cross-vendor budgeting is an open research question.

## 8. Unproven Economic Claims

The following claims are not proven and remain open questions:

1.  **Publisher Adoption**: Whether publishers will accept the accounting overhead of declaring `list_item_tokens_p95` and maintaining cost metadata.
2.  **Micro-Billing Adoption**: Whether micro-billing will see real adoption in the agent ecosystem, or if flat-rate subscriptions will dominate.
3.  **Token-Proxy Accuracy**: Whether token-proxy pricing reflects true compute cost, or if it will lead to systematic under- or over-pricing.

This document defines the mechanics of ADC economics, not proven market incentives. The economic model is a draft and requires empirical validation.