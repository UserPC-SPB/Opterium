"""Стресс-тест целостности: Pt → float → Pt → matmul → float → Pt (многошаг)"""

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

print("=" * 60)
print("  СТРЕСС-ТЕСТ ЦЕЛОСТНОСТИ")
print("=" * 60)

# ═══════════════════════════════════════════════════════════
# 1. Pt → float → Pt roundtrip (замкнутый цикл)
# ═══════════════════════════════════════════════════════════
print("\n=== 1. Pt → float → Pt (замкнутый цикл) ===")
test_pts = [
    Pt(347, 3), Pt(0, 0), Pt(1, 1), Pt(100, 0),
    Pt(-347, 3), Pt(123456789, 4), Pt(1, 10),
]
for p in test_pts:
    val = p.to_real()
    p2 = Pt.from_real(val)
    ok = p2.x == p.x and p2.y == p.y
    check(f"Pt({p.x},{p.y}) → {val} → Pt({p2.x},{p2.y})", ok)

# ═══════════════════════════════════════════════════════════
# 2. Float → Pt → matmul → float → Pt
# ═══════════════════════════════════════════════════════════
print("\n=== 2. float → Pt → matmul → float → Pt ===")
def matmul_pt(A, B):
    m, k, n = len(A), len(A[0]), len(B[0])
    C = [[Pt.from_real(0.0) for _ in range(n)] for _ in range(m)]
    for i in range(m):
        for p in range(k):
            bp = B[p]
            for j in range(n):
                C[i][j] = radd(C[i][j], rmul(A[i][p], bp[j]))
    return C

def matmul_ref(A, B):
    m, k, n = len(A), len(A[0]), len(B[0])
    return [[sum(A[i][p] * B[p][j] for p in range(k)) for j in range(n)] for i in range(m)]

# 3×3 float матрица
A_vals = [[0.1, 0.2, 0.3], [0.4, 0.5, 0.6], [0.7, 0.8, 0.9]]
B_vals = [[0.9, 0.8, 0.7], [0.6, 0.5, 0.4], [0.3, 0.2, 0.1]]

# float → Pt
A_pt = [[Pt.from_real(v) for v in row] for row in A_vals]
B_pt = [[Pt.from_real(v) for v in row] for row in B_vals]

# matmul в Pt
C_pt = matmul_pt(A_pt, B_pt)

# Pt → float
C_float = [[c.to_real() for c in row] for row in C_pt]
C_ref = matmul_ref(A_vals, B_vals)

for i in range(3):
    for j in range(3):
        ok = abs(C_float[i][j] - C_ref[i][j]) < 1e-10
        check(f"float matmul 3×3 [{i}][{j}] {C_float[i][j]} ≈ {C_ref[i][j]}", ok)

# ═══════════════════════════════════════════════════════════
# 3. Многошаг: Pt → операция → float → Pt → операция → float
# ═══════════════════════════════════════════════════════════
print("\n=== 3. Многошаговый pipeline ===")
# (0.347 + 1.653) × (5.0 - 3.0) ÷ 0.5 = 2.0 × 2.0 ÷ 0.5 = 8.0
steps = [
    ("add", 0.347, 1.653),
    ("sub", 5.0, 3.0),
    ("mul", None, None),  # предыдущие два перемножить
    ("div", None, 0.5),    # результат разделить
]
# Реализуем pipeline
a = Pt.from_real(0.347)
b = Pt.from_real(1.653)
c = Pt.from_real(5.0)
d = Pt.from_real(3.0)

step1 = radd(a, b)          # 2.0
step2 = rsub(c, d)          # 2.0
step3 = rmul(step1, step2)  # 4.0
step4 = rdiv(step3, Pt.from_real(0.5))  # 8.0

ok = abs(step4.to_real() - 8.0) < 1e-12
check("pipeline: (0.347+1.653)×(5-3)÷0.5=8.0", ok, f"got {step4.to_real()}")

# ═══════════════════════════════════════════════════════════
# 4. Большой pipeline: 10 шагов
# ═══════════════════════════════════════════════════════════
print("\n=== 4. Длинный pipeline (10 шагов) ===")
v = Pt.from_real(1.0)
# 10 × умножение на 2 = 1024
for _ in range(10):
    v = rmul(v, Pt.from_real(2.0))
ok = abs(v.to_real() - 1024.0) < 1e-10
check("10×rmul(×2): 1→1024", ok, f"got {v.to_real()}")

# 5 × деление на 2 = обратно к ~32
for _ in range(5):
    v = rdiv(v, Pt.from_real(2.0))
ok = abs(v.to_real() - 32.0) < 1e-10
check("5×rdiv(÷2): 1024→32", ok, f"got {v.to_real()}")

# ═══════════════════════════════════════════════════════════
# 5. Cross-check: gcd-scaling + mantissa-rank (очень большие float)
# ═══════════════════════════════════════════════════════════
print("\n=== 5. GCD + mantissa-rank совместно ===")
# 123456.789 × 987654.321 через mantissa-rank
# Мантиссы: 123456789 × 987654321 — обе >1024 → gcd-scaling
big_a = Pt.from_real(123456.789)
big_b = Pt.from_real(987654.321)
big_c = rmul(big_a, big_b)
expected = 123456.789 * 987654.321
ok = abs(big_c.to_real() - expected) / expected < 1e-10
check(f"rmul big: {big_c.to_real()} ≈ {expected}", ok, 
      f"Pt({big_c.x},{big_c.y})")

# ═══════════════════════════════════════════════════════════
print(f"\n{'='*60}")
print(f"ИТОГ: {passed}/{passed+failed} passed, {failed} FAILED")
assert failed == 0, f"{failed} failures!"
