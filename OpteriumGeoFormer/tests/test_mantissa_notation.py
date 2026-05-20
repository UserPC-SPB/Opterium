"""
test_mantissa_notation.py  —  US11: Real-number mantissa notation x|y|

Tests:
  1. repr shows x|y| format
  2. verbose shows old Pt format
  3. parse("347|3|") → Pt(347, 3)
  4. parse("0|3|") → Pt(0, 3)
  5. roundtrip: str(Pt(x,y)) == str(Pt.parse(str(Pt(x,y))))
  6. Pt(347, 3).to_real() → 0.347
  7. Pt.from_real(0.347) → Pt(347, 3)
  8. Pt(0, 3).to_real() → 0.0
  9. Pt(-347, 3).to_real() → -0.347
  10. Pt(1234, 2).to_real() → 12.34
  11. Pt.parse("not|valid") → ValueError
  12. Pt.from_real(0.0) → Pt(0, 1)
  13. Pt.from_real(0.001) → Pt(1, 3)
  14. parse with no rank defaults to y=1
"""

import sys, os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), '..', 'src'))
sys.path.insert(0, os.path.join(os.path.dirname(__file__), '..', 'src', 'spec-kit'))

from methods import Pt


def test_repr_format():
    """Pt(347, 3) → '347|3|'"""
    p = Pt(347, 3)
    assert repr(p) == "347|3|", f"repr={repr(p)!r}"


def test_verbose_old_format():
    """verbose() preserves old Pt(... S=... D=... P=...) format."""
    p = Pt(347, 3)
    v = p.verbose()
    assert v.startswith("Pt("), f"verbose={v!r}"
    assert "S=" in v, f"verbose missing S: {v}"
    assert "D=" in v, f"verbose missing D: {v}"
    assert "P=" in v, f"verbose missing P: {v}"
    assert "P=1041" in v, f"verbose wrong P: {v}"


def test_parse_basic():
    """parse('347|3|') → Pt(347, 3)."""
    p = Pt.parse("347|3|")
    assert p.x == 347, f"x={p.x}"
    assert p.y == 3, f"y={p.y}"
    assert p.S == 350
    assert p.D == 344
    assert p.P == 1041


def test_parse_zero_mantissa():
    """parse('0|3|') → Pt(0, 3)."""
    p = Pt.parse("0|3|")
    assert p.x == 0
    assert p.y == 3


def test_roundtrip():
    """str(Pt(x,y)) == str(Pt.parse(str(Pt(x,y)))) for small range."""
    for x in range(0, 11):
        for y in range(1, 6):
            orig = Pt(x, y)
            s = str(orig)
            parsed = Pt.parse(s)
            assert parsed == orig, f"roundtrip fail: orig={orig}, parsed={parsed}, str={s!r}"


def test_to_real_347():
    """Pt(347, 3).to_real() ≈ 0.347."""
    p = Pt(347, 3)
    val = p.to_real()
    assert abs(val - 0.347) < 1e-10, f"to_real={val}"


def test_from_real_347():
    """Pt.from_real(0.347) → Pt(347, 3)."""
    p = Pt.from_real(0.347)
    assert p.x == 347, f"x={p.x}"
    assert p.y == 3, f"y={p.y}"


def test_to_real_zero():
    """Pt(0, 3).to_real() → 0.0."""
    p = Pt(0, 3)
    assert p.to_real() == 0.0


def test_to_real_negative():
    """Pt(-347, 3).to_real() → -0.347."""
    p = Pt(-347, 3)
    assert abs(p.to_real() - (-0.347)) < 1e-10


def test_to_real_large():
    """Pt(1234, 2).to_real() → 12.34."""
    p = Pt(1234, 2)
    assert abs(p.to_real() - 12.34) < 1e-10


def test_parse_invalid():
    """parse('not|valid') → ValueError."""
    try:
        Pt.parse("not|valid")
        assert False, "expected ValueError"
    except ValueError:
        pass


def test_parse_missing_pipe():
    """parse('347|3') (no trailing |) → ValueError."""
    try:
        Pt.parse("347|3")
        assert False, "expected ValueError"
    except ValueError:
        pass


def test_from_real_zero():
    """Pt.from_real(0.0) → Pt(0, 0) (debt=0, нет десятичного сдвига)."""
    p = Pt.from_real(0.0)
    assert p.x == 0
    assert p.y == 0


def test_from_real_small():
    """Pt.from_real(0.001) → Pt(1, 3)."""
    p = Pt.from_real(0.001)
    assert p.x == 1, f"x={p.x}"
    assert p.y == 3, f"y={p.y}"


def test_parse_default_rank():
    """parse('42|') defaults y=1."""
    p = Pt.parse("42|")
    assert p.x == 42
    assert p.y == 1


def test_full_roundtrip_from_real():
    """Pt.from_real(r).to_real() approximates r."""
    cases = [0.0, 3.14, -0.5, 0.001, 100.0, 0.0001]
    for r in cases:
        p = Pt.from_real(r)
        back = p.to_real()
        assert abs(back - r) / max(1.0, abs(r)) < 1e-6, \
            f"r={r} → Pt({p.x},{p.y}) → {back}"


if __name__ == '__main__':
    print("=== Mantissa Notation Tests ===\n")

    tests = [
        ("repr format", test_repr_format),
        ("verbose old format", test_verbose_old_format),
        ("parse basic", test_parse_basic),
        ("parse zero mantissa", test_parse_zero_mantissa),
        ("roundtrip", test_roundtrip),
        ("to_real 0.347", test_to_real_347),
        ("from_real 0.347", test_from_real_347),
        ("to_real zero", test_to_real_zero),
        ("to_real negative", test_to_real_negative),
        ("to_real large", test_to_real_large),
        ("parse invalid", test_parse_invalid),
        ("parse missing pipe", test_parse_missing_pipe),
        ("from_real zero", test_from_real_zero),
        ("from_real small", test_from_real_small),
        ("parse default rank", test_parse_default_rank),
        ("full roundtrip from_real", test_full_roundtrip_from_real),
    ]

    passed = 0
    for name, fn in tests:
        try:
            fn()
            print(f"  OK   [{name}]")
            passed += 1
        except Exception as e:
            print(f"  FAIL [{name}]: {e}")

    total = len(tests)
    print(f"\n  {passed}/{total} passed")
    assert passed == total, f"{passed}/{total} FAILED"
    print("=== Done ===")
