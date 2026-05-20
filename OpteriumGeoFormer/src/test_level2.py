"""УРОВЕНЬ 2 — Cross-module тестирование"""
import sys, os
SRC = os.path.dirname(__file__)
sys.path.insert(0, SRC)
sys.path.insert(0, os.path.join(SRC, 'spec-kit'))

from arith_table import PT
from methods import Pt, rmul, radd
from cube27 import Cube27
from hashgrid import HashGrid, geometric_weight, geometric_attention
from delta_ops import (HealthVector, DELTA_SHIFT, DELTA_MUL, DELTA_INV,
    DELTA_PPH, DELTA_ADD, DELTA_OPTG, HEALTH_OK)
from geoformer import GeoFormer, GeometricBlock, GeometricEmbedding
from e8_twist import TwistEngine, address_to_root, root_properties
from decimal import Decimal
import random

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
# 2.1 PtTable + Pt class
# ═══════════════════════════════════════════════════════════
print("\n=== 2.1 PtTable + Pt class ===")

# Pt constructor S/D/P = PT.lookup для табличных значений
for x in range(11):
    for y in range(1, 11):
        p = Pt(x, y)
        sdp = PT.lookup(x, y)
        check("Pt+PT", f"Pt({x},{y}).S==PT.S", p.S == sdp['S'], f"{p.S} != {sdp['S']}")
        check("Pt+PT", f"Pt({x},{y}).D==PT.D", p.D == sdp['D'], f"{p.D} != {sdp['D']}")
        check("Pt+PT", f"Pt({x},{y}).P==PT.P", p.P == sdp['P'], f"{p.P} != {sdp['P']}")

# Pt fallback для > MAX_COORD
p = Pt(2000, 3000)
check("Pt+PT", "Pt(2000,3000) formula S", p.S == 2000+3000, f"{p.S} != {2000+3000}")
check("Pt+PT", "Pt(2000,3000) formula D", p.D == 2000-3000, f"{p.D} != {2000-3000}")
check("Pt+PT", "Pt(2000,3000) formula P", p.P == 2000*3000, f"{p.P} != {2000*3000}")

# Pt.from_int → PT.product
for v in range(1, 50):
    p = Pt.from_int(v)
    check("Pt+PT", f"from_int({v}).P==PT.P({v},1)", p.P == PT.P(v, 1), f"{p.P} != {PT.P(v,1)}")

# Pt rmul/radd → PT lookup
a = Pt(3, 2); b = Pt(5, 4)
c = rmul(a, b)
check("Pt+PT", "rmul: P = PT.product(x1,x2)", c.x == PT.product(3, 5), f"{c.x}")
check("Pt+PT", "rmul: y = PT.sum(y1,y2)", c.y == PT.sum(2, 4), f"{c.y}")

c = radd(a, b)
check("Pt+PT", "radd: rank alignment y=4", c.y == PT.max(2, 4), f"{c.y}")

# inv() → PT.product ≈ 1
for m, r in [(347, 3), (2, 1), (10, 1), (1, 1)]:
    p = Pt(m, r)
    pinv = p.inv()
    prod_val = pinv.to_decimal() * p.to_decimal()
    check("Pt+PT", f"inv({m}|{r}|): p*inv≈1", abs(prod_val - Decimal(1)) < Decimal('1e-25'),
          f"p*inv={prod_val}")

# ═══════════════════════════════════════════════════════════
# 2.2 PtTable + Cube27
# ═══════════════════════════════════════════════════════════
print("\n=== 2.2 PtTable + Cube27 ===")
c = Cube27()

# Все группы 0..999 → PtTable hit
for g in range(0, 1000):
    if g == 0:
        check("Cube+PT", f"group {g} hit", True, "")  # 0 виртуально
    else:
        hit = PT.has(g, 1)
        if not hit:
            check("Cube+PT", f"group {g} PtTable miss", False, f"group {g} not in PtTable")
            break
else:
    check("Cube+PT", "all groups 0..999 PtTable hit", True, "")

# Большое число: путь → PtTable → восстановление
# 123456789 = 123*10^6 + 456*10^3 + 789
# Через PtTable: Pt(123,1).P * 10^6 + Pt(456,1).P * 10^3 + Pt(789,1).P
n = 123456789
path = c.path_27(n)
groups = c.encode(n)
# Проверка что каждая группа ≤ 999 (PtTable hit)
for i, g in enumerate(groups):
    check("Cube+PT", f"group[{i}]={g} ≤ 999", g <= 999, f"group {g} > 999")
    if g > 0:
        hit = PT.has(g, 1)
        check("Cube+PT", f"group[{i}]={g} PtTable", hit, f"miss")

# path_27 длина = encode длина
check("Cube+PT", "path length == encode length", len(path) == len(groups), f"")

# ═══════════════════════════════════════════════════════════
# 2.3 PtTable + Δ-ops
# ═══════════════════════════════════════════════════════════
print("\n=== 2.3 PtTable + Δ-ops ===")

