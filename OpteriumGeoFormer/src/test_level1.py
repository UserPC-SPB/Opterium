"""УРОВЕНЬ 1 — Изолированное тестирование всех модулей"""
import sys, os
SRC = os.path.dirname(__file__)
sys.path.insert(0, SRC)
sys.path.insert(0, os.path.join(SRC, 'spec-kit'))

from arith_table import PT
from methods import Pt, rmul, radd, geo_mul, geo_add, validate_shape, to_pt_matrix, sd_tuple_matrix
from cube27 import Cube27
from hashgrid import HashGrid, geometric_weight, geometric_attention
from delta_ops import (HealthVector, DeltaOp, CompositeDelta, HEALTH_OK,
    DELTA_ADD, DELTA_MUL, DELTA_INV, DELTA_INV_NS, DELTA_PPH, DELTA_OPTG,
    DELTA_SHIFT, DELTA_ROT, DELTA_ZERO_DETECT,
    compose_sequential, compose_parallel, check_domain, select_fallback, identity)
from phi_algebra import (PHI1_SHIFT, PHI2_PHASE, PHI3_FIXEDPOINT, PHI4_RECURSION,
    PHI5_PROJECTION, PhiPath, periodic_orbit, harmonic_series)
from swarm import IntelligentSwarm, SwarmNode, BayesReplacement
from e8_twist import TwistEngine, address_to_root, root_properties
from decimal import Decimal
import math, random

PASS = 0
FAIL = 0
ERRORS = []

def check(module, test, cond, msg=""):
    global PASS, FAIL
    if cond:
        PASS += 1
    else:
        FAIL += 1
        ERRORS.append((module, test, msg))
        print(f"  ❌ [{module}] {test}: {msg}")

# ═══════════════════════════════════════════════════════════
# 1.1 PtTable
# ═══════════════════════════════════════════════════════════
print("\n=== 1.1 PtTable ===")

# S/D/P 4 quadrants
for x, y, exp_S, exp_D, exp_P in [
    (3, 5, 8, -2, 15),   # (+,+)
    (-3, 5, 2, -8, -15), # (-,+)
    (3, -5, -2, 8, -15), # (+,-)
    (-3, -5, -8, 2, 15), # (-,-)
]:
    s = PT.S(x, y); d = PT.D(x, y); p = PT.P(x, y)
    check("PtTable", f"S({x},{y})={s} exp={exp_S}", s == exp_S, f"S({x},{y})={s}")
    check("PtTable", f"D({x},{y})={d} exp={exp_D}", d == exp_D, f"D({x},{y})={d}")
    check("PtTable", f"P({x},{y})={p} exp={exp_P}", p == exp_P, f"P({x},{y})={p}")

# from_sd roundtrip
for s, d, exp_x, exp_y in [(8, -2, 3, 5), (8, 2, 5, 3), (10, 0, 5, 5), (7, -3, 2, 5)]:
    x, y = PT.from_sd(s, d)
    check("PtTable", f"from_sd({s},{d})→({x},{y})", x==exp_x and y==exp_y,
          f"got ({x},{y}) expected ({exp_x},{exp_y})")

# from_sd fallback (not in table)
x, y = PT.from_sd(2001, 1)
check("PtTable", f"from_sd(2001,1) fallback", x==1001 and y==1000, f"got ({x},{y}) for (2001,1)")

# pairs_for_product
for p, expected_count in [(0, 2049), (6, 4), (12, 6), (17, 2), (25, 3)]:
    pairs = PT.pairs_for_product(p)
    check("PtTable", f"pairs_for_product({p}) count={len(pairs)}", len(pairs)==expected_count,
          f"P={p}: got {len(pairs)} pairs, expected {expected_count}")
    for x, y in pairs:
        check("PtTable", f"P={p}: {x}*{y}=={p}", x*y==p, f"{x}*{y}={x*y}!={p}")

# isqrt
for n, exp in [(0, 0), (1, 1), (4, 2), (144, 12), (2, 1)]:
    r = PT.isqrt(n)
    check("PtTable", f"isqrt({n})={r} exp={exp}", r==exp, f"got {r}")

