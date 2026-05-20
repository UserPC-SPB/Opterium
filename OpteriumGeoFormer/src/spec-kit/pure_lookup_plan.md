# DETAIL PLAN — Pure Lookup Engine
# Opterium GeoFormer → Zero Arithmetic Hot Paths
# 2026-05-19

## OVERVIEW

9 phases, 37 tasks. Each phase has explicit acceptance criteria.
Stop on failure. Run full test suite after each phase.

Critical path: Phase 1 → Phase 2 → {3,4,5,6} → Phase 7 → Phase 8 → Phase 9

---

## PHASE 1: PT._SP Table (arith_table.py)

### Task 1.1: Add _SP array
**File:** `src/arith_table.py`, line ~36 (after `self._pow10 = []`)
**Add:**
```python
self._SP: List[List[int]] = []      # _SP[S][D+offset] = P
self._offset = max_coord             # offset for negative D indexing
self._prox: List[int] = []           # _prox[dist] = int weight
```

### Task 1.2: Populate _SP in build loop
**File:** `src/arith_table.py`, line ~54-64 (the build loop)
**Modify:** After `row_P[y] = p`, add:
```python
# _SP[S][D+offset] = P
self._SP[s][d + self._offset] = p
```
**Also:** Initialize _SP array before the loop:
```python
sp_dim = 2 * max_coord + 1  # 0..2048 for S, D ranges from -1024..1024
for _ in range(sp_dim):
    self._SP.append([0] * sp_dim)
```

### Task 1.3: Add p_from_sd method
**File:** `src/arith_table.py`, after `from_sd` method (~line 311)
**Add:**
```python
def p_from_sd(self, s: int, d: int) -> int:
    """P = x*y via _SP lookup from (S, D) coordinates.
    Zero arithmetic: direct table read.
    Fallback to formula only if out of range."""
    if 0 <= s <= self.max_coord * 2:
        idx = d + self._offset
        if 0 <= idx < len(self._SP[0]):
            return self._SP[s][idx]
    # Fallback: formula (should not happen in normal use)
    return (s * s - d * d) // 4
```

### Task 1.4: Add p_from_xy method
**File:** `src/arith_table.py`, after p_from_sd
**Add:**
```python
def p_from_xy(self, x: int, y: int) -> int:
    """P = x*y via _P lookup. Zero arithmetic for in-range."""
    if 0 <= x <= self.max_coord and 0 <= y <= self.max_coord:
        return self._P[x][y]
    return self._product_scaled(x, y)
```

### Task 1.5: Update cache
**File:** `src/arith_table.py`, line ~21 (CACHE_VERSION)
**Change:** `CACHE_VERSION = 5` → `CACHE_VERSION = 6`

**File:** `src/arith_table.py`, `_save_cache` method (~line 101)
**Add to pickle dict:**
```python
'SP': self._SP,
'offset': self._offset,
```

**File:** `src/arith_table.py`, `_try_load_cache` method (~line 80)
**Add after loading existing keys:**
```python
self._SP = data['SP']
self._offset = data.get('offset', self.max_coord)
```

### Task 1.6: Selftest
**File:** `src/arith_table.py`, `selftest()` function (~line 352)
**Add:**
```python
# _SP table tests
assert PT.p_from_sd(7, 1) == 12    # Pt(4,3): S=7, D=1
assert PT.p_from_sd(24, 0) == 144  # Pt(12,12): S=24, D=0
assert PT.p_from_sd(2, -8) == -15  # Pt(-3,5): S=2, D=-8
assert PT.p_from_xy(4, 3) == 12
assert PT.p_from_xy(1024, 1024) == 1048576

# Verify _SP matches formula for 10 random points
import random
random.seed(42)
for _ in range(10):
    x, y = random.randint(1, 1000), random.randint(1, 1000)
    s, d = x + y, x - y
    assert PT.p_from_sd(s, d) == x * y, f"Mismatch at ({x},{y})"
print("  PtTable: _SP table OK")
```

---

## PHASE 2: PT._PROX Integer Proximity Table (arith_table.py)

### Task 2.1: Add _PROX array
**File:** `src/arith_table.py`, in __init__ (same place as Task 1.1)
Already added in Task 1.1: `self._prox: List[int] = []`

**In build section, after _SP initialization:**
```python
# Proximity weight table: _prox[dist] = SCALE // (1 + dist)
SCALE = 10000
max_dist = 4 * max_coord  # max |ΔS| + |ΔD| = 2*1024 + 2*1024 = 4096
self._prox = [SCALE // (1 + d) for d in range(max_dist + 1)]
```

### Task 2.2: Add proximity method
**File:** `src/arith_table.py`, after p_from_xy
**Add:**
```python
def proximity(self, dist: int) -> int:
    """Integer proximity weight. dist = |ΔS| + |ΔD|.
    Returns SCALE // (1 + dist). Zero float."""
    if 0 <= dist < len(self._prox):
        return self._prox[dist]
    return 0
```

