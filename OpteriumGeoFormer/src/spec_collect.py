"""spec_collect.py — Сбор спецификации через тестирование всех модулей.

Для каждой функции/метода записывает:
  - сигнатуру (имя, аргументы, типы)
  - примеры вызовов с разными входными данными
  - возвращаемые значения
  - граничные случаи и ошибки
  - описание

Сохраняет: spec.json рядом с каждым модулем.
Генерирует: spec_compiled.json со всей информацией.
"""
import sys, os, json, inspect, math, random
from decimal import Decimal
from typing import Any, Dict, List, Tuple, Optional
from dataclasses import dataclass

SRC = os.path.dirname(__file__)
sys.path.insert(0, SRC)
sys.path.insert(0, os.path.join(SRC, 'spec-kit'))

# ═══════════════════════════════════════════════════════════
# Collector
# ═══════════════════════════════════════════════════════════
@dataclass
class Example:
    input: Any
    output: Any
    description: str = ""

@dataclass
class MethodSpec:
    name: str
    signature: str
    description: str
    inputs: List[Dict]
    outputs: List[Dict]
    examples: List[Example]
    edges: List[Example]
    errors: List[str]

class SpecCollector:
    def __init__(self):
        self.data: Dict[str, Dict] = {}
    
    def module(self, name: str, filepath: str = "") -> 'ModuleCollector':
        if name not in self.data:
            self.data[name] = {
                "module": name,
                "filepath": filepath,
                "classes": {},
                "functions": {},
            }
        return ModuleCollector(self.data[name])

class ModuleCollector:
    def __init__(self, data: Dict):
        self.data = data
    
    def class_(self, name: str, description: str = "") -> 'ClassCollector':
        if name not in self.data["classes"]:
            self.data["classes"][name] = {
                "name": name,
                "description": description,
                "methods": {},
            }
        return ClassCollector(self.data["classes"][name])
    
    def function(self, name: str, signature: str = "", description: str = "", 
                 inputs: list = None, outputs: list = None) -> 'FnCollector':
        if name not in self.data["functions"]:
            self.data["functions"][name] = {
                "name": name,
                "signature": signature,
                "description": description,
                "inputs": inputs or [],
                "outputs": outputs or [],
                "examples": [],
                "edges": [],
                "errors": [],
            }
        return FnCollector(self.data["functions"][name])

class ClassCollector:
    def __init__(self, data: Dict):
        self.data = data
    
    def method(self, name: str, signature: str = "", description: str = "",
               inputs: list = None, outputs: list = None) -> 'FnCollector':
        if name not in self.data["methods"]:
            self.data["methods"][name] = {
                "name": name,
                "signature": signature,
                "description": description,
                "inputs": inputs or [],
                "outputs": outputs or [],
                "examples": [],
                "edges": [],
                "errors": [],
            }
        return FnCollector(self.data["methods"][name])

class FnCollector:
    def __init__(self, data: Dict):
        self.data = data
    
    def example(self, input_val: Any, output_val: Any, desc: str = ""):
        self.data["examples"].append({
            "input": self._serialize(input_val),
            "output": self._serialize(output_val),
            "description": desc,
        })
        return self
    
    def edge(self, input_val: Any, output_val: Any, desc: str = ""):
        self.data["edges"].append({
            "input": self._serialize(input_val),
            "output": self._serialize(output_val),
            "description": desc,
        })
        return self
    
    def error(self, input_val: Any, error_type: str, desc: str = ""):
        self.data["errors"].append({
            "input": self._serialize(input_val),
            "error": error_type,
            "description": desc,
        })
        return self
    
    def _serialize(self, v):
        if isinstance(v, (int, float, bool, str)):
            return v
        if isinstance(v, Decimal):
            return str(v)
        if isinstance(v, (list, tuple)):
            if v and hasattr(v[0], '__dict__'):
                return [str(x) for x in v]
            return list(v)
        if isinstance(v, dict):
            return {k: self._serialize(val) for k, val in v.items()}
        if hasattr(v, '__dict__'):
            d = {}
            for k in dir(v):
                if not k.startswith('_'):
                    try:
                        d[k] = self._serialize(getattr(v, k))
                    except:
                        pass
            return d
        return str(v)

# ═══════════════════════════════════════════════════════════
# Сбор спецификации
# ═══════════════════════════════════════════════════════════
collector = SpecCollector()

# ── 1. arith_table.PtTable ──────────────────────────────
from arith_table import PT, PtTable
mod = collector.module("PtTable", "src/arith_table.py")
cls = mod.class_("PtTable", "Zero-arithmetic address table: all (x,y) → {S, D, P} via precomputed lookup")

# S
m = cls.method("S", "S(x, y) -> int", "Sum: x + y via table with negative reflection",
    inputs=[{"x": "int", "range": "[-1024..1024]"}, {"y": "int", "range": "[-1024..1024]"}],
    outputs=[{"type": "int", "formula": "x+y"}])
for x, y in [(3,5), (-3,5), (3,-5), (-3,-5), (0,5), (5,0), (-5,0), (0,0)]:
    m.example((x,y), PT.S(x,y), f"S({x},{y}) = {PT.S(x,y)}")

m.edge((1024,1024), PT.S(1024,1024), "max coord")
m.edge((0,0), PT.S(0,0), "zero+zero")

# D
m = cls.method("D", "D(x, y) -> int", "Difference: x - y via table with negative reflection",
    inputs=[{"x": "int", "range": "[-1024..1024]"}, {"y": "int", "range": "[-1024..1024]"}],
    outputs=[{"type": "int", "formula": "x-y"}])
