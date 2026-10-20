# Agent Data Circuit (ADC)

> **Status: DRAFT — experimental, no production implementations yet.**
> This repository contains a protocol proposal, JSON Schemas, a reference
> server, a validator, and an MCP tool bridge. Nothing here is a standard.
> Feedback and independent implementations are the entire point.

ADC is not a library, not an SDK, not a framework. It is a set of
instructions a publisher applies to their own site. No dependency,
no build step, no vendor.

ADC is a **publisher-side protocol** that lets a website expose its data as a
navigable, cost-aware, freshness-tagged structure for AI agents — without model
retraining, proprietary frameworks, or middleware.

A site publishes a manifest, a navigation tree, per-node passports, and
sliceable data blocks. Agents navigate by stable anchors, budget in tokens
*before* requesting, fetch only the slices they need, and ask the server for
aggregates instead of downloading entire catalogs.

---

## Why this exists

Agents that need data from the web today face five structural failures:

1. **Token waste** — they download full HTML, strip markup noise, and spend
   context on navigation chrome instead of content.
2. **Conflation** — official and secondary sources mix into one stream, so
   per-entity attributes are lost (e.g. "this model is a dense 70B" generalized
   to the whole product line).
3. **Stale data** — agents read cached pages after the price, stock, or
   announcement already changed.
4. **No cost signal** — an agent cannot know how much context a request will
   consume before making it.
5. **Macro-query waste** — computing "average price across this catalog"
   requires downloading thousands of entity pages.

A page that costs 500 KB of HTML and tens of thousands of tokens to scrape
costs around 10 KB of JSON and a few thousand tokens through ADC. That is the
compression the protocol provides.

The publisher already has the correct per-entity data. It is just not
*machine-addressable*, *cost-budgeted*, or *aggregatable*.

## What ADC is not

ADC is **not** a replacement for MCP, `llms.txt`, schema.org, or sitemaps. It
is complementary. See [RATIONALE.md](RATIONALE.md) for a detailed comparison.

| Concern | schema.org / HTML | `llms.txt` | MCP | **ADC** |
|---|---|---|---|---|
| Machine-readable site structure | partial (markup) | flat list | tool calls | **tree + anchors** |
| Per-node cost in tokens | no | no | no | **yes (passport)** |
| Slice-level freshness generations | no | no | no | **yes** |
| Server-side aggregates | no | no | app-specific | **yes (k-anonymous)** |
| Live/static separation + degradation | no | no | no | **yes** |
| No retraining / no SDK required | yes | yes | yes | **yes (plain HTTP + JSON)** |

## Two entry layers

A site that knows nothing about ADC can still offer agents a cheap way in.
The **cold entry** is a single static path, `GET /AI/`, that returns a small
JSON instruction map: a handful of variables standing in for paths, and the
keys of content the site can produce. Any agent can probe it with one
request, without prior knowledge, without registration — and if it is not
there, the agent falls back to ordinary HTML parsing. See
[AI-ENTRY.md](AI-ENTRY.md).

Agents that already know ADC go straight to the **warm entry**,
`GET /.well-known/adc.json`, and get the full protocol: manifest,
navigation tree, passports with token costs, freshness generations,
anchors, filters, and aggregates. `/AI/` is the door; `/.well-known/adc.json`
is the room behind it. A publisher may ship either layer or both.

## Use cases

Why a publisher — especially a marketplace — would expose `/AI/` at all.
The examples show what a full implementation of [AI-ENTRY.md](AI-ENTRY.md)
looks like; the bundled demo ships the minimal subset described in
AI-ENTRY.md section 11.

### The marketplace funnel

Today a marketplace serves the same HTML page to every visitor, regardless
of intent: ads, banners, promoted listings, rendering cost, ad auctions —
on visitors who mostly never convert.

An agent working for a ready buyer does not browse. It queries.

```text
GET marketplace.com/AI/
GET marketplace.com/AI[s1]?q=wireless+earbuds+anc&max_price=80
    &fields=model,price,anc,battery&limit=20
```

Two requests, about 10 KB, all current. The agent filters on the server
rather than locally, retrieves only the fields the buyer cares about, and
sees no ads and no promoted listings. The server saves too: it stops
building pages nobody finishes reading.

| Without `/AI/` | With `/AI/` |
|---|---|
| Full HTML with ads, JS, CSS | Typed JSON fragment |
| 300–800 KB per page | ~10 KB per query |
| Thousands of pages parsed | One filtered query |
| Server renders what most users ignore | Server produces only what was asked |

### Cross-border commerce

A buyer on a cross-border marketplace faces three obstacles at once: product
pages in a language they do not read, machine translation that mangles
technical specifications, and ordering, shipping, and customs steps
described in unfamiliar terms. Many purchases are abandoned not because the
product is wrong, but because the process is opaque.

The agent retrieves product, shipping, and ordering information through
typed keys, in the buyer's language, and assembles one coherent answer.
The buyer reads a clear summary instead of a half-translated page. The
publisher controls what the agent can see: fields that are sensitive,
private, or licensed are simply not in the `keys` list.

### The help-desk scenario

A user lands on a support page with 40 sections, cannot find the one that
answers their question, and opens a ticket — or leaves. With a `help`
variable published in `/AI/`, the agent navigates the content on the user's
behalf and returns one relevant answer with concrete steps. The support
team writes the content once; the agent retrieves whatever the user
actually needs, in the moment, without anyone browsing a tree.

### Why the publisher controls the keys

This is the central design point. A publisher that lists `/AI/` is not
opening its database. It is declaring which fields an agent may retrieve,
which content types the site can produce, which paths are addressable
through variables, and what is excluded — never in the keys list, never
reachable.

