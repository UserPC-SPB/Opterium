"""
Opterium Decoder — S/D Address → Token

Maps geometric coordinates (S, D) back to semantic tokens.
The decoder is a small trainable layer that translates table
addresses back into tokens.

Architecture:
  (S, D) address → normalize → MLP → token distribution → argmax

Zero weights in reasoning — all reasoning happens in the table.
"""

import math
import random
from typing import Dict, List, Tuple, Optional

class TokenDecoder:
    """Decodes S/D addresses back to tokens via projection + classification."""
    
    def __init__(self, vocab_size: int = 1000, embed_dim: int = 64, max_coord: int = 1024):
        self.vocab_size = vocab_size
        self.embed_dim = embed_dim
        self.max_coord = max_coord
        
        # Projection weights: 2 (S, D) → embed_dim
        self.W = [[random.gauss(0, 0.1) for _ in range(2)] for _ in range(embed_dim)]
        self.b = [0.0] * embed_dim
        
        # Classification weights: embed_dim → vocab_size
        self.W_out = [[random.gauss(0, 0.01) for _ in range(embed_dim)] for _ in range(vocab_size)]
        self.b_out = [0.0] * vocab_size
        
        # Address → token cache (for fast lookup after training)
        self._cache: Dict[Tuple[int, int], int] = {}

    def decode(self, s: int, d: int) -> int:
        """Decode (S, D) address to token_id.
        
        Steps:
        1. Normalize coordinates
        2. Project to embedding space
        3. Classify to token
        """
        addr = (s, d)
        if addr in self._cache:
            return self._cache[addr]
        
        # Normalize to [-1, 1]
        s_norm = (s / self.max_coord) * 2 - 1
        d_norm = (d / self.max_coord) * 2 - 1
        
        # Project to embedding space
        hidden = [0.0] * self.embed_dim
        for i in range(self.embed_dim):
            hidden[i] = self.b[i] + self.W[i][0] * s_norm + self.W[i][1] * d_norm
            hidden[i] = math.tanh(hidden[i])
        
        # Classify to token
        scores = [0.0] * self.vocab_size
        for t in range(self.vocab_size):
            score = self.b_out[t]
            for i in range(self.embed_dim):
                score += self.W_out[t][i] * hidden[i]
            scores[t] = score
        
        # Argmax
        token_id = max(range(self.vocab_size), key=lambda t: scores[t])
        
        self._cache[addr] = token_id
        return token_id

    def decode_batch(self, addresses: List[Tuple[int, int]]) -> List[int]:
        """Decode a batch of (S, D) addresses."""
        return [self.decode(s, d) for s, d in addresses]

    def decode_probs(self, s: int, d: int) -> List[float]:
        """Decode (S, D) to token probability distribution."""
        # Normalize
        s_norm = (s / self.max_coord) * 2 - 1
        d_norm = (d / self.max_coord) * 2 - 1
        
        # Project
        hidden = [0.0] * self.embed_dim
        for i in range(self.embed_dim):
            hidden[i] = self.b[i] + self.W[i][0] * s_norm + self.W[i][1] * d_norm
            hidden[i] = math.tanh(hidden[i])
        
        # Scores
        scores = [0.0] * self.vocab_size
        for t in range(self.vocab_size):
            score = self.b_out[t]
            for i in range(self.embed_dim):
                score += self.W_out[t][i] * hidden[i]
            scores[t] = score
        
        # Softmax
        max_score = max(scores)
        exp_scores = [math.exp(s - max_score) for s in scores]
        total = sum(exp_scores)
        probs = [e / total for e in exp_scores]
        
        return probs

    def train_step(self, s: int, d: int, target_token: int, lr: float = 0.01) -> float:
        """Single training step: adjust decoder to match target token.
        
        Returns loss value.
        """
        # Simplified training: just adjust the output weights for the target token
        # to increase its score for this address
        
        # Normalize
        s_norm = (s / self.max_coord) * 2 - 1
        d_norm = (d / self.max_coord) * 2 - 1
        
        # Project
        hidden = [0.0] * self.embed_dim
        for i in range(self.embed_dim):
            hidden[i] = self.b[i] + self.W[i][0] * s_norm + self.W[i][1] * d_norm
            hidden[i] = math.tanh(hidden[i])
        
        # Increase score for target token
        for i in range(self.embed_dim):
            self.W_out[target_token][i] += lr * hidden[i]
        self.b_out[target_token] += lr
        
        # Clear cache
        if (s, d) in self._cache:
            del self._cache[(s, d)]
        
        # Compute loss (cross-entropy approximation)
        probs = self.decode_probs(s, d)
        loss = -math.log(max(probs[target_token], 1e-10))
        return loss

    def precompute_all(self, addresses: List[Tuple[int, int]]):
        """Precompute tokens for a list of addresses."""
        for s, d in addresses:
            self.decode(s, d)

    def save(self, path: str):
        """Save decoder state to file."""
        import json
        state = {
            'vocab_size': self.vocab_size,
            'embed_dim': self.embed_dim,
            'max_coord': self.max_coord,
            'W': self.W,
            'b': self.b,
            'W_out': self.W_out,
            'b_out': self.b_out,
            'cache': {f"{k[0]},{k[1]}": v for k, v in self._cache.items()},
        }
        with open(path, 'w') as f:
            json.dump(state, f)

    @classmethod
    def load(cls, path: str) -> 'TokenDecoder':
        """Load decoder state from file."""
        import json
        with open(path, 'r') as f:
            state = json.load(f)
        
        decoder = cls(
            vocab_size=state['vocab_size'],
            embed_dim=state['embed_dim'],
            max_coord=state['max_coord'],
        )
        decoder.W = state['W']
        decoder.b = state['b']
        decoder.W_out = state['W_out']
        decoder.b_out = state['b_out']
        decoder._cache = {tuple(map(int, k.split(','))): v for k, v in state['cache'].items()}
        return decoder
