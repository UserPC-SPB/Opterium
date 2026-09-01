# learn_full_parser — Examples Library

Detailed examples, verified test data, and YAML templates. AI reads this on-demand when
specific examples are needed for the current task.

---

## 1. Verified Test Data

### 1.1 Project with mixed files (Rust + Python + Markdown)

Source: `D:\command_without_crons\` — mixed project with agent protocols, web tests, and library code

| Metric | Value |
|---|---|
| Files indexed | 20+ |
| Text files | varies |
| Total lines | 2,000+ |
| Total tokens | 13,000+ |
| deep2 symbols | Markdown headings + code symbols |

deep2 indexes:
- Markdown headings as navigation symbols (e.g., "Agent Core Protocol", "STEP 1 — Navigator")
- Python function names in `.py` files
- Rust function names in `.rs` files (bare `fn` only)

**Key insight:** deep2 indexes ONLY bare `fn`/`struct`/`enum`/`impl` for code. `pub fn`, `async fn`, `trait`, `const`, `static` are NOT indexed. For markdown, it indexes headings as navigation points.

### 1.2 Python Documentation (HTML)

Source: `https://docs.python.org/3/tutorial/datastructures.html`

| Metric | Raw HTML | Converted .txt | CSS Parser | MD Parser |
|---|---|---|---|---|
| File size | 101,284 B | 24,393 B | 10,570 B | 15,534 B |
| Lines | 1,064 | 912 | 442 | 590 |
| Tokens (navguard) | 16,888 | 4,505 | N/A | N/A |
| deep2 symbols | EMPTY | 19 headings | N/A | N/A |

**Size reduction:** 101KB → 24KB = 75.9% smaller (by bytes)
**Token reduction:** 16,888 → 4,505 = 73.3% fewer tokens

**Key insight:** deep2 is EMPTY for HTML files — no symbol navigation possible. Use CSS selectors instead.

### 1.3 MarkupSafe Repository

Source: `pallets/markupsafe` GitHub repo (tarball download)

| Metric | Value |
|---|---|
| Total files | 46 |
| Text files | 26 |
| Total lines | 2,752 |
| Total tokens | 20,963 |
| Bytes per token | ~12 |
| deep2 symbols | Python classes, methods, functions with line numbers |

deep2 indexed symbols in `__init__.py`:
- `_HasHTML` (line 16), `escape` (line 24), `escape_silent` (line 48)
- `soft_str` (line 64), `Markup` (line 84)
- All methods of `Markup` class: `__add__`, `join`, `split`, `format`, etc.

### 1.4 TOML File Test

Source: Custom TOML fixture

```toml
[database]
host = "localhost"
port = 5432

[server]
host = "0.0.0.0"
port = 8080
```

**Correct template:**
```yaml
version: 1
name: toml_test
input: {format: auto}
rules:
  - id: db_host
    select: {type: path, expression: "database.host"}
    output: {key: db_host}
  - id: server_host
    select: {type: path, expression: "server.host"}
    output: {key: server_host}
```

**Result:** Exit 0, both values extracted: `localhost` and `0.0.0.0`

**Wrong syntaxes (all fail with exit 2):**
- `type: yaml` → "unknown selector type 'yaml'"
- `path: "[database].host"` → "invalid array index 'database'"
- `path: "$.database.host"` → 0 items extracted
- `expression: "[database].host"` with `type: path` → "invalid array index 'database'"

### 1.5 Markdown Table Test

Source: Custom markdown fixture

```markdown
| Name | Age | City |
|------|-----|------|
| Alice | 30 | NYC |
| Bob | 25 | LA |
```

**Correct template:**
```yaml
version: 1
name: table_test
input: {format: auto}
rules:
  - id: tables
    select: {type: markdown, node: table}
    output: {key: tables}
```

**Result:** Exit 1, 1 table extracted

**BUG: `table_row` node returns 0 items — always use `table` instead**

