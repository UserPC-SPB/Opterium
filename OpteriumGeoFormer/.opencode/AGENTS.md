# AGENTS  —  Opterium GeoFormer

Контекст для загрузки новой AI-сессии.

## Структура

```
Desktop/OpteriumGeoFormer/
├── README.md              ← начать отсюда
├── docs/
│   ├── PARADIGM.md        ← парадигма: 5 аксиом, Φ-таблица, Δ-таблица, 19 констант
│   ├── ARCHITECTURE.md    ← GeoFormer: zero-MM архитектура
│   ├── spec-kit-spec.md   ← spec-kit: что хотели
│   ├── spec-kit-plan.md   ← spec-kit: как делали
│   └── spec-kit-tasks.md  ← spec-kit: задачи
├── src/
│   ├── delta_ops.py       ← Δ-операторы (8 шт, композиция, fallback, HealthVector)
│   ├── phi_algebra.py     ← Φ-алгебра (5 глаголов, Φ-пути)
│   ├── swarm.py           ← Intelligent Swarm V2 (замена Байеса)
│   ├── hashgrid.py        ← O(1) neighbor lookup в (S,D)
│   ├── geoformer.py       ← GeoFormer (GeometricBlock, SwarmTrainer)
│   ├── doctor_geo.py      ← SwarmDoctor (DoctorCore + IntelligentSwarm)
│   ├── e8_twist.py        ← TWIST 2520-cycle, triality, 70.1° closure
│   ├── spec-kit/          ← 5 методов geometric MM + тесты
│   └── geo_matmul_rs/     ← Rust/PyO3: 1.38× быстрее torch на CPU
├── tests/
│   ├── run_all.py         ← мастер-раннер всех тестов
│   └── TEST_COVERAGE.md   ← что протестировано, где gaps
└── results/
    ├── benchmark.csv      ← сырые замеры
    └── SUMMARY.md         ← анализ и выводы
```

## Загрузка

```python
import sys; sys.path.insert(0, 'путь/к/OpteriumGeoFormer/src')

from delta_ops import *
from phi_algebra import *
from swarm import IntelligentSwarm
from hashgrid import geometric_attention, HashGrid
from geoformer import GeoFormer, GeometricBlock
from doctor_geo import SwarmDoctor
from e8_twist import TwistEngine

# Проверка что всё живое
from tests.run_all import collect_tests
tests = collect_tests()
print(f"Загружено модулей: 7, тестов: {len(tests)}")
```

## Ключевые факты

- **Одна формула:** `P = (S²−D²)//4` на любой битности
- **Zero matrix multiply:** GeoFormer не делает MM — hashgrid + Pt3
- **Zero backprop:** Swarm reinforcement вместо SGD
- **Zero GPU:** все операции int add/subtract/lookup
- **Rust бьёт BLAS:** sd_matmul_parallel — 1.38× torch на CPU при n=128

## Что срочно (🔴 CRIT gaps из TEST_COVERAGE.md)

1. GeoFormer end-to-end: forward + SwarmTrainer convergence
2. Multi-layer GeometricBlock тест
3. GeoFormer ↔ opterium_field.DoctorCore интеграция
4. Rust cargo test
5. cross-verify: Python == Cython == Rust

## Что было сделано в последней сессии

- Rust/PyO3 модуль: `geo_matmul_rs` с `sd_matmul`, `sd_matmul_parallel`, `HashGrid`, `geometric_attention`
- Cython v2: flat C arrays, 5-10× быстрее v1
- Все 7 модулей + spec-kit перенесены на Desktop в OpteriumGeoFormer/
- TEST_COVERAGE.md — 67 тестов, 0 падений, 5 🔴 CRIT gaps
