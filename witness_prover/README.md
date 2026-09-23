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
The mapping Phi: N_0 -> P(N_+ x N_+) defined by Phi(n) = R_n is strictly injective.

Proof. Let m, n be elements of N_0, and assume R_m = R_n.
If R_m is the empty set, then by Definition 2.1, m = 0. Consequently, R_n is empty, which implies n = 0, so m = n.
If R_m is non-empty, then m >= 1. By elementary arithmetic, 1 * m = m, which implies that the ordered pair (1, m) belongs to R_m.
Because R_m = R_n, the pair (1, m) must also belong to R_n.
By Definition 2.1, any pair (x, y) in R_n satisfies x * y = n. Therefore, 1 * m = n, which directly yields m = n.
Thus, Phi is injective.

Corollary 2.1. The zero element is the unique natural number whose multiplicative web contains zero positive witnesses:
R_0 intersection (N_+ x N_+) = empty set.

This characterization of zero is purely structural. It does not define zero as an ad hoc additive identity; it identifies zero as the unique vacuum state of the multiplicative domain, having no relational factorization among positive integers.

Corollary 2.2. The identity element 1 is characterized by the singleton web R_1 = { (1, 1) }, representing the unique minimal self-dual fixed point. A number p > 1 is prime if and only if its web has cardinality exactly 2, namely R_p = { (1, p), (p, 1) }.


3. THE DISCRETE MANIFOLD AND ITS CANONICAL FOLIATIONS

Let the discrete Cartesian product N_+ x N_+ be regarded as a two-dimensional grid of unit cells. Every cell is referenced by an address (x, y).

Geometrically, the product field P at address (x, y) is defined as the discrete area of the rectangle spanned from the origin (0, 0) to (x, y), measured by the total count of elementary unit squares enclosed:
P(x, y) = count of unit cells in [0, x] x [0, y].

The grid admits three natural geometric foliations, each corresponding to an equivalence relation on N_+ x N_+:

Foliation 1: The Anti-Diagonal Foliation (Constant Sum S).
For each integer S >= 2, the level set L_S^+ is defined by:
L_S^+ = { (x, y) in N_+ x N_+ : x + y = S }.
This is a straight discrete segment oriented at an angle of negative 45 degrees to the horizontal axis. It contains exactly S - 1 cells. When extended to the boundary of the positive quadrant, this segment terminates at the boundary point (S, 0) on the horizontal axis and (0, S) on the vertical axis.

Foliation 2: The Diagonal Foliation (Constant Difference D).
For each integer D in Z, the level set L_D^- is defined by:
L_D^- = { (x, y) in N_+ x N_+ : x - y = D }.
These are discrete rays parallel to the principal diagonal x = y. 
The ray D = 0 contains all symmetric square nodes (x, x).
For D > 0, the ray terminates on the horizontal boundary at (D, 0).
For D < 0, the ray terminates on the vertical boundary at (0, |D|).

Foliation 3: The Hyperbolic Foliation (Constant Area P).
For each integer P >= 1, the level set H_P is defined by:
H_P = { (x, y) in N_+ x N_+ : x * y = P } = R_P.
These level sets form discrete hyperbolas symmetric with respect to the principal diagonal D = 0.


4. THE POLARIZATION IDENTITY AS GEOMETRIC DECOMPOSITION

In continuous algebra, the product of two numbers can be expressed in terms of quadratic forms via polarization. On the discrete lattice N_+ x N_+, this identity takes an exact geometric form.

Theorem 4.1 (Discrete Metric Balance).
For every cell (x, y) in N_+ x N_+, let S = x + y, D = x - y, and P = x * y. Then:
P = (S^2 - D^2) / 4.

Proof. Direct substitution over the ring of integers:
(S^2 - D^2) / 4 = ((x + y)^2 - (x - y)^2) / 4
= ((x^2 + 2xy + y^2) - (x^2 - 2xy + y^2)) / 4
= (4xy) / 4 = xy = P.

This relation is not a numerical trick; it is a structural decomposition theorem. It demonstrates that the area of any rectangle on the lattice is identically equal to the difference between the square constructed on its semi-sum and the square constructed on its semi-difference. On the discrete manifold, multiplication is the geometric interference between the anti-diagonal foliation S and the diagonal foliation D.


5. KINEMATICS OF OPERATIONS: PROCEDURES AS SPATIAL READS

We now resolve the cognitive transition between the procedural Peano definitions and the spatial navigation of the lattice.

5.1 Addition
In Peano arithmetic, addition a + b requires executing the successor operation b times, starting from a. 
In the discrete lattice, the operation a + b is executed by locating the node (a, b) and following its anti-diagonal leaf L_{a+b}^+ to its horizontal boundary intercept.
The leaf L_{a+b}^+ consists of all pairs whose coordinates sum to a + b. Tracing this line downward and to the right brings the path to the boundary coordinate (a + b, 0). The value a + b is not generated by an arithmetic processor; it is read as the spatial coordinate of the boundary intersection of the leaf on which (a, b) resides. The operation is an O(1) coordinate extraction.

