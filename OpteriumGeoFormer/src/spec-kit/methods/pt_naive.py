"""
pt_naive.py  —  Method 1: Pure lookup matrix multiplication

Every integer value → Pt(v, 1). Matrix product = Σ_k PT.product(A[i][k].P, B[k][j].P).
Pure integer lookup. No float. No Pt creation in inner loop. Returns HealthVector.
"""

import sys, os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), '..', '..'))
sys.path.insert(0, os.path.join(os.path.dirname(__file__), '..'))
from delta_ops import HealthVector, HEALTH_OK
from arith_table import PT
from . import Pt, validate_shape, to_pt_matrix

def pt_naive(A, B):
    """Geometric matrix multiply via pure PT.product lookup.
    Zero Pt creation in inner loop. Zero float.

    A, B: lists of lists of int or Pt values.
    Returns (C_matrix, HealthVector).
    """
    A_pt = to_pt_matrix(A)
    B_pt = to_pt_matrix(B)
    m, k, n = validate_shape(A_pt, B_pt)

    C = [[0 for _ in range(n)] for _ in range(m)]

    for i in range(m):
        Ai = A_pt[i]
        Ci = C[i]
        for p in range(k):
            a_val = Ai[p].P
            Bp = B_pt[p]
            for j in range(n):
                Ci[j] += PT.product(a_val, Bp[j].P)

    C_pt = [[Pt(v, 1) for v in row] for row in C]
    return C_pt, HEALTH_OK


def pt_naive_fast(A, B):
    """Fast version: direct int accumulation via PT.product, skip intermediate Pt objects."""
    A_pt = to_pt_matrix(A)
    B_pt = to_pt_matrix(B)
    m, k, n = validate_shape(A_pt, B_pt)

    C = [[0 for _ in range(n)] for _ in range(m)]

    for i in range(m):
        Ai = A_pt[i]
        Ci = C[i]
        for p in range(k):
            a_val = Ai[p].P
            Bp = B_pt[p]
            for j in range(n):
                Ci[j] += PT.product(a_val, Bp[j].P)

    C_pt = [[Pt(v, 1) for v in row] for row in C]
    return C_pt, HEALTH_OK
