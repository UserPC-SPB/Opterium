# any_txt_parser — Full Guide (v1.5.0)

The complete guide for `any_txt_parser` **1.5.0**, written for AI operators:
everything an agent needs to use the program, with verified examples. The
quick references are `HELP.md` (template/report mode) and the built-in
`any_txt_parser help [topic]`; this document is the full one.

Everything marked as an example in this guide was executed against 1.5.0
(2026-10-19, 6 443 008 bytes) and the output is reproduced as printed.

---

## 1. What it is

One portable Windows executable (Rust, statically linked, ~6.4 MB): no
installer, no runtime, no network, no configuration. Copy it anywhere and run
it. It exists so that an AI agent (or a person) can get **exact data** out of
files and **find its way** around a code project without reading everything.

Two modules behind one command line:

- **parser** — rule-based extraction from plain text and logs, JSON, YAML
  (including multi-document), TOML, XML (DTD-safe), HTML and Markdown.
  Deterministic; every result reports what was truncated, skipped or failed.
- **navguard** — project navigation: file tree with line/token estimates,
  functions/classes/structs with line ranges, symbol search, exact line
  reading, text search, TODO listing, change report, and three linked
  navigator files with exact line pointers.

Three ways to use it:

1. **Short commands** print the answer to stdout (no files written):
   `extract`, `recon`, `map`, `symbols`, `find`, `lines`, `search`, `todos`,
   `changes`, `nav`.
2. **Report mode** (the original one): `-i input -t template.yml -o out.md`
   writes a self-describing Markdown document with block anchors.
3. **MCP server**: `any_txt_parser mcp --root DIR` exposes 10 tools and
   2 resources over JSON-RPC on stdio.

## 2. Quick start

    :: put the exe anywhere, e.g. C:\tools\
    C:\tools\any_txt_parser.exe --version          ->  any_txt_parser 1.5.0
    C:\tools\any_txt_parser.exe help               ->  overview topic, lists all topics
    C:\tools\any_txt_parser.exe help navigation    ->  the navigation half
    C:\tools\any_txt_parser.exe help reference     ->  the full template DSL reference

Tell an agent simply:

> `C:\tools\any_txt_parser.exe` is installed. Run `… help` and use it to
> extract data from files and to navigate code.

For MCP clients (Claude Code, Claude Desktop, Codex CLI, Cursor, LM Studio…):

    claude mcp add any_txt_parser -- C:\tools\any_txt_parser.exe mcp --root C:\work\project

    { "mcpServers": { "any_txt_parser": {
        "command": "C:\\tools\\any_txt_parser.exe",
        "args": ["mcp", "--root", "C:\\work\\project"] } } }

## 3. Commands at a glance

    any_txt_parser recon logs/                    # what is in these files?
    any_txt_parser extract -i logs/ -r contains:ERROR --context 1
    any_txt_parser extract -i data.json -r "jsonpath:$.users[?(@.active == true)].email" --json
    any_txt_parser extract -i Cargo.toml -r "path:[package].version"
    any_txt_parser extract -i pom.xml -r "xpath:/project/version/text()"
    any_txt_parser extract -i page.html -r "css:a[href]" --extract attr:href --transform unique
    any_txt_parser extract -i README.md -r "markdown:heading:^Install" -r line:1-5
    any_txt_parser map --root C:\work\project     # tree + per-file lines/tokens
    any_txt_parser symbols src/main.rs            # functions with line ranges
    any_txt_parser find checkout [--exact]        # where is it defined?
    any_txt_parser lines src/main.rs 120 180      # just those lines
    any_txt_parser search "parse_config"          # text search
    any_txt_parser todos [--loose]                # TODO/FIXME/HACK/XXX
    any_txt_parser changes --persist              # what changed since last time
    any_txt_parser nav --oneshot                  # navigator.md + deep + deep2
    any_txt_parser mcp --root C:\work\project     # MCP server on stdio
    any_txt_parser -i <input> -t tpl.yml -o out.md    # report mode (section 5)

`extract --help`, `map --help`, … print a short per-command help; `nav --help`
lists the navguard modes.

