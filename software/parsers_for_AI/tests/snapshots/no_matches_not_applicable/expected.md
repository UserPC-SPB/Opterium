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
Total extracted items: 2
Total output characters: 5635
Total output blocks: 6
Effective format: auto
Format source: auto detection
Encoding: utf-8
Run status: partial_success
Exit code: 1

## Files

| Path | Size | Modified | Format | Status | Encoding | Hash |
|---|---:|---|---|---|---|---|
| sources/notes.txt | 19 | <volatile> | text | warning | utf-8 | - |
| sources/users.json | 48 | <volatile> | json | warning | utf-8 | - |

## Applied Template

Template name: statuses
Template version: 1
Template source: template.yml

Input format: auto
Format source: auto detection
Encoding: utf-8
Encoding source: default

Rules:

### Rule: user_names

Selector type: jsonpath
Selector:
$.users[*].name

Output key:
user_names

Transforms:
- none
Limits:
- none

### Rule: missing_field

Selector type: jsonpath
Selector:
$.users[*].email

Output key:
missing_field

Transforms:
- none
Limits:
- none

### Rule: todo_lines

Selector type: contains
Selector:
TODO

Output key:
todo_lines

Transforms:
- none
Limits:
- none


## Effective Template

```yaml
version: 1
name: statuses
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
- id: user_names
  select:
    type: jsonpath
    expression: $.users[*].name
  output:
    key: user_names
- id: missing_field
  select:
    type: jsonpath
    expression: $.users[*].email
  output:
    key: missing_field
- id: todo_lines
  select:
    type: contains
    expression: TODO
  output:
    key: todo_lines
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
| B0001 | sources/notes.txt | user_names | user_names | 0 | 0.4 KB | 178 | 202 | B0001 |
| B0002 | sources/notes.txt | missing_field | missing_field | 0 | 0.4 KB | 203 | 227 | B0002 |
| B0003 | sources/notes.txt | todo_lines | todo_lines | 0 | 0.2 KB | 228 | 248 | B0003 |
| B0004 | sources/users.json | user_names | user_names | 2 | 0.2 KB | 249 | 270 | B0004 |
| B0005 | sources/users.json | missing_field | missing_field | 0 | 0.2 KB | 271 | 291 | B0005 |
| B0006 | sources/users.json | todo_lines | todo_lines | 0 | 0.3 KB | 292 | 316 | B0006 |

### Block Anchors

- B0001 → BLOCK:B0001 (lines 178-202)
- B0002 → BLOCK:B0002 (lines 203-227)
- B0003 → BLOCK:B0003 (lines 228-248)
- B0004 → BLOCK:B0004 (lines 249-270)
- B0005 → BLOCK:B0005 (lines 271-291)
- B0006 → BLOCK:B0006 (lines 292-316)

## Data Blocks

<!-- BLOCK: B0001 START -->
## Block: B0001

Source: sources/notes.txt
Format: text
Format source: extension

Rule: user_names
Output key: user_names

Status: rule not applicable
File status: warning
Items: 0
Truncated: no
Split: no

### Data

rule not applicable

### Warnings

- rule not applicable: selector type jsonpath does not apply to text files (they take regex, contains, starts_with, ends_with, line)

<!-- BLOCK: B0001 END -->
<!-- BLOCK: B0002 START -->
## Block: B0002

Source: sources/notes.txt
Format: text
Format source: extension

Rule: missing_field
Output key: missing_field

Status: rule not applicable
File status: warning
Items: 0
Truncated: no
Split: no

### Data

rule not applicable

### Warnings

- rule not applicable: selector type jsonpath does not apply to text files (they take regex, contains, starts_with, ends_with, line)

<!-- BLOCK: B0002 END -->
<!-- BLOCK: B0003 START -->
## Block: B0003

Source: sources/notes.txt
Format: text
Format source: extension

Rule: todo_lines
Output key: todo_lines

Status: no matches found
File status: warning
Items: 0
Truncated: no
Split: no

### Data

no matches found

<!-- BLOCK: B0003 END -->
<!-- BLOCK: B0004 START -->
## Block: B0004

Source: sources/users.json
Format: json
Format source: extension

Rule: user_names
Output key: user_names

Status: success
File status: warning
Items: 2
Truncated: no
Split: no

### Data

- alice
- bob

<!-- BLOCK: B0004 END -->
<!-- BLOCK: B0005 START -->
## Block: B0005

Source: sources/users.json
Format: json
Format source: extension

Rule: missing_field
Output key: missing_field

Status: no matches found
File status: warning
Items: 0
Truncated: no
Split: no

### Data

no matches found

<!-- BLOCK: B0005 END -->
<!-- BLOCK: B0006 START -->
## Block: B0006

Source: sources/users.json
Format: json
Format source: extension

Rule: todo_lines
Output key: todo_lines

Status: rule not applicable
File status: warning
Items: 0
Truncated: no
Split: no

### Data

rule not applicable

### Warnings

- rule not applicable: selector type contains does not apply to json files (they take jsonpath, path)

<!-- BLOCK: B0006 END -->

End of any_txt_parser output
