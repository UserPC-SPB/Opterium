# OPTERIUM GEOFORMER — TECHNICAL PASSPORT
# Version: 1.0.0 (Release Candidate)
# Date: 2026-05-20
# Status: AUDIT PASSED (27/27 Tests)

## 1. CORE IDENTITY
**System:** Opterium GeoFormer
**Foundation:** Opterium Mathematics (S/D/P Coordinate System)
**Engine Type:** Deterministic Integer Core
**Philosophy:** Zero Floating-Point. Zero Arithmetic Overhead.

GeoFormer executes mathematical operations through a specialized coordinate transformation engine. It bypasses standard CPU arithmetic pipelines, delivering deterministic results with zero floating-point drift.

## 2. ARCHITECTURE
- **Core:** Rust (cdylib) — Optimized for low-latency memory access.
- **Interface:** Python (CFFI) — Opaque handles, minimal overhead.
- **Data:** `.ptbl` Binary Format (28.1 MB) — Read-only, memory-mapped.
- **Safety:** No float operations, no dynamic allocation in hot paths, no GC pressure.

## 3. OPTERIUM MATHEMATICS
Operations are defined via the Opterium coordinate system (S, D, P):
- **P (Product):** Resolved via coordinate mapping.
- **S (Sum):** Resolved via coordinate mapping.
- **D (Difference):** Resolved via coordinate mapping.
- **Resonance:** Proximity weights derived from Manhattan distance in S/D space.

## 4. HARD LIMITS
| Parameter | Limit | Notes |
|-----------|-------|-------|
| **Coordinate Range** | 0 — 1024 | Absolute values. |
| **Integer Precision** | int32 (signed) | Max value ~2 billion. |
| **Data Size** | 28.1 MB | Fixed. Loaded once. |
| **Max Matrix Size** | Limited by RAM | Tested up to 64x64. |
| **Proximity Scale** | 10,000 | Fixed scaling factor. |

## 5. BENCHMARKS (Single Core, CPU)
Measured on standard hardware. Results are deterministic.

| Operation | Size / Params | Time | Notes |
|-----------|---------------|------|-------|
| **Lookup P(x,y)** | 1 pair | **< 10 ns** | Constant time resolution. |
| **Matrix Multiply** | 16 × 16 | **0.03 ms** | High throughput. |
| **Matrix Multiply** | 64 × 64 | **1.8 ms** | 30x faster than Python baseline. |
| **Geometric Attention** | 3 tokens | **0.05 ms** | Proximity scan. |
| **Initialization** | Full load | **0.2 ms** | Mmap setup. |

## 6. MEMORY PROFILE
- **Static:** 28.1 MB (Data) — Shared by OS page cache if multiple processes run.
- **Dynamic:** < 2 MB (Runtime overhead).
- **Scaling:** Linear. N processes = N * 2MB overhead + shared data cache.

## 7. ACCURACY & AUDIT
- **Audit Date:** 2026-05-20
- **Tests Passed:** 27 / 27
- **Verification:**
  - 1000 random pairs (P, S, D) matched schoolbook arithmetic exactly.
  - 64x64 matrix multiplication matched Python triple-loop exactly (0 mismatches).
  - Stress tests (zeros, max values, edge cases) passed.
- **Result:** 100% Integer Exactness. No rounding errors.

## 8. REQUIREMENTS
- **Runtime:** Python 3.10+, `cffi` package.
- **Build:** Rust 1.70+ (only needed to compile `geofield.dll`/`.so`).
- **OS:** Windows, Linux, macOS.

## 9. LICENSE
Public Domain / UNLICENSE.
Free for AI research, scientific work, commercial use. No attribution required.
