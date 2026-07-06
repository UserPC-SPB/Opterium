# navguard v4.1 — Project Navigator

Scans a project folder and produces a machine-readable map: file tree, line/token counts, symbol index. Designed for **vibe coding** — AI-assisted development where the model must navigate a codebase without loading every file into context.

## How It Works

navguard scans its **parent directory** (where the `.exe` lives). It writes three files:

| File | Content |
|------|---------|
| `navigator.md` | File tree + line/token table + triangle state |
| `navigator_deep.md` | File table only |
| `navigator_deep2.md` | Symbol index: every function/class/section with line numbers |

A **Triangle** `[A,B,C]` enforces the read-before-use contract:

| State | Who sets it | Meaning |
|-------|-------------|---------|
| `[1,0,0]` | navguard (at scan start) | scan in progress, data invalid |
| `[0,1,0]` | navguard (at scan end) | scan complete, data fresh, ready to read |
| `[0,0,1]` | **AI must set this** (after reading) | data confirmed consumed |

**Rules:**
- navguard sets `[1,0,0]` in `navigator.md` when it begins scanning, `[0,1,0]` when it finishes.
- The AI **must** read `navigator.md` completely, then edit its `Triangle:` field from `[0,1,0]` to `[0,0,1]`.
- If the triangle is **not** `[0,1,0]`, reading `navigator.md` is forbidden — the data is stale or corrupt.
- Forward only: `[1,0,0]` → `[0,1,0]` → `[0,0,1]` → new cycle. No backward transitions.

This guarantees that the next AI can check the triangle and know whether the previous AI actually consumed the tree or merely triggered navguard and ignored the output.

## File Hierarchy

The three navigator files form a point-access chain:

```
navigator.md        ← file tree + line/token counts + triangle state
    ↓
navigator_deep.md   ← flat file table (path|lines|tokens)
    ↓
navigator_deep2.md  ← symbol index (function/class/section → line numbers)
```

The AI always starts at `navigator.md`, reads it fully, sets `[0,0,1]`, then drills into deeper files only for specific symbols. Never load the deep files into context unless you need their contents.

## Commands

| Command | What it does |
|---------|-------------|
| `navguard` (no args) | Double-click mode: full scan, wait for key |
| `navguard --check` | Full report (T/F/B), writes all 3 navigator files |
| `navguard --check --brief` | Brief report (T/B only), no scan |
| `navguard --check --diff` | Changed files only, no navigator rewrite |
| `navguard grep <pattern>` | Search navigator_deep2.md's symbol index |
| `navguard extract <pat> [path]` | Search file contents with ±3 lines of context |
| `navguard lines <path> <s> [e]` | Print lines s..e (1-indexed) from a file |
| `navguard todos` | List TODO/FIXME/HACK/XXX/BUG across the project |
| `navguard --triangle status` | Show current Triangle state |
| `navguard --validate` | Check anchor_map.md anchors against .md headers |
| `navguard --build <plan>` | Assemble files from a plan's ---BEGIN---/---END--- blocks |

## Output Format

```
T: 0,1,0              ← triangle state after scan
F: file1.py|120|1500  ← each file: path, lines, tokens
B: sync|2026-07-06T…  ← last scan timestamp
B: rules|none         ← active rules
```

## Why

Without navguard, an AI must guess the project structure or load entire files into context. With navguard, the AI reads a tiny tree file, finds symbols by name, and reads exactly the needed lines — no waste.

This is the core loop of **vibe coding**: scan (navguard) → read tree (navigator.md) → find symbol (grep) → read lines (lines) — all without loading the full codebase into the model's context window.
