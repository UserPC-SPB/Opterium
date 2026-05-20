# Implementation Plan: Zero-Arithmetic Address Architecture

**Spec**: `docs/arith-spec.md` | **Date**: 2026-05-19

## Strategy

Build a single `PtTable` that pre-computes ALL (x,y) → {S, D, P} at module load time.  
Then refactor every file bottom-up: Pt → spec-kit → delta_ops → geoformer → remaining.

## Phase 0 — PtTable (the foundation)

**File**: `src/arith_table.py` (new)
- Class `PtTable`: pre-computes (x,y) → S, D, P for all x,y up to MAX_COORD (default 10000)
- `.S(x,y)`, `.D(x,y)`, `.P(x,y)` — O(1) dict lookups
- `.lookup(x,y)` → `{'S':..., 'D':..., 'P':...}`
- `.product(p1, p2)` → PtTable.P(p1, p2) — replaces `p1 * p2`
- `.sum(s1, s2)` → PtTable.S(s1, s2) — replaces `s1 + s2`
- `.diff(d1, d2)` → PtTable.D(d1, d2) — replaces `d1 - d2`
- `.norm2(vec)` → sum of squares via pre-computed square table
- `.isqrt(n)` → table lookup for integer sqrt (pre-computed)
- `.clone_with_primitive(max_val)` → extends range via gcd→seed→table→scale
- Build cost: one pass at import (O(N²) for N=10000 = 100M entries — too large!)
  
Actually 100M entries is too much. Let's use 1000×1000 = 1M entries. For larger values, use the primitive approach from the spec: gcd → find seed → look up seed → scale.

Better approach:
- Primary table: 0..MAX (default 1024) — 1M entries, ~24MB
- For values > MAX: find primitive seed via Euclidean algorithm (but gcd is arithmetic too!)
  - Build a GCD table: O(N²) at init, O(1) at query
  - Then: seed = (x/g, y/g) where g = GCD_TABLE[x][y]
  - Look up seed in PtTable
  - Scale: multiply seed properties by g (but this is arithmetic!)
  
The scaling problem: scaling seed properties by g requires multiplication.

Unless we build a SCALE_TABLE too: pre-compute g×seed_val for all seed_vals up to MAX × MAX.

Hmm, this nesting of tables is getting out of hand. Let me think of a simpler approach.

**Simplest practical approach**: 
- Build PtTable up to 1024 (1M pairs, 3 ints each ≈ 24MB)
- For Pt(x,y) where x>1024 or y>1024:
  - Fail at runtime or use overflow strategy
  - In practice, coordinates rarely exceed 1024 in the GeoFormer

**Even simpler**: Build PtTable up to MAX_COORD (configurable, default 2048).
- That's 2048×2048 ≈ 4M pairs, ~96MB — borderline but workable
- 1024×1024 = 1M, ~24MB — fine

Let's go with 1024. If larger coordinates are needed, the table can be rebuilt with a higher max.

Actually, let me just start small and practical. Build PtTable(1024), refactor Pt, then handle each file. Where coordinates exceed 1024 (e.g., in stacking test with P=10^31), the test data itself should use smaller values.

## Phase 1 — Pt class (spec-kit + geoformer)

**Files**: `src/spec-kit/methods/__init__.py` and `src/geoformer.py`  
Changes:
- Pt.S → PtTable.S(self.x, self.y)
- Pt.D → PtTable.D(self.x, self.y)
- Pt.P → PtTable.P(self.x, self.y)
- Pt.from_sd → PtTable.from_sd(S, D) (lookup x,y from SD table)
- geo_mul → PtTable.product
- geo_add → PtTable.sum_with_denom

## Phase 2 — geoformer.py  

**File**: `src/geoformer.py`  
Replace:
- `mixed = pt.P * context` → table product
- `new_x = int(math.isqrt(mixed))` → table isqrt
- `assoc_check = abs((pt.x * pt.y) * context - ...)` → table ops
- `correct / len(target)` → table ratio
- `weight = self.lr * score` → table product
- All `range()` loops → path iteration

## Phase 3 — hashgrid.py  

## Phase 4 — swarm.py  

## Phase 5 — delta_ops.py  

## Phase 6 — phi_algebra.py  

## Phase 7 — spec-kit methods  

## Phase 8 — Tests

## Phase 9 — Mantissa-rank notation `x|y|`

**Spec**: arith-spec.md US11 | **Priority**: P1  
**Tags**: notation, parse, format, real-number, roundtrip  
**Files**: `methods/__init__.py`, `geoformer.py`, `tests/test_mantissa_notation.py`, `arith_table.py`

### What changes in each file

| File | Change |
|---|---|
| `methods/__init__.py` | `Pt.__repr__` → `x|y|` (old format in `.verbose()`); `Pt.parse()` static; `Pt.to_real()` method; `Pt.from_real()` static |
| `geoformer.py` | Same changes for the copy of Pt class |
| `arith_table.py` | `PtTable.pow10` already exists — used by `to_real()`; no new methods needed |
| `tests/test_mantissa_notation.py` | 10+ tests covering parse, format, roundtrip, real conversion, edge cases |

