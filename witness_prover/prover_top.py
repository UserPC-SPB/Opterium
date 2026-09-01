# =====================================================================
# witness_prover: machine-checkable proof-by-witness certificates
# for the Opterium model (Borisov's speculative geometry).
#
#   python prover_top.py --list
#   python prover_top.py --run-all
#   python prover_top.py --cert T-e8-spectrum
#   python prover_top.py --cert T-decimal --json
#
# What this is NOT:
#   - NOT a Lean-style proof term. Lean creates proofs by deduction inside
#     a formal system with axioms; if an axiom is wrong, Lean still
#     produces "valid" derivations (garbage in, garbage out).
#   - NOT a runtime self-assertion ("verified: true" from the engine).
#
# What this IS:
#   A PROOF-BY-WITNESS certificate. Each theorem statement is accompanied
#   by concrete witnesses (independent routes that reach the same address)
#   and by a KERNEL that *recomputes* the claim from first principles.
#   The certificate only certifies what the kernel actually re-derived
#   independently of the engine.  Verdict = "CERTIFIED" only if every
#   witness route AND the kernel re-computation agree exactly (tau=0).
#
# Because the kernel recomputes from definitions (not from server output),
# the certificate is checkable offline, by any third party, with a handful
# of standard-library routines.  In that sense the certificate is stronger
# than a Lean proof term: it carries reproducible computation, not a
# chain of accepted axioms.
#
# =====================================================================
"""Emit proof-by-witness certificates for Opterium invariants.

Zero-dependency (Python 3 stdlib only). The MCP native engine is OPTIONAL:
a probe session through `mcp_server.exe` is used to *report* the engine's
own witness, but the certificate verdict is decided by `prover_kernel.py`
recomputing from first principles.
"""
import argparse
import hashlib
import json
import os
import sys
import time

# ---------------------------------------------------------------------
# Discovery: locate cube_v5 native engine if present (optional witness).
# ---------------------------------------------------------------------
CUBE_DEF_PATHS = [
    r"D:\cube_v5_native\mcp_server\mcp_server.exe",
    r"C:\cube_v5_native\mcp_server\mcp_server.exe",
]

DISCLAIMER = (
    "This certificate implements Borisov's speculative geometry \"Opterium\"."
    " It is an experimental, unorthodox framework: computations are "
    "self-consistent under its own axioms and verified tables, but it is "
    "NOT established peer-reviewed mathematics. Treat certificates as "
    "internal to this system."
)


def find_engine():
    for p in CUBE_DEF_PATHS:
        if os.path.isfile(p):
            return p
    env = os.environ.get("CUBE_V5_EXE")
    if env and os.path.isfile(env):
        return env
    return None


def sha256(obj) -> str:
    """Stable canonical hash of a JSON-serialisable object."""
    if isinstance(obj, str):
        raw = obj.encode("utf-8")
    else:
        raw = json.dumps(obj, sort_keys=True, ensure_ascii=False).encode("utf-8")
    return hashlib.sha256(raw).hexdigest()


def load_theorems():
    """Import theorem descriptors and their kernel checkers."""
    from prover_kernel import make_theorem_kernels
    return make_theorem_kernels()


# ---------------------------------------------------------------------
# Optional live witness from the native engine (best-effort, never fatal).
# ---------------------------------------------------------------------
def _try_engine_calls(exe, calls):
    """calls: list of (tool_name, arguments). Returns list of texts."""
    if not exe:
        return []
    import queue as _q
    import subprocess
    import threading

    try:
        proc = subprocess.Popen(
            [exe], cwd=os.path.dirname(exe),
            stdin=subprocess.PIPE, stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL, bufsize=0,
            creationflags=getattr(subprocess, "CREATE_NO_WINDOW", 0),
        )
    except Exception:
        return []
    q = _q.Queue()

    def reader():
        for line in iter(proc.stdout.readline, b""):
            q.put(line)
        q.put(None)
    threading.Thread(target=reader, daemon=True).start()

    def one(name, args, rid):
        req = {"jsonrpc": "2.0", "id": rid, "method": "tools/call",
               "params": {"name": name, "arguments": args or {}}}
        proc.stdin.write((json.dumps(req) + "\n").encode())
        proc.stdin.flush()
        deadline = time.time() + 30
        while time.time() < deadline:
            try:
                raw = q.get(timeout=0.5)
            except Exception:
                continue
            if raw is None:
                return None
            line = raw.decode(errors="replace").strip()
            if not line:
                continue
            try:
                msg = json.loads(line)
            except Exception:
                continue
            if msg.get("id") == rid:
                if "error" in msg:
                    return msg["error"]
                res = msg.get("result", {})
                return "".join(c.get("text", "") for c in res.get("content", [])
                               if isinstance(c, dict) and c.get("type") == "text")
        return None

    out = []
    for i, (name, args) in enumerate(calls):
        try:
            out.append(one(name, args, i + 1000))
        except Exception:
            out.append(None)
    try:
        proc.terminate()
    except Exception:
        pass
    return out


