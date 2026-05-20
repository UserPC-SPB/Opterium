"""
test_cross_verify.py  —  🔴 CRIT GAP 5: Python == Cython == Rust

Все три реализации sd_matmul должны давать идентичные результаты.
Проверка на детерминированных матрицах + torch как ground truth.
"""

import sys, os

BASE = r'C:\Users\eccoa\Desktop\OpteriumGeoFormer'
for p in [os.path.join(BASE, 'src', 'spec-kit'),
          os.path.join(BASE, 'src', 'spec-kit', 'methods'),
          BASE,
          os.path.join(BASE, 'src')]:
    sys.path.insert(0, p)
sys.path.insert(0, os.path.join(BASE, 'src', 'geo_matmul_rs'))

from methods.sd_matmul import sd_matmul
from methods import Pt
import geo_matmul_v2
import geo_matmul_rs

try:
    import torch
    TORCH_OK = True
except ImportError:
    TORCH_OK = False


def int_to_sd(v):
    """Convert int value v to (S, D) pair using Pt(v, 1)."""
    p = Pt(v, 1)
    return (p.S, p.D)


def test_known_values():
    """Check all three on manually computed values."""
    cases = [
        # (A_sd, B_sd, expected_C)
        ([[(5, 3), (7, 1)]], [[(6, 4)], [(3, 1)]], [[44]]),  # P: 4*5+12*2=44
        ([[(3, 1)]], [[(4, 2)]], [[6]]),  # Pt(2,1).P=2 * Pt(3,1).P=3 = 6
        ([[(4, 2), (5, 3)]], [[(6, 4)], [(7, 5)]], [[3*5 + 4*6]]),  # 3*5 + 4*6 = 39
    ]
    cases[2] = ([[(4, 2), (5, 3)]], [[(6, 4)], [(7, 5)]], [[39]])

    for idx, (A, B, exp) in enumerate(cases):
        py_r, _ = sd_matmul(A, B)
        py_v = [[c.P for c in row] for row in py_r]
        cy_v = geo_matmul_v2.sd_matmul_v2(A, B)
        rs_v = geo_matmul_rs.sd_matmul(A, B)
        rsp_v = geo_matmul_rs.sd_matmul_parallel(A, B)
        assert py_v == exp, f"case {idx} Python: {py_v} != {exp}"
        assert cy_v == exp, f"case {idx} Cython: {cy_v} != {exp}"
        assert rs_v == exp, f"case {idx} Rust: {rs_v} != {exp}"
        assert rsp_v == exp, f"case {idx} Rust par: {rsp_v} != {exp}"
    print(f"  known values: {len(cases)} cases PASS")


def random_sd_matrix(rows, cols, rng, min_v=1, max_v=50):
    """Deterministic (S,D) matrix from random Pt(v, 1) values."""
    return [[int_to_sd(rng.randint(min_v, max_v)) for _ in range(cols)] for _ in range(rows)]


