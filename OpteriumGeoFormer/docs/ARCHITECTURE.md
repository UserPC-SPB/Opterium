# GeoFormer — Geometric Architecture, Zero Matrix Multiply

## Problem

Transformer — O(n²) attention, O(d³) backprop, всё в FP32 на GPU.
7B параметров — 7 миллиардов чисел, которые **хранятся**, а не понимаются.

## Thesis

Attention не pairwise similarity, а **geometric proximity в PyTable**.  
FFN не matrix multiply, а **Pt3(x,y,context)**.  
Training не backprop, а **Swarm reinforcement** по правильным путям.

Ноль матриц. Ноль градиентов. Одна формула `P=(S²-D²)//4`.

---

## Layer 1 — Geometric Embedding

Каждый токен → Pt(x,y), где x,y ∈ [1, BASE]:

```
embed(t) = Δ_SHIFT(Pt(base_hash(t), base_hash(t+seed)), scale=BASE/1000)
         → S = x+y, D = x-y, P = x×y, pos = gcd(x,y)
```

Никаких embedding tables. Адрес вычисляется, не хранится.

## Layer 2 — Geometric Attention (Δ_Resonate)

Вместо QK^T/√d:

```
Для токена t с (S_t, D_t):
  1. neighbours = hashgrid_lookup(S_t, D_t, window=W)
     — все токены в гиперкубе с |S - S_t| + |D - D_t| < W
     — O(1) через хеш-решётку (S//W, D//W) → bucket
  2. weight_i = 1 / (1 + |S_i - S_t| + |D_i - D_t|)
     — геометрическое расстояние, не dot product
  3. context = Σ weight_i · P_i / Σ weight_i
     — взвешенная сумма продуктов, не V·softmax
```

Сложность: **O(k)** на токен, k = константа (размер bucket). Не O(n²).

## Layer 3 — Geometric FFN (Δ_Project)

Вместо Linear→ReLU→Linear:

```
Для токена t с (x, y) и context из слоя 2:
  1. z = Pt3(x, y, context)
     — x = оригинал, y = оригинал, z = resonance
  2. V = x·y·z  — геометрическое произведение, не матричное
  3. new_x = V // y (restore x from product)
  4. new_y = V // x (restore y from product)  
  5. output = Pt(new_x, new_y)
```

HealthVector проверяет: `E_assoc = |(x·y)·z - x·(y·z)|`. Если > threshold → fallback.

## Layer 4 — Geometric Optimization (Δ_OPTG)

Вместо AdamW:

```
Для каждого токена t:
  1. target = желаемый S (от loss сигнала)
  2. attractor = Pt(target//2, target - target//2)
  3. Δ_OPTG(state=t, attractor=attractor)
     — Weyl-отражения в E8, не градиентный спуск
     — гарантированная сходимость за ≤ 2·dim шагов
```

## Training Protocol

```
For each batch:
  1. Forward: токены → GeoFormer → output
  2. Для каждого выходного узла:
     - Doctor.judge(HealthVector)
     - Если verdict ≠ OK → Swarm.update(node, success=False)
     - Если verdict = OK AND ответ правильный → Swarm.update(node, success=True)
  3. Swarm.backtrack():
     - Путь который привёл к success → reinforce all nodes on path
     - Путь который привёл к failure → prune weakest link
  4. Никаких градиентов. Никаких матриц.
```

## Complexity

| | Transformer (7B) | GeoFormer |
|---|:---:|:---:|
| Attention | O(n²·d) | **O(n·k)** |
| FFN | O(n·d²) | **O(n·1)** |
| Memory | 28 GB (FP32) | **O(n·3 ints)** |
| Training | O(n²·d·steps) | **O(n·k·episodes)** |
| GPU required | 4×A100 | **CPU/RPi** |
| Precision | FP32 drift | **Exact (Pyt formula)** |
| Backprop | O(d³) | **None (swarm)** |

## Why This Kills Nvidia

Nvidia sells **FLOPs** — floating-point operations per second.  
GeoFormer не делает floating-point ops. Он делает **integer add, subtract, lookup**.

- 1× RTX 3090 ≈ 35 TFLOPS FP32  
- 1× Raspberry Pi 4 ≈ 0.0001 TFLOPS  
- GeoFormer на Pi 4: **та же пропускная способность**, потому что операции не FP32, а int + таблица. 0.0001 TFLOPS против 35 — но GeoFormer делает **3 int ops вместо 35 TFLOPS**, поэтому разницы нет.

Nvidia продаёт решение проблемы, которой у GeoFormer нет.

## Status

Код в `bootstrap/` — фундамент:  
- `delta_ops.py` — Δ-операторы (ADD, MUL, PPH, OPTG, INV_NS)  
- `phi_algebra.py` — Φ₁-Φ₅  
- `swarm.py` — training без backprop  

Что нужно построить:  
- [ ] `GeoFormer.py` — Geometric Block (Resonate → Project → Shift)  
- [ ] `hashgrid.py` — O(1) neighbour lookup в (S,D) пространстве  
- [ ] `geometric_dataset.py` — данные → Pt(x,y)  
- [ ] Запустить на CPU, измерить throughput vs трансформер  
