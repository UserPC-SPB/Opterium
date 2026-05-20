//! geofield — Opterium GeoFormer native core.
//!
//! Pure-lookup geometric math engine. Zero arithmetic in hot paths.
//! Exposes opaque C API via `geofield.h`.

mod tables;
mod lookup;
mod debt;
mod e8;

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::path::Path;
use std::sync::Mutex;

use tables::Tables;
use lookup::Result as GeoResult;
use debt::{DebtNumber, ByPIndex};
use e8::{E8Root, address_to_root, generate_all_roots, e8_attention as e8_attn};

/// Opaque handle to GeoField.
pub struct GeoField {
    tables: Tables,
    by_p: ByPIndex,
    last_error: Mutex<Option<String>>,
}

/// Opaque handle to result data.
pub struct GeoResultHandle {
    result: GeoResult,
}

// ── Lifecycle ──

#[no_mangle]
pub extern "C" fn geofield_init(table_path: *const c_char) -> *mut GeoField {
    if table_path.is_null() {
        return std::ptr::null_mut();
    }

    let path = unsafe { CStr::from_ptr(table_path) };
    let path = match path.to_str() {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };

    match Tables::load(Path::new(path)) {
        Ok(tables) => {
            let max_coord = tables.max_coord();
            let by_p = ByPIndex::build(max_coord);
            Box::into_raw(Box::new(GeoField {
                tables,
                by_p,
                last_error: Mutex::new(None),
            }))
        }
        Err(_e) => {
            // Return a dummy handle with error (caller can check last_error)
            // For now, return null — caller should check errno or similar
            std::ptr::null_mut()
        }
    }
}

#[no_mangle]
pub extern "C" fn geofield_init_embedded() -> *mut GeoField {
    #[cfg(feature = "embedded-tables")]
    {
        // Tables embedded at compile time via include_bytes!
        static TABLE_DATA: &[u8] = include_bytes!("../../src/tables.ptbl");
        match Tables::from_bytes(TABLE_DATA) {
            Ok(tables) => Box::into_raw(Box::new(GeoField {
                tables,
                last_error: Mutex::new(None),
            })),
            Err(_) => std::ptr::null_mut(),
        }
    }
    #[cfg(not(feature = "embedded-tables"))]
    {
        std::ptr::null_mut()
    }
}

#[no_mangle]
pub extern "C" fn geofield_destroy(gf: *mut GeoField) {
    if !gf.is_null() {
        unsafe {
            drop(Box::from_raw(gf));
        }
    }
}

// ── Pure Lookup Operations ──

#[no_mangle]
pub extern "C" fn geofield_P(gf: *mut GeoField, x: i32, y: i32) -> i32 {
    if gf.is_null() {
        return 0;
    }
    unsafe { (*gf).tables.p(x, y) }
}

#[no_mangle]
pub extern "C" fn geofield_S(gf: *mut GeoField, x: i32, y: i32) -> i32 {
    if gf.is_null() {
        return 0;
    }
    unsafe { (*gf).tables.s(x, y) }
}

#[no_mangle]
pub extern "C" fn geofield_D(gf: *mut GeoField, x: i32, y: i32) -> i32 {
    if gf.is_null() {
        return 0;
    }
    unsafe { (*gf).tables.d(x, y) }
}

#[no_mangle]
pub extern "C" fn geofield_p_from_sd(gf: *mut GeoField, s: i32, d: i32) -> i32 {
    if gf.is_null() {
        return 0;
    }
    unsafe { (*gf).tables.p_from_sd(s, d) }
}

#[no_mangle]
pub extern "C" fn geofield_p_from_xy(gf: *mut GeoField, x: i32, y: i32) -> i32 {
    if gf.is_null() {
        return 0;
    }
    unsafe { (*gf).tables.p(x, y) }
}

#[no_mangle]
pub extern "C" fn geofield_proximity(gf: *mut GeoField, dist: i32) -> i32 {
    if gf.is_null() {
        return 0;
    }
    unsafe { (*gf).tables.proximity(dist) }
}

#[no_mangle]
pub extern "C" fn geofield_int_weight(
    gf: *mut GeoField,
    s1: i32,
    d1: i32,
    s2: i32,
    d2: i32,
) -> i32 {
    if gf.is_null() {
        return 0;
    }
    unsafe { (*gf).tables.int_weight(s1, d1, s2, d2) }
}

#[no_mangle]
pub extern "C" fn geofield_product(gf: *mut GeoField, a: i32, b: i32) -> i32 {
    if gf.is_null() {
        return 0;
    }
    unsafe { (*gf).tables.product(a, b) }
}

#[no_mangle]
pub extern "C" fn geofield_isqrt(gf: *mut GeoField, n: i32) -> i32 {
    if gf.is_null() {
        return 0;
    }
    unsafe { (*gf).tables.isqrt(n) }
}

// ── Matrix Multiply ──

