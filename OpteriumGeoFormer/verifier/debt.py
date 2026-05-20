"""
Debt Verifier — Verifies fractional arithmetic claims via (mantissa, debt) system.
"""

import sys
import os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), '..'))
from src.arith_table import PtTable

class DebtVerifier:
    def __init__(self, max_coord: int = 1024):
        self.PT = PtTable(max_coord)

    def verify(self, parsed: dict) -> dict:
        op = parsed['op']
        if op == 'mul':
            return self._verify_mul(parsed)
        return {'valid': False, 'error': f'Unknown debt op: {op}'}

    def _verify_mul(self, parsed: dict) -> dict:
        a, b, expected = parsed['a'], parsed['b'], parsed['result']
        
        # Convert to (mantissa, debt)
        ma, da = self._to_debt(a)
        mb, db = self._to_debt(b)
        me, de = self._to_debt(expected)
        
        # Lookup product of mantissas
        actual_mantissa = self.PT.product(ma, mb)
        actual_debt = da + db
        
        # Convert back to float
        actual = self._from_debt(actual_mantissa, actual_debt)
        
        # Allow small tolerance for float comparison
        valid = abs(actual - expected) < 1e-9
        
        return {
            'valid': valid,
            'expected': expected,
            'actual': actual,
            'witness': f'({ma},{da}) × ({mb},{db}) → ({actual_mantissa},{actual_debt})',
            'type': 'debt'
        }

    def _to_debt(self, value: float) -> tuple:
        s = f"{value:.10f}".rstrip('0')
        if '.' in s:
            int_part, dec_part = s.split('.')
            mantissa = int(int_part + dec_part)
            debt = -len(dec_part)
        else:
            mantissa = int(s)
            debt = 0
        return mantissa, debt

    def _from_debt(self, mantissa: int, debt: int) -> float:
        return mantissa * (10 ** debt)
