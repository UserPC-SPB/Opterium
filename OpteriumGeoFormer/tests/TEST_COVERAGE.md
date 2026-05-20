# TEST COVERAGE  —  Opterium GeoFormer

Живой документ. Показывает где есть замыкание (тесты проходят, связи целы)
и где его нет (нет теста, тест падает, компонент не связан с остальными).

## Легенда

| Символ | Значение |
|--------|----------|
| ✅ PASS | Тест проходит — замыкание есть |
| ❌ FAIL | Тест падает — замыкание разорвано |
| ⚠️ GAP | Компонент не тестирован — замыкания нет |
| 🔴 CRIT | Критический пробел — блокирует production |
| 🟡 WARN | Важный пробел — снижает надёжность |
| 🟢 INFO | Косметический пробел — можно отложить |

---

## 1. Модульные тесты (self-test в каждом файле)

### 1.1 Δ-операторы — `delta_ops.py`

| Тест | Статус | Что проверяет |
|------|--------|---------------|
| `DELTA_ADD` | ✅ | 0+0, a+b |
| `DELTA_MUL` | ✅ | 0*0, a*b |
| `DELTA_INV` | ✅ | inv(2)=0.5, zero→inf |
| `DELTA_INV_NS` | ✅ | zero divisor tunneling |
| `DELTA_PPH` | ✅ | projection residue >0 |
| `DELTA_OPTG` | ✅ | Weyl geodesic |
| `compose_sequential` | ✅ | type check, piping |
| `check_domain` | ✅ | algebra classification |
| `select_fallback` | ✅ | fallback chain |
| `HealthVector` | ✅ | ok/warn/critical, merge |

**Пробелы:**
- 🟡 WARN Нет теста на `DELTA_ZERO_DETECT` в selftest (но класс есть)
- 🟡 WARN Нет теста параллельной композиции `compose_parallel`
- 🟡 WARN Нет теста на `FUSION_TABLE['FMA']`
- 🟢 INFO Нет теста на `identity()` фабрику

### 1.2 Φ-алгебра — `phi_algebra.py`

| Тест | Статус | Что проверяет |
|------|--------|---------------|
| Φ₁ SHIFT | ✅ | translate |
| Φ₂ PHASE | ✅ | rotate |
| Φ₃ FIXEDPOINT | ✅ | converge |
| Φ₄ RECURSION | ✅ | self-apply |
| Φ₅ PROJECTION | ✅ | shadow |
| Φ-path composition | ✅ | SHIFT∘PHASE |
| K-complexity | ✅ | K(ξ) = path length |

**Пробелы:**
- 🟡 WARN Нет теста на длинные Φ-пути (>5 шагов)
- 🟢 INFO Нет теста Riemann hypothesis restatement

### 1.3 Swarm — `swarm.py`

| Тест | Статус | Что проверяет |
|------|--------|---------------|
| Node registration | ✅ | 4 nodes |
| Probabilities sum | ✅ | Σ=1.0 |
| Deterministic decide | ✅ | best wins |
| Reinforcement | ✅ | success↑, failure↓ |
| BayesReplacement | ✅ | update/posterior/predict |

**Пробелы:**
- 🟡 WARN Нет теста на temperature ≠ 1.0 sampling
- 🟡 WARN Нет теста с 0 nodes (empty swarm)
- 🟢 INFO Нет теста на `_history` persistence

### 1.4 HashGrid — `hashgrid.py`

| Тест | Статус | Что проверяет |
|------|--------|---------------|
| insert/lookup | ✅ | O(1) spatial |
| empty lookup | ✅ | no crash |
| weight symmetry | ✅ | w(a,b)=w(b,a) |
| geometric_attention | ✅ | 20 tokens, all output fields |
| clear | ✅ | reset state |
| stats | ✅ | bucket/entry counting |

**Пробелы:**
- 🟡 WARN Нет теста на insert_many
- 🟡 WARN Нет теста на window=1 (extreme)
- 🟢 INFO Нет теста на lookup с exclude_self

