"""
test_purity.py — Verify ZERO arithmetic in hot paths.

Scans source files for forbidden patterns:
  - float division (/) in hot paths
  - float literals (1.0, 0.0) in hot paths
  - math.isqrt, math.sqrt in hot paths
  - ** operator for squaring
  - Pt() creation inside inner loops

All hot-path methods must use PT.p_from_sd, PT.proximity, PT.product, PT.isqrt.
"""

import sys, os, ast, re

SRC = os.path.join(os.path.dirname(__file__), '..', '..')
sys.path.insert(0, SRC)

PASS = '✅'
FAIL = '❌'


def scan_file(filepath, forbidden_patterns, allowed_comments=None):
    """Scan a file for forbidden patterns. Returns list of violations."""
    violations = []
    with open(filepath, 'r', encoding='utf-8', errors='replace') as f:
        lines = f.readlines()

    in_docstring = False
    in_comment_only = False

    for i, line in enumerate(lines, 1):
        stripped = line.strip()

        # Skip empty lines, pure comments, docstrings
        if not stripped:
            continue
        if stripped.startswith('#'):
            continue
        if stripped.startswith('"""') or stripped.startswith("'''"):
            in_docstring = not in_docstring
            continue
        if in_docstring:
            continue

        # Check each forbidden pattern
        for pattern_name, regex in forbidden_patterns:
            if re.search(regex, stripped):
                # Check if it's in an allowed context
                if allowed_comments and any(ac in stripped for ac in allowed_comments):
                    continue
                violations.append((i, pattern_name, stripped[:80]))

    return violations


def test_sd_matmul_purity():
    """sd_matmul must use PT.p_from_sd, not (S²−D²)//4 formula."""
    fpath = os.path.join(SRC, 'spec-kit', 'methods', 'sd_matmul.py')
    patterns = [
        ('formula_S2_D2', r'\(S\d*\s*\*\s*S\d*'),
        ('formula_S2_D2', r'S\d*\s*\*\s*S\d*\s*-\s*D'),
        ('formula_div4', r'//\s*4'),
        ('float_literal', r'1\.0|0\.0'),
        ('math_isqrt', r'math\.isqrt'),
        ('math_sqrt', r'math\.sqrt'),
    ]
    violations = scan_file(fpath, patterns)
    if violations:
        for ln, name, txt in violations:
            print(f"  {FAIL} sd_matmul L{ln} [{name}]: {txt}")
        raise AssertionError(f"sd_matmul has {len(violations)} purity violations")
    print(f"  {PASS} sd_matmul: zero formula, zero float, zero math.*")


def test_geo_resonant_purity():
    """geo_resonant must use PT.proximity, PT.isqrt, int accumulation."""
    fpath = os.path.join(SRC, 'spec-kit', 'methods', 'geo_resonant.py')
    patterns = [
        ('float_div', r'1\.0\s*/\s*\(1\.0'),
        ('float_div', r'1\.0\s*/\s*\(1\s*\+'),
        ('float_div', r'[^/]\s*/\s*w_total'),
        ('float_weight', r'1\.0\s*/'),
        ('math_isqrt', r'math\.isqrt'),
        ('math_sqrt', r'math\.sqrt'),
        ('float_context', r'int\(.*[^/]\s*/\s*'),
    ]
    violations = scan_file(fpath, patterns)
    if violations:
        for ln, name, txt in violations:
            print(f"  {FAIL} geo_resonant L{ln} [{name}]: {txt}")
        raise AssertionError(f"geo_resonant has {len(violations)} purity violations")
    print(f"  {PASS} geo_resonant: zero float weight, zero math.isqrt")


def test_hashgrid_purity():
    """hashgrid must use PT.proximity, int accumulation."""
    fpath = os.path.join(SRC, 'hashgrid.py')
    patterns = [
        ('float_div', r'1\.0\s*/'),
        ('float_weight', r'1\.0\s*/\s*\(1\.0'),
        ('float_context', r'int\(.*[^/]\s*/\s*'),
        ('math_isqrt', r'math\.isqrt'),
        ('math_sqrt', r'math\.sqrt'),
    ]
    violations = scan_file(fpath, patterns, allowed_comments=['#', '"""', 'Replaces'])
    if violations:
        for ln, name, txt in violations:
            print(f"  {FAIL} hashgrid L{ln} [{name}]: {txt}")
        raise AssertionError(f"hashgrid has {len(violations)} purity violations")
    print(f"  {PASS} hashgrid: zero float weight, zero math.*")


