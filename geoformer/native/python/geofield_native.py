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
