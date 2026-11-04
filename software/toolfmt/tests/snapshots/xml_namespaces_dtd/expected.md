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
Total extracted items: 14
Total output characters: 7875
Total output blocks: 9
Effective format: auto
Format source: auto detection
Encoding: utf-8
Run status: success
Exit code: 0

## Files

| Path | Size | Modified | Format | Status | Encoding | Hash |
|---|---:|---|---|---|---|---|
| sources/catalog.xml | 842 | <volatile> | xml | success | utf-8 | - |

## Applied Template

Template name: book_catalog
Template version: 1
Template source: template.yml

Input format: auto
Format source: auto detection
Encoding: utf-8
Encoding source: default

Rules:

### Rule: book_titles

Selector type: xpath
Selector:
//item[@type='book']/title/text()

Output key:
book_titles

Transforms:
- none
Limits:
- none

### Rule: item_ids

Selector type: xpath
Selector:
/catalog/item/@id

Output key:
item_ids

Transforms:
- none
Limits:
- none

### Rule: creators

Selector type: xpath
Selector:
//dc:creator/text()

Output key:
creators

Transforms:
- none
Limits:
- none

### Rule: currencies

Selector type: xpath
Selector:
//price

Output key:
currencies

Extract: attr:currency
Transforms:
- none
Limits:
- none

### Rule: languages

Selector type: xpath
Selector:
//item

Output key:
languages

Extract: attr:xml:lang
Transforms:
- none
Limits:
- none

### Rule: note_inner

Selector type: xpath
Selector:
//note

Output key:
note_inner

Extract: inner_html
Transforms:
- none
Limits:
- none

### Rule: second_item

Selector type: xpath
Selector:
/catalog/item[2]

Output key:
second_item

Extract: outer_html
Transforms:
- none
Limits:
- none

### Rule: footer_text

Selector type: xpath
Selector:
/catalog/footer/text()

Output key:
footer_text

Transforms:
- none
Limits:
- none

### Rule: catalog_version

Selector type: xpath
Selector:
/catalog/@version

Output key:
catalog_version

Transforms:
- none
Limits:
- none


## Effective Template

