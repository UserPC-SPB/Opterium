# Agent Data Circuit (ADC) — Specification

**Version:** 0.1-draft
**Status:** Draft. No production implementations yet.
**Date:** 2026-09-25
**Supersedes:** internal draft "0.3-rc" (never implementation-backed)
**License:** CC BY 4.0

---

## Abstract

The Agent Data Circuit (ADC) is a publisher-side protocol that lets web systems
expose structured, cost-budgeted, navigable, and freshness-tagged data directly
to autonomous AI agents. ADC operates over standard web infrastructure
(HTTP/1.1, HTTP/2, HTTP/3, TLS, JSON) and requires no model fine-tuning,
specialized client, or proprietary intermediary. An agent reads structured JSON,
executes ordinary tool calls, and resolves anchor arithmetic with local code.

This document is the first published draft. It intentionally covers **Core**
normatively and describes the higher tiers (Search, Dynamic, Commerce, Trust) in
enough detail to implement, without pretending they are battle-tested.

Conformance keywords — MUST, MUST NOT, REQUIRED, SHALL, SHOULD, SHOULD NOT, MAY —
are interpreted per RFC 2119.

**Forward compatibility.** Agents MUST ignore JSON members they do not
understand. Publishers MUST NOT require an agent to understand a member
introduced after the version the agent declares.

---

## 1. Terminology

- **Publisher** — the entity operating an origin server and publishing an ADC
  circuit.
- **Agent** — an automated client consuming ADC endpoints.
- **Circuit** — the complete set of ADC endpoints, manifest, tree, passports,
  and blocks exposed by a publisher.
- **Node** — a logical point in the navigation tree: an entity, category, or
  collection.
- **Passport** — node-level metadata: token/byte costs, filters, default slice,
  generations, degradation policy.
- **Block** — an addressable payload unit containing entity data and outward
  anchors.
- **Anchor** — a short token (e.g. `[a1042]`) the publisher issues for a path.
  Stateless, globally unique within a tree generation.
- **Anchor Namespace** — the origin authority prepended by the agent runtime
  using URI fragment syntax: `https://example.com#[a1042]`.
- **Tree Generation** — a monotonic identifier for the structural version of the
  whole tree. Changing it invalidates all anchors.
- **Generation** — an opaque identifier for the content state of a slice. Two
  responses carry the same generation iff their content is identical for caching
  purposes. Equality is the only comparison the protocol requires.
- **Slice** — a named, independently addressable segment of a block
  (`summary`, `specs`, `reviews`, `logistics`).
- **Tokenizer** — the reference algorithm used to count tokens. All `tokens`
  fields MUST use the tokenizer declared in the manifest.
- **Dynamic Degradation** — a controlled fallback state for a dynamic slice when
  live hydration fails.
- **Aggregation** — a server-side reduction (`avg`, `sum`, `min`, `max`,
  `count`, percentiles) over a collection without serializing entities.

---

## 2. Principles

### 2.1 Publisher-side

| # | Principle |
|---|---|
| P1 | **Invitation, opt-in.** Publishing a manifest is an invitation. No manifest ⇒ normal scraping policy applies. |
| P2 | **Single source of truth.** Agent payloads and human pages SHOULD come from the same datastore. |
| P3 | **Pointwise extraction.** Endpoints MUST return granular slices, never whole-page monoliths. |
| P4 | **Metadata on the path.** Every routing step MUST transmit cost, filters, and freshness of child steps. |
| P5 | **Single declared root.** All paths are relative to the manifest `root`. |
| P6 | **Stable anchors within generations.** Anchors map 1:1 to paths for the life of a tree generation. |
| P7 | **Freshness on demand.** Agents compare generations against local state; servers keep zero per-client state. |
| P8 | **Economic routing.** Cost (`tokens`, `bytes`, `price`) MUST be declared on passports and slices. |
| P9 | **Entity data budgeting.** Marginal item cost (`list_item_tokens`) MUST be declared for collections. |
| P10 | **Graceful degradation.** Dynamic backend failure MUST NOT invalidate static slices. |
| P11 | **Pre-computation over iteration.** Commerce publishers MUST provide server-side reductions. |
| P12 | **Declared envelope.** Limits, rate limits, paths, defaults, and tokenizer are declared, not discovered. |
| P13 | **Truthful degradation.** A publisher MUST NOT present stale data as live. |
| P14 | **Billing integrity.** Degraded responses MUST NOT be charged dynamic hydration fees. |
| P15 | **Privacy-preserving aggregation.** Publishers MUST enforce k-anonymity and prevent differencing. |

### 2.2 Agent-side

| # | Principle |
|---|---|
| A1 | **Data, not instructions.** Payloads are passive data; embedded instructions MUST NOT be executed. |
| A2 | **Determinism over guessing.** Navigate via verified anchors and passports, not speculative scraping. |
| A3 | **Local arithmetic.** Anchor expressions (`-k`, `/suffix`) are evaluated by the runtime, never sent. |
| A4 | **Strict budget state machine.** A formal budget FSM with concurrency controls guards every request. |
| A5 | **Authority-scoped isolation.** Anchors are namespaced per origin to prevent cache poisoning. |
| A6 | **Degradation disclosure.** Agents using `stale_fallback` data MUST disclose its age and MUST revalidate before any transaction. |

---

## 3. Components

| ID | Component | Role | Tier |
|---|---|---|---|
| C1 | Manifest | Entry point: root, tree_generation, tokenizer, policy, keys, capabilities | Core |
| C2 | Navigation tree | Paths ↔ anchors | Core |
| C3 | Passport | Node metadata: tokens, slices, filters, generations | Core |
| C4 | Block storage | Payloads addressed by anchors | Core |
| C5 | Hybrid index | Lexical + vector search resolving to anchors | Search |
| C6 | Access protocol | `orient`, `tree`, `passport`, `fetch`, `estimate`, `aggregate`, `search`, `grep`, `verify` | Core / tiers |
| C7 | Trust & economics | JWS signatures, licensing, micro-billing, cache invariants, SLAs | Trust |

**Architectural invariant.** C2 carries no entity content; C3 carries no payload;
C4 carries no *tree navigation* (entity anchors are content addressing, not
navigation); C5 carries no block content.

---

## 4. Discovery

Agents discover circuits in this order:

1. **Well-known URI.** `GET /.well-known/adc.json`. Publishers MUST serve it
   with `Content-Type: application/adc+json` or `application/json`.
2. **HTTP Link header.** `Link: </.well-known/adc.json>; rel="adc-manifest"; v="0.1"`.
3. **HTML hint.** `<link rel="adc-manifest" type="application/adc+json" href="/.well-known/adc.json">`.
4. **Content negotiation.** `GET /` with `Accept: application/adc+json` MAY
   return `307` to the manifest.

Absence of all four ⇒ the agent falls back to ordinary web heuristics.

