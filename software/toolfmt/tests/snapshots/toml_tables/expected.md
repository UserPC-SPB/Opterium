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
Files successful: 1
Files warnings: 0
Files failed: 0
Total extracted items: 10
Total output characters: 6814
Total output blocks: 8
Effective format: auto
Format source: auto detection
Encoding: utf-8
Run status: success
Exit code: 0

## Files

| Path | Size | Modified | Format | Status | Encoding | Hash |
|---|---:|---|---|---|---|---|
| sources/service.toml | 404 | <volatile> | toml | success | utf-8 | - |

## Applied Template

Template name: service_config
Template version: 1
Template source: template.yml

Input format: auto
Format source: auto detection
Encoding: utf-8
Encoding source: default

Rules:

### Rule: title

Selector type: path
Selector:
title

Output key:
title

Transforms:
- none
Limits:
- none

### Rule: db_host

Selector type: path
Selector:
[database].host

Output key:
db_host

Transforms:
- none
Limits:
- none

### Rule: db_pool

Selector type: path
Selector:
[database].pool

Output key:
db_pool

Transforms:
- none
Limits:
- none

### Rule: alpha_ip

Selector type: path
Selector:
[servers.alpha].ip

Output key:
alpha_ip

Transforms:
- none
Limits:
- none

### Rule: server_roles

Selector type: path
Selector:
$.servers.*.role

Output key:
server_roles

Transforms:
- none
Limits:
- none

### Rule: budget_2024

Selector type: path
Selector:
["2024"].budget

Output key:
budget_2024

Transforms:
- none
Limits:
- none

### Rule: product_names

Selector type: path
Selector:
products[*].name

Output key:
product_names

Transforms:
- none
Limits:
- none

### Rule: released

Selector type: path
Selector:
released

Output key:
released

Transforms:
- none
Limits:
- none


## Effective Template

```yaml
version: 1
name: service_config
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
- id: title
  select:
    type: path
    expression: title
  output:
    key: title
- id: db_host
  select:
    type: path
    expression: '[database].host'
  output:
    key: db_host
- id: db_pool
  select:
    type: path
    expression: '[database].pool'
  output:
    key: db_pool
- id: alpha_ip
  select:
    type: path
    expression: '[servers.alpha].ip'
  output:
    key: alpha_ip
- id: server_roles
  select:
    type: path
    expression: $.servers.*.role
  output:
    key: server_roles
- id: budget_2024
  select:
    type: path
    expression: '["2024"].budget'
  output:
    key: budget_2024
- id: product_names
  select:
    type: path
    expression: products[*].name
  output:
    key: product_names
- id: released
  select:
    type: path
    expression: released
  output:
    key: released
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
| B0001 | sources/service.toml | title | title | 1 | 0.2 KB | 281 | 300 | B0001 |
| B0002 | sources/service.toml | db_host | db_host | 1 | 0.2 KB | 301 | 320 | B0002 |
| B0003 | sources/service.toml | db_pool | db_pool | 1 | 0.2 KB | 321 | 340 | B0003 |
| B0004 | sources/service.toml | alpha_ip | alpha_ip | 1 | 0.2 KB | 341 | 360 | B0004 |
| B0005 | sources/service.toml | server_roles | server_roles | 2 | 0.2 KB | 361 | 381 | B0005 |
| B0006 | sources/service.toml | budget_2024 | budget_2024 | 1 | 0.2 KB | 382 | 401 | B0006 |
| B0007 | sources/service.toml | product_names | product_names | 2 | 0.2 KB | 402 | 422 | B0007 |
| B0008 | sources/service.toml | released | released | 1 | 0.2 KB | 423 | 442 | B0008 |

### Block Anchors

- B0001 → BLOCK:B0001 (lines 281-300)
- B0002 → BLOCK:B0002 (lines 301-320)
- B0003 → BLOCK:B0003 (lines 321-340)
- B0004 → BLOCK:B0004 (lines 341-360)
- B0005 → BLOCK:B0005 (lines 361-381)
- B0006 → BLOCK:B0006 (lines 382-401)
- B0007 → BLOCK:B0007 (lines 402-422)
- B0008 → BLOCK:B0008 (lines 423-442)

## Data Blocks

<!-- BLOCK: B0001 START -->
## Block: B0001

Source: sources/service.toml
Format: toml
Format source: extension

Rule: title
Output key: title

Status: success
Items: 1
Truncated: no
Split: no

### Data

- Сервис заказов

<!-- BLOCK: B0001 END -->
<!-- BLOCK: B0002 START -->
## Block: B0002

Source: sources/service.toml
Format: toml
Format source: extension

Rule: db_host
Output key: db_host

Status: success
Items: 1
Truncated: no
Split: no

### Data

- db.internal

<!-- BLOCK: B0002 END -->
<!-- BLOCK: B0003 START -->
## Block: B0003

Source: sources/service.toml
Format: toml
Format source: extension

Rule: db_pool
Output key: db_pool

Status: success
Items: 1
Truncated: no
Split: no

### Data

- {"min":2,"max":16}

<!-- BLOCK: B0003 END -->
<!-- BLOCK: B0004 START -->
## Block: B0004

Source: sources/service.toml
Format: toml
Format source: extension

Rule: alpha_ip
Output key: alpha_ip

Status: success
Items: 1
Truncated: no
Split: no

### Data

- 10.0.0.1

<!-- BLOCK: B0004 END -->
<!-- BLOCK: B0005 START -->
## Block: B0005

Source: sources/service.toml
Format: toml
Format source: extension

Rule: server_roles
Output key: server_roles

Status: success
Items: 2
Truncated: no
Split: no

### Data

- replica
- primary

<!-- BLOCK: B0005 END -->
<!-- BLOCK: B0006 START -->
## Block: B0006

Source: sources/service.toml
Format: toml
Format source: extension

Rule: budget_2024
Output key: budget_2024

Status: success
Items: 1
Truncated: no
Split: no

### Data

- 1500000.0

<!-- BLOCK: B0006 END -->
<!-- BLOCK: B0007 START -->
## Block: B0007

Source: sources/service.toml
Format: toml
Format source: extension

Rule: product_names
Output key: product_names

Status: success
Items: 2
Truncated: no
Split: no

### Data

- Молоток
- Гвоздь

<!-- BLOCK: B0007 END -->
<!-- BLOCK: B0008 START -->
## Block: B0008

Source: sources/service.toml
Format: toml
Format source: extension

Rule: released
Output key: released

Status: success
Items: 1
Truncated: no
Split: no

### Data

- 2026-10-01T12:00:00Z

<!-- BLOCK: B0008 END -->

End of any_txt_parser output
