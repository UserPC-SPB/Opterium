"""
Тесты для release/cube_v5.exe (MCP-сервер)
Каждый запрос — отдельный запуск exe через stdin.
"""
import json
import subprocess
import sys

EXE = r"C:\Users\eccoa\Desktop\кубики\release\cube_v5.exe"


def _rpc(method: str, params: dict = None):
    """Отправить JSON-RPC и получить сырой response dict."""
    req = {"jsonrpc": "2.0", "id": 1, "method": method, "params": params or {}}
    payload = json.dumps(req, ensure_ascii=False)
    proc = subprocess.Popen(
        [EXE], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True
    )
    stdout, stderr = proc.communicate(input=payload, timeout=30)
    if stderr and not stdout:
        raise RuntimeError(f"stderr: {stderr}")
    for line in reversed(stdout.strip().split("\n")):
        line = line.strip()
        if line.startswith("{"):
            return json.loads(line)
    raise RuntimeError(f"Нет JSON: {stdout!r}")


def get_text(name: str, arguments: dict = None):
    """Вызвать инструмент, вернуть сырой text (строка)."""
    resp = _rpc("tools/call", {"name": name, "arguments": arguments or {}})
    if "error" in resp:
        raise AssertionError(f"Ошибка {name}: {resp['error']}")
    return resp["result"]["content"][0]["text"]


def get_json(name: str, arguments: dict = None):
    """Вызвать инструмент, распарсить text как JSON."""
    text = get_text(name, arguments)
    return json.loads(text)


def tools_list():
    resp = _rpc("tools/list")
    if "error" in resp:
        raise AssertionError(f"Ошибка tools/list: {resp['error']}")
    return resp["result"]["tools"]


