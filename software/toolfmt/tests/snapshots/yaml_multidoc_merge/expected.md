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
Total extracted items: 14
Total output characters: 5635
Total output blocks: 6
Effective format: yaml
Format source: template
Encoding: utf-8
Run status: success
Exit code: 0

## Files

| Path | Size | Modified | Format | Status | Encoding | Hash |
|---|---:|---|---|---|---|---|
| sources/manifest.yaml | 667 | <volatile> | yaml | success | utf-8 | - |

## Applied Template

Template name: k8s_manifest
Template version: 1
Template source: template.yml

Input format: yaml
Format source: template
Encoding: utf-8
Encoding source: default

Rules:

### Rule: kinds

Selector type: path
Selector:
$.kind

Output key:
kinds

Transforms:
- none
Limits:
- none

### Rule: names

Selector type: path
Selector:
$.metadata.name

Output key:
names

Transforms:
- none
Limits:
- none

### Rule: tiers

Selector type: path
Selector:
$.metadata.labels.tier

Output key:
tiers

Transforms:
- none
Limits:
- none

### Rule: pod_labels

Selector type: path
Selector:
$.spec.template.metadata.labels

Output key:
pod_labels

Transforms:
- none
Limits:
- none

### Rule: config_values

Selector type: path
Selector:
$.data.*

Output key:
config_values

Transforms:
- none
Limits:
- none

### Rule: ports

Selector type: path
Selector:
spec.ports[*].port

Output key:
ports

Transforms:
- none
Limits:
- none


## Effective Template

```yaml
version: 1
name: k8s_manifest
input:
  format: yaml
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
- id: kinds
  select:
    type: path
    expression: $.kind
  output:
    key: kinds
- id: names
  select:
    type: path
    expression: $.metadata.name
  output:
    key: names
- id: tiers
  select:
    type: path
    expression: $.metadata.labels.tier
  output:
    key: tiers
- id: pod_labels
  select:
    type: path
    expression: $.spec.template.metadata.labels
  output:
    key: pod_labels
- id: config_values
  select:
    type: path
    expression: $.data.*
  output:
    key: config_values
- id: ports
  select:
    type: path
    expression: spec.ports[*].port
  output:
    key: ports
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
| B0001 | sources/manifest.yaml | kinds | kinds | 3 | 0.2 KB | 237 | 258 | B0001 |
| B0002 | sources/manifest.yaml | names | names | 3 | 0.2 KB | 259 | 280 | B0002 |
| B0003 | sources/manifest.yaml | tiers | tiers | 2 | 0.2 KB | 281 | 301 | B0003 |
| B0004 | sources/manifest.yaml | pod_labels | pod_labels | 1 | 0.2 KB | 302 | 321 | B0004 |
| B0005 | sources/manifest.yaml | config_values | config_values | 3 | 0.2 KB | 322 | 345 | B0005 |
| B0006 | sources/manifest.yaml | ports | ports | 2 | 0.2 KB | 346 | 366 | B0006 |

### Block Anchors

- B0001 → BLOCK:B0001 (lines 237-258)
- B0002 → BLOCK:B0002 (lines 259-280)
- B0003 → BLOCK:B0003 (lines 281-301)
- B0004 → BLOCK:B0004 (lines 302-321)
- B0005 → BLOCK:B0005 (lines 322-345)
- B0006 → BLOCK:B0006 (lines 346-366)

## Data Blocks

<!-- BLOCK: B0001 START -->
## Block: B0001

Source: sources/manifest.yaml
Format: yaml
Format source: template

Rule: kinds
Output key: kinds

Status: success
Items: 3
Truncated: no
Split: no

### Data

- Deployment
- Service
- ConfigMap

<!-- BLOCK: B0001 END -->
<!-- BLOCK: B0002 START -->
## Block: B0002

Source: sources/manifest.yaml
Format: yaml
Format source: template

Rule: names
Output key: names

Status: success
Items: 3
Truncated: no
Split: no

### Data

- web
- web-svc
- web-config

<!-- BLOCK: B0002 END -->
<!-- BLOCK: B0003 START -->
## Block: B0003

Source: sources/manifest.yaml
Format: yaml
Format source: template

Rule: tiers
Output key: tiers

Status: success
Items: 2
Truncated: no
Split: no

### Data

- frontend
- edge

<!-- BLOCK: B0003 END -->
<!-- BLOCK: B0004 START -->
## Block: B0004

Source: sources/manifest.yaml
Format: yaml
Format source: template

Rule: pod_labels
Output key: pod_labels

Status: success
Items: 1
Truncated: no
Split: no

### Data

- {"app":"web","tier":"frontend","version":"1.27"}

<!-- BLOCK: B0004 END -->
<!-- BLOCK: B0005 START -->
## Block: B0005

Source: sources/manifest.yaml
Format: yaml
Format source: template

Rule: config_values
Output key: config_values

Status: success
Items: 3
Truncated: no
Split: no

### Data

- Привет, мир
- Добро пожаловать
    Welcome
    
- true

<!-- BLOCK: B0005 END -->
<!-- BLOCK: B0006 START -->
## Block: B0006

Source: sources/manifest.yaml
Format: yaml
Format source: template

Rule: ports
Output key: ports

Status: success
Items: 2
Truncated: no
Split: no

### Data

- 80
- 443

<!-- BLOCK: B0006 END -->

End of any_txt_parser output