```yaml
version: 1
name: book_catalog
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
- id: book_titles
  select:
    type: xpath
    expression: //item[@type='book']/title/text()
  output:
    key: book_titles
- id: item_ids
  select:
    type: xpath
    expression: /catalog/item/@id
  output:
    key: item_ids
- id: creators
  select:
    type: xpath
    expression: //dc:creator/text()
  output:
    key: creators
- id: currencies
  select:
    type: xpath
    expression: //price
  output:
    key: currencies
  extract: attr:currency
- id: languages
  select:
    type: xpath
    expression: //item
  output:
    key: languages
  extract: attr:xml:lang
- id: note_inner
  select:
    type: xpath
    expression: //note
  output:
    key: note_inner
  extract: inner_html
- id: second_item
  select:
    type: xpath
    expression: /catalog/item[2]
  output:
    key: second_item
  extract: outer_html
- id: footer_text
  select:
    type: xpath
    expression: /catalog/footer/text()
  output:
    key: footer_text
- id: catalog_version
  select:
    type: xpath
    expression: /catalog/@version
  output:
    key: catalog_version
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
| B0001 | sources/catalog.xml | book_titles | book_titles | 2 | 0.2 KB | 311 | 331 | B0001 |
| B0002 | sources/catalog.xml | item_ids | item_ids | 3 | 0.2 KB | 332 | 353 | B0002 |
| B0003 | sources/catalog.xml | creators | creators | 2 | 0.2 KB | 354 | 374 | B0003 |
| B0004 | sources/catalog.xml | currencies | currencies | 2 | 0.2 KB | 375 | 395 | B0004 |
| B0005 | sources/catalog.xml | languages | languages | 1 | 0.2 KB | 396 | 415 | B0005 |
| B0006 | sources/catalog.xml | note_inner | note_inner | 1 | 0.2 KB | 416 | 435 | B0006 |
| B0007 | sources/catalog.xml | second_item | second_item | 1 | 0.3 KB | 436 | 458 | B0007 |
| B0008 | sources/catalog.xml | footer_text | footer_text | 1 | 0.2 KB | 459 | 478 | B0008 |
| B0009 | sources/catalog.xml | catalog_version | catalog_version | 1 | 0.2 KB | 479 | 498 | B0009 |

### Block Anchors

- B0001 → BLOCK:B0001 (lines 311-331)
- B0002 → BLOCK:B0002 (lines 332-353)
- B0003 → BLOCK:B0003 (lines 354-374)
- B0004 → BLOCK:B0004 (lines 375-395)
- B0005 → BLOCK:B0005 (lines 396-415)
- B0006 → BLOCK:B0006 (lines 416-435)
- B0007 → BLOCK:B0007 (lines 436-458)
- B0008 → BLOCK:B0008 (lines 459-478)
- B0009 → BLOCK:B0009 (lines 479-498)

## Data Blocks

<!-- BLOCK: B0001 START -->
## Block: B0001

Source: sources/catalog.xml
Format: xml
Format source: extension

Rule: book_titles
Output key: book_titles

Status: success
Items: 2
Truncated: no
Split: no

### Data

- Война и мир
- Master & Margarita

<!-- BLOCK: B0001 END -->
<!-- BLOCK: B0002 START -->
## Block: B0002

Source: sources/catalog.xml
Format: xml
Format source: extension

Rule: item_ids
Output key: item_ids

Status: success
Items: 3
Truncated: no
Split: no

### Data

- b1
- m1
- b2

<!-- BLOCK: B0002 END -->
<!-- BLOCK: B0003 START -->
## Block: B0003

Source: sources/catalog.xml
Format: xml
Format source: extension

Rule: creators
Output key: creators

Status: success
Items: 2
Truncated: no
Split: no

### Data

- Лев Толстой
- Михаил Булгаков

<!-- BLOCK: B0003 END -->
<!-- BLOCK: B0004 START -->
## Block: B0004

Source: sources/catalog.xml
Format: xml
Format source: extension

Rule: currencies
Output key: currencies

Status: success
Items: 2
Truncated: no
Split: no

### Data

- RUB
- RUB

<!-- BLOCK: B0004 END -->
<!-- BLOCK: B0005 START -->
## Block: B0005

Source: sources/catalog.xml
Format: xml
Format source: extension

Rule: languages
Output key: languages

Status: success
Items: 1
Truncated: no
Split: no

### Data

- ru

<!-- BLOCK: B0005 END -->
<!-- BLOCK: B0006 START -->
## Block: B0006

Source: sources/catalog.xml
Format: xml
Format source: extension

Rule: note_inner
Output key: note_inner

Status: success
Items: 1
Truncated: no
Split: no

### Data

- Издание &vendor;, <b>твёрдый</b> переплёт

<!-- BLOCK: B0006 END -->
<!-- BLOCK: B0007 START -->
## Block: B0007

Source: sources/catalog.xml
Format: xml
Format source: extension

Rule: second_item
Output key: second_item

Status: success
Items: 1
Truncated: no
Split: no

### Data

- <item id="m1" type="magazine">
        <title>Наука и жизнь</title>
        <price currency="RUB">250</price>
      </item>

<!-- BLOCK: B0007 END -->
<!-- BLOCK: B0008 START -->
## Block: B0008

Source: sources/catalog.xml
Format: xml
Format source: extension

Rule: footer_text
Output key: footer_text

Status: success
Items: 1
Truncated: no
Split: no

### Data

- © 2026 ООО «Книжный дом»

<!-- BLOCK: B0008 END -->
<!-- BLOCK: B0009 START -->
## Block: B0009

Source: sources/catalog.xml
Format: xml
Format source: extension

Rule: catalog_version
Output key: catalog_version

Status: success
Items: 1
Truncated: no
Split: no

### Data

- 2.1

<!-- BLOCK: B0009 END -->

End of any_txt_parser output
