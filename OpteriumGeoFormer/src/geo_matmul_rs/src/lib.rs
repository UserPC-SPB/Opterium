use pyo3::prelude::*;
use pyo3::exceptions::PyValueError;
use rayon::prelude::*;
use std::collections::HashMap;

// ─────────────────────────────────────────────────────
// Core formula: P = (S² − D²) / 4
// ─────────────────────────────────────────────────────
#[inline(always)]
pub fn formula(s: i64, d: i64) -> i64 {
    (s * s - d * d) / 4
}

// ─────────────────────────────────────────────────────
// Inner sd_matmul — pure Rust, no PyO3 (testable)
// ─────────────────────────────────────────────────────
fn sd_matmul_inner(a_sd: &[Vec<(i64, i64)>], b_sd: &[Vec<(i64, i64)>]) -> Result<Vec<Vec<i64>>, String> {
    let m = a_sd.len();
    if m == 0 { return Ok(vec![]); }
    let k = a_sd[0].len();
    let n = b_sd[0].len();

    if k != b_sd.len() {
        return Err(format!("Shape mismatch: ({}x{}) x ({}x{})", m, k, b_sd.len(), n));
    }

    let mut s_a = vec![0i64; m * k];
    let mut d_a = vec![0i64; m * k];
    for i in 0..m {
        for p in 0..k {
            s_a[i * k + p] = a_sd[i][p].0;
            d_a[i * k + p] = a_sd[i][p].1;
        }
    }

    let mut s_b = vec![0i64; k * n];
    let mut d_b = vec![0i64; k * n];
    for p in 0..k {
        for j in 0..n {
            s_b[p * n + j] = b_sd[p][j].0;
            d_b[p * n + j] = b_sd[p][j].1;
        }
    }

    let mut c = vec![0i64; m * n];

    for i in 0..m {
        for p in 0..k {
            let p1 = formula(s_a[i * k + p], d_a[i * k + p]);
            for j in 0..n {
                let p2 = formula(s_b[p * n + j], d_b[p * n + j]);
                c[i * n + j] += p1 * p2;
            }
        }
    }

    Ok(c.chunks_exact(n).map(|chunk| chunk.to_vec()).collect())
}

fn sd_matmul_parallel_inner(a_sd: &[Vec<(i64, i64)>], b_sd: &[Vec<(i64, i64)>]) -> Result<Vec<Vec<i64>>, String> {
    let m = a_sd.len();
    if m == 0 { return Ok(vec![]); }
    let k = a_sd[0].len();
    let n = b_sd[0].len();

    if k != b_sd.len() {
        return Err(format!("Shape mismatch: ({}x{}) x ({}x{})", m, k, b_sd.len(), n));
    }

    let s_a: Vec<i64> = a_sd.iter().flat_map(|row| row.iter().map(|&(s, _)| s)).collect();
    let d_a: Vec<i64> = a_sd.iter().flat_map(|row| row.iter().map(|&(_, d)| d)).collect();
    let s_b: Vec<i64> = b_sd.iter().flat_map(|row| row.iter().map(|&(s, _)| s)).collect();
    let d_b: Vec<i64> = b_sd.iter().flat_map(|row| row.iter().map(|&(_, d)| d)).collect();

    let mut c: Vec<i64> = vec![0i64; m * n];

    c.par_chunks_mut(n).enumerate().for_each(|(i, row)| {
        let base_i = i * k;
        for p in 0..k {
            let p1 = formula(s_a[base_i + p], d_a[base_i + p]);
            let base_p = p * n;
            for j in 0..n {
                let p2 = formula(s_b[base_p + j], d_b[base_p + j]);
                row[j] += p1 * p2;
            }
        }
    });

    Ok(c.chunks_exact(n).map(|chunk| chunk.to_vec()).collect())
}

// ─────────────────────────────────────────────────────
// PyO3 wrappers — pure Rust logic above
// ─────────────────────────────────────────────────────
#[pyfunction]
fn sd_matmul(a_sd: Vec<Vec<(i64, i64)>>, b_sd: Vec<Vec<(i64, i64)>>) -> PyResult<Vec<Vec<i64>>> {
    sd_matmul_inner(&a_sd, &b_sd).map_err(|e| PyValueError::new_err(e))
}

#[pyfunction]
fn sd_matmul_parallel(a_sd: Vec<Vec<(i64, i64)>>, b_sd: Vec<Vec<(i64, i64)>>) -> PyResult<Vec<Vec<i64>>> {
    sd_matmul_parallel_inner(&a_sd, &b_sd).map_err(|e| PyValueError::new_err(e))
}

