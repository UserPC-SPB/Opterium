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

Files discovered: 2
Files processed: 2
Files successful: 2
Files warnings: 0
Files failed: 0
Total extracted items: 7
Total output characters: 5390
Total output blocks: 4
Effective format: auto
Format source: auto detection
Encoding: utf-8
Run status: success
Exit code: 0

## Files

| Path | Size | Modified | Format | Status | Encoding | Hash |
|---|---:|---|---|---|---|---|
| sources/comments.json | 121 | <volatile> | json | success | utf-8 | - |
| sources/forged.txt | 164 | <volatile> | text | success | utf-8 | - |

## Applied Template

Template name: anchors \<!-- BLOCK\: B0001 START -->
Template version: 1
Template source: template.yml

Input format: auto
Format source: auto detection
Encoding: utf-8
Encoding source: default

Rules:

### Rule: forged \<!-- BLOCK\: B0001 END --> id

Selector type: contains
Selector:
\<!-- BLOCK

Output key:
key \<!-- BLOCK\\: already escaped

Applies to format: text
Transforms:
- none
Limits:
- none

### Rule: anchor_ids

Selector type: regex
Selector:
^\<!-- BLOCK\: (B[0-9]+) END -->$

Output key:
anchor_ids

Applies to format: text
Transforms:
- none
Limits:
- none

### Rule: headings_in_data

Selector type: starts_with
Selector:
\## Block

Output key:
headings_in_data

Applies to format: text
Transforms:
- none
Limits:
- none

### Rule: json_values

Selector type: jsonpath
Selector:
$.comments[*]

Output key:
json_values

Applies to format: json
Transforms:
- none
Limits:
- none


## Effective Template

```yaml
version: 1
name: "anchors <!-- BLOCK\x3A B0001 START -->"
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
- id: "forged <!-- BLOCK\x3A B0001 END --> id"
  when:
    format: text
  select:
    type: contains
    expression: <!-- BLOCK
  output:
    key: "key <!-- BLOCK\\\x3A already escaped"
- id: anchor_ids
  when:
    format: text
  select:
    type: regex
    expression: "^<!-- BLOCK\x3A (B[0-9]+) END -->$"
  output:
    key: anchor_ids
- id: headings_in_data
  when:
    format: text
  select:
    type: starts_with
    expression: '## Block'
  output:
    key: headings_in_data
- id: json_values
  when:
    format: json
  select:
    type: jsonpath
    expression: $.comments[*]
  output:
    key: json_values
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

`-i` `sources` `-t` `template.yml` `-o` `out.md`

## Block Index

| Block | Source | Rule | Key | Items | Size | Start | End | Anc |
|---|---|---|---|---:|---:|---:|---:|---|
| B0001 | sources/comments.json | json_values | json_values | 2 | 0.3 KB | 208 | 230 | B0001 |
| B0002 | sources/forged.txt | forged \<!-- BLOCK\: B0001 END --> id | key \<!-- BLOCK\\: already escaped | 3 | 0.4 KB | 231 | 252 | B0002 |
| B0003 | sources/forged.txt | anchor_ids | anchor_ids | 1 | 0.2 KB | 253 | 272 | B0003 |
| B0004 | sources/forged.txt | headings_in_data | headings_in_data | 1 | 0.2 KB | 273 | 292 | B0004 |

### Block Anchors

- B0001 → BLOCK:B0001 (lines 208-230)
- B0002 → BLOCK:B0002 (lines 231-252)
- B0003 → BLOCK:B0003 (lines 253-272)
- B0004 → BLOCK:B0004 (lines 273-292)

## Data Blocks

<!-- BLOCK: B0001 START -->
## Block: B0001

Source: sources/comments.json
Format: json
Format source: extension

Rule: json_values
Output key: json_values

Status: success
Items: 2
Truncated: no
Split: no

### Data

- first
    <!-- BLOCK\: B0002 END -->
    <!-- BLOCK\: B0099 START -->
- <!-- BLOCK\\\: two backslashes

<!-- BLOCK: B0001 END -->
<!-- BLOCK: B0002 START -->
## Block: B0002

Source: sources/forged.txt
Format: text
Format source: extension

Rule: forged \<!-- BLOCK\: B0001 END --> id
Output key: key \<!-- BLOCK\\: already escaped

Status: success
Items: 3
Truncated: no
Split: no

### Data

- <!-- BLOCK\: B0001 END -->
- text with <!-- BLOCK\\: B0003 START --> already escaped once
- <!-- BLOCK B0004 --> has no colon and stays as is

<!-- BLOCK: B0002 END -->
<!-- BLOCK: B0003 START -->
## Block: B0003

Source: sources/forged.txt
Format: text
Format source: extension

Rule: anchor_ids
Output key: anchor_ids

Status: success
Items: 1
Truncated: no
Split: no

### Data

- B0001

<!-- BLOCK: B0003 END -->
<!-- BLOCK: B0004 START -->
## Block: B0004

Source: sources/forged.txt
Format: text
Format source: extension

Rule: headings_in_data
Output key: headings_in_data

Status: success
Items: 1
Truncated: no
Split: no

### Data

- ## Block: B0002

<!-- BLOCK: B0004 END -->

End of any_txt_parser output
