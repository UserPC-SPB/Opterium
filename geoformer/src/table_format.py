"""table_format.py — Binary table format (.ptbl) for Opterium GeoFormer.

Replaces pickle cache with a flat, platform-independent binary format.
Tables are stored as contiguous int32 arrays, memory-mappable by native code.

Format:
  Header (256 bytes):
    0-3:   Magic 'PTBL'
    4-7:   Version (uint32)
    8-11:  max_coord (uint32)
    12-15: offset (uint32) — D offset for _SP
    16-19: scale (uint32) — proximity SCALE
    20-23: prox_len (uint32)
    24-27: sp_dim (uint32)
    28-31: dim (uint32)
    32-35: _P offset (uint32, from start of file)
    36-39: _S offset
    40-43: _D offset
    44-47: _SP offset
    48-51: _prox offset
    52-55: _isqrt offset
    56-59: _pow10 offset
    60-63: _abs offset
    64-255: Reserved (zeros)

  Tables (contiguous int32 arrays, little-endian):
    _P:    dim * dim ints
    _S:    dim * dim ints
    _D:    dim * dim ints
    _SP:   sp_dim * sp_dim ints
    _prox: prox_len ints
    _isqrt: 1025 ints
    _pow10: 11 ints
    _abs:  2*max_coord+1 ints
"""

import struct
import os
import sys

MAGIC = b'PTBL'
VERSION = 1
HEADER_SIZE = 256


def save_ptbl(pt, path):
    """Save PtTable to .ptbl binary file."""
    mc = pt.max_coord
    dim = mc + 1
    sp_dim = 2 * mc + 1
    offset = mc
    scale = 10000
    prox_len = 4 * mc + 1

    # Calculate offsets (absolute from start of file)
    base = HEADER_SIZE
    p_off = base
    s_off = p_off + dim * dim * 4
    d_off = s_off + dim * dim * 4
    sp_off = d_off + dim * dim * 4
    prox_off = sp_off + sp_dim * sp_dim * 4
    isqrt_off = prox_off + prox_len * 4
    pow10_off = isqrt_off + 1025 * 4
    abs_off = pow10_off + 11 * 8  # int64 for pow10

    total_size = abs_off + (2 * mc + 1) * 4

    with open(path, 'wb') as f:
        # Header
        f.write(MAGIC)
        f.write(struct.pack('<IIIIIII', VERSION, mc, offset, scale, prox_len, sp_dim, dim))
        f.write(struct.pack('<IIIIIII', p_off, s_off, d_off, sp_off, prox_off, isqrt_off, pow10_off))
        f.write(struct.pack('<I', abs_off))
        f.write(b'\x00' * (HEADER_SIZE - f.tell()))

        # Tables
        for x in range(dim):
            for y in range(dim):
                f.write(struct.pack('<i', pt._P[x][y]))
        for x in range(dim):
            for y in range(dim):
                f.write(struct.pack('<i', pt._S[x][y]))
        for x in range(dim):
            for y in range(dim):
                f.write(struct.pack('<i', pt._D[x][y]))
        for s in range(sp_dim):
            for d in range(sp_dim):
                f.write(struct.pack('<i', pt._SP[s][d]))
        for v in pt._prox:
            f.write(struct.pack('<i', v))
        for n in range(1025):
            f.write(struct.pack('<i', pt._isqrt.get(n, 0)))
        # _pow10: 10^10 exceeds int32, use int64 for these 11 values
        for v in pt._pow10:
            f.write(struct.pack('<q', v))
        for v in range(-mc, mc + 1):
            f.write(struct.pack('<i', pt._abs.get(v, abs(v))))

    return total_size


