# any_txt_parser — Complete Guide (HELP)

Document version: 1.0.0 · Matches program version 1.0.0 (crate `any_txt_parser`, edition 2026, Rust)

---

## 1. What It Is

`any_txt_parser` is a deterministic CLI text extractor: it extracts data from arbitrary files
(JSON, YAML, TOML, XML, HTML, Markdown, plain text) according to a YAML template and publishes
the results into a **self-describing** Markdown file `log_any_txt.md`.

The program is designed to pass data to a "machine" (AI assistant): the output is self-contained —
it includes the original template, block structure, anchors, status diagrams, and line indexes.

Two-phase pipeline:

1. **Phase 1 — Extraction**: discovery (file search) → decode (decoding) → collect
   (extraction by rules) — for each file.
2. **Phase 2 — Materialization**: materialize (grouping into blocks, split, dedup, sorting,
   limits) → render (document assembly) → validate (internal check) → publish
   (atomic write: temporary file + rename).

Key properties:

- **Determinism**: the same input produces the same output (except the `Generation`
  field and timestamps).
- **Self-describing**: output includes `## Effective Template` (YAML), `## Applied Template`,
  `## Result Semantics`, `## CLI Arguments`.
- **Strict contracts**: fixed header `# any_txt_parser output`, marker
  `Triangle: [0,1,0]`, end-of-document marker `End of any_txt_parser output`.
- **Atomic publishing**: the output file is never corrupted on failure; on startup errors
  the existing output is left untouched and no new one is created.
- **Resource limits** at every stage, strict exit codes.

---

## 2. Build & Run

```bash
cargo build --release          # binary: target/release/any_txt_parser.exe
cargo test                     # unit tests (27) + integration tests (9)
cargo fmt --all                # formatting
cargo clippy --all-targets     # lint check (warnings only)

any_txt_parser --input <PATH> --template <FILE> --output <FILE> [options]
```

Minimal run (all three required options):

```bash
any_txt_parser -i data.json -t tpl.yml -o log_any_txt.md
```

---

## 3. CLI Arguments

| Option | Required | Meaning | Default |
|---|---|---|---|
| `-i, --input PATH` | yes | File, directory, or glob pattern (see §6) | — |
| `-t, --template FILE` | yes | Path to YAML template (see §5) | — |
| `-o, --output FILE` | yes | Output file; `log_any_txt.md` by convention | — |
| `--format FMT` | no | Forced format: `json\|xml\|html\|markdown\|yaml\|toml\|text` (not `auto`) | from template, else `auto` |
| `--encoding ENC` | no | `utf-8`, `utf-8-sig`, `utf-16le`, `utf-16` (=LE), `utf-16be`, `windows-1251`/`cp1251` | from template, else `utf-8` |
| `--dry-run` | no | Only validate template/files, prints a plan, publishes nothing | off |
| `-v, --verbose` | no | Diagnostics to stderr (versions, files, formats, counters) | off |
| `-q, --quiet` | no | Suppress the final stderr message | off |
| `--follow-symlinks` | no | Traverse symlinks during discovery | off (symlinks skipped) |
| `--hash` | no | Compute sha256 of each source file, output into `## Files` | off |
| `--max-file-size SIZE` | no | Maximum input file size | 200 MiB |
| `--max-memory SIZE` | no | Decoded text budget (bytes) | unlimited |
| `--max-files N` | no | Maximum number of discovered files | 5000 |
| `--max-matches N` | no | Match limit per rule (resource bound, applies before transforms) | unlimited |
| `--max-output-size SIZE` | no | Maximum final document size (characters) | from template |
| `--max-block-chars N` | no | Overrides `output.block.max_chars` (target = 7/8 of it) | 32000 |

`SIZE` accepts suffixes: `b`/`B`, `k`/`K`/`KiB`, `m`/`M`/`MiB`, `g`/`G`/`GiB` (powers of 1024), or a bare number.

Parameter precedence: **CLI → template → default**. CLI overrides are recorded
in the output (`Format source: CLI override`, `Encoding source: CLI override`,
in `## Effective Template` + `## CLI Arguments`).

---

## 4. Exit Codes

| Code | Meaning | When |
|---|---|---|
| `0` | Success | All files successful, no limits hit |
| `1` | PartialSuccess | Some files have status `warning` (no matches, not applicable, truncation) or `error` |
| `2` | InvalidTemplate | Template cannot be read/parsed (see §5) |
| `3` | InvalidInputOrCli | No input files, bad pattern, missing `--output`, template not on disk |
| `4` | OutputFailure | Internal document validation or publish error |
| `5` | InternalFatal | Internal error |
| `6` | ResourceLimit | Resource limit exceeded (files, size, blocks, memory) |

