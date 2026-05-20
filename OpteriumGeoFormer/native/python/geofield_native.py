"""
geofield_native.py — «Тупые» Python-обёртки над Rust GeoField.

ПРАВИЛА:
- Ноль арифметики в этом файле.
- Ноль создания объектов в hot paths.
- Ноль приведений типов.
- Только: передать указатели → вызвать Rust → вернуть int.

Использование:
    from geofield_native import GeoField
    gf = GeoField("tables.ptbl")
    p = gf.P(4, 3)        # → 12
    result = gf.matmul(a, m, k, b, n)  # → list[int]
"""

from __future__ import annotations

import os
from cffi import FFI

ffi = FFI()

# ── C declarations (копия geofield.h) ──
ffi.cdef("""
    typedef struct GeoField GeoField;
    typedef struct GeoResult GeoResult;

    GeoField* geofield_init(const char* table_path);
    void geofield_destroy(GeoField* gf);

    int32_t geofield_P(GeoField* gf, int32_t x, int32_t y);
    int32_t geofield_S(GeoField* gf, int32_t x, int32_t y);
    int32_t geofield_D(GeoField* gf, int32_t x, int32_t y);
    int32_t geofield_p_from_sd(GeoField* gf, int32_t s, int32_t d);
    int32_t geofield_p_from_xy(GeoField* gf, int32_t x, int32_t y);
    int32_t geofield_proximity(GeoField* gf, int32_t dist);
    int32_t geofield_int_weight(GeoField* gf, int32_t s1, int32_t d1, int32_t s2, int32_t d2);
    int32_t geofield_product(GeoField* gf, int32_t a, int32_t b);
    int32_t geofield_isqrt(GeoField* gf, int32_t n);

    GeoResult* geofield_matmul(GeoField* gf,
                               const int32_t* A, int32_t m, int32_t k,
                               const int32_t* B, int32_t n);
    GeoResult* geofield_attention(GeoField* gf,
                                  const int32_t* tokens, int32_t n_tokens,
                                  int32_t window);

    const int32_t* geofield_result_data(const GeoResult* r);
    int32_t geofield_result_rows(const GeoResult* r);
    int32_t geofield_result_cols(const GeoResult* r);
    int32_t geofield_result_len(const GeoResult* r);
    void geofield_result_free(GeoResult* r);

    int32_t geofield_max_coord(const GeoField* gf);
    size_t geofield_table_size(const GeoField* gf);

    // Debt System
    int64_t geofield_debt_from_float(double f);
    int64_t geofield_debt_mantissa(int64_t packed);
    int8_t geofield_debt_debt(int64_t packed);
    int64_t geofield_debt_mul(int64_t a, int64_t b, GeoField* gf);
    int64_t geofield_debt_add(int64_t a, int64_t b, GeoField* gf);
    int64_t geofield_debt_div(int64_t a, int64_t b, GeoField* gf);
    double geofield_debt_to_float(int64_t packed);

    // by_P Index
    uint32_t geofield_byp_find(GeoField* gf, uint32_t p, uint32_t factor);
    uint32_t geofield_byp_count(GeoField* gf, uint32_t p);
    int32_t geofield_byp_get_pair(GeoField* gf, uint32_t p, uint32_t idx, uint32_t* out_x, uint32_t* out_y);

    // E8 Root Lattice
    void geofield_e8_address_to_root(uint32_t x, uint32_t y, int8_t* out_coords);
    int16_t geofield_e8_dot_product(const int8_t* coords1, const int8_t* coords2);
    void geofield_e8_attention(const int8_t* queries, uint32_t n_queries,
                               const int8_t* keys, uint32_t n_keys,
                               const int8_t* values, int8_t* out_coords);
    uint32_t geofield_e8_root_count(void);
""")

# ── Загрузка библиотеки ──
_lib_path = os.path.join(os.path.dirname(__file__), "..", "target", "release", "geofield.dll")
_lib = ffi.dlopen(_lib_path)


