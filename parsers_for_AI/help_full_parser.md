# any_txt_parser + navguard — combined field guide

Two portable Windows tools for AI-assisted code work. Give an AI assistant THIS file plus the two
executables and it can orient in a project and extract data from it without any other documentation.

| Tool | Version | Role | Needs config? |
|---|---|---|---|
| `navguard.exe` | 0.1.0 | Structural map: file tree, line/token counts, symbol index | No — just run it |
| `any_txt_parser.exe` | 1.0.2 | Rule-based extractor: YAML template -> self-describing Markdown log | Yes — a small YAML template |

Both are single static exes with zero dependencies. Both write outputs that start with the same
freshness contract:

    Triangle: [0,1,0]          <- "data is fresh, safe to read"
    Generation: NNNNNNNN       <- increments on every run; compare to detect new data

## 1. Setup (the cheap way for AI development)

Copy BOTH executables into the ROOT of the working folder and work from there:

    <project>\
        any_txt_parser.exe
        navguard.exe
        src\ ...          <- only what you actually need

- navguard scans the directory where its exe lives (if a `Cargo.toml` exists higher up, it uses that
  root). Small root = small tree = fewer tokens.
- Copy only the files the task needs. `target`, `.git`, `node_modules`, `__pycache__` are ignored
  automatically; everything else you copy gets counted into the map.
- The two exes together cost ~6.5 MB and pull in no dependencies.

## 2. Workflow (orient -> drill -> recon -> extract -> consume)

    1. ORIENT   navguard --check                  (run from the project root)
    2. READ     navigator.md fully, then set its Triangle line [0,1,0] -> [0,0,1]
    3. DRILL    navigator_deep.md  L{N}      -> stats for one file
               navigator_deep2.md L{a}-L{b} -> symbols of that file with line numbers
    4. RECON    unknown file? sample its structure first (section 5) — never guess a template
    5. EXTRACT  any_txt_parser -i <glob> -t tpl.yml -o log_x.md
    6. CONSUME  exit code -> `## Files` table -> blocks by anchors / Block Index lines

Choosing the tool:
- WHERE / HOW MUCH in code or Markdown? **navguard alone.** deep2 gives symbol names with exact line
  numbers — open the file at those lines and read only what you need. Point access needs no second run.
- WHAT IS INSIDE across many files (logs, configs, data): **any_txt_parser** — rule-based extraction
  with statuses, limits and an audit trail.
- HTML: **never load raw HTML into context.** navguard maps it like any text file; the parser extracts
  exactly the nodes you need via css selectors (`h1`, `a[href]`, `table tr td`) — typically ~80% fewer
  tokens than the raw file.

## 3. navguard — quick reference

    navguard --check     full report to stdout + writes the 3 navigator files   (exit 0)
    navguard             double-click mode: same scan, waits for a keypress
    navguard --version   prints version

| File | Content |
|---|---|
| `navigator.md` | tree with `(deep L{N})` anchors + Triangle state |
| `navigator_deep.md` | per-file `lines / tokens` + deep2 range; the entry for a file sits at line L{N} |
| `navigator_deep2.md` | symbol index: functions/structs/sections with line numbers inside each source file |

Triangle protocol (forward only): navguard sets `[1,0,0]` while scanning, `[0,1,0]` when done. The AI
MUST read `navigator.md` completely and then set it to `[0,0,1]`. If the triangle is not `[0,1,0]`,
re-run `--check` before trusting anything.

Text files counted (lines + tokens): `.md .rs .py .js .ts .html .css .json .toml .yaml .yml .lock
.sh .bash .txt .log`. Everything else (`.exe`, `.bat`, images, ...) is binary: size only, `0|0` stats.

Real example (Rust project): tree entry `template.rs (deep L55)` -> deep line 55 says
`### template.rs | 1326 lines | 4931 tokens | deep2 L285-L323` -> deep2 lines 285-323 list every fn/struct
(`compile_raw`, `validate_encoding`, `parse_item_transform`, ...) with its line number. That is enough to
open the file at the right place without reading it first.

Quirk (verified): a root-level file whose name sorts after directory contents in Unicode order (e.g. a
Cyrillic filename) may be RENDERED inside the last directory group of `navigator.md` — cosmetic only;
its deep entry and stats are correct.

## 4. any_txt_parser — quick reference

    any_txt_parser -i <file|dir|glob> -t <template.yml> -o out.md [options]

- `-i` file, directory (recursive), or glob (`*`, `**/`). The `-o` file is never scanned.
- Precedence: **CLI > template > default**; overrides are recorded in the output
  (`Format source: CLI override`).

