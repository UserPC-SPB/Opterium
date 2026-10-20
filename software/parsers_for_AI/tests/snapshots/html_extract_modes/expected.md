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
Total extracted items: 15
Total output characters: 7850
Total output blocks: 9
Effective format: auto
Format source: auto detection
Encoding: utf-8
Run status: success
Exit code: 0

## Files

| Path | Size | Modified | Format | Status | Encoding | Hash |
|---|---:|---|---|---|---|---|
| sources/product.html | 1100 | <volatile> | html | success | utf-8 | - |

## Applied Template

Template name: product_page
Template version: 1
Template source: template.yml

Input format: auto
Format source: auto detection
Encoding: utf-8
Encoding source: default

Rules:

### Rule: title

Selector type: css
Selector:
title

Output key:
title

Transforms:
- none
Limits:
- none

### Rule: headings

Selector type: css
Selector:
h1, h2

Output key:
headings

Transforms:
- none
Limits:
- none

### Rule: nav_links

Selector type: css
Selector:
nav a[href]

Output key:
nav_links

Extract: attr:href
Transforms:
- none
Limits:
- none

### Rule: nav_texts

Selector type: css
Selector:
nav a

Output key:
nav_texts

Transforms:
- none
Limits:
- none

### Rule: lead_outer

Selector type: css
Selector:
article > p.lead

Output key:
lead_outer

Extract: outer_html
Transforms:
- none
Limits:
- none

### Rule: note_inner

Selector type: css
Selector:
.note

Output key:
note_inner

Extract: inner_html
Transforms:
- none
Limits:
- none

### Rule: price_rows

Selector type: css
Selector:
table.prices tr

Output key:
price_rows

Transforms:
- none
Limits:
- none

### Rule: image_alts

Selector type: css
Selector:
img

Output key:
image_alts

Extract: attr:alt
Transforms:
- none
Limits:
- none

### Rule: article_text

Selector type: css
Selector:
article

Output key:
article_text

Transforms:
- none
Limits:
- none


## Effective Template

