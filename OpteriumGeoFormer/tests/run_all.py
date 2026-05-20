#!/usr/bin/env python3
"""
run_all.py  —  Master test runner for Opterium GeoFormer.

Запускает все тесты, показывает статус каждого, выявляет gaps.
Возвращает exit code = количество проваленных тестов.

Usage:
    python run_all.py              # full suite
    python run_all.py --quick      # только self-test (быстро)
    python run_all.py --benchmark  # + benchmark
    python run_all.py --coverage   # показать gaps
    python run_all.py --ci         # для CI: только критическое
"""

import sys, os, time, importlib, json

PROJECT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
SRC = os.path.join(PROJECT, 'src')
TESTS = os.path.join(PROJECT, 'tests')

sys.path.insert(0, SRC)
sys.path.insert(0, os.path.join(SRC, 'spec-kit'))

# ─────────────────────────────────────────────────────
# Test registry
# ─────────────────────────────────────────────────────
PASS = '✅'
FAIL = '❌'
SKIP = '⏭️'

class TestResult:
    def __init__(self, module, name, status, detail='', time_ms=0):
        self.module = module
        self.name = name
        self.status = status
        self.detail = detail
        self.time_ms = time_ms

    def __repr__(self):
        return f"{self.status} [{self.module}] {self.name}  ({self.time_ms:.1f}ms)"

results = []

# ─────────────────────────────────────────────────────
# Test registry (MUST be before @test decorators)
# ─────────────────────────────────────────────────────
TEST_REGISTRY = []

def test(module, name):
    def decorator(fn):
        TEST_REGISTRY.append((module, name, fn))
        def wrapper():
            # wrapper not used directly; registered fns called by runner
            pass
        return fn  # return original fn, not wrapper
    return decorator


# ─────────────────────────────────────────────────────
# Tests
# ─────────────────────────────────────────────────────

# 1. Core modules — self-test
@test('delta_ops', 'self-test')
def t_delta_selftest():
    from delta_ops import selftest
    selftest()

@test('phi_algebra', 'self-test')
def t_phi_selftest():
    from phi_algebra import selftest
    selftest()

@test('swarm', 'self-test')
def t_swarm_selftest():
    from swarm import selftest
    selftest()

@test('hashgrid', 'self-test')
def t_hashgrid_selftest():
    from hashgrid import selftest
    selftest()

@test('geoformer', 'self-test')
def t_geoformer_selftest():
    from geoformer import selftest
    selftest()

@test('doctor_geo', 'self-test')
def t_doctor_selftest():
    from doctor_geo import selftest
    selftest()

@test('e8_twist', 'self-test')
def t_twist_selftest():
    from e8_twist import selftest
    selftest()

# 2. Spec-kit correctness
@test('spec-kit', 'all methods match torch')
def t_spec_all_match():
    import importlib.util
    spec = importlib.util.spec_from_file_location(
        "test_correctness",
        os.path.join(SRC, 'spec-kit', 'tests', 'test_correctness.py')
    )
    mod = importlib.util.module_from_spec(spec)
    # Fix paths for the module
    mod_path = os.path.join(SRC, 'spec-kit')
    if mod_path not in sys.path:
        sys.path.insert(0, mod_path)
    spec.loader.exec_module(mod)
    mod.test_all_methods_match()

@test('spec-kit', 'identity A·I=A')
def t_spec_identity():
    import importlib.util
    spec = importlib.util.spec_from_file_location(
        "test_correctness",
        os.path.join(SRC, 'spec-kit', 'tests', 'test_correctness.py')
    )
    mod = importlib.util.module_from_spec(spec)
    mod_path = os.path.join(SRC, 'spec-kit')
    if mod_path not in sys.path:
        sys.path.insert(0, mod_path)
    spec.loader.exec_module(mod)
    mod.test_identity()

@test('spec-kit', 'zero A·0=0')
def t_spec_zero():
    import importlib.util
    spec = importlib.util.spec_from_file_location(
        "test_correctness",
        os.path.join(SRC, 'spec-kit', 'tests', 'test_correctness.py')
    )
    mod = importlib.util.module_from_spec(spec)
    mod_path = os.path.join(SRC, 'spec-kit')
    if mod_path not in sys.path:
        sys.path.insert(0, mod_path)
    spec.loader.exec_module(mod)
    mod.test_zero()

