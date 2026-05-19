/*
 * geofield.h — Opaque C API for Opterium GeoFormer
 *
 * This is the ONLY public interface. All internals are hidden.
 * Users see a magic box that does geometric math. Nothing more.
 *
 * Usage:
 *   GeoField* gf = geofield_init("tables.ptbl");
 *   int32_t p = geofield_P(gf, 4, 3);        // returns 12
 *   GeoResult* r = geofield_matmul(gf, A, m, k, B, n);
 *   const int32_t* data = geofield_result_data(r);
 *   geofield_result_free(r);
 *   geofield_destroy(gf);
 */

#ifndef GEOFIELD_H
#define GEOFIELD_H

#include <stdint.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

/* ── Opaque handles (internals hidden) ── */
typedef struct GeoField GeoField;
typedef struct GeoResult GeoResult;

/* ── Lifecycle ── */

/**
 * Initialize GeoField from a .ptbl table file.
 * Returns opaque handle, or NULL on error.
 * Thread-safe: multiple handles can coexist.
 */
GeoField* geofield_init(const char* table_path);

/**
 * Initialize GeoField with embedded tables (Static Build).
 * Only available when compiled with GEOFIELD_EMBEDDED_TABLES.
 */
GeoField* geofield_init_embedded(void);

/**
 * Destroy GeoField and release resources.
 */
void geofield_destroy(GeoField* gf);

/* ── Pure Lookup Operations ── */

/**
 * Product: P = x * y via table lookup.
 * Returns 0 if out of range.
 */
int32_t geofield_P(GeoField* gf, int32_t x, int32_t y);

/**
 * Sum: S = x + y via table lookup.
 * Returns 0 if out of range.
 */
int32_t geofield_S(GeoField* gf, int32_t x, int32_t y);

/**
 * Difference: D = x - y via table lookup.
 * Returns 0 if out of range.
 */
int32_t geofield_D(GeoField* gf, int32_t x, int32_t y);

/**
 * Product from (S, D) coordinates: P via _SP table lookup.
 * This is the core operation — zero arithmetic, direct memory read.
 */
int32_t geofield_p_from_sd(GeoField* gf, int32_t s, int32_t d);

/**
 * Product from (x, y) coordinates: P via _P table lookup.
 */
int32_t geofield_p_from_xy(GeoField* gf, int32_t x, int32_t y);

/**
 * Integer proximity weight: SCALE / (1 + dist).
 * dist = |ΔS| + |ΔD|. Returns 0 if dist out of range.
 */
int32_t geofield_proximity(GeoField* gf, int32_t dist);

/**
 * Integer proximity weight between two (S, D) points.
 * dist = |s1-s2| + |d1-d2|.
 */
int32_t geofield_int_weight(GeoField* gf, int32_t s1, int32_t d1,
                            int32_t s2, int32_t d2);

/**
 * Product of two values via table lookup with gcd-scaling fallback.
 * For values within table range: direct lookup.
 * For larger values: gcd-decomposition.
 */
int32_t geofield_product(GeoField* gf, int32_t a, int32_t b);

/**
 * Integer square root via table lookup.
 * Returns 0 for n <= 0 or out of range.
 */
int32_t geofield_isqrt(GeoField* gf, int32_t n);

/* ── Matrix Multiply ── */

/**
 * Matrix multiply: C = A × B
 *
 * A: m×k matrix (row-major flat array)
 * B: k×n matrix (row-major flat array)
 * C: m×n result (allocated internally)
 *
 * All operations: table lookup + integer accumulation.
 * Zero float, zero GPU.
 */
GeoResult* geofield_matmul(GeoField* gf,
                           const int32_t* A, int32_t m, int32_t k,
                           const int32_t* B, int32_t n);

/* ── Geometric Attention ── */

/**
 * Geometric attention via hashgrid proximity.
 *
 * tokens: flat array of [id, S, D, P] tuples (length = n_tokens * 4)
 * window: hashgrid bucket size
 *
 * Returns result as flat array of [id, context, n_neighbors, output_x, output_y].
 * All operations: integer lookup, zero float.
 */
GeoResult* geofield_attention(GeoField* gf,
                              const int32_t* tokens, int32_t n_tokens,
                              int32_t window);

/* ── Result Access ── */

/**
 * Get pointer to result data (flat int32 array).
 * Valid until geofield_result_free() is called.
 */
const int32_t* geofield_result_data(const GeoResult* r);

/**
 * Get number of rows in result matrix.
 */
int32_t geofield_result_rows(const GeoResult* r);

/**
 * Get number of columns in result matrix.
 */
int32_t geofield_result_cols(const GeoResult* r);

/**
 * Get total length of flat result array.
 */
int32_t geofield_result_len(const GeoResult* r);

/**
 * Free result memory.
 */
void geofield_result_free(GeoResult* r);

/* ── Error Handling ── */

/**
 * Get last error message (thread-local).
 * Returns NULL if no error.
 */
const char* geofield_last_error(const GeoField* gf);

/**
 * Get max_coord value (table size).
 */
int32_t geofield_max_coord(const GeoField* gf);

/**
 * Get table file size in bytes.
 */
size_t geofield_table_size(const GeoField* gf);

#ifdef __cplusplus
}
#endif

#endif /* GEOFIELD_H */
