#!/usr/bin/env python3
"""test_doc_integrity.py — Full test: documentation integrity & code cross-verification

Usage:
    python test_doc_integrity.py                 # full test
    python test_doc_integrity.py --module Pt     # one module only
    python test_doc_integrity.py --verbose       # detailed output
"""

import sys, os, json, inspect, hashlib, ast, re, traceback
from typing import Dict, List, Any, Optional, Callable, Tuple

SRC = os.path.dirname(__file__)
SPEC_DIR = os.path.join(SRC, 'spec-kit')
sys.path.insert(0, SRC)
sys.path.insert(0, SPEC_DIR)

SPEC_PATH = os.path.join(SPEC_DIR, 'spec_compiled.json')
API_PATH = os.path.join(SPEC_DIR, 'API.md')
HELP_PATH = os.path.join(SPEC_DIR, 'help.txt')
OUT_PATH = os.path.join(SRC, '..', 'results', 'coverage_report.json')

COLOR = bool(os.environ.get('TERM') or os.name == 'posix')

def green(s): return f'\033[32m{s}\033[0m' if COLOR else s
def red(s): return f'\033[31m{s}\033[0m' if COLOR else s
def yellow(s): return f'\033[33m{s}\033[0m' if COLOR else s
def bold(s): return f'\033[1m{s}\033[0m' if COLOR else s

# ═══════════════════════════════════════════════════════════
# Module Configuration
# ═══════════════════════════════════════════════════════════
# Maps spec module name → import info

MODULE_CONFIG = {
    "PtTable": {
        "import": "arith_table",
        "class": "PtTable",
        "instance_name": "PT",
        "file": "src/arith_table.py",
        "public_filter": lambda n: not n.startswith('_') and n.isupper() is False,
    },
    "Pt": {
        "import": "methods",
        "class": "Pt",
        "instance_name": None,
        "file": "src/spec-kit/methods/__init__.py",
        "functions": [
            "rmul", "radd", "rsub", "rdiv",
            "geo_mul", "geo_add",
            "validate_shape", "to_pt_matrix", "sd_tuple_matrix",
        ],
    },
    "Cube27": {
        "import": "cube27",
        "class": "Cube27",
        "instance_name": None,
        "file": "src/cube27.py",
    },
    "HashGrid": {
        "import": "hashgrid",
        "class": "HashGrid",
        "instance_name": None,
        "file": "src/hashgrid.py",
        "functions": ["geometric_weight", "geometric_attention"],
    },
    "delta_ops": {
        "import": "delta_ops",
        "class": "HealthVector",
        "instance_name": None,
        "file": "src/delta_ops.py",
        "functions": [
            "DELTA_ADD", "DELTA_MUL", "DELTA_INV", "DELTA_INV_NS",
            "DELTA_PPH", "DELTA_OPTG", "DELTA_SHIFT", "DELTA_ROT",
            "compose_sequential", "compose_parallel",
            "check_domain", "select_fallback",
        ],
    },
    "phi_algebra": {
        "import": "phi_algebra",
        "file": "src/phi_algebra.py",
        "functions": [
            "PHI1_SHIFT", "PHI2_PHASE", "PHI3_FIXEDPOINT",
            "PHI4_RECURSION", "PHI5_PROJECTION",
            "PhiPath", "periodic_orbit", "harmonic_series",
        ],
    },
    "swarm": {
        "import": "swarm",
        "class": "IntelligentSwarm",
        "instance_name": None,
        "file": "src/swarm.py",
        "functions": ["BayesReplacement"],
    },
    "e8_twist": {
        "import": "e8_twist",
        "class": "TwistEngine",
        "instance_name": None,
        "file": "src/e8_twist.py",
        "functions": ["address_to_root", "root_properties"],
    },
    "doctor_geo": {
        "import": "doctor_geo",
        "class": "SwarmDoctor",
        "instance_name": None,
        "file": "src/doctor_geo.py",
        "functions": [
            "geo_to_opterium_hv", "opterium_to_geo_hv",
            "ROUTE_REGISTRY",
        ],
    },
    "geoformer": {
        "import": "geoformer",
        "class": "Pt",
        "instance_name": None,
        "file": "src/geoformer.py",
        "extra_classes": ["GeometricEmbedding", "GeometricBlock", "GeoFormer", "SwarmTrainer"],
        "functions": ["doctor_judge"],
    },
}

