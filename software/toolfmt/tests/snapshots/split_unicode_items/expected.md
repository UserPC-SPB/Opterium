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
Total extracted items: 11
Total output characters: 6524
Total output blocks: 6
Effective format: auto
Format source: auto detection
Encoding: utf-8
Run status: success
Exit code: 0

## Files

| Path | Size | Modified | Format | Status | Encoding | Hash |
|---|---:|---|---|---|---|---|
| sources/release_notes.txt | 1182 | <volatile> | text | success | utf-8 | - |

## Applied Template

Template name: release_notes
Template version: 1
Template source: template.yml

Input format: auto
Format source: auto detection
Encoding: utf-8
Encoding source: default

Rules:

### Rule: whole_file

Selector type: line
Selector:
1-100

Output key:
whole_file

Transforms:
- none
Limits:
- none

### Rule: numbered

Selector type: regex
Selector:
^[0-9]+\. (.+)$

Output key:
numbered

Transforms:
- none
Limits:
- none


## Effective Template

```yaml
version: 1
name: release_notes
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
- id: whole_file
  select:
    type: line
    from: 1
    to: 100
  output:
    key: whole_file
- id: numbered
  select:
    type: regex
    expression: ^[0-9]+\. (.+)$
  output:
    key: numbered
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
| B0001 | sources/release_notes.txt | whole_file | whole_file | 1 | 0.3 KB | 158 | 178 | B0001 |
| B0001-01 | sources/release_notes.txt | whole_file | whole_file | 1 | 0.6 KB | 179 | 208 | B0001-01 |
| B0001-02 | sources/release_notes.txt | whole_file | whole_file | 1 | 0.6 KB | 209 | 236 | B0001-02 |
| B0002 | sources/release_notes.txt | numbered | numbered | 10 | 0.3 KB | 237 | 257 | B0002 |
| B0002-01 | sources/release_notes.txt | numbered | numbered | 6 | 0.5 KB | 258 | 284 | B0002-01 |
| B0002-02 | sources/release_notes.txt | numbered | numbered | 4 | 0.5 KB | 285 | 309 | B0002-02 |

### Block Anchors

- B0001 → BLOCK:B0001 (lines 158-178) (parent; continuation parts: 2)
- B0001-01 → BLOCK:B0001-01 (lines 179-208) (part 1/2 of B0001)
- B0001-02 → BLOCK:B0001-02 (lines 209-236) (part 2/2 of B0001)
- B0002 → BLOCK:B0002 (lines 237-257) (parent; continuation parts: 2)
- B0002-01 → BLOCK:B0002-01 (lines 258-284) (part 1/2 of B0002)
- B0002-02 → BLOCK:B0002-02 (lines 285-309) (part 2/2 of B0002)

## Data Blocks

<!-- BLOCK: B0001 START -->
## Block: B0001

Parent block: continuation parts follow (2)
Source: sources/release_notes.txt
Format: text
Format source: extension

Rule: whole_file
Output key: whole_file

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
Part: 1/2
Item continuation: 1
Source: sources/release_notes.txt
Format: text
Format source: extension

Rule: whole_file
Output key: whole_file

Status: success
Items: 1
Truncated: no
Split: yes

### Data

- Release notes 2.0 — заметки к выпуску 🦀
    ========================================
    1. Быстрый разбор журналов: ускорение в 3× на больших файлах.
    2. 新しいパーサー: поддержка японского текста 日本語 в отчётах.
    3. Исправлено: падение при пустом шаблоне (issue #42).
    4. Добавлено: флаг --hash для контрольных сумм SHA-256.
    5. Combining marks: café → café, ё → ё (два символа).
    

<!-- BLOCK: B0001-01 END -->
<!-- BLOCK: B0001-02 START -->
## Block: B0001-02

Parent: B0001
Part: 2/2
Item continuation: 1
Source: sources/release_notes.txt
Format: text
Format source: extension

Rule: whole_file
Output key: whole_file

Status: success
Items: 1
Truncated: no
Split: yes

### Data

- 6. Emoji: 🧪 тесты, 📦 пакеты, 🚀 релиз, 👩‍💻 разработчики.
    7. Улучшено: сообщения об ошибках теперь указывают строку и столбец.
    8. Документация: новый раздел «Как читать вывод» для ИИ-агентов.
    9. Удалено: устаревший параметр --legacy-mode.
    10. Безопасность: ограничение раскрытия сущностей DTD (8 МиБ).
    Благодарим всех участников! Спасибо 🙏 谢谢 ありがとう.

<!-- BLOCK: B0001-02 END -->
<!-- BLOCK: B0002 START -->
## Block: B0002

Parent block: continuation parts follow (2)
Source: sources/release_notes.txt
Format: text
Format source: extension

Rule: numbered
Output key: numbered

Status: success
Items: 10
Truncated: no
Split: yes

### Data

(in the continuation parts)

<!-- BLOCK: B0002 END -->
<!-- BLOCK: B0002-01 START -->
## Block: B0002-01

Parent: B0002
Part: 1/2
Source: sources/release_notes.txt
Format: text
Format source: extension

Rule: numbered
Output key: numbered

Status: success
Items: 6
Truncated: no
Split: yes

### Data

- Быстрый разбор журналов: ускорение в 3× на больших файлах.
- 新しいパーサー: поддержка японского текста 日本語 в отчётах.
- Исправлено: падение при пустом шаблоне (issue #42).
- Добавлено: флаг --hash для контрольных сумм SHA-256.
- Combining marks: café → café, ё → ё (два символа).
- Emoji: 🧪 тесты, 📦 пакеты, 🚀 релиз, 👩‍💻 разработчики.

<!-- BLOCK: B0002-01 END -->
<!-- BLOCK: B0002-02 START -->
## Block: B0002-02

Parent: B0002
Part: 2/2
Source: sources/release_notes.txt
Format: text
Format source: extension

Rule: numbered
Output key: numbered

Status: success
Items: 4
Truncated: no
Split: yes

### Data

- Улучшено: сообщения об ошибках теперь указывают строку и столбец.
- Документация: новый раздел «Как читать вывод» для ИИ-агентов.
- Удалено: устаревший параметр --legacy-mode.
- Безопасность: ограничение раскрытия сущностей DTD (8 МиБ).

<!-- BLOCK: B0002-02 END -->

End of any_txt_parser output
