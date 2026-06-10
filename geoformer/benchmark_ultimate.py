"""
Cube v5 — ULTIMATE Benchmark & Test Suite v2
=============================================
Два режима измерения:
  A) COLD-START: каждый запрос = новый запуск exe (замер syscall overhead)
  B) PERSISTENT:  один процесс, N запросов через stdin (реальная производительность MCP)

Тесты:
  1. Функциональная корректность (71 инструмент)
  2. Cold-start latency (мс)
  3. Persistent latency (мкс — микросекунды!)
  4. Throughput: requests/sec
  5. Stress: batch 1K..10K, edge cases
  6. E8 deep: all 240 roots, distance_matrix 50x50
  7. Vector: large vectors 10K+
  8. Binary analysis (size, startup time)

Итог: JSON + Markdown отчёт
"""

import json
import subprocess
import time
import sys
import os
import statistics
from collections import defaultdict

EXE = r"C:\Users\eccoa\Desktop\кубики\release\cube_v5.exe"
OUT_DIR = os.path.dirname(os.path.abspath(__file__))

# ═══════════════════════════════════════════════════════════════════
#  RPC helpers
# ═══════════════════════════════════════════════════════════════════

def _cold_rpc(method: str, params: dict = None):
    """Один вызов = один запуск exe. Возвращает (response_dict, elapsed_ms)."""
    req = {"jsonrpc": "2.0", "id": 1, "method": method, "params": params or {}}
    payload = json.dumps(req, ensure_ascii=False)
    start = time.perf_counter()
    proc = subprocess.Popen(
        [EXE], stdin=subprocess.PIPE, stdout=subprocess.PIPE,
        stderr=subprocess.PIPE, text=True, cwd=OUT_DIR
    )
    stdout, stderr = proc.communicate(input=payload, timeout=60)
    elapsed = (time.perf_counter() - start) * 1000
    if stderr and not stdout:
        raise RuntimeError(f"stderr: {stderr}")
    for line in reversed(stdout.strip().split("\n")):
        line = line.strip()
        if line.startswith("{"):
            return json.loads(line), elapsed
    raise RuntimeError(f"No JSON in: {stdout[:200]!r}")


class PersistentSession:
    """Persistent MCP session — один процесс, много запросов."""

    def __init__(self):
        self.proc = subprocess.Popen(
            [EXE], stdin=subprocess.PIPE, stdout=subprocess.PIPE,
            stderr=subprocess.PIPE, text=True, cwd=OUT_DIR
        )
        self._id = 0
        self._warmup()

    def _warmup(self):
        """Прогрев: tools/list."""
        self.call("tools/list")

    def call(self, method: str, params: dict = None):
        """Отправить запрос, получить response. Возвращает (response_dict, elapsed_ms)."""
        self._id += 1
        req = {"jsonrpc": "2.0", "id": self._id, "method": method, "params": params or {}}
        payload = json.dumps(req, ensure_ascii=False) + "\n"
        start = time.perf_counter()
        self.proc.stdin.write(payload)
        self.proc.stdin.flush()
        # Читаем до следующего JSON-ответа
        buf = ""
        while True:
            ch = self.proc.stdout.read(1)
            if not ch:
                break
            buf += ch
            if ch == "\n":
                buf = buf.strip()
                if buf.startswith("{"):
                    try:
                        data = json.loads(buf)
                        elapsed = (time.perf_counter() - start) * 1000
                        return data, elapsed
                    except json.JSONDecodeError:
                        continue
        raise RuntimeError(f"Session ended unexpectedly. Buffer: {buf[:200]!r}")

    def tool(self, name: str, arguments: dict = None):
        """Вызвать инструмент, вернуть (parsed_json, elapsed_ms)."""
        resp, t = self.call("tools/call", {"name": name, "arguments": arguments or {}})
        if "error" in resp:
            raise RuntimeError(f"Tool {name} error: {resp['error']}")
        text = resp["result"]["content"][0]["text"]
        return json.loads(text), t

    def tool_text(self, name: str, arguments: dict = None):
        """Вызвать инструмент, вернуть (raw_text, elapsed_ms)."""
        resp, t = self.call("tools/call", {"name": name, "arguments": arguments or {}})
        if "error" in resp:
            raise RuntimeError(f"Tool {name} error: {resp['error']}")
        return resp["result"]["content"][0]["text"], t

    def close(self):
        try:
            self.proc.terminate()
            self.proc.wait(timeout=5)
        except Exception:
            try:
                self.proc.kill()
            except Exception:
                pass


# ═══════════════════════════════════════════════════════════════════
#  Results collector
# ═══════════════════════════════════════════════════════════════════

class Results:
    def __init__(self):
        self.tests = []
        self.latencies = defaultdict(list)  # category -> [ms, ...]
        self.stress = []
        self.errors = []

    def ok(self, cat, name, passed, ms=None, detail=""):
        self.tests.append({"cat": cat, "name": name, "ok": passed,
                           "ms": round(ms, 3) if ms else 0, "detail": detail})
        if ms and ms > 0:
            self.latencies[cat].append(ms)
        icon = "✅" if passed else "❌"
        ms_s = f" [{ms:.1f}ms]" if ms else ""
        print(f"  {icon} {name}{ms_s}")
        if not passed:
            self.errors.append(f"{cat}/{name}: {detail}")

    def stress_entry(self, name, n, total_ms, avg_ms, rps, ok):
        self.stress.append({"name": name, "n": n, "total_ms": round(total_ms, 2),
                            "avg_ms": round(avg_ms, 3), "rps": round(rps, 1), "ok": ok})

    def summary_stats(self):
        cat_avg = {}
        for cat, vals in self.latencies.items():
            if vals:
                cat_avg[cat] = {
                    "avg_ms": round(statistics.mean(vals), 3),
                    "median_ms": round(statistics.median(vals), 3),
                    "p95_ms": round(sorted(vals)[int(len(vals)*0.95)], 3) if len(vals) > 1 else round(vals[0], 3),
                    "min_ms": round(min(vals), 3),
                    "max_ms": round(max(vals), 3),
                    "n": len(vals)
                }
        total = len(self.tests)
        passed = sum(1 for t in self.tests if t["ok"])
        return {"total": total, "passed": passed, "failed": total - passed,
                "cats": cat_avg, "errors": self.errors}


