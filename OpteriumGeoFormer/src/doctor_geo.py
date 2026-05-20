"""
doctor_geo.py  —  Swarm-powered DoctorCore integration

Replaces Bayesian routing in DoctorCore with Intelligent Swarm V2.
Doctor still issues verdicts, but route CHOICE is Swarm-driven.

Bridge between:
  - bootstrap/delta_ops.py HealthVector (E_assoc, E_commut, ...)
  - opterium_field.py HealthVector (closure, support_loss, ambiguity, ...)
  - bootstrap/swarm.py IntelligentSwarm (replaces P(H|E))

Usage:
    from doctor_geo import SwarmDoctor
    sd = SwarmDoctor()
    sd.register_route('direct', potential=0.8)
    sd.register_route('farey', potential=0.5)
    chosen = sd.choose_route(hv, context='matrix multiply')
    verdict = sd.judge(hv)
"""

from __future__ import annotations
import sys, os, time
from typing import Dict, List, Optional, Tuple, Any
from dataclasses import dataclass

sys.path.insert(0, os.path.dirname(__file__))
from delta_ops import HealthVector as GeoHealthVector, HEALTH_OK
from swarm import IntelligentSwarm

# ───────────────────────────────────────────────────────
# Import opterium types safely (may not always be available)
# ───────────────────────────────────────────────────────
OPTERIUM_AVAILABLE = False
OpHealthVector = None
OpDoctorCore = None
OpDoctorVerdict = None

try:
    import sys
    sys.path.insert(0, os.path.join(os.path.dirname(__file__), '..', 'src'))
    from opterium_field import (
        HealthVector as OpHealthVector,
        DoctorCore as OpDoctorCore,
        DoctorVerdict as OpDoctorVerdict,
    )
    OPTERIUM_AVAILABLE = True
except ImportError:
    pass


# ───────────────────────────────────────────────────────
# HealthVector bridge — maps between geo and opterium
# ───────────────────────────────────────────────────────
def geo_to_opterium_hv(ghv: GeoHealthVector) -> Any:
    """Convert bootstrap HealthVector → opterium_field HealthVector."""
    if not OPTERIUM_AVAILABLE:
        return None
    # Map: E_assoc → closure, E_precision → support_loss, E_entropy → ambiguity,
    # E_tension → stress, PPH → projection_loss, E_commut → modality_conflict
    return OpHealthVector(
        closure=ghv.E_assoc,
        support_loss=ghv.E_precision,
        ambiguity=ghv.E_entropy,
        drift=ghv.E_tension * 0.5,
        projection_loss=ghv.PPH,
        modality_conflict=ghv.E_commut,
        stress=ghv.E_tension,
    )


def opterium_to_geo_hv(ohv: Any) -> GeoHealthVector:
    """Convert opterium_field HealthVector → bootstrap HealthVector."""
    return GeoHealthVector(
        E_assoc=getattr(ohv, 'closure', 0.0),
        E_commut=getattr(ohv, 'modality_conflict', 0.0),
        E_closure=getattr(ohv, 'support_loss', 0.0),
        E_precision=getattr(ohv, 'drift', 0.0),
        E_entropy=getattr(ohv, 'ambiguity', 0.0),
        E_tension=getattr(ohv, 'stress', 0.0),
        PPH=getattr(ohv, 'projection_loss', 0.0),
    )


