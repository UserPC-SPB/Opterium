# SPEC — Universal Tool-Output Formatter (toolfmt, formatter v3)

Status: IMPLEMENTED (Rust, `crates/toolfmt`, version 3.1.3). Revision 3.1,
2026-11-04 (3.1 adds the catalog, section 2.1). Supersedes revision 2 (the
Python prototype: formatter.py + mcp_gateway.py).
This file is authoritative. Where the code differs from it, the code has a bug.
Section 14 lists what changed since revision 2 and why.

---

## 1. MISSION

One resident component standing between the agent and ANY tool. The agent
calls a tool and receives the output already in the format chosen once, per
tool. The agent never spends an extra tool call on cleaning, reformatting or
post-processing.

Primary use case: an agent connects a new MCP server, captures real outputs
of its tools, writes a standard for each tool it cares about and binds it to
the server.tool key. From then on every call of that tool reaches the agent
already in the format the agent chose.

Registering a new MCP server never requires creating standards for its
tools. Tools without registered standards remain fully operational and
return their original upstream output unchanged. Standards are optional
enhancements, not prerequisites.

A standard is a small YAML file written by the agent against a REAL captured
output: which renderer to use, its parameters, and the PRE/POST chains.

### Dogmas

- D1. No standard -> the upstream result is forwarded exactly as received:
  all content blocks, structuredContent and isError, no header line.
  Built-in auto mode is off by default and is enabled per server in
  upstreams.json (`"auto": true`).
- D2. Output is NEVER blocked: any failure -> raw output plus a one-line
  note. The upstream call is never cancelled or rolled back because a
  formatter failed.
- D3. A standard is written ONLY against a real captured output. The
  gateway refuses to save a standard for a tool that has no capture yet.
- D4. Different operations get different standards. "Reversed order" is a
  parameter (`sort_desc`), not a special case.
- D5. The formatter does not know who the agent is. Its contract is only
  server + tool + args + data + keys. Several agents with different tastes
  use different home directories (`--home`).
- D6. Tool identity = server + tool name. Standards are keyed by the pair
  (`standards/<server>.<tool>.yml`). Tools keep their original names; a name
  already taken is exposed as `<server>__<tool>`, and if that is taken too,
  as `<server>__<tool>_2`, `_3`, ... Nothing becomes unreachable. Names are
  fixed the first time the client sees or uses them (`tools/list`, a call,
  the catalog): from then on a name never moves to another tool for the
  life of the gateway, even when an earlier server starts late, a server
  goes down or a tool disappears — such a name answers with an error for
  its own server, never with another server's tool. Before that point the
  file order decides.
- D7. Structural operations (unique, sort, limit, select) run BEFORE
  rendering, on typed data. Text operations (join, replace, truncate, mask,
  drop/keep lines) run AFTER rendering, on text. The validator rejects an
  operation in the wrong phase.

---

## 2. PLACE IN THE SYSTEM

```
Agent (any MCP client)
   | stdio, JSON-RPC
   v
toolfmt gateway  ── one process, one binary ──────────────────────────────
   | 1. spawns every upstream from upstreams.json (stdio), lists their tools
   | 2. takes override params (_render, ...) out of the arguments
   | 3. calls the upstream tool, gets the MCP result
   | 4. captures it as a fixture (first N distinct outputs per tool)
   | 5. pipeline in-process: DETECT -> RESOLVE -> ROOT/PRE -> RENDER -> POST
   |    -> NOTES/CURSORS -> HEADER; template rules run any_txt_parser as a
   |    library on the payload in memory (no files, no subprocess, no HTTP)
   v
Agent receives: untouched result | "[<render>]\n<text>" | "[fallback:raw] <why>" + raw
```

`toolfmt serve` keeps the revision-2 HTTP contract (`POST /clean` on
127.0.0.1:8787) so the Python mcp_gateway.py can use the Rust formatter
unchanged. `toolfmt clean` is the same contract on stdin/stdout.

Memory: the payload is parsed once; renderers work on borrowed views (PRE
operations reorder and drop references, only `select` builds new objects);
the untouched path forwards the upstream result object itself.

### What the gateway does with an MCP result

