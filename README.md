# Opterium — Number ontology and AI software for development

Opterium is a repository with two halves: a **number ontology and theory** equipped with machine-checkable witnesses (`Theory/`), and **AI software for development** (`software/`) — the exact data parser `any_txt_parser` and the MCP gateway `toolfmt` that formats tool outputs to save tokens and gives the agent exact project navigation. How an agent composes these tools into task-specific methods is described in [`ADAPTIVE_METHOD.md`](ADAPTIVE_METHOD.md). The repository is structured for AI consumption first, featuring a reading protocol, claims index, and per-folder AI sections. Correctness is enforced by verification and audit rather than trust.

## Repository map

| Path | What it is | Entry point |
|---|---|---|
| `Theory/ontology_of_number/` | The number ontology: theory, reading guide, claims index | [`README.md`](Theory/ontology_of_number/README.md) — compact reading guide (start here) |
| `Theory/agent-data-circuit/` | Agent Data Circuit (ADC): publisher-side protocol proposal — manifest, schemas, reference server, MCP bridge | [`AI-ENTRY.md`](Theory/agent-data-circuit/AI-ENTRY.md) — cold entry; [`README.md`](Theory/agent-data-circuit/README.md) |
| `software/parsers_for_AI/` | Unified parser `any_txt_parser.exe` (CLI + MCP server) | [`README.md`](software/parsers_for_AI/README.md) — usage; [`software/README.md`](software/README.md) |
| `software/witness_prover/` | Proof-by-witness certificate prover | [`README.md`](software/witness_prover/README.md) — proof model; [`software/README.md`](software/README.md) |
| `software/toolfmt/` | MCP gateway that formats tool outputs and fronts all of an agent's MCP servers (`toolfmt` v3.1.3; Windows x64 binary + Linux x86_64 build — [release asset](https://github.com/UserPC-SPB/Opterium/releases/download/v3.1.3/toolfmt-3.1.3-linux-x86_64.tar.gz)) | [`crates/toolfmt/README.md`](software/toolfmt/crates/toolfmt/README.md) — gateway usage; [`crates/toolfmt/SPEC.md`](software/toolfmt/crates/toolfmt/SPEC.md) — authoritative spec; [`README.md`](software/toolfmt/README.md) — bundled parser docs; [`software/README.md`](software/README.md) |
| `ADAPTIVE_METHOD.md` | Adaptive method architecture: how the agent builds task-specific methods from the available capabilities (toolfmt gateway + any_txt_parser) | — |
| `README.md` | This router | — |

## How to read (AI agents)

1. This file — repository map and context budget.
2. [`ADAPTIVE_METHOD.md`](ADAPTIVE_METHOD.md) — adaptive method architecture: how the agent composes the tools into task-specific methods (read this when your task is about tooling or methods).
3. [`Theory/ontology_of_number/README.md`](Theory/ontology_of_number/README.md) — compact reading guide: surface path (one file) or deep path (protocol order).
4. [`Theory/ontology_of_number/nav_index.md`](Theory/ontology_of_number/nav_index.md) — claims index: every claim with witness and closure.
5. [`Theory/agent-data-circuit/AI-ENTRY.md`](Theory/agent-data-circuit/AI-ENTRY.md) — ADC protocol cold entry (only if your task is about data circuits).
6. The documents themselves, chosen by the context budget below.

Convention: **every section folder has its own README with a `For AI agents` section — read it before the folder's content.** Nested snapshot and test-fixture folders are exempt. No surprises by design.

## Verification

Internally audited 2026-10-21: structure, links, token budgets, artifact/reference claims, and AI metadata checked; all findings fixed. `software/toolfmt/` was added 2026-11-05 (v3.1.2 at the time; the gateway is now v3.1.3) and follows the same conventions.

## For AI agents

### Context budget

| path | lines | ~tokens |
|---|---|---|
| `ADAPTIVE_METHOD.md` | 273 | ~3 200 |
| `Theory/agent-data-circuit/AI-ENTRY.md` | 254 | ~2 600 |
| `Theory/agent-data-circuit/README.md` | 288 | ~3 250 |
| `Theory/agent-data-circuit/SPEC.md` | 1182 | ~12 500 |
| `Theory/ontology_of_number/APPENDIX_MATH.md` | 30 | ~350 |
| `Theory/ontology_of_number/AnchorFile_SuperTable_v1.0.json` | 98 | ~2 600 |
| `Theory/ontology_of_number/Borisov_K_theory.md` | 1777 | ~22 550 |
| `Theory/ontology_of_number/CRITIQUE.md` | 42 | ~900 |
| `Theory/ontology_of_number/New_Peano.md` | 313 | ~2 600 |
| `Theory/ontology_of_number/README.md` | 75 | ~1 000 |
| `Theory/ontology_of_number/helpdoc.py` | 1419 | ~14 200 |
| `Theory/ontology_of_number/independent_verify.py` | 483 | ~5 650 |
| `Theory/ontology_of_number/nav_index.md` | 345 | ~4 050 |
| `Theory/ontology_of_number/opterium_v1.0_spec.txt` | 908 | ~12 850 |
| `Theory/ontology_of_number/outside_AI_opinion.md` | 104 | ~3 850 |
| `Theory/ontology_of_number/readme_supplement.md` | 269 | ~3 250 |
| `software/README.md` | 80 | ~700 |
| `software/parsers_for_AI/README.md` | 143 | ~2 050 |
| `software/toolfmt/README.md` | 163 | ~1 600 |
| `software/toolfmt/crates/toolfmt/README.md` | 153 | ~1 800 |
| `software/toolfmt/crates/toolfmt/SPEC.md` | 711 | ~8 400 |
| `software/witness_prover/README.md` | 125 | ~1 450 |
| `software/witness_prover/cube.exe` | — | — |
| `software/witness_prover/CUBE_REFERENCE.md` | 3151 | ~37 800 |

Parser reference docs are budgeted in `software/README.md` (`HELP.md` ~5 050, `FULL_GUIDE.md` ~7 950). The parser binary (6.4 MB) and `software/witness_prover/cube.exe` (6.6 MB) are not text reading targets and are excluded from the token budget.

### Entry points

- Theory: `Theory/ontology_of_number/README.md` → surface (`outside_AI_opinion.md`) or deep path → `nav_index.md` → docs.
- Tools: `software/README.md` → parser, witness, or toolfmt gateway README (`software/toolfmt/crates/toolfmt/README.md`).

### No surprises

- `Theory/agent-data-circuit/` is a snapshot of github.com/UserPC-SPB/agent-data-circuit (master `ab939aa`) with local link/reference fixes in `rfcs/`, `CONTRIBUTING.md`, and `AGENT-RUNTIME.md`, plus an added `For AI agents` section; its own `LICENSE-CODE` / `LICENSE-DOCS` apply inside. No nested `.git`.
- `_relay/` at repo root is a local agent workspace, gitignored; it may be absent.
- Text files are UTF-8 (many with BOM). Branch: `v3.1-clean`.
- The parser is one unified executable: `software/parsers_for_AI/any_txt_parser.exe` (CLI + MCP); navguard is compiled into it, so there is no separate `navguard.exe`.
- `software/toolfmt/` bundles the parser source as its workspace root crate, so it builds standalone; the shipped `toolfmt.exe` hash is in `software/toolfmt/SHA256SUMS.txt`.
