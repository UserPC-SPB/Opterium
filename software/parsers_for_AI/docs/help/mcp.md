# MCP server

`any_txt_parser mcp` runs the program as a Model Context Protocol server on
stdio (newline-delimited JSON-RPC 2.0). Any MCP client — Claude Code, Claude
Desktop, Codex CLI, Cursor, Continue, LM Studio, custom agents — can then call
the tools below directly: no template files, no reports to read back.

The server works inside one folder, the server root: `mcp --root DIR`, or
the folder the client started it in. Relative paths in tool arguments start
there. Every file a tool reads, every `root` argument, `extract` input and
template file must lie inside the server root; anything else (`..`, absolute
paths elsewhere, symlinks or junctions pointing outside) is refused with
`isError: true`. All tools and resources are read-only except `changes`
with `persist: true`, which saves a snapshot file `.navguard_cache.json` in
the project root.

## Connect a client

Put the exe anywhere (no installer, no runtime needed), e.g. `C:\tools\any_txt_parser.exe`.

Claude Code:

    claude mcp add any_txt_parser -- C:\tools\any_txt_parser.exe mcp --root C:\work\shop

Claude Desktop (`claude_desktop_config.json`) and Cursor (`.cursor/mcp.json`):

    {
      "mcpServers": {
        "any_txt_parser": { "command": "C:\\tools\\any_txt_parser.exe", "args": ["mcp", "--root", "C:\\work\\shop"] }
      }
    }

Codex CLI (`~/.codex/config.toml`):

    [mcp_servers.any_txt_parser]
    command = "C:\\tools\\any_txt_parser.exe"
    args = ["mcp", "--root", "C:\\work\\shop"]

Any other client: start `any_txt_parser.exe mcp --root <project>` as a stdio
server. Without `--root` the server root is the client's working directory.

## Tools

| Tool | What it returns | Key arguments |
|---|---|---|
| `help` | this documentation | `topic` (overview, extract, rules, recon, navigation, mcp, reference) |
| `recon` | format, encoding, shape and sample lines per file | `path` |
| `extract` | matching data from files, per file and rule; text rules give the source line of each item (`L<n>:` / `lines`) | `input`, `rules` (or `template`) |
| `map` | project tree and files with line/token estimates | `root` |
| `symbols` | functions, classes, … with line ranges | `root`, `path` |
| `find_symbol` | symbols whose name or path contains the pattern, or (`exact: true`) whose name equals it | `root`, `pattern`, `exact` |
| `read_lines` | numbered lines of a file | `path`, `start`, `end` |
| `search_text` | text matches: the line in code, the sentence in prose | `root`, `pattern`, `path`, `context` |
| `todos` | TODO / FIXME / HACK / XXX marker lines (whole words; `loose: true` = any substring) | `root`, `loose` |
| `changes` | files new / changed / deleted since the last snapshot | `root`, `persist` |

Most tools accept `output: "text"` (default, compact) or `output: "json"`.
Call `help` with a topic for the full argument description of a tool family.

## Resources

Two resources of the server root, without arguments (`resources/list`,
`resources/read`):

| URI | Type | Content |
|---|---|---|
| `project://tree` | text | the tree and every file with lines and ~tokens: the `map` tool without arguments, the root shown absolute, at most 1000 files |
| `project://state` | JSON | freshness, see below |

`project://state` fields: `root` (absolute), `checked` (now), `totals`
(`files`, `lines`, `tokens` on disk now); `navigators`: `status` — `fresh`
(navigator.md, navigator_deep.md and navigator_deep2.md come from one scan
and match every file on disk), `stale` (files were added, changed or deleted
since: `new`, `changed`, `deleted` counts and up to 50 `paths`), `incomplete`
(a file missing or a different `Updated:`) or `missing` — and `updated`
(their scan time); `snapshot` (time of the snapshot `changes` compares with,
or null); `workflow` (`triangle` `[A,B,C]`, `transitions`, `timestamp` of
`nav --triangle`). Anything but `fresh`: run
`any_txt_parser nav --oneshot --root <root>`.

    {"root":"C:\\work\\shop","checked":"2026-10-19T15:04:05+03:00","totals":{"files":42,"lines":6810,"tokens":61200},
     "navigators":{"status":"stale","updated":"2026-10-19T14:40:41+03:00","new":1,"changed":1,"deleted":0,
     "paths":["src/cart.rs","src/new.rs"]},"snapshot":"2026-10-19T14:40:41+03:00",
     "workflow":{"triangle":[0,0,1],"transitions":0,"timestamp":""}}

## Typical session

1. `recon` on the files you were given — learn formats and structure cheaply.
2. `extract` with one or more rules — get only the data you need.
3. For code: `map` → `symbols` / `find_symbol` → `read_lines` for the exact range.

Errors come back as tool results with `isError: true` and a message that says
what to change; the server keeps running.
