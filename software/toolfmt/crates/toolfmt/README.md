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
- Rust, one process: the formatter and the parser (any_txt_parser, used for
  `template` rules) run in memory on the payload — no HTTP hop, no temporary
  files, no subprocess per call.
- Exact: numbers keep the digits the tool wrote, key order is kept, every
  cut is declared (`… 37 more rows`), pagination cursors are never dropped,
  a broken standard gives the raw output plus a note.
- The authoritative specification is [SPEC.md](SPEC.md).

## Build

```bash
cargo build --release -p toolfmt
```

The binary is `target/release/toolfmt(.exe)`.

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
`_2`, `_3`), plus two tools of toolfmt itself. Once the agent has seen the
names they stay put for the session: a server that starts late never takes
over a name the agent already uses for another server's tool.

The two own tools, `formatter_inspect` and `formatter_save_standard`, are
for the agent that writes standards. For an agent that only does its work,
leave them out with `"formatter_tools": false` in `upstreams.json`:
standards still apply, the agent just is not offered the formatter. In
measured runs with a local model most failures with these tools visible
were an agent spending its turns on them instead of on the task, and none
were left once they were hidden.

## What it is, and what it is not

toolfmt is a gateway for **tools**, not a general MCP proxy:

- Tools: listed, called, formatted. Cancellations and progress
  notifications from the client pass through to the upstream call.
- Resources and prompts of the upstream servers are **not** aggregated
  (`resources/list` and `prompts/list` answer empty lists).
- Requests an upstream server sends to its client are **not** forwarded:
  `roots/list` gets an empty list, sampling, elicitation and the like get
  an error. A server that needs them will not work through toolfmt.
- MCP versions 2025-06-18, 2025-03-26 and 2024-11-05; an upstream that
  answers another version is disconnected (`formatter_inspect` says why).

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
declared money price to the cards. Details: [SPEC.md](SPEC.md) section 2.1.

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

A formatted result keeps everything the formatted text did not replace:
other text blocks, images, resources, `_meta`, extra fields.

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

`examples/home` is a complete home with the demo server, standards for each
of its tools and a reviewed golden corpus (`toolfmt replay --home
examples/home`).
