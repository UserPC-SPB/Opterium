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

Files discovered: 9
Files processed: 9
Files successful: 9
Files warnings: 0
Files failed: 0
Total extracted items: 11
Total output characters: 8005
Total output blocks: 9
Effective format: auto
Format source: auto detection
Encoding: utf-8
Run status: success
Exit code: 0

## Files

| Path | Size | Modified | Format | Status | Encoding | Hash |
|---|---:|---|---|---|---|---|
| sources/Cargo.toml | 46 | <volatile> | toml | success | utf-8 | - |
| sources/build.log | 26 | <volatile> | text | success | utf-8 | - |
| sources/config/app.yaml | 37 | <volatile> | yaml | success | utf-8 | - |
| sources/config/service.conf | 21 | <volatile> | yaml | success | utf-8 | - |
| sources/docs/guide.md | 27 | <volatile> | markdown | success | utf-8 | - |
| sources/docs/index.html | 89 | <volatile> | html | success | utf-8 | - |
| sources/manifest | 25 | <volatile> | json | success | utf-8 | - |
| sources/modules.xml | 64 | <volatile> | xml | success | utf-8 | - |
| sources/package.json | 38 | <volatile> | json | success | utf-8 | - |

## Applied Template

Template name: mixed_project
Template version: 1
Template source: template.yml

Input format: auto
Format source: auto detection
Encoding: utf-8
Encoding source: default

Rules:

### Rule: json_name

Selector type: jsonpath
Selector:
$.name

Output key:
json_name

Applies to format: json
Transforms:
- none
Limits:
- none

### Rule: yaml_port

Selector type: path
Selector:
$.server.port

Output key:
yaml_port

Applies to format: yaml
Transforms:
- none
Limits:
- none

### Rule: toml_version

Selector type: path
Selector:
[package].version

Output key:
toml_version

Applies to format: toml
Transforms:
- none
Limits:
- none

### Rule: xml_ids

Selector type: xpath
Selector:
//module/@id

Output key:
xml_ids

Applies to format: xml
Transforms:
- none
Limits:
- none

### Rule: html_title

Selector type: css
Selector:
title

Output key:
html_title

Applies to format: html
Transforms:
- none
Limits:
- none

### Rule: md_headings

Selector type: markdown
Selector:
heading

Output key:
md_headings

Applies to format: markdown
Transforms:
- none
Limits:
- none

### Rule: log_errors

Selector type: contains
Selector:
ERROR

Output key:
log_errors

Applies to format: text
Transforms:
- none
Limits:
- none


## Effective Template

