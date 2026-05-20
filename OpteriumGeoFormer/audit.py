#!/usr/bin/env python3
"""
АУДИТ GEOFORMER ПЕРЕД ПУБЛИКАЦИЕЙ
4 следствия. Если все пройдены — невиновен, можно выпускать.
"""

import sys
import os
import time
import random
import shutil

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "native", "python"))

PASS = 0
FAIL = 0

def verdict(name, ok):
    global PASS, FAIL
    if ok:
        PASS += 1
        print(f"  ✅ {name}")
    else:
        FAIL += 1
        print(f"  ❌ {name}")

# ═══════════════════════════════════════════════════════════
# СЛЕДСТВИЕ 1: Допрос с пристрастием (Проверка на чистоту)
# ═══════════════════════════════════════════════════════════
print("=" * 60)
print("  СЛЕДСТВИЕ 1: Допрос с пристрастием")
print("  Ломаем таблицу — модуль должен упасть")
print("=" * 60)

table_path = os.path.join(os.path.dirname(__file__), "src", "tables.ptbl")
backup_path = table_path + ".bak"

# Переименовываем таблицу
if os.path.exists(table_path):
    shutil.move(table_path, backup_path)

# Пытаемся загрузить
try:
    from geofield_native import GeoField
    gf = GeoField(table_path)
    # Если дошли сюда — пробуем P(4,3)
    val = gf.P(4, 3)
    verdict("Модуль НЕ упал без таблицы — ХИТРЕЦ (вернул {} вместо ошибки)".format(val), False)
    gf._ptr = None  # prevent double free
except (OSError, TypeError, FileNotFoundError) as e:
    verdict(f"Модуль упал как положено: {type(e).__name__}", True)
except Exception as e:
    verdict(f"Модуль упал с неожиданной ошибкой: {type(e).__name__}: {e}", True)

# Возвращаем таблицу
if os.path.exists(backup_path):
    shutil.move(backup_path, table_path)
    print(f"  ↩ Таблица возвращена на место")

print()

# ═══════════════════════════════════════════════════════════
# СЛЕДСТВИЕ 2: Очная ставка с Эталоном (Проверка на точность)
# ═══════════════════════════════════════════════════════════
print("=" * 60)
print("  СЛЕДСТВИЕ 2: Очная ставка с Эталоном")
print("  1000 случайных пар: GeoFormer vs чистый Python")
print("=" * 60)

from geofield_native import GeoField
gf = GeoField(table_path)

random.seed(42)
p_ok = 0
s_ok = 0
d_ok = 0
n = 1000

for _ in range(n):
    x = random.randint(0, 1024)
    y = random.randint(0, 1024)

    p_etalon = x * y
    s_etalon = x + y
    d_etalon = abs(x - y)

    p_rust = gf.P(x, y)
    s_rust = gf.S(x, y)
    d_rust = gf.D(x, y)

    if p_rust == p_etalon:
        p_ok += 1
    else:
        verdict(f"P({x},{y}): GeoFormer={p_rust}, Эталон={p_etalon}", False)

    if s_rust == s_etalon:
        s_ok += 1
    else:
        verdict(f"S({x},{y}): GeoFormer={s_rust}, Эталон={s_etalon}", False)

    if d_rust == d_etalon:
        d_ok += 1
    else:
        verdict(f"D({x},{y}): GeoFormer={d_rust}, Эталон={d_etalon}", False)

verdict(f"P: {p_ok}/{n} совпало", p_ok == n)
verdict(f"S: {s_ok}/{n} совпало", s_ok == n)
verdict(f"D: {d_ok}/{n} совпало", d_ok == n)

print()

# ═══════════════════════════════════════════════════════════
# СЛЕДСТВИЕ 3: Следственный эксперимент с матрицами
# ═══════════════════════════════════════════════════════════
print("=" * 60)
print("  СЛЕДСТВИЕ 3: Матрицы 64×64")
print("  GeoFormer matmul vs тройной цикл на чистом Python")
print("=" * 60)

size = 64
random.seed(42)
a = [random.randint(0, 100) for _ in range(size * size)]
b = [random.randint(0, 100) for _ in range(size * size)]

# Эталон: тройной цикл на чистом Python
print("  Считаем эталон (тройной цикл, это займёт время)...")
t0 = time.perf_counter()
c_etalon = [0] * (size * size)
for i in range(size):
    for j in range(size):
        s = 0
        for k in range(size):
            s += a[i * size + k] * b[k * size + j]
        c_etalon[i * size + j] = s
t1 = time.perf_counter()
print(f"  Эталон посчитан за {(t1 - t0) * 1000:.0f} мс")

