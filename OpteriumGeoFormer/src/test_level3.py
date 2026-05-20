"""УРОВЕНЬ 3 — Интеграционное тестирование (3+ модулей)"""
import sys, os
SRC = os.path.dirname(__file__)
sys.path.insert(0, SRC)
sys.path.insert(0, os.path.join(SRC, 'spec-kit'))

from arith_table import PT
from methods import Pt, rmul, radd
from cube27 import Cube27
from hashgrid import HashGrid, geometric_attention
from delta_ops import HealthVector, HEALTH_OK, HEALTH_WARN
from swarm import IntelligentSwarm
from phi_algebra import PHI1_SHIFT
from geoformer import GeoFormer, GeometricEmbedding, SwarmTrainer, doctor_judge
from doctor_geo import SwarmDoctor, GeoHealthVector, ROUTE_REGISTRY
from e8_twist import TwistEngine, address_to_root, root_properties
from decimal import Decimal
import random, math

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
# 3.1 Full AI pipeline
# ═══════════════════════════════════════════════════════════
print("\n=== 3.1 Full AI pipeline ===")
"""
Вход: [1, 2, 3, 4, 5]
  PtTable → Pt(1,1)... Pt(5,1) → S/D/P
  HashGrid → geometric_attention
  GeoFormer → forward
  SwarmTrainer → train_step
  Doctor → verdict
"""

# 3.1a Pt → HashGrid → geometric_attention
tokens_pt = [Pt(i+1, 1) for i in range(10)]
pt_data = [(i, pt.S, pt.D, pt.P) for i, pt in enumerate(tokens_pt)]
attn_out = geometric_attention(pt_data, window=8)
check("Pipe3.1", "attention output len=10", len(attn_out) == 10, f"got {len(attn_out)}")
for o in attn_out:
    check("Pipe3.1", f"attention[{o['id']}] context>0", o.get('context', 0) > 0,
          f"context={o.get('context')} for id={o['id']}")

# 3.1b GeoFormer forward
random.seed(42)
gf = GeoFormer(layers=2, window=8)
out, hv = gf.forward([1, 2, 3, 4, 5])
check("Pipe3.1", "GeoFormer out len=5", len(out) == 5, f"got {len(out)}")
check("Pipe3.1", "GeoFormer hv ok type", isinstance(hv.ok, bool), "")

# 3.1c SwarmTrainer train_step
trainer = SwarmTrainer(gf)
result = trainer.train_step([1, 2, 3], [2, 4, 6])
for key in ('episode', 'score', 'success', 'hv_ok'):
    check("Pipe3.1", f"train_step has {key}", key in result, f"missing {key}")

# 3.1d Doctor verdict
verdict = doctor_judge(out, [1, 2, 3, 4, 5], hv)
check("Pipe3.1", "doctor_judge verdict known", verdict in ('OK', 'WARN', 'FAIL'), f"got {verdict}")

# 3.1e SwarmDoctor
sd = SwarmDoctor(swarm_seed=42)
for name, info in ROUTE_REGISTRY.items():
    sd.register_route(name, potential=info['default_potential'])
ghv = GeoHealthVector(0, 0, 0, 0, 0, 0, 0)
check("Pipe3.1", "SwarmDoctor OK", sd.judge(ghv) == 'OK', f"got {sd.judge(ghv)}")

# 3.1f Mantissa-rank pipeline: Pt(347, 3) = 0.347
p = Pt(347, 3)
val = p.to_real()
check("Pipe3.1", "Pt(347,3).to_real()=0.347", abs(val - 0.347) < 1e-12, f"got {val}")

# Pt(3,5) как координаты: S=8, D=-2, P=15 (не mantissa-rank)
p = Pt(3, 5)
check("Pipe3.1", "Pt(3,5).S=8", p.S == 8, f"got {p.S}")
check("Pipe3.1", "Pt(3,5).P=15", p.P == 15, f"got {p.P}")

# ═══════════════════════════════════════════════════════════
# 3.2 Full Theory pipeline
# ═══════════════════════════════════════════════════════════
print("\n=== 3.2 Full Theory pipeline ===")
"""
Δ-ops → Φ → Swarm → E8 → PtTable
"""

# 3.2a Swarm: register routes → choose → reinforce
swarm = IntelligentSwarm(seed=42)
for name in ['route_A', 'route_B', 'route_C']:
    swarm.register(name, potential=1.0)
