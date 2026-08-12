# any_txt_parser — полное руководство (HELP)

Версия документа: 1.0.0 · Соответствует версии программы 1.0.0 (crate `any_txt_parser`, edition 2024, Rust)

---

## 1. Что это

`any_txt_parser` — детерминированный CLI-экстрактор текста: из произвольных файлов
(JSON, YAML, TOML, XML, HTML, Markdown, plain text) по YAML-шаблону извлекает данные
и публикует их в **самоописываемый** Markdown-файл `log_any_txt.md`.

Программа создана для передачи данных «машине» (ИИ-ассистенту): вывод самодостаточен —
содержит исходный шаблон, структуру блоков, анкоры, диаграммы статусов, индексы строк.

Пайплайн из двух фаз:

1. **Фаза 1 — добыча**: discovery (поиск файлов) → decode (декодирование) → collect
   (извлечение по правилам) — для каждого файла.
2. **Фаза 2 — материализация**: materialize (группировка в блоки, split, dedup, сортировка,
   лимиты) → render (сборка документа) → validate (внутренняя проверка) → publish
   (атомарная запись: временный файл + перенос).

Ключевые свойства:

- **Детерминированность**: один и тот же ввод даёт одинаковый вывод (кроме поля `Generation`
  и временных меток).
- **Self-describing**: вывод включает `## Effective Template` (YAML), `## Applied Template`,
  `## Result Semantics`, `## CLI Arguments`.
- **Строгие контракты**: фиксированный заголовок `# any_txt_parser output`, маркер
  `Triangle: [0,1,0]`, конец документа `End of any_txt_parser output`.
- **Атомарная публикация**: файл вывода не портится при сбое; при ошибке запуска
  существующий вывод не трогается, новый не создаётся.
- **Ресурсные лимиты** на всех этапах, строгие exit-коды.

---

## 2. Сборка и запуск

```bash
cargo build --release          # бинарь: target/release/any_txt_parser.exe
cargo test                     # юнит-тесты (27) + интеграционные (9)
cargo fmt --all                # форматирование
cargo clippy --all-targets     # проверка стиля (только предупреждения)

any_txt_parser --input <PATH> --template <FILE> --output <FILE> [опции]
```

Минимальный запуск (все три обязательные опции):

```bash
any_txt_parser -i data.json -t tpl.yml -o log_any_txt.md
```

---

## 3. CLI-аргументы

| Опция | Обязательная | Значение | По умолчанию |
|---|---|---|---|
| `-i, --input PATH` | да | Файл, каталог или glob-паттерн (см. §6) | — |
| `-t, --template FILE` | да | Путь к YAML-шаблону (см. §5) | — |
| `-o, --output FILE` | да | Файл вывода; `log_any_txt.md` по договорённости | — |
| `--format FMT` | нет | Принудительный формат: `json\|xml\|html\|markdown\|yaml\|toml\|text` (не `auto`) | из шаблона, иначе `auto` |
| `--encoding ENC` | нет | `utf-8`, `utf-8-sig`, `utf-16le`, `utf-16` (=LE), `utf-16be`, `windows-1251`/`cp1251` | из шаблона, иначе `utf-8` |
| `--dry-run` | нет | Только проверить шаблон/файлы, печатает план, ничего не публикует | выкл |
| `-v, --verbose` | нет | Диагностика в stderr (версии, файлы, форматы, счётчики) | выкл |
| `-q, --quiet` | нет | Подавить итоговое сообщение в stderr | выкл |
| `--follow-symlinks` | нет | Обходить симлинки при discovery | выкл (симлинки пропускаются) |
| `--hash` | нет | Вычислять sha256 каждого исходного файла, выводить в `## Files` | выкл |
| `--max-file-size SIZE` | нет | Максимальный размер входного файла | 200 MiB |
| `--max-memory SIZE` | нет | Бюджет декодированного текста (байты) | безлимит |
| `--max-files N` | нет | Максимум найденных файлов | 5000 |
| `--max-matches N` | нет | Лимит совпадений на правило (ресурсный, действует до трансформаций) | безлимит |
| `--max-output-size SIZE` | нет | Максимальный итоговый документ (characters) | из шаблона |
| `--max-block-chars N` | нет | Переопределяет `output.block.max_chars` (target = 7/8 от него) | 32000 |