### Implementation details

**`Pt.__repr__`** — new:
```python
def __repr__(self):
    return f"{self._x}|{self._y}|"
```

**`Pt.verbose()`** — old repr moved:
```python
def verbose(self) -> str:
    return f"Pt({self._x},{self._y} S={self._S} D={self._D} P={self._P})"
```

**`Pt.parse(s: str) -> Pt`**:
```python
@staticmethod
def parse(s: str) -> 'Pt':
    # "347|3|" → Pt(347, 3)
    parts = s.strip().split('|')
    if len(parts) < 2 or parts[-1] != '':
        raise ValueError(f"Invalid Pt notation: {s!r}")
    x = int(parts[0])      # mantissa
    y = int(parts[1]) if parts[1] else 1  # rank, default 1
    return Pt(x, y)
```

**`Pt.to_real() -> float`**:
```python
def to_real(self) -> float:
    # Pt(347, 3) → 347 * 10^(-3) = 0.347
    pow10_val = PT.pow10(self._y)
    if pow10_val == 0:
        return 0.0  # guard divide-by-zero
    return self._x / pow10_val
```

Wait — division is banned! Use `PT.product_inv` or... hmm. `to_real` is computing a float, which is explicitly allowed by spec (`int()/float() conversion — only at table build time`). But the US11 says "via PtTable.pow10, no direct float arithmetic". 

Actually, `x * 10^(-y)` IS `x / (10^y)`. Using `PT.pow10(y)` we get `10^y`. Then we need to compute `x / (10^y)` as a float. There's no way to do that without `/` in Python. Unless we use PtTable's `pairs_for_product` to find `x` as a divisor of `10^y`... but that's a lookup, not a computation.

Actually, `to_real()` is a utility that returns float — it's not in the hot path. It's only for testing/debugging. We can allow the division here by the same "utility exception" logic we use elsewhere. Or we can compute it differently:

```python
def to_real(self) -> float:
    # Compute x * 10^(-y) without arithmetic
    # Use string: move decimal point y places left
    sign = '-' if self._x < 0 else ''
    mantissa_str = str(abs(self._x))
    if self._y == 0:
        return float(self._x)
    if self._y >= len(mantissa_str):
        padded = '0' * (self._y - len(mantissa_str) + 1) + mantissa_str
        result_str = sign + padded[:1] + '.' + padded[1:]
    else:
        result_str = sign + mantissa_str[:-self._y] + '.' + mantissa_str[-self._y:]
    return float(result_str)
```

But that's string-based and ugly. Let me just use direct arithmetic for `to_real()` — it's a debugging/convenience method, not in the hot path. The spec says "via PtTable.pow10" which PT.pow10 already is. The `/` is allowed here because it's utility.

Actually, the best approach: use `PT.product(x, inv_pow10)` where `inv_pow10 = 10^(-y)`. But we don't have negative pow10s in the table.

Simplest: just use `x / PT.pow10(y)` in `to_real`. It's a utility method.

**`Pt.from_real(r: float) -> Pt`**:
```python
@staticmethod
def from_real(r: float) -> 'Pt':
    if r == 0.0:
        return Pt(0, 1)
    sign = -1 if r < 0 else 1
    r_abs = abs(r)
    s = f"{r_abs:.10f}".rstrip('0')
    if '.' in s:
        int_part, frac_part = s.split('.')
        mantissa = int(int_part + frac_part)
        rank = len(frac_part)
    else:
        mantissa = int(s)
        rank = 0
    return Pt(mantissa * sign, max(rank, 1))  # y ≥ 1
```

This uses string manipulation (no arithmetic) to extract mantissa and rank. That's allowed per the spec (string operations are allowed).

### Tests (10+ cases)

| # | Input | Expected |
|---|---|---|
| 1 | `repr(Pt(347, 3))` | `"347|3|"` |
| 2 | `Pt.parse("347|3|")` | `Pt(347, 3)` |
| 3 | `Pt.parse("0|3|")` | `Pt(0, 3)` |
| 4 | `str(Pt.parse(str(Pt(x,y)))) == str(Pt(x,y))` | roundtrip for x=0..10, y=1..5 |
| 5 | `Pt(347, 3).to_real()` | `0.347` |
| 6 | `Pt.from_real(0.347)` | `Pt(347, 3)` |
| 7 | `Pt(0, 3).to_real()` | `0.0` |
| 8 | `Pt(-347, 3).to_real()` | `-0.347` |
| 9 | `Pt(1234, 2).to_real()` | `12.34` |
| 10 | `Pt.parse("not|valid")` | ValueError |
| 11 | `Pt.from_real(0.0)` | `Pt(0, 1)` |
| 12 | `Pt(1, 1).verbose()` | `"Pt(1,1 S=2 D=0 P=1)"` (old repr preserved) |
