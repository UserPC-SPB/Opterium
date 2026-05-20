# План глубокого рекурсивного тестирования

## УРОВЕНЬ 1 — Изолированное (модуль vs модуль)

### 1.1 PtTable (arith_table.py)
- S/D/P lookup: 4 квадранта (++), (+-), (-+), (--)
- from_sd: (S,D) → (x,y), проверка x+y=S, x-y=D
- pairs_for_product: каждое P → все пары, x*y=P
- isqrt: квадраты, не-квадраты, 0
- conj: (x,y) → (y,x), P сохраняется, отрицательные
- pow10: 0..10, fallback
- abs: ±, 0
- product/sum/diff: все знаки, fallback для >MAX_COORD
- _in_range границы: x=0, y=0, x=max_coord, y=max_coord
- cache: save/load, version mismatch

### 1.2 Pt class (methods/__init__.py)
- Конструкторы: direct, from_int, from_sd, parse, from_real, from_decimal
- S/D/P верификация: x+y=S, x-y=D, x*y=P
- mantissa-rank: parse → repr → parse roundtrip
- to_real/from_real roundtrip: 0, малые, большие, отрицательные
- to_decimal/from_decimal: точный roundtrip, 50 digits
- inv: точность для разных рангов, x=0 edge
- __add__/__mul__: компонентно
- rmul/radd: мантисса-ранг (shift alignment, product)
- geo_mul/geo_add: геометрическая арифметика
- validate_shape: правильные, mismatch
- to_pt_matrix/sd_tuple_matrix: roundtrip

### 1.3 Cube27 (cube27.py)
- encode: 0, 1digit, 3digit, 6digit, 9digit, mixed
- cell_index: 0→0, 36→0, 37→1, 999→26
- cell_27: 0..26 → (cx,cy,cz), проверка всех 27
- path_27: длина = depth, каждый group ≤ 999
- verify: 100% PtTable hit для любых чисел
- depth: правильно считает для разных длин

### 1.4 HashGrid (hashgrid.py)
- insert/lookup: соседи в 3×3 окне
- lookup пустой: []
- geometric_weight: симметрия w(a,b)==w(b,a)
- geometric_attention: 0, 1, N токенов; структура выхода
- clear: сброс bucket
- window=1 edge case

### 1.5 Δ-ops (delta_ops.py)
- HealthVector: ok, warn, critical, merge, max_channel
- DeltaOp: call, >> pipe, | parallel, inv()
- CompositeDelta: sequential, parallel
- Все builtins: ADD, MUL, INV, INV_NS, PPH, OPTG, SHIFT, ROT
- check_domain: все алгебры
- select_fallback: S, F16, O

### 1.6 Φ-algebra (phi_algebra.py)
- 5 PhiOperator: SHIFT, PHASE, FIXEDPOINT, RECURSION, PROJECTION
- PhiPath: композиция, call, длина
- молекулы: periodic_orbit, harmonic_series, fixed_point_iteration

### 1.7 Swarm (swarm.py)
- SwarmNode: reinforce, visits, success_rate, pheromone
- IntelligentSwarm: register, score, probabilities, decide, update
- sum(probabilities) == 1
- Детерминизм: одинаковый seed → одинаковый выбор
- BayesReplacement: update_belief, posterior, predict

### 1.8 E8 Twist (e8_twist.py)
- D8 roots: 112, правильно (±2,±2,...)
- Spinor roots: 128, even parity
- Все 240 roots: V=112, S+=64, S-=64
- address_to_root: 2D → 8D mapping
- root_properties: sector, scale, parity, norm2
- 2520-cycle: 35°, 70°, 105°, 140° → steps
- TWIST operation: phase+config → status
- closure_angle: energy, status

### 1.9 Doctor Bridge (doctor_geo.py)
- geo_to_opterium_hv: mapping всех 7 каналов
- opterium_to_geo_hv: обратно
- roundtrip: geo → opterium → geo
- SwarmDoctor: register, choose route, reinforce, judge, quarantine

---

## УРОВЕНЬ 2 — Cross-module (пары)

### 2.1 PtTable + Pt class
- Pt constructor: табличный S/D/P против формульного для >MAX_COORD
- Все Pt операции против PT.lookup
- inv(): Decimal → PtTable product(original, inv) ≈ 1

