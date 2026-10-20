# any_txt_parser Changelog

Versions are listed from newest to oldest. The **[contract]** marker indicates a change after which
a consumer of the output or an existing template may see a different result: a different exit
code, a different status or warning text, a different format by sniffing, a different data view
or `## Effective Template`. Behavior details are in `FULL_GUIDE.md`, the brief
quick reference is `HELP.md`. Section numbers in parentheses (§N) refer to `FULL_GUIDE.md`.

"Before" in all 1.0.4 items refers to the released exe 1.0.3 (dist from 2026-10-14).

## Unreleased

Repository fixes from the external review of 1.5.0. Program behavior did not change.

- Documentation consolidated: `FULL_GUIDE.md` is the single full guide.
- `tests/fuzz.rs` did not compile on rustc 1.95.0 (24 errors E0277/E0308): `Rng::pick`
  returned `&T`, and the old compiler inferred `T = str` from the expected `&str` in calls of the form
  `push_str(rng.pick(&["…"]))`. Now `pick` returns an element by value; the random
  sequence and the cases themselves are unchanged.
- Compiler pinned: `rust-toolchain.toml` — rustc 1.97.1 (the released 1.5.0 was built with it;
  rustup will install it on the first build).
- Build is reproducible: the linker receives `/Brepro` (`.cargo/config.toml`) and does not write the
  build time into PE headers; `scripts/release.ps1` removes `<CARGO_HOME>\registry\src` paths from the exe
  of the build machine (`--remap-path-prefix`). Identical sources, rustc, and MSVC set produce a
  byte-for-byte identical exe.
- Single line ending: `.gitattributes` — `* text=auto eol=lf` (`.bat` — CRLF, `tests/snapshots/**`
  without transformations), so the working copy, `git archive`, and source archives are all LF,
  regardless of `core.autocrlf`.
- BOM at the start of `crates/navguard/src/watch.rs` removed.
- `examples/errors.yml` and `examples/example.log` are mentioned in `README.md` and checked by the test
  `readme_example_finds_the_error_with_its_context` (`tests/golden.rs`).
- `README.md`: exe size "~6.4 MB" (was "~6.5 MB"); guide references in README and test
  comments now point to `FULL_GUIDE.md`.

## 1.5.0 — 2026-10-19

Three inconveniences for working with code, found during the evaluation of 1.4.0: noisy `todos`, `find` without exact matching, and `extract` without line numbers. "Before" refers to the released 1.4.0. Tests are in `tests/one_exe.rs` (`extract_lines_exact_find_and_strict_todos`, via a real exe and MCP), `tests/api.rs` (`text_rules_report_source_lines`, `stable_deduplicate_keeps_the_lines_of_the_kept_items`), and module tests for navguard `api` and the `text` adapter.

### `extract`: line numbers

- **[contract]** Text rule elements (`contains`, `starts_with`, `ends_with`, `regex`, `line`) in text output start with the source line number in grep style:
  `- L12: text` — matching line, `L11- ` — context lines (`--context`) and other lines in a `line:A-B` range; every line of an element is numbered. If a transformation changed the number of lines in an element (`truncate_lines`, `--max-item-chars`, `trim`, …), only the first line is marked — with the original range: `- L41-L43: …`. Before, there were no numbers, and to edit what was found, the agent had to search for the line again. Anyone parsing text output programmatically must strip the prefix (or use `--json`).
- **[contract]** JSON (`--json`, MCP `output: "json"`): text rule results have new fields `lines` — the matching line number for each element, parallel to `items` (for `line:A-B` — A) — and `spans` — `[first, last]` line of each element, if at least one element spans multiple lines (context, range). Previous fields are unchanged; other selectors, empty results, and results after `join` do not have the new fields. Numbers follow elements through `unique`, `sort`, `limit_items`, `max_items`, `stable_deduplicate`, and truncation by `max_output_chars`.
- Report mode (`-i -t -o`, Markdown document) is unchanged: it has no line numbers.

### `find --exact`

- `find PATTERN --exact` and the MCP tool argument `exact: true` for `find_symbol`: only symbols whose name equals the pattern (for C++ `Foo::bar` — also `bar`); signature and path are not considered, case is as without the flag (case-insensitive if `--case-sensitive` is not present). Before, it was only substring inclusion in name, signature, or path: on serde_json sources, `find from_str` gave 8 symbols, including `from_string`, `from_string_unchecked`, and `test_deserialize_from_stream`; with `--exact` — 5 real `from_str`. Text header:
  `find 'cart' (exact name): 2 symbol(s)`. API: `FindOptions::exact`.

### `todos`: whole-word markers

- **[contract]** `todos` (CLI, MCP, `nav --todos`) looks for TODO, FIXME, HACK, and XXX markers as whole words (no letter, digit, or `_` adjacent): uppercase, or in any case if immediately followed by `:`, `(`, `!`, or preceded by `@` — `// TODO`, `todo: x`, `Fixme(bob)`, `todo!()`, `@todo`. Not counted are `todos`, `TODO_LIMIT`, `todo_hits`, `hackathon`, `a todo list`. Before, any occurrence of TODO/FIXME/HACK in any case was counted: on the 1.4.0 tree of this repository, 124 lines, almost all — the word "todos" in documentation and names like `todo_hits`; now 37, and all of them are uppercase markers (test data and description of the command itself). New marker — XXX.
- `todos --loose` and MCP argument `loose: true` — previous behavior (substring, any case, no XXX). API: `todos(root, TodoMatch)`, `TodoMatch::{Strict, Loose}`.

