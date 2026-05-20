#!/usr/bin/env python3
"""
test_full_suite.py  —  Полный тест GPU-free Matrix Multiply (Opterium GeoFormer)

Покрывает все 5 методов geometric MM + geo_resonant (zero-MM attention).
Группы: A=Correctness, B=Pt Arithmetic, C=HealthVector, D=GeoResonant,
        E=Stress/Edge, F=Cross-verify, G=Benchmark.

Usage:
    python test_full_suite.py              # full suite
    python test_full_suite.py --group A    # only correctness
    python test_full_suite.py --quick      # A+B+C (fast)
    python test_full_suite.py --json       # JSON output
"""

import sys, os, time, json, random, argparse
from collections import defaultdict
from decimal import Decimal

# ── Path setup ─────────────────────────────────────────────────────
PROJECT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__)))))
SRC = os.path.join(PROJECT, 'src')
SK = os.path.join(SRC, 'spec-kit')
sys.path.insert(0, SRC)
sys.path.insert(0, SK)

# ── Imports ────────────────────────────────────────────────────────
from delta_ops import HealthVector, HEALTH_OK
from methods import Pt, rmul, radd, rsub, rdiv, geo_mul, geo_add
from methods import validate_shape, to_pt_matrix, sd_tuple_matrix
from methods.pt_naive import pt_naive, pt_naive_fast
from methods.pytable_mm import pytable_matmul, pytable_matmul_cached
from methods.sd_matmul import sd_matmul_from_ints, sd_matmul, sd_product
from methods.geo_resonant import geo_resonant, geo_attention, embed_int_sequence, HashGrid
from methods.baseline import torch_matmul, HAS_TORCH

random.seed(42)

# ── Helpers ────────────────────────────────────────────────────────
PASS, FAIL, SKIP = 'PASS', 'FAIL', 'SKIP'

class Result:
    def __init__(self, group, id, name, status, detail='', time_ms=0):
        self.group = group
        self.id = id
        self.name = name
        self.status = status
        self.detail = detail
        self.time_ms = time_ms

results = []

def rand_int_matrix(rows, cols, lo=-100, hi=100):
    return [[random.randint(lo, hi) for _ in range(cols)] for _ in range(rows)]

def extract_ints(C_pt):
    if isinstance(C_pt[0][0], Pt):
        return [[pt.P for pt in row] for row in C_pt]
    return C_pt

def int_matmul(A, B):
    m, k, n = len(A), len(A[0]), len(B[0])
    C = [[0]*n for _ in range(m)]
    for i in range(m):
        for p in range(k):
            a = A[i][p]
            Bp = B[p]
            for j in range(n):
                C[i][j] += a * Bp[j]
    return C

def matrices_match(C1, C2, tol=1):
    for i in range(len(C1)):
        for j in range(len(C1[0])):
            if abs(C1[i][j] - C2[i][j]) > tol:
                return False, i, j, C1[i][j], C2[i][j]
    return True, -1, -1, -1, -1

MM_METHODS = [
    ("pt_naive", pt_naive),
    ("pt_naive_fast", pt_naive_fast),
    ("sd_matmul", sd_matmul_from_ints),
]

try:
    from methods.pytable_mm import load_pytable
    load_pytable()
    MM_METHODS.append(("pytable_matmul", pytable_matmul))
    MM_METHODS.append(("pytable_cached", pytable_matmul_cached))
except Exception:
    pass

class SkipTest(Exception):
    pass

# ── GROUP A: Correctness ──────────────────────────────────────────

def test_A1_square_match():
    A = rand_int_matrix(4, 4, 1, 50)
    B = rand_int_matrix(4, 4, 1, 50)
    expected = int_matmul(A, B)
    for name, fn in MM_METHODS:
        C_pt, hv = fn(A, B)
        C = extract_ints(C_pt)
        ok, i, j, got, exp = matrices_match(C, expected)
        assert ok, f"[{name}] mismatch at [{i}][{j}]: got {got}, expected {exp}"

