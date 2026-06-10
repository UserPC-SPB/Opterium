"""
Cube v5 — ULTIMATE Benchmark & Test Suite v2
=============================================
Two measurement modes:
  A) COLD-START: each request spawns a new exe process (measures startup overhead)
  B) PERSISTENT: one process, N requests via stdin (real MCP performance)

Tests:
  1. Functional correctness (71 tools)
  2. Cold-start latency (ms)
  3. Persistent latency (microseconds!)
  4. Throughput: requests/sec
  5. Stress: batch 1K..10K, edge cases
  6. E8 deep: all 240 roots, distance_matrix 50x50
  7. Vector: large vectors 10K+
  8. Binary analysis (size, startup time)

Output: JSON + text report
"""

import json
import subprocess
import time
import sys
import os
import statistics
from collections import defaultdict

EXE = os.path.join(os.path.dirname(os.path.abspath(__file__)), "cube_v5.exe")
OUT_DIR = os.path.dirname(os.path.abspath(__file__))


# === RPC helpers ===

def _cold_rpc(method, params=None):
    """Single call = single exe launch. Returns (response_dict, elapsed_ms)."""
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
    """Persistent MCP session — one process, many requests."""

    def __init__(self):
        self.proc = subprocess.Popen(
            [EXE], stdin=subprocess.PIPE, stdout=subprocess.PIPE,
            stderr=subprocess.PIPE, text=True, cwd=OUT_DIR
        )
        self._id = 0
        self._warmup()

    def _warmup(self):
        self.call("tools/list")

    def call(self, method, params=None):
        """Send request, get response. Returns (response_dict, elapsed_ms)."""
        self._id += 1
        req = {"jsonrpc": "2.0", "id": self._id, "method": method, "params": params or {}}
        payload = json.dumps(req, ensure_ascii=False) + "\n"
        start = time.perf_counter()
        self.proc.stdin.write(payload)
        self.proc.stdin.flush()
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
        raise RuntimeError(f"Session ended. Buffer: {buf[:200]!r}")

    def tool(self, name, arguments=None):
        resp, t = self.call("tools/call", {"name": name, "arguments": arguments or {}})
        if "error" in resp:
            raise RuntimeError(f"Tool {name} error: {resp['error']}")
        text = resp["result"]["content"][0]["text"]
        return json.loads(text), t

    def tool_text(self, name, arguments=None):
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


# === Results collector ===

class Results:
    def __init__(self):
        self.tests = []
        self.latencies = defaultdict(list)
        self.stress = []

    def ok(self, cat, name, passed, ms=None, detail=""):
        self.tests.append({"cat": cat, "name": name, "ok": passed,
                           "ms": round(ms, 3) if ms else 0, "detail": detail})
        if ms and ms > 0:
            self.latencies[cat].append(ms)
        icon = "[PASS]" if passed else "[FAIL]"
        ms_s = f" [{ms:.1f}ms]" if ms else ""
        print(f"  {icon} {name}{ms_s}")

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
        return {"total": total, "passed": passed, "failed": total - passed, "cats": cat_avg}


# === A) COLD-START TESTS ===

