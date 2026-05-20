---
description: "Task list for full test: documentation integrity & code cross-verification"
---

# Tasks: Full Test — Documentation Integrity

**Input**: fulltest-spec.md, fulltest-plan.md
**Prerequisites**: Все 10 модулей существуют и работают. spec_compiled.json, API.md, help.txt — сгенерированы.

## Format: `[ID] [P] [Story] Description`

- **[P]**: Can run in parallel
- **[Story]**: US-FT1 through US-FT7

---

## Phase 1: Foundation — Introspection & Doc Loaders

**Purpose**: Утилиты для извлечения публичного API из кода и парсинга документации.

- [ ] T001 [P] [US-FT1] `get_public_api(module) -> Dict[str, Dict]`
  - Загружает модуль по имени (из src/ или spec-kit/)
  - Собирает все `def name` и `class name` через `dir()` + фильтр `not startswith('_')`
  - Для каждого: реальная сигнатура через `inspect.signature()`
  - Возвращает `{name: {'kind': 'function'|'class', 'sig': str, 'doc': str}}`

- [ ] T002 [P] [US-FT1] `load_spec(path) -> Dict`
  - Загружает spec_compiled.json
  - Индексирует: `spec_index[(module, class, method)] = entry`
  - Индексирует: `spec_index[(module, function)] = entry`
  - Упрощает lookup из Phase 2

- [ ] T003 [P] [US-FT3] `load_md_sections(path) -> Dict[str, List[str]]`
  - Парсит API.md, разбивая на секции `## Module:`
  - Для каждой секции извлекает: module name, class names, method names
  - Возвращает `{module: {class: [methods], functions: [names]}}`

- [ ] T004 [P] [US-FT4] `load_help(path) -> Dict[str, str]`
  - Парсит help.txt, извлекая все строки с `def ` и `.methodName`
  - Возвращает `{name: 'class.method' or 'function'}`

- [ ] T005 [P] [US-FT2] `exec_example(example) -> Any`
  - Парсит description как вызов функции (безопасно, через `ast.literal_eval`)
  - Для простых случаев: разбирает строку вида `func(arg1, arg2)`
  - Выполняет через eval в контексте модуля
  - Возвращает результат для сравнения

**Checkpoint**: Foundation ready — Phase 2-5 могут работать независимо.

---

## Phase 2: Code-Doc Coverage Scan

**Purpose**: Для каждого из 10 модулей — сверка actual API против documented API.

- [ ] T010 [P] [US-FT1] Scan PtTable (`arith_table.py`)
- [ ] T011 [P] [US-FT1] Scan Pt (`spec-kit/methods/__init__.py`)
- [ ] T012 [P] [US-FT1] Scan Cube27 (`cube27.py`)
- [ ] T013 [P] [US-FT1] Scan HashGrid (`hashgrid.py`)
- [ ] T014 [P] [US-FT1] Scan delta_ops (`delta_ops.py`)
- [ ] T015 [P] [US-FT1] Scan phi_algebra (`phi_algebra.py`)
- [ ] T016 [P] [US-FT1] Scan swarm (`swarm.py`)
- [ ] T017 [P] [US-FT1] Scan e8_twist (`e8_twist.py`)
- [ ] T018 [P] [US-FT1] Scan doctor_geo (`doctor_geo.py`)
- [ ] T019 [P] [US-FT1] Scan geoformer (`geoformer.py`)

Для каждого модуля:
1. Извлечь публичные имена из кода → `code_api`
2. Извлечь документированные имена из spec → `spec_api`
3. Вычислить: `missing = code_api - spec_api` (есть в коде, нет в docs)
4. Вычислить: `phantom = spec_api - code_api` (есть в docs, нет в коде)
5. Для совпадающих: сверить сигнатуры

**Checkpoint**: Coverage gaps выявлены для всех 10 модулей.

---

## Phase 3: Example Execution

**Purpose**: Выполнить каждый documented example, сверить с documented output.