`SIZE` принимает суффиксы: `b`/`B`, `k`/`K`/`KiB`, `m`/`M`/`MiB`, `g`/`G`/`GiB` (в степенях 1024), либо голое число.

Приоритет параметров: **CLI → шаблон → дефолт**. CLI-переопределения фиксируются
в выводе (`Format source: CLI override`, `Encoding source: CLI override`,
в `## Effective Template` + `## CLI Arguments`).

---

## 4. Выходные коды (exit codes)

| Код | Значение | Когда |
|---|---|---|
| `0` | Success | Все файлы успешны, без ограничений |
| `1` | PartialSuccess | Есть файлы со статусом `warning` (no matches, not applicable, truncation) или `error` |
| `2` | InvalidTemplate | Шаблон не читается/не валиден (см. §5) |
| `3` | InvalidInputOrCli | Нет входных файлов, неверный паттерн, нет `--output`, нет шаблона на диске |
| `4` | OutputFailure | Внутренняя валидация документа или ошибка публикации |
| `5` | InternalFatal | Внутренняя ошибка |
| `6` | ResourceLimit | Превышен лимит ресурсов (файлы, размер, блоки, память) |

`--dry-run` всегда возвращает `0` (или `2`/`3` при ошибках до dry-run).

---

## 5. Шаблон (Template DSL, version: 1)

Файл YAML. Полная схема:

```yaml
version: 1                     # обязательно; поддерживается только 1
name: имя_шаблона              # обязательно, непустое
input:                         # необязательно
  format: auto                 # json|xml|html|markdown|yaml|toml|text|auto
  encoding: utf-8              # см. --encoding
output:                        # необязательно
  max_output_chars: 100000     # лимит символов всего документа
  block:                       # разбиение на блоки (см. §8)
    target_chars: 28000        # целевой размер блока (0 < target <= max)
    max_chars: 32000           # жёсткий максимум блока; большие item'ы сплитятся
document:                      # необязательно
  sort: none                   # none|asc|desc — сортировка блоков документа
  stable_deduplicate: false    # true: убрать дубликаты item'ов (сохраняется первый)
  limit_total: 100000          # = output.max_output_chars (устаревший синоним)
  split_blocks: true           # true: разбивать блоки по target/max (по умолчанию true)
rules:                         # обязательно, >= 1 правило
  - id: r1                     # обязательно, уникальные id
    when:                      # необязательно
      format: json             # применять правило только к этому формату
    select:                    # обязательно, см. §7
      type: jsonpath
      expression: "$.users[*].name"
    context:                   # необязательно (только text-адаптер)
      before: 1                # строк контекста ДО совпадения
      after: 1                 # строк контекста ПОСЛЕ совпадения
    output:                    # необязательно
      key: имена               # ключ в выводе (по умолчанию = id правила)
    extract: text              # text|attr:NAME|inner_html|outer_html (§7.4)
    transforms:                # по-элементно, порядок важен (§7.5)
      - trim
      - replace: {from: "a", to: "b"}
    limits:                    # необязательно
      max_items: 10            # максимум item'ов после трансформаций (лишние отбрасываются, засчитываются)
      max_item_chars: 500      # обрезка каждого item'а по символам
```

Ошибки шаблона (код 2): неизвестные поля (`deny_unknown_fields` на всех секциях),
дубликаты `id`, `target_chars > max_chars`, нулевые размеры блоков, пустые списки правил,
неверная версия, некорректные селекторы/регулярки/трансформации.

---

## 6. Discovery — как ищутся входные файлы

`--input` может быть:

- **файлом** — берётся он один;
- **каталогом** — рекурсивный обход (глубина до 64), файлы сортируются по пути;
- **glob-паттерном** — `*.json`, `data/*.txt`;

  - с `**` — рекурсивный обход (`src/**/*.rs`), сопоставление относительно базы;
  - без `**` — через `glob` (без учёта регистра, точечные файлы допускаются).

Исключения из результатов:

- выходной файл (`--output`) исключается всегда (по canonical path);
- симлинки пропускаются, если не передан `--follow-symlinks`;
- каталоги игнорируются (только файлы).

