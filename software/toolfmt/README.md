# toolfmt — tool outputs in the format the agent chose

One small binary between an AI agent and all of its MCP tools. The agent
calls a tool as usual; the output arrives already shaped the way the agent
decided once, for that tool, in a short YAML file (a *standard*). Tools
without a standard work exactly as before: their output passes through
untouched.

```
before (list_files, 1629 chars of pretty JSON)        after (564 chars)
{                                                      [objects_table:lossy]
  "items": [                                           total: 24
    {                                                  next_cursor: page-2
      "path": "src/module_00.rs",                      items[10]{path,size,modified}:
      "size": 1000,                                      src/module_00.rs,1000 B,2026-11-01T10:00:00Z
      "modified": "2026-11-01T10:00:00Z",                src/module_01.rs,2.5 KB,2026-11-02T10:01:00Z
      "kind": "file"                                     ...
    },                                                 … 2 more rows
    ...
```

- Open source (MIT), local, no keys, no telemetry.
- Rust, one process: the formatter and the parser run in memory on the
  payload — no HTTP hop, no temporary files, no subprocess per call.
- Exact: numbers keep the digits the tool wrote, key order is kept, every
  cut is declared (`… 37 more rows`), pagination cursors are never dropped,
  a broken standard gives the raw output plus a note.

## Requires any_txt_parser

toolfmt uses [any_txt_parser](../parsers_for_AI/) as a library for its
`template` rules (exact data extraction on the payload). This folder is a
self-contained Cargo workspace: the parser source is the workspace root
crate (`any_txt_parser` v1.5.0), so the tree builds standalone with no
external checkout:

```
toolfmt/                  <- this folder (workspace root = any_txt_parser)
  toolfmt.exe             <- prebuilt binary (Windows x64, see SHA256SUMS.txt)
  Cargo.toml              <- workspace: any_txt_parser + crates/navguard + crates/toolfmt
  crates/toolfmt/         <- the toolfmt crate (formatter + gateway)
  crates/navguard/        <- navguard library
  src/, tests/, docs/     <- any_txt_parser source (bundled for standalone builds)
```

If you already keep `../parsers_for_AI/` as your parser checkout, the two
parser copies must stay in sync when you update the parser.

## Build

```bash
cargo build --release -p toolfmt
```

The binary is `target/release/toolfmt(.exe)`. The shipped `toolfmt.exe`
matches `SHA256SUMS.txt`.

## Set up

1. Make a home directory, e.g. `C:\toolfmt`, with `upstreams.json` listing
   your MCP servers. It uses the same layout as MCP client configs, so you
   can paste your existing `mcpServers` block:

   ```json
   {
     "mcpServers": {
       "blender": {"command": "uvx", "args": ["blender-mcp"]},
       "files": {"command": "npx", "args": ["-y", "@modelcontextprotocol/server-filesystem", "C:/work"]}
     }
   }
   ```

2. Register toolfmt in your MCP client instead of those servers:

   ```json
   {"mcpServers": {"toolfmt": {"command": "C:/toolfmt/toolfmt.exe", "args": ["gateway", "--home", "C:/toolfmt"]}}}
   ```

The agent now sees every tool of every server under its original name (a
name taken by an earlier server appears as `<server>__<tool>`, then with
`_2`, `_3`). Once the agent has seen the names they stay put for the
session: a server that starts late never takes over a name the agent
already uses for another server's tool.

## Many servers: the catalog

Every tool schema is sent to the model with every request. Above 30 tools
(`"expose": "auto"`, the default) toolfmt lists only `catalog` and
`call_tool` (plus tools you `pin`). The agent browses or searches:

```
catalog                      -> catalog: 3 sections, 32 tools (as name subsections_tools — description)
                                  parsers 0_10 — any_txt_parser gives you exact data from files …
                                  math 2_11 — exact arithmetic, vectors
catalog query="search text"  -> search_text — Case-sensitive text search over the project … [parsers]
catalog tools=[search_text]  -> call: search_text(root?: string, pattern: string, max_hits?: integer, …)
                                measured: 12 calls, avg 1549 -> 1122 chars, avg 35 ms
call_tool tool=search_text args={"pattern": "fn main"}
```