def test_A2_non_square():
    A = rand_int_matrix(3, 5, 1, 20)
    B = rand_int_matrix(5, 2, 1, 20)
    expected = int_matmul(A, B)
    for name, fn in MM_METHODS:
        C_pt, hv = fn(A, B)
        C = extract_ints(C_pt)
        assert len(C) == 3 and len(C[0]) == 2, f"[{name}] shape: {len(C)}x{len(C[0])}"
        ok, i, j, got, exp = matrices_match(C, expected)
        assert ok, f"[{name}] mismatch at [{i}][{j}]"

def test_A3_identity():
    A = rand_int_matrix(4, 4, 1, 50)
    I = [[1 if i == j else 0 for j in range(4)] for i in range(4)]
    for name, fn in MM_METHODS:
        C_pt, _ = fn(A, I)
        C = extract_ints(C_pt)
        for i in range(4):
            for j in range(4):
                assert C[i][j] == A[i][j], f"[{name}] identity [{i}][{j}]: {C[i][j]} != {A[i][j]}"

def test_A4_zero():
    A = rand_int_matrix(4, 4, 1, 50)
    Z = [[0]*4 for _ in range(4)]
    for name, fn in MM_METHODS:
        C1, _ = fn(A, Z)
        C1 = extract_ints(C1)
        assert all(C1[i][j] == 0 for i in range(4) for j in range(4)), f"[{name}] A·0 != 0"
    Z2 = [[0]*4 for _ in range(4)]
    for name, fn in MM_METHODS:
        C2, _ = fn(Z2, A)
        C2 = extract_ints(C2)
        assert all(C2[i][j] == 0 for i in range(4) for j in range(4)), f"[{name}] 0·A != 0"

def test_A5_negative():
    A = rand_int_matrix(4, 4, -50, 50)
    B = rand_int_matrix(4, 4, -50, 50)
    expected = int_matmul(A, B)
    for name, fn in MM_METHODS:
        C_pt, _ = fn(A, B)
        C = extract_ints(C_pt)
        ok, i, j, got, exp = matrices_match(C, expected, tol=2)
        assert ok, f"[{name}] neg mismatch at [{i}][{j}]: {got} vs {exp}"

def test_A6_large_values():
    A = rand_int_matrix(4, 4, 1, 1000000)
    B = rand_int_matrix(4, 4, 1, 1000000)
    expected = int_matmul(A, B)
    for name, fn in MM_METHODS:
        C_pt, _ = fn(A, B)
        C = extract_ints(C_pt)
        ok, i, j, got, exp = matrices_match(C, expected, tol=100)
        assert ok, f"[{name}] large mismatch at [{i}][{j}]: {got} vs {exp}"

def test_A7_all_methods_agree():
    A = rand_int_matrix(8, 8, 1, 30)
    B = rand_int_matrix(8, 8, 1, 30)
    refs = {}
    for name, fn in MM_METHODS:
        C_pt, _ = fn(A, B)
        refs[name] = extract_ints(C_pt)
    names = list(refs.keys())
    for i in range(1, len(names)):
        n1, n2 = names[0], names[i]
        ok, r, c, g, e = matrices_match(refs[n1], refs[n2])
        assert ok, f"{n1} vs {n2} at [{r}][{c}]: {g} vs {e}"

def test_A8_single_element():
    A = [[7]]
    B = [[3]]
    for name, fn in MM_METHODS:
        C_pt, _ = fn(A, B)
        C = extract_ints(C_pt)
        assert C[0][0] == 21, f"[{name}] 1x1: {C[0][0]} != 21"

def test_A9_scalar_edge():
    for n in [1, 2]:
        A = rand_int_matrix(n, n, 1, 10)
        B = rand_int_matrix(n, n, 1, 10)
        expected = int_matmul(A, B)
        for name, fn in MM_METHODS:
            C_pt, _ = fn(A, B)
            C = extract_ints(C_pt)
            ok, i, j, got, exp = matrices_match(C, expected)
            assert ok, f"[{name}] n={n} mismatch [{i}][{j}]"

# ── GROUP B: Pt Arithmetic ────────────────────────────────────────

def test_B1_from_int_roundtrip():
    for v in [0, 1, 42, -7, 1024, -1024]:
        assert Pt.from_int(v).P == v, f"from_int({v}).P = {Pt.from_int(v).P}"

