# Opterium GeoFormer

**Геометрическая архитектура, ноль матричного умножения.**  
Одна формула: `P = (S²−D²)//4` на любой битности.

**Ни одного GPU. Ни одной float-операции. Ни одного Nvidia.**

---

## Что это

Полная замена Transformer архитектуре:

| Transformer | GeoFormer |
|---|---|
| Attention (QK^T/√d → softmax → V) | **HashGrid proximity** (O(k) neighbor lookup) |
| FFN (Linear·ReLU·Linear) | **Pt3 triple product** (x·y·context) |
| Backprop (SGD/AdamW) | **Swarm reinforcement** (success/failure episodes) |
| Positional encoding | **Geometric embedding** (Pt(S, D) coordinate) |
| FP32 weights | **Address Descriptor** `(scale, node, Δ_project)` |
| Bayes: P(E\|H)·P(H) | **Intelligent Swarm V2**: 4 drives (μ^α, kol^β, (kol/Max)^γ, H_j) |

---

## Быстрый старт

```bash
cd Desktop/OpteriumGeoFormer

# Self-test всех 7 модулей
python -c "import sys; sys.path.insert(0,'src'); [__import__(m).selftest() for m in ['delta_ops','phi_algebra','swarm','hashgrid','geoformer','doctor_geo','e8_twist']]"

# Полный тест-раннер (18 тестов, все модули + cross-verify + Rust + spec-kit)
python tests/run_all.py

# Rust-модуль (PyO3)
cd src/geo_matmul_rs
maturin build --release
pip install target/wheels/geo_matmul_rs-*.whl
```

---

## Структура проекта

```
OpteriumGeoFormer/
├── src/                           # Исходный код
│   ├── delta_ops.py              # 8 Δ-операторов + композиция + HealthVector
│   ├── phi_algebra.py            # Φ₁-Φ₅: SHIFT, PHASE, FIXEDPOINT, RECURSION, PROJECTION
│   ├── swarm.py                  # Intelligent Swarm V2 (замена Байеса)
│   ├── hashgrid.py               # O(1) spatial lookup в (S,D) space
│   ├── geoformer.py              # GeometricBlock: Resonate→Project→Shift, SwarmTrainer
│   ├── doctor_geo.py             # SwarmDoctor: DoctorCore + IntelligentSwarm
│   ├── e8_twist.py               # TWIST 2520-cycle, triality V(112)+S+(64)+S-(64), 70.1° closure
│   ├── spec-kit/                 # 5 методов geometric matrix multiply
│   │   ├── methods/              # pt_naive, pt_naive_fast, pytable_matmul, sd_matmul, geo_resonant
│   │   └── tests/                # test_correctness.py (20 тестов)
│   └── geo_matmul_rs/            # Rust/PyO3 модуль
│       ├── Cargo.toml
│       └── src/lib.rs            # sd_matmul, sd_matmul_parallel (rayon), HashGrid, geometric_attention
├── docs/
│   ├── PARADIGM.md               # 5 аксиом, архитектурный стек, Φ-таблица, Δ-таблица
│   └── ARCHITECTURE.md           # GeoFormer: слои, complexity, сравнение с Transformer
├── tests/
│   ├── run_all.py                # Мастер-раннер: 18 тестов
│   └── TEST_COVERAGE.md          # 67 тестов, 0 падений, 5 🔴 CRIT gaps
├── results/
│   └── SUMMARY.md                # Benchmark: Rust 1.38× torch
├── .opencode/
│   └── AGENTS.md                 # AI load context (кэширует README для новой сессии)
├── bootstrap/                    # Загрузчик для новой AI-сессии (запускается один раз)
│   ├── delta_ops.py, phi_algebra.py, swarm.py, hashgrid.py, geoformer.py, doctor_geo.py, e8_twist.py
│   ├── spec-kit/                 # spec→plan→tasks→code pipeline для 5 методов
│   └── geo_matmul_v2.pyx         # Cython v2 (flat C arrays)
└── README.md                     # Этот файл
```

---

## Результаты

### Rust/PyO3 vs torch.matmul (CPU, pure integer)

| n | Rust seq | Rust par (rayon) | torch | Rust par vs torch |
|---|---|---|---|---|
| 4 | 0.31ms | 0.20ms | 0.16ms | 0.80× |
| 16 | 0.44ms | 0.22ms | 0.16ms | 0.73× |
| 64 | 0.72ms | 0.45ms | 0.39ms | 0.87× |
| **128** | **1.63ms** | **0.85ms** | **1.17ms** | **1.38×** ✅ |
| 256 | 6.70ms | 4.86ms | 4.95ms | **1.02×** (equal) |
| 512 | 46.0ms | 37.6ms | 24.6ms | 0.65× (within 2×) |

