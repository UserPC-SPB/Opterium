# Opterium — Number ontology with machine-checkable witnesses and AI-first structure

Opterium is a repository containing a number ontology and theory equipped with machine-checkable witnesses. It is structured for AI consumption first, featuring a reading protocol, claims index, and per-folder AI sections. Correctness is enforced by verification and audit rather than trust.

## Repository map

| Path | What it is | Entry point |
|---|---|---|
| `Theory/ontology_of_number/` | The number ontology: theory, reading protocol, claims index | [`README.md`](Theory/ontology_of_number/README.md) — AI reading protocol (start here) |
| `Theory/agent-data-circuit/` | STANDALONE nested repository (upstream: github.com/UserPC-SPB/agent-data-circuit) | its own README inside |
| `software/parsers_for_AI/` | Unified parser `any_txt_parser.exe` (CLI + MCP server) | [`software/README.md`](software/README.md) |
| `software/witness_prover/` | Proof-by-witness certificate prover | [`software/README.md`](software/README.md) |
| `AUDIT_2026-10-21.md` | Structural audit record: 34 findings, all fixed | — |
| `README.md` | This router | — |

## How to read (AI agents)

1. This file — repository map and context budget.
2. [`Theory/ontology_of_number/README.md`](Theory/ontology_of_number/README.md) — the reading protocol: what to read, in which order, with token budgets.
3. [`Theory/ontology_of_number/nav_index.md`](Theory/ontology_of_number/nav_index.md) — claims index: every claim with witness and closure.
4. The documents themselves, chosen by the context budget below.

Convention: **every folder has its own README with a `For AI agents` section — read it before the folder's content.** No surprises by design.

## Verification

Structural audit 2026-10-21: 34 findings (stale counts, section pointers, docs-vs-code drift) — all fixed in the two commits that follow it. Record: `AUDIT_2026-10-21.md`.

## For AI agents

### Context budget

| path | lines | ~tokens |
|---|---|---|
| `AUDIT_2026-10-21.md` | 44 | ~3 100 |
| `Theory/ontology_of_number/APPENDIX_MATH.md` | 30 | ~350 |
| `Theory/ontology_of_number/AnchorFile_SuperTable_v1.0.json` | 98 | ~2 600 |
| `Theory/ontology_of_number/Borisov_K_theory.md` | 1777 | ~22 550 |
| `Theory/ontology_of_number/CRITIQUE.md` | 42 | ~900 |
| `Theory/ontology_of_number/New_Peano.md` | 126 | ~2 600 |
| `Theory/ontology_of_number/README.md` | 382 | ~4 750 |
| `Theory/ontology_of_number/helpdoc.py` | 1419 | ~14 200 |
| `Theory/ontology_of_number/independent_verify.py` | 483 | ~5 650 |
| `Theory/ontology_of_number/nav_index.md` | 345 | ~4 050 |
| `Theory/ontology_of_number/opterium_v1.0_spec.txt` | 908 | ~12 850 |
| `Theory/ontology_of_number/readme_supplement.md` | 269 | ~3 250 |
| `software/README.md` | 48 | ~600 |
| `software/parsers_for_AI/any_txt_parser.exe` | 0 | ~1 610 750 |
| `software/witness_prover/README.md` | 104 | ~1 250 |
| `software/witness_prover/prover_kernel.py` | 538 | ~5 050 |
| `software/witness_prover/prover_top.py` | 310 | ~3 000 |

### Entry points

- Theory: `Theory/ontology_of_number/README.md` (protocol) → `nav_index.md` (claims) → docs.
- Tools: `software/README.md`.

### No surprises

- `A general descriptive opinion from an outside AI.` exists ONLY in git history/index (name ends with a dot — NTFS forbids it on disk); read it via `git show HEAD` if needed.
- `Theory/agent-data-circuit/` is a standalone nested git repository, not part of this history.
- `_relay/` at repo root is a local agent workspace, gitignored; it may be absent.
- Text files are UTF-8 (many with BOM). Branch: `v3.1-clean`.
- The parser is one unified executable: `software/parsers_for_AI/any_txt_parser.exe` (CLI + MCP); navguard.exe was merged into it and no longer exists as a file.