| Upstream result | Agent receives |
|---|---|
| `isError: true` | exactly the upstream result (errors are never formatted) |
| no standard (and auto off or not smaller) | exactly the upstream result (D1) |
| standard applied | the text block `"[<header>]\n<text>"` takes the place of exactly what the payload was made of (below): structuredContent and the text blocks that are its JSON copy, all the text blocks it joined, or, for image_meta, the one image it described. It goes where the first of them was and carries the fields they all share (`annotations`, `_meta`). Every other block (other text, images, audio, resources, links) keeps its place; every other field of the result (`_meta`, `isError`, extensions) stays |
| standard failed | `"[fallback:raw] <reason>"` block, then every original block |

Input for the pipeline: `structuredContent` when present (typed JSON, no text
parsing); else the single text block; several text blocks that are all JSON
become one JSON array, otherwise they are joined with newlines; no text but
an image block -> a `data:` URI.

`tools/list` exposes every upstream tool with its original schema and
annotations, minus `outputSchema` (a formatted result is text, so an output
schema would be false). Two meta-tools are added (section 8). The gateway
answers `tools/list` once every server has started or `startup_wait`
(default 30 s) has passed; servers that come up later trigger
`notifications/tools/list_changed`. An upstream `list_changed` is followed.
A dead upstream is restarted on the next call (at most every `restart_backoff`,
default 10 s); of many calls that find it dead at once exactly one restarts
it, the others wait for it. A line on a server's stdout that is not JSON, or
not even UTF-8, is skipped; it never ends the connection.

**Protocol versions.** toolfmt speaks MCP 2025-06-18, 2025-03-26 and
2024-11-05. A client gets the version it asked for when it is one of these,
else 2025-06-18. Every upstream is offered 2025-06-18; one that answers a
version outside the list, or none, is disconnected and shown as failed in
`formatter_inspect` with the reason. Tool definitions and results are
forwarded as they are, not translated between versions.

**Cancellation and progress.** A client `notifications/cancelled` for a
call in progress is passed to the upstream with the upstream's own request
id and the client's reason; the call then gets no answer (MCP). A call
cancelled before it reached the upstream is never sent. When the gateway's
own `timeout` runs out it cancels the upstream request the same way
(reason `timeout`) and answers with an error. The call's `_meta` (with its
`progressToken`) goes to the upstream, and the upstream's
`notifications/progress` go back to the client unchanged.

**Scope: a gateway for tools.** Only tools are aggregated. `resources/*`
and `prompts/list` answer empty lists and are not offered in the
capabilities. Requests from an upstream to its client are not forwarded:
`ping` is answered, `roots/list` gets an empty list, anything else
(`sampling/createMessage`, `elicitation/create`, ...) gets a
method-not-supported error, so an upstream that depends on them fails
there. Upstream log messages (`notifications/message`) are dropped.

### 2.1 Catalog: schemas on demand

Every tool schema in `tools/list` is sent to the model with every request.
With many servers that costs more than any output formatting saves (34
tools: 13 708 chars per request; 123 tools: several times that), and a long
flat list makes tool choice worse. In catalog mode the gateway lists only:

- `catalog` — the tree of sections and tools, a search, tool cards;
- `call_tool` — calls any tool by name (`tool`, `args`);
- the pinned tools (`pin`, globs over listed names) as ordinary tools;
- the two formatter meta-tools (section 8).

The same 34 tools then cost 1 988 chars per request; a schema is paid for
once, when the agent asks for the card of the tool it is about to use.

`expose` in upstreams.json: `all` (flat list), `catalog`, or `auto`
(default): catalog when there are more than `catalog_threshold` (default
30) tools, flat otherwise. Tools stay callable under their own names in both
modes.

`formatter_tools` (default `true`): whether the agent is offered
`formatter_inspect` and `formatter_save_standard`. With `false` they are
left out of `tools/list` and of the initialize instructions, and a call to
them is answered as an unknown tool. For a working agent, not for the one
writing standards: in measured runs local models spent their limited turns
inspecting and saving standards instead of answering. Standards are still
applied; they are written with the CLI or by an agent on another gateway.