def test_B2_from_sd_roundtrip():
    pairs = [(3, 5), (-3, 5), (0, 0), (50, 50)]
    for x, y in pairs:
        pt = Pt(x, y)
        pt2 = Pt.from_sd(pt.S, pt.D)
        assert pt2.x == x and pt2.y == y, f"from_sd({pt.S},{pt.D}) = ({pt2.x},{pt2.y}) != ({x},{y})"

def test_B3_parse_repr():
    assert repr(Pt.parse("347|3|")) == "347|3|"
    assert repr(Pt.parse("42|")) == "42|1|"
    assert Pt.parse("-347|3|").x == -347
    assert Pt.parse("-347|3|").y == 3

def test_B4_from_real_to_real():
    for v in [0.347, 2.34, -3.14159, 1e-6, 12345.6789]:
        p = Pt.from_real(v)
        back = p.to_real()
        rel_err = abs(back - v) / max(1, abs(v))
        assert rel_err < 1e-12, f"real roundtrip {v} -> {back}, rel_err={rel_err}"

def test_B5_decimal_roundtrip():
    d = Decimal('3.14159265358979323846')
    pp = Pt.from_decimal(d)
    back = pp.to_decimal()
    assert back == d, f"Decimal roundtrip: {back} != {d}"

def test_B6_inv():
    p = Pt(2, 1)
    pinv = p.inv()
    assert abs(pinv.to_decimal() - Decimal(5)) < Decimal('1e-25'), f"inv(0.2) = {pinv.to_decimal()}"
    p2 = Pt(347, 3)
    p2inv = p2.inv()
    product = p2.to_decimal() * p2inv.to_decimal()
    assert abs(product - Decimal(1)) < Decimal('1e-25'), f"inv product = {product}"

def test_B7_mantissa_rank():
    a = Pt.from_real(0.3)
    b = Pt.from_real(0.2)
    assert abs(rmul(a, b).to_real() - 0.06) < 1e-12
    assert abs(radd(a, b).to_real() - 0.5) < 1e-12
    assert abs(rsub(Pt.from_real(0.5), Pt.from_real(0.03)).to_real() - 0.47) < 1e-12
    assert abs(rdiv(Pt.from_real(0.3), Pt.from_real(0.2)).to_real() - 1.5) < 1e-12

def test_B8_geo_ops():
    a = Pt(3, 5)
    b = Pt(2, 7)
    g = geo_mul(a, b)
    assert g.x == 6 and g.y == 35, f"geo_mul: ({g.x},{g.y})"
    ga = geo_add(a, b)
    assert ga.x == 31 and ga.y == 35, f"geo_add: ({ga.x},{ga.y})"

# ── GROUP C: HealthVector ─────────────────────────────────────────

def test_C1_hv_ok():
    A = rand_int_matrix(4, 4, 1, 50)
    B = rand_int_matrix(4, 4, 1, 50)
    for name, fn in MM_METHODS:
        _, hv = fn(A, B)
        assert hv.ok, f"[{name}] HV not ok: {hv.max_channel}"

def test_C2_hv_warn_boundary():
    hv = HealthVector(E_assoc=0.34)
    assert hv.ok, "HV at 0.34 should be ok"
    hv2 = HealthVector(E_assoc=0.35)
    assert not hv2.ok, "HV at 0.35 should not be ok"
    assert hv2.warn, "HV at 0.35 should warn"

def test_C3_hv_merge():
    h1 = HealthVector(E_assoc=0.1, E_precision=0.2)
    h2 = HealthVector(E_assoc=0.3, E_precision=0.05)
    merged = h1.merge(h2)
    assert merged.E_assoc == 0.3, f"merge E_assoc: {merged.E_assoc}"
    assert merged.E_precision == 0.2, f"merge E_precision: {merged.E_precision}"

# ── GROUP D: GeoResonant (zero-MM attention) ──────────────────────

def test_D1_embed():
    tokens = embed_int_sequence([1, 2, 3, 4, 5])
    assert len(tokens) == 5
    assert all(isinstance(t, Pt) for t in tokens)
    assert tokens[0].x == 1 and tokens[0].y == 1