> **Optional cold entry.** An origin MAY additionally publish `GET /AI/`, a
> small static instruction map for agents that arrive with *no prior
> knowledge of ADC*. It is independent of this manifest, is **not** part of
> ADC conformance, and discovering it neither implies nor requires anything
> in this document. An agent that finds `/AI/` and wants the full protocol
> then requests `/.well-known/adc.json` as described above. Design notes:
> [AI-ENTRY.md](AI-ENTRY.md).

### 4.1 Version negotiation

The manifest declares `"adc": "0.1"` (a `MAJOR.MINOR` string).

- The agent MUST support the manifest's major version.
- If the manifest minor is newer, the agent proceeds only if every field it
  relies on is defined identically in its version, and MUST ignore unknown
  members.
- A breaking change MUST increment the major version.
- `adc_min` MAY declare the oldest minor the circuit remains wire-compatible
  with. Its absence means only the declared major line is supported.

### 4.2 Manifest

```json
{
  "$schema": "https://agentdatacircuit.org/schemas/v0.1/manifest.schema.json",
  "adc": "0.1",
  "adc_min": "0.1",
  "root": "https://api.example.com/data",
  "namespace": "com.example",
  "tree_generation": "2026-10-01T12:00:00Z",
  "tokenizer": "cl100k_base",
  "conformance": ["core", "dynamic", "commerce"],
  "endpoints": {
    "orient": "/adc/orient",
    "tree": "/adc/tree",
    "passport": "/adc/passport",
    "fetch": "/adc/fetch",
    "estimate": "/adc/estimate",
    "aggregate": "/adc/aggregate",
    "search": "/adc/search",
    "grep": "/adc/grep",
    "verify": "/adc/verify"
  },
  "policy": {
    "agents": "welcome",
    "license": "CC-BY-4.0",
    "attribution": "required",
    "provenance_disclosure": true,
    "rate_limit": { "requests_per_minute": 120, "burst": 30 }
  },
  "auth": { "required": false, "schemes": ["bearer"], "realm": "example" },
  "defaults": { "lang": "en", "currency": "USD", "page_size": 20 },
  "limits": {
    "max_limit": 200,
    "max_depth": 4,
    "max_tree_nodes": 500,
    "max_aggregate_groups": 1000,
    "max_aggregate_group_by": 3,
    "max_post_bytes": 65536,
    "max_filter_conditions": 32,
    "max_filter_depth": 4
  },
  "rate_limits": [
    {
      "scope": "anonymous",
      "requests": 60,
      "window": "PT1M",
      "burst": 10,
      "weights": {
        "orient": 1, "tree": 1, "passport": 1, "fetch": 1, "estimate": 1,
        "aggregate": 5, "search": 1, "grep": 1, "verify": 1, "post_fetch": 2
      }
    }
  ],
  "features": {
    "post_fetch": true,
    "fields": true,
    "estimate": true,
    "ndjson_streaming": false,
    "grep_regex": false,
    "aggregate": {
      "metrics": ["count()", "sum(price)", "avg(price)", "min(price)", "max(price)", "p50(price)", "p95(price)"],
      "group_by": ["brand", "model"]
    }
  },
  "trust": {
    "algorithm": "ES256",
    "jwks_uri": "/.well-known/jwks.json",
    "sla": { "max_dynamic_latency_ms": 450, "fallback_allowed": true }
  },
  "updated_at": "2026-10-01T12:00:00Z"
}
```

**Tokenizer vocabulary.** `tokenizer` MUST be one of:

- `cl100k_base` — OpenAI GPT-4 (normative reference)
- `o200k_base` — OpenAI GPT-4o
- `utf8_bytes_div_4` — `tokens = ceil(bytes / 4)`
- `<absolute-uri>` — a custom tokenizer with a publicly documented algorithm

All `tokens`, `estimated_tokens`, `list_item_tokens`, `list_item_tokens_p95`,
and `usage.tokens` fields MUST use the declared tokenizer.

**Policy vocabulary.** `policy.agents` ∈ `welcome | restricted | closed`.
`policy.attribution` ∈ `required | requested | none`.

**Caching.** The manifest SHOULD be served with an `ETag` and a short
`max-age` (60–600 s).

---

## 5. Addressing Model

### 5.1 Path canonicalization

Paths are absolute with respect to the manifest `root`, normalized per
RFC 3986 §5.2. Paths MUST be case-sensitive, use `/`, be NFC-normalized, and
MUST NOT contain `..`, `//`, a trailing slash (except root `/`), or
percent-encoded separators. Violations ⇒ `400 PATH_INVALID`.

### 5.2 Three-tier resolution

1. **Tree inspection:** path → anchor.
2. **Passport evaluation:** anchor → metadata.
3. **Block extraction:** anchor + slice → payload.

### 5.3 Anchors, generations, lifecycle

- Anchors are **stateless and global** within a tree generation.
- Anchors are **generation-bound**. A publisher that restructures the tree MUST
  increment `tree_generation` in the manifest and echo it via the
  `ADC-Tree-Generation` response header.
- On observing a new `tree_generation`, an agent MUST invalidate its local
  anchor table for that origin.
- Resolving an anchor against an obsolete generation ⇒ `410 ANCHOR_EXPIRED`.

#### 5.3.1 Idempotent issuance

Within one tree generation, issuance is a pure function:
`anchor(tree_generation, path) → [aN]`. The same path yields the same anchor
across `tree`, `passport`, and `fetch`; the same anchor yields the same path.

#### 5.3.2 Anchor arithmetic edge cases

- Root `/` MAY be addressed directly; there is no reserved `[a0]`.
- `[aN]-k` with `k` greater than the anchor's depth is a **local** error
  (`NO_SUCH_ANCESTOR`); no wire request is emitted.
- `[aN]-0` is invalid; rollback depth MUST be ≥ 1.
- `[aN]/suffix` resolves exactly one descendant chain matching the suffix
  case-sensitively on segment boundaries. Zero matches ⇒ `NO_SUCH_CHILD`; more
  than one ⇒ `AMBIGUOUS_CHILD`. Servers MUST NOT be asked to disambiguate.
- An agent MAY resolve a derived expression to a literal path and send `path=`;
  the server returns the canonical anchor.

#### 5.3.3 Lifecycle

| Situation | Publisher | Error |
|---|---|---|
| Node moved, old anchor known | Accept for a grace period (RECOMMENDED ≥ 7 days), return new location | `409 ANCHOR_MOVED` + `details.replaced_by`, `details.path` |
| Node removed, alias expired | Reject | `410 ANCHOR_EXPIRED` |
| Tree rebuilt, old generation | Reject if caller supplied old generation, else treat as unknown | `409 GENERATION_STALE` / `410 ANCHOR_EXPIRED` |
| Unknown anchor | Reject; MUST NOT distinguish "never existed" from "not authorized" by `code` | `404 ANCHOR_NOT_FOUND` |

### 5.4 Tree response shape