for x, y in [(3,5), (5,3), (-3,5), (3,-5), (-3,-5), (0,5), (5,0)]:
    m.example((x,y), PT.D(x,y), f"D({x},{y}) = {PT.D(x,y)}")

# P
m = cls.method("P", "P(x, y) -> int", "Product: x * y via table with sign inversion",
    inputs=[{"x": "int", "range": "[-1024..1024]"}, {"y": "int", "range": "[-1024..1024]"}],
    outputs=[{"type": "int", "formula": "x*y"}])
for x, y in [(3,5), (-3,5), (3,-5), (-3,-5), (0,5), (5,0)]:
    m.example((x,y), PT.P(x,y), f"P({x},{y}) = {PT.P(x,y)}")

# lookup
m = cls.method("lookup", "lookup(x, y) -> dict", "Returns {S, D, P} for coordinates")
m.example((3,5), PT.lookup(3,5), "lookup(3,5)")

# from_sd
m = cls.method("from_sd", "from_sd(S, D) -> Tuple[int,int]", "Recover (x,y) from sum and difference")
for s, d in [(8,-2), (8,2), (10,0), (7,-3), (0,0)]:
    xy = PT.from_sd(s,d)
    m.example((s,d), xy, f"from_sd({s},{d}) = {xy}")

# pairs_for_product
m = cls.method("pairs_for_product", "pairs_for_product(P) -> List[Tuple]", "All factor pairs (x,y) with x*y=P")
m.example(12, PT.pairs_for_product(12), "all factor pairs of 12")
m.example(17, PT.pairs_for_product(17), "prime: only (1,17) and (17,1)")

# isqrt
m = cls.method("isqrt", "isqrt(n) -> int", "Integer square root (table or math.isqrt)")
for n in [0, 1, 4, 144, 2, 999999]:
    m.example(n, PT.isqrt(n), f"isqrt({n}) = {PT.isqrt(n)}")

# conj
m = cls.method("conj", "conj(x, y) -> Tuple[int,int]", "Quaternion conjugate: (y, x)")
for x, y in [(3,5), (-3,5), (0,5)]:
    m.example((x,y), PT.conj(x,y), f"conj({x},{y}) = {PT.conj(x,y)}")

# product/sum/diff
for fn_name in ["product", "sum", "diff"]:
    fn = getattr(PT, fn_name)
    m = cls.method(fn_name, f"{fn_name}(a, b) -> int", f"{fn_name} via table with fallback")
    for a, b in [(3,5), (-3,5), (3,-5), (-3,-5), (0,5), (5,0), (0,0)]:
        r = fn(a,b)
        expected = {"product": a*b, "sum": a+b, "diff": a-b}[fn_name]
        m.example((a,b), r, f"{fn_name}({a},{b}) = {r} (expected {expected})")

# pow10
m = cls.method("pow10", "pow10(n) -> int", "10^n via table or fallback")
for n in [0, 1, 3, 10, 20]:
    m.example(n, PT.pow10(n), f"pow10({n}) = {PT.pow10(n)}")

# abs
m = cls.method("abs", "abs(x) -> int", "Absolute value via table or fallback")
for v in [-5, -1, 0, 1, 5]:
    m.example(v, PT.abs(v), f"abs({v}) = {PT.abs(v)}")

# dot / matmul
m = cls.method("dot", "dot(a_list, b_list) -> int", "Vector dot product via product lookups")
m.example(([1,2,3],[4,5,6]), PT.dot([1,2,3],[4,5,6]), "dot([1,2,3],[4,5,6]) = 1*4+2*5+3*6")

m = cls.method("matmul", "matmul(A, B) -> List[List[int]]", "Matrix multiply via product lookups")
m.example(([[1,2],[3,4]],[[5,6],[7,8]]), PT.matmul([[1,2],[3,4]],[[5,6],[7,8]]), "2x2 multiply")

# activation_table
m = cls.method("activation_table", "activation_table(fn, lo, hi) -> dict", "Precompute activation fn over [lo,hi]")
m.example(("relu(x)=max(0,x)", -3, 5),
          PT.activation_table(lambda x: max(0,x), -3, 5),
          "ReLU over [-3,5]")

# has
m = cls.method("has", "has(x, y) -> bool", "Check if (x,y) is in table range")
for x,y in [(0,1), (1024,1024), (0,0), (9999,1)]:
    m.example((x,y), PT.has(x,y), f"has({x},{y}) = {PT.has(x,y)}")

# max / min / square
m = cls.method("max", "max(a, b) -> int", "Maximum via table or fallback")
for a,b in [(3,5), (-3,5), (0,0)]:
    m.example((a,b), PT.max(a,b), f"max({a},{b}) = {PT.max(a,b)}")

m = cls.method("min", "min(a, b) -> int", "Minimum via table or fallback")
for a,b in [(3,5), (-3,5), (0,0)]:
    m.example((a,b), PT.min(a,b), f"min({a},{b}) = {PT.min(a,b)}")

m = cls.method("square", "square(n) -> int", "n^2 via table or fallback")
for n in [0, 5, 12, -3]:
    m.example(n, PT.square(n), f"square({n}) = {PT.square(n)}")

# summary
m = cls.method("summary", "summary() -> dict", "Table statistics")
m.example((), PT.summary(), "current table stats")

# ── 2. methods.Pt (spec-kit) ────────────────────────────
from methods import Pt, rmul, radd, rsub, rdiv, geo_mul, geo_add, validate_shape, to_pt_matrix, sd_tuple_matrix
mod = collector.module("Pt", "src/spec-kit/methods/__init__.py")
cls = mod.class_("Pt", "Geometric point with mantissa-rank notation x|y| = x / 10^y")

