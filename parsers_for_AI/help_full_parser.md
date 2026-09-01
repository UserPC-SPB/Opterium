# any_txt_parser + navguard — AI Field Guide

Two portable Windows tools for AI-assisted code work. Give an AI THIS file plus the two executables
and it can orient in any project and extract data without other documentation.

| Tool | Version | Purpose | Config needed? |
|---|---|---|---|
| `navguard.exe` | 0.1.0 | Structural map: file tree, line/token counts, symbol index | No — just run it |
| `any_txt_parser.exe` | 1.0.2 | Rule-based extractor: YAML template → Markdown log with blocks | Yes — a small YAML template |

Both are single static exes, zero dependencies, ~6.5 MB total.

## 1. Universal Pipeline

Every task follows this sequence:

```
1. ORIENT   navguard --check              → structural map
2. READ     navigator.md                  → understand the project
3. DRILL    navigator_deep.md  L{N}       → stats for one file
            navigator_deep2.md L{a}-L{b}  → symbols with line numbers
4. RECON    unknown file? T1 recon first  → see its shape
5. EXTRACT  any_txt_parser -i <file> -t tpl.yml -o log.md
6. CONSUME  exit code → Files table → Block Index → data blocks
```

**Rule: Never guess a template. Always recon first.**

## 2. Source Type Routing

| Source | navguard | parser | Notes |
|---|---|---|---|
| Code (Rust/Python/JS/TS) | ✅ deep2 indexes symbols | ✅ regex for functions | deep2 is primary navigation |
| Markdown docs | ✅ line/token counts | ✅ markdown nodes | headings, paragraphs, code blocks |
| YAML/JSON config | ✅ counts | ✅ jsonpath/path selectors | structured data extraction |
| TOML config | ✅ counts | ✅ path selector | use `expression: "section.key"` syntax |
| HTML pages | ✅ maps like text | ✅ CSS selectors | NEVER load raw HTML into context |
| XML data | ✅ counts | ✅ xpath selectors | attribute and text extraction |
| Plain text/logs | ✅ counts | ✅ regex/contains/line | line-based extraction |
| Binary files | ✅ maps | ❌ exit 6 | cannot parse binary |
| .gitignore, .editorconfig | ✅ maps | skip | zero tokens, ignore |

## 3. Navguard Commands

```bash
navguard --check              # structural map (always first)
navguard --check --deep2      # with symbol index (for code files)
```

**Output files:**
- `navigator.md` — file tree with line/token counts per file
- `navigator_deep.md` — file list with token ranges (L{N} addresses)
- `navigator_deep2.md` — symbol index with line numbers (code files only)

**Key facts:**
- Triangle `[0,1,0]` = fresh data, safe to read
- Triangle `[0,0,1]` = you've read it, data is consumed
- `--check` is the only flag; everything else is automatic
- Scans from exe location upward until `Cargo.toml` or filesystem root
- Ignores `target/`, `.git/`, `node_modules/`, `__pycache__/` automatically
- deep2 indexes ONLY bare `fn`/`struct`/`enum`/`impl` — `pub fn`, `async fn`, `trait`, `const`, `static` are NOT indexed
- Navigator tree may contain repeated root paths — this is block-based assembly for integrity guarantee on large projects. Navigation by anchors (deep L{N}) remains precise.

## 4. Parser Commands

```bash
any_txt_parser -i <glob> -t <template.yml> -o <output.md>
any_txt_parser -i <glob> -t <template.yml> -o <output.md> --format markdown
any_txt_parser -i <glob> -t <template.yml> -o <output.md> --encoding utf-8
```

**Exit codes:**
| Code | Meaning | Action |
|---|---|---|
| 0 | Full success | Read output |
| 1 | Partial — some warnings | Read output (still published) |
| 2 | Invalid template | Fix YAML template |
| 3 | Invalid CLI/input | Check flags, paths |
| 4 | Output publish failure | Check disk space, permissions |
| 5 | Internal fatal | Report bug |
| 6 | Encoding error OR resource limit | Try `--encoding` or increase limits |

**Key flags:**
- `--format FMT` — force format: `json|xml|html|markdown|yaml|toml|text` (aliases: md, htm, yml, txt)
- `--encoding ENC` — force encoding: `utf-8|utf-16le|windows-1251|cp1251|latin-1`
- `-o <file>` — output file (required)

## 5. Selector Types