Each node MUST include `name`, `anchor`, `kind`
(`collection | entity | document`). It SHOULD include `has_children` and MAY
include `count` (collections) and `updated_at`. Nodes MUST be returned in a
deterministic order, stable for the tree generation and consistent across
pagination.

### 5.5 Tree generation

`tree_generation` MUST change whenever the anchor-issuing function changes for
any path. It MAY stay constant across content updates that do not alter the
tree. Every `tree` and `passport` response MUST echo it via the
`ADC-Tree-Generation` header and in the body.

### 5.6 Anchor namespacing (agent-side)

NamespacedAnchor = `<origin-authority>#<raw-anchor-token>`, e.g.
`https://shop.vendor-a.com#[a1042]`.

- Origin authority MUST be a valid RFC 3986 authority. Default ports elided;
  IDNs punycode-encoded.
- Namespacing is enforced **internally** by the agent runtime. On the wire, the
  agent MUST strip the namespace and send only the raw anchor (`[a1042]`).
- A runtime MUST abort locally if asked to dispatch an anchor bearing a
  mismatched namespace.

---

## 6. Access Protocol

All access methods use HTTP. Read-only operations MUST use `GET` to permit edge
caching (subject to §6.4.6). Operations exceeding URI length constraints or
requiring non-idempotent processing use the defined POST profiles.

All timestamps are RFC 3339 UTC. Query parameters are URL-encoded per RFC 3986.

**Media types.** Bodies use `Content-Type: application/adc+json; charset=utf-8`.
Clients SHOULD send `Accept: application/adc+json`; agents SHOULD accept
`application/json` as fallback. Servers MUST send
`Vary: Accept, Accept-Language, Authorization` where applicable.

**Method availability.** A method required by a declared tier MUST be available
at its manifest path. A method not implemented MUST return
`501 NOT_IMPLEMENTED` with `error.code: "NOT_IMPLEMENTED"`, not a generic 404.

**Tree generation header.** All ADC responses MUST include
`ADC-Tree-Generation`.

### 6.1 `orient` (GET `/adc/orient`)

Returns circuit configuration, tree generation, tokenizer, and endpoint links.

- Response headers: `Cache-Control: public, max-age=3600`,
  `ETag: W"<tree_generation>"`, `ADC-Tree-Generation: <tree_generation>`.

```json
{
  "adc": "0.1",
  "root": "https://api.example.com/data",
  "tree_generation": "2026-10-01T12:00:00Z",
  "tokenizer": "cl100k_base",
  "conformance": ["core", "dynamic", "commerce"],
  "endpoints": { "orient": "/adc/orient", "tree": "/adc/tree" },
  "namespace": "com.example"
}
```

### 6.2 `tree` (GET `/adc/tree`)

Query parameters: `path` (default `/`), `depth` (1..`limits.max_depth`, default
1), `anchor` (alternative origin), `limit` (1..`limits.max_limit`, default
`defaults.page_size`), `cursor`.

```json
{
  "tree_generation": "2026-10-01T12:00:00Z",
  "path": "/electronics/audio",
  "nodes": [
    { "name": "headphones", "path": "/electronics/audio/headphones", "anchor": "[a10]", "kind": "collection", "has_children": true, "count": 12450 },
    { "name": "speakers", "path": "/electronics/audio/speakers", "anchor": "[a11]", "kind": "collection", "has_children": true, "count": 3200 }
  ],
  "cursor": null,
  "truncated": false
}
```

Rules: `depth` MUST NOT exceed `limits.max_depth`; `limit` MUST NOT exceed
`limits.max_limit`; if node count exceeds `limit`, MUST include a non-null
`cursor` and `truncated: true`; ordering is deterministic; unknown path ⇒
`404 ANCHOR_NOT_FOUND` with `details.path`.

### 6.3 `passport` (GET `/adc/passport`)

Exactly one of `anchor` or `path` MUST be provided.

```json
{
  "tree_generation": "2026-10-01T12:00:00Z",
  "path": "/electronics/audio/headphones",
  "anchor": "[a10]",
  "kind": "collection",
  "count": 12450,
  "list_item_tokens": 160,
  "list_item_bytes": 640,
  "list_item_tokens_p95": 480,
  "updated_at": "2026-10-01T10:00:00Z",
  "filters": [
    { "name": "brand", "type": "string", "ops": ["eq", "in", "prefix"], "enum": ["sony", "bose", "apple", "sennheiser"] },
    { "name": "price", "type": "number", "ops": ["eq", "lt", "lte", "gt", "gte"], "unit": "USD", "min": 0, "max": 5000 },
    { "name": "feature", "type": "string", "ops": ["eq", "in", "contains", "contains_all", "contains_any"], "enum": ["anc", "wireless", "bluetooth"] }
  ],
  "aggregations": {
    "supported": true,
    "metrics": ["count()", "sum(price)", "avg(price)", "min(price)", "max(price)", "p50(price)", "p95(price)"],
    "group_by": ["brand", "model", "feature"],
    "min_group_size_default": 5
  },
  "slices": [
    { "name": "summary", "tokens": 420, "bytes": 1680, "generation": "V1:9b2f...", "generated_at": "2026-10-01T10:00:00Z", "dynamic": false, "provenance": "publisher", "default": true },
    { "name": "specs", "tokens": 280, "bytes": 1120, "generation": "V1:1c07...", "dynamic": false, "provenance": "publisher" },
    { "name": "reviews", "tokens": 1200, "bytes": 4800, "generation": "V1:44aa...", "dynamic": false, "provenance": "user_generated", "moderated": true },
    { "name": "logistics", "tokens": 280, "bytes": 1120, "generation": "V1:8e3d...", "dynamic": true, "price": 0.0005, "currency": "USD", "unit": "call", "degradation": { "mode": "stale_fallback", "max_staleness_seconds": 900 }, "provenance": "publisher" }
  ],
  "quality": 0.98,
  "official": true,
  "attribution": "required",
  "license": "CC-BY-4.0"
}
```

Semantics:

- `list_item_tokens` / `list_item_bytes` are the **average per item** cost of a
  collection row without a `slice`. Publishers SHOULD declare
  `list_item_tokens_p95` for worst-case budgeting.
- `count` is REQUIRED for `kind: "collection"`, omitted otherwise.
- `filters` descriptors MUST include `name`, `type`
  (`string | number | boolean | date | enum`), and `ops` (a non-empty subset of
  §6.4.1 operators). A v0.1-compatible plain string array (meaning "eq only")
  MUST also be accepted by agents.
- `aggregations` declares `supported`, accepted `metrics`, `group_by`, and
  `min_group_size_default`.
- Slice `provenance` ∈ `publisher | user_generated | third_party`;
  `moderated` MAY accompany `user_generated`.
- **Exactly one slice** MUST carry `"default": true`; it is returned when
  `fetch` omits `slice`.
- Node-level `quality`, `official`, `attribution`, `license` are defaults; a
  slice MAY override (precedence: slice > node > manifest).
