# Peano Arithmetic: Geometric Representation and Machine-Checkable Certificates

This document consolidates the Peano-related content extracted from `software/witness_prover/README.md`. It contains the formal geometric representation theorem for Peano Arithmetic, the ontology of relational webs, and the associated machine-checkable proof certificates.

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
python software/witness_prover/prover_top.py --run-all --no-engine   # offline, reproducible
```

> This tool implements Borisov's speculative geometry "Opterium". It is an
> experimental, unorthodox framework: computations are grounded solely in
> explicitly justified, derived definitions — no postulates or axioms are
> admitted, but it is **NOT established
> peer-reviewed mathematics**. Certificates certify internal closure of the
> system, not acceptance by the scientific community.
>
> A Geometric and Navigational Ontology of Number with Exact Computational Realization and a Formal Model of Peano Arithmetic

A GEOMETRIC REPRESENTATION THEOREM FOR PEANO ARITHMETIC: RELATIONAL WEBS, DISCRETE FOLIATIONS, AND THE STRUCTURAL REALIZATION OF RECURSIVE INVARIANTS

ABSTRACT

We present a formal geometric representation of first-order Peano arithmetic within the discrete two-dimensional lattice of positive integer pairs. In this framework, a natural number is identified with its complete multiplicative relational web, corresponding to the fiber of the product map over that integer. Standard arithmetic operations, traditionally formalized via primitive recursion on a one-dimensional successor structure, are shown to be isomorphic to coordinate projections along canonical geometric foliations of the lattice. Addition corresponds to the terminal boundary evaluation of anti-diagonal fibers; subtraction and order comparison correspond to boundary collisions along diagonal rays; and multiplication corresponds to the evaluation of discrete enclosed rectangular area. 

We prove a rigorous representation theorem demonstrating that the algebra of relational webs equipped with these spatial evaluation mappings is isomorphic to the standard Peano structure (N_0, 0, S, +, *). This construction does not replace or invalidate the axiomatic foundations of arithmetic; rather, it establishes an exact geometric model in which procedural inductive algorithms are realized as static structural invariants of a discrete manifold. Furthermore, we define an exact computational implementation using a canonical integer triplet (mantissa, debt, orientation) that bypasses continuous floating-point approximations, reducing multi-digit arithmetic to discrete convolutions over a finite base-10 seed.


1. INTRODUCTION AND FOUNDATIONAL SCOPE

The formalization of arithmetic developed by Richard Dedekind and Giuseppe Peano established the natural numbers as a free unary algebra generated by an initial element zero and an injective successor function satisfying the principle of mathematical induction. Within this standard framework, binary operations cannot be introduced simultaneously as primitives; they are defined sequentially through primitive recursion:

a + 0 = a
a + S(b) = S(a + b)

a * 0 = 0
a * S(b) = (a * b) + a

This formulation is mathematically complete, categorical, and optimal for constructive formal logic. By design, it models arithmetic as an inductive, one-dimensional progression. Because the underlying carrier set is ordered linearly, operations that combine two elements must be defined as iterative transitions executed along the sequence of successors.

The purpose of this work is to demonstrate that first-order Peano arithmetic admits an exact, autonomous geometric representation on the discrete two-dimensional lattice N_+ x N_+. When the Cartesian product of two discrete rays is treated as an integrated geometric space, the operations of addition, subtraction, multiplication, and comparison do not need to be generated procedurally through recursive loops. Instead, they pre-exist as static geometric invariants—specifically, as boundary intercepts and areas associated with canonical foliations of the lattice.

To prevent any foundational misunderstanding, our methodological framework is established as follows:
First, we do not claim to construct arithmetic ex nihilo or to replace formal logic. We operate within standard discrete set theory, assuming the existence of the natural numbers as an indexing set.
Second, we do not assert that Peano arithmetic is erroneous or incomplete. We establish a representation theorem: the algebraic structure generated by recursive axioms is strictly isomorphic to the spatial structure generated by the foliations of the discrete lattice.
Third, the value of this geometric realization lies in its structural and computational implications: it replaces temporal recursion with spatial localization, preserves orientation and chirality that are suppressed in scalar projections, and provides an exact, lossless arithmetic engine for discrete computation.


2. THE ONTOLOGY OF RELATIONAL WEBS

In classical set-theoretic arithmetic, an integer n is typically formalized as an ordinal (for instance, the von Neumann ordinal n = {0, 1, ..., n-1}). This formalization treats n as an isolated container of smaller ordinals. 

In contrast, our construction adopts a relational definition analogous to the Yoneda lemma in category theory, where an object is uniquely determined by the network of all morphisms directed into it.

Definition 2.1. For each natural number n in N_0, the multiplicative relational web of n, denoted R_n, is the subset of N_+ x N_+ defined by:
R_n = empty set, if n = 0.
R_n = { (x, y) in N_+ x N_+ : x * y = n }, if n >= 1.

Under this definition, the integer n is identified with the complete set of its positive multiplicative factor pairs. A digit string such as "420" is a compressed routing key; the actual mathematical object is the discrete hyperbolic slice R_420 = { (1, 420), (2, 210), ..., (20, 21), (21, 20), ..., (420, 1) }.

2.1 Remark on the Reading of *

Throughout this document the symbol * occurs in two categorically distinct roles. In the algebraic reading inherited from Peano arithmetic, a * b denotes an operation: two numbers are supplied, a recursive procedure is executed, and a single scalar result is produced. In the geometric reading used here, a * b denotes an address: the cell (a, b) in the lattice N_+ × N_+. The area enclosed at that address is read as a structural property of the cell, not produced by an operation.

The distinction is not terminological. The cells (2, 3) and (3, 2) are different addresses. They enclose the same area, six unit cells, and therefore lie on the same hyperbolic foliation H_6, but they are not the same element of the web R_6. The algebraic identity 2 * 3 = 3 * 2 is a statement about the commutativity of an operation on a one-dimensional result axis. Under the geometric reading, this identity is the statement that the two distinct addresses project onto the same area. Commutativity is a property of the projection, not an identity of the cells.

Whenever * appears in a definition below, it is used in the algebraic sense, as a condition selecting cells by enclosed area. The selected cells are geometric objects. The web R_n is a subset of the lattice, not a value on a line.

Lemma 2.1 (Injectivity of Web Mapping).
The mapping Phi: N_0 -> P(N_+ x N
