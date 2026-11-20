# Opterium Adaptive Method Architecture

**How an agent builds task-specific methods from the capabilities it already has —
without a prebuilt tool for every possible task.**

This document describes the adaptive method: how a method is constructed and
executed, the roles of `toolfmt` (the MCP gateway) and `any_txt_parser` (the
deterministic parser), the execution model, practical use cases, and the
verified limitations. It is written for AI agents first: short sections, file
pointers, and an explicit split between **implementation evidence**,
**runtime-test evidence**, and **architectural explanation**.

> **One-sentence concept.** The agent does not need a prebuilt tool for every
> possible task. It composes a task-specific **method** out of the capabilities
> already available to it, processes intermediate data inside the execution
> environment, applies a task-specific **template**, and returns only a compact
> structured result to itself.

Facts below were verified on 2026-11-20 by running the described paths
(read-only demonstration over stdio MCP). Versions: `toolfmt` gateway **3.1.3**,
`any_txt_parser` **1.5.0** — the two are versioned separately; do not conflate
them.

---

## A. Core concept

The Opterium stack gives the agent a small set of **primitives** (capabilities)
rather than one monolithic tool per use-case:

| Layer | Primitive | What it gives the agent |
|---|---|---|
| Discovery | `toolfmt` `catalog` | The available tools + params + output shape + measured cost |
| Invocation | `toolfmt` `call_tool` | Run a registered upstream tool; per-call template/format overrides |
| Deterministic parsing | `any_txt_parser` | Extract exact values from files (`jsonpath`, `path`, `xpath`, `css`, `regex`, `markdown`, `line`, `contains`) without reading whole files |
| Result processing | `toolfmt` standards + templates | Per-tool YAML standards rendered per call + task-specific templates that cut noise and cap output |
| Execution environment | the host shell/runtime (e.g. DSH `pwsh`, Python) | Where agent-written code actually **runs** (scripts, child processes, in-memory data) |

The **adaptive method** is the agent writing a small program that composes
these primitives for one concrete task, instead of relying on a single prebuilt
tool. Because the program runs in the execution environment, intermediate data
(raw payloads, per-item checks, intermediate JSON) can stay inside that process,
and only the final compact result is returned to the model.

### Do not conflate these (distinct capabilities)

- **Dynamic method creation** — the agent writes a script (PowerShell, Python,
  ...) that composes capabilities for a new task. This is the *method*. It is
  **not** a toolfmt feature; it is the agent's code running in the execution
  environment.
- **Adaptive template** — a task-specific set of instruction + extraction /
  transformation / output rules (a YAML file). It is **data, not code**; it can
  be applied per call via `toolfmt` (`extract` with a template) or implemented
  by the method's script.
- **Execution environment** — the host shell/process (DSH `pwsh`, Python, or
  any runner). This is where generated code executes and where intermediate
  data lives. A separate capability from toolfmt.
- **toolfmt (gateway)** — discovery, invocation, standard/template processing,
  and delivery of tool outputs. It is **not** an arbitrary-script executor
  (see D and F).
- **any_txt_parser** — deterministic file parsing/extraction (the
  `map → recon → extract → search_text → read_lines` pipeline). The primitive
  the method composes for file data.
- **Permanent MCP registration** — registering `toolfmt gateway` as an MCP
  server in the client. This is the **transport**, not the method. A method can
  be one-off (run in the shell) with no permanent registration.

---

## B. Adaptive methods — construction and execution

An adaptive method is a small, task-specific program that the **agent designs
itself**. Where the runtime permits, the agent generates a **temporary
executable script** (a one-off file it runs and then discards) — on a Windows
host that is typically a PowerShell or Python script; on Linux, a shell or
Python script. The script:

- **composes existing capabilities** — parser tools, gateway calls, child
  processes, plain file I/O — instead of requiring a new prebuilt tool;
- **uses conditions, loops, transformations and intermediate processing** —
  filter, group, derive, validate, retry;
- **treats multiple sources as one logical task** — several files, configs,
  or upstream tools feed one program run;
- **keeps intermediate results inside the execution process** — raw payloads
  never have to pass through the model's context; only the final compact
  result is printed;
- **can reuse useful methods/templates** — a method that worked once is kept
  as a script or a YAML template and re-applied to new inputs.

A typical shape:

1. **Discover** — learn what is available (`catalog`, or read the config).
2. **Retrieve** — pull data from one or more sources (files, config, child
   processes, upstream tools).
3. **Process** — transform / filter / derive in memory (intermediate data
   stays local).
4. **Template** — apply a task-specific template (extraction/output rules).
5. **Deliver** — print only the compact result.

### Verified demonstration (2026-11-20, read-only)

A Python script (the method) launched its own `toolfmt.exe gateway --home
<demo-home>` over stdio MCP and, in **one process run**, did:

| Step | Call | Result |
|---|---|---|
| 1 | MCP `initialize` | `serverInfo {name: toolfmt, version: 3.1.3}` |
| 2 | `tools/list` | 9 tools (5 pinned parser tools + 4 meta tools) |
| 3 | `call_tool` `map` on a plugin directory | 6 files; raw text kept **in-process** |
| 4 | `call_tool` `extract` with a task-specific template over a JSONL audit log | 146 events matched, parsed and tailed **in-process** |
| 5 | `call_tool` `cubint_add(9, 6)` | exact `"15"` |