## 4. Exit codes (all modes)

| Code | Meaning |
|---|---|
| 0 | success — every file `success` |
| 1 | partial — some files warning/error but not all error; also "no rule applies"; output is still published |
| 2 | invalid template / invalid rule / invalid expression |
| 3 | invalid CLI or input (bad `--format`/`--encoding`/SIZE, missing `-o`, no files found) — or every input file failed; in the all-failed case the output **is** published with `Run status: failure` |
| 4 | publish failure (or a failed internal output check — a bug) |
| 5 | internal error — a bug, including any panic; report the stderr text |
| 6 | resource limit exceeded (`--max-*`, template limits) |

A CLI error (2/3 before reading) publishes nothing and leaves an older output
untouched; tell it from the all-failed case by stderr or a changed `Generation`.

## 5. Report mode: invocation

    any_txt_parser -i <file|dir|glob> -t <template.yml> -o out.md [options]

- `-i` file, directory (recursive) or glob (`*`, `**/`). The `-o` file is never scanned.
- `-t` YAML template (section 6). `-o` output Markdown file — required at runtime (missing → exit 3).
- Precedence: **CLI > template > default**; overrides are recorded in the output.

| Option | Effect |
|---|---|
| `--format FMT` | force this format for ALL files: `json\|xml\|html\|markdown\|yaml\|toml\|text` (aliases md, htm, yml, txt/log/plain). Not `auto`. Invalid → exit 3 |
| `--encoding ENC` | `utf-8` (default, BOM auto-stripped), `utf-8-sig`, `utf-16le`, `utf-16` (=LE), `utf-16be`, `windows-1251`/`cp1251`. Invalid → exit 3 |
| `--dry-run` | validate template and inputs, print a plan with per-file format hints, publish nothing |
| `-v` / `-q` | diagnostics to stderr / suppress the final message |
| `--hash` | add sha256 of each source file to the output |
| `--max-files N` | max discovered files (default 5000) |
| `--max-file-size SIZE` | max size of one file (default 200m; bare bytes or k/m/g ×1024) |
| `--max-matches N` | match cap per rule (before transforms); cut matches are reported |
| `--max-memory SIZE` | cumulative decoded UTF-8 byte budget across all files (default unlimited); not a process RSS limit |
| `--max-output-size SIZE` | document character limit (default: from template, else unlimited) |
| `--max-block-chars N` | hard cap per block incl. metadata and anchors (default 32000); oversized items are split; a cap too small for metadata → exit 6 |
| `--follow-symlinks` | traverse symlinks during discovery (default: skipped) |

## 6. Template DSL (version: 1)

    version: 1
    name: template_name          # required, non-empty
    input:                       # optional
      format: auto               # json|xml|html|markdown|yaml|toml|text|auto
      encoding: utf-8
    output:                      # optional
      max_output_chars: 100000   # hard limit for the whole document
      block: {target_chars: 28000, max_chars: 32000}
    document:                    # optional
      sort: none                 # none|asc|desc — block sorting
      stable_deduplicate: false  # drop duplicate items (first kept)
      split_blocks: true         # split oversized blocks into numbered parts
    rules:                       # required, >= 1, unique ids
      - id: r1
        when: {format: json}     # optional filter — see below
        select: {type: contains, expression: "ERROR"}
        context: {before: 1, after: 1}   # text selectors only
        output: {key: errors}    # default key = rule id
        extract: text            # text | attr:NAME | inner_html | outer_html (xml/html)
        transforms: [trim]       # order matters
        limits: {max_items: 10, max_item_chars: 500}

**`when.format` is a filter, not a matcher.** A rule whose selector type does
not fit the format gets status `rule not applicable` (warning; the block names
the selector types the format takes) — the run still publishes and can exit 0
only if nothing else is wrong. A rule with `when: {format: json}` simply does
not run on other formats: those files get no block and no warning, so a
mixed-format run can exit 0. A `when.format` that contradicts a fixed format
(`--format` / `input.format`) is not an error either: the rule never applies
(stated in Applied Template, `--dry-run`, `-v`); if no rule applies at all,
every file read is `warning` (exit 1).