#[no_mangle]
pub extern "C" fn geofield_matmul(
    gf: *mut GeoField,
    a: *const i32,
    m: i32,
    k: i32,
    b: *const i32,
    n: i32,
) -> *mut GeoResultHandle {
    if gf.is_null() || a.is_null() || b.is_null() {
        return std::ptr::null_mut();
    }

    let gf = unsafe { &*gf };
    let a_slice = unsafe { std::slice::from_raw_parts(a, (m * k) as usize) };
    let b_slice = unsafe { std::slice::from_raw_parts(b, (k * n) as usize) };

    let result = lookup::matmul(&gf.tables, a_slice, m, k, b_slice, n);
    Box::into_raw(Box::new(GeoResultHandle { result }))
}

// ── Geometric Attention ──

#[no_mangle]
pub extern "C" fn geofield_attention(
    gf: *mut GeoField,
    tokens: *const i32,
    n_tokens: i32,
    window: i32,
) -> *mut GeoResultHandle {
    if gf.is_null() || tokens.is_null() {
        return std::ptr::null_mut();
    }

    let gf = unsafe { &*gf };
    let tokens_slice = unsafe { std::slice::from_raw_parts(tokens, (n_tokens * 4) as usize) };

    let result = lookup::attention(&gf.tables, tokens_slice, n_tokens, window);
    Box::into_raw(Box::new(GeoResultHandle { result }))
}

// ── Result Access ──

#[no_mangle]
pub extern "C" fn geofield_result_data(r: *const GeoResultHandle) -> *const i32 {
    if r.is_null() {
        return std::ptr::null();
    }
    unsafe { (*r).result.data_ptr() }
}

#[no_mangle]
pub extern "C" fn geofield_result_rows(r: *const GeoResultHandle) -> i32 {
    if r.is_null() {
        return 0;
    }
    unsafe { (*r).result.rows }
}

#[no_mangle]
pub extern "C" fn geofield_result_cols(r: *const GeoResultHandle) -> i32 {
    if r.is_null() {
        return 0;
    }
    unsafe { (*r).result.cols }
}

#[no_mangle]
pub extern "C" fn geofield_result_len(r: *const GeoResultHandle) -> i32 {
    if r.is_null() {
        return 0;
    }
    unsafe { (*r).result.len() }
}

#[no_mangle]
pub extern "C" fn geofield_result_free(r: *mut GeoResultHandle) {
    if !r.is_null() {
        unsafe {
            drop(Box::from_raw(r));
        }
    }
}

// ── Error Handling ──

#[no_mangle]
pub extern "C" fn geofield_last_error(gf: *const GeoField) -> *const c_char {
    if gf.is_null() {
        return std::ptr::null();
    }

    let gf = unsafe { &*gf };
    let err = gf.last_error.lock().unwrap();
    match err.as_ref() {
        Some(s) => match CString::new(s.as_str()) {
            Ok(c) => c.into_raw() as *const c_char,
            Err(_) => std::ptr::null(),
        },
        None => std::ptr::null(),
    }
}

#[no_mangle]
pub extern "C" fn geofield_max_coord(gf: *const GeoField) -> i32 {
    if gf.is_null() {
        return 0;
    }
    unsafe { (*gf).tables.max_coord() as i32 }
}

#[no_mangle]
pub extern "C" fn geofield_table_size(gf: *const GeoField) -> usize {
    if gf.is_null() {
        return 0;
    }
    unsafe { (*gf).tables.table_size() }
}

// ── Debt System ──

#[no_mangle]
pub extern "C" fn geofield_debt_from_float(f: f64) -> i64 {
    let d = DebtNumber::from_float(f);
    // Pack: mantissa in lower 48 bits, debt in upper 16 bits
    ((d.debt as i64) << 48) | (d.mantissa & 0xFFFFFFFFFFFF)
}

#[no_mangle]
pub extern "C" fn geofield_debt_mantissa(packed: i64) -> i64 {
    packed & 0xFFFFFFFFFFFF
}

#[no_mangle]
pub extern "C" fn geofield_debt_debt(packed: i64) -> i8 {
    (packed >> 48) as i8
}

#[no_mangle]
pub extern "C" fn geofield_debt_mul(a: i64, b: i64, gf: *mut GeoField) -> i64 {
    if gf.is_null() {
        return 0;
    }
    let da = DebtNumber::new(
        geofield_debt_mantissa(a),
        geofield_debt_debt(a),
    );
    let db = DebtNumber::new(
        geofield_debt_mantissa(b),
        geofield_debt_debt(b),
    );
    let result = da.mul(&db, unsafe { &(*gf).tables });
    ((result.debt as i64) << 48) | (result.mantissa & 0xFFFFFFFFFFFF)
}

#[no_mangle]
pub extern "C" fn geofield_debt_add(a: i64, b: i64, gf: *mut GeoField) -> i64 {
    if gf.is_null() {
        return 0;
    }
    let da = DebtNumber::new(
        geofield_debt_mantissa(a),
        geofield_debt_debt(a),
    );
    let db = DebtNumber::new(
        geofield_debt_mantissa(b),
        geofield_debt_debt(b),
    );
    let result = da.add(&db, unsafe { &(*gf).tables });
    ((result.debt as i64) << 48) | (result.mantissa & 0xFFFFFFFFFFFF)
}