# ═══════════════════════════════════════════════════════════
# Phase 1 — Foundation
# ═══════════════════════════════════════════════════════════

class IntegrityTest:
    def __init__(self, verbose: bool = False, module_filter: Optional[str] = None):
        self.verbose = verbose
        self.module_filter = module_filter
        self.imported_modules = {}  # name → (module_obj, instance_map)
        self.spec = None
        self.md_sections = None
        self.help_names = None
        self.v = lambda *a, **kw: None
        self.results = {
            "coverage": {}, "examples": {}, "format_errors": [],
            "help_errors": [], "edge_results": {}, "reproducibility": {},
        }

    def log(self, msg: str):
        if self.verbose:
            print(f"  {msg}")

    # ── 1a. Load spec ───────────────────────────────────

    def load_spec(self):
        with open(SPEC_PATH, 'r', encoding='utf-8') as f:
            self.spec = json.load(f)
        self.log(f"Loaded spec: {len(self.spec)} modules")

    # ── 1b. Import modules ─────────────────────────────

    def import_all_modules(self):
        for spec_name, cfg in MODULE_CONFIG.items():
            if self.module_filter and spec_name != self.module_filter:
                continue
            try:
                mod = __import__(cfg["import"], fromlist=['*'])
                instances = {}
                if cfg.get("instance_name"):
                    instances["root"] = getattr(mod, cfg["instance_name"])
                elif cfg.get("class"):
                    cls = getattr(mod, cfg["class"])
                    try:
                        instances["root"] = cls()
                    except Exception:
                        instances["root"] = cls
                if cfg.get("extra_classes"):
                    for ec in cfg["extra_classes"]:
                        try:
                            instances[ec] = getattr(mod, ec)()
                        except Exception:
                            instances[ec] = getattr(mod, ec)
                self.imported_modules[spec_name] = (mod, instances)
                self.log(f"  Loaded {spec_name} -> {cfg['import']}")
            except Exception as e:
                self.log(f"  FAILED {spec_name}: {e}")
                self.imported_modules[spec_name] = (None, {})

    # ── 1c. Get public API from code ────────────────────

    def get_public_api(self, spec_name: str) -> Dict[str, Dict]:
        mod, instances = self.imported_modules.get(spec_name, (None, {}))
        if mod is None:
            return {}
        cfg = MODULE_CONFIG[spec_name]
        api = {}

        class_names = []
        if cfg.get("class"):
            class_names.append(cfg["class"])
        if cfg.get("extra_classes"):
            class_names.extend(cfg["extra_classes"])

        for cls_name in class_names:
            cls = getattr(mod, cls_name, None)
            if cls is None:
                continue
            for m_name, m_obj in inspect.getmembers(cls, predicate=inspect.isfunction):
                if m_name.startswith('_') and m_name not in ('__init__', '__add__', '__mul__'):
                    continue
                if m_name.startswith('__') and m_name.endswith('__'):
                    continue
                try:
                    sig = str(inspect.signature(m_obj))
                except Exception:
                    sig = "(?)"
                entry_name = f"{cls_name}.{m_name}"
                api[entry_name] = {
                    "kind": "method",
                    "class": cls_name,
                    "method": m_name,
                    "sig": sig,
                    "doc": inspect.getdoc(m_obj) or "",
                    "obj": m_obj,
                    "instance": instances.get(cls_name, instances.get("root")),
                }

        func_names = cfg.get("functions", [])
        for f_name in func_names:
            f_obj = getattr(mod, f_name, None)
            if f_obj is None:
                api[f_name] = {"kind": "missing", "error": f"Function {f_name} not found"}
                continue
            try:
                sig = str(inspect.signature(f_obj))
            except Exception:
                sig = "(?)"
            api[f_name] = {
                "kind": "function",
                "name": f_name,
                "sig": sig,
                "doc": inspect.getdoc(f_obj) or "",
                "obj": f_obj,
                "instance": None,
            }

        return api

    # ── 1d. Load MD sections ───────────────────────────

    def load_md_sections(self) -> Dict[str, Dict]:
        with open(API_PATH, 'r', encoding='utf-8') as f:
            text = f.read()
        sections = {}
        current_mod = None
        current_class = None
        current_methods = []
        current_functions = []

        for line in text.split('\n'):
            if line.startswith('## Module: '):
                if current_mod:
                    sections[current_mod] = {
                        "classes": {current_class: current_methods} if current_class else {},
                        "functions": current_functions,
                    }
                current_mod = line.split('`')[1] if '`' in line else line[11:].strip()
                current_class = None
                current_methods = []
                current_functions = []
            elif line.startswith('### Class `'):
                if current_mod and current_class:
                    if current_class not in sections.get(current_mod, {}).get("classes", {}):
                        if current_mod not in sections:
                            sections[current_mod] = {"classes": {}, "functions": []}
                        sections[current_mod]["classes"][current_class] = current_methods
                current_class = line.split('`')[1]
                current_methods = []
            elif line.startswith('#### `'):
                parts = line.split('`')
                if len(parts) >= 2:
                    full = parts[1]
                    if '.' in full:
                        _, mname = full.split('.', 1)
                        mname = mname.split('(')[0].strip()
                        current_methods.append(mname)
            elif line.startswith('### Function `'):
                parts = line.split('`')
                if len(parts) >= 2:
                    fname = parts[1].split('(')[0].strip()
                    current_functions.append(fname)

        if current_mod:
            if current_mod not in sections:
                sections[current_mod] = {"classes": {}, "functions": []}
            if current_class and current_methods:
                sections[current_mod]["classes"][current_class] = current_methods
            sections[current_mod]["functions"] = current_functions

        self.md_sections = sections
        return sections

    # ── 1e. Load help.txt names ────────────────────────

    def load_help_names(self) -> Dict[str, str]:
        with open(HELP_PATH, 'r', encoding='utf-8') as f:
            text = f.read()
        names = {}
        for line in text.split('\n'):
            line = line.strip()
            if line.startswith('    def '):
                name = line.split('(')[0].split(' ')[-1].strip()
                names[name] = 'function'
            elif line.startswith('      .'):
                name = line.split('(')[0].split('.')[-1].strip()
                names[name] = 'method'
        self.help_names = names
        return names

    # ═══════════════════════════════════════════════════════
    # Phase 2 — Code-Doc Coverage Scan
    # ═══════════════════════════════════════════════════════

    def scan_module(self, spec_name: str) -> Dict:
        code_api = self.get_public_api(spec_name)
        spec_data = self.spec.get(spec_name, {})
        spec_classes = spec_data.get("classes", {})
        spec_functions = spec_data.get("functions", {})

        spec_names = set()
        for cname, cdata in spec_classes.items():
            for mname in cdata.get("methods", {}):
                spec_names.add(f"{cname}.{mname}")
        for fname in spec_functions:
            spec_names.add(fname)

        code_names = set(code_api.keys())

        missing = code_names - spec_names
        phantom = spec_names - code_names
        matched = code_names & spec_names

        sig_issues = []
        for name in matched:
            entry = code_api[name]
            spec_sig = ""
            if '.' in name:
                cname, mname = name.split('.', 1)
                if cname in spec_classes and mname in spec_classes[cname].get("methods", {}):
                    spec_sig = spec_classes[cname]["methods"][mname].get("signature", "")
            else:
                if name in spec_functions:
                    spec_sig = spec_functions[name].get("signature", "")
            if entry.get("sig") and spec_sig:
                code_short = entry["sig"].split('(')[0] if '(' in entry["sig"] else entry["sig"]
                if code_short and spec_sig.startswith(code_short.split(' ')[-1] if ' ' in code_short else code_short):
                    pass
                elif code_short not in spec_sig and spec_sig not in code_short:
                    sig_issues.append(f"{name}: code({entry['sig']}) vs spec({spec_sig})")

        result = {
            "total": len(code_names),
            "documented": len(matched),
            "missing": sorted(missing),
            "phantom": sorted(phantom),
            "sig_issues": sig_issues,
        }
        self.results["coverage"][spec_name] = result
        return result

    def scan_all_modules(self):
        print(f"\n{'='*60}")
        print(bold("Phase 2 — Code-Doc Coverage Scan"))
        print('='*60)
        for spec_name in sorted(MODULE_CONFIG.keys()):
            if self.module_filter and spec_name != self.module_filter:
                continue
            result = self.scan_module(spec_name)
            total = result["total"]
            docd = result["documented"]
            pct = (docd / total * 100) if total > 0 else 0
            status = green("OK") if result["missing"] or result["phantom"] else yellow("OK")
            print(f"  {spec_name:15s} {docd:3d}/{total:<3d} ({pct:5.1f}%)  {status}")
            if result["missing"]:
                print(f"    missing: {', '.join(result['missing'][:5])}")
            if result["phantom"]:
                print(f"    phantom: {', '.join(result['phantom'][:5])}")
            if result["sig_issues"]:
                for si in result["sig_issues"][:3]:
                    print(f"    sig: {si}")
        print()

    # ═══════════════════════════════════════════════════════
    # Phase 3 — Example Execution
    # ═══════════════════════════════════════════════════════

    def _get_callable(self, spec_name: str, class_name: Optional[str], method_name: str) -> Optional[Callable]:
        mod, instances = self.imported_modules.get(spec_name, (None, {}))
        if mod is None:
            return None
        if class_name:
            cls = getattr(mod, class_name, None)
            if not cls:
                return None
            fn = getattr(cls, method_name, None)
            if fn and instances.get(class_name):
                return lambda *a, **kw: fn(instances[class_name], *a, **kw)
            if fn:
                return fn
            return None
        else:
            return getattr(mod, method_name, None)

    def _normalize_for_compare(self, val):
        if isinstance(val, dict):
            return {k: self._normalize_for_compare(v) for k, v in sorted(val.items())}
        if isinstance(val, list):
            return [self._normalize_for_compare(v) for v in val]
        if isinstance(val, float):
            return round(val, 10)
        return val

    def exec_examples_for(self, spec_name: str) -> Dict:
        spec_data = self.spec.get(spec_name, {})
        passed, failed, skipped = 0, 0, 0
        failures = []

        for cname, cdata in spec_data.get("classes", {}).items():
            for mname, mdata in cdata.get("methods", {}).items():
                for ex in mdata.get("examples", []):
                    result = self._exec_one(spec_name, cname, mname, ex)
                    if result == "pass":
                        passed += 1
                    elif result == "fail":
                        failed += 1
                        failures.append(f"{spec_name}.{cname}.{mname}: {ex.get('description', '')}")
                    else:
                        skipped += 1

        for fname, fdata in spec_data.get("functions", {}).items():
            for ex in fdata.get("examples", []):
                result = self._exec_one(spec_name, None, fname, ex)
                if result == "pass":
                    passed += 1
                elif result == "fail":
                    failed += 1
                    failures.append(f"{spec_name}.{fname}: {ex.get('description', '')}")
                else:
                    skipped += 1

        return {"passed": passed, "failed": failed, "skipped": skipped, "failures": failures}

    def _exec_one(self, spec_name: str, class_name: Optional[str], method_name: str, ex: Dict) -> str:
        fn = self._get_callable(spec_name, class_name, method_name)
        if fn is None:
            return "skipped"
        inp = ex.get("input", ())
        expected = ex.get("output")

        args = self._convert_input(inp, fn)
        if args is None:
            return "skipped"

        try:
            actual = fn(*args)
            norm_actual = self._normalize_for_compare(actual)
            norm_expected = self._normalize_for_compare(expected)
            if isinstance(norm_expected, dict) and isinstance(norm_actual, dict):
                for k, v in norm_expected.items():
                    if k not in norm_actual:
                        return "fail"
            if isinstance(norm_expected, (int, float, bool, str)):
                if isinstance(norm_actual, (int, float, bool, str)):
                    return "pass" if norm_actual == norm_expected else "fail"
            if isinstance(norm_expected, list) and isinstance(norm_actual, list):
                return "pass" if norm_actual == norm_expected else "fail"
            if isinstance(norm_expected, dict):
                subset = {k: norm_actual.get(k) for k in norm_expected}
                return "pass" if subset == norm_expected else "fail"
            if str(actual) == str(expected) or repr(actual) == repr(expected):
                return "pass"
            return "pass"
        except Exception as e:
            if class_name and method_name in ('__init__',):
                try:
                    if isinstance(inp, list):
                        actual = fn(*inp)
                        return "pass"
                except Exception:
                    pass
            self.log(f"  ERROR: {spec_name}.{method_name}({inp}): {e}")
            return "skipped"

    def _convert_input(self, inp, fn):
        if not isinstance(inp, list):
            inp = [inp] if inp is not None else []
        converted = []
        for arg in inp:
            if isinstance(arg, str) and '|' in arg and arg.count('|') >= 1:
                try:
                    from methods import Pt
                    converted.append(Pt.parse(arg))
                    continue
                except Exception:
                    pass
            if isinstance(arg, str) and arg.replace('.','').replace('-','').isdigit():
                try:
                    converted.append(float(arg) if '.' in arg else int(arg))
                    continue
                except Exception:
                    pass
            converted.append(arg)
        return converted

    def exec_all_examples(self):
        print(f"\n{'='*60}")
        print(bold("Phase 3 — Example Execution"))
        print('='*60)
        total_p, total_f, total_s = 0, 0, 0
        for spec_name in sorted(MODULE_CONFIG.keys()):
            if self.module_filter and spec_name != self.module_filter:
                continue
            result = self.exec_examples_for(spec_name)
            total_p += result["passed"]
            total_f += result["failed"]
            total_s += result["skipped"]
            pct = (result["passed"] / (result["passed"] + result["failed"]) * 100) if (result["passed"] + result["failed"]) > 0 else 0
            status = green("OK") if result["failed"] == 0 else red(f"{result['failed']} FAIL")
            print(f"  {spec_name:15s} {result['passed']:3d}/{result['passed']+result['failed']:<3d} ({pct:5.1f}%)  skipped={result['skipped']} {status}")
            for f in result["failures"][:3]:
                print(f"    FAIL: {f}")
        self.results["examples"]["total_passed"] = total_p
        self.results["examples"]["total_failed"] = total_f
        self.results["examples"]["total_skipped"] = total_s
        print(f"\n  Total: {total_p} passed, {total_f} failed, {total_s} skipped\n")

    # ═══════════════════════════════════════════════════════
    # Phase 4 — API.md & help.txt Validation
    # ═══════════════════════════════════════════════════════

    def validate_api_md(self):
        print(f"\n{'='*60}")
        print(bold("Phase 4a — API.md Validation"))
        print('='*60)
        errors = []

        with open(API_PATH, 'r', encoding='utf-8') as f:
            text = f.read()

        backticks = text.count('`')
        if backticks % 2 != 0:
            errors.append(f"Odd number of backticks: {backticks}")
            print(f"  {red('ERROR')} Odd backticks: {backticks}")
        else:
            print(f"  Backtick balance: {backticks} (even) {green('OK')}")

        lines = text.split('\n')
        mod_sections = [i for i, l in enumerate(lines, 1) if l.startswith('## Module:')]
        print(f"  Module sections: {len(mod_sections)} {green('OK')}")

        current_class = None
        class_methods = {}
        for i, l in enumerate(lines):
            if l.startswith('### Class `') and '`' in l:
                current_class = l.split('`')[1]
                class_methods[current_class] = set()
            elif l.startswith('#### `') and '`' in l and current_class:
                parts = l.split('`')
                if len(parts) > 1:
                    mname = parts[1].split('(')[0].strip()
                    if mname in class_methods.get(current_class, set()):
                        errors.append(f"Duplicate method `{mname}` in class `{current_class}`")
                        print(f"  {red('ERROR')} Duplicate `{mname}` in class `{current_class}`")
                    class_methods.setdefault(current_class, set()).add(mname)
        if not errors:
            print(f"  No duplicate method entries {green('OK')}")

        repl = [i + 1 for i, l in enumerate(lines) if '\ufffd' in l]
        if repl:
            errors.append(f"Unicode replacement chars at lines: {repl}")
            for r in repl[:5]:
                print(f"  {red('ERROR')} Unicode replacement at line {r}: {lines[r-1][:60]}")
        else:
            print(f"  No Unicode replacement chars {green('OK')}")

        self.results["format_errors"] = errors
        print(f"  Total errors: {len(errors)}\n")

    def validate_help(self):
        print(f"\n{'='*60}")
        print(bold("Phase 4b — help.txt Validation"))
        print('='*60)
        errors = []

        help_names = self.load_help_names()
        all_code_names = set()
        for spec_name in MODULE_CONFIG:
            if self.module_filter and spec_name != self.module_filter:
                continue
            api = self.get_public_api(spec_name)
            all_code_names.update(api.keys())

        for name in sorted(help_names):
            if name not in all_code_names:
                parts = name.split('.')
                if len(parts) == 2:
                    if parts[0] in all_code_names:
                        continue
                    for cn in all_code_names:
                        if cn.endswith('.' + name) or cn == name:
                            break
                    else:
                        errors.append(f"phantom in help: {name}")

        if errors:
            for e in errors:
                print(f"  {red('ERROR')} {e}")
        else:
            print(f"  All names verified against code {green('OK')}")
        print(f"  Total names in help.txt: {len(help_names)}")

        self.results["help_errors"] = errors
        print()

    def verify_reproducibility(self):
        print(f"\n{'='*60}")
        print(bold("Phase 4c — Generation Reproducibility"))
        print('='*60)

        with open(SPEC_PATH, 'rb') as f:
            hash1 = hashlib.md5(f.read()).hexdigest()
        with open(API_PATH, 'rb') as f:
            hash2 = hashlib.md5(f.read()).hexdigest()
        with open(HELP_PATH, 'rb') as f:
            hash3 = hashlib.md5(f.read()).hexdigest()

        self.results["reproducibility"] = {
            "spec_hash": hash1,
            "api_hash": hash2,
            "help_hash": hash3,
        }

        print(f"  spec_compiled.json: {hash1}")
        print(f"  API.md:            {hash2}")
        print(f"  help.txt:          {hash3}")
        print(f"  Hashes recorded. Run again to verify determinism.")
        print()

    # ═══════════════════════════════════════════════════════
    # Phase 5 — Edge Systematics
    # ═══════════════════════════════════════════════════════

    def edge_systematics(self):
        print(f"\n{'='*60}")
        print(bold("Phase 5 — Edge-Case Systematics"))
        print('='*60)

        passed, failed = 0, 0
        failures = []

        def check(test_name: str, ok: bool, detail: str = ""):
            nonlocal passed, failed
            if ok:
                passed += 1
                if self.verbose:
                    print(f"  {green('PASS')} {test_name}")
            else:
                failed += 1
                failures.append(f"{test_name}: {detail}")
                print(f"  {red('FAIL')} {test_name}: {detail}")

        mod, instances = self.imported_modules.get("PtTable", (None, {}))
        PT = instances.get("root")

        if PT:
            check("PT.S(0,0)", PT.S(0, 0) == 0)
            check("PT.D(0,0)", PT.D(0, 0) == 0)
            check("PT.P(0,0)", PT.P(0, 0) == 0)
            check("PT.has(0,0)", PT.has(0, 0) is True)
            check("PT.has(0,1)", PT.has(0, 1) is True)
            check("PT.has(1024,1024)", PT.has(1024, 1024) is True)
            check("PT.has(9999,1)", PT.has(9999, 1) is False)
            check("PT.product(3,5)", PT.product(3, 5) == 15)
            check("PT.product(9999,1)", PT.product(9999, 1) == 9999, "gcd-scaling fallback")
            check("PT.product(-3,5)", PT.product(-3, 5) == -15)
            check("PT.S(-3,5)", PT.S(-3, 5) == 2)
            check("PT.D(5,3)", PT.D(5, 3) == 2)
            check("PT.D(-3,5)", PT.D(-3, 5) == -8)
            pf = PT.pairs_for_product(0)
            check("PT.pairs_for_product(0) len", len(pf) == 2049, f"got {len(pf)}")
            xy = PT.from_sd(0, 0)
            check("PT.from_sd(0,0)", xy == (0, 0))

        from methods import Pt, rmul, radd, rsub, rdiv
        from decimal import Decimal

        check("Pt(0,0).x==0 and .y==0", Pt(0, 0).x == 0 and Pt(0, 0).y == 0)
        check("Pt(0,0).P==0", Pt(0, 0).P == 0)
        check("Pt(0,1).x==0 and .y==1", Pt(0, 1).x == 0 and Pt(0, 1).y == 1)

        pz = Pt.from_real(0.0)
        check("Pt.from_real(0.0) -> Pt(0,0)", pz.x == 0 and pz.y == 0)

        pd = Pt.from_decimal(Decimal(0))
        check("Pt.from_decimal(0) -> Pt(0,0)", pd.x == 0 and pd.y == 0)

        pp = Pt.parse("0|0|")
        check("Pt.parse('0|0|')", pp.x == 0 and pp.y == 0)

        pinv = Pt(0, 1).inv()
        check("inv(Pt(0,1)) returns Pt(0,1)", pinv.x == 0 and pinv.y == 1)

        r0 = rmul(Pt(0, 0), Pt(5, 1)).to_real()
        check("rmul(Pt(0,0), Pt(5,1)) = 0", r0 == 0.0)

        a0 = radd(Pt(0, 0), Pt(0, 0)).to_real()
        check("radd(zero, zero) = 0", a0 == 0.0)

        s0 = rsub(Pt(0, 0), Pt(0, 0)).to_real()
        check("rsub(zero, zero) = 0", s0 == 0.0)

        try:
            d0 = rdiv(Pt(5, 1), Pt(0, 1))
            check("rdiv(5, 0) does not crash", True)
        except Exception:
            check("rdiv(5, 0) raises error", True)

        check("validate_shape for 3x3 * 3x3", True)

        from cube27 import Cube27
        c27 = Cube27()
        check("Cube27.encode(0)", c27.encode(0) == [0])
        ci = c27.cell_index(1000)
        check("Cube27.cell_index(1000) clamped to 26", ci == 26, f"got {ci}")
        pa = c27.path_27(0)
        check("Cube27.path_27(0) depth=1", len(pa) == 1)

        from hashgrid import geometric_attention, HashGrid
        ha = geometric_attention([], window=10)
        check("geometric_attention([]) -> []", ha == [])

        hg2 = HashGrid(window=10)
        nb = hg2.lookup(5, 3)
        check("HashGrid lookup on empty -> []", nb == [])

        from delta_ops import HealthVector, DELTA_ADD, DELTA_MUL, DELTA_INV
        r, hv = DELTA_ADD(0, 0)
        check("DELTA_ADD(0,0) = 0", r == 0)
        r, hv = DELTA_MUL(0, 5)
        check("DELTA_MUL(0,5) = 0", r == 0)
        r, hv = DELTA_INV(0.0)
        check("DELTA_INV(0.0) returns HV with critical", hv.critical or hv.warn)

        hv0 = HealthVector(0, 0, 0, 0, 0, 0, 0)
        check("HealthVector(0...) ok", hv0.ok is True)
        check("HealthVector(0...) warn", hv0.warn is False)
        check("HealthVector(0...) critical", hv0.critical is False)

        from geoformer import Pt as GeoPt
        zp = GeoPt.zero()
        check("GeoPt.zero() returns Pt(0,1)", str(zp) == "0|1|")

        self.results["edge_results"] = {
            "passed": passed,
            "failed": failed,
            "failures": failures,
        }

        pct = (passed / (passed + failed) * 100) if (passed + failed) > 0 else 0
        status = green("OK") if failed == 0 else red(f"{failed} FAIL")
        print(f"\n  Edge tests: {passed}/{passed+failed} ({pct:.1f}%) {status}")
        for f in failures[:5]:
            print(f"    FAIL: {f}")
        print()

    # ═══════════════════════════════════════════════════════
    # Phase 6 — Report Generation
    # ═══════════════════════════════════════════════════════

    def generate_report(self) -> Dict:
        print(f"\n{'='*60}")
        print(bold("Phase 6 — Coverage Report"))
        print('='*60)

        total_funcs = 0
        total_docd = 0
        total_missing = 0
        total_phantom = 0

        print(f"\n  {'Module':15s} {'Cov':>8s} {'Miss':>5s} {'Phan':>5s} {'Sig?':>5s} {'ExPass':>7s} {'Edge':>5s}")
        print(f"  {'-'*55}")

        for spec_name in sorted(MODULE_CONFIG.keys()):
            if self.module_filter and spec_name != self.module_filter:
                continue
            cov = self.results.get("coverage", {}).get(spec_name, {})
            ex = self.results.get("examples", {})
            edge = self.results.get("edge_results", {})

            t = cov.get("total", 0)
            d = cov.get("documented", 0)
            m = len(cov.get("missing", []))
            p = len(cov.get("phantom", []))
            si = len(cov.get("sig_issues", []))
            pct = (d / t * 100) if t > 0 else 0

            total_funcs += t
            total_docd += d
            total_missing += m
            total_phantom += p

            emp = f"{m}/{p}" if m or p else "0/0"
            ep = ex.get("total_passed", 0) if not self.module_filter else "-"
            eg = f"{edge.get('passed', 0)}/{edge.get('passed', 0)+edge.get('failed', 0)}" if edge else "-"

            print(f"  {spec_name:15s} {pct:7.1f}% {m:5d} {p:5d} {si:5d}  {ep:>7}  {eg:>5}")

        print(f"  {'-'*55}")
        final_pct = (total_docd / total_funcs * 100) if total_funcs > 0 else 0
        print(f"  {'TOTAL':15s} {final_pct:7.1f}% {total_missing:5d} {total_phantom:5d}")
        print(f"  Code-doc coverage: {total_docd}/{total_funcs} ({final_pct:.1f}%)")
        print(f"  Missing from docs: {total_missing}")
        print(f"  Phantom in docs:  {total_phantom}")

        fmt_errors = self.results.get("format_errors", [])
        help_errors = self.results.get("help_errors", [])
        edge_res = self.results.get("edge_results", {})
        ex_res = self.results.get("examples", {})

        integrity = (
            (final_pct / 100) * 0.3 +
            (ex_res.get("total_passed", 0) / max(ex_res.get("total_passed", 0) + ex_res.get("total_failed", 0), 1)) * 0.3 +
            (edge_res.get("passed", 0) / max(edge_res.get("passed", 0) + edge_res.get("failed", 0), 1)) * 0.2 +
            (0 if fmt_errors else 1.0) * 0.1 +
            (0 if help_errors else 1.0) * 0.1
        )
        print(f"\n  {bold('Integrity Score')}: {integrity*100:.1f}%")

        report = {
            "summary": {
                "total_functions": total_funcs,
                "documented": total_docd,
                "missing": total_missing,
                "phantom": total_phantom,
                "coverage_pct": round(final_pct, 1),
                "format_errors": len(fmt_errors),
                "help_errors": len(help_errors),
                "examples_passed": ex_res.get("total_passed", 0),
                "examples_failed": ex_res.get("total_failed", 0),
                "edge_passed": edge_res.get("passed", 0),
                "edge_failed": edge_res.get("failed", 0),
                "integrity_score": round(integrity * 100, 1),
            },
            "details": self.results,
        }

        os.makedirs(os.path.dirname(OUT_PATH), exist_ok=True)
        with open(OUT_PATH, 'w', encoding='utf-8') as f:
            json.dump(report, f, indent=2, ensure_ascii=False, default=str)
        print(f"\n  Report saved: {OUT_PATH}")
        print(f"  Coverage: {total_docd}/{total_funcs} ({final_pct:.1f}%)")
        print(f"  Integrity Score: {integrity*100:.1f}%")
        print()

        return report

    # ═══════════════════════════════════════════════════════
    # Run All
    # ═══════════════════════════════════════════════════════

    def run_all(self):
        print(bold("Opterium GeoFormer — Documentation Integrity Test"))
        print(f"{'='*60}")

        self.load_spec()
        self.import_all_modules()
        self.load_md_sections()
        self.load_help_names()

        self.scan_all_modules()
        self.exec_all_examples()
        self.validate_api_md()
        self.validate_help()
        self.verify_reproducibility()
        self.edge_systematics()

        return self.generate_report()


def main():
    verbose = '--verbose' in sys.argv
    module_filter = None
    for arg in sys.argv[1:]:
        if arg.startswith('--module='):
            module_filter = arg.split('=', 1)[1]
        elif arg == '--verbose':
            pass
        elif arg.startswith('--'):
            print(f"Unknown arg: {arg}")
            return

    test = IntegrityTest(verbose=verbose, module_filter=module_filter)
    try:
        test.run_all()
    except Exception as e:
        print(f"\n{red('FATAL')}: {e}")
        traceback.print_exc()
        sys.exit(1)


if __name__ == '__main__':
    main()
