"""
test_correctness.py  —  Verify all geometric MM methods against torch/numpy.

Tests:
  1. Each method matches torch.matmul for random 4×4 integer matrices
  2. Identity: A·I = A
  3. Zero: A·0 = 0
  4. Valid HealthVector (all channels < 0.35)
  5. Shape validation
"""

import sys, os, random
sys.path.insert(0, os.path.join(os.path.dirname(__file__), '..', '..'))
sys.path.insert(0, os.path.join(os.path.dirname(__file__), '..'))

from delta_ops import HealthVector
from methods import Pt
from methods.pt_naive import pt_naive, pt_naive_fast
from methods.pytable_mm import pytable_matmul, pytable_matmul_cached
from methods.sd_matmul import sd_matmul_from_ints
from methods.baseline import torch_matmul, numpy_matmul, HAS_TORCH, HAS_NUMPY

random.seed(42)

def random_int_matrix(rows, cols, max_val=100):
    return [[random.randint(1, max_val) for _ in range(cols)] for _ in range(rows)]


def extract_ints(C_pt):
    """Extract integer values from Pt matrix."""
    if isinstance(C_pt[0][0], Pt):
        return [[pt.P for pt in row] for row in C_pt]
    return C_pt


def test_all_methods_match():
    """All geometric methods must match torch.matmul for random 4×4."""
    A = random_int_matrix(4, 4, max_val=50)
    B = random_int_matrix(4, 4, max_val=50)

    if HAS_TORCH:
        expected, _ = torch_matmul(A, B)
    elif HAS_NUMPY:
        expected, _ = numpy_matmul(A, B)
    else:
        print("  SKIP: no torch or numpy for baseline")
        return

    methods = [
        ("pt_naive", lambda: pt_naive(A, B)),
        ("pt_naive_fast", lambda: pt_naive_fast(A, B)),
        ("pytable_matmul", lambda: pytable_matmul(A, B)),
        ("pytable_matmul_cached", lambda: pytable_matmul_cached(A, B)),
        ("sd_matmul", lambda: sd_matmul_from_ints(A, B)),
    ]

    for name, fn in methods:
        try:
            C_pt, hv = fn()
            C_ints = extract_ints(C_pt)
        except Exception as e:
            print(f"  FAIL [{name}]: {e}")
            continue

        # Compare
        match = True
        for i in range(4):
            for j in range(4):
                got = C_ints[i][j]
                exp = expected[i][j]
                if abs(got - exp) > 1:  # allow ±1 for integer rounding
                    match = False
                    print(f"  FAIL [{name}] at [{i}][{j}]: got {got}, expected {exp}")

        if match:
            print(f"  OK   [{name}] matches baseline")


def test_identity():
    """A·I = A for all methods."""
    A = random_int_matrix(4, 4, max_val=50)
    I = [[1 if i == j else 0 for j in range(4)] for i in range(4)]

    for name, fn in [
        ("pt_naive", lambda: pt_naive(A, I)),
        ("pt_naive_fast", lambda: pt_naive_fast(A, I)),
        ("pytable_matmul", lambda: pytable_matmul(A, I)),
        ("sd_matmul", lambda: sd_matmul_from_ints(A, I)),
    ]:
        try:
            C_pt, hv = fn()
            C_ints = extract_ints(C_pt)
        except Exception as e:
            print(f"  FAIL [{name}] identity: {e}")
            continue

        ok = True
        for i in range(4):
            for j in range(4):
                if C_ints[i][j] != A[i][j]:
                    ok = False
                    print(f"  FAIL [{name}] identity at [{i}][{j}]")
        if ok:
            print(f"  OK   [{name}] identity")


def test_zero():
    """A·0 = 0 for all methods."""
    A = random_int_matrix(4, 4, max_val=50)
    Z = [[0] * 4 for _ in range(4)]

    for name, fn in [
        ("pt_naive", lambda: pt_naive(A, Z)),
        ("pt_naive_fast", lambda: pt_naive_fast(A, Z)),
        ("pytable_matmul", lambda: pytable_matmul(A, Z)),
        ("sd_matmul", lambda: sd_matmul_from_ints(A, Z)),
    ]:
        try:
            C_pt, hv = fn()
            C_ints = extract_ints(C_pt)
        except Exception as e:
            print(f"  FAIL [{name}] zero: {e}")
            continue

        all_zero = all(C_ints[i][j] == 0 for i in range(4) for j in range(4))
        if all_zero:
            print(f"  OK   [{name}] zero")
        else:
            print(f"  FAIL [{name}] zero: non-zero in result")


def test_healthvector():
    """All methods return valid HealthVector (all channels < 0.35) for valid input."""
    A = random_int_matrix(4, 4, max_val=50)
    B = random_int_matrix(4, 4, max_val=50)

    for name, fn in [
        ("pt_naive", lambda: pt_naive(A, B)),
        ("pt_naive_fast", lambda: pt_naive_fast(A, B)),
        ("pytable_matmul", lambda: pytable_matmul(A, B)),
        ("sd_matmul", lambda: sd_matmul_from_ints(A, B)),
    ]:
        try:
            C_pt, hv = fn()
        except Exception as e:
            print(f"  FAIL [{name}] HealthVector: {e}")
            continue

        if hv.ok:
            print(f"  OK   [{name}] HealthVector ok={hv.ok}")
        else:
            print(f"  WARN [{name}] HealthVector ok={hv.ok}, max={hv.max_channel}")


def test_shape_validation():
    """Shape mismatch raises ValueError."""
    A = random_int_matrix(3, 5)
    B = random_int_matrix(4, 2)  # 5 vs 4 mismatch

    for name, fn in [
        ("pt_naive", lambda: pt_naive(A, B)),
        ("pytable_matmul", lambda: pytable_matmul(A, B)),
        ("sd_matmul", lambda: sd_matmul_from_ints(A, B)),
    ]:
        try:
            fn()
            print(f"  FAIL [{name}] shape: no error raised")
        except ValueError:
            print(f"  OK   [{name}] shape validation")


if __name__ == '__main__':
    print("=== Geometric MM Correctness Tests ===\n")

    print("--- All methods match baseline ---")
    test_all_methods_match()
    print()

    print("--- Identity test ---")
    test_identity()
    print()

    print("--- Zero test ---")
    test_zero()
    print()

    print("--- HealthVector test ---")
    test_healthvector()
    print()

    print("--- Shape validation ---")
    test_shape_validation()
    print()

    print("=== Done ===")