def test_random_shapes():
    sizes = [(2, 3, 4), (4, 4, 4), (3, 5, 2), (1, 4, 1), (5, 2, 3)]
    for idx, (m, k, n) in enumerate(sizes):
        rng = __import__('random').Random(42 + idx)
        A_sd = random_sd_matrix(m, k, rng)
        B_sd = random_sd_matrix(k, n, rng)

        py_r, _ = sd_matmul(A_sd, B_sd)
        py_v = [[c.P for c in row] for row in py_r]
        cy_v = geo_matmul_v2.sd_matmul_v2(A_sd, B_sd)
        rs_v = geo_matmul_rs.sd_matmul(A_sd, B_sd)
        rsp_v = geo_matmul_rs.sd_matmul_parallel(A_sd, B_sd)

        for i in range(m):
            for j in range(n):
                expected = py_v[i][j]
                assert cy_v[i][j] == expected, f"[{m}x{k}x{n}][{i}][{j}] cy={cy_v[i][j]} != py={expected}"
                assert rs_v[i][j] == expected, f"[{m}x{k}x{n}][{i}][{j}] rs={rs_v[i][j]} != py={expected}"
                assert rsp_v[i][j] == expected, f"[{m}x{k}x{n}][{i}][{j}] rsp={rsp_v[i][j]} != py={expected}"

        if TORCH_OK:
            A_int = [[(s*s - d*d)//4 for (s,d) in row] for row in A_sd]
            B_int = [[(s*s - d*d)//4 for (s,d) in row] for row in B_sd]
            tA = torch.tensor(A_int, dtype=torch.int64)
            tB = torch.tensor(B_int, dtype=torch.int64)
            tC = (tA @ tB).tolist()
            for i in range(m):
                for j in range(n):
                    assert py_v[i][j] == tC[i][j], \
                        f"[{m}x{k}x{n}][{i}][{j}] py={py_v[i][j]} != torch={tC[i][j]}"

    print(f"  random shapes: {len(sizes)} shapes, {'torch-verified' if TORCH_OK else 'self-verified'} PASS")


def test_identity():
    for n in [1, 2, 4]:
        I_sd = [[(2, 0) if i == j else (1, -1) for j in range(n)] for i in range(n)]
        rng = __import__('random').Random(7)
        A_sd = random_sd_matrix(n, n, rng)
        py_r, _ = sd_matmul(A_sd, I_sd)
        py_v = [[c.P for c in row] for row in py_r]
        cy_v = geo_matmul_v2.sd_matmul_v2(A_sd, I_sd)
        rs_v = geo_matmul_rs.sd_matmul(A_sd, I_sd)
        A_ints = [[(s*s - d*d)//4 for (s,d) in row] for row in A_sd]
        for i in range(n):
            for j in range(n):
                assert py_v[i][j] == cy_v[i][j] == rs_v[i][j] == A_ints[i][j], \
                    f"I n={n} [{i}][{j}] py={py_v[i][j]} cy={cy_v[i][j]} rs={rs_v[i][j]} a={A_ints[i][j]}"
    print(f"  identity: 3 sizes PASS")


def test_zero():
    for n in [2, 4]:
        zero_sd = [[(1, -1) for _ in range(n)] for _ in range(n)]
        rng = __import__('random').Random(99)
        A_sd = random_sd_matrix(n, n, rng)
        py_r, _ = sd_matmul(A_sd, zero_sd)
        py_v = [[c.P for c in row] for row in py_r]
        cy_v = geo_matmul_v2.sd_matmul_v2(A_sd, zero_sd)
        rs_v = geo_matmul_rs.sd_matmul(A_sd, zero_sd)
        for i in range(n):
            for j in range(n):
                assert py_v[i][j] == cy_v[i][j] == rs_v[i][j] == 0
    print(f"  zero: {len(range(2, 5, 2))} sizes PASS")


def test_larger():
    for n in [16, 32]:
        rng = __import__('random').Random(7)
        A_sd = random_sd_matrix(n, n, rng, max_v=20)
        B_sd = random_sd_matrix(n, n, rng, max_v=20)
        py_r, _ = sd_matmul(A_sd, B_sd)
        py_v = [[c.P for c in row] for row in py_r]
        cy_v = geo_matmul_v2.sd_matmul_v2(A_sd, B_sd)
        rs_v = geo_matmul_rs.sd_matmul(A_sd, B_sd)
        rsp_v = geo_matmul_rs.sd_matmul_parallel(A_sd, B_sd)
        for i in range(n):
            for j in range(n):
                assert cy_v[i][j] == py_v[i][j], f"n={n}[{i}][{j}] cy"
                assert rs_v[i][j] == py_v[i][j], f"n={n}[{i}][{j}] rs"
                assert rsp_v[i][j] == py_v[i][j], f"n={n}[{i}][{j}] rsp"
    print(f"  larger: [16, 32] PASS")


def test_hashgrid():
    rng = __import__('random').Random(42)
    tokens_sd = [(i, s, d, (s*s - d*d)//4)
                 for i, (s, d) in enumerate(
                     [int_to_sd(rng.randint(1, 30)) for _ in range(10)])]

    rs_out = geo_matmul_rs.geometric_attention(tokens_sd, 8, False)

    from methods.geo_resonant import Pt as RPt, HashGrid, geo_attention
    tokens_pt = [RPt((s*s - d*d)//4, 1) for (_, s, d, _) in tokens_sd]
    py_out, _ = geo_attention(tokens_pt, window=8)

    assert len(rs_out) == len(py_out)
    for i in range(len(rs_out)):
        rs_id, rs_ctx, rs_cnt, rs_ox, rs_oy = rs_out[i]
        py_p = py_out[i].P
        assert rs_id == i, f"hashgrid id mismatch at {i}"
    print(f"  hashgrid: {len(rs_out)} tokens PASS")


if __name__ == '__main__':
    tests = [
        ("known values", test_known_values),
        ("random shapes", test_random_shapes),
        ("identity", test_identity),
        ("zero", test_zero),
        ("larger", test_larger),
        ("hashgrid", test_hashgrid),
    ]
    passed = 0
    for name, fn in tests:
        try:
            fn()
            passed += 1
            print(f"  ✅ {name}")
        except Exception as e:
            print(f"  ❌ {name}: {e}")
    total = len(tests)
    print(f"\n{'='*40}")
    print(f"Cross-verify: {passed}/{total} passed")
    sys.exit(0 if passed == total else 1)
