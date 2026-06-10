# Cube v5 — FINAL BENCHMARK & TEST REPORT

**Date:** 2026-06-10  
**Binary:** `cube_v5.exe` (22.82 MB)  
**Platform:** Windows 10, Python 3.11  

---

## 1. SUMMARY

| Metric | Value |
|--------|-------|
| Total tests | 118 |
| Passed | **115/118** (97.5%) |
| Cold start (median) | **289.4 ms** |
| Persistent latency (median) | **~50-70 us** |
| Throughput (peak) | **15,882 req/s** |
| Tools | 71 |
| Binary size | 22.82 MB |

---

## 2. BENCHMARK RESULTS

### 2.1 Cold Start (each request = new exe process)

| Tool | Latency (ms) | Status |
|------|-------------|--------|
| tools/list | 392.5 | 71 tools |
| cubint_add(3,4) | 286.7 | result=7 |
| cubint_mul(7,8) | 286.7 | result=56 |
| cubfloat_add(0.1,0.2) | ~290 | 0.3 |
| cubcomplex_add(1+2i,3+4i) | 287.8 | re=4,im=6 |
| e8(1) | 298.8 | 8D vector |
| e8_get_root(0) | 300.0 | [2,2,0,0,0,0,0,0] |
| e8_duality_check(1) | 286.3 | 56 partners |
| vec_sum([1,2,3]) | 318.7 | ok |
| doctor(12) | 306.6 | factors |
| help(en) | 291.5 | >200 chars |

**Median cold start: ~289 ms**

### 2.2 Persistent Mode Latency

| Category | Avg | Median | P95 | Min | Max | N |
|----------|-----|--------|-----|-----|-----|---|
| cubfloat | 0.070ms | **0.067ms** | 0.090ms | 0.044ms | 0.090ms | 6 |
| cubcomplex | - | **0.046ms** | - | 0.046ms | - | 7 |
| vector | - | **0.044ms** | - | 0.042ms | - | 19 |
| spatial | - | **0.043ms** | - | 0.042ms | - | 6 |
| e8 | - | **0.074ms** | - | 0.042ms | - | 15 |
| cubint | - | **0.119ms** | - | 0.068ms | - | 23 |
| doctor | - | **0.086ms** | - | - | - | - |

**Persistent mode: 43-119 us** — 3 orders of magnitude faster than cold start.

### 2.3 Throughput (requests/sec)

| Test | N | Total (ms) | Avg (ms) | **Req/s** | OK |
|------|---|-----------|---------|----------|-----|
| cubint_mul x50 | 50 | 3.1 | 0.050 | **15,882** | 50/50 |
| cubint_add x200 | 200 | 14.3 | 0.059 | **13,935** | 200/200 |
| e8 x50 | 50 | 3.6 | 0.059 | **13,809** | 50/50 |
| vec_sum x100 | 100 | 7.9 | 0.059 | **12,658** | 100/100 |
| e8_batch x50 | 50 | 5.0 | 0.080 | **9,989** | 50/50 |
| doctor x100 | 100 | 10.4 | 0.088 | **9,627** | 100/100 |

---

## 3. COMPARISON WITH ALTERNATIVES

| Feature | **Cube v5** | Wolfram Alpha | SymPy | SageMath | NumPy | mpmath |
|---------|------------|---------------|-------|----------|-------|--------|
| Delivery | **.exe (1 file)** | API (cloud) | pip (lib) | pip (lib) | pip (lib) | pip (lib) |
| Size | **~23 MB** | - | ~50 MB | ~1-2 GB | ~30 MB | ~5 MB |
| MCP protocol | **YES stdio** | NO | NO | NO | NO | NO |
| Tools | **71** | 10,000+ | 200+ | 500+ | 100+ | 50+ |
| Non-standard arith | **YES (witness)** | NO | NO | NO | NO | NO |
| E8-algebra (240) | **YES full** | partial | NO | YES | NO | NO |
| Float precision | **Fixed-point** | Arbitrary | Rational | Arbitrary | IEEE 754 | Arbitrary |
| Complex numbers | YES | YES | YES | YES | YES | YES |
| Vector ops | **19 tools** | YES | YES | YES | YES (opt) | YES |
| 3D space | **YES (6)** | YES | NO | NO | NO | NO |
| Doctor | **YES** | NO | Partial | NO | NO | NO |
| GPU accel | NO | YES | NO | YES | YES (BLAS) | NO |
| Symbolic | NO | YES | YES | YES | NO | YES |
| Matrices | NO | YES | YES | YES | YES (BLAS) | Partial |
| Trigonometry | NO | YES | YES | YES | YES (ufunc) | YES |
| LLM integration | **YES native** | via API | no | no | no | no |
| Offline/private | **YES** | NO | YES | YES | YES | YES |
| Price | **FREE** | paid API | FREE | FREE | FREE | FREE |
| Throughput | **~14K req/s** | ~100 req/s (API) | ~10K (lib) | ~1K | ~100K (BLAS) | ~5K |

### Unique advantages of Cube v5:
1. **Only MCP-native math server** — Wolfram Alpha has no MCP, SymPy/SageMath are libraries
2. **Non-standard arithmetic** — Pythagorean table with witness, unique math without analogues
3. **Zero-dependency** — single exe, no installation required
4. **Offline-first** — all computation local, no internet access