| Selector | Use for | Key property |
|---|---|---|
| `jsonpath` | JSON files | `expression: "$.users[*].name"` |
| `path` | YAML, TOML files | `expression: "database.host"` |
| `xpath` | XML files | `expression: "//item/title/text()"` |
| `css` | HTML files | `expression: "h2"`, `expression: "a[href]"` |
| `markdown` | Markdown files | `node: heading\|paragraph\|code_block\|table\|list\|link` |
| `regex` | Any text file | `expression: "fn\\s+(\\w+)"` |
| `contains` | Any text file | `expression: "TODO"` |
| `starts_with` | Any text file | `expression: "def "` |
| `ends_with` | Any text file | `expression: ":"` |
| `line` | Any text file | `from: 1`, `to: 30` |

**TOML syntax:** `expression: "database.host"` (NOT `[database].host`, NOT `$.database.host`)

**Markdown nodes:** `heading`, `paragraph`, `list`, `list_item`, `table`, `table_cell`, `code_block`, `blockquote`, `link`, `image`
**Aliases:** `h`, `p`, `li`, `td`, `code`, `quote`, `img`
**BUG: `table_row` returns 0 items — use `table` instead**

## 6. Transforms

Per item: `trim`, `normalize_whitespace`, `lowercase`, `uppercase`, `unescape`, `truncate_chars N`, `truncate_lines N`, `replace {from, to}`, `regex_replace {from, to}`

Result list: `drop_empty`, `unique`, `sort`, `sort_desc`, `limit_items N`, `join "SEP"`

## 7. Quality Validation

After every parser run, check:

1. **Exit code** — 0=perfect, 1=partial (normal for mixed projects), 2+=problem
2. **Files table** — Status column: `success`/`warning`/`error` per file
3. **Block Index** — Items count per block; 0 items = selector didn't match
4. **Truncation warnings** — `max_items` or `max_item_chars` hit → increase limits
5. **Generation number** — Compare to detect if data changed between runs

## 8. Error Recovery

| Problem | Cause | Fix |
|---|---|---|
| Exit 2: "unknown selector type" | Wrong selector name | Check supported types (section 5) |
| Exit 2: "unknown select properties" | Wrong property name | Check selector-specific properties |
| Exit 2: "invalid path" | Wrong path syntax | TOML: `section.key`, YAML: `$.key` |
| Exit 6: "invalid UTF-8" | Encoding mismatch | Add `--encoding windows-1251` or `cp1251` |
| Exit 6: resource limit | File too large | Increase `max_items`/`max_item_chars` |
| 0 items extracted | Selector doesn't match file content | Recon first, check actual format |
| Garbled output | Wrong encoding | Try `--encoding utf-16le` or `latin-1` |
| deep2 empty for code | File uses `pub fn`, `async fn`, `trait` | Not indexed — use regex selector instead |
| deep2 empty for HTML | HTML has no code symbols | Use CSS selectors instead |

## 9. Core Rules

1. **Orient before extract** — always `navguard --check` first
2. **Recon before template** — never guess selectors; use T1 recon to see file shape
3. **Exit code is truth** — read it before consuming output
4. **Triangle is freshness** — `[0,1,0]` means data is current
5. **Generation is versioning** — compare to detect changes
6. **deep2 is navigation, not content** — use it to find WHERE, then read at those lines
7. **HTML never raw** — convert or use CSS selectors; deep2 is useless for HTML
8. **TOML path is `section.key`** — not `[section].key`, not `$.section.key`
9. **`table_row` is broken** — use `table` node for markdown tables
10. **Exit 1 is normal** — partial success means some files matched, output is valid

## 10. Quick Reference

```bash
# Orient in a project
navguard --check

# See symbols in a code file
navguard --check --deep2
# then read navigator_deep2.md

# Recon an unknown file
any_txt_parser -i mystery.csv -t tpl_recon.yml -o log.md

# Extract with CSS from HTML
any_txt_parser -i page.html -t tpl_css.yml -o log.md

# Extract headings from markdown
any_txt_parser -i docs.md -t tpl_md.yml -o log.md --format markdown

# Extract TOML config values
any_txt_parser -i config.toml -t tpl_toml.yml -o log.md

# Force encoding for non-UTF-8 files
any_txt_parser -i legacy.txt -t tpl.yml -o log.md --encoding windows-1251

# Extract first 30 lines (recon)
any_txt_parser -i file.ext -t tpl_recon.yml -o log.md
```
