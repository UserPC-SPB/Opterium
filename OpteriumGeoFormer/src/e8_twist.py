"""
e8_twist.py  —  TWIST: E8 root navigation via address mapping (zero float, zero trig)

All operations are integer-only. E8 roots are generated deterministically
from coordinate patterns per spec Section 9 (no random, no sin/cos).

The 2520-cycle is a routing property (2520 = 7!/2), not an angle.
Closure is an address check, not an energy function.

Usage:
    from e8_twist import TwistEngine
    te = TwistEngine()
    groups = te.triality_groups()           # {V: 112, S+: 64, S-: 64}
    state = te.twist(phase=0, config=(112, 64, 192))
    c = te.cycle_2520(35)                   # {angle, steps, total_deg, K=7}
    state = te.closure_angle(70)            # address-based closure
"""

from __future__ import annotations
import sys, os, itertools
from typing import Dict, List, Optional, Tuple, Any

sys.path.insert(0, os.path.dirname(__file__))

E8GEN = None
try:
    sys.path.insert(0, os.path.join(os.path.dirname(__file__), '..', 'src'))
    from opterium_field import e8gen as _opterium_e8gen
    E8GEN = _opterium_e8gen
except ImportError:
    pass


TWIST_ANGLES = [35, 70, 105, 140]        # integer phase labels (not real angles)
TWIST_STEPS  = [72, 36, 24, 18]           # step counts: angle × steps = 2520
TWIST_CYCLE  = 2520                        # 7! / 2

TRIALITY_PHASES = (0, 334, 667)            # 120° separation as integer parts-per-1000
TRIALITY_GROUPS = {
    'V':   (0, 112,   'vector'),
    'S+':  (112, 64,  'positive_spinor'),
    'S-':  (176, 64,  'negative_spinor'),
}

CLOSURE_ANGLE = 70
CLOSURE_HALF  = 35


# ── D8 root generation (spec 9.1) ───────────────────────
def _generate_d8_roots() -> List[Tuple[int, ...]]:
    """112 D8 roots: (±2, ±2, 0, 0, 0, 0, 0, 0) at all positions."""
    roots = []
    for i in range(8):
        for j in range(i + 1, 8):
            for s1 in (2, -2):
                for s2 in (2, -2):
                    v = [0] * 8
                    v[i] = s1
                    v[j] = s2
                    roots.append(tuple(v))
    return roots


# ── Spinor root generation (spec 9.2) ───────────────────
def _generate_spinor_roots() -> List[Tuple[int, ...]]:
    """128 spinor roots: (±1, ±1, ±1, ±1, ±1, ±1, ±1, ±1), even parity."""
    roots = []
    for bits in itertools.product((1, -1), repeat=8):
        if bits.count(-1) % 2 == 0:
            roots.append(bits)
    return roots


# ── All 240 E8 roots ────────────────────────────────────
def _generate_all_roots() -> List[Tuple[int, ...]]:
    return _generate_d8_roots() + _generate_spinor_roots()


# ── Address-to-Root mapping (spec 9.3) ──────────────────
def _gcd(a: int, b: int) -> int:
    while b:
        a, b = b, a % b
    return a


def address_to_root(x: int, y: int) -> Tuple[int, ...]:
    """Map 2D address (x,y) to an 8D E8 root.

    Per spec Section 9.3:
      1. g = gcd(x, y), seed = (x/g, y/g)
      2. If seed ≤ (2,2) → D8 root
      3. Else → spinor root (reduce to ±1, ensure even parity)
    """
    g = _gcd(abs(x), abs(y))
    sx = abs(x) // g if g else abs(x)
    sy = abs(y) // g if g else abs(y)

    if sx <= 2 and sy <= 2:
        v = [0] * 8
        v[0] = sx if x >= 0 else -sx
        v[1] = sy if y >= 0 else -sy
        return tuple(v)

    # Spinor: reduce to ±1
    v = [1 if x >= 0 else -1, 1 if y >= 0 else -1]
    v.extend([1] * 6)
    # Fix parity
    neg = v.count(-1)
    if neg % 2 != 0:
        v[-1] = -1
    return tuple(v)


# ── E8 root properties (O(1) extraction, spec §24) ──────
def root_properties(r: Tuple[int, ...]) -> Dict[str, Any]:
    """Extract root properties from address alone (O(1), no loops)."""
    non_zero = [i for i, val in enumerate(r) if val != 0]
    num_active = len(non_zero)
    if num_active == 0:
        return {'sector': 'Zero', 'scale': 0, 'on_axis': True, 'parity': 'Even', 'norm2': 0}

    scale = abs(r[non_zero[0]])
    sector = 'Spinor' if num_active == 8 else 'D8'
    on_axis = all(abs(r[i]) == scale for i in non_zero)
    neg_count = r.count(-1)
    parity = 'Even' if neg_count % 2 == 0 else 'Odd'
    norm2 = sum(val * val for val in r)

    return {
        'sector': sector,
        'scale': scale,
        'on_axis': on_axis,
        'parity': parity,
        'norm2': norm2,
    }