5.2 Subtraction and Order Comparison
In classical logic, testing whether a > b requires computing a - b and evaluating the sign of the result.
In the discrete lattice, the difference ray through (a, b) is traversed downward and to the left:
If a = b, the node resides on the central diagonal D = 0; the difference is zero, indicating identity.
If a > b, the ray L_{a-b}^- strikes the horizontal boundary floor at (a - b, 0). The horizontal collision certifies the relation a > b, and the intercept coordinate directly yields the magnitude of the difference.
If a < b, the ray L_{a-b}^- strikes the vertical boundary wall at (0, b - a). The vertical collision certifies the relation a < b, and the intercept coordinate directly yields the magnitude of the deficit.
Order comparison is therefore not an arithmetic subtraction; it is a physical collision test against the orthogonal boundaries of the quadrant.

5.3 Multiplication and Division
Multiplication of a and b is the spatial extraction of the enclosed area P at the intersection cell (a, b).
Division of an integer P by a divisor d corresponds to navigating to the hyperbolic leaf H_P = R_P and locating the cell whose coordinate along one axis is d. The quotient is the coordinate along the orthogonal axis. If no integer cell exists at coordinate d on the leaf H_P, the division does not close in N_+; the remainder represents the geometric gap between H_P and the nearest sub-hyperbola containing d.


6. DISCRETE HALVING AND THE NECESSITY OF ASYMMETRY

The discrete nature of the lattice imposes a fundamental topological constraint on the partition of sums.

Definition 6.1. The discrete halving operator H: N_+ -> N_+ x N_+ partitions an integer S into its most balanced integer pair:
H(S) = ( floor(S / 2), ceil(S / 2) ).

Theorem 6.1 (The Asymmetry Theorem).
Let (x, y) = H(S). 
1. If S is even (S = 2k), then x = y = k, and D = x - y = 0. The node lies on the diagonal ridge of symmetry.
2. If S is odd (S = 2k + 1), then x = k, y = k + 1, and D = x - y = -1. The node cannot lie on the diagonal ridge.

Proof. If S = 2k + 1, then floor(S/2) = k and ceil(S/2) = k + 1. The difference is k - (k + 1) = -1. For any integer pair (u, v) such that u + v = 2k + 1, the difference u - v = (2k + 1) - 2v is an odd integer. Because 0 is even, u - v cannot equal 0 for any integer choice of u and v.

This theorem establishes that on a discrete lattice, exact bilateral symmetry (D = 0) is forbidden for odd sums. When an odd sum is partitioned with maximum efficiency (maximizing enclosed area), it must release an irreducible difference quantum: |D| = 1.
This structural asymmetry breaks mirror parity. The cell (k, k+1) collides with the vertical wall, whereas its reflection (k+1, k) collides with the horizontal floor. The halving of odd numbers forces an intrinsic chirality into the discrete space, establishing a preferred directional orientation that cannot be eliminated by coordinate transformation.


7. THE REPRESENTATION ISOMORPHISM

We now formalize the equivalence between the Peano structure and the relational geometric manifold.

Let N_0 denote the standard natural numbers with zero, equipped with the successor function S_N, addition +_N, and multiplication *_N.
Let R denote the set of all relational webs R_n for n in N_0.

We define autonomous geometric operations on R without using arithmetic operations in their definitions:
1. Zero Element: R_0 = empty set.
2. Web Successor S_R: For any web W in R, let x be the unique positive integer such that (1, x) in W (if W is non-empty), and 0 if W is empty. The successor S_R(W) is defined as the web occupying the next adjacent hyperbola R_{x+1}.
3. Web Addition (+_R): For non-empty webs A, B in R, extract their scalar addresses a and b via their (1, a) and (1, b) witnesses. Navigate to cell (a, b) in the lattice. Trace the anti-diagonal foliation through (a, b) to its horizontal boundary intercept (s*, 0). Define A +_R B = R_{s*}. If either web is R_0, the addition returns the other web.
4. Web Multiplication (*_R): For non-empty webs A, B in R, extract scalar addresses a and b. Navigate to cell (a, b). Read the enclosed unit-cell area P*. Define A *_R B = R_{P*}. If either web is R_0, the product returns R_0.

Theorem 7.1 (Isomorphism of Peano and Relational Structures).
The mapping Phi: N_0 -> R defined by Phi(n) = R_n is an isomorphism of algebraic systems:
Phi: (N_0, 0, S_N, +_N, *_N) -> (R, R_0, S_R, +_R, *_R).

