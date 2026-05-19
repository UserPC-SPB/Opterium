//! geofield — Opterium GeoFormer native core.
//!
//! Pure-lookup geometric math engine. Zero arithmetic in hot paths.
//! Exposes opaque C API via `geofield.h`.

mod tables;
mod lookup;

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::path::Path;
use std::sync::Mutex;

use tables::Tables;
use lookup::Result as GeoResult;

/// Opaque handle to GeoField.
pub struct GeoField {
    tables: Tables,
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
        Ok(tables) => Box::into_raw(Box::new(GeoField {
            tables,
            last_error: Mutex::new(None),
        })),
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