`--dry-run` always returns `0` (or `2`/`3` for errors before dry-run).

---

## 5. Template (Template DSL, version: 1)

A YAML file. Full schema:

```yaml
version: 1                     # required; only 1 is supported
name: template_name            # required, non-empty
input:                         # optional
  format: auto                 # json|xml|html|markdown|yaml|toml|text|auto
  encoding: utf-8              # see --encoding
output:                        # optional
  max_output_chars: 100000     # character limit for the whole document
  block:                       # block splitting (see §8)
    target_chars: 28000        # target block size (0 < target <= max)
    max_chars: 32000           # hard block max; oversized items are split
document:                      # optional
  sort: none                   # none|asc|desc — document block sorting
  stable_deduplicate: false    # true: remove duplicate items (first kept)
  limit_total: 100000          # = output.max_output_chars (deprecated synonym)
  split_blocks: true           # true: split blocks by target/max (default true)
rules:                         # required, >= 1 rule
  - id: r1                     # required, unique ids
    when:                      # optional
      format: json             # apply rule only to this format
    select:                    # required, see §7
      type: jsonpath
      expression: "$.users[*].name"
    context:                   # optional (text adapter only)
      before: 1                # lines of context BEFORE the match
      after: 1                 # lines of context AFTER the match
    output:                    # optional
      key: names               # output key (default = rule id)
    extract: text              # text|attr:NAME|inner_html|outer_html (§7.4)
    transforms:                # per-item, order matters (§7.5)
      - trim
      - replace: {from: "a", to: "b"}
    limits:                    # optional
      max_items: 10            # max items after transforms (extras dropped, counted)
      max_item_chars: 500      # per-item character truncation
```

Template errors (code 2): unknown fields (`deny_unknown_fields` on all sections),
duplicate `id`s, `target_chars > max_chars`, zero block sizes, empty rule lists,
invalid version, bad selectors/regexes/transforms.

---

## 6. Discovery — How Input Files Are Found

`--input` can be:

- a **file** — that single file is taken;
- a **directory** — recursive traversal (depth up to 64), files sorted by path;
- a **glob pattern** — `*.json`, `data/*.txt`;

  - with `**` — recursive traversal (`src/**/*.rs`), matching relative to the base;
  - without `**` — via `glob` (case-insensitive, dotfiles allowed).

Exclusions from results:

- the output file (`--output`) is always excluded (by canonical path);
- symlinks are skipped unless `--follow-symlinks` is passed;
- directories are ignored (files only).

Post-discovery limits (code 6): `max_files` (default 5000), `max_file_size`
(default 200 MiB) — checked per file.

If 0 files are found — code `3`.

**Format auto-detection** (`format: auto`), in order:

1. file extension (`.json`, `.xml`, `.html/.htm`, `.md/.markdown`, `.yaml/.yml`,
   `.toml`, `.txt/.log` = text);
2. content sniffing: `{`/`[` + valid JSON → json; `<` + `<!doctype html`/`<html` → html;
   `<` + valid XML → xml; `key = value` (parses as toml) → toml; `key: value` or `---` → yaml;
3. fallback: text.

Encodings: utf-8 (BOM automatically stripped and recorded in `Encoding`), utf-8-sig,
utf-16le/utf-16 (BOM honored), utf-16be, windows-1251/cp1251.

---

## 7. Selectors and Adapters

The file format defines an **adapter**, which determines which selectors are applicable.
A rule whose selector type is not applicable to the file format gets status `rule not applicable`
— it is not an error. `when.format` lets you restrict a rule to a format.

| Format (adapter) | Available selectors |
|---|---|
| json | `jsonpath`, `path` |
| yaml | `path` |
| toml | `path` |
| xml | `xpath` |
| html | `css` |
| markdown | `markdown` |
| text | `regex`, `contains`, `starts_with`, `ends_with`, `line` |

### 7.1 jsonpath (JSON)

Syntax:

- `$` — root (optional); steps: `.name`, `..name` (recursive search),
  `.*`, `..*`, `[N]`, `[N,M,...]`, `['name']`/`["name"]`, `[*]`;
- filter: `[?(<expression>)]`; comparisons `== != < <= > >=` over paths, strings,
  numbers, booleans, null; combinators `&&`, `||`; `@` = current node, bare `@` = the node itself.