**Tree.** Each server's tools go to its `section` (default: the server
name; a path such as `math/cube` nests). `groups` (tool-name glob ->
subsection) splits a server further. A section that still holds more than
20 tools is split by name prefix (`cubint_add`, `cubint_mul` -> `cubint`)
when that gives at least two groups of three or more. A section is shown as
`name subsections_tools — description` (`math 2_187`); descriptions come from
`sections` in upstreams.json, else from the first sentence of the server's
`instructions`.

**catalog** arguments:

| Args | Answer |
|---|---|
| none | top sections with counts |
| `path` | that section: subsections with counts, its own tools (name, `(ro)` read-only / `(!)` destructive hint, first sentence of the description) |
| `query` | tools ranked by the words in name (x3), description and section; the best 15 |
| `tools: [names]` | one card per tool, see below |

**Card** — the "methods, metadata, price" of a tool:

```
search_text [parsers]
<description, up to 600 chars>
call: search_text(root?: string, pattern: string, max_hits?: integer, context?: "auto"|"line"|"sentence")
  root: project root inside the server root ...
output: formatted by standard (objects_table) | as the tool returns it (no standard)
measured: 12 calls, avg 1549 -> 1122 chars, avg 35 ms, 1 errors
price: $0.01/call
example: {"pattern":"pub fn render","max_hits":40}
```

The signature replaces the JSON schema (types, `?` optional, defaults,
enums). `measured` comes from the audit log, so it covers earlier sessions
too; "price" in the sense of cost to the context is measured, never
estimated. A money price is declared per server (`price`) or per tool glob
(`prices`) and shown as written. `example` is the arguments of a real
captured call.

**call_tool** runs exactly the path of a direct call (overrides,
formatting, captures, audit). `args` may also be a JSON string. Because no
client checked those arguments against the schema, missing required
parameters are reported with the signature before the upstream is called;
an unknown name answers with the closest search hits.

---

## 3. PIPELINE

### 3.1 DETECT — shape

- empty       — empty text, `[]`, `{}`
- scalar      — JSON scalar, or one line under 200 chars
- json_obj    — JSON object
- json_list   — JSON array of objects
- matrix_2d   — JSON array of non-empty arrays of numbers
- num_list    — JSON array of numbers
- str_list    — JSON array of strings
- json_mixed  — any other JSON array
- log         — at least 3 lines and 40 % of lines carry a level marker
                (`INFO`, `[warn]`, `ERROR:` ...)
- image       — `data:image/...` or a one-line path ending in an image
                extension
- text        — everything else

A JSON string payload is detected by its decoded text.

### 3.2 RESOLVE — first match wins

1. `_template` (inline YAML) replaces any standard for this call.
2. Tool standard `standards/<server>.<tool>.yml` (or `.yaml`).
3. Family standard `standards/family/<family>.yml`; the family of a tool
   comes from `families` in upstreams.json (tool-name glob -> family).
4. Built-in auto mode, only where enabled, only lossless, and only when the
   result is shorter than the raw output:
   matrix_2d -> matrix_table; json_list and envelopes (an object holding an
   array of objects) -> objects_table; pretty-printed JSON -> compact json.
5. Nothing: the result goes out untouched (D1).

`_render`, `_pre`, `_post`, `_limit` modify whatever 1-4 resolved (or a
standard built for the detected shape). `_keys` filters template rules.
A standard file that does not compile gives a fallback (D2) naming the error.

### 3.3 ROOT and PRE

`root: data.items` selects the node to render. Fields beside the path
(totals, cursors) are kept and printed as `key: value` lines before the
render. Without `root`, list renders on an object pick the envelope list
automatically (the only array field, else the longest array of objects).
PRE operations then run on the items of the list.

### 3.4 RENDER

One function per render: `(input, standard) -> text`. A render that does not
fit the input returns a reason; the pipeline falls through to auto mode
(when enabled) and otherwise to the raw output with
`[fallback:raw] <render>: <reason>`. It never crashes.

### 3.5 POST

Text operations on the rendered text, in list order.

### 3.6 NOTES and CURSORS