# conj
for x, y in [(3, 5), (-3, 5), (3, -5), (0, 5), (0, 0)]:
    cx, cy = PT.conj(x, y)
    check("PtTable", f"conj({x},{y})=({cx},{cy})", cx==y and cy==x, f"got ({cx},{cy})")
    check("PtTable", f"conj P preserved ({x},{y})", PT.P(x, y)==PT.P(cx, cy), f"P mismatch")

# product/sum/diff all signs
for a, b in [(3, 5), (-3, 5), (3, -5), (-3, -5), (0, 5), (5, 0)]:
    check("PtTable", f"sum({a},{b})={PT.sum(a,b)}", PT.sum(a,b)==a+b, f"")
    check("PtTable", f"diff({a},{b})={PT.diff(a,b)}", PT.diff(a,b)==a-b, f"")
    check("PtTable", f"product({a},{b})={PT.product(a,b)}", PT.product(a,b)==a*b, f"")

# pow10
for n in range(11):
    check("PtTable", f"pow10({n})={PT.pow10(n)}", PT.pow10(n)==10**n, f"")
check("PtTable", f"pow10(20) fallback", PT.pow10(20)==10**20, f"")

# abs
for v in range(-5, 6):
    check("PtTable", f"abs({v})={PT.abs(v)}", PT.abs(v)==abs(v), f"")

# has/InRange границы
check("PtTable", "has(0,1)", PT.has(0, 1), "")
check("PtTable", "has(1024,1024)", PT.has(1024, 1024), "")
check("PtTable", "!has(1025,1)", not PT.has(1025, 1), "")
check("PtTable", "has(0,0)", PT.has(0, 0), "y=0 now in range (debt=0 allowed)")

# ═══════════════════════════════════════════════════════════
# 1.2 Pt class
# ═══════════════════════════════════════════════════════════
print("\n=== 1.2 Pt class ===")

# constructors
p = Pt(3, 5)
check("Pt", "Pt(3,5).x=3, .y=5", p.x==3 and p.y==5, "")
check("Pt", "Pt(3,5).S=8", p.S==8, "")
check("Pt", "Pt(3,5).D=-2", p.D==-2, "")
check("Pt", "Pt(3,5).P=15", p.P==15, "")

# S/D/P invariant
for x in range(11):
    for y in range(1, 11):
        p = Pt(x, y)
        check("Pt", f"Pt({x},{y}) S invariant", p.S==x+y,
              f"S={p.S} != {x+y}")
        check("Pt", f"Pt({x},{y}) D invariant", p.D==x-y,
              f"D={p.D} != {x-y}")
        check("Pt", f"Pt({x},{y}) P invariant", p.P==x*y,
              f"P={p.P} != {x*y}")

# from_int
p = Pt.from_int(42)
check("Pt", "from_int(42).P=42", p.P==42, "")
check("Pt", "from_int(42).y=1", p.y==1, "")

# from_sd
p = Pt.from_sd(8, -2)
check("Pt", "from_sd(8,-2)=(3,5)", p.x==3 and p.y==5, f"got ({p.x},{p.y})")

# mantissa parse/repr/roundtrip
p = Pt.parse("347|3|")
check("Pt", "parse(347|3|).x=347", p.x==347, "")
check("Pt", "parse(347|3|).y=3", p.y==3, "")
check("Pt", "repr roundtrip", repr(Pt.parse(repr(p))) == repr(p), "")
check("Pt", "parse(0|3|)", Pt.parse("0|3|").x==0, "")

# to_real/from_real
for r in [0.0, 0.347, -0.5, 3.14, 0.001, 100.0, 0.0001, -3.14159, 1e-6, 12345.6789]:
    p = Pt.from_real(r)
    back = p.to_real()
    diff = abs(back - r) / max(1, abs(r))
    check("Pt", f"from_real({r})→to_real()={back} rel_err={diff:.2e}",
          diff < 1e-10, f"r={r}: Pt({p.x},{p.y})→{back} err={diff}")

# to_decimal/from_decimal
d = Decimal('3.1415926535897932384626433832795028841971')
p = Pt.from_decimal(d)
back = p.to_decimal()
check("Pt", f"Decimal roundtrip {d}", back == d, f"{d} → Pt({p.x},{p.y}) → {back}")

