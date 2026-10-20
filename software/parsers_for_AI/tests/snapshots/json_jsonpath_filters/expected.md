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
Total extracted items: 11
Total output characters: 6233
Total output blocks: 6
Effective format: auto
Format source: auto detection
Encoding: utf-8
Run status: success
Exit code: 0

## Files

| Path | Size | Modified | Format | Status | Encoding | Hash |
|---|---:|---|---|---|---|---|
| sources/orders.json | 1239 | <volatile> | json | success | utf-8 | - |

## Applied Template

Template name: shop_orders
Template version: 1
Template source: template.yml

Input format: auto
Format source: auto detection
Encoding: utf-8
Encoding source: default

Rules:

### Rule: failed_big_orders

Selector type: jsonpath
Selector:
$.orders[?(@.status == 'failed' && @.total > 100)].id

Output key:
failed_order_ids

Transforms:
- none
Limits:
- none

### Rule: customers_to_review

Selector type: jsonpath
Selector:
$.customers[?(@.tier == 'vip' && @.city == 'Санкт-Петербург' || @.blocked == true)].name

Output key:
customers_to_review

Transforms:
- none
Limits:
- none

### Rule: skus

Selector type: jsonpath
Selector:
$.orders[*].items[*].sku

Output key:
skus

Transforms:
- unique
- sort
Limits:
- none

### Rule: notes

Selector type: jsonpath
Selector:
$..note

Output key:
notes

Transforms:
- trim
- drop_empty
Limits:
- none

### Rule: first_order

Selector type: path
Selector:
$.orders[0]

Output key:
first_order

Transforms:
- none
Limits:
- none

### Rule: api_version

Selector type: path
Selector:
meta.api.version

Output key:
api_version

Transforms:
- none
Limits:
- none


## Effective Template

```yaml
version: 1
name: shop_orders
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
- id: failed_big_orders
  select:
    type: jsonpath
    expression: $.orders[?(@.status == 'failed' && @.total > 100)].id
  output:
    key: failed_order_ids
- id: customers_to_review
  select:
    type: jsonpath
    expression: $.customers[?(@.tier == 'vip' && @.city == 'Санкт-Петербург' || @.blocked == true)].name
  output:
    key: customers_to_review
- id: skus
  select:
    type: jsonpath
    expression: $.orders[*].items[*].sku
  output:
    key: skus
  transforms:
  - unique
  - sort
- id: notes
  select:
    type: jsonpath
    expression: $..note
  output:
    key: notes
  transforms:
  - trim
  - drop_empty
- id: first_order
  select:
    type: path
    expression: $.orders[0]
  output:
    key: first_order
- id: api_version
  select:
    type: path
    expression: meta.api.version
  output:
    key: api_version
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
| B0001 | sources/orders.json | failed_big_orders | failed_order_ids | 2 | 0.2 KB | 245 | 265 | B0001 |
| B0002 | sources/orders.json | customers_to_review | customers_to_review | 2 | 0.2 KB | 266 | 286 | B0002 |
| B0003 | sources/orders.json | skus | skus | 3 | 0.2 KB | 287 | 308 | B0003 |
| B0004 | sources/orders.json | notes | notes | 2 | 0.2 KB | 309 | 330 | B0004 |
| B0005 | sources/orders.json | first_order | first_order | 1 | 0.3 KB | 331 | 350 | B0005 |
| B0006 | sources/orders.json | api_version | api_version | 1 | 0.2 KB | 351 | 370 | B0006 |

### Block Anchors

- B0001 → BLOCK:B0001 (lines 245-265)
- B0002 → BLOCK:B0002 (lines 266-286)
- B0003 → BLOCK:B0003 (lines 287-308)
- B0004 → BLOCK:B0004 (lines 309-330)
- B0005 → BLOCK:B0005 (lines 331-350)
- B0006 → BLOCK:B0006 (lines 351-370)

## Data Blocks

<!-- BLOCK: B0001 START -->
## Block: B0001

Source: sources/orders.json
Format: json
Format source: extension

Rule: failed_big_orders
Output key: failed_order_ids

Status: success
Items: 2
Truncated: no
Split: no

### Data

- A-1001
- A-1005

<!-- BLOCK: B0001 END -->
<!-- BLOCK: B0002 START -->
## Block: B0002

Source: sources/orders.json
Format: json
Format source: extension

Rule: customers_to_review
Output key: customers_to_review

Status: success
Items: 2
Truncated: no
Split: no

### Data

- Анна Ёлкина
- Chen Wei 陈伟

<!-- BLOCK: B0002 END -->
<!-- BLOCK: B0003 START -->
## Block: B0003

Source: sources/orders.json
Format: json
Format source: extension

Rule: skus
Output key: skus

Status: success
Items: 3
Truncated: no
Split: no

### Data

- HAM-01
- NAIL-50
- SAW-02

<!-- BLOCK: B0003 END -->
<!-- BLOCK: B0004 START -->
## Block: B0004

Source: sources/orders.json
Format: json
Format source: extension

Rule: notes
Output key: notes

Status: success
Items: 2
Truncated: no
Split: no

### Data

- Позвонить клиенту
    после 18:00
- Оставить у двери 🚪

<!-- BLOCK: B0004 END -->
<!-- BLOCK: B0005 START -->
## Block: B0005

Source: sources/orders.json
Format: json
Format source: extension

Rule: first_order
Output key: first_order

Status: success
Items: 1
Truncated: no
Split: no

### Data

- {"id":"A-1001","status":"failed","total":250.5,"items":[{"sku":"HAM-01","qty":1},{"sku":"NAIL-50","qty":3}],"note":"  Позвонить клиенту\nпосле 18:00  "}

<!-- BLOCK: B0005 END -->
<!-- BLOCK: B0006 START -->
## Block: B0006

Source: sources/orders.json
Format: json
Format source: extension

Rule: api_version
Output key: api_version

Status: success
Items: 1
Truncated: no
Split: no

### Data

- 2026-10

<!-- BLOCK: B0006 END -->

End of any_txt_parser output
