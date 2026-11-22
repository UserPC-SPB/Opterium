# Cube v5 — Complete Technical Reference

**[← Cube v5 Product Overview](README.md)**

This document is the complete, self-contained technical reference for Cube v5. An unfamiliar AI agent can operate the published executable using only this document; no source code or prior conversation is needed. Section numbering continues from the [Product Overview](README.md) (sections 1–5); this reference covers sections 6–17.

**Orientation only** (the full overview is in the README): Cube v5 is a single, self-contained Windows x64 executable that speaks MCP over stdio and exposes 87 tools. Per the `help` tool's canonical reference (the engine's own documentation, retrievable with the `help` tool, section 9.13), operation results pre-exist in a precomputed verified lookup table — the runtime performs no arithmetic; each call resolves to an address in that table. The matching practice is to audit addresses (witnesses), not values.

## Table of Contents

- [6. Installation and first startup](#6-installation-and-first-startup)
  - [6.1 Installation procedure](#61-installation-procedure)
  - [6.2 Startup behavior](#62-startup-behavior)
  - [6.3 Command-line reference](#63-command-line-reference)
  - [6.4 MCP client configuration](#64-mcp-client-configuration)
  - [6.5 Verifying the installation](#65-verifying-the-installation)
- [7. MCP protocol and integration](#7-mcp-protocol-and-integration)
- [8. AI agent operating guide](#8-ai-agent-operating-guide)
- [9. Complete tool reference](#9-complete-tool-reference)
  - [9.1 Integer arithmetic](#91-integer-arithmetic)
  - [9.2 Decimal-style arithmetic](#92-decimal-style-arithmetic)
  - [9.3 Complex-form operations](#93-complex-form-operations)
  - [9.4 Diagnostics](#94-diagnostics)
  - [9.5 Structural tools](#95-structural-tools)
  - [9.6 Spatial operations](#96-spatial-operations)
  - [9.7 E8 bridge and addresses](#97-e8-bridge-and-addresses)
  - [9.8 E8 root system](#98-e8-root-system)
  - [9.9 E8 batch and statistics](#99-e8-batch-and-statistics)
  - [9.10 E8 table matmul](#910-e8-table-matmul)
  - [9.11 Vector algebra](#911-vector-algebra)
  - [9.12 Verification and symbolic](#912-verification-and-symbolic)
  - [9.13 Utilities](#913-utilities)
- [10. Practical workflows and examples](#10-practical-workflows-and-examples)
- [11. Data formats and conventions](#11-data-formats-and-conventions)
- [12. Errors and troubleshooting](#12-errors-and-troubleshooting)
- [13. Performance characteristics](#13-performance-characteristics)
- [14. Limitations and important behavioral characteristics](#14-limitations-and-important-behavioral-characteristics)
- [15. Version, build identification, and license](#15-version-build-identification-and-license)
- [16. Verification and Witness-Based Workflows](#16-verification-and-witness-based-workflows)
- [17. Elementary Worked Examples: Address Witnesses and Verification](#17-elementary-worked-examples-address-witnesses-and-verification)

---

## 6. Installation and first startup

### 6.1 Installation procedure

1. Put `cube.exe` in a **writable** folder, for example `C:\Tools\cube_v5\cube.exe`. Do not run it from read-only locations such as `Program Files`.
2. The runtime table is generated **next to `cube.exe`**, not next to the MCP client. If your client supports setting the server's working directory, set it to the folder containing `cube.exe`.
3. Either connect an MCP client (the table generates automatically on first start) or generate the table explicitly:

   ```powershell
   .\cube.exe --generate
   ```

   This creates `cube_v5.bin` (measured 272,000,016 bytes). First startup including generation takes about 13 seconds on a reference host.
4. Register the executable in your MCP client (configuration in 6.4) and restart the client.

### 6.2 Startup behavior

Observed startup diagnostics on stderr:

- Warm start (table present): `Cube file found at <path> (size: 259.4 MB)` — the size is printed in MiB.
- Cold start (table absent): `Cube file not found. Generating at <path> (about 13 seconds, 272 MB)...`
- `No cube.sha256 found; skipping hash verification.` — printed only when the optional `cube.sha256` file is absent; when present, the table is hash-verified at startup.
- `Cube initialized` when the server is ready; `Shutting down MCP server` on stdin close.

Typical warm-start stderr sequence:

```text
Starting MCP server, cwd=<dir> ...
MCP server starting: cube_v5 v5.0.0
Cube file found at <path> (size: 259.4 MB)
No cube.sha256 found; skipping hash verification.
Cube initialized
Shutting down MCP server
```

### 6.3 Command-line reference

| Command | Behavior |
|---|---|
| `cube.exe` | Starts the MCP server on stdio (JSON-RPC 2.0). Reads requests from stdin, writes responses to stdout, diagnostics to stderr. |
| `cube.exe --generate` | Creates `cube_v5.bin` in the directory containing `cube.exe` (does not start the server). |
| `cube.exe --generate --force` | Rebuilds `cube_v5.bin` even if it already exists. |
| `cube.exe --version` | Prints `cube_v5 5.0.0`. |
| `cube.exe --help` | Prints the help text below. |
| `cube.exe --<anything-else>` | Unknown flags are **ignored**: the process starts the MCP server on stdio as usual (exit code 0, no error output). |

Observed `--help` output (verbatim):

```text
Cube v5 - exact geometric arithmetic and E8 toolkit

Usage:
  cube.exe                 Start the MCP server on stdio (JSON-RPC 2.0)
  cube.exe --generate      Create cube_v5.bin in this directory
  cube.exe --generate --force
                           Rebuild cube_v5.bin even if it already exists
  cube.exe --version       Print the version
  cube.exe --help          Show this help

Runtime files:
  cube_v5.bin              Generated on first start (~272 MB, about 13 s)
  mcp_debug.log            Written to the system temp directory

MCP client configuration (stdio):
  {
    "mcpServers": {
      "cube_v5": {
        "command": "C:\\path\\to\\cube.exe",
        "args": []
      }
    }
  }

The server is transport-agnostic: it reads JSON-RPC requests from stdin
and writes responses to stdout. Diagnostics go to stderr.
```

No other flags are documented or exposed by the executable.

### 6.4 MCP client configuration

The `mcpServers` block below is exactly the one printed by `cube.exe --help` (the stdio configuration format used by Claude Desktop's `claude_desktop_config.json` and other clients supporting an `mcpServers` block). Clients with a different configuration format should translate this block into their own syntax per their own documentation:

```json
{
  "mcpServers": {
    "cube_v5": {
      "command": "C:\\Tools\\cube_v5\\cube.exe",
      "args": []
    }
  }
}
```

For clients that support it, set the working directory to the folder containing `cube.exe` so that `cube_v5.bin` is generated and found there.

### 6.5 Verifying the installation

- `cube.exe --version` prints `cube_v5 5.0.0`.
- Or connect an MCP client and call the `help` tool: the server returns the canonical machine reference (a self-sufficient per-tool specification with verified micro-examples; 16,425 characters in this build).

## 7. MCP protocol and integration

### 7.1 Transport

- Newline-delimited JSON-RPC 2.0 over stdio: one JSON object per line; requests on stdin, responses on stdout.
- Diagnostics go to stderr.
- The server is synchronous: calls on one connection are processed one at a time.
- Supported methods: `initialize`, `notifications/initialized`, `tools/list`, `tools/call`. Any other method returns JSON-RPC error `-32601`.
- A `timeout_ms` key inside tool arguments is accepted but ignored; tools execute synchronously.

### 7.2 Initialization

The client spawns `cube.exe` (no arguments) with the executable's directory as the working directory, then:

```text
→ {"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"probe","version":"1.0"}}}
← {"jsonrpc":"2.0","id":1,"result":{"protocolVersion":"2024-11-05","serverInfo":{"name":"cube_v5","version":"5.0.0"},"capabilities":{"tools":{"listChanged":false}}}}
→ {"jsonrpc":"2.0","method":"notifications/initialized"}
```

### 7.3 Tool discovery and schema inspection

- `{"jsonrpc":"2.0","id":2,"method":"tools/list"}` returns all **87 tools**, each with `name`, `description`, and `inputSchema`.
- Each tool's `inputSchema` is a JSON schema with `properties` and `required`. Required fields must be supplied; missing required arguments raise an error naming the missing field (section 12).
- The `help` tool returns the canonical machine reference, which contains a signature line and verified example for every tool.

### 7.4 Tool invocation and result extraction

```text
→ {"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"cubint_add","arguments":{"a":"7","b":"5"}}}
← {"jsonrpc":"2.0","id":3,"result":{"content":[{"type":"text","text":"12"}]}}
```

Call a tool with `tools/call` and `{"name": ..., "arguments": {...}}`. Every successful call returns its result as TEXT in `result.content[0].text`: a decimal string (arithmetic), a JSON object/array serialized as text (structured results), or plain text (the `help` tool only). See section 11 for all conventions.

### 7.5 Disconnection and restart

- Closing the client's stdin pipe or terminating the process ends the session. The executable runs as a two-process pair; terminate the whole process tree (for example `taskkill /T /F /PID <pid>` on Windows), not just the first process.
- A blocked call (for example a large diagnostic input) stalls later calls on the same connection. Restart the session rather than queueing more requests.
- After changing or replacing `cube_v5.bin`, call `reset_cube` to discard runtime state; the engine rebuilds lazily on the next tool call.

### 7.6 Quick-start loop

The whole loop is: discover (`tools/list`) → inspect a tool's `inputSchema` → call (`tools/call`) → read `result.content[0].text`.

```text
$> {"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"demo","version":"1.0"}}}
{"jsonrpc":"2.0","id":1,"result":{"protocolVersion":"2024-11-05","serverInfo":{"name":"cube_v5","version":"5.0.0"},"capabilities":{"tools":{"listChanged":false}}}}
$> {"jsonrpc":"2.0","method":"notifications/initialized"}
$> {"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"cubint_add","arguments":{"a":"7","b":"5"}}}
{"jsonrpc":"2.0","id":2,"result":{"content":[{"type":"text","text":"12"}]}}
```

## 8. AI agent operating guide

This section is the operating contract for language models interacting with Cube. It complements — and does not replace — the per-tool entries in section 9.

### 8.1 Reliable workflow

1. **Discover the available tools** with `tools/list`. Do not assume tool names from memory or from another Cube installation.
2. **Identify the relevant functional category** (arithmetic, decimal, complex, diagnostics, structural, spatial, E8 roots, E8 bridge, E8 batch, vectors, table matmul, symbolic, utilities — section 9).
3. **Inspect the exact tool schema** (`inputSchema`) before building arguments: exact parameter names, types, required vs. optional.
4. **Read the tool's documentation**: its entry in section 9 and the canonical `help` reference, which contains verified micro-examples per tool.
5. **Prepare arguments in the required format.** Numbers are generally accepted as JSON numbers or decimal strings (JSON floats are truncated to integers — see 14.2; use strings for values with many digits); vector arguments are JSON arrays of numbers; E8 root tools take integer indices in the documented `0..239` range.
6. **Invoke the tool** via `tools/call`.
7. **Interpret the actual response according to Cube's semantics**, not conventional algebra. Parse numbers as strings when exactness matters (for example, `cubint_truediv(1,3)` returns a 30-digit decimal string).
8. **Chain operations deliberately** when the task requires more than one operation: use the result of one call as an input to the next.
9. **Preserve diagnostic and structural information** in your reasoning and reporting.
10. **Report results without inventing missing information.** If a value is unknown, say so.

### 8.2 Bulk work

- **Never loop single calls for bulk projections.** Use `e8_batch` (count, kinds, per-value dot3) or `e8_batch_timed` (adds engine timing). The canonical help cites ~300 ms for 100,000 projections on a warm process (330k/s); an observed end-to-end figure was ~508 ms (section 13).
- For parallel bulk work, prespawn N persistent worker processes once and multiplex batches over them. Per the canonical help, each process cold-start costs ~200 ms initialization plus ~500 MB RAM. Never respawn a process per task.
- MCP transport adds ~20 ms per call on a reference host, so batching is essential at scale (section 13).

### 8.3 Explicit prohibitions

- **Never assume the internal dimensions of Cube.** The engine owns its addressing, geometry, bounds handling, and data access.
- **Never impose self-invented coordinate or capacity restrictions.** Use the argument formats the API requires; only API-documented restrictions (for example `idx 0..239` for E8 root tools) are public facts.
- **Never reconstruct the engine's internal addressing.**
- **Never substitute conventional algebra for Cube's geometric semantics.** An expression like `(1+2i)` may represent a spatial address in Cube's model; do not judge its operations by textbook complex arithmetic unless the specific tool's documented semantics establish that interpretation.
- **Never judge a geometric operation against an unrelated mathematical expectation.**
- **Never treat an unusual diagnostic verdict** (`REJECTED`, `QUARANTINE`, ...) **as automatic evidence of an engine defect.** Document what the tool reports and what its documented verdict scale means.
- **Never fabricate tool names or argument fields.**
- **Never guess undocumented defaults.**
- **Never silently alter returned values.**
- **Never claim that an untested capability has been verified.**

### 8.4 Error handling (agent side)

- On JSON-RPC error `-32601` (unknown method): use only `initialize`, `notifications/initialized`, `tools/list`, `tools/call`.
- On `-32603` (tool exception): read the message; it names the offending argument, a zero divisor, or an out-of-range index. Fix the arguments and retry; do not change the tool.
- On a call that blocks: terminate and restart that MCP session (the server is synchronous).
- On an unusual but well-formed result (for example `REJECTED`, a route-form witness — witness values may print in ROUTE form such as `42e1`, section 11 — `QUARANTINE`): it is a legitimate Cube output — report it as-is; do not "correct" it.
- A `timeout_ms` key inside arguments is accepted but ignored; tools execute synchronously.

## 9. Complete tool reference

Tool count: **87**, as returned by `tools/list`. The server returns tools in an order that differs from the functional grouping below; the inventory table is the complete list, and every tool has an individual entry in 9.1–9.13.

| # | Tool | Category |
|---|---|---|
| 0 | `cubint_add` | 9.1 Integer arithmetic |
| 1 | `cubint_sub` | 9.1 |
| 2 | `cubint_mul` | 9.1 |
| 3 | `cubint_floordiv` | 9.1 |
| 4 | `cubint_truediv` | 9.1 |
| 5 | `cubint_pow` | 9.1 |
| 6 | `cubint_mod` | 9.1 |
| 7 | `cubint_neg` | 9.1 |
| 8 | `cubint_abs` | 9.1 |
| 9 | `cubint_witness` | 9.1 |
| 10 | `cubint_validate` | 9.1 |
| 11 | `cubfloat_add` | 9.2 Decimal-style arithmetic |
| 12 | `cubfloat_sub` | 9.2 |
| 13 | `cubfloat_mul` | 9.2 |
| 14 | `cubfloat_truediv` | 9.2 |
| 15 | `cubfloat_neg` | 9.2 |
| 16 | `cubfloat_abs` | 9.2 |
| 17 | `cubcomplex_add` | 9.3 Complex-form operations |
| 18 | `cubcomplex_sub` | 9.3 |
| 19 | `cubcomplex_mul` | 9.3 |
| 20 | `cubcomplex_conjugate` | 9.3 |
| 21 | `cubcomplex_abs` | 9.3 |
| 22 | `cubcomplex_pow` | 9.3 |
| 23 | `cubcomplex_neg` | 9.3 |
| 24 | `doctor` | 9.4 Diagnostics |
| 25 | `doctor_pair` | 9.4 |
| 26 | `doctor_pair_coords` | 9.4 |
| 27 | `doctor_group` | 9.4 |
| 28 | `optg_path` | 9.5 Structural tools |
| 29 | `addr3_stack` | 9.5 |
| 30 | `neighbors26` | 9.5 |
| 31 | `spatial_check_support` | 9.6 Spatial operations |
| 32 | `spatial_place` | 9.6 |
| 33 | `spatial_move` | 9.6 |
| 34 | `spatial_align_floor` | 9.6 |
| 35 | `spatial_distance_xy` | 9.6 |
| 36 | `spatial_depth_shift` | 9.6 |
| 37 | `nearest_root` | 9.7 E8 bridge and addresses |
| 38 | `nearest_root_extended` | 9.7 |
| 39 | `e8_address` | 9.7 |
| 40 | `e8` | 9.7 |
| 41 | `e8_get_root` | 9.8 E8 root system |
| 42 | `e8_partners` | 9.8 |
| 43 | `e8_aligned` | 9.8 |
| 44 | `e8_partners_split` | 9.8 |
| 45 | `e8_antipode` | 9.8 |
| 46 | `e8_weyl_depth` | 9.8 |
| 47 | `e8_root_dot` | 9.8 |
| 48 | `e8_dot_spectrum` | 9.8 |
| 49 | `e8_dot` | 9.8 |
| 50 | `e8_distance_matrix` | 9.8 |
| 51 | `e8_triangle_geometry` | 9.8 |
| 52 | `e8_duality_check` | 9.8 |
| 53 | `e8_batch` | 9.9 E8 batch and statistics |
| 54 | `e8_batch_timed` | 9.9 |
| 55 | `e8_stats` | 9.9 |
| 56 | `e8_spectrum_check` | 9.9 |
| 57 | `e8_table_build` | 9.10 E8 table matmul |
| 58 | `e8_table_matmul` | 9.10 |
| 59 | `vec_add` | 9.11 Vector algebra |
| 60 | `vec_sub` | 9.11 |
| 61 | `vec_mul` | 9.11 |
| 62 | `vec_dot` | 9.11 |
| 63 | `vec_sum` | 9.11 |
| 64 | `vec_min` | 9.11 |
| 65 | `vec_max` | 9.11 |
| 66 | `vec_mean_x1000` | 9.11 |
| 67 | `vec_variance_x1000` | 9.11 |
| 68 | `vec_std_x1000` | 9.11 |
| 69 | `vec_norm` | 9.11 |
| 70 | `vec_norm_x1000` | 9.11 |
| 71 | `vec_normalize_x1000` | 9.11 |
| 72 | `vec_normalize_l1_x1000` | 9.11 |
| 73 | `vec_scale` | 9.11 |
| 74 | `vec_cumsum` | 9.11 |
| 75 | `vec_diff` | 9.11 |
| 76 | `vec_clip` | 9.11 |
| 77 | `vec_sort` | 9.11 |
| 78 | `vec_unique` | 9.11 |
| 79 | `warp_dot` | 9.11 |
| 80 | `tension_verify` | 9.12 Verification and symbolic |
| 81 | `cascade_fix` | 9.12 |
| 82 | `symbolic_rewrite` | 9.12 |
| 83 | `symbolic_invariants` | 9.12 |
| 84 | `random_n` | 9.13 Utilities |
| 85 | `reset_cube` | 9.13 |
| 86 | `help` | 9.13 |

### 9.1 Integer arithmetic

All tools take plain integers (JSON numbers or decimal strings); divisions require `b != 0`. Per the canonical help, the exactness guarantees include `a + b - b == a` and `3^100` as an exact integer.

#### `cubint_add`

- **Category:** Integer arithmetic
- **Purpose:** Exact integer addition.
- **Input schema:** `a` (number or decimal string, required), `b` (number or decimal string, required).
- **Parameters:** `a`, `b` — the two addends. Both are accepted as JSON numbers or as decimal strings (use strings for very large values).
- **Operational semantics:** Exact addition; the identity `a + b - b == a` holds by design (address-read execution model).
- **Return format:** Plain decimal string of the sum.
- **Example:** `tools/call {"name":"cubint_add","arguments":{"a":"7","b":"5"}}` → `"12"`; also `{"a":7,"b":3}` → `"10"`.
- **Behavioral notes:** Missing `b` → `-32603` naming `'b'`. Non-integer text → `-32603` `invalid literal for int() with base 10: ...`. A JSON float such as `1.5` is accepted and truncated to an integer (observed: `cubint_add(1.5, 2)` → `3`).
- **Related tools:** `cubint_sub`, `cubint_witness` (inspect the internal mantissa/debt of a value).

#### `cubint_sub`

- **Category:** Integer arithmetic
- **Purpose:** Exact integer subtraction.
- **Input schema:** `a` (required), `b` (required); number or decimal string.
- **Parameters:** `a` − `b`.
- **Operational semantics:** Exact subtraction; negative results are returned as usual signed decimals.
- **Return format:** Plain decimal string (may be negative).
- **Example:** `{"a":7,"b":3}` → `"4"`; `{"a":3,"b":7}` → `"-4"`.
- **Behavioral notes:** Same error behavior as `cubint_add` (missing argument, non-integer literal).
- **Related tools:** `cubint_add` (inverse operation).

#### `cubint_mul`

- **Category:** Integer arithmetic
- **Purpose:** Exact integer multiplication.
- **Input schema:** `a` (required), `b` (required); number or decimal string.
- **Parameters:** The two factors.
- **Operational semantics:** Exact multiplication; results matched independent exact integer computation at 64, 256, 1,024, and 4,096-bit operand sizes.
- **Return format:** Plain decimal string of the product.
- **Example:** `{"a":7,"b":3}` → `"21"`; `{"a":-7,"b":3}` → `"-21"`; 1,024-bit operands → 617-digit product matching independent computation.
- **Behavioral notes:** Same error behavior as `cubint_add`. Size-related conversion failures were observed at 16,384-bit and 65,536-bit seed sizes (section 14); do not treat that as a documented maximum input size.
- **Related tools:** `cubint_pow` (repeated multiplication), `cubint_truediv`.

#### `cubint_floordiv`

- **Category:** Integer arithmetic
- **Purpose:** Integer division of `a` by `b`.
- **Input schema:** `a` (required), `b` (required, non-zero); number or decimal string.
- **Parameters:** Dividend `a`, divisor `b`.
- **Operational semantics:** The observed result truncates toward zero: `cubint_floordiv(-7, 3)` → `-2` (Python's floor division would give `-3`). Despite the name, the observed semantics are truncating division.
- **Return format:** Plain decimal string.
- **Example:** `{"a":7,"b":3}` → `"2"`; `{"a":-7,"b":3}` → `"-2"`; a 4,096-bit value divided by `97` matched an independent check.
- **Behavioral notes:** `b = 0` → `-32603` `division by zero`.
- **Related tools:** `cubint_mod` (companion remainder), `cubint_truediv` (decimal result).

#### `cubint_truediv`

- **Category:** Integer arithmetic
- **Purpose:** Decimal (true) division of `a` by `b`, returned as an exact decimal string.
- **Input schema:** `a` (required), `b` (required, non-zero); number or decimal string.
- **Parameters:** Dividend `a`, divisor `b`.
- **Operational semantics:** Non-terminating quotients are truncated to 30 decimal digits (no `(n)` repeat notation). Terminating quotients are exact (`10 / 4` → `2.5`).
- **Return format:** Decimal string.
- **Example:** `{"a":1,"b":3}` → `"0.333333333333333333333333333333"` (30 digits); `{"a":10,"b":4}` → `"2.5"`.
- **Behavioral notes:** `b = 0` → `-32603` `division by zero`.
- **Related tools:** `cubint_floordiv` (integer part), `cubfloat_truediv`.

#### `cubint_pow`

- **Category:** Integer arithmetic
- **Purpose:** Integer exponentiation `a^b`.
- **Input schema:** `a` (required), `b` (required); number or decimal string.
- **Parameters:** Base `a`, exponent `b`.
- **Operational semantics:** Exact powers for moderate exponents. `0^0` → `1` (observed). The canonical help states `b>=0 enforced`; the observed behavior at negative and very large exponents differs — see the behavioral notes.
- **Return format:** Plain decimal string (integer for the tested non-negative exponents).
- **Example:** `{"a":3,"b":100}` → `"515377520732011331036461129765621272702107522001"`; `{"a":3,"b":0}` → `"1"`; `{"a":0,"b":0}` → `"1"`; `{"a":-2,"b":3}` → `"-8"`.
- **Behavioral notes (documented anomalies, cause unresolved):**
  - `cubint_pow(2, -1)` → `"0.5"` — negative exponents are **not** rejected, contradicting the canonical help's `b>=0 enforced` statement (see section 14).
  - `cubint_pow(3, 10000)` → `"0"` — an incorrect result for a large exponent. Do not rely on this operation for large exponents.
- **Related tools:** `cubcomplex_pow` (complex base/exponent; its negative exponents are also accepted and return decimals).

#### `cubint_mod`

- **Category:** Integer arithmetic
- **Purpose:** Remainder of `a` divided by `b`.
- **Input schema:** `a` (required), `b` (required, non-zero); number or decimal string.
- **Parameters:** Dividend `a`, divisor `b`.
- **Operational semantics:** Remainder with the sign following the **dividend** (fmod-style), as stated in the canonical help. This differs from Python's `%` (whose result follows the divisor): `cubint_mod(-7, 3)` → `-1` (Python gives `2`); `cubint_mod(7, -3)` → `1`.
- **Return format:** Plain decimal string.
- **Example:** `{"a":7,"b":3}` → `"1"`; `{"a":-7,"b":3}` → `"-1"`; `{"a":7,"b":-3}` → `"1"`; a 4,096-bit case matched an independent check.
- **Behavioral notes:** `b = 0` → `-32603` `modulo by zero`.
- **Related tools:** `cubint_floordiv`.

#### `cubint_neg`

- **Category:** Integer arithmetic
- **Purpose:** Unary negation.
- **Input schema:** `a` (required); number or decimal string.
- **Parameters:** Value to negate.
- **Operational semantics:** Exact sign flip.
- **Return format:** Plain decimal string.
- **Example:** `{"a":5}` → `"-5"`; `{"a":-5}` → `"5"`.
- **Behavioral notes:** Missing `a` → `-32603` `'a'`.
- **Related tools:** `cubint_abs`.

#### `cubint_abs`

- **Category:** Integer arithmetic
- **Purpose:** Absolute value.
- **Input schema:** `a` (required); number or decimal string.
- **Parameters:** Value.
- **Operational semantics:** Exact absolute value.
- **Return format:** Plain decimal string.
- **Example:** `{"a":-5}` → `"5"`; `{"a":5}` → `"5"`.
- **Behavioral notes:** Missing `a` → `-32603` `'a'`.
- **Related tools:** `cubint_neg`.

#### `cubint_witness`

- **Category:** Integer arithmetic
- **Purpose:** Inspect the engine's representation of an integer as a witness: mantissa, debt (cursor metadata per the canonical help; observed to print as a ×10^debt ROUTE suffix in the `value` field, e.g. `42e1` for 420), orientation, and string value.
- **Input schema:** `a` (required); number or decimal string.
- **Parameters:** Value to witness.
- **Operational semantics:** Per the canonical help, results pre-exist in the verified table; witnesses are how you audit an address rather than re-checking a value. The `value` field is a **string** and may be in ROUTE form (e.g. `42e1` for 420, `1e1` for 10) rather than plain digits.
- **Return format:** JSON object `{"mantissa": int, "debt": int, "orientation": int, "value": string}`.
- **Example:** `{"a":42}` → `{"mantissa":42,"debt":0,"orientation":0,"value":"42"}`; `{"a":420}` → `{"mantissa":42,"debt":1,"orientation":0,"value":"42e1"}`; `{"a":10}` → `{"mantissa":1,"debt":1,"orientation":0,"value":"1e1"}`; `{"a":0}` → `{"mantissa":0,"debt":0,"orientation":0,"value":"0"}`; `{"a":-42}` → `{"mantissa":42,"debt":0,"orientation":2,"value":"-42"}` (negative values carry `orientation: 2`, positive values `orientation: 0`; the mantissa stays non-negative — observed).
- **Behavioral notes:** Missing `a` → `-32603` `'a'`.
- **Related tools:** `cubint_validate`; all arithmetic tools (their results exist internally in this representation).

#### `cubint_validate`

- **Category:** Integer arithmetic
- **Purpose:** Validate that a value is a well-formed cube integer.
- **Input schema:** `a` (required); number or decimal string.
- **Parameters:** Value to validate.
- **Operational semantics:** Returns a boolean for well-formedness.
- **Return format:** JSON boolean (`true`/`false`).
- **Example:** `{"a":42}` → `true`; `{"a":-5}` → `true`; `{"a":3.7}` → `true` (JSON float truncated to an integer first).
- **Behavioral notes:** Missing `a` → `-32603` `'a'`. Non-integer text does **not** return `false` — it raises `-32603` `invalid literal for int() with base 10: 'abc'`.
- **Related tools:** `cubint_witness`.

### 9.2 Decimal-style arithmetic

Exact decimal arithmetic on (mantissa, debt) pairs; no IEEE floating-point error (`0.1 + 0.2 = 0.3` exactly, per the canonical help). Arguments accept number-or-string; scientific notation is supported (e.g. `'1234567e25'`, `'1e-500'`).

#### `cubfloat_add`

- **Category:** Decimal-style arithmetic
- **Purpose:** Exact decimal addition.
- **Input schema:** `a` (required), `b` (required); JSON number or decimal string (scientific notation accepted).
- **Parameters:** The two addends as exact decimal values.
- **Operational semantics:** Exact decimal addition — `0.1 + 0.2 = 0.3` exactly (no IEEE error). Operands with differing debt values combine exactly (observed); a large positive debt prints compact (e.g. `...e482` style per the canonical help), a large negative debt prints fully expanded.
- **Return format:** Decimal string (possibly the fully expanded form for very small values).
- **Example:** `{"a":"0.1","b":"0.2"}` → `"0.3"`; `{"a":0.1,"b":0.2}` (JSON numbers) → `"0.3"`; `{"a":"-0.0","b":"0"}` → `"0.0"`; `{"a":"1.5","b":"-0.5"}` → `"1"`; `{"a":"1234567e25","b":"1e-500"}` → fully expanded 533-character decimal (exact sum).
- **Behavioral notes:** Missing argument → `-32603` naming the missing field; non-decimal text → `-32603` `invalid literal for int() with base 10: ...`.
- **Related tools:** `cubfloat_sub`, `cubfloat_mul`, `cubfloat_truediv`.

#### `cubfloat_sub`

- **Category:** Decimal-style arithmetic
- **Purpose:** Exact decimal subtraction.
- **Input schema:** `a` (required), `b` (required); number or decimal string.
- **Parameters:** Minuend, subtrahend.
- **Operational semantics:** Exact decimal subtraction.
- **Return format:** Decimal string (may be negative).
- **Example:** `{"a":"1.5","b":"0.5"}` → `"1"`; `{"a":"0.5","b":"1.5"}` → `"-1"`.
- **Behavioral notes:** Same as `cubfloat_add`.
- **Related tools:** `cubfloat_add`.

#### `cubfloat_mul`

- **Category:** Decimal-style arithmetic
- **Purpose:** Exact decimal multiplication.
- **Input schema:** `a` (required), `b` (required); number or decimal string.
- **Parameters:** The two factors.
- **Operational semantics:** Exact decimal multiplication — `0.1 × 0.2 = 0.02` exactly.
- **Return format:** Decimal string.
- **Example:** `{"a":"0.1","b":"0.2"}` → `"0.02"`; the 41-character value `12345678901234567890.12345678901234567890` squared → `152415787532388367504953515625666819450.05334557625361987875019051998750190521` (matched Python `Decimal`); `{"a":"1e-500","b":"1e-500"}` → fully expanded 1,002-character decimal (= 1e-1000).
- **Behavioral notes:** Same as `cubfloat_add`.
- **Related tools:** `cubfloat_add`, `cubfloat_truediv`.

#### `cubfloat_truediv`

- **Category:** Decimal-style arithmetic
- **Purpose:** Decimal division, returned as an exact decimal string.
- **Input schema:** `a` (required), `b` (required, non-zero); number or decimal string.
- **Parameters:** Dividend, divisor.
- **Operational semantics:** Terminating quotients are exact (`1/8` → `0.125`); non-terminating quotients are truncated to 30 decimal digits.
- **Return format:** Decimal string.
- **Example:** `{"a":"1","b":"3"}` → `"0.333333333333333333333333333333"`; `{"a":"1","b":"8"}` → `"0.125"`; `{"a":"0.1","b":"0.3"}` → `"0.333333333333333333333333333333"`.
- **Behavioral notes:** `b = 0` → `-32603` `division by zero`.
- **Related tools:** `cubint_truediv`.

#### `cubfloat_neg`

- **Category:** Decimal-style arithmetic
- **Purpose:** Unary negation of a decimal value.
- **Input schema:** `a` (required); number or decimal string.
- **Parameters:** Value to negate.
- **Operational semantics:** Exact sign flip.
- **Return format:** Decimal string.
- **Example:** `{"a":"-2.5"}` → `"2.5"`.
- **Behavioral notes:** Missing `a` → `-32603` `'a'`.
- **Related tools:** `cubfloat_abs`.

#### `cubfloat_abs`

- **Category:** Decimal-style arithmetic
- **Purpose:** Absolute value of a decimal value.
- **Input schema:** `a` (required); number or decimal string.
- **Parameters:** Value.
- **Operational semantics:** Exact absolute value.
- **Return format:** Decimal string.
- **Example:** `{"a":"-2.5"}` → `"2.5"`; `{"a":"0.3"}` → `"0.3"`.
- **Behavioral notes:** Missing `a` → `-32603` `'a'`.
- **Related tools:** `cubfloat_neg`.

### 9.3 Complex-form operations

Operations on integer (re, im) pairs. Results print as `a + bi` / `a - bi`, or as a plain integer when the imaginary part is zero. The canonical help describes this family **without a conventional complex-algebra interpretation**: for `mul` it states `exact; orientation adds mod 4, no float angles`; for `pow` it gives the verified witness `cubcomplex_pow(1,1,100) -> -1125899906842624 (=2^50*(-1))`; for `abs` it states `sqrt(a^2+b^2) exact int`. Treat returned values as Cube-native results: use them as returned, without re-deriving them in external algebra (section 2).

#### `cubcomplex_add`

- **Category:** Complex-form operations
- **Purpose:** Combine two (re, im) pairs (Cube's complex-form add operation).
- **Input schema:** `a_re`, `a_im`, `b_re`, `b_im` (all required; integers).
- **Parameters:** The two (re, im) operand pairs.
- **Operational semantics:** The canonical help documents no conventional complex-algebra rule for this operation; the observed result for the tested operands is printed in `a ± bi` form and is used as-is.
- **Return format:** Text of the form `"<re> + <im>i"` / `"<re> - <im>i"`.
- **Example:** `{"a_re":1,"a_im":2,"b_re":3,"b_im":4}` → `"4 + 6i"`.
- **Behavioral notes:** Missing required field → `-32603` naming the field; non-integer text → `-32603` `invalid literal for int() with base 10: ...`.
- **Related tools:** `cubcomplex_sub`, `cubcomplex_mul`.

#### `cubcomplex_sub`

- **Category:** Complex-form operations
- **Purpose:** Combine two (re, im) pairs (Cube's complex-form sub operation).
- **Input schema:** `a_re`, `a_im`, `b_re`, `b_im` (all required; integers).
- **Parameters:** The two (re, im) operand pairs.
- **Operational semantics:** Same interface as `cubcomplex_add`; the canonical help documents no conventional complex-algebra rule, and the observed result for the tested operands is used as-is.
- **Return format:** `a ± bi` text.
- **Example:** `{"a_re":3,"a_im":4,"b_re":1,"b_im":2}` → `"2 + 2i"`.
- **Behavioral notes:** Same as `cubcomplex_add`.
- **Related tools:** `cubcomplex_add`.

#### `cubcomplex_mul`

- **Category:** Complex-form operations
- **Purpose:** Combine two (re, im) pairs (Cube's complex-form mul operation).
- **Input schema:** `a_re`, `a_im`, `b_re`, `b_im` (all required; integers).
- **Parameters:** The two (re, im) factor pairs.
- **Operational semantics:** Per the canonical help: `mul(3,4 ; 1,-2) -> '11 - 2i' exact; orientation adds mod 4, no float angles`. The public reference does not document the combination rule as conventional Gaussian algebra; the observed outputs for the tested operands are recorded and used as-is.
- **Return format:** `a ± bi` text (plain integer if the imaginary part is 0).
- **Example:** `{"a_re":3,"a_im":4,"b_re":5,"b_im":6}` → `"-9 + 38i"`; `{"a_re":3,"a_im":4,"b_re":1,"b_im":-2}` → `"11 - 2i"` (the second matches the canonical help witness).
- **Behavioral notes:** Same as `cubcomplex_add`.
- **Related tools:** `cubcomplex_pow`, `cubcomplex_conjugate`.

#### `cubcomplex_conjugate`

- **Category:** Complex-form operations
- **Purpose:** Conjugate operation on a (re, im) pair.
- **Input schema:** `re` (required), `im` (required); integers.
- **Parameters:** The (re, im) pair.
- **Operational semantics:** The observed result is the pair with the sign of the imaginary part flipped (`cubcomplex_conjugate(3,4) -> '3 - 4i'`, matching the canonical help witness).
- **Return format:** `a ± bi` text.
- **Example:** `{"re":3,"im":4}` → `"3 - 4i"`.
- **Behavioral notes:** Missing `im` → `-32603` `'im'`.
- **Related tools:** `cubcomplex_abs`.

#### `cubcomplex_abs`

- **Category:** Complex-form operations
- **Purpose:** Sum-of-squares magnitude per the canonical help (`sqrt(a^2+b^2) exact int`), returned as an exact integer for integral results.
- **Input schema:** `re` (required), `im` (required); integers.
- **Parameters:** The (re, im) pair.
- **Operational semantics:** Per the canonical help, `sqrt(a^2+b^2) exact int` — an exact integer square root of the sum of squares where that value is integral.
- **Return format:** Integer (decimal string).
- **Example:** `{"re":3,"im":4}` → `"5"`; `{"re":-3,"im":4}` → `"5"`; `{"re":5,"im":12}` → `"13"`; `{"re":1,"im":1}` → `"1"` (non-integral magnitude √2 returns the integer part — observed; do not expect a decimal for such inputs).
- **Behavioral notes:** Non-integer `im` → `-32603` `invalid literal for int() with base 10: ...`.
- **Related tools:** `cubcomplex_conjugate`.

#### `cubcomplex_pow`

- **Category:** Complex-form operations
- **Purpose:** Raise a (re, im) base to an integer exponent (Cube's complex-form pow operation).
- **Input schema:** `re` (required), `im` (required), `exp` (required); integers.
- **Parameters:** The (re, im) base pair and an integer exponent.
- **Operational semantics:** Per the canonical help: verified witness `cubcomplex_pow(1,1,100) -> -1125899906842624 (=2^50*(-1))`, "no float angles anywhere". The public reference does not document this operation as conventional complex exponentiation; observed outputs are used as-is.
- **Return format:** `a ± bi` text, or plain integer when the imaginary part is zero.
- **Example:** `{"re":1,"im":1,"exp":100}` → `"-1125899906842624"`; `{"re":3,"im":4,"exp":2}` → `"-7 + 24i"`; `{"re":1,"im":0,"exp":0}` → `"1"`. The `(3,4)` base raised to exponent 1000 returned a 699-digit real part (recorded as-is).
- **Behavioral notes:** Missing field → `-32603` naming the field. Negative exponents are accepted and return decimal values: `{"re":1,"im":0,"exp":-2}` → `"1"`; `{"re":2,"im":0,"exp":-1}` → `"0.5"` (analogous to `cubint_pow(2,-1)`).
- **Related tools:** `cubcomplex_mul`, `cubint_pow` (same negative-exponent behavior).

#### `cubcomplex_neg`

- **Category:** Complex-form operations
- **Purpose:** Unary negation of a (re, im) pair.
- **Input schema:** `re` (required), `im` (required); integers.
- **Parameters:** Real and imaginary parts.
- **Operational semantics:** Sign flip of both parts.
- **Return format:** `a ± bi` text.
- **Example:** `{"re":3,"im":4}` → `"-3 - 4i"`.
- **Behavioral notes:** Missing field → `-32603` naming the field.
- **Related tools:** `cubcomplex_conjugate`.

### 9.4 Diagnostics

Factorization (~20 ms/number per the canonical help), divisor structure, 14-channel pair comparison with verdict scale **PERFECT (14) / OK (≥10) / WARN (≥7) / QUARANTINE (≥4) / REJECTED (<4)** (match-count thresholds per the canonical help), and group agreement.

> **Interpretation note.** A verdict such as `REJECTED` is a classification produced by the diagnostic system on the given inputs; it is not evidence that the underlying operation failed.

#### `doctor`

- **Category:** Diagnostics
- **Purpose:** Full diagnostic report for one integer: factorization, divisor structure, classification, tension measures, a 14-channel self-pair comparison, an axis-doctor closure report, and axis distance.
- **Input schema:** `value` (required; integer).
- **Parameters:** The integer to diagnose.
- **Operational semantics:** Factorization is trial-based and costs ~20 ms/number including overhead (canonical help). The `pair` sub-report compares the value against itself through 14 channels; each channel is `MATCH`, `UNCERTAIN`, or `MISMATCH`. The `axis_doctor` sub-report checks six closures `C1_hxy … C6_dbody` and emits a `signature` (e.g. `FRACTURED:['C4_axis']`).
- **Return format:** JSON object with fields: `factors`, `divisors`, `divisor_count`, `is_perfect`, `is_abundant`, `is_deficient`, `closed_count`, `tension`, `sigma_tension`, `route_quality`, `pair` (`channels`, `match_count`, `uncertain_count`, `mismatch_count`, `net_score`, `weighted_score`, `penalty_multiplier`, `is_closed`, `verdict`), `axis_doctor` (`kind`, `point`, `closures`, `passed`, `failed`, `tension`, `on_axis`, `D_body`, `stack`, `perturbations[]`, `signature`), `axis_distance` (`d_body`, `phase`, `s_lin`, `c_plan`, `volume`).
- **Example:** `{"value":42}` → factors `[2,3,7]`, divisors `[1,2,3,6,7,14,21,42]`, `divisor_count 8`, `is_abundant true`, `tension 2`, `sigma_tension 12`, `route_quality 110`, pair verdict `REJECTED` (match_count 0, weighted_score −240, penalty_multiplier 0), `axis_doctor` point `[2,3,7]` with failed closure `C4_axis`, `axis_distance {"d_body":5,"phase":0,"s_lin":12,"c_plan":41,"volume":42}`, signature `FRACTURED:['C4_axis']`. Also `{"value":420}` → factors `[2,2,3,5,7]`, `divisor_count 24`, `tension 3`, `sigma_tension 504`, `route_quality 190`, pair verdict `REJECTED` (match_count 0); the canonical help's witness for 420 says pair verdict `WARN match_count 8` (see section 14, discrepancies). `{"value":1000003}` (prime) → factors `[1000003]`, verdict `REJECTED`, completed in ~4 ms.
- **Behavioral notes:** Large operands: calls with operands of at least 64 bits did not return within 25–45 s on the reference host (CPU idle); avoid large diagnostic inputs and restart the session if a call blocks. The exact threshold is unknown (section 14).
- **Related tools:** `doctor_pair`, `doctor_group`, `tension_verify`.

#### `doctor_pair`

- **Category:** Diagnostics
- **Purpose:** 14-channel comparison of two integers.
- **Input schema:** `a` (required), `b` (required); integers.
- **Parameters:** The two values to compare.
- **Operational semantics:** Channels: `P_XY`, `P_XZ`, `P_YZ`, `S_XY`, `S_XZ`, `S_YZ`, `|D|_XY`, `|D|_XZ`, `|D|_YZ`, `sgn_XY`, `sgn_XZ`, `sgn_YZ`, `Volume`, `Shared`; each `MATCH`/`UNCERTAIN`/`MISMATCH`. Verdict scale: PERFECT (14 matches), OK (≥10), WARN (≥7), QUARANTINE (≥4), REJECTED (<4); `is_closed` at match_count ≥ 7 (canonical help).
- **Return format:** JSON object: `channels` (14 entries), `match_count`, `uncertain_count`, `mismatch_count`, `net_score`, `weighted_score`, `penalty_multiplier`, `is_closed`, `verdict`.
- **Example:** `{"a":12,"b":21}` → `REJECTED`, match_count 3, weighted_score −160, penalty_multiplier 25, `is_closed false` (matches the canonical help witness). `{"a":42,"b":21}` → `WARN`, match_count 7, `is_closed true`. `{"a":420,"b":210}` → `QUARANTINE`, match_count 4, weighted_score −155, penalty_multiplier 50, `is_closed false` — the canonical help witness claims `PERFECT, match_count 14, weighted_score 285` for this input (see section 14, discrepancies).
- **Behavioral notes:** Missing `a` or `b` → `-32603` naming the field. Large-operand blocking as in `doctor`.
- **Related tools:** `doctor` (the self-pair is embedded in it), `doctor_pair_coords` (coordinate form), `doctor_group` (multi-value).

#### `doctor_pair_coords`

- **Category:** Diagnostics
- **Purpose:** 14-channel comparison of two 3D coordinate points.
- **Input schema:** `x1`, `y1`, `z1`, `x2`, `y2`, `z2` — **all six required** (a missing field raises a KeyError naming the first missing one).
- **Parameters:** The two points in cube coordinates.
- **Operational semantics:** Same 14-channel comparison as `doctor_pair`, applied to explicit coordinate pairs.
- **Return format:** Same structure as `doctor_pair`.
- **Example:** `{"x1":1,"y1":2,"z1":0,"x2":3,"y2":4,"z2":0}` → `WARN`, match_count 8, `is_closed true`.
- **Behavioral notes:** Missing any of the six fields → `-32603` naming the first missing field (e.g. `{"x1":1,"y1":2}` → `'z1'`).
- **Related tools:** `doctor_pair`.

#### `doctor_group`

- **Category:** Diagnostics
- **Purpose:** Group agreement analysis over a set of integers.
- **Input schema:** `values` (required; array of integers; empty array accepted).
- **Parameters:** The group of values.
- **Operational semantics:** Measures pairwise agreements and structural stress of the group.
- **Return format:** JSON object: `agreements`, `total_routes`, `stress`, `is_closed`, plus `canon[]` (canonical representatives with `base`, `perm`, `phase`) and `signatures[]` (e.g. `FRACTURED:['C4_axis']`).
- **Example:** `{"values":[3,7,21]}` → `agreements 16, total_routes 42, stress 61, is_closed false` plus 3 canon entries and 3 `FRACTURED:['C4_axis']` signatures (matches the canonical help witness for the core fields). `{"values":[]}` → `agreements 0, total_routes 1, stress 0, is_closed true, canon [], signatures []`. `{"values":[42,42,42]}` → `agreements 42, total_routes 42, stress 0, is_closed true` with 3 identical canon entries `{base:[2,3,7], perm:[0,1,2], phase:0}` and 3 `FRACTURED:['C4_axis']` signatures (a group of identical values is closed with zero stress).
- **Behavioral notes:** Missing `values` → `-32603` `'values'`.
- **Related tools:** `doctor`, `doctor_pair`.

### 9.5 Structural tools

#### `optg_path`

- **Category:** Structural tools
- **Purpose:** Decompose a value into an OPTG path — a sequence of 3D coordinate steps from the origin.
- **Input schema:** `value` (required; integer).
- **Parameters:** The value to decompose.
- **Operational semantics:** Returns a list of `[x, y, z]` triples forming a step-by-step route; each triple is a move, and the path encodes the value's structure. Interpreting the route beyond its returned coordinates is Cube-specific and should not be re-derived externally.
- **Return format:** JSON array of 3-element integer arrays.
- **Example:** `{"value":7}` → `[[7,0,0],[6,0,0],[3,3,3]]`; `{"value":420}` → `[[42,0,0],[39,3,3],[30,3,3],[3,3,3]]` (both match the canonical help witnesses).
- **Behavioral notes:** Missing `value` → `-32603` `'value'`.
- **Related tools:** `addr3_stack` (stack decomposition), `doctor` (similar structure internally).

#### `addr3_stack`

- **Category:** Structural tools
- **Purpose:** Decompose a value into an address stack — a list of digit-arrays of decreasing arity.
- **Input schema:** `value` (required; integer).
- **Parameters:** The value to decompose.
- **Operational semantics:** Returns a list of integer arrays whose lengths decrease by one per level (e.g. 4 digits, then 3, then 2, then 1). This is Cube's own addressing representation; use it as returned.
- **Return format:** JSON array of integer arrays.
- **Example:** `{"value":42}` → `[[0,2,1,1],[2,1,1],[1,2],[0,1],[2],[1]]`; `{"value":420}` → `[[0,2,1,0,2,1],[0,0,0,2],[2,2],[0,1],[2],[1]]` (both match the canonical help witnesses).
- **Behavioral notes:** Missing `value` → `-32603` `'value'`.
- **Related tools:** `optg_path`, `cubint_witness`.

#### `neighbors26`

- **Category:** Structural tools
- **Purpose:** List the 26-neighborhood (all 3³−1 offset cells) around a grid point.
- **Input schema:** `value` (required; integer).
- **Parameters:** The value, which maps to a grid point. Observed mapping for the tested values: `0 → [0,0,0]`, `5 → [5,0,0]`. The value→center mapping for other values is Cube's own non-public addressing; use the returned center as-is, do not re-derive it externally.
- **Operational semantics:** Returns the 26 neighboring coordinates as `[x,y,z]` arrays around the center point. The returned neighborhood is **not** clipped to a fixed coordinate window — for `value 0` it contains negative coordinates (`[-1,0,0]`, `[0,-1,0]`, ...). The internal grid extent is not part of the public contract and is not documented here; the engine owns its bounds handling.
- **Return format:** JSON array of 26 three-element integer arrays.
- **Example:** `{"value":5}` → 26 entries, e.g. `[6,0,0], [4,0,0], [5,1,0], [5,-1,0], [5,0,1], [5,0,-1], ...`, all at offset ±1 from `[5,0,0]`. `{"value":0}` → 26 entries around `[0,0,0]` including negative coordinates (no error, no clipping).
- **Behavioral notes:** Missing `value` → `-32603` `'value'`. No out-of-range error observed for the tested values.
- **Related tools:** `spatial_check_support`, `spatial_move`.

### 9.6 Spatial operations

FRONT_VIEW_STANDARD (canonical help): **X+ = right, Y+ = up with Y=0 = floor/gravity, Z+ = away.**

#### `spatial_check_support`

- **Category:** Spatial operations
- **Purpose:** Check whether the cell below `(x, y-1, z)` is supported.
- **Input schema:** `x`, `y`, `z` (all required; integers).
- **Parameters:** The point to check.
- **Operational semantics:** The floor plane `y = 0` is always supported. Observed behavior extends this: any `y <= 0` (on or below the floor) is reported supported, while `y > 0` is not — support exists only at/below floor level.
- **Return format:** JSON boolean.
- **Example:** `{"x":3,"y":2,"z":5}` → `false`; `{"x":3,"y":0,"z":5}` → `true`; `{"x":3,"y":-1,"z":5}` → `true`; `{"x":0,"y":-10,"z":0}` → `true`.
- **Behavioral notes:** Missing field → `-32603` naming the field.
- **Related tools:** `spatial_place`, `spatial_align_floor`.

#### `spatial_place`

- **Category:** Spatial operations
- **Purpose:** Place an object at a point; it drops to the floor.
- **Input schema:** `x`, `y`, `z` (required; integers), `obj_type` (optional; string, default `"object"`).
- **Parameters:** Target point and object type label.
- **Operational semantics:** The returned `y` is `min(y, 0)` — a positive input `y` snaps down to the floor (`0`), a negative input `y` passes through unchanged (observed for positive, zero, and negative inputs).
- **Return format:** JSON object `{"x": int, "y": int, "z": int, "placed": true, "obj_type": string}` (with `y` per the semantics above).
- **Example:** `{"x":5,"y":10,"z":3}` → `{"x":5,"y":0,"z":3,"placed":true,"obj_type":"object"}`; `{"x":3,"y":2,"z":5,"obj_type":"object"}` → `{"x":3,"y":0,"z":5,"placed":true,"obj_type":"object"}`; `{"x":-5,"y":2,"z":1}` → `{"x":-5,"y":0,"z":1,"placed":true,"obj_type":"object"}`; `{"x":1,"y":-5,"z":2}` → `{"x":1,"y":-5,"z":2,"placed":true,"obj_type":"object"}` (negative `y` unchanged).
- **Behavioral notes:** Missing field → `-32603` naming the field.
- **Related tools:** `spatial_check_support`, `spatial_move`, `spatial_align_floor` (same `min(y, 0)` rule).

#### `spatial_move`

- **Category:** Spatial operations
- **Purpose:** Move an object by a delta, optionally applying gravity.
- **Input schema:** `x`, `y`, `z` (required), `dx`, `dy`, `dz` (**all required** — pass `0` when no change), `gravity` (optional; boolean).
- **Parameters:** Current point, movement delta along each axis, and gravity flag.
- **Operational semantics:** The delta is applied component-wise. With `gravity: true`, a downward move does not go below the floor (`y` stays at `0`); without gravity, moves are unconstrained (negative coordinates are allowed).
- **Return format:** JSON object `{"x": int, "y": int, "z": int, "moved": true}`.
- **Example:** `{"x":5,"y":0,"z":3,"dx":2,"dy":5,"dz":0}` → `{"x":7,"y":5,"z":3,"moved":true}`; `{"x":5,"y":0,"z":3,"dx":0,"dy":-5,"dz":0,"gravity":true}` → `{"x":5,"y":0,"z":3,"moved":true}` (held at the floor); `{"x":5,"y":0,"z":3,"dx":-7,"dy":1,"dz":2}` → `{"x":-2,"y":1,"z":5,"moved":true}`; `{"x":0,"y":0,"z":0,"dx":0,"dy":0,"dz":0}` → `{"x":0,"y":0,"z":0,"moved":true}` (zero delta).
- **Behavioral notes:** Missing `dz` (or any required field) → `-32603` naming the field. The canonical help's example `move(5,0,3,dx=2,dy=5)` **fails** because `dz` is required — always pass all six positional fields (see also section 14).
- **Related tools:** `spatial_place`, `spatial_check_support`.

#### `spatial_align_floor`

- **Category:** Spatial operations
- **Purpose:** Align a y-coordinate to the floor.
- **Input schema:** `y` (required; integer).
- **Parameters:** The y-value.
- **Operational semantics:** The observed result is `min(y, 0)`: positive values become `0`, negative values are unchanged.
- **Return format:** Integer.
- **Example:** `{"y":10}` → `0`; `{"y":-3}` → `-3`.
- **Behavioral notes:** Missing `y` → `-32603` `'y'`.
- **Related tools:** `spatial_check_support`, `spatial_place` (same `min(y, 0)` rule).

#### `spatial_distance_xy`

- **Category:** Spatial operations
- **Purpose:** Euclidean distance between two points in the XY plane (z is ignored).
- **Input schema:** `x1`, `y1`, `z1`, `x2`, `y2`, `z2` (all required; integers).
- **Parameters:** The two points.
- **Operational semantics:** `sqrt((x2-x1)^2 + (y2-y1)^2)`; the z coordinates are accepted but do not affect the result.
- **Return format:** Decimal string of a float (e.g. `"5.0"`).
- **Example:** `{"x1":0,"y1":0,"z1":0,"x2":3,"y2":4,"z2":0}` → `"5.0"`; the same points with `z2:2` → `"5.0"`.
- **Behavioral notes:** Missing field → `-32603` naming the field. Non-3-4-5 (irrational) distances are not covered by documented examples (section 14).
- **Related tools:** `spatial_move`.

#### `spatial_depth_shift`

- **Category:** Spatial operations
- **Purpose:** Shift a z-coordinate by a number of depth layers.
- **Input schema:** `z` (required), `layers` (required; integer).
- **Parameters:** Current depth and layer offset.
- **Operational semantics:** `z + layers` (positive and negative offsets).
- **Return format:** Integer.
- **Example:** `{"z":0,"layers":3}` → `3`; `{"z":5,"layers":-2}` → `3`.
- **Behavioral notes:** Missing field → `-32603` naming the field.
- **Related tools:** `spatial_place` (z is the depth axis).

### 9.7 E8 bridge and addresses

#### `e8`

- **Category:** E8 bridge and addresses
- **Purpose:** Project a single integer value into 8D: returns the mapped 8D vector, its root classification, and derivation metadata.
- **Input schema:** `value` (required; integer).
- **Parameters:** The integer to project.
- **Operational semantics:** The value is mapped to an 8D vector `vec8` (entries 0/±2), classified as `D8`, `spinor`, or `zero`, with `norm2` (8 for valid roots), `parity_even`, `is_valid_root`, `seed` (observed: `[v/g, v/g, v/g]` where `g` is the `gcd` field; e.g. 1,000,000 → seed `[80,80,80]`, gcd 12,500), and `gcd`.
- **Return format:** JSON object: `vec8` (8 ints), `kind`, `norm2`, `parity_even`, `is_valid_root`, `seed` (3 ints), `gcd`.
- **Example:** `{"value":42}` → `{"vec8":[0,2,0,0,0,-2,0,0],"kind":"D8","norm2":8,"parity_even":true,"is_valid_root":true,"seed":[42,42,42],"gcd":1}`; `{"value":7}` → `vec8 [-2,0,2,0,0,0,0,0]`, `parity_even false`; `{"value":1000000}` → `vec8 [0,0,0,-2,0,-2,0,0]`, `seed [80,80,80]`, `gcd 12500`; `{"value":-42}` → identical to `e8(42)`: `vec8 [0,2,0,0,0,-2,0,0]`, `kind D8`, `seed [42,42,42]`, `gcd 1` (the tested negative value projects exactly like its absolute value).
- **Behavioral notes:** Missing `value` → `-32603` `'value'`.
- **Related tools:** **`e8_batch` for bulk projection — never loop single `e8` calls.**

#### `nearest_root`

- **Category:** E8 bridge and addresses
- **Purpose:** Find the E8 root nearest to a 3D point, with its 8D extension.
- **Input schema:** `x`, `y` (required), `z` (required per the canonical help — "ALL THREE REQUIRED" for the bridge family; the schema lists `z` optional, but bridge calls are documented to need all three — pass all three).
- **Parameters:** The 3D point.
- **Operational semantics:** Returns the nearest root index and vector, the dot product between the input (zero-extended to 8D) and the root, the root kind, and both 3D and 8D forms of the input.
- **Return format:** JSON object: `root_idx`, `root_vec` (8 ints), `dot`, `kind`, `input_3d`, `input_8d`.
- **Example:** `{"x":3,"y":4,"z":0}` → `{"root_idx":0,"root_vec":[2,2,0,0,0,0,0,0],"dot":14,"kind":"D8","input_3d":[3,4,0],"input_8d":[3,4,0,0,0,0,0,0]}` (matches the canonical help); `{"x":1000000,"y":7,"z":3}` → `root_idx 0`, `dot 2000014`; `{"x":1,"y":2,"z":3}` → `root_idx 28`, `root_vec [0,2,2,0,0,0,0,0]`, `dot 10`.
- **Behavioral notes:** Missing field → `-32603` naming the field.
- **Related tools:** `nearest_root_extended`, `e8_address`, `e8`.

#### `nearest_root_extended`

- **Category:** E8 bridge and addresses
- **Purpose:** Nearest-root lookup with five extra dimensions encoding depth/debt/scale into 8D.
- **Input schema:** `x`, `y`, `z` (required), `extra` (optional; array of 5 numbers — pass `[0,0,0,0,0]` when unused).
- **Parameters:** The 3D point plus five extension values.
- **Operational semantics:** Same nearest-root result as `nearest_root`, plus the `extra_dims` echoed back in the result.
- **Return format:** Same as `nearest_root` plus `extra_dims` (5 numbers).
- **Example:** `{"x":1,"y":2,"z":3,"extra":[0,0,0,0,0]}` → `{"root_idx":28,"root_vec":[0,2,2,0,0,0,0,0],"dot":10,"kind":"D8","input_3d":[1,2,3],"input_8d":[1,2,3,0,0,0,0,0],"extra_dims":[0,0,0,0,0]}`.
- **Behavioral notes:** Missing field → `-32603` naming the field. Non-zero `extra` values are not covered by documented examples (section 14).
- **Related tools:** `nearest_root`.

#### `e8_address`

- **Category:** E8 bridge and addresses
- **Purpose:** Convert a 3D point into a full E8 address (root + depth + debt + phase) and recover the corresponding xyz.
- **Input schema:** `x`, `y`, `z` — **all three required** (a missing field raises a KeyError naming the first missing one).
- **Parameters:** The 3D point.
- **Operational semantics:** Returns the E8 address structure (root index, root vector, kind, depth, debt, phase, norm2) and the recovered xyz derived from that address. The recovered coordinates are Cube's own decode — compare them against your input to audit the address; do not re-derive them externally.
- **Return format:** JSON object: `e8_address` `{root_idx, root_vec, kind, depth, debt, phase, norm2}`, `recovered_xyz` (3 ints), `input_xyz` (3 ints).
- **Example:** `{"x":1,"y":2,"z":0}` → `{"e8_address":{"root_idx":0,"root_vec":[2,2,0,0,0,0,0,0],"kind":"D8","depth":1,"debt":0,"phase":0,"norm2":8},"recovered_xyz":[6,6,0],"input_xyz":[1,2,0]}` (matches the canonical help); `{"x":3,"y":4,"z":0}` → depth 2, `recovered_xyz [18,18,0]`.
- **Behavioral notes:** Missing `z` → `-32603` `'z'` (e.g. `{"x":1,"y":2}` fails).
- **Related tools:** `nearest_root`, `e8_get_root` (inspect the root vector for a given index).

### 9.8 E8 root system

240 roots: idx 0..111 D8 (two non-zero ±2 entries), idx 112..239 spinor (all ±1, even number of minus signs). All norm² = 8. Dot spectrum {−8:1, −4:56, 0:126, +4:56, +8:1}. Weyl diameter 2 (canonical help; confirmed below where tested).

#### `e8_get_root`

- **Category:** E8 root system
- **Purpose:** Fetch the 8D vector of the root at the given index.
- **Input schema:** `idx` (required; integer).
- **Parameters:** Root index.
- **Operational semantics:** Roots are decoded on the fly by combinatorics (canonical help: "DECODE, do not GENERATE"). **Documented API range: idx 0..239.** Index 240 raises an index error. Observed: a negative index wraps (Python tuple semantics) — `e8_get_root(-1)` returns root 239.
- **Return format:** JSON array of 8 integers.
- **Example:** `{"idx":0}` → `[2,2,0,0,0,0,0,0]`; `{"idx":111}` → `[0,0,0,0,0,0,-2,-2]` (last D8 root); `{"idx":112}` → `[1,1,1,1,1,1,1,1]` (first spinor); `{"idx":239}` → `[-1,-1,-1,-1,-1,-1,-1,-1]`; `{"idx":-1}` → `[-1,-1,-1,-1,-1,-1,-1,-1]` (wraps to 239).
- **Behavioral notes:** `{"idx":240}` → `-32603` `tuple index out of range`.
- **Related tools:** `e8_partners`, `e8_aligned`, `e8_antipode`, `e8_root_dot`.

#### `e8_partners`

- **Category:** E8 root system
- **Purpose:** List the 56 roots with dot product −4 with the given root.
- **Input schema:** `idx` (required; integer, 0..239).
- **Parameters:** Root index.
- **Operational semantics:** The "partner" shell of the dot spectrum (56 roots at −4).
- **Return format:** JSON array of 56 indices.
- **Example:** `{"idx":0}` → 56 entries: `[6,7,10,11,14,15,18,19,22,23,26,27,30,31,34,35,38,39,42,43,46,47,50,51,113,141,...,239]` (starts with 6,7,10,11,14,15 per the canonical help).
- **Behavioral notes:** `{"idx":240}` → `-32603` `tuple index out of range`.
- **Related tools:** `e8_aligned` (the +4 shell), `e8_partners_split`, `e8_duality_check`.

#### `e8_aligned`

- **Category:** E8 root system
- **Purpose:** List the 56 roots with dot product +4 with the given root.
- **Input schema:** `idx` (required; integer, 0..239).
- **Parameters:** Root index.
- **Operational semantics:** The "aligned" shell of the dot spectrum (56 roots at +4).
- **Return format:** JSON array of 56 indices.
- **Example:** `{"idx":0}` → 56 entries: `[4,5,8,9,12,13,16,17,20,21,24,25,28,29,32,33,36,37,40,41,44,45,48,49,112,126,...,238]` (starts with 4,5,8,9,12,13 per the canonical help).
- **Behavioral notes:** `{"idx":240}` → `-32603` `tuple index out of range`.
- **Related tools:** `e8_partners`.

#### `e8_partners_split`

- **Category:** E8 root system
- **Purpose:** Split the 56 partners of a root into D8 and spinor subsets.
- **Input schema:** `idx` (required; integer, 0..239).
- **Parameters:** Root index.
- **Operational semantics:** For a D8 root: 24 D8 partners + 32 spinor partners (canonical help type decomposition; confirmed for idx 0).
- **Return format:** JSON object `{"D8": [indices], "spinor": [indices]}`.
- **Example:** `{"idx":0}` → `{"D8": [6,7,10,11,14,15,18,19,22,23,26,27,30,31,34,35,38,39,42,43,46,47,50,51], "spinor": [113,141,...,239]}` (24 and 32 entries).
- **Behavioral notes:** As `e8_partners` (index 240 raises an index error).
- **Related tools:** `e8_partners`.

#### `e8_antipode`

- **Category:** E8 root system
- **Purpose:** Index of the antipodal root (dot product −8).
- **Input schema:** `idx` (required; integer, 0..239).
- **Parameters:** Root index.
- **Operational semantics:** Exactly one root has dot −8 with the given root (the negative of it).
- **Return format:** Integer index.
- **Example:** `{"idx":0}` → `3` (root 3 is `[-2,-2,0,...]`; confirmed via `e8_root_dot(0,3)` → −8).
- **Behavioral notes:** As `e8_get_root` (index 240 raises an index error).
- **Related tools:** `e8_root_dot`.

#### `e8_weyl_depth`

- **Category:** E8 root system
- **Purpose:** Weyl-shell depth profile relative to the given root.
- **Input schema:** `idx` (required; integer, 0..239).
- **Parameters:** Root index.
- **Operational semantics:** Counts roots at each dot-product distance shell from the given root; uniform for every root (theorem per the canonical help).
- **Return format:** JSON object: `root_idx`, `depth0` (1 — itself), `depth1` (112), `depth2` (127), `diameter` (2), `weyl_order` (696,729,600).
- **Example:** `{"idx":0}` → `{"root_idx":0,"depth0":1,"depth1":112,"depth2":127,"diameter":2,"weyl_order":696729600}`.
- **Behavioral notes:** As `e8_get_root`.
- **Related tools:** `e8_dot_spectrum`.

#### `e8_root_dot`

- **Category:** E8 root system
- **Purpose:** Dot product of two roots by index.
- **Input schema:** `i` (required), `j` (required); integers, 0..239.
- **Parameters:** The two root indices.
- **Operational semantics:** Exact dot product, computed on the fly (the result carries `computed_on_the_fly: true`).
- **Return format:** JSON object `{"result": int, "computed_on_the_fly": true}`.
- **Example:** `{"i":0,"j":0}` → `{"result":8,"computed_on_the_fly":true}`; `{"i":0,"j":3}` → `{"result":-8,"computed_on_the_fly":true}`.
- **Behavioral notes:** Out-of-range indices raise an index error (per the `e8_get_root` pattern; not separately tested for this tool — see section 14). The canonical help's signature line shows a bare number; the actual return is the structured object (additive, not contradictory).
- **Related tools:** `e8_antipode`, `e8_dot_spectrum`.

#### `e8_dot_spectrum`

- **Category:** E8 root system
- **Purpose:** The full dot-product spectrum of the 240 roots.
- **Input schema:** none (empty object).
- **Parameters:** —.
- **Operational semantics:** Constant spectrum, a theorem per the canonical help: one root at ±8, 56 at ∓4, 126 at 0.
- **Return format:** JSON object mapping dot value → count.
- **Example:** `{}` → `{"-8":1,"-4":56,"0":126,"4":56,"8":1}`.
- **Behavioral notes:** None observed.
- **Related tools:** `e8_weyl_depth`, `e8_spectrum_check`.

#### `e8_dot`

- **Category:** E8 root system
- **Purpose:** Plain 8D dot product of two arbitrary 8-component vectors (not restricted to roots).
- **Input schema:** `a` (required, array of 8 numbers), `b` (required, array of 8 numbers).
- **Parameters:** The two 8D vectors.
- **Operational semantics:** Standard component-wise dot product of the given 8D vectors.
- **Return format:** Integer (decimal string).
- **Example:** `{"a":[2,2,0,0,0,0,0,0],"b":[2,2,0,0,0,0,0,0]}` → `"8"`; `{"a":[2,2,0,0,0,0,0,0],"b":[2,-2,0,0,0,0,0,0]}` → `"0"`.
- **Behavioral notes:** Length mismatch raises an error (the "shape mismatch" pattern of the vector tools; not separately tested for `e8_dot` with wrong lengths).
- **Related tools:** `e8_root_dot` (index-based), `vec_dot`.

#### `e8_distance_matrix`

- **Category:** E8 root system
- **Purpose:** Pairwise squared-distance matrix over a set of root indices.
- **Input schema:** `indices` (required; array of integers, 0..239).
- **Parameters:** The root indices.
- **Operational semantics:** For roots of norm² 8, `|r_i − r_j|² = 8 + 8 − 2·dot(r_i, r_j)`; the matrix is symmetric with zeros on the diagonal.
- **Return format:** JSON 2D array of integers (n×n).
- **Example:** `{"indices":[0,1]}` → `[[0,16],[16,0]]` (matches the canonical help — roots 0 and 1 are orthogonal, distance² = 16); `{"indices":[0]}` → `[[0]]`.
- **Behavioral notes:** Out-of-range indices raise an index error (per the `e8_get_root` pattern; not separately tested).
- **Related tools:** `e8_root_dot`.

#### `e8_triangle_geometry`

- **Category:** E8 root system
- **Purpose:** Geometric analysis of the triangle formed by three roots.
- **Input schema:** `i`, `j`, `k` (all required; integers, 0..239).
- **Parameters:** The three root indices.
- **Operational semantics:** Pairwise dots, squared side length, squared median length (×4 scaling), a ratio scaled ×1000, and `closure_ok` — true iff `r_i + r_j + r_k = 0` (a "tunnel": a closed equilateral triangle of 3 roots; 2,240 total, 28 per root per the canonical help).
- **Return format:** JSON object: `indices`, `dot_ij`, `dot_ik`, `dot_jk`, `side2`, `median2`, `median2_x4`, `ratio_x1000`, `closure_ok`.
- **Example:** `{"i":0,"j":6,"k":31}` (the canonical verified tunnel) → `{"indices":[0,6,31],"dot_ij":-4,"dot_ik":-4,"dot_jk":-4,"side2":24,"median2":18,"median2_x4":72,"ratio_x1000":750,"closure_ok":true}`; `{"i":0,"j":1,"k":2}` → dots 0/0/−8, `side2 16`, `closure_ok false`.
- **Behavioral notes:** Out-of-range indices raise an index error (not separately tested).
- **Related tools:** `e8_partners`, `e8_duality_check`.

#### `e8_duality_check`

- **Category:** E8 root system
- **Purpose:** Verify the duality relation "ref + partner → aligned (dot = +4)" for all 56 partners of a root.
- **Input schema:** `idx` (required; integer, 0..239).
- **Parameters:** The reference root index.
- **Operational semantics:** Checks all 56 partners and reports the count and whether all pass, plus an algebraic proof string (`|r|²=8, dot(r, r+p)=8+(−4)=4, |r+p|²=8`).
- **Return format:** JSON object: `checked`, `all_ok`, `expected`, `algebraic_proof`.
- **Example:** `{"idx":0}` → `{"checked":56,"all_ok":true,"expected":"ref + partner → aligned (dot=+4)","algebraic_proof":"|r|²=8, dot(r,r+p)=8+(-4)=4, |r+p|²=8"}`; `{"idx":239}` → same with `checked 56, all_ok true`.
- **Behavioral notes:** As `e8_get_root`.
- **Related tools:** `e8_partners`, `e8_aligned`.

### 9.9 E8 batch and statistics

#### `e8_batch`

- **Category:** E8 batch and statistics
- **Purpose:** Batch projection of many integer values into 8D — the bulk replacement for repeated single `e8` calls.
- **Input schema:** `values` (required; array of integers), `return_vecs` (optional; boolean, default false).
- **Parameters:** The list of values to project; flag to include the 8D vectors in the result.
- **Operational semantics:** Projects every value once; returns counts by root kind, the count of valid roots, and the per-value dot3. Observed end-to-end: 100,000 values in ~508 ms; the canonical help cites ~300 ms for 100k on a warm process (330k/s).
- **Return format:** JSON object: `count`, `kinds` (`D8`, `spinor`, `zero`), `is_valid_root_count`, `dot3_per_value` (array, one per input), and `vectors` (array of 8D arrays) when `return_vecs` is true.
- **Example:** `{"values":[1,2,3]}` → `{"count":3,"kinds":{"D8":3,"spinor":0,"zero":0},"is_valid_root_count":3,"dot3_per_value":[8,8,8]}`; with `return_vecs: true` → adds `vectors: [[2,2,0,0,0,0,0,0],[2,-2,0,0,0,0,0,0],[-2,2,0,0,0,0,0,0]]`; `{"values":[0..99]}` → `kinds {D8:99, spinor:0, zero:1}` (value 0 is the `zero` kind).
- **Behavioral notes:** Missing `values` → `-32603` `'values'`.
- **Related tools:** `e8_batch_timed` (adds engine timing), `e8_stats`, `e8` (single value).

#### `e8_batch_timed`

- **Category:** E8 batch and statistics
- **Purpose:** Same batch projection as `e8_batch` plus the engine's own elapsed-time measurement.
- **Input schema:** `values` (required; array of integers).
- **Parameters:** The list of values.
- **Operational semantics:** Identical projection results; the added `elapsed_ns`/`elapsed_ms` fields measure engine compute only (MCP transport excluded) — use this to separate engine time from transport time.
- **Return format:** `e8_batch` fields plus `elapsed_ns` (integer) and `elapsed_ms` (integer).
- **Example:** `{"values":[1,2,3]}` → `{"count":3,"kinds":{"D8":3,"spinor":0,"zero":0},"is_valid_root_count":3,"dot3_per_value":[8,8,8],"elapsed_ns":19600,"elapsed_ms":0}`; 1,000 values → 5.16 ms round trip with the engine reporting its own elapsed fields (1.23 ms).
- **Behavioral notes:** Missing `values` → `-32603` `'values'`.
- **Related tools:** `e8_batch`.

#### `e8_stats`

- **Category:** E8 batch and statistics
- **Purpose:** Aggregate statistics over batch-projected values (norm² moments and valid-root ratio).
- **Input schema:** `values` (required; array of integers).
- **Parameters:** The list of values.
- **Operational semantics:** Projects all values and summarizes norm² statistics; the `*_x1000` fields are scaled integers (×1000), not floats.
- **Return format:** JSON object: `count`, `sum_norm2`, `mean_norm2_x1000`, `variance_norm2_x1000`, `min_norm2`, `max_norm2`, `kinds`, `valid_root_ratio_x1000`.
- **Example:** `{"values":[1,2,3]}` → `{"count":3,"sum_norm2":24,"mean_norm2_x1000":8000,"variance_norm2_x1000":0,"min_norm2":8,"max_norm2":8,"kinds":{"D8":3,"spinor":0,"zero":0},"valid_root_ratio_x1000":1000}`; `{"values":[0..99]}` → `sum_norm2 792`, `valid_root_ratio_x1000 990`.
- **Behavioral notes:** Missing `values` → `-32603` `'values'`.
- **Related tools:** `e8_batch`.

#### `e8_spectrum_check`

- **Category:** E8 batch and statistics
- **Purpose:** Self-check of the dot-product spectrum uniformity theorem.
- **Input schema:** `ref_value` (optional; integer, default observed 1), `sample_size` (optional; integer, default observed 240).
- **Parameters:** Reference value/root and sample size.
- **Operational semantics:** Verifies that the dot spectrum is uniform for the reference; returns the expected spectrum and a uniformity flag with a theorem note.
- **Return format:** JSON object: `ref_value`, `expected_spectrum`, `sum` (240), `is_uniform`, `sample_size`, `note`.
- **Example:** `{}` → `{"ref_value":1,"expected_spectrum":{"-8":1,"-4":56,"0":126,"4":56,"8":1},"sum":240,"is_uniform":true,"sample_size":240,"note":"Spectrum is uniform for any root (theorem)."}; `{"sample_size":50}` → same with `sample_size 50`.
- **Behavioral notes:** None observed.
- **Related tools:** `e8_dot_spectrum`.

### 9.10 E8 table matmul

#### `e8_table_build`

- **Category:** E8 table matmul
- **Purpose:** Build/load the 240×240 E8 product cache used by verified matrix operations.
- **Input schema:** none (empty object).
- **Parameters:** —.
- **Operational semantics:** Constructs the product table over the 240 roots; idempotent within a session: a second call reports `build_time_ms: 0.0` (the table persists across calls; in-memory caching is inferred from this behavior).
- **Return format:** JSON object: `size` (240), `entries` (57,600), `build_time_ms`, `min_product`, `max_product`.
- **Example:** `{}` → `{"size":240,"entries":57600,"build_time_ms":0.0,"min_product":0,"max_product":57121}` (first call in the session; total call ~7.9 ms).
- **Behavioral notes:** None observed. The tool description mentions a cache file `data/e8_table_240.json`, but the published build creates **no such file** locally; the 240×240 table persists across calls within a session and there is nothing for the user to manage.
- **Related tools:** `e8_table_matmul`.

#### `e8_table_matmul`

- **Category:** E8 table matmul
- **Purpose:** Exact matrix multiplication, optionally through the verified A3 carry-chain path.
- **Input schema:** `A` (required; 2D array of numbers), `B` (required; 2D array of numbers, inner dimensions must match), `verified` (optional; boolean).
- **Parameters:** The two matrices; `verified: true` selects the slower certifiable A3 carry-chain path (canonical help).
- **Operational semantics:** Standard matrix product semantics, exact. Both the plain path and the verified path returned identical results in the documented tests.
- **Return format:** JSON 2D array.
- **Example:** `{"A":[[1,0],[0,1]],"B":[[5,6],[7,8]]}` → `[[5,6],[7,8]]`; with `verified: true` → `[[5,6],[7,8]]` (slower: ~1 ms vs ~0 ms round trip); `{"A":[[1,2,3],[4,5,6]],"B":[[7,8],[9,10],[11,12]]}` → `[[58,64],[139,154]]` (matches an independent exact calculation).
- **Behavioral notes:** Dimension mismatch raises an error (not separately tested — section 14).
- **Related tools:** `e8_table_build`.

### 9.11 Vector algebra

Exact integer vectors; `*_x1000` tools return scaled integers (×1000), never floats. Elementwise tools accept a vector or a scalar as the second operand.

> **Important observed limitation.** `vec_dot` fails when any element product is negative: `vec_dot([-1,2],[3,-4])` → `-32603` `Cannot decompose negative product: -3`. Use non-negative operands, or split signs, when dot products of signed vectors are needed.

#### `vec_add`

- **Category:** Vector algebra
- **Purpose:** Elementwise addition.
- **Input schema:** `a` (required; array of numbers or scalar), `b` (required; array of numbers **or a scalar integer**).
- **Parameters:** Vector and second vector/scalar.
- **Operational semantics:** Elementwise `a[i] + b` (or `a[i] + b[i]`); a scalar broadcasts over the vector.
- **Return format:** JSON array.
- **Example:** `{"a":[1,2,3],"b":[4,5,6]}` → `[5,7,9]`; `{"a":[1,2],"b":10}` → `[11,12]`.
- **Behavioral notes:** Length mismatch → `-32603` `shape mismatch: 1 vs 2` (observed).
- **Related tools:** `vec_sub`, `vec_mul`, `vec_scale`.

#### `vec_sub`

- **Category:** Vector algebra
- **Purpose:** Elementwise subtraction.
- **Input schema:** `a` (required; array or scalar), `b` (required; array or scalar).
- **Parameters:** Vector and second vector/scalar.
- **Operational semantics:** Elementwise `a[i] - b` (or `a[i] - b[i]`); negatives in the result are fine. The negative-product limitation is documented only for `vec_dot` (section note); it was not observed for `vec_sub`.
- **Return format:** JSON array.
- **Example:** `{"a":[4,5,6],"b":[1,2,3]}` → `[3,3,3]`; `{"a":[1,2],"b":10}` → `[-9,-8]`.
- **Behavioral notes:** Length mismatch → `shape mismatch`.
- **Related tools:** `vec_add`.

#### `vec_mul`

- **Category:** Vector algebra
- **Purpose:** Elementwise multiplication.
- **Input schema:** `a` (required; array or scalar), `b` (required; array or scalar).
- **Parameters:** Vector and second vector/scalar.
- **Operational semantics:** Elementwise product. The documented negative-element-product limitation is established for `vec_dot` (section note); signed-operand behavior of `vec_mul` itself is not covered by documented examples (section 14).
- **Return format:** JSON array.
- **Example:** `{"a":[1,2,3],"b":[4,5,6]}` → `[4,10,18]`; `{"a":[2,3],"b":5}` → `[10,15]`.
- **Behavioral notes:** Length mismatch → `shape mismatch`.
- **Related tools:** `vec_dot`.

#### `vec_dot`

- **Category:** Vector algebra
- **Purpose:** Dot product of two vectors.
- **Input schema:** `a` (required; array of numbers), `b` (required; array of numbers, same length).
- **Parameters:** The two vectors.
- **Operational semantics:** Sum of element products, exact for non-negative integers. **Rejects negative element products** with the error `Cannot decompose negative product: <value>`.
- **Return format:** Integer (decimal string).
- **Example:** `{"a":[1,2,3],"b":[4,5,6]}` → `"32"`. Deterministic arrays `a[i]=(37i)%100`, `b[i]=(53i)%100` at lengths 10/100/1,000/10,000 → `18585 / 249150 / 2491500 / 24915000`, all matching independent exact computation. `{"a":[-1,2],"b":[3,-4]}` → **error** `Cannot decompose negative product: -3`.
- **Behavioral notes:** Length mismatch → `-32603` `shape mismatch: 2 vs 3`; negative element product → error as above. Signed-operand behavior beyond the documented negative-product rejection is not covered by documented examples (section 14).
- **Related tools:** `warp_dot` (experimental cross-check), `vec_mul`, `e8_dot`.

#### `vec_sum`

- **Category:** Vector algebra
- **Purpose:** Sum of all elements.
- **Input schema:** `a` (required; array of numbers; empty array accepted).
- **Parameters:** The vector.
- **Operational semantics:** Exact sum; negative values are fine (observed `[-1,5,-3]` → `1`); empty vector → `0`.
- **Return format:** Integer (decimal string).
- **Example:** `{"a":[1,2,3]}` → `"6"`; `{"a":[-1,5,-3]}` → `"1"`; `{"a":[]}` → `"0"`; length-10,000 deterministic array → `495000` (matches independent computation).
- **Behavioral notes:** None observed.
- **Related tools:** `vec_min`, `vec_max`, `vec_mean_x1000`.

#### `vec_min`

- **Category:** Vector algebra
- **Purpose:** Minimum element.
- **Input schema:** `a` (required; array of numbers).
- **Parameters:** The vector.
- **Operational semantics:** Standard minimum.
- **Return format:** Integer.
- **Example:** `{"a":[3,1,2]}` → `"1"`.
- **Behavioral notes:** Empty vector is not covered by documented examples (section 14).
- **Related tools:** `vec_max`.

#### `vec_max`

- **Category:** Vector algebra
- **Purpose:** Maximum element.
- **Input schema:** `a` (required; array of numbers).
- **Parameters:** The vector.
- **Operational semantics:** Standard maximum.
- **Return format:** Integer.
- **Example:** `{"a":[3,1,2]}` → `"3"`.
- **Behavioral notes:** Empty vector is not covered by documented examples (section 14).
- **Related tools:** `vec_min`.

#### `vec_mean_x1000`

- **Category:** Vector algebra
- **Purpose:** Mean of the elements, scaled ×1000 and returned as an integer.
- **Input schema:** `a` (required; array of numbers).
- **Parameters:** The vector.
- **Operational semantics:** `mean × 1000` as an integer (scaled integer, never a float — canonical help).
- **Return format:** Integer (decimal string).
- **Example:** `{"a":[1,2,3]}` → `"2000"` (= 2.0 × 1000).
- **Behavioral notes:** None observed.
- **Related tools:** `vec_variance_x1000`, `vec_std_x1000`.

#### `vec_variance_x1000`

- **Category:** Vector algebra
- **Purpose:** Variance of the elements, scaled ×1000 and returned as an integer.
- **Input schema:** `a` (required; array of numbers).
- **Parameters:** The vector.
- **Operational semantics:** Population variance × 1000, integer.
- **Return format:** Integer (decimal string).
- **Example:** `{"a":[1,2,3]}` → `"666"` (population variance 0.666… × 1000, integer-truncated per the canonical help example).
- **Behavioral notes:** None observed.
- **Related tools:** `vec_std_x1000` (derived from this value).

#### `vec_std_x1000`

- **Category:** Vector algebra
- **Purpose:** "Standard deviation" measure derived from `vec_variance_x1000`.
- **Input schema:** `a` (required; array of numbers).
- **Parameters:** The vector.
- **Operational semantics:** **CAUTION (canonical help, confirmed by test): this returns the integer square root of `vec_variance_x1000`, not std×1000.** Examples: `[1,2,3]` → variance 666 → `int(sqrt(666))` = **25**; `[2,4,4,4,5,5,7,9]` → `int(sqrt(4000))` = **63**. Do not interpret the result as a conventional standard deviation scaled by 1000.
- **Return format:** Integer (decimal string).
- **Example:** `{"a":[1,2,3]}` → `"25"`; `{"a":[2,4,4,4,5,5,7,9]}` → `"63"` (both match the canonical help caution).
- **Behavioral notes:** None observed.
- **Related tools:** `vec_variance_x1000`.

#### `vec_norm`

- **Category:** Vector algebra
- **Purpose:** Euclidean norm of the vector, exact integer when integral.
- **Input schema:** `a` (required; array of numbers).
- **Parameters:** The vector.
- **Operational semantics:** `sqrt(sum of squares)` returned as an exact integer for integral results.
- **Return format:** Integer (decimal string).
- **Example:** `{"a":[3,4]}` → `"5"`; `{"a":[1,2,2]}` → `"3"`.
- **Behavioral notes:** Non-integral norms are not covered by documented examples (section 14).
- **Related tools:** `vec_norm_x1000`, `vec_normalize_x1000`.

#### `vec_norm_x1000`

- **Category:** Vector algebra
- **Purpose:** Euclidean norm scaled ×1000, returned as an integer.
- **Input schema:** `a` (required; array of numbers).
- **Parameters:** The vector.
- **Operational semantics:** `norm × 1000` as an integer.
- **Return format:** Integer (decimal string).
- **Example:** `{"a":[3,4]}` → `"5000"`.
- **Behavioral notes:** None observed.
- **Related tools:** `vec_norm`.

#### `vec_normalize_x1000`

- **Category:** Vector algebra
- **Purpose:** L2-normalize the vector, scaled ×1000 per component.
- **Input schema:** `a` (required; array of numbers).
- **Parameters:** The vector.
- **Operational semantics:** Each component divided by the L2 norm and scaled ×1000 (integer results).
- **Return format:** JSON array of integers.
- **Example:** `{"a":[3,4]}` → `[600,800]` (matches the canonical help).
- **Behavioral notes:** None observed.
- **Related tools:** `vec_normalize_l1_x1000`.

#### `vec_normalize_l1_x1000`

- **Category:** Vector algebra
- **Purpose:** L1-normalize the vector, scaled ×1000 per component.
- **Input schema:** `a` (required; array of numbers).
- **Parameters:** The vector.
- **Operational semantics:** Each component divided by the L1 norm and scaled ×1000, integer-rounded per component.
- **Return format:** JSON array of integers.
- **Example:** `{"a":[1,2,3]}` → `[166,333,500]` (L1 = 6; 1000/6 ≈ 166.7 → 166, etc.).
- **Behavioral notes:** None observed.
- **Related tools:** `vec_normalize_x1000`.

#### `vec_scale`

- **Category:** Vector algebra
- **Purpose:** Scale a vector by a scalar.
- **Input schema:** `a` (required; array of numbers), `k` (required; scalar).
- **Parameters:** The vector and the scalar.
- **Operational semantics:** Elementwise `a[i] × k`; negative and zero scalars work.
- **Return format:** JSON array.
- **Example:** `{"a":[1,2,3],"k":2}` → `[2,4,6]`; `{"a":[1,2,3],"k":-3}` → `[-3,-6,-9]`; `{"a":[1,2,3],"k":0}` → `[0,0,0]`.
- **Behavioral notes:** None observed.
- **Related tools:** `vec_add` (scalar form).

#### `vec_cumsum`

- **Category:** Vector algebra
- **Purpose:** Cumulative sum.
- **Input schema:** `a` (required; array of numbers).
- **Parameters:** The vector.
- **Operational semantics:** Running prefix sums.
- **Return format:** JSON array (same length).
- **Example:** `{"a":[1,2,3]}` → `[1,3,6]`.
- **Behavioral notes:** None observed.
- **Related tools:** `vec_diff` (inverse).

#### `vec_diff`

- **Category:** Vector algebra
- **Purpose:** First differences (discrete derivative).
- **Input schema:** `a` (required; array of numbers).
- **Parameters:** The vector.
- **Operational semantics:** `a[i+1] - a[i]`; the result is one element shorter.
- **Return format:** JSON array (length − 1).
- **Example:** `{"a":[1,4,9]}` → `[3,5]`.
- **Behavioral notes:** Length-1 input is not covered by documented examples (section 14).
- **Related tools:** `vec_cumsum`.

#### `vec_clip`

- **Category:** Vector algebra
- **Purpose:** Clip each element into the interval `[lo, hi]`.
- **Input schema:** `a` (required; array of numbers), `lo` (required), `hi` (required).
- **Parameters:** The vector and bounds.
- **Operational semantics:** Elementwise `max(lo, min(hi, a[i]))`.
- **Return format:** JSON array.
- **Example:** `{"a":[1,5,9],"lo":2,"hi":8}` → `[2,5,8]`.
- **Behavioral notes:** None observed.
- **Related tools:** `vec_scale`.

#### `vec_sort`

- **Category:** Vector algebra
- **Purpose:** Sort the vector.
- **Input schema:** `a` (required; array of numbers), `reverse` (optional; boolean, default false).
- **Parameters:** The vector and sort direction.
- **Operational semantics:** Standard ascending sort; `reverse: true` sorts descending.
- **Return format:** JSON array.
- **Example:** `{"a":[3,1,2],"reverse":true}` → `[3,2,1]`; `{"a":[3,1,2]}` → `[1,2,3]`.
- **Behavioral notes:** None observed.
- **Related tools:** `vec_unique`.

#### `vec_unique`

- **Category:** Vector algebra
- **Purpose:** Remove duplicate elements (order-preserving, first occurrence kept).
- **Input schema:** `a` (required; array of numbers).
- **Parameters:** The vector.
- **Operational semantics:** Deduplication keeping first-occurrence order.
- **Return format:** JSON array.
- **Example:** `{"a":[1,2,1,3]}` → `[1,2,3]`.
- **Behavioral notes:** None observed.
- **Related tools:** `vec_sort`.

#### `warp_dot`

- **Category:** Vector algebra
- **Purpose:** Experimental "log-N" dot product (canonical help: "EXPERIMENTAL log-N dot … Cross-check against vec_dot before trusting").
- **Input schema:** `a` (required; array of numbers), `b` (required; array of numbers).
- **Parameters:** The two vectors.
- **Operational semantics:** Not established from the public specification. For the tested cases it returned the same value as `vec_dot` (including at length 1,000). Treat results as experimental and cross-check with `vec_dot`.
- **Return format:** Integer (decimal string).
- **Example:** `{"a":[1,2,3],"b":[4,5,6]}` → `"32"` (same as `vec_dot`).
- **Behavioral notes:** Negative-product behavior is not covered by documented examples (section 14). Semantics are EXPERIMENTAL — the tool exists and matches `vec_dot` for the tested cases, but its theoretical interpretation is not established.
- **Related tools:** `vec_dot` (the cross-check reference).

### 9.12 Verification and symbolic

#### `tension_verify`

- **Category:** Verification and symbolic
- **Purpose:** Cheap three-plane closure check `S² − 4P == D_net²` for a pair (a, b), with an optional third value z. Zero-cost self-check (canonical help).
- **Input schema:** `a` (required), `b` (required), `z` (optional).
- **Parameters:** The two values (and optional third).
- **Operational semantics:** Verifies the closure identity across the three planes (xy, xz, yz); returns per-plane `lhs`/`rhs`/`tau` plus a verbose cell dump (divisor/quotient webs, volume terms, rotations).
- **Return format:** JSON object: `product`, `verified` (bool), `max_tau`, `planes[]` (each `plane`, `ok`, `s`, `p`, `d`, `lhs`, `rhs`, `tau`), `cell` (verbose dump).
- **Example:** `{"a":7,"b":3}` → `{"product":21,"verified":true,"max_tau":0, planes all ok (xy: s=10,p=21,d=4, 16=16; xz: 7²=7²; yz: 3²=3²)}`; `{"a":12,"b":35}` → `product 420, verified true` (47² − 4·420 = 529 = 23², matching the canonical help witness); `{"a":7,"b":3,"z":1}` → verified true with z-participating planes.
- **Behavioral notes:** Missing `a`/`b` → `-32603` naming the field.
- **Related tools:** `cascade_fix`, `doctor`, `symbolic_invariants` (the tau_invariant is exactly this identity).

#### `cascade_fix`

- **Category:** Verification and symbolic
- **Purpose:** Attempt to repair an inconsistent (a, b[, z]) relation; reports when nothing needs fixing.
- **Input schema:** `a` (required), `b` (required), `z` (optional).
- **Parameters:** The values to check/fix.
- **Operational semantics:** For consistent inputs it reports `fixed: false` with reason `already_consistent`, the product, and the residual tension `tau`.
- **Return format:** JSON object: `fixed` (bool), `reason`, `product`, `tau`.
- **Example:** `{"a":7,"b":3}` → `{"fixed":false,"reason":"already_consistent","product":21,"tau":0}`; `{"a":12,"b":35}` → `{"fixed":false,"reason":"already_consistent","product":420,"tau":0}`.
- **Behavioral notes:** Missing `a`/`b` → `-32603` naming the field. The inconsistent-input (fixing) path is not covered by documented examples (section 14).
- **Related tools:** `tension_verify`.

#### `symbolic_rewrite`

- **Category:** Verification and symbolic
- **Purpose:** Rewrite a symbolic expression into its normal form.
- **Input schema:** `expr_type` (required; one of `tau`, `coord`, `valid`, `ternary`), `args` (required; object).
- **Parameters:** Expression type and its arguments (`tau` uses `{S, P, D}`; `coord`/`ternary` use `{x, y, z}`; `valid` uses `{value}`).
- **Operational semantics:** Produces the normal form plus a (usually empty) derivation trace. Unknown types return an error object rather than a JSON-RPC error.
- **Return format:** JSON object: `normal_form` (string) and `trace` (array) — or `{"error": "Unknown expr_type: <type>"}`.
- **Example:** `{"expr_type":"tau","args":{"S":5,"P":6,"D":1}}` → `{"normal_form":"τ(5, 6, 1)","trace":[]}`; `{"expr_type":"coord","args":{"x":1,"y":2,"z":3}}` → `{"normal_form":"(1:2:3)[XY]","trace":[]}`; `{"expr_type":"valid","args":{"value":42}}` → `{"normal_form":"True","trace":["tau_valid: valid({'value': 42}) → True"]}`; `{"expr_type":"ternary","args":{"x":1,"y":2,"z":3}}` → `{"normal_form":"<>","trace":[]}` (placeholder); `{"expr_type":"bogus"}` → `{"error":"Unknown expr_type: bogus"}`.
- **Behavioral notes:** Unknown `expr_type` → in-result error object (above).
- **Related tools:** `symbolic_invariants`, `tension_verify`.

#### `symbolic_invariants`

- **Category:** Verification and symbolic
- **Purpose:** List the known symbolic invariants of the system.
- **Input schema:** none (empty object).
- **Parameters:** —.
- **Operational semantics:** Returns the invariant formulas as strings (first-order pattern notation).
- **Return format:** JSON object: `invariants` with `coord_symmetry`, `tau_invariant`, `e8_self_dot`.
- **Example:** `{}` → `{"invariants":{"coord_symmetry":"((?x:?y:?z)[XY] = (?z:?x:?y)[XY])","tau_invariant":"(τ(?S, ?P, ?D) = (((?S * ?S) - (4 * ?P)) - (?D * ?D)))","e8_self_dot":"((?r dot ?r) = 8)"}}`.
- **Behavioral notes:** None observed.
- **Related tools:** `symbolic_rewrite`, `tension_verify`.

### 9.13 Utilities

#### `random_n`

- **Category:** Utilities
- **Purpose:** Generate n deterministic pseudo-random integers (LCG, period 2³¹−1 per the canonical help).
- **Input schema:** `n` (required; integer), `seed` (optional; integer — the response shows default 42 when omitted), `bound` (optional; integer; `bound > 0` → values in `[0, bound)`).
- **Parameters:** Count, seed, upper bound.
- **Operational semantics:** Deterministic given (n, seed, bound): `random_n(5, seed=42, bound=100)` is stable across calls and processes (canonical help; confirmed). **Observed response quirk:** when `bound` is absent/0 the top-level `values` field is itself a nested result object (`{"values": [...], "n": ..., "seed": ..., "bound": 0}`) instead of a plain list; with `bound > 0` it is a plain list. Parse accordingly.
- **Return format:** JSON object: `values` (list — or nested object when bound ≤ 0, see quirk), `n`, `seed`, `bound`.
- **Example:** `{"n":5,"seed":42,"bound":100}` → `{"values":[27,64,53,6,35],"n":5,"seed":42,"bound":100}` (matches the canonical help witness); `{"n":3,"seed":42}` → `{"values":{"values":[1250496027,1116302264,1000676753],"n":3,"seed":42,"bound":0},"n":3,"seed":42}` (nested quirk); `{"n":3,"bound":10}` → `{"values":[7,4,3],"n":3,"seed":42,"bound":10}`; `{"n":0}` → nested empty.
- **Behavioral notes:** `n = 0` returns empty values (no error). Missing `n` → `-32603` `'n'`. Period/sequence properties beyond the tested arguments are not established (section 14).
- **Related tools:** — (standalone utility).

#### `reset_cube`

- **Category:** Utilities
- **Purpose:** Reset runtime state after data-file changes (e.g. after replacing `cube_v5.bin`).
- **Input schema:** none (empty object).
- **Parameters:** —.
- **Operational semantics:** Discards cached runtime state; the engine rebuilds lazily on the next tool call (observed: an `e8_get_root` call immediately after reset succeeded with no error).
- **Return format:** JSON object: `status` (`"ok"`), `message` (`"Cube reset. Will rebuild on next use."`).
- **Example:** `{}` → `{"status":"ok","message":"Cube reset. Will rebuild on next use."}`; a subsequent `e8_get_root(0)` → `[2,2,0,0,0,0,0,0]`.
- **Behavioral notes:** None observed.
- **Related tools:** — (call after manually modifying/regenerating `cube_v5.bin`).

#### `help`

- **Category:** Utilities
- **Purpose:** Return the canonical machine reference — the self-sufficient per-tool specification with verified micro-examples.
- **Input schema:** `lang` (optional; **ignored** — accepted for backward compatibility only).
- **Parameters:** —.
- **Operational semantics:** Always returns the same canonical document (16,425 characters in this build) regardless of `lang`.
- **Return format:** Text in `content[0].text` (not JSON).
- **Example:** `{"lang":"en"}` → the full machine reference (sections [0] Transport and Errors through [15] Unique Structural Knowledge).
- **Behavioral notes:** None observed.
- **Related tools:** Every tool (the help contains a signature line and verified example for each).

## 10. Practical workflows and examples

All examples below are directly executable MCP `tools/call` invocations against the published build, demonstrating Cube's actual semantics.

### 10.1 First call (raw JSON-RPC over stdio)

```text
$> {"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"demo","version":"1.0"}}}
{"jsonrpc":"2.0","id":1,"result":{"protocolVersion":"2024-11-05","serverInfo":{"name":"cube_v5","version":"5.0.0"},"capabilities":{"tools":{"listChanged":false}}}}
$> {"jsonrpc":"2.0","method":"notifications/initialized"}
$> {"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"cubint_add","arguments":{"a":"7","b":"5"}}}
{"jsonrpc":"2.0","id":2,"result":{"content":[{"type":"text","text":"12"}]}}
```

### 10.2 Exact decimal where IEEE would fail

Task: verify `0.1 + 0.2` exactly.

```text
tools/call {"name":"cubfloat_add","arguments":{"a":"0.1","b":"0.2"}}
→ "0.3"          (exact; IEEE double would give 0.30000000000000004)
tools/call {"name":"cubfloat_truediv","arguments":{"a":"1","b":"3"}}
→ "0.333333333333333333333333333333"   (30 digits, truncated; parse as string)
```

### 10.3 Complex-form power and product

Task: call the complex-form `pow` and `mul` tools and use the returned values as-is (no external-algebra interpretation — see 9.3).

```text
tools/call {"name":"cubcomplex_pow","arguments":{"re":1,"im":1,"exp":100}}
→ "-1125899906842624"      (plain integer: imaginary part is exactly 0)
tools/call {"name":"cubcomplex_mul","arguments":{"a_re":3,"a_im":4,"b_re":5,"b_im":6}}
→ "-9 + 38i"
```

### 10.4 Diagnose a number, then verify its closure

Task: understand the structure of 420 and confirm it closes exactly.

```text
tools/call {"name":"doctor","arguments":{"value":420}}
→ factors [2,2,3,5,7], divisor_count 24, tension 3, sigma_tension 504, route_quality 190 (field meanings: section 9.4),
  pair verdict REJECTED (self-pair), axis_distance {d_body:4, phase:0, s_lin:23, c_plan:172, volume:420}

tools/call {"name":"doctor_pair","arguments":{"a":420,"b":210}}
→ QUARANTINE, match_count 4, weighted_score -155   (see section 14 for the help-witness discrepancy)

tools/call {"name":"tension_verify","arguments":{"a":12,"b":35}}
→ {"product":420,"verified":true,"max_tau":0}      (47² − 4·420 = 529 = 23²)
```

Next operation if the pair verdict matters: compare with `doctor_pair_coords` on explicit coordinates, or run `doctor_group` over the whole factor set.

### 10.5 Bulk E8 projection (never loop single calls)

Task: classify 1,000 values against the E8 root system.

```text
tools/call {"name":"e8_batch","arguments":{"values":[0,1,2,...,999]}}
→ {"count":1000,"kinds":{"D8":865,"spinor":134,"zero":1},"is_valid_root_count":999,"dot3_per_value":[...]}
   (observed round trip ~4.0 ms for this call)

tools/call {"name":"e8_batch_timed","arguments":{"values":[0,...,999]}}
→ same result plus {"elapsed_ns":1227400,"elapsed_ms":1}   (engine-side time only, transport excluded)

tools/call {"name":"e8_stats","arguments":{"values":[0,...,999]}}
→ {"count":1000,"sum_norm2":7992,"mean_norm2_x1000":8000,"variance_norm2_x1000":0,"min_norm2":8,"max_norm2":8,"kinds":{"D8":865,"spinor":134,"zero":1},"valid_root_ratio_x1000":999}
   (mean/min/max are over the 999 valid roots; `valid_root_ratio_x1000` is over all 1000 values)
```

At 100,000 values the same call took ~508 ms end-to-end on the reference host; the canonical help cites ~300 ms (330k/s) on its warm-process measurement.

### 10.6 Walk the E8 structure of one root

Task: explore root 0 and its tunnel.

```text
tools/call {"name":"e8_get_root","arguments":{"idx":0}}
→ [2,2,0,0,0,0,0,0]

tools/call {"name":"e8_partners_split","arguments":{"idx":0}}
→ {"D8": [6,7,10,11,14,15,...] (24), "spinor": [113,141,...] (32)}

tools/call {"name":"e8_triangle_geometry","arguments":{"i":0,"j":6,"k":31}}
→ {"dot_ij":-4,"dot_ik":-4,"dot_jk":-4,"side2":24,"closure_ok":true, ...}   (a tunnel)

tools/call {"name":"e8_duality_check","arguments":{"idx":0}}
→ {"checked":56,"all_ok":true,"expected":"ref + partner → aligned (dot=+4)", ...}
```

### 10.7 Vector pipeline

Task: normalize, clip, sort, dedupe — all in exact integers.

```text
tools/call {"name":"vec_normalize_l1_x1000","arguments":{"a":[1,2,3]}}   → [166,333,500]
tools/call {"name":"vec_clip","arguments":{"a":[1,5,9],"lo":2,"hi":8}}   → [2,5,8]
tools/call {"name":"vec_sort","arguments":{"a":[3,1,2],"reverse":true}}  → [3,2,1]
tools/call {"name":"vec_unique","arguments":{"a":[1,2,1,3]}}             → [1,2,3]
tools/call {"name":"vec_mean_x1000","arguments":{"a":[1,2,3]}}           → 2000
```

### 10.8 Witness audit (address check, not value check)

Task: audit how the engine represents 420.

```text
tools/call {"name":"cubint_witness","arguments":{"a":420}}
→ {"mantissa":42,"debt":1,"orientation":0,"value":"42e1"}

tools/call {"name":"cubint_validate","arguments":{"a":420}}
→ true
```

Per the execution model (README section 2), audit addresses (witnesses); do not re-check values.

### 10.9 Address round-trip through the E8 bridge

Task: map a 3D point to its E8 address and read back the recovered coordinates.

```text
tools/call {"name":"e8_address","arguments":{"x":1,"y":2,"z":0}}
→ {"e8_address":{"root_idx":0,"root_vec":[2,2,0,0,0,0,0,0],"kind":"D8","depth":1,"debt":0,"phase":0,"norm2":8},
   "recovered_xyz":[6,6,0],"input_xyz":[1,2,0]}

tools/call {"name":"nearest_root","arguments":{"x":1,"y":2,"z":3}}
→ {"root_idx":28,"root_vec":[0,2,2,0,0,0,0,0],"dot":10,"kind":"D8", ...}
```

### 10.10 Deterministic test data

Task: generate reproducible random inputs for your own tests.

```text
tools/call {"name":"random_n","arguments":{"n":5,"seed":42,"bound":100}}
→ {"values":[27,64,53,6,35],"n":5,"seed":42,"bound":100}    (stable across runs)
```

### 10.11 Matrix multiplication

Task: verify exact 2×3 · 3×2.

```text
tools/call {"name":"e8_table_matmul","arguments":{"A":[[1,2,3],[4,5,6]],"B":[[7,8],[9,10],[11,12]]}}
→ [[58,64],[139,154]]     (matches an independent exact calculation)
```

## 11. Data formats and conventions

- **Transport framing.** Newline-delimited JSON-RPC 2.0. One JSON object per line on stdout; requests on stdin.
- **Result envelope.** Every successful `tools/call` returns `result.content[0].text`. The text is a decimal string (arithmetic), a JSON object/array string (structured results), or plain text (only `help`).
- **Numbers as strings.** Parse result text as strings when exactness matters. Integer results are plain decimal digits (possibly with a leading minus). Decimal results use a `.` separator; non-terminating divisions are truncated to 30 fractional digits.
- **ROUTE (scientific) form for integers.** The internal representation is `(mantissa, debt, orientation)`. `str()`/witness `value` fields may print in ROUTE form: `1e1` = 1·10¹ = 10, `42e1` = 420. Do not assume plain digits in witness value fields; arithmetic tool outputs were observed in plain decimal form for all tested values.
- **CubFloat printing.** Positive-debt results print compact (e.g. `1234567890123456789e482` style); negative-debt results print fully expanded (e.g. 1e-1000 as a 1,002-character decimal). Scientific-notation inputs (`'1234567e25'`, `'1e-500'`) are accepted.
- **Scaled-integer (`_x1000`) fields.** Tools named `*_x1000` return the quantity ×1000 as an integer, never a float. Exception to the naive reading: `vec_std_x1000` is `int(sqrt(variance_x1000))`, not std×1000 (see 9.11).
- **Complex-form results.** Printed as `"a + bi"` / `"a - bi"`; plain integer string when the imaginary part is 0.
- **Vectors.** JSON arrays of numbers (integers in all tested usage). `*_x1000` vector tools return integer arrays. Scalar broadcast is supported for elementwise tools (`b` may be a scalar).
- **8D vectors.** Arrays of exactly 8 integers (entries 0/±2 for roots).
- **Root indices.** Integers 0..239 (documented API range of the `e8_*` root tools). Index 240 raises an index error; a negative index wraps (observed on `e8_get_root`).
- **Spatial coordinates.** FRONT_VIEW_STANDARD: X+ right, Y+ up (Y=0 floor/gravity), Z+ away. `spatial_distance_xy` returns a float string (e.g. `"5.0"`) and ignores z.
- **Diagnosis verdicts.** `PERFECT` (14 channel matches), `OK` (≥10), `WARN` (≥7), `QUARANTINE` (≥4), `REJECTED` (<4); `is_closed` at match_count ≥ 7. Channel states: `MATCH` / `UNCERTAIN` / `MISMATCH`.
- **Booleans and JSON.** Standard JSON booleans/objects/arrays in structured results.
- **Deterministic randomness.** `random_n` is a deterministic LCG (period 2³¹−1); identical (n, seed, bound) give identical sequences. Note the response-shape quirk when `bound` is absent (see 9.13).
- **No internal dimensions exposed.** The public API exposes no coordinate-space limits beyond the documented ones above (e.g. the E8 root index range 0..239). The engine owns its addressing, geometry, bounds handling, and data access; the internal Cube dimensions are not public and must not be inferred or published.

## 12. Errors and troubleshooting

### 12.1 Error patterns

| Condition | Observed response |
|---|---|
| Unknown JSON-RPC method | `{"jsonrpc":"2.0","error":{"code":-32601,"message":"Unknown method: bogus/method"},"id":N}` |
| Unknown tool name in `tools/call` | `-32603` with message `Unknown tool: <name>` |
| Missing required tool argument | `-32603` with the **name** of the missing field (e.g. `'b'`, `'z1'`, `'im'`, `'values'`) |
| Non-integer text where an integer is required | `-32603` `invalid literal for int() with base 10: '<text>'` |
| Division/modulo by zero | `-32603` `division by zero` / `modulo by zero` |
| Out-of-range E8 root index (e.g. 240) | `-32603` `tuple index out of range` |
| Vector length mismatch | `-32603` `shape mismatch: <n> vs <m>` |
| `vec_dot` with a negative element product | `-32603` `Cannot decompose negative product: <value>` |
| Unknown `symbolic_rewrite` expr_type | Not a JSON-RPC error: the result contains `{"error":"Unknown expr_type: <type>"}` |

### 12.2 Protocol behavior

- **Missing-argument errors name the first missing required field** (observed for `cubint_add`, `cubfloat_add`, `cubcomplex_conjugate`, `doctor_pair_coords`, `e8_address`, `spatial_move`).
- **The process survives all observed error cases.** A valid call after an error succeeds; the session remains usable.
- **Synchronous server.** One call at a time per connection; a blocked call (e.g. large diagnostic inputs) stalls later calls on that connection. Restart the session rather than queueing.
- **`timeout_ms` in arguments is accepted but ignored** (see 7.1).
- **Blocking risk.** `doctor` / `doctor_pair` with operands of at least 64 bits did not return within 25–45 s on the reference host (CPU idle). Use controlled client-side timeouts and terminate/restart the affected session if a call blocks.

### 12.3 Recovery procedures (agent side)

1. On `-32601`: check the method name; only the four methods of 7.1 exist.
2. On `-32603`: read the message; it names the offending field, value, or condition. Fix the arguments and retry; do not change the tool.
3. On a blocked call: stop the process for that session, restart it, and avoid re-sending the same large input without a reason.
4. On an unusual but well-formed result (e.g. `REJECTED`, a route-form witness, `QUARANTINE`): it is a legitimate Cube output — document/report it as-is; do not "correct" it.

### 12.4 Troubleshooting table

| Symptom | Fix |
|---|---|
| `Access is denied` when generating | Move `cube.exe` to a writable folder; do not run it from `Program Files` |
| Client cannot connect | Check that the `command` path in the MCP config points to `cube.exe` and that `args` is `[]` |
| First start is slow (~13 s) | Expected: the first start generates the 272 MB table `cube_v5.bin`; warm starts measured ~0.2–0.3 s (section 13) |
| The table is generated somewhere unexpected, or not found | The table is created **next to `cube.exe`**, not next to the client — set the MCP server's working directory to the folder containing `cube.exe` (section 7) |
| Antivirus warning | Single-file executables are sometimes flagged heuristically; add an exclusion if you trust the build |
| Cube hash mismatch | Delete `cube_v5.bin` (and `cube.sha256`) and run `cube.exe --generate` again |
| `Bad magic` in the log | The table may be truncated or incompatible; preserve a copy if needed, then regenerate it with `cube.exe --generate --force` |
| A `tools/call` returns `-32603` naming a field | That tool requires the named argument; pass all required fields of its `inputSchema` (section 12.1) |
| Behavior seems stale after regenerating or replacing `cube_v5.bin` | Call `reset_cube` (9.13) — runtime state is discarded and the engine rebuilds lazily on the next tool call |
| A very large integer operation returns an error or an implausible result | Large arithmetic calls at tested sizes returned an integer/string-conversion error while parsing decimal-string operands; `cubint_pow(3, 10000)` returned `0`. Do not use these cases as exact computations (section 14) |
| A diagnostic call appears stuck | `doctor` / `doctor_pair` operands of at least 64 bits did not return within 25–45 s while CPU was idle. A blocked call can hold up later calls in the same server session; restart that session rather than queueing more requests |
| The server does not exit, or processes remain after disconnect | The executable runs as a two-process pair; terminate the whole process tree (see 7.5), not just the first process (README section 5) |

## 13. Performance characteristics

All figures below are single-run, host-specific observations on the reference host (Windows x64, CPU only, published build) — representative values, **not** throughput or latency guarantees. Timings are end-to-end MCP round trips (including stdio serialization) unless marked as engine time.

### 13.1 Startup, table, and memory

| Measurement | Observed |
|---|---:|
| Cold start (no table), including generation through MCP `initialize` | 12.9–13.4 s |
| `--generate --force` rebuild | ~13.2 s, byte-identical table (same SHA-256) |
| Warm start (table present), through MCP `initialize` | 0.21–0.29 s |
| Generated table size | 272,000,016 bytes |
| Process working set after startup / small call | 12.3 MB cold; 12.4 MB warm |

Startup timing depends on hardware, storage, and antivirus scanning; the memory figures are working-set snapshots, not peak-memory measurements.

### 13.2 Arithmetic

- Canonical help cost model (measured on a warm process): comparison ~0.3 µs independent of magnitude; add/sub/mul ~1.6–2.9 µs typical; scale jumps ×10^k cost the same for any k.
- `cubint_add(7,5)`: ≤1 ms per call (5 calls).
- `cubint_mul` at 1,024-bit operands: ~1.0–1.2 ms round trip (617-digit product, exact match against independent computation); results also matched at 64/256/256/4,096-bit sizes.
- `cubint_floordiv` / `cubint_mod` at 4,096-bit values: ~0.24 / ~0.19 ms.
- `cubint_pow(3,100)` / `cubint_pow(3,1000)`: ~0.24 / ~0.29 ms (exact). `cubint_pow(3,10000)` returned `0` — see section 14.
- Decimal operations on the documented 41-character value: ~0.16–0.18 ms round trip (results matched Python `Decimal`).
- Complex-form: `(3+4i)×(5+6i)` ~0.23 ms; `(3+4i)^1000` (699-digit real part) ~0.81 ms; magnitude of `3+4i` ~0.18 ms.

The documented decimal checks cover the listed examples only; they do not establish arbitrary precision or correctness for every decimal length. Integer arithmetic beyond the 4,096-bit size is a size-related limitation of the published build's exposed interface (section 14), not an established maximum input size.

### 13.3 Vectors

| Operation | Size (n) | Observed round trip |
|---|---|---:|
| `vec_dot` | 10 / 100 / 1,000 / 10,000 | ~1.0 / ~4.5 / ~32 / ~295 ms (all exact vs independent computation) |
| `vec_sum` | 10 / 100 / 1,000 / 10,000 | ≤1 ms / ~1.0 / ~0.6 / ~1.2 ms |

`vec_dot` cost grows approximately linearly with n (×~29 per ×10 of length in the tested range). `warp_dot` at n = 1,000 matched `vec_dot` (observed median ~16.6 ms).

### 13.4 E8 batch throughput

| Operation | Size (n) | Observed |
|---|---|---:|
| `e8_batch` | 100 / 1,000 / 10,000 / 100,000 | ~1.0 / ~4.0 / ~47.1 / ~508 ms end-to-end |
| `e8_batch_timed` | 1,000 | 5.16 ms round trip; **engine reports 1.23 ms** (`elapsed_ns 1,227,400`) — the difference is transport/serialization |
| `e8_table_build` | first build in session | ~7.9 ms total (table then cached; `build_time_ms 0.0` on subsequent loads) |
| `e8_table_matmul` | 2×3 · 3×2 | ~1 ms (verified path: ~1 ms vs ~0 ms round trip) |

At 100,000 values, `e8_batch` ran at ~197k values/s end-to-end on this host; the canonical help cites ~330k/s engine-side. The engine's own `elapsed_ns` field (via `e8_batch_timed`) is the correct way to isolate engine-side time from transport time.

### 13.5 Diagnostics and spatial

- `doctor(1000003)` (prime): ~4 ms. `doctor_pair(42,21)` on a fresh process: ~0.2 ms.
- `e8_get_root` at indices 0 and 239: ~0.16 / ~0.15 ms; `nearest_root(1000000,7,3)`: ~0.27 ms.
- `spatial_place(3,2,5)`: ~0.21 ms; `spatial_distance_xy((0,0,0),(3,4,0))`: ~0.21 ms; `neighbors26(5)`: ~0.26 ms.
- **Blocking:** `doctor` / `doctor_pair` with operands of at least 64 bits did not return within 25–45 s (CPU idle); a blocked call stalls later calls on the same synchronous connection. Avoid large diagnostic inputs; if a call blocks, terminate and restart that MCP session.

### 13.6 Parallel sessions

- Four parallel MCP sessions, each making 20 exact multiplications of 1,024-bit operands: 80/80 results matched independent computation; wall time 0.64 s; median per call 4.04 ms; max 10.70 ms.
- A separate run used four independent clients, each making 100 checked multiplications of 1,024-bit operands: all 400 results matched; wall time 2.58 s; median per-client per-call 0.12 ms.

Both are single runs, not throughput guarantees. MCP transport adds ~20 ms per call on this host, so batch tools are strongly preferred over single calls for bulk work.

## 14. Limitations and important behavioral characteristics

Genuine restrictions, unusual-but-legitimate behavior, documented discrepancies, and unspecified areas — organized so users and agents can find the relevant response quickly.

### 14.1 Documented API restrictions

- E8 root index range is **0..239**; index 240 raises `tuple index out of range`.
- Division and modulo require a non-zero divisor (`b != 0`).
- `spatial_move` requires **all** of `dx`, `dy`, `dz` even when zero.
- `doctor_pair_coords` requires all six coordinates `x1, y1, z1, x2, y2, z2`.
- `e8_address` requires all three of `x, y, z`; the canonical help marks all three required for the bridge family (for `nearest_root`, the schema lists `z` optional — pass all three).
- `spatial_distance_xy` requires all six coordinates (z is accepted but ignored in the result).
- `symbolic_rewrite` accepts only `expr_type` ∈ {`tau`, `coord`, `valid`, `ternary`}.
- `random_n`: `bound > 0` gives values in `[0, bound)`; without a positive `bound` the response shape differs (see 9.13).

### 14.2 Confirmed observed behavior (unusual but legitimate — not defects)

- `cubint_floordiv(-7,3)` → `-2` (truncation toward zero, not Python floor) and `cubint_mod(-7,3)` → `-1` (sign follows the dividend) — both consistent with the canonical help's "sign follows dividend".
- JSON floats are accepted and truncated to integers (e.g. `cubint_add(1.5, 2)` → `3`; `cubint_validate(3.7)` → `true`).
- `vec_std_x1000` returns `int(sqrt(vec_variance_x1000))`, **not** std×1000 (canonical help + confirmed: `[1,2,3]` → 25).
- Witness `value` fields may print in ROUTE form (e.g. `1e1` for ten, `42e1` for 420) — parse accordingly.
- `random_n` without a positive `bound` returns a double-wrapped `values` object instead of a plain list.
- `spatial_check_support` returns `true` for any `y <= 0` (floor and below), `false` for `y > 0`; `spatial_place` snaps positive `y` to 0 but passes negative `y` through unchanged.
- `e8(-42)` projects identically to `e8(42)` (the tested negative value behaves like its absolute value).
- `cubcomplex_abs` returns the integer part of a non-integral magnitude (e.g. (1,1) → 1).
- `e8_get_root(-1)` wraps to root 239 (negative index wraps); index 240 raises an index error.
- Unusual diagnostic verdicts (`REJECTED`, `QUARANTINE`, `UNCERTAIN`) are normal outputs of the diagnostic system (see the interpretation note in 9.4).
- Unknown CLI flags are ignored: the server starts normally with exit code 0.
- The server is synchronous: one call at a time per connection.
- The canonical help lists extra vector tools (`vec_div`, `vec_mod`, `vec_argmin`, `vec_argmax`, `vec_cumprod`, `vec_where`, `vec_filter`) beyond the MCP surface; **none of them are exposed via MCP**.

### 14.3 Confirmed limitations

- **Large-operand integer arithmetic errors.** At 16,384-bit and 65,536-bit seed sizes, integer tools returned `-32603` "failed converting decimal input to integer" errors while parsing decimal-string operands. This is a size-related limitation of the published build's exposed interface, not evidence of a documented maximum input size.
- **`cubint_pow(3, 10000)` returns `0`.** Do not rely on large powers for exact results.
- **`cubint_pow(2, -1)` returns `0.5`.** Negative exponents are accepted and return decimals, contradicting the canonical help's `b>=0 enforced` statement (see 14.4).
- **`vec_dot` rejects negative element products.** `vec_dot([-1,2],[3,-4])` → `-32603` `Cannot decompose negative product: -3`. Use non-negative operands or a workaround for signed dot products.
- **Diagnosis tools can block on large operands.** `doctor` / `doctor_pair` with operands of at least 64 bits did not return within 25–45 s (CPU idle). A blocked call stalls later calls on the same synchronous connection.

### 14.4 Discrepancies between canonical help and observed build behavior (UNRESOLVED)

The following discrepancies between the canonical `help` reference and the actual behavior of the published build are documented neutrally; their causes are not established by this manual, and no result below has been altered to match the other source.

| Canonical help statement / witness | Observed behavior in this build |
|---|---|
| `doctor_pair(420,210)` → `PERFECT, match_count 14, weighted_score 285` | `QUARANTINE, match_count 4, weighted_score −155, penalty_multiplier 50` |
| `doctor(420)` self-pair verdict `WARN, match_count 8` | self-pair verdict `REJECTED, match_count 0` |
| `cubint_pow`: `b>=0 enforced` | `cubint_pow(2,-1)` → `0.5` (negative exponent accepted, decimal returned) |
| `cubint_pow` exactness guarantees | `cubint_pow(3,10000)` → `0` |
| `spatial_move` example `move(5,0,3,dx=2,dy=5)` | fails: `dz` is required |
| `doctor(420)` "tension 504" | actual fields: `tension 3`, `sigma_tension 504` (naming mismatch) |

Minor field-level notes (additive, not contradictory): `e8_root_dot` returns a structured `{"result": ..., "computed_on_the_fly": true}` rather than the bare number shown in the help's signature line; `doctor`/`doctor_group` results carry extra fields (`axis_doctor`, `axis_distance`, `canon`, `signatures`) beyond the help's summary.

### 14.5 Unspecified or experimental behavior

The following areas are **not** established by the public interface and must not be treated as guaranteed functionality:

- **`warp_dot`:** semantics beyond the `vec_dot` cross-check are not established; treat results as experimental.
- **`random_n`:** period/sequence properties beyond the tested arguments are not verified (period 2³¹−1 is per the canonical help).
- **`doctor` / `doctor_pair`:** the large-operand blocking threshold is unknown (≥64-bit operands blocked; exact threshold unspecified).
- **`nearest_root_extended`:** non-zero `extra` values are not covered by documented examples.
- **Not covered by documented examples:** out-of-range indices for `e8_root_dot` / `e8_distance_matrix` / `e8_triangle_geometry` (an index error is expected per the `e8_get_root` pattern); dimension mismatch for `e8_table_matmul`; empty vectors for `vec_min` / `vec_max`; length-1 input for `vec_diff`; non-integral norms for `vec_norm`; the fixing path of `cascade_fix` on inconsistent input; non-3-4-5 distances for `spatial_distance_xy`; signed operands for `vec_mul`.

## 15. Version, build identification, and license

- **Product version:** `cube_v5 5.0.0` (from `--version` and the MCP `initialize` `serverInfo`).
- **Protocol version:** `2024-11-05`.
- **Published executable:** `cube.exe` (Windows x64, 6,612,992 bytes).
- **Generated runtime table:** `cube_v5.bin` (272,000,016 bytes). The table is generated locally on first start and is not part of the distribution.
- **Canonical help reference:** 16,425 characters, returned by the `help` tool.
- **Distribution contents:** exactly `cube.exe`, `README.md` (product overview), and `CUBE_REFERENCE.md` (complete technical reference). `cube_v5.bin` is a runtime table the executable generates on first start (not distributed; never commit to Git).

### License

MIT License.

Copyright (c) 2026 Cube v5 authors

Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

## 16. Verification and Witness-Based Workflows

> **Status of this section.** Built from a live black-box run against the published `cube.exe`
> (87 tools, server `cube_v5 v5.0.0`). All JSON blocks below are verbatim results from a
> recorded transcript. Every call returned in time; no timeouts, no errors. Per-workflow status: the τ plane-closure workflow is **DEMONSTRATED** for the specific
> inputs called (16.3–16.6); the cubic-identity encoding is **NOT ESTABLISHED** (16.8); witness
> independence in the strong sense is **NOT ESTABLISHED** (16.9). Nothing here is a general
> mathematical proof, and no external arithmetic was used to judge any result.

### 16.1 Objective

The relation examined is the engine's own τ invariant, as returned verbatim by
`symbolic_invariants` (call 2):

```json
{
  "invariants": {
    "coord_symmetry": "((?x:?y:?z)[XY] = (?z:?x:?y)[XY])",
    "tau_invariant": "(τ(?S, ?P, ?D) = (((?S * ?S) - (4 * ?P)) - (?D * ?D)))",
    "e8_self_dot": "((?r dot ?r) = 8)"
  }
}
```

The intended closure condition, in Cube's own model, is that the τ expression closes to zero on a
plane's address values `(S, P, D)`: the public help documents `tension_verify` as
a "three-plane check S^2-4P==D_net^2" with a `planes[]` payload, and the verdict fields
(`verified`, `max_tau`, per-plane `tau`) carry the closure outcome.

**Cube is not an arithmetic calculator.** Per its own help (`[1] EXECUTION MODEL`): operations are
ADDRESS READS in a precomputed verified table; runtime performs no arithmetic. Verification here
is therefore address/witness-based: we audit witnesses (address forms, plane cells, channel
comparisons, group agreements) and read the engine's verdict fields. Nothing in this section is a
general mathematical proof, and no external arithmetic was used to judge any result.

### 16.2 Initial representation

Exact inputs supplied, and why they are appropriate per the public tool contracts (help sections
`[4]`, `[5]`, `[12]`):

| Input | Value | Role in the canonical witness case |
|---|---|---|
| `a` | `12` | first factor of the documented canonical pair |
| `b` | `35` | second factor of the documented canonical pair |
| product | `420` | `12·35`; the target value address (help: "MORE VERIFIED WITNESSES (420-family)") |
| `S` | `47` | plane sum value (`S=47` per help's canonical line) |
| `D` | `23` | plane difference value (`D=23` per help's canonical line) |
| `z` (negative control only) | `7` | optional third input accepted by `tension_verify{a,b,z?}` / `cascade_fix{a,b,z?}` |

The help's canonical line for this case (verbatim from `help`, call 1):

```
tension_verify(12,35) -> product 420, verified true, max_tau 0
                        (S=47,P=420,D=23: 47^2-4*420=529=23^2)
```

All arguments are small integers — the accepted domain of the public tools (`doctor` takes
`value`; `doctor_pair` takes `a,b`; `doctor_group` takes `values[]`; `cubint_witness` takes `a`;
`tension_verify` takes `a,b,z?`; `symbolic_rewrite` takes `expr_type` + `args`).

### 16.3 Witness construction (one subsection per route)

#### 16.3.1 Route W1 — Address witnesses (`cubint_witness`)

Tool: `cubint_witness`. Purpose: read the address representation of each operand of the relation
(per help `[3]`: `.witness() -> {'mantissa':42,'debt':0,'orientation':0,'value':'42'}`, where
`value` is a STRING and may be in route form, e.g. `1e1`).

Exact calls and ACTUAL results:

`cubint_witness` `{"a":12}`:
```json
{
  "mantissa": 12,
  "debt": 0,
  "orientation": 0,
  "value": "12"
}
```

`cubint_witness` `{"a":35}`:
```json
{
  "mantissa": 35,
  "debt": 0,
  "orientation": 0,
  "value": "35"
}
```

`cubint_witness` `{"a":420}`:
```json
{
  "mantissa": 42,
  "debt": 1,
  "orientation": 0,
  "value": "42e1"
}
```

`cubint_witness` `{"a":47}`:
```json
{
  "mantissa": 47,
  "debt": 0,
  "orientation": 0,
  "value": "47"
}
```

`cubint_witness` `{"a":23}`:
```json
{
  "mantissa": 23,
  "debt": 0,
  "orientation": 0,
  "value": "23"
}
```

Relevant witness information: `mantissa`/`debt`/`orientation` are the address components; `value`
is the engine's own string form. The important datum: **420 is represented as mantissa 42 with
debt 1 (`"42e1"`)**, exactly as the help's 420-family block documents. Note the representation is
not the plain digit string — this is the "route form" the help warns about (`[3]`: "str(CubInt)
may return ROUTE form"). These witnesses establish the address forms of the operands. They do
not by themselves establish any relation between the operands.

#### 16.3.2 Route W2 — Plane/structural witness (`tension_verify`)

Tool: `tension_verify`. Exact call: `{"a":12,"b":35}`. ACTUAL result, verbatim:

```json
{
  "product": 420,
  "verified": true,
  "max_tau": 0,
  "planes": [
    {
      "plane": "xy",
      "ok": true,
      "s": 47,
      "p": 420,
      "d": 23,
      "lhs": 529,
      "rhs": 529,
      "tau": 0
    },
    {
      "plane": "xz",
      "ok": true,
      "s": 12,
      "p": 0,
      "d": 12,
      "lhs": 144,
      "rhs": 144,
      "tau": 0
    },
    {
      "plane": "yz",
      "ok": true,
      "s": 35,
      "p": 0,
      "d": 35,
      "lhs": 1225,
      "rhs": 1225,
      "tau": 0
    }
  ],
  "cell": {
    "s_xy": 47,
    "s_xz": 12,
    "s_yz": 35,
    "d_xy_pos": 0,
    "d_xy_neg": 23,
    "d_xy_sig": 1,
    "d_xz_pos": 12,
    "d_xz_neg": 0,
    "d_xz_sig": 0,
    "d_yz_pos": 35,
    "d_yz_neg": 0,
    "d_yz_sig": 0,
    "p_xy": 420,
    "p_xz": 0,
    "p_yz": 0,
    "web_divs": [
      1,
      420,
      2,
      210,
      3,
      140,
      4,
      105,
      5,
      84,
      6,
      70,
      7,
      60,
      10,
      42
    ],
    "web_quots": [
      420,
      1,
      210,
      2,
      140,
      3,
      105,
      4,
      84,
      5,
      70,
      6,
      60,
      7,
      42,
      10
    ],
    "vol_v": 0,
    "vol_av": 15,
    "vol_s3": 420,
    "vol_c3": 0,
    "rotations": [
      0,
      0,
      0,
      0,
      0,
      0,
      0,
      0,
      0,
      0,
      0,
      0,
      0,
      0,
      0,
      0,
      0,
      0,
      0,
      0,
      0,
      0,
      0,
      0,
      0,
      0,
      0,
      0,
      0,
      0,
      0,
      0,
      0,
      0,
      0,
      0
    ],
    "neighbours": [
      0,
      0,
      0,
      0,
      0,
      0,
      0,
      0,
      0,
      0,
      0,
      0,
      0,
      0,
      0,
      0,
      0,
      0,
      0,
      0,
      0,
      0,
      0,
      0,
      0,
      0
    ],
    "inversions": [
      0,
      0,
      0,
      0,
      0,
      0,
      0,
      0
    ],
    "e8_vec8": [
      0,
      0,
      0,
      0,
      0,
      0,
      0,
      0
    ],
    "doctor": {
      "cc": 0,
      "ws": 0
    }
  }
}
```

Relevant witness information:
- `product` = 420 — the target value reported by the (precomputed) cell entry.
- `verified` = true, `max_tau` = 0 — the engine's own closure verdict for the three planes.
- Per-plane `s`, `p`, `d`, `lhs`, `rhs`, `tau`: for the xy plane the witness values are exactly the
  documented canonical `S=47, P=420, D=23`, with `lhs` = 529, `rhs` = 529, `tau` = 0. Per the
  public help the check is `S^2-4P==D_net^2`; the `lhs`/`rhs`/`tau` fields are where that check's
  outcome is read.
- `cell.vol_s3` = 420 — the cell's S3 volume field carries the product address.
- `cell.web_divs` / `cell.web_quots` — an ordered divisor/complement web of 420 (pairs
  `[1,420], [2,210], [3,140], [4,105], [5,84], [6,70], [7,60], [10,42]`).
- Black-box input–output pattern observed across all four `tension_verify` calls of this run
  (calls 3, 12, 13, 22): the xy plane reports `s` = input sum, `p` = input product, `d` = input
  absolute difference; without `z`, the xz/yz planes report `p`=0 with `s`/`d` equal to each input;
  with `z`, the xz/yz planes report values for each input combined with `z` in the same pattern.
  This is an observed behavioral pattern (not an internal implementation claim), and it matters:
  see 16.6 — the same pattern holds for the altered pair (12,34), so zero tension by itself is not
  a discriminator between canonical and non-canonical inputs.

#### 16.3.3 Route W3 — Value diagnostic (`doctor`)

Tool: `doctor`. Exact call: `{"value":420}`. ACTUAL result, verbatim:

```json
{
  "factors": [
    2,
    2,
    3,
    5,
    7
  ],
  "divisors": [
    1,
    2,
    3,
    4,
    5,
    6,
    7,
    10,
    12,
    14,
    15,
    20,
    21,
    28,
    30,
    35,
    42,
    60,
    70,
    84,
    105,
    140,
    210,
    420
  ],
  "divisor_count": 24,
  "is_perfect": false,
  "is_abundant": true,
  "is_deficient": false,
  "closed_count": 12,
  "tension": 3,
  "sigma_tension": 504,
  "route_quality": 190,
  "pair": {
    "channels": {
      "P_XY": "MISMATCH",
      "P_XZ": "MISMATCH",
      "P_YZ": "MISMATCH",
      "S_XY": "MISMATCH",
      "S_XZ": "MISMATCH",
      "S_YZ": "MISMATCH",
      "|D|_XY": "MISMATCH",
      "|D|_XZ": "MISMATCH",
      "|D|_YZ": "MISMATCH",
      "sgn_XY": "UNCERTAIN",
      "sgn_XZ": "UNCERTAIN",
      "sgn_YZ": "UNCERTAIN",
      "Volume": "MISMATCH",
      "Shared": "MISMATCH"
    },
    "match_count": 0,
    "uncertain_count": 3,
    "mismatch_count": 11,
    "net_score": -11,
    "weighted_score": -240,
    "penalty_multiplier": 0,
    "is_closed": false,
    "verdict": "REJECTED"
  },
  "axis_doctor": {
    "kind": "axis_doctor",
    "point": [
      6,
      7,
      10
    ],
    "closures": {
      "C1_hxy": true,
      "C2_hxz": true,
      "C3_hyz": true,
      "C4_axis": false,
      "C5_phase": true,
      "C6_dbody": true
    },
    "passed": [
      "C1_hxy",
      "C2_hxz",
      "C3_hyz",
      "C5_phase",
      "C6_dbody"
    ],
    "failed": [
      "C4_axis"
    ],
    "tension": 1,
    "on_axis": false,
    "D_body": 4,
    "stack": [
      [
        0,
        0,
        1
      ],
      [
        2,
        2,
        0
      ],
      [
        0,
        1,
        1
      ]
    ],
    "perturbations": [
      {
        "level": 0,
        "trit": [
          0,
          0,
          1
        ],
        "type": "face",
        "perturbation": [
          0,
          0,
          1
        ],
        "scale": 9
      },
      {
        "level": 1,
        "trit": [
          2,
          2,
          0
        ],
        "type": "edge",
        "perturbation": [
          1,
          1,
          -1
        ],
        "scale": 3
      },
      {
        "level": 2,
        "trit": [
          0,
          1,
          1
        ],
        "type": "edge",
        "perturbation": [
          -1,
          0,
          0
        ],
        "scale": 1
      }
    ],
    "signature": "FRACTURED:['C4_axis']"
  },
  "axis_distance": {
    "d_body": 4,
    "phase": 0,
    "s_lin": 23,
    "c_plan": 172,
    "volume": 420
  }
}
```

Relevant witness information:
- `factors`, `divisors`, `divisor_count` — the factorization of 420 as read from the table
  (24 divisors; note their order in the live result is not sorted: 20 precedes 21).
- `is_abundant` = true, `is_perfect` = false.
- `tension` = 3, `sigma_tension` = 504, `closed_count` = 12, `route_quality` = 190.
- `pair` — a 14-channel self-diagnosis: `match_count` 0, `verdict` `REJECTED`, `is_closed` false.
  **This is the live, ACTUAL value.** The canonical help's 420-family block claims `pair verdict
  WARN match_count 8` for `doctor(420)` — a known documented discrepancy, confirmed live. Report
  the ACTUAL `REJECTED`/0, not the canonical claim.
- `axis_doctor` — `point` [6,7,10]; closures C1/C2/C3/C5/C6 true, **C4_axis false** → signature
  `FRACTURED:['C4_axis']`, `on_axis` false, `tension` 1.
- `axis_distance` — `volume` 420, `s_lin` 23 (the same 23 that is the canonical plane D),
  `d_body` 4, `c_plan` 172.
- Discrepancy also documented in 14.4: the help's 420-family block says `doctor(420) -> ... tension
  504 ...`, but the live result carries `tension` = 3 and `sigma_tension` = 504 (a `tension` /
  `sigma_tension` naming mismatch). Confirmed live in this run. The live fields are the
  authoritative behavior. Preserve, do not "fix".

#### 16.3.4 Route W4 — Pair channel diagnostic (`doctor_pair`)

Tool: `doctor_pair`. Exact call: `{"a":12,"b":35}`. ACTUAL result, verbatim:

```json
{
  "channels": {
    "P_XY": "MISMATCH",
    "P_XZ": "MISMATCH",
    "P_YZ": "MISMATCH",
    "S_XY": "MISMATCH",
    "S_XZ": "MISMATCH",
    "S_YZ": "MISMATCH",
    "|D|_XY": "MISMATCH",
    "|D|_XZ": "MISMATCH",
    "|D|_YZ": "MISMATCH",
    "sgn_XY": "UNCERTAIN",
    "sgn_XZ": "MATCH",
    "sgn_YZ": "MATCH",
    "Volume": "MISMATCH",
    "Shared": "MISMATCH"
  },
  "match_count": 2,
  "uncertain_count": 1,
  "mismatch_count": 11,
  "net_score": -9,
  "weighted_score": -210,
  "penalty_multiplier": 25,
  "is_closed": false,
  "verdict": "REJECTED"
}
```

Relevant witness information: 14 channels each MATCH / UNCERTAIN / MISMATCH. The two matching
channels are `sgn_XZ` and `sgn_YZ` (sign channels); `sgn_XY` is UNCERTAIN. Per the public help the
verdict ladder is `PERFECT(14) OK(>=10) WARN(>=7) QUARANTINE(>=4) REJECTED(<4)` with
`is_closed` at `match_count>=7`. With `match_count` 2 the verdict is `REJECTED` and `is_closed`
false. (The documented `doctor_pair(420,210) -> PERFECT match_count 14` line in the help was NOT
called in this run; the ACTUAL result for the called pair (12,35) is the REJECTED/2 above.)

#### 16.3.5 Route W5 — Group agreement (`doctor_group`)

Tool: `doctor_group`. Exact call: `{"values":[12,35,420]}`. ACTUAL result,
verbatim:

```json
{
  "agreements": 7,
  "total_routes": 42,
  "stress": 83,
  "is_closed": false,
  "canon": [
    {
      "base": [
        2,
        2,
        3
      ],
      "perm": [
        0,
        1,
        2
      ],
      "phase": 0
    },
    {
      "base": [
        1,
        5,
        7
      ],
      "perm": [
        0,
        1,
        2
      ],
      "phase": 0
    },
    {
      "base": [
        6,
        7,
        10
      ],
      "perm": [
        0,
        1,
        2
      ],
      "phase": 0
    }
  ],
  "signatures": [
    "FRACTURED:['C4_axis']",
    "FRACTURED:['C4_axis']",
    "FRACTURED:['C4_axis']"
  ]
}
```

Relevant witness information: `agreements` 7 of `total_routes` 42, `stress` 83, `is_closed` false.
The three `canon` bases are [2,2,3] (product 12), [1,5,7] (product 35), [6,7,10] (product 420) —
the engine's coordinate decompositions of the three inputs, matching the complementary-divisor
point [6,7,10] that `doctor(420).axis_doctor` reports. Each canon element carries the same
`FRACTURED:['C4_axis']` signature that `doctor(420).axis_doctor.signature` emits (see 16.9).

### 16.4 Witness comparison

The witnesses are brought into the diagnostic procedure by the comparison tools themselves, each
supplied with fresh inputs (no tool's output was fed into another during this run):

1. `doctor_pair {"a":12,"b":35}` — compares the two operand addresses across the 14 channels.
   ACTUAL: `match_count` 2, `verdict` `REJECTED`, `is_closed` false (full JSON in 16.3.4).
2. `doctor_group {"values":[12,35,420]}` — compares the three value addresses across 42 routes.
   ACTUAL: `agreements` 7, `stress` 83, `is_closed` false, all three signatures
   `FRACTURED:['C4_axis']` (full JSON in 16.3.5).
3. `tension_verify {"a":12,"b":35}` — the plane check that compares each plane's `lhs` vs `rhs`.
   ACTUAL: all three planes `ok` true with `lhs`==`rhs` and `tau` 0; `verified` true,
   `max_tau` 0 (full JSON in 16.3.2).

Note that these three comparison operations use **different closure notions**: the plane check
closes (tau 0), while both channel-comparison diagnostics do not close (`REJECTED`,
`is_closed` false). This is not an inconsistency in the results — they read different structural
relations (plane identity values vs channel agreement between addresses) — but it must be stated
explicitly rather than papered over.

### 16.5 Closure verification

The actual closure-check operations and the returned fields that establish the observed results:

| Operation | Fields that establish the result | ACTUAL observed result |
|---|---|---|
| `tension_verify {a:12,b:35}` | `verified`, `max_tau`; per-plane `ok`, `lhs` vs `rhs`, `tau` | `verified` true, `max_tau` 0; xy 529==529 tau 0; xz 144==144 tau 0; yz 1225==1225 tau 0 |
| `doctor_pair {a:12,b:35}` | `match_count`, `verdict`, `is_closed` | `match_count` 2 (<7), `verdict` REJECTED, `is_closed` false |
| `doctor {value:420}` → `pair` | `pair.match_count`, `pair.verdict`, `pair.is_closed` | `match_count` 0, `verdict` REJECTED, `is_closed` false (KNOWN DISCREPANCY vs help's "WARN match_count 8") |
| `doctor {value:420}` → `axis_doctor` | `closures`, `failed`, `signature` | C4_axis false, `signature` `FRACTURED:['C4_axis']`, `on_axis` false |
| `doctor_group {values:[12,35,420]}` | `agreements`, `total_routes`, `stress`, `is_closed`, `signatures` | 7/42, stress 83, `is_closed` false, 3× `FRACTURED:['C4_axis']` |

So for the canonical case the DEMONSTRATED picture is: **plane-level tension closure holds
(zero tau on all three planes), while the channel-agreement diagnostics reject the pair and the
group** (`REJECTED`, `is_closed` false). Both halves are live actual results of this executable.

### 16.6 Negative control

All outcomes below are ACTUAL; no expected rejection was assumed. (Full verbatim payloads for
calls 12 and 13 are in the recorded transcript; the discriminating fields are quoted here.)

**Call 12 — `tension_verify {"a":12,"b":34}` (altered pair).** NOT rejected. ACTUAL:
`"product": 408, "verified": true, "max_tau": 0`; xy plane `"s": 46, "p": 408, "d": 22, "lhs": 484,
"rhs": 484, "tau": 0`; xz `"s": 12, "p": 0, "d": 12, "lhs": 144, "rhs": 144`; yz `"s": 34, "p": 0,
"d": 34, "lhs": 1156, "rhs": 1156`.
Meaning: `tension_verify` does not distinguish the canonical pair from an altered one. Per the
observed input–output pattern (16.3.2), the plane values are the table entries keyed by the input
sum, product, and absolute difference; the documented check `S^2-4P==D_net^2` reads zero tau on
those address triples for the pairs observed in this run. (The algebraic identity
(a+b)²−4ab=(a−b)² holds for every integer pair, so the pattern would be expected to continue —
but the engine reads a precomputed table, and entries were only read for the inputs actually
called.) A zero-tension `verified` result is therefore a structural self-consistency outcome of
this tool, **not** an independent confirmation that the inputs are "the" canonical witnesses. Do
not cite call 12's `verified: true` as evidence in favor of the (12,34) pair.

**Call 13 — `tension_verify {"a":12,"b":35,"z":7}` (extra input).** Not rejected; `z` is absorbed.
ACTUAL: `"product": 420, "verified": true, "max_tau": 0`; xz plane `"s": 19, "p": 84, "d": 5,
"lhs": 25, "rhs": 25, "tau": 0`; yz plane `"s": 42, "p": 245, "d": 28, "lhs": 784, "rhs": 784,
"tau": 0`; cell fields change: `"vol_v": 2940, "vol_av": 18, "vol_s3": 749, "vol_c3": 2940`.
Meaning: with `z`, the xz/yz planes are the table entries for (12,7) and (35,7) respectively in the same
pattern, and all three planes still close with tau 0. The `product` field still reports 420
(12·35), unaffected by `z`.

**Call 14 — `cascade_fix {"a":12,"b":35,"z":7}`.** ACTUAL:
```json
{
  "fixed": false,
  "reason": "already_consistent",
  "product": 420,
  "tau": 0
}
```
Meaning: no correction was applied; the engine reports the cell as already consistent, with
`product` 420 and `tau` 0. Note again: `z` does not appear in the reported product.

**Call 15 — `symbolic_rewrite {"expr_type":"tau","args":{"S":47,"P":420,"D":23}}`.** ACTUAL:
```json
{
  "normal_form": "τ(47, 420, 23)",
  "trace": []
}
```

**Call 16 — `symbolic_rewrite {"expr_type":"tau","args":{"S":47,"P":420,"D":1}}` (altered D).**
ACTUAL:
```json
{
  "normal_form": "τ(47, 420, 1)",
  "trace": []
}
```
Meaning of calls 15–16 together: `symbolic_rewrite` normalizes a τ expression into its printed
normal form and returns an empty `trace`. It does **not** evaluate τ, does **not** check closure,
and does **not** reject a non-closing assignment (D=1 is treated exactly the same as D=23). A
`normal_form` result with empty `trace` is therefore **not** a verification of
`S²−4P=D²`; it is an address/notation witness only.

**Call 17 — `cascade_fix {"a":12,"b":35}`.** ACTUAL: identical to call 14:
```json
{
  "fixed": false,
  "reason": "already_consistent",
  "product": 420,
  "tau": 0
}
```

**Changed cubic cases** (calls 20–23, see 16.8 for full results): `doctor_pair_coords` on two
IDENTICAL points (1,1,1) vs (1,1,1) and (1,1,2) vs (1,1,2) both returned `PERFECT` /
`match_count` 14 / `is_closed` true. These are self-comparisons of identical coordinate addresses;
the perfect score is degenerate (a point trivially matches itself on all 14 channels) and
establishes nothing about any cubic identity. `tension_verify {a:1,b:1}` returned
`verified` true, `max_tau` 0 (xy plane s=2, p=1, d=0, lhs=rhs=0) — again a tautological pair
outcome. `doctor_group [1,1,2]` returned `agreements` 24, `total_routes` 42, `stress` 42,
`is_closed` TRUE with signatures `CLOSED`, `CLOSED`, `FRACTURED:['C4_axis']` — note the group can
report `is_closed` true even while one canon signature is `FRACTURED`, so `is_closed` is a
group-level aggregate, not the AND of the per-canon signatures.

### 16.7 Levels of verification (be precise)

| Level | Definition (in this workflow's terms) | Which demonstrated result reaches it |
|---|---|---|
| Address witness | The engine's own address representation of a value was read (`cubint_witness`) | W1 for 12/35/420/47/23 — DEMONSTRATED as address facts (420 = 42e1, debt 1) |
| Structural closure | The engine's plane check reports closed for a specific input: `verified` true, `max_tau` 0, all planes `lhs`==`rhs` | `tension_verify(12,35)` — DEMONSTRATED for this input. NOT a universal claim: call 12 shows the same outcome for the altered pair (12,34), i.e. the check closes on values derived from the inputs themselves |
| Diagnostic agreement | A channel-comparison tool reports its verdict for a specific pair/value/group: `doctor_pair`, `doctor.pair`, `doctor_group` | `doctor_pair(12,35)` REJECTED/2, `doctor(420).pair` REJECTED/0, `doctor_group([12,35,420])` is_closed false — DEMONSTRATED as actual diagnostic verdicts (and they CONTRADICT the canonical help's claimed WARN/8 and the canonical witness narrative) |
| Multiple witness agreement | Two or more tools report mutually consistent data about the same addresses | The eight divisor pairs of 420 appear both within `doctor(420).divisors` (part of the 24-divisor list) and as ordered pairs in `tension_verify.cell.web_divs`/`web_quots`; the value 420 appears in `cubint_witness` (42e1), `tension_verify.product`, `doctor(420).axis_distance.volume`, `doctor_group` canon [6,7,10]. Consistency is DEMONSTRATED — but all routes read the same precomputed table (help `[1]`), so agreement is corroboration of one table, not independent confirmation |
| General mathematical proof | A derivation that the relation holds for all admissible values, independent of the table | NOT REACHED by any result in this section. Zero tau on the planes is not a theorem; matching outputs across tools are not independent witnesses (16.9); a `normal_form` from `symbolic_rewrite` is not an evaluation |

Hard rules for using this section:
- A successful `tension_verify` proves nothing beyond its input; it is a self-check on derived plane values.
- A zero-tension result is not a universal theorem.
- Matching outputs from different tools are not automatically independent witnesses; here they share underlying representations (16.9).
- `is_closed`/`verdict` from the doctor family and `verified`/`max_tau` from the tension family are
  different closure notions; never cite one to settle a question the other answers.

### 16.8 Cubic identity experiment

Identity under exploration: `x³+y³+z³−3xyz = (x+y+z)(x²+y²+z²−xy−yz−zx)`.

**Status: NOT ESTABLISHED. No defensible public-interface encoding was found.**

ACTUAL results (verbatim except where noted as abbreviated):

Call 18 — `symbolic_rewrite {"expr_type":"ternary","args":{"x":1,"y":1,"z":1}}`:
```json
{
  "normal_form": "<>",
  "trace": []
}
```
The documented `expr_type` domain is `tau|coord|valid|ternary` (help `[12]`); for `ternary` with
these arguments the engine returned an **empty** normal form `"<>"` with empty trace. No cubic
structure was produced.

Call 19 — `symbolic_rewrite {"expr_type":"coord","args":{"x":1,"y":1,"z":1}}`:
```json
{
  "normal_form": "(1:1:1)[XY]",
  "trace": []
}
```
A coordinate address in the engine's `(x:y:z)[plane]` notation — a spatial address, not a
polynomial expression.

Call 20 — `doctor_pair_coords {"x1":1,"y1":1,"z1":1,"x2":1,"y2":1,"z2":1}`:
```json
{
  "channels": {
    "P_XY": "MATCH",
    "P_XZ": "MATCH",
    "P_YZ": "MATCH",
    "S_XY": "MATCH",
    "S_XZ": "MATCH",
    "S_YZ": "MATCH",
    "|D|_XY": "MATCH",
    "|D|_XZ": "MATCH",
    "|D|_YZ": "MATCH",
    "sgn_XY": "MATCH",
    "sgn_XZ": "MATCH",
    "sgn_YZ": "MATCH",
    "Volume": "MATCH",
    "Shared": "MATCH"
  },
  "match_count": 14,
  "uncertain_count": 0,
  "mismatch_count": 0,
  "net_score": 14,
  "weighted_score": 285,
  "penalty_multiplier": 100,
  "is_closed": true,
  "verdict": "PERFECT"
}
```
Call 21 — `doctor_pair_coords {"x1":1,"y1":1,"z1":2,"x2":1,"y2":1,"z2":2}`:
```json
{
  "channels": {
    "P_XY": "MATCH",
    "P_XZ": "MATCH",
    "P_YZ": "MATCH",
    "S_XY": "MATCH",
    "S_XZ": "MATCH",
    "S_YZ": "MATCH",
    "|D|_XY": "MATCH",
    "|D|_XZ": "MATCH",
    "|D|_YZ": "MATCH",
    "sgn_XY": "MATCH",
    "sgn_XZ": "MATCH",
    "sgn_YZ": "MATCH",
    "Volume": "MATCH",
    "Shared": "MATCH"
  },
  "match_count": 14,
  "uncertain_count": 0,
  "mismatch_count": 0,
  "net_score": 14,
  "weighted_score": 285,
  "penalty_multiplier": 100,
  "is_closed": true,
  "verdict": "PERFECT"
}
```
Both calls compare a point to ITSELF. The `PERFECT` 14/14 outcome is degenerate (identical
addresses always match) and contains no information about the cubic identity.

Call 22 — `tension_verify {"a":1,"b":1}` (top-level fields; the `cell` dump is omitted):
```json
{
  "product": 1,
  "verified": true,
  "max_tau": 0,
  "planes": [
    { "plane": "xy", "ok": true, "s": 2, "p": 1, "d": 0, "lhs": 0, "rhs": 0, "tau": 0 },
    { "plane": "xz", "ok": true, "s": 1, "p": 0, "d": 1, "lhs": 1, "rhs": 1, "tau": 0 },
    { "plane": "yz", "ok": true, "s": 1, "p": 0, "d": 1, "lhs": 1, "rhs": 1, "tau": 0 }
  ]
}
```
(Abbreviated here only in the `cell` dump, which is all-zero in the same shape as 16.3.2; the
transcript has the full payload.) This is the quadratic pair check on (1,1) — unrelated to the
cubic identity.

Call 23 — `doctor_group {"values":[1,1,2]}`:
```json
{
  "agreements": 24,
  "total_routes": 42,
  "stress": 42,
  "is_closed": true,
  "canon": [
    { "base": [1, 1, 1], "perm": [0, 1, 2], "phase": 0 },
    { "base": [1, 1, 1], "perm": [0, 1, 2], "phase": 0 },
    { "base": [1, 1, 2], "perm": [0, 1, 2], "phase": 0 }
  ],
  "signatures": [
    "CLOSED",
    "CLOSED",
    "FRACTURED:['C4_axis']"
  ]
}
```
(The `canon` objects are shown compactly; the recorded transcript has the expanded form.)

Call 24 — `symbolic_rewrite {"expr_type":"tau","args":{"S":3,"P":1,"D":2}}`:
```json
{
  "normal_form": "τ(3, 1, 2)",
  "trace": []
}
```
The engine normalizes `τ(3,1,2)` and stops. It does not evaluate it, does not report whether it
closes, and provides no field that could carry the cubic identity's truth value.

**Verdict on encoding:** None of the tools exercised in this run represents a cubic polynomial expression or
compares two sides of a polynomial identity, and the public help documents no tool whose purpose is
polynomial-identity comparison. The tools that came closest — `symbolic_rewrite`
(normalizes only; `ternary` yields an empty form), `doctor_pair_coords` (pointwise 14-channel
comparison of two coordinate addresses; only meaningful for distinct points, and then it compares
addresses, not expressions), `tension_verify`/`cascade_fix` (pair-based quadratic plane check on
derived values), and `doctor_group` (route agreement over value decompositions) — none produced a
field whose value depends on the cubic identity being true. The E8 family's genuine closure
concept (`e8_triangle_geometry`: `closure_ok` true iff `r_i+r_j+r_k=0`) concerns E8 root triples,
a different mathematical statement; it was not part of this experiment's call list. Encoding the
cubic identity as, e.g., "S=3, P=1, D=2 from (1,1,1)" would be an invention: no returned field
references x³, y³, z³, xyz, or either side of the identity. **Defensible encoding: none exists in
the public interface as observed.**

### 16.9 Witness independence findings

Based on the ACTUAL returned structures:

- **The routes are distinct projections, not independent derivations.**
  - W1 (`cubint_witness`) reads the address form (mantissa/debt/orientation/value) of each value.
  - W2 (`tension_verify`) reads the plane cell built from the pair (per-plane s/p/d/lhs/rhs/tau,
    divisor web, volumes).
  - W3 (`doctor`) reads the factorization, divisor list, pair self-diagnosis, axis doctor, and
    axis distance of one value.
  - W4 (`doctor_pair`) reads the 14-channel comparison of two values.
  - W5 (`doctor_group`) reads route agreements over the value decompositions.
- **Shared underlying data was observed, so strong independence is NOT supported:**
  - The divisor set of 420 appears in `doctor(420).divisors` and as the same pairs in
    `tension_verify.cell.web_divs`/`web_quots`.
  - `doctor(420).axis_doctor.point` is [6,7,10], the same triple that `doctor_group` uses as the
    third `canon` base (product 420).
  - `doctor_group`'s per-canon `signatures` are exactly the `axis_doctor.signature` string
    `FRACTURED:['C4_axis']` that `doctor(420).axis_doctor` emits — the group result reuses the
    same axis-diagnosis signature.
  - W4's channel vocabulary (P_*, S_*, |D|_*, sgn_*, Volume, Shared) is the same schema as
    W3's `pair.channels`.
- **No data flow between tools was observed:** every call in this run used fresh integer inputs;
  no tool's output was fed to another. One tool does not merely "reuse another's output" at the
  call level — but they demonstrably read overlapping representations of the same precomputed
  table (help `[1]`: "Operations are ADDRESS READS in a precomputed verified table").
- **Consistency of diagnostic evidence:** within each family the evidence is consistent (all
  tension calls: max_tau 0; all doctor-family closure fields: REJECTED / is_closed false for the
  canonical case). Across families the closure notions differ (see 16.4/16.5), which is a
  difference of question, not a contradiction.

Conclusion: label these routes **"multiple witness agreement over a shared table"** — do NOT label
them independent witnesses in the strong sense.

### 16.10 Final interpretation & scope

What was DEMONSTRATED (live, against this build):
1. **Address facts** (Level: address witness): 420 ↔ mantissa 42, debt 1, value `"42e1"`; 12/35/47/23
   are debt-0 addresses. Matches the help's 420-family witness line.
2. **Plane structural closure for (12,35)** (Level: structural closure): `verified` true,
   `max_tau` 0, xy plane 529==529, xz 144==144, yz 1225==1225, all `tau` 0; canonical values
   S=47, P=420, D=23 appear in the xy plane exactly as documented.
3. **Diagnostic verdicts for the canonical case** (Level: diagnostic agreement):
   `doctor_pair(12,35)` → REJECTED, match_count 2, is_closed false; `doctor(420).pair` → REJECTED,
   match_count 0, is_closed false (confirms the KNOWN discrepancy vs the help's "WARN match_count
   8"); `doctor(420).axis_doctor` → `FRACTURED:['C4_axis']`, on_axis false;
   `doctor_group([12,35,420])` → 7/42 agreements, stress 83, is_closed false.
4. **Discrepancy confirmed live** (also documented in 14.4): the help's 420-family block says
   `doctor(420) -> ... tension 504`, while the live result is `tension` 3 with `sigma_tension`
   504. The live fields stand.
5. **Behavioral boundaries** (Level: demonstrated behavior of the tools): `tension_verify` does not
   reject altered pairs (12,34 → verified true, max_tau 0); `z` is absorbed into the xz/yz planes;
   `cascade_fix` reports `already_consistent` in both z-cases; `symbolic_rewrite` normalizes τ
   expressions without judging closure (D=1 is accepted exactly like D=23, trace empty in both).

What was NOT established:
- No general mathematical proof of `S²−4P = D²` for arbitrary values (the engine's check is a
  table read / self-consistency on derived values for specific inputs).
- The cubic identity `x³+y³+z³−3xyz = (x+y+z)(x²+y²+z²−xy−yz−zx)`: **NOT ESTABLISHED** through the
  public interface; no defensible encoding exists (16.8). The `PERFECT`/`is_closed` results in
  16.8 are degenerate self-comparisons or unrelated quadratic checks.
- Witness independence in the strong sense (16.9).

Scope statement: everything demonstrated here is specific to the inputs actually called, on this
build, and is a set of address/witness facts plus the engine's own diagnostic verdicts. It does
not license any broader mathematical claim.

### 16.11 Reproducibility

Self-contained recipe. Every result in this section was produced by running the call list below
against the published executable; no placeholders.

**1. Environment.** Windows x64. The published `cube.exe` in a writable folder, plus any
MCP JSON-RPC-over-stdio client (a small script, or the MCP client configured per section 6.4).
On first start the executable generates its runtime table `cube_v5.bin` next to itself
(a file on the order of a few hundred MB); it is not part of the distribution and must not be
modified by the run.

**2. Integrity check (recommended).** Record the executable's SHA-256 before and after the run
and confirm it is unchanged:

```powershell
Get-FileHash -Algorithm SHA256 <path-to-cube.exe>
```

**3. Driver.** A driver script that launches `cube.exe` with its working directory set to the
folder containing `cube.exe` (required so the runtime table is found next to it), performs the
MCP handshake (initialize → notifications/initialized → tools/list), executes the call list
one call at a time (the server is synchronous: wait for each response before sending the next
request), and writes one JSON record per call (request arguments + raw response).

**4. Exact call list** (in this order):

```json
[
  {"name":"help","arguments":{}},
  {"name":"symbolic_invariants","arguments":{}},
  {"name":"tension_verify","arguments":{"a":12,"b":35}},
  {"name":"cubint_witness","arguments":{"a":12}},
  {"name":"cubint_witness","arguments":{"a":35}},
  {"name":"cubint_witness","arguments":{"a":420}},
  {"name":"cubint_witness","arguments":{"a":47}},
  {"name":"cubint_witness","arguments":{"a":23}},
  {"name":"doctor","arguments":{"value":420}},
  {"name":"doctor_pair","arguments":{"a":12,"b":35}},
  {"name":"doctor_group","arguments":{"values":[12,35,420]}},
  {"name":"tension_verify","arguments":{"a":12,"b":34}},
  {"name":"tension_verify","arguments":{"a":12,"b":35,"z":7}},
  {"name":"cascade_fix","arguments":{"a":12,"b":35,"z":7}},
  {"name":"symbolic_rewrite","arguments":{"expr_type":"tau","args":{"S":47,"P":420,"D":23}}},
  {"name":"symbolic_rewrite","arguments":{"expr_type":"tau","args":{"S":47,"P":420,"D":1}}},
  {"name":"cascade_fix","arguments":{"a":12,"b":35}},
  {"name":"symbolic_rewrite","arguments":{"expr_type":"ternary","args":{"x":1,"y":1,"z":1}}},
  {"name":"symbolic_rewrite","arguments":{"expr_type":"coord","args":{"x":1,"y":1,"z":1}}},
  {"name":"doctor_pair_coords","arguments":{"x1":1,"y1":1,"z1":1,"x2":1,"y2":1,"z2":1}},
  {"name":"doctor_pair_coords","arguments":{"x1":1,"y1":1,"z1":2,"x2":1,"y2":1,"z2":2}},
  {"name":"tension_verify","arguments":{"a":1,"b":1}},
  {"name":"doctor_group","arguments":{"values":[1,1,2]}},
  {"name":"symbolic_rewrite","arguments":{"expr_type":"tau","args":{"S":3,"P":1,"D":2}}}
]
```

**5. After the run** you have a per-call JSON transcript (one record per call) and the server's
tools list (87 tools). All calls in this run returned `ok` (no timeouts, no JSON-RPC errors, no
`isError` results). The stderr log contains only startup lines, including the informational
`No cube.sha256 found; skipping hash verification` — the executable's own optional self-check,
not an error.

---

## 17. Elementary Worked Examples: Address Witnesses and Verification

This section is a practical, step-by-step introduction to Cube's evidence model, written for a reader who has not yet worked through section 16. It examines the simplest relation the public interface can represent — the relation among 3, 5, and 8 — and shows, field by field, what each tool returns, what that return establishes, and what it does not establish. Every result below is an actual observed response from a live black-box run against the published `cube.exe` (34 calls; no timeouts, no unexpected errors).

### 17.1 Objective and conceptual introduction

The section demonstrates one principle in action: **Cube returns addresses and diagnostics from a precomputed table; it does not compute.** The execution model (README section 2, reference header) has already been stated — every tool call resolves to an address in a precomputed verified table, and the returned value is a pre-existing entry, not a runtime-computed result. This section shows what that model looks like on the simplest possible inputs.

A small vocabulary is used throughout. The same vocabulary should be used when reporting Cube's results:

| Term | Meaning |
|---|---|
| **Address witness** | A representation a tool returns for an address: the `{mantissa, debt, orientation, value}` object of `cubint_witness`, or a plain string result such as `"8"` |
| **Structural closure** | A tool's reported closure outcome for a specific input (`verified: true`, `max_tau: 0`, `is_closed: true`, verdict `CLOSED`/`PERFECT`, …). It is a property of the table entries for that input — not a proof of any external mathematical statement |
| **Diagnostic agreement** | Agreement or disagreement reported by a comparison tool (`doctor_pair`, `doctor_group`) for specific inputs |
| **Multiple witness agreement** | Consistent evidence from different tool views of the shared precomputed structure. The views are not independent of each other unless independence is explicitly established |
| **Independent verification** | Established only when it has been shown that the sources of evidence do not share underlying data. This has not been established for any pair of Cube tools (see 16.9) |
| **General mathematical proof** | Not claimed by any result in this document |

### 17.2 The elementary relationship under examination

**Primary case: the relation 3 + 5 = 8**, posed as an address question: *does the address Cube assigns to the result of `cubint_add(3,5)` coincide with the address Cube assigns to the literal 8?*

**Secondary case: 5² = 25**, posed as: *do `cubint_mul(5,5)` and `cubint_pow(5,2)` land on the same address, and does that address coincide with the witness of the literal 25?*

These cases were chosen because they are the simplest in which the interface can distinguish a result's address from a literal's address, and because each can be perturbed (`3+6`, `5³`) to serve as a negative control. They were chosen for experimental tractability, not because the interface "knows" them.

### 17.3 Address representation

All calls below are `tools/call` requests; every response shown is the verbatim `result.content[0].text` of that call.

**Witness basics.** The witness tool returns the engine's internal representation `(mantissa, debt, orientation)` (ROUTE form is defined in section 11):

```text
tools/call {"name":"cubint_witness","arguments":{"a":3}}
→ {"mantissa":3,"debt":0,"orientation":0,"value":"3"}
tools/call {"name":"cubint_witness","arguments":{"a":5}}
→ {"mantissa":5,"debt":0,"orientation":0,"value":"5"}
tools/call {"name":"cubint_witness","arguments":{"a":8}}
→ {"mantissa":8,"debt":0,"orientation":0,"value":"8"}
tools/call {"name":"cubint_witness","arguments":{"a":-8}}
→ {"mantissa":8,"debt":0,"orientation":2,"value":"-8"}
```

The three small positive integers are **canonical zero-debt addresses**. The sign is carried by `orientation` (2 = negative); the mantissa stays non-negative, exactly as the 9.1 contract states.

**Well-formedness.** `cubint_validate` accepts well-formed integer input and rejects malformed input with a JSON-RPC error rather than a `false` return:

```text
tools/call {"name":"cubint_validate","arguments":{"a":3}}
→ true
tools/call {"name":"cubint_validate","arguments":{"a":"abc"}}
→ JSON-RPC error -32603 "invalid literal for int() with base 10: 'abc'"
```

**Address view of 3+5=8.** The integer tools return plain strings — the result's address:

```text
tools/call {"name":"cubint_add","arguments":{"a":3,"b":5}}
→ "8"
tools/call {"name":"cubint_add","arguments":{"a":3,"b":6}}
→ "9"
```

The second call is a negative control: changing one input changes the result's address. The table distinguishes addresses; nothing about the pair (3,5) is singled out.

**Address view of 5²=25.** Two different operations land on the same address, and the literal witness lands on it too:

```text
tools/call {"name":"cubint_mul","arguments":{"a":5,"b":5}}
→ "25"
tools/call {"name":"cubint_pow","arguments":{"a":5,"b":2}}
→ "25"
tools/call {"name":"cubint_pow","arguments":{"a":5,"b":3}}
→ "125"
tools/call {"name":"cubint_witness","arguments":{"a":25}}
→ {"mantissa":25,"debt":0,"orientation":0,"value":"25"}
```

### 17.4 Witness retrieval: what it establishes

The central observation of this section is the pair:

- `cubint_add(3,5)` returns the address `"8"`.
- `cubint_witness(8)` — called with the literal 8 and no knowledge of how the 8 was obtained — returns the canonical address `{"mantissa":8,"debt":0,"orientation":0,"value":"8"}`.

The sum's result address is **indistinguishable** from the literal's witness. This is an *address fact* of the engine: within the precomputed table, the addition's output entry is the same entry the literal 8 resolves to. The same holds for 25 — `mul(5,5)` and `pow(5,2)` agree with `witness(25)`.

This is a **multiple witness agreement over one precomputed structure**: the arithmetic tools and the witness tool read the same table. It establishes that the table is internally consistent for these inputs (a result address equals the literal address of the same numeral). It does **not** establish an independent verification of the external statement "3 + 5 = 8", nor any property of unobserved table entries. Reading a result's address and comparing it with the literal's witness is precisely the auditing practice the engine's own reference describes — audit addresses (witnesses), not values (10.8, 16.1).

### 17.5 Diagnostic comparison

The `doctor` family provides structural diagnostics. For each of 3, 5, 8:

```text
tools/call {"name":"doctor","arguments":{"value":3}}
→ factors [3], divisors [1,3], divisor_count 2, tension 2, sigma_tension -2, route_quality 35;
  self-pair: REJECTED, match_count 2, uncertain_count 2, mismatch_count 10,
  net_score -8, weighted_score -185, penalty_multiplier 25, is_closed false;
  axis_doctor: point [1,1,3], failed ["C4_axis"], on_axis false, D_body 2,
  signature "FRACTURED:['C4_axis']";
  axis_distance {d_body:2, phase:0, s_lin:5, c_plan:7, volume:3}

tools/call {"name":"doctor","arguments":{"value":5}}
→ factors [5], divisors [1,5], divisor_count 2, tension 2, sigma_tension -4, route_quality 35;
  self-pair: REJECTED, match_count 2, uncertain_count 2, mismatch_count 10,
  net_score -8, weighted_score -185, penalty_multiplier 25, is_closed false;
  axis_doctor: point [1,1,5], failed ["C4_axis"], on_axis false, D_body 4,
  signature "FRACTURED:['C4_axis']";
  axis_distance {d_body:4, phase:0, s_lin:7, c_plan:11, volume:5}

tools/call {"name":"doctor","arguments":{"value":8}}
→ factors [2,2,2], divisors [1,2,4,8], divisor_count 4, tension 0, sigma_tension -1, route_quality 60;
  self-pair: QUARANTINE, match_count 6, uncertain_count 0, mismatch_count 8,
  net_score -2, weighted_score -75, penalty_multiplier 50, is_closed false;
  axis_doctor: point [2,2,2], all six closures pass, failed [], on_axis true,
  D_body 0, signature "CLOSED";
  axis_distance {d_body:0, phase:0, s_lin:6, c_plan:12, volume:8}
```

(Field meanings: section 9.4.) Two observations:

1. The 14-channel **self-pair** comparison does not single out 8. The values 3 and 5 share *identical* self-pair scores (REJECTED, 2/−185/25), while 8's self-pair is QUARANTINE. The self-pair metric is value-dependent — a property of each value's own address — not a measure of how 3, 5, and 8 relate to one another.
2. The **axis doctor** does single out 8: it is the only value of the three whose axis closures all pass (`on_axis: true`, `D_body: 0`, signature `CLOSED`). That is a structural property of the address of 8 in the table — not a property of the sum 3+5.

Note also the `point` fields: for values with fewer than three prime factors the factor list is padded with **1** to three components (3 → `[1,1,3]`, 5 → `[1,1,5]`); 9.4 documents only the 42 → `[2,3,7]` case. See the discrepancy note in 17.8.

Pair and group comparison:

```text
tools/call {"name":"doctor_pair","arguments":{"a":3,"b":5}}
→ WARN, match_count 7, uncertain_count 0, mismatch_count 7, net_score 0,
  weighted_score -35, penalty_multiplier 75, is_closed true
tools/call {"name":"doctor_pair","arguments":{"a":3,"b":8}}
→ QUARANTINE, match_count 4, uncertain_count 2, mismatch_count 8, net_score -4,
  weighted_score -85, penalty_multiplier 50, is_closed false
tools/call {"name":"doctor_pair","arguments":{"a":8,"b":5}}
→ REJECTED, match_count 2, uncertain_count 2, mismatch_count 10, net_score -8,
  weighted_score -185, penalty_multiplier 25, is_closed false
tools/call {"name":"doctor_pair","arguments":{"a":3}}
→ JSON-RPC error -32603 "'b'"   (missing required argument, named — as documented)
```

The verdicts are **input-sensitive**: (3,5) is the best-scoring of the three pairs, and the pairs that would encode "the sum with its partner" — (3,8) and (8,5) — score *worse* (QUARANTINE, then REJECTED). The relation "3+5=8" receives no preferential verdict from the comparison machinery.

The same comparison can be driven on explicit coordinates. Two calls:

```text
tools/call {"name":"doctor_pair_coords","arguments":{"x1":1,"y1":2,"z1":0,"x2":3,"y2":4,"z2":0}}
→ WARN, match_count 8, is_closed true   (reproduces the documented 9.4 example exactly)
tools/call {"name":"doctor_pair_coords","arguments":{"x1":1,"y1":1,"z1":3,"x2":1,"y2":1,"z2":5}}
→ WARN, match_count 7, is_closed true
```

The second call feeds the `point` arrays returned by `doctor(3)` and `doctor(5)` — `[1,1,3]` and `[1,1,5]` — and produces the **identical 14-channel score pattern** as `doctor_pair(3,5)` (net 0, weighted −35, penalty 75). The coordinate interface and the value interface agree, again over the shared table.

Group-level view:

```text
tools/call {"name":"doctor_group","arguments":{"values":[3,5,8]}}
→ agreements 13, total_routes 42, stress 69, is_closed false;
  canon bases [1,1,3], [1,1,5], [2,2,2];
  signatures ["FRACTURED:['C4_axis']", "FRACTURED:['C4_axis']", "CLOSED"]
tools/call {"name":"doctor_group","arguments":{"values":[3,3,3]}}
→ agreements 42, total_routes 42, stress 0, is_closed true;
  three identical canon entries [1,1,3]; signatures all "FRACTURED:['C4_axis']"
```

The control group `[3,3,3]` is stress-0 and closed while `[3,5,8]` is not: group closure tracks uniformity within the group, not arithmetic relations among the members. Note also that the `is_closed` of `[3,3,3]` is `true` although all three per-member signatures are `FRACTURED` — `is_closed` is a group-level aggregate, not the AND of the per-member signatures (also observed in 16.6).

### 17.6 Negative controls

The most important experiments in this section change the input and observe what changes.

**The τ closure is universal.** `tension_verify` checks the engine's τ identity on the plane entries keyed by the input's sum, product, and absolute difference (9.12). Three different pairs:

```text
tools/call {"name":"tension_verify","arguments":{"a":3,"b":5}}
→ product 15, verified true, max_tau 0;
  planes: xy s 8, p 15, d 2, lhs 4 = rhs 4, tau 0;
          xz s 3, p 0, d 3, lhs 9 = rhs 9, tau 0;
          yz s 5, p 0, d 5, lhs 25 = rhs 25, tau 0
tools/call {"name":"tension_verify","arguments":{"a":3,"b":6}}
→ product 18, verified true, max_tau 0;
  planes: xy s 9, p 18, d 3, lhs 9 = rhs 9, tau 0;
          xz s 3, p 0, d 3, lhs 9 = rhs 9, tau 0;
          yz s 6, p 0, d 6, lhs 36 = rhs 36, tau 0
tools/call {"name":"tension_verify","arguments":{"a":3,"b":4}}
→ product 12, verified true, max_tau 0;
  planes: xy s 7, p 12, d 1, lhs 1 = rhs 1, tau 0;
          xz s 3, p 0, d 3, lhs 9 = rhs 9, tau 0;
          yz s 4, p 0, d 4, lhs 16 = rhs 16, tau 0
```

The verdict (`verified: true`, `max_tau: 0`) is **identical** for all three pairs. What changes is exactly the plane values: `product` 15→18→12, the xy `s/p/d` triple 8/15/2 → 9/18/3 → 7/12/1, the yz `s/d` pair 5/5 → 6/6 → 4/4 — while the xz plane (`s = d = a = 3`) is unchanged. This is the algebraic identity `(a+b)² − 4ab = (a−b)²` reading as zero-tau closure on the plane entries. **A `verified: true` for (3,5) therefore establishes nothing special about 3, 5, or 8** — the same closure holds identically for (3,6) and (3,4).

**Newly observed discrepancy: `z` in `tension_verify`.** Spec 9.12 describes an optional third value `z` and states that z-participating planes appear in the result. Observed:

```text
tools/call {"name":"tension_verify","arguments":{"a":3,"b":5,"z":100}}
→ product 18, verified true, max_tau 0;
  planes: xy s 9, p 18, d 3, lhs 9 = rhs 9, tau 0;
          xz s 3, p 0, d 3, lhs 9 = rhs 9, tau 0;
          yz s 6, p 0, d 6, lhs 36 = rhs 36, tau 0
```

This response is **parsed-JSON identical** to `tension_verify(3,6)`: the input `b = 5` is effectively replaced by 6 in every output field, and `z = 100` appears in no field of the response. The documented "z participates" behavior could not be confirmed in this run; the discrepancy is listed in 17.8. The practical lesson for an agent: **a returned verdict does not necessarily reflect every input it received — audit the full response, not just `verified`.**

**`symbolic_rewrite` is syntactic only.** The plane values of (3,5) are `S = 8, P = 15, D = −2`:

```text
tools/call {"name":"symbolic_rewrite","arguments":{"expr_type":"tau","args":{"S":8,"P":15,"D":-2}}}
→ {"normal_form":"τ(8, 15, -2)","trace":[]}
tools/call {"name":"symbolic_rewrite","arguments":{"expr_type":"tau","args":{"S":8,"P":15,"D":3}}}
→ {"normal_form":"τ(8, 15, 3)","trace":[]}
```

`D = 3` is inconsistent with the observed plane (8² − 4·15 = 4, not 9), yet the second call returns a perfectly ordinary normal-form string — **no flag, no error, no mismatch detection**. `symbolic_rewrite` rewrites; it does not verify (the same lesson as the D=1 vs D=23 control in 16.6). For contrast, `symbolic_invariants` (called once, no arguments) returns the engine's listed invariants verbatim, including `tau_invariant` = `"(τ(?S, ?P, ?D) = (((?S * ?S) - (4 * ?P)) - (?D * ?D)))"` — a description of the system's own closure predicate, matching spec 9.12 exactly, and a result about the system, not about 3, 5, 8.

**Second structural view: `e8`.** Each of the three values projects independently to a valid E8 root:

```text
tools/call {"name":"e8","arguments":{"value":3}}
→ vec8 [-2,2,0,0,0,0,0,0], kind "D8", norm2 8, parity_even false, is_valid_root true, seed [3,3,3], gcd 1
tools/call {"name":"e8","arguments":{"value":5}}
→ vec8 [2,0,2,0,0,0,0,0], kind "D8", norm2 8, parity_even false, is_valid_root true, seed [5,5,5], gcd 1
tools/call {"name":"e8","arguments":{"value":8}}
→ vec8 [-2,0,-2,0,0,0,0,0], kind "D8", norm2 8, parity_even true, is_valid_root true, seed [8,8,8], gcd 1
```

No field returned by `e8` encodes any relation among 3, 5, 8 — each value is projected on its own. This is a view, not a verifier.

### 17.7 Interpretation of the evidence

Layer by layer, what an AI is justified in concluding from the results above:

1. **Address facts (demonstrated).** The address of `cubint_add(3,5)` is the address of the literal 8; the address of `mul(5,5)` equals the address of `pow(5,2)` equals the address of the literal 25. These are internal-consistency facts of the precomputed table for these specific inputs.
2. **Structural closure (demonstrated for the specific inputs called).** `tension_verify(3,5)` reports `verified true, max_tau 0` — a structural closure of the τ identity on the plane entries of that input, holding equally for (3,6) and (3,4).
3. **Diagnostic agreement/disagreement (demonstrated).** The `doctor`, `doctor_pair`, and `doctor_group` verdicts are input-sensitive: only 8 is axis-closed among {3,5,8}; the pair (3,5) is `is_closed true` at WARN while the sum-with-partner pairs (3,8) and (8,5) score worse; the group [3,5,8] is not closed (stress 69), while the uniform group [3,3,3] is (stress 0).
4. **Multiple witness agreement (demonstrated, shared table).** The agreement across `cubint_add`, `cubint_mul`, `cubint_pow`, `cubint_witness`, `doctor`, `doctor_pair_coords`, and `tension_verify` is consistent because these tools read one shared precomputed table. It is corroboration of one table — not independent confirmation (see 16.9 for the enumerated shared data in the 420 case).
5. **Independent verification (not established).** No pair of tools in this run supplies evidence that does not derive from the shared table.
6. **General mathematical proof (not claimed).** None of these results is a proof that 3 + 5 = 8 as an external mathematical fact. The engine's table is the ground of every result, and the public interface does not independently verify the table itself.

A practical consequence: when a Cube tool returns `verified: true`, `CLOSED`, or `PERFECT`, the justified conclusion is bounded by the tool's contract for that specific input — "the table entries for this input report closure" — and never "this equation has been verified". The `z = 100` result of 17.6 shows why: even the response's field set can fail to reflect the input that was supplied.

### 17.8 What was demonstrated, and what was not

**Demonstrated** (live, against the published build, 34 calls):

- Canonical zero-debt address representations of 3, 5, 8, and 25, and of the negative −8 (orientation 2).
- Result-address = literal-address for `add(3,5)`, and for `mul(5,5)` / `pow(5,2)` against 25 — multiple-view agreement within one table.
- Input sensitivity of the `doctor` self-pair, `doctor_pair`, and `doctor_group` verdicts, including the point that (3,5) scores better than (3,8) and (8,5), and that uniform groups close while mixed groups do not.
- The `doctor_pair_coords` documented example reproduced exactly (WARN, `match_count 8`, `is_closed true`), and the value-derived coordinate call reproducing the `doctor_pair(3,5)` score pattern.
- The universality of the τ structural closure across the observed pairs (3,5), (3,6), (3,4).
- The purely syntactic behavior of `symbolic_rewrite` (no mismatch detection for the deliberately wrong `D = 3`).
- Independent per-value `e8` projections, with no field encoding any relation among 3, 5, 8.
- Two **new discrepancies** versus the documented behavior:
  1. `tension_verify` with `z` (see 17.6): the response was parsed-JSON identical to `tension_verify(3,6)`; the supplied `z = 100` appears in no output field, and `b = 5` behaves as 6 in every output field. The 9.12 "z-participating planes" behavior was not confirmed in this build.
  2. `doctor`'s `axis_doctor.point` pads factor lists of fewer than three factors with **1** (3 → `[1,1,3]`, 5 → `[1,1,5]`); spec 9.4 documents only the 42 → `[2,3,7]` case.

**Not demonstrated:**

- An independent verification of any external mathematical statement, including "3 + 5 = 8".
- Closure of the group [3,5,8] — it was observed to be `is_closed false`, `stress 69`.
- Any property of table entries or inputs beyond the 34 calls of this run.
- The existence of the documented "z-participating planes" behavior of `tension_verify` in this build (see the discrepancy above).

### 17.9 Reproducibility

Every result in this section was produced by driving the published `cube.exe` through the documented MCP JSON-RPC-over-stdio interface (sections 6–7): launch the executable with its own folder as the working directory (so `cube_v5.bin` is found next to it), perform the handshake (`initialize` → `notifications/initialized` → `tools/list`, 87 tools), then execute the call list below one call at a time (the server is synchronous), recording each raw response. All 34 calls returned `ok` (the two expected `-32603` error cases are in the list and behaved exactly as documented); no timeouts, no `isError` results.

Exact call list (in order):

```json
[
  {"name":"cubint_witness","arguments":{"a":3}},
  {"name":"cubint_witness","arguments":{"a":5}},
  {"name":"cubint_witness","arguments":{"a":8}},
  {"name":"cubint_witness","arguments":{"a":-8}},
  {"name":"cubint_validate","arguments":{"a":3}},
  {"name":"cubint_validate","arguments":{"a":"abc"}},
  {"name":"cubint_add","arguments":{"a":3,"b":5}},
  {"name":"cubint_add","arguments":{"a":3,"b":6}},
  {"name":"cubint_mul","arguments":{"a":5,"b":5}},
  {"name":"cubint_pow","arguments":{"a":5,"b":2}},
  {"name":"cubint_pow","arguments":{"a":5,"b":3}},
  {"name":"cubint_witness","arguments":{"a":25}},
  {"name":"doctor","arguments":{"value":3}},
  {"name":"doctor","arguments":{"value":5}},
  {"name":"doctor","arguments":{"value":8}},
  {"name":"doctor_pair","arguments":{"a":3,"b":5}},
  {"name":"doctor_pair","arguments":{"a":3,"b":8}},
  {"name":"doctor_pair","arguments":{"a":8,"b":5}},
  {"name":"doctor_pair","arguments":{"a":3}},
  {"name":"doctor_pair_coords","arguments":{"x1":1,"y1":2,"z1":0,"x2":3,"y2":4,"z2":0}},
  {"name":"doctor_pair_coords","arguments":{"x1":1,"y1":1,"z1":3,"x2":1,"y2":1,"z2":5}},
  {"name":"doctor_group","arguments":{"values":[3,5,8]}},
  {"name":"doctor_group","arguments":{"values":[3,3,3]}},
  {"name":"symbolic_invariants","arguments":{}},
  {"name":"tension_verify","arguments":{"a":3,"b":5}},
  {"name":"tension_verify","arguments":{"a":3,"b":6}},
  {"name":"tension_verify","arguments":{"a":3,"b":4}},
  {"name":"tension_verify","arguments":{"a":3,"b":5,"z":100}},
  {"name":"symbolic_rewrite","arguments":{"expr_type":"tau","args":{"S":8,"P":15,"D":-2}}},
  {"name":"symbolic_rewrite","arguments":{"expr_type":"tau","args":{"S":8,"P":15,"D":3}}},
  {"name":"symbolic_rewrite","arguments":{"expr_type":"coord","args":{"x":3,"y":5,"z":8}}},
  {"name":"e8","arguments":{"value":3}},
  {"name":"e8","arguments":{"value":5}},
  {"name":"e8","arguments":{"value":8}}
]
```
