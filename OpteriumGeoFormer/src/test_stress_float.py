"""Стресс-тесты мантисса-ранг: float roundtrip, rmul/radd/rsub/rdiv, edge cases"""

import sys, os, time, random
sys.path.insert(0, os.path.dirname(__file__))
sys.path.insert(0, os.path.join(os.path.dirname(__file__), 'spec-kit'))

from arith_table import PT
from methods import Pt, rmul, radd, rsub, rdiv
from decimal import Decimal, getcontext
getcontext().prec = 100

passed = 0
failed = 0

def check(test_name: str, ok: bool, detail: str = ""):
    global passed, failed
    if ok:
        passed += 1
    else:
        failed += 1
        print(f"  ❌ {test_name}: {detail}")

# ═══════════════════════════════════════════════════════════
# 1. Float roundtrip (from_real → to_real)
# ═══════════════════════════════════════════════════════════
print("\n=== 1. Float roundtrip (from_real → to_real) ===")
test_vals = [
    0.0, 1.0, -1.0, 0.347, -0.347, 3.14159, -3.14,
    0.001, 0.0001, 1e-6, -1e-6, 1e-10, -1e-10,
    100.0, -100.0, 12345.6789, -12345.6789,
    0.5, 0.25, 0.125, 0.0625,  # двоичные дроби
    0.1, 0.2, 0.3, 0.4, 0.7, 0.8, 0.9,  # проблемные float
]
for v in test_vals:
    p = Pt.from_real(v)
    back = p.to_real()
    ok = abs(back - v) / max(1.0, abs(v)) < 1e-12
    check(f"roundtrip({v})", ok, f"→ Pt({p.x},{p.y}) → {back}")

# ═══════════════════════════════════════════════════════════
# 2. Decimal roundtrip (from_decimal → to_decimal)
# ═══════════════════════════════════════════════════════════
print("\n=== 2. Decimal roundtrip ===")
dec_vals = [
    Decimal('0'), Decimal('1'), Decimal('-1'),
    Decimal('3.14159265358979323846264338328'),
    Decimal('1e-30'), Decimal('1e30'),
    Decimal('-2.71828'), Decimal('0.001'),
    Decimal('100'), Decimal('-100'),
    Decimal('12.340'), Decimal('0.0010'),
]
for d in dec_vals:
    p = Pt.from_decimal(d)
    back = p.to_decimal()
    ok = back == d
    check(f"dec_roundtrip({d})", ok, f"→ Pt({p.x},{p.y}) → {back}")

# ═══════════════════════════════════════════════════════════
# 3. rmul stress
# ═══════════════════════════════════════════════════════════
print("\n=== 3. rmul stress ===")
pairs = [
    (0.3, 0.2, 0.06),
    (0.347, 0.5763, 0.1999761),
    (-0.3, 0.2, -0.06),
    (0.3, -0.2, -0.06),
    (-0.3, -0.2, 0.06),
    (100.0, 0.01, 1.0),
    (0.0, 5.0, 0.0),
    (1e-6, 1e6, 1.0),
    (0.125, 8.0, 1.0),
    (3.14159, 2.0, 6.28318),
]
for va, vb, expected in pairs:
    a, b = Pt.from_real(va), Pt.from_real(vb)
    c = rmul(a, b)
    ok = abs(c.to_real() - expected) / max(1e-10, abs(expected)) < 1e-6
    check(f"rmul({va}×{vb})={expected}", ok,
          f"got Pt({c.x},{c.y})={c.to_real()}")

# ═══════════════════════════════════════════════════════════
# 4. radd stress
# ═══════════════════════════════════════════════════════════
print("\n=== 4. radd stress ===")
add_pairs = [
    (0.3, 0.2, 0.5),
    (0.3, 0.05, 0.35),
    (-0.3, 0.05, -0.25),
    (0.05, -0.3, -0.25),
    (-0.3, -0.1, -0.4),
    (100.0, 0.0, 100.0),
    (0.001, 0.0001, 0.0011),
    (1e-6, 1e-6, 2e-6),
    (0.125, 0.875, 1.0),
    (-0.5, 0.5, 0.0),
]
for va, vb, expected in add_pairs:
    a, b = Pt.from_real(va), Pt.from_real(vb)
    c = radd(a, b)
    ok = abs(c.to_real() - expected) / max(1e-10, abs(expected)) < 1e-6
    check(f"radd({va}+{vb})={expected}", ok,
          f"got Pt({c.x},{c.y})={c.to_real()}")

