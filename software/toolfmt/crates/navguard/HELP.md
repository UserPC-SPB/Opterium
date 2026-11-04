# navguard 0.1.0 — quick reference

Project navigator and symbol search, run as `any_txt_parser nav <mode>`.
`any_txt_parser nav --help` lists all modes.

    any_txt_parser nav --root PROJECT --oneshot --force
    any_txt_parser nav --root PROJECT --grep PATTERN
    any_txt_parser nav --root PROJECT --lines src/main.rs 1 80
    any_txt_parser nav --root PROJECT --watch --timeout 60
    any_txt_parser nav --root PROJECT --patch replace src/a.rs OLD NEW --dry-run

`--root`, `--force`, `--timeout` and `--dry-run` may appear before or after
the mode. Watch remains active with a fresh cache; timeout is measured in
seconds of inactivity (default 10800). Exit codes: 0 success, 1 failure.

Every path a mode reads or writes lies inside the root (`crate::paths`):
relative paths start at the root, absolute paths must point inside it, and
`..` or links leading outside are refused. Scans never follow symlinks or
junctions. `navguard.json` (`exclude`, `timeout_sec`, `triangle_path`) is
loaded from the root; a BOM is accepted.

`--oneshot` and `--watch` write `.navguard_cache.json` and three navigators
into the root, rendered together from one read of every file
(`crate::navigator`): `navigator.md` (Root, Updated, Triangle [0,1,0] = READY,
the tree with `deep L<n>` pointers), `navigator_deep.md` (one row per file:
bytes, lines, tokens, sha, `deep2 L<a>-L<b>`) and `navigator_deep2.md`
(symbols with line ranges, each section pointing back to its deep row).
Paths are not repeated outside the tree. Each file is replaced atomically.

Lines and tokens (`crate::text`): LF, CRLF and a lone CR end a line, an empty
file has 0 lines; tokens are about one per 3 bytes of UTF-8, a little high on
purpose. Binary files (a NUL in the first 8 KiB) have neither.

Every reconciliation compares the current file set and all content chunks.
Added/deleted/renamed files and middle edits update the symbol index. Merkle
roots use sorted paths and include empty files. `--force` rebuilds the cache.
Generated cache/navigator files are excluded even with an empty exclude list.
`--focus PATH` allows unchanged symbols to be reused for events under PATH;
events outside PATH trigger a full symbol rebuild.

Symbols are discovered with heuristics, not an AST. Rust, Python, JS/TS, Go,
C/C++, C#, Java and Kotlin have dedicated rules (comments and strings are
masked; ranges end at the matching brace, or by indentation for Python);
other files use a generic mode. Nested symbols (methods, inner functions,
anchors inside anchors) are indexed too. Caches written by older versions
re-index their symbols automatically.

`--patch replace|insert|delete` edits one file: it keeps the line breaks and
the BOM, writes atomically, prints the change with `--dry-run` instead of
writing, and exits 1 when nothing could be changed. Other modes: `--check`,
`--extract`, `--todos`, `--triangle` (workflow state in the root),
`--validate`, `--build`, `--merkle-verify`, `--set-rules`.

Library use: `navguard::api` returns the same data as plain, serializable
values (map, symbols, symbol search, line ranges, text search, TODOs,
changes since the last scan) and only writes the cache when asked to.

Tests run in parallel (`cargo test -p navguard`) and only touch temporary
directories.
