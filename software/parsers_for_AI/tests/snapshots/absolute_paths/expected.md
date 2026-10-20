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
<CASE>/sources

Output:
<CASE>/out.md

Files discovered: 2
Files processed: 2
Files successful: 2
Files warnings: 0
Files failed: 0
Total extracted items: 3
Total output characters: 3550
Total output blocks: 2
Effective format: auto
Format source: auto detection
Encoding: utf-8
Run status: success
Exit code: 0

## Files

| Path | Size | Modified | Format | Status | Encoding | Hash |
|---|---:|---|---|---|---|---|
| <CASE>/sources/nested/orders.json | 90 | <volatile> | json | success | utf-8 | - |
| <CASE>/sources/notes.txt | 35 | <volatile> | text | success | utf-8 | - |

## Applied Template

Template name: absolute_paths
Template version: 1
Template source: <CASE>/template.yml

Input format: auto
Format source: auto detection
Encoding: utf-8
Encoding source: default

Rules:

### Rule: order_ids

Selector type: jsonpath
Selector:
$.orders[*].id

Output key:
order_ids

Applies to format: json
Transforms:
- none
Limits:
- none

### Rule: warnings

Selector type: contains
Selector:
WARN

Output key:
warnings

Applies to format: text
Transforms:
- none
Limits:
- none


## Effective Template

```yaml
version: 1
name: absolute_paths
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
- id: order_ids
  when:
    format: json
  select:
    type: jsonpath
    expression: $.orders[*].id
  output:
    key: order_ids
- id: warnings
  when:
    format: text
  select:
    type: contains
    expression: WARN
  output:
    key: warnings
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

A rule with "Applies to format" runs only on source files of that format; other files get no block for it, and their status is not affected.

A warning or error means the result may be incomplete for that source or rule.

## CLI Arguments

`-i` `<CASE>/sources` `-t` `<CASE>/template.yml` `-o` `<CASE>/out.md`

## Block Index

| Block | Source | Rule | Key | Items | Size | Start | End | Anc |
|---|---|---|---|---:|---:|---:|---:|---|
| B0001 | <CASE>/sources/nested/orders.json | order_ids | order_ids | 2 | 0.2 KB | 158 | 178 | B0001 |
| B0002 | <CASE>/sources/notes.txt | warnings | warnings | 1 | 0.2 KB | 179 | 198 | B0002 |

### Block Anchors

- B0001 → BLOCK:B0001 (lines 158-178)
- B0002 → BLOCK:B0002 (lines 179-198)

## Data Blocks

<!-- BLOCK: B0001 START -->
## Block: B0001

Source: <CASE>/sources/nested/orders.json
Format: json
Format source: extension

Rule: order_ids
Output key: order_ids

Status: success
Items: 2
Truncated: no
Split: no

### Data

- A-100
- A-101

<!-- BLOCK: B0001 END -->
<!-- BLOCK: B0002 START -->
## Block: B0002

Source: <CASE>/sources/notes.txt
Format: text
Format source: extension

Rule: warnings
Output key: warnings

Status: success
Items: 1
Truncated: no
Split: no

### Data

- WARN disk 91%

<!-- BLOCK: B0002 END -->

End of any_txt_parser output
