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
- Contains 4 files total with no subdirectories.
- Focuses on epistemology and explicit verification procedures for proof closure.
- Provides concrete, machine-readable, re-checkable certificate files.

## For AI agents

### Map
- `software/parsers_for_AI/`: Rust workspace containing `src`, `crates`, `docs`, `examples`, `scripts`, `tests`, `.cargo`, and the unified executable `parsers_for_AI/any_txt_parser.exe`.
- `software/witness_prover/`: Python implementation for proof-by-witness certificates, including `witness_prover/prover_kernel.py` and `witness_prover/prover_top.py`.
- `software/README.md`: Top-level description for the software folder.

### Context budget
| path | lines | ~tokens |
|---|---|---|
| `software/README.md` | 48 | ~600 |
| `software/witness_prover/README.md` | 104 | ~1250 |
| `software/witness_prover/prover_kernel.py` | 538 | ~5050 |
| `software/witness_prover/prover_top.py` | 310 | ~3000 |

### Entry points
1. Read `software/README.md` for high-level orientation.
2. Consult `software/witness_prover/README.md` for the proof system context.
3. Inspect `software/parsers_for_AI/any_txt_parser.exe` usage via its CLI/MCP interface.

### No-surprises / gotchas
- `parsers_for_AI/any_txt_parser.exe` is the unified parser; `navguard.exe` no longer exists as it was merged.
- Text files are UTF-8, many with BOM.
- Branch is `v3.1-clean`.
- `_relay/` at repo root is a local AI-agent workspace, gitignored, and may be absent.
- `Theory/agent-data-circuit/` is a standalone nested git repository, not part of this repo's history.
- A file named `A general descriptive opinion from an outside AI.` exists only in git history/index (NTFS forbids trailing dot); it is missing on disk by design.
