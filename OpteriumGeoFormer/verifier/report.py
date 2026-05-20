"""
Verifier Report — Formats verification results.
"""

class VerifierReport:
    def format_result(self, result: dict) -> str:
        if not result.get('valid'):
            return f"❌ {result.get('error', result.get('witness', 'Unknown error'))}"
        
        witness = result.get('witness', '')
        return f"✅ {witness}"

    def error(self, claim: str, reason: str) -> dict:
        return {
            'valid': False,
            'error': reason,
            'claim': claim,
            'type': 'error'
        }

    def format_batch(self, results: list) -> str:
        lines = []
        passed = 0
        for r in results:
            if r.get('valid'):
                passed += 1
                lines.append(f"✅ {r.get('witness', '')}")
            else:
                lines.append(f"❌ {r.get('error', r.get('witness', ''))}")
        
        lines.append(f"\n{passed}/{len(results)} passed")
        return '\n'.join(lines)