Лимиты после discovery (код 6): `max_files` (по умолчанию 5000), `max_file_size`
(по умолчанию 200 MiB) — проверяются по каждому файлу.

Если найдено 0 файлов — код `3`.

**Автоопределение формата** (`format: auto`), по порядку:

1. расширение файла (`.json`, `.xml`, `.html/.htm`, `.md/.markdown`, `.yaml/.yml`,
   `.toml`, `.txt/.log` = text);
2. снайфинг содержимого: `{`/`[` + валидный JSON → json; `<` + `<!doctype html`/`<html` → html;
   `<` + валидный XML → xml; `key = value` (toml-парсится) → toml; `key: value` или `---` → yaml;
3. fallback: text.

Кодировки: utf-8 (BOM автоматически снимается и фиксируется в `Encoding`), utf-8-sig,
utf-16le/utf-16 (BOM учтён), utf-16be, windows-1251/cp1251.

---

## 7. Селекторы и адаптеры

Формат файла определяет **адаптер**, который задаёт, какие селекторы применимы.
Правило, чей тип селектора неприменим к формату файла → статус `rule not applicable`,
не считается ошибкой. `when.format` позволяет ограничить правило форматом.

| Формат (адаптер) | Доступные селекторы |
|---|---|
| json | `jsonpath`, `path` |
| yaml | `path` |
| toml | `path` |
| xml | `xpath` |
| html | `css` |
| markdown | `markdown` |
| text | `regex`, `contains`, `starts_with`, `ends_with`, `line` |

### 7.1 jsonpath (JSON)

Синтаксис:

- `$` — корень (необязательно); шаги: `.name`, `..name` (рекурсивный поиск),
  `.*`, `..*`, `[N]`, `[N,M,...]`, `['имя']`/`["имя"]`, `[*]`;
- фильтр: `[?(<выражение>)]`; сравнения `== != < <= > >=` над путями, строками,
  числами, булевыми, null; связки `&&`, `||`; `@` = текущий узел, голый `@` = сам узел.

Примеры:

```
$.users[*].name            # имена всех пользователей
$..title                   # все поля title на любой глубине
$.orders[0].id             # id первого заказа
$.items[0,2,4]             # элементы массива по индексам
$.store.book[?(@.price < 10)]       # книги дешевле 10
$.users[?(@.active == true && @.role == 'admin')]
```

### 7.2 path (yaml, toml, json)

Упрощённый путь без JSP-логики: `$`/`.` разделяют ключи, `[N]` — индекс массива,
`['k']`/`["k"]` — ключ с пробелами/спецсимволами, `[*]` — все элементы массива.

```
$.settings.timeout        # эквивалентно settings.timeout
$['log level']            # ключ с пробелом
$.servers[0].host         # хост первого сервера
$.list[*].value           # value у всех элементов list
```

### 7.3 xpath (XML)

Подмножество XPath:

- `/root/a/b` — абсолютный путь, `//a` — поиск от корня; `/a//b` — потомки где угодно;
- шаги: `имя`, `*` (любой элемент), `имя[N]` (1-индексный или 0), атрибут-предикаты
  `имя[@attr]`, `имя[@attr="val"]`, текстовый предикат `имя[text="val"]`;
- окончание: элемент (текст узла), `@attr` — атрибут, `text()` — текст узла.

Примеры:

```
/root/catalog/item/@id          # атрибуты id всех item
/root/item[@type="book"]/title/text()
/root/item[price > 100]/name    # сравнения в предикатах поддерживаются
//item[1]                       # первый item где угодно
```

### 7.4 css и markdown (html / markdown)

**css** — селекторы через `scraper` (стандартный CSS): `p`, `a[href]`, `div.card > h2`,
`table tr td`, `#main .item:first-child` и т.п.

**markdown** — аргументы `node` (+ необязательный `pattern` regex по содержимому текста узла):

```yaml
select: {type: markdown, node: heading}      # все заголовки
select: {type: markdown, node: code_block, pattern: "^const"}   # блоки кода, начинающиеся с const
select: {type: markdown, node: list_item}    # элементы списков
```