If a marketplace does not want pricing exposed to agents, it does not list
`get=price`. This is `robots.txt` in reverse: not "do not enter", but
"enter here, and only here." The publisher chooses what to expose; the
agent chooses what to retrieve.

### The numbers

A single buyer query on a mid-size marketplace:

| | Scraping | `/AI/` |
|---|---|---|
| HTTP requests | 1 search + 10–20 product pages | 1 probe + 1 query |
| Bytes transferred | 5–15 MB | ~10 KB |
| Agent tokens consumed | 30 000–80 000 | ~2 500 |
| Server render cost | Full HTML, CSS, JS per page | One datastore projection |
| Time to answer | 10–60 seconds | 1–3 seconds |
| Result freshness | As of last crawl | As of the query |

Compression: roughly two orders of magnitude in bytes and tokens, one order
of magnitude in time.

### Who this is for

Marketplaces with large catalogs and changing price/stock. Cross-border
commerce where language and process are barriers. Documentation sites where
users cannot find the right section. Government and public services with
procedural content. News and media where freshness matters and article
bodies are buried under chrome. Any site whose content is locked inside
HTML that an agent has to parse.

If your content is already structured in a database and served through
templates, you are one route away from `/AI/`. Nothing else changes.

## Design in one paragraph

A publisher serves `/.well-known/adc.json` and a small set of HTTP endpoints
(`orient`, `tree`, `passport`, `fetch`, plus optional `search`, `grep`,
`aggregate`, `estimate`, `verify`). Every node has a **passport** declaring its
token/byte cost, filters, default slice, and freshness generations. Agents use
**namespaced anchors** (`https://origin#[aN]`) resolved locally, with local
arithmetic (`[aN]-k`, `[aN]/suffix`) that never touches the wire. Dynamic
slices hydrate live data and must declare an explicit **degradation mode**
(`stale_fallback` / `partial_omit` / `fail_closed`) if the backend fails. The
agent runs a finite **budget state machine** and reconciles estimates against
actual `usage`. If a circuit is absent or unreachable, the agent falls back
to standard parsing with no side effects.

## Repository layout

```
agent-data-circuit/
├── README.md
├── SPEC.md                 normative protocol (Core + tiers)
├── AI-ENTRY.md             cold entry design note (`/AI/`), informative
├── RATIONALE.md            why ADC, and how it differs from neighbours
├── NON-GOALS.md            deliberately out of scope
├── SECURITY.md             threat model and privacy invariants
├── ECONOMICS.md            cost model, billing, budget machine
├── CONFORMANCE.md          levels and test vectors
├── CASE-STUDY.md           real-site measurement: scraping vs `/AI/`
├── CHANGELOG.md
├── CONTRIBUTING.md
├── CODE_OF_CONDUCT.md
├── schemas/v0.1/           JSON Schemas for every artefact
├── examples/ecommerce/     runnable demo circuit
├── reference/
│   ├── adc/                Python reference server (stdlib only)
│   ├── validator/          `adc validate ./site`
│   └── mcp-tools/          MCP bridge exposing ADC methods as tools
├── agent-runtime/          normative agent-side algorithms
├── rfcs/                   design records for each subsystem
├── tools/                  reproduces CASE-STUDY.md end to end
└── tests/                  pytest suite over the SDK and the example
```

## Quick start

Requires Python 3.11+ (stdlib only for the server and validator).

Start the reference server on the bundled example circuit:

```bash
python -m reference.adc.server examples/ecommerce
```

8080 is what the example manifest declares as `root`. In a second
terminal, talk to it — first the warm entry, then the optional cold entry:

```bash
# warm entry (SPEC.md section 6)
curl "http://127.0.0.1:8080/adc/orient"
curl "http://127.0.0.1:8080/adc/tree?path=/&depth=2"
curl "http://127.0.0.1:8080/adc/fetch?anchor=%5Ba30%5D&limit=2&order_by=-price"
curl "http://127.0.0.1:8080/adc/aggregate?anchor=%5Ba30%5D&metrics=avg(price):avg_price,count():count&group_by=brand&min_group_size=5"

# cold entry (AI-ENTRY.md); -g stops curl from globbing the [...] variable
curl -g "http://127.0.0.1:8080/AI/"
curl -g "http://127.0.0.1:8080/AI[h1]?get=meta"
```

With the server stopped, validate the example circuit:

```bash
python reference/validator/adc.py validate examples/ecommerce
```

`orient` echoes the manifest's `root`, and every path in the circuit is
relative to it (SPEC P5). If you serve on a different port, update
`root` in `examples/ecommerce/manifest.json` to match — otherwise `orient`
points clients at an origin that is not the one they just talked to.

Run the test suite:

```bash
python -m pytest tests -q
```

## Conformance levels

Levels are cumulative; declaring a higher level implies every level below it.

| Level | Adds |
|---|---|
| **Core** | manifest + tree + passport + fetch, estimate recommended |
| **Search** | hybrid index: `search`, `grep` |
| **Dynamic** | live hydration and explicit degradation |
| **Commerce** | `aggregate`, POST fetch, pagination, micro-pricing |
| **Trust** | JWS signatures, JWKS, licensing, SLAs |

The reference implementation targets **Core + Dynamic + Commerce**, and
validates Search/Trust artefacts.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). This is a v0.1 draft: the fastest way to
help is to build a conforming implementation in another language and try to pass
the vectors in [CONFORMANCE.md](CONFORMANCE.md).

## License

- Specification and documentation: **CC BY 4.0** ([LICENSE-DOCS](LICENSE-DOCS)).
- Code, schemas, and tools: **Apache-2.0** ([LICENSE-CODE](LICENSE-CODE)).