# ═══════════════════════════════════════════════════════════
# 5. rsub stress
# ═══════════════════════════════════════════════════════════
print("\n=== 5. rsub stress ===")
sub_pairs = [
    (0.5, 0.03, 0.47),
    (0.03, 0.5, -0.47),
    (-0.3, -0.1, -0.2),
    (-0.1, -0.3, 0.2),
    (0.05, -0.3, 0.35),
    (-0.3, 0.05, -0.35),
    (1.0, 0.5, 0.5),
    (0.001, 0.0001, 0.0009),
    (0.0, 5.0, -5.0),
    (3.14, 3.14, 0.0),
]
for va, vb, expected in sub_pairs:
    a, b = Pt.from_real(va), Pt.from_real(vb)
    c = rsub(a, b)
    ok = abs(c.to_real() - expected) / max(1e-10, abs(expected)) < 1e-6
    check(f"rsub({va}-{vb})={expected}", ok,
          f"got Pt({c.x},{c.y})={c.to_real()}")

# ═══════════════════════════════════════════════════════════
# 6. rdiv stress
# ═══════════════════════════════════════════════════════════
print("\n=== 6. rdiv stress ===")
div_pairs = [
    (0.3, 0.2, 1.5),
    (0.2, 0.3, 0.6666666666666666),
    (-0.3, 0.2, -1.5),
    (0.3, -0.2, -1.5),
    (1.0, 0.5, 2.0),
    (0.5, 2.0, 0.25),
    (3.0, 2.0, 1.5),
    (1.0, 3.0, 0.3333333333333333),
    (100.0, 0.01, 10000.0),
    (0.001, 0.0001, 10.0),
]
for va, vb, expected in div_pairs:
    a, b = Pt.from_real(va), Pt.from_real(vb)
    c = rdiv(a, b)
    ok = abs(c.to_real() - expected) / max(1e-10, abs(expected)) < 1e-6
    check(f"rdiv({va}÷{vb})={expected}", ok,
          f"got Pt({c.x},{c.y})={c.to_real()}")

# ═══════════════════════════════════════════════════════════
# 7. inv stress  
# ═══════════════════════════════════════════════════════════
print("\n=== 7. inv stress ===")
for m, r in [(347, 3), (2, 1), (10, 1), (1, 1), (1, 5), (-2, 1)]:
    p = Pt(m, r)
    pinv = p.inv()
    prod = rmul(p, pinv)
    ok = abs(prod.to_real() - 1.0) < 1e-25
    check(f"inv({m}|{r}|) → {pinv}, prod={prod.to_real()}", ok,
          f"prod={prod.to_real()}")

# ═══════════════════════════════════════════════════════════
# 8. Chained float operations (real-world pipeline)
# ═══════════════════════════════════════════════════════════
print("\n=== 8. Chained operations ===")
# (0.3 + 0.2) * (0.5 - 0.1) / 0.2 = 0.5 * 0.4 / 0.2 = 1.0
a = Pt.from_real(0.3); b = Pt.from_real(0.2)
c = Pt.from_real(0.5); d = Pt.from_real(0.1)
e = Pt.from_real(0.2)
step1 = radd(a, b)       # 0.5
step2 = rsub(c, d)       # 0.4
step3 = rmul(step1, step2)  # 0.2
result = rdiv(step3, e)  # 1.0
ok = abs(result.to_real() - 1.0) < 1e-12
check("chain (0.3+0.2)*(0.5-0.1)/0.2=1.0", ok, f"got {result.to_real()}")

# ═══════════════════════════════════════════════════════════
# 9. Random stress
# ═══════════════════════════════════════════════════════════
print("\n=== 9. Random float operations ===")
random.seed(42)
n_rand = 500
rand_ok = 0
for _ in range(n_rand):
    va = random.uniform(-1000, 1000)
    vb = random.uniform(-1000, 1000)
    a, b = Pt.from_real(va), Pt.from_real(vb)
    # rmul
    c = rmul(a, b)
    if abs(c.to_real() - va*vb) / max(1e-10, abs(va*vb)) < 1e-6:
        rand_ok += 1
check(f"random rmul {n_rand} pairs", rand_ok == n_rand,
      f"{rand_ok}/{n_rand} passed")

# ═══════════════════════════════════════════════════════════
print(f"\n{'='*50}")
print(f"ИТОГ: {passed}/{passed+failed} passed, {failed} FAILED")
assert failed == 0, f"{failed} failures!"
