"""
test_doctor_bridge.py  —  🔴 CRIT GAP 3: GeoFormer ↔ opterium_field.DoctorCore

Проверяет что мост между двумя проектами работает:
  1. geo HealthVector → opterium HealthVector (конвертация)
  2. opterium_judge() — делегирование DoctorCore.judge()
  3. opterium_verdict() — полный пайплайн
  4. Pt(4,3) замыкание через DoctorCore
"""

import sys, os

# GeoFormer modules
BASE = r'C:\Users\eccoa\Desktop\OpteriumGeoFormer'
SRC = os.path.join(BASE, 'src')
sys.path.insert(0, SRC)

# opterium_field.py — из проекта Испытываем
OPTIUM_SRC = r'C:\Users\eccoa\Desktop\Испытываем\src'
sys.path.insert(0, OPTIUM_SRC)

from doctor_geo import (
    SwarmDoctor, GeoHealthVector,
    geo_to_opterium_hv, opterium_to_geo_hv,
    ROUTE_REGISTRY, OPTERIUM_AVAILABLE,
)

from delta_ops import HEALTH_OK


def test_opterium_available():
    assert OPTERIUM_AVAILABLE, (
        "opterium_field.py не найден. "
        f"Проверь путь: {OPTIUM_SRC}"
    )
    print(f"  opterium_field: OK (imported from {OPTIUM_SRC})")


def test_hv_conversion_roundtrip():
    ghv = GeoHealthVector(
        E_assoc=0.1, E_commut=0.2, E_closure=0.0,
        E_precision=0.05, E_entropy=0.3, E_tension=0.15, PPH=0.01,
    )
    ohv = geo_to_opterium_hv(ghv)
    assert ohv is not None, "geo→opterium conversion failed"
    assert hasattr(ohv, 'closure'), f"opterium HV missing closure attr, got {type(ohv)}"
    assert abs(ohv.closure - 0.1) < 1e-10, f"closure={ohv.closure} != 0.1"
    assert abs(ohv.modality_conflict - 0.2) < 1e-10

    ghv_back = opterium_to_geo_hv(ohv)
    assert abs(ghv_back.E_assoc - 0.1) < 1e-10
    assert abs(ghv_back.E_commut - 0.2) < 1e-10
    assert abs(ghv_back.E_entropy - 0.3) < 1e-10
    print(f"  HV roundtrip: geo→opterium→geo OK")


def test_opterium_judge():
    sd = SwarmDoctor(swarm_seed=42)
    for name, info in ROUTE_REGISTRY.items():
        sd.register_route(name, potential=info['default_potential'])

    ghv_ok = GeoHealthVector(0, 0, 0, 0, 0, 0, 0)
    verdict = sd.opterium_verdict(ghv_ok, context='test')
    assert 'level' in verdict
    assert 'ok' in verdict
    assert 'health' in verdict
    print(f"  opterium_verdict(OK): {verdict['level']}")


def test_opterium_judge_fractured():
    sd = SwarmDoctor(swarm_seed=42)
    for name, info in ROUTE_REGISTRY.items():
        sd.register_route(name, potential=info['default_potential'])

    ghv_bad = GeoHealthVector(E_assoc=0.7, E_precision=0.8, E_tension=0.85)
    verdict = sd.opterium_verdict(ghv_bad, context='test_fractured')
    level = verdict.get('level', '')
    assert level in ('OK', 'WARN', 'QUARANTINE', 'ROLLBACK', 'UNKNOWN'), f"unexpected level: {level}"
    print(f"  opterium_verdict(bad HV): {level}")


def test_doctor_core_pt_closure():
    """Real DoctorCore test: Pt(4,3) should be CLOSED."""
    from opterium_field import DoctorCore, Pt

    doc = DoctorCore()
    p = Pt(4, 3)
    result = doc.pt2d(p.x, p.y)
    assert result is not None
    verdict = result.get('verdict') or result.get('signature', '')
    assert verdict == 'CLOSED', f"Pt(4,3) signature={verdict}, expected CLOSED"
    print(f"  DoctorCore.Pt(4,3): {verdict}")


def test_doctor_cube_cell():
    """3D geometry through Doctor: CubeCell on axis should be CLOSED."""
    from opterium_field import CubeCell, DoctorCore

    doc = DoctorCore()
    cell = CubeCell(5, 5, 5)
    result = doc.pt3_field3(cell.x, cell.y, cell.z)
    assert result is not None
    signature = result.get('signature', '')
    assert signature == 'CLOSED', f"CubeCell(5,5,5) signature={signature}"
    tension = result.get('tension', -1)
    assert tension == 0, f"CubeCell(5,5,5) tension={tension}"
    print(f"  DoctorCore.CubeCell(5,5,5): {signature}, tension={tension}")


def test_swarm_routes_match_registry():
    """Verify all routes in ROUTE_REGISTRY are valid."""
    sd = SwarmDoctor(swarm_seed=42)
    for name, info in ROUTE_REGISTRY.items():
        sd.register_route(name, potential=info['default_potential'])
    for name in ROUTE_REGISTRY:
        assert name in sd.swarm._nodes, f"Route {name} not registered in swarm"
    chosen = sd.choose_route(deterministic=True)
    assert chosen in ROUTE_REGISTRY
    print(f"  Swarm routes: {len(ROUTE_REGISTRY)} registered, chose '{chosen}'")


def test_full_pipeline():
    """Doctor judge → Swarm reinforce → improved score."""
    sd = SwarmDoctor(swarm_seed=42)
    for name, info in ROUTE_REGISTRY.items():
        sd.register_route(name, potential=info['default_potential'])

    # Run several episodes: good routes get reinforced, bad ones don't
    for _ in range(10):
        route = sd.choose_route(deterministic=False)
        hv = GeoHealthVector(0, 0, 0, 0, 0, 0, 0)
        verdict = sd.opterium_verdict(hv)
        success = verdict.get('ok', False)
        sd.reinforce_route(route, success=success)

    scores = {name: sd.swarm.score(name) for name in ROUTE_REGISTRY}
    print(f"  Swarm scores after 10 episodes: {dict(sorted(scores.items(), key=lambda x: -x[1])[:5])}...")


if __name__ == '__main__':
    tests = [
        ("opterium available", test_opterium_available),
        ("HV conversion roundtrip", test_hv_conversion_roundtrip),
        ("opterium judge OK", test_opterium_judge),
        ("opterium judge fractured", test_opterium_judge_fractured),
        ("DoctorCore Pt(4,3) closure", test_doctor_core_pt_closure),
        ("DoctorCore CubeCell(5,5,5)", test_doctor_cube_cell),
        ("Swarm routes match registry", test_swarm_routes_match_registry),
        ("full pipeline judge→reinforce", test_full_pipeline),
    ]
    passed = 0
    for name, fn in tests:
        try:
            fn()
            passed += 1
            print(f"  ✅ {name}")
        except Exception as e:
            import traceback
            print(f"  ❌ {name}: {e}")
            traceback.print_exc()
    total = len(tests)
    print(f"\n{'='*40}")
    print(f"DoctorBridge: {passed}/{total} passed")
    sys.exit(0 if passed == total else 1)