# __init__
m = cls.method("__init__", "Pt(x, y=1)", "Construct Pt with S/D/P auto-computed")
for x, y in [(0,1), (3,5), (1024,1024), (-3,5), (0,0)]:
    p = Pt(x, y)
    m.example((x,y), f"Pt({p.x},{p.y}) S={p.S} D={p.D} P={p.P}",
              f"Pt({x},{y})")

# from_int
m = cls.method("from_int", "from_int(v) -> Pt", "Integer → Pt for geometric use: P=v")
for v in [0, 1, 42, -7]:
    p = Pt.from_int(v)
    m.example(v, f"Pt({p.x},{p.y}) P={p.P}", f"from_int({v})")

# from_sd
m = cls.method("from_sd", "from_sd(S,D) -> Pt", "Construct Pt from (S,D) coordinates")
for s, d in [(8,-2), (10,0), (7,-3)]:
    p = Pt.from_sd(s,d)
    m.example((s,d), f"Pt({p.x},{p.y})", f"from_sd({s},{d})")

# parse
m = cls.method("parse", "parse(s) -> Pt", 'Parse mantissa|rank| notation: "347|3|"')
for s in ["347|3|", "0|3|", "42|", "-347|3|"]:
    try:
        p = Pt.parse(s)
        m.example(s, f"Pt({p.x},{p.y})", f'parse("{s}")')
    except:
        m.error(s, "ValueError", f'parse("{s}") raises ValueError')

# from_real / to_real
m = cls.method("from_real", "from_real(r) -> Pt", "Float → Pt(mantissa, rank) via Decimal")
for r in [0.0, 0.347, -3.14, 1e-6, 100.0]:
    p = Pt.from_real(r)
    back = p.to_real()
    m.example(r, {"Pt": f"({p.x},{p.y})", "to_real_back": back},
              f"from_real({r}) → Pt({p.x},{p.y}) → to_real={back}")

# to_real
m = cls.method("to_real", "to_real() -> float", "Pt → float")
for pt in [Pt(347,3), Pt(0,0), Pt(-314,2)]:
    m.example(str(pt), pt.to_real(), f"Pt({pt.x},{pt.y}).to_real() = {pt.to_real()}")

# from_decimal / to_decimal
m = cls.method("from_decimal", "from_decimal(d) -> Pt", "Decimal → Pt with full precision")
for d in [Decimal('0'), Decimal('3.14'), Decimal('1e30'), Decimal('1e-30'), Decimal('-2.718')]:
    p = Pt.from_decimal(d)
    back = p.to_decimal()
    m.example(str(d), {"Pt": f"({p.x},{p.y})", "to_decimal_back": str(back)},
              f"from_decimal({d}) → Pt({p.x},{p.y}) → to_decimal={back}")

# inv
m = cls.method("inv", "inv() -> Pt", "Inverse via Decimal lift-solve-project")
for x, y in [(347,3), (2,1), (10,1), (1,1), (0,1)]:
    p = Pt(x,y)
    pinv = p.inv()
    prod = pinv.to_decimal() * p.to_decimal()
    m.example((x,y), {"inv": f"Pt({pinv.x},{pinv.y})", "product": str(prod)},
              f"inv({x}|{y}|) → Pt({pinv.x},{pinv.y}), p*inv={prod}")

# to_decimal
m = cls.method("to_decimal", "to_decimal() -> Decimal", "Pt → Decimal with exact precision")
m.example("Pt(347,3)", str(Pt(347,3).to_decimal()), "Pt(347,3).to_decimal()")

# verbose / repr
m = cls.method("verbose", "verbose() -> str", "Full description: Pt(x,y S= D= P=)")
p = Pt(347,3)
m.example((), p.verbose(), f"verbose: {p.verbose()}")

# __add__ / __mul__
m = cls.method("__add__", "__add__(other) -> Pt", "Component-wise addition")
a,b = Pt(3,5), Pt(2,7)
c = a + b
m.example((a,b), f"Pt({c.x},{c.y})", f"Pt(3,5)+Pt(2,7)=Pt({c.x},{c.y})")

m = cls.method("__mul__", "__mul__(other) -> Pt", "Component-wise multiplication")
c = a * b
m.example((a,b), f"Pt({c.x},{c.y})", f"Pt(3,5)*Pt(2,7)=Pt({c.x},{c.y})")

# rmul / radd
m = mod.function("rmul", "rmul(a:Pt, b:Pt) -> Pt", "Mantissa-rank multiply")
a,b = Pt.from_real(0.3), Pt.from_real(0.2)
c = rmul(a,b)
m.example(("0.3","0.2"), {"Pt": f"({c.x},{c.y})", "to_real": c.to_real()},
          f"rmul(0.3,0.2) = {c.to_real()}")

m = mod.function("radd", "radd(a:Pt, b:Pt) -> Pt", "Mantissa-rank add with rank alignment")
c = radd(a,b)
m.example(("0.3","0.2"), {"Pt": f"({c.x},{c.y})", "to_real": c.to_real()},
          f"radd(0.3,0.2) = {c.to_real()}")

m = mod.function("rsub", "rsub(a:Pt, b:Pt) -> Pt", "Mantissa-rank subtract with rank alignment")
for va, vb in [(0.5, 0.03), (-0.3, -0.1), (0.05, -0.3)]:
    a, b = Pt.from_real(va), Pt.from_real(vb)
    c = rsub(a, b)
    m.example((va,vb), {"Pt": f"({c.x},{c.y})", "to_real": c.to_real()},
              f"rsub({va},{vb}) = {c.to_real()}")

