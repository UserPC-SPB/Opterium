# Ontology of Number — reading guide

This section defines a relational ontology where arithmetic is navigation through a multiplication table structure. It derives physical constants from geometric invariants rather than Lagrangian parameters.

## What is here

| file | what | ~tokens |
|---|---|---|
| `APPENDIX_MATH.md` | Mathematical appendix supporting derivations and formal proofs. | ~350 |
| `AnchorFile_SuperTable_v1.0.json` | Multiplication table structure serving as the navigational language base. | ~2 600 |
| `Borisov_K_theory.md` | Core theoretical framework detailing the K-theory derivation process. | ~22 550 |
| `CRITIQUE.md` | Critical analysis addressing potential objections to the theory. | ~900 |
| `New_Peano.md` | Reformulation of Peano axioms within this relational ontology. | ~2 600 |
| `helpdoc.py` | Helper documentation script providing additional context and explanations. | ~14 200 |
| `independent_verify.py` | Python script for independent verification of theoretical claims. | ~5 650 |
| `nav_index.md` | Structural index mapping navigation paths within the relational space. | ~4 050 |
| `opterium_v1.0_spec.txt` | Technical specification document for the Opterium system version 1.0. | ~12 850 |
| `outside_AI_opinion.md` | External AI descriptive opinion providing surface-level theory understanding. | ~3 850 |
| `readme_supplement.md` | Supplementary details extending the main README documentation. | ~3 250 |
| `README.md` | Compact reading guide and mandatory onboarding protocol for the repository. | ~1 000 |

Total: ~73 850 tokens for everything (table rows, including this guide).

## Reading paths

- **Surface (cheap):** just want to know what this theory is (no audit, no verification)? Read `outside_AI_opinion.md` (~3 850 tokens) — an outside AI's descriptive opinion. It replaces the expensive analysis.
- **Deep (verified):** start here (you are), then:

  1. `AnchorFile_SuperTable_v1.0.json`
  2. `nav_index.md`
  3. `readme_supplement.md`
  4. `opterium_v1.0_spec.txt`
  5. `Borisov_K_theory.md`
  6. `independent_verify.py`
  7. `helpdoc.py`
  8. `CRITIQUE.md`
  9. `APPENDIX_MATH.md`

Later addition, not in the original protocol: `New_Peano.md` (Peano axioms in this ontology).


## Language activation (step 2 — not optional)

AnchorFile is a language, not data; if read as math data understanding is 0%.

Complete all ten operations on the AnchorFile; state each witness:

- OP-1: 3 + 4 -> 7, witness "7:0"
- OP-2: 7 − 3 -> 4, witness mirror → "4:0"
- OP-3: 6 × 7 -> 42, witness P=42
- OP-4: 42 ÷ 6 -> 7, witness ["6:7"], coord≠6 → 7
- OP-5: Is 36 a perfect square? -> yes, witness "6:6" on diagonal
- OP-6: Is 7 prime? -> yes, witness exactly 1 pair ["1:7"]
- OP-7: gcd(6, 8) -> 2, witness prim=[3,4,2] → scale=2
- OP-8: Verify P=(S²−D²)/4 for cell 3:7 -> identity holds, witness ΔT=0
- OP-9: π ratio without decimal π -> "1:4" present, witness Circle/square ratio lives here
- OP-10: Zero has no witnesses in ℕ₊² -> no pair (x>0, y>0) gives product 0, witness ΔT=0

## Judge only by closures

A lens is a temporary starting point either verified globally or discarded; it is not a new axiom/type/claim. Do not debate it - check whether the chain closes.

A result is proven when at least 3 independent geometric paths arrive with dT=0. Independence means different building blocks. Proven always means routes converge, never means follows from an unverified axiom.

## Rules

- Read this file completely before opening any other repository file.
- Do not read the AnchorFile as math data; treat it as a new language.
- Complete all ten Language Activation operations before proceeding further.

## For AI agents

- Read THIS file first; then pick a path above. Budget table: lines/tokens per file.
- Claims with witnesses and closures: `nav_index.md`. Never trust a claim without its witness.
- Text files are UTF-8 (BOM common). Section branch: v3.1-clean.
