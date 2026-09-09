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

Abstract

We present an ontology of number in which a number is identified with a complete multiplicative relational web together with a navigational address in the multiplication table regarded as a geometric space. In this ontology arithmetic operations do not exist as an independent class of computational procedures. What is conventionally called addition, multiplication or division is realized exclusively as the act of reading a pre-existing geometric invariant (sum coordinate S, product field P, difference coordinate D, or an equivalent navigational path) from a definite address in the table. The zero element is the unique empty web, characterized by the total absence of positive multiplicative witnesses. An exact machine representation is given in which every number is stored as a triple consisting of a non-negative integer mantissa, an integer debt (cursor position) and a phase in {0,1,2,3} that encodes orientation. Floating-point representation is excluded by construction at every level. All arithmetic is performed by integer navigation and cursor movement; scaling by powers of ten is a pure debt shift that never alters digits.

The resulting structure is fully compatible with the theorems of classical Peano arithmetic: the mapping that sends each standard natural number to its relational web is an isomorphism of structures. This isomorphism is proved as a formal theorem. The construction therefore supplies a faithful geometric realization and an exact computational layer, while asserting that the primary ontological status belongs to the relational-navigational field rather than to a separate class of arithmetic operations. The model does not replace the Peano axioms; it provides a rigorous geometric representation of them and demonstrates that the natural numbers can be realized as pure navigational reads on the multiplication table.

Scope and Methodological Position

This work is written inside ordinary mathematics in the following precise sense. We assume the existence of the set of natural numbers (including zero) and of the standard operations of addition and multiplication on that set, as given by any of the usual constructions (Peano axioms, set-theoretic constructions, or the arithmetic of a host programming language with arbitrary-precision integers). We do not claim to derive these from a more primitive formal system.

What we claim is an ontological and representational priority: once the multiplication table is regarded as a geometric space of addresses, every classical arithmetic operation can be recovered purely as navigation and reading of already present fields. In that sense arithmetic as an autonomous class of generative procedures is absent; only navigation remains. The classical operations appear as convenient projections or wrappers over this navigational activity. The machine implementation described below makes the claim concrete by banning floating-point representation entirely and by realizing every operation as exact integer navigation on mantissa, debt and phase.

Relational Webs

For each natural number n (including zero) we define its relational web R_n as follows.

If n equals zero, R_n is the empty set.
If n is positive, R_n is the set of all ordered pairs (x, y) of positive natural numbers such that the standard product x multiplied by y equals n.

The decimal (or any base) digit string is treated solely as a compressed routing key that points to this web. The web itself is the mathematical object.

Lemma. The mapping that sends each natural number n to the web R_n is injective.

Proof. Suppose R_n equals R_m. If both are empty then n and m are both zero. If they are non-empty then the pair (1, n) belongs to R_n and therefore also to R_m, which forces n equal to m.

The zero web is characterized by the complete absence of positive factor pairs. This is a direct consequence of the standard fact that the product of two positive natural numbers is never zero. The characterization is internal to the assumed framework; it is not offered as an independent existence proof of zero.

The Multiplication Table as Navigational Space

Consider the one-dimensional sequence of natural numbers along an axis. This sequence exists prior to any notion of multiplication: it is a simple order of discrete steps. Any point on this axis is an address x.

Now take a second, independent copy of this same sequence and stack the axes parallel to one another. The stack of axes is a two-dimensional plane. The number of axes in the stack is formalized as y, and this stacking is what gives the second dimension its orthogonal relation to the first: the axes are perpendicular because each axis in the stack is a parallel copy of the original line, and the enumeration of copies creates a direction distinct from the original steps. A point in this plane is an address of the form x:y.

The product P at an address x:y is defined as the total number of unit squares enclosed by the rectangle whose sides are the intervals from the origin to x along one axis and from the origin to y along the other axis. This total is obtained by counting unit cells along the grid, which is a sequential process of accumulation. It does not require any prior multiplication. For example, the address 5:5 encloses a square consisting of twenty-five unit cells. The square root of twenty-five is therefore not the number five but the address 5:5, because the geometric object is a square of side five, and its area is the sum of all cells within that square.

Similarly, stacking two-dimensional planes gives a three-dimensional space. The number of planes in the stack is formalized as z, and an address in three dimensions has the form x:y:z. The value at such an address is the total number of unit cubes enclosed by the corresponding box. For example, the address 2:2:2 encloses a cube consisting of eight unit cubes. The volume is the sum of all unit cubes within that cube.

