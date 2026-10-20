# AI-ENTRY: the `/AI/` cold entry

> **Status: informative design note. Not normative. Not required for ADC
> conformance.** Nothing here modifies [SPEC.md](SPEC.md); section 4 of the
> SPEC points here for context only. The material below is the design as
> shipped by the reference server, kept deliberately minimal for v1.

## 1. The problem: a protocol only helps agents that know it

Every discovery convention in use today - `robots.txt`, `sitemap.xml`,
`llms.txt`, `schema.org`, and `/.well-known/adc.json` - requires the agent
to already know the convention exists. That is a chicken-and-egg problem: a
rich protocol only helps agents that already know the protocol.

A cold agent meeting a random site for the first time has two options, and
both are bad:

1. **Scrape the HTML** - expensive in tokens, bandwidth, and time.
2. **Use an API** - requires registration, keys, and contracts.

Neither works for a first contact. `/AI/` is the third option: a single,
trivial, static entry point that any agent can probe with no prior
knowledge, no registration, and one request.

## 2. The path

    GET https://example.com/AI/

Why this exact path:

- Short: two characters plus a slash.
- Self-descriptive.
- Of the same class as `robots.txt`, `sitemap.xml`, `security.txt` -
  conventions the ecosystem already understands.
- Unlikely to collide with a real site section. If it does collide, the
  response will not match the shape an agent expects, and the agent falls
  back cleanly (section 7).

## 3. What `/AI/` returns

A small instruction map. Inert, machine-readable, under 4 KB. Not HTML. Not
a page. Not a script.

    GET https://example.com/AI/

    {
      "ai": "1.0",
      "vars": {
        "d1": "/AI/docs",
        "p1": "/AI/pricing",
        "v1": "/AI/watch",
        "s1": "/AI/search"
      },
      "keys": ["text", "meta", "image", "transcript", "chapters"]
    }

Three things are declared:

1. That this is an AI tunnel - the `"ai"` field.
2. A map of variables to paths - `vars`.
3. A list of content types the site can produce - `keys`.

Nothing else is required in the first version.

## 4. Variables

Variables are short opaque tokens that stand in for paths. The agent fetches
the map once, stores it, and reuses it. Subsequent requests use variables,
not full URLs.

    GET https://example.com/AI[d1]?topic=function_calling
    GET https://example.com/AI[p1]?tier=pro
    GET https://example.com/AI[v1]?id=zLs2QG7lU7Q&get=transcript

Why variables:

- **Short** - a few tokens instead of a long URL, on every request.
- **Stable** - the site may move a path without invalidating the variable.
- **Readable** - the agent's context stays clean and legible.
- **Cacheable** - the map is fetched once per origin per session.

An anchor is a variable. A path is its expansion. The variable lives in the
agent's working memory; the path appears on the wire only when the map is
first fetched.

This is the same idea as ADC anchors (`https://origin#[aN]`), but without the
fragment syntax and without requiring the agent to know about ADC. The cold
entry uses plain path syntax so that a naive agent can use it with no
additional parsing rules.

## 5. Keys

Keys select **what to produce**, not just what format to return.

    ?get=text         text content only
    ?get=meta         metadata (title, date, author, duration)
    ?get=image        one image, by id or default
    ?get=transcript   video or audio transcript
    ?get=chapters     video chapters
    ?get=comments     comments, if the site exposes them
    ?get=price        current price (dynamic)
    ?get=stock        stock level (dynamic)

This is **not** content negotiation. Content negotiation says "give me JSON
instead of HTML" but the server still produces the same content. Keys say
"produce ONLY this fragment." The server never builds the rest.

The difference is where the work happens. In scraping, the server builds
everything and the agent discards most of it. With keys, the server produces
only what was requested, and nothing else is materialized.

The site declares which keys it supports; an agent asks for one of them.

## 6. Context compression - the core economic argument

A product page might be 500 KB of HTML with images, styles, scripts, ads,
and analytics. As agent context, that is tens of thousands of tokens.

The same page requested as `?get=text&fields=model,price,anc` might be 10 KB
of clean JSON. As agent context, roughly 2 500 tokens.

Compression factor: one to two orders of magnitude.

The server does not render HTML. It does not generate CSS. It does not
invoke JavaScript. It reads its own datastore once, projects the requested
fields, and returns them. Production cost drops along with response size.

## 7. Fallback - mandatory

If `GET /AI/` returns 404, or returns a response that is not a valid tunnel
map, the agent MUST fall back to standard HTML parsing. No retries, no hangs,
no side effects.

If `/AI/` exists but a specific key is unavailable, the server returns an
error **inside** the tunnel format:

    { "error": "key_unavailable", "available": ["text", "meta"] }

The agent treats the tunnel as present, degrades gracefully, and continues.

This is what makes `/AI/` safe to probe. Absence costs one request. Presence
is an explicit invitation.

## 8. Relationship to ADC

Two entry layers coexist, for two classes of agent.

**Cold entry: `/AI/`.** Any agent can find it. No prior knowledge. Minimal:
a map of variables and a list of keys.

