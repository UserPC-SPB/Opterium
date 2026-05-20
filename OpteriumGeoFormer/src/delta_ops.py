"""
delta_ops.py  —  Canonical Δ-operator library
Source: bootloader.txt lines 51-100, AI_BIOS.txt lines 33-40

The only rule:
  Δ maps (state, params) → (state', HealthVector)
  Composition Δ_out = Δ_A ◦ Δ_B  iff  codomain(Δ_B) ⊆ domain(Δ_A)

Every Δ carries:
  .domain       — algebraic domain (ℝ,ℂ,ℍ,𝕆,𝕊,ℳ_OP,E8,F16)
  .codomain     — output algebra
  .arity        — number of state arguments
  .assoc        — associative_flag
  .commut       — commutative_flag
  .invertible   — True/False
  .norm_preserving
  .entropy_estimate
"""
from __future__ import annotations
import math, time, functools
from typing import Any, Callable, Dict, List, Optional, Sequence, Tuple, Union

# ───────────────────────────────────────────────────────
# Health Vector — every Δ returns one
# ───────────────────────────────────────────────────────
@functools.total_ordering
class HealthVector:
    """7-channel cognitive stability monitor (bootloader line 55, AI_BIOS K1.3)."""
    __slots__ = ('E_assoc','E_commut','E_closure','E_precision',
                 'E_entropy','E_tension','PPH')
    def __init__(self, E_assoc=0.0, E_commut=0.0, E_closure=0.0,
                 E_precision=0.0, E_entropy=0.0, E_tension=0.0, PPH=0.0):
        self.E_assoc = float(E_assoc)
        self.E_commut = float(E_commut)
        self.E_closure = float(E_closure)
        self.E_precision = float(E_precision)
        self.E_entropy = float(E_entropy)
        self.E_tension = float(E_tension)
        self.PPH = float(PPH)

    @property
    def ok(self) -> bool:
        return all(getattr(self, c) < 0.35 for c in self.__slots__)

    @property
    def warn(self) -> bool:
        return any(0.35 <= getattr(self, c) < 0.65 for c in self.__slots__)

    @property
    def critical(self) -> bool:
        return any(getattr(self, c) >= 0.65 for c in self.__slots__)

    @property
    def max_channel(self) -> Tuple[str, float]:
        return max(((c, getattr(self, c)) for c in self.__slots__), key=lambda x: x[1])

    def merge(self, other: HealthVector) -> HealthVector:
        return HealthVector(*(max(a, b) for a, b in zip(self._vec(), other._vec())))

    def _vec(self): return tuple(getattr(self, c) for c in self.__slots__)

    def __eq__(self, o): return self._vec() == o._vec() if isinstance(o, HealthVector) else NotImplemented
    def __lt__(self, o):
        if not isinstance(o, HealthVector): return NotImplemented
        return self._vec() < o._vec()

    def __repr__(self):
        ch, v = self.max_channel
        return f"HV({ch}={v:.4f}, ok={self.ok})"

HEALTH_OK   = HealthVector(0,0,0,0,0,0,0)
HEALTH_WARN = HealthVector(0.35,0,0,0,0,0,0)

# ───────────────────────────────────────────────────────
# Δ Operator base
# ───────────────────────────────────────────────────────
class DeltaOp:
    """One geometric transformation. Immutable after creation."""
    def __init__(self, name: str, *, domain: str, codomain: Optional[str]=None,
                 arity: int=1, assoc: bool=True, commut: bool=True,
                 invertible: bool=True, norm_preserving: bool=False,
                 entropy_estimate: float=0.0,
                 fn: Optional[Callable]=None):
        self.name = name
        self.domain = domain
        self.codomain = codomain or domain
        self.arity = arity
        self.assoc = assoc
        self.commut = commut
        self.invertible = invertible
        self.norm_preserving = norm_preserving
        self.entropy_estimate = entropy_estimate
        self._fn = fn

    def __call__(self, *args, **kw) -> Tuple[Any, HealthVector]:
        if self._fn:
            return self._fn(*args, **kw)
        raise NotImplementedError(f"{self.name} has no callable")

    def __rshift__(self, other: DeltaOp) -> DeltaOp:
        """Δ_A >> Δ_B = Δ_B ◦ Δ_A  (pipe: apply self then other)"""
        return compose_sequential(self, other)

    def __or__(self, other: DeltaOp) -> DeltaOp:
        """Δ_A | Δ_B = parallel composition over disjoint subspaces"""
        return compose_parallel(self, other)

    def inv(self) -> Optional[DeltaOp]:
        if not self.invertible:
            return None
        return DeltaOp(f"inv({self.name})", domain=self.codomain, codomain=self.domain,
                       arity=self.arity, assoc=self.assoc, commut=self.commut,
                       invertible=True, entropy_estimate=self.entropy_estimate)

    def __repr__(self):
        return f"Δ_{self.name}({self.domain}→{self.codomain})"

