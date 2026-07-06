# SOUL.md — navguard v4.1 Protocol

## 1. State Check

```bash
navguard --check --brief    # scans its own directory (where the exe lives)
```

Output:
```
T: 0,1,0              ← TernaryCycle(scope="scan") state [A,B,C]
B: sync|2026-07-02T... ← timestamp of last scan
B: rules|none         ← active rules
```

**Triangle = `TernaryCycle(owner=navguard, scope="scan", state_file=navigator.md)`:**
- `[1,0,0]` (A) — scan initiated (navguard sets this at the start of `--check`)
- `[0,1,0]` (B) — data ready/fresh (navguard sets this after writing 3 files)
- `[0,0,1]` (C) — confirmed read (the **reader** sets this manually in `navigator.md`)

Forward-only transitions: A → B → C → new cycle. No backward transitions — physical impossibility of looping.

**If `[0,1,0]`** → fresh data, ready to work.
**If `[1,0,0]`** → previous scan crashed (crash recovery). Re-run `navguard --check`.
**If `[0,0,1]`** → needs rescan: `navguard --check`.

---

## 2. Project Navigation

```bash
navguard --check                            # full scan + tree + symbols
navguard grep <pattern>                     # search the symbol index
navguard extract <pattern> [<path>]         # search with ±3 line context
navguard lines <rel_path> <M> [N]           # read lines M..N (1-indexed)
navguard todos                              # all TODO/FIXME/XXX/BUG
navguard --triangle status                  # read triangle from navigator.md
navguard --check --diff                     # changes since last scan (from cache)
```

---

## 3. What NOT to Use

- Do NOT read `navigator_deep.md` or `navigator_deep2.md` into context — use `lines`/`grep`/`extract` instead.
- Do NOT parse files directly — use `lines`.
- Do NOT manually edit the triangle (except setting `[0,0,1]` after reading the tree — this confirms freshness).

---

## 4. Build from Plan

```bash
navguard --build plans/plan_N.md
```

---

## 5. navigator.md Format

```
# Navigator
Updated: 2026-07-02T05:31:01.902703500+03:00
Root: D:\plans_hermes\projects\<project>
Triangle: [0,1,0]

## Tree
[project file tree]

## File Table
| Path | Lines | Tokens |
```

---

## 6. Why This Exists

navguard lets AI work with code surgically:
- No need to parse the entire project — see the structure immediately via `navigator.md`
- Find any function/struct in seconds via `grep`
- Read exactly the code block needed via `lines` (start+end in symbol index)
- Don't waste context on navigation files

---

## 7. Prohibitions

- Starting a step without `--check --brief`
- Ignoring `[1,0,0]` (writing when scan is not complete)
- Reading files directly instead of using `lines`
- Loading `navigator_deep*.md` into memory (use grep/extract only)


---

## 8. Core Principle

Never consider the first working version the final result. Review, restructure, and improve until no meaningful issues remain.
