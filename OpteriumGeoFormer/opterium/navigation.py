"""
Opterium Navigation Core — Reasoning in Table (Zero Weights)

This is where all reasoning happens. No trainable parameters.
Navigation through the Opterium table using S/D coordinates.

Operations:
- navigate(start_addr, operations) → end_addr
- analogy(A, B, C) → D (A:B :: C:D)
- chain(operations) → result
- verify(claim) → True/False + witness

All operations are pure table lookups. Zero arithmetic in hot paths.
"""

import sys
import os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), '..', 'src'))
from arith_table import PtTable
from typing import List, Tuple, Optional, Dict

class NavigationCore:
    """Core reasoning engine using Opterium table navigation."""
    
    def __init__(self, max_coord: int = 1024):
        self.PT = PtTable(max_coord)
        self.max_coord = max_coord
    
    def navigate(self, start: Tuple[int, int], operations: List[str]) -> Tuple[int, int]:
        """Navigate from start address through a sequence of operations.
        
        Operations:
        - "S" → move along sum curve (x+y = constant)
        - "D" → move along difference curve (x-y = constant)
        - "P" → move along product curve (xy = constant)
        - "H" → jump to hyperbola (divisors of P)
        - "R" → reflect across diagonal
        """
        x, y = start
        
        for op in operations:
            if op == "S":
                # Move along sum curve: keep x+y constant
                s = self.PT.S(x, y)
                # Find new point on same sum curve
                # Simple: swap x and y (preserves sum)
                x, y = y, x
            
            elif op == "D":
                # Move along difference curve: keep x-y constant
                d = self.PT.D(x, y)
                # Find new point on same difference curve
                # Simple: shift by 1
                new_x = min(x + 1, self.max_coord)
                new_y = new_x - d
                if 0 <= new_y <= self.max_coord:
                    x, y = new_x, new_y
            
            elif op == "P":
                # Move along product curve: keep xy constant
                p = self.PT.P(x, y)
                # Find divisors of p
                pairs = self.PT._pairs.get(p, [])
                if pairs:
                    # Find next pair
                    idx = pairs.index((min(x, y), max(x, y))) if (min(x, y), max(x, y)) in pairs else 0
                    next_idx = (idx + 1) % len(pairs)
                    x, y = pairs[next_idx]
            
            elif op == "H":
                # Jump to hyperbola: find all divisors of P
                p = self.PT.P(x, y)
                pairs = self.PT._pairs.get(p, [])
                if pairs:
                    # Return to first divisor pair
                    x, y = pairs[0]
            
            elif op == "R":
                # Reflect across diagonal: swap x and y
                x, y = y, x
        
        return (x, y)
    
    def analogy(self, A: Tuple[int, int], B: Tuple[int, int], C: Tuple[int, int]) -> Tuple[int, int]:
        """Solve analogy A:B :: C:D using geometric navigation.
        
        Method:
        1. Compute transformation from A to B (delta_S, delta_D)
        2. Apply same transformation to C to get D
        """
        s_a, d_a = self.PT.S(*A), self.PT.D(*A)
        s_b, d_b = self.PT.S(*B), self.PT.D(*B)
        
        # Transformation
        delta_s = s_b - s_a
        delta_d = d_b - d_a
        
        # Apply to C
        s_c = self.PT.S(*C)
        d_c = self.PT.D(*C)
        
        s_d = s_c + delta_s
        d_d = d_c + delta_d
        
        # Convert (S, D) back to (x, y)
        x = (s_d + d_d) // 2
        y = (s_d - d_d) // 2
        
        # Clamp to valid range
        x = max(0, min(self.max_coord, x))
        y = max(0, min(self.max_coord, y))
        
        return (x, y)
    
    def chain(self, operations: List[Tuple[str, Tuple[int, int]]]) -> List[Tuple[int, int]]:
        """Execute a chain of operations, recording intermediate addresses.
        
        Each operation: (operation_name, address)
        Returns: list of (operation_name, address, result_address)
        """
        results = []
        current = (0, 0)
        
        for op_name, addr in operations:
            if op_name == "navigate":
                result = self.navigate(current, ["S", "D", "P"])
            elif op_name == "analogy":
                # addr should be (A, B, C)
                result = self.analogy(*addr[:3])
            elif op_name == "product":
                x, y = addr
                p = self.PT.P(x, y)
                result = (p, 0)
            elif op_name == "sum":
                x, y = addr
                s = self.PT.S(x, y)
                result = (s, 0)
            elif op_name == "diff":
                x, y = addr
                d = self.PT.D(x, y)
                result = (d, 0)
            else:
                result = addr
            
            results.append((op_name, addr, result))
            current = result
        
        return results
    
    def verify_claim(self, claim: str) -> Tuple[bool, str]:
        """Verify a claim using table navigation.
        
        Claims:
        - "A × B = C" → check product
        - "A + B = C" → check sum
        - "A - B = C" → check difference
        - "A:B :: C:D" → check analogy
        """
        # Parse claim
        if "×" in claim or "*" in claim:
            parts = claim.replace("×", "*").split("=")
            if len(parts) != 2:
                return False, "Invalid claim format"
            left = parts[0].strip().split("*")
            if len(left) != 2:
                return False, "Invalid multiplication claim"
            a, b = int(left[0].strip()), int(left[1].strip())
            expected = int(parts[1].strip())
            actual = self.PT.P(a, b)
            return actual == expected, f"P({a},{b}) = {actual}"
        
        elif "+" in claim:
            parts = claim.split("=")
            if len(parts) != 2:
                return False, "Invalid claim format"
            left = parts[0].strip().split("+")
            if len(left) != 2:
                return False, "Invalid addition claim"
            a, b = int(left[0].strip()), int(left[1].strip())
            expected = int(parts[1].strip())
            actual = self.PT.S(a, b)
            return actual == expected, f"S({a},{b}) = {actual}"
        
        elif "-" in claim:
            parts = claim.split("=")
            if len(parts) != 2:
                return False, "Invalid claim format"
            left = parts[0].strip().split("-")
            if len(left) != 2:
                return False, "Invalid subtraction claim"
            a, b = int(left[0].strip()), int(left[1].strip())
            expected = int(parts[1].strip())
            actual = self.PT.D(a, b)
            return actual == expected, f"D({a},{b}) = {actual}"
        
        elif "::" in claim:
            # Analogy claim: A:B :: C:D
            parts = claim.split("::")
            if len(parts) != 2:
                return False, "Invalid analogy claim"
            left = parts[0].strip().split(":")
            right = parts[1].strip().split(":")
            if len(left) != 2 or len(right) != 2:
                return False, "Invalid analogy format"
            
            A = tuple(map(int, left[0].strip().split(",")))
            B = tuple(map(int, left[1].strip().split(",")))
            C = tuple(map(int, right[0].strip().split(",")))
            D_expected = tuple(map(int, right[1].strip().split(",")))
            
            D_actual = self.analogy(A, B, C)
            return D_actual == D_expected, f"analogy({A},{B},{C}) = {D_actual}"
        
        return False, "Unknown claim type"
    
    def get_witness(self, x: int, y: int) -> Dict[str, int]:
        """Get full witness for an address (x, y)."""
        return {
            'x': x,
            'y': y,
            'P': self.PT.P(x, y),
            'S': self.PT.S(x, y),
            'D': self.PT.D(x, y),
            'divisors': len(self.PT._pairs.get(self.PT.P(x, y), [])),
        }