Proof.
First, bijectivity of Phi is established by Lemma 2.1 (injectivity) and the definition of R as the image of N_0 under Phi (surjectivity).
Second, preservation of zero holds by definition: Phi(0) = R_0.
Third, preservation of the successor:
Phi(S_N(n)) = Phi(n + 1) = R_{n+1}.
S_R(Phi(n)) = S_R(R_n). By Definition of S_R, the scalar witness of R_n is n, so S_R(R_n) = R_{n+1}.
Thus, Phi(S_N(n)) = S_R(Phi(n)).
Fourth, preservation of addition:
For a, b >= 1, the anti-diagonal line L through (a, b) satisfies x + y = a +_N b for all points on L. The intersection of L with the line y = 0 is the unique point (a +_N b, 0). Thus, the extracted boundary coordinate is s* = a +_N b, which yields:
Phi(a) +_R Phi(b) = R_a +_R R_b = R_{s*} = R_{a +_N b} = Phi(a +_N b).
If a = 0 or b = 0, preservation holds trivially by definition.
Fifth, preservation of multiplication:
For a, b >= 1, the cell (a, b) encloses a discrete rectangular grid of width a and height b. The total number of unit cells enclosed is a *_N b. Thus, the extracted area is P* = a *_N b, which yields:
Phi(a) *_R Phi(b) = R_a *_R R_b = R_{P*} = R_{a *_N b} = Phi(a *_N b).
If a = 0 or b = 0, the product yields R_0 = Phi(0) by definition.

Since Phi is a bijective homomorphism across all primitives and operations of the signature, Phi is an isomorphism of structures.

Corollary 7.1. Every first-order theorem provable in Peano arithmetic is a valid structural theorem concerning the intersections, boundary collisions, and enclosed areas of the discrete foliations on N_+ x N_+.


8. MACHINE REPRESENTATION: DEBT-PHASE ARITHMETIC

The realization of numbers as relational coordinates enables a machine implementation that eliminates the rounding errors and representation anomalies inherent in standard floating-point architectures (such as IEEE-754).

Definition 8.1. An exact number is represented as a canonical triplet:
X = (M, d, o) in N_0 x Z x Z_4,
where:
M is an arbitrary-precision non-negative integer mantissa representing the active excitation pattern, with trailing zeros absorbed into debt for M > 0.
d is an integer cursor debt representing positional scale by 10^d.
o is an orientation phase in Z_4 = {0, 1, 2, 3}, encoding directional rotation:
o = 0 corresponds to +1 (real positive),
o = 1 corresponds to +i (imaginary positive),
o = 2 corresponds to -1 (real negative),
o = 3 corresponds to -i (imaginary negative).

The numerical value represented is:
Val(M, d, o) = M * 10^d * i^o.
The zero element is canonically represented as (0, 0, 0).

Operational Rules:
1. Scaling: Multiplication by 10^k is a pure cursor shift:
Scale_k(M, d, o) = (M, d + k, o).
The operation modifies only the metadata debt coordinate d; the digits of M are untouched, incurring an execution cost of O(1).

2. Multiplication:
(M_1, d_1, o_1) * (M_2, d_2, o_2) = (M_1 * M_2, d_1 + d_2, (o_1 + o_2) mod 4).
Mantissas multiply as integers; debts add linearly; orientation phases add modulo 4. The identity i^2 = -1 is realized as the phase transition (1 + 1) mod 4 = 2, replacing algebraic sign rules with modular rotation.

3. Addition:
Given (M_1, d_1, o_1) and (M_2, d_2, o_2) with d_1 >= d_2:
Compute delta_d = d_1 - d_2 >= 0.
Align the debts by appending delta_d zeros to M_1: M_1' = M_1 * 10^{delta_d}.
If o_1 = o_2, the mantissas add directly: (M_1' + M_2, d_2, o_1).
If o_1 and o_2 differ by 2 (opposite signs on the same axis), the operation resolves as an exact integer difference on the mantissas, with the phase assigned according to the dominant term.
Under this system, expressions such as 0.1 + 0.2 evaluate as:
(1, -1, 0) + (2, -1, 0) = (1 + 2, -1, 0) = (3, -1, 0) = 0.3,
with zero floating-point error.

4. Multi-Digit Execution via the Finite 10 x 10 Seed:
Arbitrary-precision arithmetic does not require an infinite physical table. The canonical 10 x 10 table serves as an irreducible seed. Multi-digit operations decompose into the discrete algebraic convolution of single-digit event maps. Each single-digit product is read in O(1) from the seed, while positional carries are absorbed into the debt cursor. The computational burden is shifted from procedural arithmetic recalculation to deterministic address routing.


9. CONCLUSION

The relational geometry of numbers developed here demonstrates that the procedural machinery of Peano arithmetic can be completely mapped to static geometric invariants on a discrete two-dimensional lattice. 

Peano's recursive formulation and Borisov's relational geometry are not competing or contradictory systems; they are isomorphic models of the same mathematical reality. Peano provides the minimal sequential language for inductive proof along a single dimension. Relational geometry provides the spatial landscape in which those sequential operations are seen to be the boundary reads, areas, and collisions of pre-existing geometric foliations. 

By restoring the spatial dimensions of the multiplication table, arithmetic is liberated from the illusion of purely procedural computation, offering both a deeper structural ontology of number and an exact, lossless foundation for machine computation.
