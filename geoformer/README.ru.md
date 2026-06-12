Весь код написан с использованием искусственного интеллекта. Я не программист, поэтому возможны ошибки!

# Cube v5 — MCP-сервер

**71 инструмент** нестандартной математики. Один исполняемый файл — `cube_v5.exe`.

## Подключение к LM Studio

В `mcp.json`:

```json
{
  "mcpServers": {
    "cube_v5": {
      "command": "путь\\до\\cube_v5.exe",
      "args": [],
      "type": "stdio"
    }
  }
}
```

После загрузки модели сервер появится в списке инструментов (71 шт).

### Как это работает (persistent MCP)

Сервер — это **долгоживущий процесс**. LM Studio запускает его один раз при загрузке модели.
Все последующие вызовы инструментов идут в тот же процесс через stdin/stdout.
Данные загружаются в память 1 раз (~200 мс) при старте, дальше каждый запрос занимает микросекунды.
**НЕ нужно** запускать `cube_v5.exe` для каждого вызова — это убивает смысл.

## Что сказать ИИ

Копируйте в промпт любой LLM:

```
У тебя есть доступ к MCP-серверу Cube v5 — 71 инструмент нестандартной математики.

ВАЖНО: это НЕ обычная арифметика. 3+4 не обязательно равно 7, 3*4 не равно 4*3.
Числа имеют свидетеля (witness) с внутренней структурой.

Категории инструментов:
1. CubInt (11): add, sub, mul, floordiv, truediv, pow, mod, neg, abs, witness, validate
2. CubFloat (6): add, sub, mul, truediv, neg, abs
3. CubComplex (7): add, sub, mul, conjugate, abs, pow, neg
4. E8 (15): e8, get_root, partners, partners_split, antipode, aligned, weyl_depth, triangle_geometry, duality_check, distance_matrix, dot, spectrum_check, batch, batch_timed, stats
5. Vector (19): add, sub, mul, dot, sum, mean_x1000, variance_x1000, std_x1000, min, max, scale, norm_x1000, normalize_x1000, normalize_l1_x1000, cumsum, diff, clip, sort, unique
6. Spatial (6): check_support, place, move, align_floor, distance_xy, depth_shift
7. Прочие (7): addr3_stack, neighbors26, optg_path, doctor, random_n, reset_cube, help
8. Все названия с префиксом категории: cubint_*, cubfloat_*, cubcomplex_*, e8_*, vec_*, spatial_*


ПРИМЕРЫ:
- cubint_mul(a=4, b=3) → {"result":12,"witness":"points[4:3].P=12"}
- cubint_witness(a=12) → S_XY, D_XY, P, web, volume
- e8_batch(values=[1,2,3]) → dot3, kinds, sum_dot3
- doctor(value=12) → факторы, делители, замкнутость
- help(lang="ru") → справка на русском (en, zh, de)

Начинай с tools/list.
```

## Запуск вручную (один процесс, много запросов)

```powershell
cube_v5.exe
```

Сервер читает stdin в цикле. Можно отправить несколько запросов в один процесс:

```powershell
# Запустить, ввести запросы, завершить по Ctrl+C:
cube_v5.exe
{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}
{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"cubint_mul","arguments":{"a":4,"b":3}}}
```

**Один вызов через pipe (только для быстрой проверки):**

```powershell
'{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}' | cube_v5.exe
```

Для тестирования из Python используйте `test_persistence.py` (в той же папке) —
он запускает сервер один раз и шлёт все запросы в один процесс.
