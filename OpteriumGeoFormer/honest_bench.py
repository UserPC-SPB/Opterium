#!/usr/bin/env python3
"""
HONEST BENCHMARK: GeoFormer vs NumPy (OpenBLAS) on identical matrices.
Same data, same machine, same CPU. No excuses.
"""
import sys, os, time, random
import numpy as np

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "native", "python"))
from geofield_native import GeoField

gf = GeoField(os.path.join(os.path.dirname(__file__), "src", "tables.ptbl"))

print(f"NumPy version: {np.__version__}")
print(f"NumPy BLAS: {np.show_config() or 'checking...'}")
print()

# Get BLAS info
try:
    import numpy.__config__ as nc
    info = nc.show(mode='dicts')
    print(f"BLAS backend: {info}")
except:
    pass

sizes = [16, 32, 64, 128, 256, 512, 1024]

print(f"{'Size':>8} | {'GeoFormer':>12} | {'NumPy':>12} | {'Ratio':>8} | {'Match?':>8}")
print("-" * 70)

for size in sizes:
    random.seed(42)
    raw_a = [random.randint(0, 100) for _ in range(size * size)]
    raw_b = [random.randint(0, 100) for _ in range(size * size)]

    a_np = np.array(raw_a, dtype=np.float64).reshape(size, size)
    b_np = np.array(raw_b, dtype=np.float64).reshape(size, size)

    # GeoFormer
    t0 = time.perf_counter()
    c_gf = gf.matmul(raw_a, size, size, raw_b, size)
    t1 = time.perf_counter()
    gf_ms = (t1 - t0) * 1000

    # NumPy
    t0 = time.perf_counter()
    c_np = a_np @ b_np
    t1 = time.perf_counter()
    np_ms = (t1 - t0) * 1000

    # Verify results match (within float tolerance)
    c_gf_arr = np.array(c_gf, dtype=np.float64).reshape(size, size)
    max_diff = np.max(np.abs(c_gf_arr - c_np))
    match = "YES" if max_diff < 1e-6 else f"NO ({max_diff:.1f})"

    ratio = gf_ms / np_ms if np_ms > 0 else float('inf')

    print(f"{size:>8} | {gf_ms:>10.1f} ms | {np_ms:>10.1f} ms | {ratio:>8.1f}x | {match:>8}")

print()
print("Ratio > 1 means GeoFormer is slower.")
print("Ratio < 1 means GeoFormer is faster.")