```yaml
version: 1
name: mixed_project
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
- id: json_name
  when:
    format: json
  select:
    type: jsonpath
    expression: $.name
  output:
    key: json_name
- id: yaml_port
  when:
    format: yaml
  select:
    type: path
    expression: $.server.port
  output:
    key: yaml_port
- id: toml_version
  when:
    format: toml
  select:
    type: path
    expression: '[package].version'
  output:
    key: toml_version
- id: xml_ids
  when:
    format: xml
  select:
    type: xpath
    expression: //module/@id
  output:
    key: xml_ids
- id: html_title
  when:
    format: html
  select:
    type: css
    expression: title
  output:
    key: html_title
- id: md_headings
  when:
    format: markdown
  select:
    type: markdown
    node: heading
  output:
    key: md_headings
- id: log_errors
  when:
    format: text
  select:
    type: contains
    expression: ERROR
  output:
    key: log_errors
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
| B0001 | sources/Cargo.toml | toml_version | toml_version | 1 | 0.2 KB | 294 | 313 | B0001 |
| B0002 | sources/build.log | log_errors | log_errors | 1 | 0.2 KB | 314 | 333 | B0002 |
| B0003 | sources/config/app.yaml | yaml_port | yaml_port | 1 | 0.2 KB | 334 | 353 | B0003 |
| B0004 | sources/config/service.conf | yaml_port | yaml_port | 1 | 0.2 KB | 354 | 373 | B0004 |
| B0005 | sources/docs/guide.md | md_headings | md_headings | 2 | 0.2 KB | 374 | 394 | B0005 |
| B0006 | sources/docs/index.html | html_title | html_title | 1 | 0.2 KB | 395 | 414 | B0006 |
| B0007 | sources/manifest | json_name | json_name | 1 | 0.2 KB | 415 | 434 | B0007 |
| B0008 | sources/modules.xml | xml_ids | xml_ids | 2 | 0.2 KB | 435 | 455 | B0008 |
| B0009 | sources/package.json | json_name | json_name | 1 | 0.2 KB | 456 | 475 | B0009 |

### Block Anchors

- B0001 → BLOCK:B0001 (lines 294-313)
- B0002 → BLOCK:B0002 (lines 314-333)
- B0003 → BLOCK:B0003 (lines 334-353)
- B0004 → BLOCK:B0004 (lines 354-373)
- B0005 → BLOCK:B0005 (lines 374-394)
- B0006 → BLOCK:B0006 (lines 395-414)
- B0007 → BLOCK:B0007 (lines 415-434)
- B0008 → BLOCK:B0008 (lines 435-455)
- B0009 → BLOCK:B0009 (lines 456-475)

## Data Blocks

<!-- BLOCK: B0001 START -->
## Block: B0001

Source: sources/Cargo.toml
Format: toml
Format source: extension

Rule: toml_version
Output key: toml_version

Status: success
Items: 1
Truncated: no
Split: no

### Data

- 0.3.1

<!-- BLOCK: B0001 END -->
<!-- BLOCK: B0002 START -->
## Block: B0002

Source: sources/build.log
Format: text
Format source: extension

Rule: log_errors
Output key: log_errors

Status: success
Items: 1
Truncated: no
Split: no

### Data

- ERROR disk full

<!-- BLOCK: B0002 END -->
<!-- BLOCK: B0003 START -->
## Block: B0003

Source: sources/config/app.yaml
Format: yaml
Format source: extension

Rule: yaml_port
Output key: yaml_port

Status: success
Items: 1
Truncated: no
Split: no

### Data

- 8080

<!-- BLOCK: B0003 END -->
<!-- BLOCK: B0004 START -->
## Block: B0004

Source: sources/config/service.conf
Format: yaml
Format source: content

Rule: yaml_port
Output key: yaml_port

Status: success
Items: 1
Truncated: no
Split: no

### Data

- 9090

<!-- BLOCK: B0004 END -->
<!-- BLOCK: B0005 START -->
## Block: B0005

Source: sources/docs/guide.md
Format: markdown
Format source: extension

Rule: md_headings
Output key: md_headings

Status: success
Items: 2
Truncated: no
Split: no

### Data

- Guide
- Usage

<!-- BLOCK: B0005 END -->
<!-- BLOCK: B0006 START -->
## Block: B0006

Source: sources/docs/index.html
Format: html
Format source: extension

Rule: html_title
Output key: html_title

Status: success
Items: 1
Truncated: no
Split: no

### Data

- Dashboard

<!-- BLOCK: B0006 END -->
<!-- BLOCK: B0007 START -->
## Block: B0007

Source: sources/manifest
Format: json
Format source: content

Rule: json_name
Output key: json_name

Status: success
Items: 1
Truncated: no
Split: no

### Data

- sniffed-json

<!-- BLOCK: B0007 END -->
<!-- BLOCK: B0008 START -->
## Block: B0008

Source: sources/modules.xml
Format: xml
Format source: extension

Rule: xml_ids
Output key: xml_ids

Status: success
Items: 2
Truncated: no
Split: no

### Data

- core
- cli

<!-- BLOCK: B0008 END -->
<!-- BLOCK: B0009 START -->
## Block: B0009

Source: sources/package.json
Format: json
Format source: extension

Rule: json_name
Output key: json_name

Status: success
Items: 1
Truncated: no
Split: no

### Data

- any_tool

<!-- BLOCK: B0009 END -->

End of any_txt_parser output
