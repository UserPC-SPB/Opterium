# any_txt_parser — exact data and project maps for AI agents

One portable executable, no installer, no runtime, no network. Two modules
behind one command line (and one MCP server):

- **parser** — pulls exactly the lines, values or elements you need out of
  large or many files: logs and plain text, JSON, YAML, TOML, XML, HTML,
  Markdown. Deterministic; every result says whether data was cut or missing.
- **navguard** — maps a code project: tree, files with line/token estimates,
  functions/classes with line ranges; finds symbols, prints exact line ranges,
  searches text, lists TODOs, reports what changed.

## Why use it instead of reading files yourself

- **Context**: get the 30 lines that matter instead of a 50 MB log or a
  whole repository. Large inputs stay outside your context window.
- **Exactness**: selection is done by a rule, not by skimming; nothing is
  dropped silently — truncation, skipped files and failures are reported.
- **Navigation**: know where a function lives and how big a file is before
  opening it; read only the line range you need.
- **Repeatable**: the same rule on the same files gives the same output.

For a small file you are about to read anyway, just read it. It is fast: a
50 000-line, 1.8 MB log is searched in tens of milliseconds.

## Start here

    any_txt_parser recon logs/                                 # formats, encodings, shape, first lines
    any_txt_parser extract -i logs/ -r contains:ERROR          # matching lines from every text file
    any_txt_parser extract -i data.json -r "jsonpath:$.users[*].email" --json
    any_txt_parser map --root .                                # project tree with sizes
    any_txt_parser symbols src/main.rs                         # functions/classes with line ranges
    any_txt_parser find parse_config --exact --root .          # where is it defined?
    any_txt_parser lines src/main.rs 120 180                   # just those lines
    any_txt_parser mcp --root .                                # serve all of this over MCP

## Commands

| Command | Module | Purpose |
|---|---|---|
| `extract` | parser | pull matching data out of files (`help extract`, `help rules`) |
| `recon` | parser | inspect files first: format, encoding, shape, sample lines (`help recon`) |
| `map` | navguard | project tree and files with line/token estimates |
| `symbols` | navguard | functions, classes, ... with line ranges |
| `find` | navguard | find symbols by name (`--exact`: the whole name) |
| `lines` | navguard | numbered line range of a file |
| `search` | navguard | text search: the line in code, the sentence in prose |
| `todos` | navguard | TODO / FIXME / HACK / XXX marker lines |
| `changes` | navguard | files new / changed / deleted since the last snapshot |
| `nav` | navguard | the full navguard command line (watch mode, navigator files, patch, ...) |
| `mcp` | both | run as an MCP server on stdio (`help mcp`) |
| `help` | — | this documentation: `help <topic>` |

Report mode (the original interface): `any_txt_parser -i <input> -t <template.yml> -o <out.md>`
writes a self-describing Markdown report with block anchors (`help reference`).

## Output and exit codes

Text output is compact and made to be read by a model; add `--json` for
structured output. Exit codes: 0 success · 1 partial (no match, data cut,
some file failed) · 2 invalid rule or template · 3 invalid option/input, no
file matched, or every file failed · 4 output failure · 5 internal error (a
bug) · 6 resource limit.

## Core rules

1. Look before you extract: `recon` the files, `map` the project, and write
   rules from what you saw.
2. The exit code is the truth. 0: complete. 1: partial — read every `note:`
   and `warning:` line before you use the data. 2, 3: nothing was extracted;
   the message says what to change.
3. Ask for the smallest thing that answers the question: one rule, the files
   that matter, one line range. Raise a cap only when a note says something
   was cut.
4. Line numbers hold for the file as it was read. After an edit ask again
   (`symbols`, `find`, `extract`; `nav --oneshot` for the navigator files).
5. In YAML templates write regexes in single quotes; on the command line
   quote rules for your shell (`help extract`).

## When something goes wrong

| You see | Meaning | Do |
|---|---|---|
| exit 1, `found nothing in the N file(s) it read` | the rule matched nothing | `recon` the file, check its shape, loosen the rule |
| `skipped files it cannot read`, `read no file` | the rule type does not fit those formats | a selector for that format (`help rules`) or `--format text` |
| exit 1, `output truncated`, `note: showing N of M` | a cap cut the result | raise the cap the note names, or narrow the rule |
| exit 2, `invalid regex`, a template error | the rule or template does not parse | fix what the message points at |
| exit 3, `no input files matched`, `file not found` | wrong path or glob | paths start in the working folder (navigation: in `--root`) |
| exit 3, `path is outside the project root` | the path leaves `--root` or the MCP server root | give the `--root` of the folder that holds the file |
| exit 6 | a resource limit (files, size, memory) | narrow the input or raise the limit the message names |
| `nav`: exit 1, `ERROR: ...` | the mode failed for the reason given | e.g. `--patch`: text not found or line out of range; read the lines again |

## Help topics

    any_txt_parser help extract      rules on the command line, options, output format
    any_txt_parser help rules        shorthand TYPE:EXPR, rule objects, selectors, transforms
    any_txt_parser help recon        inspecting files
    any_txt_parser help navigation   map, symbols, find, lines, search, todos, changes, nav
    any_txt_parser help mcp          MCP server, client setup, tool list
    any_txt_parser help reference    template DSL and the Markdown report, in full