### Task 2.3: Add int_weight method
**File:** `src/arith_table.py`, after proximity
**Add:**
```python
def int_weight(self, s1: int, d1: int, s2: int, d2: int) -> int:
    """Integer proximity weight between two (S,D) points.
    dist = |S1-S2| + |D1-D2|. Returns _prox[dist]."""
    ds = self.abs(s1 - s2) if abs(s1 - s2) <= self.max_coord else abs(s1 - s2)
    dd = self.abs(d1 - d2) if abs(d1 - d2) <= self.max_coord else abs(d1 - d2)
    return self.proximity(ds + dd)
```

### Task 2.4: Update cache
**File:** `src/arith_table.py`, `_save_cache`
**Add:** `'prox': self._prox`

**File:** `src/arith_table.py`, `_try_load_cache`
**Add:** `self._prox = data.get('prox', [])`

---

## PHASE 3: sd_matmul Pure Lookup (spec-kit/methods/sd_matmul.py)

### Task 3.1: Replace formula with lookup
**File:** `src/spec-kit/methods/sd_matmul.py`, lines 21-29
**Replace entire sd_product function:**
```python
def sd_product(S1: int, D1: int, S2: int, D2: int) -> int:
    """Compute P1 * P2 via pure table lookup. Zero arithmetic."""
    P1 = PT.p_from_sd(S1, D1)   # lookup, not formula
    P2 = PT.p_from_sd(S2, D2)   # lookup, not formula
    return PT.product(P1, P2)    # lookup with gcd-scaling fallback
```

### Task 3.2: Update imports
**File:** `src/spec-kit/methods/sd_matmul.py`, top of file
**Add:** `from arith_table import PT`

### Task 3.3: Verify no Pt in inner loop
**File:** `src/spec-kit/methods/sd_matmul.py`, sd_matmul function
The inner loop (lines 46-54) already uses int accumulation. No change needed.
The final `C_pt = [[Pt(v, 1) for v in row] for row in C]` is output-only, acceptable.

### Task 3.4: Run tests
```bash
cd src/spec-kit
python tests/test_correctness.py
```

---

## PHASE 4: geo_resonant Zero Float (spec-kit/methods/geo_resonant.py)

### Task 4.1: Replace float weight
**File:** `src/spec-kit/methods/geo_resonant.py`, line 87
**Replace:**
```python
# OLD:
weight = 1.0 / (1.0 + dist)
# NEW:
weight = PT.proximity(dist)
```

### Task 4.2: Replace float accumulation
**File:** `src/spec-kit/methods/geo_resonant.py`, lines 81-92
**Replace:**
```python
# OLD:
w_total = 0
p_weighted = 0
for nb in neighbors:
    if nb['id'] == i: continue
    dist = abs(nb['S'] - pt.S) + abs(nb['D'] - pt.D)
    weight = 1.0 / (1.0 + dist)
    w_total += weight
    p_weighted += weight * nb['P']
if w_total > 0:
    context = int(p_weighted / w_total)
else:
    context = pt.P

# NEW:
w_total = 0
p_weighted = 0
for nb in neighbors:
    if nb['id'] == i: continue
    dist = abs(nb['S'] - pt.S) + abs(nb['D'] - pt.D)
    weight = PT.proximity(dist)
    w_total += weight
    p_weighted += weight * nb['P']
if w_total > 0:
    context = p_weighted // w_total
else:
    context = pt.P
```

### Task 4.3: Replace math.isqrt
**File:** `src/spec-kit/methods/geo_resonant.py`, line 98
**Replace:**
```python
# OLD:
new_x = int(math.isqrt(new_P)) if new_P > 0 else 0
# NEW:
new_x = PT.isqrt(new_P) if new_P > 0 else 0
```

### Task 4.4: Remove math import
**File:** `src/spec-kit/methods/geo_resonant.py`, line 19
**Remove:** `import math` (no longer needed)

### Task 4.5: Run selftest
```bash
cd src/spec-kit/methods
python -c "from geo_resonant import *; print('OK')"
```

---

## PHASE 5: hashgrid Zero Float (hashgrid.py)

### Task 5.1: Replace geometric_weight
**File:** `src/hashgrid.py`, lines 97-104
**Replace:**
```python
# OLD:
def geometric_weight(S1: int, D1: int, S2: int, D2: int, eps: float = 1.0) -> float:
    return 1.0 / (eps + abs(S1 - S2) + abs(D1 - D2))

# NEW:
def geometric_weight(S1: int, D1: int, S2: int, D2: int, eps: int = 0) -> int:
    """Integer proximity weight. eps is additional distance penalty."""
    dist = abs(S1 - S2) + abs(D1 - D2) + eps
    return PT.proximity(dist)
```

### Task 5.2: Replace float accumulation in geometric_attention
**File:** `src/hashgrid.py`, lines 150-158
**Replace:**
```python
# OLD:
w_total = 0.0
p_weighted = 0.0
for nb in neighbors:
    w = geometric_weight(S_q, D_q, nb['S'], nb['D'], eps=eps)
    w_total += w
    p_weighted += w * nb['P']
context = int(p_weighted / w_total) if w_total > 0 else P_q

# NEW:
w_total = 0
p_weighted = 0
for nb in neighbors:
    w = geometric_weight(S_q, D_q, nb['S'], nb['D'])
    w_total += w
    p_weighted += w * nb['P']
context = p_weighted // w_total if w_total > 0 else P_q
```