def test_D2_single_layer():
    tokens = embed_int_sequence([10, 20, 30, 40, 50])
    out, hv = geo_attention(tokens)
    assert len(out) == 5
    assert all(isinstance(t, Pt) for t in out)
    assert hv.ok

def test_D3_multi_layer():
    tokens = embed_int_sequence([1, 2, 3, 4, 5])
    out, hv = geo_resonant(tokens, layers=4, window=16)
    assert len(out) == 5
    assert hv.ok

def test_D4_empty():
    out, hv = geo_attention([])
    assert out == []
    assert hv.ok

def test_D5_single_token():
    tokens = [Pt(1, 1)]
    out, hv = geo_attention(tokens)
    assert len(out) == 1
    assert isinstance(out[0], Pt)

def test_D6_hashgrid():
    g = HashGrid(window=16)
    g.insert(0, Pt(10, 5))
    g.insert(1, Pt(12, 3))
    g.insert(2, Pt(100, 50))
    nb = g.lookup(11, 4)
    assert len(nb) >= 2, f"expected >=2 neighbors, got {len(nb)}"
    nb_empty = g.lookup(200, 100)
    assert len(nb_empty) == 0

def test_D7_output_shape():
    for n in [1, 5, 20, 100]:
        tokens = embed_int_sequence(list(range(n)))
        out, _ = geo_attention(tokens)
        assert len(out) == n, f"n={n}: output len={len(out)}"

# ── GROUP E: Stress & Edge Cases ──────────────────────────────────

def test_E1_n64():
    A = rand_int_matrix(64, 64, 1, 30)
    B = rand_int_matrix(64, 64, 1, 30)
    expected = int_matmul(A, B)
    for name, fn in [("pt_naive_fast", pt_naive_fast), ("sd_matmul", sd_matmul_from_ints)]:
        C_pt, _ = fn(A, B)
        C = extract_ints(C_pt)
        ok, i, j, got, exp = matrices_match(C, expected, tol=5)
        assert ok, f"[{name}] 64x64 mismatch [{i}][{j}]: {got} vs {exp}"

def test_E2_n128():
    A = rand_int_matrix(128, 128, 1, 20)
    B = rand_int_matrix(128, 128, 1, 20)
    expected = int_matmul(A, B)
    for name, fn in [("pt_naive_fast", pt_naive_fast), ("sd_matmul", sd_matmul_from_ints)]:
        C_pt, _ = fn(A, B)
        C = extract_ints(C_pt)
        ok, i, j, got, exp = matrices_match(C, expected, tol=10)
        assert ok, f"[{name}] 128x128 mismatch [{i}][{j}]"

def test_E3_shape_mismatch():
    A = rand_int_matrix(3, 5)
    B = rand_int_matrix(4, 2)
    for name, fn in MM_METHODS:
        try:
            fn(A, B)
            assert False, f"[{name}] no ValueError raised"
        except ValueError:
            pass

def test_E4_max_coord():
    A = [[1024, -1024], [0, 1024]]
    B = [[1024, 0], [-1024, 1024]]
    expected = int_matmul(A, B)
    for name, fn in MM_METHODS:
        C_pt, _ = fn(A, B)
        C = extract_ints(C_pt)
        ok, i, j, got, exp = matrices_match(C, expected)
        assert ok, f"[{name}] max_coord mismatch [{i}][{j}]"

def test_E5_no_float_in_sd():
    import inspect
    src = inspect.getsource(sd_product)
    assert 'float(' not in src, "sd_product should not use float()"
    assert 'import math' not in src, "sd_product should not import math"

def test_E6_pttable_summary():
    from arith_table import PT
    s = PT.summary()
    assert 'max_coord' in s
    assert s['max_coord'] == 1024
    assert s['cached'] == True

# ── GROUP F: Cross-verify ─────────────────────────────────────────