### 1.5 GeoFormer — `geoformer.py`

| Тест | Статус | Что проверяет |
|------|--------|---------------|
| Pt class | ✅ | S, D, P, from_int, zero |
| GeometricEmbedding | ✅ | int sequence embed |
| GeometricBlock forward | ✅ | 10 tokens, HV returned |
| GeoFormer forward | ✅ | 10 tokens, layers=2 |
| SwarmTrainer step | ✅ | score, success |
| doctor_judge | ✅ | OK/WARN/FAIL verdicts |

**Пробелы:**
- ✅ CRIT **end-to-end GeoFormer → SwarmTrainer → convergence** — `test_geoformer_convergence.py` (6/6)
- ✅ CRIT **multi-layer GeometricBlock stacking** — `test_geoformer_stacking.py` (7/7)
- 🟡 WARN Нет теста на пустой вход
- 🟡 WARN Нет теста на `embed_sequence` со строками

### 1.6 Doctor Geo (SwarmDoctor) — `doctor_geo.py`

| Тест | Статус | Что проверяет |
|------|--------|---------------|
| Swarm выбор маршрута | ✅ | выбирает из ROUTE_REGISTRY |
| Route probabilities | ✅ | Σ=1.0 |
| Judge OK | ✅ | clean HV → OK |
| Judge bad HV | ✅ | high E_assoc → QUARANTINE/ROLLBACK |
| Reinforce | ✅ | score после reinforce |
| Quarantine | ✅ | get/set/clear |
| Opterium bridge | ✅ | geo↔opterium HV conversion |

**Пробелы:**
- ✅ CRIT **Интеграция с реальным opterium_field.DoctorCore** — `test_doctor_bridge.py` (8/8)
- ✅ CRIT **`opterium_judge()` / bridge** — `test_doctor_bridge.py` (8/8)
- 🟡 WARN Нет теста на `judge_full()` края (hv на границе порога)
- 🟡 WARN Нет теста на `ROUTE_REGISTRY` — все ли маршруты корректны
- 🟢 INFO Нет теста на пустой quarantine

### 1.7 E8 Twist — `e8_twist.py`

| Тест | Статус | Что проверяет |
|------|--------|---------------|
| Triality | ✅ | V(112)+S+(64)+S-(64) |
| 2520-cycle 35° | ✅ | 72 steps |
| 2520-cycle 70.1° | ✅ | 36 steps |
| TWIST operation | ✅ | amplitude, config |
| Closure 70.1° | ✅ | energy, status |
| Scan configs | ✅ | best config found |
| Summary | ✅ | complete state |

**Пробелы:**
- 🟡 WARN Нет теста на phase=0.333 и 0.666 (120° separation)
- 🟡 WARN Нет верификации: TWIST amplitude корректен?
- 🟡 WARN Нет теста на closure_angle при разных углах (35, 105, 140)
- 🟢 INFO Нет теста без E8Gen (standalone mode)
- 🟢 INFO Нет теста на `_nonlinear()` функцию

### 1.8 Spec-Kit методы — `spec-kit/methods/`

| Тест | Статус | Что проверяет |
|------|--------|---------------|
| pt_naive vs torch | ✅ | 4×4 random |
| pt_naive_fast vs torch | ✅ | 4×4 random |
| pytable_matmul vs torch | ✅ | 4×4 random |
| pytable_cached vs torch | ✅ | 4×4 random |
| sd_matmul vs torch | ✅ | 4×4 random |
| Identity (A·I=A) | ✅ | все методы |
| Zero (A·0=0) | ✅ | все методы |
| HealthVector OK | ✅ | все методы |
| Shape validation | ✅ | mismatch → ValueError |

**Пробелы:**
- 🟡 WARN Нет теста на не-квадратные матрицы (m×k × k×n)
- 🟡 WARN Нет теста на большие значения (>10⁶)
- 🟢 INFO Нет теста на float → Pt conversion

### 1.9 Rust модуль — `geo_matmul_rs`

