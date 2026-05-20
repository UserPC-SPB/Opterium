"""
hashgrid.py  —  O(1) neighbor lookup in (S, D) space

Partition: (S, D) plane → W×W buckets.
Lookup: query bucket + 3×3 adjacent = O(k) where k = avg bucket size.

Replaces: QK^T / √d pairwise attention (O(n²·d)).
Usage: geometric proximity for GeoFormer attention.
"""

from __future__ import annotations
import sys, os
sys.path.insert(0, os.path.dirname(__file__))
import struct
from typing import Dict, List, Optional, Tuple, Any
from arith_table import PT

# ───────────────────────────────────────────────────────
# Spatial hash grid
# ───────────────────────────────────────────────────────
class HashGrid:
    """Spatial hash over (S, D) integer coordinates.

    Bucket key: (S // W, D // W).  Lookup returns all points
    in the bucket and its 8 neighbors (Moor neighborhood).

    Complexity:
        insert:  O(1)
        lookup:  O(k) where k = 9 × avg_bucket_size
    """

    def __init__(self, window: int = 16):
        if window < 1:
            raise ValueError(f"window must be >= 1, got {window}")
        self.W = window
        self._buckets: Dict[Tuple[int, int], List[Dict[str, Any]]] = {}

    # ── bucket key ─────────────────────────────────────
    def _key(self, S: int, D: int) -> Tuple[int, int]:
        return (S // self.W, D // self.W)

    def _keys_3x3(self, S: int, D: int) -> List[Tuple[int, int]]:
        bk = self._key(S, D)
        return [(bk[0] + dx, bk[1] + dy)
                for dx in (-1, 0, 1)
                for dy in (-1, 0, 1)]

    # ── insert ──────────────────────────────────────────
    def insert(self, token_id: int, S: int, D: int, **extra) -> int:
        """Insert point, return bucket size after insertion."""
        k = self._key(S, D)
        if k not in self._buckets:
            self._buckets[k] = []
        self._buckets[k].append({
            'id': token_id, 'S': S, 'D': D, **extra,
        })
        return len(self._buckets[k])

    # ── batch insert ────────────────────────────────────
    def insert_many(self, tokens: List[Tuple[int, int, int]], **extras):
        """tokens: list of (id, S, D).  extras added to every entry."""
        for tid, S, D in tokens:
            self.insert(tid, S, D, **extras)

    # ── lookup ──────────────────────────────────────────
    def lookup(self, S: int, D: int) -> List[Dict[str, Any]]:
        """Return all entries in the bucket + 8 neighbors."""
        result: List[Dict[str, Any]] = []
        seen: set = set()
        for key in self._keys_3x3(S, D):
            for entry in self._buckets.get(key, ()):
                eid = entry['id']
                if eid not in seen:
                    seen.add(eid)
                    result.append(entry)
        return result

    # ── stats ───────────────────────────────────────────
    def stats(self) -> Dict:
        if not self._buckets:
            return {'buckets': 0, 'total': 0, 'avg': 0.0, 'max': 0}
        sizes = [len(v) for v in self._buckets.values()]
        return {
            'buckets': len(self._buckets),
            'total': sum(sizes),
            'avg': sum(sizes) / len(sizes),
            'max': max(sizes),
        }

    def clear(self):
        self._buckets.clear()


# ───────────────────────────────────────────────────────
# Proximity weighting  (replaces QK^T / √d)
# ───────────────────────────────────────────────────────
def geometric_weight(S1: int, D1: int, S2: int, D2: int, eps: int = 0) -> int:
    """Integer proximity weight between two points in (S,D) space.

    weight = PT.proximity(|ΔS| + |ΔD| + eps)
    Replaces 1/(eps + |ΔS| + |ΔD|) — no float, pure table lookup.
    """
    dist = abs(S1 - S2) + abs(D1 - D2) + eps
    return PT.proximity(dist)


# ───────────────────────────────────────────────────────
# Geometric attention (single layer, replaces QK^T·V)
# ───────────────────────────────────────────────────────
def geometric_attention(
    tokens: List[Tuple[int, int, int, int]],  # (id, S, D, P)
    window: int = 16,
    eps: float = 1.0,
    include_self: bool = False,
) -> List[Dict]:
    """One layer of geometric attention.

    For each token:
      1. hashgrid lookup → neighbors
      2. geometric_weight → proximity score
      3. weighted sum of P values → context
      4. Pt3 triple product → output (x, y)

    No QK^T, no softmax, no V matrix.
    Complexity: O(n · k) where k = avg bucket size.

    Args:
        tokens: list of (id, S, D, P)
        window: hashgrid bucket size
        eps: distance smoothing
        include_self: whether to include the query token in its own context

    Returns:
        list of dict with 'id', 'context', 'neighbors', 'output_x', 'output_y'
    """
    if not tokens:
        return []

    grid = HashGrid(window=window)
    for tid, S, D, P in tokens:
        grid.insert(tid, S, D, P=P)

    outputs = []
    for tid, S_q, D_q, P_q in tokens:
        neighbors = grid.lookup(S_q, D_q)

        if not include_self:
            neighbors = [nb for nb in neighbors if nb['id'] != tid]

        w_total = 0
        p_weighted = 0

        for nb in neighbors:
            w = geometric_weight(S_q, D_q, nb['S'], nb['D'])
            w_total += w
            p_weighted += w * nb['P']

        context = p_weighted // w_total if w_total > 0 else P_q
        n_neighbors = len(neighbors)

        # Pt3 triple product: x·y·context → geometric mixing
        mixed = PT.product(P_q, context)
        out_x = PT.isqrt(mixed) if mixed > 0 else 0
        out_y = out_x if out_x > 0 else 1

        outputs.append({
            'id': tid,
            'context': context,
            'neighbors': n_neighbors,
            'output_x': out_x,
            'output_y': out_y,
            'output_P': PT.product(out_x, out_y),
        })

    return outputs


# ───────────────────────────────────────────────────────
# Self-test
# ───────────────────────────────────────────────────────
def selftest():
    import random
    random.seed(42)

    # 1. HashGrid insert/lookup
    g = HashGrid(window=10)
    g.insert(0, 5, 3)
    g.insert(1, 7, 2)
    g.insert(2, 100, 50)

    nb = g.lookup(6, 3)
    assert len(nb) >= 2  # ids 0 and 1 in same bucket neighborhood
    assert g.stats()['total'] == 3
    print("  hashgrid: insert/lookup OK")

    # 2. HashGrid empty lookup
    assert g.lookup(999, 999) == []
    print("  hashgrid: empty lookup OK")

    # 3. Geometric weight symmetry (integer)
    w1 = geometric_weight(10, 5, 20, 15)
    w2 = geometric_weight(20, 15, 10, 5)
    assert w1 == w2, f"weight asymmetry: {w1} != {w2}"
    assert isinstance(w1, int), f"weight not int: {type(w1)}"
    print("  hashgrid: weight symmetry OK (int)")

    # 4. Geometric attention
    pts = [(i, random.randint(1, 100), random.randint(-50, 50), random.randint(1, 100))
           for i in range(20)]
    out = geometric_attention(pts, window=20)
    assert len(out) == 20
    for o in out:
        assert 'context' in o
        assert 'output_P' in o
    print("  hashgrid: geometric_attention OK")

    # 5. Clear
    g.clear()
    assert g.stats()['total'] == 0
    print("  hashgrid: clear OK")

    print("  hashgrid: all tests pass")


if __name__ == '__main__':
    selftest()
