from dataclasses import dataclass
import sys, os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), '..', '..'))
from arith_table import PT
from decimal import Decimal, getcontext
getcontext().prec = 50


class Pt:
    """Geometric point with mantissa-rank notation x|y| = x / 10^y.

    Carries S = x+y, D = x-y, P = x*y for zero-arithmetic coordinate ops.
    to_real() → mantissa-rank value x / 10^y  (for from_real / parse).
    P = x*y       → geometric product         (for matrix multiply).
    """

    __slots__ = ('_x', '_y', '_S', '_D', '_P')

    def __init__(self, x: int, y: int = 1):
        self._x = int(x)
        self._y = max(int(y), 0)  # debt = 0 разрешён
        # PT.has теперь допускает y=0; gcd-scaling работает для >max_coord
        self._S = PT.S(self._x, self._y)
        self._D = PT.D(self._x, self._y)
        self._P = PT.P(self._x, self._y)

    @property
    def x(self) -> int: return self._x

    @property
    def y(self) -> int: return self._y

    @property
    def S(self) -> int: return self._S

    @property
    def D(self) -> int: return self._D

    @property
    def P(self) -> int: return self._P

    @staticmethod
    def from_int(v: int) -> 'Pt':
        """Integer → Pt for geometric use: P = v."""
        return Pt(v, 1)

    @staticmethod
    def from_sd(S: int, D: int) -> 'Pt':
        x, y = PT.from_sd(S, D)
        return Pt(x, y)

    @staticmethod
    def parse(s: str) -> 'Pt':
        "'347|3|' → Pt(347, 3).  Mantissa|rank| notation."
        stripped = s.strip()
        parts = stripped.split('|')
        if len(parts) < 2 or parts[-1] != '':
            raise ValueError(f"Invalid Pt notation: {s!r}")
        x = int(parts[0])
        y = int(parts[1]) if parts[1] else 1
        return Pt(x, y)

    @staticmethod
    def from_real(r: float) -> 'Pt':
        """Float → Pt(mantissa, rank) via Decimal."""
        return Pt.from_decimal(Decimal(repr(r)))

    def to_real(self) -> float:
        """Pt(347, 3) → 347 × 10^(-3) = 0.347.  Mantissa-rank value."""
        return self._x / PT.pow10(self._y)

    def verbose(self) -> str:
        return f"Pt({self._x},{self._y} S={self._S} D={self._D} P={self._P})"

    def __add__(self, other):
        """Component-wise Pt addition: Pt(x1+x2, y1+y2).  For geometric use."""
        if isinstance(other, Pt):
            return Pt(PT.sum(self._x, other._x), PT.sum(self._y, other._y))
        return NotImplemented

    def __mul__(self, other):
        """Component-wise Pt multiplication: Pt(x1*x2, y1*y2).  For geometric use."""
        if isinstance(other, Pt):
            return Pt(PT.product(self._x, other._x), PT.product(self._y, other._y))
        return NotImplemented

    def inv(self) -> 'Pt':
        """Inverse via mantissa-rank Decimal space (Doctor Lift-Solve-Project).

        Lift:    Pt(x,y) = x/10^y → Decimal(value)
        Solve:   invert in Decimal (1/value, exact precision)
        Project: Decimal → Pt(mantissa, rank)

        Falls back to (0,1) for degenerate zero.
        """
        if self._x == 0:
            return Pt(0, 1)
        d_val = Decimal(self._x) / Decimal(PT.pow10(self._y))
        d_inv = Decimal(1) / d_val
        return Pt.from_decimal(d_inv)

    @staticmethod
    def from_decimal(d: Decimal) -> 'Pt':
        """Decimal → Pt(mantissa, rank) с полной точностью.

        Использует d.as_tuple() → (sign, digits, exponent).
        Убирает хвостовые нули из digits, корректируя exponent.
        Примеры:
          Decimal('12.34')  → digits=(1,2,3,4), exp=-2 → Pt(1234, 2)
          Decimal('1e-30')  → digits=(1,), exp=-30    → Pt(1, 30)
          Decimal('100')    → digits=(1,0,0), exp=0   → Pt(100, 0)
        """
        if d.is_zero():
            return Pt(0, 0)
        sign = -1 if d.is_signed() else 1
        tup = d.as_tuple()
        digits = list(tup.digits)
        exp = tup.exponent
        while digits and digits[-1] == 0:
            digits.pop()
            exp += 1
        if not digits:
            return Pt(0, 0)
        mantissa_str = ''.join(str(d) for d in digits)
        mantissa = int(mantissa_str)
        if exp >= 0:
            mantissa *= 10 ** exp
            return Pt(mantissa * sign, 0)
        return Pt(mantissa * sign, -exp)

    def to_decimal(self) -> Decimal:
        """Pt → Decimal(mantissa/10^rank) with exact precision."""
        return Decimal(self._x) / Decimal(PT.pow10(self._y))

    def __repr__(self):
        return f"{self._x}|{self._y}|"

    def __eq__(self, other):
        if isinstance(other, Pt):
            return self._x == other._x and self._y == other._y
        return NotImplemented

    def __hash__(self):
        return hash((self._x, self._y))


# ── mantissa-rank arithmetic (for real-number Pt values) ────────

def rmul(a: Pt, b: Pt) -> Pt:
    """Mantissa-rank multiply: (a/10^ya) × (b/10^yb) = (a×b) / 10^(ya+yb)."""
    return Pt(PT.product(a.x, b.x), PT.sum(a.y, b.y))