# ───────────────────────────────────────────────────────
# Composition (bootloader Δ_LIB_COMPOSE_V1 lines 86-99)
# ───────────────────────────────────────────────────────
class CompositeDelta(DeltaOp):
    def __init__(self, name: str, ops: Sequence[DeltaOp], mode: str='sequential'):
        self.ops = tuple(ops)
        self.mode = mode  # 'sequential' | 'parallel'
        domain = ops[0].domain if mode == 'sequential' else '⊕'.join(o.domain for o in ops)
        codomain = ops[-1].codomain if mode == 'sequential' else '⊕'.join(o.codomain for o in ops)
        arity = max(o.arity for o in ops)
        assoc = all(o.assoc for o in ops)
        commut = all(o.commut for o in ops)
        ne = sum(o.entropy_estimate for o in ops)
        super().__init__(name, domain=domain, codomain=codomain, arity=arity,
                         assoc=assoc, commut=commut,
                         invertible=all(o.invertible for o in ops),
                         entropy_estimate=ne)

    def __call__(self, *args, **kw) -> Tuple[Any, HealthVector]:
        if self.mode == 'sequential':
            state, hv = self.ops[0](*args, **kw)
            for op in self.ops[1:]:
                s2, h2 = op(state)
                hv = hv.merge(h2)
                state = s2
            return state, hv
        results = [op(*args, **kw) for op in self.ops]
        hv = functools.reduce(lambda a, b: a.merge(b), (r[1] for r in results))
        return tuple(r[0] for r in results), hv

def compose_sequential(*ops: DeltaOp) -> CompositeDelta:
    if not ops:
        raise ValueError("need >=1 op")
    if len(ops) == 1:
        return CompositeDelta(ops[0].name, ops)
    names = '○'.join(o.name for o in ops)
    for i in range(len(ops)-1):
        if ops[i].codomain != ops[i+1].domain:
            raise ValueError(f"type mismatch: {ops[i]} >> {ops[i+1]}")
    return CompositeDelta(names, ops, mode='sequential')

def compose_parallel(*ops: DeltaOp) -> CompositeDelta:
    if not ops:
        raise ValueError("need >=1 op")
    names = '⊗'.join(o.name for o in ops)
    return CompositeDelta(names, ops, mode='parallel')

# ───────────────────────────────────────────────────────
# Algebraic domain classification (bootloader line 111)
# ───────────────────────────────────────────────────────
ALGEBRA_MATRIX = {
    'R':  {'assoc':True,  'commut':True,  'div':True,  'dim':1,  'parent':None},
    'C':  {'assoc':True,  'commut':True,  'div':True,  'dim':2,  'parent':'R'},
    'H':  {'assoc':True,  'commut':False, 'div':True,  'dim':4,  'parent':'C'},
    'O':  {'assoc':False, 'commut':False, 'div':True,  'dim':8,  'parent':'H'},
    'S':  {'assoc':False, 'commut':False, 'div':False, 'dim':16, 'parent':'O'},
    'F16':{'assoc':False, 'commut':False, 'div':False, 'dim':16, 'parent':'O'},
    'F32':{'assoc':False, 'commut':False, 'div':False, 'dim':32, 'parent':'F16'},
    'E8': {'assoc':True,  'commut':False, 'div':False, 'dim':8,  'parent':None},
    'M_OP':{'assoc':True, 'commut':True,  'div':False, 'dim':0,  'parent':None},
}

def check_domain(domain: str, assoc: bool=None, commut: bool=None, div: bool=None) -> bool:
    info = ALGEBRA_MATRIX.get(domain)
    if not info:
        return True
    if assoc is not None and info['assoc'] != assoc:
        return False
    if commut is not None and info['commut'] != commut:
        return False
    if div is not None and info['div'] != div:
        return False
    return True

