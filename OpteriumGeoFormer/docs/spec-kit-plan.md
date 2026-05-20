# Implementation Plan: Geometric Matrix Multiplication

**Branch**: `geo-matmul` | **Date**: 2026-05-18 | **Spec**: `spec-kit/spec.md`

## Summary

Implement 5 methods for matrix multiplication using geometric Δ-operators and PyTable relations, replacing all FP32 multiply-add with integer operations. Methods range from naive Δ-operator composition (P1) to hashgrid-resonant attention (P3). Benchmark, compare, and calibrate the winner.

## Technical Context

**Language/Version**: Python 3.10+  
**Primary Dependencies**: `numpy` (baseline only), `struct` (PyTable binary), `dataclasses` (Pt types)  
**Storage**: PyTable at `D:\gemma-4-geometric\dataset\PYTH_TABLE_1000.bin` (1M × 11 bytes, `<ihhbH` format)  
**Testing**: `pytest` for correctness, manual benchmark script for performance  
**Target Platform**: CPU (Windows x64)  
**Project Type**: Library (geometric computing primitives)  
**Performance Goals**: Beat torch.matmul on CPU for integer matrices by ≥2× (PyTable method), achieve O(n·k) scaling (hashgrid method)  
**Constraints**: Zero float operations in geometric methods; exact integer results; shape validation  
**Scale/Scope**: Matrix sizes 4×4 to 256×256; token counts up to 10K for hashgrid method

## Project Structure

```
bootstrap/
├── spec-kit/
│   ├── spec.md                # This specification
│   ├── plan.md                # This implementation plan
│   ├── tasks.md               # Task list
│   ├── methods/
│   │   ├── __init__.py        # Exports all methods
│   │   ├── baseline.py        # torch.matmul wrapper (comparison only)
│   │   ├── pt_naive.py        # Naive Δ-operator MM
│   │   ├── pytable_mm.py      # PyTable lookup MM
│   │   ├── sd_matmul.py       # S-D composition MM
│   │   └── geo_resonant.py    # Hashgrid-resonant attention
│   ├── tests/
│   │   ├── __init__.py
│   │   ├── test_correctness.py  # Correctness vs torch/numpy
│   │   └── benchmark.py         # Wall-time + memory benchmark
│   └── results/
│       └── (generated CSV files)
├── delta_ops.py               # Δ-operator library (loaded by all methods)
├── phi_algebra.py             # Φ-algebra (used for Pt repr)
├── swarm.py                   # Swarm (used for training, not MM)
```

## Method Designs

### Method 1 — PtNaive (Δ-operator direct)
- Every integer value v → `Pt(v, 1)` where P = v·1 = v
- Triple loop (i, j, k): `C[i][j] = Σ_k Δ_MUL(A[i][k], B[k][j])`
- `Δ_MUL` is `Pt(x₁x₂, y₁y₂)` with HealthVector from `delta_ops.py`
- `Δ_ADD` accumulates sum: `Pt(sum, 1)` for integers
- Complexity: O(n³) Δ-calls. Shape-validated.
- Uses: `delta_ops.DELTA_MUL`, `delta_ops.DELTA_ADD`

### Method 2 — PyTable Lookup (P = (S²−D²)//4)
- Same triple loop but replace Δ_MUL with PyTable lookup
- For each (a, b), look up `P = read_pytable(S_a_b, D_a_b)` from binary file
- Multiply P values as integers: `product = P_a * P_b`
- Sum products as integers: `C[i][j] += product`
- PyTable lookup: `offset = ((x-1)*1000 + (y-1)) * 11`, unpack `<ihhbH`
- Complexity: O(n³) lookups + O(n³) int MUL. Replace float MUL with int MUL.
- HealthVector: check P = (S²−D²)//4 equality as verification

### Method 3 — S-D Composition (No product computation)
- Represent each value as (S, D) pair: `S = x+y`, `D = x-y`
- Product rule in S-D space: 
  - `S_product = S₁·S₂ + D₁·D₂ - 2·(x₁·y₁ + x₂·y₂)` — too complex
  - Simpler: compute intermediate Pt via formula, then read P from PyTable
- Actually, the composition is: given A(i,k) = (S₁, D₁) and B(k,j) = (S₂, D₂):
  - The product value = P₁·P₂ where P₁ = (S₁²−D₁²)//4, P₂ = (S₂²−D₂²)//4
  - This IS the PyTable lookup — same as Method 2 but hides the P extraction
- Key difference: all intermediate values remain in (S,D) form; no int MUL needed until final composition
- Complexity: O(n³) with int arithmetic only

### Method 4 — Hashgrid Resonant (No matrix multiply at all)
- GeoFormer approach: replace QK^T with spatial proximity in (S,D) space
- Hashgrid: divide (S,D) plane into W×W buckets. Token falls into bucket `(S//W, D//W)`.
- For each token t with (S_t, D_t):
  1. Hashgrid lookup: fetch all tokens in same and adjacent buckets (≤ neighbors)
  2. Weight = 1 / (1 + |S_i − S_t| + |D_i − D_t|)  — geometric proxity, not dot product
  3. Context = Σ weight_i · P_i / Σ weight_i — weighted sum
- Complexity: O(n·k) where k = bucket size (constant). No matrix multiplication.
- FFN replacement: Pt3(x, y, context) — triple product, no Linear·ReLU·Linear.

## Benchmark Protocol
- Matrix sizes: [4, 8, 16, 32, 64, 128, 256]
- Each method × each size: 5 warmup + 10 measured runs
- Metrics: wall-time (ms), relative speedup vs torch.matmul
- Output: CSV to `results/benchmark.csv`
- Environment: CPU only, single thread