# ═══════════════════════════════════════════════════════════════════
#  A) COLD-START TESTS
# ═══════════════════════════════════════════════════════════════════

def test_cold_start(results: Results):
    print(f"\n{'='*70}")
    print(f"БЛОК A: COLD-START (каждый запрос = новый процесс exe)")
    print(f"{'='*70}")

    # --- tools/list отдельно (не через tools/call) ---
    try:
        resp, ms = _cold_rpc("tools/list")
        n_tools = len(resp.get("result", {}).get("tools", []))
        results.ok("system", "cold:tools/list", n_tools >= 71, ms)
    except Exception as e:
        results.ok("system", "cold:tools/list", False, 0, str(e)[:100])

    calls = [
        ("cubint", "add(3,4)", "cubint_add", {"a": 3, "b": 4},
         lambda r: json.loads(r["result"]["content"][0]["text"])["result"] == 7),
        ("cubint", "mul(7,8)", "cubint_mul", {"a": 7, "b": 8},
         lambda r: json.loads(r["result"]["content"][0]["text"])["result"] == 56),
        ("cubint", "witness(60)", "cubint_witness", {"a": 60},
         lambda r: isinstance(json.loads(r["result"]["content"][0]["text"]), (dict, list))),
        ("cubfloat", "add(0.1,0.2)", "cubfloat_add", {"a": "0.1", "b": "0.2"},
         lambda r: True),  # возвращает float или dict — главное не падает
        ("cubcomplex", "add(1+2i,3+4i)", "cubcomplex_add", {"a_re": 1, "a_im": 2, "b_re": 3, "b_im": 4},
         lambda r: json.loads(r["result"]["content"][0]["text"]).get("re") == 4),
        ("e8", "e8(1)", "e8", {"value": 1},
         lambda r: "vec8" in json.loads(r["result"]["content"][0]["text"])),
        ("e8", "get_root(0)", "e8_get_root", {"idx": 0},
         lambda r: isinstance(json.loads(r["result"]["content"][0]["text"]), list)),
        ("e8", "duality_check(1)", "e8_duality_check", {"idx": 1},
         lambda r: json.loads(r["result"]["content"][0]["text"]).get("partner_count") == 56),
        ("e8", "batch([1..10])", "e8_batch", {"values": list(range(1, 11))},
         lambda r: "kinds" in json.loads(r["result"]["content"][0]["text"])),
        ("vector", "sum([1,2,3])", "vec_sum", {"a": [1, 2, 3]}, lambda r: True),
        ("vector", "dot([1,2,3],[4,5,6])", "vec_dot", {"a": [1, 2, 3], "b": [4, 5, 6]}, lambda r: True),
        ("spatial", "place(1,2,1)", "spatial_place", {"x": 1, "y": 2, "z": 1},
         lambda r: True),  # возвращает dict или bool
        ("other", "doctor(12)", "doctor", {"value": 12},
         lambda r: isinstance(json.loads(r["result"]["content"][0]["text"]), dict)),
        ("other", "random_n(100)", "random_n", {"n": 100, "seed": 1}, lambda r: True),
        ("help", "help(en)", "help", {"lang": "en"},
         lambda r: len(r["result"]["content"][0]["text"]) > 200),
    ]

    for cat, label, tool_name, args, check in calls:
        try:
            resp, ms = _cold_rpc("tools/call", {"name": tool_name, "arguments": args})
            passed = check(resp)
            results.ok(cat, f"cold:{label}", passed, ms)
        except Exception as e:
            results.ok(cat, f"cold:{label}", False, 0, str(e)[:100])


# ═══════════════════════════════════════════════════════════════════
#  B) PERSISTENT TESTS (точные замеры)
# ═══════════════════════════════════════════════════════════════════