Only the final ~25-line compact JSON report was printed to the model; all
intermediate payloads stayed inside the script process. Two conclusions:

- The execution path is **multiple MCP calls within one script run** — there
  is no single "run a method" call anywhere in the stack.
- The demo home (a temporary `upstreams.json` plus one template) required
  **no permanent MCP registration**: the method owned its own gateway
  instance for the duration of one script run.

---

## C. Adaptive templates and clean data

An **adaptive template** is a task-specific **instruction + extraction
structure** — not a presentation setting. For one task it says:

1. **Identify the relevant information** — which files, fields, lines, or
   events the task actually needs.
2. **Extract fields** — pull exactly those values with a rule
   (`jsonpath`, `path`, `xpath`, `css`, `regex`, `line`, `contains`, ...).
3. **Filter noise** — drop what is irrelevant (caps, selections, negation of
   boilerplate).
4. **Normalize / transform** — units, encodings, key order, derivations
   (`transforms` such as `unique`, `sort_desc`, `limit_items`).
5. **Produce predictable compact output** — a stable shape the agent can rely
   on from run to run.
6. **Reduce context consumption** — the budget is data-per-token; raw dumps
   carry no value.

**any_txt_parser** does the deterministic extraction. Given a file and a rule,
it returns exactly the matching values (with line numbers) and reports what
was truncated, skipped, or broken — it never guesses. Its default pipeline:
`map` (what is in the project) → `recon` (what a file is) → `extract` /
`search_text` (exact slice) → `read_lines` (narrow window). Templates use
**template YAML v1**: a file of rule objects, each with `select` (the rule),
`limits` (caps such as `max_items` / `max_item_chars`), `transforms`, and
`output`.

**toolfmt** adds discovery, invocation, and result processing:

- `catalog` — find tools and read their cards (params, output format,
  measured cost per captured call, example).
- `call_tool` — run a registered upstream tool; per-call overrides
  (`_template`, `_render`, `_pre`, `_post`, `_limit`, `_keys`) are stripped
  before the upstream call.
- **YAML standards** — a per-tool standard (`home/standards/<server>.<tool>.yml`)
  is rendered on **every call** of that tool (e.g. the `objects_table`
  renderer), so the agent sees a compact result, not a raw dump. Tools without
  a standard pass through untouched.
- **Per-call template application** — `extract` accepts a task-specific
  template, so a repeatable selection is a saved template, not a fresh rule
  set each time.

**Division of labor (clean data):** the **template** decides *what matters*;
the **parser** decides *how to pull it exactly*; **toolfmt** decides *how to
deliver it compactly*.

---

## D. Execution model — five distinct parts

| # | Part | What it is | What it is not |
|---|---|---|---|
| 1 | **Agent-generated code** (the method) | A task-specific program the agent writes: conditions, loops, transformations, intermediate processing | Not stored in toolfmt; not a registered tool |
| 2 | **Execution environment** | The host shell/runtime (DSH `pwsh`, Python, ...). Agent-generated code runs here; intermediate data lives here; child processes (including a gateway instance) are launched from here | Not part of the gateway |
| 3 | **toolfmt gateway** | An MCP stdio server: `toolfmt.exe gateway --home <home>`. Discovery via `catalog`, invocation of registered upstream tools via `call_tool`, standard/template processing, result delivery | **Not** an arbitrary-script executor; **not** a general MCP proxy |
| 4 | **Parser + template processing** | `any_txt_parser` (v1.5.0): `any_txt_parser.exe mcp --root <root>`; deterministic extraction; template YAML v1; navigation (navguard) | Versioned and maintained separately from toolfmt |
| 5 | **Transient methods vs permanent MCP tools** | A method can be one-off: a temporary script + a temporary home, run once in the shell. Registering `toolfmt gateway` in a client is an **optional transport** for later use — not a prerequisite for any method | Registration does not create capabilities; it only makes them reachable from a client |

**Key distinctions:**

- The **execution environment** is where agent-written code runs. `toolfmt`
  does **not** execute that code: `call_tool` invokes tools already registered
  as upstream servers (or pinned parser tools), not arbitrary agent scripts.
  Do not attribute the execution environment's capabilities (running scripts,
  in-memory data, background processes, scheduling) to toolfmt.
- **Transient vs permanent:** a method run once in the shell needs no
  registration; a saved standard, a pinned upstream, and a registered gateway
  are the *permanent* forms that make capabilities reusable from a client.
- **Registration is transport, not a prerequisite:** the verified
  demonstration above ran a full method against a gateway instance the
  method itself launched — nothing was registered in any client.

---

## E. Practical use cases

- **Repository investigation** — `map` → `recon` → `extract` to orient and
  pull exact slices without filling the context window; `catalog` to discover
  which tools cover the job.
- **Multi-tool data retrieval** — one method runs several upstream tools
  through `call_tool` in a single process run and merges their results.