| Тест | Статус | Что проверяет |
|------|--------|---------------|
| sd_matmul (seq) | ✅ | benchmark verify |
| sd_matmul_parallel | ✅ | benchmark verify |
| HashGrid insert/lookup | ✅ | 3 entries |
| geometric_attention | ✅ | 100/1000 tokens |

**Пробелы:**
- ✅ CRIT **Модульные тесты в самом Rust (cargo test)** — 12 unit-тестов: formula, sd_matmul, seq=parallel, HashGrid
- ✅ CRIT **Верификация: sd_matmul_rs == Python** — `test_cross_verify.py` (6/6)
- 🟡 WARN Нет теста на пустые матрицы в Rust
- 🟡 WARN Нет теста на shape mismatch в Rust
- 🟢 INFO Нет бенчмарка regression tracking

---

## 2. Интеграционные тесты (cross-module)

| Связка | Статус | Описание |
|--------|--------|----------|
| delta_ops + phi_algebra | ❌ GAP | Нет теста: Φ-пути вызывают Δ-операторы |
| delta_ops + swarm | ❌ GAP | Нет теста: Swarm решает, Δ выполняет |
| delta_ops + geoformer | ✅ | Geoformer использует DELTA_SHIFT, DELTA_OPTG |
| phi_algebra + geoformer | ❌ GAP | Geoformer не вызывает Φ — потенциально |
| hashgrid + geoformer | ✅ | Geoformer.GeometricBlock вызывает hashgrid |
| swarm + doctor_geo | ✅ | SwarmDoctor оборачивает IntelligentSwarm |
| doctor_geo + opterium_field | ✅ | bridge работает — 8 тестов |
| e8_twist + opterium_field.E8Gen | ✅ | реальные 240 корней загружены |
| spec-kit + delta_ops | ✅ | Pt класс + geo_mul/geo_add |
| Cython + Python verify | ✅ | cross-verify 6/6 — все размеры |
| Rust + Python verify | ✅ | cross-verify 6/6 — Python==Cython==Rust==torch |
| GeoFormer + SwarmTrainer | ✅ | end-to-end pipeline — 6 тестов |

---

## 3. Тесты производительности (benchmark)

| Тест | Статус | Размеры | Замеры |
|------|--------|---------|--------|
| spec-kit benchmark | ✅ | 4-128 | warmup=3, measured=10 |
| Cython vs torch vs Python | ✅ | 4-256 | warmup=5, measured=20 |
| Rust vs Cython vs torch | ✅ | 4-1024 | warmup=5, measured=20 |
| Hashgrid scaling (Python) | ✅ | 10-1000 | warmup=3, measured=5 |
| Hashgrid scaling (Rust) | ✅ | 100-1000 | warmup=3, measured=10 |

**Пробелы:**
- 🟡 WARN Нет regression tracking — как benchmark меняется между версиями
- 🟡 WARN Нет сравнения с RPi/ARM (все замеры на x64)
- 🟢 INFO Нет теста на throughput (токенов/сек)

---

## 4. Качество замыканий (closure gaps)

### 4.1 Прямые разрывы (код вызывает, тест не проверяет)

| Вызов | Где определён | Где используется | Тест? |
|-------|---------------|------------------|-------|
| `opterium_field.e8gen` | src/opterium_field.py | e8_twist.E8Gen | ✅ |
| `opterium_field.DoctorCore` | src/opterium_field.py | doctor_geo.SwarmDoctor | ❌ |
| `DELTA_SHIFT` | delta_ops.py | geoformer.GeometricBlock | ✅ |
| `DELTA_OPTG` | delta_ops.py | geoformer (запланирован) | ❌ |
| `PYTH_TABLE_1000.bin` | D:\gemma-4-geometric\dataset | pytable_mm.py | ✅ |
| `PHI1_SHIFT...PHI5_PROJECTION` | phi_algebra.py | geoformer — не вызываются | ❌ |

### 4.2 Косвенные разрывы (должны быть связаны, но не связаны)