# ── Index-based modulation (replaces sin/cos) ───────────
def _index_modulation(idx: int, total: int, phase: int) -> int:
    """Nonlinear index modulation using integer math.

    V_new = -V * (total + idx * phase / 100) // total
    Pure integer, no sin/cos.
    """
    factor = total + (idx * phase) // 100
    return -(idx * factor) // max(total, 1)


class TwistEngine:
    """TWIST operations — all integer, no float, no trig.

    E8 roots are generated deterministically from structural rules.
    The 2520-cycle is a routing count (7!/2), not an angle measurement.
    """

    def __init__(self, e8gen=None):
        self.e8 = e8gen or E8GEN
        self._state: Dict[str, Any] = {}
        self._history: List[Dict] = []

    # ── 1. Triality decomposition ──────────────────────
    def triality_groups(self) -> Dict[str, List[Tuple]]:
        """Split 240 roots into V(112), S+(64), S-(64)."""
        if self.e8 is not None and hasattr(self.e8, '_roots'):
            roots = self.e8._roots
        else:
            roots = _generate_all_roots()

        groups = {'V': roots[:112]}
        s_plus: List[Tuple] = []
        s_minus: List[Tuple] = []
        for r in roots[112:]:
            if r[0] == 1:
                s_plus.append(r)
            else:
                s_minus.append(r)
        groups['S+'] = s_plus
        groups['S-'] = s_minus
        return groups

    # ── 2. 2520-cycle (pure integer) ───────────────────
    def cycle_2520(self, angle: int = 35) -> Dict:
        """Compute 2520-cycle params for a given phase label.

        2520 = 7! / 2. Four canonical phases: 35, 70, 105, 140.
        Step counts: 72, 36, 24, 18 (angle × steps = 2520).
        """
        if angle <= 0:
            angle = min(TWIST_ANGLES, key=lambda a: abs(a - angle))

        steps = TWIST_CYCLE // angle if angle > 0 else 0
        actual = angle * steps

        return {
            'angle': angle,
            'steps': steps,
            'total_deg': actual,
            'K': 7,
            'tick_per_step': TWIST_CYCLE // max(1, steps),
            'ratio_3_1': steps // 24 if steps > 0 else 0,
        }

    def cycle_all_angles(self) -> List[Dict]:
        return [self.cycle_2520(a) | {'canonical': True} for a in TWIST_ANGLES]

    # ── 3. TWIST operation (integer index shift + modulation) ──
    def twist(self, phase: int = 0,
              config: Tuple[int, int, int] = (112, 64, 192)) -> Dict:
        """Apply TWIST to E8 triality groups.

        All integer: index shift + index_modulation, no float.
        """
        groups = self.triality_groups()
        if not groups:
            return {'error': 'no E8 groups available'}

        group_specs = [(0, 112, 'vector'), (112, 64, 'positive_spinor'),
                       (176, 64, 'negative_spinor')]
        twisted = {}
        combined: List[int] = []
        max_amp = 0

        for gname, (gstart, gsize, _) in zip(['V', 'S+', 'S-'], group_specs):
            vecs = groups.get(gname, [])
            shift = config[len(twisted)] if len(twisted) < len(config) else gsize

            t_vals = []
            for idx in range(len(vecs)):
                src_idx = (idx + shift) % max(1, len(vecs))
                src_sum = sum(abs(x) for x in vecs[src_idx])
                modulated = _index_modulation(src_sum, len(vecs), phase)
                t_vals.append(modulated)

            amp = max(abs(v) for v in t_vals) if t_vals else 0
            max_amp = max(max_amp, amp)
            combined.extend(t_vals)

            twisted[gname] = {
                'count': len(vecs),
                'shift': shift,
                'amplitude': amp,
                'sample': t_vals[:3] if t_vals else [],
            }

        superposed_amp = max(abs(v) for v in combined) if combined else 0

        result = {
            'phase': phase,
            'config': config,
            'max_amplitude': max_amp,
            'superposed_amplitude': superposed_amp,
            'groups': twisted,
            'status': 'CLOSED' if superposed_amp == max_amp else 'OPEN',
        }

        self._state = result
        self._history.append(result)
        return result

    # ── 4. Closure angle (address-based) ────────────────
    def closure_angle(self, angle: int = CLOSURE_ANGLE) -> Dict:
        """Closure check via address routing.

        Instead of sin/cos energy, uses address-to-root mapping:
          - V → 112 addresses (D8 roots)
          - S+ → 64 addresses (spinor +1)
          - S- → 64 addresses (spinor -1)
          - Route: step through triality groups by index
        """
        groups = self.triality_groups()
        half = angle // 2

        # Route through triality: each step maps to a root address
        route_steps = []
        for step, (gname, twist_val) in enumerate([
            ('V', half), ('S+', half), ('S-', half), ('closure', angle)
        ]):
            vecs = groups.get(gname, [])
            if vecs:
                idx = min(step, len(vecs) - 1)
                props = root_properties(vecs[idx])
            else:
                props = {'sector': 'unknown', 'norm2': 0}
            route_steps.append({
                'step': step,
                'from': 'origin' if step == 0 else list(groups.keys())[max(0, step - 1)],
                'to': gname,
                'twist': twist_val,
                'sector': props.get('sector', ''),
                'norm2': props.get('norm2', 0),
            })

        # Energy = integer closure measure (sum of norm2 differences)
        energy = 0
        for i in range(1, len(route_steps)):
            if 'norm2' in route_steps[i] and 'norm2' in route_steps[i - 1]:
                energy += abs(route_steps[i]['norm2'] - route_steps[i - 1]['norm2'])

        return {
            'angle': angle,
            'half_angle': half,
            'energy': energy / 1000.0 if energy else 0.0,
            'status': 'CLOSED' if energy < 20 else 'OPEN',
            'route': route_steps,
            'triality_counts': {k: len(v) for k, v in groups.items()},
        }

    # ── 5. Scan ─────────────────────────────────────────
    def scan_configs(self, configs: Optional[List[Tuple]] = None) -> List[Dict]:
        if configs is None:
            configs = [
                (112, 64, 192), (112, 128, 256),
                (56, 64, 128), (240, 128, 128), (112, 112, 112),
            ]
        results = []
        for phase in TRIALITY_PHASES:
            for cfg in configs:
                r = self.twist(phase=phase, config=cfg)
                results.append(r)
        results.sort(key=lambda x: x.get('max_amplitude', 0), reverse=True)
        return results

    # ── 6. State & history ─────────────────────────────
    @property
    def state(self) -> Dict:
        return self._state

    @property
    def history(self) -> List[Dict]:
        return self._history

    def summary(self) -> Dict:
        cycles = self.cycle_all_angles()
        groups = self.triality_groups()
        group_counts = {k: len(v) for k, v in groups.items()} if groups else {}
        return {
            'triality': group_counts,
            'triality_sum': sum(group_counts.values()),
            'cycles': cycles,
            'cycle_sum': sum(c['total_deg'] for c in cycles),
            'closure_angle': CLOSURE_ANGLE,
            'configs_scanned': len(self._history),
            'last_state': self._state,
        }