- `generation` is REQUIRED per slice; `generated_at` is optional RFC 3339 UTC.
- `degradation.max_staleness_seconds` is the maximum age for `stale_fallback`;
  beyond it the publisher MUST use `fail_closed` or `partial_omit`.

### 6.4 `fetch`

#### 6.4.1 Standard `fetch` (GET `/adc/fetch`)

Query parameters: `anchor` XOR `path` (exactly one REQUIRED), `slice` (default:
passport default), `limit` (1..`limits.max_limit`, default
`defaults.page_size`), `cursor`, `filter[key]`, `fields`, `order_by`,
`tree_generation`.

**Filter grammar (GET).** One normative grammar:

- `filter[field]=value` — implicit `eq`. Booleans `true`/`false`; null `null`;
  strings percent-encoded.
- `filter[field][op]=value` — explicit operator. Vocabulary:
  `eq, neq, lt, lte, gt, gte, in, nin, contains, contains_all, contains_any,
  contains_none, prefix, exists`.
- `in`/`nin` and `contains_*` use **repeated parameters**; comma-separated lists
  are NOT used.
- Multiple conditions on the same field are ANDed (range).
- `exists` takes `true`/`false`.
- Unknown fields ⇒ `400 FILTER_INVALID` with `details.unknown_fields` and
  `details.allowed`.
- Operators not declared for a field ⇒ `400 FILTER_INVALID` with
  `details.unsupported_ops`.
- Condition count MUST NOT exceed `limits.max_filter_conditions`.

GET cannot express disjunction or negation beyond `nin`/`neq`; for `or`/`not`
or nesting, use POST (`§6.4.5`).

**Field projection.** `fields` is a comma-separated list of dot paths. `anchor`
and `generation` are always included. Servers unable to project return full
items and MAY include `"projection": "unsupported"`. Publishers SHOULD declare
`features.fields` when supported.

#### 6.4.2 Response polymorphism

**Collection** (`kind == "collection"`):

```json
{
  "tree_generation": "2026-10-01T12:00:00Z",
  "parent_anchor": "[a10]",
  "slice": "summary",
  "items": [
    {
      "anchor": "[a1042]",
      "path": "/electronics/audio/headphones/sony-wh1000xm5",
      "generation": "V1:9b2f...",
      "hydration": { "logistics": { "status": "live", "hydrated_at": "2026-10-01T15:47:03Z", "staleness_seconds": 0 } },
      "fields": { "brand": "Sony", "model": "WH-1000XM5", "price": 348.00, "currency": "USD", "specs": { "anc": true } }
    }
  ],
  "cursor": "eyJvZmZzZXQiOjIsImVwb2NoIjoiMjAyNi0xMC0wMSJ9",
  "truncated": true,
  "usage": { "tokens": 3210, "bytes": 12800, "items": 20, "cost": { "amount": 0.0, "currency": "USD", "unit": "call" } }
}
```

**Entity/document** (`kind == "entity" | "document"`):

```json
{
  "tree_generation": "2026-10-01T12:00:00Z",
  "anchor": "[a1042]",
  "path": "/electronics/audio/headphones/sony-wh1000xm5",
  "slice": "summary",
  "generation": "V1:9b2f...",
  "hydration": { "logistics": { "status": "live", "hydrated_at": "2026-10-01T15:47:03Z", "staleness_seconds": 0 } },
  "fields": { "brand": "Sony", "model": "WH-1000XM5", "price": 348.00, "currency": "USD", "specs": { "anc": true, "battery_life": "30 hours" } },
  "usage": { "tokens": 420, "bytes": 1680, "items": 1, "cost": { "amount": 0.0, "currency": "USD", "unit": "call" } }
}
```

Differences: collection responses carry `parent_anchor`, `items`, `cursor`,
`truncated`; entity responses carry top-level `anchor`, `path`, `slice`,
`generation`, `hydration`, `fields`.

#### 6.4.3 Per-slice hydration

`hydration` is a map keyed by slice name:

```json
"hydration": {
  "logistics": { "status": "stale_fallback", "hydrated_at": "2026-10-01T15:30:00Z", "staleness_seconds": 612, "max_staleness_seconds": 900, "failure_reason": "upstream_timeout", "revalidation_required": true },
  "pricing": { "status": "live", "hydrated_at": "2026-10-01T15:47:03Z", "staleness_seconds": 0 }
}
```

`hydration.status` ∈ `live | cached | stale_fallback | degraded_partial | synthetic_stub`.
If `staleness_seconds > max_staleness_seconds`, the server MUST NOT return
`stale_fallback`; it MUST apply `fail_closed` (503) or `partial_omit`.
An object that pairs `status: "stale_fallback"` with
`staleness_seconds > max_staleness_seconds` is a conformance violation and MUST
be rejected by validators.

#### 6.4.4 Pagination

`cursor` is opaque. Servers MUST bind a cursor to the query (anchor, filter,
ordering, `tree_generation`) and MUST reject tampered cursors with
`400 CURSOR_INVALID`. If the bound tree generation changed ⇒ `410 CURSOR_EXPIRED`
and the agent MUST restart from page one. Cursors MUST NOT contain secrets and
MUST NOT be guessable. Totals MUST NOT be inferable from cursor structure.
Cursors bind to `tree_generation` and membership, **not** to dynamic slice
generations.

#### 6.4.5 Heavy filter `fetch` (POST `/adc/fetch`)

- Headers: `Content-Type: application/adc+json`,
  `Idempotency-Key: <UUIDv4>` (recommended).
- Servers MUST respond `Cache-Control: no-store, private` and MUST NOT emit
  `ETag`.
- Body MAY include `generation` (slice → expected generation) and/or
  `tree_generation`.

```json
{
  "anchor": "[a10]",
  "tree_generation": "2026-10-01T12:00:00Z",
  "generation": { "summary": "V1:9b2f..." },
  "slice": "summary",
  "limit": 50,
  "cursor": null,
  "order_by": ["-price", "brand"],
  "filter": { "and": [ { "brand": { "eq": "sony" } }, { "price": { "lte": 200 } }, { "feature": { "contains_all": ["anc", "multipoint", "ldac"] } } ] },
  "fields": ["anchor", "price", "specs.anc"],
  "estimate_only": false
}
```

**POST filter grammar (ABNF):**

```abnf
filter          = object
object          = and-filter / or-filter / not-filter / field-condition
and-filter      = { "and" : [ filter *("," filter) ] }
or-filter       = { "or"  : [ filter *("," filter) ] }
not-filter      = { "not" : filter }
field-condition = { field-name : condition }
condition       = scalar / { filter-op : value }
filter-op       = "eq" / "neq" / "lt" / "lte" / "gt" / "gte" /
                  "in" / "nin" / "contains" / "contains_all" /
                  "contains_any" / "contains_none" / "prefix" / "exists"
```

**Semantics:**

- Application-level idempotent read. MUST NOT change server state. Responses
  MUST be `no-store` and MUST NOT carry `ETag`. (HTTP POST is not "safe" per
  RFC 9110 §9.2.1; this is an application-level semantic.)