```yaml
version: 1
name: product_page
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
- id: title
  select:
    type: css
    expression: title
  output:
    key: title
- id: headings
  select:
    type: css
    expression: h1, h2
  output:
    key: headings
- id: nav_links
  select:
    type: css
    expression: nav a[href]
  output:
    key: nav_links
  extract: attr:href
- id: nav_texts
  select:
    type: css
    expression: nav a
  output:
    key: nav_texts
- id: lead_outer
  select:
    type: css
    expression: article > p.lead
  output:
    key: lead_outer
  extract: outer_html
- id: note_inner
  select:
    type: css
    expression: .note
  output:
    key: note_inner
  extract: inner_html
- id: price_rows
  select:
    type: css
    expression: table.prices tr
  output:
    key: price_rows
- id: image_alts
  select:
    type: css
    expression: img
  output:
    key: image_alts
  extract: attr:alt
- id: article_text
  select:
    type: css
    expression: article
  output:
    key: article_text
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
| B0001 | sources/product.html | title | title | 1 | 0.2 KB | 311 | 330 | B0001 |
| B0002 | sources/product.html | headings | headings | 2 | 0.2 KB | 331 | 351 | B0002 |
| B0003 | sources/product.html | nav_links | nav_links | 2 | 0.2 KB | 352 | 372 | B0003 |
| B0004 | sources/product.html | nav_texts | nav_texts | 3 | 0.2 KB | 373 | 394 | B0004 |
| B0005 | sources/product.html | lead_outer | lead_outer | 1 | 0.3 KB | 395 | 414 | B0005 |
| B0006 | sources/product.html | note_inner | note_inner | 1 | 0.2 KB | 415 | 434 | B0006 |
| B0007 | sources/product.html | price_rows | price_rows | 3 | 0.2 KB | 435 | 456 | B0007 |
| B0008 | sources/product.html | image_alts | image_alts | 1 | 0.2 KB | 457 | 476 | B0008 |
| B0009 | sources/product.html | article_text | article_text | 1 | 0.4 KB | 477 | 503 | B0009 |

### Block Anchors

- B0001 → BLOCK:B0001 (lines 311-330)
- B0002 → BLOCK:B0002 (lines 331-351)
- B0003 → BLOCK:B0003 (lines 352-372)
- B0004 → BLOCK:B0004 (lines 373-394)
- B0005 → BLOCK:B0005 (lines 395-414)
- B0006 → BLOCK:B0006 (lines 415-434)
- B0007 → BLOCK:B0007 (lines 435-456)
- B0008 → BLOCK:B0008 (lines 457-476)
- B0009 → BLOCK:B0009 (lines 477-503)

## Data Blocks

<!-- BLOCK: B0001 START -->
## Block: B0001

Source: sources/product.html
Format: html
Format source: extension

Rule: title
Output key: title

Status: success
Items: 1
Truncated: no
Split: no

### Data

- Молоток — Инструменты

<!-- BLOCK: B0001 END -->
<!-- BLOCK: B0002 START -->
## Block: B0002

Source: sources/product.html
Format: html
Format source: extension

Rule: headings
Output key: headings

Status: success
Items: 2
Truncated: no
Split: no

### Data

- Молоток столярный
- Цены

<!-- BLOCK: B0002 END -->
<!-- BLOCK: B0003 START -->
## Block: B0003

Source: sources/product.html
Format: html
Format source: extension

Rule: nav_links
Output key: nav_links

Status: success
Items: 2
Truncated: no
Split: no

### Data

- /
- /tools?sort=price&dir=asc

<!-- BLOCK: B0003 END -->
<!-- BLOCK: B0004 START -->
## Block: B0004

Source: sources/product.html
Format: html
Format source: extension

Rule: nav_texts
Output key: nav_texts

Status: success
Items: 3
Truncated: no
Split: no

### Data

- Главная
- Инструменты
- Без ссылки

<!-- BLOCK: B0004 END -->
<!-- BLOCK: B0005 START -->
## Block: B0005

Source: sources/product.html
Format: html
Format source: extension

Rule: lead_outer
Output key: lead_outer

Status: success
Items: 1
Truncated: no
Split: no

### Data

- <p class="lead">Кованый боёк, <b>ясеневая</b> рукоять&nbsp;— 450&nbsp;г.</p>

<!-- BLOCK: B0005 END -->
<!-- BLOCK: B0006 START -->
## Block: B0006

Source: sources/product.html
Format: html
Format source: extension

Rule: note_inner
Output key: note_inner

Status: success
Items: 1
Truncated: no
Split: no

### Data

- Гарантия <em>2 года</em>.<br>Доставка — бесплатно.

<!-- BLOCK: B0006 END -->
<!-- BLOCK: B0007 START -->
## Block: B0007

Source: sources/product.html
Format: html
Format source: extension

Rule: price_rows
Output key: price_rows

Status: success
Items: 3
Truncated: no
Split: no

### Data

- Магазин	Цена
- Склад №1	1 290 ₽
- Склад №2	1 500 ₽ 1 390 ₽

<!-- BLOCK: B0007 END -->
<!-- BLOCK: B0008 START -->
## Block: B0008

Source: sources/product.html
Format: html
Format source: extension

Rule: image_alts
Output key: image_alts

Status: success
Items: 1
Truncated: no
Split: no

### Data

- Молоток, вид сбоку

<!-- BLOCK: B0008 END -->
<!-- BLOCK: B0009 START -->
## Block: B0009

Source: sources/product.html
Format: html
Format source: extension

Rule: article_text
Output key: article_text

Status: success
Items: 1
Truncated: no
Split: no

### Data

- Молоток столярный
    Кованый боёк, ясеневая рукоять — 450 г.
    Цены
    Магазин	Цена
    Склад №1	1 290 ₽
    Склад №2	1 500 ₽ 1 390 ₽
    Гарантия 2 года.
    Доставка — бесплатно.

<!-- BLOCK: B0009 END -->

End of any_txt_parser output
