All code was written using AI. I'm not a programmer, so errors are possible!
# Cube v5 — MCP server

**71 tools** for non-standard mathematics. Single executable — `cube_v5.exe`.

## Connecting to LM Studio

In `mcp.json`:

```json
{
  "mcpServers": {
    "cube_v5": {
      "command": "path\\to\\cube_v5.exe",
      "args": [],
      "type": "stdio"
    }
  }
}
```

After loading a model, the server appears in the tools list (71 items).

### How it works (persistent MCP)

The server is a **long-lived process**. LM Studio starts it once when a model loads.
All subsequent tool calls go to the same process via stdin/stdout.
Data loads into RAM once (~200 ms) at startup, then each call takes microseconds.
**DO NOT** launch `cube_v5.exe` per call — that defeats the purpose.

## AI prompt

Copy this into any LLM prompt:

```
You have access to a Cube v5 MCP server — 71 non-standard math tools.

IMPORTANT: This is NOT ordinary arithmetic. 3+4 may not equal 7, 3*4 does not equal 4*3.
Numbers have a witness with internal structure.

Tool categories:
1. CubInt (11): add, sub, mul, floordiv, truediv, pow, mod, neg, abs, witness, validate
2. CubFloat (6): add, sub, mul, truediv, neg, abs
3. CubComplex (7): add, sub, mul, conjugate, abs, pow, neg
4. E8 (15): e8, get_root, partners, partners_split, antipode, aligned, weyl_depth, triangle_geometry, duality_check, distance_matrix, dot, spectrum_check, batch, batch_timed, stats
5. Vector (19): add, sub, mul, dot, sum, mean_x1000, variance_x1000, std_x1000, min, max, scale, norm_x1000, normalize_x1000, normalize_l1_x1000, cumsum, diff, clip, sort, unique
6. Spatial (6): check_support, place, move, align_floor, distance_xy, depth_shift
7. Other (7): addr3_stack, neighbors26, optg_path, doctor, random_n, reset_cube, help
8. Full names use category prefix: cubint_*, cubfloat_*, cubcomplex_*, e8_*, vec_*, spatial_*


EXAMPLES:
- cubint_mul(a=4, b=3) → {"result":12,"witness":"points[4:3].P=12"}
- cubint_witness(a=12) → S_XY, D_XY, P, web, volume
- e8_batch(values=[1,2,3]) → dot3, kinds, sum_dot3
- doctor(value=12) → factors, divisors, closed-ness
- help(lang="en") → help in English (ru, zh, de)

Start with tools/list.
```

## Running manually (one process, many requests)

```powershell
cube_v5.exe
```

The server reads stdin in a loop. Send multiple requests to the same process:

```powershell
# Start, type requests, exit with Ctrl+C:
cube_v5.exe
{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}
{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"cubint_mul","arguments":{"a":4,"b":3}}}
```

**Single pipe call (quick check only):**

```powershell
'{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}' | cube_v5.exe
```

For Python testing use `test_persistence.py` (same folder) —
it starts the server once and sends all requests to the same process.