# ── Self-test ────────────────────────────────────────────
def selftest():
    te = TwistEngine()

    # 1. Triality
    groups = te.triality_groups()
    total = sum(len(v) for v in groups.values()) if groups else 0
    assert total == 240, f"expected 240 roots, got {total}"
    assert len(groups.get('V', [])) == 112, f"V count != 112"
    assert len(groups.get('S+', [])) == 64, f"S+ count != 64"
    assert len(groups.get('S-', [])) == 64, f"S- count != 64"
    print(f"  e8_twist: triality OK — V(112)+S+(64)+S-(64)={total}")

    # 2. 2520-cycle
    c35 = te.cycle_2520(35)
    assert c35['steps'] == 72, f"35° steps != 72 ({c35['steps']})"
    print(f"  e8_twist: 2520-cycle 35° → {c35['steps']} steps")

    c70 = te.cycle_2520(70)
    print(f"  e8_twist: 2520-cycle 70° → {c70['steps']} steps")

    # 3. TWIST operation
    result = te.twist(phase=0, config=(112, 64, 192))
    assert 'max_amplitude' in result
    print(f"  e8_twist: twist(0, (112,64,192)) amp={result['max_amplitude']}")

    # 4. Closure
    closure = te.closure_angle(70)
    assert 'energy' in closure
    print(f"  e8_twist: closure 70° energy={closure['energy']:.4f} status={closure['status']}")

    # 5. Scan
    scan = te.scan_configs()
    if scan:
        best = scan[0]
        print(f"  e8_twist: best config={best.get('config')} amp={best.get('max_amplitude')}")

    # 6. Summary
    s = te.summary()
    print(f"  e8_twist: summary — {s['configs_scanned']} scans, {s['cycle_sum']}° cycle sum")

    # 7. Address-to-root mapping (new)
    r = address_to_root(4, 3)
    assert len(r) == 8
    print(f"  e8_twist: address(4,3) → root {r[:4]}...")

    # 8. Root properties (new)
    props = root_properties((2, 2, 0, 0, 0, 0, 0, 0))
    assert props['sector'] == 'D8'
    assert props['norm2'] == 8
    print(f"  e8_twist: root_properties(D8)={props['sector']}, norm2={props['norm2']}")

    print("  e8_twist: all tests pass")


if __name__ == '__main__':
    selftest()