Examples:

```
$.users[*].name            # names of all users
$..title                   # all title fields at any depth
$.orders[0].id             # id of the first order
$.items[0,2,4]             # array items by index
$.store.book[?(@.price < 10)]       # books cheaper than 10
$.users[?(@.active == true && @.role == 'admin')]
```

### 7.2 path (yaml, toml, json)

A simplified path without JSONPath logic: `$`/`.` separate keys, `[N]` is an array index,
`['k']`/`["k"]` is a key with spaces/special characters, `[*]` is all array items.

```
$.settings.timeout        # equivalent to settings.timeout
$['log level']            # key with a space
$.servers[0].host         # host of the first server
$.list[*].value           # value of every list item
```

### 7.3 xpath (XML)

A subset of XPath:

- `/root/a/b` — absolute path, `//a` — search from root; `/a//b` — descendants anywhere;
- steps: `name`, `*` (any element), `name[N]` (1-indexed or 0), attribute predicates
  `name[@attr]`, `name[@attr="val"]`, text predicate `name[text="val"]`;
- terminals: element (node text), `@attr` — attribute, `text()` — node text.

Examples:

```
/root/catalog/item/@id          # id attributes of all items
/root/item[@type="book"]/title/text()
/root/item[price > 100]/name    # comparisons in predicates are supported
//item[1]                       # first item anywhere
```

### 7.4 css and markdown (html / markdown)

**css** — selectors via `scraper` (standard CSS): `p`, `a[href]`, `div.card > h2`,
`table tr td`, `#main .item:first-child`, etc.

**markdown** — argument `node` (+ optional `pattern` regex against node text content):

```yaml
select: {type: markdown, node: heading}      # all headings
select: {type: markdown, node: code_block, pattern: "^const"}   # code blocks starting with const
select: {type: markdown, node: list_item}    # list items
```

`node` values: `heading`/`h`, `paragraph`/`p`, `list`, `list_item`/`item`/`li`,
`table`, `table_row`/`row`/`tr`, `table_cell`/`cell`/`td`, `code_block`/`code`,
`blockquote`/`quote`, `link`, `image`/`img` (underscores and hyphens in names are ignored).

### 7.5 text selectors

Work line-by-line (lines are an array split by `\n`):

| type | Semantics |
|---|---|
| `regex` | Lines matching. One capture group `(…)` → the group is the item, otherwise the whole line |
| `contains` | Line contains the `expression` substring |
| `starts_with` | Line starts with `expression` |
| `ends_with` | Line ends with `expression` |
| `line` | Range: `{type: line, from: 1, to: 5}` — 1-indexed, inclusive of `to`; the result is a single item of joined lines |

`context.before/after` works for `regex/contains/starts_with/ends_with`:
neighboring lines (with a context label) are added to the item — used for logging.

Example:

```yaml
select: {type: regex, expression: "^ERROR (.+)$"}     # one group -> error text
select: {type: regex, expression: "ERROR"}            # no group -> whole line
select: {type: contains, expression: "TODO"}
select: {type: line, from: 10, to: 20}
```

### 7.6 extract — extraction mode

| Mode | Where applied | What it extracts |
|---|---|---|
| `text` (default) | all | Text content |
| `attr:NAME` | xml (`@attr`), html (css elements) | Attribute value |
| `inner_html` | html | `inner_html()` of the element |
| `outer_html` | html | `html()` of the element (empty ones are not dropped) |

### 7.7 transforms — post-processing (order matters)

**Item transforms** (applied to each match before other constraints):

| Name | Parameter | Action |
|---|---|---|
| `trim` | — | Trim whitespace |
| `normalize_whitespace` | — | Collapse whitespace characters into a single space, trim edges |
| `lowercase` / `uppercase` | — | Case conversion |
| `unescape` | — | `\n \t \r \" \' \\ \uXXXX` into real characters |
| `truncate_chars` | `int` | Keep N characters |
| `truncate_lines` | `int` | Keep N lines |
| `replace` | `{from, to}` | Substring replacement (all occurrences) |
| `regex_replace` | `{from, to}` | Regex replacement (`from` is a pattern) |

