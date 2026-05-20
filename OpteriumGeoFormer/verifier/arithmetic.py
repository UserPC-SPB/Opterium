"""
Arithmetic Verifier — Verifies integer arithmetic claims via lookup tables.
"""

import sys
import os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), '..'))
from src.arith_table import PtTable

class ArithmeticVerifier:
    def __init__(self, max_coord: int = 1024):
        self.PT = PtTable(max_coord)

    def verify(self, parsed: dict) -> dict:
        op = parsed['op']
        
        if op == 'mul':
            return self._verify_mul(parsed)
        elif op == 'div':
            return self._verify_div(parsed)
        elif op == 'add':
            return self._verify_add(parsed)
        elif op == 'sub':
            return self._verify_sub(parsed)
        elif op == 'sqrt':
            return self._verify_sqrt(parsed)
        
        return {'valid': False, 'error': f'Unknown op: {op}'}

    def _verify_mul(self, parsed: dict) -> dict:
        a, b, expected = parsed['a'], parsed['b'], parsed['result']
        actual = self.PT.product(a, b)
        return self._result(actual, expected, f'P({a},{b})')

    def _verify_div(self, parsed: dict) -> dict:
        a, b, expected = parsed['a'], parsed['b'], parsed['result']
        pairs = self.PT._pairs.get(a, [])
        found = any(x == b or y == b for x, y in pairs)
        actual = None
        if found:
            for x, y in pairs:
                if x == b:
                    actual = y
                    break
                if y == b:
                    actual = x
                    break
        return self._result(actual, expected, f'by_P[{a}] with {b}')

    def _verify_add(self, parsed: dict) -> dict:
        a, b, expected = parsed['a'], parsed['b'], parsed['result']
        actual = self.PT.S(a, b)
        return self._result(actual, expected, f'S({a},{b})')

    def _verify_sub(self, parsed: dict) -> dict:
        a, b, expected = parsed['a'], parsed['b'], parsed['result']
        actual = self.PT.D(a, b)
        return self._result(actual, expected, f'D({a},{b})')

    def _verify_sqrt(self, parsed: dict) -> dict:
        a, expected = parsed['a'], parsed['result']
        actual = self.PT.isqrt(a)
        return self._result(actual, expected, f'isqrt({a})')

    def _result(self, actual, expected, witness: str) -> dict:
        valid = actual == expected
        return {
            'valid': valid,
            'expected': expected,
            'actual': actual,
            'witness': witness,
            'type': 'arithmetic'
        }