# ───────────────────────────────────────────────────────
# Fallback strategies (bootloader Δ_FALLBACK_STRATEGIES)
# ───────────────────────────────────────────────────────
FALLBACK_MAP = {
    'S':  {'Δ_INV': 'Δ_ROBUST_INV', 'Δ_MUL': 'Δ_EMBED_H'},
    'F16':{'Δ_INV': 'Δ_ROBUST_INV', 'Δ_MUL': 'Δ_EMBED_H'},
    'O':  {'Δ_MUL': 'Δ_MUL_TERNARY'},
}

def select_fallback(op_name: str, domain: str) -> Optional[str]:
    return FALLBACK_MAP.get(domain, {}).get(op_name)

# ───────────────────────────────────────────────────────
# Built-in Δ operators
# ───────────────────────────────────────────────────────

# Δ_ADD — vector translation
def _add(a: float, b: float, **kw) -> Tuple[float, HealthVector]:
    return a + b, HealthVector(0, 0, 0, 0, 0, abs(a+b-a-b)*1e-15, 0)
DELTA_ADD = DeltaOp('ADD', domain='R', arity=2, fn=_add)

# Δ_MUL — scaling (bootloader: Δ_MUL○Δ_MUL associative in ℝℂℍ, alt in 𝕆)
def _mul(a: float, b: float, **kw) -> Tuple[float, HealthVector]:
    p = a * b
    assoc_err = 0.0
    return p, HealthVector(assoc_err, 0, 0, abs(p - a*b)*1e-15, 0, 0, 0)
DELTA_MUL = DeltaOp('MUL', domain='R', arity=2, assoc=True, commut=True, fn=_mul)

# Δ_INV — inversion = balance restoration (bootloader lines 132-141)
def _inv_scalar(x: float, **kw) -> Tuple[float, HealthVector]:
    if abs(x) < 1e-300:
        return float('inf'), HealthVector(0, 0, 1, 1, 1, 1, 1)  # closure fail
    inv = 1.0 / x
    resid = abs(x * inv - 1.0)
    return inv, HealthVector(0, 0, 0, resid, 0, 0, resid)
DELTA_INV = DeltaOp('INV', domain='R', arity=1, invertible=True, fn=_inv_scalar)

# Δ_INV_NS — non-singular inversion for zero divisors (bootloader line 66)
def _inv_ns(v: Tuple[float,...], **kw) -> Tuple[Tuple[float,...], HealthVector]:
    nrm = sum(x*x for x in v)
    if nrm > 1e-300:
        inv = tuple(x/nrm for x in v)
        return inv, HealthVector(0, 0, 0, abs(sum(i*j for i,j in zip(v,inv))-1), 0, 0, 0)
    vv = list(v)
    if vv:
        vv[0] += 0.001
        nrm2 = sum(x*x for x in vv)
        inv = tuple(x/nrm2 for x in vv)
        return inv, HealthVector(0.1, 0, 0.5, abs(sum(i*j for i,j in zip(v,inv))-1), 0.5, 0.5, 0.5)
DELTA_INV_NS = DeltaOp('INV_NS', domain='S', arity=1, invertible=False,
                       assoc=False, commut=False, fn=_inv_ns)

# Δ_PPH — analytic projection residue (bootloader line 73)
def _pph_analytic(singular_values: Sequence[float], **kw) -> Tuple[float, HealthVector]:
    sv = [max(1e-300, abs(s)) for s in singular_values]
    vol = math.prod(sv)
    pph = -math.log(vol) if vol > 0 else float('inf')
    return pph, HealthVector(0, 0, 0, 0, 0, 0, min(pph/10, 1))
DELTA_PPH = DeltaOp('PPH', domain='R', arity=1, norm_preserving=False, fn=_pph_analytic)

# Δ_OPTG — Weyl flow geodesic descent (bootloader line 69)
def _optg_weyl(state: List[float], attractor: List[float], **kw) -> Tuple[List[float], HealthVector]:
    d = len(state)
    path = 0
    s = list(state)
    tension0 = sum((a-b)**2 for a,b in zip(s, attractor))
    for _ in range(d * 2):
        best = None
        best_t = tension0
        for axis in range(d):
            s[axis] = -s[axis]
            t = sum((a-b)**2 for a,b in zip(s, attractor))
            if t < best_t:
                best_t = t
                best = axis
            s[axis] = -s[axis]
        if best is not None:
            s[best] = -s[best]
            path += 1
        else:
            break
    tension = sum((a-b)**2 for a,b in zip(s, attractor))
    return s, HealthVector(0, 0, 0, 0, tension/tension0 if tension0 else 0, tension, 0)
