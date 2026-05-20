"""
Opterium Encoder — Token → S/D Address

Maps semantic tokens to geometric coordinates (S, D).
The encoder is a small trainable layer that translates tokens
into addresses in the Opterium table.

Architecture:
  token_id → embedding → MLP → (S, D) address

Zero weights in reasoning — all reasoning happens in the table.
"""

import math
import random
from typing import Dict, List, Tuple, Optional

class TokenEncoder:
    """Encodes tokens to S/D addresses via trainable embedding + projection."""
    
    def __init__(self, vocab_size: int = 1000, embed_dim: int = 64, max_coord: int = 1024):
        self.vocab_size = vocab_size
        self.embed_dim = embed_dim
        self.max_coord = max_coord
        
        # Embedding table: token_id → vector
        self.embeddings = self._init_embeddings()
        
        # Projection weights: embed_dim → 2 (S, D)
        self.W = [[random.gauss(0, 0.1) for _ in range(embed_dim)] for _ in range(2)]
        self.b = [0.0, 0.0]
        
        # Token → address cache (for fast lookup after training)
        self._cache: Dict[int, Tuple[int, int]] = {}

    def _init_embeddings(self) -> List[List[float]]:
        """Initialize embeddings with small random values."""
        return [
            [random.gauss(0, 0.02) for _ in range(self.embed_dim)]
            for _ in range(self.vocab_size)
        ]

    def encode(self, token_id: int) -> Tuple[int, int]:
        """Encode a token to (S, D) address.
        
        Steps:
        1. Lookup embedding
        2. Project to (S, D)
        3. Scale to table coordinates
        """
        if token_id in self._cache:
            return self._cache[token_id]
        
        # Lookup embedding
        emb = self.embeddings[token_id]
        
        # Project to (S, D)
        s = self.b[0]
        d = self.b[1]
        for i in range(self.embed_dim):
            s += self.W[0][i] * emb[i]
            d += self.W[1][i] * emb[i]
        
        # Scale to table coordinates [0, max_coord]
        s = int((math.tanh(s) + 1) / 2 * self.max_coord)
        d = int((math.tanh(d) + 1) / 2 * self.max_coord)
        
        # Ensure valid coordinates
        s = max(0, min(self.max_coord, s))
        d = max(0, min(self.max_coord, d))
        
        result = (s, d)
        self._cache[token_id] = result
        return result

    def encode_batch(self, token_ids: List[int]) -> List[Tuple[int, int]]:
        """Encode a batch of tokens."""
        return [self.encode(t) for t in token_ids]

    def train_step(self, token_id: int, target_s: int, target_d: int, lr: float = 0.01) -> float:
        """Single training step: adjust embedding to match target (S, D).
        
        Returns loss value.
        """
        emb = self.embeddings[token_id]
        
        # Forward pass
        s = self.b[0]
        d = self.b[1]
        for i in range(self.embed_dim):
            s += self.W[0][i] * emb[i]
            d += self.W[1][i] * emb[i]
        
        # Target normalized
        target_s_norm = (target_s / self.max_coord) * 2 - 1
        target_d_norm = (target_d / self.max_coord) * 2 - 1
        
        # Loss: MSE
        loss_s = (math.tanh(s) - target_s_norm) ** 2
        loss_d = (math.tanh(d) - target_d_norm) ** 2
        loss = (loss_s + loss_d) / 2
        
        # Gradient descent (simplified)
        # Update embedding
        grad_s = 2 * (math.tanh(s) - target_s_norm) * (1 - math.tanh(s) ** 2)
        grad_d = 2 * (math.tanh(d) - target_d_norm) * (1 - math.tanh(d) ** 2)
        
        for i in range(self.embed_dim):
            self.embeddings[token_id][i] -= lr * (grad_s * self.W[0][i] + grad_d * self.W[1][i])
        
        # Clear cache
        if token_id in self._cache:
            del self._cache[token_id]
        
        return loss

    def precompute_all(self) -> Dict[int, Tuple[int, int]]:
        """Precompute addresses for all tokens in vocabulary."""
        for token_id in range(self.vocab_size):
            self.encode(token_id)
        return self._cache

    def save(self, path: str):
        """Save encoder state to file."""
        import json
        state = {
            'vocab_size': self.vocab_size,
            'embed_dim': self.embed_dim,
            'max_coord': self.max_coord,
            'embeddings': self.embeddings,
            'W': self.W,
            'b': self.b,
            'cache': {str(k): list(v) for k, v in self._cache.items()},
        }
        with open(path, 'w') as f:
            json.dump(state, f)

    @classmethod
    def load(cls, path: str) -> 'TokenEncoder':
        """Load encoder state from file."""
        import json
        with open(path, 'r') as f:
            state = json.load(f)
        
        encoder = cls(
            vocab_size=state['vocab_size'],
            embed_dim=state['embed_dim'],
            max_coord=state['max_coord'],
        )
        encoder.embeddings = state['embeddings']
        encoder.W = state['W']
        encoder.b = state['b']
        encoder._cache = {int(k): tuple(v) for k, v in state['cache'].items()}
        return encoder
