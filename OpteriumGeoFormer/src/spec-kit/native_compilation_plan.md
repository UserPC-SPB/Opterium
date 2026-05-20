# NATIVE COMPILATION — DETAIL PLAN
# Opterium GeoFormer → Opaque Native Binary
# 2026-05-19

## OVERVIEW

7 phases, 28 tasks. Target: standalone native library with opaque C API.
Python bindings as thin wrapper. Tables as memory-mapped read-only data.

Critical path: 1 → 2 → 3 → 4 → 6 → 7
Phase 5 (build system) runs in parallel.

---

## PHASE 1: Binary Table Format

### Task 1.1: Design .ptbl format

**Binary layout:**
```
Offset  Size    Field
------  ----    -----
0       4       Magic: b'PTBL'
4       4       Version: uint32 (1)
8       4       max_coord: uint32 (1024)
12      4       offset: uint32 (1024) — D offset for _SP
16      4       scale: uint32 (10000) — proximity SCALE
20      4       prox_len: uint32 (4097)
24      4       sp_dim: uint32 (2049)
28      4       dim: uint32 (1025)
32      4       _P_offset: uint32 (relative to header end)
36      4       _S_offset: uint32
40      4       _D_offset: uint32
44      4       _SP_offset: uint32
48      4       _prox_offset: uint32
52      4       _isqrt_offset: uint32
56      4       _pow10_offset: uint32
60      4       _abs_offset: uint32
64      ...     [Padding to 256 bytes]
256     ...     Tables (contiguous int32 arrays)
```

All tables are flat int32 arrays in row-major order.
Total size: ~56 MB + 256 byte header.

### Task 1.2: Implement table_format.py writer

**File:** `src/table_format.py`

```python
import struct
import os

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

    # Calculate offsets (relative to start of table data at byte 256)
    base = HEADER_SIZE
    p_off = base
    s_off = p_off + dim * dim * 4
    d_off = s_off + dim * dim * 4
    sp_off = d_off + dim * dim * 4
    prox_off = sp_off + sp_dim * sp_dim * 4
    isqrt_off = prox_off + prox_len * 4
    pow10_off = isqrt_off + 1025 * 4
    abs_off = pow10_off + 11 * 4

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
        # _isqrt: store as dense array (index = n, value = isqrt or 0)
        for n in range(1025):
            f.write(struct.pack('<i', pt._isqrt.get(n, 0)))
        for v in pt._pow10:
            f.write(struct.pack('<i', v))
        for v in range(-mc, mc+1):
            f.write(struct.pack('<i', pt._abs.get(v, abs(v))))
```

### Task 1.3: Implement reader

```python
class BinaryTables:
    """Read-only view into .ptbl file via memoryview."""

    def __init__(self, path):
        with open(path, 'rb') as f:
            self._data = f.read()

        self._mv = memoryview(self._data)
        hdr = struct.unpack('<4s17I', self._mv[0:72].tobytes())

        self.magic = hdr[0]
        self.max_coord = hdr[2]
        self.offset = hdr[3]
        self.scale = hdr[4]
        self.dim = hdr[7]
        self.sp_dim = hdr[6]

        # Create typed views
        p_off = hdr[8]
        self._P = self._int32_view(p_off, self.dim * self.dim)

        s_off = hdr[9]
        self._S = self._int32_view(s_off, self.dim * self.dim)

        d_off = hdr[10]
        self._D = self._int32_view(d_off, self.dim * self.dim)

        sp_off = hdr[11]
        self._SP = self._int32_view(sp_off, self.sp_dim * self.sp_dim)

        prox_off = hdr[12]
        self._prox = self._int32_view(prox_off, 4 * self.max_coord + 1)

    def _int32_view(self, offset, count):
        return memoryview(self._data)[offset:offset + count * 4].cast('i')

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
```

### Task 1.4: Generate tables.ptbl

Run: `python -c "from arith_table import PT; from table_format import save_ptbl; save_ptbl(PT, 'tables.ptbl')"`

---

## PHASE 2: C Header

### File: `native/include/geofield.h`