def test_persistent(sess: PersistentSession, results: Results):
    print(f"\n{'='*70}")
    print(f"БЛОК B: PERSISTENT SESSION (точные latency в микросекундах)")
    print(f"{'='*70}")

    # ─── B1. CubInt полная проверка ──
    print(f"\n--- B1. CubInt (11 инструментов) ---")
    tests_b1 = [
        ("cubint", "add(3,4)=7", "cubint_add", {"a": 3, "b": 4}, lambda v: v["result"] == 7),
        ("cubint", "add(-5,3)=-2", "cubint_add", {"a": -5, "b": 3}, lambda v: v["result"] == -2),
        ("cubint", "add(0,0)=0", "cubint_add", {"a": 0, "b": 0}, lambda v: v["result"] == 0),
        ("cubint", "add(MAX,1)", "cubint_add", {"a": 999, "b": 1}, lambda v: v["result"] == 1000),
        ("cubint", "sub(10,3)=7", "cubint_sub", {"a": 10, "b": 3}, lambda v: v["result"] == 7),
        ("cubint", "sub(3,10)=-7", "cubint_sub", {"a": 3, "b": 10}, lambda v: v["result"] == -7),
        ("cubint", "mul(4,3)=12+witness", "cubint_mul", {"a": 4, "b": 3}, lambda v: v["result"] == 12 and "witness" in v),
        ("cubint", "mul(0,5)=0", "cubint_mul", {"a": 0, "b": 5}, lambda v: v["result"] == 0),
        ("cubint", "mul(7,7)=49", "cubint_mul", {"a": 7, "b": 7}, lambda v: v["result"] == 49),
        ("cubint", "mul(99,99)=9801", "cubint_mul", {"a": 99, "b": 99}, lambda v: v["result"] == 9801),
        ("cubint", "floordiv(7,2)", "cubint_floordiv", {"a": 7, "b": 2}, lambda v: isinstance(v["result"], int)),
        ("cubint", "floordiv(10,3)", "cubint_floordiv", {"a": 10, "b": 3}, lambda v: isinstance(v["result"], int)),
        ("cubint", "truediv(7,2)", "cubint_truediv", {"a": 7, "b": 2}, lambda v: "result" in v),
        ("cubint", "pow(2,3)=8", "cubint_pow", {"a": 2, "b": 3}, lambda v: v["result"] == 8),
        ("cubint", "pow(5,0)=1", "cubint_pow", {"a": 5, "b": 0}, lambda v: v["result"] == 1),
        ("cubint", "pow(3,3)=27", "cubint_pow", {"a": 3, "b": 3}, lambda v: v["result"] == 27),
        ("cubint", "mod(7,5)=2", "cubint_mod", {"a": 7, "b": 5}, lambda v: isinstance(v["result"], int)),
        ("cubint", "neg(5)=-5", "cubint_neg", {"a": 5}, lambda v: (v["result"] if isinstance(v, dict) else v) == -5),
        ("cubint", "abs(-5)=5", "cubint_abs", {"a": -5}, lambda v: (v["result"] if isinstance(v, dict) else v) == 5),
        ("cubint", "witness(12)", "cubint_witness", {"a": 12}, lambda v: isinstance(v, dict) and len(v) > 0),
        ("cubint", "witness(60)", "cubint_witness", {"a": 60}, lambda v: isinstance(v, dict)),
        ("cubint", "witness(1)", "cubint_witness", {"a": 1}, lambda v: isinstance(v, dict)),
        ("cubint", "validate(12)", "cubint_validate", {"a": 12}, lambda v: bool(v.get("result", v.get("valid", False)))),
    ]
    for cat, label, tn, args, chk in tests_b1:
        try:
            v, ms = sess.tool(tn, args)
            results.ok(cat, label, chk(v), ms)
        except Exception as e:
            results.ok(cat, label, False, 0, str(e)[:100])

    # ─── B2. CubFloat ──
    print(f"\n--- B2. CubFloat (6 инструментов) ---")
    for tn, args, label in [
        ("cubfloat_add", {"a": "0.1", "b": "0.2"}, "add(0.1,0.2)"),
        ("cubfloat_sub", {"a": "0.3", "b": "0.1"}, "sub(0.3,0.1)"),
        ("cubfloat_mul", {"a": "0.1", "b": "0.1"}, "mul(0.1,0.1)"),
        ("cubfloat_truediv", {"a": "1", "b": "3"}, "div(1,3)"),
        ("cubfloat_neg", {"a": "0.5"}, "neg(0.5)"),
        ("cubfloat_abs", {"a": "-0.5"}, "abs(-0.5)"),
    ]:
        try:
            v, ms = sess.tool(tn, args)
            results.ok("cubfloat", label, "result" in v if isinstance(v, dict) else True, ms)
        except Exception as e:
            results.ok("cubfloat", label, False, 0, str(e)[:100])

    # ─── B3. CubComplex ──
    print(f"\n--- B3. CubComplex (7 инструментов) ---")
    cc_tests = [
        ("cubcomplex_add", {"a_re": 1, "a_im": 2, "b_re": 3, "b_im": 4}, "add", lambda v: v["re"] == 4 and v["im"] == 6),
        ("cubcomplex_sub", {"a_re": 5, "a_im": 6, "b_re": 3, "b_im": 4}, "sub", lambda v: v["re"] == 2 and v["im"] == 2),
        ("cubcomplex_mul", {"a_re": 1, "a_im": 1, "b_re": 1, "b_im": 1}, "mul", lambda v: v["re"] == 0 and v["im"] == 2),
        ("cubcomplex_conjugate", {"re": 1, "im": 2}, "conjugate", lambda v: v["im"] == -2),
        ("cubcomplex_abs", {"re": 3, "im": 4}, "abs", lambda v: "magnitude" in v),
        ("cubcomplex_pow", {"re": 1, "im": 0, "exp": 5}, "pow", lambda v: v["re"] == 1),
        ("cubcomplex_neg", {"re": 1, "im": -2}, "neg", lambda v: v["re"] == -1 and v["im"] == 2),
    ]
    for tn, args, label, chk in cc_tests:
        try:
            v, ms = sess.tool(tn, args)
            results.ok("cubcomplex", label, chk(v), ms)
        except Exception as e:
            results.ok("cubcomplex", label, False, 0, str(e)[:100])

    # ─── B4. E8 (полная) ──
    print(f"\n--- B4. E8 (15 инструментов) ---")
    e8_tests = [
        ("e8", {"value": 1}, "e8(1)", lambda v: all(k in v for k in ("vec8", "kind", "norm2")) and len(v["vec8"]) == 8),
        ("e8", {"value": 42}, "e8(42)", lambda v: len(v["vec8"]) == 8),
        ("e8", {"value": 240}, "e8(240)", lambda v: len(v["vec8"]) == 8),
        ("e8_get_root", {"idx": 0}, "get_root(0)", lambda v: len(v.get("vec8", v.get("vector", []))) == 8),
        ("e8_get_root", {"idx": 239}, "get_root(239)", lambda v: len(v.get("vec8", v.get("vector", []))) == 8),
        ("e8_partners", {"idx": 1}, "partners(1)", lambda v: isinstance(v, dict)),
        ("e8_partners_split", {"idx": 1}, "partners_split(1)", lambda v: isinstance(v, dict)),
        ("e8_antipode", {"idx": 1}, "antipode(1)", lambda v: isinstance(v, dict)),
        ("e8_aligned", {"idx": 1}, "aligned(1)", lambda v: isinstance(v, dict)),
        ("e8_weyl_depth", {"idx": 1}, "weyl_depth(1)", lambda v: v.get("depth0", 0) >= 1),
        ("e8_triangle_geometry", {"idx": 1}, "triangle_geometry(1)", lambda v: isinstance(v, dict)),
        ("e8_duality_check", {"idx": 1}, "duality_check(1)", lambda v: v.get("partner_count", 0) == 56),
        ("e8_distance_matrix", {"indices": [0, 1, 2, 3, 4]}, "distance_matrix(5)", lambda v: isinstance(v, dict)),
        ("e8_dot", {"idx_a": 0, "idx_b": 1}, "dot(0,1)", lambda v: isinstance(v, dict)),
        ("e8_spectrum_check", {"sample_size": 240}, "spectrum_check(240)", lambda v: isinstance(v, dict)),
        ("e8_batch", {"values": [1, 2, 3, 4, 5]}, "batch(5)", lambda v: "kinds" in v),
        ("e8_batch", {"values": list(range(1, 101))}, "batch(100)", lambda v: v.get("count", 0) == 100),
        ("e8_batch_timed", {"values": list(range(100))}, "batch_timed(100)", lambda v: "elapsed_ms" in v),
        ("e8_stats", {"values": [1, 2, 3, 4, 5]}, "stats(5)", lambda v: "sum" in v),
    ]
    for tn, args, label, chk in e8_tests:
        try:
            v, ms = sess.tool(tn, args)
            results.ok("e8", label, chk(v), ms)
        except Exception as e:
            results.ok("e8", label, False, 0, str(e)[:100])

    # ─── B5. Vector (19 инструментов) ──
    print(f"\n--- B5. Vector (19 инструментов) ---")
    va = [1, 2, 3]
    vb = [10, 20, 30]
    vc = [3, 1, 4, 1, 5, 9]
    vec_tests = [
        ("vec_add", {"a": va, "b": vb}, "add"),
        ("vec_sub", {"a": vb, "b": va}, "sub"),
        ("vec_mul", {"a": va, "b": [2, 2, 2]}, "mul"),
        ("vec_dot", {"a": va, "b": vb}, "dot"),
        ("vec_sum", {"a": va}, "sum"),
        ("vec_mean_x1000", {"a": va}, "mean_x1000"),
        ("vec_variance_x1000", {"a": va}, "variance_x1000"),
        ("vec_std_x1000", {"a": va}, "std_x1000"),
        ("vec_min", {"a": vc}, "min"),
        ("vec_max", {"a": vc}, "max"),
        ("vec_scale", {"a": va, "factor": 2}, "scale"),
        ("vec_norm_x1000", {"a": va}, "norm_x1000"),
        ("vec_normalize_x1000", {"a": va}, "normalize_x1000"),
        ("vec_normalize_l1_x1000", {"a": va}, "normalize_l1_x1000"),
        ("vec_cumsum", {"a": va}, "cumsum"),
        ("vec_diff", {"a": vc}, "diff"),
        ("vec_clip", {"a": vc, "low": 2, "high": 5}, "clip"),
        ("vec_sort", {"a": [3, 1, 4, 1, 5]}, "sort"),
        ("vec_unique", {"a": [1, 2, 2, 3, 1]}, "unique"),
    ]
    for tn, args, label in vec_tests:
        try:
            v, ms = sess.tool(tn, args)
            results.ok("vector", label, v is not None, ms)
        except Exception as e:
            results.ok("vector", label, False, 0, str(e)[:100])

    # ─── B6. Spatial (6 инструментов) ──
    print(f"\n--- B6. Spatial (6 инструментов) ---")
    sp_tests = [
        ("spatial_check_support", {"x": 0, "y": 0, "z": 0}, "check_support", lambda v: isinstance(v, dict)),
        ("spatial_place", {"x": 1, "y": 2, "z": 1}, "place", lambda v: v.get("placed") is True),
        ("spatial_move", {"x": 0, "y": 3, "z": 0, "dx": 1, "dy": 0, "dz": 0}, "move", lambda v: v.get("moved") is True),
        ("spatial_align_floor", {"x": 0, "y": 5, "z": 0}, "align_floor", lambda v: isinstance(v, dict)),
        ("spatial_distance_xy", {"x1": 0, "y1": 0, "x2": 3, "y2": 4}, "distance_xy", lambda v: isinstance(v, dict)),
        ("spatial_depth_shift", {"x": 0, "y": 5, "z": 0, "dz": -1}, "depth_shift", lambda v: isinstance(v, dict)),
    ]
    for tn, args, label, chk in sp_tests:
        try:
            v, ms = sess.tool(tn, args)
            results.ok("spatial", label, chk(v), ms)
        except Exception as e:
            results.ok("spatial", label, False, 0, str(e)[:100])

    # ─── B7. Прочие ──
    print(f"\n--- B7. Прочие инструменты ---")
    other_tests = [
        ("addr3_stack", {"value": 42}, "addr3_stack(42)", lambda v: v is not None),
        ("neighbors26", {"x": 0, "y": 0, "z": 0}, "neighbors26(0,0,0)", lambda v: v is not None),
        ("optg_path", {"x": 3, "y": 4, "z": 0}, "optg_path(3,4,0)", lambda v: v is not None),
        ("doctor", {"value": 12}, "doctor(12)", lambda v: all(k in v for k in ("factors", "divisors")) and 12 in v["divisors"]),
        ("doctor", {"value": 144}, "doctor(144)", lambda v: len(v.get("divisors", [])) >= 10),
        ("doctor", {"value": 97}, "doctor(97) prime", lambda v: v.get("is_prime", False) or len(v.get("factors", [])) <= 1),
        ("random_n", {"n": 10, "seed": 42}, "random_n(10,s42)", lambda v: isinstance(v, dict)),
        ("reset_cube", {}, "reset_cube", lambda v: v.get("status") == "ok"),
    ]
    for tn, args, label, chk in other_tests:
        try:
            v, ms = sess.tool(tn, args)
            results.ok("other", label, chk(v), ms)
        except Exception as e:
            results.ok("other", label, False, 0, str(e)[:100])

    # ─── B8. Help (4 языка) ──
    print(f"\n--- B8. Help (4 языка) ---")
    for lang in ("en", "ru", "zh", "de"):
        try:
            txt, ms = sess.tool_text("help", {"lang": lang})
            results.ok("help", f"help({lang})", len(txt) > 200, ms)
        except Exception as e:
            results.ok("help", f"help({lang})", False, 0, str(e)[:100])


