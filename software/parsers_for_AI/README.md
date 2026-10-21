# any_txt_parser

**One small program that lets an AI agent get exact data out of files and find
its way around a code project — without reading everything.**

`any_txt_parser.exe` is a single portable Windows executable (Rust, statically
linked, ~6.4 MB): no installer, no runtime, no network, no configuration. Copy it
anywhere and tell your AI where it is. The program is made for AI agents; a
person does not need to learn it.

## What it gives an AI

| Without it | With it |
|---|---|
| Reads a 50 MB log or a whole folder to find the errors — the context window fills up, the rest is guessed | `extract -i logs/ -r contains:ERROR` returns just the matching lines, per file, each with its line number |
| Skims a big JSON/YAML/XML/HTML file and may miss or invent values | One rule (`jsonpath`, `path`, `xpath`, `css`, `markdown`, `regex`, ...) returns exactly the values, deterministically |
| Does not know whether data was cut or a file failed to parse | Every result says what was truncated, skipped or broken |
| Opens file after file to find where a function lives | `map`, `symbols`, `find` give the tree, file sizes in tokens and functions with line ranges; `lines` reads only that range |
| Rereads a big project at every step | `nav --oneshot` writes three linked navigator files: the tree with the absolute root and scan time, then exact line pointers to sizes and to symbols |
| Needs instructions on how to use a new tool | Built-in English help with examples (`help`), MCP tool descriptions, and this README |

Under the hood one executable contains two modules: the **parser**
(extraction) and **navguard** (project navigation). The AI picks the tool and
builds its own instruction from short keys. There is no separate navguard.exe.

## For AI agents

- **Structure:** this README → `HELP.md` (CLI/template reference) → `FULL_GUIDE.md` (long guide) → `docs/help/` (built-in help sources). Code: `src/` (parser) and `crates/navguard/` (navigation library, linked into the same exe); fixtures: `tests/snapshots/`.
- **Entry points:** `any_txt_parser.exe help`; MCP server: `any_txt_parser.exe mcp --root <dir>` (10 tools + 2 resources).
- **Budget:** `HELP.md` ~5 050 tokens; `FULL_GUIDE.md` ~7 950; `CHANGELOG.md` ~10 800. Prefer `help <topic>` over the full guide.
- **Gotchas:** Windows x64 executable; navigation is confined to `--root`; `nav --patch` is the only write command; `tests/snapshots/` contains intentionally broken fixtures; exit codes are listed below.

## Give it to an AI

**Any agent that can run commands** — tell it:

> `C:\tools\any_txt_parser.exe` is installed. Run `C:\tools\any_txt_parser.exe help`
> and use it to extract data from files and to navigate code.

**MCP clients** (Claude Code, Claude Desktop, Codex CLI, Cursor, LM Studio, ...)
— register it as a stdio server with the argument `mcp` and the project
folder it may read:

    claude mcp add any_txt_parser -- C:\tools\any_txt_parser.exe mcp --root C:\work\project

    {
      "mcpServers": {
        "any_txt_parser": { "command": "C:\\tools\\any_txt_parser.exe", "args": ["mcp", "--root", "C:\\work\\project"] }
      }
    }

The server offers ten tools: `help`, `recon`, `extract`, `map`, `symbols`,
`find_symbol`, `read_lines`, `search_text`, `todos`, `changes`, and two
resources: `project://tree` (the project map) and `project://state` (are the
navigator files still current?). Details: `any_txt_parser help mcp`.

**It stays inside the project.** Navigation commands read only inside
`--root` (default: the current folder), the MCP server only inside its root;
`..`, absolute paths elsewhere and symlinks or junctions pointing outside are
refused, and scans never follow links. `nav --patch`, the only command that
edits files, has the same limit and a `--dry-run`.

## Commands at a glance

    any_txt_parser recon logs/                                    # what is in these files?
    any_txt_parser extract -i logs/ -r contains:ERROR --context 1  # matching lines, numbered (L12:)
    any_txt_parser extract -i data.json -r "jsonpath:$.users[?(@.active == true)].email" --json
    any_txt_parser extract -i site/ -r "css:a[href]" --extract attr:href --transform unique
    any_txt_parser extract -i Cargo.toml -r "path:[package].version"
    any_txt_parser map --root C:\work\project                     # tree + sizes
    any_txt_parser symbols src/main.rs                            # functions with line ranges
    any_txt_parser find parse_config --exact                      # where is it defined?
    any_txt_parser lines src/main.rs 120 180                      # just those lines
    any_txt_parser todos                                          # TODO/FIXME/HACK/XXX markers
    any_txt_parser changes --persist                              # what changed since last time
    any_txt_parser nav --oneshot                                  # navigator.md + deep + deep2
    any_txt_parser mcp --root C:\work\project                     # MCP server on stdio
    any_txt_parser help [overview|extract|rules|recon|navigation|mcp|reference]

Formats: plain text and logs, JSON, YAML (also multi-document), TOML, XML
(DTD-safe), HTML, Markdown; UTF-8, UTF-16 and Windows-1251.

The original report mode is still there: `any_txt_parser -i <input> -t
<template.yml> -o <out.md>` writes a self-describing Markdown report with
block anchors (`help reference`). The source repository has a pair to try
it on: `-i examples/example.log -t examples/errors.yml -o report.md`.

## What changed in 1.5

- `extract` numbers the lines of text rules (`contains`, `regex`, `line`,
  ...): `- L12: text` is the match, `L11- ` a context line; `--json` adds
  `lines` (one per item) and `spans` (first and last line of multi-line
  items). Programs that parse the text output strip the prefix or use
  `--json`.
- `find NAME --exact` (MCP `find_symbol` with `exact: true`) matches the
  symbol name itself, not any name, signature or path containing it.
- `todos` counts TODO, FIXME, HACK and XXX only as whole-word markers
  (`// TODO`, `todo: x`, `@todo`), no longer words like `todos` or
  `todo_hits`; `--loose` (MCP `loose: true`) keeps the old substring match.

The full history is in `CHANGELOG.md` in the source repository.

## Exit codes

`0` success · `1` partial (a rule found nothing, data was cut, a file failed) ·
`2` invalid rule or template · `3` invalid option or input, or every file
failed · `4` output failure · `5` internal error · `6` resource limit.

## Building from source

Rust 1.97.1, pinned in `rust-toolchain.toml` (rustup installs it on the first
build), MSVC on Windows. `cargo build --release --locked`; the release script
`scripts\release.bat` runs format check, tests, the release build and copies
the exe, `HELP.md` and this README to `dist\` and the bundled skill folders.
Its build leaves no build-machine paths in the exe and the linker writes no
timestamps, so the same sources, rustc and MSVC toolset (the release prints
it) give the same bytes. `HELP.md` (template DSL reference, English) ships
with the exe; the source repository also has `FULL_GUIDE.md` (full guide),
`CHANGELOG.md`, `docs/help/` (the built-in help), `examples/` and `LICENSE`.

## License

MIT License

Copyright (c) 2026 the any_txt_parser authors

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