Значения `node`: `heading`/`h`, `paragraph`/`p`, `list`, `list_item`/`item`/`li`,
`table`, `table_row`/`row`/`tr`, `table_cell`/`cell`/`td`, `code_block`/`code`,
`blockquote`/`quote`, `link`, `image`/`img` (подчёркивания и дефисы в именах игнорируются).

### 7.5 text-селекторы

Работают построчно (строки — массив по `\n`):

| type | Семантика |
|---|---|
| `regex` | Строки с совпадением. Одна группа захвата `(…)` → в item попадает группа, иначе вся строка |
| `contains` | Строка содержит подстроку `expression` |
| `starts_with` | Строка начинается с `expression` |
| `ends_with` | Строка заканчивается на `expression` |
| `line` | Диапазон: `{type: line, from: 1, to: 5}` — 1-индексный, включая `to`; результат — один item из склеенных строк |

Для `regex/contains/starts_with/ends_with` работает `context.before/after`:
в item добавляются соседние строки (с меткой контекста) — используется для логирования.

Пример:

```yaml
select: {type: regex, expression: "^ERROR (.+)$"}     # одна группа -> текст ошибки
select: {type: regex, expression: "ERROR"}            # без группы -> вся строка
select: {type: contains, expression: "TODO"}
select: {type: line, from: 10, to: 20}
```

### 7.6 extract — режим извлечения

| Режим | Где применяется | Что извлекает |
|---|---|---|
| `text` (по умолчанию) | все | Текстовое содержимое |
| `attr:NAME` | xml (`@attr`), html (css-элементы) | Значение атрибута |
| `inner_html` | html | `inner_html()` элемента |
| `outer_html` | html | `html()` элемента (пустые не выпадают) |

### 7.7 transforms — постобработка (порядок важен)

**Item-трансформации** (применяются к каждому совпадению до других ограничений):

| Имя | Параметр | Действие |
|---|---|---|
| `trim` | — | Обрезать пробелы |
| `normalize_whitespace` | — | Схлопнуть пробельные символы в один пробел, обрезать края |
| `lowercase` / `uppercase` | — | Регистр |
| `unescape` | — | `\n \t \r \" \' \\ \uXXXX` в реальные символы |
| `truncate_chars` | `int` | Оставить N символов |
| `truncate_lines` | `int` | Оставить N строк |
| `replace` | `{from, to}` | Замена подстрок (все вхождения) |
| `regex_replace` | `{from, to}` | regex-замена (`from` — паттерн) |

