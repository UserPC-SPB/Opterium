"""
Opterium Pipeline — Encoder → Navigation → Decoder

Full architecture flip pipeline:
  token → [encoder] → S/D → [navigation] → S/D → [decoder] → token

The navigation core does all reasoning with zero weights.
Encoder and decoder are small trainable layers.
"""

import sys
import os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), '..'))

from opterium.encoder import TokenEncoder
from opterium.decoder import TokenDecoder
from opterium.navigation import NavigationCore
from typing import List, Tuple, Dict, Optional

class OpteriumPipeline:
    """Full pipeline: token → S/D → reasoning → S/D → token."""
    
    def __init__(self, vocab_size: int = 1000, embed_dim: int = 64, max_coord: int = 1024):
        self.encoder = TokenEncoder(vocab_size, embed_dim, max_coord)
        self.decoder = TokenDecoder(vocab_size, embed_dim, max_coord)
        self.navigation = NavigationCore(max_coord)
        self.vocab_size = vocab_size
        self.max_coord = max_coord
    
    def forward(self, token_id: int, operations: List[str] = None) -> int:
        """Forward pass: token → encode → navigate → decode → token.
        
        Args:
            token_id: input token
            operations: list of navigation operations (default: ["S", "D"])
        
        Returns:
            output token_id
        """
        if operations is None:
            operations = ["S", "D"]
        
        # Encode: token → (S, D)
        s, d = self.encoder.encode(token_id)
        
        # Navigate: (S, D) → reasoning → (S', D')
        # Convert (S, D) back to (x, y) for navigation
        x = (s + d) // 2
        y = (s - d) // 2
        x = max(0, min(self.max_coord, x))
        y = max(0, min(self.max_coord, y))
        
        result_addr = self.navigation.navigate((x, y), operations)
        
        # Decode: (S', D') → token
        s_out = self.navigation.PT.S(*result_addr)
        d_out = self.navigation.PT.D(*result_addr)
        
        output_token = self.decoder.decode(s_out, d_out)
        return output_token
    
    def analogy(self, token_a: int, token_b: int, token_c: int) -> int:
        """Solve analogy: A:B :: C:D using Opterium navigation.
        
        Args:
            token_a: token A
            token_b: token B
            token_c: token C
        
        Returns:
            token D (the answer to the analogy)
        """
        # Encode all tokens to addresses
        addr_a = self.encoder.encode(token_a)
        addr_b = self.encoder.encode(token_b)
        addr_c = self.encoder.encode(token_c)
        
        # Convert to (x, y) coordinates
        def to_xy(s, d):
            x = (s + d) // 2
            y = (s - d) // 2
            return (max(0, min(self.max_coord, x)), max(0, min(self.max_coord, y)))
        
        xy_a = to_xy(*addr_a)
        xy_b = to_xy(*addr_b)
        xy_c = to_xy(*addr_c)
        
        # Navigate: analogy
        xy_d = self.navigation.analogy(xy_a, xy_b, xy_c)
        
        # Get S/D for decoder
        s_d = self.navigation.PT.S(*xy_d)
        d_d = self.navigation.PT.D(*xy_d)
        
        # Decode to token
        token_d = self.decoder.decode(s_d, d_d)
        return token_d
    
    def train_analogy(self, examples: List[Tuple[int, int, int, int]], epochs: int = 10, lr: float = 0.01) -> List[float]:
        """Train encoder and decoder on analogy examples.
        
        Examples: [(A, B, C, D), ...] where A:B :: C:D
        """
        losses = []
        
        for epoch in range(epochs):
            epoch_loss = 0.0
            
            for a, b, c, d in examples:
                # Encode A, B, C
                addr_a = self.encoder.encode(a)
                addr_b = self.encoder.encode(b)
                addr_c = self.encoder.encode(c)
                
                # Compute analogy result
                def to_xy(s, d):
                    x = (s + d) // 2
                    y = (s - d) // 2
                    return (max(0, min(self.max_coord, x)), max(0, min(self.max_coord, y)))
                
                xy_a = to_xy(*addr_a)
                xy_b = to_xy(*addr_b)
                xy_c = to_xy(*addr_c)
                
                xy_d = self.navigation.analogy(xy_a, xy_b, xy_c)
                
                # Decode to get predicted token
                s_d = self.navigation.PT.S(*xy_d)
                d_d = self.navigation.PT.D(*xy_d)
                
                # Train decoder to output D for this address
                loss = self.decoder.train_step(s_d, d_d, d, lr)
                epoch_loss += loss
                
                # Train encoder to produce addresses that lead to D
                # (simplified: just adjust A, B, C embeddings)
                self.encoder.train_step(a, addr_a[0], addr_a[1], lr * 0.1)
                self.encoder.train_step(b, addr_b[0], addr_b[1], lr * 0.1)
                self.encoder.train_step(c, addr_c[0], addr_c[1], lr * 0.1)
            
            avg_loss = epoch_loss / len(examples)
            losses.append(avg_loss)
            
            if epoch % 5 == 0:
                print(f"Epoch {epoch}: loss = {avg_loss:.4f}")
        
        return losses
    
    def train_token_address(self, token_id: int, target_addr: Tuple[int, int], epochs: int = 10, lr: float = 0.01) -> List[float]:
        """Train encoder to map token_id to target (S, D) address."""
        losses = []
        
        for epoch in range(epochs):
            loss = self.encoder.train_step(token_id, target_addr[0], target_addr[1], lr)
            losses.append(loss)
        
        return losses
    
    def verify(self, claim: str) -> Tuple[bool, str]:
        """Verify a claim using the navigation core."""
        return self.navigation.verify_claim(claim)
    
    def get_witness(self, token_id: int) -> Dict:
        """Get full witness for a token's reasoning path."""
        addr = self.encoder.encode(token_id)
        x = (addr[0] + addr[1]) // 2
        y = (addr[0] - addr[1]) // 2
        x = max(0, min(self.max_coord, x))
        y = max(0, min(self.max_coord, y))
        
        return {
            'token_id': token_id,
            'address': addr,
            'xy': (x, y),
            'witness': self.navigation.get_witness(x, y),
        }
    
    def save(self, path: str):
        """Save pipeline state."""
        import json
        os.makedirs(path, exist_ok=True)
        self.encoder.save(os.path.join(path, 'encoder.json'))
        self.decoder.save(os.path.join(path, 'decoder.json'))
    
    @classmethod
    def load(cls, path: str) -> 'OpteriumPipeline':
        """Load pipeline state."""
        import json
        encoder = TokenEncoder.load(os.path.join(path, 'encoder.json'))
        decoder = TokenDecoder.load(os.path.join(path, 'decoder.json'))
        
        pipeline = cls(
            vocab_size=encoder.vocab_size,
            embed_dim=encoder.embed_dim,
            max_coord=encoder.max_coord,
        )
        pipeline.encoder = encoder
        pipeline.decoder = decoder
        return pipeline
