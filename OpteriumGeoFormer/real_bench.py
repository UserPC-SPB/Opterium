#!/usr/bin/env python3
"""
REAL BENCHMARK — GeoFormer matmul at ML-scale sizes.
"""
import sys, os, time
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "native", "python"))
from geofield_native import GeoField

gf = GeoField(os.path.join(os.path.dirname(__file__), "src", "tables.ptbl"))

sizes = [16, 64, 128, 256, 512, 1024]

print(f"{'Size':>8} | {'Time (ms)':>12} | {'GFLOP/s equiv':>16} | {'Status'}")
print("-" * 65)

for size in sizes:
    import random
    random.seed(42)
    a = [random.randint(0, 100) for _ in range(size * size)]
    b = [random.randint(0, 100) for _ in range(size * size)]

    # Warmup
    if size <= 128:
        gf.matmul(a[:256], 16, 16, b[:256], 16)

    t0 = time.perf_counter()
    try:
        c = gf.matmul(a, size, size, b, size)
        t1 = time.perf_counter()
        ms = (t1 - t0) * 1000
        # 2*n^3 ops (multiply+add)
        ops = 2 * size ** 3
        gflops = (ops / 1e9) / (ms / 1000) if ms > 0 else float('inf')
        print(f"{size:>8} | {ms:>12.1f} | {gflops:>16.2f} | OK")
    except Exception as e:
        print(f"{size:>8} | {'N/A':>12} | {'N/A':>16} | {str(e)[:40]}")