def radd(a: Pt, b: Pt) -> Pt:
    """Mantissa-rank add: a/10^ya + b/10^yb with rank alignment."""
    if a.y <= b.y:
        shift = PT.pow10(b.y - a.y)
        nx = PT.sum(PT.product(a.x, shift), b.x)
        return Pt(nx, b.y)
    else:
        shift = PT.pow10(a.y - b.y)
        nx = PT.sum(a.x, PT.product(b.x, shift))
        return Pt(nx, a.y)


def rsub(a: Pt, b: Pt) -> Pt:
    """Mantissa-rank subtract: a/10^ya - b/10^yb с выравниванием рангов."""
    if a.y <= b.y:
        shift = PT.pow10(b.y - a.y)
        nx = PT.diff(PT.product(a.x, shift), b.x)
        return Pt(nx, b.y)
    else:
        shift = PT.pow10(a.y - b.y)
        nx = PT.diff(a.x, PT.product(b.x, shift))
        return Pt(nx, a.y)


def rdiv(a: Pt, b: Pt) -> Pt:
    """Mantissa-rank divide: a/10^ya ÷ b/10^yb = rmul(a, inv(b))."""
    return rmul(a, b.inv())


# ── legacy aliases (geometric interpretation: Pt as x/y, not x/10^y) ──
def geo_mul(a: Pt, b: Pt) -> Pt:
    """Geometric multiply via Pt coordinates."""
    return Pt(PT.product(a.x, b.x), PT.product(a.y, b.y))


def geo_add(a: Pt, b: Pt) -> Pt:
    """Geometric add via Pt coordinates."""
    t1 = PT.product(a.x, b.y)
    t2 = PT.product(b.x, a.y)
    nx = PT.sum(t1, t2)
    ny = PT.product(a.y, b.y)
    return Pt(nx, ny)


# ── matrix utilities ────────────────────────────────────────

def validate_shape(A, B):
    m = len(A)
    k = len(A[0]) if m else 0
    n = len(B[0]) if B else 0
    if k != len(B):
        raise ValueError(f"Shape mismatch: A:({m}×{k}) B:({len(B)}×{n})")
    return m, k, n


def to_pt_matrix(M):
    return [[Pt.from_int(v) if isinstance(v, int) else v for v in row] for row in M]


def sd_tuple_matrix(M):
    return [[(Pt.from_int(v).S, Pt.from_int(v).D) if isinstance(v, int)
             else (v.S, v.D) for v in row] for row in M]


def selftest():
    # 1. Pt basics
    p = Pt(3, 5)
    assert p.S == 8 and p.D == -2 and p.P == 15
    print("  methods: Pt(3,5) OK")

    # 2. Mantissa-rank parse/repr
    p2 = Pt.parse("347|3|")
    assert p2.x == 347 and p2.y == 3
    assert repr(p2) == "347|3|"
    assert abs(p2.to_real() - 0.347) < 1e-12
    print("  methods: parse/repr/roundtrip OK")

    # 3. from_real / to_real
    for v in [0.347, 2.34, -3.14159, 0.999999999999, 1e-6, 12345.6789]:
        p = Pt.from_real(v)
        back = p.to_real()
        assert abs(back - v) / max(1, abs(v)) < 1e-12, f"roundtrip {v} → {back}"
    print("  methods: from_real/to_real roundtrip OK")

    # 4. rmul / radd (mantissa-rank arithmetic)
    a = Pt.from_real(0.3)
    b = Pt.from_real(0.2)
    prod = rmul(a, b)
    assert abs(prod.to_real() - 0.06) < 1e-12
    summ = radd(a, b)
    assert abs(summ.to_real() - 0.5) < 1e-12
    print("  methods: rmul/radd OK")

    # 5. from_int (geometric: P = v)
    v = Pt.from_int(42)
    assert v.P == 42
    print("  methods: from_int(42).P = 42 OK")

    # 6. from_sd
    xy = Pt.from_sd(7, 1)
    assert xy.x == 4 and xy.y == 3
    print("  methods: from_sd OK")

    # 7. inv: Doctor lift-solve-project
    p = Pt(347, 3)       # 0.347
    pinv = p.inv()       # ≈ 2.881...
    diff = abs(pinv.to_decimal() - (Decimal(1) / Decimal('0.347')))
    assert diff < Decimal('1e-25'), f"inv mismatch: {pinv} = {pinv.to_decimal()}"
    print(f"  methods: inv({p}) = {pinv} ({pinv.to_decimal()}) OK")

    p2 = Pt(2, 1)        # 0.2
    pinv2 = p2.inv()     # should be 5
    diff2 = abs(pinv2.to_decimal() - Decimal(5))
    assert diff2 < Decimal('1e-25'), f"inv(0.2) mismatch: {pinv2.to_decimal()}"
    print(f"  methods: inv(2|1|) = {pinv2} ({pinv2.to_decimal()}) OK")

    p3 = Pt(10, 1)       # 10/10 = 1
    pinv3 = p3.inv()     # 1
    diff3 = abs(pinv3.to_decimal() - Decimal(1))
    assert diff3 < Decimal('1e-25'), f"inv(1) mismatch: {pinv3.to_decimal()}"
    print(f"  methods: inv(10|1|) = {pinv3} ({pinv3.to_decimal()}) OK")

    # 8. from_decimal / to_decimal roundtrip
    d = Decimal('3.1415926535897932384626433832795028841971')
    pp = Pt.from_decimal(d)
    back = pp.to_decimal()
    assert back == d, f"Decimal roundtrip fail: {back} != {d}"
    print("  methods: Decimal roundtrip OK")

    print("  methods: all tests pass")


if __name__ == '__main__':
    selftest()