- **Generation pre-validation.** If `generation` or `tree_generation` is
  supplied, the server compares against current values **before executing the
  query and before hydration** (but after filter/metric validation per §6.10).
  On mismatch ⇒ `409 GENERATION_STALE` with `details.current`, so the agent can
  update its cache and re-plan without paying for execution.
- Subject to the same auth, filtering, budgeting, and hydration rules as GET.
- Body size MUST NOT exceed `limits.max_post_bytes`, else
  `413 PAYLOAD_TOO_LARGE`.
- Servers SHOULD advertise `features.post_fetch: true`.

> **Note on validation order.** Earlier internal drafts said generation
> pre-validation aborts "before processing filters", which contradicted the
> normative order in §6.10 (filter validation precedes generation checks). This
> draft resolves it: schema and filter validation happen first (cheap, and lets
> an agent learn about schema errors even with a stale cache); generation
> pre-validation happens immediately before execution. See §6.10.

#### 6.4.6 Billing and caching invariants

When a response carries non-zero `usage.cost.amount` or `ADC-Cost-Amount`:

- The response MUST be `Cache-Control: private, no-store` unless the cost is for
  a static slice and the publisher explicitly permits caching.
- A cached copy served by a CDN/proxy MUST present `ADC-Cost-Amount: 0` and
  `usage.cost.amount: 0`.
- Dynamic slices (`dynamic: true`) MUST be `no-store`.
- Static slices MAY be `public` iff `cost.amount == 0`; otherwise `private`.
- Authenticated responses MUST include `Vary: Authorization`.

#### 6.4.7 Usage and reconciliation

Every successful fetch MUST include `usage` with `tokens` and `items`, and
SHOULD include `bytes` and (when charging) `cost`. Agents MUST reconcile budgets
against `usage`, not estimates.

#### 6.4.8 NDJSON streaming (optional)

When `features.ndjson_streaming` is `true` and the client sends
`Accept: application/x-ndjson`, the server MAY stream one item object per line,
with a final summary line `{"cursor":...,"truncated":...,"usage":{...}}`.

### 6.5 `estimate` (GET `/adc/estimate`)

Same parameters as `fetch`, plus optional aggregate parameters.

```json
{
  "anchor": "[a10]",
  "count": 412,
  "count_exact": true,
  "items_returned": 20,
  "estimated_tokens": 8600,
  "estimated_bytes": 34400,
  "breakdown": [
    { "part": "list", "items": 20, "tokens": 6400 },
    { "part": "slice", "slice": "specs", "tokens": 2200 }
  ],
  "price": { "amount": 0.0, "currency": "USD", "unit": "call" },
  "generation": { "specs": "V1:1c07..." },
  "confidence": "exact"
}
```

- `confidence` ∈ `exact | bounded | sample`. Agents MUST treat non-exact
  estimates conservatively.
- MUST NOT execute hydration.
- Publishers SHOULD implement `estimate`; Commerce publishers MUST.
- It MUST be cheap (O(1) or index-only).

### 6.6 `search` (GET `/adc/search`) — Search tier

Parameters: `query` (required), `mode` (`lexical | vector | hybrid`, default
`hybrid`), `scope`, `filter[key]`, `limit` (1..50, default 10), `cursor`.

Returns ordered anchors, not content. Each hit SHOULD carry `slice` and
`generation`. Results MUST be ordered by `score` descending with a deterministic
tiebreak (ascending lexical sort of canonical paths). `score` is opaque; only
ordering is meaningful. Unknown `mode` ⇒ `400 INVALID_BODY`-class error.

### 6.7 `grep` (GET `/adc/grep`) — Search tier

Parameters: `pattern` (required), `scope` (required when the global index
exceeds `limits.max_tree_nodes`), `limit`, `cursor`.

`pattern` is literal unless `features.grep_regex: true`, in which case the
engine MUST be linear-time (RE2 / Rust regex / Thompson NFA); catastrophic
backtracking MUST be impossible.

**Location addressing.** `pointer` is an RFC 6901 JSON Pointer into the block's
canonical JSON (RFC 8785 JCS). `byte_offset` is the 0-based byte offset into the
raw JCS payload. `line` MUST NOT be used for JSON blocks.

### 6.8 `verify` (GET `/adc/verify`) — Trust tier

Parameters: `anchor`, `generation`, `slice`, `kid`.

Trust publishers MUST accompany every block with a `signature` member and/or an
`ADC-Signature` header, a JWS over the JCS serialization of the block including
`path`, `generation`, and `tree_generation`. Algorithms MUST be from
`ES256 | ES384 | EdDSA`; `none` and MAC algorithms MUST be rejected. Agents MUST
verify locally when they hold the key and MUST NOT execute signed content as
instructions (A1). `verify` is a convenience and MUST answer truthfully about the
requested generation, returning `404`/`410` when it is no longer retained.

### 6.9 `aggregate` — Commerce tier

Methods: GET (standard) and POST (heavy filters). Path `/adc/aggregate`.

#### 6.9.1 GET parameters

`anchor` XOR `path`; `filter[key]`; `metrics` (required), comma-separated
`func(field):alias` (alias optional, default `func_field`); `group_by`;
`order_by`; `having` (`alias:op:value`); `limit` (default 10, ≤
`limits.max_aggregate_groups`); `cursor`; `min_group_size` (≥ passport
default); `max_staleness`; `approximate` (default `false`); `tree_generation`.

Supported functions: `count`, `count(field)`, `sum`, `avg`, `min`, `max`;
optional (declared in passport): `distinct`, `p50`, `p90`, `p95`, `p99`,
`median`.

#### 6.9.3 POST body

```json
{
  "anchor": "[a10]",
  "tree_generation": "2026-10-01T12:00:00Z",
  "filter": { "and": [ { "brand": { "eq": "sony" } }, { "feature": { "contains_all": ["anc", "wireless"] } } ] },
  "metrics": [
    { "expr": "avg(price)", "alias": "avg_price" },
    { "expr": "count()", "alias": "count" }
  ],
  "group_by": ["model"],
  "having": [ { "alias": "count", "op": "gt", "value": 5 } ],
  "order_by": ["-avg_price", "model"],
  "limit": 20,
  "min_group_size": 10,
  "approximate": false
}
```

#### 6.9.4 Response

```json
{
  "tree_generation": "2026-10-01T12:00:00Z",
  "anchor": "[a10]",
  "group_by": ["model"],
  "metrics": [ { "expr": "avg(price)", "alias": "avg_price" }, { "expr": "count()", "alias": "count" } ],
  "groups": [
    { "key": { "model": "WH-1000XM5" }, "values": { "count": 412, "avg_price": 348.2 } },
    { "key": { "model": "WH-CH720N" }, "values": { "count": 287, "avg_price": 128.5 } }
  ],
  "totals": { "count": 699, "avg_price": 258.4 },
  "totals_visible_only": true,
  "cursor": null,
  "truncated": false,
  "approximate": false,
  "suppressed_groups": 0,
  "partial": false,
  "coverage": 1.0,
  "computed_at": "2026-10-01T15:47:03Z",
  "generation": "AGG:7f31...",
  "usage": { "tokens": 812, "bytes": 2100, "items": 0, "cost": { "amount": 0.0, "currency": "USD", "unit": "call" } }
}
```