# inv
for val_str, mantissa, rank in [("0.347", 347, 3), ("0.2", 2, 1), ("5", 5, 1), ("1", 1, 1)]:
    p = Pt(mantissa, rank)
    pinv = p.inv()
    product = pinv.to_decimal() * p.to_decimal()
    check("Pt", f"inv({val_str}): product≈1", abs(product - Decimal(1)) < Decimal('1e-25'),
          f"p={p} inv={pinv} p*inv={product}")

# inv zero
p = Pt(0, 1)
pinv = p.inv()
check("Pt", "inv(0)=Pt(0,1)", pinv.x == 0 and pinv.y == 1, f"got {pinv}")

# Pt __add__ __mul__ (component-wise)
a = Pt(3, 5); b = Pt(2, 7)
c = a + b
check("Pt", f"(3,5)+(2,7)=({c.x},{c.y})", c.x==5 and c.y==12, f"got ({c.x},{c.y})")
c = a * b
check("Pt", f"(3,5)*(2,7)=({c.x},{c.y})", c.x==6 and c.y==35, f"got ({c.x},{c.y})")

# rmul/radd
a = Pt.from_real(0.3); b = Pt.from_real(0.2)
prod = rmul(a, b)
check("Pt", f"rmul(0.3,0.2)={prod.to_real()}", abs(prod.to_real()-0.06)<1e-12, f"got {prod.to_real()}")
summ = radd(a, b)
check("Pt", f"radd(0.3,0.2)={summ.to_real()}", abs(summ.to_real()-0.5)<1e-12, f"got {summ.to_real()}")

# geo_mul/geo_add
a = Pt(3, 5); b = Pt(2, 7)
c = geo_mul(a, b)
check("Pt", f"geo_mul((3,5),(2,7))=(6,35)", c.x==6 and c.y==35, f"got ({c.x},{c.y})")
c = geo_add(a, b)
check("Pt", f"geo_add((3,5),(2,7))", c.x==3*7+2*5 and c.y==5*7, f"got ({c.x},{c.y})")

# validate_shape
check("Pt", "validate_shape 3x4,4x2", validate_shape([[0]*4]*3, [[0]*2]*4) == (3,4,2), "")
try:
    validate_shape([[0]*3]*2, [[0]*2]*4)
    check("Pt", "validate_shape mismatch", False, "should raise ValueError")
except ValueError:
    check("Pt", "validate_shape mismatch", True, "")

# from_decimal: не-дробные числа
for d_val in [Decimal('42'), Decimal('100'), Decimal('0')]:
    p = Pt.from_decimal(d_val)
    back = p.to_decimal()
    check("Pt", f"from_decimal({d_val})→to_decimal={back}", back == d_val,
          f"{d_val} → Pt({p.x},{p.y}) → {back}")

# ═══════════════════════════════════════════════════════════
# 1.3 Cube27
# ═══════════════════════════════════════════════════════════
print("\n=== 1.3 Cube27 ===")
c = Cube27()

# encode
check("Cube27", "encode(0)=[0]", c.encode(0) == [0], "")
check("Cube27", "encode(1)=[1]", c.encode(1) == [1], "")
check("Cube27", "encode(347)=[347]", c.encode(347) == [347], "")
check("Cube27", "encode(999)=[999]", c.encode(999) == [999], "")
check("Cube27", "encode(123456789)=[123,456,789]", c.encode(123456789) == [123,456,789], "")
check("Cube27", "encode(1000000)=[1,0,0]", c.encode(1000000) == [1,0,0], "")

# cell_index границы
check("Cube27", "cell_index(0)=0", c.cell_index(0) == 0, "")
check("Cube27", "cell_index(36)=0", c.cell_index(36) == 0, "")
check("Cube27", "cell_index(37)=1", c.cell_index(37) == 1, "")
check("Cube27", "cell_index(999)=26", c.cell_index(999) == 26, "")
check("Cube27", "cell_index(1000)=26 (clamp)", c.cell_index(1000) == 26, "")

# cell_27 все 27 клеток
for ci in range(27):
    cx, cy, cz = c.cell_27(ci)
    check("Cube27", f"cell_27 idx={ci} in 0..2", 0<=cx<=2 and 0<=cy<=2 and 0<=cz<=2,
          f"({cx},{cy},{cz}) for idx {ci}")