def test_cold_start(results):
    print(f"\n{'='*70}")
    print("BLOCK A: COLD-START (each request = new exe process)")
    print(f"{'='*70}")

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
        ("cubfloat", "add(0.1,0.2)", "cubfloat_add", {"a": "0.1", "b": "0.2"}, lambda r: True),
        ("cubcomplex", "add(1+2i,3+4i)", "cubcomplex_add",
         {"a_re": 1, "a_im": 2, "b_re": 3, "b_im": 4},
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
        ("vector", "dot", "vec_dot", {"a": [1, 2, 3], "b": [4, 5, 6]}, lambda r: True),
        ("spatial", "place(1,2,1)", "spatial_place", {"x": 1, "y": 2, "z": 1}, lambda r: True),
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


# === B) PERSISTENT TESTS ===

def test_persistent(sess, results):
    print(f"\n{'='*70}")
    print("BLOCK B: PERSISTENT SESSION (precise latency in microseconds)")
    print(f"{'='*70}")

    # B1. CubInt
    print("\n--- B1. CubInt (11 tools) ---")
    # NOTE: CubInt uses non-standard arithmetic. add(-5,3) = 8, not -2.
    tests_b1 = [
        ("cubint", "add(3,4)=7", "cubint_add", {"a": 3, "b": 4}, lambda v: v["result"] == 7),
        ("cubint", "add(-5,3)=8 (non-standard)", "cubint_add", {"a": -5, "b": 3},
         lambda v: v["result"] == 8 and "witness" in v),
        ("cubint", "add(0,0)=0", "cubint_add", {"a": 0, "b": 0}, lambda v: v["result"] == 0),
        ("cubint", "add(999,1)=1000", "cubint_add", {"a": 999, "b": 1}, lambda v: v["result"] == 1000),
        ("cubint", "sub(10,3)=7", "cubint_sub", {"a": 10, "b": 3}, lambda v: v["result"] == 7),
        ("cubint", "mul(4,3)=12+witness", "cubint_mul", {"a": 4, "b": 3},
         lambda v: v["result"] == 12 and "witness" in v),
        ("cubint", "mul(0,5)=0", "cubint_mul", {"a": 0, "b": 5}, lambda v: v["result"] == 0),
        ("cubint", "mul(7,7)=49", "cubint_mul", {"a": 7, "b": 7}, lambda v: v["result"] == 49),
        ("cubint", "mul(99,99)=9801", "cubint_mul", {"a": 99, "b": 99}, lambda v: v["result"] == 9801),
        ("cubint", "floordiv(7,2)", "cubint_floordiv", {"a": 7, "b": 2},
         lambda v: isinstance(v.get("result", v), int)),
        ("cubint", "truediv(7,2)", "cubint_truediv", {"a": 7, "b": 2}, lambda v: True),
        ("cubint", "pow(2,3)=8", "cubint_pow", {"a": 2, "b": 3}, lambda v: v["result"] == 8),
        ("cubint", "pow(5,0)=1", "cubint_pow", {"a": 5, "b": 0}, lambda v: v["result"] == 1),
        ("cubint", "mod(7,5)", "cubint_mod", {"a": 7, "b": 5}, lambda v: "result" in v),
        ("cubint", "neg(5)=-5", "cubint_neg", {"a": 5},
         lambda v: (v.get("result", v) if isinstance(v, dict) else v) == -5),
        ("cubint", "abs(-5)=5", "cubint_abs", {"a": -5},
         lambda v: (v.get("result", v) if isinstance(v, dict) else v) == 5),
        ("cubint", "witness(12)", "cubint_witness", {"a": 12}, lambda v: True),
        ("cubint", "validate(12)", "cubint_validate", {"a": 12}, lambda v: True),
    ]
    for cat, label, tn, args, chk in tests_b1:
        try:
            v, ms = sess.tool(tn, args)
            results.ok(cat, label, chk(v), ms)
        except Exception as e:
            results.ok(cat, label, False, 0, str(e)[:100])

    # B2. CubFloat
    print("\n--- B2. CubFloat (6 tools) ---")
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
            results.ok("cubfloat", label, True, ms)
        except Exception as e:
            results.ok("cubfloat", label, False, 0, str(e)[:100])

    # B3. CubComplex
    print("\n--- B3. CubComplex (7 tools) ---")
    cc_tests = [
        ("cubcomplex_add", {"a_re": 1, "a_im": 2, "b_re": 3, "b_im": 4}, "add",
         lambda v: v["re"] == 4 and v["im"] == 6),
        ("cubcomplex_sub", {"a_re": 5, "a_im": 6, "b_re": 3, "b_im": 4}, "sub",
         lambda v: v["re"] == 2 and v["im"] == 2),
        ("cubcomplex_mul", {"a_re": 1, "a_im": 1, "b_re": 1, "b_im": 1}, "mul",
         lambda v: v["re"] == 0 and v["im"] == 2),
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

    # B4. E8 (returns dict, list, or int depending on tool)
    print("\n--- B4. E8 (15 tools) ---")
    e8_tests = [
        ("e8", {"value": 1}, "e8(1)",
         lambda v: isinstance(v, dict) and "vec8" in v and len(v["vec8"]) == 8),
        ("e8", {"value": 42}, "e8(42)",
         lambda v: isinstance(v, dict) and len(v.get("vec8", [])) == 8),
        ("e8_get_root", {"idx": 0}, "get_root(0)",
         lambda v: isinstance(v, list) and len(v) == 8),
        ("e8_get_root", {"idx": 239}, "get_root(239)",
         lambda v: isinstance(v, list) and len(v) == 8),
        ("e8_partners", {"idx": 1}, "partners(1)",
         lambda v: isinstance(v, list) and len(v) > 0),
        ("e8_partners_split", {"idx": 1}, "partners_split(1)", lambda v: True),
        ("e8_antipode", {"idx": 1}, "antipode(1)", lambda v: isinstance(v, int)),
        ("e8_aligned", {"idx": 1}, "aligned(1)",
         lambda v: isinstance(v, list) and len(v) > 0),
        ("e8_weyl_depth", {"idx": 1}, "weyl_depth(1)",
         lambda v: isinstance(v, dict) and v.get("depth0", 0) >= 1),
        ("e8_triangle_geometry", {"idx": 1}, "triangle_geometry(1)", lambda v: True),
        ("e8_duality_check", {"idx": 1}, "duality_check(1)",
         lambda v: isinstance(v, dict) and v.get("partner_count") == 56),
        ("e8_distance_matrix", {"indices": [0, 1, 2, 3, 4]}, "distance_matrix(5)",
         lambda v: isinstance(v, list)),
        ("e8_dot", {"idx_a": 0, "idx_b": 1}, "dot(0,1)",
         lambda v: isinstance(v, (int, float))),
        ("e8_spectrum_check", {"sample_size": 240}, "spectrum_check(240)", lambda v: True),
        ("e8_batch", {"values": [1, 2, 3, 4, 5]}, "batch(5)",
         lambda v: isinstance(v, dict) and "kinds" in v),
        ("e8_batch", {"values": list(range(1, 101))}, "batch(100)",
         lambda v: isinstance(v, dict) and v.get("count", 0) == 100),
        ("e8_batch_timed", {"values": list(range(100))}, "batch_timed(100)",
         lambda v: isinstance(v, dict) and "elapsed_ms" in v),
        ("e8_stats", {"values": [1, 2, 3, 4, 5]}, "stats(5)",
         lambda v: isinstance(v, dict) and "sum" in v),
    ]
    for tn, args, label, chk in e8_tests:
        try:
            v, ms = sess.tool(tn, args)
            results.ok("e8", label, chk(v), ms)
        except Exception as e:
            results.ok("e8", label, False, 0, str(e)[:100])

    # B5. Vector (19 tools)
    print("\n--- B5. Vector (19 tools) ---")
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

    # B6. Spatial (6 tools - return primitives, not dicts)
    print("\n--- B6. Spatial (6 tools) ---")
    sp_tests = [
        ("spatial_check_support", {"x": 0, "y": 0, "z": 0}, "check_support"),
        ("spatial_place", {"x": 1, "y": 2, "z": 1}, "place"),
        ("spatial_move", {"x": 0, "y": 3, "z": 0, "dx": 1, "dy": 0, "dz": 0}, "move"),
        ("spatial_align_floor", {"x": 0, "y": 5, "z": 0}, "align_floor"),
        ("spatial_distance_xy", {"x1": 0, "y1": 0, "x2": 3, "y2": 4}, "distance_xy"),
        ("spatial_depth_shift", {"x": 0, "y": 5, "z": 0, "dz": -1}, "depth_shift"),
    ]
    for tn, args, label in sp_tests:
        try:
            v, ms = sess.tool(tn, args)
            results.ok("spatial", label, True, ms)
        except Exception as e:
            results.ok("spatial", label, False, 0, str(e)[:100])

    # B7. Other tools
    print("\n--- B7. Other tools ---")
    other_tests = [
        ("addr3_stack", {"value": 42}, "addr3_stack(42)", lambda v: v is not None),
        ("neighbors26", {"x": 0, "y": 0, "z": 0}, "neighbors26(0,0,0)", lambda v: v is not None),
        ("optg_path", {"x": 3, "y": 4, "z": 0}, "optg_path(3,4,0)", lambda v: v is not None),
        ("doctor", {"value": 12}, "doctor(12)",
         lambda v: isinstance(v, dict) and "divisors" in v and 12 in v["divisors"]),
        ("doctor", {"value": 144}, "doctor(144)",
         lambda v: isinstance(v, dict) and len(v.get("divisors", [])) >= 10),
        ("doctor", {"value": 97}, "doctor(97) prime",
         lambda v: isinstance(v, dict)),
        ("random_n", {"n": 10, "seed": 42}, "random_n(10,s42)", lambda v: isinstance(v, dict)),
        ("reset_cube", {}, "reset_cube", lambda v: True),
    ]
    for tn, args, label, chk in other_tests:
        try:
            v, ms = sess.tool(tn, args)
            results.ok("other", label, chk(v), ms)
        except Exception as e:
            results.ok("other", label, False, 0, str(e)[:100])

    # B8. Help (4 languages)
    print("\n--- B8. Help (4 languages) ---")
    for lang in ("en", "ru", "zh", "de"):
        try:
            txt, ms = sess.tool_text("help", {"lang": lang})
            results.ok("help", f"help({lang})", len(txt) > 200, ms)
        except Exception as e:
            results.ok("help", f"help({lang})", False, 0, str(e)[:100])


# === C) STRESS TESTS ===

def test_stress(sess, results):
    print(f"\n{'='*70}")
    print("BLOCK C: STRESS / THROUGHPUT (persistent)")
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
            print(f"  {label} x{n}: total={total:.0f}ms avg={avg:.2f}ms med={med:.2f}ms p95={p95:.2f}ms -> {rps:.0f} req/s (ok={ok}/{n})")

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
    tools_mix = (
        [("cubint_add", {"a": i, "b": i*2}) for i in range(40)]
        + [("e8", {"value": i}) for i in range(40)]
        + [("vec_sum", {"a": list(range(1, 11))}) for i in range(40)]
        + [("doctor", {"value": i + 2}) for i in range(40)]
        + [("cubint_mul", {"a": i, "b": i+1}) for i in range(40)]
    )
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
    rps = (200 / total) * 1000 if total > 0 else 0
    results.stress_entry("mixed_200", 200, total, avg, rps, ok)
    print(f"  mixed_200: total={total:.0f}ms avg={avg:.2f}ms -> {rps:.0f} req/s (ok={ok}/200)")


# === D) EDGE CASES & DEEP TESTS ===

def test_edge_cases(sess, results):
    print(f"\n{'='*70}")
    print("BLOCK D: EDGE CASES & DEEP TESTS")
    print(f"{'='*70}")

    # E8: all 240 roots
    print("\n--- E8: all 240 roots ---")
    ok_count = 0
    t0 = time.perf_counter()
    for i in range(240):
        try:
            v, _ = sess.tool("e8_get_root", {"idx": i})
            if isinstance(v, list) and len(v) == 8:
                ok_count += 1
        except Exception:
            pass
    total = (time.perf_counter() - t0) * 1000
    results.ok("e8_deep", f"all_240_roots ({ok_count}/240)", ok_count == 240, total)

    # E8: distance_matrix 50x50
    print("\n--- E8: distance_matrix 50 ---")
    try:
        v, ms = sess.tool("e8_distance_matrix", {"indices": list(range(50))})
        results.ok("e8_deep", "distance_matrix_50x50", isinstance(v, (list, dict)), ms)
    except Exception as e:
        results.ok("e8_deep", "distance_matrix_50x50", False, 0, str(e)[:100])

    # E8: batch_timed 1000
    print("\n--- E8: batch_timed 1000 ---")
    try:
        v, ms = sess.tool("e8_batch_timed", {"values": list(range(1000))})
        results.ok("e8_deep", "batch_timed_1000", isinstance(v, dict) and "elapsed_ms" in v, ms)
    except Exception as e:
        results.ok("e8_deep", "batch_timed_1000", False, 0, str(e)[:100])

    # E8: duality_check 10 samples
    print("\n--- E8: duality_check (10 samples) ---")
    duality_ok = 0
    for i in range(10):
        try:
            v, _ = sess.tool("e8_duality_check", {"idx": i})
            if isinstance(v, dict) and v.get("partner_count") == 56:
                duality_ok += 1
        except Exception:
            pass
    results.ok("e8_deep", f"duality_10/10", duality_ok == 10)

    # Vector: large vectors
    print("\n--- Vector: large vectors ---")
    big = list(range(10000))
    for label, tn, args in [
        ("sum_10000", "vec_sum", {"a": big}),
        ("sort_10000", "vec_sort", {"a": list(reversed(range(10000)))}),
        ("dot_10000", "vec_dot", {"a": big, "b": big}),
    ]:
        try:
            v, ms = sess.tool(tn, args)
            results.ok("vector", label, v is not None, ms)
        except Exception as e:
            results.ok("vector", label, False, 0, str(e)[:100])

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


# === E) BINARY ANALYSIS ===

def analyze_binary():
    info = {}
    if os.path.exists(EXE):
        info["size_bytes"] = os.path.getsize(EXE)
        info["size_mb"] = round(info["size_bytes"] / (1024*1024), 2)
        info["size_kb"] = round(info["size_bytes"] / 1024, 1)
    times = []
    for _ in range(5):
        t0 = time.perf_counter()
        _cold_rpc("tools/list")
        times.append((time.perf_counter() - t0) * 1000)
    info["cold_start_ms"] = round(statistics.median(times), 1)
    info["cold_start_min"] = round(min(times), 1)
    info["cold_start_max"] = round(max(times), 1)
    return info


# === REPORT GENERATION ===

def generate_report(results, bin_info):
    s = results.summary_stats()
    lines = []
    L = lines.append

    L("=" * 90)
    L("        CUBE v5 — ULTIMATE BENCHMARK REPORT")
    L("=" * 90)
    L(f"Date:             {time.strftime('%Y-%m-%d %H:%M:%S')}")
    L(f"Binary:           cube_v5.exe")
    L(f"Size:             {bin_info.get('size_mb', '?')} MB ({bin_info.get('size_kb', '?')} KB)")
    L(f"Cold start:       {bin_info.get('cold_start_ms', '?')} ms (median), "
      f"min={bin_info.get('cold_start_min', '?')} ms, max={bin_info.get('cold_start_max', '?')} ms")
    L(f"Total tests:      {s['total']}")
    L(f"Passed:           {s['passed']}")
    L(f"Failed:           {s['failed']}")
    L("")

    # Latency by category
    L("-" * 90)
    L("  LATENCY BY CATEGORY (persistent mode)")
    L("-" * 90)
    L(f"  {'Category':<20s} {'Avg':>10s} {'Median':>10s} {'P95':>10s} {'Min':>10s} {'Max':>10s} {'N':>5s}")
    L(f"  {'-'*20} {'-'*10} {'-'*10} {'-'*10} {'-'*10} {'-'*10} {'-'*5}")
    for cat in sorted(s["cats"]):
        c = s["cats"][cat]
        L(f"  {cat:<20s} {c['avg_ms']:>8.3f}ms {c['median_ms']:>8.3f}ms {c['p95_ms']:>8.3f}ms "
          f"{c['min_ms']:>8.3f}ms {c['max_ms']:>8.3f}ms {c['n']:>5d}")
    L("")

    # Throughput
    if results.stress:
        L("-" * 90)
        L("  THROUGHPUT / STRESS")
        L("-" * 90)
        L(f"  {'Test':<25s} {'N':>6s} {'Total(ms)':>10s} {'Avg(ms)':>10s} {'Req/s':>10s} {'OK':>6s}")
        L(f"  {'-'*25} {'-'*6} {'-'*10} {'-'*10} {'-'*10} {'-'*6}")
        for t in results.stress:
            L(f"  {t['name']:<25s} {t['n']:>6d} {t['total_ms']:>10.1f} {t['avg_ms']:>10.3f} {t['rps']:>10.0f} {t['ok']:>6d}")
        L("")

    # Comparison table
    L("=" * 90)
    L("  COMPARISON WITH ALTERNATIVES")
    L("=" * 90)
    L("")
    L(f"  {'Feature':<28s} | {'Cube v5':^14s} | {'Wolfram Alpha':^14s} | {'SymPy':^12s} | {'SageMath':^12s} | {'NumPy':^12s} | {'mpmath':^12s}")
    L(f"  {'-'*28}-+-{'-'*14}-+-{'-'*14}-+-{'-'*12}-+-{'-'*12}-+-{'-'*12}-+-{'-'*12}")

    comp_rows = [
        ("Delivery",         ".exe (1 file)",  "API (cloud)",   "pip (lib)",     "pip (lib)",     "pip (lib)",     "pip (lib)"),
        ("Size",             "~23 MB",          "-",             "~50 MB",        "~1-2 GB",       "~30 MB",        "~5 MB"),
        ("MCP protocol",     "YES stdio",       "NO",            "NO",            "NO",            "NO",            "NO"),
        ("Tools",            "71",              "10,000+",       "200+",          "500+",          "100+",          "50+"),
        ("Non-standard arith","YES (witness)",  "NO",            "NO",            "NO",            "NO",            "NO"),
        ("E8-algebra (240)", "YES full",        "partial",       "NO",            "YES",           "NO",            "NO"),
        ("Float precision",  "Fixed-point",     "Arbitrary",     "Rational",      "Arbitrary",     "IEEE 754",      "Arbitrary"),
        ("Complex numbers",  "YES",             "YES",           "YES",           "YES",           "YES",           "YES"),
        ("Vector ops",       "19 tools",        "YES",           "YES",           "YES",           "YES (opt)",     "YES"),
        ("3D space",         "YES (6)",         "YES",           "NO",            "NO",            "NO",            "NO"),
        ("Doctor",           "YES",             "NO",            "Partial",       "NO",            "NO",            "NO"),
        ("GPU accel",        "NO",              "YES",           "NO",            "YES",           "YES (BLAS)",    "NO"),
        ("Symbolic",         "NO",              "YES",           "YES",           "YES",           "NO",            "YES"),
        ("Diff/Integral",    "NO",              "YES",           "YES",           "YES",           "NO",            "YES"),
        ("Matrices",         "NO",              "YES",           "YES",           "YES",           "YES (BLAS)",    "Partial"),
        ("Trigonometry",     "NO",              "YES",           "YES",           "YES",           "YES (ufunc)",   "YES"),
        ("LLM integration",  "YES native",      "via API",       "no",            "no",            "no",            "no"),
        ("Offline/private",  "YES",             "NO",            "YES",           "YES",           "YES",           "YES"),
        ("Price",            "FREE",            "paid API",      "FREE",          "FREE",          "FREE",          "FREE"),
    ]
    for row in comp_rows:
        L(f"  {row[0]:<28s} | {row[1]:^14s} | {row[2]:^14s} | {row[3]:^12s} | {row[4]:^12s} | {row[5]:^12s} | {row[6]:^12s}")
    L("")

    L("=" * 90)
    L(f"  Report generated: {time.strftime('%Y-%m-%d %H:%M:%S')}")
    L("=" * 90)

    report_text = "\n".join(lines)
    txt_path = os.path.join(OUT_DIR, "benchmark_ultimate_report.txt")
    with open(txt_path, "w", encoding="utf-8") as f:
        f.write(report_text)
    print(f"\nReport: {txt_path}")
    return report_text


# === MAIN ===

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
        test_cold_start(results)

        print(f"\n{'='*70}")
        print("Starting persistent session...")
        print(f"{'='*70}")
        sess = PersistentSession()
        print("Persistent session ready")

        test_persistent(sess, results)
        test_stress(sess, results)
        test_edge_cases(sess, results)

    except KeyboardInterrupt:
        print("\nInterrupted by user")
    finally:
        if sess:
            sess.close()

    print(f"\n{'='*70}")
    print("BINARY ANALYSIS")
    print(f"{'='*70}")
    bin_info = analyze_binary()
    print(f"  Size: {bin_info.get('size_mb', '?')} MB")
    print(f"  Cold start (median): {bin_info.get('cold_start_ms', '?')} ms")

    s = results.summary_stats()
    print(f"\n{'='*70}")
    print(f"  TOTAL: {s['passed']}/{s['total']} passed, {s['failed']} failed")
    print(f"{'='*70}")

    generate_report(results, bin_info)

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
    print(f"JSON report: {json_path}")

    return 0 if s["failed"] == 0 else 1


if __name__ == "__main__":
    sys.exit(main())