- [ ] T030 [P] [US-FT2] Execute all PtTable examples (77 examples)
- [ ] T031 [P] [US-FT2] Execute all Pt examples (47 examples)
- [ ] T032 [P] [US-FT2] Execute all Cube27 examples (25 examples)
- [ ] T033 [P] [US-FT2] Execute all HashGrid examples (6 examples)
- [ ] T034 [P] [US-FT2] Execute all delta_ops examples (24 examples)
- [ ] T035 [P] [US-FT2] Execute all phi_algebra examples (7 examples)
- [ ] T036 [P] [US-FT2] Execute all swarm examples (6 examples)
- [ ] T037 [P] [US-FT2] Execute all e8_twist examples (10 examples)
- [ ] T038 [P] [US-FT2] Execute all doctor_geo examples (7 examples)
- [ ] T039 [P] [US-FT2] Execute all geoformer examples (8 examples)

Для каждого примера:
1. Извлечь имя функции и аргументы из description
2. Вызвать функцию с аргументами
3. Сериализовать результат через `json.dumps` (или `repr`)
4. Сравнить с documented `output`
5. Записать pass/fail + фактический вывод при расхождении

**Edge cases to test additionally for each function**:
- Вызов с нулевыми аргументами (0, Pt(0,0), Decimal(0))
- Вызов с отрицательными аргументами (включая -0)
- Вызов с None/null (ожидается TypeError)
- Вызов с out-of-range (>1024, gcd-scaling)

**Checkpoint**: Все 217+ examples выполнены, результаты зафиксированы.

---

## Phase 4: API.md & help.txt Validation

**Purpose**: Формальная верификация API.md и help.txt.

- [ ] T040 [P] [US-FT3] Validate API.md formatting
  - Проверить баланс `` ` `` (backtick spans — чётное число)
  - Проверить структуру: каждый `## Module:` имеет подсекции
  - Проверить дубликаты: нет двух `### Class SameName`
  - Проверить, что все `->` имеют тип слева и справа
  - Проверить, нет ли `�` (Unicode replacement characters — encoding error)

- [ ] T041 [P] [US-FT4] Validate help.txt
  - Извлечь все имена (после `def ` и `.`)
  - Сверить с code_api на наличие phantom/missing
  - Проверить, что сигнатуры парсятся (нет сломанного синтаксиса)

- [ ] T042 [P] [US-FT6] Validate generation reproducibility
  - Запустить spec_collect.py, сохранить хеш spec_compiled.json
  - Запустить spec_to_help.py, сохранить хеш API.md и help.txt
  - Запустить снова, проверить что хеши совпадают
  - Запустить в чистом окружении (sys.modules clean), проверить

**Checkpoint**: API.md и help.txt валидированы.

---

## Phase 5: Edge-Case Systematics

**Purpose**: Систематическое тестирование на граничных значениях каждой функции.

- [ ] T050 [P] [US-FT7] PtTable edge systematics
  - `PT.S(0,0)`, `PT.D(0,0)`, `PT.P(0,0)` — нулевые
  - `PT.S(-1024, 1024)`, `PT.D(1024, -1024)` — граничные
  - `PT.has(9999, 1)` = False — out-of-range
  - `PT.product(9999, 1)` — gcd-scaling fallback
  - `PT.pairs_for_product(0)` — 2049 factor pairs
  - `PT.from_sd(0, 0)` — zero coordinates

- [ ] T051 [P] [US-FT7] Pt edge systematics
  - `Pt(0, 0)`, `Pt(0, 1)`, `Pt(0, 5)` — rank-zero variants
  - `Pt.from_real(0.0)` → `Pt(0, 0)`
  - `Pt.from_decimal(Decimal('0'))` → `Pt(0, 0)`
  - `Pt.parse("0|0|")` — zero mantissa + zero rank
  - `inv(Pt(0, 1))` — inv of zero → Pt(0,1)
  - `rmul(Pt(0,0), Pt(5,1))` — zero × anything
  - `rdiv(Pt(5,1), Pt(0,1))` — division by zero (error or fallback)
  - `radd(Pt(0,0), Pt(0,0))` — zero + zero
  - `rsub(Pt(0,0), Pt(0,0))` — zero - zero
  - `validate_shape(0, 0)` — empty matrices
  - Out-of-range mantissa: (9999999999999999999999999999999, 0)

