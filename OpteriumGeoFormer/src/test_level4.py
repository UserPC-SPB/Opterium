"""УРОВЕНЬ 4 — Стресс-тестирование и границы"""
import sys, os, time, random
SRC = os.path.dirname(__file__)
sys.path.insert(0, SRC)
sys.path.insert(0, os.path.join(SRC, 'spec-kit'))

from arith_table import PT
from methods import Pt, rmul, radd
from cube27 import Cube27
from hashgrid import HashGrid, geometric_attention
from delta_ops import HealthVector, HEALTH_OK, DELTA_ADD, DELTA_MUL, DELTA_INV
from geoformer import GeoFormer
from decimal import Decimal, getcontext
getcontext().prec = 100

PASS = 0; FAIL = 0; ERRORS = []
def check(module, test, cond, msg=""):
    global PASS, FAIL
    if cond:
        PASS += 1
    else:
        FAIL += 1
        ERRORS.append((module, test, msg))
        print(f"  ❌ [{module}] {test}: {msg}")

# ═══════════════════════════════════════════════════════════
# 4.1 Размерность
# ═══════════════════════════════════════════════════════════
print("\n=== 4.1 Размерность ===")

# Cube27: длинные мантиссы
c = Cube27()
for digits in [1, 3, 6, 9, 12, 15, 30, 99]:
    n = int('1' + '0' * (digits - 1)) if digits > 1 else 1
    path = c.path_27(n)
    expected_depth = (digits + 2) // 3
    check("Stress", f"Cube27 depth for {digits}-digit num={len(path)}",
          len(path) == expected_depth,
          f"digits={digits}: depth={len(path)} expected={expected_depth}")
    # Проверка что все группы ≤ 999
    groups = c.encode(n)
    for g in groups:
        check("Stress", f"   group {g} ≤ 999", g <= 999, f"group={g} > 999")

# Matrix: spec-kit matmul (через Pt)
from methods import validate_shape
A = [[1]*50 for _ in range(50)]
B = [[2]*30 for _ in range(50)]
m, k, n = validate_shape(A, B)
check("Stress", "matmul shape 50×50 × 50×30", m==50 and k==50 and n==30, f"got {m}x{k}x{n}")

# Pt для максимальных координат
for x, y in [(1024, 1024), (0, 1), (1024, 1)]:
    p = Pt(x, y)
    check("Stress", f"Pt({x},{y}) S invariant", p.S == x+y, f"S={p.S}")
    check("Stress", f"Pt({x},{y}) D invariant", p.D == x-y, f"D={p.D}")
    check("Stress", f"Pt({x},{y}) P invariant", p.P == x*y, f"P={p.P}")

# HashGrid: много токенов
for n_tokens in [0, 1, 10, 100, 1000]:
    random.seed(42)
    pts = [(i, random.randint(1, 1000), random.randint(-500, 500), random.randint(1, 1000))
           for i in range(n_tokens)]
    out = geometric_attention(pts, window=32 if n_tokens > 100 else 16)
    check("Stress", f"geometric_attention({n_tokens} tokens) out len",
          len(out) == n_tokens, f"got {len(out)}")
    if n_tokens > 0:
        check("Stress", f"   first has context", out[0].get('context', -1) != -1,
              f"no context for first token")

# ═══════════════════════════════════════════════════════════
# 4.2 Диапазон значений
# ═══════════════════════════════════════════════════════════
print("\n=== 4.2 Диапазон значений ===")

# Очень большие Decimal (через from_decimal)
large_dec = Decimal('1e30')
p = Pt.from_decimal(large_dec)
back = p.to_decimal()
check("Range", f"from_decimal(1e30) roundtrip",
      back == large_dec, f"Pt({p.x},{p.y}) → {back}")

# Очень малые Decimal
small_dec = Decimal('1e-30')
p = Pt.from_decimal(small_dec)
back = p.to_decimal()
check("Range", f"from_decimal(1e-30) roundtrip",
      back == small_dec, f"Pt({p.x},{p.y}) → {back}")

