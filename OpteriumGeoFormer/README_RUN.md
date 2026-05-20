# Opterium GeoFormer — Quick Start

## What it does

A library for integer computation without a GPU. Matrix multiplication, attention, lookup operations, debt system, E8 attention, and architecture flip pipeline.

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

The demo displays table lookups, matrix multiplication, geometric attention, debt system, E8 roots, verifier, and architecture flip pipeline.

Example output:

```
============================================================
  Opterium GeoFormer — Full Demo
============================================================
Loading tables: src/tables.ptbl
  Loaded in 34.5 ms
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

  4. Debt System
  3.4 × 2.33 = 7.9220
  0.1 + 0.2 = 0.3000
  by_P[12] = 3 pairs
  12 / 3 = 4

  5. E8 Root Lattice
  E8 roots count: 240
  address_to_root(2,2) = [2, 2, 0, 0, 0, 0, 0, 0]
  dot(root1, root2) = 8

  6. Verifier
  ✅ 234 × 567 = 132678
  ✅ 12 × 12 = 144
  ❌ 12 × 12 = 145

  7. Architecture Flip
  forward(42) = 96
  analogy(10, 20, 30) = 96
  verify('5 × 6 = 30') = True

  8. Benchmark (16×16 matmul)
  100 iterations × 16×16 matmul
  Average time: 0.07 ms

  All modules working!
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

# Debt System
d1 = gf.debt_from_float(3.4)
d2 = gf.debt_from_float(2.33)
result = gf.debt_mul(d1, d2)
print(gf.debt_to_float(result))  # 7.922

# E8 Attention
root = gf.e8_address_to_root(2, 2)
dot = gf.e8_dot_product(root1, root2)

# Verifier
from verifier import OpteriumVerifier
v = OpteriumVerifier()
result = v.verify("234 × 567 = 132678")

# Architecture Flip Pipeline
from opterium import OpteriumPipeline
pipeline = OpteriumPipeline(vocab_size=100, embed_dim=32, max_coord=1024)
output = pipeline.forward(42)
token_d = pipeline.analogy(10, 20, 30)
```

## Project structure

```
OpteriumGeoFormer/
├── run.bat / run.sh          ← Launch demo
├── demo.py                   ← Full demo (8 modules)
├── README_RUN.md             ← This guide
├── test_verifier.py          ← Verifier tests (24 tests)
├── test_debt.py              ← Debt tests (13 tests)
├── test_native_debt.py       ← Native debt tests
├── test_native_e8.py         ← Native E8 tests
├── test_pipeline.py          ← Pipeline tests (6 tests)
├── verifier/                 ← Verifier module
│   ├── __init__.py
│   ├── parser.py
│   ├── arithmetic.py
│   ├── debt.py
│   ├── closure.py
│   └── report.py
├── opterium/                 ← Architecture Flip module
│   ├── __init__.py
│   ├── encoder.py            ← Token → S/D address
│   ├── decoder.py            ← S/D address → token
│   ├── navigation.py         ← Reasoning (zero weights)
│   └── pipeline.py           ← Full pipeline
├── src/
│   ├── tables.ptbl           ← Tables (generated once)
│   └── table_format.py       ← Table generator
└── native/
    ├── include/geofield.h    ← C API (32+ functions)
    ├── python/geofield_native.py ← Python wrapper
    └── src/
        ├── lib.rs            ← Rust C API
        ├── tables.rs         ← Memory-mapped tables
        ├── lookup.rs         ← Matmul + attention
        ├── debt.rs           ← Debt system
        └── e8.rs             ← E8 root lattice
```

## Modules

| Module | Description | Tests |
|--------|-------------|-------|
| Pure Lookup | P, S, D, SP, prox, isqrt tables | — |
| Matrix Multiply | Rust + Rayon parallel | — |
| Geometric Attention | Hashgrid proximity | — |
| Debt System | (mantissa, debt) pairs, by_P index | 13 + native |
| E8 Attention | 240 roots on-the-fly | 12 + native |
| Verifier | Arithmetic claim verification | 24 |
| Architecture Flip | Encoder → Nav → Decoder | 6 |
| **Total** | | **60+ tests** |
