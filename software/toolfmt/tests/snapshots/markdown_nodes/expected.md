# any_txt_parser output

Triangle: [0,1,0]
Generation: <volatile>
Updated: <volatile>

Parser version: <version>

## Execution

Parser version: <version>
Template version: 1
Generation: <volatile>
Started: <volatile>
Completed: <volatile>

Input:
sources

Output:
out.md

Files discovered: 1
Files processed: 1
Files successful: 1
Files warnings: 0
Files failed: 0
Total extracted items: 32
Total output characters: 9767
Total output blocks: 12
Effective format: auto
Format source: auto detection
Encoding: utf-8
Run status: success
Exit code: 0

## Files

| Path | Size | Modified | Format | Status | Encoding | Hash |
|---|---:|---|---|---|---|---|
| sources/README.md | 763 | <volatile> | markdown | success | utf-8 | - |

## Applied Template

Template name: readme_nodes
Template version: 1
Template source: template.yml

Input format: auto
Format source: auto detection
Encoding: utf-8
Encoding source: default

Rules:

### Rule: headings

Selector type: markdown
Selector:
heading

Output key:
headings

Transforms:
- none
Limits:
- none

### Rule: install_heading

Selector type: markdown
Selector:
heading:^Уст

Output key:
install_heading

Transforms:
- none
Limits:
- none

### Rule: license_paragraphs

Selector type: markdown
Selector:
paragraph:(?i)license|лицензи

Output key:
license_paragraphs

Transforms:
- none
Limits:
- none

### Rule: lists

Selector type: markdown
Selector:
list

Output key:
lists

Transforms:
- none
Limits:
- none

### Rule: list_items

Selector type: markdown
Selector:
list_item

Output key:
list_items

Transforms:
- none
Limits:
- none

### Rule: table

Selector type: markdown
Selector:
table

Output key:
table

Transforms:
- none
Limits:
- none

### Rule: table_rows

Selector type: markdown
Selector:
table_row

Output key:
table_rows

Transforms:
- none
Limits:
- none

### Rule: table_cells

Selector type: markdown
Selector:
table_cell

Output key:
table_cells

Transforms:
- none
Limits:
- none

### Rule: code

Selector type: markdown
Selector:
code_block

Output key:
code

Transforms:
- none
Limits:
- none

### Rule: quotes

Selector type: markdown
Selector:
blockquote

Output key:
quotes

Transforms:
- none
Limits:
- none

### Rule: links

Selector type: markdown
Selector:
link

Output key:
links

Transforms:
- none
Limits:
- none

### Rule: images

Selector type: markdown
Selector:
image

Output key:
images

Transforms:
- none
Limits:
- none


## Effective Template

```yaml
version: 1
name: readme_nodes
input:
  format: auto
  encoding: utf-8
output:
  block:
    target_chars: 28000
    max_chars: 32000
document:
  stable_deduplicate: false
  sort: none
  split_blocks: true
rules:
- id: headings
  select:
    type: markdown
    node: heading
  output:
    key: headings
- id: install_heading
  select:
    type: markdown
    node: heading
    pattern: ^Уст
  output:
    key: install_heading
- id: license_paragraphs
  select:
    type: markdown
    node: paragraph
    pattern: (?i)license|лицензи
  output:
    key: license_paragraphs
- id: lists
  select:
    type: markdown
    node: list
  output:
    key: lists
- id: list_items
  select:
    type: markdown
    node: list_item
  output:
    key: list_items
- id: table
  select:
    type: markdown
    node: table
  output:
    key: table
- id: table_rows
  select:
    type: markdown
    node: table_row
  output:
    key: table_rows
- id: table_cells
  select:
    type: markdown
    node: table_cell
  output:
    key: table_cells
- id: code
  select:
    type: markdown
    node: code_block
  output:
    key: code
- id: quotes
  select:
    type: markdown
    node: blockquote
  output:
    key: quotes
- id: links
  select:
    type: markdown
    node: link
  output:
    key: links
- id: images
  select:
    type: markdown
    node: image
  output:
    key: images
```

## Result Semantics

This file contains extracted data from the declared source files.

Each data block:
- belongs to one source file;
- belongs to one extraction rule;
- contains only data accepted by that rule;
- may be transformed or truncated according to the Applied Template;
- has a stable block anchor.

A block with "no matches found" means the source was processed successfully but the rule produced zero results.

A warning or error means the result may be incomplete for that source or rule.

## CLI Arguments

`-i` `sources` `-t` `template.yml` `-o` `out.md`

## Block Index

| Block | Source | Rule | Key | Items | Size | Start | End | Anc |
|---|---|---|---|---:|---:|---:|---:|---|
| B0001 | sources/README.md | headings | headings | 3 | 0.2 KB | 371 | 392 | B0001 |
| B0002 | sources/README.md | install_heading | install_heading | 1 | 0.2 KB | 393 | 412 | B0002 |
| B0003 | sources/README.md | license_paragraphs | license_paragraphs | 1 | 0.3 KB | 413 | 432 | B0003 |
| B0004 | sources/README.md | lists | lists | 3 | 0.3 KB | 433 | 460 | B0004 |
| B0005 | sources/README.md | list_items | list_items | 7 | 0.3 KB | 461 | 488 | B0005 |
| B0006 | sources/README.md | table | table | 1 | 0.2 KB | 489 | 510 | B0006 |
| B0007 | sources/README.md | table_rows | table_rows | 3 | 0.2 KB | 511 | 532 | B0007 |
| B0008 | sources/README.md | table_cells | table_cells | 6 | 0.2 KB | 533 | 557 | B0008 |
| B0009 | sources/README.md | code | code | 2 | 0.2 KB | 558 | 579 | B0009 |
| B0010 | sources/README.md | quotes | quotes | 2 | 0.2 KB | 580 | 601 | B0010 |
| B0011 | sources/README.md | links | links | 2 | 0.3 KB | 602 | 622 | B0011 |
| B0012 | sources/README.md | images | images | 1 | 0.2 KB | 623 | 642 | B0012 |

