"""
Claim Parser — Extracts structured claims from text.
"""

import re

class ClaimParser:
    def parse(self, text: str) -> dict:
        text = text.strip()
        
        # Check for debt (float) claims first
        if self._has_float(text):
            return self._parse_debt(text)
        
        # Arithmetic patterns
        patterns = [
            (r'(.+?)\s*[\*×x]\s*(.+?)\s*=\s*(.+)', 'mul'),
            (r'(.+?)\s*/\s*(.+?)\s*=\s*(.+)', 'div'),
            (r'(.+?)\s*\+\s*(.+?)\s*=\s*(.+)', 'add'),
            (r'(.+?)\s*-\s*(.+?)\s*=\s*(.+)', 'sub'),
            (r'√\s*(.+?)\s*=\s*(.+)', 'sqrt'),
        ]
        
        for pattern, op in patterns:
            m = re.match(pattern, text)
            if m:
                if op == 'sqrt':
                    return {'type': 'arithmetic', 'op': 'sqrt', 'a': int(m.group(1)), 'result': int(m.group(2))}
                return {
                    'type': 'arithmetic',
                    'op': op,
                    'a': int(m.group(1)),
                    'b': int(m.group(2)),
                    'result': int(m.group(3))
                }
        
        return None

    def _has_float(self, text: str) -> bool:
        return bool(re.search(r'\d+\.\d+', text))

    def _parse_debt(self, text: str) -> dict:
        # Simple parsing for "3.4 * 2.33 = 7.922"
        m = re.match(r'(.+?)\s*[\*×x]\s*(.+?)\s*=\s*(.+)', text)
        if m:
            return {
                'type': 'debt',
                'op': 'mul',
                'a': float(m.group(1)),
                'b': float(m.group(2)),
                'result': float(m.group(3))
            }
        # Add other debt patterns as needed
        return None