# Δ_SHIFT на PtTable значении
val = PT.P(3, 5)  # 15
r, hv = DELTA_SHIFT(val, power=2)
check("Pt+Δ", f"SHIFT(P(3,5))={r}", r == 1500, f"got {r}")

# Δ_MUL на двух PtTable значениях
r, hv = DELTA_MUL(PT.P(3, 5), PT.P(2, 7))  # 15 * 14
check("Pt+Δ", f"MUL(P(3,5),P(2,7))={r}", r == 210, f"got {r}")

# Δ_INV на PtTable значении → через Doctor
r, hv = DELTA_INV(PT.P(4, 1))  # inv(4) = 0.25
check("Pt+Δ", f"INV(P(4,1))={r}", abs(r - 0.25) < 1e-15, f"got {r}")

# SHIFT >> INV через PT
seq = DELTA_SHIFT >> DELTA_INV
r, hv = seq(PT.P(5, 1), power=0)  # shift(5,0) → 5, inv(5) → 0.2
check("Pt+Δ", f"SHIFT>>INV on P(5,1)={r}", abs(r - 0.2) < 1e-15, f"got {r}")

# Δ_PPH на PtTable values
pph, hv = DELTA_PPH([PT.P(3, 5), PT.P(2, 7)])
check("Pt+Δ", "PPH is finite", not (pph == float('inf') or pph == float('-inf') or pph != pph),
      f"pph={pph} (non-finite)")
check("Pt+Δ", "PPH hv returns ok type", isinstance(hv.ok, bool), f"hv.ok type={type(hv.ok)}")

# ═══════════════════════════════════════════════════════════
# 2.4 PtTable + E8 twist
# ═══════════════════════════════════════════════════════════
print("\n=== 2.4 PtTable + E8 twist ===")

# address_to_root из from_sd
for s, d in [(8, -2), (8, 2), (10, 0), (7, -3)]:
    x, y = PT.from_sd(s, d)
    r = address_to_root(x, y)
    check("Pt+E8", f"address({x},{y})→root len=8", len(r) == 8, f"got len {len(r)}")
    check("Pt+E8", f"from_sd({s},{d})→({x},{y})→root", all(-2 <= v <= 2 for v in r),
          f"root {r} out of range")

# root_properties адресов из PtTable
for x, y in [(3, 5), (5, 3), (4, 4), (2, 2)]:
    r = address_to_root(x, y)
    props = root_properties(r)
    check("Pt+E8", f"root_properties({x},{y}) sector known", props['sector'] in ('D8', 'Spinor'), f"{props}")

# адрес и conj должны давать одинаковые root_properties P
x, y = PT.conj(3, 5)
r1 = address_to_root(3, 5)
r2 = address_to_root(x, y)
p1 = root_properties(r1)
p2 = root_properties(r2)
check("Pt+E8", "conj roots same norm2", p1['norm2'] == p2['norm2'], f"{p1['norm2']} != {p2['norm2']}")

# ═══════════════════════════════════════════════════════════
# 2.5 Pt class + Cube27
# ═══════════════════════════════════════════════════════════
print("\n=== 2.5 Pt class + Cube27 ===")

# from_real → Cube27 → PtTable hit
for r_val in [0.347, 2.34, -3.14159, 0.001, 100.0]:
    p = Pt.from_real(r_val)
    if p.x > 0:
        path = c.path_27(p.x)
        check("Pt+Cube", f"from_real({r_val})→Pt({p.x},{p.y})→Cube27 path len={len(path)}",
              len(path) >= 1, f"empty path")

# from_decimal → Cube27
d = Decimal('3.1415926535897932384626433832795028841971')
p = Pt.from_decimal(d)
if p.x > 0:
    path = c.path_27(p.x)
    check("Pt+Cube", f"from_decimal({d})→Pt({p.x},{p.y})→Cube27 path",
          len(path) >= 1, f"empty path")

# ═══════════════════════════════════════════════════════════
# 2.6 Pt class + HashGrid
# ═══════════════════════════════════════════════════════════
print("\n=== 2.6 Pt class + HashGrid ===")

g = HashGrid(window=16)
pts = [Pt(i*3+1, 1) for i in range(10)]
for i, pt in enumerate(pts):
    g.insert(i, pt.S, pt.D, P=pt.P)

# lookup для каждого Pt
for i, pt in enumerate(pts):
    nb = g.lookup(pt.S, pt.D)
    # Должен найти себя и соседей
    ids = [e['id'] for e in nb]
    check("Pt+HG", f"Pt[{i}] lookup self in neighbors", i in ids,
          f"id {i} not found in lookup")

# ═══════════════════════════════════════════════════════════
# 2.7 HashGrid + GeoFormer
# ═══════════════════════════════════════════════════════════
print("\n=== 2.7 HashGrid + GeoFormer ===")

# GeometricBlock использует geometric_attention внутри
block = GeometricBlock(window=10)
tokens = [Pt(i * 2 + 1, 1) for i in range(10)]
out, hv = block.forward(tokens)
check("HG+Geo", "block forward length", len(out) == 10, f"got {len(out)}")
check("HG+Geo", "block forward returns HealthVector", isinstance(hv, HealthVector), f"got {type(hv)}")
check("HG+Geo", "block forward hv returns ok", isinstance(hv.ok, bool), f"")