m = mod.function("rdiv", "rdiv(a:Pt, b:Pt) -> Pt", "Mantissa-rank divide = rmul(a, inv(b))")
for va, vb in [(0.3, 0.2), (-0.3, 0.2)]:
    a, b = Pt.from_real(va), Pt.from_real(vb)
    c = rdiv(a, b)
    m.example((va,vb), {"Pt": f"({c.x},{c.y})", "to_real": c.to_real()},
              f"rdiv({va},{vb}) = {c.to_real()}")

# geo_mul / geo_add
m = mod.function("geo_mul", "geo_mul(a:Pt, b:Pt) -> Pt", "Geometric multiply")
a,b = Pt(3,5), Pt(2,7)
c = geo_mul(a,b)
m.example((a,b), f"Pt({c.x},{c.y})", f"geo_mul(Pt(3,5),Pt(2,7))=Pt({c.x},{c.y})")

m = mod.function("geo_add", "geo_add(a:Pt, b:Pt) -> Pt", "Geometric addition (cross-multiply)")
c = geo_add(a,b)
m.example((a,b), f"Pt({c.x},{c.y})", f"geo_add(Pt(3,5),Pt(2,7))=Pt({c.x},{c.y})")

# validate_shape
m = mod.function("validate_shape",
    "validate_shape(A, B) -> (m,k,n)", "Validate matrix dimensions")
m.example(([[0]*3]*2, [[0]*2]*3), validate_shape([[0]*3]*2, [[0]*2]*3), "2x3 * 3x2 = (2,3,2)")
m.error("mismatch: 2x3 * 2x4",
        "ValueError", "validate_shape(2x3, 2x4) raises ValueError")

# to_pt_matrix / sd_tuple_matrix
m = mod.function("to_pt_matrix", "to_pt_matrix(M) -> list", "Convert int matrix to Pt matrix")
M = to_pt_matrix([[1,2],[3,4]])
m.example("[[1,2],[3,4]]", [[str(p) for p in row] for row in M], "to_pt_matrix")

m = mod.function("sd_tuple_matrix", "sd_tuple_matrix(M) -> list", "Convert to (S,D) pairs")
M_sd = sd_tuple_matrix([[1,2],[3,4]])
m.example("[[1,2],[3,4]]", M_sd, "sd_tuple_matrix")

# ── 3. cube27.Cube27 ──────────────────────────────────
from cube27 import Cube27
mod = collector.module("Cube27", "src/cube27.py")
cls = mod.class_("Cube27", "Self-similar 3-digit decimal addressing")

c = Cube27()
m = cls.method("encode", "encode(mantissa) -> list", "Split into 3-digit groups MSB-first")
for v in [0, 1, 347, 123456789, 1000000]:
    m.example(v, c.encode(v), f"encode({v}) = {c.encode(v)}")

m = cls.method("cell_index", "cell_index(group) -> int", "3-digit group → 27-ary cell 0..26")
for g in [0, 36, 37, 296, 333, 665, 666, 999, 1000]:
    m.example(g, c.cell_index(g), f"cell_index({g}) = {c.cell_index(g)}")
m.edge(1000, c.cell_index(1000), "clamped to 26")

m = cls.method("cell_27", "cell_27(group) -> tuple", "3-digit group → (cx,cy,cz) in 3×3×3")
for g in [0, 37, 296, 333, 665, 666, 999]:
    m.example(g, c.cell_27(g), f"cell_27({g}) = {c.cell_27(g)}")

m = cls.method("path_27", "path_27(mantissa) -> list", "Full Cube27 path: [(cx,cy,cz), ...]")
for v in [0, 347, 123456789]:
    m.example(v, c.path_27(v), f"path_27({v}) depth={c.depth(v)}")

m = cls.method("depth", "depth(mantissa) -> int", "Levels of Cube27 needed for this mantissa")
for v in [0, 347, 123456789]:
    m.example(v, c.depth(v), f"depth({v}) = {c.depth(v)}")

m = cls.method("format_path", "format_path(mantissa) -> str", "Human-readable: 123|456|789| (cells)")
for v in [0, 347, 123456789]:
    m.example(v, c.format_path(v), f"format_path({v})")

m = cls.method("verify", "verify(mantissa) -> dict", "Encode + verify PtTable hit")
m.example(123456789, c.verify(123456789), "verify(123456789) all_hit check")

# ── 4. hashgrid ────────────────────────────────────────
from hashgrid import HashGrid, geometric_weight, geometric_attention
mod = collector.module("HashGrid", "src/hashgrid.py")
cls = mod.class_("HashGrid", "O(1) neighbor lookup in (S,D) space")

g = HashGrid(window=10)
m = cls.method("insert", "insert(id, S, D, **extra) -> int", "Insert point into bucket")
m.example((0, 5, 3), g.insert(0,5,3), "insert(0,5,3)")
m = cls.method("insert_many", "insert_many(tokens) -> None", "Batch insert tokens")
m.example([(0,5,3),(1,7,2)], g.insert_many([(0,5,3),(1,7,2)]), "insert_many 2 tokens")
m = cls.method("lookup", "lookup(S, D) -> list", "Return entries in 3×3 neighborhood")
g.insert(1, 7, 2); g.insert(2, 100, 50)
nb = g.lookup(6,3)
m.example((6,3), f"{len(nb)} neighbors with ids {[e['id'] for e in nb]}", "lookup(6,3)")
m = cls.method("stats", "stats() -> dict", "Bucket statistics")
m.example((), g.stats(), "current stats")
m = cls.method("clear", "clear()", "Clear all buckets")

# geometric_weight
m = mod.function("geometric_weight",
    "geometric_weight(S1,D1, S2,D2, eps=1.0) -> float",
    "Proximity weight: 1/(eps + |ΔS| + |ΔD|)")
m.example((10,5,20,15), geometric_weight(10,5,20,15), "weight between (10,5) and (20,15)")
m.example((20,15,10,5), geometric_weight(20,15,10,5), "symmetric")

