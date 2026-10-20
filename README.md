# Opterium — Number ontology with machine-checkable witnesses and AI-first structure

Opterium is a repository containing a number ontology and theory equipped with machine-checkable witnesses. It is structured for AI consumption first, featuring a reading protocol, claims index, and per-folder AI sections. Correctness is enforced by verification and audit rather than trust.

## Repository map

| Path | What it is | Entry point |
|---|---|---|
| `Theory/ontology_of_number/` | The number ontology: theory, reading guide, claims index | [`README.md`](Theory/ontology_of_number/README.md) — compact reading guide (start here) |
| `Theory/agent-data-circuit/` | Agent Data Circuit (ADC): publisher-side protocol proposal — manifest, schemas, reference server, MCP bridge | [`AI-ENTRY.md`](Theory/agent-data-circuit/AI-ENTRY.md) — cold entry; [`README.md`](Theory/agent-data-circuit/README.md) |
| `software/parsers_for_AI/` | Unified parser `any_txt_parser.exe` (CLI + MCP server) | [`software/README.md`](software/README.md) |
| `software/witness_prover/` | Proof-by-witness certificate prover | [`software/README.md`](software/README.md) |
| `README.md` | This router | — |

## How to read (AI agents)

1. This file — repository map and context budget.
2. [`Theory/ontology_of_number/README.md`](Theory/ontology_of_number/README.md) — compact reading guide: surface path (one file) or deep path (protocol order).
3. [`Theory/ontology_of_number/nav_index.md`](Theory/ontology_of_number/nav_index.md) — claims index: every claim with witness and closure.
4. [`Theory/agent-data-circuit/AI-ENTRY.md`](Theory/agent-data-circuit/AI-ENTRY.md) — ADC protocol cold entry (only if your task is about data circuits).
5. The documents themselves, chosen by the context budget below.

Convention: **every folder has its own README with a `For AI agents` section — read it before the folder's content.** No surprises by design.

## Verification

Internally audited 2026-10-21: 34 structural findings, all fixed in the same round.

## For AI agents

### Context budget

| path | lines | ~tokens |
|---|---|---|
| `Theory/agent-data-circuit/AI-ENTRY.md` | 254 | ~2 600 |
| `Theory/agent-data-circuit/README.md` | 281 | ~3 000 |
| `Theory/agent-data-circuit/SPEC.md` | 1182 | ~12 500 |
| `Theory/ontology_of_number/APPENDIX_MATH.md` | 30 | ~350 |
| `Theory/ontology_of_number/AnchorFile_SuperTable_v1.0.json` | 98 | ~2 600 |
| `Theory/ontology_of_number/Borisov_K_theory.md` | 1777 | ~22 550 |
| `Theory/ontology_of_number/CRITIQUE.md` | 42 | ~900 |
| `Theory/ontology_of_number/New_Peano.md` | 126 | ~2 600 |
| `Theory/ontology_of_number/README.md` | 75 | ~950 |
| `Theory/ontology_of_number/helpdoc.py` | 1419 | ~14 200 |
| `Theory/ontology_of_number/independent_verify.py` | 483 | ~5 650 |
| `Theory/ontology_of_number/nav_index.md` | 345 | ~4 050 |
| `Theory/ontology_of_number/opterium_v1.0_spec.txt` | 908 | ~12 850 |
| `Theory/ontology_of_number/outside_AI_opinion.md` | 104 | ~3 850 |
| `Theory/ontology_of_number/readme_supplement.md` | 269 | ~3 250 |
| `software/README.md` | 48 | ~600 |
| `software/parsers_for_AI/any_txt_parser.exe` | 0 | ~1 610 750 |
| `software/witness_prover/README.md` | 104 | ~1 250 |
| `software/witness_prover/prover_kernel.py` | 538 | ~5 050 |
| `software/witness_prover/prover_top.py` | 310 | ~3 000 |

### Entry points

- Theory: `Theory/ontology_of_number/README.md` → surface (`outside_AI_opinion.md`) or deep path → `nav_index.md` → docs.
- Tools: `software/README.md`.

### No surprises

- `Theory/agent-data-circuit/` is a snapshot of github.com/UserPC-SPB/agent-data-circuit (master `ab939aa`); its own LICENSE-CODE / LICENSE-DOCS apply inside.
- `_relay/` at repo root is a local agent workspace, gitignored; it may be absent.
- Text files are UTF-8 (many with BOM). Branch: `v3.1-clean`.
- The parser is one unified executable: `software/parsers_for_AI/any_txt_parser.exe` (CLI + MCP); navguard.exe was merged into it and no longer exists as a file.
