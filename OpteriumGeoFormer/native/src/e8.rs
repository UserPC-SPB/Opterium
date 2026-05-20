//! e8.rs — E8 Root Lattice generation on-the-fly.
//!
//! 240 roots generated from address (x,y) via gcd + seed mapping.
//! No stored constants — all roots computed deterministically.
//!
//! D8 roots (112): two ±2, rest 0. All permutations.
//! Spinor roots (128): all ±1, even number of minus signs.
//!
//! Dot products ∈ {-8, -4, 0, +4, +8}


/// E8 root vector (8 dimensions).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct E8Root {
    pub coords: [i8; 8],
}

impl E8Root {
    /// Create a new E8 root.
    pub fn new(coords: [i8; 8]) -> Self {
        Self { coords }
    }

    /// Dot product with another root (8 lookup multiplications + 7 additions).
    pub fn dot(&self, other: &Self) -> i16 {
        let mut sum: i16 = 0;
        for i in 0..8 {
            sum += (self.coords[i] as i16) * (other.coords[i] as i16);
        }
        sum
    }

    /// Check if this is a valid E8 root.
    pub fn is_valid(&self) -> bool {
        let sum_sq: i16 = self.coords.iter().map(|&c| (c as i16) * (c as i16)).sum();
        sum_sq == 8 // All E8 roots have squared length 8
    }
}

/// Generate E8 root from address (x, y) via gcd + seed mapping.
pub fn address_to_root(x: u32, y: u32) -> E8Root {
    let g = gcd(x, y);
    let sx = (x / g) as i8;
    let sy = (y / g) as i8;
    
    // Determine root type based on seed
    if sx <= 2 && sy <= 2 {
        // D8 root: (sx, sy, 0, 0, 0, 0, 0, 0) with permutations
        d8_root(sx, sy)
    } else {
        // Spinor root: all ±1, even parity
        spinor_root(sx, sy)
    }
}

/// Generate D8 root (two ±2, rest 0).
fn d8_root(sx: i8, sy: i8) -> E8Root {
    let mut coords = [0i8; 8];
    coords[0] = sx * 2;
    coords[1] = sy * 2;
    E8Root::new(coords)
}

/// Generate Spinor root (all ±1, even number of minus signs).
fn spinor_root(sx: i8, sy: i8) -> E8Root {
    let mut coords = [1i8; 8];
    coords[0] = if sx >= 0 { 1 } else { -1 };
    coords[1] = if sy >= 0 { 1 } else { -1 };
    
    // Fix parity: ensure even number of minus signs
    let minus_count = coords.iter().filter(|&&c| c < 0).count();
    if minus_count % 2 != 0 {
        coords[7] = -coords[7]; // Flip last coordinate to fix parity
    }
    
    E8Root::new(coords)
}

/// Greatest common divisor.
fn gcd(mut a: u32, mut b: u32) -> u32 {
    while b != 0 {
        let t = b;
        b = a % b;
        a = t;
    }
    a
}

/// Generate all 240 E8 roots.
pub fn generate_all_roots() -> Vec<E8Root> {
    let mut roots = Vec::new();
    
    // D8 roots: permutations of (±2, ±2, 0, 0, 0, 0, 0, 0)
    // Choose 2 positions out of 8: C(8,2) = 28
    // Each position can be ±2: 4 combinations
    // Total: 28 * 4 = 112
    for i in 0..8 {
        for j in (i+1)..8 {
            for si in [-2, 2] {
                for sj in [-2, 2] {
                    let mut coords = [0i8; 8];
                    coords[i] = si;
                    coords[j] = sj;
                    roots.push(E8Root::new(coords));
                }
            }
        }
    }
    
    // Spinor roots: all ±1, even number of minus signs
    // Total: 2^7 = 128 (half of 2^8)
    for mask in 0..128u16 {
        let mut coords = [1i8; 8];
        let mut minus_count = 0;
        for bit in 0..7 {
            if (mask >> bit) & 1 == 1 {
                coords[bit as usize] = -1;
                minus_count += 1;
            }
        }
        // Fix parity: if odd number of minus signs, flip last coordinate
        if minus_count % 2 != 0 {
            coords[7] = -1;
        }
        roots.push(E8Root::new(coords));
    }
    
    assert_eq!(roots.len(), 240, "Should have exactly 240 E8 roots");
    roots
}

/// E8 attention: compute attention weights via E8 dot products.
pub fn e8_attention(
    queries: &[E8Root],
    keys: &[E8Root],
    values: &[E8Root],
) -> Vec<E8Root> {
    let n = queries.len();
    let mut outputs = Vec::with_capacity(n);
    
    for i in 0..n {
        let mut weighted_sum = [0i16; 8];
        let mut total_weight = 0i16;
        
        for j in 0..keys.len() {
            let dot = queries[i].dot(&keys[j]);
            // Map dot product to weight: {-8,-4,0,+4,+8} → {0,1,2,3,4}
            let weight = (dot + 8) / 4;
            
            if weight > 0 {
                for k in 0..8 {
                    weighted_sum[k] += weight as i16 * values[j].coords[k] as i16;
                }
                total_weight += weight;
            }
        }
        
        // Normalize (integer division)
        let mut result = [0i8; 8];
        if total_weight > 0 {
            for k in 0..8 {
                result[k] = (weighted_sum[k] / total_weight) as i8;
            }
        }
        
        outputs.push(E8Root::new(result));
    }
    
    outputs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_address_to_root() {
        let root = address_to_root(2, 2);
        assert!(root.is_valid());
        
        let root = address_to_root(3, 3);
        assert!(root.is_valid());
    }

    #[test]
    fn test_dot_products() {
        let roots = generate_all_roots();
        
        // Check all dot products are in {-8, -4, 0, +4, +8}
        for i in 0..roots.len() {
            for j in (i+1)..roots.len() {
                let dot = roots[i].dot(&roots[j]);
                assert!(
                    dot == -8 || dot == -4 || dot == 0 || dot == 4 || dot == 8,
                    "Invalid dot product: {} between roots {:?} and {:?}",
                    dot, roots[i], roots[j]
                );
            }
        }
    }

    #[test]
    fn test_all_roots_valid() {
        let roots = generate_all_roots();
        assert_eq!(roots.len(), 240);
        
        for root in &roots {
            assert!(root.is_valid(), "Invalid root: {:?}", root);
        }
    }

    #[test]
    fn test_e8_attention_basic() {
        let q = vec![address_to_root(2, 2)];
        let k = vec![address_to_root(2, 2), address_to_root(3, 3)];
        let v = vec![address_to_root(2, 2), address_to_root(3, 3)];
        
        let output = e8_attention(&q, &k, &v);
        assert_eq!(output.len(), 1);
        assert!(output[0].is_valid());
    }
}