The address 2:3 in the two-dimensional plane and the address 2:3:1 in the three-dimensional space are the same two-dimensional address; the third coordinate is a plane indicator that selects which layer of the stack is being referenced. The two-dimensional address is unchanged; only the plane index differs. Thus, the dimension of an address is a pointer to a layer in a stack, not a new kind of coordinate.

Because these structures arise entirely from the sequential order and the stacking operation, the multiplication table is not constructed or computed. It is an inevitable geometric record of all possible addresses and their associated areas (or volumes in higher dimensions). The table is the set of all addresses together with three derived invariants that can be read at each address:

S is the sum of the two coordinates: S = x + y. This is the total number of steps from the origin to the point (x, y) along the anti-diagonal path.

D is the difference of the two coordinates: D = x - y. This is the signed distance from the main diagonal.

P is the area enclosed by the rectangle from the origin to (x, y): P = the number of unit squares inside that rectangle.

These three invariants are linked by the identity

P = (S squared minus D squared) divided by 4,

which holds for every address. This identity is not a theorem about operations; it is a geometric fact about rectangles and squares. It says that the area of a rectangle can be expressed in terms of the lengths of its sides' sum and difference.

The natural curves on this space are the hyperbola of constant product (all addresses with the same area), the anti-diagonal of constant sum (all addresses with the same total step count), and the diagonal of constant difference (all addresses with the same signed distance from the main diagonal). Navigation consists in moving along these curves or reading the invariant directly from the address.

No generative computation is required to produce these values; they are pre-existing geometric properties of the address. The table is not a lookup table of precomputed products; it is the geometric record of areas and volumes generated by the underlying sequences and stacks. This eliminates any circularity: the table does not presuppose multiplication; it presupposes only the existence of sequences and their Cartesian stacking.

Operations as Navigation

Zero is the empty web R_0.

The successor of the web R_n is the web R_{n+1}, which corresponds geometrically to the next hyperbola of product n+1.

The sum of the webs R_a and R_b is the web R_{a+b}. The integer a+b is obtained by reading the endpoint of the anti-diagonal S-path that begins at the cell (a, b).

The product of the webs R_a and R_b is the web R_{a multiplied by b}. The integer a multiplied by b is obtained by reading the field P at the cell (a, b).

In each case the operation consists solely in locating an address and reading a coordinate or a field that already exists in the table. There is no separate arithmetic engine that produces a new quantity; there is only navigation.

These definitions are not mere restatements of the classical operations. They replace the recursive equations of Peano arithmetic with direct table lookups: instead of defining addition recursively as a + 0 = a and a + S(b) = S(a + b), we read the sum from the anti-diagonal; instead of defining multiplication recursively as a * 0 = 0 and a * S(b) = a * b + a, we read the product from the P field. The existence and correctness of these reads are certified by the structural identity of the table.

Isomorphism with the Standard Peano Structure

We now establish that the relational structure defined above is a faithful model of Peano arithmetic. Let N_0 denote the set of natural numbers including zero, with the usual successor function S(n) = n+1, addition +, and multiplication *. Let R denote the set of all relational webs R_n for n in N_0, equipped with the operations zero, successor, addition, and multiplication defined in the previous section.

Theorem. The mapping phi from N_0 to R that sends each natural number n to the web R_n is an isomorphism of structures.

Proof.

Injectivity of phi follows from the lemma of the previous section. Surjectivity is immediate from the definition of the carrier set R. The homomorphism properties hold by construction:

phi(0) = R_0, which is the zero element of R.
phi(S(n)) = phi(n+1) = R_{n+1}, which is the successor of R_n.
phi(a + b) = R_{a+b}, which is the sum of R_a and R_b.
phi(a * b) = R_{a*b}, which is the product of R_a and R_b.

Thus phi preserves zero, successor, addition, and multiplication. Hence phi is an isomorphism.

Corollary. The relational structure R satisfies all Peano axioms, because it is isomorphic to the standard model N_0.

Remark. This is a representation theorem, not a foundational derivation. The proof of the isomorphism uses the standard induction principle on N_0. We do not claim to eliminate the Peano axioms; we claim to give a faithful geometric realization of them. The novelty lies in the fact that the operations are not defined by recursive equations but by navigational reads, and this alternative definition yields a structure that is provably equivalent to the classical one.

Exact Machine Representation

An exact computational realization is obtained by representing every number as a triple (mantissa, debt, phase), where