**Result transforms** (applied to the rule's item list):

| Name | Parameter | Action |
|---|---|---|
| `drop_empty` | — | Remove empty (trimmed) items |
| `unique` | — | Remove duplicates (first occurrence kept) |
| `sort` | — | Sort ascending |
| `sort_desc` | — | Sort descending |
| `limit_items` | `int` | Keep N items |
| `join` | `string` | Join into one item using a separator |

### 7.8 rule limits

- `max_items` — after all transforms; extras are dropped, the counter goes into
  the warning «N item(s) omitted after max_items limit».
- `max_item_chars` — per-character truncation BEFORE result transforms; for each truncated
  item a warning «item truncated N -> M chars» is emitted, plus the total «N item(s) truncated».

### 7.9 Rule and file statuses

Rule status: `success` (has items) · `no matches found` (nothing found) ·
`rule not applicable` (selector not applicable to the format) · `error`.

File status: `success`; `warning` (any rule no-match/not-applicable or truncation);
`error` (file parse error or rule error). A file with status `error` produces
entries in `## Diagnostics` and no blocks.

---

## 8. Materialization and Blocks

After collecting items across all files/rules:

1. **union by key**: items of all rules with the same `output.key` are merged
   (in file order, then rule order) into one group — one "shell";
2. **`document.sort`**: `asc`/`desc` sorts blocks by the text of the first item;
3. **`document.stable_deduplicate: true`**: within each block, duplicate items are removed
   (string equality, first occurrence kept) — applied to the whole document
   with the same principle for overlapping keys;
4. **size splitting** (`split_blocks: true`, default): a block grows up to
   `target_chars`; if the next item exceeds `max_chars` (including the block header
   ≈220 chars) — the block is "split" into parts:
   - a parent block with marker `Parent block: continuation parts follow (N)`;
   - parts `Part: 1/N … N/N` with ID suffix `B0001-01`, `B0001-02`, …;
   - a single item that does not fit even into `max_chars` (minus the header) is cut
     character-by-character down to the maximum allowed size.
5. `output.max_output_chars` — global limit: when reached, further blocks
   are dropped with a note in `## Truncation`;

About block ids: first block `B0001`, then ascending; parts get
`BXXXX-NN`. Anchors:

```
<!-- BLOCK: B0001 START -->
<!-- BLOCK: B0001 END -->
```

A block includes:

```
## Block: B0001
Parent: B0001            # parts only
Part: 1/3                # parts only
Parent block: continuation parts follow (3)   # split parent only
Source: <file path>
Format: json
Format source: extension
Rule: r1
Output key: names
Status: success          # RULE status (rule-level)
File status: warning     # only if the file is not success
Items: N
Truncated: yes|no
Split: yes|no

### Data

- item 1
- item 2 (multiline — continuations indented by 4 spaces)
```

Behavior with empty data: `no matches found` / `rule not applicable` / `(error)` / `(no data)`.

The `### Warnings` section — rule warnings (omitted/truncated); `### Errors` — errors.

---

## 9. Output Document (Structure)

```
# any_txt_parser output
Triangle: [0,1,0]                 # "done" contract
Generation: 00000001              # incremented from the previous run (8 digits)
Updated: <ISO-8601 UTC>

Parser version: 1.0.0

## Execution                        # run metadata
## Files                            # | Path | Size | Modified | Format | Status | Hash |
## Diagnostics                      # only for file errors
## Truncation                       # only for global cuts/drops
## Applied Template                 # human-readable template summary
## Effective Template               # YAML block with actual settings (after CLI overrides)
## Result Semantics                 # conventions for reading the document
## CLI Arguments                    # raw command-line arguments
## Block Index                      # | Block | Source | Rule | Key | Items | Size | Start | End | Anc |
### Block Anchors                   # list of blocks with line ranges
## Data Blocks
<!-- BLOCK: B0001 START -->
## Block: B0001
...
<!-- BLOCK: B0001 END -->
End of any_txt_parser output       # mandatory final marker
```

Block `start_line`/`end_line` are recomputed after rendering (accounting for the prelude and index),
each block must have unique START/END anchor pairs (internal validation, code 4).

How to read a block: `Source` + `Rule` + `Output key` identify the origin;
`Status` — completeness; the `lines X–Y` range from `Block Index` — where the block sits in the document.

---

## 10. Examples

### 10.1 Simple text file

`input.txt`:
```
version 3.2.1
NOTE: first release
ERROR: something broke
DONE
```

`tpl.yml`:
```yaml
version: 1
name: text_sample
input: {format: text}
rules:
  - id: errors
    select: {type: starts_with, expression: "ERROR"}
    output: {key: errors}
    transforms: [trim, normalize_whitespace]
  - id: notes
    select: {type: contains, expression: "NOTE"}
    output: {key: notes}
```

### 10.2 JSON with filter and transforms

`data.json`:
```json
{"users": [
  {"name": "Alice", "active": true,  "tags": ["a","b"]},
  {"name": "Bob",   "active": false, "tags": ["c"]}
]}
```

`tpl.yml`:
```yaml
version: 1
name: users
input: {format: json}
output:
  block: {target_chars: 2000, max_chars: 4000}
rules:
  - id: active_names
    when: {format: json}
    select: {type: jsonpath, expression: "$.users[?(@.active == true)].name"}
    output: {key: names}
  - id: tags
    select: {type: jsonpath, expression: "$.users[*].tags[*]"}
    output: {key: tags}
    transforms:
      - uppercase                  # item transform (per match)
      - trim                       # item transform
      - unique                     # result transform (on the match list)
      - sort                       # result transform
```

Transforms are given as a single list — item and result transforms
are recognized automatically by name (see §7.7) and applied in order.

### 10.3 YAML via path

```yaml
version: 1
name: yaml_cfg
input: {format: yaml}
rules:
  - id: timeout
    select: {type: path, expression: "$.settings.timeout"}
    output: {key: timeout}
  - id: hosts
    select: {type: path, expression: "$.servers[*].host"}
    output: {key: hosts}
```

### 10.4 TOML

```yaml
version: 1
name: toml_cfg
input: {format: toml}
rules:
  - id: database_host
    select: {type: path, expression: "[database].host"}
    output: {key: host}
  - id: ports
    select: {type: path, expression: "[database].ports[*]"}
    output: {key: ports}
```

### 10.5 XML with attributes

```yaml
version: 1
name: xml_catalog
input: {format: xml}
rules:
  - id: ids
    select: {type: xpath, expression: "/catalog/item/@id"}
    output: {key: ids}
  - id: book_titles
    select: {type: xpath, expression: "/catalog/item[@type='book']/title/text()"}
    output: {key: titles}
  - id: first_item
    select: {type: xpath, expression: "/catalog/item[1]"}
    output: {key: first}
```

### 10.6 html via css

```yaml
version: 1
name: html_page
input: {format: html}
rules:
  - id: titles
    select: {type: css, expression: "h2"}
    output: {key: titles}
  - id: links
    select: {type: css, expression: "a[href]"}
    output: {key: links}
    extract: attr:href
  - id: products
    select: {type: css, expression: "div.product"}
    output: {key: products}
    extract: outer_html
    limits: {max_items: 5}
```

### 10.7 markdown

```yaml
version: 1
name: md_doc
input: {format: markdown}
rules:
  - id: headings
    select: {type: markdown, node: heading}
    output: {key: headings}
  - id: codes
    select: {type: markdown, node: code_block, pattern: "^#include"}
    output: {key: code}
  - id: items
    select: {type: markdown, node: list_item}
    output: {key: items}
```

### 10.8 Batch run over a directory with limits and hash

```bash
any_txt_parser \
  --input ./logs/**/*.log \
  --template tpl.yml \
  --output log_any_txt.md \
  --hash \
  --max-files 1000 \
  --max-file-size 50MB \
  --max-matches 2000
```

### 10.9 Dry-run — check without publishing

```bash
any_txt_parser -i data.json -t tpl.yml -o log_any_txt.md --dry-run
# prints: Template: valid, effective format/encoding, file list,
#         block limits, «Dry run: no output published.»
```

---

## 11. Notes for the Output Consumer (AI Contract)

- Read `## Effective Template` — it holds the actual settings (including CLI overrides).
- Statuses: `success` — data is complete; `warning` at `File status` level — a rule did not
  apply/found nothing/truncated, output may be incomplete; `error` — data is absent.
- `Generation` increments on every successful run — usable as a "new data" marker.
- For repeated machine reading, the most reliable approach is to find blocks by anchors
  `<!-- BLOCK: B0001 START --> … END -->` and `## Block Index` (lines).
- Multiline items: continuation is indented by 4 spaces.
- The final document always ends with the line `End of any_txt_parser output`.

---

## 12. Tests and Development

```bash
cargo test              # 27 unit + 9 integration (tests/golden.rs)
cargo clippy --all-targets
cargo fmt --all
```

Integration tests (`tests/golden.rs`) run the full pipeline in temp directories:
self-describing output, generation increment and determinism, split, `--max-matches` limit,
`--hash`, stable dedup, dry-run, code 2 on an invalid template, distinguishing
`no matches found` / `rule not applicable` / `error`.
