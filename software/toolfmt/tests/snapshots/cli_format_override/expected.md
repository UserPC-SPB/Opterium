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
Total extracted items: 2
Total output characters: 3576
Total output blocks: 2
Effective format: text
Format source: CLI override
Encoding: utf-8
Run status: success
Exit code: 0

## Files

| Path | Size | Modified | Format | Status | Encoding | Hash |
|---|---:|---|---|---|---|---|
| sources/alert.yaml | 41 | <volatile> | text | success | utf-8 | - |
| sources/event.json | 43 | <volatile> | text | success | utf-8 | - |

## Applied Template

Template name: grep_everything
Template version: 1
Template source: template.yml

Input format: text
Format source: CLI override
Encoding: utf-8
Encoding source: default

Rules:

### Rule: json_level

Selector type: jsonpath
Selector:
$.level

Output key:
json_level

Applies to format: json (never applies in this run: every file is processed as text, format source: CLI override)
Transforms:
- none
Limits:
- none

### Rule: error_lines

Selector type: contains
Selector:
ERROR

Output key:
error_lines

Transforms:
- none
Limits:
- none


## Effective Template

```yaml
version: 1
name: grep_everything
input:
  format: text
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
- id: json_level
  when:
    format: json
  select:
    type: jsonpath
    expression: $.level
  output:
    key: json_level
- id: error_lines
  select:
    type: contains
    expression: ERROR
  output:
    key: error_lines
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

`-i` `sources` `-t` `template.yml` `-o` `out.md` `--format` `text`

## Block Index

| Block | Source | Rule | Key | Items | Size | Start | End | Anc |
|---|---|---|---|---:|---:|---:|---:|---|
| B0001 | sources/alert.yaml | error_lines | error_lines | 1 | 0.2 KB | 155 | 174 | B0001 |
| B0002 | sources/event.json | error_lines | error_lines | 1 | 0.2 KB | 175 | 194 | B0002 |

### Block Anchors

- B0001 → BLOCK:B0001 (lines 155-174)
- B0002 → BLOCK:B0002 (lines 175-194)

## Data Blocks

<!-- BLOCK: B0001 START -->
## Block: B0001

Source: sources/alert.yaml
Format: text
Format source: CLI override

Rule: error_lines
Output key: error_lines

Status: success
Items: 1
Truncated: no
Split: no

### Data

- message: ERROR budget at 80%

<!-- BLOCK: B0001 END -->
<!-- BLOCK: B0002 START -->
## Block: B0002

Source: sources/event.json
Format: text
Format source: CLI override

Rule: error_lines
Output key: error_lines

Status: success
Items: 1
Truncated: no
Split: no

### Data

- {"level": "ERROR", "message": "disk full"}

<!-- BLOCK: B0002 END -->

End of any_txt_parser output
