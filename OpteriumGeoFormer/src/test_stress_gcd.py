"""Стресс-тесты gcd-scaling: coprime пары, глубокая рекурсия, matmul с float"""

import sys, os, time, random
sys.path.insert(0, os.path.dirname(__file__))
sys.path.insert(0, os.path.join(os.path.dirname(__file__), 'spec-kit'))

from arith_table import PT
from methods import Pt, rmul, radd, rsub, rdiv
from math import gcd
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
print("  СТРЕСС-ТЕСТЫ GCD-SCALING")
print("=" * 60)

# ═══════════════════════════════════════════════════════════
# 1. Out-of-range product (pure int)
# ═══════════════════════════════════════════════════════════
print("\n=== 1. Out-of-range P(x,y) vs a*b ===")
int_pairs = [
    (0, 0), (0, 5000), (1, 1024), (1024, 1024),
    (1234, 5678), (9999, 8888), (12345, 67890),
    (100000, 200000), (999999, 999999),
    (2**20, 3**10), (123456789, 987654321),
    (10**9, 10**9), (10**12, 1),
    (1234567890123, 9876543210987),
]
for x, y in int_pairs:
    p = PT.P(x, y)
    expected = x * y
    ok = p == expected
    check(f"P({x}, {y})={p} == {expected}", ok, f"got {p}")

# ═══════════════════════════════════════════════════════════
# 2. Out-of-range S/D
# ═══════════════════════════════════════════════════════════
print("\n=== 2. Out-of-range S(x,y) and D(x,y) ===")
for x, y in [(10000, 1), (1, 9999), (10**6, 10**6), (5000, -3000), (-5000, 2000)]:
    s = PT.S(x, y)
    d = PT.D(x, y)
    ok_s = s == x + y
    ok_d = d == x - y
    check(f"S({x},{y})={s} == {x+y}", ok_s, f"got {s}")
    check(f"D({x},{y})={d} == {x-y}", ok_d, f"got {d}")

# ═══════════════════════════════════════════════════════════
# 3. Product via gcd-scaling: verify gcd logic
# ═══════════════════════════════════════════════════════════
print("\n=== 3. gcd-scaling trace ===")
test_cases = [
    (123456, 789012),  # gcd=12
    (1000, 2000),      # gcd=1000
    (999999, 777777),  # gcd=111111? no, gcd(999999,777777)=111111
    (2*3*5*7*11*13*17, 2*3*5*7*11*13*19),  # gcd=2*3*5*7*11*13
    (10**6, 10**6),    # gcd=10^6
]
for x, y in test_cases:
    p = PT.P(x, y)
    expected = x * y
    g = gcd(x, y)
    ok = p == expected
    check(f"P({x},{y}) g={g}: {p} == {expected}", ok, f"got {p}")

# ═══════════════════════════════════════════════════════════
# 4. Coprime pairs (gcd=1, outside table)
# ═══════════════════════════════════════════════════════════
print("\n=== 4. Coprime out-of-range (gcd=1 → fallback) ===")
coprime_pairs = [
    (1025, 1027),   # gcd=1, оба >1024
    (9999, 10000),  # gcd=1
    (12345, 67892), # gcd=1
    (999983, 999979),  # оба простые
    (2**20 + 1, 3**10 + 1),
]
for x, y in coprime_pairs:
    p = PT.P(x, y)
    expected = x * y
    ok = p == expected
    check(f"P({x},{y}) coprime: {p} == {expected}", ok, f"got {p}")

# ═══════════════════════════════════════════════════════════
# 5. Matmul with out-of-range values
# ═══════════════════════════════════════════════════════════
print("\n=== 5. Matmul out-of-range ===")
def matmul_ref(A, B):
    m, k, n = len(A), len(A[0]), len(B[0])
    return [[sum(A[i][p] * B[p][j] for p in range(k)) for j in range(n)] for i in range(m)]

# Small matrix with huge values
A = [[10**6, 2*10**6], [3*10**6, 4*10**6]]
B = [[5*10**6, 6*10**6], [7*10**6, 8*10**6]]
C_pt = PT.matmul(A, B)
C_ref = matmul_ref(A, B)
ok = C_pt == C_ref
check("matmul 2×2 huge values", ok, f"got {C_pt}, ref {C_ref}")

# Random matmul 10×10 with values 0..10000
random.seed(42)
m10 = [[random.randint(0, 10000) for _ in range(10)] for _ in range(10)]
n10 = [[random.randint(0, 10000) for _ in range(10)] for _ in range(10)]
C10 = PT.matmul(m10, n10)
C10r = matmul_ref(m10, n10)
ok = C10 == C10r
check("matmul 10×10 random 0..10000", ok, f"mismatch")

# Matmul with negative values
negA = [[-1000, 2000], [3000, -4000]]
negB = [[5000, -6000], [-7000, 8000]]
Cneg = PT.matmul(negA, negB)
Cnegr = matmul_ref(negA, negB)
ok = Cneg == Cnegr
check("matmul 2×2 negative", ok, f"got {Cneg}")

# ═══════════════════════════════════════════════════════════
# 6. Float matmul via rmul (mantissa-rank)
# ═══════════════════════════════════════════════════════════
print("\n=== 6. Float matmul via mantissa-rank ===")
def matmul_float(A, B):
    """Matmul using Pt rmul/sum: C[i][j] = Σ rmul(A[i][k], B[k][j])"""
    m, k, n = len(A), len(A[0]), len(B[0])
    C = [[Pt.from_real(0.0) for _ in range(n)] for _ in range(m)]
    for i in range(m):
        for p in range(k):
            ap = A[i][p]
            bp = B[p]
            for j in range(n):
                C[i][j] = radd(C[i][j], rmul(ap, bp[j]))
    return [[c.to_real() for c in row] for row in C]

A_float = [[Pt.from_real(v) for v in [0.3, 0.2]],
           [Pt.from_real(v) for v in [0.1, 0.4]]]
B_float = [[Pt.from_real(v) for v in [0.5, 0.6]],
           [Pt.from_real(v) for v in [0.7, 0.8]]]
C_float = matmul_float(A_float, B_float)
C_float_ref = matmul_ref([[0.3, 0.2], [0.1, 0.4]], [[0.5, 0.6], [0.7, 0.8]])
for i in range(2):
    for j in range(2):
        ok = abs(C_float[i][j] - C_float_ref[i][j]) < 1e-10
        check(f"float matmul [{i}][{j}] {C_float[i][j]} ≈ {C_float_ref[i][j]}", ok)

# ═══════════════════════════════════════════════════════════
# 7. Performance: average lookup time with gcd-scaling
# ═══════════════════════════════════════════════════════════
print("\n=== 7. Performance (gcd-scaling overhead) ===")
sizes = [10, 100, 500, 1000]
for sz in sizes:
    pairs = [(random.randint(sz+1, sz*100), random.randint(1, sz)) for _ in range(5000)]
    t0 = time.perf_counter()
    for x, y in pairs:
        _ = PT.P(x, y)
    dt = time.perf_counter() - t0
    check(f"P() lookup {sz}+ range: {dt*1000/len(pairs):.3f}ms avg", dt < 5.0,
          f"too slow: {dt*1000:.1f}ms total")

# ═══════════════════════════════════════════════════════════
print(f"\n{'='*60}")
print(f"ИТОГ: {passed}/{passed+failed} passed, {failed} FAILED")
assert failed == 0, f"{failed} failures!"
