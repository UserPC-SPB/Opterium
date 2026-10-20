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
sources/*.txt

Output:
out.md

Files discovered: 2
Files processed: 2
Files successful: 2
Files warnings: 0
Files failed: 0
Total extracted items: 2
Total output characters: 3107
Total output blocks: 2
Effective format: auto
Format source: auto detection
Encoding: utf-8
Run status: success
Exit code: 0

## Files

| Path | Size | Modified | Format | Status | Encoding | Hash |
|---|---:|---|---|---|---|---|
| sources/a_plain.txt | 31 | <volatile> | text | success | utf-8 | c138bab429925614e158ced61705536fbf03324ee2437f1c08e527f0a81dbb7a |
| sources/b_bom.txt | 27 | <volatile> | text | success | utf-8 (BOM stripped) | a00e3030b6f67d5ebb9d5ab178c13a087b17e23368f86a6254fc9cad66561319 |

## Applied Template

Template name: todo_hashes
Template version: 1
Template source: template.yml

Input format: auto
Format source: auto detection
Encoding: utf-8
Encoding source: default

Rules:

### Rule: todos

Selector type: starts_with
Selector:
TODO

Output key:
todos

Transforms:
- none
Limits:
- none


## Effective Template

```yaml
version: 1
name: todo_hashes
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
- id: todos
  select:
    type: starts_with
    expression: TODO
  output:
    key: todos
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

`-t` `template.yml` `-o` `out.md` `-i` `sources/*.txt` `--hash`

## Block Index

| Block | Source | Rule | Key | Items | Size | Start | End | Anc |
|---|---|---|---|---:|---:|---:|---:|---|
| B0001 | sources/a_plain.txt | todos | todos | 1 | 0.2 KB | 130 | 149 | B0001 |
| B0002 | sources/b_bom.txt | todos | todos | 1 | 0.2 KB | 150 | 169 | B0002 |

### Block Anchors

- B0001 → BLOCK:B0001 (lines 130-149)
- B0002 → BLOCK:B0002 (lines 150-169)

## Data Blocks

<!-- BLOCK: B0001 START -->
## Block: B0001

Source: sources/a_plain.txt
Format: text
Format source: extension

Rule: todos
Output key: todos

Status: success
Items: 1
Truncated: no
Split: no

### Data

- TODO: rotate keys

<!-- BLOCK: B0001 END -->
<!-- BLOCK: B0002 START -->
## Block: B0002

Source: sources/b_bom.txt
Format: text
Format source: extension

Rule: todos
Output key: todos

Status: success
Items: 1
Truncated: no
Split: no

### Data

- TODO: renew certificate

<!-- BLOCK: B0002 END -->

End of any_txt_parser output