### Documentation

- `help extract` (line numbers, `lines`/`spans` fields), `help navigation` (`--exact` with a WRONG/RIGHT example, `todos` rules, `--loose`, MCP arguments), `help mcp`, `help overview`, `HELP.md`, README; descriptions of MCP tools `extract`, `find_symbol`, `todos`.

## 1.4.0 — 2026-10-19

Response to the verification report for 1.3.0 (`ISSUES_EN.md`): service files in the scan, MCP resources, repeated `Root:`, two meanings of `[A,B,C]`, time in the state file, names without folders in deep, `nav --version`, packaging and help. "Before" — released 1.3.0. Tests —
`tests/navigator_state.rs` (via a real exe) and navguard module tests (`scanner`,
`navigator`, `api`, `triangle`) and `mcp`.

### Scan

- **[contract]** The scan (`map`, navigators, cache, `search`, `todos`, MCP) always skips
  navguard's own files: in addition to navigators and `.navguard_cache.json`, now also
  `.navguard_rules` and the working cycle `--triangle` files `.abc_triangle_state.json`,
  `.abc_triangle_log.json` — regardless of `exclude`. Before, they appeared in the map and summaries
  (`scripts/.abc_triangle_log.json  58 ~213`).
- **[contract]** The default `exclude` list has been supplemented with `.zvec-grep` (index of the
  zvec-grep tool): before, its `CURRENT`, `*.sst` inflated the summaries. Your own `exclude` in `navguard.json`
  still replaces the entire list.

### Navigators

- **[contract]** `Root:` is written only in `navigator.md`; it is no longer present in `navigator_deep.md` and
  `navigator_deep2.md` (there were three copies of one path). The `Updated:` line
  remained in all three — by it you can see that they are from the same scan.
- **[contract]** The File column in `navigator_deep.md` and the section header in `navigator_deep2.md` —
  path relative to Root (`| src/cart.rs | ...`, `### src/cart.rs  (deep L41)`). Before —
  only the file name, and two `dispatch.md` from different folders were distinguished only by line position.
- **[contract]** `Triangle: [0,1,0]` in `navigator.md` has been replaced with `Status: READY`. The record
  `[A,B,C]` now means only the state of the working cycle `nav --triangle` (before in
  the navigator it was always `[0,1,0]`, while `--triangle status` at the same moment gave `[0,0,1]`).
  Deep/deep2 headers became one line shorter due to the removed `Root:` line, pointers
  `deep L…` have been recalculated.

### Working cycle `--triangle`

- **[contract]** `timestamp` in `scripts/.abc_triangle_state.json` and in the log
  `.abc_triangle_log.json` — local time ISO 8601 with offset
  (`2026-10-19T14:40:41+03:00`), like `Updated:` of navigators. Before — Unix seconds as a string
  (`"1792410059"`). Old files are read as before. The line `B: sync|…` for `nav --check`
  remained in Unix seconds (scripts parse it).

### `nav`

- `nav --check` still only prints the report and does not write navigators — this was also the case in
  navguard 0.1.0 (checked on its exe: `--check` on a clean folder does not create files). Now this is
  stated in `nav --help` and `help navigation`; those who need files — `nav --oneshot`.
- `nav --version` and `nav -V` print the same line as `any_txt_parser --version`
  (`any_txt_parser 1.4.0`). This key was also absent in navguard 0.1.0.

### MCP

- **[contract]** Resources `project://tree` (text/plain: tree of the server root and files with
  lines and tokens, like `map` without arguments, root shown as absolute) and
  `project://state` (JSON: `root`, `checked`, `totals`; `navigators.status` — `fresh`, `stale`
  with numbers `new`/`changed`/`deleted` and paths, `incomplete`, `missing` — and `updated`;
  `snapshot` — time of the `changes` snapshot; `workflow` — state of `--triangle`). Freshness
  of navigators is determined by sha of each file from `navigator_deep.md`. In `initialize`
  the capability `resources` is declared; `resources/read` with an unknown URI — error -32002.
  Before, `resources/list` returned an empty list.

### Documentation and packaging

- `help overview`: sections Core rules (five working rules) and When something goes wrong
  (table "what is visible → what it means → what to do" based on real messages and exit codes).
- `help navigation`: full list of skipped folders and files, `--check` without writing,
  format `T:/F:/I:/B:`, `--version`; `help mcp`: Resources section.
