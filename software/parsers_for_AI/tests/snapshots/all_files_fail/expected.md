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
Files successful: 0
Files warnings: 0
Files failed: 3
Total extracted items: 0
Total output characters: 3051
Total output blocks: 0
Effective format: auto
Format source: auto detection
Encoding: utf-8
Run status: failure
Exit code: 3

## Files

| Path | Size | Modified | Format | Status | Encoding | Hash |
|---|---:|---|---|---|---|---|
| sources/latin1.json | 13 | <volatile> | json | error | utf-8 | - |
| sources/mismatched.xml | 27 | <volatile> | xml | error | utf-8 | - |
| sources/truncated.json | 12 | <volatile> | json | error | utf-8 | - |

## Diagnostics

- sources/latin1.json: invalid UTF-8 content: invalid utf-8 sequence of 1 bytes from index 3
- sources/mismatched.xml: invalid XML: expected 'user' tag, not 'users' at 1:19
- sources/truncated.json: invalid JSON: EOF while parsing a list at line 2 column 0

## Applied Template

Template name: users_everywhere
Template version: 1
Template source: template.yml

Input format: auto
Format source: auto detection
Encoding: utf-8
Encoding source: default

Rules:

### Rule: json_users

Selector type: jsonpath
Selector:
$.users[*]

Output key:
json_users

Applies to format: json
Transforms:
- none
Limits:
- none

### Rule: xml_users

Selector type: xpath
Selector:
//user/text()

Output key:
xml_users

Applies to format: xml
Transforms:
- none
Limits:
- none


## Effective Template

```yaml
version: 1
name: users_everywhere
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
- id: json_users
  when:
    format: json
  select:
    type: jsonpath
    expression: $.users[*]
  output:
    key: json_users
- id: xml_users
  when:
    format: xml
  select:
    type: xpath
    expression: //user/text()
  output:
    key: xml_users
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

### Block Anchors


## Data Blocks


End of any_txt_parser output