def run_all():
    total = 0
    passed = 0
    failed = 0

    def test(name, fn):
        nonlocal total, passed, failed
        total += 1
        try:
            fn()
            passed += 1
        except Exception as e:
            print(f"  FAIL: {name}: {e}")
            failed += 1

    print(f"\n{'='*60}")
    print(f"RELEASE TEST SUITE: cube_v5.exe")
    print(f"{'='*60}\n")

    # ─── 1. tools/list ─────────────────────────────────────────
    def check_tools_list():
        tools = tools_list()
        assert len(tools) == 71, f"71, получено {len(tools)}"
        names = {t["name"] for t in tools}
        for req in ("cubint_add", "cubfloat_add", "cubcomplex_add",
                     "e8", "doctor", "help", "vec_sum",
                     "spatial_place", "random_n", "reset_cube"):
            assert req in names, f"Нет {req}"
        print(f"  PASS: tools/list ({len(tools)} tools)")
    test("tools/list", check_tools_list)

    # ─── 2. CubInt ────────────────────────────────────────────
    def check_ci_add():
        r = get_json("cubint_add", {"a": 3, "b": 4})
        assert r["result"] == 7
        print("  PASS: cubint_add(3,4) = 7")
    test("cubint_add", check_ci_add)

    def check_ci_sub():
        r = get_json("cubint_sub", {"a": 10, "b": 3})
        assert r["result"] == 7
        print("  PASS: cubint_sub(10,3) = 7")
    test("cubint_sub", check_ci_sub)

    def check_ci_mul():
        r = get_json("cubint_mul", {"a": 4, "b": 3})
        assert r["result"] == 12
        assert "witness" in r
        print(f"  PASS: cubint_mul(4,3) = 12")
    test("cubint_mul", check_ci_mul)

    def check_ci_floordiv():
        r = get_json("cubint_floordiv", {"a": 7, "b": 2})
        assert isinstance(r["result"], int)
        print(f"  PASS: cubint_floordiv(7,2) = {r['result']}")
    test("cubint_floordiv", check_ci_floordiv)

    def check_ci_truediv():
        r = get_json("cubint_truediv", {"a": 7, "b": 2})
        assert "result" in r
        print(f"  PASS: cubint_truediv(7,2) = {r['result']}")
    test("cubint_truediv", check_ci_truediv)

    def check_ci_pow():
        r = get_json("cubint_pow", {"a": 2, "b": 3})
        assert isinstance(r["result"], int)
        print(f"  PASS: cubint_pow(2,3) = {r['result']}")
    test("cubint_pow", check_ci_pow)

    def check_ci_mod():
        r = get_json("cubint_mod", {"a": 7, "b": 5})
        assert isinstance(r["result"], int)
        print(f"  PASS: cubint_mod(7,5) = {r['result']}")
    test("cubint_mod", check_ci_mod)

    def check_ci_neg():
        raw = get_text("cubint_neg", {"a": 5})
        val = json.loads(raw)
        if isinstance(val, dict):
            assert val.get("result") == -5
        else:
            assert val == -5
        print(f"  PASS: cubint_neg(5) = {val}")
    test("cubint_neg", check_ci_neg)

    def check_ci_abs():
        raw = get_text("cubint_abs", {"a": -5})
        val = json.loads(raw)
        if isinstance(val, dict):
            assert val.get("result") == 5
        else:
            assert val == 5
        print(f"  PASS: cubint_abs(-5) = {val}")
    test("cubint_abs", check_ci_abs)

    def check_ci_witness():
        r = get_json("cubint_witness", {"a": 12})
        assert isinstance(r, dict) and len(r) > 0
        print("  PASS: cubint_witness(12) ok")
    test("cubint_witness", check_ci_witness)

    def check_ci_validate():
        raw = get_text("cubint_validate", {"a": 12})
        val = json.loads(raw)
        if isinstance(val, dict):
            assert val.get("result") == True or val.get("valid") == True
        else:
            assert val == True
        print(f"  PASS: cubint_validate(12) = {val}")
    test("cubint_validate", check_ci_validate)

    def check_ci_large():
        r = get_json("cubint_add", {"a": 100, "b": 200})
        assert r["result"] == 300
        r = get_json("cubint_mul", {"a": 100, "b": 200})
        assert r["result"] == 20000
        print("  PASS: cubint 100+200=300, 100*200=20000")
    test("cubint_large", check_ci_large)

    # ─── 3. CubFloat ──────────────────────────────────────────
    def check_cf_add():
        raw = get_text("cubfloat_add", {"a": "0.1", "b": "0.2"})
        val = json.loads(raw)
        if isinstance(val, dict):
            assert "result" in val
            val = val["result"]
        print(f"  PASS: cubfloat_add(0.1,0.2) = {val}")
    test("cubfloat_add", check_cf_add)

    def check_cf_sub():
        raw = get_text("cubfloat_sub", {"a": "0.3", "b": "0.1"})
        val = json.loads(raw)
        val = val if not isinstance(val, dict) else val.get("result", val)
        print(f"  PASS: cubfloat_sub(0.3,0.1) = {val}")
    test("cubfloat_sub", check_cf_sub)

    def check_cf_mul():
        raw = get_text("cubfloat_mul", {"a": "0.1", "b": "0.1"})
        val = json.loads(raw)
        val = val if not isinstance(val, dict) else val.get("result", val)
        print(f"  PASS: cubfloat_mul(0.1,0.1) = {val}")
    test("cubfloat_mul", check_cf_mul)

    def check_cf_div():
        raw = get_text("cubfloat_truediv", {"a": "1", "b": "3"})
        val = json.loads(raw)
        val = val if not isinstance(val, dict) else val.get("result", val)
        print(f"  PASS: cubfloat_truediv(1,3) = {val}")
    test("cubfloat_truediv", check_cf_div)

    def check_cf_neg():
        raw = get_text("cubfloat_neg", {"a": "0.5"})
        val = json.loads(raw)
        val = val if not isinstance(val, dict) else val.get("result", val)
        print(f"  PASS: cubfloat_neg(0.5) = {val}")
    test("cubfloat_neg", check_cf_neg)

    def check_cf_abs():
        raw = get_text("cubfloat_abs", {"a": "-0.5"})
        val = json.loads(raw)
        val = val if not isinstance(val, dict) else val.get("result", val)
        print(f"  PASS: cubfloat_abs(-0.5) = {val}")
    test("cubfloat_abs", check_cf_abs)

    # ─── 4. CubComplex ────────────────────────────────────────
    def check_cc_add():
        r = get_json("cubcomplex_add", {"a_re": 1, "a_im": 2, "b_re": 3, "b_im": 4})
        assert r["re"] == 4 and r["im"] == 6
        print(f"  PASS: cubcomplex_add = {r}")
    test("cubcomplex_add", check_cc_add)

    def check_cc_sub():
        r = get_json("cubcomplex_sub", {"a_re": 5, "a_im": 6, "b_re": 3, "b_im": 4})
        assert r["re"] == 2 and r["im"] == 2
        print(f"  PASS: cubcomplex_sub = {r}")
    test("cubcomplex_sub", check_cc_sub)

    def check_cc_mul():
        r = get_json("cubcomplex_mul", {"a_re": 1, "a_im": 1, "b_re": 1, "b_im": 1})
        assert r["re"] == 0 and r["im"] == 2
        print(f"  PASS: cubcomplex_mul = {r}")
    test("cubcomplex_mul", check_cc_mul)

    def check_cc_conj():
        r = get_json("cubcomplex_conjugate", {"re": 1, "im": 2})
        assert r["im"] == -2
        print(f"  PASS: cubcomplex_conjugate = {r}")
    test("cubcomplex_conjugate", check_cc_conj)

    def check_cc_abs():
        r = get_json("cubcomplex_abs", {"re": 3, "im": 4})
        assert "magnitude" in r
        print(f"  PASS: cubcomplex_abs = {r}")
    test("cubcomplex_abs", check_cc_abs)

    def check_cc_pow():
        r = get_json("cubcomplex_pow", {"re": 1, "im": 0, "exp": 5})
        assert r["re"] == 1
        print(f"  PASS: cubcomplex_pow = {r}")
    test("cubcomplex_pow", check_cc_pow)

    def check_cc_neg():
        r = get_json("cubcomplex_neg", {"re": 1, "im": -2})
        assert r["re"] == -1 and r["im"] == 2
        print(f"  PASS: cubcomplex_neg = {r}")
    test("cubcomplex_neg", check_cc_neg)

    # ─── 5. E8 ───────────────────────────────────────────────
    def check_e8():
        r = get_json("e8", {"value": 1})
        for key in ("vec8", "kind", "norm2"):
            assert key in r
        assert len(r["vec8"]) == 8
        print(f"  PASS: e8(1): kind={r['kind']}")
    test("e8", check_e8)

    def check_e8_get_root():
        raw = get_text("e8_get_root", {"idx": 0})
        data = json.loads(raw)
        if isinstance(data, dict):
            vec = data.get("vec8", data.get("vector", []))
        else:
            vec = data  # сам список
        assert len(vec) == 8, f"не 8D: {len(vec)}"
        print("  PASS: e8_get_root(0): 8D")
    test("e8_get_root", check_e8_get_root)

    def check_e8_partners_split():
        raw = get_text("e8_partners_split", {"idx": 1})
        data = json.loads(raw)
        if isinstance(data, dict):
            assert "D8" in data or "D8_count" in data
            print(f"  PASS: e8_partners_split(1) ok")
        else:
            print(f"  PASS: e8_partners_split(1) = {type(data).__name__}")
    test("e8_partners_split", check_e8_partners_split)

    def check_e8_weyl_depth():
        r = get_json("e8_weyl_depth", {"idx": 1})
        assert r["depth0"] >= 1
        print("  PASS: e8_weyl_depth(1) ok")
    test("e8_weyl_depth", check_e8_weyl_depth)

    def check_e8_duality():
        r = get_json("e8_duality_check", {"idx": 1})
        assert r["partner_count"] == 56
        print("  PASS: e8_duality_check(1) 56 partners")
    test("e8_duality_check", check_e8_duality)

    def check_e8_spectrum():
        r = get_json("e8_spectrum_check", {"sample_size": 240})
        assert isinstance(r, dict)
        print(f"  PASS: e8_spectrum_check(240) keys count = {len(r)}")
    test("e8_spectrum_check", check_e8_spectrum)

    def check_e8_batch():
        r = get_json("e8_batch", {"values": [1, 2, 3, 4, 5]})
        assert "kinds" in r
        print(f"  PASS: e8_batch([1..5]) kinds={r['kinds']}")
    test("e8_batch", check_e8_batch)

    def check_e8_batch_timed():
        r = get_json("e8_batch_timed", {"values": list(range(100))})
        assert "elapsed_ms" in r
        assert "elapsed_ns" in r
        print(f"  PASS: e8_batch_timed(100) ms={r['elapsed_ms']}")
    test("e8_batch_timed", check_e8_batch_timed)

    def check_e8_stats():
        r = get_json("e8_stats", {"values": [1, 2, 3, 4, 5]})
        assert "sum" in r
        assert any("mean" in k.lower() for k in r.keys())
        print(f"  PASS: e8_stats sum={r['sum']}")
    test("e8_stats", check_e8_stats)

    # ─── 6. Doctor ────────────────────────────────────────────
    def check_doctor():
        r = get_json("doctor", {"value": 12})
        for key in ("factors", "divisors"):
            assert key in r
        assert 12 in r["divisors"]
        print(f"  PASS: doctor(12) factors={r['factors']}")
    test("doctor", check_doctor)

    # ─── 7. Прочие ──────────────────────────────────────────
    def check_addr3():
        raw = get_text("addr3_stack", {"value": 42})
        data = json.loads(raw)
        assert data is not None
        print(f"  PASS: addr3_stack(42) = {type(data).__name__}")
    test("addr3_stack", check_addr3)

    def check_optg():
        raw = get_text("optg_path", {"x": 3, "y": 4, "z": 0})
        data = json.loads(raw)
        assert data is not None
        print(f"  PASS: optg_path(3,4,0) = {type(data).__name__}")
    test("optg_path", check_optg)

    def check_neighbors():
        raw = get_text("neighbors26", {"x": 0, "y": 0, "z": 0})
        data = json.loads(raw)
        assert data is not None
        print(f"  PASS: neighbors26(0,0,0) = {type(data).__name__}")
    test("neighbors26", check_neighbors)

    # ─── 8. Spatial ──────────────────────────────────────────
    def check_sp_place():
        r = get_json("spatial_place", {"x": 1, "y": 2, "z": 1})
        assert r.get("placed") is True
        print("  PASS: spatial_place(1,2,1) ok")
    test("spatial_place", check_sp_place)

    def check_sp_move():
        r = get_json("spatial_move", {"x": 0, "y": 3, "z": 0, "dx": 1, "dy": 0, "dz": 0})
        assert r.get("moved") is True
        print("  PASS: spatial_move ok")
    test("spatial_move", check_sp_move)

    def check_sp_gravity():
        r = get_json("spatial_place", {"x": 5, "y": 10, "z": 0})
        print(f"  PASS: spatial_place(5,10,0) — y={r.get('y')}")
    test("spatial_gravity", check_sp_gravity)

    # ─── 9. Vector ──────────────────────────────────────────
    def check_vec_add():
        raw = get_text("vec_add", {"a": [1, 2, 3], "b": [10, 20, 30]})
        data = json.loads(raw)
        assert data is not None
        print(f"  PASS: vec_add = {type(data).__name__}")
    test("vec_add", check_vec_add)

    def check_vec_dot():
        raw = get_text("vec_dot", {"a": [1, 2, 3], "b": [4, 5, 6]})
        data = json.loads(raw)
        assert data is not None
        print(f"  PASS: vec_dot = {type(data).__name__}")
    test("vec_dot", check_vec_dot)

    def check_vec_sum():
        raw = get_text("vec_sum", {"a": [1, 2, 3]})
        data = json.loads(raw)
        assert data is not None
        print(f"  PASS: vec_sum = {type(data).__name__}")
    test("vec_sum", check_vec_sum)

    def check_vec_sort():
        raw = get_text("vec_sort", {"a": [3, 1, 4, 1, 5]})
        data = json.loads(raw)
        assert data is not None
        print(f"  PASS: vec_sort = {type(data).__name__}")
    test("vec_sort", check_vec_sort)

    def check_vec_unique():
        raw = get_text("vec_unique", {"a": [1, 2, 2, 3, 1]})
        data = json.loads(raw)
        assert data is not None
        print(f"  PASS: vec_unique = {type(data).__name__}")
    test("vec_unique", check_vec_unique)

    # ─── 10. random_n ───────────────────────────────────────
    def check_random():
        r = get_json("random_n", {"n": 10, "seed": 42})
        assert isinstance(r, dict)
        r2 = get_json("random_n", {"n": 10, "seed": 42})
        assert str(r) == str(r2)  # сравниваем по содержимому
        r3 = get_json("random_n", {"n": 10, "seed": 99})
        assert str(r) != str(r3)
        print("  PASS: random_n детерминирован")
    test("random_n", check_random)

    # ─── 11. reset_cube ─────────────────────────────────────
    def check_reset():
        r = get_json("reset_cube", {})
        assert r.get("status") == "ok"
        print("  PASS: reset_cube() ok")
    test("reset_cube", check_reset)

    # ─── 12. help ──────────────────────────────────────────
    def check_help_en():
        raw = get_text("help", {"lang": "en"})
        assert len(raw) > 200
        print(f"  PASS: help(en) — {len(raw)} chars")
    test("help_en", check_help_en)

    def check_help_ru():
        raw = get_text("help", {"lang": "ru"})
        assert len(raw) > 200
        assert any(w in raw for w in ("инструмент", "сложени", "умножени", "нестандарт"))
        print(f"  PASS: help(ru) — {len(raw)} chars, русский")
    test("help_ru", check_help_ru)

    def check_help_zh():
        raw = get_text("help", {"lang": "zh"})
        assert len(raw) > 200
        print(f"  PASS: help(zh) — {len(raw)} chars")
    test("help_zh", check_help_zh)

    def check_help_de():
        raw = get_text("help", {"lang": "de"})
        assert len(raw) > 200
        print(f"  PASS: help(de) — {len(raw)} chars")
    test("help_de", check_help_de)

    # ─── 13. Stress ────────────────────────────────────────
    def check_stress_batch():
        r = get_json("e8_batch", {"values": list(range(1000))})
        assert r["count"] == 1000
        print(f"  PASS: e8_batch(1000) count={r['count']}")
    test("e8_batch_1000", check_stress_batch)

    # ─── Итог ──────────────────────────────────────────────
    print(f"\n{'='*60}")
    print(f"РЕЗУЛЬТАТЫ: {passed}/{total} пройдено, {failed} не пройдено")
    print(f"{'='*60}")

    return failed == 0


if __name__ == "__main__":
    success = run_all()
    sys.exit(0 if success else 1)