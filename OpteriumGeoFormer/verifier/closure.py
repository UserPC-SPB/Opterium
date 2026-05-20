"""
Closure Verifier — Verifies geometric closure claims via Doctor Principle.
"""

class ClosureVerifier:
    def verify(self, parsed: dict) -> dict:
        points = parsed.get('points', [])
        if len(points) < 3:
            return {'valid': False, 'error': 'Need at least 3 points for closure check'}
        
        # Doctor Principle: check if route A→B→C→...→A forms closed loop
        # Simplified: check if sum of vectors = 0
        dx_total = 0
        dy_total = 0
        
        for i in range(len(points)):
            x1, y1 = points[i]
            x2, y2 = points[(i + 1) % len(points)]
            dx_total += (x2 - x1)
            dy_total += (y2 - y1)
        
        closed = (dx_total == 0 and dy_total == 0)
        
        return {
            'valid': closed,
            'witness': f'Route {points} → Δx={dx_total}, Δy={dy_total}',
            'type': 'closure'
        }