def test_F1_py_cython():
    try:
        methods_dir = os.path.join(SK, 'methods')
        if methods_dir not in sys.path:
            sys.path.insert(0, methods_dir)
        import geo_matmul_v2
        A = rand_int_matrix(16, 16, 1, 100)
        B = rand_int_matrix(16, 16, 1, 100)
        A_sd = [[(Pt.from_int(v).S, Pt.from_int(v).D) for v in row] for row in A]
        B_sd = [[(Pt.from_int(v).S, Pt.from_int(v).D) for v in row] for row in B]
        py_pt, _ = sd_matmul_from_ints(A, B)
        cy = geo_matmul_v2.sd_matmul_v2(A_sd, B_sd)
        for i in range(16):
            for j in range(16):
                py_val = py_pt[i][j].P if isinstance(py_pt[i][j], Pt) else py_pt[i][j]
                cy_val = cy[i][j].P if hasattr(cy[i][j], 'P') else cy[i][j]
                assert py_val == cy_val, f"[{i}][{j}] py={py_val} cy={cy_val}"
    except ImportError:
        raise SkipTest("Cython not installed")

def test_F2_py_rust():
    try:
        import geo_matmul_rs
        A = rand_int_matrix(16, 16, 1, 100)
        B = rand_int_matrix(16, 16, 1, 100)
        A_sd = [[(Pt.from_int(v).S, Pt.from_int(v).D) for v in row] for row in A]
        B_sd = [[(Pt.from_int(v).S, Pt.from_int(v).D) for v in row] for row in B]
        py_pt, _ = sd_matmul_from_ints(A, B)
        rs = geo_matmul_rs.sd_matmul(A_sd, B_sd)
        for i in range(16):
            for j in range(16):
                assert py_pt[i][j].P == rs[i][j], f"[{i}][{j}] py={py_pt[i][j].P} rs={rs[i][j]}"
    except ImportError:
        raise SkipTest("Rust not installed")

def test_F3_rust_seq_par():
    try:
        import geo_matmul_rs
        A = rand_int_matrix(16, 16, 1, 100)
        B = rand_int_matrix(16, 16, 1, 100)
        A_sd = [[(Pt.from_int(v).S, Pt.from_int(v).D) for v in row] for row in A]
        B_sd = [[(Pt.from_int(v).S, Pt.from_int(v).D) for v in row] for row in B]
        rs_seq = geo_matmul_rs.sd_matmul(A_sd, B_sd)
        rs_par = geo_matmul_rs.sd_matmul_parallel(A_sd, B_sd)
        for i in range(16):
            for j in range(16):
                assert rs_seq[i][j] == rs_par[i][j], f"[{i}][{j}] seq={rs_seq[i][j]} par={rs_par[i][j]}"
    except ImportError:
        raise SkipTest("Rust not installed")

def test_F4_all_equal_4x4():
    if not HAS_TORCH:
        raise SkipTest("torch not available")
    A = rand_int_matrix(4, 4, 1, 50)
    B = rand_int_matrix(4, 4, 1, 50)
    torch_result, _ = torch_matmul(A, B)
    for name, fn in MM_METHODS:
        C_pt, _ = fn(A, B)
        C = extract_ints(C_pt)
        ok, i, j, got, exp = matrices_match(C, torch_result)
        assert ok, f"[{name}] vs torch at [{i}][{j}]: {got} vs {exp}"

# ── GROUP G: Benchmark ────────────────────────────────────────────

def test_G1_walltime():
    sizes = [4, 16, 64]
    for n in sizes:
        A = rand_int_matrix(n, n, 1, 50)
        B = rand_int_matrix(n, n, 1, 50)
        for name, fn in MM_METHODS:
            t0 = time.perf_counter()
            fn(A, B)
            t1 = time.perf_counter()
    assert True

def test_G2_pttable_size():
    from arith_table import PT
    s = PT.summary()
    assert s['sd_size'] > 0, "sd table should have entries"
    assert s['pairs'] > 0, "pairs table should have entries"

# ── GROUP H: Language Handicap (H-factor) ─────────────────────────
# Поправка на уровень языка: torch=C/CUDA, Rust=C-level, Cython=C-extension,
# Pure Python=interpreted. Сравнивать raw ms некорректно — нужен H-factor.

def _bench_one(fn, A, B, warmup=3, runs=5):
    for _ in range(warmup):
        fn(A, B)
    times = []
    for _ in range(runs):
        t0 = time.perf_counter()
        fn(A, B)
        t1 = time.perf_counter()
        times.append((t1 - t0) * 1000)
    return sum(times) / len(times)