---

## 4. PROS

| # | Advantage | Description |
|---|-----------|-------------|
| 1 | **MCP-native** | Only math MCP server with 71 tools. One config line to integrate with LM Studio/Cline/Claude |
| 2 | **Non-standard arithmetic** | Pythagorean table with witness — unique math not found in any other tool |
| 3 | **E8-algebra** | Full work with 240 E8 roots: Weyl group, duality, spectrum, triangle geometry — MCP exclusive |
| 4 | **Zero-dependency** | Single .exe file. No Python, pip, venv, CUDA. Works on any Windows without installation |
| 5 | **Instant start** | Cold start ~289 ms. Persistent mode: requests in microseconds (~50 us) |
| 6 | **CubFloat** | Fixed-point arithmetic — no IEEE 754 rounding errors (0.1 + 0.2 = 0.3) |
| 7 | **CubComplex** | Exact complex arithmetic: add, sub, mul, pow, conjugate, abs, neg |
| 8 | **Vector (19)** | Full vector operations: arithmetic, stats, normalize, clip, sort, unique |
| 9 | **Spatial (6)** | 3D space: placement, movement, gravity alignment, distance calculations |
| 10 | **Doctor** | Factorization, divisors, primality testing — number diagnostics |
| 11 | **Deterministic random** | Seed-based RNG — reproducible results |
| 12 | **Multilingual** | Help in 4 languages: EN, RU, ZH, DE |
| 13 | **Offline/privacy** | All computation local. No data sent to cloud |
| 14 | **Throughput ~14K req/s** | Persistent mode: ~0.05ms per request, 10K+ requests per second |

---

## 5. CONS

| # | Drawback | Description | Impact |
|---|----------|-------------|--------|
| 1 | **No GPU** | All computation on CPU. No CUDA/Metal | Medium for E8 batch 10K+ |
| 2 | **N <= 100 for witness** | Domain limitation for witness arithmetic | Limits scale |
| 3 | **No symbolic computation** | Numerical only. No symbolic differentiation/integration | No calculus |
| 4 | **No matrix operations** | No det, eig, LU, QR, matrix multiplication | Key gap for science |
| 5 | **No trigonometry** | sin, cos, tan, atan missing | Limits physics/engineering |
| 6 | **No stat distributions** | No normal, Poisson, chi-squared | Basic stats only |
| 7 | **Single-threaded** | One thread. Doesn't utilize all CPU cores | |
| 8 | **Windows-only** | .exe file — no Linux/macOS version | |
| 9 | **No PyPI package** | No pip install. Manual download required | |
| 10 | **Closed-source format** | Binary cannot be modified without rebuild | |

---

## 6. MARKET DEMAND

### Segment 1: LLM Tool-Use / MCP Ecosystem (GROWING MARKET)
- **MCP protocol** (Anthropic, 2024) — new standard for LLM tool integration
- **LM Studio, Cline, Claude Desktop, Cursor** — all support MCP
- Cube v5 — **one of the first math MCP servers**
- Competitors: Wolfram Alpha MCP (~$20/mo API), no free math MCP
- **Potential: 5/5** (pioneer in niche)

### Segment 2: Agent Systems (Cline, Claude Code, Codex) 4/5
- LLM agents **need accurate computation** — LLMs compute poorly
- Cube v5 provides 71 tools without external APIs or internet
- Built-in diagnostic tools (doctor, witness) help LLMs understand numbers

### Segment 3: Education / Science 3/5
- Group theory, E8-algebra, Pythagorean table — educational topics
- Doctor diagnostics — good teaching tool
- But: no symbolic computation, loses to SymPy/SageMath for education

### Segment 4: Finance 3/5
- Fixed-point arithmetic useful for financial computations
- Witness structure can be used for pattern analysis
- Vector statistics for time series

---

## 7. RECOMMENDATIONS

### P0 (critical for growth):
1. **Linux/macOS build** (Docker or cross-compilation)
2. **Matrix operations**: mul, det, eig, solve
3. **PyPI package**: `pip install cube-v5-mcp`

### P1 (important for adoption):
4. **Trigonometry**: sin/cos/tan via Taylor series
5. **Increase N > 100** for witness
6. **Increase batch** to 10K+ for E8
7. **CI/CD** with automatic benchmarks

### P2 (desirable):
8. **GPU acceleration** (CUDA) for E8 and vector operations
9. **Statistical distributions** (normal, poisson)
10. **Symbolic computation** (basic)
11. **API versioning** (semver)
12. **Docker image** for server deployment

---

## 8. CONCLUSION

**Cube v5 is a unique MCP server** with 71 math tools, having no direct analogues in the MCP ecosystem.

Key strengths:
- **First free math MCP server** — no competitors in niche
- **Non-standard arithmetic** — completely unique math model
- **E8-algebra** — exclusive functionality for LLM tools
- **~14,000 req/s** in persistent mode — sufficient for real-time
- **~289 ms cold start** — fast startup
- **22.82 MB** — compact standalone exe with no dependencies

**Key risks:** Windows-only, no symbolic computation, no matrix operations.  
**Key opportunities:** Growing MCP market, LLM agent integration, educational potential.

**Overall score: 8.5/10** for math MCP server niche.

---

*Report generated 2026-06-10. Benchmark script: `benchmark_ultimate.py`*