# inv для разных рангов
for m, r in [(347, 3), (1, 10), (1, 1), (999, 5)]:
    p = Pt(m, r)
    pinv = p.inv()
    prod = pinv.to_decimal() * p.to_decimal()
    check("Range", f"inv({m}|{r}|) product≈1",
          abs(prod - Decimal(1)) < Decimal('1e-25'),
          f"p={p} inv={pinv} prod={prod}")

# Positive и negative числа: все комбинации в PtTable
for x in [-5, -1, 1, 5]:
    for y in [-5, -1, 1, 5]:
        s = PT.S(x, y)
        d = PT.D(x, y)
        p = PT.P(x, y)
        check("Range", f"PtTable({x},{y}) S={s}", True, "")  # just verify no crash
        check("Range", f"  D={d}", True, "")
        check("Range", f"  P={p}", True, "")

# product/sum/diff с отрицательными
for a in [-10, -1, 0, 1, 10]:
    for b in [-10, -1, 0, 1, 10]:
        check("Range", f"sum({a},{b})", PT.sum(a,b) == a+b, f"got {PT.sum(a,b)}")
        check("Range", f"diff({a},{b})", PT.diff(a,b) == a-b, f"got {PT.diff(a,b)}")
        check("Range", f"product({a},{b})", PT.product(a,b) == a*b, f"got {PT.product(a,b)}")

# from_sd крайние случаи
for s, d in [(0, 0), (1, 1), (0, 2), (2, 0)]:
    x, y = PT.from_sd(s, d)
    check("Range", f"from_sd({s},{d})→({x},{y}) S invariant", x+y==s,
          f"({x},{y}): x+y={x+y} != {s}")
    check("Range", f"from_sd({s},{d}) D invariant", x-y==d,
          f"({x},{y}): x-y={x-y} != {d}")

# ═══════════════════════════════════════════════════════════
# 4.3 Детерминизм
# ═══════════════════════════════════════════════════════════
print("\n=== 4.3 Детерминизм ===")

# PtTable: всегда одинаковый
for trial in range(3):
    for x in range(11):
        for y in range(1, 11):
            s = PT.S(x, y)
            if s != x+y:
                check("Det", f"PtTable determinism trial {trial}", False,
                      f"({x},{y}).S={s} != {x+y}")
                break
        else:
            continue
        break
    else:
        continue
    break
else:
    check("Det", "PtTable deterministic", True, "")

# Cube27: всегда одинаковый
for trial in range(3):
    path = c.path_27(123456789)
    if path != c.path_27(123456789):
        check("Det", "Cube27 determinism", False, f"trial {trial}")
        break
else:
    check("Det", "Cube27 deterministic", True, "")

# From_decimal: детерминизм
for trial in range(3):
    p1 = Pt.from_decimal(Decimal('3.14159265358979'))
    p2 = Pt.from_decimal(Decimal('3.14159265358979'))
    if p1 != p2:
        check("Det", "from_decimal determinism", False, f"trial {trial}: {p1} != {p2}")
        break
else:
    check("Det", "from_decimal deterministic", True, "")

# ═══════════════════════════════════════════════════════════
# 4.4 Производительность
# ═══════════════════════════════════════════════════════════
print("\n=== 4.4 Производительность ===")

N = 100000

# PtTable lookup performance
t0 = time.perf_counter()
s = 0
for i in range(N):
    x = (i % 1000) + 1
    y = ((i * 7) % 1000) + 1
    s += PT.product(x, y)
t1 = time.perf_counter()
check("Perf", f"{N} PtTable lookups in {t1-t0:.3f}s",
      (t1 - t0) < 5.0, f"too slow: {t1-t0:.3f}s ({N/(t1-t0)/1000:.0f}K/s)")