# geometric_attention
m = mod.function("geometric_attention",
    "geometric_attention(tokens, window=16, ...) -> list",
    "One layer of hashgrid attention")
random.seed(42)
pts = [(i, random.randint(1,100), random.randint(-50,50), random.randint(1,100)) for i in range(10)]
out = geometric_attention(pts, window=20)
m.example(f"10 tokens", {f"id={o['id']}": {"context": o['context'], "neighbors": o['neighbors']} for o in out[:3]},
          "first 3 tokens attention")
m.edge("empty", geometric_attention([], window=10), "empty token list returns []")

# ── 5. delta_ops ──────────────────────────────────────
from delta_ops import (HealthVector, DeltaOp, CompositeDelta, HEALTH_OK, HEALTH_WARN,
    DELTA_ADD, DELTA_MUL, DELTA_INV, DELTA_INV_NS, DELTA_PPH, DELTA_OPTG,
    DELTA_SHIFT, DELTA_ROT, DELTA_ZERO_DETECT,
    compose_sequential, compose_parallel, check_domain, select_fallback, identity)
mod = collector.module("delta_ops", "src/delta_ops.py")

cls = mod.class_("HealthVector", "7-channel stability monitor")
hv = HealthVector(0.1, 0.2, 0.0, 0.05, 0.3, 0.15, 0.01)
m = cls.method("ok", "ok -> bool", "All channels < 0.35")
m.example(hv, hv.ok, "ok check")
m = cls.method("warn", "warn -> bool", "Any channel in [0.35, 0.65)")
m.example(hv, hv.warn, "warn check")
m = cls.method("critical", "critical -> bool", "Any channel >= 0.65")
m.example(hv, hv.critical, "critical check")
m = cls.method("max_channel", "max_channel -> Tuple[str, float]", "Highest channel name+value")
m.example(hv, hv.max_channel, "max channel")
m = cls.method("merge", "merge(other) -> HealthVector", "Element-wise max of two HVs")
hv2 = HealthVector(0.8,0,0,0,0,0,0)
hv3 = hv.merge(hv2)
m.example((hv, hv2), f"E_assoc={hv3.E_assoc}", "merged")

# Built-in DeltaOps
for op_name, op, inputs in [
    ("DELTA_ADD", DELTA_ADD, [(3,5), (-3,5), (0,0)]),
    ("DELTA_MUL", DELTA_MUL, [(3,5), (-3,5), (0,5)]),
    ("DELTA_INV", DELTA_INV, [(2.0,), (4.0,), (0.0,)]),
    ("DELTA_INV_NS", DELTA_INV_NS, [((0,1,0),), ((0,0,0),)]),
    ("DELTA_SHIFT", DELTA_SHIFT, [(5.0,), (5.0,)]),
    ("DELTA_ROT", DELTA_ROT, [(1+0j,)]),
]:
    fn_mod = mod.function if op_name.startswith("DELTA_") else cls.method
    m = fn_mod(op_name, f"{op_name}(...) -> (result, HealthVector)",
               f"{op.name} operator: {op.domain} → {op.codomain}")
    for inp in inputs:
        if op_name == "DELTA_SHIFT":
            r, hv = op(inp[0], power=0)
        elif op_name == "DELTA_ROT":
            r, hv = op(inp[0], angle_deg=90)
        else:
            r, hv = op(*inp)
        m.example(inp, {"result": r, "hv_ok": hv.ok}, f"{op_name}{inp} → {r}")

# DELTA_PPH (arity=1, expects sequence of singular values)
m = mod.function("DELTA_PPH", "DELTA_PPH(singular_values) -> (result, HealthVector)",
                 "PPH operator: S → S (projection residue)")
for sv in [[5.0, 3.0], [0.0, 0.0], [10.0]]:
    r, hv = DELTA_PPH(sv)
    m.example(sv, {"result": round(r, 4), "hv_ok": hv.ok}, f"PPH({sv}) → {round(r,4)}")

# DELTA_OPTG (arity=2)
m = mod.function("DELTA_OPTG", "DELTA_OPTG(state, attractor) -> (result, HealthVector)",
                 "OPTG operator: E8 → E8 (Weyl geodesic)")
r, hv = DELTA_OPTG([1.0, 0.0], [0.0, 1.0])
m.example(([1,0],[0,1]), {"result": r[:4], "hv_ok": hv.ok}, "OPTG([1,0],[0,1])")
r2, hv2 = DELTA_OPTG([0.5, 0.5], [1.0, 0.0])
m.example(([0.5,0.5],[1,0]), {"result": r2[:4], "hv_ok": hv2.ok}, "OPTG([0.5,0.5],[1,0])")

# compose
m = mod.function("compose_sequential",
    "compose_sequential(*ops) -> CompositeDelta",
    "Sequential composition: apply ops[0], then ops[1], ...")
seq = compose_sequential(DELTA_SHIFT, DELTA_INV)
m.example(("DELTA_SHIFT>>DELTA_INV", 5.0),
          [str(seq(5.0, power=1)[0]), f"hv={seq(5.0, power=1)[1].ok}"],
          "SHIFT>>INV = 1/(5*10) = 0.02")

m = mod.function("compose_parallel",
    "compose_parallel(*ops) -> CompositeDelta",
    "Parallel composition over disjoint subspaces")
par = compose_parallel(DELTA_ADD, DELTA_MUL)
m.example(("DELTA_ADD||DELTA_MUL", 3.0, 5.0),
          [str(par(3.0, 5.0))],
          "ADD||MUL on (3,5)")

m = mod.function("check_domain",
    "check_domain(domain, assoc=..., commut=..., div=...) -> bool",
    "Verify algebra properties")