def test_H1_cython_vs_python():
    """Cython speedup over pure Python for same algorithm (sd_matmul)."""
    methods_dir = os.path.join(SK, 'methods')
    if methods_dir not in sys.path:
        sys.path.insert(0, methods_dir)
    try:
        import geo_matmul_v2
    except ImportError:
        raise SkipTest("Cython not installed")
    A = rand_int_matrix(64, 64, 1, 30)
    B = rand_int_matrix(64, 64, 1, 30)
    A_sd = [[(Pt.from_int(v).S, Pt.from_int(v).D) for v in row] for row in A]
    B_sd = [[(Pt.from_int(v).S, Pt.from_int(v).D) for v in row] for row in B]
    py_time = _bench_one(lambda a, b: sd_matmul_from_ints(a, b), A, B)
    cy_time = _bench_one(lambda a, b: geo_matmul_v2.sd_matmul_v2(a, b), A_sd, B_sd)
    h_factor = py_time / cy_time if cy_time > 0 else float('inf')
    assert h_factor > 1.0, f"Cython should be faster: py={py_time:.2f}ms cy={cy_time:.2f}ms"
    print(f"    H-factor Cython/Python @ 64x64: {h_factor:.1f}x (py={py_time:.1f}ms, cy={cy_time:.2f}ms)")

def test_H2_rust_vs_python():
    """Rust speedup over pure Python for same algorithm (sd_matmul)."""
    try:
        import geo_matmul_rs
    except ImportError:
        raise SkipTest("Rust not installed")
    A = rand_int_matrix(64, 64, 1, 30)
    B = rand_int_matrix(64, 64, 1, 30)
    A_sd = [[(Pt.from_int(v).S, Pt.from_int(v).D) for v in row] for row in A]
    B_sd = [[(Pt.from_int(v).S, Pt.from_int(v).D) for v in row] for row in B]
    py_time = _bench_one(lambda a, b: sd_matmul_from_ints(a, b), A, B)
    rs_time = _bench_one(lambda a, b: geo_matmul_rs.sd_matmul(a, b), A_sd, B_sd)
    h_factor = py_time / rs_time if rs_time > 0 else float('inf')
    assert h_factor > 1.0, f"Rust should be faster: py={py_time:.2f}ms rs={rs_time:.2f}ms"
    print(f"    H-factor Rust/Python @ 64x64: {h_factor:.1f}x (py={py_time:.1f}ms, rs={rs_time:.2f}ms)")

def test_H3_normalized_table():
    """Print level-adjusted comparison: all methods normalized to compiled baseline."""
    A = rand_int_matrix(64, 64, 1, 30)
    B = rand_int_matrix(64, 64, 1, 30)
    A_sd = [[(Pt.from_int(v).S, Pt.from_int(v).D) for v in row] for row in A]
    B_sd = [[(Pt.from_int(v).S, Pt.from_int(v).D) for v in row] for row in B]

    timings = {}
    for name, fn in MM_METHODS:
        timings[name] = _bench_one(fn, A, B)

    has_rust = False
    has_cython = False
    try:
        import geo_matmul_rs
        timings['rust_seq'] = _bench_one(lambda a, b: geo_matmul_rs.sd_matmul(a, b), A_sd, B_sd)
        timings['rust_par'] = _bench_one(lambda a, b: geo_matmul_rs.sd_matmul_parallel(a, b), A_sd, B_sd)
        has_rust = True
    except ImportError:
        pass
    try:
        methods_dir = os.path.join(SK, 'methods')
        if methods_dir not in sys.path:
            sys.path.insert(0, methods_dir)
        import geo_matmul_v2
        timings['cython'] = _bench_one(lambda a, b: geo_matmul_v2.sd_matmul_v2(a, b), A_sd, B_sd)
        has_cython = True
    except ImportError:
        pass

    if HAS_TORCH:
        timings['torch'] = _bench_one(lambda a, b: torch_matmul(a, b)[0], A, B)

    best = min(t for t in timings.values() if t > 0)
    print(f"\n    Level-adjusted comparison @ 64x64 (best={best:.2f}ms):")
    print(f"    {'Method':>20} {'Raw (ms)':>10} {'H-factor':>10} {'Level':>12}")
    level_map = {
        'torch': 'C/CUDA',
        'rust_seq': 'Rust/native',
        'rust_par': 'Rust/parallel',
        'cython': 'Cython/C-ext',
        'pt_naive': 'Pure Python',
        'pt_naive_fast': 'Pure Python',
        'sd_matmul': 'Pure Python',
        'pytable_matmul': 'Pure Python',
        'pytable_cached': 'Pure Python',
    }
    for name, t in sorted(timings.items(), key=lambda x: x[1]):
        h = t / best if best > 0 else 0
        level = level_map.get(name, 'unknown')
        print(f"    {name:>20} {t:>10.2f} {h:>10.1f}x {level:>12}")

    assert True