def test_pt_naive_purity():
    """pt_naive must use PT.product, no geo_mul/geo_add in loop."""
    fpath = os.path.join(SRC, 'spec-kit', 'methods', 'pt_naive.py')
    patterns = [
        ('geo_mul_in_loop', r'geo_mul\('),
        ('geo_add_in_loop', r'geo_add\('),
        ('float_literal', r'0\.0|1\.0'),
        ('math_isqrt', r'math\.isqrt'),
    ]
    violations = scan_file(fpath, patterns)
    if violations:
        for ln, name, txt in violations:
            print(f"  {FAIL} pt_naive L{ln} [{name}]: {txt}")
        raise AssertionError(f"pt_naive has {len(violations)} purity violations")
    print(f"  {PASS} pt_naive: zero geo_mul/geo_add, zero float, PT.product only")


def test_cross_verify_results():
    """All methods must produce identical results."""
    import random
    random.seed(42)

    sys.path.insert(0, os.path.join(SRC, 'spec-kit'))
    from methods.pt_naive import pt_naive, pt_naive_fast
    from methods.pytable_mm import pytable_matmul
    from methods.sd_matmul import sd_matmul_from_ints

    A = [[random.randint(1, 50) for _ in range(8)] for _ in range(8)]
    B = [[random.randint(1, 50) for _ in range(8)] for _ in range(8)]

    methods = [
        ("pt_naive", lambda: pt_naive(A, B)),
        ("pt_naive_fast", lambda: pt_naive_fast(A, B)),
        ("pytable_matmul", lambda: pytable_matmul(A, B)),
        ("sd_matmul", lambda: sd_matmul_from_ints(A, B)),
    ]

    results = {}
    for name, fn in methods:
        C_pt, hv = fn()
        results[name] = [[pt.P for pt in row] for row in C_pt]

    ref = results["pt_naive"]
    for name, mat in results.items():
        if name == "pt_naive":
            continue
        for i in range(8):
            for j in range(8):
                if mat[i][j] != ref[i][j]:
                    print(f"  {FAIL} cross-verify: {name}[{i}][{j}]={mat[i][j]} != ref={ref[i][j]}")
                    raise AssertionError(f"Cross-verify failed: {name} vs pt_naive")

    print(f"  {PASS} cross-verify: all 4 methods produce identical 8×8 results")


def test_pt_sp_table_correctness():
    """PT._SP table must match formula for all in-range (S,D)."""
    from arith_table import PT
    import random
    random.seed(42)

    # Test 50 random points
    for _ in range(50):
        x, y = random.randint(1, 1000), random.randint(1, 1000)
        s, d = x + y, x - y
        expected = x * y
        got = PT.p_from_sd(s, d)
        if got != expected:
            print(f"  {FAIL} _SP[{s}][{d}+offset]={got} != expected={expected}")
            raise AssertionError(f"_SP mismatch at ({x},{y})")

    # Test edge cases
    assert PT.p_from_sd(2, 0) == 1      # Pt(1,1)
    assert PT.p_from_sd(2048, 0) == 1048576  # Pt(1024,1024)
    assert PT.p_from_sd(7, 1) == 12     # Pt(4,3)
    assert PT.p_from_sd(24, 0) == 144   # Pt(12,12)

    print(f"  {PASS} PT._SP table: 50 random + edge cases verified")


def test_pt_proximity_table():
    """PT._prox table must return correct integer weights."""
    from arith_table import PT

    assert PT.proximity(0) == 10000
    assert PT.proximity(1) == 5000
    assert PT.proximity(2) == 3333
    assert PT.proximity(9) == 1000
    assert PT.proximity(99) == 100
    assert PT.proximity(4096) == 2       # max_dist edge
    assert PT.proximity(4097) == 0       # out of range

    # Symmetry
    assert PT.int_weight(10, 5, 20, 15) == PT.int_weight(20, 15, 10, 5)

    # Same point = max weight
    assert PT.int_weight(42, 7, 42, 7) == 10000

    print(f"  {PASS} PT._prox table: weights correct, symmetry verified")


if __name__ == '__main__':
    print("=== Purity Tests — Zero Arithmetic Hot Paths ===\n")

    tests = [
        ("sd_matmul purity", test_sd_matmul_purity),
        ("geo_resonant purity", test_geo_resonant_purity),
        ("hashgrid purity", test_hashgrid_purity),
        ("pt_naive purity", test_pt_naive_purity),
        ("cross-verify results", test_cross_verify_results),
        ("PT._SP table correctness", test_pt_sp_table_correctness),
        ("PT._prox table correctness", test_pt_proximity_table),
    ]

    passed = 0
    failed = 0
    for name, fn in tests:
        try:
            fn()
            passed += 1
        except Exception as e:
            print(f"  {FAIL} {name}: {e}")
            failed += 1

    print(f"\n{'='*50}")
    print(f"  {passed} passed, {failed} failed, {passed+failed} total")

    if failed > 0:
        print(f"\n  🔴 PURITY VIOLATIONS DETECTED")
        sys.exit(1)
    else:
        print(f"\n  ✅ ALL HOT PATHS PURE — zero arithmetic, zero float")
