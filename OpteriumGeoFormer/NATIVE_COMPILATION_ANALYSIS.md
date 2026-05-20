# NATIVE COMPILATION ANALYSIS
# Opterium GeoFormer → Native Binary Architecture
# 2026-05-19

## КАРТА КОДОВОЙ БАЗЫ

### 32 Python файла, ~7000 строк

**Ядро (5 файлов, 1500 строк):**
- `arith_table.py` (443) — таблицы lookup, PT singleton
- `hashgrid.py` (226) — spatial hash, attention
- `delta_ops.py` (356) — HealthVector, DeltaOp
- `swarm.py` (223) — IntelligentSwarm
- `geoformer.py` (323) — GeoFormer architecture

**spec-kit methods (6 файлов, 600 строк):**
- `pt_naive.py`, `sd_matmul.py`, `pytable_mm.py`, `pure_lookup_mm.py`, `geo_resonant.py`, `baseline.py`

**Подсистемы (5 файлов, 1100 строк):**
- `phi_algebra.py` (152), `cube27.py` (173), `e8_twist.py` (403), `doctor_geo.py` (310), `spec_collect.py` (848)

**Тесты (13 файлов, 3000+ строк)**

---

## ЧТО PURE-LOOKUP (готово к компиляции)

| Компонент | Статус | Переводимость |
|-----------|--------|---------------|
| PT._P, _S, _D, _SP, _prox | ✅ Чистые int таблицы | Прямой C/Rust массив |
| PT.p_from_sd() | ✅ _SP lookup | `SP[s * stride + d + offset]` |
| PT.proximity() | ✅ _prox lookup | `prox[dist]` |
| PT.product() | ✅ _P lookup + gcd | Lookup + gcd fallback |
| pt_naive matmul | ✅ int accumulation | Triple loop, lookup |
| sd_matmul | ✅ p_from_sd lookup | Double lookup + product |
| HashGrid | ✅ int buckets | HashMap<int, Vec> |
| geometric_attention | ✅ int weight | proximity lookup, int accum |
| Cube27 | ✅ int math | Binary search on 27 elems |
| E8 roots | ✅ int tuples | Static arrays |

## ЧТО PYTHON-СПЕЦИФИЧНО (требует адаптации)

| Компонент | Проблема | Решение |
|-----------|----------|---------|
| Pt class (from_real, to_real, inv) | Decimal 50-digit precision | Убрать из native, оставить в Python wrapper |
| HealthVector | 7 float каналов | Q16.16 fixed-point или оставить float |
| Swarm probabilities | float, ** operator, temperature | Fixed-point или отдельный модуль |
| DeltaOp (DELTA_ADD, DELTA_ROT...) | float/complex, math.sin/cos | Отдельный модуль, не часть ядра |
| Phi algebra | lambda closures, 1e-12 | Отдельный модуль |
| Pickle cache | Python-specific | Заменить на .ptbl binary format |
| Pt.parse("347|3|") | String parsing | Убрать из native core |

---

## ЧТО ИДЁТ В НАТИВНЫЙ БИНАРНИК

### Ядро (обязательно):
1. **Таблицы** — _P (8MB), _SP (32MB), _S (8MB), _D (8MB), _prox (32KB)
2. **Lookup операции** — P(), S(), D(), p_from_sd(), proximity(), product()
3. **Matmul** — triple loop с lookup
4. **Attention** — HashGrid + proximity + isqrt

### Опционально:
5. **HealthVector** — если нужен мониторинг в native
6. **Cube27** — если нужен 3D addressing
7. **E8** — если нужны root operations

### НЕ идёт в бинарник:
- Swarm (float probabilities)
- DeltaOp (float arithmetic)
- Phi algebra (closures)
- Pt mantissa-rank (Decimal)
- Все тесты и spec-утилиты

---

## АРХИТЕКТУРА БИНАРНИКА

