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
Files successful: 3
Files warnings: 0
Files failed: 0
Total extracted items: 18
Total output characters: 10246
Total output blocks: 15
Effective format: auto
Format source: auto detection
Encoding: utf-8
Run status: success
Exit code: 0

## Files

| Path | Size | Modified | Format | Status | Encoding | Hash |
|---|---:|---|---|---|---|---|
| sources/app.log | 427 | <volatile> | text | success | utf-8 | - |
| sources/classic_mac.log | 147 | <volatile> | text | success | utf-8 | - |
| sources/windows.log | 155 | <volatile> | text | success | utf-8 | - |

## Applied Template

Template name: service_logs
Template version: 1
Template source: template.yml

Input format: auto
Format source: auto detection
Encoding: utf-8
Encoding source: default

Rules:

### Rule: errors_with_context

Selector type: contains
Selector:
ERROR

Output key:
errors_with_context

Context: before=1, after=1
Transforms:
- none
Limits:
- none

### Rule: request_ids

Selector type: regex
Selector:
request_id=([0-9a-f]+)

Output key:
request_ids

Transforms:
- unique
Limits:
- none

### Rule: warnings

Selector type: starts_with
Selector:
WARN

Output key:
warnings

Transforms:
- none
Limits:
- none

### Rule: timings

Selector type: ends_with
Selector:
 ms

Output key:
timings

Transforms:
- none
Limits:
- none

### Rule: head

Selector type: line
Selector:
1-2

Output key:
head

Transforms:
- none
Limits:
- none


## Effective Template

