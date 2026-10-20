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

Files discovered: 3
Files processed: 3
Files successful: 2
Files warnings: 0
Files failed: 1
Total extracted items: 2
Total output characters: 3239
Total output blocks: 2
Effective format: auto
Format source: auto detection
Encoding: utf-8
Run status: partial_success
Exit code: 1

## Files

| Path | Size | Modified | Format | Status | Encoding | Hash |
|---|---:|---|---|---|---|---|
| sources/a_users.json | 81 | <volatile> | json | success | utf-8 | - |
| sources/b_broken.json | 82 | <volatile> | json | error | utf-8 | - |
| sources/c_users.json | 46 | <volatile> | json | success | utf-8 | - |

## Diagnostics

- sources/b_broken.json: invalid JSON: expected `,` or `}` at line 2 column 19

## Applied Template

Template name: active_users
Template version: 1
Template source: template.yml

Input format: auto
Format source: auto detection
Encoding: utf-8
Encoding source: default

Rules:

### Rule: active_names

Selector type: jsonpath
Selector:
$.users[?(@.active == true)].name

Output key:
active_names

Transforms:
- none
Limits:
- none


## Effective Template

```yaml
version: 1
name: active_users
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
- id: active_names
  select:
    type: jsonpath
    expression: $.users[?(@.active == true)].name
  output:
    key: active_names
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
| B0001 | sources/a_users.json | active_names | active_names | 1 | 0.2 KB | 135 | 154 | B0001 |
| B0002 | sources/c_users.json | active_names | active_names | 1 | 0.2 KB | 155 | 174 | B0002 |

### Block Anchors

- B0001 → BLOCK:B0001 (lines 135-154)
- B0002 → BLOCK:B0002 (lines 155-174)

## Data Blocks

<!-- BLOCK: B0001 START -->
## Block: B0001

Source: sources/a_users.json
Format: json
Format source: extension

Rule: active_names
Output key: active_names

Status: success
Items: 1
Truncated: no
Split: no

### Data

- alice

<!-- BLOCK: B0001 END -->
<!-- BLOCK: B0002 START -->
## Block: B0002

Source: sources/c_users.json
Format: json
Format source: extension

Rule: active_names
Output key: active_names

Status: success
Items: 1
Truncated: no
Split: no

### Data

- erin

<!-- BLOCK: B0002 END -->

End of any_txt_parser output