**Selectors by format**

| Format | Selectors |
|---|---|
| json | `jsonpath` `$.users[*].name`, filters `[?(@.active == true)]`; `path` `$.a.b[0]` |
| yaml / toml | `path` `$.settings.timeout`; toml section headers `[database].host`, `[servers.alpha].ip` |
| xml | `xpath` `/catalog/item/@id`, `//item[@type='book']/title/text()` |
| html | `css` `h2`, `a[href]` (+ `extract: attr:href`) |
| markdown | `markdown` node `heading\|paragraph\|list\|list_item\|table\|table_row\|table_cell\|code_block\|blockquote\|link\|image` (aliases h, p, li, tr, td, code, quote, img; + optional `pattern` regex) |
| text | line-based: `regex`, `contains`, `starts_with`, `ends_with`, `line {from: 1, to: 5}`. One capture group in a regex → the group is the item, else the whole line |

## 7. Selectors in depth

### 7.1 `jsonpath` (JSON)

Supported subset: no slices, no negative indices, no parentheses in filters; a
filter needs a comparison (no bare `[?(@.x)]`); `&&` binds before `||`.
Filters follow RFC 9535: they select matching array elements and object member
values (`$.services[?(@.enabled == true)].port`). A missing field is
"Nothing": `@.x != 1` is true when `x` is absent; `== < <= > >=` are false
(two missing fields are equal); `==` on arrays/objects is structural. Numbers
parse like JSON — integers in [-2^63, 2^64) exact, others IEEE doubles;
`1.`, `.5`, `+1`, `1e400` are exit 2 (write `1e20`). Quoted strings decode
`\\ \/ \b \f \n \r \t \uXXXX` and the escaped own quote; other escapes are
exit 2 — a literal backslash is `\\` (`$['C:\\new']`). In a YAML
double-quoted `expression` double every `\` again; YAML single quotes keep it.

### 7.2 `path` (yaml, toml, json)

`$.settings.timeout`, `$.a.b[0]`; TOML table headers `[database].host`,
`[servers.alpha].ip`; keys starting with a digit need quotes (`["2024"]`).
`$..key`, negative indices and slices are rejected (exit 2) — write the full
path with `*` / `[*]` steps, or use `jsonpath` for JSON. `path` does not
decode escapes: `['a\b']` is the key `a\b`.

YAML streams: several `---` documents (k8s manifests) are one source; each
rule runs on every document and matches are concatenated in document order.
Merge keys are expanded before matching (`<<: *base`, `<<: [*a, *b]`; own
keys win, earlier sources beat later ones). Duplicate keys: YAML/TOML →
per-file error; JSON → the last value wins silently. Mappings and TOML tables
keep source key order.

### 7.3 `xpath` (XML)

Supported subset. Query prefixes use namespace declarations on the root;
unknown prefixes are a rule error. Unprefixed names match elements without a
namespace and, when the root declares `xmlns="…"` (pom.xml, Atom, SVG),
elements of that default namespace: `/project/version/text()` works on a
pom.xml. XML `extract` on an xpath ending at an element: `text` (default) and
`outer_html` = the element's exact source; `inner_html` = the exact source
between its tags; `attr:NAME` = what `/@NAME` gives (no trim, empty kept,
absent → no item).

**DTD safety.** A DOCTYPE uses only its internal subset; external DTDs and
entities are never loaded (`&nbsp;` from the XHTML DTD → file error). A file
is rejected before parsing if its entities expand to more than 8 MiB, copy
more than 1 GiB of text, nest deeper than 10 levels, are recursive, or contain
unbalanced markup (entity-bomb protection).

Depth limits: XML deeper than 128 elements (counting entity-produced ones) is
a per-file error; the JSON, YAML and TOML parsers stop at 127, 128 and 80.

### 7.4 `css` and `markdown` (html / markdown)

HTML `extract: text` reads an element like a browser's innerText: inline tags
add nothing (`H<sub>2</sub>O` → `H2O`), whitespace runs collapse to one space
except in `pre`/`textarea`/`xmp`/`listing`/`plaintext`, block elements end
lines, each `<br>` adds one, cells of a row are TAB-separated, values are
trimmed. Not rendered: `script`, `style`, `noscript`, `template`, `datalist`,
`head`/`title`, fallback content of `iframe`/`video`/`audio`/`canvas`.
`attr:NAME` is trimmed; empty values are items (add `drop_empty`); a missing
attribute yields no item. `inner_html`/`outer_html` re-serialize with
attributes in source order (stable across runs).

Markdown selection includes nested nodes in source order; tables answer both
`table` and `table_row` / `table_cell`:

    $ any_txt_parser extract -i table.md -r markdown:table_row
    - Name | Qty
    - apple | 3
    - pear | 5                 (3 items: header row + 2 rows)

    $ any_txt_parser extract -i table.md -r markdown:table_cell   ->  6 items

### 7.5 Text selectors

`regex`, `contains`, `starts_with`, `ends_with`, `line {from, to}`. One capture
group in a regex → the group is the item, else the whole line. `context` adds
neighbouring lines (report mode and `--context` in command mode).

Since 1.5, every item of a text rule carries its **source line** — in command
mode (`- L12: text`) and in JSON (`lines[]` parallel to `items[]`), see
section 12.

### 7.6 `extract` modes (xml/html)

`text` (default) · `attr:NAME` · `inner_html` · `outer_html`. After an
expression that already ends at `@attr` or `text()`, any other mode → exit 2.

### 7.7 `transforms` (order matters; the list runs as written)

Item (per match): `trim`, `normalize_whitespace`, `lowercase`, `uppercase`,
`unescape`, `truncate_chars N`, `truncate_lines N`, `replace {from, to}`,
`regex_replace {from, to}`.
Result (item list): `drop_empty`, `unique`, `sort`, `sort_desc`,
`limit_items N`, `join "SEP"`.

`unescape` decodes only `\n \t \r \" \' \\` and `\uXXXX` (4 hex digits;
surrogate pairs merge). Everything else stays literal. `truncate_chars`,
`truncate_lines` and `limit_items` that cut something count as truncation
(`Truncated: yes`, file `warning`, one warning line per step).

