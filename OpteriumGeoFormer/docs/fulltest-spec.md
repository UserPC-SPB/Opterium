# Feature Specification: Full Test — Documentation Integrity & Code Cross-Verification

**Status**: Draft | **Date**: 2026-05-19
**Input**: All 10 modules + spec_compiled.json + API.md + help.txt

## Core Principle

Каждая функция в коде должна быть задокументирована. Каждый пример в документации
должен выполняться и давать правильный результат. Документация должна быть внутренне
непротиворечива.

## User Stories

### US-FT1 — Code-Doc Coverage (Priority: P0)
As a developer, I want to know that **every public function/method** in all 10 modules
has a corresponding entry in spec_compiled.json, API.md, and help.txt.

**Acceptance**:
1. Given all 10 modules, When scan for `def ` and `class `, Then every name appears in docs
2. Given documented functions, When check signatures, Then they match actual signatures
3. Given documented descriptions, When read, Then they are non-empty and informative

### US-FT2 — Example Correctness (Priority: P0)
As a developer, I want every example in spec_compiled.json to **actually run** and
produce the documented output.

**Acceptance**:
1. Given spec_compiled.json with N examples, When execute each example, Then all pass
2. Given edge cases in docs, When tested, Then behavior matches documented description
3. Given error cases, When triggered, Then correct exception raised

### US-FT3 — API.md Integrity (Priority: P1)
As a developer, I want API.md to be a **faithful rendering** of spec_compiled.json
with no formatting errors, missing sections, or broken structure.

**Acceptance**:
1. Given API.md, When parse sections, Then every module section matches spec_compiled.json
2. Given method names in API.md, When extract, Then no duplicate names exist
3. Given formatting, When validate, Then all `code` spans are properly closed

### US-FT4 — help.txt Integrity (Priority: P1)
As a developer, I want help.txt to be a **valid quick-reference** — every function
listed must exist, every signature must be parseable.

**Acceptance**:
1. Given help.txt, When scan for `def ` and `.methodName`, Then all names exist in code
2. Given signatures, When checked, They match actual function signatures
3. Given class lines, Class names are valid

### US-FT5 — spec_compiled.json Integrity (Priority: P2)
As a developer, I want spec_compiled.json to be **self-consistent**: no duplicate
entries, no empty examples arrays without explanation, no dangling references.

**Acceptance**:
1. Given spec_compiled.json, When validate, No duplicate `(module, class, method)` paths
2. Given each example, `input` and `output` fields exist and are non-null
3. Given each method, Input types match output types of previous methods where referenced

### US-FT6 — Generation Pipeline Integrity (Priority: P2)
As a developer, I want the pipeline `code → spec_collect.py → spec_compiled.json →
spec_to_help.py → API.md + help.txt` to be **deterministic and reproducible**.

**Acceptance**:
1. Given same code, Two runs of spec_collect.py produce identical spec_compiled.json
2. Given same spec_compiled.json, Two runs of spec_to_help.py produce identical files
3. Given regeneration, Output files are timestamped and can be diffed

### US-FT7 — Stress & Edge Coverage (Priority: P2)
As a developer, I want **systematic edge-case testing** for every documented behavior,
not just happy-path examples.

**Acceptance**:
1. Given Pt(0,0), All operations produce documented results
2. Given negative values in all operations, Behavior matches docs
3. Given extreme values (near table bounds, very large mantissa), Graceful handling per docs
4. Given invalid inputs (mismatched types, out-of-range), Proper error per docs

## Requirements

### Functional Requirements
- **FR-001**: System MUST enumerate ALL public functions from ALL modules
- **FR-002**: System MUST compare each function against its documentation entry
- **FR-003**: System MUST execute every documented example and verify output
- **FR-004**: System MUST validate API.md formatting (balanced code spans, section structure)
- **FR-005**: System MUST detect undocumented functions (present in code, absent from docs)
- **FR-006**: System MUST detect phantom docs (present in docs, absent from code)
- **FR-007**: System MUST verify generation reproducibility (same input → same output)
- **FR-008**: System MUST produce a coverage report showing pass/fail per module

### Key Entities
- **Module**: One Python file exporting classes/functions (10 total)
- **DocEntry**: One function/method entry in spec_compiled.json
- **Example**: One (input, output, description) triple in a DocEntry
- **CoverageReport**: Per-module summary of {total, documented, missing, phantom, examples_tested, examples_passed}

## Success Criteria

- **SC-001**: 100% code-doc coverage for all 10 modules (zero undocumented public functions)
- **SC-002**: 100% example execution success (all N examples produce documented output)
- **SC-003**: API.md has zero formatting errors (balanced spans, no broken section hierarchy)
- **SC-004**: Generation pipeline is reproducible (same input → byte-identical output)
- **SC-005**: Coverage report generated and stored in results/

## What Is Out of Scope
- Unit tests for internal/private functions (prefixed with `_`)
- Performance benchmarking (covered by existing benchmark suite)
- Integration tests between GeoFormer and external systems