- [ ] T052 [P] [US-FT7] Cube27 edge systematics
  - `encode(0)` — single zero
  - `encode(-1)` — negative (error expected)
  - `cell_index(1000)` — clamped to 26
  - `path_27(0)` — depth=1
  - `verify(0)` — zero mantissa

- [ ] T053 [P] [US-FT7] HashGrid edge systematics
  - `lookup` при пустом grid → []
  - `geometric_attention([])` → []
  - `geometric_weight(SAME, SAME)` → 1.0
  - `geometric_weight(FAR, FAR)` → ≈ 0.0

- [ ] T054 [P] [US-FT7] delta_ops edge systematics
  - `DELTA_ADD(0, 0)` → zero
  - `DELTA_MUL(0, 5)` → zero
  - `DELTA_INV(0.0)` → inf HV
  - `HealthVector.max_channel` на нулевом HV
  - `compose_sequential()` — пустая композиция
  - `compose_parallel()` — пустая композиция

- [ ] T055 [P] [US-FT7] phi_algebra / swarm / e8_twist / doctor_geo / geoformer edges
  - PHI1_SHIFT с нулевым dx
  - swarm.register с нулевым potential
  - TwistEngine с нулевым углом
  - SwarmDoctor.judge с нулевым HV → OK
  - GeoFormer.forward с пустым списком токенов

**Checkpoint**: Все edge-cases протестированы.

---

## Phase 6: Report Generation

**Purpose**: Сводный отчёт о покрытии и найденных проблемах.

- [ ] T060 [US-FT1 to FT7] `generate_report() -> dict`
  - Агрегировать данные из Phase 2-5
  - Для каждого модуля:
    - `total_functions`: публичных функций
    - `documented`: есть в spec
    - `missing`: в коде, нет в docs
    - `phantom`: в docs, нет в коде
    - `examples_total`: примеров
    - `examples_passed`: успешно выполненных
    - `edge_total`: edge-тестов
    - `edge_passed`: успешных edge-тестов
    - `format_errors`: ошибки форматирования
  - Итоговые метрики:
    - Coverage % = documented / total_functions
    - Example pass rate % = examples_passed / examples_total
    - Edge pass rate % = edge_passed / edge_total
    - Integrity score (среднее 3 метрик)

- [ ] T061 Сохранить отчёт в `results/coverage_report.json`

- [ ] T062 Вывести сводку в stdout в читаемом виде (таблица ASCII)

**Checkpoint**: Коverage report выпущен, все gaps задокументированы.

---

## Phase Dependencies

```
Phase 1 (Foundation)
  ├── Phase 2 (Coverage) — зависит от T001-T002
  ├── Phase 3 (Examples) — зависит от T002, T005
  ├── Phase 4 (Format)   — зависит от T003-T004
  └── Phase 5 (Edges)    — зависит от T001-T002, может идти параллельно с P2-P4
       └── Phase 6 (Report) — зависит от P2+P3+P4+P5
```

## Execution Order

1. **T001-T005** — Foundation (параллельно все 5)
2. **T010-T019** — Coverage scan (параллельно все 10 модулей)
3. **T030-T039** — Example execution (параллельно все 10 модулей)
4. **T040-T042** — Format validation (параллельно 3 задачи)
5. **T050-T055** — Edge systematics (параллельно 6 групп)
6. **T060-T062** — Report (последовательно)

## Exit Criteria

- [ ] Все 10 модулей имеют 100% code-doc coverage
- [ ] Все 217+ examples выполнены с 100% pass rate
- [ ] API.md — 0 formatting errors
- [ ] help.txt — 0 phantom/missing entries
- [ ] Pipeline deterministic (same input → same output)
- [ ] Все edge cases протестированы и задокументированы (pass или expected fail)
- [ ] Coverage report сохранён в results/coverage_report.json
