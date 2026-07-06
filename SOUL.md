# SOUL.md — navguard v4.1 Protocol & Hermes Rules

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

## 4. Workflow — Phase Lifecycle

Each phase (`Build`, `Test`, `Debug`, `Optimize`, `Binding`, `Refactor`) is an instance of `Phase(name, step_task, entry_gate, exit_gate, next_on_pass)`. Skipping a phase is structurally impossible: the outer `TernaryCycle` of the phase will not close until all nested steps are closed (NESTING).

### 4.1 Gate — Phase Transition Condition

```
Gate(phase_from, phase_to) requires ALL conditions:
    - all_sections(phase_from).status == ✅
    - no_pending_DI(severity=high)
    - navguard.validate() == true
    - phase_cycle_count(phase_from) <= phase_timeout (default 10)
```

If `Gate() == false` → Hermes logs `PROTOCOL BREACH` in `logs/problems.md`, returns to `phase_from`, notifies user.

### 4.2 Before an AI Step

Each step is a `TernaryCycle(owner=AI, scope="step", state_file=progress.md)`:

1. `navguard --check --brief` — verify scan triangle
2. If `[0,1,0]` → ready; if `[1,0,0]` or `[0,0,1]` → `navguard --check`
3. Read the full tree from `navigator.md`
4. Set `Triangle: [0,0,1]` in `navigator.md` (confirm read)
5. WRITE-FIRST: step marker advances BEFORE work begins (`[1,0,0]`)

### 4.3 After an AI Step

1. Hermes validates anchors via `navguard --validate`
2. `navguard --check --diff` — check what changed
3. On success: step gets `[0,0,1]` + next step opens with `[1,0,0]` in one write

---

## 5. HermesContext — Orchestrator Memory

**File:** `hermes_context.md` (sits next to Plan_full.md)
**Reader:** ONLY Hermes. AI never reads this file.

```
STRUCTURE:
    ## Current   (overwritten each orchestration_turn)
        User goal, Active task, Marker [A,B,C],
        Instruction sent to AI, Expecting from AI, Priority notes
    
    ## History   (append-only, never overwritten)
        [timestamp] gave AI instruction X, expected Y, received Z
```

Every Hermes action MUST be wrapped in `TernaryCycle(owner=Hermes, scope="orchestration_turn")` — otherwise the system treats it as if it never happened (MANDATORY COVERAGE).

---

## 6. Audits — Auditor Class

All audits (`Critical Review`, `Audit`, `Triple Independent Audit`) are instances of a single `Auditor(name, memory, focus, output_prefix, pass_criteria)` class.

**memory:**
- `"clean"` → AI receives ONLY code + original requirements (independent third-party)
- `"full"` → AI receives full context: history, decision_log, previous findings

**PASS criterion:** zero substantial findings. `assess_findings()` checks 4 conditions: concrete problem + concrete location + concrete fix + measurable impact.

### Triple Independent Audit — Final Gate
Three parallel auditors (`memory="clean"`), isolated from each other:
- **TA1 Skeptic:** bugs, race conditions, off-by-one, incorrect logic/math
- **TA2 Architect:** structure, modularity, performance, scalability, tech debt
- **TA3 UX:** user behavior, responsiveness, edge cases, UX consistency

All three PASS → Completion (request user confirmation).
Any FAIL → `Phase("Refactor")` with findings from failed auditors.

---

## 7. Saturation — Cycle Stop Criterion

Not a fixed iteration count, but a saturation state:

```
saturation_reached() requires ALL conditions:
    - substantial high-priority == 0
    - substantial medium < 3
    - non_substantial findings on plateau (match previous pass)
    - no new findings in last two passes
    - all Deferred Issues resolved
```

Result written to `reports/audit_quality.md`.

---

## 8. Build from Plan

```bash
navguard --build plans/plan_N.md
```

---

## 9. navigator.md Format

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

## 10. Why This Exists

navguard lets AI work with code surgically:
- No need to parse the entire project — see the structure immediately via `navigator.md`
- Find any function/struct in seconds via `grep`
- Read exactly the code block needed via `lines` (start+end in symbol index)
- Don't waste context on navigation files

---

## 11. Prohibitions

- Starting a step without `--check --brief`
- Ignoring `[1,0,0]` (writing when scan is not complete)
- Reading files directly instead of using `lines`
- Loading `navigator_deep*.md` into memory (use grep/extract only)
- Skipping phases — Gate will not allow it, NESTING will not close
- Doing "post-hoc" audits — audits are always independent and clean

---

## 12. Core Principle

Never consider the first working version the final result. The system must repeatedly review its own decisions, find accumulated problems, replan the project structure, and improve until saturation — when no new meaningful improvements can be found.