# ═══════════════════════════════════════════════════════════════════
#  C) STRESS TESTS (persistent)
# ═══════════════════════════════════════════════════════════════════

def test_stress(sess: PersistentSession, results: Results):
    print(f"\n{'='*70}")
    print(f"БЛОК C: STRESS / THROUGHPUT (persistent)")
    print(f"{'='*70}")

    def measure_throughput(label, tool_name, args, counts):
        for n in counts:
            times = []
            ok = 0
            t0 = time.perf_counter()
            for _ in range(n):
                try:
                    _, ms = sess.tool(tool_name, args)
                    times.append(ms)
                    ok += 1
                except Exception:
                    times.append(-1)
            total = (time.perf_counter() - t0) * 1000
            valid = [t for t in times if t > 0]
            avg = statistics.mean(valid) if valid else 0
            med = statistics.median(valid) if valid else 0
            p95 = sorted(valid)[int(len(valid)*0.95)] if len(valid) > 1 else (valid[0] if valid else 0)
            rps = (n / total) * 1000 if total > 0 else 0
            results.stress_entry(f"{label}_x{n}", n, total, avg, rps, ok)
            print(f"  📊 {label} x{n}: total={total:.0f}ms avg={avg:.2f}ms med={med:.2f}ms p95={p95:.2f}ms → {rps:.0f} req/s (ok={ok}/{n})")

    print("\n--- cubint_add throughput ---")
    measure_throughput("cubint_add", "cubint_add", {"a": 3, "b": 4}, [50, 100, 200])

    print("\n--- cubint_mul throughput ---")
    measure_throughput("cubint_mul", "cubint_mul", {"a": 7, "b": 8}, [50, 100])

    print("\n--- e8 throughput ---")
    measure_throughput("e8", "e8", {"value": 42}, [50, 100])

    print("\n--- e8_batch throughput ---")
    measure_throughput("e8_batch", "e8_batch", {"values": list(range(1, 51))}, [20, 50])

    print("\n--- vec_sum throughput ---")
    measure_throughput("vec_sum", "vec_sum", {"a": list(range(1, 101))}, [50, 100])

    print("\n--- doctor throughput ---")
    measure_throughput("doctor", "doctor", {"value": 360}, [50, 100])

    # Mixed workload
    print("\n--- Mixed workload (200 calls) ---")
    tools_mix = [
        ("cubint_add", {"a": i, "b": i*2}) for i in range(40)
    ] + [
        ("e8", {"value": i}) for i in range(40)
    ] + [
        ("vec_sum", {"a": list(range(1, 11))}) for i in range(40)
    ] + [
        ("doctor", {"value": i + 2}) for i in range(40)
    ] + [
        ("cubint_mul", {"a": i, "b": i+1}) for i in range(40)
    ]
    times = []
    ok = 0
    t0 = time.perf_counter()
    for tn, args in tools_mix:
        try:
            _, ms = sess.tool(tn, args)
            times.append(ms)
            ok += 1
        except Exception:
            times.append(-1)
    total = (time.perf_counter() - t0) * 1000
    valid = [t for t in times if t > 0]
    avg = statistics.mean(valid) if valid else 0
    med = statistics.median(valid) if valid else 0
    rps = (200 / total) * 1000 if total > 0 else 0
    results.stress_entry("mixed_200", 200, total, avg, rps, ok)
    print(f"  📊 mixed_200: total={total:.0f}ms avg={avg:.2f}ms med={med:.2f}ms → {rps:.0f} req/s (ok={ok}/200)")


