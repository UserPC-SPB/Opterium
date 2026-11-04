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
Files successful: 0
Files warnings: 2
Files failed: 0
Total extracted items: 15
Total output characters: 5980
Total output blocks: 6
Effective format: auto
Format source: auto detection
Encoding: utf-8
Run status: partial_success
Exit code: 1

## Files

| Path | Size | Modified | Format | Status | Encoding | Hash |
|---|---:|---|---|---|---|---|
| sources/a_server.log | 149 | <volatile> | text | warning | utf-8 | - |
| sources/b_worker.log | 83 | <volatile> | text | warning | utf-8 | - |

## Truncation

- sources/a_server.log [messages]: 1 item(s) truncated to max_item_chars: item 2 (27 -> 16 chars)
- sources/a_server.log [messages]: 1 truncated item(s) dropped as duplicates by stable_deduplicate
- sources/b_worker.log [messages]: 2 item(s) truncated to max_item_chars: item 1 (23 -> 16 chars), item 2 (17 -> 16 chars)
- sources/b_worker.log [messages]: 1 truncated item(s) dropped as duplicates by stable_deduplicate

## Applied Template

Template name: dedup_and_sort
Template version: 1
Template source: template.yml

Input format: auto
Format source: auto detection
Encoding: utf-8
Encoding source: default

Rules:

### Rule: levels

Selector type: regex
Selector:
^([A-Z]+) 

Output key:
levels

Transforms:
- none
Limits:
- none

### Rule: messages

Selector type: regex
Selector:
^[A-Z]+ (.+)$

Output key:
messages

Transforms:
- none
Limits:
- max_item_chars: 16

### Rule: users_desc

Selector type: regex
Selector:
user=([a-z]+)

Output key:
users_desc

Transforms:
- sort_desc
Limits:
- none


## Effective Template

```yaml
version: 1
name: dedup_and_sort
input:
  format: auto
  encoding: utf-8
output:
  block:
    target_chars: 28000
    max_chars: 32000
document:
  stable_deduplicate: true
  sort: asc
  split_blocks: true
rules:
- id: levels
  select:
    type: regex
    expression: '^([A-Z]+) '
  output:
    key: levels
- id: messages
  select:
    type: regex
    expression: ^[A-Z]+ (.+)$
  output:
    key: messages
  limits:
    max_item_chars: 16
- id: users_desc
  select:
    type: regex
    expression: user=([a-z]+)
  output:
    key: users_desc
  transforms:
  - sort_desc
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
| B0001 | sources/b_worker.log | levels | levels | 2 | 0.2 KB | 189 | 210 | B0001 |
| B0002 | sources/a_server.log | levels | levels | 3 | 0.2 KB | 211 | 233 | B0002 |
| B0003 | sources/b_worker.log | users_desc | users_desc | 2 | 0.2 KB | 234 | 255 | B0003 |
| B0004 | sources/a_server.log | messages | messages | 3 | 0.4 KB | 256 | 283 | B0004 |
| B0005 | sources/b_worker.log | messages | messages | 2 | 0.4 KB | 284 | 310 | B0005 |
| B0006 | sources/a_server.log | users_desc | users_desc | 3 | 0.2 KB | 311 | 333 | B0006 |

### Block Anchors

- B0001 → BLOCK:B0001 (lines 189-210)
- B0002 → BLOCK:B0002 (lines 211-233)
- B0003 → BLOCK:B0003 (lines 234-255)
- B0004 → BLOCK:B0004 (lines 256-283)
- B0005 → BLOCK:B0005 (lines 284-310)
- B0006 → BLOCK:B0006 (lines 311-333)

## Data Blocks

<!-- BLOCK: B0001 START -->
## Block: B0001

Source: sources/b_worker.log
Format: text
Format source: extension

Rule: levels
Output key: levels

Status: success
File status: warning
Items: 2
Truncated: no
Split: no

### Data

- ERROR
- INFO

<!-- BLOCK: B0001 END -->
<!-- BLOCK: B0002 START -->
## Block: B0002

Source: sources/a_server.log
Format: text
Format source: extension

Rule: levels
Output key: levels

Status: success
File status: warning
Items: 3
Truncated: no
Split: no

### Data

- INFO
- WARN
- ERROR

<!-- BLOCK: B0002 END -->
<!-- BLOCK: B0003 START -->
## Block: B0003

Source: sources/b_worker.log
Format: text
Format source: extension

Rule: users_desc
Output key: users_desc

Status: success
File status: warning
Items: 2
Truncated: no
Split: no

### Data

- bob
- amy

<!-- BLOCK: B0003 END -->
<!-- BLOCK: B0004 START -->
## Block: B0004

Source: sources/a_server.log
Format: text
Format source: extension

Rule: messages
Output key: messages

Status: success
File status: warning
Items: 3
Truncated: yes
Split: no

### Data

- login user=zoe
- slow response fo
- timeout user=mia

### Warnings

- 1 item(s) truncated to max_item_chars: item 2 (27 -> 16 chars)
- 1 truncated item(s) dropped as duplicates by stable_deduplicate

<!-- BLOCK: B0004 END -->
<!-- BLOCK: B0005 START -->
## Block: B0005

Source: sources/b_worker.log
Format: text
Format source: extension

Rule: messages
Output key: messages

Status: success
File status: warning
Items: 2
Truncated: yes
Split: no

### Data

- queue overflow u
- job done user=am

### Warnings

- 2 item(s) truncated to max_item_chars: item 1 (23 -> 16 chars), item 2 (17 -> 16 chars)
- 1 truncated item(s) dropped as duplicates by stable_deduplicate

<!-- BLOCK: B0005 END -->
<!-- BLOCK: B0006 START -->
## Block: B0006

Source: sources/a_server.log
Format: text
Format source: extension

Rule: users_desc
Output key: users_desc

Status: success
File status: warning
Items: 3
Truncated: no
Split: no

### Data

- zoe
- mia
- adam

<!-- BLOCK: B0006 END -->

End of any_txt_parser output
