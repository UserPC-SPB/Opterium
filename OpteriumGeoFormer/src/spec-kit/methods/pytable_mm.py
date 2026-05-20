"""
pytable_mm.py  —  Method 2: PyTable lookup matrix multiplication

Replace FP32 MUL with PyTable read: P = (S²−D²)//4 via precomputed table.
Each product = P_A · P_B (integer multiplication after lookup).
Sum is integer accumulation. All int, no float.
"""

import sys, os, struct
sys.path.insert(0, os.path.join(os.path.dirname(__file__), '..', '..'))
from delta_ops import HealthVector, HEALTH_OK
from . import Pt, validate_shape, to_pt_matrix, geo_mul, geo_add

PYTABLE_PATH = r"D:\gemma-4-geometric\dataset\PYTH_TABLE_1000.bin"
PYTABLE_CACHE = None

_PYT_RECORD = struct.Struct('<ihhbH')  # P(int32), S(int16), D(int16), pos(int8), gcd(uint16)

def load_pytable(path=None):
    """Load PyTable into memory. Returns (data_bytes, size)."""
    global PYTABLE_CACHE
    if PYTABLE_CACHE is not None:
        return PYTABLE_CACHE
    p = path or PYTABLE_PATH
    if not os.path.exists(p):
        raise FileNotFoundError(f"PyTable not found at {p}")
    with open(p, 'rb') as f:
        data = f.read()
    n_records = len(data) // _PYT_RECORD.size
    PYTABLE_CACHE = (data, n_records)
    return PYTABLE_CACHE


def read_pytable(S: int, D: int):
    """Look up P = (S²−D²)//4 from PyTable.

    Falls back to direct computation for values outside PyTable range [1, 1000].
    Args:
        S: sum coordinate (x + y)
        D: difference coordinate (x - y)
    Returns:
        P = x·y = (S²−D²)//4
    """
    data, n = load_pytable()
    x = (S + D) // 2
    y = (S - D) // 2
    if not (1 <= x <= 1000 and 1 <= y <= 1000):
        return (S * S - D * D) // 4  # direct formula fallback
    offset = ((x - 1) * 1000 + (y - 1)) * _PYT_RECORD.size
    P, S_file, D_file, pos, gcd = _PYT_RECORD.unpack_from(data, offset)
    if S_file != S or D_file != D:
        return (S * S - D * D) // 4  # integrity check fallback
    return P


def pytable_matmul(A, B):
    """Matrix multiply using PyTable lookups.

    Each element value = Pt(x, y) → P = x·y read from PyTable.
    Product = P_A · P_B (integer multiply). Sum = integer accumulation.
    Returns (C_matrix, HealthVector) with C as Pt values.
    """
    A_pt = to_pt_matrix(A)
    B_pt = to_pt_matrix(B)
    m, k, n = validate_shape(A_pt, B_pt)

    load_pytable()
    C = [[0 for _ in range(n)] for _ in range(m)]
    integrity_errors = 0

    for i in range(m):
        Ci = C[i]
        Ai = A_pt[i]
        for p in range(k):
            ap = Ai[p]
            Pa = read_pytable(ap.S, ap.D)
            Bp = B_pt[p]
            for j in range(n):
                bp = Bp[j]
                Pb = read_pytable(bp.S, bp.D)
                Ci[j] += Pa * Pb

    C_pt = [[Pt(v, 1) for v in row] for row in C]
    hv = HealthVector(
        E_assoc=integrity_errors / max(1, m * n * k),
        E_closure=0.0,
        E_precision=0.0,
    )
    return C_pt, hv


def pytable_matmul_cached(A, B):
    """PyTable method with per-matrix caching (avoid redundant lookups).

    Pre-extract all P values from A and B, then do int multiply-accumulate.
    Faster for large matrices.
    """
    A_pt = to_pt_matrix(A)
    B_pt = to_pt_matrix(B)
    m, k, n = validate_shape(A_pt, B_pt)

    load_pytable()
    A_P = [[read_pytable(pt.S, pt.D) for pt in row] for row in A_pt]
    B_P = [[read_pytable(pt.S, pt.D) for pt in row] for row in B_pt]

    C = [[0 for _ in range(n)] for _ in range(m)]
    for i in range(m):
        Ai = A_P[i]
        Ci = C[i]
        for p in range(k):
            a_val = Ai[p]
            Bp = B_P[p]
            for j in range(n):
                Ci[j] += a_val * Bp[j]

    return [[Pt(v, 1) for v in row] for row in C], HEALTH_OK