# GeoFormer
print("  Считаем через GeoFormer...")
t0 = time.perf_counter()
c_rust = gf.matmul(a, size, size, b, size)
t1 = time.perf_counter()
print(f"  GeoFormer посчитал за {(t1 - t0) * 1000:.1f} мс")

# Сравнение
mismatches = 0
for i in range(size * size):
    if c_rust[i] != c_etalon[i]:
        mismatches += 1
        if mismatches <= 5:
            row, col = divmod(i, size)
            print(f"  ❌ [{row},{col}]: GeoFormer={c_rust[i]}, Эталон={c_etalon[i]}")

verdict(f"64×64 matmul: {mismatches} расхождений из {size*size}", mismatches == 0)

print()

# ═══════════════════════════════════════════════════════════
# СЛЕДСТВИЕ 4: Допрос свидетелей (Стресс-тест)
# ═══════════════════════════════════════════════════════════
print("=" * 60)
print("  СЛЕДСТВИЕ 4: Стресс-тест (крайние значения)")
print("=" * 60)

# P(0, 1024) = 0
verdict("P(0, 1024) = 0", gf.P(0, 1024) == 0)

# P(1024, 1024) = 1048576
verdict("P(1024, 1024) = 1048576", gf.P(1024, 1024) == 1048576)

# P(0, 0) = 0
verdict("P(0, 0) = 0", gf.P(0, 0) == 0)

# P(1, 1) = 1
verdict("P(1, 1) = 1", gf.P(1, 1) == 1)

# S(0, 0) = 0
verdict("S(0, 0) = 0", gf.S(0, 0) == 0)

# S(1024, 1024) = 2048
verdict("S(1024, 1024) = 2048", gf.S(1024, 1024) == 2048)

# D(0, 0) = 0
verdict("D(0, 0) = 0", gf.D(0, 0) == 0)

# D(1024, 0) = 1024
verdict("D(1024, 0) = 1024", gf.D(1024, 0) == 1024)

# D(0, 1024) = 1024
verdict("D(0, 1024) = 1024", gf.D(0, 1024) == 1024)

# proximity(0) = 10000
verdict("proximity(0) = 10000", gf.proximity(0) == 10000)

# proximity(4096) = 2 (SCALE / (1 + 4096) = 10000 / 4097)
verdict("proximity(4096) = 2", gf.proximity(4096) == 2)

# isqrt(0) = 0
verdict("isqrt(0) = 0", gf.isqrt(0) == 0)

# isqrt(1) = 1
verdict("isqrt(1) = 1", gf.isqrt(1) == 1)

# isqrt(1048576) = 1024
verdict("isqrt(1048576) = 1024", gf.isqrt(1048576) == 1024)

# 1×1 matmul
verdict("1×1 matmul: [3]×[7]=[21]", gf.matmul([3], 1, 1, [7], 1) == [21])

# 1×1 matmul с нулём
verdict("1×1 matmul: [0]×[5]=[0]", gf.matmul([0], 1, 1, [5], 1) == [0])

# Внимание с одним токеном
tokens_1 = [0, 10, 10, 100]
r1 = gf.attention(tokens_1, 1, 5)
verdict("attention(1 токен): id=0, neighbors=1", r1[0] == 0 and r1[3] == 1)

# Внимание с нулевыми координатами
tokens_zero = [0, 0, 0, 0, 1, 0, 0, 0]
r0 = gf.attention(tokens_zero, 2, 5)
verdict("attention(нулевые коорд): не падает", len(r0) == 10)

# p_from_sd(0, 0) = 0
verdict("p_from_sd(0, 0) = 0", gf.p_from_sd(0, 0) == 0)

# p_from_sd(2, 0) = 1 (P = (S² - D²)/4 = (4-0)/4 = 1)
verdict("p_from_sd(2, 0) = 1", gf.p_from_sd(2, 0) == 1)

# product(0, 5) = 0
verdict("product(0, 5) = 0", gf.product(0, 5) == 0)

# product(10, 10) = 100
verdict("product(10, 10) = 100", gf.product(10, 10) == 100)

print()

# ═══════════════════════════════════════════════════════════
# ИТОГОВЫЙ ВЕРДИКТ
# ═══════════════════════════════════════════════════════════
print("=" * 60)
print(f"  ИТОГО: {PASS} ✅, {FAIL} ❌")
print("=" * 60)

if FAIL == 0:
    print()
    print("  🟢 GeoFormer НЕВИНОВЕН. Все 4 следствия пройдены.")
    print("  Можно выпускать в люди.")
    print()
else:
    print()
    print(f"  🔴 GeoFormer ВИНОВЕН. {FAIL} нарушений обнаружено.")
    print("  Садимся разбираться.")
    print()