**Ungrouped aggregate:** when `group_by` is absent, the response MUST contain a
top-level `values` object and MUST NOT contain `groups`. When `group_by` is
present, `groups` is REQUIRED and `totals` SHOULD be present (MAY be omitted when
privacy forbids totals).

**Privacy invariants (§9.3):**

- If `suppressed_groups > 0`, `totals` MUST be computed over visible groups only
  AND carry `"totals_visible_only": true`, OR be omitted entirely. Suppressed
  group data MUST NOT be folded into `totals`.
- If `approximate: false` was requested but exact computation is impossible, the
  server MUST return `422 AGGREGATION_TOO_BROAD` or `503 HYDRATION_FAILED`, NOT
  silently return `approximate: true`.
- `partial` is true if and only if at least one group was suppressed.
  `coverage` is `sum(row counts of returned groups) / rows matched`; it is a
  scalar over the matched set and MUST NOT be used to reconstruct any
  individual suppressed group.
- When `suppressed_groups > 0` and no `totals` can be emitted without
  disclosing suppressed mass, the server MUST omit BOTH `totals` and
  `totals_visible_only`. The two members travel together; a response MUST NOT
  carry `totals_visible_only` without `totals`.

**Generations and caching.** `generation` MUST be a deterministic function of
`(tree_generation, anchor, filter, metrics, group_by, source generations)`.
`computed_at` is the field agents use for age policy. Cacheable responses SHOULD
carry `ETag: "<generation>"`.

**Cardinality protection.** If grouping would exceed `limits.max_aggregate_groups`
without a sufficient `limit`, the server MAY return `422 AGGREGATION_TOO_BROAD`
with `details.estimated_groups`. It MUST NOT stream unbounded groups.

**Errors:** `AGGREGATE_UNSUPPORTED`, `METRIC_UNSUPPORTED`, `GROUP_BY_UNSUPPORTED`,
`AGGREGATION_TOO_BROAD`, `FILTER_INVALID`, `BUDGET_EXCEEDED`, `RATE_LIMIT`,
`HYDRATION_FAILED`.

#### 6.9.5 Caching and constraints

- GET aggregate responses ARE cacheable. Publishers SHOULD emit
  `Cache-Control: public, max-age=300` and an `ETag` derived from the dataset
  generation hash.
- Requests to group by un-indexed or unbounded high-cardinality fields MUST be
  rejected with `400 INVALID_AGGREGATION_TARGET`.

### 6.10 Validation order (normative for all methods)

Servers MUST evaluate in this order and return the first failure:

1. **Transport/shape** — method known; media type (`415`); body size (`413`).
2. **Auth** — `401 AUTH_REQUIRED`, `403 FORBIDDEN`.
3. **Rate limit** — `429 RATE_LIMIT` with `RateLimit-*` and `Retry-After`.
4. **Parse/schema** — `400 INVALID_BODY`.
5. **Anchor/path resolution, conformance** — `404 ANCHOR_NOT_FOUND`,
   `400 PATH_INVALID`, `409 ANCHOR_MOVED`, `410 ANCHOR_EXPIRED`,
   `501 NOT_IMPLEMENTED`.
6. **Filter/metric/group-by validation** — `400 FILTER_INVALID`,
   `400 METRIC_UNSUPPORTED`, `400 GROUP_BY_UNSUPPORTED`.
7. **Generation pre-validation** — `409 GENERATION_STALE` with `details.current`.
8. **Cost admission** — `402 BUDGET_EXCEEDED`.
9. **Execution and hydration** — `200`; `503 HYDRATION_FAILED`; `200` with
   `partial`/`hydration` flags.

**Rationale:** cheap and security-relevant checks precede expensive ones;
generation checks precede *execution* so agents never pay for a fetch they will
discard; filter validation precedes generation checks so agents learn about
schema errors even with a stale cache.

**Precedence when the caller supplies a stale `tree_generation`.** Anchor
resolution runs against the *current* generation. Three cases:

1. The anchor/path resolves → continue normally; step 7 applies.
2. It does not resolve AND the caller supplied a `tree_generation` that differs
   from the current one → `409 GENERATION_STALE` with `details.current`, never
   `404`. The caller's anchor table is known to be out of date (§5.3.3), so
   reporting "never existed" would be false.
3. It does not resolve and no generation was supplied → `404 ANCHOR_NOT_FOUND`.

Filter validation still runs before this decision's *step 7 counterpart*: a
request carrying both an unknown filter field and a stale generation fails with
`FILTER_INVALID`, so an agent learns about schema errors from a stale cache.

### 6.11 Rate limiting

Publishers declare `rate_limits` per scope. Every response SHOULD carry
`RateLimit-Limit`, `RateLimit-Remaining`, `RateLimit-Reset`. `429` responses MUST
carry `Retry-After` and `error.code: "RATE_LIMIT"`, `error.retryable: true`.
Method weights are declared by the publisher (defaults: `fetch` 1, `post_fetch`
2, `estimate` 1, `aggregate` 5, others 1). Agents MUST honour `Retry-After` and
SHOULD use exponential backoff with jitter.

### 6.12 Errors (RFC 9457 Problem Details)

All errors MUST use `Content-Type: application/problem+json`:

```json
{
  "type": "https://agentdatacircuit.org/errors/ANCHOR_EXPIRED",
  "title": "Anchor Expired",
  "status": 410,
  "detail": "Anchor [a1042] belongs to a retired tree_generation.",
  "instance": "/adc/fetch?anchor=%5Ba1042%5D",
  "tree_generation": "2026-10-01T12:00:00Z",
  "retryable": false,
  "details": { "path": "/electronics/audio/headphones/sony-wh1000xm5" },
  "request_id": "01J...",
  "retry_after": null
}
```

