"""
geo_resonant.py  —  Method 4: Hashgrid-Resonant Geometric Attention

No matrix multiplication at all. Replaces:
  - QK^T attention → hashgrid neighbor proximity weighting
  - FFN (Linear·ReLU·Linear) → Pt3(x, y, context) triple product
  - No torch.matmul, no numpy.dot, no float operations

Complexity: O(n·k) where k = bucket size (constant), not O(n²·d).

Architecture:
  embed(token) → Pt(S, D)    # geometric embedding
  hashgrid(S//W, D//W) → bucket  # O(1) spatial index
  weight = 1/(1+|ΔS|+|ΔD|)      # geometric distance, not dot product
  context = Σ w·P / Σ w          # weighted resonance, not V·softmax
  output = Pt3(x, y, context)    # triple product, not Linear·ReLU·Linear
"""

import sys, os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), '..', '..'))
from delta_ops import HealthVector, HEALTH_OK
from . import Pt
from arith_table import PT


class HashGrid:
    """O(1) neighbor lookup in (S, D) space.

    Buckets by (S // W, D // W). Each bucket holds tokens within a W×W window.
    lookup(S, D) returns tokens from (S//W, D//W) and 3×3 adjacent buckets.
    """
    def __init__(self, window: int = 16):
        self.W = window
        self.buckets: dict[tuple[int, int], list[dict]] = {}

    def _key(self, S: int, D: int) -> tuple[int, int]:
        return (S // self.W, D // self.W)

    def insert(self, token_id: int, pt: Pt):
        k = self._key(pt.S, pt.D)
        if k not in self.buckets:
            self.buckets[k] = []
        self.buckets[k].append({'id': token_id, 'x': pt.x, 'y': pt.y,
                                 'S': pt.S, 'D': pt.D, 'P': pt.P})

    def lookup(self, S: int, D: int) -> list[dict]:
        bk = self._key(S, D)
        neighbors = []
        for dx in (-1, 0, 1):
            for dy in (-1, 0, 1):
                key = (bk[0] + dx, bk[1] + dy)
                if key in self.buckets:
                    neighbors.extend(self.buckets[key])
        return neighbors


def geo_attention(tokens: list[Pt], window: int = 16) -> tuple[list[Pt], HealthVector]:
    """Geometric attention via hashgrid proximity.

    Args:
        tokens: list of Pt values (token embeddings in geometric space)
        window: bucket size for hashgrid

    Returns:
        (output_tokens, health_vector)
        Each output token = Pt(new_x, new_y) with context-mixed values.
    """
    n = len(tokens)
    if n == 0:
        return [], HEALTH_OK

    grid = HashGrid(window=window)
    for i, pt in enumerate(tokens):
        grid.insert(i, pt)

    outputs = []
    hv = HEALTH_OK

    for i, pt in enumerate(tokens):
        neighbors = grid.lookup(pt.S, pt.D)
        w_total = 0
        p_weighted = 0

        for nb in neighbors:
            if nb['id'] == i:
                continue
            dist = abs(nb['S'] - pt.S) + abs(nb['D'] - pt.D)
            weight = PT.proximity(dist)
            w_total += weight
            p_weighted += weight * nb['P']

        if w_total > 0:
            context = p_weighted // w_total
        else:
            context = pt.P

        # Pt3 triple product: x · y · context as geometric mixing
        new_P = pt.P * context
        new_x = PT.isqrt(new_P) if new_P > 0 else 0
        new_y = new_x  # symmetric restoration
        if new_x == 0:
            new_x, new_y = 1, 1

        outputs.append(Pt(new_x, new_y))

    return outputs, hv


def geo_resonant(tokens: list[Pt], layers: int = 4, window: int = 16) -> tuple[list[Pt], HealthVector]:
    """Full GeoFormer forward pass: stack of attention + projection layers.

    Each layer:
      1. geo_attention — hashgrid proximity mixing
      2. implicit projection via Pt3 (already done in geo_attention)
    No Linear, no ReLU, no matrix multiply.

    Args:
        tokens: list of Pt values
        layers: number of attention layers to stack
        window: hashgrid bucket size

    Returns:
        (output_tokens, health_vector)
    """
    current = tokens[:]
    hv = HEALTH_OK

    for layer in range(layers):
        current, h = geo_attention(current, window=window)
        hv = hv.merge(h)

    return current, hv


def embed_int_sequence(values: list[int], seed: int = 0) -> list[Pt]:
    """Embed a sequence of integers as Pt values in (S, D) space.

    Simple embedding: value → Pt(value, 1). No hash, no table.
    For production, use Δ_SHIFT with base_hash for distribution.
    """
    return [Pt(v, 1) for v in values]