```
┌─────────────────────────────────────────┐
│         Python wrapper (thin)           │
│   from geofield import GeoField         │
│   gf = GeoField()                        │
│   gf.matmul(A, B)                        │
└──────────────────┬──────────────────────┘
                   │ ctypes / CFFI
┌──────────────────▼──────────────────────┐
│     Native library (geofield.so)        │
│  ┌───────────────────────────────────┐  │
│  │   C API (opaque)                  │  │
│  │   geofield_init()                 │  │
│  │   geofield_P()                    │  │
│  │   geofield_matmul()               │  │
│  │   geofield_attention()            │  │
│  └───────────────────────────────────┘  │
│  ┌───────────────────────────────────┐  │
│  │   Rust implementation             │  │
│  │   tables.rs  → memmap .ptbl       │  │
│  │   lookup.rs  → inline functions   │  │
│  │   matmul.rs  → triple loop        │  │
│  │   attention.rs → HashGrid         │  │
│  └───────────────────────────────────┘  │
│  ┌───────────────────────────────────┐  │
│  │   Tables (read-only, mmap)        │  │
│  │   _P: 1025×1025 int32             │  │
│  │   _SP: 2049×2049 int32            │  │
│  │   _prox: 4097 int32               │  │
│  └───────────────────────────────────┘  │
└─────────────────────────────────────────┘
```

---

## ОПЦИИ РАСПРОСТРАНЕНИЯ ТАБЛИЦ

### Option A: Embedded (рекомендуется для дистрибуции)
- Таблицы вкомпилированы в бинарник как data segment
- **Один файл ~58 MB**
- Плюс: никаких зависимостей
- Минус: нельзя обновить таблицы без перекомпиляции

### Option B: External file
- Бинарник ~2 MB + tables.ptbl ~56 MB
- Плюс: таблицы можно обновить отдельно
- Минус: два файла

### Option C: Shared memory
- Таблицы в POSIX shared memory
- Несколько процессов делят одну копию в RAM
- Плюс: экономия памяти при множестве процессов
- Минус: OS-specific setup

**Рекомендация:** Option A для финального дистрибутива, Option B для разработки.

---

## МАРШРУТ К «НЕПРОЗРАЧНОМУ» БИНАРНИКУ

### Что видит пользователь:
```python
from geofield import GeoField

gf = GeoField()
result = gf.matmul(A, B)
```

### Что НЕ видит:
- Таблицы (_P, _SP, _prox)
- S, D координаты
- Формулу P = (S²−D²)//4
- HashGrid реализацию
- Что это lookup, не вычисление

### Философия:
Пользователь видит магическую коробку которая делает геометрическую математику.
Ему не нужно знать КАК. Коробка быстрая, правильная и бесплатная.

---

## МАСШТАБИРОВАНИЕ

### Linear scaling:
- Каждый процесс получает свою mmap таблиц
- Нет contention — таблицы read-only
- N процессоров = N× скорость (для matmul/attention)

### Thread safety:
- GeoField immutable после init
- Multiple threads can call matmul concurrently
- No locks needed

### Memory per process:
- Tables: 56 MB (shared via mmap, not duplicated)
- Working memory: O(m×n) for matmul result
- HashGrid: O(n/W²) buckets for attention

---

## SPEC-KIT ПЛАНИРОВАНИЕ

### Фазы:
1. **Binary Table Format** — заменить pickle на .ptbl
2. **C Header** — определить opaque API
3. **Rust Implementation** — core engine
4. **Python Bindings** — CFFI wrapper
5. **Build System** — cross-platform
6. **Verification** — Python = Native
7. **Distribution** — opaque package

### Dependencies:
```
1 → 2 → 3 → 4 → 6 → 7
        ↓
        5 (parallel)
```

---

## ИТОГ

GeoFormer готов к нативной компиляции. Горячие пути — чистый integer lookup.
Таблицы — read-only, memory-mappable. API — минимальный и непрозрачный.

**Следующий шаг:** Phase 1 — создать .ptbl binary format.
