# Software

Programs kept in this repository, apart from the theory files in `Theory/`.

## parsers_for_AI

- Path: `parsers_for_AI/`
- A Rust workspace (edition 2024) for exact data extraction and project navigation for AI agents.
- Contains the `any_txt_parser` crate (v1.5.0) and the `navguard` crate (v0.1.0).
- Key subfolders: `crates`, `docs`, `examples`, `scripts`, `src`, `tests`.
- Executable: `parsers_for_AI/any_txt_parser.exe` (6.4 MB).
- Works standalone via CLI (`-i input -t <template.yml> -o <log.md>`) and as an MCP server.

## witness_prover

- Path: `witness_prover/`
- Implements machine-checkable proof certificates for Opterium.
- Contains 4 files (three visible plus `.gitignore`) and no subdirectories.
- Focuses on epistemology and explicit verification procedures for proof closure.
- Provides concrete, machine-readable, re-checkable certificate files.

## For AI agents

### Map
- `software/parsers_for_AI/`: Rust workspace containing `src`, `crates`, `docs`, `examples`, `scripts`, `tests`, `.cargo`, and the unified executable `parsers_for_AI/any_txt_parser.exe`.
- `software/parsers_for_AI/README.md`: AI-first usage guide; `HELP.md` is the CLI/template reference and `FULL_GUIDE.md` the long guide.
- `software/witness_prover/`: Python implementation for proof-by-witness certificates, including `witness_prover/prover_kernel.py` and `witness_prover/prover_top.py`.
- `software/README.md`: Top-level description for the software folder.

### Context budget

| path | lines | ~tokens |
|---|---|---|
| `software/README.md` | 53 | ~650 |
| `software/parsers_for_AI/README.md` | 143 | ~2 050 |
| `software/parsers_for_AI/HELP.md` | 238 | ~5 050 |
| `software/parsers_for_AI/FULL_GUIDE.md` | 626 | ~7 950 |
| `software/witness_prover/README.md` | 114 | ~1 450 |
| `software/witness_prover/prover_kernel.py` | 538 | ~5 050 |
| `software/witness_prover/prover_top.py` | 310 | ~3 000 |

### Entry points
1. Read this file for high-level orientation.
2. Parser work: `software/parsers_for_AI/README.md` → `HELP.md` for CLI/template details.
3. Proof work: `software/witness_prover/README.md`.

### No-surprises / gotchas
- `parsers_for_AI/any_txt_parser.exe` is the unified parser; `navguard.exe` no longer exists as it was merged.
- The parser binary is Windows x64; MCP clients must pass `--root`.
- Text files are UTF-8, many with BOM.
- Branch is `v3.1-clean`.
- `_relay/` at repo root is a local AI-agent workspace, gitignored, and may be absent.
- `Theory/agent-data-circuit/` is a vendored snapshot (no nested `.git`) of `github.com/UserPC-SPB/agent-data-circuit` @ `ab939aa`; its own `LICENSE-CODE` / `LICENSE-DOCS` apply inside.
