# Opterium GeoFormer — API Reference

Auto-generated from spec_compiled.json. All examples verified by tests.

---

## Module: `Cube27`
  File: `src/cube27.py`

### Class `Cube27`
  Self-similar 3-digit decimal addressing

#### `Cube27.cell_27(group) -> tuple`
  3-digit group → (cx,cy,cz) in 3×3×3

  **Examples:**
      • `cell_27(0) = (0, 0, 0)` → [0, 0, 0]
      • `cell_27(37) = (0, 0, 1)` → [0, 0, 1]
      • `cell_27(296) = (0, 2, 2)` → [0, 2, 2]
      *(+4 more)*

#### `Cube27.cell_index(group) -> int`
  3-digit group → 27-ary cell 0..26

  **Examples:**
      • `cell_index(0) = 0` → 0
      • `cell_index(36) = 0` → 0
      • `cell_index(37) = 1` → 1
      *(+6 more)*

  **Edge cases:**
      • clamped to 26

#### `Cube27.depth(mantissa) -> int`
  Levels of Cube27 needed for this mantissa

  **Examples:**
      • `depth(0) = 1` → 1
      • `depth(347) = 1` → 1
      • `depth(123456789) = 3` → 3

#### `Cube27.encode(mantissa) -> list`
  Split into 3-digit groups MSB-first

  **Examples:**
      • `encode(0) = [0]` → [0]
      • `encode(1) = [1]` → [1]
      • `encode(347) = [347]` → [347]
      *(+2 more)*

#### `Cube27.format_path(mantissa) -> str`
  Human-readable: 123|456|789| (cells)

  **Examples:**
      • `format_path(0)` → 000| ((0,0,0))
      • `format_path(347)` → 347| ((1,0,0))
      • `format_path(123456789)` → 123|456|789| ((0,1,0)|(1,1,0)|(2,1,0))

#### `Cube27.path_27(mantissa) -> list`
  Full Cube27 path: [(cx,cy,cz), ...]

  **Examples:**
      • `path_27(0) depth=1` → [[0, 0, 0]]
      • `path_27(347) depth=1` → [[1, 0, 0]]
      • `path_27(123456789) depth=3` → [[0, 1, 0], [1, 1, 0], [2, 1, 0]]