Key options (full list with defaults: `any_txt_parser --help`):

| Option | Effect |
|---|---|
| `--format FMT` | Force format for ALL files: json\|xml\|html\|markdown\|yaml\|toml\|text (aliases md, htm, yml, txt/log/plain). Not `auto`. Independent of extension. Invalid value -> exit 3 |
| `--encoding ENC` | utf-8 (default; BOM auto-stripped), utf-8-sig, utf-16le, utf-16, utf-16be, windows-1251/cp1251 |
| `--dry-run` | Validate + print plan with per-file format hints; publishes nothing; exit 0 (or 2/3) |
| `-v` / `-q` | verbose diagnostics to stderr / silence final message |
| `--hash` | add sha256 of each source file to the output |
| limits | `--max-files N` (5000), `--max-file-size SIZE` (200m), `--max-matches N`, `--max-memory SIZE`, `--max-output-size SIZE`, `--max-block-chars N` (32000); SIZE = bare bytes or k/m/g x1024 |

Template (YAML):

    version: 1
    name: template_name            # required
    input:                         # optional
      format: auto                 # json|xml|html|markdown|yaml|toml|text|auto
      encoding: utf-8
    output:                        # optional
      max_output_chars: 100000     # hard limit for the whole document (characters)
      block: {target_chars: 28000, max_chars: 32000}
    document:                      # optional
      sort: none                   # none|asc|desc
      stable_deduplicate: false
      split_blocks: true
    rules:                         # required, >= 1, unique ids
      - id: r1
        when: {format: json}       # optional — apply rule only to this format
        select:                    # required — see table below
          type: contains
          expression: "ERROR"
        context: {before: 1, after: 1}   # text selectors only
        output: {key: errors}      # default key = rule id
        extract: text              # text | attr:NAME | inner_html | outer_html (xml/html)
        transforms: [trim]         # order matters
        limits: {max_items: 10, max_item_chars: 500}

### Writing templates — strict rules

1. **Backslash strings (regexes) MUST be single-quoted in YAML.** Double-quoted YAML treats `\` as an
   escape and rejects unknown ones — every AI that writes `"^\s*fn"` gets exit 2:

        WRONG   expression: "^\s*ERROR (\w+)"    # exit 2: unknown escape character
        RIGHT   expression: '^\s*ERROR (\w+)'    # backslashes stay literal

   Double quotes are fine only for strings WITHOUT backslashes (`"ERROR"`).
2. One capture group `( ... )` in a regex => the group is the item; no groups => the whole line.
3. `--format` forces the format for ALL files of the run — split runs for mixed-format directories,
   or keep `auto`.

Selectors by format (a wrong type is NOT an error — the rule becomes `rule not applicable`, warning):

| Format | Selectors |
|---|---|
| json | `jsonpath` `$.users[*].name`, filters `[?(@.active == true)]`; `path` `$.a.b[0]` |
| yaml / toml | `path` `$.settings.timeout` (toml: `[database].host`) |
| xml | `xpath` `/catalog/item/@id`, `//item[@type='book']/title/text()` |
| html | `css` `h2`, `a[href]` (+ `extract: attr:href`) |
| markdown | `markdown` node `heading\|paragraph\|list\|list_item\|table\|table_row\|table_cell\|code_block\|blockquote\|link\|image` (aliases h, p, li, tr, td, code, quote, img; + optional `pattern` regex) |
| text | line-based: `regex`, `contains`, `starts_with`, `ends_with`, `line {from: 1, to: 5}`. One capture group in a regex -> the group is the item, else the whole line |

Transforms — item (per match): `trim`, `normalize_whitespace`, `lowercase`, `uppercase`, `unescape`,
`truncate_chars N`, `truncate_lines N`, `replace {from, to}`, `regex_replace {from, to}`.
Result (item list): `drop_empty`, `unique`, `sort`, `sort_desc`, `limit_items N`, `join "SEP"`.

Format detection (`format: auto`): 1) extension (`.json .xml .html/.htm .md/.markdown .yaml/.yml .toml
.txt/.log`=text); 2) content sniffing (valid JSON -> json; doctype/`<html` -> html; valid XML -> xml;
`key = value` -> toml; `key: value`/`---` -> yaml); 3) fallback text. The output records the origin:
`Format source:` = `extension` | `content fallback` | `template` | `CLI override`.

Exit codes: `0` success · `1` partial — some files warning/error, output STILL published (NORMAL when a
project scan matches only some files) · `2` invalid template · `3` invalid CLI/input (bad --format/
--encoding/SIZE, missing -o, no files found) · `4` output validation/publish failure · `5` internal fatal ·
`6` resource limit exceeded.