# ---------------------------------------------------------------------
# Certificate assembly.
# ---------------------------------------------------------------------
def make_cert(theorem, engine_exe=None):
    """Build a certificate dict for one theorem descriptor.

    theorem: {id, statement, claims, routes:[{id, desc, compute}], kernel}
    """
    routes = []
    for r in theorem["routes"]:
        try:
            res, detail = r["compute"]()
        except Exception as ex:
            res, detail = False, f"route raised {type(ex).__name__}: {ex}"
        routes.append({"id": r["id"], "desc": r["desc"],
                       "witness": bool(res), "detail": detail,
                       "route_hash": sha256(detail)})
    # kernel recomputation from first principles
    try:
        kres, kdetail = theorem["kernel"]()
    except Exception as ex:
        kres, kdetail = False, f"kernel raised {type(ex).__name__}: {ex}"

    ok_routes = sum(1 for r in routes if r["witness"])
    closed = ok_routes == len(routes)  # all independent routes agree
    tautau = 0 if (closed and kres) else 1
    verdict = "CERTIFIED" if tautau == 0 else "UNRESOLVED"

    # optional live witness from native engine (informational only)
    live = None
    if engine_exe and theorem.get("engine_probe"):
        live = _try_engine_calls(engine_exe, theorem["engine_probe"])

    cert = {
        "schema": "opterium-witness-certificate-v1",
        "certificate": theorem["id"],
        "statement": theorem["statement"],
        "claims": theorem.get("claims", []),
        "method": (
            "proof-by-witness: independent routes must converge on the same "
            "address; kernel recomputes from first principles; tau=0 required."
        ),
        "routes": routes,
        "kernel_recompute": {"witness": kres, "detail": kdetail},
        "closure": {
            "routes_total": len(routes),
            "routes_closed": ok_routes,
            "kernel_ok": kres,
            "tau": tautau,
            "verdict": verdict,
        },
        "engine_live_witness": live,
        "disclaimer": DISCLAIMER,
        "generated_utc": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
    }
    cert["certificate_hash"] = sha256({
        "statement": theorem["statement"],
        "claims": theorem.get("claims", []),
        "routes": [r["detail"] for r in routes],
        "kernel": kdetail,
    })
    return cert


# ---------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------
def main(argv=None):
    ap = argparse.ArgumentParser(
        description="Opterium proof-by-witness certificate generator")
    ap.add_argument("--list", action="store_true", help="list available theorems")
    ap.add_argument("--cert", metavar="ID", help="emit one certificate")
    ap.add_argument("--run-all", action="store_true",
                    help="emit certificates for every theorem into certificates/")
    ap.add_argument("--json", action="store_true", help="raw JSON output")
    ap.add_argument("--no-engine", action="store_true",
                    help="skip optional live engine witness")
    ap.add_argument("--out", metavar="DIR", default="certificates",
                    help="output directory for --run-all / --cert")
    args = ap.parse_args(argv)

    theorems = load_theorems()
    by_id = {t["id"]: t for t in theorems}
    engine = None if args.no_engine else find_engine()

    if args.list or not (args.cert or args.run_all):
        print(f"engine: {engine or 'NOT FOUND (kernel-only mode)'}")
        for t in theorems:
            print(f"  {t['id']:24s} {t['statement']}")
        if args.list:
            return 0
        print("\nuse --cert <ID>  or  --run-all")
        return 0

    if args.cert:
        if args.cert not in by_id:
            sys.exit(f"unknown theorem id: {args.cert}")
        cert = make_cert(by_id[args.cert], engine)
        if args.json:
            print(json.dumps(cert, ensure_ascii=False, indent=2))
        else:
            print_cert_human(cert)
        return 0

    if args.run_all:
        os.makedirs(args.out, exist_ok=True)
        summary = []
        for t in theorems:
            cert = make_cert(t, engine)
            path = os.path.join(args.out, cert["certificate"] + ".cert.json")
            with open(path, "w", encoding="utf-8") as f:
                json.dump(cert, f, ensure_ascii=False, indent=2)
            summary.append({
                "certificate": cert["certificate"],
                "verdict": cert["closure"]["verdict"],
                "tau": cert["closure"]["tau"],
                "routes": f'{cert["closure"]["routes_closed"]}/{cert["closure"]["routes_total"]}',
                "file": path,
            })
            print_cert_human(cert)
        print("\n=== SUMMARY ===")
        for s in summary:
            print(f"  {s['certificate']:22s} tau={s['tau']} "
                  f"routes={s['routes']} verdict={s['verdict']}  -> {s['file']}")
        ok = all(s["verdict"] == "CERTIFIED" for s in summary)
        print("ALL CERTIFIED" if ok else "SOME UNRESOLVED")
        return 0 if ok else 1


def print_cert_human(cert):
    c = cert["closure"]
    print("=" * 72)
    print(f"CERT  {cert['certificate']}")
    print(f"  {cert['statement']}")
    for cl in cert["claims"]:
        print(f"    - {cl}")
    for r in cert["routes"]:
        mark = "CLOSED" if r["witness"] else "OPEN  "
        print(f"  route {r['id']:<5s} [{mark}] {r['desc']}")
    k = cert["kernel_recompute"]
    mark = "CLOSED" if k["witness"] else "OPEN  "
    print(f"  kernel  [{mark}] {k['detail']}")
    if cert.get("engine_live_witness"):
        print("  engine live witness:", cert["engine_live_witness"])
    print(f"  closure: routes {c['routes_closed']}/{c['routes_total']} "
          f"kernel={c['kernel_ok']} tau={c['tau']} => {c['verdict']}")
    print(f"  hash: {cert['certificate_hash'][:16]}...")
    print("=" * 72)


if __name__ == "__main__":
    sys.exit(main())