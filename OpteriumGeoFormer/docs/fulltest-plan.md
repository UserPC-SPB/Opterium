# Implementation Plan: Full Test — Documentation Integrity & Code Cross-Verification

**Date**: 2026-05-19 | **Spec**: `docs/fulltest-spec.md`
**Project**: Opterium GeoFormer — all 10 modules

## Summary

Создать инструмент `test_doc_integrity.py`, который автоматически:
1. Сканирует все 10 модулей на наличие публичных функций/методов
2. Сверяет каждую с spec_compiled.json
3. Выполняет каждый documented example и сверяет вывод
4. Проверяет форматирование API.md
5. Сверяет help.txt с кодом
6. Проверяет воспроизводимость генерации
7. Выдаёт coverage report

Дополнительно: систематические edge-case тесты для каждой документированной функции,
покрывающие нулевые/отрицательные/граничные входы.

## Technical Context

**Язык**: Python 3.10+
**Зависимости**: stdlib (inspect, json, ast, difflib)
**Входные данные**:
- 10 модулей в `src/` (или `src/spec-kit/methods/`)
- `src/spec-kit/spec_compiled.json` (10 модулей, 217+ примеров)
- `src/spec-kit/API.md` (~800 строк)
- `src/spec-kit/help.txt` (~140 строк)
**Выход**: `results/coverage_report.json` + stdout summary

## Project Structure (дополнение к существующей)

```
src/
└── test_doc_integrity.py     # Главный тест-раннер (новый)
docs/
├── fulltest-spec.md           # Этот spec
├── fulltest-plan.md           # Этот plan
└── fulltest-tasks.md          # Task list
results/
└── coverage_report.json       # Coverage report (генерируется)
```

## Phase Design

### Phase 1 — Foundation: Introspection & Doc Loader
Создать утилиты для:
- `get_public_api(module)` — извлекает все публичные имена из модуля через `dir()` + фильтр
- `load_spec(path)` — загружает spec_compiled.json
- `load_md_sections(path)` — парсит API.md на модули/классы/методы
- `load_help(path)` — парсит help.txt на имена + сигнатуры

### Phase 2 — Code-Doc Coverage Scan
Для каждого из 10 модулей:
1. Извлечь публичные имена из кода
2. Найти соответствующие DocEntry в spec_compiled.json
3. Отметить недокументированные (в коде, нет в docs)
4. Отметить phantom (в docs, нет в коде)
5. Сверить сигнатуры

### Phase 3 — Example Execution
Для каждого Example в spec_compiled.json:
1. Парсить description и input как вызов функции
2. Выполнить вызов
3. Сверить результат с documented output
4. Засчитать pass/fail

### Phase 4 — API.md & help.txt Validation
- API.md: проверить форматирование, дубликаты, секции
- help.txt: сверить все имена с actual code API
- Проверить, что pipeline spec_collect.py → spec_compiled.json → spec_to_help.py воспроизводим

### Phase 5 — Edge-Case Systematics
Для каждой функции из documentation:
- Тест с нулевыми значениями (0, 0, Pt(0,0), Decimal(0))
- Тест с отрицательными значениями
- Тест с граничными значениями (1024, -1024, 1e-30, 1e30)
- Тест с инвалидными типами (строка вместо int, None вместо Pt)
- Тест out-of-range (>1024, gcd-scaling fallback)

### Phase 6 — Report Generation
- Итоговый coverage report: JSON + stdout summary
- Включает: per-module coverage, example pass rate, formatting errors
- Сравнение с предыдущим прогоном (если есть results/coverage_report.json)

## Parallelism

| Phase | Зависит от | Параллельные задачи |
|-------|-----------|--------------------|
| P1 Foundation | — | get_public_api, load_spec, load_md_sections, load_help — независимы |
| P2 Coverage | P1 | Каждый модуль может сканироваться независимо (10 parallel tasks) |
| P3 Examples | P1 | Каждый Example может выполняться независимо |
| P4 API.md | P1 | API.md validation, help.txt validation, reproducibility — независимы |
| P5 Edge Cases | P2, P3 | Каждая функция тестируется независимо |
| P6 Report | Все | Один финальный шаг |

## Data Flow

```
code/*.py ──→ get_public_api() ──→ {func_name: signature, ...}
                                              ↓
spec_compiled.json ──→ load_spec() ──→ {module: {class: {method: {examples, ...}}}}
                                              ↓
                                        coverage_scan() → report
                                              ↓
API.md ──→ load_md_sections() ──→ {section: content, ...}
                                              ↓
                                        format_validate() → errors[]
                                              ↓
help.txt ──→ load_help() ──→ {name: sig, ...}
                                              ↓
                                        help_validate(code_api) → diffs[]
                                              ↓
                                        edge_case_test() → failures[]
                                              ↓
                                    merge → coverage_report.json
```

## Risk Assessment

| Риск | Вероятность | Влияние | Mitigation |
|------|------------|---------|------------|
| Dynamic imports (eval/exec) усложняют сканирование | Low | Medium | Использовать ast.parse + анализатор bindings |
| Некоторые examples требуют сложной настройки (объекты, не repr) | Medium | Medium | Разрешить skip с annotation |
| spec_compiled.json outputs не всегда repr-совместимы | Medium | Low | Normalize через json.dumps для сравнения |
| API.md formatting — поверхностная проверка | Low | Low | Только базовые проверки (скобки, секции) |