# ───────────────────────────────────────────────────────
# SwarmDoctor — replaces Bayesian routing
# ───────────────────────────────────────────────────────
class SwarmDoctor:
    """DoctorCore augmented with Intelligent Swarm for route decisions.

    Doctor judge() still works as before (threshold-based verdicts).
    Swarm routes replace P(H|E) computations.

    Usage:
        sd = SwarmDoctor()
        sd.register_route('e8_direct')
        sd.register_route('cube_project')
        # Judge health
        verdict = sd.judge(geo_hv)
        # Choose route (swarm-powered)
        route = sd.choose_route(['e8_direct', 'cube_project'], context='vision')
        # Reinforce success/failure
        sd.reinforce_route('e8_direct', success=True)
    """

    def __init__(self, warn: float = 0.35, quarantine: float = 0.65,
                 rollback: float = 0.90, swarm_seed: int = 0):
        self.warn = warn
        self.quarantine = quarantine
        self.rollback = rollback
        self.swarm = IntelligentSwarm(seed=swarm_seed)
        self._quarantine: Dict[str, Any] = {}
        self._opterium_doctor = OpDoctorCore(
            warn=warn, quarantine=quarantine, rollback=rollback
        ) if OPTERIUM_AVAILABLE else None

    # ── Route registration ─────────────────────────────
    def register_route(self, name: str, potential: float = 1.0):
        self.swarm.register(name, potential=potential)

    # ── Swarm decision (replaces Bayes) ─────────────────
    def choose_route(self, candidates: Optional[List[str]] = None,
                     context: str = '', deterministic: bool = False) -> str:
        """Choose the best route via Swarm probabilities.

        Instead of computing P(route | health_vector), the Swarm
        compares four drives: exploitation (pheromone), exploration
        (inverse visit count), potential (visit ratio), wisdom (memory).
        """
        return self.swarm.decide(candidates=candidates, deterministic=deterministic).id

    def route_probabilities(self, candidates: Optional[List[str]] = None) -> Dict[str, float]:
        """Get Swarm probability distribution over routes."""
        return self.swarm.probabilities()

    def reinforce_route(self, route: str, success: bool):
        """Reinforce a route after Doctor's verdict.

        Called after each forward pass:
          - success=True:  route led to OK verdict and correct output
          - success=False: route led to WARN/FAIL
        """
        self.swarm.update(route, success=success)

    # ── Doctor judgment ─────────────────────────────────
    def judge(self, hv: GeoHealthVector, context: str = '') -> str:
        """Issue verdict based on HealthVector thresholds.

        Returns: 'OK', 'WARN', 'QUARANTINE', 'ROLLBACK'
        """
        if not hasattr(hv, 'max_channel'):
            return 'OK' if hv.ok else 'WARN'

        mc = hv.max_channel
        if isinstance(mc, tuple):
            _, peak = mc
        else:
            _, peak = mc()

        if peak >= self.rollback:
            return 'ROLLBACK'
        if peak >= self.quarantine:
            return 'QUARANTINE'
        if peak >= self.warn:
            return 'WARN'
        return 'OK'

    def judge_full(self, hv: GeoHealthVector, context: str = '') -> dict:
        """Full verdict dict with reasons."""
        mc = hv.max_channel
        if isinstance(mc, tuple):
            _, peak = mc
        else:
            _, peak = mc()
        level = self.judge(hv, context)

        reasons = []
        if hv.E_assoc >= self.warn:
            reasons.append(f'assoc:{hv.E_assoc:.3f}')
        if hv.E_precision >= self.warn:
            reasons.append(f'precision:{hv.E_precision:.3f}')
        if hv.PPH >= self.warn:
            reasons.append(f'pph:{hv.PPH:.3f}')

        return {
            'level': level,
            'ok': level == 'OK',
            'peak': peak,
            'reasons': reasons,
            'hv': hv,
            'context': context,
        }

    # ── Quarantine ──────────────────────────────────────
    def quarantine_item(self, key: str, payload: Any, verdict: dict):
        self._quarantine[key] = {'payload': payload, 'verdict': verdict, 'time': time.time()}

    def get_quarantine(self, key: str) -> Optional[Any]:
        return self._quarantine.get(key)

    def clear_quarantine(self):
        self._quarantine.clear()

    # ── Opterium bridge ─────────────────────────────────
    def opterium_judge(self, ohv: Any, context: str = '') -> Any:
        """Delegate to opterium DoctorCore if available."""
        if self._opterium_doctor and hasattr(self._opterium_doctor, 'judge'):
            return self._opterium_doctor.judge(ohv, context=context)
        return None

    def opterium_verdict(self, geo_hv: GeoHealthVector, context: str = '') -> dict:
        """Full pipeline: convert geo HV → opterium → judge → return dict."""
        ohv = geo_to_opterium_hv(geo_hv)
        if ohv is None:
            return self.judge_full(geo_hv, context)
        verdict = self.opterium_judge(ohv, context)
        return {
            'level': verdict.level if verdict else 'UNKNOWN',
            'ok': verdict.ok if verdict else False,
            'health': ohv.d() if ohv else {},
        }


