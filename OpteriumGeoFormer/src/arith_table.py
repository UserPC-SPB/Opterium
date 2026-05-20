"""
arith_table.py  —  PtTable: pre-computed address table (zero arithmetic at query time)

Builds a 2D lookup table (list-of-lists) for all (x,y) → {S, D, P}.
All operations: O(1) table read, zero FP, zero arithmetic at query time.

Extends to:
  - dot(a_list, b_list)   — vector dot product (all lookups)
  - matmul(A, B)          — matrix multiply    (all lookups)
  - pow10, isqrt, product — single-value lookups

Cached to pickle for fast reload.
"""

import os, pickle, sys, math
from math import gcd
from typing import Dict, List, Tuple, Sequence

CACHE_DIR = os.path.join(os.path.dirname(__file__), '.cache')
CACHE_FILE = os.path.join(CACHE_DIR, 'pt_table.pkl')
CACHE_VERSION = 6
MAX_COORD = 1024  # 1024×1024 ≈ 1M entries


class PtTable:
    def __init__(self, max_coord: int = MAX_COORD):
        self.max_coord = max_coord
        self._S: List[List[int]] = []
        self._D: List[List[int]] = []
        self._P: List[List[int]] = []
        self._pairs: Dict[int, List[Tuple[int, int]]] = {}
        self._sd_to_xy: Dict[Tuple[int, int], Tuple[int, int]] = {}
        self._isqrt: Dict[int, int] = {}
        self._square: Dict[int, int] = {}
        self._abs: Dict[int, int] = {}
        self._pow10: List[int] = []
        self._cache_hits = 0
        self._cache_misses = 0

        # Pure-lookup extensions: SD→P direct map + integer proximity weights
        self._SP: List[List[int]] = []   # _SP[S][D + offset] = P
        self._offset: int = max_coord     # offset for negative D indexing
        self._prox: List[int] = []        # _prox[dist] = int weight rank

        if self._try_load_cache():
            return

        # ── Build ──
        dim = max_coord + 1
        sp_dim = 2 * max_coord + 1  # S: 0..2048, D+offset: 0..2048
        for _ in range(dim):
            self._S.append([0] * dim)
            self._D.append([0] * dim)
            self._P.append([0] * dim)
        for _ in range(sp_dim):
            self._SP.append([0] * sp_dim)

        for x in range(dim):
            row_S = self._S[x]
            row_D = self._D[x]
            row_P = self._P[x]
            for y in range(dim):
                s = x + y
                d = x - y
                p = x * y
                row_S[y] = s
                row_D[y] = d
                row_P[y] = p
                self._SP[s][d + self._offset] = p
                self._sd_to_xy[(s, d)] = (x, y)
                if p not in self._pairs:
                    self._pairs[p] = []
                self._pairs[p].append((x, y))

        for i in range(dim):
            self._isqrt[i * i] = i
            self._square[i] = i * i

        for v in range(-max_coord, max_coord + 1):
            self._abs[v] = abs(v)

        p10 = 1
        for _ in range(11):
            self._pow10.append(p10)
            p10 *= 10

        # Integer proximity weight table: _prox[dist] = SCALE // (1 + dist)
        SCALE = 10000
        max_dist = 4 * max_coord  # max |ΔS| + |ΔD| = 4096
        self._prox = [SCALE // (1 + d) for d in range(max_dist + 1)]

        self._save_cache()

    def _try_load_cache(self) -> bool:
        if not os.path.exists(CACHE_FILE):
            return False
        try:
            with open(CACHE_FILE, 'rb') as f:
                data = pickle.load(f)
            if data.get('version', 0) != CACHE_VERSION:
                return False
            self._S = data['S']
            self._D = data['D']
            self._P = data['P']
            self._pairs = data['pairs']
            self._sd_to_xy = data.get('sd_to_xy', {})
            self._isqrt = data['isqrt']
            self._square = data['square']
            self._abs = data['abs']
            self._pow10 = data.get('pow10', [1, 10, 100])
            self._SP = data.get('SP', [])
            self._offset = data.get('offset', self.max_coord)
            self._prox = data.get('prox', [])
            return True
        except Exception:
            return False

    def _save_cache(self):
        os.makedirs(CACHE_DIR, exist_ok=True)
        with open(CACHE_FILE, 'wb') as f:
            pickle.dump({
                'version': CACHE_VERSION,
                'S': self._S, 'D': self._D, 'P': self._P,
                'pairs': self._pairs, 'sd_to_xy': self._sd_to_xy,
                'isqrt': self._isqrt, 'square': self._square, 'abs': self._abs,
                'pow10': self._pow10,
                'SP': self._SP, 'offset': self._offset, 'prox': self._prox,
            }, f, protocol=pickle.HIGHEST_PROTOCOL)

    def _in_range(self, a: int, b: int = None) -> bool:
        if not (0 <= a <= self.max_coord):
            return False
        if b is not None:
            if not (1 <= b <= self.max_coord):
                return False
        return True

    def _safe_get(self, table, a: int, b: int = None) -> int:
        if not self._in_range(a, b):
            return None
        try:
            if b is not None:
                return table[a][b]
            return table[a]
        except (IndexError, KeyError):
            return None

    def _product_scaled(self, a: int, b: int) -> int:
        """a * b через gcd-декомпозицию (lern1.txt §8,21):
           g = gcd(a,b) → seed = (a/g, b/g) → P = P_seed * g²
           Если seed вне таблицы — рекурсия (макс глубина 10)."""
        if a < 0 or b < 0:
            sa = -1 if a < 0 else 1
            sb = -1 if b < 0 else 1
            raw = self._product_scaled(abs(a), abs(b))
            return raw if sa == sb else -raw
        if a > b:
            return self._product_scaled(b, a)
        if a == 0 or b == 0:
            return 0
        if a <= self.max_coord and b <= self.max_coord:
            return self._P[a][b]
        g = gcd(a, b)
        if g <= 1:
            return a * b
        seed = self._product_scaled(a // g, b // g)
        return seed * g * g

    def S(self, x: int, y: int) -> int:
        """x + y via table.  Handles negative x, y via inversion.
           S(-x, y) = -D(x,y), S(x,-y) = D(x,y), S(-x,-y) = -S(x,y).
           Out-of-range fallback = x + y (всегда эквивалентно)."""
        if x >= 0 and y >= 0:
            if x <= self.max_coord and y <= self.max_coord:
                return self._S[x][y]
            return x + y
        a, b = abs(x), abs(y)
        if x < 0 and y >= 0:
            return -self._D[a][b] if a <= self.max_coord and b <= self.max_coord else x + y
        if x >= 0 and y < 0:
            return self._D[a][b] if a <= self.max_coord and b <= self.max_coord else x + y
        return -self._S[a][b] if a <= self.max_coord and b <= self.max_coord else x + y

    def D(self, x: int, y: int) -> int:
        """x - y via table.  Handles negatives via inversion.
           D(-x,y) = -S(x,y), D(x,-y) = S(x,y), D(-x,-y) = -D(x,y).
           Out-of-range fallback = x - y."""
        if x >= 0 and y >= 0:
            if x <= self.max_coord and y <= self.max_coord:
                return self._D[x][y]
            return x - y
        a, b = abs(x), abs(y)
        if x < 0 and y >= 0:
            return -self._S[a][b] if a <= self.max_coord and b <= self.max_coord else x - y
        if x >= 0 and y < 0:
            return self._S[a][b] if a <= self.max_coord and b <= self.max_coord else x - y
        return -self._D[a][b] if a <= self.max_coord and b <= self.max_coord else x - y

    def P(self, x: int, y: int) -> int:
        """x * y via table + gcd-scaling."""
        if x >= 0 and y >= 0:
            if x <= self.max_coord and y <= self.max_coord:
                return self._P[x][y]
            return self._product_scaled(x, y)
        a = -x if x < 0 else x
        b = -y if y < 0 else y
        p = self._product_scaled(a, b)
        return -p if (x < 0) != (y < 0) else p

    def lookup(self, x: int, y: int) -> Dict[str, int]:
        return {'S': self.S(x, y), 'D': self.D(x, y), 'P': self.P(x, y)}

    def has(self, x: int, y: int) -> bool:
        return 0 <= x <= self.max_coord and 0 <= y <= self.max_coord

    def isqrt(self, n: int) -> int:
        if n in self._isqrt:
            return self._isqrt[n]
        if n <= 0:
            return 0
        return math.isqrt(n)

    def square(self, n: int) -> int:
        if n >= 0:
            val = self._safe_get(self._square, n)
            return val if val is not None else n * n
        return n * n

    def abs(self, x: int) -> int:
        val = self._safe_get(self._abs, x)
        return val if val is not None else (x if x >= 0 else -x)

    def product(self, a: int, b: int) -> int:
        """a * b via table + gcd-scaling для out-of-range."""
        if a >= 0 and b >= 0:
            if a <= self.max_coord and b <= self.max_coord:
                return self._P[a][b]
            return self._product_scaled(a, b)
        sa = -1 if a < 0 else 1
        sb = -1 if b < 0 else 1
        raw = self._product_scaled(abs(a), abs(b))
        return raw if sa == sb else -raw

    def sum(self, a: int, b: int) -> int:
        """a + b via table (zero-arithmetic).  Handles negatives via S/D inversion."""
        if a >= 0 and b >= 0:
            val = self._safe_get(self._S, a, b)
            return val if val is not None else a + b
        return self.S(a, b)

    def diff(self, a: int, b: int) -> int:
        """a - b via table (zero-arithmetic).  Handles negatives via S/D inversion."""
        if a >= 0 and b >= 0:
            val = self._safe_get(self._D, a, b)
            return val if val is not None else a - b
        return self.D(a, b)

    def pow10(self, n: int) -> int:
        if 0 <= n < len(self._pow10):
            return self._pow10[n]
        if n < 0:
            return 1
        return 10 ** n

    # ── AI: dot product ────────────────────────────────
    def dot(self, a: Sequence[int], b: Sequence[int]) -> int:
        """Vector dot product via table lookups.
        
        Σ_k PT.product(a[k], b[k])
        Falls back to a[k]*b[k] for values outside table range.
        """
        s = 0
        for va, vb in zip(a, b):
            s += self.product(va, vb)
        return s

    # ── AI: matrix multiply ────────────────────────────
    def matmul(self, A: Sequence[Sequence[int]],
               B: Sequence[Sequence[int]]) -> List[List[int]]:
        """Matrix multiply via table lookups (zero arithmetic at query time).

        C[i][j] = Σ_k PT.product(A[i][k], B[k][j])

        All products via PT.product lookup (falls back to a*b for >MAX_COORD).
        All sums via integer addition.
        """
        if not A or not B:
            return []
        m = len(A)
        k = len(A[0])
        n = len(B[0]) if B else 0
        if k != len(B):
            raise ValueError(f"Shape mismatch: ({m}x{k}) x ({len(B)}x{n})")

        C = [[0] * n for _ in range(m)]
        for i in range(m):
            Ci = C[i]
            Ai = A[i]
            for p in range(k):
                ap = Ai[p]
                Bp = B[p]
                for j in range(n):
                    Ci[j] += self.product(ap, Bp[j])
        return C

    # ── AI: activation table generation ────────────────
    def activation_table(self, fn, lo: int, hi: int) -> Dict[int, int]:
        """Precompute activation fn over [lo, hi] as lookup dict.
        
        Args:
            fn: int → int activation function
            lo: lower bound (inclusive)
            hi: upper bound (inclusive)
        Returns:
            dict mapping input → output
        """
        return {v: fn(v) for v in range(lo, hi + 1)}

    def pairs_for_product(self, p: int) -> List[Tuple[int, int]]:
        return self._pairs.get(p, [])

    def from_sd(self, s: int, d: int) -> Tuple[int, int]:
        """Lookup (x,y) from (S,D) pair.  Reconstructs via formula if not in table."""
        val = self._sd_to_xy.get((s, d))
        if val is not None:
            return val
        x = (s + d) // 2
        y = (s - d) // 2
        return (x, y)

    # ── Pure-lookup: SD → P (zero arithmetic) ─────────────
    def p_from_sd(self, s: int, d: int) -> int:
        """P = x*y via _SP[S][D+offset] lookup. Zero arithmetic.
        Fallback to formula only if out of table range."""
        if 0 <= s < len(self._SP):
            idx = d + self._offset
            if 0 <= idx < len(self._SP[0]):
                return self._SP[s][idx]
        return (s * s - d * d) // 4  # formula fallback

    def p_from_xy(self, x: int, y: int) -> int:
        """P = x*y via _P[x][y] lookup. Zero arithmetic for in-range."""
        if 0 <= x <= self.max_coord and 0 <= y <= self.max_coord:
            return self._P[x][y]
        return self._product_scaled(x, y)

    # ── Pure-lookup: integer proximity weights (zero float) ──
    def proximity(self, dist: int) -> int:
        """Integer proximity weight. dist = |ΔS| + |ΔD|.
        Returns SCALE // (1 + dist). Zero float."""
        if 0 <= dist < len(self._prox):
            return self._prox[dist]
        return 0

    def int_weight(self, s1: int, d1: int, s2: int, d2: int) -> int:
        """Integer proximity weight between two (S,D) points.
        dist = |S1-S2| + |D1-D2|. Returns _prox[dist]."""
        ds = abs(s1 - s2)
        dd = abs(d1 - d2)
        return self.proximity(ds + dd)

    def min(self, a: int, b: int) -> int:
        return a if a < b else b

    def max(self, a: int, b: int) -> int:
        return a if a > b else b

    # ── Geometric conjugate (quaternion reflection) ───────
    def conj(self, x: int, y: int) -> Tuple[int, int]:
        """Quaternion conjugation in (S, D) space: conj(S, D) → (D, S).

        Per bootloader: quaternion q → conj(q) flips sign of imaginary components.
        In (S, D) coordinates: conj(S, D) = (D, S) → coordinates swapped.
        Product preserved: PT.product(*conj(x,y)) = PT.product(x, y).

        Additive inversion (sign handling) is already implemented in S/D/P
        methods via reflection formulas (S(-x,y) = -D(x,y), etc.)
        Full quaternion inversion q⁻¹ = conj(q)/|q|² requires rational
        division and is not representable with integer PtTable.
        """
        return (y, x)

    def summary(self) -> Dict:
        return {
            'max_coord': self.max_coord,
            'cached': os.path.exists(CACHE_FILE),
            'sd_size': len(self._S) * len(self._S[0]) if self._S else 0,
            'pairs': len(self._pairs),
            'isqrt_entries': len(self._isqrt),
        }


def _build_global():
    pt = PtTable(MAX_COORD)
    return pt


PT = _build_global()


def selftest():
    assert PT.S(4, 3) == 7
    assert PT.D(4, 3) == 1
    assert PT.P(4, 3) == 12
    assert PT.isqrt(144) == 12
    assert PT.product(6, 7) == 42
    assert PT.sum(6, 7) == 13
    assert PT.diff(6, 7) == -1
    assert PT.abs(-5) == 5
    assert PT.from_sd(7, 1) == (4, 3)
    assert PT.has(0, 1)
    assert not PT.has(9999, 9999)

    # _SP table: SD → P direct lookup
    assert PT.p_from_sd(7, 1) == 12     # Pt(4,3): S=7, D=1
    assert PT.p_from_sd(24, 0) == 144   # Pt(12,12): S=24, D=0
    assert PT.p_from_sd(8, -2) == 15    # Pt(3,5): S=8, D=-2
    assert PT.p_from_xy(4, 3) == 12
    assert PT.p_from_xy(1024, 1024) == 1048576

    # _prox table: integer proximity weights
    assert PT.proximity(0) == 10000
    assert PT.proximity(1) == 5000
    assert PT.proximity(100) == 99
    assert PT.int_weight(10, 5, 10, 5) == 10000  # same point

    # Verify _SP matches formula for 20 random points
    import random
    random.seed(42)
    for _ in range(20):
        x, y = random.randint(1, 1000), random.randint(1, 1000)
        s, d = x + y, x - y
        assert PT.p_from_sd(s, d) == x * y, f"_SP mismatch at ({x},{y})"

    print(f"  ✅ PtTable: S(4,3)=7 D(4,3)=1 P(4,3)=12")
    print(f"  ✅ PtTable: _SP table OK (20 random points verified)")
    print(f"  ✅ PtTable: _prox table OK (proximity[0]=10000)")
    print(f"  ✅ PtTable: {PT.summary()}")
    print("  PtTable: all tests pass")


if __name__ == '__main__':
    selftest()
