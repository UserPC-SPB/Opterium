"""
sd_matmul.py  —  Method 3: S-D Composition matrix multiplication

Operate entirely in (S, D) coordinate space. No intermediate P extraction.
The product rule in S-D space:
  Given A[i][k] = (S₁, D₁) and B[k][j] = (S₂, D₂):
    P₁ = (S₁² − D₁²) // 4
    P₂ = (S₂² − D₂²) // 4
    product = P₁ · P₂  (integer multiply)
  Sum over k: integer accumulation.

This is mathematically equivalent to Method 2 but works with raw (S,D) pairs,
making the geometric nature explicit: every operation is on triangle coordinates.
"""

import sys, os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), '..', '..'))
from delta_ops import HealthVector, HEALTH_OK
from . import Pt, validate_shape
from arith_table import PT

def sd_product(S1: int, D1: int, S2: int, D2: int) -> int:
    """Compute P1 * P2 via pure table lookup. Zero arithmetic.
    Uses PT.p_from_sd(S,D) → direct _SP[S][D+offset] read."""
    P1 = PT.p_from_sd(S1, D1)
    P2 = PT.p_from_sd(S2, D2)
    return PT.product(P1, P2)


def sd_matmul(A_sd, B_sd):
    """Matrix multiply in (S,D) coordinate space.

    A_sd, B_sd: lists of lists of (S, D) tuples.
    Returns (C_matrix, HealthVector) where C is list of lists of Pt.
    """
    m = len(A_sd)
    k = len(A_sd[0]) if m else 0
    n = len(B_sd[0]) if B_sd else 0
    if k != len(B_sd):
        raise ValueError(f"Shape mismatch: A:({m}×{k}) B:({len(B_sd)}×{n})")

    C = [[0 for _ in range(n)] for _ in range(m)]

    for i in range(m):
        Ai = A_sd[i]
        Ci = C[i]
        for p in range(k):
            S1, D1 = Ai[p]
            Bp = B_sd[p]
            for j in range(n):
                S2, D2 = Bp[j]
                Ci[j] += sd_product(S1, D1, S2, D2)

    C_pt = [[Pt(v, 1) for v in row] for row in C]
    return C_pt, HEALTH_OK


def sd_matmul_from_ints(A, B):
    """Convenience wrapper: accepts int matrices, converts to S-D form."""
    A_sd = _to_sd(A)
    B_sd = _to_sd(B)
    return sd_matmul(A_sd, B_sd)


def _to_sd(M):
    return [[(Pt.from_int(v).S, Pt.from_int(v).D) if isinstance(v, int)
             else (v.S, v.D) for v in row] for row in M]


def sd_product_formula():
    """Return the derived formula string for documentation."""
    return ("P_ij = Σ_k ((S_ik² − D_ik²) // 4) · ((S_kj² − D_kj²) // 4)\n"
            "All operations: int MUL, int ADD. Zero float.")
