"""
Test Native Debt System — Verifies Rust debt functions via Python CFFI bindings.
"""

import sys
import os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), 'native', 'python'))

from geofield_native import GeoField

# Initialize GeoField
table_path = os.path.join(os.path.dirname(__file__), 'src', 'tables.ptbl')
gf = GeoField(table_path)

print("Testing Native Debt System...\n")

# Test 1: debt_from_float
print("Test 1: debt_from_float")
d1 = gf.debt_from_float(0.23)
print(f"  0.23 → mantissa={gf.debt_mantissa(d1)}, debt={gf.debt_debt(d1)}")
assert gf.debt_mantissa(d1) == 23
assert gf.debt_debt(d1) == -2

d2 = gf.debt_from_float(3.4)
print(f"  3.4 → mantissa={gf.debt_mantissa(d2)}, debt={gf.debt_debt(d2)}")
assert gf.debt_mantissa(d2) == 34
assert gf.debt_debt(d2) == -1

# Test 2: debt_mul
print("\nTest 2: debt_mul")
d1 = gf.debt_from_float(3.4)
d2 = gf.debt_from_float(2.33)
result = gf.debt_mul(d1, d2)
print(f"  3.4 × 2.33 = {gf.debt_to_float(result)}")
assert abs(gf.debt_to_float(result) - 7.922) < 1e-9

# Test 3: debt_add
print("\nTest 3: debt_add")
d1 = gf.debt_from_float(0.1)
d2 = gf.debt_from_float(0.2)
result = gf.debt_add(d1, d2)
print(f"  0.1 + 0.2 = {gf.debt_to_float(result)}")
assert abs(gf.debt_to_float(result) - 0.3) < 1e-9

# Test 4: by_P index
print("\nTest 4: by_P index")
count = gf.byp_count(12)
print(f"  by_P[12] has {count} pairs")
assert count == 3  # (1,12), (2,6), (3,4)

for i in range(count):
    pair = gf.byp_get_pair(12, i)
    print(f"    Pair {i}: {pair}")

# Test 5: by_P find
print("\nTest 5: by_P find")
q = gf.byp_find(12, 3)
print(f"  12 / 3 = {q}")
assert q == 4

q = gf.byp_find(12, 4)
print(f"  12 / 4 = {q}")
assert q == 3

q = gf.byp_find(144, 12)
print(f"  144 / 12 = {q}")
assert q == 12

print("\n✅ All native debt tests passed!")