class BinaryTables:
    """Read-only view into .ptbl file via memoryview.
    
    This is the Python equivalent of what native code will do with mmap.
    All tables are accessed as int32 arrays, zero Python objects created.
    """

    def __init__(self, path):
        with open(path, 'rb') as f:
            self._data = f.read()

        self._mv = memoryview(self._data)

        # Parse header
        hdr = struct.unpack('<4s17I', self._mv[0:72].tobytes())
        assert hdr[0] == MAGIC, f"Invalid magic: {hdr[0]}"

        self.max_coord = hdr[2]
        self.offset = hdr[3]
        self.scale = hdr[4]
        self.dim = hdr[7]
        self.sp_dim = hdr[6]

        # Create typed views (memoryview.cast to int32)
        self._P = self._mv[hdr[8]:hdr[8] + self.dim * self.dim * 4].cast('i')
        self._S = self._mv[hdr[9]:hdr[9] + self.dim * self.dim * 4].cast('i')
        self._D = self._mv[hdr[10]:hdr[10] + self.dim * self.dim * 4].cast('i')
        self._SP = self._mv[hdr[11]:hdr[11] + self.sp_dim * self.sp_dim * 4].cast('i')
        self._prox = self._mv[hdr[12]:hdr[12] + (4 * self.max_coord + 1) * 4].cast('i')
        self._isqrt = self._mv[hdr[13]:hdr[13] + 1025 * 4].cast('i')
        # _pow10 is int64 (11 values)
        self._pow10 = self._mv[hdr[14]:hdr[14] + 11 * 8].cast('q')
        self._abs = self._mv[hdr[15]:hdr[15] + (2 * self.max_coord + 1) * 4].cast('i')

    def P(self, x, y):
        return self._P[x * self.dim + y]

    def S(self, x, y):
        return self._S[x * self.dim + y]

    def D(self, x, y):
        return self._D[x * self.dim + y]

    def p_from_sd(self, s, d):
        return self._SP[s * self.sp_dim + (d + self.offset)]

    def proximity(self, dist):
        if 0 <= dist < len(self._prox):
            return self._prox[dist]
        return 0

    def isqrt(self, n):
        if 0 <= n < 1025:
            return self._isqrt[n]
        return 0

    def pow10(self, n):
        if 0 <= n < 11:
            return self._pow10[n]
        return 10 ** n

    def abs(self, x):
        idx = x + self.max_coord
        if 0 <= idx < len(self._abs):
            return self._abs[idx]
        return abs(x)

    def product(self, a, b):
        """Product via _P lookup. No gcd-scaling in binary format."""
        if 0 <= a <= self.max_coord and 0 <= b <= self.max_coord:
            return self._P[a * self.dim + b]
        # Fallback: direct multiplication (should not happen in hot paths)
        return a * b


def verify(pt, bt):
    """Verify BinaryTables matches PtTable."""
    import random
    random.seed(42)
    errors = 0

    for _ in range(1000):
        x, y = random.randint(0, pt.max_coord), random.randint(0, pt.max_coord)
        if pt.P(x, y) != bt.P(x, y):
            errors += 1
            print(f"  P mismatch: {x},{y} -> {pt.P(x,y)} vs {bt.P(x,y)}")
        if pt.S(x, y) != bt.S(x, y):
            errors += 1
            print(f"  S mismatch: {x},{y} -> {pt.S(x,y)} vs {bt.S(x,y)}")
        if pt.D(x, y) != bt.D(x, y):
            errors += 1
            print(f"  D mismatch: {x},{y} -> {pt.D(x,y)} vs {bt.D(x,y)}")
        s, d = x + y, x - y
        if pt.p_from_sd(s, d) != bt.p_from_sd(s, d):
            errors += 1
            print(f"  p_from_sd mismatch: {s},{d} -> {pt.p_from_sd(s,d)} vs {bt.p_from_sd(s,d)}")
        dist = random.randint(0, 4096)
        if pt.proximity(dist) != bt.proximity(dist):
            errors += 1
            print(f"  proximity mismatch: {dist} -> {pt.proximity(dist)} vs {bt.proximity(dist)}")
        if errors > 10:
            print("  ... (stopping after 10 errors)")
            break

    return errors


if __name__ == '__main__':
    from arith_table import PT

    path = os.path.join(os.path.dirname(__file__), 'tables.ptbl')
    size = save_ptbl(PT, path)
    print(f"Saved {path}: {size} bytes ({size/1024/1024:.1f} MB)")

    bt = BinaryTables(path)
    errors = verify(PT, bt)
    if errors == 0:
        print("Verification: 1000 random tests passed")
    else:
        print(f"Verification: {errors} errors")
        sys.exit(1)