// ─────────────────────────────────────────────────────
// HashGrid — O(1) neighbor lookup in (S, D) space
// ─────────────────────────────────────────────────────
#[pyclass]
struct HashGrid {
    window: i64,
    buckets: HashMap<(i64, i64), Vec<(usize, i64, i64, i64)>>, // (id, S, D, P)
}

#[pymethods]
impl HashGrid {
    #[new]
    fn new(window: i64) -> Self {
        HashGrid {
            window,
            buckets: HashMap::new(),
        }
    }

    fn insert(&mut self, token_id: usize, s: i64, d: i64, p: i64) {
        let key = (s / self.window, d / self.window);
        self.buckets.entry(key).or_default().push((token_id, s, d, p));
    }

    fn lookup(&self, s: i64, d: i64) -> Vec<(usize, i64, i64, i64)> {
        let bk = (s / self.window, d / self.window);
        let mut result = Vec::new();
        for dx in -1..=1 {
            for dy in -1..=1 {
                let key = (bk.0 + dx, bk.1 + dy);
                if let Some(entries) = self.buckets.get(&key) {
                    result.extend(entries.iter().copied());
                }
            }
        }
        result
    }

    fn stats(&self) -> (usize, usize, f64) {
        let total: usize = self.buckets.values().map(|v| v.len()).sum();
        let nb = self.buckets.len();
        let avg = if nb > 0 { total as f64 / nb as f64 } else { 0.0 };
        (nb, total, avg)
    }
}

// ─────────────────────────────────────────────────────
// Geometric attention — one layer
// ─────────────────────────────────────────────────────
#[pyfunction]
fn geometric_attention(
    tokens: Vec<(usize, i64, i64, i64)>, // (id, S, D, P)
    window: i64,
    include_self: bool,
) -> Vec<(usize, i64, usize, i64, i64)> {
    // returns (id, context, n_neighbors, output_x, output_y)
    if tokens.is_empty() {
        return vec![];
    }

    let mut grid = HashGrid::new(window);
    for &(id, s, d, p) in &tokens {
        grid.insert(id, s, d, p);
    }

    tokens
        .par_iter()
        .map(|&(id, s_q, d_q, p_q)| {
            let neighbors = grid.lookup(s_q, d_q);
            let (w_total, p_weighted, count) = neighbors
                .iter()
                .filter(|&&(nid, _, _, _)| include_self || nid != id)
                .fold((0.0, 0.0, 0usize), |(wt, pw, cnt), &(_, ns, nd, np)| {
                    let dist = (ns - s_q).unsigned_abs() + (nd - d_q).unsigned_abs();
                    let w = 1.0 / (1.0 + dist as f64);
                    (wt + w, pw + w * np as f64, cnt + 1)
                });

            let context = if w_total > 0.0 {
                (p_weighted / w_total) as i64
            } else {
                p_q
            };

            // Pt3 triple product: x·y·context
            let mixed = p_q * context;
            let out_x = if mixed > 0 {
                (mixed as f64).sqrt() as i64
            } else {
                0
            };
            let out_y = if out_x > 0 { out_x } else { 1 };

            (id, context, count, out_x, out_y)
        })
        .collect()
}

// ─────────────────────────────────────────────────────
// PyO3 module definition
// ─────────────────────────────────────────────────────
#[pymodule]
fn geo_matmul_rs(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(sd_matmul, m)?)?;
    m.add_function(wrap_pyfunction!(sd_matmul_parallel, m)?)?;
    m.add_function(wrap_pyfunction!(geometric_attention, m)?)?;
    m.add_class::<HashGrid>()?;
    Ok(())
}