```c
#ifndef GEOFIELD_H
#define GEOFIELD_H

#include <stdint.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Opaque handle */
typedef struct GeoField GeoField;
typedef struct GeoResult GeoResult;

/* Lifecycle */
GeoField* geofield_init(const char* table_path);
GeoField* geofield_init_embedded(void);  /* use embedded tables */
void      geofield_destroy(GeoField*);

/* Pure lookup operations */
int32_t geofield_P(GeoField*, int32_t x, int32_t y);
int32_t geofield_S(GeoField*, int32_t x, int32_t y);
int32_t geofield_D(GeoField*, int32_t x, int32_t y);
int32_t geofield_p_from_sd(GeoField*, int32_t s, int32_t d);
int32_t geofield_p_from_xy(GeoField*, int32_t x, int32_t y);
int32_t geofield_proximity(GeoField*, int32_t dist);
int32_t geofield_int_weight(GeoField*, int32_t s1, int32_t d1,
                            int32_t s2, int32_t d2);
int32_t geofield_product(GeoField*, int32_t a, int32_t b);
int32_t geofield_isqrt(GeoField*, int32_t n);

/* Matrix multiply: C = A × B */
/* A is m×k, B is k×n, C is m×n (row-major flat arrays) */
GeoResult* geofield_matmul(GeoField*,
                           const int32_t* A, int m, int k,
                           const int32_t* B, int n);

/* Geometric attention */
/* tokens: flat array of [id, S, D, P] tuples, length = n_tokens * 4 */
/* output: flat array of [id, context, n_neighbors, output_x, output_y] */
GeoResult* geofield_attention(GeoField*,
                              const int32_t* tokens, int n_tokens,
                              int window);

/* Result access */
const int32_t* geofield_result_data(const GeoResult*);
int            geofield_result_rows(const GeoResult*);
int            geofield_result_cols(const GeoResult*);
int            geofield_result_len(const GeoResult*);  /* flat length */
void           geofield_result_free(GeoResult*);

/* Error handling */
const char* geofield_last_error(const GeoField*);
int         geofield_max_coord(const GeoField*);

#ifdef __cplusplus
}
#endif

#endif /* GEOFIELD_H */
```

---

## PHASE 3: Rust Implementation

### File: `native/Cargo.toml`

```toml
[package]
name = "geofield"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib", "staticlib"]

[dependencies]
memmap2 = "0.9"
libc = "0.2"
```

### File: `native/src/tables.rs`

```rust
use std::fs::File;
use std::path::Path;

pub struct Tables {
    _data: Vec<i32>,  // owned data (could be memmap in future)
    max_coord: u32,
    offset: u32,
    dim: usize,
    sp_dim: usize,
}

impl Tables {
    pub fn load(path: &Path) -> Result<Self, String> {
        let data = std::fs::read(path).map_err(|e| e.to_string())?;
        if &data[0..4] != b"PTBL" {
            return Err("Invalid magic".into());
        }

        let header = &data[0..72];
        let max_coord = u32::from_le_bytes([header[8], header[9], header[10], header[11]]);
        let offset = u32::from_le_bytes([header[12], header[13], header[14], header[15]]);
        let dim = (max_coord + 1) as usize;
        let sp_dim = (2 * max_coord + 1) as usize;

        // Convert bytes to i32 slice
        let int_data: Vec<i32> = data[256..]
            .chunks_exact(4)
            .map(|c| i32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect();

        Ok(Tables {
            _data: int_data,
            max_coord,
            offset,
            dim,
            sp_dim,
        })
    }

    #[inline]
    pub fn p(&self, x: i32, y: i32) -> i32 {
        self._data[(x as usize) * self.dim + (y as usize)]
    }

    #[inline]
    pub fn p_from_sd(&self, s: i32, d: i32) -> i32 {
        let idx = (s as usize) * self.sp_dim + ((d + self.offset as i32) as usize);
        self._data[idx]
    }

    #[inline]
    pub fn proximity(&self, dist: i32) -> i32 {
        let prox_start = self.dim * self.dim * 3 + self.sp_dim * self.sp_dim;
        if dist >= 0 && (dist as usize) < 4 * self.max_coord as usize + 1 {
            self._data[prox_start + dist as usize]
        } else {
            0
        }
    }

    // ... more lookup methods
}
```

### File: `native/src/lib.rs`

