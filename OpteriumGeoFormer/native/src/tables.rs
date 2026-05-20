//! tables.rs — Memory-mapped .ptbl table reader.
//!
//! Loads the binary table format created by table_format.py.
//! All tables are read-only views into the mmap'd file.
//! Zero allocations after init.

use std::path::Path;
use memmap2::Mmap;

/// Magic bytes at start of .ptbl file.
const MAGIC: &[u8; 4] = b"PTBL";
const HEADER_SIZE: usize = 256;

/// Read-only view into a .ptbl file.
/// All tables are accessed as &[i32] slices — zero allocation.
pub struct Tables {
    _mmap: Mmap,
    max_coord: u32,
    offset: u32,
    dim: usize,
    sp_dim: usize,

    // Table slices (views into mmap'd data)
    p: &'static [i32],
    s: &'static [i32],
    d: &'static [i32],
    sp: &'static [i32],
    prox: &'static [i32],
    isqrt: &'static [i32],
    pow10: &'static [i64],
    abs: &'static [i32],
}

impl Tables {
    /// Load tables from a .ptbl file.
    pub fn load(path: &Path) -> Result<Self, String> {
        let file = std::fs::File::open(path)
            .map_err(|e| format!("Cannot open {}: {}", path.display(), e))?;

        // Safety: mmap is read-only, file outlives mmap, we never modify data
        let mmap = unsafe { Mmap::map(&file) }
            .map_err(|e| format!("Cannot mmap {}: {}", path.display(), e))?;

        if mmap.len() < HEADER_SIZE {
            return Err("File too small for header".into());
        }

        if &mmap[0..4] != MAGIC {
            return Err(format!("Invalid magic: {:?}", &mmap[0..4]));
        }

        // Parse header (little-endian uint32)
        let read_u32 = |off: usize| -> u32 {
            let b = &mmap[off..off + 4];
            u32::from_le_bytes([b[0], b[1], b[2], b[3]])
        };

        let max_coord = read_u32(8);
        let offset = read_u32(12);
        let dim = (max_coord + 1) as usize;
        let sp_dim = (2 * max_coord + 1) as usize;

        let p_off = read_u32(32) as usize;
        let s_off = read_u32(36) as usize;
        let d_off = read_u32(40) as usize;
        let sp_off = read_u32(44) as usize;
        let prox_off = read_u32(48) as usize;
        let isqrt_off = read_u32(52) as usize;
        let pow10_off = read_u32(56) as usize;
        let abs_off = read_u32(60) as usize;

        // Create typed slices from mmap data
        // Safety: we're creating &'static slices from mmap'd data.
        // The mmap owns the data and outlives this struct.
        // We never modify the data.
        let data_ptr = mmap.as_ptr();

        let slice_i32 = |off: usize, len: usize| -> &'static [i32] {
            unsafe {
                std::slice::from_raw_parts(
                    data_ptr.add(off) as *const i32,
                    len,
                )
            }
        };

