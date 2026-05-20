"""
Test Verifier — 20+ test cases for OpteriumVerifier.
"""

import sys
import os
sys.path.insert(0, os.path.dirname(__file__))
from verifier import OpteriumVerifier

v = OpteriumVerifier()

tests = [
    # Multiplication (10 tests)
    ("234 × 567 = 132678", True),
    ("12 × 12 = 144", True),
    ("1000 × 1000 = 1000000", True),
    ("7 × 8 = 56", True),
    ("0 × 999 = 0", True),
    ("1 × 42 = 42", True),
    ("13 × 13 = 169", True),
    ("99 × 99 = 9801", True),
    ("256 × 256 = 65536", True),
    ("512 × 512 = 262144", True),
    
    # Addition (4 tests)
    ("234 + 567 = 801", True),
    ("1000 + 1000 = 2000", True),
    ("0 + 42 = 42", True),
    ("999 + 1 = 1000", True),
    
    # Subtraction (3 tests)
    ("567 - 234 = 333", True),
    ("1000 - 1 = 999", True),
    ("42 - 42 = 0", True),
    
    # Division (2 tests)
    ("144 / 12 = 12", True),
    ("100 / 10 = 10", True),
    
    # Square root (2 tests)
    ("√144 = 12", True),
    ("√1000000 = 1000", True),
    
    # Fail cases (3 tests)
    ("12 × 12 = 145", False),
    ("100 + 100 = 201", False),
    ("50 - 25 = 26", False),
]

passed = 0
failed = 0

print("Running verifier tests...\n")
for claim, expected in tests:
    result = v.verify(claim)
    ok = result.get('valid') == expected
    if ok:
        passed += 1
        print(f"✅ {claim}")
    else:
        failed += 1
        print(f"❌ {claim} (expected {expected}, got {result.get('valid')})")

print(f"\n{passed}/{len(tests)} passed")
if failed == 0:
    print("All tests passed! ✅")
else:
    print(f"{failed} tests failed ❌")
