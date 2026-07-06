# navguard v4.1 — Project Navigator

Scans a project folder and produces a machine-readable map: file tree, line/token counts, symbol index. Designed for AI consumption.

## How It Works

navguard scans its **parent directory** (where the `.exe` lives). It writes three files:

| File | Content |
|------|---------|
| `navigator.md` | File tree + line/token table + triangle state |
| `navigator_deep.md` | File table only |
| `navigator_deep2.md` | Symbol index: every function/class/section with line numbers |

A **Triangle** `[A,B,C]` protects against crash corruption:
- `[1,0,0]` (A) — scan started
- `[0,1,0]` (B) — scan complete, data fresh
- `[0,0,1]` (C) — data confirmed read

Forward only: A → B → C → new cycle. No backward transitions.

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
