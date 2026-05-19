
---

### README_RUN.md (русский)

```markdown
# Opterium GeoFormer — Быстрый старт

## Что делает

Библиотека для целочисленных вычислений без GPU. Умножение матриц, attention, lookup-операции.

## Требования

- Python 3.10+
- Rust 1.70+ (только для сборки)
- cffi (установится автоматически)

## Как запустить (Windows)

1. Установите Python 3.10+ с сайта python.org (при установке отметьте "Add to PATH").
2. Установите Rust через rustup.rs (запустите установщик).
3. Откройте папку с проектом.
4. Дважды кликните `run.bat`.
5. В чёрном окне появится результат.

## Как запустить (Linux / Mac)

```bash
chmod +x run.sh
./run.sh
```

## Ручной запуск

```bash
pip install cffi
python src/table_format.py
cd native && cargo build --release && cd ..
python demo.py
```

## Что вы увидите

Демо покажет табличные вычисления (lookup), умножение матриц, geometric attention и бенчмарк.

Пример вывода:

```
============================================================
  Opterium GeoFormer — Demo
============================================================
Загрузка таблиц: src/tables.ptbl
  Загружено за 2.1 мс
  Размер таблиц: 28.1 MB
  Max coord: 1024

  1. Pure Lookup (таблицы)
  P(4, 3) = 12
  S(4, 3) = 7
  D(4, 3) = 1
  proximity(0) = 10000
  isqrt(144) = 12

  2. Matrix Multiply (Rust)
  A = [[1, 2], [3, 4]]
  B = [[5, 6], [7, 8]]
  C = A × B = [[19, 22], [43, 50]]
  Результат: OK

  3. Geometric Attention (Rust)
  Токены: 3 шт
  Результат: [id, ctx_S, ctx_D, neighbors, output_P]

  4. Benchmark (16×16 matmul)
  100 итераций × 16×16 matmul
  Среднее время: 0.15 мс

  Всё работает!
```

## Как использовать в своём коде

```python
import sys
sys.path.insert(0, "native/python")
from geofield_native import GeoField

gf = GeoField("src/tables.ptbl")

# Lookup
p = gf.P(4, 3)  # 12

# Матричное умножение (плоские списки)
a = [1, 2, 3, 4]
b = [5, 6, 7, 8]
c = gf.matmul(a, 2, 2, b, 2)  # [19, 22, 43, 50]

# Attention
tokens = [0, 10, 10, 100, 1, 11, 10, 110]
result = gf.attention(tokens, 2, 5)
```

## Структура проекта

```
OpteriumGeoFormer/
├── run.bat / run.sh      ← Запуск демо
├── demo.py               ← Демо-скрипт
├── README_RUN.md         ← Эта инструкция
├── src/
│   ├── tables.ptbl       ← Таблицы (генерируются один раз)
│   └── table_format.py   ← Генератор таблиц
└── native/
    ├── include/geofield.h   ← C API
    ├── python/geofield_native.py ← Python-обёртка
    └── target/release/geofield.dll (или .so) ← Скомпилированная библиотека
```
```

---

### README_RUN.md (English)

```markdown
# Opterium GeoFormer — Quick Start

## What it does

A library for integer computation without a GPU. Matrix multiplication, attention, lookup operations.

## Requirements

- Python 3.10+
- Rust 1.70+ (build only)
- cffi (installed automatically)

## How to run (Windows)

1. Install Python 3.10+ from python.org (check "Add to PATH" during setup).
2. Install Rust via rustup.rs (run the installer).
3. Open the project folder.
4. Double-click `run.bat`.
5. Results will appear in the console window.

## How to run (Linux / Mac)

```bash
chmod +x run.sh
./run.sh
```

## Manual run

```bash
pip install cffi
python src/table_format.py
cd native && cargo build --release && cd ..
python demo.py
```

## What you will see

The demo displays table lookups, matrix multiplication, geometric attention, and a benchmark.

Example output:

```
============================================================
  Opterium GeoFormer — Demo
============================================================
Loading tables: src/tables.ptbl
  Loaded in 2.1 ms
  Table size: 28.1 MB
  Max coord: 1024

  1. Pure Lookup
  P(4, 3) = 12
  S(4, 3) = 7
  D(4, 3) = 1
  proximity(0) = 10000
  isqrt(144) = 12

  2. Matrix Multiply (Rust)
  A = [[1, 2], [3, 4]]
  B = [[5, 6], [7, 8]]
  C = A × B = [[19, 22], [43, 50]]
  Result: OK

  3. Geometric Attention (Rust)
  Tokens: 3 items
  Result: [id, ctx_S, ctx_D, neighbors, output_P]

  4. Benchmark (16×16 matmul)
  100 iterations × 16×16 matmul
  Average time: 0.15 ms

  All good!
```

## Usage in your own code

```python
import sys
sys.path.insert(0, "native/python")
from geofield_native import GeoField

gf = GeoField("src/tables.ptbl")

# Lookup
p = gf.P(4, 3)  # 12

# Matrix multiplication (flat lists)
a = [1, 2, 3, 4]
b = [5, 6, 7, 8]
c = gf.matmul(a, 2, 2, b, 2)  # [19, 22, 43, 50]

# Attention
tokens = [0, 10, 10, 100, 1, 11, 10, 110]
result = gf.attention(tokens, 2, 5)
```

## Project structure

```
OpteriumGeoFormer/
├── run.bat / run.sh      ← Launch demo
├── demo.py               ← Demo script
├── README_RUN.md         ← This guide
├── src/
│   ├── tables.ptbl       ← Tables (generated once)
│   └── table_format.py   ← Table generator
└── native/
    ├── include/geofield.h   ← C API
    ├── python/geofield_native.py ← Python wrapper
    └── target/release/geofield.dll (or .so) ← Compiled library
```
```