| Компонент A | Компонент B | Тип связи | Статус |
|-------------|-------------|-----------|--------|
| GeoFormer.Resonate | HashGrid | data flow | ✅ |
| GeoFormer.Project | Pt3(x,y,context) | data flow | ✅ |
| GeoFormer.Shift | DELTA_SHIFT | data flow | ✅ |
| GeoFormer.Optimization | Δ_OPTG | design — не реализовано | ❌ |
| GeoFormer.Training | SwarmTrainer | data flow | ✅ |
| GeoFormer.Doctor | doctor_geo.SwarmDoctor | design — не связано | ❌ |
| E8Gen.TWIST | GeoFormer.Optimization | design — E8 routing | ❌ |
| Cython sd_matmul | spec-kit SDK | импорт | ✅ |
| Rust sd_matmul | Python via PyO3 | FFI | ✅ |

### 4.3 Внешние зависимости

| Зависимость | Тип | Есть fallback? | Тест без неё? |
|-------------|-----|----------------|---------------|
| `PYTH_TABLE_1000.bin` | файл | ✅ (формула) | ❌ |
| `torch` | библиотека | ✅ (numpy) | ❌ |
| `numpy` | библиотека | ✅ (чистый Python) | ❌ |
| `opterium_field` | модуль | ❌ (ImportError) | ❌ |
| `geo_matmul_v2.pyd` | .pyd | ❌ (ImportError) | ❌ |
| `geo_matmul_rs` | PyO3 модуль | ❌ (ImportError) | ❌ |

---

## 5. Сводка

| Метрика | Значение |
|---------|----------|
| 📄 Всего тестов | 90 |
| ✅ Проходит | 90 (100%) |
| ❌ Падает | 0 |
| ⚠️ GAP прямых разрывов | 3 |
| ⚠️ GAP интеграционных | 4 |
| ⚠️ GAP косвенных | 4 |
| 🔴 CRIT пробелов | 0 (все 5 закрыты) |
| 🟡 WARN пробелов | 20 |
| 🟢 INFO пробелов | 8 |

### 5 критических пробелов (🔴 CRIT) — ✅ все закрыты

1. ✅ **GeoFormer → SwarmTrainer convergence** — `test_geoformer_convergence.py` (6/6)
2. ✅ **Multi-layer GeometricBlock** — `test_geoformer_stacking.py` (7/7)
3. ✅ **GeoFormer ↔ opterium_field.DoctorCore** — `test_doctor_bridge.py` (8/8)
4. ✅ **Rust cargo test** — lib.rs: 12 unit-тестов, все pass
5. ✅ **Rust/Cython cross-verify** — `test_cross_verify.py` (6/6) Python==Cython==Rust==torch

---

## 6. Что делать дальше

### Все 🔴 CRIT gaps закрыты ✅

Далее — плановая доработка:

1. **E8 TWIST → float-free** — заменить sin/cos/random на адресную навигацию (D_INDEX/P_INDEX)
2. **E8 TWIST → GeoFormer** — использовать TwistEngine.TWIST как оптимизатор в GeoFormer
3. **Cython 32-bit int** — проверить overflow при больших значениях, перейти на 64-bit
4. **Swarm updates** — реализовать настоящие Swarm node weight updates в SwarmTrainer
5. **Gemma 4 bridge** — интеграция по gemma4_opterium_spec.md

2. **GeoFormer → E8Gen.TWIST**: Δ_OPTG должен использовать TWIST-цикл для спуска
3. **GeoFormer → SwarmDoctor**: после forward → Doctor.judge → Swarm.reinforce
4. **Spec-kit Rust сборка**: `maturin develop --release` из папки проекта

---

## 7. Как добавить тест

```python
# tests/test_что_то.py
import sys; sys.path.insert(0, '../src')

def test_название():
    # Arrange
    # Act  
    # Assert — без assert нет замыкания!
    assert условие, "что пошло не так"
```

Правило: **один тест = одно замыкание**. Если assert нет — замыкания нет.
Если assert есть — замыкание есть. Если тест зелёный — замыкание держит.