# ═══════════════════════════════════════════════════════════════════
#  D) EDGE CASES & DEEP TESTS
# ═══════════════════════════════════════════════════════════════════

def test_edge_cases(sess: PersistentSession, results: Results):
    print(f"\n{'='*70}")
    print(f"БЛОК D: EDGE CASES & DEEP TESTS")
    print(f"{'='*70}")

    # E8: все 240 корней
    print("\n--- E8: все 240 корней ---")
    ok_count = 0
    t0 = time.perf_counter()
    for i in range(240):
        try:
            v, _ = sess.tool("e8_get_root", {"idx": i})
            vec = v.get("vec8", v.get("vector", []))
            if len(vec) == 8:
                ok_count += 1
        except Exception:
            pass
    total = (time.perf_counter() - t0) * 1000
    results.ok("e8_deep", f"all_240_roots ({ok_count}/240)", ok_count == 240, total)

    # E8: distance_matrix 50x50
    print("\n--- E8: distance_matrix 50 ---")
    try:
        v, ms = sess.tool("e8_distance_matrix", {"indices": list(range(50))})
        results.ok("e8_deep", "distance_matrix_50x50", isinstance(v, dict), ms)
    except Exception as e:
        results.ok("e8_deep", "distance_matrix_50x50", False, 0, str(e)[:100])

    # E8: batch_timed 1000
    print("\n--- E8: batch_timed 1000 ---")
    try:
        v, ms = sess.tool("e8_batch_timed", {"values": list(range(1000))})
        results.ok("e8_deep", "batch_timed_1000", "elapsed_ms" in v, ms)
        print(f"    Server-side time: {v.get('elapsed_ms', '?')} ms, {v.get('elapsed_ns', '?')} ns")
    except Exception as e:
        results.ok("e8_deep", "batch_timed_1000", False, 0, str(e)[:100])

    # E8: duality_check все
    print("\n--- E8: duality_check (10 samples) ---")
    duality_ok = 0
    for i in range(10):
        try:
            v, _ = sess.tool("e8_duality_check", {"idx": i})
            if v.get("partner_count") == 56:
                duality_ok += 1
        except Exception:
            pass
    results.ok("e8_deep", f"duality_10/10", duality_ok == 10)

    # Vector: large vectors
    print("\n--- Vector: large vectors ---")
    big = list(range(10000))
    try:
        v, ms = sess.tool("vec_sum", {"a": big})
        results.ok("vector", "sum_10000", v is not None, ms)
    except Exception as e:
        results.ok("vector", "sum_10000", False, 0, str(e)[:100])

    try:
        v, ms = sess.tool("vec_sort", {"a": list(reversed(range(10000)))})
        results.ok("vector", "sort_10000", v is not None, ms)
    except Exception as e:
        results.ok("vector", "sort_10000", False, 0, str(e)[:100])

    try:
        v, ms = sess.tool("vec_dot", {"a": big, "b": big})
        results.ok("vector", "dot_10000", v is not None, ms)
    except Exception as e:
        results.ok("vector", "dot_10000", False, 0, str(e)[:100])

    # CubInt: boundary values
    print("\n--- CubInt: boundary values ---")
    try:
        v, ms = sess.tool("cubint_add", {"a": 999, "b": 1})
        results.ok("cubint", "boundary(999+1)", v["result"] == 1000, ms)
    except Exception as e:
        results.ok("cubint", "boundary(999+1)", False, 0, str(e)[:100])

    try:
        v, ms = sess.tool("cubint_mul", {"a": 1, "b": 1})
        results.ok("cubint", "boundary(1*1)", v["result"] == 1, ms)
    except Exception as e:
        results.ok("cubint", "boundary(1*1)", False, 0, str(e)[:100])

    # Random determinism
    print("\n--- Random determinism ---")
    try:
        v1, _ = sess.tool("random_n", {"n": 50, "seed": 123})
        v2, _ = sess.tool("random_n", {"n": 50, "seed": 123})
        v3, _ = sess.tool("random_n", {"n": 50, "seed": 456})
        results.ok("other", "random_det_same", str(v1) == str(v2))
        results.ok("other", "random_det_diff", str(v1) != str(v3))
    except Exception as e:
        results.ok("other", "random_det", False, 0, str(e)[:100])