for _ in range(20):
    chosen = swarm.decide(deterministic=False)
    swarm.update(chosen.id, success=True)
probs = swarm.probabilities()
check("Pipe3.2", "swarm probs sum≈1", abs(sum(probs.values()) - 1.0) < 1e-12, f"sum={sum(probs.values())}")

# 3.2b E8 roots → address_to_root → root_properties
te = TwistEngine()
e8_root = address_to_root(3, 5)
props = root_properties(e8_root)
check("Pipe3.2", "E8 root norm2=8 or 4", props['norm2'] in (8, 4), f"norm2={props['norm2']}")

# 3.2c E8 → PtTable: from_sd восстанавливает адрес
s = PT.sum(3, 5)
d = PT.diff(3, 5)
x, y = PT.from_sd(s, d)
check("Pipe3.2", "E8 address roundtrip", x==3 and y==5, f"({x},{y})")

# 3.2d E8 closure → PtTable
closure = te.closure_angle(70)
# closure energy это integer measure
check("Pipe3.2", "closure has route steps", len(closure.get('route', [])) > 0,
      f"no route steps")

# ═══════════════════════════════════════════════════════════
# 3.3 Cross-verify разных методов
# ═══════════════════════════════════════════════════════════
print("\n=== 3.3 Cross-verify ===")

# PtTable product == int product == Pt.P
for a in range(1, 50):
    for b in range(1, 50):
        p1 = PT.product(a, b)
        p2 = a * b
        p3 = Pt(a, 1).P * Pt(b, 1).P
        check("Cross", f"product({a},{b}) PT==int==Pt", p1==p2==p3,
              f"PT={p1} int={p2} Pt={p3}")
        if p1 != p2 or p2 != p3:
            break
    else:
        continue
    break
else:
    check("Cross", "product consistency 1..49×1..49", True, "")

# mantissa-rank float == Decimal
for r in [0.347, 2.34, -3.14159, 0.001, 100.0]:
    p = Pt.from_real(r)
    d_val = p.to_decimal()
    d_expected = Decimal(repr(r))
    check("Cross", f"from_real({r})→to_decimal match", abs(d_val - d_expected) < Decimal('1e-25'),
          f"from_real({r})→Pt({p.x},{p.y})→{d_val} expected {d_expected}")

# Cube27 verify: 100% hit для разных mantiss
c = Cube27()
for v in [0, 1, 42, 347, 999, 1000, 999999, 1000000, 123456789, 999999999999]:
    info = c.verify(v)
    check("Cross", f"Cube27 verify({v})", info['all_hit'],
          f"Cube27 verify({v}) miss: {info}")

# ═══════════════════════════════════════════════════════════
# 3.4 Обратный pipeline
# ═══════════════════════════════════════════════════════════
print("\n=== 3.4 Обратный pipeline ===")
"""
E8 root → address_to_root → PtTable(S,D) → from_sd → Pt → mantissa-rank → to_real
"""

e8_roots = [
    (2, 2, 0, 0, 0, 0, 0, 0),   # D8 root
    (1, 1, 1, 1, 1, 1, 1, 1),   # Spinor root
    (2, -2, 0, 0, 0, 0, 0, 0),  # D8 with negative
]

for r in e8_roots:
    # E8 root → (x,y) из первых двух координат
    x, y = r[0], r[1]
    # PtTable
    s = PT.S(abs(x), abs(y))
    d = PT.D(abs(x), abs(y))
    # from_sd
    x2, y2 = PT.from_sd(s, d)
    # Pt
    p = Pt(x2, y2)
    check("RevPipe", f"E8→Pt({x},{y})→(x,y)={x2},{y2}) S={s} D={d}",
          x2==abs(x) and y2==abs(y), f"")
    check("RevPipe", f"E8→Pt→PtTable P matches", p.P == abs(x)*abs(y),
          f"Pt.P={p.P} != {abs(x)*abs(y)}")

# ═══════════════════════════════════════════════════════════
# SUMMARY
# ═══════════════════════════════════════════════════════════
print(f"\n{'='*50}")
print(f"УРОВЕНЬ 3: {PASS}/{PASS+FAIL} passed, {FAIL} FAILED")
if ERRORS:
    print(f"\nПРОБЛЕМЫ:")
    for module, test, msg in ERRORS:
        print(f"  [{module}] {test}: {msg}")
else:
    print(f"✅ Все тесты Уровня 3 прошли")
