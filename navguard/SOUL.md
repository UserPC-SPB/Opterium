# navguard v0.1.0 — Project Navigator

Scans a project folder and produces a machine-readable map: file tree, line/token counts, symbol index. Designed for **vibe coding** — AI-assisted development where the model must navigate a codebase without loading every file into context.

> **Version note:** this is v0.1.0. The next release will be v0.0.1 (pre-release numbering). Expect minor API changes until v1.0.0.

## How It Works

navguard scans the **directory where the `.exe` lives** (current working directory). It walks up looking for `Cargo.toml`; if none is found, it scans the exe's own directory. Three files are written:

| File | Content |
|------|---------|
| `navigator.md` | File tree + `(deep L{N})` anchors + triangle state |
| `navigator_deep.md` | Per-file stats (lines, tokens) + deep2 line ranges |
| `navigator_deep2.md` | Symbol index: every function/class/section with line numbers |

A **Triangle** `[A,B,C]` enforces the read-before-use contract:

| State | Who sets it | Meaning |
|-------|-------------|---------|
| `[1,0,0]` | navguard (at scan start) | scan in progress, data invalid |
| `[0,1,0]` | navguard (at scan end) | scan complete, data fresh, ready to read |
| `[0,0,1]` | **AI must set this** (after reading) | data confirmed consumed |

**Rules:**
- navguard sets `[1,0,0]` when it begins scanning, `[0,1,0]` when it finishes.
- The AI **must** read `navigator.md` completely, then edit its `Triangle:` field from `[0,1,0]` to `[0,0,1]`.
- If the triangle is **not** `[0,1,0]`, reading `navigator.md` is forbidden — the data is stale or corrupt.
- Forward only: `[1,0,0]` → `[0,1,0]` → `[0,0,1]` → new cycle. No backward transitions.

This guarantees that the next AI can check the triangle and know whether the previous AI actually consumed the tree or merely triggered navguard and ignored the output.

## File Hierarchy

The three navigator files form a point-access chain:

```
navigator.md        ← file tree + (deep L{N}) anchors + triangle state
    ↓
navigator_deep.md   ← per-file stats (lines, tokens) + deep2 line ranges
    ↓
navigator_deep2.md  ← symbol index (function/class/section → line numbers)
```

The AI always starts at `navigator.md`, reads it fully, sets `[0,0,1]`, then drills into deeper files only for specific symbols. Never load the deep files into context unless you need their contents.

## Commands

| Command | What it does |
|---------|-------------|
| `navguard` (no args) | Double-click mode: full scan, wait for key |
| `navguard --check` | Full report (T/F/B), writes all 3 navigator files |
| `navguard --version` | Prints version (`navguard v0.1.0`) |

## Output Format (--check)

```
T: 0,1,0              ← triangle state after scan
F: file1.py|120|1500  ← each file: path, lines, tokens
B: sync|2026-07-06T…  ← last scan timestamp
B: rules|none         ← active rules
```

## Why It's Cheaper

Without navguard, an AI must guess the project structure or load entire files into context. A typical codebase has hundreds of files; loading them all wastes context tokens and slows reasoning.

With navguard, the AI reads a tiny tree file (~2 KB), finds symbols by name, and reads exactly the needed lines — no waste. The three-file chain is designed for **point access**:

1. `navigator.md` — see what files exist (tree only, no metrics)
2. `navigator_deep.md` — get stats for a specific file (lines, tokens, deep2 range)
3. `navigator_deep2.md` — see functions/symbols inside that file (line numbers only)

Each level adds only new information; paths are never repeated. This keeps total transferred data under 12 KB for a typical project, versus hundreds of KB if you load full file contents.

## Example Usage

```bash
# Full scan (double-click mode)
navguard

# Machine-readable report
navguard --check

# Check version
navguard --version
```

After `navguard` runs, the AI reads `navigator.md`, sets `[0,0,1]`, then drills into `navigator_deep.md` and `navigator_deep2.md` only for the files it needs.

## Supported File Types

Text files: `.md`, `.rs`, `.py`, `.js`, `.ts`, `.html`, `.css`, `.json`, `.toml`, `.yaml`, `.yml`, `.lock`, `.sh`, `.bash`, `.txt`, `.log`

Binary files: everything else (size only, no line/token counts)

## Ignored Directories

`target`, `.git`, `node_modules`, `__pycache__`

## Notes

- navguard is a standalone `.exe` — no dependencies, no installation required.
- The three navigator files are self-contained; they can be read by any AI or editor.
- The triangle state is the single source of truth for data freshness.