### 7.8 Limits

Per rule: `limits: {max_items, max_item_chars}`. Run-wide: the `--max-*`
options and `output.max_output_chars` / `output.block`. Exceeding a hard limit
fails with exit 6 and preserves the previous output.

### 7.9 Errors per file and per rule

A file that fails to read/decode/parse is `error` in `## Files` with the reason
in `## Diagnostics`; other files still produce results (exit 1). If every file
fails, the run exits 3 and the errors are published.

## 8. Discovery

- `-i` accepts a file, a directory (recursive) or a glob (`*`, `**/`).
- The `-o` output file is never scanned.
- Symlinks are skipped unless `--follow-symlinks`.
- Caps: `--max-files` (5000), `--max-file-size` (200m), `--max-memory`.

## 9. Format detection (`format: auto`) and encodings

1. **Extension:** `.json .xml .html/.htm .md/.markdown .yaml/.yml .toml
   .txt/.log` (= text). Unknown or none (a file named `json`) or `.auto` → 2.
2. **Sniffing:** valid JSON → json; `<!doctype html`/`<html` → html; valid XML
   → xml (XML too deep to parse counts only if all its tags pair up, so `<34>`
   syslog or `<nick>` chat logs stay text); TOML with at least one key → toml;
   YAML only if the text has `: ` or starts with `---` and every non-empty
   document is a config shape (top-level mapping with identifier-like keys, or
   a list of mappings). Log lines, `Error:` headings and PEM files stay text.
3. **Fallback:** text. If sniffing guesses wrong, set `--format` or
   `input.format`.

Encodings: `utf-8` (default, BOM stripped), `utf-8-sig`, `utf-16le`, `utf-16`
(=LE), `utf-16be`, `windows-1251`/`cp1251`. The `## Files` table shows the
encoding actually used, e.g. `utf-8 (BOM stripped)`, `utf-16le (BOM)`.