- README: exe size (~6.5 MB, was "~5 MB"), MCP resources, What changed in 1.4 section;
  the report-mode description no longer links to the full guide (it is not shipped in `dist\`).
  `dist\` still = exe + `HELP.md` + `README.md` (MIT license — in README).

### Known limitations (not fixed in 1.4.0)

- `exclude` compares a substring of the name: `.git` hides both `.gitignore`, `.github`,
  `.gitattributes`; `target` — and `targets.rs`. `.gitignore` is not read.
- Templates T1–T8 and the performance section from the old field guide have not
  been returned: in `help extract` there are nine examples, in `help overview` — a line about speed.
- The remaining items of 1.3.0 remain in force.

## 1.3.0 — 2026-10-19

Project root boundaries, honest lines and tokens, navigators with linked pointers, safe `nav --patch`. The motivation was a comparative report on 1.1.0: everything confirmed in 1.2.0 has been fixed. "Before" refers to the released 1.2.0. Tests are `tests/root_bounds.rs` (via the actual exe) and navguard module tests (`paths`, `text`, `scanner`, `navigator`, `cli`).

### Root boundaries

- **[contract]** Navigation commands (`map`, `symbols`, `find`, `lines`, `search`, `todos`,
  `changes`) read only within `--root` (default is the current folder). Before, without `--root`,
  the root became the file's own folder, and `lines C:\Windows\win.ini 1 2` could read any file.
  A path going outside (`..`, an absolute path to another location, a symlink or junction leading outside) —
  code 3, `path is outside the project root <root>: <path>`.
- **[contract]** MCP: `mcp --root FOLDER` sets the server root (the server moves into it;
  without the flag — the working folder). The `root` argument of tools, files for `read_lines`/`symbols`/
  `search_text`, input and template for `extract`, path for `recon` must lie within it, otherwise
  `isError: true`. An input mask cannot go outside (`..` after `*` is forbidden), each
  found file is checked after resolving links. Before, `read_lines` without `root` could read
  any absolute path.
- **[contract]** `nav` writes and reads only within the root: `--lines`, `--extract`, `--patch`,
  `--build` (plan, sources and targets), `--validate` (files from `anchor_map.md`). `--set-rules`
  writes `.navguard_rules` to the root (before — to the current folder), `--triangle` takes the state file
  and log in the root (before — relative to the current folder, bypassing `--root`).
- **[contract]** Project traversal (map, navigators, cache, search) does not enter symlinks or junctions.
  Before, a junction inside the project pointing to an external folder added its contents to the map.

### Lines, tokens, binary files

- **[contract]** navguard counts lines by parser rules: LF, CRLF and single CR — end of
  line; empty file — 0 lines; the last newline does not add a line. Before —
  "newlines + 1": `one\ntwo\nthree\n` gave 4, empty file 1. Single CR is now end of
  line for `lines`, `search`, `todos`, `symbols` (the 1.2.0 limitation was removed).
- **[contract]** Tokens — an estimate with a margin: approximately one per 3 bytes of UTF-8 (4-byte character
  — 2). Before "lines × expansion coefficient", which underestimated by orders of magnitude (`runner.py` 97 KB —
  "2992 tokens"). Character estimate in `symbols`/deep2 — based on the text of its lines (before "non-empty
  lines × 1.5"). `token_coeff` in `navguard.json` is no longer used.
- **[contract]** Binary files (NUL in the first 8 KiB, except UTF-16 with BOM) — without lines and tokens:
  `map` writes `binary, N bytes`; in JSON, the file has new fields `bytes` and `binary`.
- `navguard.json` and the triangle state file with BOM are read (before BOM silently gave
  default settings and triangle `[0,0,0]`).

### Navigators

- **[contract]** `navigator.md`, `navigator_deep.md`, `navigator_deep2.md` are built in memory
  from a single read of each file (size, lines, sha and symbols from the same bytes) and written
  atomically: deep2, deep, navigator. Before, deep2 re-read files from disk.
- **[contract]** `navigator.md`: `Root:` — absolute path of the root (before `Target: .`),
  `Updated:` — system date and time of the scan with timezone (before there was no time),
  `Triangle: [0,1,0]` — READY (before `abc:` — state of the working cycle, which
  immediately after a scan showed `[0,0,1]`), summaries, "how to read" note and tree; each
  file has pointer `deep L<n>`. The FILES table was removed (it verbatim repeated deep).
- **[contract]** `navigator_deep.md`: one line per file in tree order — name, bytes, lines,
  tokens, sha, `deep2 L<a>-L<b>`. `navigator_deep2.md`: sections only for files with symbols,
  header `### name  (deep L<n>)`, columns Symbol, Kind, Lines, Tokens, In. Full path —
  only in the tree (Root + folders); in deep and deep2 — file names, identical names are distinguished
  by pointers. The header "from anchor_map.md" and repetition of protocol anchors from `anchor_map.md`
  were removed (they are already present as `[sX]…[sXend]`).

### `nav --patch`

- **[contract]** `replace`, `insert`, `delete`: code 1 and nothing written if the text is not
  found, line number is outside the file, path is outside the root, file is binary or not UTF-8. Before
  `replace` without a match gave code 0.
- `--dry-run` prints the change (`-`/`+` lines with numbers) and writes nothing.
- File newlines (CRLF, CR) and BOM are preserved; multi-line `<old>` with LF finds text in
  CRLF files. Before, `insert`/`delete` converted the entire file to LF and always added a newline
  at the end. Writing is atomic (temporary file and rename).
- Empty `<old>` — error (before it inserted `<new>` between all characters).

### Other

- `help --help`, `nav --help`, `nav -h`, `nav help`, `nav --patch help` print help with
  code 0 (before — error).
- `nav` returns the exit code of the mode; for `--validate`, `--merkle-verify`, `--triangle` with an error
  — 1, as before.
- README: MCP connection with `--root`, section "It stays inside the project", MIT license text
  (README ships with the exe in `dist\`).

### Known limitations (not fixed in 1.3.0)

- CLI `extract` and `recon` read what they are given, without root restriction (they have no
  `--root`); in MCP they are limited by the server root.
- Traversal exceptions — only the `exclude` list in `navguard.json` (substring of name, by
  default `.git`, `target`, `node_modules`, `__pycache__`); `.gitignore` is not read, service
  folders of other tools (for example `.zvec-grep`) need to be added to `exclude`.
- `nav --patch replace` changes all occurrences; there is no "exactly one occurrence" check, `--dry-run`
  shows their count.
- Absolute paths of the build machine (`cargo-home\registry\...`) remain in the exe in panic messages
  of dependencies.
- Items from 1.2.0 regarding Markdown escaping in metadata, `--max-block-chars` for a split group
  and items from 1.0.4 regarding XML `inner_html`/`outer_html`, `&lt;` in entities and JSON numbers greater than 2^53
  remain in effect.

## 1.2.0 — 2026-10-19

Fixed the "known limitations" from 1.0.4 (except for the last three items in that list).
The version is minor because the output contract changes: different data, new lines and a document column,
different metadata format. "Before" refers to the released 1.1.0. Regression tests are
`tests/known_limits.rs` (each fails on 1.1.0).

### Data

- **[contract]** `document.stable_deduplicate` compares values before truncation by
  `max_item_chars`. Before, already truncated values were compared, and different records with a common
  beginning were removed as duplicates (data loss): `aaaaaaaaaaX`, `aaaaaaaaaaY` with
  `max_item_chars: 10` yielded one item, now two. True duplicates are removed as before,
  the line `N truncated item(s) dropped as duplicates by stable_deduplicate` remains (§8).
- **[contract]** One newline translation rule for all formats: LF, CRLF, and single CR
  (old Mac) — end of line. Decoded text is immediately normalized to LF, before format detection.
  Therefore a file with only CRs is multiple lines, not one (`line:`, `contains`,
  `ends_with`, context, `recon` see the same lines), and `\r` from CRLF no longer ends up in
  source XML fragments (element, `inner_html`) and other values copying source text.
  There are no CRs from line endings in the document, so line numbers in `## Block Index`
  do not diverge for readers who count a single CR as an end of line. A CR inside a value
  (JSON `"\r"`, XML `&#13;`) is data and is preserved (§6).
- **[contract]** TOML tables preserve the file's key order (`preserve_order` is enabled for
  `toml`): `pool = { min = 2, max = 16 }` → `{"min":2,"max":16}` (before it was
  `{"max":16,"min":2}`), `$.servers.*` iterates tables in declaration order (§7.2).
- **[contract]** HTML `inner_html`/`outer_html` output attributes in their original order
  (`deterministic` is enabled for `scraper`), not by name as in 1.0.4. Output remains
  deterministic (§7.6).
- **[contract]** XML with a DOCTYPE and no `.xml` extension (plist, MyBatis mapper) is
  detected as xml when `format: auto`: sniffing parses it the same way as the adapter, with an
  internal DTD subset. Before, such a file was text. A file whose DTD is rejected
  (`XML DTD rejected: …`, for example an entity "bomb") is also xml and receives this file error (§6).

### Document self-description

- **[contract]** In `## Execution` after `Run status` — line `Exit code: N`: the code with which
  the run that published the document finished.
- **[contract]** In `## Files` a new column `Encoding` after `Status`: the encoding with which
  the file was read (`utf-8`, `utf-8 (BOM stripped)`, `utf-16le (BOM)`, …; for a file with a decoding error —
  the requested one). Substrings like `| json | success |` still find the line. The numeric column `Size` is right-aligned (before it was `Modified`).
- **[contract]** The parent of a split group shows in `Items:` the number of items in the entire group
  (fragments of one item — one item; before it was `Items: 0`), and instead of `(no data)` —
  `(in the continuation parts)`. The same number is in the `Items` column of the parent line in
  `## Block Index` (§8).
- **[contract]** For a block with `rule not applicable` in `### Warnings`, the reason is specified:
  `rule not applicable: selector type css does not apply to json files (they take jsonpath,
  path)`. The same warning is in the results of `extract`/MCP with `--include-empty` (§7.9).
- **[contract]** Applied Template shows `Extract: attr:href` (`inner_html`,
  `outer_html`) if the mode is specified (`text` by default is not written, as in Effective
  Template). The markdown selector is written as in the shortened rule: `heading:^Уст`, `list_item`
  (before it was `node=h pattern="^Уст"`), the `line` selector — `1-20` (before it was `lines 1..=20`).
- **[contract]** Metadata is escaped for Markdown minimally, so that when rendered they look as written and hide nothing: every `<!--` is written as `\<!--` (before, text up to the nearest `-->` became a comment); a value that starts a line or list item and would start a block there (`## x`, `> x`, `- x`, `1. x`, `---`, code fence, `<…`, table row separator, link definition) gets a `\` before this character
  (`Selector:` → `\## x`, `1\. x`); `|` in table cells is written as `\|` and no longer breaks the
  line `## Files`/`## Block Index`. Single-line-ness and anchor escaping are preserved,
  data is never escaped (§9).

### Navigation

- **[contract]** `search` (and MCP `search_text`) by default outputs in code and data files
  the entire truncated matching line, one hit per line; in prose (`.md`, `.txt`, `.rst`,
  `.adoc`, `.org`, `.tex`) — a sentence, as before. Before, a sentence was output everywhere, and
  on code the line was cut at every period (`get("port").`). New option `--context
  auto|line|sentence` (MCP: `context`). API navguard: `search_text(…, SearchContext)`.

### Known limitations (not fixed in 1.2.0)

- navguard (`map`, `symbols`, `lines`, `search`, `todos`) splits lines only by LF/CRLF:
  a file with only CRs is one line for it (the parser and `recon` see multiple).
- Markdown escaping in metadata does not touch inline markup (`*x*`, `` `x` ``,
  `[a](b)`, HTML tags except comments): when rendered, it is formatted, but the text is visible.
  Data is not escaped: `<!-- x` in an item when rendered opens an HTML block up to the nearest
  `-->` (in raw text everything is in place).
- The minimum working `--max-block-chars` for a split group increased by 18 characters
  (the parent text `(in the continuation parts)` is longer than `(no data)`).
- Items from 1.0.4 about XML `inner_html`/`outer_html` (references to internal entities are not
  expanded), `&lt;` in an entity value in an attribute, and fractional JSON numbers greater than 2^53
  remain in effect.

## 1.1.0 — 2026-10-19

One portable program for AI agents: data extraction and project navigation in a single exe, with key commands, an MCP server, and built-in English help. The report mode `-i -t -o` works as before; its output is unchanged byte-for-byte.

### New

- **Commands** (`any_txt_parser <command>`, response on stdout, `--json` for structure):
  `extract` (rules directly in arguments: `-r contains:ERROR`, `-r "jsonpath:$.a[*]"`,
  repeatable `-r`, or `-t template`), `recon` (format, encoding, structure, and first lines of
  files, without extracting anything), `map`, `symbols`, `find`, `lines`, `search`, `todos`,
  `changes` (project navigation), `nav` (full navguard command line), `mcp`,
  `help [topic]`. Without arguments, the program prints a list of commands (code 3).
- **Short rules `TYPE:EXPR`**: contains, starts_with, ends_with, regex, jsonpath, path,
  xpath, css, line (`line:10-20`), markdown (`markdown:heading:^Install`). The expression is taken
  as-is, after the first `:`. A short rule reads only files of formats that contain its data type; others are skipped with a note (not a warning).
- **`extract` response for AI**: results with no matches are not printed but counted in a note
  (`--include-empty` returns them); a file is not considered partial just because a rule found nothing in it; a rule that found nothing in any file is a warning, code 1.
  Paths are printed with `/`. `--max-output-chars` limits the response size and states where it stopped.
- **MCP server** (`any_txt_parser mcp`, JSON-RPC over stdio, protocols 2024-11-05 …
  2025-06-18): ten tools — `help`, `recon`, `extract`, `map`, `symbols`,
  `find_symbol`, `read_lines`, `search_text`, `todos`, `changes`. By default, responses are limited (50,000 characters of data, 1000 files, 200 matches, 2000 lines). All tools are read-only except `changes` with `persist: true`.
- **Built-in help** in English with examples (`help`, topics overview, extract, rules,
  recon, navigation, mcp, reference) and `README.md`; `README.md` is placed in `dist\` next to the exe.
- **navguard built-in**: module `crates/navguard` (formerly separate navguard 0.1.0).
  Fixed per review: symbol search sees nested methods, more languages and kinds
  (Rust `pub(crate)`, trait, type, macro; Python async; JS/TS export, interface, arrow functions; Go; C/C++/Java/C#/Kotlin), symbol boundaries by brackets for bracketed languages; tree is built from the list of scanned files, with `/` and correct branches; search in the middle of a line no longer shifts the line number; navguard tests run in parallel.
- `LICENSE` (MIT).

### Behavior changes

- **[contract]** `any_txt_parser` without arguments prints a list of commands (still code 3).
- **[contract]** `nav` without a mode does not start hourly folder watching but asks to specify a mode (code 3).

## 1.0.4 — 2026-10-18

### Exit codes and statuses (§4, §7.9)

- **[contract]** A run in which **all** input files received the status `error` exits with code `3` (previously `1`). The document is still published: `Run status: failure`, per-file errors under `## Diagnostics`; on stderr — `any_txt_parser: no input file could be processed
  (N of M failed); the errors are listed under Diagnostics in '<out>'`, even with `-q`.
  CLI errors with code 3 still publish nothing and do not touch the document from a previous run with the same `-o`, so these two cases must be distinguished by stderr, not by the presence of a document.
- **[contract]** The file status `error` now means "nothing was extracted from the file": a read, decoding, or parsing error, or all rules that were applied to it failed (rules with `rule not applicable` are not counted). A single rule failure alongside working rules gives the file `warning` (previously `error`).
- **[contract]** `Run status: failure` — only when all files are `error`. One failed file among successful ones now yields `partial_success` (previously `failure`, with exit code 1).
- **[contract]** `when.format` became a filter: for files of a different format, the rule is skipped without a block, without a warning, and without affecting the file status. Previously such files received a `rule not applicable` block, status `warning`, and exit code 1, so a template targeting multiple formats could not finish with exit code 0. The phrase `A rule with "Applies to format" runs only on source files of that format; …` was added to `## Result Semantics`.
- **[contract]** A `when.format` different from the fixed format (`--format` or the template's `input.format`) is no longer a template error (previously exit code 2:
  `when.format 'json' contradicts resolved input format 'text'`). The rule simply does not apply in this run; this is noted in `## Applied Template`
  (`Applies to format: json (never applies in this run: …)`), in the `--dry-run` plan, and with `-v`
  (`Rule 'x' never applies: …`). Therefore, the `## Effective Template` of a run with `--format` is suitable for reuse.
- New: if the fixed format excludes all rules, every read file receives `warning` (exit code 1), the first line of `## Diagnostics` is
  `No rule applies in this run: …; nothing was extracted`, the same appears in `## Result Semantics`,
  on stderr (`any_txt_parser: warning: …`, except with `-q`), and in `--dry-run`.

### Truncation by transformations (§7.7)

- **[contract]** `truncate_chars`, `truncate_lines`, and `limit_items`, if they cut anything, are considered truncation: `Truncated: yes`, file `warning`, run `partial_success`,
  exit code 1 (previously `Truncated: no` and exit code 0). Each such step produces one line in
  `### Warnings` and `## Truncation`, after the limit lines:
  `3 item(s) truncated by truncate_chars(3): item 1 (8 -> 3 chars), item 2 (8 -> 3 chars); 1 not published`,
  `1 item(s) truncated by truncate_lines(2): item 1 (5 -> 2 lines)`,
  `4 item(s) omitted by limit_items(1)`. If the truncated items are then joined by `join`, the line
  looks like `item 1 (3 joined items cut to 3 chars)`; truncating an already joined value gives
  the usual form `item 1 (30 -> 3 chars)`.
  Numbers are the positions of published items (after subsequent transformations, `max_items`, and
  `stable_deduplicate`); "not published" — truncated items removed later.

### JSONPath per RFC 9535 (§7.1)

- **[contract]** Comparison with a missing field: `!=` yields true (records without the field are no longer
  silently discarded), `==`, `<`, `<=`, `>`, `>=` — false, two missing fields are equal.
- **[contract]** The filter `[?(...)]` on an object selects matching values of its members in
  original order (previously — nothing): `$.services[?(@.enabled == true)].port`.
- Decimal and exponential literals in filters (`1.5`, `-2.25`, `1e3`, `1E+3`,
  `2.5E-2`); previously `@.t > 1.5` caused a template error. Integer and double comparison is exact.
  Template error (exit code 2): `1.`, `.5`, `+1`, `1e`, overflow `1e400`, non-zero literals
  underflowing to 0 (`1e-400`), integers outside [-2^63, 2^64) (previously silently rounded).
- **[contract]** `<=`/`>=` mean "`<`/`>` or `==`" (`@ <= true` selects `true`,
  `@ >= null` — `null`); `==`/`!=` compare arrays and objects structurally.
- **[contract]** Quoted strings (`['имя']`, literals, `@['k']`) decode
  `\\ \/ \b \f \n \r \t \uXXXX` (with surrogate pairs) and their escaped quote.
  Backslash is no longer literal: key `a\b` is now written as `$['a\\b']`, while the previous
  notation `$['a\b']` silently searches for a key with a backspace character (`\b` — a valid escape). Unknown
  escapes (`\x`, `\d`), foreign quotes, incomplete `\u`, unpaired surrogates, and unclosed strings are
  template errors (exit code 2). The `path` selector does not decode escape sequences.

### HTML (§7.4, §7.6)

- **[contract]** `extract: text` reads as in a browser: inline tags do not insert spaces
  (`H<sub>2</sub>O` → `H2O`, before `H 2 O`); markup whitespace is collapsed
  (`a\n     b` → `a b`); block elements and `<br>` produce line breaks (`<p>A</p><p>B</p>` →
  `A\nB`, before `A B`); table row cells are separated by tabs, empty cells retain the tab; text from `script`, `style`, `noscript`, `template`, `head`, `title`, and fallback content of `iframe`/`video`/`audio`/`canvas` does not enter the parent's text
  (before it did). A directly selected `script`/`title` still yields its own text;
  `head` — empty value.
- **[contract]** Matches with an empty value (empty text, empty `inner_html`,
  `alt=""`) are published as items and counted in `Items:` (before they were silently discarded, and the rule could receive `no matches found`). To remove them — `drop_empty`. An element without an attribute still does not produce an item.

### XML (§7.3, §7.6)

- **[contract]** `extract: attr:NAME` on an xpath ending with an element returns
  the attribute value (like `/@NAME`, no trim, empty — item, missing — no item,
  root prefixes work: `attr:xml:lang`). `extract: inner_html` — exact source text
  between tags. Before both modes were ignored and the markup of the entire element was returned.
- **[contract]** Modes `attr:`/`inner_html`/`outer_html` on an xpath ending with
  `@attr` or `text()`, as well as `attr:` without a name or `attr:@id` — template error (code 2).
- XML with DOCTYPE (plist, XHTML, MyBatis, configs) is no longer rejected
  (`invalid XML: XML with DTD detected`): internal entities are expanded in text and
  attributes, external DTDs and entities are never loaded (`&nbsp;` from XHTML DTD —
  file error `unknown entity reference`). Dangerous DTDs are rejected before parsing as a file
  error `XML DTD rejected: …`: expansion exceeding 8 MiB, copying more than 1 GiB of text,
  more than 64 Mi entity comparisons or 1 GiB of compared names, nesting deeper than 10,
  recursion, unbalanced markup in an entity.
- The depth limit of 128 accounts for elements from entities
  (`… (entity 'deep' adds 100 levels of elements)`).
- **[contract]** Unprefixed names in XPath match both default namespace elements
  declared at the root (pom.xml, Atom, SVG, XHTML): `/project/version/text()`
  now works on pom.xml (before — `no matches found`). Elements with `xmlns=""` are considered
  elements without a namespace. A default namespace below the root is not selected by unprefixed names.

### YAML (§6, §7.2)

- Multi-document files separated by `---` are processed (before it was a file error
  `deserializing from YAML containing more than one document is not supported`): each
  rule is executed on each document, results — in document order. Empty and
  `~` documents are skipped, anchors do not work between documents, an error in document
  N ≥ 2 — `invalid YAML in document N: …`. A trailing `---` (`a: 1\n---\n`) is no longer
  an error.
- **[contract]** Merge keys `<<: *a`, `<<: [*a, *b]` are expanded before search: own keys
  take precedence, earlier sources take precedence over later ones, chains are expanded, keys go to
  the place of `<<`; the key `<<` is no longer in the output. `<<` with a non-mapping value
  (`<<: 5`, `<<: ~`) remains an ordinary key; quotes do not matter (`"<<": {a: 1}` merges).
- **[contract]** Sniffing: text from multiple documents `---` without an extension is determined
  as yaml if each non-empty document looks like a config; a document consisting only of comments
  `#` is considered empty (so front matter with a body of only headings `# …` is now also yaml).

### Effective Template (§8, §9)

- **[contract]** `## Effective Template` — a valid template in canonical form: it can be
  saved to `.yml` and passed to `-t` without edits, it compiles to the same configuration.
  The appearance changed: `line` is written as `{type: line, from, to}` (was
  `expression: "lines N..=M"`), `markdown` — as `{node, pattern}` with the canonical name
  of the node (was `expression: "node=… pattern=…"`), `regex_replace` — as `{from, to}` (was
  `{pattern, replacement}`), numeric `expression` in `contains` is preserved (`'404'`, was
  empty), multi-line strings — in double quotes with `\n` (was `|-`).
- **[contract]** Lines with anchor text in Effective Template are written in double quotes with
  `:` as `\x3A`, so general anchor escaping does not affect them and decoding
  YAML before reuse is no longer necessary. Ordinary templates are output the same
  as before.

### unescape (§7.7)

- **[contract]** A surrogate pair from two escapes (`\uD83E\uDD80`) yields one character (🦀);
  before both halves were lost.
- **[contract]** Nothing is lost: a single or unpaired surrogate, `\uZZZZ`, `\u12`,
  `\u+041`, `\u{41}` and `\u` at the end remain in the text literally (before `\u` and up to four
  following characters were removed, and `\u+041` yielded `A`). Text after an invalid `\u`
  is parsed again: `\u12\n` → `\u12` and a line break.

### Review fixes from commit c91533b (not included in released 1.0.3)

1. **[contract]** Anchor text in sources (`<!-- BLOCK: B0001 END -->`) no longer aborts
   execution with code 4 and does not forge anchors: in data and metadata each
   sequence `<!-- BLOCK\…:` receives one more `\` (reversibly). Internal validation
   checks anchors line by line in a single pass.
2. A file with the `.auto` extension or no extension (`json`) is determined by format sniffing, rather than
   causing the program to panic (previously code 101). The CLI runs in a separate thread with a stack
   of 64 MiB; any panic results in code 5.
3. XML deeper than 128 elements — file error, not a stack overflow crashing the process;
   other files are published.
4. **[contract]** `max_item_chars` truncation — one summary warning per rule with
   numbers of the first five items (previously one line per item); applied after
   `max_items`, numbers counted after `stable_deduplicate`; parts of a split block do
   not repeat the parent's warnings.
5. Block splitting is linear in number of lines and items (per commit description: 200,000 short
   lines — 1.3 s instead of 69 s in debug build; in release build one item from 200,000
   lines splits in 0.1 s versus 0.6 s in 1.0.3).
6. `path` accepts TOML table headers (`[database].host`, `[servers.alpha].ip`).
   **[contract]** `$..key`, negative indices and slices in `path` — template error (code 2);
   previously `$..key` was silently read as `$.key` (and usually returned `no matches found`).
7. **[contract]** Sniffing is stricter: logs, prose and PEM with colons remain text (previously
   lines like `2026-10-01 12:00:00 ERROR: x` and PEM certificates were identified as yaml);
   strict TOML is checked before YAML (a file starting with `[table]` was previously text);
   YAML — only in "configuration" form; too deep XML is considered XML, only
   if all tags are paired (`<34>` syslog and `<nick>` remain text).

Also: a command-line string argument with invalid Unicode (for example, `-i` or
`--format` with an unpaired UTF-16 surrogate) results in code 3 with a clap message, not a panic with code
101; indices like `[+1]` in the `path` selector still work.

### Release and documentation

- New `scripts\release.ps1` and `scripts\release.bat` — the only release path: version
  match check in `src/lib.rs`, Cargo.toml, Cargo.lock and HELP.md header, check of
  occupied assignments, `cargo fmt --check`, `cargo test --locked`,
  `cargo build --release --locked`, copying exe and HELP.md to `dist\` and to skills
  `.dsh`/`.pi`/`.hermes`, version update in SKILL.md, SHA256 verification, manifest. Options
  `-DryRun`, `-SkipTests`, `-SkipBuild [-ExePath]`, `-NoSkills`, `-RepoRoot`.
- **[contract for maintainers]** `build_release.bat` now runs the full release
   (previously only `cargo build --release --locked`); arguments are passed to the script.
- `HELP.md` updated; this `CHANGELOG.md` added.

### Found by fuzzer (§9)

- New `tests/fuzz.rs`: random input generators for all seven formats and random
  templates. Checks absence of panics, correctness of anchors, Block Index lines and character
  counter, block limits, data independence from splitting, determinism of two runs and
  Effective Template reuse. Long run:
  `ATP_FUZZ_ITERS=50000 cargo test --release --test fuzz`, case replay — `ATP_FUZZ_REPLAY`.
- **[contract]** HTML `inner_html`/`outer_html` output attributes in name order. Previously
   the order could change from run to run, meaning output was not deterministic.
- **[contract]** Metadata (paths, rule ids, keys, selectors, error messages, table cells)
  is written as a single line: newline inside a value is output as `\n`, `\r` as
  `\r`. Previously a parser message quoting a source line could add to the document
  forged Block Index lines and headers.
- Integers greater than 64 bits in YAML are read the same way as in JSON (`1e+29`), including during
   sniffing.

### Known limitations (not fixed in 1.0.4; except for the last three items, fixed in 1.2.0)

- `document.stable_deduplicate` compares values after `max_item_chars`: different records with
  a common beginning may be removed as duplicates (this is noted by the line "dropped as
  duplicates", but data is lost).
- A single CR (old Mac format) is not considered a newline in the text adapter, and `\r`
  from CRLF may end up in XML fragment output.
- TOML tables are published with keys alphabetically, not in file order.
- Metadata does not escape Markdown: a selector like `## x` or `<!-- x` in Applied Template
  becomes a header or comment during rendering (data is unaffected, anchor and index structure
  is protected).
- Self-description: parent of a split group has `Items: 0`; Applied Template has no `extract` mode, markdown selector shown in debug form; encoding of each file and exit code are not output to the document; `rule not applicable` does not specify the reason.
- XML: a file with DOCTYPE without the `.xml` extension under `format: auto` is identified as text;
  `inner_html`/`outer_html` leave references to internal entities unresolved; an entity with
  `&lt;` in an attribute value — file error (roxmltree).
- JSON fractional numbers greater than 2^53 may be rounded by one unit in the least significant digit.

## 1.0.3 — 2026-10-14

Stabilization (released exe: `dist\any_txt_parser.exe`, SHA256 `C021D9FC…2753A62`).
Before — released 1.0.2.

- JSONPath: `&&` binds tighter than `||` (before `a || b && c` was evaluated left to right);
  recursive `..*` (before template error); exact equality of integer and fractional without
  rounding through double (before `9007199254740992.0 == 9007199254740993` was true).
- **[contract]** XPath: position `[N]` is counted among selected children of each parent;
  `//` preserves document order and does not duplicate nodes (`//i[1]/text()` on
  `<a><i>1</i><i>2</i></a><a><i>3</i></a>` → `1`, `3`; before `3`, `2`, `1`); endings
  `text()`, `//text()`, `@attr`; prefixes by root declarations; element is output as exact
  original fragment.
- XML/Text/Markdown adapters read source text without copying (removed memory leak
  XML). **[contract]** Nested Markdown nodes are selected (nested `list_item` — separate
  item; before its text was glued with parent: `anested`).
- **[contract]** Mixed list of transformations is executed strictly in order of writing
  (`[{join: ","}, {truncate_chars: 3}]` → `aaa`, before `aaa,bbb`); rule limits — after.
- **[contract]** `--max-memory` — total budget of decoded UTF-8 text for entire
  run (before per file); this is not a limit of process RSS. Files are processed one by one.
- **[contract]** Document and block limits count all rendered text (metadata and
  anchors); excess gives code 6 and preserves previous output. CLI output limit is more important
  than template.
- Splitting large items without loss (by lines, then by Unicode characters),
  `Item continuation: N` — number of original item.
- **[contract]** File reading/decoding error — error of this file, others
  are processed (before invalid UTF-8 broke entire run with code 6).
- Hash is calculated on read bytes; atomic publication without backup deletion of old
  file.
- Tests `tests/stabilization.rs` and common helpers `tests/support`.

## 1.0.2 — 2026-08-21

First version in git (commit 0f41c7a), includes work from 1.0.0 → 1.0.2.

- The compact English `HELP.md` for AI — exactly this file ships with the exe.
- All `--help` flags received self-contained descriptions. Parser behavior did not change.
- From 1.0.1: **[contract]** CLI flags `--format` and `--encoding` actually override
  the template (in 1.0.0 they were parsed but ignored); invalid `--format`
  (including `auto`), `--encoding`, and SIZE arguments give exit code 3 instead of silent ignoring.