---

## 2. YAML Template Library

### T1 — Recon: first look at unknown file

```yaml
version: 1
name: recon
input: {format: auto}
rules:
  - id: head
    select: {type: line, from: 1, to: 30}
    output: {key: sample}
```

**Usage:** `any_txt_parser -i <file> -t tpl_recon.yml -o log_recon.md`
**Result:** One block `sample` = first 30 lines of the file
**Works on:** Any text file (.py, .rs, .csv, .xyz, etc.) without `--format` flag

### T2 — Function names across source files

```yaml
version: 1
name: code_fns
input: {format: auto}
rules:
  - id: fns
    when: {format: text}
    select:
      type: regex
      expression: "^(?:pub\\s+)?(?:async\\s+)?fn\\s+(\\w+)"
    transforms: [unique]
    output: {key: functions}
```

**Usage:** `any_txt_parser -i src/**/*.rs -t tpl_code_fns.yml -o log.md`

### T3 — CSS extraction from HTML

```yaml
version: 1
name: css_extract
input: {format: auto}
rules:
  - id: h1
    select: {type: css, expression: "h1"}
    output: {key: h1}
  - id: h2
    select: {type: css, expression: "h2"}
    output: {key: h2}
  - id: paragraphs
    select: {type: css, expression: "p"}
    transforms: [trim, normalize_whitespace]
    limits: {max_items: 20, max_item_chars: 300}
    output: {key: paragraphs}
  - id: links
    select: {type: css, expression: "a[href]"}
    limits: {max_items: 30}
    output: {key: links}
```

**Result on Python docs HTML:**
- h1: 1 item
- h2: 8 items
- h3: 10 items
- paragraphs: 20 items (capped, 79 found, 63 omitted)
- links: 30 items (capped, 113 found, 83 omitted)

### T4 — Markdown nodes extraction

```yaml
version: 1
name: md_extract
input: {format: auto}
rules:
  - id: headings
    select: {type: markdown, node: heading}
    output: {key: headings}
  - id: paragraphs
    select: {type: markdown, node: paragraph}
    transforms: [trim, normalize_whitespace]
    limits: {max_items: 20, max_item_chars: 300}
    output: {key: paragraphs}
  - id: code_blocks
    select: {type: markdown, node: code_block}
    output: {key: code}
```

**Usage:** `any_txt_parser -i docs.md -t tpl_md.yml -o log.md --format markdown`
**Note:** Requires `--format markdown` if file has `.txt` extension

### T5 — YAML/JSON config extraction

```yaml
version: 1
name: config_extract
input: {format: auto}
rules:
  - id: database
    select: {type: path, expression: "$.database"}
    output: {key: database}
  - id: server
    select: {type: path, expression: "$.server"}
    output: {key: server}
```

### T6 — TOML config extraction

```yaml
version: 1
name: toml_extract
input: {format: auto}
rules:
  - id: database
    select: {type: path, expression: "database"}
    output: {key: database}
  - id: server
    select: {type: path, expression: "server"}
    output: {key: server}
```

### T7 — XML extraction with xpath

```yaml
version: 1
name: xml_extract
input: {format: auto}
rules:
  - id: items
    select: {type: xpath, expression: "//item"}
    output: {key: items}
  - id: titles
    select: {type: xpath, expression: "//item/title/text()"}
    output: {key: titles}
```

### T8 — Log file pattern matching

```yaml
version: 1
name: log_extract
input: {format: auto}
rules:
  - id: errors
    select: {type: contains, expression: "ERROR"}
    output: {key: errors}
  - id: warnings
    select: {type: contains, expression: "WARN"}
    output: {key: warnings}
  - id: timestamps
    select: {type: regex, expression: "\\d{4}-\\d{2}-\\d{2} \\d{2}:\\d{2}:\\d{2}"}
    output: {key: timestamps}
```

---

## 3. Output Structure Reference