# ── Test Registry ─────────────────────────────────────────────────
TESTS = [
    ('A', 'A1', 'Square 4x4 match reference', test_A1_square_match),
    ('A', 'A2', 'Non-square 3x5x5x2', test_A2_non_square),
    ('A', 'A3', 'Identity A.I=A', test_A3_identity),
    ('A', 'A4', 'Zero A.0=0, 0.A=0', test_A4_zero),
    ('A', 'A5', 'Negative values', test_A5_negative),
    ('A', 'A6', 'Large values (10^6)', test_A6_large_values),
    ('A', 'A7', 'All methods agree (8x8)', test_A7_all_methods_agree),
    ('A', 'A8', 'Single element 1x1', test_A8_single_element),
    ('A', 'A9', 'Scalar edge n=1,2', test_A9_scalar_edge),
    ('B', 'B1', 'Pt.from_int roundtrip', test_B1_from_int_roundtrip),
    ('B', 'B2', 'Pt.from_sd roundtrip', test_B2_from_sd_roundtrip),
    ('B', 'B3', 'Pt parse/repr', test_B3_parse_repr),
    ('B', 'B4', 'Pt from_real/to_real', test_B4_from_real_to_real),
    ('B', 'B5', 'Pt Decimal roundtrip', test_B5_decimal_roundtrip),
    ('B', 'B6', 'Pt inv', test_B6_inv),
    ('B', 'B7', 'rmul/radd/rsub/rdiv', test_B7_mantissa_rank),
    ('B', 'B8', 'geo_mul/geo_add', test_B8_geo_ops),
    ('C', 'C1', 'HV ok for valid input', test_C1_hv_ok),
    ('C', 'C2', 'HV warn boundary', test_C2_hv_warn_boundary),
    ('C', 'C3', 'HV merge', test_C3_hv_merge),
    ('D', 'D1', 'Embed int sequence', test_D1_embed),
    ('D', 'D2', 'Single layer attention', test_D2_single_layer),
    ('D', 'D3', 'Multi-layer (4 layers)', test_D3_multi_layer),
    ('D', 'D4', 'Empty input', test_D4_empty),
    ('D', 'D5', 'Single token', test_D5_single_token),
    ('D', 'D6', 'HashGrid insert/lookup', test_D6_hashgrid),
    ('D', 'D7', 'Output shape invariant', test_D7_output_shape),
    ('E', 'E1', '64x64 correctness', test_E1_n64),
    ('E', 'E2', '128x128 correctness', test_E2_n128),
    ('E', 'E3', 'Shape mismatch ValueError', test_E3_shape_mismatch),
    ('E', 'E4', 'Max coord +/-1024', test_E4_max_coord),
    ('E', 'E5', 'No float in sd_product', test_E5_no_float_in_sd),
    ('E', 'E6', 'PtTable summary', test_E6_pttable_summary),
    ('F', 'F1', 'Py = Cython (16x16)', test_F1_py_cython),
    ('F', 'F2', 'Py = Rust seq (16x16)', test_F2_py_rust),
    ('F', 'F3', 'Rust seq = par (16x16)', test_F3_rust_seq_par),
    ('F', 'F4', 'All = torch (4x4)', test_F4_all_equal_4x4),
    ('G', 'G1', 'Wall-time benchmark', test_G1_walltime),
    ('G', 'G2', 'PtTable size', test_G2_pttable_size),
    ('H', 'H1', 'Cython vs Python H-factor', test_H1_cython_vs_python),
    ('H', 'H2', 'Rust vs Python H-factor', test_H2_rust_vs_python),
    ('H', 'H3', 'Level-adjusted table', test_H3_normalized_table),
]

