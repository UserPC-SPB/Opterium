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

/* ── Debt System (Fractional Arithmetic) ── */

/**
 * Convert float to packed DebtNumber (mantissa + debt).
 * Returns 64-bit packed value: mantissa in lower 48 bits, debt in upper 16.
 */
int64_t geofield_debt_from_float(double f);

/**
 * Extract mantissa from packed DebtNumber.
 */
int64_t geofield_debt_mantissa(int64_t packed);

/**
 * Extract debt from packed DebtNumber.
 */
int8_t geofield_debt_debt(int64_t packed);

/**
 * Multiply two DebtNumbers via table lookup.
 * Returns packed DebtNumber, or -1 on error.
 */
int64_t geofield_debt_mul(int64_t a, int64_t b, GeoField* gf);

/**
 * Add two DebtNumbers (aligns debts automatically).
 * Returns packed DebtNumber, or -1 on error.
 */
int64_t geofield_debt_add(int64_t a, int64_t b, GeoField* gf);

/**
 * Divide two DebtNumbers via by_P lookup.
 * Returns packed DebtNumber, or -1 if not exact division.
 */
int64_t geofield_debt_div(int64_t a, int64_t b, GeoField* gf);

/**
 * Convert packed DebtNumber back to float (for display only).
 */
double geofield_debt_to_float(int64_t packed);

/* ── by_P Index (Division Lookup) ── */

/**
 * Find quotient: given P and one factor, return the other.
 * Example: geofield_byp_find(gf, 12, 3) → 4
 * Returns 0 if not found.
 */
uint32_t geofield_byp_find(GeoField* gf, uint32_t p, uint32_t factor);

/**
 * Get number of factor pairs for P.
 * Example: geofield_byp_count(gf, 12) → 3 (pairs: 1×12, 2×6, 3×4)
 */
uint32_t geofield_byp_count(GeoField* gf, uint32_t p);

/**
 * Get factor pair at index.
 * Returns 0 on success, -1 if index out of range.
 * out_x, out_y: pointers to receive the pair values.
 */
int32_t geofield_byp_get_pair(GeoField* gf, uint32_t p, uint32_t idx,
                              uint32_t* out_x, uint32_t* out_y);

/* ── E8 Root Lattice ── */

/**
 * Generate E8 root from address (x, y) via gcd + seed mapping.
 * out_coords: array of 8 int8_t values.
 */
void geofield_e8_address_to_root(uint32_t x, uint32_t y, int8_t* out_coords);

/**
 * Compute dot product of two E8 roots.
 * Returns value in {-8, -4, 0, +4, +8}.
 */
int16_t geofield_e8_dot_product(const int8_t* coords1, const int8_t* coords2);

/**
 * E8 attention: compute attention weights via E8 dot products.
 * queries: n_queries × 8 array
 * keys: n_keys × 8 array
 * values: n_keys × 8 array
 * out_coords: n_queries × 8 output array
 */
void geofield_e8_attention(const int8_t* queries, uint32_t n_queries,
                           const int8_t* keys, uint32_t n_keys,
                           const int8_t* values, int8_t* out_coords);

/**
 * Get total number of E8 roots (always 240).
 */
uint32_t geofield_e8_root_count(void);

/* ── Generative 3D Cube ── */

/**
 * Opaque C representation of a 3D cube node.
 */
typedef struct {
    int32_t x, y, z;
    int64_t v;       // Volume: x*y*z
    int32_t s;       // Sum: x+y+z
    int64_t c;       // Planar: xy+xz+yz
    int32_t d_body;  // Body difference: |x-y|+|y-z|+|x-z|
    uint8_t phase;   // 3-bit octant
    int64_t disc;    // Cubic discriminant
} CCubeNode;

/**
 * Get or generate a cube node at address (x, y, z).
 * out_node: pointer to receive the node data.
 */
void geofield_cube_get_node(GeoField* gf, int32_t x, int32_t y, int32_t z,
                            CCubeNode* out_node);

/**
 * Get neighbors within radius via bucket spatial index.
 * Returns malloc'd array of CCubeNode. Caller must free with geofield_cube_free().
 * out_count: pointer to receive the number of neighbors.
 */
CCubeNode* geofield_cube_get_neighbors(GeoField* gf, int32_t x, int32_t y, int32_t z,
                                       int32_t radius, int32_t* out_count);

/**
 * Compute tension between two nodes.
 */
int32_t geofield_cube_tension(GeoField* gf,
                              int32_t ax, int32_t ay, int32_t az,
                              int32_t bx, int32_t by, int32_t bz);

/**
 * Solve analogy A:B :: C:D in 3D.
 * out_node: pointer to receive the result node D.
 */
void geofield_cube_analogy(GeoField* gf,
                           int32_t ax, int32_t ay, int32_t az,
                           int32_t bx, int32_t by, int32_t bz,
                           int32_t cx, int32_t cy, int32_t cz,
                           CCubeNode* out_node);

/**
 * Create or strengthen a morpho link between two nodes.
 */
void geofield_cube_morpho_link(GeoField* gf,
                               int32_t sx, int32_t sy, int32_t sz,
                               int32_t tx, int32_t ty, int32_t tz,
                               double weight);

/**
 * Get cube statistics.
 */
void geofield_cube_stats(GeoField* gf, int32_t* out_cached, int32_t* out_buckets,
                         int32_t* out_morpho, int64_t* out_address_space);

/**
 * Free malloc'd cube node array.
 */
void geofield_cube_free(void* ptr);

#ifdef __cplusplus
}
#endif

#endif /* GEOFIELD_H */