**Warm entry: `/.well-known/adc.json`.** For agents that already know ADC.
Full protocol: manifest, navigation tree, passports with token costs,
freshness generations, anchors, filters, aggregations, signatures.

`/AI/` is not a replacement for ADC. It is the door that leads to it. An
agent that arrives cold through `/AI/` may upgrade to the warm entry and use
the full protocol. An agent that already knows ADC goes straight to
`/.well-known/adc.json` and never touches `/AI/`.

A site may publish only `/AI/`, only `/.well-known/adc.json`, or both. The
choice is the publisher's.

## 9. Example walk-through

User request: "Find over-ear headphones with ANC under $100."

**Without `/AI/`:**

1. Search the site's HTML page - 300 to 800 KB.
2. Parse JavaScript-rendered results.
3. Open 10 to 20 product pages - 5 to 15 MB total.
4. Extract model, price, ANC support from each.
5. Filter and sort locally.

Total: tens of megabytes, tens of thousands of tokens, minutes.

**With `/AI/`:**

1. `GET site.com/AI/` - receive the map, under 4 KB.
2. `GET site.com/AI[s1]?q=over-ear+anc&max_price=100`
   `&fields=model,price,anc&limit=20`
3. Receive 20 rows, roughly 10 KB of JSON.

Total: two requests, roughly 2 500 tokens, seconds.

Two round trips. One cold probe. One typed query. Done.

## 10. Invariants

1. `/AI/` is optional. No site is required to publish it.
2. Absence changes nothing. Agents fall back to standard parsing.
3. The response is small. The entry map should fit in a few KB.
4. Variables are stable within a session.
5. Keys are typed selectors. The site declares which keys it supports.
6. The server produces only what was requested. No unrequested content is
   materialized.
7. The entry itself requires no authentication. Individual keys may.
8. The response is inert data. Never HTML, never JavaScript, never anything
   a browser would execute.

## 11. Reference implementation notes

The bundled example site (`examples/ecommerce`) and the reference server
demonstrate the **minimal subset** of this note, because the point of the
demo is that the cold entry is cheap, not that it is complete.

**Configuration.** The example site declares its tunnel in a sibling file,
`examples/ecommerce/ai.json`, next to `manifest.json`. The file is outside
the ADC manifest on purpose: `/AI/` is independent of the manifest, and a
site may publish either layer without the other. A site directory without
`ai.json` serves no `/AI/` map - the probe simply 404s and the agent falls
back.

**Variables.** `ai.json` maps opaque tokens onto real node paths of the
example circuit, for example `"h1": "/electronics/audio/headphones"`.

**Keys.** The example declares `text`, `meta`, `price`, and `stock` - the
ones this dataset can actually produce. `image`, `transcript`, `chapters`,
and `comments` are part of the key vocabulary (section 5) but are not
produced here, so asking for them yields `key_unavailable`.

**Responses.** Every response is `application/json`. `get` defaults to
`text`.

| Request | Status | Body |
|---|---|---|
| `GET /AI/` | 200 | the tunnel map (`ai`, `vars`, `keys`) |
| `GET /AI/` with no `ai.json` | 404 | Problem Details, not a tunnel map -> fall back |
| `GET /AI[<var>]?get=<key>` | 200 | `{"get": ..., "path": ..., "name": ..., ...}` |
| `GET /AI[<var>]?get=text&fields=a,b&limit=20` | 200 | collections: `{"fields": [...], "items": [...], "count": N}`; entities: `text` with only those fields |
| `GET /AI[<var>]?get=text&limit=20` | 200 | the collection names line, first 20 names listed (total count kept) |
| `GET /AI[<unknown>]` | 404 | `{"error": "unknown_variable", "available": [...]}` |
| `GET /AI[<var>]?get=<unsupported>` | 400 | `{"error": "key_unavailable", "available": [...]}` |
| `GET /AI[<var>]?fields=<bad>` or `?limit=<bad>` | 400 | `{"error": "invalid_parameter", "parameter": ..., "detail": ...}` |

`available` in an error lists what *is* usable at that point - variables for
`unknown_variable`, keys for `key_unavailable`. `invalid_parameter` instead
reports which `parameter` was rejected and why (`detail`). `limit` must be an
integer between 1 and 999999; `fields` is a comma-separated list of plain
field names; both apply only to `get=text`.

**Implemented here, partly.** The reference server accepts `get`, `fields`,
and `limit` on the tunnel: a collection asked for `fields` returns projected
rows, an entity returns only the selected fields, and `limit` caps listed
names or returned rows. Query parameters such as `q` and filters - shown in
the walk-through above - belong to a fuller implementation of this note, and
today are served by the warm entry (`SPEC.md` 6.4 and 6.9) once an agent has
upgraded.

## References

- [SPEC.md](SPEC.md) section 4 - discovery of the warm entry
- [RATIONALE.md](RATIONALE.md) - how ADC relates to neighbours
- [reference/adc/server.py](reference/adc/server.py) - the handlers
