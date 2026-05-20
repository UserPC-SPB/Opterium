# Feature Specification: Geometric Matrix Multiplication

**Feature Branch**: `geo-matmul`  
**Created**: 2026-05-18  
**Status**: Draft  
**Input**: Replace all matrix multiplication (torch.matmul, numpy.dot) with geometric Pt-table operations — no FP32, no GPU, no Nvidia.

## User Stories

### User Story 1 — Naive Delta MatMul (Priority: P1)
As a developer, I want to multiply two matrices using Δ-operators (DELTA_MUL, DELTA_ADD) on Pt values so that the result is geometrically exact and verifiable via HealthVector.

**Why this priority**: Baseline correctness — proves that any matrix multiply CAN be done with Δ-ops. All other methods derive from this.

**Independent Test**: `C = pt_matmul(A, B)` matches `torch.matmul(A, B)` within integer precision for 5 random 4×4 matrices.

**Acceptance Scenarios**:
1. **Given** two 4×4 integer matrices A and B, **When** `pt_matmul(A, B)`, **Then** result matches torch.matmul elementwise
2. **Given** a 3×5 × 5×2 multiply, **When** shapes mismatch, **Then** ValueError raised
3. **Given** any multiply, **When** completed, **Then** HealthVector is returned with all channels < 0.35

---

### User Story 2 — PyTable Lookup MatMul (Priority: P1)
As a developer, I want to multiply matrices using direct PyTable lookups (P = (S²−D²)//4) instead of float arithmetic, so that the cost is one read per element rather than one FP32 multiply.

**Why this priority**: Core insight — the product IS the relation, not a computation. PyTable lookup replaces FP32 MUL with integer formula.

**Independent Test**: `C = pytable_matmul(A, B)` matches `pt_matmul(A, B)` for 5 random 4×4 matrices.

**Acceptance Scenarios**:
1. **Given** two integer matrices, **When** `pytable_matmul(A, B)`, **Then** result matches naive Δ-operator version
2. **Given** PyTable is available, **When** lookup performed, **Then** P = (S²−D²)//4 always holds

---

### User Story 3 — S-D Composition MatMul (Priority: P2)
As a developer, I want to multiply matrices entirely in (S, D) coordinate space — no intermediate P lookups — so that the multiply reduces to S-D composition rules.

**Why this priority**: Eliminates the final FP32 dependency. All arithmetic is integer add/subtract on (S, D) pairs. No float product, no float sum.

**Independent Test**: `C = sd_matmul(A_sd, B_sd)` matches `pt_matmul(A, B)` for 5 random 4×4 matrices.

**Acceptance Scenarios**:
1. **Given** two matrices in (S, D) form, **When** `sd_matmul(sd_A, sd_B)`, **Then** result matches naive version
2. **Given** a multiply with zero values, **When** processed, **Then** S and D correctly represent zero product

---

### User Story 4 — Hashgrid Resonant MatMul (Priority: P3)
As a developer, I want to compute attention-style interaction without ANY explicit matrix multiply — using hashgrid neighbor lookup and Pt3 projection — so that complexity drops from O(n²·d) to O(n·k).

**Why this priority**: Maximum performance. GeoFormer architecture replaces transformer MM entirely. No QK^T, no FFN weights, no backprop.

**Independent Test**: For 100 tokens, `geo_resonant(embeddings)` produces same-dimension output with O(n·k) operations.

**Acceptance Scenarios**:
1. **Given** n token embeddings as Pt values, **When** `geo_resonant(tokens)`, **Then** output has same shape
2. **Given** k neighbors per bucket, **When** hashgrid lookup, **Then** each token finds ≤ k neighbors in O(1)
3. **Given** no explicit attention, **When** full forward pass, **Then** no torch.matmul is called

---

### User Story 5 — Benchmark & Compare (Priority: P2)
As a developer, I want to benchmark all methods against torch.matmul on wall-time and memory, so that I can choose the optimal method for production.

**Why this priority**: Proves the geometric approach is not just conceptually correct but practically faster on CPU.

**Independent Test**: `run_benchmark()` produces a table of method × matrix_size × wall_time × memory.

**Acceptance Scenarios**:
1. **Given** all methods implemented, **When** `run_benchmark()`, **Then** CSV with results is produced
2. **Given** matrix sizes [4, 16, 64, 256], **When** each size tested 5 times, **Then** mean and std reported
3. **Given** results, **When** hashgrid method for large n, **Then** wall-time grows as O(n·k) not O(n²)

---

## Requirements

### Functional Requirements

- **FR-001**: System MUST multiply any two conformable integer matrices using only geometric (Pt-based) operations
- **FR-002**: Each method MUST return a HealthVector alongside the result matrix
- **FR-003**: Methods MUST accept both Pt objects and plain Python ints as input values
- **FR-004**: All operations MUST use integer-only arithmetic (no float, no FP32)
- **FR-005**: Benchmark MUST measure wall-clock time and approximate memory usage
- **FR-006**: Hashgrid method MUST support arbitrary bucket size W
- **FR-007**: S-D method MUST work with zero values (S=1, D=1 → P=0)

### Key Entities

- **Pt**: Geometric point (S, D) where P = (S²−D²)//4. Canonical representation of a value.
- **DeltaOp**: A geometric transformation mapping (state) → (state', HealthVector).
- **HealthVector**: 7-channel cognitive stability monitor returned by every Δ operation.
- **PyTable**: Precomputed lookup table mapping (S, D) → P. File: `PYTH_TABLE_1000.bin`.
- **HashGrid**: O(1) neighbor lookup structure over (S, D) space.

## Success Criteria

### Measurable Outcomes

- **SC-001**: All methods return mathematically correct results for integer matrices (matching torch.matmul)
- **SC-002**: PyTable method is ≥2× faster than naive Δ-operator method on CPU
- **SC-003**: S-D method uses no float operations (verified by function trace)
- **SC-004**: Hashgrid method achieves O(n·k) scaling: 10K tokens processed in <100ms on CPU
- **SC-005**: Best method selected, optimized, and calibrated: no HealthVector warnings on standard inputs

## Assumptions

- Matrices contain small-to-moderate integers (values ≤ 10⁶) — within PyTable 1000² range
- PyTable at `D:\gemma-4-geometric\dataset\PYTH_TABLE_1000.bin` is available for Method 2
- Python 3.10+ with `numpy` and optionally `torch` installed
- Baseline torch/numpy used only for comparison, not for geometric methods