# ───────────────────────────────────────────────────────
# Route table — registry of known routes and their functions
# ───────────────────────────────────────────────────────
ROUTE_REGISTRY: Dict[str, dict] = {
    'e8_direct': {
        'description': 'Δ_OPTG Weyl flow in E8',
        'domain': 'E8',
        'default_potential': 0.9,
    },
    'cube_project': {
        'description': 'Cube27 → Field9 → E8 projection',
        'domain': 'Cube27',
        'default_potential': 0.7,
    },
    'farey_path': {
        'description': 'Farey anti-diagonal traversal',
        'domain': 'Farey',
        'default_potential': 0.6,
    },
    'pytable_lookup': {
        'description': 'PyTable direct P = (S²−D²)//4',
        'domain': 'PyTable',
        'default_potential': 0.8,
    },
    'hashgrid_resonate': {
        'description': 'GeoFormer hashgrid attention',
        'domain': 'HashGrid',
        'default_potential': 1.0,
    },
    'doctor_closure': {
        'description': 'Doctor closure triangle (70.1°)',
        'domain': 'E8_Closure',
        'default_potential': 0.85,
    },
    'twist_2520': {
        'description': 'TWIST 2520-cycle (triality)',
        'domain': 'E8_Twist',
        'default_potential': 0.75,
    },
}


# ───────────────────────────────────────────────────────
# Self-test
# ───────────────────────────────────────────────────────
def selftest():
    # 1. SwarmDoctor basic
    sd = SwarmDoctor(swarm_seed=42)
    for name, info in ROUTE_REGISTRY.items():
        sd.register_route(name, potential=info['default_potential'])

    chosen = sd.choose_route(deterministic=True)
    assert chosen in ROUTE_REGISTRY
    print(f"  doctor_geo: Swarm chose '{chosen}'")

    # 2. Route probabilities
    probs = sd.route_probabilities()
    assert abs(sum(probs.values()) - 1.0) < 1e-12
    print("  doctor_geo: route probs sum to 1.0")

    # 3. Judge with geo health vector
    hv_ok = GeoHealthVector(0, 0, 0, 0, 0, 0, 0)
    assert sd.judge(hv_ok) == 'OK'
    print("  doctor_geo: judge OK")

    hv_bad = GeoHealthVector(E_assoc=0.8, E_precision=0.9, E_tension=0.95)
    assert sd.judge(hv_bad) in ('QUARANTINE', 'ROLLBACK')
    print("  doctor_geo: judge bad HV")

    # 4. Reinforce
    sd.reinforce_route('hashgrid_resonate', success=True)
    sd.reinforce_route('e8_direct', success=False)
    p = sd.swarm.score('hashgrid_resonate')
    assert p > 0
    print("  doctor_geo: reinforce OK")

    # 5. Quarantine
    sd.quarantine_item('test', {'data': 42}, {'level': 'WARN'})
    assert sd.get_quarantine('test')['payload']['data'] == 42
    sd.clear_quarantine()
    assert sd.get_quarantine('test') is None
    print("  doctor_geo: quarantine OK")

    print("  doctor_geo: all tests pass")


if __name__ == '__main__':
    selftest()