| Status | Code | Description | Retryable |
|---|---|---|---|
| 400 | `INVALID_BODY` | Body malformed or invalid schema. | No |
| 400 | `FILTER_INVALID` | Filter malformed, unknown field, unsupported op. | No |
| 400 | `INVALID_AGGREGATION` | Aggregation field/operator not permitted. | No |
| 400 | `INVALID_AGGREGATION_TARGET` | `group_by` field not indexed or not permitted. | No |
| 400 | `METRIC_UNSUPPORTED` | Metric not declared in this passport's `aggregations.metrics`. | No |
| 400 | `GROUP_BY_UNSUPPORTED` | Field not declared in this passport's `aggregations.group_by`. | No |
| 400 | `AGGREGATE_UNSUPPORTED` | Node has `aggregations.supported: false`. | No |
| 400 | `PATH_INVALID` | Path violates canonicalization. | No |
| 400 | `CURSOR_INVALID` | Cursor tampered or malformed. | No |
| 400 | `ANCHOR_INVALID` | Anchor syntax invalid. | No |
| 401 | `AUTH_REQUIRED` | Missing/invalid Authorization. | No |
| 402 | `BUDGET_EXCEEDED` | Exceeds server cost limit or account balance. | No |
| 403 | `FORBIDDEN` | Authenticated but not authorized. | No |
| 404 | `ANCHOR_NOT_FOUND` | Valid syntax, not in current generation. | No |
| 404 | `SLICE_NOT_FOUND` | Slice not defined in passport. | No |
| 409 | `GENERATION_STALE` | Expected generation mismatch. | Yes (refresh) |
| 409 | `ANCHOR_MOVED` | Anchor moved; see `details.replaced_by`. | Yes |
| 410 | `ANCHOR_EXPIRED` | Anchor from invalidated generation. | No |
| 410 | `CURSOR_EXPIRED` | Cursor invalidated by catalog update. | Yes |
| 413 | `PAYLOAD_TOO_LARGE` | Body exceeds `limits.max_post_bytes`. | No |
| 415 | `UNSUPPORTED_MEDIA_TYPE` | Content-Type missing/malformed. | No |
| 422 | `AGGREGATION_TOO_BROAD` | Group cardinality exceeds limit. | No |
| 429 | `RATE_LIMIT` | Rate/quota exceeded. | Yes |
| 500 | `INTERNAL_ERROR` | Internal failure. | Yes |
| 501 | `NOT_IMPLEMENTED` | Capability outside declared tier. | No |
| 502 | `BAD_GATEWAY` | Upstream unparseable during hydration. | Yes |
| 503 | `HYDRATION_FAILED` | Dynamic backend unavailable, `fail_closed`. | Yes |
| 504 | `BACKEND_TIMEOUT` | Upstream exceeded latency SLA. | Yes |

**Prompt injection defense.** All error fields (`title`, `detail`, `details`)
MUST be treated as untrusted text and isolated in inert data frames before LLM
processing (§8.1).

---

## 7. Freshness & Degradation

### 7.1 Freshness generations

1. **Content invariance:** a `generation` is a deterministic token identifying a
   slice's byte-level state. `SliceA ≡ SliceB ⇔ generation(SliceA) = generation(SliceB)`.
2. **Hierarchy:**
   - `tree_generation` covers structural topology (paths/anchors).
   - Slice-level `generation` covers one named slice's payload.
3. **Cache lifecycle:** cache key is
   `⟨OriginAuthority, AnchorToken, SliceName, Generation⟩`. On passport
   inspection, matching generation ⇒ **clean (read)**; differing ⇒ **stale
   (unread)**.

### 7.2 Dynamic slice hydration

Slices declared `dynamic: true` hydrate on demand. Static slices MUST be
cacheable at edge CDNs and MUST NOT depend on dynamic backends. Dynamic slice
descriptors MUST declare expected latency and micro-costs where applicable.

### 7.3 Degradation semantics

When a dependency required for a dynamic slice fails, times out, or exceeds its
declared SLA, the publisher MUST NOT return synthesized or un-flagged stale data.
It MUST execute one of three declared modes:

- **`stale_fallback`** — return the most recent cached representation, provided
  `staleness_seconds ≤ max_staleness_seconds`, with an explicit `hydration` object
  declaring `stale_fallback`, the age, and the failure reason.
- **`partial_omit`** — HTTP 200, all static slices present, the dynamic slice
  omitted, `hydration[slice].status = "degraded_partial"`.
- **`fail_closed`** — if the slice is critical or
  `staleness_seconds > max_staleness_seconds`, abort with HTTP 503 and
  `HYDRATION_FAILED`.

#### 7.3.2 The `hydration` object

Every item involving dynamic data MUST carry `hydration`, a map keyed by slice
name. Vocabulary: `live`, `cached`, `stale_fallback`, `degraded_partial`,
`synthetic_stub`.

#### 7.3.3 Agent transactional safety

- **Informational:** an agent MAY consume `stale_fallback` for read-only
  summaries, provided it discloses the staleness age (A6).
- **Transactional gate:** an agent MUST NOT initiate a write, payment, binding
  reservation, or external side effect using `stale_fallback` or
  `degraded_partial` data. It MUST re-request with a precondition or wait for a
  `live` cycle.

#### 7.3.4 Billing invariants for degraded responses

Publishers MUST NOT charge dynamic hydration fees for slices that fail live
hydration. If `hydration.status ∈ {stale_fallback, degraded_partial,
synthetic_stub}`, the server MUST set `ADC-Cost-Amount: 0.0` and
`usage.cost.amount: 0.0`. An agent MUST reject a non-zero charge on a degraded
response with `DEGRADATION_BILLING_VIOLATION` and refuse to reconcile.

---

## 8. Security & Privacy

### 8.1 Passive parsing & prompt injection defense (A1)

ADC content (trees, passports, blocks, aggregates, error payloads) MUST be
handled as **passive, inert data**. Runtimes MUST serialize payload fields into
isolated data frames before LLM context and MUST prepend directives that text
within data frames contains zero executable commands. Publishers MUST NOT include
executable scripts unless the passport declares the field as executable code.

### 8.2 Denial-of-service bounds

Publishers MUST enforce `limits.max_filter_depth` (default 4),
`limits.max_filter_conditions` (default 32), `limits.max_post_bytes` (default
65,536, else `413`), and `limits.max_aggregate_groups` (default 1,000, else
`422`).

### 8.3 Authorization & isolation

When `auth.required: true`, the manifest MUST declare `auth.schemes`. An
unauthenticated request to a protected node MUST return `401 AUTH_REQUIRED`
uniformly across endpoints and MUST NOT leak existence via timing or mismatched
codes. Agents MUST NOT forward credentials minted for one origin to another,
even via outward anchors.

### 8.4 Privacy in aggregates (k-anonymity)

1. **Minimum group size.** If `0 < count(G) < k`, suppress group `G`.
2. **Suppression auditing.** Increment `suppressed_groups` for each suppressed
   group.
3. **Extremum masking.** When a group size is below `2k`, publishers MUST NOT
   emit exact `min`/`max` if it reveals an individual's attribute; emit `null`
   and include `metric_suppressed: true`.

### 8.5 Privacy invariants for `totals`

When `suppressed_groups > 0`, `totals` MUST be computed over visible groups only
and carry `totals_visible_only: true`, OR be omitted. Including suppressed data
enables differencing:
`Value_suppressed = Totals − Σ Visible`. The `min_group_size` request parameter
is a publisher-enforced floor: agents MAY request higher, servers MUST reject
lower with `400 FILTER_INVALID` and `details.privacy_floor`.

---

## 9. Economics & Budget

