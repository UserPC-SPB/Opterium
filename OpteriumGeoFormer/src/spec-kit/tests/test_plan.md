# Полный тест GPU-free Matrix Multiply — Opterium GeoFormer

## Цель
Верифицировать что все 5 методов geometric matrix multiply работают **без GPU, без float**, на чистой целочисленной арифметике.

## Методы

| # | Метод | Файл | Принцип |
|---|-------|------|---------|
| 1 | `pt_naive` | pt_naive.py | Pt geo_mul/geo_add, O(n³) |
| 2 | `pt_naive_fast` | pt_naive.py | Прямой int accumulator, O(n³) |
| 3 | `pytable_matmul` | pytable_mm.py | PyTable binary lookup |
| 4 | `pytable_matmul_cached` | pytable_mm.py | PyTable + pre-cache P values |
| 5 | `sd_matmul` | sd_matmul.py | (S,D) формула: P=(S²−D²)//4 |
| 6 | `geo_resonant` | geo_resonant.py | Hashgrid attention (zero MM) |

## План тестов

### Группа A: Correctness (корректность)

| ID | Тест | Описание | Методы |
|----|------|----------|--------|
| A1 | Square match | 4×4 random int vs torch baseline | 1-5 |
| A2 | Non-square | 3×5 × 5×2 = 3×2 | 1-5 |
| A3 | Identity | A·I = A | 1-5 |
| A4 | Zero | A·0 = 0, 0·A = 0 | 1-5 |
| A5 | Negative values | Матрицы с отрицательными элементами | 1-5 |
| A6 | Large values | Элементы до 10⁶ | 1-5 |
| A7 | All methods agree | Все 5 методов дают одинаковый результат | 1-5 |
| A8 | Single element | 1×1 матрицы | 1-5 |
| A9 | Scalar multiply | n=1, n=2 edge cases | 1-5 |

### Группа B: Pt Arithmetic (арифметика точек)

| ID | Тест | Описание |
|----|------|----------|
| B1 | Pt from_int roundtrip | Pt.from_int(v).P == v |
| B2 | Pt from_sd roundtrip | from_sd(S,D) → (x,y) → S,D |
| B3 | Pt parse/repr | "347|3|" → Pt(347,3) → "347|3|" |
| B4 | Pt from_real/to_real | Float roundtrip точность |
| B5 | Pt from_decimal/to_decimal | Decimal exact roundtrip |
| B6 | Pt inv | inv(inv(x)) ≈ x |
| B7 | rmul/radd/rsub/rdiv | Mantissa-rank arithmetic |
| B8 | geo_mul/geo_add | Geometric operations |

### Группа C: HealthVector (мониторинг)

| ID | Тест | Описание | Методы |
|----|------|----------|--------|
| C1 | HV ok | Все каналы < 0.35 для валидных входов | 1-5 |
| C2 | HV warn | HV на границе порога | 1-5 |
| C3 | HV merge | Слияние двух HV | 1-5 |

### Группа D: GeoResonant (zero-MM attention)

| ID | Тест | Описание |
|----|------|----------|
| D1 | Embed int sequence | embed_int_sequence([1,2,3]) |
| D2 | Single layer attention | geo_attention(tokens) |
| D3 | Multi-layer | geo_resonant(tokens, layers=4) |
| D4 | Empty input | geo_attention([]) |
| D5 | Single token | geo_attention([Pt(1,1)]) |
| D6 | HashGrid insert/lookup | Bucket operations |
| D7 | Output shape | len(output) == len(input) |

### Группа E: Stress & Edge Cases

| ID | Тест | Описание | Методы |
|----|------|----------|--------|
| E1 | n=64 correctness | 64×64 random | 1-5 |
| E2 | n=128 correctness | 128×128 random | 2,5 |
| E3 | Zero matrix | 0×0 edge case | 1-5 |
| E4 | Max coord | Элементы ±1024 (граница PtTable) | 1-5 |
| E5 | Shape mismatch | ValueError при несовместимых размерах | 1-5 |
| E6 | No float assertion | Проверка что нет float операций | 1-5 |

### Группа F: Cross-verify (Python = Cython = Rust)

| ID | Тест | Описание |
|----|------|----------|
| F1 | Py = Cy (16×16) | sd_matmul Python vs Cython |
| F2 | Py = Rs seq (16×16) | Python vs Rust sequential |
| F3 | Rs seq = Rs par | Rust sequential vs parallel |
| F4 | All equal (4×4) | Py = Cy = Rs = torch |

### Группа G: Benchmark (производительность)

| ID | Тест | Описание |
|----|------|----------|
| G1 | Wall-time n=[4,16,64] | Все методы |
| G2 | Speedup vs torch | Относительная скорость |
| G3 | Memory footprint | PtTable size |

### Группа H: Language Handicap (H-factor — поправка на уровень языка)

| ID | Тест | Описание |
|----|------|----------|
| H1 | Cython vs Python | H-factor: во сколько раз Cython быстрее pure Python |
| H2 | Rust vs Python | H-factor: во сколько раз Rust быстрее pure Python |
| H3 | Level-adjusted table | Все методы нормализованы к compiled baseline (best=1.0x) |

**Зачем:** Сравнивать raw ms torch (C/CUDA) и pure Python некорректно — это разные уровни.
H-factor показывает "налог интерпретатора" и даёт честное сравнение алгоритмов.

## Критерии прохождения

- **Все A-тесты**: PASS — корректность гарантирована
- **Все B-тесты**: PASS — Pt arithmetic работает
- **Все C-тесты**: PASS — HealthVector валиден
- **Все D-тесты**: PASS — geo_resonant работает без MM
- **E1-E2**: PASS — масштабируемость
- **E3-E6**: PASS — edge cases handled
- **F1-F4**: PASS — cross-verify (если доступны Cython/Rust)
- **G1-G3**: INFO — benchmark данные

## Запуск

```bash
cd C:\Users\eccoa\Desktop\OpteriumGeoFormer
python src/spec-kit/tests/test_full_suite.py              # full suite (42 tests)
python src/spec-kit/tests/test_full_suite.py --quick      # A+B+C only (fast)
python src/spec-kit/tests/test_full_suite.py --group A B  # specific groups
python src/spec-kit/tests/test_full_suite.py --json       # JSON output
python src/spec-kit/tests/benchmark.py                     # detailed benchmark
```
