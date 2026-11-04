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
Files successful: 0
Files warnings: 1
Files failed: 0
Total extracted items: 13
Total output characters: 6198
Total output blocks: 4
Effective format: json
Format source: template
Encoding: utf-8
Run status: partial_success
Exit code: 1

## Files

| Path | Size | Modified | Format | Status | Encoding | Hash |
|---|---:|---|---|---|---|---|
| sources/tickets.json | 1254 | <volatile> | json | warning | utf-8 | - |

## Truncation

- sources/tickets.json [subjects]: 2 item(s) truncated to max_item_chars: item 1 (47 -> 24 chars), item 3 (45 -> 24 chars)
- sources/tickets.json [subjects]: 1 item(s) omitted after max_items limit
- sources/tickets.json [short_bodies]: 2 item(s) truncated by truncate_lines(1): item 1 (3 -> 1 lines), item 3 (2 -> 1 lines)
- sources/tickets.json [short_bodies]: 3 item(s) truncated by truncate_chars(30): item 3 (75 -> 30 chars); 2 not published
- sources/tickets.json [short_bodies]: 2 item(s) omitted by limit_items(3)
- sources/tickets.json [tags_joined]: 8 item(s) truncated by truncate_chars(4): item 1 (7 joined items cut to 4 chars); 1 not published

## Applied Template

Template name: support_tickets
Template version: 1
Template source: template.yml

Input format: json
Format source: template
Encoding: utf-8
Encoding source: default

Rules:

### Rule: subjects

Selector type: jsonpath
Selector:
$.tickets[*].subject

Output key:
subjects

Transforms:
- none
Limits:
- max_items: 4
- max_item_chars: 24

### Rule: short_bodies

Selector type: jsonpath
Selector:
$.tickets[*].body

Output key:
short_bodies

Transforms:
- truncate_lines(1)
- truncate_chars(30)
- limit_items(3)
Limits:
- none

### Rule: tags_joined

Selector type: jsonpath
Selector:
$.tickets[*].tags[*]

Output key:
tags_joined

Transforms:
- truncate_chars(4)
- unique
- join(, )
Limits:
- none

### Rule: untouched

Selector type: jsonpath
Selector:
$.tickets[*].id

Output key:
untouched

Transforms:
- none
Limits:
- none


## Effective Template

```yaml
version: 1
name: support_tickets
input:
  format: json
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
- id: subjects
  select:
    type: jsonpath
    expression: $.tickets[*].subject
  output:
    key: subjects
  limits:
    max_items: 4
    max_item_chars: 24
- id: short_bodies
  select:
    type: jsonpath
    expression: $.tickets[*].body
  output:
    key: short_bodies
  transforms:
  - truncate_lines: 1
  - truncate_chars: 30
  - limit_items: 3
- id: tags_joined
  select:
    type: jsonpath
    expression: $.tickets[*].tags[*]
  output:
    key: tags_joined
  transforms:
  - truncate_chars: 4
  - unique
  - join: ', '
- id: untouched
  select:
    type: jsonpath
    expression: $.tickets[*].id
  output:
    key: untouched
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
| B0001 | sources/tickets.json | subjects | subjects | 4 | 0.4 KB | 218 | 246 | B0001 |
| B0002 | sources/tickets.json | short_bodies | short_bodies | 3 | 0.5 KB | 247 | 275 | B0002 |
| B0003 | sources/tickets.json | tags_joined | tags_joined | 1 | 0.4 KB | 276 | 300 | B0003 |
| B0004 | sources/tickets.json | untouched | untouched | 5 | 0.2 KB | 301 | 325 | B0004 |

### Block Anchors

- B0001 → BLOCK:B0001 (lines 218-246)
- B0002 → BLOCK:B0002 (lines 247-275)
- B0003 → BLOCK:B0003 (lines 276-300)
- B0004 → BLOCK:B0004 (lines 301-325)

## Data Blocks

<!-- BLOCK: B0001 START -->
## Block: B0001

Source: sources/tickets.json
Format: json
Format source: template

Rule: subjects
Output key: subjects

Status: success
File status: warning
Items: 4
Truncated: yes
Split: no

### Data

- Не приходит письмо с под
- Refund
- Приложение падает при за
- Вопрос по тарифу

### Warnings

- 2 item(s) truncated to max_item_chars: item 1 (47 -> 24 chars), item 3 (45 -> 24 chars)
- 1 item(s) omitted after max_items limit

<!-- BLOCK: B0001 END -->
<!-- BLOCK: B0002 START -->
## Block: B0002

Source: sources/tickets.json
Format: json
Format source: template

Rule: short_bodies
Output key: short_bodies

Status: success
File status: warning
Items: 3
Truncated: yes
Split: no

### Data

- Здравствуйте!
- Please refund order A-1001.
- После обновления до версии 3.2

### Warnings

- 2 item(s) truncated by truncate_lines(1): item 1 (3 -> 1 lines), item 3 (2 -> 1 lines)
- 3 item(s) truncated by truncate_chars(30): item 3 (75 -> 30 chars); 2 not published
- 2 item(s) omitted by limit_items(3)

<!-- BLOCK: B0002 END -->
<!-- BLOCK: B0003 START -->
## Block: B0003

Source: sources/tickets.json
Format: json
Format source: template

Rule: tags_joined
Output key: tags_joined

Status: success
File status: warning
Items: 1
Truncated: yes
Split: no

### Data

- emai, regi, bill, andr, cras, mobi, pric, misc

### Warnings

- 8 item(s) truncated by truncate_chars(4): item 1 (7 joined items cut to 4 chars); 1 not published

<!-- BLOCK: B0003 END -->
<!-- BLOCK: B0004 START -->
## Block: B0004

Source: sources/tickets.json
Format: json
Format source: template

Rule: untouched
Output key: untouched

Status: success
File status: warning
Items: 5
Truncated: no
Split: no

### Data

- 101
- 102
- 103
- 104
- 105

<!-- BLOCK: B0004 END -->

End of any_txt_parser output
