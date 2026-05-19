#!/usr/bin/env python3
"""
Opterium GeoFormer — Demo
Показывает, что всё работает: lookup, matmul, attention.
"""

import sys
import os
import time

# Добавляем путь к native/python
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "native", "python"))

from geofield_native import GeoField

def main():
    print("=" * 60)
    print("  Opterium GeoFormer — Demo")
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
    print(f"  P(4, 3) = {gf.P(4, 3)}")          # 12
    print(f"  S(4, 3) = {gf.S(4, 3)}")          # 7
    print(f"  D(4, 3) = {gf.D(4, 3)}")          # 1
    print(f"  p_from_sd(7, 1) = {gf.p_from_sd(7, 1)}")  # 12
    print(f"  proximity(0) = {gf.proximity(0)}")      # 10000
    print(f"  proximity(1) = {gf.proximity(1)}")      # 5000
    print(f"  isqrt(144) = {gf.isqrt(144)}")          # 12
    print()

    # 2. Matrix Multiply
    print("─" * 40)
    print("  2. Matrix Multiply (Rust)")
    print("─" * 40)

    # A = [[1, 2], [3, 4]]  B = [[5, 6], [7, 8]]
    # C = A × B = [[19, 22], [43, 50]]
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

    # 3 токена: [id, S, D, P]
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

    # 4. Benchmark
    print("─" * 40)
    print("  4. Benchmark (16×16 matmul)")
    print("─" * 40)

    size = 16
    a_big = list(range(1, size * size + 1))
    b_big = list(range(size * size, 0, -1))

    # Warmup
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
    print("  ✅ Всё работает!")
    print("=" * 60)

if __name__ == "__main__":
    main()
