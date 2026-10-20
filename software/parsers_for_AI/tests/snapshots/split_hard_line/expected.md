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
Total extracted items: 1
Total output characters: 5835
Total output blocks: 5
Effective format: auto
Format source: auto detection
Encoding: utf-8
Run status: success
Exit code: 0

## Files

| Path | Size | Modified | Format | Status | Encoding | Hash |
|---|---:|---|---|---|---|---|
| sources/one_line.txt | 2710 | <volatile> | text | success | utf-8 | - |

## Applied Template

Template name: hard_split
Template version: 1
Template source: template.yml

Input format: auto
Format source: auto detection
Encoding: utf-8
Encoding source: default

Rules:

### Rule: whole_line

Selector type: line
Selector:
1-1

Output key:
whole_line

Transforms:
- none
Limits:
- none


## Effective Template

```yaml
version: 1
name: hard_split
input:
  format: auto
  encoding: utf-8
output:
  block:
    target_chars: 613
    max_chars: 700
document:
  stable_deduplicate: false
  sort: none
  split_blocks: true
rules:
- id: whole_line
  select:
    type: line
    from: 1
    to: 1
  output:
    key: whole_line
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

`-i` `sources` `-t` `template.yml` `-o` `out.md` `--max-block-chars` `700`

## Block Index

| Block | Source | Rule | Key | Items | Size | Start | End | Anc |
|---|---|---|---|---:|---:|---:|---:|---|
| B0001 | sources/one_line.txt | whole_line | whole_line | 1 | 0.3 KB | 136 | 156 | B0001 |
| B0001-01 | sources/one_line.txt | whole_line | whole_line | 1 | 0.6 KB | 157 | 179 | B0001-01 |
| B0001-02 | sources/one_line.txt | whole_line | whole_line | 1 | 0.6 KB | 180 | 202 | B0001-02 |
| B0001-03 | sources/one_line.txt | whole_line | whole_line | 1 | 0.6 KB | 203 | 225 | B0001-03 |
| B0001-04 | sources/one_line.txt | whole_line | whole_line | 1 | 0.4 KB | 226 | 248 | B0001-04 |

### Block Anchors

- B0001 → BLOCK:B0001 (lines 136-156) (parent; continuation parts: 4)
- B0001-01 → BLOCK:B0001-01 (lines 157-179) (part 1/4 of B0001)
- B0001-02 → BLOCK:B0001-02 (lines 180-202) (part 2/4 of B0001)
- B0001-03 → BLOCK:B0001-03 (lines 203-225) (part 3/4 of B0001)
- B0001-04 → BLOCK:B0001-04 (lines 226-248) (part 4/4 of B0001)

## Data Blocks

<!-- BLOCK: B0001 START -->
## Block: B0001

Parent block: continuation parts follow (4)
Source: sources/one_line.txt
Format: text
Format source: extension

Rule: whole_line
Output key: whole_line

Status: success
Items: 1
Truncated: no
Split: yes

### Data

(in the continuation parts)

<!-- BLOCK: B0001 END -->
<!-- BLOCK: B0001-01 START -->
## Block: B0001-01

Parent: B0001
Part: 1/4
Item continuation: 1
Source: sources/one_line.txt
Format: text
Format source: extension

Rule: whole_line
Output key: whole_line

Status: success
Items: 1
Truncated: no
Split: yes

### Data

- [01] Ёжик 👩‍💻 café 👨‍👩‍👧 🇷🇺 №1; [02] Ёжик 👩‍💻 café 👨‍👩‍👧 🇷🇺 №2; [03] Ёжик 👩‍💻 café 👨‍👩‍👧 🇷🇺 №3; [04] Ёжик 👩‍💻 café 👨‍👩‍👧 🇷🇺 №4; [05] Ёжик 👩‍💻 café 👨‍👩‍👧 🇷🇺 №5; [06] Ёжик 👩‍💻 café 👨‍👩‍👧 🇷🇺 №6; [07] Ёжик 👩‍💻 café 👨‍👩‍👧 🇷🇺 №7; [08] Ёжик 👩‍💻 café 👨‍👩‍👧 🇷🇺 №8; [09] Ёжик 👩‍💻 café 👨‍👩‍👧 🇷🇺 №9; [10] Ёжик 👩‍💻 café 👨‍👩‍👧 🇷🇺 №10; [11] Ёжик 👩‍💻 café 👨‍👩‍👧 🇷🇺 №11; [12] Ёжик 👩‍💻 café 👨‍👩‍👧 🇷🇺 №12; [1

<!-- BLOCK: B0001-01 END -->
<!-- BLOCK: B0001-02 START -->
## Block: B0001-02

Parent: B0001
Part: 2/4
Item continuation: 1
Source: sources/one_line.txt
Format: text
Format source: extension

Rule: whole_line
Output key: whole_line

Status: success
Items: 1
Truncated: no
Split: yes

### Data

- 3] Ёжик 👩‍💻 café 👨‍👩‍👧 🇷🇺 №13; [14] Ёжик 👩‍💻 café 👨‍👩‍👧 🇷🇺 №14; [15] Ёжик 👩‍💻 café 👨‍👩‍👧 🇷🇺 №15; [16] Ёжик 👩‍💻 café 👨‍👩‍👧 🇷🇺 №16; [17] Ёжик 👩‍💻 café 👨‍👩‍👧 🇷🇺 №17; [18] Ёжик 👩‍💻 café 👨‍👩‍👧 🇷🇺 №18; [19] Ёжик 👩‍💻 café 👨‍👩‍👧 🇷🇺 №19; [20] Ёжик 👩‍💻 café 👨‍👩‍👧 🇷🇺 №20; [21] Ёжик 👩‍💻 café 👨‍👩‍👧 🇷🇺 №21; [22] Ёжик 👩‍💻 café 👨‍👩‍👧 🇷🇺 №22; [23] Ёжик 👩‍💻 café 👨‍👩‍👧 🇷🇺 №23; [24] Ёжик 👩‍💻 café 👨‍👩‍👧 🇷🇺 

<!-- BLOCK: B0001-02 END -->
<!-- BLOCK: B0001-03 START -->
## Block: B0001-03

Parent: B0001
Part: 3/4
Item continuation: 1
Source: sources/one_line.txt
Format: text
Format source: extension

Rule: whole_line
Output key: whole_line

Status: success
Items: 1
Truncated: no
Split: yes

### Data

- №24; [25] Ёжик 👩‍💻 café 👨‍👩‍👧 🇷🇺 №25; [26] Ёжик 👩‍💻 café 👨‍👩‍👧 🇷🇺 №26; [27] Ёжик 👩‍💻 café 👨‍👩‍👧 🇷🇺 №27; [28] Ёжик 👩‍💻 café 👨‍👩‍👧 🇷🇺 №28; [29] Ёжик 👩‍💻 café 👨‍👩‍👧 🇷🇺 №29; [30] Ёжик 👩‍💻 café 👨‍👩‍👧 🇷🇺 №30; [31] Ёжик 👩‍💻 café 👨‍👩‍👧 🇷🇺 №31; [32] Ёжик 👩‍💻 café 👨‍👩‍👧 🇷🇺 №32; [33] Ёжик 👩‍💻 café 👨‍👩‍👧 🇷🇺 №33; [34] Ёжик 👩‍💻 café 👨‍👩‍👧 🇷🇺 №34; [35] Ёжик 👩‍💻 café 👨‍👩‍👧 🇷🇺 №35; [36] Ёжик 👩‍💻 café 👨‍

<!-- BLOCK: B0001-03 END -->
<!-- BLOCK: B0001-04 START -->
## Block: B0001-04

Parent: B0001
Part: 4/4
Item continuation: 1
Source: sources/one_line.txt
Format: text
Format source: extension

Rule: whole_line
Output key: whole_line

Status: success
Items: 1
Truncated: no
Split: yes

### Data

- 👩‍👧 🇷🇺 №36; [37] Ёжик 👩‍💻 café 👨‍👩‍👧 🇷🇺 №37; [38] Ёжик 👩‍💻 café 👨‍👩‍👧 🇷🇺 №38; [39] Ёжик 👩‍💻 café 👨‍👩‍👧 🇷🇺 №39; [40] Ёжик 👩‍💻 café 👨‍👩‍👧 🇷🇺 №40;

<!-- BLOCK: B0001-04 END -->

End of any_txt_parser output
