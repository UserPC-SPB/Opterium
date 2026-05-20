#!/usr/bin/env python3
"""
pure_lookup_mm.py  —  Гипотетическая pure-lookup matrix multiplication.

Ноль вычислений. Только memory access.
Каждый product = table read, каждый sum = table read.

Это proof-of-concept: показывает что МОЖЕТ БЫТЬ если убрать все вычисления.
"""

import sys, os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), '..', '..'))
sys.path.insert(0, os.path.join(os.path.dirname(__file__), '..'))
from arith_table import PT
from methods import Pt, validate_shape

# ── Pre-computed product lookup table ─────────────────────────────
# Для matrix multiply нужно: P_result = P_A * P_B
# Вместо умножения: PRODUCT_LOOKUP[P_A][P_B] = P_A * P_B
# Диапазон P: 0..1024*1024 = 1,048,576
# Но полная таблица 1M×1M = 1T entries — невозможно.
#
# Решение: gcd-decomposition уже есть в PtTable.
# Для pure lookup нужен другой подход:
#   - Разбить P на chunks, lookup по chunk
#   - Или: использовать PtTable._pairs для обратного lookup

# Для демонстрации: используем PtTable.product() как proxy для pure lookup.
# В реальной реализации это было бы:
#   - FPGA: BRAM lookup, 1 cycle
#   - C: mmap'd array, pointer dereference

def pure_lookup_mm(A, B):
    """Matrix multiply via pure table lookup.

    Каждый элемент:
      1. A[i][k] → (S1, D1) → P1 = TABLE[S1][D1]  # lookup
      2. B[k][j] → (S2, D2) → P2 = TABLE[S2][D2]  # lookup
      3. result += PRODUCT_TABLE[P1][P2]            # lookup + accumulate

    Ноль ALU операций (в идеале).
    """
    A_pt = [[Pt.from_int(v) if isinstance(v, int) else v for v in row] for row in A]
    B_pt = [[Pt.from_int(v) if isinstance(v, int) else v for v in row] for row in B]
    m, k, n = validate_shape(A_pt, B_pt)

    C = [[0 for _ in range(n)] for _ in range(m)]

    for i in range(m):
        Ai = A_pt[i]
        Ci = C[i]
        for p in range(k):
            # Step 1: lookup P1 from table (not compute)
            P1 = Ai[p].P  # в FPGA: BRAM[Ai[p].S][Ai[p].D]

            Bp = B_pt[p]
            for j in range(n):
                # Step 2: lookup P2 from table
                P2 = Bp[j].P  # в FPGA: BRAM[Bp[j].S][Bp[j].D]

                # Step 3: lookup product from table (not multiply)
                # В идеале: PRODUCT_TABLE[P1][P2]
                # Сейчас: используем PtTable.product как proxy
                prod = PT.product(P1, P2) if P1 <= 1024 and P2 <= 1024 else P1 * P2

                # Step 4: accumulate (в FPGA: adder chain)
                Ci[j] += prod

    C_pt = [[Pt(v, 1) for v in row] for row in C]
    return C_pt


def pure_lookup_mm_optimized(A, B):
    """Optimized: pre-extract all P values, then pure lookup accumulate.

    Минимизирует overhead: все table lookups done upfront.
    """
    # Pre-extract P values (в FPGA: parallel load from BRAM)
    A_P = [[(Pt.from_int(v).P if isinstance(v, int) else v.P) for v in row] for row in A]
    B_P = [[(Pt.from_int(v).P if isinstance(v, int) else v.P) for v in row] for row in B]

    m, k, n = len(A), len(A[0]), len(B[0])
    C = [[0] * n for _ in range(m)]

    for i in range(m):
        Ai = A_P[i]
        Ci = C[i]
        for p in range(k):
            a_val = Ai[p]
            Bp = B_P[p]
            for j in range(n):
                # Pure lookup: PRODUCT_TABLE[a_val][Bp[j]]
                # Для small values: direct table lookup
                if a_val <= 1024 and Bp[j] <= 1024:
                    prod = PT._P[a_val][Bp[j]] if a_val >= 0 and Bp[j] >= 0 else a_val * Bp[j]
                else:
                    prod = a_val * Bp[j]
                Ci[j] += prod

    return [[Pt(v, 1) for v in row] for row in C]


if __name__ == '__main__':
    import random, time
    random.seed(42)

    # Test correctness
    A = [[random.randint(1, 50) for _ in range(4)] for _ in range(4)]
    B = [[random.randint(1, 50) for _ in range(4)] for _ in range(4)]

    # Reference
    ref = [[sum(A[i][k] * B[k][j] for k in range(4)) for j in range(4)] for i in range(4)]

    # Pure lookup
    C1 = pure_lookup_mm(A, B)
    C1_ints = [[pt.P for pt in row] for row in C1]

    C2 = pure_lookup_mm_optimized(A, B)
    C2_ints = [[pt.P for pt in row] for row in C2]

    print("Reference:")
    for row in ref:
        print(f"  {row}")
    print("\nPure lookup:")
    for row in C1_ints:
        print(f"  {row}")
    print("\nPure lookup optimized:")
    for row in C2_ints:
        print(f"  {row}")

    match1 = all(ref[i][j] == C1_ints[i][j] for i in range(4) for j in range(4))
    match2 = all(ref[i][j] == C2_ints[i][j] for i in range(4) for j in range(4))
    print(f"\nPure lookup matches: {match1}")
    print(f"Pure lookup optimized matches: {match2}")

    # Benchmark
    print("\n--- Benchmark 64x64 ---")
    A64 = [[random.randint(1, 30) for _ in range(64)] for _ in range(64)]
    B64 = [[random.randint(1, 30) for _ in range(64)] for _ in range(64)]

    t0 = time.perf_counter()
    pure_lookup_mm(A64, B64)
    t1 = time.perf_counter()
    print(f"  pure_lookup_mm: {(t1-t0)*1000:.1f}ms")

    t0 = time.perf_counter()
    pure_lookup_mm_optimized(A64, B64)
    t1 = time.perf_counter()
    print(f"  pure_lookup_optimized: {(t1-t0)*1000:.1f}ms")

    from methods.sd_matmul import sd_matmul_from_ints
    t0 = time.perf_counter()
    sd_matmul_from_ints(A64, B64)
    t1 = time.perf_counter()
    print(f"  sd_matmul (current): {(t1-t0)*1000:.1f}ms")
