"""
Test Native E8 Attention — Verifies Rust E8 functions via Python CFFI bindings.
"""

import sys
import os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), 'native', 'python'))

from geofield_native import GeoField

# Initialize GeoField
table_path = os.path.join(os.path.dirname(__file__), 'src', 'tables.ptbl')
gf = GeoField(table_path)

print("Testing Native E8 Attention...\n")

# Test 1: E8 root count
print("Test 1: E8 root count")
count = gf.e8_root_count()
print(f"  E8 roots: {count}")
assert count == 240

# Test 2: address_to_root
print("\nTest 2: address_to_root")
root1 = gf.e8_address_to_root(2, 2)
print(f"  address_to_root(2, 2) = {root1}")
assert len(root1) == 8
assert all(isinstance(c, int) for c in root1)

root2 = gf.e8_address_to_root(3, 3)
print(f"  address_to_root(3, 3) = {root2}")
assert len(root2) == 8

# Test 3: dot product
print("\nTest 3: dot product")
dot = gf.e8_dot_product(root1, root2)
print(f"  dot(root1, root2) = {dot}")
assert dot in [-8, -4, 0, 4, 8], f"Invalid dot product: {dot}"

# Test 4: self dot product
print("\nTest 4: self dot product")
dot_self = gf.e8_dot_product(root1, root1)
print(f"  dot(root1, root1) = {dot_self}")
assert dot_self == 8, f"Self dot product should be 8, got {dot_self}"

# Test 5: E8 attention
print("\nTest 5: E8 attention")
queries = [gf.e8_address_to_root(2, 2)]
keys = [gf.e8_address_to_root(2, 2), gf.e8_address_to_root(3, 3)]
values = [gf.e8_address_to_root(2, 2), gf.e8_address_to_root(3, 3)]

output = gf.e8_attention(queries, keys, values)
print(f"  attention output: {output}")
assert len(output) == 1
assert len(output[0]) == 8

print("\n✅ All native E8 tests passed!")