# ── Runner ────────────────────────────────────────────────────────

def run_suite(groups=None):
    if groups:
        filtered = [(g, i, n, fn) for g, i, n, fn in TESTS if g in groups]
    else:
        filtered = TESTS

    print(f"\n  Opterium GeoFormer -- Full Test Suite")
    print(f"  {'='*60}")
    print(f"  Running {len(filtered)} tests\n")

    for group, id, name, fn in filtered:
        t0 = time.perf_counter()
        try:
            fn()
            t1 = time.perf_counter()
            results.append(Result(group, id, name, PASS, time_ms=(t1-t0)*1000))
        except SkipTest as e:
            t1 = time.perf_counter()
            results.append(Result(group, id, name, SKIP, str(e), time_ms=(t1-t0)*1000))
        except Exception as e:
            t1 = time.perf_counter()
            results.append(Result(group, id, name, FAIL, str(e), time_ms=(t1-t0)*1000))

    by_group = defaultdict(list)
    for r in results:
        by_group[r.group].append(r)

    for group in sorted(by_group.keys()):
        group_results = by_group[group]
        print(f"\n  [{'='*56}]")
        print(f"  Group {group}:")
        for r in group_results:
            icon = 'PASS' if r.status == PASS else ('SKIP' if r.status == SKIP else 'FAIL')
            print(f"    [{icon}] [{r.id}] {r.name}  ({r.time_ms:.1f}ms)")
            if r.status == FAIL:
                print(f"       -> {r.detail}")
            elif r.status == SKIP:
                print(f"       -> {r.detail}")

    passed = sum(1 for r in results if r.status == PASS)
    failed = sum(1 for r in results if r.status == FAIL)
    skipped = sum(1 for r in results if r.status == SKIP)
    total = len(results)

    print(f"\n  {'='*60}")
    print(f"  SUMMARY: {passed} passed, {failed} failed, {skipped} skipped, {total} total")

    if failed > 0:
        print(f"\n  FAILED TESTS:")
        for r in results:
            if r.status == FAIL:
                print(f"    [{r.group}:{r.id}] {r.name}: {r.detail}")

    benchmark_results = [r for r in results if r.group == 'G' and r.id == 'G1' and r.status == PASS]
    if benchmark_results:
        print(f"\n  {'='*60}")
        print(f"  BENCHMARK (wall-time ms):")
        print(f"  {'Size':>6} {'Method':>20} {'Time (ms)':>12}")
        print(f"  {'-'*40}")
        random.seed(42)
        for n in [4, 16, 64]:
            A = rand_int_matrix(n, n, 1, 50)
            B = rand_int_matrix(n, n, 1, 50)
            for name, fn in MM_METHODS:
                t0 = time.perf_counter()
                fn(A, B)
                t1 = time.perf_counter()
                print(f"  {n:>6} {name:>20} {(t1-t0)*1000:>10.2f}ms")

    return failed

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--group', nargs='+', help='Run specific groups (A B C D E F G)')
    parser.add_argument('--quick', action='store_true', help='Run A+B+C only')
    parser.add_argument('--json', action='store_true', help='JSON output')
    args = parser.parse_args()

    if args.quick:
        groups = {'A', 'B', 'C'}
    elif args.group:
        groups = set(args.group)
    else:
        groups = None

    failed = run_suite(groups)

    if args.json:
        output = {
            'total': len(results),
            'passed': sum(1 for r in results if r.status == PASS),
            'failed': sum(1 for r in results if r.status == FAIL),
            'skipped': sum(1 for r in results if r.status == SKIP),
            'results': [{'group': r.group, 'id': r.id, 'name': r.name,
                         'status': r.status, 'time_ms': round(r.time_ms, 2),
                         'detail': r.detail} for r in results]
        }
        print(json.dumps(output, indent=2))

    return 1 if failed > 0 else 0

if __name__ == '__main__':
    sys.exit(main())
