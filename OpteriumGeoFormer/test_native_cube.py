"""
Test Native GenerativeCube — Verifies Rust cube functions via Python CFFI bindings.
"""

import sys
import os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), 'native', 'python'))

from geofield_native import GeoField

# Initialize GeoField
table_path = os.path.join(os.path.dirname(__file__), 'src', 'tables.ptbl')
gf = GeoField(table_path)

print("Testing Native GenerativeCube...\n")

# Test 1: Get node
print("Test 1: cube_get_node")
n = gf.cube_get_node(10, 20, 30)
print(f"  get_node(10,20,30) = {n}")
assert n['x'] == 10 and n['y'] == 20 and n['z'] == 30
assert n['v'] == 6000
assert n['s'] == 60
assert n['d_body'] == 40
assert n['phase'] == 0  # 10%2=0, 20%2=0, 30%2=0

# Test 2: Cache (same node returned)
print("\nTest 2: Cache")
n2 = gf.cube_get_node(10, 20, 30)
assert n == n2
print("  ✅ Cache works")

# Test 3: Neighbors
print("\nTest 3: cube_get_neighbors")
# Create nodes nearby
for x in range(5, 15):
    for y in range(15, 25):
        for z in range(25, 35):
            gf.cube_get_node(x, y, z)

neighbors = gf.cube_get_neighbors(10, 20, 30, radius=5)
print(f"  Neighbors (10,20,30) radius=5: {len(neighbors)}")
assert len(neighbors) > 0
print("  ✅ Neighbors works")

# Test 4: Tension
print("\nTest 4: cube_tension")
t = gf.cube_tension(10, 20, 30, 11, 21, 31)
print(f"  tension((10,20,30), (11,21,31)) = {t}")
assert t > 0
print("  ✅ Tension works")

# Test 5: Analogy
print("\nTest 5: cube_analogy")
d = gf.cube_analogy(1, 1, 1, 2, 2, 2, 3, 3, 3)
print(f"  analogy((1,1,1), (2,2,2), (3,3,3)) = ({d['x']},{d['y']},{d['z']})")
assert d['x'] == 4 and d['y'] == 4 and d['z'] == 4
print("  ✅ Analogy works")

# Test 6: Morpho
print("\nTest 6: cube_morpho_link")
gf.cube_morpho_link(100, 100, 100, 101, 101, 101, 0.5)
gf.cube_morpho_link(100, 100, 100, 101, 101, 101, 0.8)
print("  ✅ Morpho link works")

# Test 7: Stats
print("\nTest 7: cube_stats")
s = gf.cube_stats()
print(f"  Stats: {s}")
assert s['cached_nodes'] > 0
assert s['address_space'] == 1024**3
print("  ✅ Stats works")

print("\n✅ All native cube tests passed!")