m.example(("O",), check_domain('O', assoc=False, commut=False, div=True), "Octonions correct")
m.example(("O",), check_domain('O', assoc=True), "Octonions not associative")

m = mod.function("select_fallback",
    "select_fallback(op_name, domain) -> str|None",
    "Fallback strategy for non-invertible domains")
m.example(("Δ_INV", "S"), select_fallback('Δ_INV', 'S'), "Sedenion inv fallback")

# ── 6. phi_algebra ────────────────────────────────────
from phi_algebra import (PHI1_SHIFT, PHI2_PHASE, PHI3_FIXEDPOINT, PHI4_RECURSION,
    PHI5_PROJECTION, PhiPath, periodic_orbit, harmonic_series)
mod = collector.module("phi_algebra", "src/phi_algebra.py")

for phi, name in [(PHI1_SHIFT, "PHI1_SHIFT"), (PHI2_PHASE, "PHI2_PHASE"),
                  (PHI3_FIXEDPOINT, "PHI3_FIXEDPOINT"), (PHI4_RECURSION, "PHI4_RECURSION"),
                  (PHI5_PROJECTION, "PHI5_PROJECTION")]:
    m = mod.function(name, f"{name}(state, *args) -> state'",
                     f"Φ{phi.index}({phi.symbol}) {phi.description}")
    if phi is PHI1_SHIFT:
        m.example(((0,0), 1), phi((0,0), dx=1), "SHIFT(0,0,dx=1)")
    elif phi is PHI2_PHASE:
        m.example(([1,2,3], 1), phi([1,2,3], steps=1), "PHASE([1,2,3])")
    elif phi is PHI3_FIXEDPOINT:
        m.example((10.0, 0.0), phi(10.0, target=0.0), "FIXEDPOINT(10→5)")
    elif phi is PHI5_PROJECTION:
        m.example(((1,2,3,4), (0,2)), phi((1,2,3,4), keep=(0,2)), "PROJECTION")

m = mod.function("PhiPath", "PhiPath(ops)", "Sequence of Φ-operators")
p = PHI1_SHIFT >> PHI2_PHASE
m.example("PHI1_SHIFT>>PHI2_PHASE", f"len={len(p)} ops", "2-operator path")

m = mod.function("periodic_orbit", "periodic_orbit(period) -> PhiPath", "Periodic cycle via Φ₂")
m.example(4, str(periodic_orbit(4)), "4-step orbit")

m = mod.function("harmonic_series", "harmonic_series(fundamental, harmonics) -> PhiPath",
                 "Φ₁∘Φ₂ repeated")
m.example((1.0, 3), str(harmonic_series(1.0, 3)), "3 harmonics")

# ── 7. swarm ──────────────────────────────────────────
from swarm import IntelligentSwarm, SwarmNode, BayesReplacement
mod = collector.module("swarm", "src/swarm.py")

cls = mod.class_("IntelligentSwarm", "Decision engine replacing Bayes")
sw = IntelligentSwarm(seed=42)
for name in ['A','B','C']:
    sw.register(name, 1.0)

m = cls.method("register", "register(nid, potential=1.0) -> SwarmNode", "Register a node")
m = cls.method("probabilities", "probabilities() -> dict", "Probability distribution over nodes")
probs = sw.probabilities()
m.example((), probs, "3-node probs")

m = cls.method("decide", "decide(candidates=None, deterministic=False) -> SwarmNode",
               "Choose best node")
m.example((True,), sw.decide(deterministic=True).id, "deterministic choice")

m = cls.method("update", "update(nid, success)", "Reinforce node")
sw.update('A', True); sw.update('B', False)
m.example(("A", True), sw.score('A'), "after reinforce A+ B-")

m = cls.method("score", "score(nid) -> float", "Raw score for a node")
m.example("A", sw.score('A'), "score A")

# BayesReplacement
m = mod.function("BayesReplacement", "BayesReplacement(swarm)", "Bayesian interface over Swarm")
br = BayesReplacement(sw)
br.update_belief('A', 0.9)
m.example(("A", 0.9), br.posterior(['A','B']), "posterior after update")
m.example((['A','B']), br.predict(['A','B']), "predict best")

# ── 8. e8_twist ───────────────────────────────────────
from e8_twist import TwistEngine, address_to_root, root_properties
mod = collector.module("e8_twist", "src/e8_twist.py")

cls = mod.class_("TwistEngine", "E8 TWIST operations (zero float, zero trig)")
te = TwistEngine()

m = cls.method("triality_groups", "triality_groups() -> dict", "Split 240 roots: V(112), S+(64), S-(64)")
g = te.triality_groups()
m.example((), {k: len(v) for k,v in g.items()}, "triality groups")

m = cls.method("cycle_2520", "cycle_2520(angle) -> dict", "2520-cycle params")
m.example(35, te.cycle_2520(35), "35° → 72 steps")
m.example(70, te.cycle_2520(70), "70° → 36 steps")

m = cls.method("twist", "twist(phase, config) -> dict", "Apply TWIST to triality groups")
m.example((0, (112,64,192)), te.twist(0, (112,64,192)), "TWIST(0, (112,64,192))")

m = cls.method("closure_angle", "closure_angle(angle=70) -> dict", "Closure via address routing")
m.example(70, te.closure_angle(70), "closure 70°")

m = cls.method("scan_configs", "scan_configs() -> list", "Scan configs, sorted by amplitude")
m.example((), len(te.scan_configs()), "scan results count")

m = cls.method("cycle_all_angles", "cycle_all_angles() -> list", "Cycle angles for 35°, 70°, 105°, 140°")
r_angles = te.cycle_all_angles()
m.example((), f"{len(r_angles)} angle results", "cycles for all angles")