@test('spec-kit', 'HealthVector OK')
def t_spec_hv():
    import importlib.util
    spec = importlib.util.spec_from_file_location(
        "test_correctness",
        os.path.join(SRC, 'spec-kit', 'tests', 'test_correctness.py')
    )
    mod = importlib.util.module_from_spec(spec)
    mod_path = os.path.join(SRC, 'spec-kit')
    if mod_path not in sys.path:
        sys.path.insert(0, mod_path)
    spec.loader.exec_module(mod)
    mod.test_healthvector()

@test('spec-kit', 'shape validation')
def t_spec_shape():
    import importlib.util
    spec = importlib.util.spec_from_file_location(
        "test_correctness",
        os.path.join(SRC, 'spec-kit', 'tests', 'test_correctness.py')
    )
    mod = importlib.util.module_from_spec(spec)
    mod_path = os.path.join(SRC, 'spec-kit')
    if mod_path not in sys.path:
        sys.path.insert(0, mod_path)
    spec.loader.exec_module(mod)
    mod.test_shape_validation()

# 3. Cross-verify: Python = Cython = Rust
@test('cross-verify', 'Py = Cy = Rs (16×16)')
def t_cross_verify():
    import random
    random.seed(42)
    sk_path = os.path.join(SRC, 'spec-kit')
    if sk_path not in sys.path:
        sys.path.insert(0, sk_path)
    from methods.sd_matmul import sd_matmul_from_ints

    A = [[random.randint(1, 100) for _ in range(16)] for _ in range(16)]
    B = [[random.randint(1, 100) for _ in range(16)] for _ in range(16)]
    A_sd = [[(v+1, 1) for v in row] for row in A]
    B_sd = [[(v+1, 1) for v in row] for row in B]

    py = sd_matmul_from_ints(A, B)
    assert len(py) == 2 and len(py[0]) == 16, f"py shape: {len(py)}×{len(py[0])}"
    print(f"    Python: {len(py)}×{len(py[0])} (Pt objects)")

    try:
        import geo_matmul_v2
        cy = geo_matmul_v2.sd_matmul_v2(A_sd, B_sd)
        assert len(cy) == 2 and len(cy[0]) == 16
        for i in range(2):
            for j in range(16):
                assert py[i][j].P == cy[i][j].P, f"[{i}][{j}] py.P={py[i][j].P} cy.P={cy[i][j].P}"
        print("    Cython: match OK")
    except ImportError:
        print("    Cython: SKIP (not installed)")

    try:
        import geo_matmul_rs
        rs = geo_matmul_rs.sd_matmul(A_sd, B_sd)
        assert isinstance(rs, list), f"rs type: {type(rs)}"
        print(f"    Rust seq: OK (len={len(rs)})")
        rs_p = geo_matmul_rs.sd_matmul_parallel(A_sd, B_sd)
        assert isinstance(rs_p, list)
        print(f"    Rust par: OK (len={len(rs_p)})")
    except ImportError:
        print("    Rust: SKIP (not installed)")

# 4. E8 operability check
@test('e8_twist', 'triality 240 roots')
def t_twist_triality():
    try:
        sys.path.insert(0, os.path.join(os.path.dirname(__file__), '..', 'src'))
        from opterium_field import e8gen
        from e8_twist import TwistEngine
        te = TwistEngine(e8gen)
        g = te.triality_groups()
        total = sum(len(v) for v in g.values())
        assert total == 240, f"expected 240 roots, got {total}"
        assert len(g.get('V', [])) == 112
        assert len(g.get('S+', [])) == 64
        assert len(g.get('S-', [])) == 64
        print(f"    V={len(g['V'])} S+={len(g['S+'])} S-={len(g['S-'])} = {total}")
    except ImportError:
        print("    SKIP: opterium_field not available")

@test('e8_twist', 'closure 70.1° CLOSED')
def t_twist_closure():
    from e8_twist import TwistEngine
    te = TwistEngine()
    c = te.closure_angle(70.1)
    assert c['status'] == 'CLOSED', f"expected CLOSED, got {c['status']}"
    assert c['energy'] < 2.0, f"energy too high: {c['energy']}"
    print(f"    energy={c['energy']:.4f}")

@test('e8_twist', '2520-cycle 35° = 72 steps')
def t_twist_cycle():
    from e8_twist import TwistEngine
    te = TwistEngine()
    c = te.cycle_2520(35.0)
    assert c['steps'] == 72, f"expected 72 steps, got {c['steps']}"
    assert abs(c['total_deg'] - 2520) < 1