Every removal is declared after the text, one line each: `… 37 more rows`,
`… 3 duplicate items removed`, `… 12 lines dropped (drop_lines)` (I3).
Pagination cursors of the raw document (I8: `cursor`, `next_cursor`,
`nextPageToken`, `has_more`, `continuation`, `nextLink`, ... at the top level
or under `meta`/`pagination`/`pageInfo`/`links`) are appended as
`cursor: <key>=<value>` unless the render reports that it wrote that exact
field out: a `key: value` envelope line, the whole document or envelope
(`json`, passthrough), or an `expression` placeholder `{result.<path>}`.
Whether the text happens to contain the value is not evidence: a page may
mention another page's cursor. POST operations void the report.

The built-in auto table (`objects_table`) is used only when every cell reads
back as its value: a cell holding a nested array or object, which would
print like a string holding the same JSON, or an item repeating a key,
which would show one of the values, leaves the output as received. An
explicit `objects_table` standard falls back (`[fallback:raw]`) on a
repeated key. In `key: value` lines a string that reads as another type
(`null`, `true`, `42`, `[...]`, `{...}`, `"..."`) is quoted.

### 3.7 HEADER

First line of a formatted result: `[<render>]`, `[builtin:<render>]`,
`[template[k1,k2]]`, with `:lossy` or `:interpretive` appended when the
result is not lossless. Loss = max(declared `loss`, what actually happened:
a cut, a projection, a dropped line, a rounded value, a template extraction).
Lossless results are unmarked. Untouched results have no header at all.

The `used` value of the HTTP contract and the audit log keeps the
revision-2 vocabulary: `objects_table:<tool>`, `template:<tool>[keys]`,
`builtin:<shape>-><render>`, `passthrough`, `fallback:raw`.

---

## 4. CONTRACTS

### 4.1 MCP (gateway)

Standard MCP over stdio. Override parameters are accepted in the arguments
of any tool and removed before the upstream call — unless the upstream tool
itself declares a parameter of that name, which then stays a real argument:

| Param | Meaning |
|---|---|
| `_render` | render type for this call |
| `_pre`, `_post` | chains (JSON array, or YAML text) |
| `_template` | a whole standard as YAML text (or object) |
| `_limit` | cap: PRE `limit_items` for list renders, `max_rows` for dict_table, POST `truncate_lines` otherwise |
| `_keys` | template rules to run (array or "a,b") |

They are not added to tool schemas (that would cost tokens on every tool);
the agent learns them from `formatter_inspect topic=schema`.

### 4.2 HTTP `/clean` and `toolfmt clean`

```
POST /clean   {"server","tool","args","data","keys","_render","_pre","_post",
               "_template","_limit","_keys","auto"?,"family"?}
-> {"text","used","loss","header","raw_chars","out_chars"}
GET /health   -> {"ok":true,"version"}
```

`text` is the formatted text without the header (the raw data when nothing
applied). Requests carrying an `Origin` header (browsers) are refused.
Characters are counted as Unicode scalar values.

---

## 5. RENDERERS

| Type | Input | Output (default) | Params |
|---|---|---|---|
| passthrough | any | as is (for POST chains) | — |
| json | JSON | compact JSON of root/list after PRE | root |
| expression | args + result | `2 + 3 = 5` | format |
| matrix_table | 2D array | `RxC` line + space-separated rows; `mode: markdown` | max_rows, max_cols, formats |
| objects_table | list of objects | TOON; `mode: markdown` | columns (dotted paths), rows/max_rows, max_cell_chars, formats |
| dict_table | object | `key: value` lines; `mode: markdown` | max_rows, max_cell_chars, formats |
| template | any | `key: value` / `key[N]:` + `- item` | input, rules (any_txt_parser) |
| log | log text | level counts, unique errors and warnings with `×N` | max_items (10) |
| num_stats | list of numbers | count, min, max, sum, mean (exact fraction for integers) + sample | sample (5) |
| tree | list of paths | common prefix, then one line per directory: `dir/: a, b` | max_items |
| image_meta | data URI / path | `image: png 1920x1080, 245.3 KB` | — |
| image_verdict, image_downscale | | planned, rejected by the validator | |

Defaults favour compactness (TOON, plain grid, lines); markdown is opt-in.

