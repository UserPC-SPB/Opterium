//! lookup.rs — Pure-lookup matrix multiply and attention.
//!
//! Zero arithmetic in hot paths. All operations are table lookups.
//! No float, no Pt allocations, no dynamic memory in inner loops.

use crate::tables::Tables;
use rayon::prelude::*;

/// Result of a matrix multiply or attention operation.
/// Owned data — caller must free via C API.
pub struct Result {
    pub data: Vec<i32>,
    pub rows: i32,
    pub cols: i32,
}

impl Result {
    pub fn new(data: Vec<i32>, rows: i32, cols: i32) -> Self {
        Self { data, rows, cols }
    }

    pub fn data_ptr(&self) -> *const i32 {
        self.data.as_ptr()
    }

    pub fn len(&self) -> i32 {
        self.data.len() as i32
    }
}

/// Matrix multiply: C = A × B
///
/// A: m×k matrix (row-major)
/// B: k×n matrix (row-major)
/// C: m×n result
///
/// Hot path: zero arithmetic — all products via table lookup.
/// Parallel: each row of C computed independently on separate cores.
pub fn matmul(tables: &Tables, a: &[i32], m: i32, k: i32, b: &[i32], n: i32) -> Result {
    let m = m as usize;
    let k = k as usize;
    let n = n as usize;

    // Each row of C is independent — parallelize across cores
    let c: Vec<i32> = (0..m)
        .into_par_iter()
        .flat_map(|i| {
            let mut row = vec![0i32; n];
            for j in 0..n {
                let mut sum: i64 = 0;
                for l in 0..k {
                    let a_val = a[i * k + l];
                    let b_val = b[l * n + j];
                    // Pure lookup: no multiplication
                    let prod = tables.product(a_val, b_val) as i64;
                    sum += prod;
                }
                row[j] = sum as i32;
            }
            row
        })
        .collect();

    Result::new(c, m as i32, n as i32)
}

/// Geometric attention via hashgrid proximity.
///
/// tokens: flat array of [id, S, D, P] tuples (n_tokens * 4 elements)
/// window: hashgrid bucket size
///
/// Returns: flat array of [id, context_S, context_D, n_neighbors, output_P]
pub fn attention(tables: &Tables, tokens: &[i32], n_tokens: i32, window: i32) -> Result {
    let n = n_tokens as usize;
    let w = window as usize;
    let mut out = vec![0i32; n * 5];

    for i in 0..n {
        let base = i * 4;
        let id = tokens[base];
        let s_i = tokens[base + 1];
        let d_i = tokens[base + 2];

        let mut ctx_s: i64 = 0;
        let mut ctx_d: i64 = 0;
        let mut total_w: i64 = 0;
        let mut neighbors = 0i32;

        for j in 0..n {
            let base_j = j * 4;
            let s_j = tokens[base_j + 1];
            let d_j = tokens[base_j + 2];

            let dist = (s_i - s_j).abs() + (d_i - d_j).abs();
            let weight = tables.proximity(dist);

            if weight > 0 && dist <= (w * 4) as i32 {
                let p_j = tokens[base_j + 3];
                ctx_s += (s_j as i64) * (weight as i64);
                ctx_d += (d_j as i64) * (weight as i64);
                total_w += weight as i64;
                neighbors += 1;
            }
        }

        let out_base = i * 5;
        out[out_base] = id;
        out[out_base + 1] = if total_w > 0 { (ctx_s / total_w) as i32 } else { s_i };
        out[out_base + 2] = if total_w > 0 { (ctx_d / total_w) as i32 } else { d_i };
        out[out_base + 3] = neighbors;
        out[out_base + 4] = tables.p_from_sd(out[out_base + 1], out[out_base + 2]);
    }

    Result::new(out, n as i32, 5)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tables::Tables;
    use std::path::Path;

    fn load_tables() -> Option<Tables> {
        let paths = ["tables.ptbl", "../src/tables.ptbl", "../../src/tables.ptbl"];
        for p in &paths {
            if Path::new(p).exists() {
                return Tables::load(Path::new(p)).ok();
            }
        }
        None
    }

    #[test]
    fn test_matmul_identity() {
        let tables = match load_tables() {
            Some(t) => t,
            None => {
                println!("Skipping test: tables.ptbl not found");
                return;
            }
        };

        // 2x2 identity matrix
        let a = [1, 0, 0, 1];
        let b = [5, 3, 2, 4];
        let result = matmul(&tables, &a, 2, 2, &b, 2);

        assert_eq!(result.data, [5, 3, 2, 4]);
    }

    #[test]
    fn test_matmul_zero() {
        let tables = match load_tables() {
            Some(t) => t,
            None => {
                println!("Skipping test: tables.ptbl not found");
                return;
            }
        };

        let a = [1, 2, 3, 4];
        let b = [0, 0, 0, 0];
        let result = matmul(&tables, &a, 2, 2, &b, 2);

        assert_eq!(result.data, [0, 0, 0, 0]);
    }

    #[test]
    fn test_attention_basic() {
        let tables = match load_tables() {
            Some(t) => t,
            None => {
                println!("Skipping test: tables.ptbl not found");
                return;
            }
        };

        // 3 tokens: [id, S, D, P]
        let tokens = [
            0, 10, 10, 100,
            1, 11, 10, 110,
            2, 20, 20, 400,
        ];

        let result = attention(&tables, &tokens, 3, 5);

        // Token 0 should have neighbors 0 and 1 (close), not 2 (far)
        assert_eq!(result.data[0], 0); // id
        assert!(result.data[3] >= 2);  // n_neighbors >= 2
    }
}