#### `Cube27.verify(mantissa) -> dict`
  Encode + verify PtTable hit

  **Examples:**
      • `verify(123456789) all_hit check` → {"mantissa": 123456789, "groups": [123, 456, 789], "depth...

---

## Module: `HashGrid`
  File: `src/hashgrid.py`

### Class `HashGrid`
  O(1) neighbor lookup in (S,D) space

#### `HashGrid.clear()`
  Clear all buckets

#### `HashGrid.insert(id, S, D, **extra) -> int`
  Insert point into bucket

  **Examples:**
      • `insert(0,5,3)` → 1

#### `HashGrid.insert_many(tokens) -> None`
  Batch insert tokens

  **Examples:**
      • `insert_many 2 tokens` → None

#### `HashGrid.lookup(S, D) -> list`
  Return entries in 3×3 neighborhood

  **Examples:**
      • `lookup(6,3)` → 2 neighbors with ids [0, 1]

#### `HashGrid.stats() -> dict`
  Bucket statistics

  **Examples:**
      • `current stats` → {"buckets": 2, "total": 5, "avg": 2.5, "max": 4}

### Function `geometric_attention(tokens, window=16, ...) -> list`
  One layer of hashgrid attention

  **Examples:**
      • `first 3 tokens attention` → {"id=0": {"context": 72, "neighbors": 3}, "id=1": {"conte...

### Function `geometric_weight(S1,D1, S2,D2, eps=1.0) -> float`
  Proximity weight: 1/(eps + |ΔS| + |ΔD|)

  **Examples:**
      • `weight between (10,5) and (20,15)` → 0.047619047619047616
      • `symmetric` → 0.047619047619047616

---

## Module: `Pt`
  File: `src/spec-kit/methods/__init__.py`

### Class `Pt`
  Geometric point with mantissa-rank notation x|y| = x / 10^y

#### `Pt.__add__(other) -> Pt`
  Component-wise addition

  **Examples:**
      • `Pt(3,5)+Pt(2,7)=Pt(5,12)` → Pt(5,12)

#### `Pt.__init__Pt(x, y=1)`
  Construct Pt with S/D/P auto-computed

  **Examples:**
      • `Pt(0,1)` → Pt(0,1) S=1 D=-1 P=0
      • `Pt(3,5)` → Pt(3,5) S=8 D=-2 P=15
      • `Pt(1024,1024)` → Pt(1024,1024) S=2048 D=0 P=1048576
      *(+2 more)*

#### `Pt.__mul__(other) -> Pt`
  Component-wise multiplication

  **Examples:**
      • `Pt(3,5)*Pt(2,7)=Pt(6,35)` → Pt(6,35)

#### `Pt.from_decimal(d) -> Pt`
  Decimal → Pt with full precision

  **Examples:**
      • `from_decimal(0) → Pt(0,0) → to_decimal=0` → {"Pt": "(0,0)", "to_decimal_back": "0"}
      • `from_decimal(3.14) → Pt(314,2) → to_decimal=3.14` → {"Pt": "(314,2)", "to_decimal_back": "3.14"}
      • `from_decimal(1E+30) → Pt(1000000000000000000000000000000,0) → to_decimal=1000000000000000000000000000000` → {"Pt": "(1000000000000000000000000000000,0)", "to_decimal...
      *(+2 more)*

#### `Pt.from_int(v) -> Pt`
  Integer → Pt for geometric use: P=v

  **Examples:**
      • `from_int(0)` → Pt(0,1) P=0
      • `from_int(1)` → Pt(1,1) P=1
      • `from_int(42)` → Pt(42,1) P=42
      *(+1 more)*

#### `Pt.from_real(r) -> Pt`
  Float → Pt(mantissa, rank) via Decimal

  **Examples:**
      • `from_real(0.0) → Pt(0,0) → to_real=0.0` → {"Pt": "(0,0)", "to_real_back": 0.0}
      • `from_real(0.347) → Pt(347,3) → to_real=0.347` → {"Pt": "(347,3)", "to_real_back": 0.347}
      • `from_real(-3.14) → Pt(-314,2) → to_real=-3.14` → {"Pt": "(-314,2)", "to_real_back": -3.14}
      *(+2 more)*

#### `Pt.from_sd(S,D) -> Pt`
  Construct Pt from (S,D) coordinates

  **Examples:**
      • `from_sd(8,-2)` → Pt(3,5)
      • `from_sd(10,0)` → Pt(5,5)
      • `from_sd(7,-3)` → Pt(2,5)

#### `Pt.inv() -> Pt`
  Inverse via Decimal lift-solve-project

  **Examples:**
      • `inv(347|3|) → Pt(28818443804034582132564841498559077809798270893372,49), p*inv=1.0000000000000000000000000000000000000000000000000` → {"inv": "Pt(288184438040345821325648414985590778097982708...
      • `inv(2|1|) → Pt(5,0), p*inv=1.0` → {"inv": "Pt(5,0)", "product": "1.0"}
      • `inv(10|1|) → Pt(1,0), p*inv=1` → {"inv": "Pt(1,0)", "product": "1"}
      *(+2 more)*

#### `Pt.parse(s) -> Pt`
  Parse mantissa|rank| notation: "347|3|"

  **Examples:**
      • `parse("347|3|")` → Pt(347,3)
      • `parse("0|3|")` → Pt(0,3)
      • `parse("42|")` → Pt(42,1)
      *(+1 more)*

#### `Pt.to_decimal() -> Decimal`
  Pt → Decimal with exact precision

  **Examples:**
      • `Pt(347,3).to_decimal()` → 0.347

#### `Pt.to_real() -> float`
  Pt → float

  **Examples:**
      • `Pt(347,3).to_real() = 0.347` → 0.347
      • `Pt(0,0).to_real() = 0.0` → 0.0
      • `Pt(-314,2).to_real() = -3.14` → -3.14

#### `Pt.verbose() -> str`
  Full description: Pt(x,y S= D= P=)

  **Examples:**
      • `verbose: Pt(347,3 S=350 D=344 P=1041)` → Pt(347,3 S=350 D=344 P=1041)

### Function `geo_add(a:Pt, b:Pt) -> Pt`
  Geometric addition (cross-multiply)

  **Examples:**
      • `geo_add(Pt(3,5),Pt(2,7))=Pt(31,35)` → Pt(31,35)

### Function `geo_mul(a:Pt, b:Pt) -> Pt`
  Geometric multiply

  **Examples:**
      • `geo_mul(Pt(3,5),Pt(2,7))=Pt(6,35)` → Pt(6,35)

### Function `radd(a:Pt, b:Pt) -> Pt`
  Mantissa-rank add with rank alignment

  **Examples:**
      • `radd(0.3,0.2) = 0.5` → {"Pt": "(5,1)", "to_real": 0.5}

### Function `rdiv(a:Pt, b:Pt) -> Pt`
  Mantissa-rank divide = rmul(a, inv(b))

  **Examples:**
      • `rdiv(0.3,0.2) = 1.5` → {"Pt": "(15,1)", "to_real": 1.5}
      • `rdiv(-0.3,0.2) = -1.5` → {"Pt": "(-15,1)", "to_real": -1.5}

### Function `rmul(a:Pt, b:Pt) -> Pt`
  Mantissa-rank multiply

  **Examples:**
      • `rmul(0.3,0.2) = 0.06` → {"Pt": "(6,2)", "to_real": 0.06}

### Function `rsub(a:Pt, b:Pt) -> Pt`
  Mantissa-rank subtract with rank alignment

  **Examples:**
      • `rsub(0.5,0.03) = 0.47` → {"Pt": "(47,2)", "to_real": 0.47}
      • `rsub(-0.3,-0.1) = -0.2` → {"Pt": "(-2,1)", "to_real": -0.2}
      • `rsub(0.05,-0.3) = 0.35` → {"Pt": "(35,2)", "to_real": 0.35}

### Function `sd_tuple_matrix(M) -> list`
  Convert to (S,D) pairs

  **Examples:**
      • `sd_tuple_matrix` → [[[2, 0], [3, 1]], [[4, 2], [5, 3]]]

### Function `to_pt_matrix(M) -> list`
  Convert int matrix to Pt matrix

  **Examples:**
      • `to_pt_matrix` → [["1|1|", "2|1|"], ["3|1|", "4|1|"]]

### Function `validate_shape(A, B) -> (m,k,n)`
  Validate matrix dimensions

  **Examples:**
      • `2x3 * 3x2 = (2,3,2)` → [2, 3, 2]

---

## Module: `PtTable`
  File: `src/arith_table.py`

### Class `PtTable`
  Zero-arithmetic address table: all (x,y) → {S, D, P} via precomputed lookup

#### `PtTable.D(x, y) -> int`
  Difference: x - y via table with negative reflection

  **Args:**
    - `?`: `?`  [-1024..1024]
    - `?`: `?`  [-1024..1024]

  **Returns:**
    - `int`  

  **Examples:**
      • `D(3,5) = -2` → -2
      • `D(5,3) = 2` → 2
      • `D(-3,5) = -8` → -8
      *(+4 more)*

#### `PtTable.P(x, y) -> int`
  Product: x * y via table with sign inversion

  **Args:**
    - `?`: `?`  [-1024..1024]
    - `?`: `?`  [-1024..1024]

  **Returns:**
    - `int`  

  **Examples:**
      • `P(3,5) = 15` → 15
      • `P(-3,5) = -15` → -15
      • `P(3,-5) = -15` → -15
      *(+3 more)*

#### `PtTable.S(x, y) -> int`
  Sum: x + y via table with negative reflection

  **Args:**
    - `?`: `?`  [-1024..1024]
    - `?`: `?`  [-1024..1024]

  **Returns:**
    - `int`  

  **Examples:**
      • `S(3,5) = 8` → 8
      • `S(-3,5) = 2` → 2
      • `S(3,-5) = -2` → -2
      *(+5 more)*

  **Edge cases:**
      • max coord
      • zero+zero

#### `PtTable.abs(x) -> int`
  Absolute value via table or fallback

  **Examples:**
      • `abs(-5) = 5` → 5
      • `abs(-1) = 1` → 1
      • `abs(0) = 0` → 0
      *(+2 more)*

#### `PtTable.activation_table(fn, lo, hi) -> dict`
  Precompute activation fn over [lo,hi]

  **Examples:**
      • `ReLU over [-3,5]` → {"-3": 0, "-2": 0, "-1": 0, "0": 0, "1": 1, "2": 2, "3": ...

#### `PtTable.conj(x, y) -> Tuple[int,int]`
  Quaternion conjugate: (y, x)

  **Examples:**
      • `conj(3,5) = (5, 3)` → [5, 3]
      • `conj(-3,5) = (5, -3)` → [5, -3]
      • `conj(0,5) = (5, 0)` → [5, 0]

#### `PtTable.diff(a, b) -> int`
  diff via table with fallback

  **Examples:**
      • `diff(3,5) = -2 (expected -2)` → -2
      • `diff(-3,5) = -8 (expected -8)` → -8
      • `diff(3,-5) = 8 (expected 8)` → 8
      *(+4 more)*

#### `PtTable.dot(a_list, b_list) -> int`
  Vector dot product via product lookups

  **Examples:**
      • `dot([1,2,3],[4,5,6]) = 1*4+2*5+3*6` → 32

#### `PtTable.from_sd(S, D) -> Tuple[int,int]`
  Recover (x,y) from sum and difference

  **Examples:**
      • `from_sd(8,-2) = (3, 5)` → [3, 5]
      • `from_sd(8,2) = (5, 3)` → [5, 3]
      • `from_sd(10,0) = (5, 5)` → [5, 5]
      *(+2 more)*

#### `PtTable.has(x, y) -> bool`
  Check if (x,y) is in table range

  **Examples:**
      • `has(0,1) = True` → true
      • `has(1024,1024) = True` → true
      • `has(0,0) = True` → true
      *(+1 more)*

#### `PtTable.isqrt(n) -> int`
  Integer square root (table or math.isqrt)

  **Examples:**
      • `isqrt(0) = 0` → 0
      • `isqrt(1) = 1` → 1
      • `isqrt(4) = 2` → 2
      *(+3 more)*

#### `PtTable.lookup(x, y) -> dict`
  Returns {S, D, P} for coordinates

  **Examples:**
      • `lookup(3,5)` → {"S": 8, "D": -2, "P": 15}

#### `PtTable.matmul(A, B) -> List[List[int]]`
  Matrix multiply via product lookups

  **Examples:**
      • `2x2 multiply` → [[19, 22], [43, 50]]

#### `PtTable.max(a, b) -> int`
  Maximum via table or fallback

  **Examples:**
      • `max(3,5) = 5` → 5
      • `max(-3,5) = 5` → 5
      • `max(0,0) = 0` → 0

#### `PtTable.min(a, b) -> int`
  Minimum via table or fallback

  **Examples:**
      • `min(3,5) = 3` → 3
      • `min(-3,5) = -3` → -3
      • `min(0,0) = 0` → 0

#### `PtTable.pairs_for_product(P) -> List[Tuple]`
  All factor pairs (x,y) with x*y=P

  **Examples:**
      • `all factor pairs of 12` → [[1, 12], [2, 6], [3, 4], [4, 3], [6, 2], [12, 1]]
      • `prime: only (1,17) and (17,1)` → [[1, 17], [17, 1]]

#### `PtTable.pow10(n) -> int`
  10^n via table or fallback

  **Examples:**
      • `pow10(0) = 1` → 1
      • `pow10(1) = 10` → 10
      • `pow10(3) = 1000` → 1000
      *(+2 more)*

#### `PtTable.product(a, b) -> int`
  product via table with fallback

  **Examples:**
      • `product(3,5) = 15 (expected 15)` → 15
      • `product(-3,5) = -15 (expected -15)` → -15
      • `product(3,-5) = -15 (expected -15)` → -15
      *(+4 more)*

#### `PtTable.square(n) -> int`
  n^2 via table or fallback

  **Examples:**
      • `square(0) = 0` → 0
      • `square(5) = 25` → 25
      • `square(12) = 144` → 144
      *(+1 more)*

#### `PtTable.sum(a, b) -> int`
  sum via table with fallback

  **Examples:**
      • `sum(3,5) = 8 (expected 8)` → 8
      • `sum(-3,5) = 2 (expected 2)` → 2
      • `sum(3,-5) = -2 (expected -2)` → -2
      *(+4 more)*

#### `PtTable.summary() -> dict`
  Table statistics

  **Examples:**
      • `current table stats` → {"max_coord": 1024, "cached": true, "sd_size": 1050625, "...

---

## Module: `delta_ops`
  File: `src/delta_ops.py`

### Class `HealthVector`
  7-channel stability monitor

#### `HealthVector.critical -> bool`
  Any channel >= 0.65

  **Examples:**
      • `critical check` → false

#### `HealthVector.max_channel -> Tuple[str, float]`
  Highest channel name+value

  **Examples:**
      • `max channel` → ["E_entropy", 0.3]

#### `HealthVector.merge(other) -> HealthVector`
  Element-wise max of two HVs

  **Examples:**
      • `merged` → E_assoc=0.8

#### `HealthVector.ok -> bool`
  All channels < 0.35

  **Examples:**
      • `ok check` → true

#### `HealthVector.warn -> bool`
  Any channel in [0.35, 0.65)

  **Examples:**
      • `warn check` → false

### Function `DELTA_ADD(...) -> (result, HealthVector)`
  ADD operator: R → R

  **Examples:**
      • `DELTA_ADD(3, 5) → 8` → {"result": 8, "hv_ok": true}
      • `DELTA_ADD(-3, 5) → 2` → {"result": 2, "hv_ok": true}
      • `DELTA_ADD(0, 0) → 0` → {"result": 0, "hv_ok": true}

### Function `DELTA_INV(...) -> (result, HealthVector)`
  INV operator: R → R

  **Examples:**
      • `DELTA_INV(2.0,) → 0.5` → {"result": 0.5, "hv_ok": true}
      • `DELTA_INV(4.0,) → 0.25` → {"result": 0.25, "hv_ok": true}
      • `DELTA_INV(0.0,) → inf` → {"result": Infinity, "hv_ok": false}

### Function `DELTA_INV_NS(...) -> (result, HealthVector)`
  INV_NS operator: S → S

  **Examples:**
      • `DELTA_INV_NS((0, 1, 0),) → (0.0, 1.0, 0.0)` → {"result": [0.0, 1.0, 0.0], "hv_ok": true}
      • `DELTA_INV_NS((0, 0, 0),) → (1000.0000000000001, 0.0, 0.0)` → {"result": [1000.0000000000001, 0.0, 0.0], "hv_ok": false}

### Function `DELTA_MUL(...) -> (result, HealthVector)`
  MUL operator: R → R

  **Examples:**
      • `DELTA_MUL(3, 5) → 15` → {"result": 15, "hv_ok": true}
      • `DELTA_MUL(-3, 5) → -15` → {"result": -15, "hv_ok": true}
      • `DELTA_MUL(0, 5) → 0` → {"result": 0, "hv_ok": true}

### Function `DELTA_OPTG(state, attractor) -> (result, HealthVector)`
  OPTG operator: E8 → E8 (Weyl geodesic)

  **Examples:**
      • `OPTG([1,0],[0,1])` → {"result": [1.0, 0.0], "hv_ok": false}
      • `OPTG([0.5,0.5],[1,0])` → {"result": [0.5, 0.5], "hv_ok": false}

### Function `DELTA_PPH(singular_values) -> (result, HealthVector)`
  PPH operator: S → S (projection residue)

  **Examples:**
      • `PPH([5.0, 3.0]) → -2.7081` → {"result": -2.7081, "hv_ok": true}
      • `PPH([0.0, 0.0]) → inf` → {"result": Infinity, "hv_ok": false}
      • `PPH([10.0]) → -2.3026` → {"result": -2.3026, "hv_ok": true}

### Function `DELTA_ROT(...) -> (result, HealthVector)`
  ROT operator: C → C

  **Examples:**
      • `DELTA_ROT((1+0j),) → (6.123233995736766e-17+1j)` → {"result": "(6.123233995736766e-17+1j)", "hv_ok": true}

### Function `DELTA_SHIFT(...) -> (result, HealthVector)`
  SHIFT operator: R → R

  **Examples:**
      • `DELTA_SHIFT(5.0,) → 5.0` → {"result": 5.0, "hv_ok": true}
      • `DELTA_SHIFT(5.0,) → 5.0` → {"result": 5.0, "hv_ok": true}

### Function `check_domain(domain, assoc=..., commut=..., div=...) -> bool`
  Verify algebra properties

  **Examples:**
      • `Octonions correct` → true
      • `Octonions not associative` → false

### Function `compose_parallel(*ops) -> CompositeDelta`
  Parallel composition over disjoint subspaces

  **Examples:**
      • `ADD||MUL on (3,5)` → ["((8.0, 15.0), HV(E_assoc=0.0000, ok=True))"]

### Function `compose_sequential(*ops) -> CompositeDelta`
  Sequential composition: apply ops[0], then ops[1], ...

  **Examples:**
      • `SHIFT>>INV = 1/(5*10) = 0.02` → ["0.02", "hv=True"]

### Function `select_fallback(op_name, domain) -> str|None`
  Fallback strategy for non-invertible domains

  **Examples:**
      • `Sedenion inv fallback` → Δ_ROBUST_INV

---

## Module: `doctor_geo`
  File: `src/doctor_geo.py`

### Class `GeoHealthVector`
  Alias for delta_ops.HealthVector (7-channel)

### Class `SwarmDoctor`
  Swarm-powered DoctorCore integration

#### `SwarmDoctor.choose_route(candidates, deterministic) -> str`
  Swarm route choice

  **Examples:**
      • `deterministic choice` → e8_direct

#### `SwarmDoctor.clear_quarantine() -> None`
  Clear all quarantined items

  **Examples:**
      • `clear quarantine` → None

#### `SwarmDoctor.get_quarantine(key) -> payload`
  Retrieve quarantined item

  **Examples:**
      • `quarantine roundtrip` → {"val": 42}

#### `SwarmDoctor.judge(hv, context) -> str`
  Verdict: OK/WARN/QUARANTINE/ROLLBACK

  **Examples:**
      • `judge(OK) → OK` → OK
      • `judge(bad) → QUARANTINE/ROLLBACK` → ROLLBACK

#### `SwarmDoctor.judge_full(hv, context='') -> dict`
  Full verdict with details

  **Examples:**
      • `judge_full(OK)` → {"level": "OK", "ok": true, "peak": 0.0, "reasons": [], "...

#### `SwarmDoctor.opterium_judge(ohv, context='') -> Any`
  Opterium-compatible judge

  **Examples:**
      • `opterium_judge string` → None

#### `SwarmDoctor.opterium_verdict(geo_hv, context='') -> dict`
  Full optical verdict

  **Examples:**
      • `opterium_verdict` → {"level": "OK", "ok": true, "peak": 0.2, "reasons": [], "...

#### `SwarmDoctor.quarantine_item(key, payload, verdict)`
  Store quarantined item

#### `SwarmDoctor.register_route(name, potential)`
  Register a route

#### `SwarmDoctor.reinforce_route(route, success)`
  Reinforce after verdict

  **Examples:**
      • `reinforce` → 5.6

#### `SwarmDoctor.route_probabilities(candidates=None) -> dict`
  Current route probabilities

  **Examples:**
      • `all route probs` → {"e8_direct": 0.13513513513513511, "cube_project": 0.1351...

### Function `ROUTE_REGISTRYdict of route_name -> info`
  All registered routes with domains

  **Examples:**
      • `route domains` → {"e8_direct": "E8", "cube_project": "Cube27", "farey_path...

### Function `geo_to_opterium_hv(ghv) -> OpHealthVector|None`
  Convert geo HealthVector to opterium format

  **Examples:**
      • `HV conversion` → None (opterium unavailable)

### Function `opterium_to_geo_hv(ohv) -> GeoHealthVector`
  Convert opterium HealthVector to geo format

---

## Module: `e8_twist`
  File: `src/e8_twist.py`

### Class `TwistEngine`
  E8 TWIST operations (zero float, zero trig)

#### `TwistEngine.closure_angle(angle=70) -> dict`
  Closure via address routing

  **Examples:**
      • `closure 70°` → {"angle": 70, "half_angle": 35, "energy": 0.008, "status"...

#### `TwistEngine.cycle_2520(angle) -> dict`
  2520-cycle params

  **Examples:**
      • `35° → 72 steps` → {"angle": 35, "steps": 72, "total_deg": 2520, "K": 7, "ti...
      • `70° → 36 steps` → {"angle": 70, "steps": 36, "total_deg": 2520, "K": 7, "ti...

#### `TwistEngine.cycle_all_angles() -> list`
  Cycle angles for 35°, 70°, 105°, 140°

  **Examples:**
      • `cycles for all angles` → 4 angle results

#### `TwistEngine.scan_configs() -> list`
  Scan configs, sorted by amplitude

  **Examples:**
      • `scan results count` → 15

#### `TwistEngine.summary() -> dict`
  TwistEngine status summary

  **Examples:**
      • `current status` → {"triality": {"V": 112, "S+": 64, "S-": 64}, "triality_su...

#### `TwistEngine.triality_groups() -> dict`
  Split 240 roots: V(112), S+(64), S-(64)

  **Examples:**
      • `triality groups` → {"V": 112, "S+": 64, "S-": 64}

#### `TwistEngine.twist(phase, config) -> dict`
  Apply TWIST to triality groups

  **Examples:**
      • `TWIST(0, (112,64,192))` → {"phase": 0, "config": [112, 64, 192], "max_amplitude": 8...

### Function `address_to_root(x, y) -> tuple`
  Map 2D address to 8D E8 root

  **Examples:**
      • `address(3,5) → root` → [1, 1, 1, 1]
      • `address(2,2) → root` → [1, 1, 0, 0]
      • `address(4,3) → root` → [1, 1, 1, 1]

### Function `root_properties(root) -> dict`
  O(1) extraction from root address

  **Examples:**
      • `properties of address(3,5) root` → {"sector": "Spinor", "scale": 1, "on_axis": true, "parity...

---

## Module: `geoformer`
  File: `src/geoformer.py`

### Class `GeoFormer`
  Full architecture: embedding + stacked blocks

#### `GeoFormer.forward(tokens) -> (List[Pt], HealthVector)`
  Full forward pass

  **Examples:**
      • `GeoFormer forward` → {"out": ["4000", "27040", "77440", "77440", "77440"], "hv...

### Class `GeometricBlock`
  One GeoFormer layer: Resonate→Project→Shift

#### `GeometricBlock.forward(tokens) -> (List[Pt], HealthVector)`
  Forward pass

  **Examples:**
      • `block forward` → {"output": ["10", "40", "90", "90", "90"], "hv_ok": true}

### Class `GeometricEmbedding`
  Token → Pt(S,D) embedding

#### `GeometricEmbedding.embed(token) -> Pt`
  Map token to Pt

  **Examples:**
      • `embed(42)` → 42|1|

#### `GeometricEmbedding.embed_sequence(tokens) -> List[Pt]`
  Batch embed

  **Examples:**
      • `embed([1,2,3])` → ["1|1|", "2|1|", "3|1|"]

### Class `Pt`
  GeoFormer Pt (inherits methods.Pt, adds .zero())

#### `Pt.from_decimal(d) -> Pt`
  Decimal → Pt (inherited)

  **Examples:**
      • `from_decimal(3.14)` → Pt(314,2)

#### `Pt.from_int(v) -> Pt`
  Integer → Pt (inherited)

  **Examples:**
      • `from_int(42)` → Pt(42,1) P=42

#### `Pt.from_real(r) -> Pt`
  Float → Pt (inherited)

  **Examples:**
      • `from_real(0.347)` → Pt(347,3)

#### `Pt.from_sd(S,D) -> Pt`
  Construct from (S,D) (inherited)

  **Examples:**
      • `from_sd(8,-2)` → Pt(3,5)

#### `Pt.inv() -> Pt`
  Inverse (inherited)

  **Examples:**
      • `inv(Pt(2,1))` → Pt(5,0)

#### `Pt.parse(s) -> Pt`
  Parse mantissa|rank| notation (inherited)

  **Examples:**
      • `parse("347|3|")` → Pt(347,3)

#### `Pt.to_decimal() -> Decimal`
  Pt → Decimal (inherited)

  **Examples:**
      • `to_decimal()` → 0.347

#### `Pt.to_real() -> float`
  Pt → float (inherited)

  **Examples:**
      • `to_real()` → 0.347

#### `Pt.verbose() -> str`
  Full description (inherited)

  **Examples:**
      • `verbose()` → Pt(347,3 S=350 D=344 P=1041)

#### `Pt.zero() -> Pt`
  Zero point: Pt(0,1)

  **Examples:**
      • `zero()` → 0|1|

### Class `SwarmTrainer`
  Swarm reinforcement trainer (no backprop)

#### `SwarmTrainer.train(dataset, epochs) -> List[Dict]`
  Multi-epoch training

  **Examples:**
      • `2 epochs` → [{"episode": 2, "score": 0.0}, {"episode": 3, "score": 0.0}]

#### `SwarmTrainer.train_step(tokens, target) -> dict`
  One training episode

  **Examples:**
      • `train_step result` → {"episode": 1, "score": 0.0, "success": false, "hv_ok": t...

### Function `doctor_judge(output, target, hv) -> str`
  Quick geometric doctor: OK/WARN/FAIL

  **Examples:**
      • `doctor_judge verdict` → WARN

---

## Module: `phi_algebra`
  File: `src/phi_algebra.py`

### Function `PHI1_SHIFT(state, *args) -> state'`
  Φ1(→) Translate: move state along vector, add coordinate, displace.

  **Examples:**
      • `SHIFT(0,0,dx=1)` → [1, 0]

### Function `PHI2_PHASE(state, *args) -> state'`
  Φ2(↻) Rotate: cycle through discrete phase states, resonate at harmonic.

  **Examples:**
      • `PHASE([1,2,3])` → [2, 3, 1]

### Function `PHI3_FIXEDPOINT(state, *args) -> state'`
  Φ3(⊙) Stabilize: project to nearest fixed point of a map, converge to attractor.

  **Examples:**
      • `FIXEDPOINT(10→5)` → 5.0

### Function `PHI4_RECURSION(state, *args) -> state'`
  Φ4(↺) Recurse: apply self again, re-enter with transformed state.

### Function `PHI5_PROJECTION(state, *args) -> state'`
  Φ5(↓) Project: reduce dimension, cast shadow, observe through lens.

  **Examples:**
      • `PROJECTION` → [1, 3]

### Function `PhiPath(ops)`
  Sequence of Φ-operators

  **Examples:**
      • `2-operator path` → len=2 ops

### Function `harmonic_series(fundamental, harmonics) -> PhiPath`
  Φ₁∘Φ₂ repeated

  **Examples:**
      • `3 harmonics` → Φ1(→) ∘ Φ2(↻) ∘ Φ1(→) ∘ Φ2(↻) ∘ Φ1(→) ∘ Φ2(↻)

### Function `periodic_orbit(period) -> PhiPath`
  Periodic cycle via Φ₂

  **Examples:**
      • `4-step orbit` → Φ2(↻) ∘ Φ2(↻) ∘ Φ2(↻) ∘ Φ2(↻)

---

## Module: `swarm`
  File: `src/swarm.py`

### Class `IntelligentSwarm`
  Decision engine replacing Bayes

#### `IntelligentSwarm.decide(candidates=None, deterministic=False) -> SwarmNode`
  Choose best node

  **Examples:**
      • `deterministic choice` → A

#### `IntelligentSwarm.probabilities() -> dict`
  Probability distribution over nodes

  **Examples:**
      • `3-node probs` → {"A": 0.3333333333333333, "B": 0.3333333333333333, "C": 0...

#### `IntelligentSwarm.register(nid, potential=1.0) -> SwarmNode`
  Register a node

#### `IntelligentSwarm.score(nid) -> float`
  Raw score for a node

  **Examples:**
      • `score A` → 5.6

#### `IntelligentSwarm.update(nid, success)`
  Reinforce node

  **Examples:**
      • `after reinforce A+ B-` → 5.6

### Function `BayesReplacement(swarm)`
  Bayesian interface over Swarm

  **Examples:**
      • `posterior after update` → {"A": 0.35998107755605074, "B": 0.313291120800685}
      • `predict best` → A

---

*Generated from 10 modules, 258 verified examples*