```rust
mod tables;
mod lookup;
mod matmul;
mod attention;

use tables::Tables;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::path::Path;

pub struct GeoField {
    tables: Tables,
    last_error: CString,
}

pub struct GeoResult {
    data: Vec<i32>,
    rows: i32,
    cols: i32,
}

#[no_mangle]
pub extern "C" fn geofield_init(path: *const c_char) -> *mut GeoField {
    if path.is_null() {
        return std::ptr::null_mut();
    }
    let path_str = unsafe { CStr::from_ptr(path).to_string_lossy() };
    match Tables::load(Path::new(path_str.as_ref())) {
        Ok(tables) => Box::into_raw(Box::new(GeoField {
            tables,
            last_error: CString::new("").unwrap(),
        })),
        Err(e) => {
            // Return null, error stored in thread-local
            std::ptr::null_mut()
        }
    }
}

#[no_mangle]
pub extern "C" fn geofield_destroy(field: *mut GeoField) {
    if !field.is_null() {
        unsafe { drop(Box::from_raw(field)); }
    }
}

#[no_mangle]
pub extern "C" fn geofield_P(field: *mut GeoField, x: i32, y: i32) -> i32 {
    if field.is_null() { return 0; }
    unsafe { (*field).tables.p(x, y) }
}

#[no_mangle]
pub extern "C" fn geofield_p_from_sd(field: *mut GeoField, s: i32, d: i32) -> i32 {
    if field.is_null() { return 0; }
    unsafe { (*field).tables.p_from_sd(s, d) }
}

#[no_mangle]
pub extern "C" fn geofield_proximity(field: *mut GeoField, dist: i32) -> i32 {
    if field.is_null() { return 0; }
    unsafe { (*field).tables.proximity(dist) }
}

#[no_mangle]
pub extern "C" fn geofield_matmul(
    field: *mut GeoField,
    a: *const i32, m: i32, k: i32,
    b: *const i32, n: i32,
) -> *mut GeoResult {
    if field.is_null() || a.is_null() || b.is_null() {
        return std::ptr::null_mut();
    }
    let a_slice = unsafe { std::slice::from_raw_parts(a, (m * k) as usize) };
    let b_slice = unsafe { std::slice::from_raw_parts(b, (k * n) as usize) };
    let result = matmul::matmul(&(*field).tables, a_slice, b_slice, m as usize, k as usize, n as usize);
    Box::into_raw(Box::new(GeoResult {
        data: result,
        rows: m,
        cols: n,
    }))
}

#[no_mangle]
pub extern "C" fn geofield_result_data(result: *const GeoResult) -> *const i32 {
    if result.is_null() { return std::ptr::null(); }
    unsafe { (*result).data.as_ptr() }
}

#[no_mangle]
pub extern "C" fn geofield_result_rows(result: *const GeoResult) -> i32 {
    if result.is_null() { return 0; }
    unsafe { (*result).rows }
}

#[no_mangle]
pub extern "C" fn geofield_result_cols(result: *const GeoResult) -> i32 {
    if result.is_null() { return 0; }
    unsafe { (*result).cols }
}

#[no_mangle]
pub extern "C" fn geofield_result_free(result: *mut GeoResult) {
    if !result.is_null() {
        unsafe { drop(Box::from_raw(result)); }
    }
}
```

---

## PHASE 4: Python Bindings

### File: `src/geofield_native.py`

```python
"""Thin wrapper around native geofield library via ctypes."""
import ctypes
import os
import sys

_lib = None

def _load_lib():
    global _lib
    if _lib is not None:
        return _lib

    # Find library
    lib_dir = os.path.join(os.path.dirname(__file__), '..', 'native', 'target', 'release')
    for name in ['libgeofield.so', 'geofield.dll', 'libgeofield.dylib']:
        path = os.path.join(lib_dir, name)
        if os.path.exists(path):
            _lib = ctypes.CDLL(path)
            break

    if _lib is None:
        return None

    # Define signatures
    _lib.geofield_init.restype = ctypes.c_void_p
    _lib.geofield_init.argtypes = [ctypes.c_char_p]
    _lib.geofield_destroy.restype = None
    _lib.geofield_destroy.argtypes = [ctypes.c_void_p]
    _lib.geofield_P.restype = ctypes.c_int32
    _lib.geofield_P.argtypes = [ctypes.c_void_p, ctypes.c_int32, ctypes.c_int32]
    # ... more signatures

    return _lib

class NativeGeoField:
    def __init__(self, table_path=None):
        lib = _load_lib()
        if lib is None:
            raise RuntimeError("Native library not found")
        path_bytes = table_path.encode() if table_path else b""
        self._handle = lib.geofield_init(path_bytes)
        if not self._handle:
            raise RuntimeError("Failed to initialize GeoField")

    def __del__(self):
        if hasattr(self, '_handle') and self._handle:
            _lib.geofield_destroy(self._handle)

    def P(self, x, y):
        return _lib.geofield_P(self._handle, x, y)

    def p_from_sd(self, s, d):
        return _lib.geofield_p_from_sd(self._handle, s, d)

    def proximity(self, dist):
        return _lib.geofield_proximity(self._handle, dist)

    def matmul(self, A, B):
        m, k = len(A), len(A[0])
        n = len(B[0])
        a_flat = (ctypes.c_int32 * (m * k))(*[A[i][j] for i in range(m) for j in range(k)])
        b_flat = (ctypes.c_int32 * (k * n))(*[B[i][j] for i in range(k) for j in range(n)])
        result_ptr = _lib.geofield_matmul(self._handle, a_flat, m, k, b_flat, n)
        data = _lib.geofield_result_data(result_ptr)
        rows = _lib.geofield_result_rows(result_ptr)
        cols = _lib.geofield_result_cols(result_ptr)
        c_data = ctypes.cast(data, ctypes.POINTER(ctypes.c_int32))
        flat = [c_data[i] for i in range(rows * cols)]
        _lib.geofield_result_free(result_ptr)
        return [[flat[i * cols + j] for j in range(cols)] for i in range(rows)]
```

