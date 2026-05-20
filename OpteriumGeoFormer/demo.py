#!/usr/bin/env python3
"""
Opterium GeoFormer — Full Demo
Показывает все модули: lookup, matmul, attention, verifier, debt, E8, pipeline.
"""

import sys
import os
import time

# Добавляем пути
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "native", "python"))
sys.path.insert(0, os.path.dirname(__file__))

from geofield_native import GeoField
from verifier import OpteriumVerifier
from opterium import TokenEncoder, TokenDecoder, NavigationCore, OpteriumPipeline

def main():
    print("=" * 60)
    print("  Opterium GeoFormer — Full Demo")
    print("=" * 60)
    print()

    # Путь к таблицам
    table_path = os.path.join(os.path.dirname(__file__), "src", "tables.ptbl")
    if not os.path.exists(table_path):
        print(f"ERROR: таблицы не найдены: {table_path}")
        print("Запустите сначала: python src/table_format.py")
        sys.exit(1)

    print(f"Загрузка таблиц: {table_path}")
    t0 = time.perf_counter()
    gf = GeoField(table_path)
    t1 = time.perf_counter()
    print(f"  Загружено за {(t1 - t0) * 1000:.1f} мс")
    print(f"  Размер таблиц: {gf.table_size() / 1024 / 1024:.1f} MB")
    print(f"  Max coord: {gf.max_coord()}")
    print()

    # 1. Pure Lookup
    print("─" * 40)
    print("  1. Pure Lookup (таблицы)")
    print("─" * 40)
    print(f"  P(4, 3) = {gf.P(4, 3)}")
    print(f"  S(4, 3) = {gf.S(4, 3)}")
    print(f"  D(4, 3) = {gf.D(4, 3)}")
    print(f"  p_from_sd(7, 1) = {gf.p_from_sd(7, 1)}")
    print(f"  proximity(0) = {gf.proximity(0)}")
    print(f"  proximity(1) = {gf.proximity(1)}")
    print(f"  isqrt(144) = {gf.isqrt(144)}")
    print()

    # 2. Matrix Multiply
    print("─" * 40)
    print("  2. Matrix Multiply (Rust)")
    print("─" * 40)

    a = [1, 2, 3, 4]
    b = [5, 6, 7, 8]

    t0 = time.perf_counter()
    c = gf.matmul(a, 2, 2, b, 2)
    t1 = time.perf_counter()

    print(f"  A = [[1, 2], [3, 4]]")
    print(f"  B = [[5, 6], [7, 8]]")
    print(f"  C = A × B = [[{c[0]}, {c[1]}], [{c[2]}, {c[3]}]]")
    print(f"  Время: {(t1 - t0) * 1000:.2f} мс")
    print(f"  Ожидание: [[19, 22], [43, 50]]")
    ok = c == [19, 22, 43, 50]
    print(f"  Результат: {'✅ OK' if ok else '❌ FAIL'}")
    print()

    # 3. Geometric Attention
    print("─" * 40)
    print("  3. Geometric Attention (Rust)")
    print("─" * 40)

    tokens = [
        0, 10, 10, 100,
        1, 11, 10, 110,
        2, 20, 20, 400,
    ]

    t0 = time.perf_counter()
    result = gf.attention(tokens, 3, 5)
    t1 = time.perf_counter()

    print(f"  Токены: 3 шт [id, S, D, P]")
    print(f"  Результат: [id, ctx_S, ctx_D, neighbors, output_P]")
    for i in range(3):
        base = i * 5
        print(f"    Токен {result[base]}: ctx_S={result[base+1]}, ctx_D={result[base+2]}, "
              f"neighbors={result[base+3]}, P={result[base+4]}")
    print(f"  Время: {(t1 - t0) * 1000:.2f} мс")
    print()

    # 4. Debt System
    print("─" * 40)
    print("  4. Debt System (дробные числа)")
    print("─" * 40)

    d1 = gf.debt_from_float(3.4)
    d2 = gf.debt_from_float(2.33)
    result = gf.debt_mul(d1, d2)
    print(f"  3.4 × 2.33 = {gf.debt_to_float(result):.4f}")

    d1 = gf.debt_from_float(0.1)
    d2 = gf.debt_from_float(0.2)
    result = gf.debt_add(d1, d2)
    print(f"  0.1 + 0.2 = {gf.debt_to_float(result):.4f}")

    print(f"  by_P[12] = {gf.byp_count(12)} пар")
    print(f"  12 / 3 = {gf.byp_find(12, 3)}")
    print()

    # 5. E8 Attention
    print("─" * 40)
    print("  5. E8 Root Lattice (on-the-fly)")
    print("─" * 40)

    print(f"  E8 roots count: {gf.e8_root_count()}")
    root1 = gf.e8_address_to_root(2, 2)
    root2 = gf.e8_address_to_root(3, 3)
    print(f"  address_to_root(2,2) = {root1}")
    print(f"  address_to_root(3,3) = {root2}")
    dot = gf.e8_dot_product(root1, root2)
    print(f"  dot(root1, root2) = {dot}")
    print()

    # 6. Verifier
    print("─" * 40)
    print("  6. Verifier (проверка утверждений)")
    print("─" * 40)

    v = OpteriumVerifier()
    claims = [
        "234 × 567 = 132678",
        "12 × 12 = 144",
        "1000 + 1000 = 2000",
        "144 / 12 = 12",
        "√144 = 12",
        "12 × 12 = 145",  # fail case
    ]
    for claim in claims:
        result = v.verify(claim)
        status = "✅" if result.get('valid') else "❌"
        print(f"  {status} {claim}")
    print()

    # 7. Architecture Flip Pipeline
    print("─" * 40)
    print("  7. Architecture Flip (Encoder → Nav → Decoder)")
    print("─" * 40)

    pipeline = OpteriumPipeline(vocab_size=100, embed_dim=32, max_coord=1024)

    output = pipeline.forward(42)
    print(f"  forward(42) = {output}")

    token_d = pipeline.analogy(10, 20, 30)
    print(f"  analogy(10, 20, 30) = {token_d}")

    valid, witness = pipeline.verify("5 × 6 = 30")
    print(f"  verify('5 × 6 = 30') = {valid}")

    witness = pipeline.get_witness(42)
    print(f"  witness(42): P={witness['witness']['P']}, S={witness['witness']['S']}, D={witness['witness']['D']}")
    print()

    # 8. Generative 3D Cube
    print("─" * 40)
    print("  8. Generative 3D Cube (Rust)")
    print("─" * 40)

    n = gf.cube_get_node(10, 20, 30)
    print(f"  get_node(10,20,30) = V={n['v']}, S={n['s']}, D_body={n['d_body']}")

    neighbors = gf.cube_get_neighbors(10, 20, 30, radius=5)
    print(f"  Neighbors (radius=5): {len(neighbors)}")

    t = gf.cube_tension(10, 20, 30, 11, 21, 31)
    print(f"  tension((10,20,30), (11,21,31)) = {t}")

    d = gf.cube_analogy(1, 1, 1, 2, 2, 2, 3, 3, 3)
    print(f"  analogy((1,1,1), (2,2,2), (3,3,3)) = ({d['x']},{d['y']},{d['z']})")

    s = gf.cube_stats()
    print(f"  Stats: {s['cached_nodes']} nodes, {s['address_space']:,} address space")
    print()

    # 9. Benchmark
    print("─" * 40)
    print("  9. Benchmark (16×16 matmul)")
    print("─" * 40)

    size = 16
    a_big = list(range(1, size * size + 1))
    b_big = list(range(size * size, 0, -1))

    gf.matmul(a_big, size, size, b_big, size)

    N = 100
    t0 = time.perf_counter()
    for _ in range(N):
        gf.matmul(a_big, size, size, b_big, size)
    t1 = time.perf_counter()

    avg_ms = (t1 - t0) / N * 1000
    print(f"  {N} итераций × {size}×{size} matmul")
    print(f"  Среднее время: {avg_ms:.2f} мс")
    print()

    print("=" * 60)
    print("  ✅ Все модули работают!")
    print("=" * 60)
    print("""
Модули:
  1. Pure Lookup — таблицы P, S, D, SP, prox, isqrt
  2. Matrix Multiply — Rust, Rayon parallel
  3. Geometric Attention — hashgrid proximity
  4. Debt System — (mantissa, debt) pairs, by_P index
  5. E8 Root Lattice — 240 roots on-the-fly
  6. Verifier — проверка арифметических утверждений
  7. Architecture Flip — encoder → nav → decoder
  8. Generative 3D Cube — O(1) node generation, bucket index
  9. Benchmark — производительность
""")

if __name__ == "__main__":
    main()