m = cls.method("summary", "summary() -> dict", "TwistEngine status summary")
m.example((), te.summary(), "current status")

# address_to_root
m = mod.function("address_to_root",
    "address_to_root(x, y) -> tuple", "Map 2D address to 8D E8 root")
for x,y in [(3,5), (2,2), (4,3)]:
    m.example((x,y), list(address_to_root(x,y)[:4]), f"address({x},{y}) → root")

# root_properties
m = mod.function("root_properties",
    "root_properties(root) -> dict", "O(1) extraction from root address")
r = address_to_root(3,5)
m.example(r, root_properties(r), "properties of address(3,5) root")

# ── 9. doctor_geo ─────────────────────────────────────
from doctor_geo import (SwarmDoctor, GeoHealthVector,
    geo_to_opterium_hv, opterium_to_geo_hv, ROUTE_REGISTRY,
    OPTERIUM_AVAILABLE)
mod = collector.module("doctor_geo", "src/doctor_geo.py")

# GeoHealthVector is alias for delta_ops.HealthVector
cls = mod.class_("GeoHealthVector", "Alias for delta_ops.HealthVector (7-channel)")

cls2 = mod.class_("SwarmDoctor", "Swarm-powered DoctorCore integration")
sd = SwarmDoctor(swarm_seed=42)
for name, info in ROUTE_REGISTRY.items():
    sd.register_route(name, info['default_potential'])

m = cls2.method("register_route", "register_route(name, potential)", "Register a route")
m = cls2.method("choose_route", "choose_route(candidates, deterministic) -> str", "Swarm route choice")
m.example((True,), sd.choose_route(deterministic=True), "deterministic choice")

m = cls2.method("judge", "judge(hv, context) -> str", "Verdict: OK/WARN/QUARANTINE/ROLLBACK")
hv_ok = GeoHealthVector(0,0,0,0,0,0,0)
m.example((hv_ok,), sd.judge(hv_ok), "judge(OK) → OK")
hv_bad = GeoHealthVector(E_assoc=0.8, E_precision=0.9, E_tension=0.95)
m.example((hv_bad,), sd.judge(hv_bad), "judge(bad) → QUARANTINE/ROLLBACK")

m = cls2.method("reinforce_route", "reinforce_route(route, success)", "Reinforce after verdict")
sd.reinforce_route('pytable_lookup', True)
m.example(('pytable_lookup', True), sd.swarm.score('pytable_lookup'), "reinforce")

m = cls2.method("quarantine_item", "quarantine_item(key, payload, verdict)", "Store quarantined item")
m = cls2.method("get_quarantine", "get_quarantine(key) -> payload", "Retrieve quarantined item")
sd.quarantine_item('x', {'val':42}, {'level':'WARN'})
m.example('x', sd.get_quarantine('x')['payload'], "quarantine roundtrip")

m = cls2.method("clear_quarantine", "clear_quarantine() -> None", "Clear all quarantined items")
m.example((), sd.clear_quarantine(), "clear quarantine")

m = cls2.method("route_probabilities", "route_probabilities(candidates=None) -> dict", "Current route probabilities")
m.example((), sd.route_probabilities(), "all route probs")

m = cls2.method("judge_full", "judge_full(hv, context='') -> dict", "Full verdict with details")
hv_ok2 = GeoHealthVector(0,0,0,0,0,0,0)
m.example((hv_ok2,), sd.judge_full(hv_ok2), "judge_full(OK)")

sd2 = SwarmDoctor(swarm_seed=42)
m = cls2.method("opterium_judge", "opterium_judge(ohv, context='') -> Any", "Opterium-compatible judge")
m.example(("HV(ok)",), sd2.opterium_judge("HV(ok)"), "opterium_judge string")

m = cls2.method("opterium_verdict", "opterium_verdict(geo_hv, context='') -> dict", "Full optical verdict")
hv_test = GeoHealthVector(0.1, 0.2, 0, 0, 0, 0, 0)
m.example((hv_test,), sd2.opterium_verdict(hv_test), "opterium_verdict")

# mapper functions
m = mod.function("geo_to_opterium_hv",
    "geo_to_opterium_hv(ghv) -> OpHealthVector|None",
    "Convert geo HealthVector to opterium format")
ghv = GeoHealthVector(0.1, 0.2, 0.0, 0.05, 0.3, 0.15, 0.01)
ohv = geo_to_opterium_hv(ghv)
m.example(ghv, str(ohv)[:50] if ohv else "None (opterium unavailable)",
          "HV conversion")

m = mod.function("opterium_to_geo_hv",
    "opterium_to_geo_hv(ohv) -> GeoHealthVector",
    "Convert opterium HealthVector to geo format")

# ROUTE_REGISTRY
m = mod.function("ROUTE_REGISTRY",
    "dict of route_name -> info",
    "All registered routes with domains")
m.example((), {k: v['domain'] for k,v in ROUTE_REGISTRY.items()}, "route domains")

# ── 10. geoformer ────────────────────────────────────
from geoformer import GeoFormer, GeometricBlock, GeometricEmbedding, SwarmTrainer, doctor_judge
mod = collector.module("geoformer", "src/geoformer.py")

cls = mod.class_("Pt", "GeoFormer Pt (inherits methods.Pt, adds .zero())")
from geoformer import Pt as GeoPt
m = cls.method("zero", "zero() -> Pt", "Zero point: Pt(0,1)")
m.example((), str(GeoPt.zero()), "zero()")

# Inherited methods from methods.Pt
m = cls.method("from_int", "from_int(v) -> Pt", "Integer → Pt (inherited)")
p = GeoPt.from_int(42)
m.example(42, f"Pt({p.x},{p.y}) P={p.P}", "from_int(42)")

