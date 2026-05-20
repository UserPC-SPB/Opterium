"""
Opterium Verifier — Verification of arithmetic and closure claims.

Usage:
    from verifier import OpteriumVerifier
    v = OpteriumVerifier()
    result = v.verify("234 × 567 = 132678")
    print(result)  # ✅ True
"""

from .parser import ClaimParser
from .arithmetic import ArithmeticVerifier
from .debt import DebtVerifier
from .closure import ClosureVerifier
from .report import VerifierReport

class OpteriumVerifier:
    def __init__(self, max_coord: int = 1024):
        self.parser = ClaimParser()
        self.arithmetic = ArithmeticVerifier(max_coord)
        self.debt = DebtVerifier(max_coord)
        self.closure = ClosureVerifier()
        self.report = VerifierReport()

    def verify(self, claim: str) -> dict:
        parsed = self.parser.parse(claim)
        if not parsed:
            return self.report.error(claim, "Could not parse claim")

        if parsed['type'] == 'arithmetic':
            return self.arithmetic.verify(parsed)
        elif parsed['type'] == 'debt':
            return self.debt.verify(parsed)
        elif parsed['type'] == 'closure':
            return self.closure.verify(parsed)
        
        return self.report.error(claim, "Unknown claim type")

    def verify_batch(self, claims: list) -> list:
        return [self.verify(c) for c in claims]
