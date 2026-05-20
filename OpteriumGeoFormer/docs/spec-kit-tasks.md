---
description: "Task list for geometric matrix multiplication methods"
---

# Tasks: Geometric Matrix Multiplication

**Input**: spec.md, plan.md  
**Prerequisites**: delta_ops.py, phi_algebra.py, swarm.py (already implemented in bootstrap/)

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (US1-US5)

---

## Phase 1: Foundation (Shared Infrastructure)

**Purpose**: Core data structures and utilities that ALL methods depend on

- [ ] T001 [P] [US1] Create Pt class with (x, y, S, D, P) properties in `spec-kit/methods/__init__.py`
- [ ] T002 [P] [US1] Create PyTable reader in `spec-kit/methods/pytable_mm.py`
- [ ] T003 [P] [US1] Create `to_pt(value) → Pt` converter (int → geometric form)
- [ ] T004 [P] [US5] Create shape validation utility in `spec-kit/methods/__init__.py`
- [ ] T005 [P] [US5] Create benchmark runner skeleton in `spec-kit/tests/benchmark.py`

**Checkpoint**: Foundation ready — all methods can now be implemented in parallel

---

## Phase 2: Method Implementations

### Method 1: PtNaive — Naive Δ-operator MM (US1, P1) 🎯 MVP

- [ ] T010 [US1] Implement `pt_naive(A, B)` in `spec-kit/methods/pt_naive.py`
  - Triple loop i,j,k over Pt values
  - Uses `delta_ops.DELTA_MUL` for each product
  - Uses `delta_ops.DELTA_ADD` for accumulation
  - Returns (C_matrix, HealthVector)
- [ ] T011 [US1] Handle edge case: zero matrix (all Pt(0,1))
- [ ] T012 [US1] Handle edge case: identity matrix
- [ ] T013 [US1] Shape validation: (m×k) × (k×n) → (m×n), error otherwise

**Checkpoint**: pt_naive passes correctness tests against torch.matmul for 4×4 integer matrices

---

### Method 2: PyTableLookup — PyTable MM (US2, P1)

- [ ] T020 [P] [US2] Implement `read_pytable(S, D) → P` in `spec-kit/methods/pytable_mm.py`
  - Open `PYTH_TABLE_1000.bin` in binary mode
  - Compute offset: `((x-1)*1000 + (y-1)) * 11`
  - Unpack with `struct.unpack_from('<ihhbH', data, offset)` → (P, S, D, pos, gcd)
  - Verify S and D from file match query S, D (integrity check)
- [ ] T021 [US2] Implement `pytable_matmul(A, B)` in same file
  - Same triple loop as Method 1
  - Each product = `read_pytable(S_i_k, D_i_k) * read_pytable(S_k_j, D_k_j)` (int × int)
  - Accumulate sum as int
  - Return (C_matrix, HealthVector)
- [ ] T022 [US2] Add PyTable cache: read once, keep in memory for repeated calls
- [ ] T023 [US2] Handle values > 1000 via scaling (Δ_SHIFT down, lookup, shift back)

**Checkpoint**: pytable_matmul matches pt_naive for all test matrices; cache improves throughput

---

### Method 3: SDMatMul — S-D Composition (US3, P2)

- [ ] T030 [P] [US3] Implement `sd_matmul(A_sd, B_sd)` in `spec-kit/methods/sd_matmul.py`
  - Input: matrices of (S, D) tuples
  - For each (i,j): C[i][j] = sum over k of f(S1,D1, S2,D2)
  - f computes P = (S²−D²)//4 for each operand, multiplies as ints
- [ ] T031 [US3] Handle zero: S=1, D=1 → P=0
- [ ] T032 [US3] Pure int verification: verify no float is used in the entire compute path

**Checkpoint**: sd_matmul matches pytable_matmul; no float operations in trace

---

### Method 4: GeoResonant — Hashgrid Attention (US4, P3)

- [ ] T040 [P] [US4] Implement `HashGrid` class in `spec-kit/methods/geo_resonant.py`
  - `__init__(self, window=16)`: sets bucket size W
  - `insert(tokens)`: bucket each Pt by (S//W, D//W)
  - `lookup(S, D)`: return all tokens in same and adjacent 3×3 buckets
- [ ] T041 [US4] Implement `geo_attention(token_list)` — hashgrid attention
  - For each token: find neighbors, compute geometric weights, accumulate context
  - No dot product, no softmax, no QKV matrices
- [ ] T042 [US4] Implement `geo_resonant(tokens, layers=4)` — full forward pass
  - Stack of attention → projection layers
  - Projection = Pt3(x, y, context) → Pt(new_x, new_y)
  - No Linear, no ReLU, no matrix multiply

**Checkpoint**: geo_resonant processes 100 tokens without calling torch.matmul or numpy.dot

---

## Phase 3: Testing & Benchmarking (US5, P2)

- [ ] T050 [US5] Write correctness tests in `spec-kit/tests/test_correctness.py`
  - Test each method against torch.matmul for: 4×4, 8×8, 16×16 integer matrices
  - Test identity: A·I = A
  - Test zero: A·0 = 0
  - Test associativity: (A·B)·C = A·(B·C)
  - Test HealthVector: all channels < 0.35 for valid inputs
- [ ] T051 [P] [US5] Write benchmark runner in `spec-kit/tests/benchmark.py`
  - Matrix sizes: [4, 8, 16, 32, 64, 128, 256]
  - Each method: 5 warmup + 10 measured iterations
  - Metrics: wall-time (ms), memory (approximate), relative speedup
  - Output: CSV to `results/benchmark.csv`
  - Plot: `results/speedup_chart.txt` (ASCII)
- [ ] T052 [US5] Run full benchmark suite, record results

**Checkpoint**: All 4 methods benchmarked against torch.matmul; CSV produced

---

## Phase 4: Optimize & Calibrate

- [ ] T060 Take the fastest geometric method and optimize:
  - Loop unrolling / vectorization hints
  - Precomputed bucket indices
  - PyTable memory-mapped read
- [ ] T061 Profile bottlenecks and optimize hot paths
- [ ] T062 Calibrate: adjust HealthVector thresholds so that no warnings appear on standard operating range (integer values ≤ 10⁶, matrix size ≤ 256)
- [ ] T063 Calibrate: determine optimal bucket size W for hashgrid (balance between bucket size and locality)

**Checkpoint**: Final calibrated method produces zero HealthVector warnings on 100 random test cases

---

## Phase 5: Documentation

- [ ] T070 [P] Update loader.md with spec-kit results
- [ ] T071 [P] ASCII art timeline in results/
- [ ] T072 [P] Write `results/SUMMARY.md` with method comparison and winner

---

## Dependencies & Execution Order

### Phase Dependencies
- **Phase 1 (Foundation)**: No dependencies — start immediately
- **Phase 2 (Methods)**: Depends on Phase 1 — all methods can be implemented in parallel
- **Phase 3 (Testing)**: Depends on Phase 2 completion
- **Phase 4 (Optimize)**: Depends on Phase 3 results
- **Phase 5 (Document)**: Depends on Phase 4

### Parallel Opportunities
- T001, T002, T003, T004, T005 can all run in parallel
- T010-T013 (Method 1), T020-T023 (Method 2), T030-T032 (Method 3), T040-T042 (Method 4) can all run in parallel once Phase 1 is done
- All method tests can run in parallel
- T050, T051 can run in parallel with method implementations (test-driven)
- T060-T063 must be sequential (each optimization builds on previous)