# format_path
fmt = c.format_path(123456789)
check("Cube27", "format contains groups", "123|456|789|" in fmt, f"fmt={fmt}")

# verify: 100% hit для разных чисел
for v in [0, 1, 42, 347, 999, 1000, 999999, 1000000, 123456789, 999999999999]:
    info = c.verify(v)
    check("Cube27", f"verify({v}) all_hit={info['all_hit']}", info['all_hit'],
          f"miss for {v}: {info}")

# encode с отрицательным
try:
    c.encode(-1)
    check("Cube27", "encode(-1) should raise ValueError", False, "no error")
except ValueError:
    check("Cube27", "encode(-1) ValueError", True, "")

# ═══════════════════════════════════════════════════════════
# 1.4 HashGrid
# ═══════════════════════════════════════════════════════════
print("\n=== 1.4 HashGrid ===")
g = HashGrid(window=10)

g.insert(0, 5, 3)
g.insert(1, 7, 2)
g.insert(2, 100, 50)

nb = g.lookup(6, 3)
check("HashGrid", "lookup соседей ≥2", len(nb) >= 2, f"got {len(nb)} neighbors")
check("HashGrid", "0 empty lookup", g.lookup(999, 999) == [], "")

g.insert_many([(3, 15, 8), (4, 25, 12)])
stats = g.stats()
check("HashGrid", "stats total=5", stats['total'] == 5, f"got {stats['total']}")
check("HashGrid", "stats buckets>0", stats['buckets'] > 0, "")

# geometric_weight симметрия
w1 = geometric_weight(10, 5, 20, 15)
w2 = geometric_weight(20, 15, 10, 5)
check("HashGrid", "weight symmetry", abs(w1-w2) < 1e-15, f"w1={w1} w2={w2}")

# geometric_attention
pts = [(i, random.randint(1, 100), random.randint(-50, 50), random.randint(1, 100))
       for i in range(20)]
out = geometric_attention(pts, window=20)
check("HashGrid", "attention output length", len(out)==20, f"got {len(out)}")
for o in out:
    check("HashGrid", f"attention[{o['id']}].context>0", o.get('context', -1) > 0, f"context={o.get('context')}")

# geometric_attention пустой
check("HashGrid", "attention empty", geometric_attention([], window=10) == [], "")

# clear
g.clear()
check("HashGrid", "clear", g.stats()['total'] == 0, "")

# window=1 edge
g2 = HashGrid(window=1)
g2.insert(0, 5, 3)
check("HashGrid", "window=1 lookup", len(g2.lookup(5, 3)) == 1, "")

# ═══════════════════════════════════════════════════════════
# 1.5 Δ-ops
# ═══════════════════════════════════════════════════════════
print("\n=== 1.5 Δ-ops ===")

# HealthVector
hv = HealthVector(0.1, 0.2, 0.0, 0.05, 0.3, 0.15, 0.01)
check("Δ-ops", "hv.ok=True (all<0.35)", hv.ok, f"ok={hv.ok} max={hv.max_channel}")
check("Δ-ops", "hv.warn=False (all<0.35)", not hv.warn, f"warn={hv.warn}")
check("Δ-ops", "hv.critical=False", not hv.critical, "")
hv2 = HealthVector(0.8, 0, 0, 0, 0, 0, 0)
check("Δ-ops", "hv2.critical=True", hv2.critical, "")
check("Δ-ops", "hv2.ok=False", not hv2.ok, "")

hv_merge = hv.merge(hv2)
check("Δ-ops", "merge max channel", hv_merge.E_assoc==0.8 and hv_merge.E_precision==0.05, "")

# DeltaOp basic
r, hv = DELTA_ADD(3.0, 5.0)
check("Δ-ops", "DELTA_ADD 3+5=8", abs(r-8.0)<1e-15, f"got {r}")
r, hv = DELTA_MUL(3.0, 5.0)
check("Δ-ops", "DELTA_MUL 3*5=15", abs(r-15.0)<1e-15, f"got {r}")
r, hv = DELTA_INV(4.0)
check("Δ-ops", "DELTA_INV 1/4=0.25", abs(r-0.25)<1e-15, f"got {r}")