**Result-трансформации** (применяются к списку item'ов правила):

| Имя | Параметр | Действие |
|---|---|---|
| `drop_empty` | — | Удалить пустые (trim) item'ы |
| `unique` | — | Убрать дубликаты (первое вхождение) |
| `sort` | — | Сортировка по возрастанию |
| `sort_desc` | — | По убыванию |
| `limit_items` | `int` | Оставить N item'ов |
| `join` | `string` | Склеить в один item через разделитель |

### 7.8 limits правила

- `max_items` — после всех трансформаций; лишние отбрасываются, счётчик попадает в
  warning «N item(s) omitted after max_items limit».
- `max_item_chars` — обрезка по символам ДО result-трансформаций; для каждого обрезанного
  item'а выдаётся warning «item truncated N -> M chars», плюс итог «N item(s) truncated».

### 7.9 Статусы правила и файла

Статус правила: `success` (есть item'ы) · `no matches found` (ничего не найдено) ·
`rule not applicable` (селектор неприменим к формату) · `error`.

Статус файла: `success`; `warning` (любое правило no-match/not-applicable или truncation);
`error` (ошибка парсинга файла или ошибка правила). Файл со статусом `error` даёт
записи в `## Diagnostics`, блоков не производит.

---

## 8. Материализация и блоки

После сбора item'ов по всем файлам/правилам:

1. **union по ключу**: item'ы всех правил с одинаковым `output.key` объединяются
   (в порядке файлов, затем правил) в одну группу — одну «оболочку» (shell);
2. **`document.sort`**: `asc`/`desc` сортирует блоки по тексту первого item'а;
3. **`document.stable_deduplicate: true`**: внутри каждого блока убираются дубликаты
   item'ов (строковое равенство, сохраняется первое вхождение) — применяется ко всему
   документу с тем же принципом для пересекающихся ключей;
4. **разбиение по размеру** (`split_blocks: true`, по умолчанию): блок растёт до
   `target_chars`; если очередной item выходит за `max_chars` (с учётом заголовка блока
   ≈220 симв.) — блок «сплитится» на части:
   - родитель-блок с маркером `Parent block: continuation parts follow (N)`;
   - части `Part: 1/N … N/N` с суффиксом ID `B0001-01`, `B0001-02`, …;
   - один item, не влезающий даже в `max_chars` (за вычетом заголовка), режется
     посимвольно до максимально допустимого размера.
5. `output.max_output_chars` — глобальный лимит: при достижении дальнейшие блоки
   отбрасываются с записью в `## Truncation`;

Про id блоков: первый блок `B0001`, дальше по возрастанию; части получают
`BXXXX-NN`. Анкоры:

```
<!-- BLOCK: B0001 START -->
<!-- BLOCK: B0001 END -->
```

Блок включает:

```
## Block: B0001
Parent: B0001            # только у частей
Part: 1/3                # только у частей
Parent block: continuation parts follow (3)   # только у родителя-сплита
Source: <путь файла>
Format: json
Format source: extension
Rule: r1
Output key: имена
Status: success          # статус ПРАВИЛА (rule-level)
File status: warning     # только если файл не success
Items: N
Truncated: yes|no
Split: yes|no

### Data

- item 1
- item 2 (многострочный — продолжения с отступом 4 пробела)
```

Поведение при пустых данных: `no matches found` / `rule not applicable` / `(error)` / `(no data)`.

Секция `### Warnings` — предупреждения правила (omitted/truncated); `### Errors` — ошибки.

---

## 9. Выходной документ (структура)

```
# any_txt_parser output
Triangle: [0,1,0]                 # контракт «готово»
Generation: 00000001              # инкремент от предыдущего запуска (8 цифр)
Updated: <ISO-8601 UTC>

Parser version: 1.0.0

## Execution                        # метаданные запуска
## Files                            # | Path | Size | Modified | Format | Status | Hash |
## Diagnostics                      # только при ошибках файлов
## Truncation                       # только при глобальных резах/отбрасываниях
## Applied Template                 # человекочитаемое резюме шаблона
## Effective Template               # YAML-блок с фактическими настройками (после CLI-оверрайдов)
## Result Semantics                 # договорённости по чтению документа
## CLI Arguments                    # сырые аргументы командной строки
## Block Index                      # | Block | Source | Rule | Key | Items | Size | Start | End | Anc |
### Block Anchors                   # список блоков с диапазонами строк
## Data Blocks
<!-- BLOCK: B0001 START -->
## Block: B0001
...
<!-- BLOCK: B0001 END -->
End of any_txt_parser output       # обязательный финальный маркер
```

`start_line`/`end_line` блоков пересчитываются после рендера (с учётом prelude и индекса),
каждый блок обязан иметь уникальные пары анкоров START/END (внутренняя валидация, код 4).

Как читать блок: `Source` + `Rule` + `Output key` идентифицируют происхождение;
`Status` — полнота; диапазон `lines X–Y` из `Block Index` — где лежит блок в документе.

---

## 10. Примеры

### 10.1 Простой текстовый файл

`input.txt`:
```
version 3.2.1
NOTE: first release
ERROR: something broke
DONE
```

`tpl.yml`:
```yaml
version: 1
name: text_sample
input: {format: text}
rules:
  - id: errors
    select: {type: starts_with, expression: "ERROR"}
    output: {key: errors}
    transforms: [trim, normalize_whitespace]
  - id: notes
    select: {type: contains, expression: "NOTE"}
    output: {key: notes}
```

### 10.2 JSON с фильтром и трансформациями

`data.json`:
```json
{"users": [
  {"name": "Alice", "active": true,  "tags": ["a","b"]},
  {"name": "Bob",   "active": false, "tags": ["c"]}
]}
```

`tpl.yml`:
```yaml
version: 1
name: users
input: {format: json}
output:
  block: {target_chars: 2000, max_chars: 4000}
rules:
  - id: active_names
    when: {format: json}
    select: {type: jsonpath, expression: "$.users[?(@.active == true)].name"}
    output: {key: names}
  - id: tags
    select: {type: jsonpath, expression: "$.users[*].tags[*]"}
    output: {key: tags}
    transforms:
      - uppercase                  # item-трансформация (по каждому совпадению)
      - trim                       # item-трансформация
      - unique                     # result-трансформация (по списку совпадений)
      - sort                       # result-трансформация
```

Трансформации задаются одним списком — item- и result-трансформации
распознаются автоматически по имени (см. §7.7) и применяются в порядке следования.

### 10.3 YAML через path

```yaml
version: 1
name: yaml_cfg
input: {format: yaml}
rules:
  - id: timeout
    select: {type: path, expression: "$.settings.timeout"}
    output: {key: timeout}
  - id: hosts
    select: {type: path, expression: "$.servers[*].host"}
    output: {key: hosts}
```

### 10.4 TOML

```yaml
version: 1
name: toml_cfg
input: {format: toml}
rules:
  - id: database_host
    select: {type: path, expression: "[database].host"}
    output: {key: host}
  - id: ports
    select: {type: path, expression: "[database].ports[*]"}
    output: {key: ports}
```

### 10.5 XML с атрибутами

```yaml
version: 1
name: xml_catalog
input: {format: xml}
rules:
  - id: ids
    select: {type: xpath, expression: "/catalog/item/@id"}
    output: {key: ids}
  - id: book_titles
    select: {type: xpath, expression: "/catalog/item[@type='book']/title/text()"}
    output: {key: titles}
  - id: first_item
    select: {type: xpath, expression: "/catalog/item[1]"}
    output: {key: first}
```

### 10.6 html через css

```yaml
version: 1
name: html_page
input: {format: html}
rules:
  - id: titles
    select: {type: css, expression: "h2"}
    output: {key: titles}
  - id: links
    select: {type: css, expression: "a[href]"}
    output: {key: links}
    extract: attr:href
  - id: products
    select: {type: css, expression: "div.product"}
    output: {key: products}
    extract: outer_html
    limits: {max_items: 5}
```

### 10.7 markdown

```yaml
version: 1
name: md_doc
input: {format: markdown}
rules:
  - id: headings
    select: {type: markdown, node: heading}
    output: {key: headings}
  - id: codes
    select: {type: markdown, node: code_block, pattern: "^#include"}
    output: {key: code}
  - id: items
    select: {type: markdown, node: list_item}
    output: {key: items}
```

### 10.8 Пакетный запуск по каталогу с лимитами и hash

```bash
any_txt_parser \
  --input ./logs/**/*.log \
  --template tpl.yml \
  --output log_any_txt.md \
  --hash \
  --max-files 1000 \
  --max-file-size 50MB \
  --max-matches 2000
```

### 10.9 Dry-run — проверка без публикации

```bash
any_txt_parser -i data.json -t tpl.yml -o log_any_txt.md --dry-run
# печатает: Template: valid, effective format/encoding, список файлов,
#           лимиты блоков, «Dry run: no output published.»
```

---

## 11. Примечания для потребителя вывода (AI-контракт)

- Читайте `## Effective Template` — это фактические настройки (включая CLI-оверрайды).
- Статусы: `success` — данные полны; `warning` на уровне `File status` — правило не
  применилось/ничего не нашло/обрезано, вывод может быть неполон; `error` — данные
  отсутствуют.
- `Generation` растёт при каждом успешном запуске — можно использовать как маркер
  «новые данные».
- Для повторного машинного чтения надёжнее всего искать блоки по анкорам
  `<!-- BLOCK: B0001 START --> … END -->` и `## Block Index` (линии).
- Многострочные item'ы: продолжение отступается 4 пробелами.
- Итоговый документ всегда оканчивается строкой `End of any_txt_parser output`.

---

## 12. Тесты и разработка

```bash
cargo test              # 27 юнит + 9 интеграционных (tests/golden.rs)
cargo clippy --all-targets
cargo fmt --all
```

Интеграционные тесты (`tests/golden.rs`) гоняют полный пайплайн в temp-каталогах:
самоописываемость, инкремент generation и детерминизм, split, лимит `--max-matches`,
`--hash`, стабильный dedup, dry-run, код 2 на невалидном шаблоне, различение
`no matches found` / `rule not applicable` / `error`.