**Line breaks:** LF, CRLF and a lone CR each end a line, in every format;
decoded text is normalized to LF before detection, so line numbers, `line`
ranges, context and samples always agree.

## 10. The report document (`-o out.md`)

Starts with `# any_txt_parser output`, then `Triangle: [0,1,0]` (done marker),
`Generation: NNNNNNNN` (increments every run — freshness check),
`Updated: <UTC>`, then sections:

- `Execution` — run status, exit code, counters
- `Files` — per file: path, size, format (+ how detected), status, encoding
- `Diagnostics` — file/rule errors; a run-level note when no rule applies
- `Applied Template` — the rules as given
- `Effective Template` — actual settings incl. CLI overrides, in canonical DSL
  form: save as `.yml` and pass to `-t` unchanged
- `Result Semantics`, `CLI Arguments`
- `Block Index` — block → line range
- Data blocks: `<!-- BLOCK: B0001 START -->` … `END -->`, header with
  Source/Format/Format source/Rule/Status/Items, then `### Data`

Rules for machines: metadata is written on one line and escaped for Markdown;
`<!-- BLOCK:` appears only on anchor lines, and every `<!-- BLOCK` in data is
escaped with an extra `\` — so a substring search for anchors is always safe.
The document always ends with `End of any_txt_parser output`.

Split blocks: a parent holds no data (`(in the continuation parts)`);
`Item continuation: N` marks fragments of original item N — concatenate the
item strings in part order without separators, then decode.

## 11. Reading the report (for AI)

1. Read the exit code first (also `Exit code:` under `Execution`), then
   `Run status` and the `Files` table.
2. `success` = complete · `warning` = a rule found nothing / was not
   applicable / was truncated / failed next to working rules — output may be
   partial · `error` = nothing extracted from that file.
3. Locate data by block anchors or the Block Index — do not re-parse prose.
4. `Generation` changed ⇒ a newer run replaced the document.
5. `Effective Template` is reusable as the next `-t`.

## 12. Command mode

### `extract` — rules on the command line

    any_txt_parser extract -i <path> -r RULE [-r RULE ...] [options]

`RULE` is `TYPE:EXPRESSION` (`contains:ERROR`, `regex:code=(\d+)`,
`jsonpath:$.users[*].email`, `path:[package].version`,
`xpath://item/title/text()`, `css:a[href]`, `markdown:heading:^Install`,
`line:10-20`) or a `{...}` rule object. Options: `--format`, `--encoding`,
`--context[|-before|-after] N`, `--extract MODE`, `--transform T` (repeatable),
`--max-items`, `--max-item-chars`, `--max-output-chars`, `--hash`,
`--include-empty`, `--json`, and the resource caps.

Output (text), verified on 1.5.0:

    $ any_txt_parser extract -i app.log -r contains:ERROR
    any_txt_parser 1.5.0: success, exit 0; files 1 (success 1, warning 0, error 0); results 1; items 2
    == D:/work/app.log [text] rule r1 (success, 2 items)
    - L2: 2026-10-01 10:00:01 ERROR boom
    - L4: 2026-10-01 10:00:03 ERROR again
    == end: success, exit 0

Every text item carries its source line (`- L2:`). Context lines are marked
with a trailing dash (`L1- …`). Items merged from several source lines by a
transform show a span (`- L41-L43: …`); if a transform loses the line
information, only the text is printed. `--json` adds `lines[]` parallel to
`items[]` and `spans[]` (`[first, last]`) for multi-line items — the arrays
survive `unique`, `sort`, `limit_items` and `join`.

Statuses: `success` / exit 0 — every file read, every rule found data
somewhere; `partial_success` / exit 1 — a rule found nothing anywhere, data
was cut, or a file failed; exit 2 — invalid rule; exit 3 — bad option, no file
matched, or every file failed; exit 6 — resource limit.

### `recon`

    any_txt_parser recon logs/            # per file: size, encoding, detected format and why,
                                          # suggested selector types, line count, first lines

### `map`

    any_txt_parser map [--root DIR] [--max-files N] [--json]