With 34 tools this cut what every request carries from 13 708 to 1 988
characters. Sections come from `section` / `groups` per server, a description
from `sections` or the server's own instructions; `price` / `prices` add a
declared money price to the cards.

## Teach it a format (the agent does this)

1. Call a tool once. Its real output is captured in
   `fixtures/<server>.<tool>/`.
2. `formatter_inspect tool=<name>` shows that output; `topic=schema` shows
   how standards are written.
3. `formatter_save_standard tool=<name> yaml=...` replays the standard on
   the captured outputs, shows the result, and saves
   `standards/<server>.<tool>.yml`. The next call is formatted. A standard
   that falls back to raw output on a capture is not saved unless
   `force=true`.

A standard:

```yaml
render: objects_table          # also: json, expression, matrix_table, dict_table,
root: items                    # template, log, num_stats, tree, image_meta
columns: [path, size]
formats: {size: bytes}
pre: [{sort_desc: size}, {limit_items: 30}]
```

One-off changes go into the call itself and never reach the upstream tool:
`_render`, `_pre`, `_post`, `_template`, `_limit`, `_keys`.

`formatter_inspect` with no arguments lists every tool with its standard,
captures, calls and the share of characters saved in this session;
`audit.jsonl` in the home directory keeps one line per call.

## Other modes

```bash
toolfmt format --server S --tool T --standard s.yml output.json   # try a standard on a file
toolfmt replay --home C:/toolfmt          # every fixture still renders as its baseline?
toolfmt check --home C:/toolfmt           # every standard valid?
toolfmt serve --home C:/toolfmt --port 8787   # revision-2 HTTP contract: POST /clean
toolfmt demo-server                       # a demo MCP server with typical output shapes
```

`crates/toolfmt/examples/home` is a complete home with the demo server,
standards for each of its tools and a reviewed golden corpus
(`toolfmt replay --home crates/toolfmt/examples/home`).

## For AI agents

### Map
- `software/toolfmt/`: Rust workspace; root crate `any_txt_parser` (bundled
  parser source), members `crates/navguard` and `crates/toolfmt`.
- `software/toolfmt/toolfmt.exe`: prebuilt Windows x64 binary; hash in
  `SHA256SUMS.txt`.
- `software/toolfmt/crates/toolfmt/`: the toolfmt crate — `src/gateway.rs`
  (MCP gateway), `src/pipeline.rs` (format pipeline), `src/render.rs`
  (renderers), `src/standard.rs` (standards), `src/catalog.rs` (catalog).
- `software/toolfmt/crates/toolfmt/examples/home/`: demo upstream, standards
  and fixtures for the golden corpus.

### Context budget

| path | lines | ~tokens |
|---|---|---|
| `software/toolfmt/README.md` | 187 | ~2 300 |
| `software/toolfmt/crates/toolfmt/README.md` | 126 | ~1 400 |
| `software/toolfmt/CHANGELOG.md` | 690 | ~7 600 |

### Entry points
1. Read this file for what toolfmt is and how to register it.
2. `crates/toolfmt/README.md` — same content, crate-local copy.
3. `crates/toolfmt/examples/home/` — the working example (demo server,
   standards, fixtures).

### No-surprises / gotchas
- `toolfmt.exe` is Windows x64; the gateway needs `--home` pointing at a
  directory with `upstreams.json`.
- The workspace root is the parser: `cargo build -p toolfmt` builds only the
  toolfmt binary, `cargo build` builds the parser too.
- toolfmt is a gateway for **tools**, not a general MCP proxy: upstream
  resources and prompts are not aggregated, and requests an upstream sends
  to its client (`roots/list`, sampling, elicitation) are not forwarded.
- Registering a new MCP server never requires creating standards for its
  tools. Tools without registered standards remain fully operational and
  return their original upstream output unchanged. Standards are optional
  enhancements, not prerequisites.
