# Software

Programs kept in this repository, apart from the theory files in `Theory/`.

## parsers_for_AI

- Path: `parsers_for_AI/`
- A Rust workspace (edition 2024) for exact data extraction and project navigation for AI agents.
- Ships as one executable, `any_txt_parser.exe` (v1.5.0).
- The workspace has two crates: `any_txt_parser` (v1.5.0, binary + library) and `navguard` (v0.1.0, library); `navguard` is compiled into the same executable — there is no separate `navguard.exe`.
- Key subfolders: `crates`, `docs`, `examples`, `scripts`, `src`, `tests`.
- Binary: 6.4 MB, Windows x64, no installer or runtime.
- Works standalone via CLI (`-i input -t <template.yml> -o <log.md>`) and as an MCP server.

## witness_prover

- Path: `witness_prover/`
- Implements machine-checkable proof certificates for Opterium.
- Contains 3 files (`cube.exe`, `README.md`, `CUBE_REFERENCE.md`) and no subdirectories.
- Focuses on epistemology and explicit verification procedures for proof closure.
- Provides concrete, machine-readable, re-checkable certificate files.

## toolfmt

- Path: `toolfmt/`
- A Rust workspace (edition 2024) that formats MCP tool outputs and acts as an
  MCP gateway: one `toolfmt.exe` (v3.1.3) between an agent and all of its MCP
  servers. Gateway documentation: `crates/toolfmt/README.md` (usage) and
  `crates/toolfmt/SPEC.md` (authoritative spec); the folder-level `README.md`
  documents the bundled `any_txt_parser` root crate.
- **Requires `any_txt_parser`** — it is used as a library for `template`
  rules. The parser lives in `../parsers_for_AI/` and is also bundled in this
  workspace (the root crate), so the tree builds standalone.
- Workspace crates: `any_txt_parser` (root, bundled), `navguard`, `toolfmt`
  (the formatter/gateway binary).
- Key subfolders: `crates`, `docs`, `examples`, `scripts`, `src`, `tests`.
- Binaries: Windows x64 (~4.8 MB; 4,834,304 B) in this folder, no installer or runtime,
  SHA256 in `SHA256SUMS.txt`; a Linux x86_64 build is published as the
  release asset [`toolfmt-3.1.3-linux-x86_64.tar.gz`](https://github.com/UserPC-SPB/Opterium/releases/download/v3.1.3/toolfmt-3.1.3-linux-x86_64.tar.gz)
  (release `v3.1.3`).
- Register it in the MCP client instead of the upstream servers:
  `toolfmt.exe gateway --home <home>` with `upstreams.json` in the home.

## For AI agents

### Map
- `software/parsers_for_AI/`: Rust workspace containing `src`, `crates`, `docs`, `examples`, `scripts`, `tests`, `.cargo`, and the unified executable `software/parsers_for_AI/any_txt_parser.exe`.
- `software/parsers_for_AI/README.md`: AI-first usage guide; `HELP.md` is the CLI/template reference and `FULL_GUIDE.md` the long guide.
- `software/witness_prover/`: proof-by-witness certificates provided by `software/witness_prover/cube.exe`; see `software/witness_prover/README.md` and `software/witness_prover/CUBE_REFERENCE.md`.
- `software/toolfmt/`: Rust workspace for tool-output formatting and the MCP gateway; ships `software/toolfmt/toolfmt.exe` (v3.1.3, Windows x64; a Linux x86_64 build is also released); gateway usage in `software/toolfmt/crates/toolfmt/README.md`, authoritative spec in `software/toolfmt/crates/toolfmt/SPEC.md`; the folder `README.md` documents the bundled `any_txt_parser` root crate; requires `any_txt_parser` (`software/parsers_for_AI/`).
- `software/README.md`: Top-level description for the software folder.

### Context budget

| path | lines | ~tokens |
|---|---|---|
| `software/README.md` | 80 | ~700 |
| `software/parsers_for_AI/README.md` | 143 | ~2 050 |
| `software/parsers_for_AI/HELP.md` | 238 | ~5 050 |
| `software/parsers_for_AI/FULL_GUIDE.md` | 626 | ~7 950 |
| `software/witness_prover/README.md` | 125 | ~1 450 |
| `software/toolfmt/README.md` | 163 | ~1 600 |
| `software/toolfmt/crates/toolfmt/README.md` | 153 | ~1 800 |
| `software/toolfmt/crates/toolfmt/SPEC.md` | 711 | ~8 400 |

### Entry points
1. Read this file for high-level orientation.
2. Parser work: `software/parsers_for_AI/README.md` → `HELP.md` for CLI/template details.
3. Proof work: `software/witness_prover/README.md`.
4. Tool-output / MCP gateway work: `software/toolfmt/crates/toolfmt/README.md` (spec: `SPEC.md` in the same folder).

### No-surprises / gotchas
- `software/parsers_for_AI/any_txt_parser.exe` is the only executable; navguard is compiled into it, so there is no `navguard.exe`.
- The parser binary is Windows x64; MCP clients must pass `--root`.
- `software/toolfmt/toolfmt.exe` is a second executable (the gateway); it needs `any_txt_parser` and is built from the bundled workspace in `software/toolfmt/`.
- Text files are UTF-8, many with BOM.
- Branch is `v3.1-clean`.
- `_relay/` at repo root is a local AI-agent workspace, gitignored, and may be absent.
- `Theory/agent-data-circuit/` is a vendored snapshot (no nested `.git`) of `github.com/UserPC-SPB/agent-data-circuit` @ `ab939aa`; its own `LICENSE-CODE` / `LICENSE-DOCS` apply inside.