# GeoFormer полный forward
gf = GeoFormer(layers=2, window=10)
out, hv = gf.forward([1, 2, 3, 4, 5])
check("HG+Geo", "GeoFormer forward len", len(out) == 5, f"got {len(out)}")
check("HG+Geo", "GeoFormer forward Pt type", all(isinstance(p, Pt) for p in out), "")
check("HG+Geo", "GeoFormer forward hv ok type", isinstance(hv.ok, bool), "")

# Детерминизм
random.seed(123)
gf1 = GeoFormer(layers=2, window=10)
out1, _ = gf1.forward([i+1 for i in range(6)])
random.seed(123)
gf2 = GeoFormer(layers=2, window=10)
out2, _ = gf2.forward([i+1 for i in range(6)])
identical = all(o1.x==o2.x and o1.y==o2.y for o1,o2 in zip(out1, out2))
check("HG+Geo", "deterministic forward", identical, "outputs differ with same seed")

# ═══════════════════════════════════════════════════════════
# 2.8 Δ-ops + Φ-algebra
# ═══════════════════════════════════════════════════════════
print("\n=== 2.8 Δ-ops + Φ-algebra ===")

from phi_algebra import PHI1_SHIFT, PHI3_FIXEDPOINT, PhiPath

# Φ-оператор напрямую (PhiPath не передает kwargs)
r = PHI1_SHIFT((0, 5), dx=1)
check("Δ+Φ", "Φ1_SHIFT on (0,5)", r == (1, 5), f"got {r}")

# Δ_SHIFT >> Δ_INV как Φ
r, hv = DELTA_SHIFT(3.0, power=1)
r2, hv2 = DELTA_INV(r)
check("Δ+Φ", "Δ_SHIFT>>Δ_INV chain", abs(r2 - 1/30) < 1e-15, f"got {r2}")

# ═══════════════════════════════════════════════════════════
# 2.9 E8 Twist + Δ-ops
# ═══════════════════════════════════════════════════════════
print("\n=== 2.9 E8 Twist + Δ-ops ===")

# Δ_OPTG на E8 roots
state = [1, 0, 0, 0, 0, 0, 0, 0]
attractor = [0, 1, 0, 0, 0, 0, 0, 0]
r, hv = DELTA_OPTG(state, attractor)
check("E8+Δ", "OPTG on E8 roots", len(r) == 8, f"got len {len(r)}")

# TWIST closure
te = TwistEngine()
closure = te.closure_angle(70)
check("E8+Δ", "closure_angle CLOSED", closure.get('status') == 'CLOSED', f"{closure.get('status')}")

# ═══════════════════════════════════════════════════════════
# 2.10 Doctor Bridge + GeoFormer
# ═══════════════════════════════════════════════════════════
print("\n=== 2.10 Doctor + GeoFormer ===")

from doctor_geo import SwarmDoctor, GeoHealthVector, ROUTE_REGISTRY

# SwarmTrainer + Doctor verdict
gf = GeoFormer(layers=2, window=6)
from geoformer import SwarmTrainer, doctor_judge

trainer = SwarmTrainer(gf)
result = trainer.train_step([1, 2, 3], [2, 4, 6])
check("Doc+Geo", "train_step has score", 'score' in result, "")
check("Doc+Geo", "train_step has success", 'success' in result, "")

# Doctor judge на output
output, hv = gf.forward([1, 2, 3])
verdict = doctor_judge(output, [2, 4, 6], hv)
check("Doc+Geo", "doctor_judge returns verdict", verdict in ('OK', 'WARN', 'FAIL'),
      f"got {verdict}")

# SwarmDoctor judge через GeoFormer HealthVector
sd = SwarmDoctor(swarm_seed=42)
for name, info in ROUTE_REGISTRY.items():
    sd.register_route(name, potential=info['default_potential'])
ghv = GeoHealthVector(0, 0, 0, 0, 0, 0, 0)
check("Doc+Geo", "SwarmDoctor judge OK", sd.judge(ghv) == 'OK', f"got {sd.judge(ghv)}")

# Через opterium если доступно
if hasattr(sd, 'opterium_verdict'):
    verdict = sd.opterium_verdict(ghv, context='geoformer_test')
    check("Doc+Geo", "opterium_verdict has level", 'level' in verdict, f"got {verdict}")

# ═══════════════════════════════════════════════════════════
# SUMMARY
# ═══════════════════════════════════════════════════════════
print(f"\n{'='*50}")
print(f"УРОВЕНЬ 2: {PASS}/{PASS+FAIL} passed, {FAIL} FAILED")
if ERRORS:
    print(f"\nПРОБЛЕМЫ:")
    for module, test, msg in ERRORS:
        print(f"  [{module}] {test}: {msg}")
else:
    print(f"✅ Все тесты Уровня 2 прошли")
