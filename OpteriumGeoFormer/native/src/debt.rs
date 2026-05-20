//! debt.rs — Debt System for fractional numbers.
//!
//! Fractional numbers represented as (mantissa, debt) pairs.
//! No float operations in hot paths.
//!
//! Examples:
//!   0.23  = DebtNumber { mantissa: 23, debt: -2 }
//!   2300  = DebtNumber { mantissa: 23, debt: +2 }
//!   7.922 = DebtNumber { mantissa: 7922, debt: -3 }

use crate::tables::Tables;

/// A fractional number in (mantissa, debt) representation.
/// value = mantissa * 10^debt
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DebtNumber {
    pub mantissa: i64,
    pub debt: i8,
}

impl DebtNumber {
    /// Create a new DebtNumber.
    pub fn new(mantissa: i64, debt: i8) -> Self {
        Self { mantissa, debt }
    }

    /// Create from integer (debt = 0).
    pub fn from_int(n: i64) -> Self {
        Self { mantissa: n, debt: 0 }
    }

    /// Create from float (for Python bindings).
    pub fn from_float(f: f64) -> Self {
        let s = format!("{:.10}", f);
        let s = s.trim_end_matches('0');
        if let Some(dot_pos) = s.find('.') {
            let int_part = &s[..dot_pos];
            let dec_part = &s[dot_pos + 1..];
            let mantissa: i64 = format!("{}{}", int_part, dec_part).parse().unwrap_or(0);
            let debt = -(dec_part.len() as i8);
            Self { mantissa, debt }
        } else {
            let mantissa: i64 = s.parse().unwrap_or(0);
            Self { mantissa, debt: 0 }
        }
    }

    /// Convert back to f64 (for display only — not in hot paths).
    pub fn to_float(&self) -> f64 {
        self.mantissa as f64 * 10f64.powi(self.debt as i32)
    }

    /// Multiply two DebtNumbers via lookup.
    pub fn mul(&self, other: &Self, tables: &Tables) -> Self {
        let mantissa = tables.product(self.mantissa as i32, other.mantissa as i32) as i64;
        let debt = self.debt + other.debt;
        Self { mantissa, debt }
    }

    /// Add two DebtNumbers (align debts first).
    pub fn add(&self, other: &Self, _tables: &Tables) -> Self {
        if self.debt == other.debt {
            let mantissa = self.mantissa + other.mantissa;
            Self { mantissa, debt: self.debt }
        } else {
            // Align to the smaller debt (more negative)
            let min_debt = self.debt.min(other.debt);
            let m1 = self.mantissa * 10i64.pow((self.debt - min_debt) as u32);
            let m2 = other.mantissa * 10i64.pow((other.debt - min_debt) as u32);
            Self { mantissa: m1 + m2, debt: min_debt }
        }
    }

    /// Divide two DebtNumbers.
    /// Returns (quotient, remainder) or None if division not exact.
    pub fn div(&self, other: &Self, tables: &Tables) -> Option<Self> {
        if other.mantissa == 0 {
            return None;
        }
        
        let result_mantissa = self.mantissa / other.mantissa;
        let remainder = self.mantissa % other.mantissa;
        
        if remainder != 0 {
            return None; // Not exact division
        }
        
        let debt = self.debt - other.debt;
        Some(Self { mantissa: result_mantissa, debt })
    }
}

/// by_P index for division lookup.
/// by_P[12] = [(1,12), (2,6), (3,4)]
pub struct ByPIndex {
    /// P → list of (x, y) pairs
    entries: Vec<Vec<(u32, u32)>>,
}

impl ByPIndex {
    /// Build by_P index from max_coord.
    pub fn build(max_coord: u32) -> Self {
        let max_p = (max_coord * max_coord) as usize;
        let mut entries = vec![Vec::new(); max_p + 1];
        
        for x in 0..=max_coord {
            for y in x..=max_coord {
                let p = (x * y) as usize;
                if p <= max_p {
                    entries[p].push((x, y));
                }
            }
        }
        
        Self { entries }
    }

    /// Find quotient: given P and one factor, return the other.
    /// by_P[12].find_with(3) → Some(4)
    pub fn find_divisor(&self, p: u32, factor: u32) -> Option<u32> {
        if p as usize >= self.entries.len() {
            return None;
        }
        
        for (x, y) in &self.entries[p as usize] {
            if *x == factor {
                return Some(*y);
            }
            if *y == factor {
                return Some(*x);
            }
        }
        None
    }

    /// Get all factor pairs for P.
    pub fn get_pairs(&self, p: u32) -> &[(u32, u32)] {
        if p as usize >= self.entries.len() {
            &[]
        } else {
            &self.entries[p as usize]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_debt_from_float() {
        let d = DebtNumber::from_float(0.23);
        assert_eq!(d.mantissa, 23);
        assert_eq!(d.debt, -2);

        let d = DebtNumber::from_float(3.4);
        assert_eq!(d.mantissa, 34);
        assert_eq!(d.debt, -1);

        let d = DebtNumber::from_float(2300.0);
        assert_eq!(d.mantissa, 2300);
        assert_eq!(d.debt, 0);
    }

    #[test]
    fn test_debt_to_float() {
        let d = DebtNumber::new(7922, -3);
        assert!((d.to_float() - 7.922).abs() < 1e-9);

        let d = DebtNumber::new(23, -2);
        assert!((d.to_float() - 0.23).abs() < 1e-9);
    }

    #[test]
    fn test_debt_add_same_debt() {
        // (1, -1) + (2, -1) = (3, -1) = 0.3
        let a = DebtNumber::new(1, -1);
        let b = DebtNumber::new(2, -1);
        // Create a minimal mock table for testing
        let result = a.add(&b, &create_test_tables());
        assert_eq!(result.mantissa, 3);
        assert_eq!(result.debt, -1);
    }

    #[test]
    fn test_byp_index() {
        let index = ByPIndex::build(12);
        
        assert_eq!(index.find_divisor(12, 3), Some(4));
        assert_eq!(index.find_divisor(12, 4), Some(3));
        assert_eq!(index.find_divisor(12, 2), Some(6));
        assert_eq!(index.find_divisor(12, 6), Some(2));
        assert_eq!(index.find_divisor(12, 1), Some(12));
        assert_eq!(index.find_divisor(12, 12), Some(1));
        assert_eq!(index.find_divisor(12, 5), None);
    }

    fn create_test_tables() -> Tables {
        // This is a minimal mock - for real tests, use actual tables
        // For now, we'll just use a placeholder since add doesn't actually use tables
        // when debts are equal
        let path = Path::new("../src/tables.ptbl");
        Tables::load(path).expect("Failed to load test tables")
    }
}