Tree + per-file line count and token estimate. Scans skip `.git`, `target`,
`node_modules`, `__pycache__`, `.zvec-grep` and the tool's own files
(`.navguard_cache.json`, `.navguard_rules`, `scripts/.abc_triangle_state.json`,
`.abc_triangle_log.json`); a `navguard.json` in the root can change the list
(an entry skips any *name containing* it — note this also hides `.gitignore`,
which contains `.git`). Line counts are exact; tokens are an estimate "a
little high on purpose". Verified: 15 real files, none of the service files.

### `symbols`

    any_txt_parser symbols [FILE] [--root DIR] [--json]

Functions, methods, classes, structs, traits, enums, interfaces… of one file
or the whole project, with line ranges. Verified on one Rust file:

      1-3   fn helper
      5-8   enum Color
     10-12  trait Shape
       11-11  method area            <- nested: indented under its owner
     14-14  type Miles
     16-19  struct Point
     21-25  impl Point
       22-24  method norm
     27-29  fn run

Boundaries are brace-based in brace languages (correct with Allman style and
nested blocks) and indentation-based in Python. Nesting is shown by
indentation (the enclosing symbol is the line above); `find` prints the owner
explicitly (`method norm  (in Point)`). JS/TS, Go, Java, C#, C/C++, Kotlin,
Rust and Python are recognized.

### `find`

    any_txt_parser find PATTERN [--exact] [--case-sensitive] [--root DIR]

Substring by default; `--exact` matches the whole symbol name (in C++ a
`Foo::bar` method also answers `bar`). Verified:

    $ find from_str          ->  src/cart.rs:1-1  fn from_str
                                 src/cart.rs:2-2  fn from_string_unchecked
    $ find from_str --exact  ->  src/cart.rs:1-1  fn from_str

### `lines`

    any_txt_parser lines FILE START [END] [--root DIR] [--json]

Numbered lines `START..END`, 1-based inclusive. `END` defaults to `START`.

### `search`

    any_txt_parser search TEXT [FILE] [--context auto|line|sentence]

Case-sensitive text search; `auto` gives the line in code and the sentence in
prose.

### `todos`

    any_txt_parser todos [--loose] [--root DIR]

Strict by default (since 1.5): TODO, FIXME, HACK and XXX as whole words — the
marker must be followed by `:`, `(`, `!` or preceded by `@`, so `todo list`,
`TODO_LIMIT`, `hackathon` and plain prose do not match. `--loose` restores the
old substring behaviour. Verified:

    strict: 6 hits —  `todo!()`, `// TODO: add taxes`, `// Fixme(bob)`,
                       `TODO: real task`, `todo!()`, `@todo x`
    loose: 10 hits —  also `todo list`, `a todo list`, `hackathon`, `TODO_LIMIT`

### `changes`

    any_txt_parser changes [--root DIR] [--persist]

Files new/changed/deleted since the last snapshot; `--persist` saves the
current state as the new snapshot.

## 13. Navigators (`nav`)

    any_txt_parser nav --oneshot            # scan once, write the three files
    any_txt_parser nav --watch [--focus P]  # rewrite on every change
    any_txt_parser nav --check [--brief|--diff]   # report to stdout, writes nothing
    any_txt_parser nav --grep PATTERN       # search the symbol index
    any_txt_parser nav --lines PATH A [B]   # numbered lines
    any_txt_parser nav --extract PAT [PATH] # text search with context
    any_txt_parser nav --todos              # same strict rules as `todos`
    any_txt_parser nav --patch replace|insert|delete ... [--dry-run]
    any_txt_parser nav --triangle status|step_done|verify_start|verify_complete|verify_fail|reset|log
    any_txt_parser nav --validate | --build | --merkle-verify | --set-rules

`nav --help` lists everything. Only `--oneshot` and `--watch` write the
navigator files; `--check` reports on stdout and writes none (documented since
1.4 — scripts that relied on the old `--check` writing files must switch to
`--oneshot`).

### The three files

`nav --oneshot` writes into the root:

- **`navigator.md`** — `Root:` (absolute, **once**; the only place the full
  path appears), `Updated:` (ISO-8601 with offset), `Status: READY`
  (all three files complete, from one scan, same `Updated`), totals, a "How to
  read" block and the tree. Tree entries carry `deep L12` pointers.
- **`navigator_deep.md`** — one row per file in tree order: path relative to
  Root, bytes, lines, ~tokens, sha, and the `deep2 L{a}-L{b}` range of its
  symbols (`-` when none).
- **`navigator_deep2.md`** — per file, its symbols with lines in the source
  file, each section naming its line in `navigator_deep.md`.

Chain, verified: tree `dispatch.md deep L12` → row 12 of `deep` (path, sizes,
`Symbols`) → `deep2 L{a}-L{b}` → the section → `lines PATH start end` reads
the source. Duplicate file names are unambiguous because all three files carry
paths relative to Root.

### Freshness

- `Updated:` in a file = the scan time of that file; `Status: READY` means all
  three come from one scan. After you edit files, run `nav --oneshot` again.
- `nav --triangle` is a separate, explicit workflow state machine stored in
  `scripts/.abc_triangle_state.json` of the root (ISO timestamps):

      reset            [anything] -> [0,0,1]
      step_done        [0,0,1]     -> [1,0,0]
      verify_start     [1,0,0]     -> [0,1,0]
      verify_complete  [0,1,0]     -> [0,0,1]

  It never mixes with the navigators' `Status:` and never reads/writes outside
  the root.

## 14. MCP server

    any_txt_parser mcp --root DIR

**10 tools:** `help`, `recon`, `extract`, `map`, `symbols`, `find_symbol`,
`read_lines`, `search_text`, `todos`, `changes`. Most take `output: "json"`;
`extract` takes the same rules and options as the CLI; `find_symbol` accepts
`exact: true`; `todos` accepts `loose: true`.

**2 resources** (since 1.4):

- `project://tree` — the tree of the server root with line/token estimates
  (the `map` tool without arguments, at most 1000 files, root absolute).
- `project://state` — JSON: root, `checked` (now), totals on disk;
  `navigators.status` = `fresh` (the three navigator files match every file) /
  `stale` (with new/changed/deleted counts and paths) / `incomplete` /
  `missing`; `snapshot` time; `workflow` = the `--triangle` state.

## 15. Confinement and safety

- Navigation commands read only inside `--root` (default: the current
  folder); the MCP server only inside its root. `..`, absolute paths
  elsewhere, and symlinks/junctions pointing outside are **refused**:

      read_lines C:\Windows\win.ini  ->  path is outside the project root …
      nav --patch <outside file>     ->  patch: path is outside the project root …

- Scans never follow symlinks or junctions.
- `nav --patch` is the only command that edits files; it has the same limit
  and a `--dry-run`.
- Report mode (`-i`) has no `--root`: it reads the paths you give it, like any
  CLI tool, and writes only `-o`.

## 16. Recipes

| Goal | Command |
|---|---|
| What is in these files? | `recon logs/` |
| Error lines with context | `extract -i logs/ -r contains:ERROR --context 1` |
| A field from JSON | `extract -i data.json -r "jsonpath:$.users[*].email"` |
| A value from TOML | `extract -i Cargo.toml -r "path:[package].version"` |
| All `<version>` of a pom | `extract -i pom.xml -r "xpath:/project/version/text()"` |
| Links from a page | `extract -i page.html -r "css:a[href]" --extract attr:href --transform unique` |
| Headings from a README | `extract -i README.md -r "markdown:heading:^Install"` |
| Where is a function? | `symbols src/` or `find checkout` (`--exact` for whole names) |
| Read only that range | `lines src/main.rs 62 128` |
| Project map without loading it | `map --root .` or the `project://tree` resource |
| Are my navigators current? | `project://state`, or `nav --check` |
| What changed since yesterday? | `changes --persist` |
| Full self-describing log | `-i logs/ -t tpl.yml -o out.md` |

## 17. Troubleshooting