class GeoField:
    """Непрозрачный хэндл к Rust GeoField. Ноль арифметики, ноль аллокаций."""

    __slots__ = ("_ptr",)

    def __init__(self, table_path: str):
        ptr = _lib.geofield_init(table_path.encode())
        if ptr == ffi.NULL:
            raise FileNotFoundError(f"Не удалось загрузить таблицы: {table_path}")
        self._ptr = ptr

    def __del__(self):
        if hasattr(self, '_ptr') and self._ptr:
            _lib.geofield_destroy(self._ptr)

    # ── Pure lookup (прямые вызовы Rust, без Python-арифметики) ──

    def P(self, x: int, y: int) -> int:
        return _lib.geofield_P(self._ptr, x, y)

    def S(self, x: int, y: int) -> int:
        return _lib.geofield_S(self._ptr, x, y)

    def D(self, x: int, y: int) -> int:
        return _lib.geofield_D(self._ptr, x, y)

    def p_from_sd(self, s: int, d: int) -> int:
        return _lib.geofield_p_from_sd(self._ptr, s, d)

    def p_from_xy(self, x: int, y: int) -> int:
        return _lib.geofield_p_from_xy(self._ptr, x, y)

    def proximity(self, dist: int) -> int:
        return _lib.geofield_proximity(self._ptr, dist)

    def int_weight(self, s1: int, d1: int, s2: int, d2: int) -> int:
        return _lib.geofield_int_weight(self._ptr, s1, d1, s2, d2)

    def product(self, a: int, b: int) -> int:
        return _lib.geofield_product(self._ptr, a, b)

    def isqrt(self, n: int) -> int:
        return _lib.geofield_isqrt(self._ptr, n)

    # ── Matrix multiply (передача указателей, возврат list) ──

    def matmul(self, a, m: int, k: int, b, n: int) -> list[int]:
        """a, b: list[int]. Ноль арифметики здесь."""
        a_ptr = ffi.new("int32_t[]", a)
        b_ptr = ffi.new("int32_t[]", b)
        r = _lib.geofield_matmul(self._ptr, a_ptr, m, k, b_ptr, n)
        length = _lib.geofield_result_len(r)
        data = ffi.cast("int32_t*", _lib.geofield_result_data(r))
        result = [data[i] for i in range(length)]
        _lib.geofield_result_free(r)
        return result

    # ── Attention (передача указателей, возврат list) ──

    def attention(self, tokens, n_tokens: int, window: int) -> list[int]:
        """tokens: list[int] [id, S, D, P, ...]. Ноль арифметики здесь."""
        t_ptr = ffi.new("int32_t[]", tokens)
        r = _lib.geofield_attention(self._ptr, t_ptr, n_tokens, window)
        length = _lib.geofield_result_len(r)
        data = ffi.cast("int32_t*", _lib.geofield_result_data(r))
        result = [data[i] for i in range(length)]
        _lib.geofield_result_free(r)
        return result

    # ── Метаданные ──

    def max_coord(self) -> int:
        return _lib.geofield_max_coord(self._ptr)

    def table_size(self) -> int:
        return _lib.geofield_table_size(self._ptr)

    # ── Debt System (fractional arithmetic) ──

    def debt_from_float(self, f: float) -> int:
        return _lib.geofield_debt_from_float(f)

    def debt_mantissa(self, packed: int) -> int:
        return _lib.geofield_debt_mantissa(packed)

    def debt_debt(self, packed: int) -> int:
        return _lib.geofield_debt_debt(packed)

    def debt_mul(self, a: int, b: int) -> int:
        return _lib.geofield_debt_mul(a, b, self._ptr)

    def debt_add(self, a: int, b: int) -> int:
        return _lib.geofield_debt_add(a, b, self._ptr)

    def debt_div(self, a: int, b: int) -> int:
        return _lib.geofield_debt_div(a, b, self._ptr)

    def debt_to_float(self, packed: int) -> float:
        return _lib.geofield_debt_to_float(packed)

    # ── by_P Index (division lookup) ──

    def byp_find(self, p: int, factor: int) -> int:
        return _lib.geofield_byp_find(self._ptr, p, factor)

    def byp_count(self, p: int) -> int:
        return _lib.geofield_byp_count(self._ptr, p)

    def byp_get_pair(self, p: int, idx: int) -> tuple:
        out_x = ffi.new("uint32_t*")
        out_y = ffi.new("uint32_t*")
        ret = _lib.geofield_byp_get_pair(self._ptr, p, idx, out_x, out_y)
        if ret != 0:
            return None
        return (int(out_x[0]), int(out_y[0]))

    # ── E8 Root Lattice ──

    def e8_address_to_root(self, x: int, y: int) -> list:
        out_coords = ffi.new("int8_t[8]")
        _lib.geofield_e8_address_to_root(x, y, out_coords)
        return [int(out_coords[i]) for i in range(8)]

    def e8_dot_product(self, coords1: list, coords2: list) -> int:
        c1 = ffi.new("int8_t[8]", coords1)
        c2 = ffi.new("int8_t[8]", coords2)
        return _lib.geofield_e8_dot_product(c1, c2)

    def e8_attention(self, queries: list, keys: list, values: list) -> list:
        n_queries = len(queries)
        n_keys = len(keys)
        
        q_flat = [c for q in queries for c in q]
        k_flat = [c for k in keys for c in k]
        v_flat = [c for v in values for c in v]
        
        q_ptr = ffi.new("int8_t[]", q_flat)
        k_ptr = ffi.new("int8_t[]", k_flat)
        v_ptr = ffi.new("int8_t[]", v_flat)
        out_ptr = ffi.new("int8_t[]", [0] * (n_queries * 8))
        
        _lib.geofield_e8_attention(q_ptr, n_queries, k_ptr, n_keys, v_ptr, out_ptr)
        
        result = []
        for i in range(n_queries):
            result.append([int(out_ptr[i * 8 + j]) for j in range(8)])
        return result

    def e8_root_count(self) -> int:
        return _lib.geofield_e8_root_count()
