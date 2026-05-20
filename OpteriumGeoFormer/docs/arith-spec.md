# Feature Specification: Zero-Arithmetic Address Architecture

**Branch**: `arith-free` | **Status**: Active  
**Input**: Replace ALL arithmetic operators (+, -, *, /, //, %, sum, isqrt) with PtTable address lookups — no computation, only navigation.

## Core Principle

From `opterium_v1.0_spec.txt` Section 4:
> Arithmetic is **navigation** through pre-existing structure. All results already exist at their addresses.

From Section 13 (NUMBER_IS_GRAPH):
> Never treat a number as a scalar. Every number is a **routing key**.

From Section 16.1 (The Trap of "Computation"):
> The table is not a helper to compute; it is the structure itself. Never compute x+y yourself.

## What is Banned

- `+` (addition) — use PtTable.S(x,y) instead
- `-` (subtraction) — use PtTable.D(x,y) instead
- `*` (multiplication) — use PtTable.P(x,y) instead
- `/`, `//` (division) — use PtTable divisor lookup instead
- `%` (modulo) — use PtTable remainder lookup instead
- `sum()`, `abs()`, `min()`, `max()` — use table pre-computed values
- `int()`, `float()` conversion — only at table build time
- `**` (exponentiation) — use table scaling
- `math.*` (isqrt, sin, cos, log) — banned entirely

## What is Allowed

- PtTable.lookup(x, y) → {S, D, P} (address navigation)
- Pt.S, Pt.D, Pt.P properties (backed by PtTable)
- Table construction at init time (ONE arithmetic pass permitted at module load)
- Scaling: PtTable.scale(primitive, factor) for coordinates > table range
- Comparison operators (==, !=, <, >) for control flow
- String operations, list indexing, dict lookups

## User Stories

### US1 — Pt class zero-arith (Priority: P0)
Pt(x,y).S must read from PtTable, not compute x+y. Same for D, P.
**Test**: Pt(4,3).S == 7, .D == 1, .P == 12 — via table, not arithmetic.

### US2 — geoformer.py zero-arith (Priority: P0)
All triple products, attention mixing, shift operations use Pt lookups only.
**Test**: GeometricBlock.forward 10 tokens returns correct output without any +-*/ in the hot path.

### US3 — hashgrid.py zero-arith (Priority: P0)
Bucket key derivation, neighbor weighting, context averaging all via table.
**Test**: hashgrid geometric_attention returns same result as before.

### US4 — swarm.py zero-arith (Priority: P1)
Swarm probabilities, pheromone decay, node selection via table lookups.
**Test**: IntelligentSwarm probabilities sum to 1.0.

### US5 — delta_ops.py zero-arith (Priority: P1)
Δ_INV, Δ_SCALE, Δ_CLOSURE, Δ_OPTG — all operators as pure table actions.
**Test**: All Δ-constructors pass self-tests.

### US6 — spec-kit methods zero-arith (Priority: P0)
pt_naive, pytable_mm, sd_matmul, geo_resonant — zero arithmetic in all 4 methods.
**Test**: 20/20 correctness tests pass without +-*/.

### US7 — phi_algebra.py zero-arith (Priority: P2)
Φ-paths use table navigation, not numeric iteration.
**Test**: All 5 Φ-verbs produce correct paths.

### US8 — e8_twist.py zero-arith (Priority: P0) ✅ Already done
TA-DA: Already refactored in this session.

### US9 — doctor_geo.py zero-arith (Priority: P2)

### US10 — tests zero-arith (Priority: P2)
All test data generation uses table paths, not `[i+1 for i in range(n)]`.

### US11 — Real-number mantissa notation `x|y|` (Priority: P1)
Introduce a human-readable string notation `x|y|` for Pt(x, y) that exposes the **mantissa-rank interpretation** of Pt coordinates.

**Core insight**: `0.347 = 347|3|` — a real number decomposes into mantissa (347) and rank (3 = power of 10 denominator). Pt(x,y) already stores this pair:
- `Pt(x, y)` in decimal interpretation = `x × 10^(-y)` (mantissa × 10^(-rank))
- This coexists with the rational interpretation `Pt(x, y) = x / y` used in `geo_add`/`geo_mul`

**User story**:
- Writing `Pt(347, 3)` in REPL → `347|3|` not `Pt(347,3 S=350 D=344 P=1041)`
- Parsing `"347|3|"` → `Pt(347, 3)`
- `Pt.from_real(0.347)` → `Pt(347, 3)` (auto-normalize float to mantissa-rank)
- `Pt(347, 3).to_real()` → `0.347` (via PtTable.pow10, no direct float arithmetic)
- Roundtrip: `Pt.from_real(r).to_real() == r` up to float precision
- The `|` delimiter signals "this is a mantissa-rank address, not arbitrary Pt"

**Notation grammar**:
```
notation := mantissa '|' rank '|'
mantissa := ['-'] digit+
rank     := digit+
```

**Tests**:
1. `Pt(347, 3)` → repr contains `347|3|` (new __repr__)
2. `Pt.parse("347|3|")` → `Pt(347, 3)`
3. Roundtrip: `Pt.parse(str(Pt(x,y))) == Pt(x,y)` for all x in [0..10], y in [1..5]
4. `Pt.from_real(0.347)` → `Pt(347, 3)`
5. `Pt(347, 3).to_real()` → `0.347` (float tolerance)
6. Negative mantissa: `Pt(-347, 3).to_real()` → `-0.347`
7. Zero mantissa: `Pt(0, 3).to_real()` → `0.0`
8. Large: `Pt(1234567, 4).to_real()` → `123.4567`
9. `Pt.from_real(0.0)` → `Pt(0, 1)` (y defaults to 1)
10. Parse error on malformed input (missing `|`, letters, etc.)