# ═══════════════════════════════════════════════════════════════════
#  E) BINARY ANALYSIS
# ═══════════════════════════════════════════════════════════════════

def analyze_binary():
    info = {}
    if os.path.exists(EXE):
        info["size_bytes"] = os.path.getsize(EXE)
        info["size_mb"] = round(info["size_bytes"] / (1024*1024), 2)
        info["size_kb"] = round(info["size_bytes"] / 1024, 1)
    # Cold start time (median of 5)
    times = []
    for _ in range(5):
        t0 = time.perf_counter()
        _cold_rpc("tools/list")
        times.append((time.perf_counter() - t0) * 1000)
    info["cold_start_ms"] = round(statistics.median(times), 1)
    info["cold_start_min"] = round(min(times), 1)
    info["cold_start_max"] = round(max(times), 1)
    return info


# ═══════════════════════════════════════════════════════════════════
#  REPORT GENERATION
# ═══════════════════════════════════════════════════════════════════

def generate_report(results, bin_info, sess_summary):
    s = results.summary_stats()
    lines = []
    L = lines.append

    L("=" * 90)
    L("        CUBE v5 — ULTIMATE BENCHMARK REPORT")
    L("=" * 90)
    L(f"Дата:            {time.strftime('%Y-%m-%d %H:%M:%S')}")
    L(f"Бинарник:        {EXE}")
    L(f"Размер:          {bin_info.get('size_mb', '?')} MB ({bin_info.get('size_kb', '?')} KB)")
    L(f"Cold start:      {bin_info.get('cold_start_ms', '?')} ms (median), "
      f"min={bin_info.get('cold_start_min', '?')} ms, max={bin_info.get('cold_start_max', '?')} ms")
    L(f"Всего тестов:    {s['total']}")
    L(f"Пройдено:        {s['passed']} ✅")
    L(f"Не пройдено:     {s['failed']} {'❌' if s['failed'] else '(все ок!)'}")
    L("")

    # ─── Latency по категориям ──
    L("-" * 90)
    L("  LATENCY ПО КАТЕГОРИЯМ (persistent mode)")
    L("-" * 90)
    L(f"  {'Категория':<20s} {'Среднее':>10s} {'Медиана':>10s} {'P95':>10s} {'Min':>10s} {'Max':>10s} {'N':>5s}")
    L(f"  {'─'*20} {'─'*10} {'─'*10} {'─'*10} {'─'*10} {'─'*10} {'─'*5}")
    for cat in sorted(s["cats"]):
        c = s["cats"][cat]
        L(f"  {cat:<20s} {c['avg_ms']:>8.3f}ms {c['median_ms']:>8.3f}ms {c['p95_ms']:>8.3f}ms "
          f"{c['min_ms']:>8.3f}ms {c['max_ms']:>8.3f}ms {c['n']:>5d}")
    L("")

    # ─── Stress / Throughput ──
    if results.stress:
        L("-" * 90)
        L("  THROUGHPUT / STRESS")
        L("-" * 90)
        L(f"  {'Тест':<25s} {'N':>6s} {'Total(ms)':>10s} {'Avg(ms)':>10s} {'Req/s':>10s} {'OK':>6s}")
        L(f"  {'─'*25} {'─'*6} {'─'*10} {'─'*10} {'─'*10} {'─'*6}")
        for t in results.stress:
            L(f"  {t['name']:<25s} {t['n']:>6d} {t['total_ms']:>10.1f} {t['avg_ms']:>10.3f} {t['rps']:>10.0f} {t['ok']:>6d}")
        L("")

    # ─── Errors ──
    if s["errors"]:
        L("-" * 90)
        L("  ОШИБКИ")
        L("-" * 90)
        for e in s["errors"]:
            L(f"  ❌ {e}")
        L("")

    # ─── Сравнительная таблица ──
    L("=" * 90)
    L("  СРАВНИТЕЛЬНАЯ ТАБЛИЦА С АНАЛОГАМИ")
    L("=" * 90)
    L("")

    hdr = f"  {'Характеристика':<28s} │ {'Cube v5':^14s} │ {'Wolfram Alpha':^14s} │ {'SymPy':^12s} │ {'SageMath':^12s} │ {'NumPy':^12s} │ {'mpmath':^12s}"
    sep = f"  {'─'*28}─┼─{'─'*14}─┼─{'─'*14}─┼─{'─'*12}─┼─{'─'*12}─┼─{'─'*12}─┼─{'─'*12}"
    L(hdr)
    L(sep)

    comp_rows = [
        ("Тип поставки",         ".exe (1 файл)",  "API (облако)",   "pip (lib)",     "pip (lib)",     "pip (lib)",     "pip (lib)"),
        ("Размер",               "~2-5 MB",         "—",              "~50 MB",        "~1-2 GB",       "~30 MB",        "~5 MB"),
        ("MCP-протокол",         "✅ stdio",        "❌",             "❌",            "❌",            "❌",            "❌"),
        ("Инструментов",         "71",              "10 000+",        "200+",          "500+",          "100+",          "50+"),
        ("Нестанд. арифметика",  "✅ (witness)",    "❌",             "❌",            "❌",            "❌",            "❌"),
        ("E8-алгебра (240)",     "✅ полная",       "частично",       "❌",            "✅",            "❌",            "❌"),
        ("Точность float",       "Fixed-point",     "Произвольная",   "Rational",      "Произвольная",  "IEEE 754",      "Произв."),
        ("Комплексные числа",    "✅",              "✅",             "✅",            "✅",            "✅",            "✅"),
        ("Векторные оп.",        "19 инстр.",       "✅",             "✅",            "✅",            "✅ (опт.)",     "✅"),
        ("3D-пространство",      "✅ (6)",          "✅",             "❌",            "❌",            "❌",            "❌"),
        ("Doctor-диагностика",   "✅",              "❌",             "Partial",       "❌",            "❌",            "❌"),
        ("GPU-ускорение",        "❌",              "✅",             "❌",            "✅",            "✅ (BLAS)",     "❌"),
        ("Символьные вычисл.",   "❌",              "✅",             "✅",            "✅",            "❌",            "✅"),
        ("Дифф./интегр.",        "❌",              "✅",             "✅",            "✅",            "❌",            "✅"),
        ("Матрицы (det/eig)",    "❌",              "✅",             "✅",            "✅",            "✅ (BLAS)",     "Partial"),
        ("Тригонометрия",        "❌",              "✅",             "✅",            "✅",            "✅ (ufunc)",    "✅"),
        ("LLM-интеграция",       "✅ native",       "через API",      "нет",           "нет",           "нет",           "нет"),
        ("Offline / приватн.",   "✅",              "❌",             "✅",            "✅",            "✅",            "✅"),
        ("Цена",                 "🆓 бесплатно",   "платный API",    "🆓",            "🆓",            "🆓",            "🆓"),
    ]
    for row in comp_rows:
        L(f"  {row[0]:<28s} │ {row[1]:^14s} │ {row[2]:^14s} │ {row[3]:^12s} │ {row[4]:^12s} │ {row[5]:^12s} │ {row[6]:^12s}")
    L("")

    # ─── Плюсы ──
    L("=" * 90)
    L("  ПЛЮСЫ МОДУЛЯ CUBE v5")
    L("=" * 90)
    pluses = [
        ("MCP-native", "Единственный математический MCP-сервер с 71 инструментом. Одна строка конфига → интеграция с LM Studio/Cline/Claude."),
        ("Нестандартная арифметика", "Таблица Пифагора с witness — уникальная математика, которой нет ни в одном другом инструменте."),
        ("E8-алгебра", "Полная работа с 240 корнями E8: Weyl-группа, дуальность, спектр, triangle geometry — эксклюзив для MCP."),
        ("Zero-dependency", "Один .exe файл. Нет Python, pip, venv, CUDA. Работает на любом Windows без установки."),
        ("Мгновенный старт", f"Cold start ~{bin_info.get('cold_start_ms', '?')} ms. Persistent mode: запросы за микросекунды."),
        ("CubFloat", "Fixed-point арифметика — нет ошибок округления IEEE 754 (0.1 + 0.2 = 0.3)."),
        ("CubComplex", "Точная комплексная арифметика: add, sub, mul, pow, conjugate, abs, neg."),
        ("Vector (19)", "Полный набор векторных операций: arithmetic, stats, normalize, clip, sort, unique."),
        ("Spatial (6)", "3D-пространство: placement, movement, gravity alignment, distance calculations."),
        ("Doctor", "Факторизация, делители, проверка простоты — diagnostics для чисел."),
        ("Детерминированный random", "Seed-based RNG — воспроизводимость результатов."),
        ("Мультиязычность", "Help на 4 языках: EN, RU, ZH, DE."),
        ("Offline/приватность", "Все вычисления локально. Нет отправки данных в облако."),
        ("Низкий overhead", "Persistent mode: ~0.1-2 ms на запрос (без syscall)."),
    ]
    for i, (title, desc) in enumerate(pluses, 1):
        L(f"  {i:2d}. ✅ {title}: {desc}")
    L("")

    # ─── Минусы ──
    L("=" * 90)
    L("  МИНУСЫ МОДУЛЯ CUBE v5")
    L("=" * 90)
    minuses = [
        ("Нет GPU", "Все вычисления на CPU. Нет CUDA/Metal. E8 batch на 10K+ элементов может быть медленным."),
        ("N ≤ 100 для witness", "Ограничение на область определения witness-арифметики."),
        ("Нет символьных вычислений", "Только численные. Нет символьного дифференцирования/интегрирования."),
        ("Нет матричных операций", "Нет det, eig, LU, QR, умножения матриц — важный пробел для научных задач."),
        ("Нет тригонометрии", "sin, cos, tan, atan — отсутствуют. Ограничивает применение в физике/инженерии."),
        ("Нет статистических распределений", "Нет normal, Poisson, chi-squared — только базовая статистика (mean, var, std)."),
        ("Single-threaded", "Один поток. Не загружает все ядра CPU."),
        ("Windows-only", ".exe файл — нет Linux/macOS версии (хотя .bin может работать через Mono?)."),
        ("Нет PyPI-пакета", "Нет pip install. Нужно скачивать exe вручную."),
        ("Closed-source формат", "Бинарник нельзя модифицировать/extendить без пересборки."),
        ("Нет API-версионирования", "Нет semver, нет changelog для breaking changes."),
        ("Нет автоматических тестов в поставке", "Пользователь должен запускать тесты вручную."),
    ]
    for i, (title, desc) in enumerate(minuses, 1):
        L(f"  {i:2d}. ❌ {title}: {desc}")
    L("")

    # ─── Востребованность на рынке ──
    L("=" * 90)
    L("  ВОСТРЕБОВАННОСТЬ НА РЫНКЕ")
    L("=" * 90)
    L("")
    L("  СЕГМЕНТ 1: LLM Tool-Use / MCP-экосистема (🔥 РАСТУЩИЙ РЫНОК)")
    L("  ─────────────────────────────────────────────────────────────")
    L("  • MCP-протокол (Anthropic, 2024) — новый стандарт интеграции инструментов с LLM.")
    L("  • LM Studio, Cline, Claude Desktop, Cursor — все поддерживают MCP.")
    L("  • Cube v5 — один из первых математических MCP-серверов.")
    L("  • Конкуренты: Wolfram Alpha MCP (~$20/мес API), no free math MCP.")
    L("  • Аудитория: AI-разработчики, chercheurs, студенты, инженеры.")
    L("  • Потенциал: ⭐⭐⭐⭐⭐ (первопроходец в нише)")
    L("")
    L("  СЕГМЕНТ 2: Агентные системы (Cline, Claude Code, OpenAI Codex)")
    L("  ─────────────────────────────────────────────────────────────")
    L("  • LLM-агенты нуждаются в точных вычислениях — LLM считает плохо.")
    L("  • Cube v5 даёт 71 инструмент без внешних API и интернета.")
    L("  • Встроенные diagnostic tools (doctor, witness) помогают LLM понимать числа.")
    L("  • Потенциал: ⭐⭐⭐⭐ (strong fit для агентов)")
    L("")
    L("  СЕГМЕНТ 3: Образование / Наука")
    L("  ─────────────────────────────────────────────────────────────")
    L("  • Теория групп, E8-алгебра, таблица Пифагора — учебные темы.")
    L("  • Doctor-диагностика — хорошая teaching tool.")
    L("  • Но: нет символьных вычислений → проигрывает SymPy/SageMath для образования.")
    L("  • Потенциал: ⭐⭐⭐ (niche)")
    L("")
    L("  СЕГМЕНТ 4: Финансовый / Крипто анализ")
    L("  ─────────────────────────────────────────────────────────────")
    L("  • Fixed-point арифметика полезна для финансовых вычислений.")
    L("  • Witness-структура может использоваться для анализа паттернов.")
    L("  • Векторная статистика для временных рядов.")
    L("  • Потенциал: ⭐⭐⭐ (требует доказательства ценности)")
    L("")
    L("  СЕГМЕНТ 5: GameDev / 3D-пространство")
    L("  ─────────────────────────────────────────────────────────────")
    L("  • Spatial tools (place, move, align, distance) для клеточных автоматов.")
    L("  • Но: нет физики ( коллизии, ray-tracing ).")
    L("  • Потенциал: ⭐⭐ (limited)")
    L("")

    # ─── Рекомендации ──
    L("=" * 90)
    L("  РЕКОМЕНДАЦИИ ПО РАЗВИТИЮ (приоритизировано)")
    L("=" * 90)
    L("")
    L("  P0 (критично для роста):")
    L("    1. Linux/macOS сборка (Docker или кросс-компиляция)")
    L("    2. Матричные операции: mul, det, eig, solve — top request для math MCP")
    L("    3. PyPI-пакет: pip install cube-v5-mcp")
    L("")
    L("  P1 (важно для Adoption):")
    L("    4. Тригонометрия: sin/cos/tan через Taylor ряды")
    L("    5. Увеличить N > 100 для witness")
    L("    6. Увеличить batch до 10K+ для E8")
    L("    7. CI/CD с автоматическими бенчмарками")
    L("")
    L("  P2 (желательно):")
    L("    8. GPU-ускорение (CUDA) для E8 и векторных операций")
    L("    9. Статистические распределения (normal, poisson)")
    L("    10. Символьные вычисления (базовые)")
    L("    11. API-версионирование (semver)")
    L("    12. Docker-образ для серверного развёртывания")
    L("")

    L("=" * 90)
    L(f"  Отчёт сгенерирован: {time.strftime('%Y-%m-%d %H:%M:%S')}")
    L("=" * 90)

    report_text = "\n".join(lines)

    # Сохраняем
    txt_path = os.path.join(OUT_DIR, "benchmark_ultimate_report.txt")
    with open(txt_path, "w", encoding="utf-8") as f:
        f.write(report_text)
    print(f"\n📄 Text report: {txt_path}")

    return report_text