#[no_mangle]
pub extern "C" fn geofield_debt_div(a: i64, b: i64, gf: *mut GeoField) -> i64 {
    if gf.is_null() {
        return 0;
    }
    let da = DebtNumber::new(
        geofield_debt_mantissa(a),
        geofield_debt_debt(a),
    );
    let db = DebtNumber::new(
        geofield_debt_mantissa(b),
        geofield_debt_debt(b),
    );
    match da.div(&db, unsafe { &(*gf).tables }) {
        Some(result) => ((result.debt as i64) << 48) | (result.mantissa & 0xFFFFFFFFFFFF),
        None => -1, // Error indicator
    }
}

#[no_mangle]
pub extern "C" fn geofield_debt_to_float(packed: i64) -> f64 {
    let d = DebtNumber::new(
        geofield_debt_mantissa(packed),
        geofield_debt_debt(packed),
    );
    d.to_float()
}

// ── by_P Index ──

#[no_mangle]
pub extern "C" fn geofield_byp_find(gf: *mut GeoField, p: u32, factor: u32) -> u32 {
    if gf.is_null() {
        return 0;
    }
    match unsafe { (*gf).by_p.find_divisor(p, factor) } {
        Some(q) => q,
        None => 0,
    }
}

#[no_mangle]
pub extern "C" fn geofield_byp_count(gf: *mut GeoField, p: u32) -> u32 {
    if gf.is_null() {
        return 0;
    }
    unsafe { (*gf).by_p.get_pairs(p).len() as u32 }
}

#[no_mangle]
pub extern "C" fn geofield_byp_get_pair(
    gf: *mut GeoField,
    p: u32,
    idx: u32,
    out_x: *mut u32,
    out_y: *mut u32,
) -> i32 {
    if gf.is_null() || out_x.is_null() || out_y.is_null() {
        return -1;
    }
    let pairs = unsafe { (*gf).by_p.get_pairs(p) };
    if idx as usize >= pairs.len() {
        return -1;
    }
    let (x, y) = pairs[idx as usize];
    unsafe {
        *out_x = x;
        *out_y = y;
    }
    0
}

// ── E8 Root Lattice ──

#[no_mangle]
pub extern "C" fn geofield_e8_address_to_root(x: u32, y: u32, out_coords: *mut i8) {
    if out_coords.is_null() {
        return;
    }
    let root = address_to_root(x, y);
    unsafe {
        for i in 0..8 {
            *out_coords.add(i) = root.coords[i];
        }
    }
}

#[no_mangle]
pub extern "C" fn geofield_e8_dot_product(coords1: *const i8, coords2: *const i8) -> i16 {
    if coords1.is_null() || coords2.is_null() {
        return 0;
    }
    let r1 = E8Root::new(unsafe { std::slice::from_raw_parts(coords1, 8).try_into().unwrap() });
    let r2 = E8Root::new(unsafe { std::slice::from_raw_parts(coords2, 8).try_into().unwrap() });
    r1.dot(&r2)
}

#[no_mangle]
pub extern "C" fn geofield_e8_attention(
    queries: *const i8,
    n_queries: u32,
    keys: *const i8,
    n_keys: u32,
    values: *const i8,
    out_coords: *mut i8,
) {
    if queries.is_null() || keys.is_null() || values.is_null() || out_coords.is_null() {
        return;
    }
    
    let q_slice = unsafe { std::slice::from_raw_parts(queries, (n_queries * 8) as usize) };
    let k_slice = unsafe { std::slice::from_raw_parts(keys, (n_keys * 8) as usize) };
    let v_slice = unsafe { std::slice::from_raw_parts(values, (n_keys * 8) as usize) };
    
    let queries: Vec<E8Root> = (0..n_queries)
        .map(|i| {
            let coords: [i8; 8] = q_slice[(i * 8) as usize..(i * 8 + 8) as usize].try_into().unwrap();
            E8Root::new(coords)
        })
        .collect();
    
    let keys: Vec<E8Root> = (0..n_keys)
        .map(|i| {
            let coords: [i8; 8] = k_slice[(i * 8) as usize..(i * 8 + 8) as usize].try_into().unwrap();
            E8Root::new(coords)
        })
        .collect();
    
    let values: Vec<E8Root> = (0..n_keys)
        .map(|i| {
            let coords: [i8; 8] = v_slice[(i * 8) as usize..(i * 8 + 8) as usize].try_into().unwrap();
            E8Root::new(coords)
        })
        .collect();
    
    let outputs = e8_attn(&queries, &keys, &values);
    
    unsafe {
        for (i, output) in outputs.iter().enumerate() {
            for j in 0..8 {
                *out_coords.add(i * 8 + j) = output.coords[j];
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn geofield_e8_root_count() -> u32 {
    240
}
