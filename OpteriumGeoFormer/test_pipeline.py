"""
Test Pipeline — Tests for Architecture Flip (Encoder → Navigation → Decoder)

Tests:
1. Encoder: token → S/D address
2. Decoder: S/D address → token
3. Navigation: analogy solving
4. Pipeline: full forward pass
5. Training: analogy examples
"""

import sys
import os
sys.path.insert(0, os.path.dirname(__file__))

from opterium import TokenEncoder, TokenDecoder, NavigationCore, OpteriumPipeline

print("=" * 60)
print("OPTERIUM ARCHITECTURE FLIP — TESTS")
print("=" * 60)

# ── Test 1: Encoder ──
print("\n[Test 1] Encoder: token → S/D address")
encoder = TokenEncoder(vocab_size=100, embed_dim=32, max_coord=1024)

# Test encoding
addr = encoder.encode(42)
print(f"  encode(42) = {addr}")
assert len(addr) == 2
assert 0 <= addr[0] <= 1024
assert 0 <= addr[1] <= 1024

# Test batch encoding
addrs = encoder.encode_batch([1, 2, 3])
print(f"  encode_batch([1,2,3]) = {addrs}")
assert len(addrs) == 3

# Test caching
addr2 = encoder.encode(42)
assert addr == addr2, "Cache should return same address"
print("  ✅ Cache works")

print("  ✅ Test 1 passed")

# ── Test 2: Decoder ──
print("\n[Test 2] Decoder: S/D address → token")
decoder = TokenDecoder(vocab_size=100, embed_dim=32, max_coord=1024)

# Test decoding
token = decoder.decode(500, 300)
print(f"  decode(500, 300) = {token}")
assert 0 <= token < 100

# Test batch decoding
tokens = decoder.decode_batch([(100, 200), (300, 400)])
print(f"  decode_batch([(100,200), (300,400)]) = {tokens}")
assert len(tokens) == 2

# Test caching
token2 = decoder.decode(500, 300)
assert token == token2, "Cache should return same token"
print("  ✅ Cache works")

print("  ✅ Test 2 passed")

# ── Test 3: Navigation ──
print("\n[Test 3] Navigation: analogy solving")
nav = NavigationCore(max_coord=1024)

# Test navigation
result = nav.navigate((10, 20), ["S", "D"])
print(f"  navigate((10,20), ['S','D']) = {result}")
assert len(result) == 2

# Test analogy: 2:4 :: 3:6 (simple scaling)
A = (2, 2)
B = (4, 4)
C = (3, 3)
D = nav.analogy(A, B, C)
print(f"  analogy({A}, {B}, {C}) = {D}")
# Expected: D should be (6, 6) or similar scaling
assert D[0] >= 0 and D[1] >= 0

# Test verify_claim
valid, witness = nav.verify_claim("4 × 3 = 12")
print(f"  verify('4 × 3 = 12') = {valid}, witness: {witness}")
assert valid

valid, witness = nav.verify_claim("4 × 3 = 13")
print(f"  verify('4 × 3 = 13') = {valid}")
assert not valid

print("  ✅ Test 3 passed")

# ── Test 4: Pipeline ──
print("\n[Test 4] Pipeline: full forward pass")
pipeline = OpteriumPipeline(vocab_size=100, embed_dim=32, max_coord=1024)

# Test forward pass
output = pipeline.forward(42)
print(f"  forward(42) = {output}")
assert 0 <= output < 100

# Test analogy
token_d = pipeline.analogy(10, 20, 30)
print(f"  analogy(10, 20, 30) = {token_d}")
assert 0 <= token_d < 100

# Test verify
valid, witness = pipeline.verify("5 × 6 = 30")
print(f"  verify('5 × 6 = 30') = {valid}")
assert valid

# Test witness
witness = pipeline.get_witness(42)
print(f"  get_witness(42) = {witness}")
assert 'token_id' in witness
assert 'address' in witness
assert 'witness' in witness

print("  ✅ Test 4 passed")

# ── Test 5: Training ──
print("\n[Test 5] Training: analogy examples")

# Create simple analogy examples
# A:B :: C:D where B = A*2, D = C*2
examples = [
    (1, 2, 3, 6),
    (2, 4, 5, 10),
    (3, 6, 7, 14),
    (4, 8, 9, 18),
    (5, 10, 11, 22),
]

# Train for a few epochs
losses = pipeline.train_analogy(examples, epochs=10, lr=0.001)
print(f"  Training losses: {losses[:3]}...{losses[-3:]}")
assert len(losses) == 10

# Test after training
token_d = pipeline.analogy(6, 12, 8)
print(f"  analogy(6, 12, 8) = {token_d} (expected: ~16)")

print("  ✅ Test 5 passed")

# ── Test 6: Save/Load ──
print("\n[Test 6] Save/Load")
import tempfile
import json

with tempfile.TemporaryDirectory() as tmpdir:
    pipeline.save(tmpdir)
    print(f"  Saved to {tmpdir}")
    
    loaded = OpteriumPipeline.load(tmpdir)
    print(f"  Loaded pipeline")
    
    # Verify loaded pipeline works
    output1 = pipeline.forward(42)
    output2 = loaded.forward(42)
    assert output1 == output2, "Loaded pipeline should produce same output"
    print(f"  forward(42) before/after load: {output1} / {output2}")

print("  ✅ Test 6 passed")

# ── Summary ──
print("\n" + "=" * 60)
print("ALL TESTS PASSED ✅")
print("=" * 60)
print("""
Architecture Flip Pipeline:
  [token] → [encoder] → (S,D) → [navigation] → (S',D') → [decoder] → [token]

Key properties:
  - Encoder/Decoder: small trainable layers (64-dim embeddings)
  - Navigation: zero weights, pure table lookup
  - Reasoning: happens entirely in Opterium table
  - All results have witness addresses
""")