# INV zero
r, hv = DELTA_INV(0.0)
check("Δ-ops", "DELTA_INV(0)→inf", r==float('inf') or hv.critical, f"r={r} hv={hv}")

# INV_NS
r, hv = DELTA_INV_NS((0.0, 1.0, 0.0))
check("Δ-ops", "INV_NS norm≈1", abs(sum(x*x for x in r)-1) < 1e-10, f"r={r}")

# PPH
pph, hv = DELTA_PPH([1.0, 0.5, 0.2])
check("Δ-ops", "PPH>0", pph > 0, f"pph={pph}")

# SHIFT
r, hv = DELTA_SHIFT(5.0, power=1)
check("Δ-ops", "SHIFT 5*10=50", abs(r-50.0)<1e-15, f"got {r}")

# ROT
r, hv = DELTA_ROT(1+0j, angle_deg=90)
check("Δ-ops", "ROT 90°=i", abs(r.imag-1)<1e-10 and abs(r.real)<1e-10, f"got {r}")

# ZERO_DETECT
r, hv = DELTA_ZERO_DETECT((1.0, 2.0))
check("Δ-ops", "zero detect false", r == False, f"got {r}")

# composition
seq = compose_sequential(DELTA_SHIFT, DELTA_INV)
r, hv = seq(5.0, power=1)
check("Δ-ops", "SHIFT>>INV 50→0.02", abs(r-0.02)<1e-15, f"got {r}")

par = compose_parallel(DELTA_ADD, DELTA_MUL)
r, hv = par(3.0, 5.0)
check("Δ-ops", "parallel composition", len(r)==2, f"got {r}")

# pipe syntax
seq2 = DELTA_SHIFT >> DELTA_INV
check("Δ-ops", ">> pipe syntax", seq2.name == seq.name, f"{seq2.name} != {seq.name}")

# inv of DeltaOp
inv_add = DELTA_ADD.inv()
check("Δ-ops", "DELTA_ADD.inv() domain=codomain", inv_add is not None, "")
check("Δ-ops", "DELTA_ADD.inv().domain=R", inv_add.domain == 'R', "")

# domain checks
check("Δ-ops", "check_domain(O, assoc=False)", check_domain('O', assoc=False, commut=False, div=True), "")
check("Δ-ops", "check_domain(O, assoc=True)", not check_domain('O', assoc=True), "")
check("Δ-ops", "fallback S INV", select_fallback('Δ_INV', 'S') == 'Δ_ROBUST_INV', "")

# identity
id_op = identity('R')
r, hv = id_op(42.0)
check("Δ-ops", "identity(42)=42", abs(r-42)<1e-15, f"got {r}")

# ═══════════════════════════════════════════════════════════
# 1.6 Φ-algebra
# ═══════════════════════════════════════════════════════════
print("\n=== 1.6 Φ-algebra ===")

check("Φ", "PHI1_SHIFT.index=1", PHI1_SHIFT.index == 1, "")
check("Φ", "PHI2_PHASE.name=PHASE", PHI2_PHASE.name == 'PHASE', "")
check("Φ", "PHI3_FIXEDPOINT.symbol=⊙", PHI3_FIXEDPOINT.symbol == '⊙', "")
check("Φ", "PHI4_RECURSION.symbol=↺", PHI4_RECURSION.symbol == '↺', "")
check("Φ", "PHI5_PROJECTION.symbol=↓", PHI5_PROJECTION.symbol == '↓', "")

p = PHI1_SHIFT >> PHI2_PHASE
check("Φ", "path length=2", len(p) == 2, f"got {len(p)}")

s = periodic_orbit(4)
check("Φ", "periodic_orbit(4) length=4", len(s) == 4, f"got {len(s)}")

h = harmonic_series(1.0, 3)
check("Φ", "harmonic_series(1,3) length=6", len(h) == 6, f"got {len(h)}")

x = PHI1_SHIFT((0, 0), dx=1)
check("Φ", "SHIFT(0,0,dx=1)→(1,0)", x == (1, 0), f"got {x}")

# ═══════════════════════════════════════════════════════════
# 1.7 Swarm
# ═══════════════════════════════════════════════════════════
print("\n=== 1.7 Swarm ===")