### 2.2 PtTable + Cube27
- Все группы 0..999 → PT.has(group, 1) == True
- Большое число: groups[x].P × scale → восстанавливает исходное
- path_27 + from_sd: координаты по пути

### 2.3 PtTable + Δ-ops
- Δ_SHIFT: PtTable value, scale → проверка
- Δ_MUL: PtTable product × factor
- Δ_INV: через Doctor (Decimal bridge)

### 2.4 PtTable + E8 twist
- address_to_root(PT.from_sd(S,D)) → проверка
- root_properties: адреса из PtTable
- closure_angle через PtTable lookup

### 2.5 Pt class + Cube27
- Большое число → Cube27 path → Pt lookup → product restore
- from_real(dec) → Cube27 → PtTable hit

### 2.6 Pt class + HashGrid
- Pt(S,D) → HashGrid.insert → lookup
- geometric_attention на Pt последовательности

### 2.7 HashGrid + GeoFormer
- GeometricBlock.forward: hashgrid внутри
- GeoFormer.forward: geometric_attention вызывается

### 2.8 Δ-ops + Φ-algebra
- Δ композиция как Φ-path
- Δ_SHIFT >> Δ_MUL >> Δ_INV последовательность

### 2.9 E8 Twist + Δ-ops
- Δ_OPTG на E8 roots
- TWIST → Δ проверка

### 2.10 Doctor Bridge + GeoFormer
- SwarmTrainer + Doctor verdict (geo + opterium)
- geo_to_opterium → judge → обратно

---

## УРОВЕНЬ 3 — Интеграция (3+ модулей)

### 3.1 Full AI pipeline
PtTable → Cube27 → Pt class → matmul → HashGrid → GeoFormer → SwarmTrainer → DoctorBridge → verdict
- Вход: мантисса-ранг числа
- PtTable lookup → S/D/P
- Большие числа → Cube27 groups → поэлементно PtTable
- matmul: все методы (pt_naive, pytable, sd_matmul)
- HashGrid: geometric_attention
- GeoFormer: forward
- SwarmTrainer: train_step
- Doctor: verdict OK

### 3.2 Full Theory pipeline
Δ-ops → Φ-algebra → Swarm → E8 Twist → PtTable
- Δ-операторы композиция
- Φ-path как спецификация
- Swarm маршрутизация
- E8 root generation
- Address → root → PtTable

### 3.3 Cross-verify
Python == Cython == Rust (sd_matmul)
+ PtTable direct == formula fallback
+ mantissa-rank == Decimal

### 3.4 Обратный pipeline
E8 root → address_to_root → PtTable(S,D) → from_sd → Pt → mantissa-rank → to_real

---

## УРОВЕНЬ 4 — Стресс и границы

### 4.1 Размерность
- Cube27: мантиссы с 1..100 цифр (3..300 групп)
- Matrix: 1×1, 256×256, 512×512
- HashGrid: 1 токен, 10000 токенов
- Swarm: 1 узел, 1000 узлов

### 4.2 Диапазон значений
- Decimal: очень малые (ранг 100+), очень большие (мантисса 10^100)
- int: 0, 1, MAX_COORD-1, MAX_COORD, MAX_COORD+1 (fallback)
- Отрицательные: все комбинации знаков в matmul

### 4.3 Детерминизм
- Одинаковый seed → одинаковый результат (все модули)
- Нет случайной вариации при фиксированном seed

### 4.4 Производительность
- PtTable lookup: 1M операций
- Cube27 encode: 1000-digit числа
- matmul: 100×100 матрицы, замер времени
- geometric_attention: 1000 токенов

### 4.5 Устойчивость
- x=0, y=0 в PtTable
- x=0 в DeltaOp
- пустой список в matmul
- None/NaN на входе
- невалидный parse

---

## Формат результатов тестирования

Каждый тест возвращает:
- имя
- статус: PASS / FAIL / SKIP
- причина: что именно не сошлось (значение, тип, исключение)
- данные: вход/выход для анализа

После прохода всех тестов:
- Сводка: У1 / У2 / У3 / У4
- Проблемы: отсортированы по модулю и уровню
- Roadmap: от самых критичных к косметическим