m = cls.method("from_real", "from_real(r) -> Pt", "Float → Pt (inherited)")
p = GeoPt.from_real(0.347)
m.example(0.347, f"Pt({p.x},{p.y})", "from_real(0.347)")

m = cls.method("from_decimal", "from_decimal(d) -> Pt", "Decimal → Pt (inherited)")
p = GeoPt.from_decimal(Decimal('3.14'))
m.example("3.14", f"Pt({p.x},{p.y})", "from_decimal(3.14)")

m = cls.method("from_sd", "from_sd(S,D) -> Pt", "Construct from (S,D) (inherited)")
p = GeoPt.from_sd(8, -2)
m.example((8,-2), f"Pt({p.x},{p.y})", "from_sd(8,-2)")

m = cls.method("inv", "inv() -> Pt", "Inverse (inherited)")
pinv = GeoPt(2,1).inv()
m.example("Pt(2,1)", f"Pt({pinv.x},{pinv.y})", "inv(Pt(2,1))")

m = cls.method("parse", "parse(s) -> Pt", 'Parse mantissa|rank| notation (inherited)')
pp = GeoPt.parse("347|3|")
m.example("347|3|", f"Pt({pp.x},{pp.y})", 'parse("347|3|")')

m = cls.method("to_decimal", "to_decimal() -> Decimal", "Pt → Decimal (inherited)")
p_dec = GeoPt(347,3)
m.example("Pt(347,3)", str(p_dec.to_decimal()), "to_decimal()")

m = cls.method("to_real", "to_real() -> float", "Pt → float (inherited)")
p_real = GeoPt(347,3)
m.example("Pt(347,3)", p_real.to_real(), "to_real()")

m = cls.method("verbose", "verbose() -> str", "Full description (inherited)")
p_verb = GeoPt(347,3)
m.example((), p_verb.verbose(), "verbose()")

cls = mod.class_("GeometricEmbedding", "Token → Pt(S,D) embedding")
emb = GeometricEmbedding()
m = cls.method("embed", "embed(token) -> Pt", "Map token to Pt")
m.example(42, str(emb.embed(42)), "embed(42)")

m = cls.method("embed_sequence", "embed_sequence(tokens) -> List[Pt]", "Batch embed")
m.example([1,2,3], [str(p) for p in emb.embed_sequence([1,2,3])], "embed([1,2,3])")

cls = mod.class_("GeometricBlock", "One GeoFormer layer: Resonate→Project→Shift")
block = GeometricBlock(window=10)
tokens = [Pt(i+1, 1) for i in range(5)]
out, hv = block.forward(tokens)
m = cls.method("forward", "forward(tokens) -> (List[Pt], HealthVector)", "Forward pass")
m.example(f"5 tokens: [{','.join(str(p.P) for p in tokens)}]",
          {"output": [str(p.P) for p in out], "hv_ok": hv.ok},
          "block forward")

cls = mod.class_("GeoFormer", "Full architecture: embedding + stacked blocks")
gf = GeoFormer(layers=2, window=10)
m = cls.method("forward", "forward(tokens) -> (List[Pt], HealthVector)", "Full forward pass")
out, hv = gf.forward([1,2,3,4,5])
m.example([1,2,3,4,5], {"out": [str(p.P) for p in out], "hv_ok": hv.ok},
          "GeoFormer forward")

cls = mod.class_("SwarmTrainer", "Swarm reinforcement trainer (no backprop)")
trainer = SwarmTrainer(gf)
m = cls.method("train_step", "train_step(tokens, target) -> dict", "One training episode")
result = trainer.train_step([1,2,3], [2,4,6])
m.example(([1,2,3],[2,4,6]),
          {k: v for k,v in result.items() if k != 'output'},
          "train_step result")

m = cls.method("train", "train(dataset, epochs) -> List[Dict]", "Multi-epoch training")
dataset = [([1,2,3],[2,4,6])]
results = trainer.train(dataset, epochs=2)
m.example((dataset, 2), [{"episode": r['episode'], "score": r['score']} for r in results],
          "2 epochs")

m = mod.function("doctor_judge",
    "doctor_judge(output, target, hv) -> str",
    "Quick geometric doctor: OK/WARN/FAIL")
m.example((out, [1,2,3,4,5], hv),
          doctor_judge(out, [1,2,3,4,5], hv),
          "doctor_judge verdict")

# ═══════════════════════════════════════════════════════════
# SAVE
# ═══════════════════════════════════════════════════════════
OUT_DIR = os.path.join(SRC, 'spec-kit')
OUT_FILE = os.path.join(OUT_DIR, "spec_compiled.json")

with open(OUT_FILE, 'w', encoding='utf-8') as f:
    json.dump(collector.data, f, indent=2, ensure_ascii=False, default=str)

print(f"\n✅ Spec saved: {OUT_FILE}")
print(f"   Modules: {len(collector.data)}")
for name, data in collector.data.items():
    classes = len(data.get("classes", {}))
    funcs = len(data.get("functions", {}))
    print(f"      {name}: {classes} classes, {funcs} functions")
    
    # Count examples
    total_examples = 0
    total_edges = 0
    for cls_name, cls_data in data.get("classes", {}).items():
        for m_name, m_data in cls_data.get("methods", {}).items():
            total_examples += len(m_data.get("examples", []))
            total_edges += len(m_data.get("edges", []))
    for f_name, f_data in data.get("functions", {}).items():
        total_examples += len(f_data.get("examples", []))
        total_edges += len(f_data.get("edges", []))
    print(f"      => {total_examples} examples, {total_edges} edge cases")