---

## PHASE 5: Build System

### `native/build.ps1` (Windows)

```powershell
Set-Location $PSScriptRoot
cargo build --release --lib
$dst = Join-Path (Split-Path $PSScriptRoot) "src"
Copy-Item "target\release\geofield.dll" $dst -Force
Write-Host "Built geofield.dll -> $dst"
```

### Table embedding (optional)

In `native/src/lib.rs`:
```rust
#[cfg(feature = "embedded-tables")]
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "tables/"]
struct TableAssets;

#[no_mangle]
pub extern "C" fn geofield_init_embedded() -> *mut GeoField {
    let file = TableAssets::get("tables.ptbl").unwrap();
    let tables = Tables::from_bytes(&file.data).unwrap();
    Box::into_raw(Box::new(GeoField { tables, ... }))
}
```

---

## PHASE 6: Verification

### `native/tests/verify.py`

```python
"""Cross-verify: Python PtTable == Native GeoField."""
import sys, os, random
sys.path.insert(0, 'src')

from arith_table import PT
from geofield_native import NativeGeoField

gf = NativeGeoField('tables.ptbl')
random.seed(42)

# Lookup verification
errors = 0
for _ in range(10000):
    x, y = random.randint(0, 1024), random.randint(0, 1024)
    if PT.P(x, y) != gf.P(x, y):
        errors += 1
    s, d = x + y, x - y
    if PT.p_from_sd(s, d) != gf.p_from_sd(s, d):
        errors += 1

assert errors == 0, f"{errors} lookup mismatches"
print("✅ 10000 lookup tests passed")

# Matmul verification
for n in [16, 32, 64]:
    A = [[random.randint(1, 50) for _ in range(n)] for _ in range(n)]
    B = [[random.randint(1, 50) for _ in range(n)] for _ in range(n)]
    py = PT.matmul(A, B)
    nat = gf.matmul(A, B)
    for i in range(n):
        for j in range(n):
            if py[i][j] != nat[i][j]:
                raise AssertionError(f"matmul mismatch [{i}][{j}]: {py[i][j]} != {nat[i][j]}")
    print(f"✅ {n}x{n} matmul verified")
```

---

## PHASE 7: Distribution

### `dist/geofield/__init__.py`

```python
"""GeoField — Geometric computation engine. No GPU required."""
__version__ = "0.1.0"

from .core import GeoField

__all__ = ["GeoField"]
```

### `dist/geofield/core.py`

```python
"""Minimal public API. Everything else is private."""
from ...src.geofield_native import NativeGeoField as GeoField
```

### Public API (all users see):

```python
from geofield import GeoField

gf = GeoField()           # auto-loads tables
p = gf.P(4, 3)            # 12
C = gf.matmul(A, B)       # matrix multiply
out = gf.attention(tokens) # geometric attention
```

That's it. 4 functions. No tables, no S/D, no formulas visible.

---

## EXECUTION ORDER

```
Phase 1 (table format)
    ↓
Phase 2 (C header)
    ↓
Phase 3 (Rust implementation)
    ↓
Phase 4 (Python bindings)
    ↓
Phase 6 (verification)
    ↓
Phase 7 (distribution)

Phase 5 (build system) — runs in parallel with 3-4
```

## STOP CRITERIA

After EACH phase:
1. Run relevant tests
2. If any fail → STOP, fix, retry
3. Only proceed if all pass

## FINAL BINARY SIZE

| Component | Size |
|-----------|------|
| geofield.dll (code) | ~2 MB |
| tables.ptbl (data) | ~56 MB |
| **Total** | **~58 MB** |

Or with embedded tables: **single ~58 MB binary**.
