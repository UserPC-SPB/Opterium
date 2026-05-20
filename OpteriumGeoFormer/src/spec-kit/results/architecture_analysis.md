# Анализ архитектуры Opterium GeoFormer — Pure Lookup vs Compute

## Фундаментальная идея

**GeoFormer не считает — он адресует.**

```
Transformer:  Q·K^T → softmax → V    (O(n²·d) FLOPs)
GeoFormer:    hashgrid(S,D) → bucket  (O(n·k) memory access)
```

Запрос = (S, D) координаты → это уже адрес в таблице. Результат (P) уже там.

## Текущее состояние: ГИБРИД

| Компонент | Что делает | Статус |
|-----------|------------|--------|
| PtTable | Pre-computed 1M entries: S, D, P | ✅ Pure lookup |
| sd_matmul | `(S²−D²)//4` каждый раз | ❌ Вычисляет |
| geo_resonant | `1.0/(1+dist)`, `isqrt` | ❌ Float + compute |
| pt_naive | `geo_mul`/`geo_add` на Pt | ❌ Создаёт объекты |

**Проблема:** PtTable построена, но sd_matmul её не использует для product lookup.

## Benchmark: 64×64 matrix multiply

| Реализация | Время | Что делает |
|------------|-------|------------|
| Python sd_matmul (current) | 77.4ms | Вычисляет (S²−D²)//4 |
| Python pure_lookup_optimized | 49.8ms | Direct array access `PT._P[a][b]` |
| Python pure_lookup (full) | 167ms | PtTable.product() с gcd overhead |
| Rust sd_matmul | 0.63ms | Native compute |
| Rust parallel | 0.42ms | Native + rayon |
| torch.matmul | 0.61ms | C/CUDA |

**Вывод:** Pure lookup в Python даёт **1.5× ускорение** vs compute, но interpreter overhead доминирует.

## Гипотетический результат на разных языках

### C (pure lookup, mmap'd tables)

```c
// Таблицы в памяти, pointer = адрес
int32_t* P_table = mmap(...);  // 4MB table
int32_t* accum_table = mmap(...);  // product lookup

for (int i = 0; i < m; i++) {
    for (int j = 0; j < n; j++) {
        int sum = 0;
        for (int k = 0; k < K; k++) {
            int p1 = P_table[A_sd[i][k]];  // 1 memory access
            int p2 = P_table[B_sd[k][j]];  // 1 memory access
            sum += accum_table[p1 * MAX_P + p2];  // 1 memory access
        }
        C[i][j] = sum;
    }
}
```

**Оценка:** ~0.5ms для 64×64
- 3 memory access per element × 64³ = 786K accesses
- L2 cache latency ~5ns → 786K × 5ns ≈ 4ms
- Но с prefetching и cache locality: ~0.5ms

### Rust (pure lookup)

```rust
// Аналогично C, но с безопасным access
let p1 = P_TABLE[a_s as usize][a_d as usize];
let p2 = P_TABLE[b_s as usize][b_d as usize];
sum += PRODUCT_TABLE[p1 as usize][p2 as usize];
```

**Оценка:** ~0.3-0.4ms для 64×64
- Текущий Rust уже 0.63ms с compute
- Pure lookup уберёт 2 mul + 1 sub + 1 div per iteration
- Останется только memory access + add

### FPGA/Verilog (pure lookup)

```verilog
// Таблицы в BRAM, parallel lookup
always @(posedge clk) begin
    p1 <= P_BRAM1[S1][D1];  // 1 cycle
    p2 <= P_BRAM2[S2][D2];  // 1 cycle
    prod <= PROD_BRAM[p1][p2];  // 1 cycle
    sum <= sum + prod;  // 1 cycle (pipelined)
end
```

**Оценка:** ~0.01ms для 64×64
- 4 pipeline stages, 1 result per cycle
- 64×64 = 4096 results @ 100MHz = 0.04ms
- С parallel processing (8 lanes): 0.005ms

## Итоговая таблица: H-factor по языкам

| Язык | 64×64 (ms) | H-factor vs Python | Почему |
|------|------------|-------------------|--------|
| Python compute | 77.4 | 1.0× | Interpreter overhead |
| Python pure lookup | 49.8 | 1.6× | Array access быстрее compute |
| C pure lookup | ~0.5 | 155× | No interpreter, direct memory |
| Rust compute | 0.63 | 123× | Native code |
| Rust pure lookup | ~0.3 | 258× | Native + no compute |
| FPGA pure lookup | ~0.01 | 7740× | Parallel BRAM, pipelined |

## Что нужно сделать для pure lookup

1. **sd_matmul:** заменить `(S²−D²)//4` на `PT._P[S][D]` lookup
2. **geo_resonant:** заменить float weight на integer table lookup
3. **Accumulation:** pre-compute product table для small values
4. **Rust:** переписать sd_matmul с lookup вместо compute
5. **FPGA:** map tables to BRAM, pipeline lookup chain

## Вывод

**Да, вы правы:** GeoFormer не должен считать. Запрос = адрес. Но текущая реализация
гибридная — таблицы построены, но не используются полностью.

**Оптимальный язык:** Rust для pure lookup (уже есть инфраструктура, 0.3ms для 64×64).
**Идеальный язык:** FPGA — таблицы в BRAM, lookup = 1 cycle, ~0.01ms для 64×64.

**Главный bottleneck:** Не ALU, а memory access. В Python — interpreter overhead.
В compiled languages — cache misses. В FPGA — parallel BRAM access решает оба.