swarm = IntelligentSwarm(seed=42)
for name in ['route_A', 'route_B', 'route_C', 'route_D']:
    swarm.register(name, potential=1.0)

probs = swarm.probabilities()
check("Swarm", "probs sum=1", abs(sum(probs.values()) - 1.0) < 1e-12, f"sum={sum(probs.values())}")
check("Swarm", "probs count=4", len(probs) == 4, f"got {len(probs)}")

# choose
chosen = swarm.decide(deterministic=True)
check("Swarm", "deterministic choose in routes", chosen.id in ['route_A', 'route_B', 'route_C', 'route_D'], "")

# update
for _ in range(50):
    chosen = swarm.decide(deterministic=False)
    swarm.update(chosen.id, success=(chosen.id != 'route_C'))

best = swarm.decide(deterministic=True)
check("Swarm", "best != route_C after 50 updates", best.id != 'route_C', f"best={best.id}")

# score range
for name in ['route_A', 'route_B', 'route_C', 'route_D']:
    s = swarm.score(name)
    check("Swarm", f"score({name})>0", s > 0, f"score={s}")

# SwarmNode
node = SwarmNode('test', potential=0.8)
check("Swarm", "node.potential=0.8", abs(node.potential - 0.8) < 1e-12, f"got {node.potential}")
node.reinforce(success=True)
check("Swarm", "node.visits=1", node.visits == 1, f"got {node.visits}")
check("Swarm", "node.success_rate=1", abs(node.success_rate - 1.0) < 1e-12, f"got {node.success_rate}")

# BayesReplacement
bayes = BayesReplacement(swarm)
bayes.update_belief('route_A', 0.9)
post = bayes.posterior(['route_A', 'route_B'])
check("Swarm", "posterior count=2", len(post) == 2, f"got {len(post)}")
pred = bayes.predict(['route_A', 'route_B'])
check("Swarm", "predict in candidates", pred in ('route_A', 'route_B'), f"pred={pred}")

# ═══════════════════════════════════════════════════════════
# 1.8 E8 Twist
# ═══════════════════════════════════════════════════════════
print("\n=== 1.8 E8 Twist ===")
te = TwistEngine()

# D8 roots
groups = te.triality_groups()
check("E8", "V count=112", len(groups.get('V', [])) == 112, f"got {len(groups.get('V',[]))}")
check("E8", "S+ count=64", len(groups.get('S+', [])) == 64, f"")
check("E8", "S- count=64", len(groups.get('S-', [])) == 64, f"")
check("E8", "total 240", sum(len(v) for v in groups.values()) == 240, f"")

# 2520-cycle
c35 = te.cycle_2520(35)
check("E8", "35° steps=72", c35['steps'] == 72, f"got {c35['steps']}")
c70 = te.cycle_2520(70)
check("E8", "70° steps=36", c70['steps'] == 36, f"got {c70['steps']}")
check("E8", "2520-cycle total", c35['total_deg'] == 2520, f"got {c35['total_deg']}")

# TWIST
result = te.twist(phase=0, config=(112, 64, 192))
check("E8", "twist has max_amplitude", 'max_amplitude' in result, "")
check("E8", "twist status in (CLOSED,OPEN)", result.get('status') in ('CLOSED','OPEN'), f"got {result.get('status')}")

# closure
closure = te.closure_angle(70)
check("E8", "closure has energy", 'energy' in closure, "")
check("E8", "closure status in (CLOSED,OPEN)", closure.get('status') in ('CLOSED','OPEN'), "")

# address_to_root
r = address_to_root(4, 3)
check("E8", "address_to_root length=8", len(r) == 8, f"got {len(r)}")

# root_properties
props = root_properties((2, 2, 0, 0, 0, 0, 0, 0))
check("E8", "D8 sector=D8", props['sector'] == 'D8', f"got {props['sector']}")
check("E8", "D8 norm2=8", props['norm2'] == 8, f"got {props['norm2']}")

# scan
scan = te.scan_configs()
check("E8", "scan results", len(scan) > 0, f"got {len(scan)}")
check("E8", "scan sorted by amplitude", scan[0].get('max_amplitude', 0) >= scan[-1].get('max_amplitude', 0),
      f"not sorted: first={scan[0].get('max_amplitude')} last={scan[-1].get('max_amplitude')}")

