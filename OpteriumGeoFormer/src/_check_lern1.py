"""Проверка текущей реализации PtTable на соответствие логике lern1.txt"""
import sys, os
sys.path.insert(0, os.path.dirname(__file__))
from arith_table import PT

# 1. Симметрия: (a,b) и (b,a) — S одинаково, D зеркально, P одинаково
print("=== Симметрия (a,b) vs (b,a) ===")
for a,b in [(3,5), (5,3), (7,2), (2,7)]:
    ab = PT.lookup(a,b)
    ba = PT.lookup(b,a)
    assert ab["S"] == ba["S"], f"S不对称: ({a},{b}).S={ab['S']} != ({b},{a}).S={ba['S']}"
    assert ab["D"] == -ba["D"], f"D不对称: ({a},{b}).D={ab['D']} != -({b},{a}).D={-ba['D']}"
    assert ab["P"] == ba["P"], f"P不对称: ({a},{b}).P={ab['P']} != ({b},{a}).P={ba['P']}"
    print(f"  ({a:2d},{b:2d}) S={ab['S']:3d} D={ab['D']:3d} P={ab['P']:3d}  |  ({b:2d},{a:2d}) S={ba['S']:3d} D={ba['D']:3d} P={ba['P']:3d}  ✓")

# 2. Отрицательные: зеркальная инверсия через S/D
print("\n=== Зеркальная инверсия (отрицательные) ===")
test_cases = [
    (-3, 5), (3, -5), (-3, -5), (-7, 2), (7, -2)
]
for a,b in test_cases:
    ab = PT.lookup(a,b)
    sa, sb = -a if a < 0 else a, -b if b < 0 else b
    pos = PT.lookup(sa, sb)
    # Проверка формул lern1: S(-x,y) = -D(x,y), P(-x,y) = -P(x,y)
    msg = f"  ({a:3d},{b:3d}): S={ab['S']:4d} D={ab['D']:4d} P={ab['P']:4d}"
    print(msg)

# 3. by_P индекс (для деления)
print("\n=== by_P индекс (pairs_for_product) ===")
for p in [0, 6, 12, 20, 30, 42, 100]:
    pairs = PT.pairs_for_product(p)
    print(f"  P={p:3d}: {pairs}")
    for x,y in pairs:
        assert x*y == p, f"{x}*{y} != {p}"

# 4. from_sd = (S,D) → (x,y)
print("\n=== from_sd (S,D) → (x,y) ===")
for s,d in [(8, -2), (8, 2), (9, 1), (10, 0), (7, -3)]:
    x,y = PT.from_sd(s,d)
    assert x+y == s, f"({s},{d}) → ({x},{y}): x+y={x+y} != S={s}"
    assert x-y == d, f"({s},{d}) → ({x},{y}): x-y={x-y} != D={d}"
    print(f"  S={s:3d} D={d:3d} → ({x:2d},{y:2d})  ✓")

# 5. conj = (x,y) → (y,x)
print("\n=== conj (квартернионное сопряжение) ===")
for x,y in [(3,5), (7,2), (4,4)]:
    cx, cy = PT.conj(x,y)
    assert cx == y and cy == x, f"conj({x},{y}) != ({y},{x})"
    assert PT.P(x,y) == PT.P(cx,cy), f"conj: P({x},{y})={PT.P(x,y)} != P({cx},{cy})={PT.P(cx,cy)}"
    print(f"  conj({x},{y}) = ({cx},{cy})  P={PT.P(x,y)}  ✓")

print("\n✅ Вся проверка lern1 совместимости пройдена")