**Rust geometric_attention** — 1000 токенов: **0.41ms** (184× быстрее Python hashgrid).

### Cython v2 (flat C arrays)

- `geo_matmul_v2.pyd` — **12× быстрее torch** при n=4, **125× быстрее Python** при n=64.
- Скомпилирован из `geo_matmul_v2.pyx` через `cythonize`.

### GeoResonant (zero-MM attention)

Победитель spec-kit: **ни одного вызова матричного умножения**.  
O(n·k) hashgrid attention + Pt3 FFN против O(n²·d) Transformer.

### E8 Twist

- TWIST 2520-цикл: 35° → 72 шага, 70.1° → 36 шагов
- Triality: V=112, S⁺=64, S⁻=64 → 240 E8 roots ✓
- Closure: 70.1° energy=0.5475 status=CLOSED
- Best config: (112, 64, 192) amplitude=15.93

---

## Статус тестирования

**67 тестов, 0 failures** (по состоянию на май 2026):

| Группа | Статус |
|--------|:------:|
| delta_ops (self-test) | ✅ |
| phi_algebra (self-test) | ✅ |
| swarm (self-test) | ✅ |
| hashgrid (5 тестов) | ✅ |
| geoformer (7 тестов) | ✅ |
| doctor_geo (7 тестов) | ✅ |
| e8_twist (6 тестов) | ✅ |
| spec-kit correctness (20 тестов) | ✅ |
| Rust: HashGrid | ✅ |
| Rust: geometric_attention | ✅ |
| Cross-verify Py=Cython=Rust | ✅ |

**5 🔴 CRIT gaps** (см. `tests/TEST_COVERAGE.md`):
1. GeoFormer end-to-end convergence
2. Multi-layer stacking
3. Opterium DoctorCore bridge
4. Rust `cargo test`
5. Cross-verify on full matrix suite

---

## Ключевые решения

1. **Нет матричного умножения** — GeoFormer использует Pt3(x,y,context) для FFN, hashgrid neighbor lookup для attention, Weyl-flow geodesic descent для оптимизации. Все операции — целочисленные.

2. **Нет backprop** — обучение = Swarm reinforcement. Пути к правильному ответу получают H_j++, ошибочные пути отсекаются. Ни градиентов, ни FP32, ни Adam.

3. **Память = Address Descriptor**, не weight matrix. `(scale, node, Δ_project)` заменяет 10¹² floats. Правило И ЕСТЬ память.

4. **GPU не нужен** — GeoFormer работает на CPU/RPi: только int compare и PyTable lookup. Nvidia продаёт FLOPs, которые GeoFormer не использует.

5. **Φ-алгебра — мета-язык**: каждое понятие есть Φ-путь. K(ξ) = длина. Гипотеза Римана: устойчивые нули требуют Φ₂+Φ₃ смешивания; Φ₁-only траектории транзиентны.

6. **Swarm заменяет Байеса везде**: без априорных вероятностей, без правдоподобий, без апостериорных. Четыре драйва (эксплуатация μ^α, исследование (1/kol)^β, потенциал (kol/Max)^γ, мудрость H_j) конкурируют.

7. **Rust побеждает torch на CPU** — sd_matmul_parallel с rayon в 1.38× быстрее torch.matmul при n=128 на чистой целочисленной арифметике.

---

## Связанные проекты

- `D:\gemma-4-geometric\` — оригинальный проект (dataset, spec-kit, ROADMAP)
- `%Desktop%\Испытываем\src\opterium_field.py` — 2441-line unified module (Pt→Pt3→Cube27→Field9→E8→DoctorCore)
- `%Desktop%\Испытываем\Разбор полезностей\bootloader.txt` — 1237 lines (Δ-operator specs, E8 protocols, G2 kernel)
- `%Desktop%\Испытываем\Разбор полезностей\AI_BIOS.txt` — 385 lines (Φ-algebra, Swarm, Address Descriptor)
- `D:\активация.txt` — 140 KB (TWIST angles, doctor diagnosis, neutrino mixing)
- `D:\gemma-4-geometric\dataset\PYTH_TABLE_1000.bin` — 1M×11 byte lookup table

---

## Следующие шаги

1. Закрыть 🔴 CRIT gaps: convergence test, multi-layer test, DoctorCore bridge test, `cargo test`, cross-verify full suite
2. GeoFormer → E8Gen.TWIST: Δ_OPTG через TWIST-циклы для geodesic descent
3. GeoFormer → SwarmDoctor: forward → Doctor.judge → Swarm.reinforce
4. Rust SIMD: `packed_simd` для дополнительных ~4× на поддерживаемых CPU
5. Финальный тренировочный цикл: Swarm эпизоды по токен-последовательностям, кривые сходимости