# Cube27 encode performance
c = Cube27()
t0 = time.perf_counter()
for i in range(N // 10):
    _ = c.encode(i * 1000 + 1)
t1 = time.perf_counter()
check("Perf", f"{N//10} Cube27 encodes",
      (t1 - t0) < 3.0, f"too slow: {t1-t0:.3f}s")

# Pt constructor performance
t0 = time.perf_counter()
for i in range(N):
    _ = Pt(i % 1000 + 1, 1)
t1 = time.perf_counter()
check("Perf", f"{N} Pt constructors",
      (t1 - t0) < 5.0, f"too slow: {t1-t0:.3f}s")

# inv performance (Decimal lift-solve-project is slower)
t0 = time.perf_counter()
for i in range(1000):
    p = Pt(i + 1, 1)
    _ = p.inv()
t1 = time.perf_counter()
check("Perf", f"1000 inv() calls in {t1-t0:.3f}s",
      (t1 - t0) < 5.0, f"too slow: {t1-t0:.3f}s")

# ═══════════════════════════════════════════════════════════
# 4.5 Устойчивость (edge cases)
# ═══════════════════════════════════════════════════════════
print("\n=== 4.5 Устойчивость ===")

# Pt с y=0 (debt=0: число без десятичного сдвига, mantissa=42)
p = Pt(42, 0)
check("Robust", "Pt(42,0).y=0 (debt=0)", p.y == 0, f"y={p.y}")
check("Robust", "Pt(42,0).P=0 (x*y)", p.P == 0, f"P={p.P}")
check("Robust", "Pt(42,0).to_real()=42.0", p.to_real() == 42.0, f"to_real={p.to_real()}")

# Pt.parse с невалидными форматами
# "347|" — валидно: "347|".split("|") = ["347",""], y=1 по умолчанию
invalid = ["", "|", "||", "abc"]
for s in invalid:
    try:
        Pt.parse(s)
        check("Robust", f"parse({s!r}) should raise", False,
              f"no error for {s!r}")
    except (ValueError, IndexError):
        check("Robust", f"parse({s!r}) error", True, "")

# from_decimal с 0
p = Pt.from_decimal(Decimal(0))
check("Robust", "from_decimal(0).x=0", p.x == 0, f"x={p.x}")

# from_decimal с отрицательным
p = Pt.from_decimal(Decimal('-3.14'))
check("Robust", "from_decimal(-3.14).x<0", p.x < 0, f"x={p.x}")
check("Robust", "from_decimal(-3.14) P = x*y (может быть отрицательным)",
      p.P == p.x * p.y, f"P={p.P} != {p.x}*{p.y}")

# Cube27 encode(0)
check("Robust", "Cube27.encode(0)=[0]", c.encode(0) == [0], f"got {c.encode(0)}")

# Cube27 verify(0)
info = c.verify(0)
check("Robust", "Cube27.verify(0) all_hit", info['all_hit'], f"{info}")

# geometric_attention с одним токеном
out = geometric_attention([(0, 10, 5, 20)], window=16)
check("Robust", "attention single token", len(out) == 1, f"got {len(out)}")
check("Robust", "attention single context>0", out[0].get('context', -1) > 0,
      f"context={out[0].get('context')}")

# HashGrid с window=1 (граничный случай)
g = HashGrid(window=1)
for i in range(10):
    g.insert(i, i*3, i*2)
nb = g.lookup(3, 2)
check("Robust", "hashgrid window=1 returns self", len(nb) >= 1, f"got {len(nb)} neighbors")

# Swarm с 1 узлом
from swarm import IntelligentSwarm
swarm = IntelligentSwarm(seed=42)
swarm.register('only_route')
chosen = swarm.decide(deterministic=True)
check("Robust", "swarm single node", chosen.id == 'only_route', f"got {chosen.id}")

# ═══════════════════════════════════════════════════════════
# SUMMARY
# ═══════════════════════════════════════════════════════════
print(f"\n{'='*50}")
print(f"УРОВЕНЬ 4: {PASS}/{PASS+FAIL} passed, {FAIL} FAILED")
if ERRORS:
    print(f"\nПРОБЛЕМЫ:")
    for module, test, msg in ERRORS:
        print(f"  [{module}] {test}: {msg}")
else:
    print(f"✅ Все тесты Уровня 4 прошли")