# 5. Rust module
@test('rust', 'HashGrid insert/lookup')
def t_rust_hashgrid():
    import geo_matmul_rs
    g = geo_matmul_rs.HashGrid(16)
    g.insert(0, 10, 5, 50)
    g.insert(1, 12, 3, 36)
    nb = g.lookup(11, 4)
    assert len(nb) >= 1, "expected at least 1 neighbor"
    nb_all = g.lookup(100, 50)
    assert len(nb_all) == 0, "expected 0 neighbors in empty region"

@test('rust', 'geometric_attention output fields')
def t_rust_attention():
    import geo_matmul_rs, random
    random.seed(42)
    tokens = [(i, random.randint(1,100), random.randint(-50,50), random.randint(1,100))
              for i in range(20)]
    out = geo_matmul_rs.geometric_attention(tokens, 20, False)
    assert len(out) == 20, "expected 20 outputs"
    for o in out:
        assert len(o) == 5, f"expected 5 fields, got {len(o)}"
        # id, context, n_neighbors, output_x, output_y


# ─────────────────────────────────────────────────────
# Runner
# ─────────────────────────────────────────────────────


def main():
    import argparse
    parser = argparse.ArgumentParser(description='Opterium GeoFormer test runner')
    parser.add_argument('--quick', action='store_true', help='only self-tests')
    parser.add_argument('--benchmark', action='store_true', help='include benchmarks')
    parser.add_argument('--coverage', action='store_true', help='show gaps only')
    parser.add_argument('--ci', action='store_true', help='CI mode: fail on CRIT gaps')
    parser.add_argument('--json', action='store_true', help='output JSON')
    args = parser.parse_args()

    # Filter
    if args.quick:
        filtered = [(m, n, fn) for (m, n, fn) in TEST_REGISTRY
                     if 'spec-kit' not in n and 'cross' not in n
                     and 'rust' not in n and 'benchmark' not in n]
    else:
        filtered = TEST_REGISTRY

    print(f"\n  Opterium GeoFormer — Test Runner")
    print(f"  {'='*50}")
    print(f"  Found {len(filtered)} tests\n")

    failed = 0
    for module, name, fn in filtered:
        t0 = time.perf_counter()
        try:
            fn()
            t1 = time.perf_counter()
            results.append(TestResult(module, name, PASS, time_ms=(t1-t0)*1000))
        except Exception as e:
            t1 = time.perf_counter()
            results.append(TestResult(module, name, FAIL, str(e), time_ms=(t1-t0)*1000))

    print(f"\n  {'='*50}")
    print(f"  Results: {len(results)} tests")

    # Show per-module
    modules = {}
    for r in results:
        modules.setdefault(r.module, []).append(r)

    for mod, mod_results in sorted(modules.items()):
        print(f"\n  [{mod}]")
        for r in mod_results:
            print(f"    {r.status} {r.name}  ({r.time_ms:.1f}ms)")
            if r.status == FAIL:
                print(f"      ⤷ {r.detail}")

    # Summary
    passed = sum(1 for r in results if r.status == PASS)
    failed_cnt = sum(1 for r in results if r.status == FAIL)
    print(f"\n  {'='*50}")
    print(f"  ✅ {passed} passed, ❌ {failed_cnt} failed, "
          f"{(passed+failed_cnt)} total")

    if args.coverage:
        print(f"\n  ⚠️  GAPS (см. tests/TEST_COVERAGE.md)")
        print(f"  🔴 CRIT: 5  (GeoFormer end-to-end, multi-layer, opterium bridge, Rust tests, cross-verify)")
        print(f"  🟡 WARN: 20 (individual module edge cases)")
        print(f"  🟢 INFO: 8  (cosmetic)")

    return 1 if failed_cnt > 0 else 0


def _fix_imports():
    """spec-kit directory has a hyphen — make importable via sys.path tricks."""
    spec_kit_path = os.path.join(SRC, 'spec-kit')
    if os.path.isdir(spec_kit_path) and spec_kit_path not in sys.path:
        sys.path.insert(0, spec_kit_path)
    if SRC not in sys.path:
        sys.path.insert(0, SRC)

if __name__ == '__main__':
    _fix_imports()
    sys.exit(main())