### 9.1 Precedence

`Manifest Policy → Passport Metadata → Slice Override`. A value on a slice
overrides node and manifest; a node value overrides manifest; otherwise manifest
applies.

### 9.2 Agent budget state machine with concurrency

States: `IDLE → ESTIMATING → EVALUATING → {COMMITTED → EXECUTING → RECONCILING}
| HALTED`.

- **IDLE** initializes `B_token_max`, `B_token_rem`, `B_fiat_max`, `B_fiat_rem`.
- **ESTIMATING** projects
  `c_tokens = slice.tokens + (limit × list_item_tokens_p95)`.
- **EVALUATING** admits iff
  `(c_tokens ≤ B_token_rem) ∧ (c_fiat ≤ B_fiat_rem)`.
- **COMMITTED** atomically reserves projected costs (mutex / compare-and-swap).
- **RECONCILING** releases the reservation and applies actual usage:
  `B_token_rem += c_tokens_projected − usage.tokens`. Overrun ⇒
  `HALTED_OVERRUN`.
- On network error / `429` / `503`, release the reservation and return to
  `ESTIMATING` (retryable) or `HALTED` (non-retryable).

Normative reference algorithm and implementation: see
[agent-runtime/AGENT-RUNTIME.md](agent-runtime/AGENT-RUNTIME.md) and
`reference/adc/budget.py`.

### 9.3 Micro-billing and settlement

Billed responses MUST declare unit costs in the passport (`price`, `currency`,
`unit`) and include `ADC-Cost-Amount`, `ADC-Cost-Currency`, `ADC-Cost-Unit`.
Degraded responses MUST NOT charge hydration fees (§7.3.4). Billed responses MUST
be `private, no-store` unless explicitly permitted. Budget exhaustion ⇒
`402 BUDGET_EXCEEDED` with `details.current_balance` and
`details.required_balance`.

---

## 10. Conformance Profiles

```
Core ──► Search ──► Dynamic ──► Commerce ──► Trust
```

See [CONFORMANCE.md](CONFORMANCE.md) for the requirements matrix and normative
test vectors.

---

## Annex A — Agent runtime algorithms (normative)

### A.1 Anchor arithmetic & namespace parsing

Wire anchor grammar (B.1) and the agent-side expression grammar (which alone
allows a local `=alias`, never sent on the wire) are distinct. The reference
implementation is `reference/adc/anchors.py`. The runtime MUST:

- Normalize the origin authority per RFC 3986 (lowercase scheme/host, elide
  default ports, punycode IDNs).
- Reject `k <= 0` rollback and `k > depth` with `ROLLBACK_INVALID` /
  `NO_SUCH_ANCESTOR`.
- Resolve `/suffix` on segment boundaries, failing with `NO_SUCH_CHILD` or
  `AMBIGUOUS_CHILD`.
- Never emit a namespaced token or an unresolved expression on the wire.

**Error space.** Annex A.1 operations raise a *local* `AnchorError`
(`NAMESPACE_INVALID`, `NAMESPACE_MISMATCH`, `ALIAS_INVALID`,
`ROLLBACK_INVALID`, `NO_SUCH_ANCESTOR`, `NO_SUCH_CHILD`, `AMBIGUOUS_CHILD`). This is a **separate error space from
§6.12**: these codes never travel as HTTP Problem Details, are not listed in
the §6.12 table, and have no status code. They occur while an agent manipulates
its own anchor table; mapping them onto transport errors is runtime-local.

### A.2 Budget FSM

See `reference/adc/budget.py`. Implementations MUST use atomic reserve/reconcile
and MUST provide `release_reservation()` for failure paths.

### A.3 Degradation handling

On receiving a payload: if all slices are `live`, proceed. If any is
`stale_fallback`, distinguish informational consumption (allowed, disclose age)
from transactional action (halt, require live revalidation). For `partial_omit`
/ `fail_closed`, drop or log the omission.

---

## Annex B — ABNF Grammars

### B.1 Wire anchor grammar

```abnf
raw-anchor        = "[" anchor-id "]" [ rollback ] [ suffix-path ]
anchor-id         = "a" 1*DIGIT
rollback          = "-" 1*DIGIT          ; k >= 1
suffix-path       = "/" path-segment *( "/" path-segment )
path-segment      = 1*( ALPHA / DIGIT / "-" / "_" / "." )
```

Namespaced form (agent-internal only):

```abnf
namespaced-anchor = origin-authority "#" raw-anchor
origin-authority  = scheme "://" host [ ":" port ]
scheme            = "https" / "http"
host              = 1*( ALPHA / DIGIT / "." / "-" )
port              = 1*DIGIT
```

Agent expression grammar (local only; MUST NOT be sent on the wire):

```abnf
anchor-expression = namespaced-anchor [ "=" alias ]
alias             = 1*( ALPHA / DIGIT / "-" / "_" )
```

### B.2 GET filter grammar

```abnf
filter-query = "filter[" field-name "]" [ "[" filter-op "]" ] "=" filter-value
field-name   = 1*( ALPHA / DIGIT / "_" / "." )
filter-op    = "eq" / "neq" / "lt" / "lte" / "gt" / "gte" /
               "in" / "nin" / "contains" / "contains_all" /
               "contains_any" / "contains_none" / "prefix" / "exists"
filter-value = 1*( unreserved / pct-encoded )
unreserved   = ALPHA / DIGIT / "-" / "." / "_" / "~"
pct-encoded  = "%" HEXDIG HEXDIG
```

### B.3 Metric expression grammar

```abnf
metric-expr = metric-func "(" [ field-name ] ")" [ ":" alias ]
metric-func = "count" / "sum" / "avg" / "min" / "max" /
              "distinct" / "p50" / "p90" / "p95" / "p99" / "median"
alias       = 1*( ALPHA / DIGIT / "_" )
```

---

## Annex C — Forward compatibility

1. **Member addition:** unknown JSON keys MUST be preserved or ignored, never
   cause failure.
2. **Enum extension:** unknown `hydration.status` ⇒ treat as `stale_fallback`;
   unknown `kind` ⇒ treat as `entity`; unknown metric ⇒ keep as opaque key.
3. **Major boundary:** an agent MUST NOT invoke endpoints on an incompatible
   major circuit unless `adc_min` declares support.

---

## Annex D — Model Context Protocol bindings

See `reference/mcp-tools/`. Conforming runtimes expose `adc_orient`, `adc_tree`,
`adc_passport`, `adc_fetch`, `adc_aggregate` as MCP tools; schemas match the
endpoints above.

The reference bridge additionally exposes `adc_cold_entry` for the optional
`/AI/` cold entry (informative, [AI-ENTRY.md](AI-ENTRY.md)); it takes
`origin` rather than `root` because it never consults the manifest, and it is
**not** a conformance requirement.

---

## Annex E — Test vectors

Normative vectors live in [CONFORMANCE.md](CONFORMANCE.md) and are executed by
`tests/`.