TOON: `items[2]{path,size}:` then one indented row per item, values joined
with `,`. A string is quoted (JSON escaping) when empty, padded, containing
`,` `"` `\` or a line break, or when it would read as another type (`true`,
`null`, `42`, `007`). A missing key is an empty field; nested values are
compact JSON.

Expression: `{name}` refers to a call argument, `{result}` to the payload,
`{result.path}` / `{points[0]}` to data inside them, `{x:thousands}` /
`{x:bytes}` / `{x:fixed:2}` formats a number, `{{` `}}` are braces. Names are
data lookups only: no calls, no attributes, no eval (I9). A missing name is a
render failure (fallback, D2).

Value formats (`formats: {key: fmt}`, `"*"` for every value): `thousands`
(lossless), `bytes` (1024-based, one decimal; lossy when rounded),
`fixed:N` (lossy when rounded). They apply to typed numbers in cells, never
to text, so years, IDs and line numbers are not touched.

---

## 6. OPERATIONS

### PRE (on items, in order)

| Op | Effect | Loss |
|---|---|---|
| `unique` | drop repeated items (exact JSON equality) | declared count |
| `sort: true` / `sort: key` / `sort: {by: key}` | stable sort; null < bool < number < string < array < object; integers compare exactly at any length | — |
| `sort_desc` | same, reversed | — |
| `limit_items: N` | keep the first N | `… M more items` |
| `select: [a, b.c]` | project objects onto keys (dotted paths allowed) | lossy if anything dropped |

### POST (on text, in order)

| Op | Effect |
|---|---|
| `join: ", "` | lines -> one line |
| `replace: {from, to, regex}` | literal by default; `regex: true` allows `$1` |
| `truncate_chars: N` | cut each line to N chars + `…` (declared) |
| `truncate_lines: N` | keep N lines (declared) |
| `drop_lines: re` / `keep_lines: re` | filter lines (declared) |
| `mask: true` / `mask: [names or regexes]` | built-ins: api_key, token, password, jwt, bearer, private_key, aws, email (email only by name) |

The validator rejects: PRE ops in post and POST ops in pre (D7);
`truncate_chars` / `truncate_lines` after a table render (I7: use
`max_cell_chars`, `rows`, `limit_items`); `thousands` / `bytes` as POST ops
(they are value formats).

Regex: the Rust `regex` engine has no backtracking and runs in linear time,
so a pattern from a standard can never hang the process and needs no
timeout (I10); output stays deterministic (I2). Lookaround and
backreferences are not available; a standard using them is rejected when it
is compiled.

---

## 7. STANDARD SCHEMA (version 3)

```yaml
version: 3                    # 2 is accepted
description: free text        # optional
loss: lossless                # optional: lossless | lossy | interpretive
render: objects_table
root: items                   # optional path
mode: toon                    # per render, see section 5
columns: [path, size]
rows: 20                      # objects_table (alias max_rows)
max_rows: 15                  # matrix_table / dict_table
max_cols: 15
max_items: 10                 # log / tree
max_cell_chars: 80
sample: 5                     # num_stats
format: "{a} + {b} = {result}"   # expression
formats: {size: bytes}
input: {format: json}         # template
rules: [...]                  # template: any_txt_parser rules
pre:  [{sort_desc: size}, {limit_items: 50}]
post: [{drop_lines: "^\\s*$"}]
```

Unknown fields are errors. Revision-2 fields `name`, `server`, `family` are
accepted and ignored (identity is the file name; families live in
upstreams.json). `toolfmt check` validates every file.

---

## 8. ONBOARDING A NEW SERVER

1. Register: one entry in upstreams.json. All tools work at once,
   untouched.
2. Capture: the gateway stores the first real outputs of each tool (up to
   `capture_limit`, default 5, distinct by content hash, at most 2 MB each)
   in `fixtures/<server>.<tool>/NN.json`, with the call arguments. Secrets
   are masked first (built-in mask patterns, values of keys like `password`
   or `token`; pagination tokens are kept). `"capture": false` turns it off
   per server.
   Nothing is ever called just to capture output: a capture comes from a
   call the agent made anyway. `formatter_inspect` lists the tools marked
   `readOnlyHint` that have no capture yet — those are safe to call once.
   (The hint is the server's claim, not a guarantee.)
3. Author: the agent uses two meta-tools.
   - `formatter_inspect` — no args: every tool with standard, captures,
     calls, savings, plus the read-only tools not called yet;
     `tool=<name>`: description, params, current standard, each capture with
     how it renders now, and the latest capture's payload (`capture=N` for
     another one in full); `topic=schema`: the standard reference.
   - `formatter_save_standard tool=<name> yaml=<standard>` — compiles the
     standard, refuses it when the tool has no capture (D3), replays it on
     every capture and returns how each one looks. A standard that falls
     back to raw output on any capture, or that no capture has a payload
     for, is not saved (an error with the replay) unless `force: true` is
     given. Then it writes `standards/<server>.<tool>.yml` and records the
     outputs as baselines in the captures; the answer says in how many of
     them, and names every capture whose baseline could not be written and
     why. `dry_run: true` only tests (and says whether saving would be
     refused); `delete: true` removes the standard.
4. Bind: the saved file applies from the next call (the store re-reads a
   file when its modification time changes).

The initialize result carries a two-sentence `instructions` text pointing the
agent to these tools.

---

## 9. IMAGES

`image_meta` reads the header of PNG, JPEG, GIF, WebP and BMP from a data URI
or a file path and returns format, size and weight. In the gateway an image
block is replaced by that line only when the standard (or `_render`) asks for
image_meta; otherwise images are forwarded as the upstream sent them.
`image_verdict` (local vision model; endpoint missing -> CAPABILITY_MISSING,
never a faked verdict) and `image_downscale` are planned.

---

## 10. INVARIANTS

- I1. No data loss on failure: any failure -> raw output + note.
- I2. Determinism: same input + same standard -> same output (no timeouts,
  no clocks, no randomness in the pipeline).
- I3. Every removal is declared after the text.
- I4. The formatter writes only inside its home directory: `standards/`
  (by formatter_save_standard), `fixtures/` (captures, baselines) and
  `audit.jsonl` (one line per call: server, tool, used, raw/out chars, ms;
  rotated at 10 MB; `"audit": false` turns it off). Never anywhere else.
- I5. mask is optional (export of outputs), not a mandatory layer.
- I6. The formatter does not know the agent (D5).
- I7. Cuts happen at boundaries: whole rows, whole lines, whole items.
- I8. Pagination cursors are never dropped (3.6).
- I9. expression has no eval, calls or attribute access.
- I10. Regexes run on a linear-time engine.
- I11. Numbers are never reinterpreted: a number keeps the exact text the
  upstream wrote (`5.0`, `1e16`, a 40-digit integer), object keys keep their
  order, duplicate keys survive; call arguments reach the upstream with the
  same digits. `template` rules keep them too: the parser's JSON adapter
  returns every selected number with its source text, also inside a
  selected object or array (its filters compare integers beyond 64 bits as
  doubles).

---

## 11. EXTENSION MODEL

- New render = one function in render.rs + one line in `Render`.
- New PRE/POST op = one enum variant + one parse arm + one apply arm.
- New family = one yml file + one glob in upstreams.json.
- New server = one entry in upstreams.json.
- New standard = one yml file, written against a captured output (D3).

---

## 12. CONFIGURATION (upstreams.json)

```json
{
  "mcpServers": {
    "cube_v5": {
      "command": "python", "args": ["cube_server.py"],
      "env": {"KEY": "value"}, "cwd": "C:/cube",
      "auto": false, "capture": true, "timeout": 600, "init_timeout": 60,
      "restart_backoff": 10,
      "families": {"cubint_*": "cube_arith", "vec_*": "cube_vec"},
      "section": "math/cube",
      "groups": {"cubint_*": "arith", "vec_*": "vectors", "e8_*": "e8"},
      "price": "free", "prices": {"render_*": "$0.04/call"},
      "disabled": false
    }
  },
  "expose": "auto", "catalog_threshold": 30, "pin": ["read_file"], "formatter_tools": true,
  "sections": {"math": "exact arithmetic, vectors, E8 lattice"},
  "capture_limit": 5, "audit": true, "startup_wait": 30,
  "defaults": {"auto": false, "capture": true}
}
```

`expose`, `catalog_threshold`, `pin`, `formatter_tools`, `sections`, `section`, `groups`,
`price`, `prices`: section 2.1.

The same layout as common MCP client configs, so an existing `mcpServers`
block can be pasted. Server order is file order (collision policy, D6).
Only stdio servers are supported (HTTP upstreams: roadmap). On Windows
`npx`-style commands are resolved through PATHEXT and children start without
a console window.

Home directory: `--home`, else `$TOOLFMT_HOME`, else the directory of
`--config`, else the directory of the executable.

---

## 13. TESTS

- Unit tests per module (JSON fidelity, detection, operations, renders,
  standards validation, masking, captures under parallel calls).
- `tests/gateway.rs`: agent -> gateway -> two demo upstreams (`toolfmt
  demo-server`): tool collision, outputSchema removal, big-integer
  arguments, untouched results, capture, D3 refusal, I7 refusal, saving a
  standard through the meta-tool, the cursor line, overrides vs a real
  `_limit` parameter, errors untouched, image_meta, a broken standard
  falling back, inspect, capture off, unknown tool, then `toolfmt replay`
  over what was captured. A second scenario runs the same servers in
  catalog mode: the short list, tree with counts and descriptions, search,
  call_tool (object and string args, capture, collisions), cards with
  signature, measurements, prices and example, missing-argument and
  unknown-tool answers, and measurements surviving a restart.
- `tests/corpus.rs`: the golden corpus in `examples/home` (standards,
  captured real outputs of the demo server, edge fixtures: empty, malformed,
  error text, keys, no standard, auto mode, overrides, bad inline template,
  scalar, family). Every baseline was reviewed by hand before being frozen.
- `tests/failures.rs` (demo server with `--test-modes`): a crash mid-call
  and the restart, a timeout that leaves the connection usable, junk and
  invalid UTF-8 on upstream stdout, broken bytes from the client, requests
  from the server to its client, a paged `tools/list`, an upstream
  `list_changed` followed to the client, 120 parallel calls over two
  servers, a 10 MB output formatted through the gateway.
- `tests/proxy.rs`: names that stay with their tool when the earlier server
  starts late (a destructive tool), a client cancellation reaching the
  upstream with its reason and no answer sent, progress passed through,
  protocol negotiation both ways, one restart for 24 calls after a crash,
  a formatted result that keeps every block and field it did not replace,
  a standard that falls back refused without `force`, a baseline that
  cannot be written reported. Unit tests in `src/gateway.rs` cover naming
  (file order before fixing, fixed names surviving a server going down,
  double collisions, reserved names), the restart claim, and the result
  assembly (several text blocks, annotations, structuredContent with and
  without its copy, several images).
- `tests/http.rs`: `serve` on a free port — health, `/clean` with and
  without a standard, a multi-megabyte UTF-8 body, refusal of `Origin`,
  bad JSON and unknown paths, parallel requests.
- `tests/renders.rs`: every render, mode and operation on exact small
  cases (TOON quoting, markdown escaping, caps and their notes, formats,
  envelopes, cursors, declared loss, fallbacks).
- `tests/fuzz.rs` (fixed seed, no dependencies): JSON round trips, the
  parser on garbage and cut documents, and every render and chain on random
  payloads — no input may make the pipeline panic.
- `toolfmt replay --home <dir>` replays any corpus; `--update` records new
  baselines (review them before committing).

Measured (release build, Windows, gateway + demo upstream, peak working set
of the gateway): idle 5 MB; a 31 MB output without a standard 67 MB and
0.5 s; the same output sorted and cut by an objects_table standard 256 MB
and 0.7 s (the parsed tree); back to about 6 MB after the call.

Revision-2 fixtures (`fixtures/<server>.<tool>/<kind>/fixture.json` with
`baseline: {text, used}`, flat or under `request`) are read as they are.
Python/Rust byte equality is not the acceptance criterion any more:
revision 3 changes outputs on purpose (section 14). Import the old corpus,
run `replay`, review each difference, then `replay --update`.

---

## 14. CHANGES FROM REVISION 2

- D1 made literal: no auto mode by default, no `[passthrough]` header, the
  result object forwarded as is (images, structuredContent, isError).
- Collisions are exposed as `<server>__<tool>` instead of being unreachable.
- Legacy bare-name standards (`standards/<tool>.yml`) dropped: they broke D6.
- The family of a tool is configured in upstreams.json (revision 2 kept it
  inside the very file whose absence it was meant to cover).
- Loss marker = max(declared, actual); `loss` is optional.
- `thousands` / `bytes` moved from POST text operations to value formats.
- `truncate_*` after table renders rejected (I7).
- Envelopes (`{items: [...], next_cursor}`) handled by `root` / auto-root,
  siblings kept; cursors appended when a render hides them (I8 enforced).
- Defaults: TOON for objects_table, plain grid for matrix_table, lines for
  dict_table (fewer tokens); markdown on request. New render `json`.
- Header shortened to `[<render>]` (the agent knows which tool it called).
- Override params are removed only when the upstream does not declare them.
- `outputSchema` removed from listed tools; structuredContent used as input.
- Regex: linear-time engine instead of time limits (I10 without
  nondeterminism).
- Numbers and key order preserved exactly (I11).
- Captures, meta-tools, D3 enforcement, audit log (section 8, I4).
- 3.1: catalog mode — a tree of sections with counts, search, tool cards
  with signature, measured cost, declared price and example, and
  `call_tool`; on by default above 30 tools (section 2.1).
- 3.1.2: names fixed once shown and every collision resolved (D6);
  client cancellations and progress forwarded; protocol versions checked
  both ways; a formatted result replaces only its payload's parts; one
  restart for many callers; `formatter_save_standard` refuses a standard
  that falls back unless `force`, and reports baseline write failures.
- 3.1.3: the built-in table never misleads about a type: a nested array or
  object in a cell (it would print like a string holding the same JSON)
  or a repeated key makes auto mode pass the output on unchanged, and an
  explicit table standard falls back on a repeated key; envelope values
  that read as another type (`null`, `42`, `[...]`) are quoted. A cursor
  counts as shown only when the render reports writing that exact field
  (I8); text that merely mentions it no longer does. `formatter_tools:
  false` hides the formatter from a working agent (section 2.1).

---

## 15. ROADMAP

1. image_verdict / image_downscale.
2. Artifact store: a large output -> local file + compact reference.
3. HTTP (streamable) upstreams.
4. Standards that depend on arguments (`when:` on args) for tools whose
   output shape changes with them.
5. A shared library of reviewed standards for popular MCP servers.
6. Catalog: optionally turn the tools an agent asked cards for into native
   tools (`tools/list_changed`) for clients that support it.
7. Zero-copy parsing for the standard path: the parsed tree costs about 8x
   the payload at its peak (strings and numbers could borrow from the
   message instead of being copied).
8. Beyond tools, if needed: resources and prompts aggregated, upstream
   sampling/elicitation forwarded to the client.

(Done since revision 3: exact numbers inside template rules — the parser's
JSON adapter keeps number text, see I11; forwarding of progress
notifications and cancellations.)

---

## 16. PUBLICATION

Open source (MIT), no keys, no telemetry, no obfuscation. Ships next to
any_txt_parser (Opterium repository), which it uses as a library for
template rules. Community standards are plain YAML files with their
fixtures.

---

## 17. FILES

- `src/json.rs` — order- and number-preserving JSON
- `src/detect.rs` — shapes
- `src/standard.rs` — schema, validation, store
- `src/ops.rs` — PRE / POST operations, value formats
- `src/render.rs`, `src/tpl.rs`, `src/image.rs` — renders
- `src/pipeline.rs` — RESOLVE ... HEADER, `/clean` contract
- `src/gateway.rs`, `src/rpc.rs`, `src/meta.rs` — MCP gateway, meta-tools
- `src/catalog.rs` — catalog tree, search, tool cards, signatures
- `src/capture.rs` — captures, payload extraction, audit log
- `src/config.rs` — upstreams.json
- `src/http.rs`, `src/replay.rs`, `src/demo.rs`, `src/main.rs` — CLI modes
- `examples/home/` — demo home: upstreams.json, standards, golden fixtures
- `tests/` — gateway end-to-end, corpus replay