        let slice_i64 = |off: usize, len: usize| -> &'static [i64] {
            unsafe {
                std::slice::from_raw_parts(
                    data_ptr.add(off) as *const i64,
                    len,
                )
            }
        };

        let p = slice_i32(p_off, dim * dim);
        let s = slice_i32(s_off, dim * dim);
        let d = slice_i32(d_off, dim * dim);
        let sp = slice_i32(sp_off, sp_dim * sp_dim);
        let prox = slice_i32(prox_off, 4 * max_coord as usize + 1);
        let isqrt = slice_i32(isqrt_off, 1025);
        let pow10 = slice_i64(pow10_off, 11);
        let abs = slice_i32(abs_off, 2 * max_coord as usize + 1);

        Ok(Tables {
            _mmap: mmap,
            max_coord,
            offset,
            dim,
            sp_dim,
            p, s, d, sp, prox, isqrt, pow10, abs,
        })
    }

    /// Load tables from a byte slice (for embedded tables).
    #[cfg(feature = "embedded-tables")]
    pub fn from_bytes(data: &'static [u8]) -> Result<Self, String> {
        if data.len() < HEADER_SIZE {
            return Err("Data too small for header".into());
        }
        if &data[0..4] != MAGIC {
            return Err(format!("Invalid magic: {:?}", &data[0..4]));
        }

        let read_u32 = |off: usize| -> u32 {
            let b = &data[off..off + 4];
            u32::from_le_bytes([b[0], b[1], b[2], b[3]])
        };

        let max_coord = read_u32(8);
        let offset = read_u32(12);
        let dim = (max_coord + 1) as usize;
        let sp_dim = (2 * max_coord + 1) as usize;

        let p_off = read_u32(32) as usize;
        let s_off = read_u32(36) as usize;
        let d_off = read_u32(40) as usize;
        let sp_off = read_u32(44) as usize;
        let prox_off = read_u32(48) as usize;
        let isqrt_off = read_u32(52) as usize;
        let pow10_off = read_u32(56) as usize;
        let abs_off = read_u32(60) as usize;

        let slice_i32 = |off: usize, len: usize| -> &'static [i32] {
            unsafe {
                std::slice::from_raw_parts(
                    data.as_ptr().add(off) as *const i32,
                    len,
                )
            }
        };

        let slice_i64 = |off: usize, len: usize| -> &'static [i64] {
            unsafe {
                std::slice::from_raw_parts(
                    data.as_ptr().add(off) as *const i64,
                    len,
                )
            }
        };

        Ok(Tables {
            _mmap: unsafe { Mmap::map(&std::fs::File::open("dummy").unwrap_or_else(|_| panic!())) },
            max_coord,
            offset,
            dim,
            sp_dim,
            p: slice_i32(p_off, dim * dim),
            s: slice_i32(s_off, dim * dim),
            d: slice_i32(d_off, dim * dim),
            sp: slice_i32(sp_off, sp_dim * sp_dim),
            prox: slice_i32(prox_off, 4 * max_coord as usize + 1),
            isqrt: slice_i32(isqrt_off, 1025),
            pow10: slice_i64(pow10_off, 11),
            abs: slice_i32(abs_off, 2 * max_coord as usize + 1),
        })
    }

    // ── Lookup operations (all #[inline] for zero overhead) ──

    #[inline]
    pub fn p(&self, x: i32, y: i32) -> i32 {
        let x = x as usize;
        let y = y as usize;
        if x < self.dim && y < self.dim {
            self.p[x * self.dim + y]
        } else {
            0
        }
    }

    #[inline]
    pub fn s(&self, x: i32, y: i32) -> i32 {
        let x = x as usize;
        let y = y as usize;
        if x < self.dim && y < self.dim {
            self.s[x * self.dim + y]
        } else {
            0
        }
    }

    #[inline]
    pub fn d(&self, x: i32, y: i32) -> i32 {
        let x = x as usize;
        let y = y as usize;
        if x < self.dim && y < self.dim {
            let val = self.d[x * self.dim + y];
            if val < 0 { -val } else { val }
        } else {
            0
        }
    }

    #[inline]
    pub fn p_from_sd(&self, s: i32, d: i32) -> i32 {
        let s = s as usize;
        let d_idx = (d + self.offset as i32) as usize;
        if s < self.sp_dim && d_idx < self.sp_dim {
            self.sp[s * self.sp_dim + d_idx]
        } else {
            // Fallback: formula (should not happen in normal use)
            (s as i64 * s as i64 - d as i64 * d as i64) as i32 / 4
        }
    }

    #[inline]
    pub fn proximity(&self, dist: i32) -> i32 {
        if dist >= 0 && (dist as usize) < self.prox.len() {
            self.prox[dist as usize]
        } else {
            0
        }
    }

    #[inline]
    pub fn int_weight(&self, s1: i32, d1: i32, s2: i32, d2: i32) -> i32 {
        let dist = (s1 - s2).abs() + (d1 - d2).abs();
        self.proximity(dist)
    }

    #[inline]
    pub fn isqrt(&self, n: i32) -> i32 {
        if n >= 0 && (n as usize) < self.isqrt.len() {
            self.isqrt[n as usize]
        } else if n > 0 {
            n.isqrt() as i32
        } else {
            0
        }
    }

    #[inline]
    pub fn pow10(&self, n: i32) -> i64 {
        if n >= 0 && (n as usize) < self.pow10.len() {
            self.pow10[n as usize]
        } else if n < 0 {
            1
        } else {
            10_i64.pow(n as u32)
        }
    }

    #[inline]
    pub fn abs_val(&self, x: i32) -> i32 {
        let idx = (x + self.max_coord as i32) as usize;
        if idx < self.abs.len() {
            self.abs[idx]
        } else {
            x.abs()
        }
    }

    /// Product via _P lookup. No gcd-scaling in binary format.
    #[inline]
    pub fn product(&self, a: i32, b: i32) -> i32 {
        if a >= 0 && b >= 0 && (a as usize) < self.dim && (b as usize) < self.dim {
            self.p[a as usize * self.dim + b as usize]
        } else {
            // Fallback: direct multiplication
            a * b
        }
    }

    pub fn max_coord(&self) -> u32 {
        self.max_coord
    }

    pub fn table_size(&self) -> usize {
        self._mmap.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_tables() {
        // This test requires tables.ptbl to exist
        let path = Path::new("tables.ptbl");
        if !path.exists() {
            // Try parent directory
            let path = Path::new("../src/tables.ptbl");
            if !path.exists() {
                println!("Skipping test: tables.ptbl not found");
                return;
            }
            let tables = Tables::load(path).unwrap();
            assert_eq!(tables.p(4, 3), 12);
            assert_eq!(tables.p_from_sd(7, 1), 12);
            assert_eq!(tables.proximity(0), 10000);
            assert_eq!(tables.proximity(1), 5000);
            assert_eq!(tables.isqrt(144), 12);
        }
    }
}