| Symptom | Cause | Fix |
|---|---|---|
| exit 2 "unknown selector type" | wrong `type` name | see the table in §6; text: `regex/contains/starts_with/ends_with/line` |
| exit 2 "markdown selector requires select.node" | `expression` instead of `node` | `select: {type: markdown, node: heading}` |
| exit 2 "invalid path" | path syntax | TOML `section.key` or `[section].key`; YAML `$.key`; digit-leading keys quoted |
| exit 2 on a jsonpath number | unsupported literal | write `1e20`, not `1.`/`.5`/`+1` |
| "rule not applicable" (warning) | selector type does not fit the format | fix the selector, force `--format text`, or add `when: {format: …}` |
| 0 items, no warning | the expression genuinely matches nothing | `recon` the file; check the real format and shape |
| exit 6 resource limit | a `--max-*` cap hit | raise the cap or narrow the input |
| exit 3 all files error | wrong `--encoding` or broken sources | try `--encoding windows-1251` / `utf-16le`; read `## Diagnostics` |
| XML "unknown entity reference" | external DTD entity (`&nbsp;`) | entities from external DTDs are never loaded; replace or drop the DOCTYPE |
| "XML DTD rejected" | entity bomb guards | the file is unsafe to expand; extract as text (`--format text`) |
| file detected as the wrong format | sniffing guessed | set `--format` or `input.format` |
| `map` does not see a file | it is skipped (`.git`, `target`, `node_modules`, `__pycache__`, `.zvec-grep`, own files) or a name contains a skip entry | adjust `navguard.json` `exclude` |
| navigators look old | only `--oneshot`/`--watch` write them | run `nav --oneshot`; check `project://state` |
| MCP call refused | path outside the server root | run the server with the right `--root`, or pass a relative path |
| `<!-- BLOCK` found inside data | it is escaped data, not an anchor | remove one `\` from each run to decode |

## 18. Performance and guarantees

- Deterministic: the same sources, paths, template and CLI produce byte-identical
  output (verified by hashing repeated runs), except `Generation` and the
  execution timestamps.
- Measured on a desktop: a 50 000-line / 1.8 MB log with `contains:ERROR` →
  ~20 ms, 100 matches; `nav --check` on a small project → ~15 ms.
- Output/block limits count Unicode characters of the full rendered document;
  exceeding a hard limit fails with exit 6 and preserves the previous output.
- Atomic publish: a failed run never truncates an older output file.

## 19. Building from source

Rust (stable, MSVC). `cargo build --release --locked`. The release script
`scripts\release.bat` runs the format check, tests, the release build, and
copies the exe, `HELP.md` and `README.md` to `dist\` and the bundled skill
folders. `docs/help/` holds the sources of the built-in help; `tests/`
(including `one_exe.rs` and snapshot suites) covers the CLI, the MCP server
and the navigators through the built exe.

## 20. What changed since 1.0 (summary)

- **1.1** — short commands on stdout (`extract -r`, `recon`, `map`,
  `symbols`, `find`, `lines`, `search`, `todos`, `changes`), the MCP server,
  built-in `help` topics.
- **1.2–1.3** — navigators rewritten: exact line counts (the old off-by-one
  is gone), `Root:` + `Updated:` + tree pointers, deep2 ranges exact,
  confinement of reads and `nav --patch`, junctions not followed,
  `.zvec-grep` and own files skipped, deterministic tree rendering with `/`.
- **1.4** — fixes from external verification: MCP resources
  `project://tree` and `project://state`; `Root:` once (in `navigator.md`);
  paths relative to Root in `deep`/`deep2`; `Triangle: [0,1,0]` in the
  navigator replaced by `Status: READY` (the `[A,B,C]` notation now belongs
  only to `nav --triangle`); ISO timestamps in the state file; `nav --version`.
- **1.5** — text rules report source lines (`- L12:` in text, `lines[]` /
  `spans[]` in JSON); `find --exact`; strict `todos` (whole-word
  TODO/FIXME/HACK/XXX) with `--loose` for the old behaviour; documentation
  refreshed end to end.

History in detail: `CHANGELOG.md`.