// ─────────────────────────────────────────────────────
// Unit tests  —  run with: cargo test
// ─────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;

    // ── formula tests (pure Rust) ─────────────────────

    #[test]
    fn test_formula_known_values() {
        assert_eq!(formula(5, 3), 4);
        assert_eq!(formula(2, 0), 1);
        assert_eq!(formula(1, -1), 0);
        assert_eq!(formula(4, 2), 3);
    }

    #[test]
    fn test_formula_symmetry() {
        assert_eq!(formula(7, 1), formula(7, -1));
        assert_eq!(formula(10, 3), formula(10, -3));
        assert_eq!(formula(5, 0), formula(5, 0));
    }

    #[test]
    fn test_formula_identity() {
        for x in [1, 2, 5, 10, 100, 1000] {
            assert_eq!(formula(x + 1, x - 1), x);
        }
    }

    #[test]
    fn test_formula_product() {
        assert_eq!(formula(5, 3) * formula(7, 5), 4 * 6);
        assert_eq!(formula(5, 3) * formula(7, 5), 24);
    }

    // ── sd_matmul_inner tests (pure Rust) ─────────────

    #[test]
    fn test_sd_matmul_inner_small() {
        let a = vec![vec![(5i64, 3i64), (7i64, 5i64)]];
        let b = vec![vec![(6i64, 4i64)], vec![(3i64, 1i64)]];
        let c = sd_matmul_inner(&a, &b).unwrap();
        assert_eq!(c[0][0], 4 * 5 + 6 * 2);
    }

    #[test]
    fn test_sd_matmul_inner_identity() {
        let i = vec![
            vec![(2, 0), (1, -1), (1, -1)],
            vec![(1, -1), (2, 0), (1, -1)],
            vec![(1, -1), (1, -1), (2, 0)],
        ];
        let a = vec![
            vec![(5i64, 3i64), (4i64, 2i64), (7i64, 1i64)],
            vec![(3i64, 1i64), (6i64, 4i64), (9i64, 3i64)],
            vec![(4i64, 0i64), (5i64, 1i64), (8i64, 2i64)],
        ];
        let c = sd_matmul_inner(&a, &i).unwrap();
        for i in 0..3 {
            for j in 0..3 {
                assert_eq!(c[i][j], formula(a[i][j].0, a[i][j].1), "A·I [{}][{}]", i, j);
            }
        }
    }

    #[test]
    fn test_sd_matmul_inner_zero() {
        let zero = vec![vec![(1i64, -1i64); 3]; 3];
        let a = vec![
            vec![(5i64, 3i64), (4i64, 2i64), (7i64, 1i64)],
            vec![(3i64, 1i64), (6i64, 4i64), (9i64, 3i64)],
        ];
        let c = sd_matmul_inner(&a, &zero).unwrap();
        for row in &c {
            for val in row {
                assert_eq!(*val, 0);
            }
        }
    }

    #[test]
    fn test_sd_matmul_inner_shape_mismatch() {
        let a = vec![vec![(1i64, 1i64); 3]; 2];
        let b_ok = vec![vec![(1i64, 1i64); 4]; 3];
        let b_bad = vec![vec![(1i64, 1i64); 4]; 2];
        assert!(sd_matmul_inner(&a, &b_ok).is_ok());
        assert!(sd_matmul_inner(&a, &b_bad).is_err());
    }

    #[test]
    fn test_sd_matmul_seq_vs_par() {
        let a = vec![
            vec![(5i64, 3i64), (7i64, 1i64), (4i64, 2i64)],
            vec![(3i64, 1i64), (6i64, 4i64), (8i64, 0i64)],
        ];
        let b = vec![
            vec![(6i64, 4i64), (3i64, 1i64)],
            vec![(5i64, 3i64), (7i64, 5i64)],
            vec![(4i64, 2i64), (6i64, 4i64)],
        ];
        let c_seq = sd_matmul_inner(&a, &b).unwrap();
        let c_par = sd_matmul_parallel_inner(&a, &b).unwrap();
        assert_eq!(c_seq, c_par);
    }

    // ── HashGrid tests (pure Rust struct) ─────────────

    #[test]
    fn test_hashgrid_insert_lookup() {
        let mut grid = HashGrid::new(16);
        grid.insert(0, 5, 3, 4);
        grid.insert(1, 7, 1, 12);
        grid.insert(2, 6, 4, 5);
        let neighbors = grid.lookup(5, 3);
        assert!(!neighbors.is_empty());
        let ids: Vec<usize> = neighbors.iter().map(|&(id, _, _, _)| id).collect();
        assert!(ids.contains(&0));
    }

    #[test]
    fn test_hashgrid_empty_lookup() {
        let grid = HashGrid::new(16);
        let r = grid.lookup(100, 200);
        assert!(r.is_empty());
    }

    #[test]
    fn test_hashgrid_stats() {
        let mut grid = HashGrid::new(16);
        assert_eq!(grid.stats(), (0, 0, 0.0));
        grid.insert(0, 5, 3, 4);
        grid.insert(1, 20, 10, 12);
        let (nb, total, _) = grid.stats();
        assert!(nb >= 1);
        assert_eq!(total, 2);
    }
}