- **File analysis / extraction** — task-specific templates over logs, JSONL,
  JSON, XML, HTML; deterministic, capped, with declared truncation.
- **Batch processing** — a loop over many files or entries with per-item
  checks and in-process aggregation (the 146-event JSONL case above).
- **Structured reporting** — standards render each tool's output compactly;
  the method prints one final compact report (the ~25-line JSON case above).
- **Validation / monitoring** — repeatable checks (health audits, standard
  validity via `toolfmt check`, `toolfmt replay`) expressed as methods that
  return a compact pass/fail table.
- **Background / recurring automation** — only where the **execution
  environment** supports it (OS schedulers such as Windows Task Scheduler or
  `cron`, or harness-level scheduled tasks such as DSH's). The gateway
  itself has no scheduler; recurring runs are the execution
  environment's capability, not toolfmt's.

---

## F. Architecture and limitations

### Capability split — who provides what

| Provided by | Capabilities |
|---|---|
| **Native to toolfmt** (v3.1.3) | `catalog` discovery (with measured per-call cost); `call_tool` invocation of registered upstream tools and pinned parser tools; per-call YAML standard rendering (`home/standards/*.yml`, e.g. `objects_table`); per-call overrides (`_render`, `_pre`, `_post`, `_template`, `_limit`, `_keys`); `formatter_inspect` / `formatter_save_standard` (standard authoring against captured outputs); `audit.jsonl` per-call log; offline modes `format`, `replay`, `check`, `serve`; a Windows x64 binary and a Linux x86_64 build |
| **Provided by the parser** (`any_txt_parser`, v1.5.0) | Deterministic extraction (`extract`, rule types `jsonpath`/`path`/`xpath`/`css`/`regex`/`markdown`/`line`/`contains`); `recon` file inspection; `map`/`symbols`/`find`/`search_text`/`read_lines`/`todos`/`changes` navigation (navguard); template YAML v1 (rule objects: `select`/`limits`/`transforms`/`output`); its own MCP server (`any_txt_parser.exe mcp --root <root>`); CLI report mode |
| **Provided by the execution environment** | Running agent-generated code; child processes (including launching a gateway instance); in-memory intermediate data; file I/O for scripts/templates; background and scheduled execution where the host provides it (Task Scheduler, `cron`, harness scheduling) |
| **Available through composition** (the adaptive method) | Multi-source retrieval into one logical task; in-process transformation/filtering/aggregation; keeping intermediate data out of the model context; temporary scripts and temporary homes with no permanent registration; reusing methods/templates across tasks |
| **Dependent on external infrastructure** | The upstream MCP servers themselves (their tools, their launch targets, their reachability); network access where an upstream needs it; the OS scheduler for recurring runs; an MCP client, when the method must be reachable from one rather than run in the shell |

### Verified limitations (2026-11-20)

1. **Whole-workspace `map` can exceed the per-upstream timeout.** The gateway
   gives each upstream call a budget (60 s in `upstreams.json`); a
   whole-workspace `map` can run past it. The call eventually completes; a
   client that aborts at that budget must raise its client-side timeout.
2. **`zvec_grep` builds its index on first use.** The first semantic search in
   a workspace is slower than later ones.
3. **The gateway is a gateway for tools, not a general MCP proxy.** Upstream
   resources and prompts are not aggregated (`resources/list`, `prompts/list`
   answer empty lists); upstream-to-client requests (`roots/list`, sampling,
   elicitation) are not forwarded. A server that needs them will not work
   through toolfmt.
4. **Parser root boundary.** `any_txt_parser` reads only inside its `--root`;
   templates and inputs outside the root are rejected. Keep both inside.
5. **No arbitrary code execution via toolfmt.** `call_tool` runs registered
   upstream tools; the *execution environment* runs agent-generated code.
   There is no single "run a method" call: a method is a sequence of ordinary
   calls inside one script run.
6. **Separate versioning.** The gateway is at 3.1.3; the parser at 1.5.0.
   Statements about one do not transfer to the other.

Avoid two mirror-image exaggerations: claiming toolfmt executes agent code
(false for v3.1.3), and claiming the method must be registered as a permanent
MCP tool to work (also false — registration is an optional transport).

---

## Links

- Adaptive method demonstration and the repo's adaptive tooling in general:
  [`README.md`](README.md)
- toolfmt gateway usage: [`software/toolfmt/crates/toolfmt/README.md`](software/toolfmt/crates/toolfmt/README.md);
  authoritative spec: [`software/toolfmt/crates/toolfmt/SPEC.md`](software/toolfmt/crates/toolfmt/SPEC.md)
- toolfmt workspace (bundled parser docs below the gateway header):
  [`software/toolfmt/README.md`](software/toolfmt/README.md)
- Parser usage: [`software/parsers_for_AI/README.md`](software/parsers_for_AI/README.md)
  (+ `HELP.md`, `FULL_GUIDE.md`)
- Software overview: [`software/README.md`](software/README.md)
- Spec (theory): [`Theory/ontology_of_number/opterium_v1.0_spec.txt`](Theory/ontology_of_number/opterium_v1.0_spec.txt)