mantissa is a non-negative integer containing the significant digits,
debt is an integer that records the position of the decimal cursor relative to the mantissa,
phase is an integer in the set {0, 1, 2, 3} that encodes orientation (0 for positive real, 1 for positive imaginary, 2 for negative real, 3 for negative imaginary).

The decimal point and the conventional minus sign are excluded from the internal storage format. Scaling by a power of ten is realized exclusively as a change of the debt; the digits of the mantissa remain untouched. Addition and subtraction are performed by aligning debts through the appending of zeros and then adding the resulting integers. Multiplication multiplies mantissas, adds debts, and adds phases modulo 4. Division is realized as a structural descent that produces successive digits of the quotient by repeated scaling of the dividend; an exactness flag records whether the descent terminates.

Floating-point types are rejected at the construction boundary. All internal arithmetic uses only arbitrary-precision integers. Consequently the classical floating-point anomaly in which one tenth plus two tenths fails to equal three tenths cannot occur. The representation is canonical: trailing zeros of the mantissa are absorbed into the debt, and the zero value is normalized to mantissa zero, debt zero, phase zero.

For natural numbers, the representation is simply (n, 0, 0), and the operations defined in the previous section coincide exactly with integer arithmetic on the mantissa, with the debt and phase remaining fixed at zero. Thus the machine implementation is a direct embodiment of the navigational reads.

For numbers with multiple decimal places, navigation is performed componentwise: the address (X, Y) is decomposed into its digits, and operations such as addition, multiplication, and the like are executed as compositions of table reads for each pair of digits, with positional shifts taken into account. This corresponds exactly to the standard algorithms, but without the use of arithmetic operations over large numbers — all intermediate results are read from the 0..10 table and scaled via the debt. In this model, any classical arithmetic expression can be transformed into a finite sequence of navigational reads from a fixed finite set of tabular data. Thus, computational complexity is transferred to routing complexity rather than computational complexity. This does not change the class of computable functions, but it provides a different implementation that is free from rounding errors and does not require recursive definitions.

Certification of the Implementation

A machine-checkable certification layer verifies the fundamental navigational identities and the exactness of the representation by means of multiple independent computational routes together with a kernel that re-derives each claim from first principles using only exact integer arithmetic. A claim is accepted only when every route agrees and the kernel reproduces the same result. The certificates confirm the internal consistency of the geometric identities and of the machine representation; they do not constitute formal proofs of the Peano axioms in a system such as Lean or Coq.

Relevant certificates include the Pythagorean identity, exact decimal arithmetic such as 0.1 + 0.2 = 0.3, and exact integer powers and complex arithmetic. These certificates verify the implementation and the geometric identities used by the model, not the Peano axioms themselves.

Ontological Reading

Within the framework developed here a number is an addressable relational web together with a navigational location in the multiplication table. What is ordinarily called an arithmetic operation is the act of reading a coordinate or a field at that location. Arithmetic as an autonomous generative class is therefore absent; only the navigational field remains. Classical arithmetic appears as a projection that recovers the same numerical values by different means.

The zero element receives a correspondingly direct characterization: it is the unique empty web. This characterization is possible only after the relational space has been adopted as primary. It is offered as an ontological reading, not as a formal replacement of the classical existence axiom.

Conclusion

We have described a geometric space whose points are multiplicative relational webs and whose natural operations are pure acts of navigation and reading. The space is isomorphic to the standard natural numbers and therefore validates every theorem of classical arithmetic. At the same time it supplies an exact machine representation that excludes floating-point error by construction and realizes every operation as integer navigation on mantissa, debt and phase.

The principal claims are therefore two. First, a transparent geometric realization of the natural numbers is possible in which arithmetic operations appear solely as navigational reads. Second, an exact computational layer exists that embodies this realization without residual floating-point discrepancy. The classical Peano structure remains the formal reference; the relational-navigational description is a faithful and exact embodiment of it in which the ontological priority is assigned to the addressable field rather than to a separate class of arithmetic procedures. The formal proof of isomorphism establishes that this embodiment is mathematically rigorous.

Closing illustration. The sum of three and four is obtained by reading the endpoint of the anti-diagonal path that begins at the cell (3, 4) and terminates at the address whose coordinates are (7, 0). The product of six and seven is the field P at the cell (6, 7). The quotient of forty-two by six is the complementary coordinate of the factor pair (6, 7) on the hyperbola of product forty-two. The zero web contains no positive factor pair. In each case the result is a pure navigational read, and independent routes that recover the same address agree.