Output file: starts with `# any_txt_parser output`, then `Triangle:` / `Generation:` / `Updated:`, sections
`Execution`, `Files` (per-file Format/Status), `Diagnostics` (errors only), `Applied Template`,
`Effective Template` (actual settings incl. CLI overrides), `Result Semantics`, `CLI Arguments`,
`Block Index` (block -> line range), then data blocks wrapped in `<!-- BLOCK: B000N START -->` ... `END -->`, ending with
`End of any_txt_parser output`. With multiple input files there is ONE BLOCK PER FILE (per-file
attribution preserved even for a shared key).

Reading it (for AI): check the exit code first, then the `Files` table; locate blocks by anchors or Block
Index lines — do not re-parse prose. Statuses: `success` = complete · `warning` = a rule found nothing /
was not applicable / was truncated · `error` = file failed to parse, no data.

## 5. Unknown file? Recon before templating

Never guess a template for an unseen file — take a structure sample first, then write selectors that
match what you actually see:

1. `any_txt_parser -i F -t <any valid tpl> -o x.md --dry-run` — shows detected format/encoding per file,
   publishes nothing.
2. Sample the structure with the parser itself (built-in preview — no OS tools needed):

       version: 1
       name: recon
       input: {format: auto}
       rules:
         - id: head
           select: {type: line, from: 1, to: 30}
           output: {key: sample}

   -> the block contains the first 30 lines. If the text is garbled, retry with
   `--encoding windows-1251` (or utf-16le). For JSON/XML the visible shape IS the schema.
3. Build the real template from the observed shape: visible keys -> `jsonpath`; repeated line prefixes ->
   `starts_with`; key/value pairs -> `regex` with one capture group; HTML tags you need -> css selectors.

## 6. Micro-examples (real runs on a Rust project)

### 6.1 Index all public functions across src/**/*.rs

    version: 1
    name: rust_fns
    input: {format: auto}
    rules:
      - id: pub_fns
        select:
          type: regex
          expression: '^\s*pub fn (\w+)'
        output: {key: functions}

    any_txt_parser -i "src/**/*.rs" -t tpl_funcs.yml -o log_fns.md

Observed: 22 files, `.rs` auto-detected as text (`Format source: content fallback`), one block per file.
Result exit code 1 — expected: files without `pub fn` get `no matches found` (warning). A filled block:

    ## Block: B0020
    Source: src/template.rs
    Format: text / Format source: content fallback
    Status: success
    Items: 11
    ### Data
    - load_template
    - compile_raw
    - validate_encoding

### 6.2 Section titles from a .txt spec via forced markdown format

The spec is `.txt` (auto would say `text`) but its content is Markdown — force it:

    any_txt_parser -i "spec.txt" -t tpl_tz.yml -o log_tz.md --format markdown      # rule: node: heading

Output: `Format: markdown / Format source: CLI override`, headings extracted as items.

## 7. Pitfalls (verified the hard way)

1. **Unbalanced code fences swallow Markdown content.** If heading/paragraph extraction from a .md/.txt
   looks incomplete, count the ``` markers in the source: an odd number means one fence was never closed —
   everything after it is inside a code block and its headings are ignored. Fix the source, or use text
   selectors (`starts_with`, `regex`) instead of markdown nodes.
2. **Exit 1 is not a failure** for project-wide scans — it means "some files had no match". Blocks with
   items are still published; read the Files table.
3. **Windows-1251 logs:** pass `--encoding windows-1251` (or cp1251); UTF-8 BOM is handled automatically.
4. **navguard tree rendering** may nest a non-ASCII-named root file under the last directory group —
   cosmetic; trust `navigator_deep.md`.
5. **Keep the working tree small.** Every copied file costs tokens in the navguard map. Delete scratch
   files (`log_*.md`, `tpl_*.yml`, `navigator*.md`) when the task is done if a clean tree matters.

## 8. How they complement each other (the logic)

- navguard answers WHERE and HOW MUCH: structure, sizes, symbols with line numbers — a ~10 KB table of
  contents for the whole project, zero configuration. For code and Markdown it usually REPLACES the need
  to open files at all: deep2 line numbers are point addresses.
- any_txt_parser answers WHAT IS INSIDE: rule-based extraction across formats with statuses, limits and a
  self-describing audit trail — the query engine over file contents.
- Typical session: navguard map (~2 KB) -> pick files/symbols from deep2 -> read only those lines -> if
  bulk data is needed, one recon + one parser run -> consume its blocks. Total context cost stays small
  because neither tool ever dumps full file contents into your context.