### Parser output format

```
# any_txt_parser output
Triangle: [0,1,0]
Generation: 00000001
Updated: 2026-09-01T08:43:13Z

## Execution
Parser version: 1.0.2
...
Input: <filename>
Output: <output_file>
Files discovered: N
Files processed: N
Files successful: N
Files warnings: N
Files failed: N
Total extracted items: N
Total output characters: N
Total output blocks: N

## Files
| Path | Size | Modified | Format | Status | Hash |

## Block Index
| B0001 | filename | rule_id | output_key | items | size | start_line | end_line | anchor |

## Block: B0001
Source: filename
Format: text
Format source: auto detection
Rule: rule_id
Output key: output_key
Status: success
Items: N
### Data
- item1
- item2
```

### Navguard output format

```
T: 0,1,0
F: path/to/file.rs|lines|tokens
F: path/to/file.py|lines|tokens
...
B: sync|2026-09-01T11:46:04
B: rules|none
```

### Navguard deep2 format

```
# Navigator Deep2 — Symbol Index
Updated: 2026-09-01T11:46:04

## filename.rs
- StructName | line 42
- function_name | line 78
- EnumVariant | line 105
```

---

## 4. Edge Cases

### 4.1 cp1251 encoding without --encoding

```
Input: windows-1251 encoded file
Command: any_txt_parser -i file.txt -t tpl.yml -o log.md
Result: Exit 6, no output published
Fix: Add --encoding windows-1251
```

### 4.2 TOML with brackets in path

```
Expression: "[database].host"
Result: Exit 2, "invalid array index 'database'"
Fix: Use "database.host" (no brackets)
```

### 4.3 Markdown table_row node

```
Node: table_row
Result: 0 items on valid GFM tables
Fix: Use "table" node instead
```

### 4.4 deep2 for HTML files

```
File: page.html
deep2: EMPTY (no symbols indexed)
Fix: Use CSS selectors for HTML content
```

### 4.5 Format detection for .txt files

```
File: page_converted.txt (has markdown headings)
Auto-detected: format=text
Markdown nodes: 0 items
Fix: Add --format markdown to force markdown parsing
```

### 4.6 max_items truncation

```
Items found: 79
max_items: 20
Result: 20 items extracted, warning in Truncation section, exit 1
Fix: Increase max_items or accept partial results
```

---

## 5. Performance Benchmarks

| Operation | Time | Notes |
|---|---|---|
| navguard --check (small project) | <100ms | 20 files, 2K lines |
| navguard --check (medium project) | <500ms | 46 files, 2.7K lines |
| parser (CSS extraction from HTML) | <200ms | 101KB input, 5 blocks |
| parser (markdown nodes) | <100ms | 24KB input, 3 blocks |
| parser (TOML path) | <50ms | 85 bytes input, 2 blocks |

Both tools are fast enough for interactive use. No performance bottlenecks for typical projects.

---

## 6. File Size Summary

| Source | Original | After Processing | Reduction |
|---|---|---|---|
| Python docs HTML | 101,284 B | 24,393 B (converted) | 75.9% |
| Python docs HTML | 101,284 B | 10,570 B (CSS parser) | 89.6% |
| MarkupSafe repo | 252,239 B | 20,963 tokens | ~92% (by tokens) |

---

## 7. Selector Syntax Quick Reference

| Format | Selector type | Expression syntax |
|---|---|---|
| JSON | jsonpath | `$.users[*].name` |
| YAML | path | `$.settings.timeout` |
| TOML | path | `database.host` (NO brackets, NO $) |
| XML | xpath | `//item/title/text()` |
| HTML | css | `h2`, `a[href]`, `table tr td` |
| Markdown | markdown | node: `heading`, `paragraph`, `code_block` |
| Text | regex | `fn\\s+(\\w+)` |
| Text | contains | `TODO` |
| Text | line | from: 1, to: 30 |
