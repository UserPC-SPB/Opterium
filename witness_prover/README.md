# Witness Prover: machine-checkable proof certificates for Opterium

> **Epistemology, not just automation.** This folder implements the proof
> model that README §2 describes: *"Proof is NOT deduction from axioms. Proof
> IS the closure of multiple independent routes on the same address, with
> zero residual tension, certified by an explicit verification procedure."*
> The program turns that definition into concrete, machine-readable,
> re-checkable certificate files.

## How it differs from Lean — and why it is stricter

**Lean** creates proofs as *terms inside a formal system with axioms*. A
Lean proof is trustworthy **only if every axiom is true**. Garbage axioms →
valid-looking but meaningless proofs (the formal system cannot detect that
the foundations are false).

**This prover** trusts no axioms — none exist here: Opterium forbids
introducing anything unjustified or merely postulated — and it does not even
trust the engine.
A claim is certified only when:

1. several **independent witness routes** (different computation paths to
   the same address) all close exactly — `tau = 0`; and
2. a **kernel routine re-derives the claim from first principles** using
   only exact standard-library arithmetic, never reading engine output.

So the certificate is a **reproducible computation record**, not a
chain of unexamined postulates (Opterium admits none at all). Any third party can re-run the kernels and get bit-identical
verdicts — including on a machine that has never seen the engine.

## Repository layout

```
witness_prover/
├── prover_top.py        # CLI + certificate assembly
├── prover_kernel.py     # independent recomputation kernel (the "proof core")
├── certificates/        # generated machine-readable certificates (JSON)
└── README.md
```

## Usage

```bash
python witness_prover/prover_top.py --list
python witness_prover/prover_top.py --cert T-e8-spectrum
python witness_prover/prover_top.py --run-all          # writes certificates/*.cert.json
python witness_prover/prover_top.py --run-all --no-engine
```

Optional: the program looks for the native engine
(`mcp_server.exe` in `C:/D:\cube_v5_native\mcp_server\` or `$CUBE_V5_EXE`)
and, if present, attaches the engine's **live witness** to each certificate
as extra supporting material. The verdict is **never** decided by the
engine — only by the kernel.

## Theorems certified (as of this revision)

| ID | Statement | Routes | Kernel |
|----|-----------|--------|--------|
| `T-tau` | τ(x,y)=(x+y)²−4xy−(x−y)²=0 (Pythagorean identity) | primitive/gcd-scaled/commutative | algebraic cancellation + 60² sweep |
| `T-e8-count` | 240 roots = 112 D8 + 128 spinor, ‖r‖²=8 | construct/norm/kind | full regeneration |
| `T-e8-spectrum` | dot spectrum {−8:1,−4:56,0:126,+4:56,+8:1} | enumeration/self-dot/neutral shell | first-principles recount |
| `T-decimal` | 0.1+0.2=0.3, 0.1·0.2=0.02 (exact) | fraction add/mul/div | Fraction arithmetic |
| `T-intpow` | 3¹⁰⁰ = 515…001 (48 digits) | repeated squaring / bigint | exact integer pow |
| `T-complex` | (1+i)¹⁰⁰ = −2⁵⁰, \|3+4i\|=5 | Gaussian squaring / 3-4-5 | exact integers |
| `T-tunnel` | r_i+r_j+r_k=0, dots −4, side²=24, 2240 tunnels | witness (0,6,31) / full enum | identity audit |
| `T-doctor` | 14 channels, OK@≥10, closed@≥7, fracture {C1_Txy,C2_Txz} | signature/gating | count inspection |

## Certificate format (v1)

Each `certificates/*.cert.json` contains:

* `schema`, `certificate`, `statement`, `claims` — human/machine statement;
* `routes[]` — independent witness routes, each with `witness` (bool),
  `detail` (reproducible text), `route_hash` (SHA-256);
* `kernel_recompute` — first-principles recomputation verdict (the decisive
  piece);
* `closure` — `routes_total`, `routes_closed`, `kernel_ok`, `tau`, `verdict`;
* `engine_live_witness` — optional engine output for cross-reference only;
* `certificate_hash` — SHA-256 over statement+routes+kernel;
* `disclaimer` — Opterium is speculative geometry, not peer-reviewed.

Verdict is `CERTIFIED` iff `tau == 0` (all routes closed *and* kernel
recomputed the claim). Any deviation makes the certificate `UNRESOLVED` —
the harness refuses to certify what it cannot re-derive.

## Standards note

**No transcendental functions, no floats, no fitting.** Every kernel step
is exact integer or `Fraction` arithmetic. The certificates are therefore
deterministic, portable, and independently reproducible.

```bash
python witness_prover/prover_top.py --run-all --no-engine   # offline, reproducible
```

> This tool implements Borisov's speculative geometry "Opterium". It is an
> experimental, unorthodox framework: computations are grounded solely in
> explicitly justified, derived definitions — no postulates or axioms are
> admitted, but it is **NOT established
> peer-reviewed mathematics**. Certificates certify internal closure of the
> system, not acceptance by the scientific community.