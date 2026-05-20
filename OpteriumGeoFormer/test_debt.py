"""
Test Debt System — Verifies fractional arithmetic via (mantissa, debt) system.
"""

import sys
import os
sys.path.insert(0, os.path.dirname(__file__))
from verifier import OpteriumVerifier

v = OpteriumVerifier()

tests = [
    # Debt multiplication (10 tests)
    ("3.4 * 2.33 = 7.922", True),
    ("0.1 * 0.2 = 0.02", True),
    ("1.5 * 2.0 = 3.0", True),
    ("10.0 * 10.0 = 100.0", True),
    ("0.5 * 0.5 = 0.25", True),
    ("2.5 * 4.0 = 10.0", True),
    ("1.1 * 1.1 = 1.21", True),
    ("3.0 * 3.0 = 9.0", True),
    ("0.01 * 0.01 = 0.0001", True),
    ("12.34 * 5.67 = 69.9678", True),
    
    # Fail cases (3 tests)
    ("3.4 * 2.33 = 7.923", False),
    ("0.1 * 0.2 = 0.03", False),
    ("1.5 * 2.0 = 3.1", False),
]

passed = 0
failed = 0

print("Running debt system tests...\n")
for claim, expected in tests:
    result = v.verify(claim)
    ok = result.get('valid') == expected
    if ok:
        passed += 1
        print(f"✅ {claim}")
    else:
        failed += 1
        print(f"❌ {claim} (expected {expected}, got {result.get('valid')})")
        if result.get('witness'):
            print(f"   Witness: {result['witness']}")

print(f"\n{passed}/{len(tests)} passed")
if failed == 0:
    print("All debt tests passed! ✅")
else:
    print(f"{failed} tests failed ❌")
