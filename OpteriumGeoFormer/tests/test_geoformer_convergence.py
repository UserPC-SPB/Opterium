"""
test_geoformer_convergence.py  —  🔴 CRIT GAP 1: GeoFormer end-to-end convergence

Проверяет полный пайплайн обучения:
  1. GeoFormer(s) forward — корректный output
  2. SwarmTrainer.train_step() — структура результата
  3. SwarmTrainer.train() — multi-epoch pipeline
  4. Doctor judge — интеграция с диагностикой
  5. История обучения накапливается

NOTE: SwarmTrainer.reinforce() пока не обновляет веса модели (stub).
      Полная сходимость требует реализации Swarm node updates.
"""

import sys, os
SRC = r'C:\Users\eccoa\Desktop\OpteriumGeoFormer\src'
if SRC not in sys.path:
    sys.path.insert(0, SRC)


def test_forward_pipeline():
    """GeoFormer.forward возвращает корректные типы."""
    from geoformer import GeoFormer, Pt, HealthVector

    gf = GeoFormer(layers=2, window=6)
    out, hv = gf.forward([1, 2, 3, 4])
    assert len(out) == 4
    assert all(isinstance(pt, Pt) for pt in out)
    assert isinstance(hv, HealthVector)
    assert isinstance(hv.ok, bool)
    print(f"  ✅ forward: {len(out)} tokens, hv.ok={hv.ok}")


def test_train_step_structure():
    """SwarmTrainer.train_step возвращает корректную структуру."""
    from geoformer import GeoFormer, SwarmTrainer

    gf = GeoFormer(layers=2, window=6)
    trainer = SwarmTrainer(gf)
    result = trainer.train_step([1, 2, 3], [2, 4, 6])

    for key in ('episode', 'score', 'success', 'hv_ok', 'output'):
        assert key in result, f"Missing key '{key}'"
    assert result['episode'] == 1
    assert isinstance(result['score'], float)
    assert isinstance(result['success'], bool)
    assert isinstance(result['hv_ok'], bool)
    assert isinstance(result['output'], list)
    print(f"  ✅ train_step: episode={result['episode']}, score={result['score']:.3f}, "
          f"success={result['success']}, hv_ok={result['hv_ok']}")


def test_train_multi_epoch():
    """SwarmTrainer.train() работает через много эпох."""
    from geoformer import GeoFormer, SwarmTrainer

    gf = GeoFormer(layers=2, window=6)
    trainer = SwarmTrainer(gf)
    dataset = [([1, 2, 3], [2, 4, 6]), ([4, 5, 6], [8, 10, 12])]
    results = trainer.train(dataset, epochs=5)

    assert len(results) == 10  # 2 samples × 5 epochs
    assert all('epoch' in r for r in results)
    assert all(0 <= r['epoch'] < 5 for r in results)
    assert all(isinstance(r['score'], float) for r in results)
    assert trainer.episode == 10
    assert len(trainer.history) == 10
    print(f"  ✅ multi-epoch: {len(results)} steps, final score={results[-1]['score']:.3f}")


def test_doctor_integration():
    """doctor_judge работает c output от обученной модели."""
    from geoformer import GeoFormer, SwarmTrainer, doctor_judge, HEALTH_OK

    gf = GeoFormer(layers=2, window=6)
    out, hv = gf.forward([1, 2, 3])
    verdict = doctor_judge(out, [1, 2, 3], hv)
    assert verdict in ('OK', 'WARN', 'FAIL'), f"Unexpected verdict: {verdict}"
    print(f"  ✅ doctor_judge: {verdict}")


def test_train_history_accumulates():
    """История обучения правильно накапливается."""
    from geoformer import GeoFormer, SwarmTrainer

    gf = GeoFormer(layers=1, window=4)
    trainer = SwarmTrainer(gf)
    assert len(trainer.history) == 0

    for i in range(3):
        trainer.train_step([1, 2], [2, 4])
        assert len(trainer.history) == i + 1
        assert trainer.history[i]['episode'] == i + 1
    print(f"  ✅ history: {len(trainer.history)} episodes")


def test_score_bounds():
    """Score всегда в [0, 1]."""
    from geoformer import GeoFormer, SwarmTrainer

    gf = GeoFormer(layers=2, window=6)
    trainer = SwarmTrainer(gf)

    for tokens, target in [([1, 2, 3], [100, 200, 300]),
                           ([1, 2, 3], [1, 2, 3]),
                           ([5, 10, 15], [5, 10, 15])]:
        r = trainer.train_step(tokens, target)
        assert 0.0 <= r['score'] <= 1.0, f"Score {r['score']} out of bounds"
        assert isinstance(r['success'], bool)
        assert isinstance(r['hv_ok'], bool)
    print(f"  ✅ score bounds OK, last score={r['score']:.3f}")


# ── main ─────────────────────────────────────────────────
if __name__ == '__main__':
    tests = [v for k, v in sorted(globals().items())
             if k.startswith('test_') and callable(v)]
    passed = 0
    failed = 0
    for fn in tests:
        try:
            fn()
            print(f"  ✅ {fn.__name__}")
            passed += 1
        except Exception as e:
            print(f"  ❌ {fn.__name__}: {e}")
            failed += 1
    print(f"\n{'=' * 40}")
    print(f"Convergence tests: {passed}/{passed + failed} passed")
    if failed:
        exit(1)