DELTA_OPTG = DeltaOp('OPTG', domain='E8', arity=2, norm_preserving=True, fn=_optg_weyl)

# Δ_SHIFT — scale shift (topological_shift from AI_BIOS K0.1)
def _shift(val: float, power: int=0, **kw) -> Tuple[float, HealthVector]:
    return val * (10 ** power), HEALTH_OK
DELTA_SHIFT = DeltaOp('SHIFT', domain='R', arity=2, norm_preserving=False, fn=_shift)

# Δ_ROT — phase rotation (orientation shift, bootloader line 1229)
def _rot(val: complex, angle_deg: float=0.0, **kw) -> Tuple[complex, HealthVector]:
    theta = math.radians(angle_deg)
    r = complex(math.cos(theta), math.sin(theta))
    return val * r, HealthVector(0, 0, 0, abs(abs(r)-1), 0, 0, 0)
DELTA_ROT = DeltaOp('ROT', domain='C', arity=2, norm_preserving=True, fn=_rot)

# Δ_ZERO_DIVISOR_DETECT — detect zero divisors (bootloader Δ_ZeroDivisor_Detect)
def _zero_detect(v: Tuple[float,...], **kw) -> Tuple[bool, HealthVector]:
    nrm = sum(x*x for x in v)
    is_zd = nrm < 1e-300 and any(abs(x) > 1e-300 for x in v)
    return is_zd, HEALTH_OK
DELTA_ZERO_DETECT = DeltaOp('ZERO_DIVISOR_DETECT', domain='S', arity=1,
                            invertible=False, assoc=False, fn=_zero_detect)

# ───────────────────────────────────────────────────────
# Fusion table — fused ops cost less (bootloader Δ_FUSION_TABLE)
# ───────────────────────────────────────────────────────
FUSION_TABLE = {
    'FMA': DeltaOp('FMA', domain='R', arity=3, assoc=False, commut=False,
                    entropy_estimate=1.2),
    'SCALE_ROT': DeltaOp('SCALE_ROT', domain='C', arity=3, norm_preserving=False,
                          entropy_estimate=1.5),
}

# ───────────────────────────────────────────────────────
# Convenience
# ───────────────────────────────────────────────────────
def identity(domain: str = 'R') -> DeltaOp:
    def _id(x): return x, HEALTH_OK
    return DeltaOp('Id', domain=domain, fn=_id)

STANDARD_LIBRARY = {
    'ADD': DELTA_ADD,
    'MUL': DELTA_MUL,
    'INV': DELTA_INV,
    'INV_NS': DELTA_INV_NS,
    'PPH': DELTA_PPH,
    'OPTG': DELTA_OPTG,
    'SHIFT': DELTA_SHIFT,
    'ROT': DELTA_ROT,
    'ZERO_DETECT': DELTA_ZERO_DETECT,
    'FMA': FUSION_TABLE['FMA'],
    'SCALE_ROT': FUSION_TABLE['SCALE_ROT'],
}

# ───────────────────────────────────────────────────────
# Self-test
# ───────────────────────────────────────────────────────
def selftest():
    assert DELTA_ADD.name == 'ADD'
    assert compose_sequential(DELTA_ADD, DELTA_MUL).mode == 'sequential'
    r, hv = DELTA_INV(2.0)
    assert abs(r - 0.5) < 1e-15
    assert hv.ok

    r, hv = DELTA_INV_NS((0.0, 1.0, 0.0))
    assert abs(sum(x*x for x in r) - 1) < 1e-10

    pph, hv = DELTA_PPH([1.0, 0.5, 0.2])
    assert pph > 0  # contraction → positive residue

    seq = DELTA_SHIFT >> DELTA_INV
    assert 'Id' not in seq.name
    r, hv = seq(5.0, power=1)
    assert abs(r - 0.02) < 1e-15
    assert hv.ok

    assert check_domain('O', assoc=False, commut=False, div=True)
    assert not check_domain('O', assoc=True)
    assert select_fallback('Δ_INV', 'S') == 'Δ_ROBUST_INV'
    print("  Δ-library: all tests pass")

if __name__ == '__main__':
    selftest()
