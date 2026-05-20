"""
test_geoformer_stacking.py  —  🔴 CRIT GAP 2: multi-layer GeometricBlock stacking

Проверяет что стекинг слоёв в GeoFormer работает корректно:
  1. Разное количество слоёв (0, 1, 3, 5) не падает
  2. Каждый слой трансформирует данные (выход ≠ вход)
  3. HV корректно мержится через слои
  4. Детерминизм: одинаковый seed → одинаковый результат
  5. SwarmTrainer работает с multi-layer моделью
"""

import sys, os, random
SRC = r'C:\Users\eccoa\Desktop\OpteriumGeoFormer\src'
if SRC not in sys.path:
    sys.path.insert(0, SRC)


# ── tests ────────────────────────────────────────────────

def test_stacking_layer_count():
    """GeoFormer должен работать с разным количеством слоёв."""
    from geoformer import GeoFormer, Pt

    for layers in [0, 1, 3, 5]:
        gf = GeoFormer(layers=layers, window=5)
        out, hv = gf.forward([i + 1 for i in range(8)])
        assert len(out) == 8, f"layers={layers}: output length {len(out)} != 8"
        assert isinstance(hv.ok, bool), f"layers={layers}: hv.ok not bool"
        print(f"  ✅ layers={layers} → hv.ok={hv.ok}, output[0].P={out[0].P}")


def test_stacking_transforms():
    """Каждый слой должен изменять данные (не identity)."""
    from geoformer import GeoFormer

    random.seed(42)
    gf = GeoFormer(layers=3, window=5)
    out, hv = gf.forward([i + 1 for i in range(8)])

    # Исходные Pt для тех же чисел
    from geoformer import GeometricEmbedding
    emb = GeometricEmbedding()
    original = emb.embed_sequence([i + 1 for i in range(8)])

    # После 3 слоёв, хотя бы 3/8 токенов должны отличаться от оригинала
    differences = sum(1 for o, p in zip(out, original) if (o.x, o.y) != (p.x, p.y))
    assert differences >= 3, f"Only {differences}/8 tokens differ after 3 layers (should differ)"
    print(f"  ✅ stacking transforms {differences}/8 tokens")


def test_stacking_hv_merge():
    """HV после стекинга слоёв должен быть OK для простых данных."""
    from geoformer import GeoFormer

    gf = GeoFormer(layers=3, window=8)
    out, hv = gf.forward([i + 1 for i in range(8)])
    assert hv.ok, f"hv should be ok"
    print(f"  ✅ hv.ok={hv.ok}, hv.warn={hv.warn}")


def test_stacking_deterministic():
    """Одинаковый seed → одинаковый результат."""
    from geoformer import GeoFormer

    random.seed(123)
    gf1 = GeoFormer(layers=3, window=6)
    out1, _ = gf1.forward([i + 1 for i in range(6)])

    random.seed(123)
    gf2 = GeoFormer(layers=3, window=6)
    out2, _ = gf2.forward([i + 1 for i in range(6)])

    pairs = [(o1.x, o1.y, o2.x, o2.y) for o1, o2 in zip(out1, out2)]
    identical = all(o1.x == o2.x and o1.y == o2.y for o1, o2 in zip(out1, out2))
    assert identical, "Outputs differ with same seed!"
    print(f"  ✅ deterministic: {pairs[:3]}...")


def test_stacking_intermediate():
    """Промежуточные выходы разных слоёв должны отличаться."""
    from geoformer import GeoFormer, GeometricBlock, GeometricEmbedding

    emb = GeometricEmbedding()
    tokens = [i + 1 for i in range(6)]
    pts = emb.embed_sequence(tokens)

    layers_outputs = []
    block1 = GeometricBlock(window=6)
    block2 = GeometricBlock(window=6)
    block3 = GeometricBlock(window=6)

    pts1, hv1 = block1.forward(pts)
    pts2, hv2 = block2.forward(pts1)
    pts3, hv3 = block3.forward(pts2)

    # Каждый слой должен менять хотя бы 1 токен
    changes_1 = sum(1 for a, b in zip(pts, pts1) if (a.x, a.y) != (b.x, b.y))
    changes_2 = sum(1 for a, b in zip(pts1, pts2) if (a.x, a.y) != (b.x, b.y))
    changes_3 = sum(1 for a, b in zip(pts2, pts3) if (a.x, a.y) != (b.x, b.y))

    assert changes_1 > 0, "Layer 1 should change tokens"
    assert changes_2 > 0, "Layer 2 should change tokens"
    assert changes_3 > 0, "Layer 3 should change tokens"
    print(f"  ✅ intermediate changes: layer1={changes_1}, layer2={changes_2}, layer3={changes_3}")


def test_stacking_swarm_trainer():
    """SwarmTrainer должен работать с multi-layer GeoFormer."""
    from geoformer import GeoFormer, SwarmTrainer

    gf = GeoFormer(layers=3, window=6)
    trainer = SwarmTrainer(gf)

    result = trainer.train_step([1, 2, 3], [2, 4, 6])
    assert 'score' in result, "Missing score in train_step result"
    assert 'success' in result, "Missing success in train_step result"
    assert 'hv_ok' in result, "Missing hv_ok in train_step result"
    assert isinstance(result['score'], float)
    print(f"  ✅ SwarmTrainer: score={result['score']:.3f}, success={result['success']}, hv_ok={result['hv_ok']}")


def test_stacking_large():
    """Stacking со многими слоями (8) не должен падать."""
    from geoformer import GeoFormer

    gf = GeoFormer(layers=8, window=4)
    out, hv = gf.forward([i + 1 for i in range(12)])
    assert len(out) == 12
    print(f"  ✅ 8 layers: output length={len(out)}, hv.ok={hv.ok}")


# ── registration ─────────────────────────────────────────
TEST_REGISTRY = []

def test(module, name):
    def decorator(fn):
        TEST_REGISTRY.append((module, name, fn))
        return fn
    return decorator

for fn_name in dir():
    if fn_name.startswith('test_stacking_'):
        fn = globals()[fn_name]
        test('geoformer', fn_name)(fn)


# ── main ─────────────────────────────────────────────────
if __name__ == '__main__':
    passed = 0
    failed = 0
    for module, name, fn in TEST_REGISTRY:
        try:
            fn()
            print(f"  ✅ {name}")
            passed += 1
        except Exception as e:
            print(f"  ❌ {name}: {e}")
            failed += 1
    print(f"\n{'=' * 40}")
    print(f"Stacking tests: {passed}/{passed + failed} passed")
    if failed:
        exit(1)
