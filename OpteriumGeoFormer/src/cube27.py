"""
cube27.py  —  Cube27 self-similar addressing

Любой integer mantissa → 3-digit группы.
Каждая группа ∈ [0, 999] — прямой адрес в PtTable.
Цепочка групп = путь по 27-ичному кубу.

27-ичный куб: 3×3×3 = 27 ячеек, равномерно покрывающих [0, 999].
Каждая ячейка (cx, cy, cz) — треть от трети.

Свойство:
  Запрос = адрес. Группа ≤ 999 ≤ MAX_COORD = 1024.
  3 цифры → всегда PtTable hit. Никаких fallback.
"""

import sys, os
sys.path.insert(0, os.path.dirname(__file__))
from arith_table import PT


class Cube27:
    """Self-similar addressing via 3-digit decimal groups."""

    GROUP_SIZE = 3
    MAX_GROUP = 999
    N_CELLS = 27

    def __init__(self):
        self.max_coord = PT.max_coord
        # Precompute 27 cell boundaries for O(log 27) = O(1) lookup
        self._cell_bounds = []
        n = 1000
        k = 27
        base = n // k
        rem = n % k
        acc = 0
        for i in range(k):
            size = base + (1 if i >= k - rem else 0)
            self._cell_bounds.append(acc + size - 1)
            acc += size

    def encode(self, mantissa: int) -> list:
        """Split positive integer into 3-digit groups (MSB first).

        123456789 → [123, 456, 789]
        """
        if mantissa < 0:
            raise ValueError(f"mantissa must be >= 0, got {mantissa}")
        if mantissa == 0:
            return [0]
        s = str(mantissa)
        pad = (self.GROUP_SIZE - len(s) % self.GROUP_SIZE) % self.GROUP_SIZE
        s = '0' * pad + s
        return [int(s[i:i + self.GROUP_SIZE])
                for i in range(0, len(s), self.GROUP_SIZE)]

    def cell_index(self, group: int) -> int:
        """3-digit group → cell index 0..26 in 27-ary cube."""
        if group > self.MAX_GROUP:
            return self.N_CELLS - 1
        # Binary search on precomputed upper bounds
        lo, hi = 0, self.N_CELLS
        while lo < hi:
            mid = (lo + hi) // 2
            if group <= self._cell_bounds[mid]:
                hi = mid
            else:
                lo = mid + 1
        return min(lo, self.N_CELLS - 1)

    def cell_27(self, group: int) -> tuple:
        """3-digit group → (cx, cy, cz) in 3×3×3 cube."""
        ci = self.cell_index(group)
        return (ci // 9, (ci // 3) % 3, ci % 3)

    def path_27(self, mantissa: int) -> list:
        """Full Cube27 path: [(cx,cy,z), ...] one per group."""
        return [self.cell_27(g) for g in self.encode(mantissa)]

    def depth(self, mantissa: int) -> int:
        """Levels of Cube27 needed for this mantissa."""
        return len(self.encode(mantissa))

    def format_path(self, mantissa: int) -> str:
        """Human-readable: '123|456|789| (cells: (0,1,0)|(1,0,0)|(2,0,0))'"""
        groups = self.encode(mantissa)
        cells = [self.cell_27(g) for g in groups]
        gs = '|'.join(f'{g:03d}' for g in groups)
        cs = '|'.join(f'({cx},{cy},{cz})' for cx, cy, cz in cells)
        return f"{gs}| ({cs})"

    def verify(self, mantissa: int) -> dict:
        """Full verification: encode → groups → PtTable hit check."""
        groups = self.encode(mantissa)
        results = []
        for g in groups:
            hit = PT.has(g, 1) or g == 0
            cell = self.cell_27(g) if g <= self.MAX_GROUP else None
            results.append({
                'group': g,
                'pt_hit': hit,
                'cell': cell,
            })
        return {
            'mantissa': mantissa,
            'groups': groups,
            'depth': len(groups),
            'all_hit': all(r['pt_hit'] for r in results),
            'details': results,
        }


def selftest():
    c = Cube27()

    # 1. Encode
    assert c.encode(347) == [347]
    assert c.encode(0) == [0]
    assert c.encode(123456789) == [123, 456, 789]
    assert c.encode(1) == [1]
    assert c.encode(1000000) == [1, 0, 0]
    print("  cube27: encode OK")

    # 2. Cell boundaries sum to 1000
    assert c._cell_bounds[-1] == 999
    print("  cube27: cell boundaries OK")

    # 3. Cell index
    assert c.cell_index(0) == 0
    assert c.cell_index(36) == 0
    assert c.cell_index(37) == 1
    assert c.cell_index(999) == 26
    print("  cube27: cell_index OK")

    # 4. Cell 27 mapping
    assert c.cell_27(0) == (0, 0, 0)
    assert c.cell_27(37) == (0, 0, 1)
    assert c.cell_27(296) == (0, 2, 2)   # last value of cell 8
    assert c.cell_27(333) == (1, 0, 0)   # cell 9: 333..369
    assert c.cell_27(665) == (1, 2, 2)   # last of cell 17
    assert c.cell_27(666) == (2, 0, 0)   # cell 18: 666..702
    assert c.cell_27(999) == (2, 2, 2)
    print("  cube27: cell_27 OK")

    # 5. Path
    path = c.path_27(123456789)
    assert len(path) == 3
    assert path[0] == c.cell_27(123)
    assert path[1] == c.cell_27(456)
    assert path[2] == c.cell_27(789)
    print("  cube27: path_27 OK")

    # 6. Depth
    assert c.depth(347) == 1
    assert c.depth(123456789) == 3
    print("  cube27: depth OK")

    # 7. All groups ≤ MAX_COORD (100% PtTable hit)
    for v in [0, 1, 42, 347, 999999, 1000000, 123456789, 999999999999]:
        info = c.verify(v)
        assert info['all_hit'], f"miss for {v}: {info}"
    print("  cube27: 100% PtTable hit OK")

    # 8. Format
    fmt = c.format_path(123456789)
    assert '123|456|789' in fmt
    print("  cube27: format_path OK")

    print("  cube27: all tests pass")


if __name__ == '__main__':
    selftest()