### Block Anchors

- B0001 → BLOCK:B0001 (lines 371-392)
- B0002 → BLOCK:B0002 (lines 393-412)
- B0003 → BLOCK:B0003 (lines 413-432)
- B0004 → BLOCK:B0004 (lines 433-460)
- B0005 → BLOCK:B0005 (lines 461-488)
- B0006 → BLOCK:B0006 (lines 489-510)
- B0007 → BLOCK:B0007 (lines 511-532)
- B0008 → BLOCK:B0008 (lines 533-557)
- B0009 → BLOCK:B0009 (lines 558-579)
- B0010 → BLOCK:B0010 (lines 580-601)
- B0011 → BLOCK:B0011 (lines 602-622)
- B0012 → BLOCK:B0012 (lines 623-642)

## Data Blocks

<!-- BLOCK: B0001 START -->
## Block: B0001

Source: sources/README.md
Format: markdown
Format source: extension

Rule: headings
Output key: headings

Status: success
Items: 3
Truncated: no
Split: no

### Data

- any_tool
- Установка
- Пример

<!-- BLOCK: B0001 END -->
<!-- BLOCK: B0002 START -->
## Block: B0002

Source: sources/README.md
Format: markdown
Format source: extension

Rule: install_heading
Output key: install_heading

Status: success
Items: 1
Truncated: no
Split: no

### Data

- Установка

<!-- BLOCK: B0002 END -->
<!-- BLOCK: B0003 START -->
## Block: B0003

Source: sources/README.md
Format: markdown
Format source: extension

Rule: license_paragraphs
Output key: license_paragraphs

Status: success
Items: 1
Truncated: no
Split: no

### Data

- Distributed under the MIT License. Лицензия: MIT, см. https://opensource.org/licenses/MIT.

<!-- BLOCK: B0003 END -->
<!-- BLOCK: B0004 START -->
## Block: B0004

Source: sources/README.md
Format: markdown
Format source: extension

Rule: lists
Output key: lists

Status: success
Items: 3
Truncated: no
Split: no

### Data

- Скачайте архив
    Распакуйте:
    bin/ — исполняемые файлы
    docs/
    Запустите any_tool --help
- bin/ — исполняемые файлы
    docs/
- [x] Windows
    [ ] Linux

<!-- BLOCK: B0004 END -->
<!-- BLOCK: B0005 START -->
## Block: B0005

Source: sources/README.md
Format: markdown
Format source: extension

Rule: list_items
Output key: list_items

Status: success
Items: 7
Truncated: no
Split: no

### Data

- Скачайте архив
- Распакуйте:
    bin/ — исполняемые файлы
    docs/
- bin/ — исполняемые файлы
- docs/
- Запустите any_tool --help
- [x] Windows
- [ ] Linux

<!-- BLOCK: B0005 END -->
<!-- BLOCK: B0006 START -->
## Block: B0006

Source: sources/README.md
Format: markdown
Format source: extension

Rule: table
Output key: table

Status: success
Items: 1
Truncated: no
Split: no

### Data

- Формат | Поддержка
    JSON | да
    YAML | да

<!-- BLOCK: B0006 END -->
<!-- BLOCK: B0007 START -->
## Block: B0007

Source: sources/README.md
Format: markdown
Format source: extension

Rule: table_rows
Output key: table_rows

Status: success
Items: 3
Truncated: no
Split: no

### Data

- Формат | Поддержка
- JSON | да
- YAML | да

<!-- BLOCK: B0007 END -->
<!-- BLOCK: B0008 START -->
## Block: B0008

Source: sources/README.md
Format: markdown
Format source: extension

Rule: table_cells
Output key: table_cells

Status: success
Items: 6
Truncated: no
Split: no

### Data

- Формат
- Поддержка
- JSON
- да
- YAML
- да

<!-- BLOCK: B0008 END -->
<!-- BLOCK: B0009 START -->
## Block: B0009

Source: sources/README.md
Format: markdown
Format source: extension

Rule: code
Output key: code

Status: success
Items: 2
Truncated: no
Split: no

### Data

- any_tool -i logs/ -t tpl.yml \
      -o out.md
- indented code

<!-- BLOCK: B0009 END -->
<!-- BLOCK: B0010 START -->
## Block: B0010

Source: sources/README.md
Format: markdown
Format source: extension

Rule: quotes
Output key: quotes

Status: success
Items: 2
Truncated: no
Split: no

### Data

- Важно: проверьте права.
    Вложенная цитата
- Вложенная цитата

<!-- BLOCK: B0010 END -->
<!-- BLOCK: B0011 START -->
## Block: B0011

Source: sources/README.md
Format: markdown
Format source: extension

Rule: links
Output key: links

Status: success
Items: 2
Truncated: no
Split: no

### Data

- документацию (https://example.org/docs)
- https://opensource.org/licenses/MIT

<!-- BLOCK: B0011 END -->
<!-- BLOCK: B0012 START -->
## Block: B0012

Source: sources/README.md
Format: markdown
Format source: extension

Rule: images
Output key: images

Status: success
Items: 1
Truncated: no
Split: no

### Data

- Логотип (logo.png)

<!-- BLOCK: B0012 END -->

End of any_txt_parser output