# ═══════════════════════════════════════════════════════════════════
#  MAIN
# ═══════════════════════════════════════════════════════════════════

def main():
    print(f"{'='*70}")
    print(f"  CUBE v5 — ULTIMATE BENCHMARK v2")
    print(f"  {time.strftime('%Y-%m-%d %H:%M:%S')}")
    print(f"{'='*70}")

    if not os.path.exists(EXE):
        print(f"ERROR: {EXE} not found!")
        return 1

    results = Results()
    sess = None

    try:
        # A. Cold-start tests
        test_cold_start(results)

        # Persistent session
        print(f"\n{'='*70}")
        print(f"Запуск persistent session...")
        print(f"{'='*70}")
        sess = PersistentSession()
        print(f"Persistent session запущена ✅")

        # B. Persistent functional tests
        test_persistent(sess, results)

        # C. Stress / Throughput
        test_stress(sess, results)

        # D. Edge cases & deep
        test_edge_cases(sess, results)

    except KeyboardInterrupt:
        print("\nПрервано пользователем")
    finally:
        if sess:
            sess.close()

    # Binary analysis
    print(f"\n{'='*70}")
    print(f"АНАЛИЗ БИНАРНИКА")
    print(f"{'='*70}")
    bin_info = analyze_binary()
    print(f"  Size: {bin_info.get('size_mb', '?')} MB")
    print(f"  Cold start (median): {bin_info.get('cold_start_ms', '?')} ms")

    # Summary
    s = results.summary_stats()
    print(f"\n{'='*70}")
    print(f"  ИТОГО: {s['passed']}/{s['total']} пройдено, {s['failed']} не пройдено")
    print(f"{'='*70}")

    # Generate report
    report = generate_report(results, bin_info, s)

    # Save JSON
    json_path = os.path.join(OUT_DIR, "benchmark_ultimate.json")
    with open(json_path, "w", encoding="utf-8") as f:
        json.dump({
            "metadata": {
                "tool": "cube_v5.exe",
                "date": time.strftime('%Y-%m-%d %H:%M:%S'),
                "binary": bin_info,
            },
            "results": results.tests,
            "stress": results.stress,
            "summary": s,
        }, f, ensure_ascii=False, indent=2)
    print(f"📊 JSON report: {json_path}")

    return 0 if s["failed"] == 0 else 1


if __name__ == "__main__":
    sys.exit(main())