### Task 5.3: Replace math.isqrt
**File:** `src/hashgrid.py`, line 163
**Replace:**
```python
# OLD:
out_x = PT.isqrt(mixed) if mixed > 0 else 0
# Already uses PT.isqrt — no change needed
```

### Task 5.4: Update selftest
**File:** `src/hashgrid.py`, selftest, line 201-203
**Replace weight symmetry test:**
```python
# OLD:
w1 = geometric_weight(10, 5, 20, 15)
w2 = geometric_weight(20, 15, 10, 5)
assert abs(w1 - w2) < 1e-15

# NEW:
w1 = geometric_weight(10, 5, 20, 15)
w2 = geometric_weight(20, 15, 10, 5)
assert w1 == w2  # integer equality
assert isinstance(w1, int)
```

---

## PHASE 6: pt_naive Zero Allocation (spec-kit/methods/pt_naive.py)

### Task 6.1: Rewrite pt_naive
**File:** `src/spec-kit/methods/pt_naive.py`, lines 14-42
**Replace pt_naive function:**
```python
def pt_naive(A, B):
    """Geometric matrix multiply via pure lookup. Zero Pt in loop."""
    A_pt = to_pt_matrix(A)
    B_pt = to_pt_matrix(B)
    m, k, n = validate_shape(A_pt, B_pt)

    C = [[0 for _ in range(n)] for _ in range(m)]

    for i in range(m):
        Ai = A_pt[i]
        Ci = C[i]
        for p in range(k):
            a_val = Ai[p].P
            Bp = B_pt[p]
            for j in range(n):
                Ci[j] += PT.product(a_val, Bp[j].P)

    C_pt = [[Pt(v, 1) for v in row] for row in C]
    return C_pt, HEALTH_OK
```

### Task 6.2: Verify pt_naive_fast
**File:** `src/spec-kit/methods/pt_naive.py`, lines 45-63
**Change line 60:** `Ci[j] += a_val * Bp[j].P` → `Ci[j] += PT.product(a_val, Bp[j].P)`

### Task 6.3: Run tests
```bash
python tests/test_correctness.py
```

---

## PHASE 7: geoformer Integration (geoformer.py)

### Task 7.1: Update GeometricBlock.forward
**File:** `src/geoformer.py`, lines 94-140
The geometric_attention call already returns int context after Phase 5.
No code change needed — just verify eps parameter is removed or set to int.

**Change line 88:** `eps: float = 1.0` → `eps: int = 0`
**Change line 101:** `eps=self.eps` → just use default (eps=0)

### Task 7.2: Replace float in HealthVector
**File:** `src/geoformer.py`, lines 125-127
**Replace:**
```python
# OLD:
ratio = assoc_check / denom if denom else 0.5
hv = HealthVector(E_assoc=min(ratio, 0.5))

# NEW:
ratio = (assoc_check * 10000) // denom if denom else 5000
hv = HealthVector(E_assoc=min(ratio / 10000, 0.5))
```

### Task 7.3: Run selftest
```bash
python -c "import sys; sys.path.insert(0,'src'); from geoformer import selftest; selftest()"
```

---

## PHASE 8: Purity Test Suite

### Task 8.1-8.5: Create test_purity.py
**File:** `src/spec-kit/tests/test_purity.py`
**Content:** AST-scan based tests (see spec for details)

### Task 8.6: Full cross-verify
Run all methods against torch baseline.

---

## PHASE 9: Full Suite + Benchmark

### Task 9.1-9.5: Run everything
```bash
cd tests
python run_all.py
python run_all.py --benchmark
python run_all.py --coverage
```

---

## EXECUTION ORDER

```
Phase 1 (arith_table.py: _SP)
    ↓
Phase 2 (arith_table.py: _PROX)
    ↓
┌── Phase 3 (sd_matmul) ──┐
├── Phase 4 (geo_resonant) ┤ ← can run in parallel
├── Phase 5 (hashgrid)     │
└── Phase 6 (pt_naive)     ┘
    ↓
Phase 7 (geoformer)
    ↓
Phase 8 (purity tests)
    ↓
Phase 9 (full suite)
```

## STOP CRITERIA

After EACH phase:
1. Run `python tests/run_all.py --quick`
2. If any test fails → STOP, fix, retry
3. Only proceed to next phase if all pass

## MEMORY IMPACT

| Table | Size | Notes |
|-------|------|-------|
| _S, _D, _P | 3 × 1025² × 4B ≈ 12 MB | Existing |
| _SP | 2049² × 4B ≈ 16 MB | New |
| _prox | 4097 × 4B ≈ 16 KB | New |
| **Total** | **~28 MB** | Acceptable |

## ROLLBACK PLAN

If any phase breaks backward compatibility:
1. All new methods are ADDITIVE (p_from_sd, proximity, int_weight)
2. Old methods (S, D, P, from_sd) unchanged
3. sd_matmul, geo_resonant, hashgrid can be reverted individually
4. Cache version bump ensures old cache is invalidated cleanly