```yaml
version: 1
name: service_logs
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
- id: errors_with_context
  select:
    type: contains
    expression: ERROR
  output:
    key: errors_with_context
  context:
    before: 1
    after: 1
- id: request_ids
  select:
    type: regex
    expression: request_id=([0-9a-f]+)
  output:
    key: request_ids
  transforms:
  - unique
- id: warnings
  select:
    type: starts_with
    expression: WARN
  output:
    key: warnings
- id: timings
  select:
    type: ends_with
    expression: ' ms'
  output:
    key: timings
- id: head
  select:
    type: line
    from: 1
    to: 2
  output:
    key: head
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
| B0001 | sources/app.log | errors_with_context | errors_with_context | 2 | 0.6 KB | 244 | 268 | B0001 |
| B0002 | sources/app.log | request_ids | request_ids | 2 | 0.2 KB | 269 | 289 | B0002 |
| B0003 | sources/app.log | warnings | warnings | 1 | 0.2 KB | 290 | 309 | B0003 |
| B0004 | sources/app.log | timings | timings | 2 | 0.3 KB | 310 | 330 | B0004 |
| B0005 | sources/app.log | head | head | 1 | 0.3 KB | 331 | 351 | B0005 |
| B0006 | sources/classic_mac.log | errors_with_context | errors_with_context | 1 | 0.4 KB | 352 | 373 | B0006 |
| B0007 | sources/classic_mac.log | request_ids | request_ids | 1 | 0.2 KB | 374 | 393 | B0007 |
| B0008 | sources/classic_mac.log | warnings | warnings | 1 | 0.2 KB | 394 | 413 | B0008 |
| B0009 | sources/classic_mac.log | timings | timings | 1 | 0.2 KB | 414 | 433 | B0009 |
| B0010 | sources/classic_mac.log | head | head | 1 | 0.3 KB | 434 | 454 | B0010 |
| B0011 | sources/windows.log | errors_with_context | errors_with_context | 1 | 0.4 KB | 455 | 476 | B0011 |
| B0012 | sources/windows.log | request_ids | request_ids | 1 | 0.2 KB | 477 | 496 | B0012 |
| B0013 | sources/windows.log | warnings | warnings | 1 | 0.2 KB | 497 | 516 | B0013 |
| B0014 | sources/windows.log | timings | timings | 1 | 0.2 KB | 517 | 536 | B0014 |
| B0015 | sources/windows.log | head | head | 1 | 0.3 KB | 537 | 557 | B0015 |

### Block Anchors

- B0001 → BLOCK:B0001 (lines 244-268)
- B0002 → BLOCK:B0002 (lines 269-289)
- B0003 → BLOCK:B0003 (lines 290-309)
- B0004 → BLOCK:B0004 (lines 310-330)
- B0005 → BLOCK:B0005 (lines 331-351)
- B0006 → BLOCK:B0006 (lines 352-373)
- B0007 → BLOCK:B0007 (lines 374-393)
- B0008 → BLOCK:B0008 (lines 394-413)
- B0009 → BLOCK:B0009 (lines 414-433)
- B0010 → BLOCK:B0010 (lines 434-454)
- B0011 → BLOCK:B0011 (lines 455-476)
- B0012 → BLOCK:B0012 (lines 477-496)
- B0013 → BLOCK:B0013 (lines 497-516)
- B0014 → BLOCK:B0014 (lines 517-536)
- B0015 → BLOCK:B0015 (lines 537-557)

## Data Blocks

<!-- BLOCK: B0001 START -->
## Block: B0001

Source: sources/app.log
Format: text
Format source: extension

Rule: errors_with_context
Output key: errors_with_context

Status: success
Items: 2
Truncated: no
Split: no

### Data

- WARN  2026-10-18T09:00:05Z slow query request_id=a1f3 took 950 ms
    ERROR 2026-10-18T09:00:06Z payment failed request_id=b7c9: card declined
    INFO  2026-10-18T09:00:06Z retry scheduled request_id=b7c9
- INFO  2026-10-18T09:00:06Z retry scheduled request_id=b7c9
    ERROR 2026-10-18T09:00:09Z сбой подключения к БД: timeout
    INFO  2026-10-18T09:00:10Z shutdown requested

<!-- BLOCK: B0001 END -->
<!-- BLOCK: B0002 START -->
## Block: B0002

Source: sources/app.log
Format: text
Format source: extension

Rule: request_ids
Output key: request_ids

Status: success
Items: 2
Truncated: no
Split: no

### Data

- a1f3
- b7c9

<!-- BLOCK: B0002 END -->
<!-- BLOCK: B0003 START -->
## Block: B0003

Source: sources/app.log
Format: text
Format source: extension

Rule: warnings
Output key: warnings

Status: success
Items: 1
Truncated: no
Split: no

### Data

- WARN  2026-10-18T09:00:05Z slow query request_id=a1f3 took 950 ms

<!-- BLOCK: B0003 END -->
<!-- BLOCK: B0004 START -->
## Block: B0004

Source: sources/app.log
Format: text
Format source: extension

Rule: timings
Output key: timings

Status: success
Items: 2
Truncated: no
Split: no

### Data

- DEBUG 2026-10-18T09:00:01Z cache warmed in 120 ms
- WARN  2026-10-18T09:00:05Z slow query request_id=a1f3 took 950 ms

<!-- BLOCK: B0004 END -->
<!-- BLOCK: B0005 START -->
## Block: B0005

Source: sources/app.log
Format: text
Format source: extension

Rule: head
Output key: head

Status: success
Items: 1
Truncated: no
Split: no

### Data

- INFO  2026-10-18T09:00:00Z service started version=1.4.2
    DEBUG 2026-10-18T09:00:01Z cache warmed in 120 ms

<!-- BLOCK: B0005 END -->
<!-- BLOCK: B0006 START -->
## Block: B0006

Source: sources/classic_mac.log
Format: text
Format source: extension

Rule: errors_with_context
Output key: errors_with_context

Status: success
Items: 1
Truncated: no
Split: no

### Data

- WARN  2026-10-18T11:00:00Z legacy mode
    ERROR 2026-10-18T11:00:01Z printer offline request_id=dead
    INFO  2026-10-18T11:00:02Z queue flushed in 5 ms

<!-- BLOCK: B0006 END -->
<!-- BLOCK: B0007 START -->
## Block: B0007

Source: sources/classic_mac.log
Format: text
Format source: extension

Rule: request_ids
Output key: request_ids

Status: success
Items: 1
Truncated: no
Split: no

### Data

- dead

<!-- BLOCK: B0007 END -->
<!-- BLOCK: B0008 START -->
## Block: B0008

Source: sources/classic_mac.log
Format: text
Format source: extension

Rule: warnings
Output key: warnings

Status: success
Items: 1
Truncated: no
Split: no

### Data

- WARN  2026-10-18T11:00:00Z legacy mode

<!-- BLOCK: B0008 END -->
<!-- BLOCK: B0009 START -->
## Block: B0009

Source: sources/classic_mac.log
Format: text
Format source: extension

Rule: timings
Output key: timings

Status: success
Items: 1
Truncated: no
Split: no

### Data

- INFO  2026-10-18T11:00:02Z queue flushed in 5 ms

<!-- BLOCK: B0009 END -->
<!-- BLOCK: B0010 START -->
## Block: B0010

Source: sources/classic_mac.log
Format: text
Format source: extension

Rule: head
Output key: head

Status: success
Items: 1
Truncated: no
Split: no

### Data

- WARN  2026-10-18T11:00:00Z legacy mode
    ERROR 2026-10-18T11:00:01Z printer offline request_id=dead

<!-- BLOCK: B0010 END -->
<!-- BLOCK: B0011 START -->
## Block: B0011

Source: sources/windows.log
Format: text
Format source: extension

Rule: errors_with_context
Output key: errors_with_context

Status: success
Items: 1
Truncated: no
Split: no

### Data

- WARN  2026-10-18T10:00:00Z disk usage 91%
    ERROR 2026-10-18T10:00:02Z backup failed request_id=c0ffee
    INFO  2026-10-18T10:00:03Z backup retried in 30 ms

<!-- BLOCK: B0011 END -->
<!-- BLOCK: B0012 START -->
## Block: B0012

Source: sources/windows.log
Format: text
Format source: extension

Rule: request_ids
Output key: request_ids

Status: success
Items: 1
Truncated: no
Split: no

### Data

- c0ffee

<!-- BLOCK: B0012 END -->
<!-- BLOCK: B0013 START -->
## Block: B0013

Source: sources/windows.log
Format: text
Format source: extension

Rule: warnings
Output key: warnings

Status: success
Items: 1
Truncated: no
Split: no

### Data

- WARN  2026-10-18T10:00:00Z disk usage 91%

<!-- BLOCK: B0013 END -->
<!-- BLOCK: B0014 START -->
## Block: B0014

Source: sources/windows.log
Format: text
Format source: extension

Rule: timings
Output key: timings

Status: success
Items: 1
Truncated: no
Split: no

### Data

- INFO  2026-10-18T10:00:03Z backup retried in 30 ms

<!-- BLOCK: B0014 END -->
<!-- BLOCK: B0015 START -->
## Block: B0015

Source: sources/windows.log
Format: text
Format source: extension

Rule: head
Output key: head

Status: success
Items: 1
Truncated: no
Split: no

### Data

- WARN  2026-10-18T10:00:00Z disk usage 91%
    ERROR 2026-10-18T10:00:02Z backup failed request_id=c0ffee

<!-- BLOCK: B0015 END -->

End of any_txt_parser output