# summary
s = te.summary()
check("E8", "summary has triality", 'triality' in s, "")
check("E8", "summary triality sum=240", s.get('triality_sum') == 240, f"got {s.get('triality_sum')}")

# ═══════════════════════════════════════════════════════════
# 1.9 Doctor Bridge
# ═══════════════════════════════════════════════════════════
print("\n=== 1.9 Doctor Bridge ===")

from doctor_geo import (SwarmDoctor, GeoHealthVector,
    geo_to_opterium_hv, opterium_to_geo_hv, ROUTE_REGISTRY)

# geo → opterium HV
from doctor_geo import OPTERIUM_AVAILABLE as OP_AVAIL
ghv = GeoHealthVector(0.1, 0.2, 0.0, 0.05, 0.3, 0.15, 0.01)
ohv = geo_to_opterium_hv(ghv)
if OP_AVAIL:
    check("Doctor", "geo→opterium not None", ohv is not None, "")
    check("Doctor", "opterium has closure", abs(ohv.closure - 0.1) < 1e-10, f"closure={ohv.closure}")
else:
    check("Doctor", "opterium unavailable (skip)", True, "")

# opterium → geo HV
if OP_AVAIL and ohv is not None:
    ghv_back = opterium_to_geo_hv(ohv)
    check("Doctor", "opterium→geo roundtrip E_assoc", abs(ghv_back.E_assoc - 0.1) < 1e-10, f"{ghv_back.E_assoc}")
    check("Doctor", "opterium→geo roundtrip E_entropy", abs(ghv_back.E_entropy - 0.3) < 1e-10, f"{ghv_back.E_entropy}")
else:
    check("Doctor", "opterium→geo roundtrip (skip)", True, "")

# SwarmDoctor
sd = SwarmDoctor(swarm_seed=42)
for name, info in ROUTE_REGISTRY.items():
    sd.register_route(name, potential=info['default_potential'])

chosen = sd.choose_route(deterministic=True)
check("Doctor", "choose in registry", chosen in ROUTE_REGISTRY, f"chose {chosen}")

hv_ok = GeoHealthVector(0, 0, 0, 0, 0, 0, 0)
check("Doctor", "judge OK", sd.judge(hv_ok) == 'OK', f"got {sd.judge(hv_ok)}")

hv_bad = GeoHealthVector(E_assoc=0.8, E_precision=0.9, E_tension=0.95)
check("Doctor", "judge bad HV", sd.judge(hv_bad) in ('QUARANTINE', 'ROLLBACK'), f"got {sd.judge(hv_bad)}")

# reinforce
sd.reinforce_route('pytable_lookup', success=True)
score = sd.swarm.score('pytable_lookup')
check("Doctor", "reinforce updates score", score > 1.0, f"score={score}")

# quarantine
sd.quarantine_item('test', {'data': 42}, {'level': 'WARN'})
q = sd.get_quarantine('test')
check("Doctor", "quarantine store", q is not None and q['payload']['data'] == 42, f"got {q}")
sd.clear_quarantine()
check("Doctor", "quarantine clear", sd.get_quarantine('test') is None, "")

# full verdict
verdict = sd.opterium_verdict(hv_ok, context='test')
if OP_AVAIL:
    check("Doctor", "full verdict has level", 'level' in verdict, f"got {verdict}")
    check("Doctor", "full verdict has ok", 'ok' in verdict, "")
else:
    check("Doctor", "full verdict (fallback geo)", 'level' in verdict and 'ok' in verdict,
          f"got {verdict}")

# ═══════════════════════════════════════════════════════════
# SUMMARY
# ═══════════════════════════════════════════════════════════
print(f"\n{'='*50}")
print(f"УРОВЕНЬ 1: {PASS}/{PASS+FAIL} passed, {FAIL} FAILED")
if ERRORS:
    print(f"\nПРОБЛЕМЫ:")
    for module, test, msg in ERRORS:
        print(f"  [{module}] {test}: {msg}")
else:
    print(f"✅ Все тесты Уровня 1 прошли")
