# navigation — understand a code project without reading all of it

The navguard module scans a project, indexes symbols with line ranges and
prints exactly what you ask for. Scans skip `.git`, `target`, `node_modules`,
`__pycache__`, `.zvec-grep` (a `navguard.json` in the root can change that
list) and always navguard's own files: the three navigators,
`.navguard_cache.json`, `.navguard_rules` and the `nav --triangle` state and
log (`.abc_triangle_state.json`, `.abc_triangle_log.json`).

    any_txt_parser map      [--root DIR] [--max-files N] [--json]
    any_txt_parser symbols  [FILE] [--root DIR] [--json]
    any_txt_parser find     PATTERN [--root DIR] [--exact] [--case-sensitive] [--max-hits N] [--json]
    any_txt_parser lines    FILE START [END] [--root DIR] [--json]
    any_txt_parser search   PATTERN [FILE] [--root DIR] [--max-hits N] [--context auto|line|sentence] [--json]
    any_txt_parser todos    [--root DIR] [--loose] [--max-hits N] [--json]
    any_txt_parser changes  [--root DIR] [--persist] [--json]

`--root` is the project root, by default the current directory. Every file
a command reads must lie inside it: FILE is relative to the root or an
absolute path inside it. Paths that leave the root (`..`, an absolute path
elsewhere, a symlink or junction pointing outside) are refused with exit 3:

    WRONG  any_txt_parser lines C:\other\notes.txt 1 20          (outside the current folder)
    RIGHT  any_txt_parser lines notes.txt 1 20 --root C:\other

Scans never follow symlinks or junctions. Paths in the output are relative
to the root and use `/`. Nothing is written to the project, except
`changes --persist`.

Lines: LF, CRLF and a lone CR each end a line; an empty file has 0 lines and
a final line break does not start another one. Tokens are an estimate, a
little high on purpose so that a budget computed from it holds for any
model: about one token per 3 bytes of UTF-8 text. Binary files have neither.

## A typical session

    any_txt_parser map --root C:\work\shop                   # what is in the project, how big
    any_txt_parser find checkout --exact --root C:\work\shop # where is "checkout" defined?
    any_txt_parser symbols src/cart.rs --root C:\work\shop   # outline of one file
    any_txt_parser lines src/cart.rs 120 168 --root C:\work\shop   # read just that function

## map

The tree of the scanned files, then each file with its line count and token
estimate (binary files: their size):

    map C:/work/shop: 42 files, 6810 lines, ~61200 tokens
    ├── src/
    │   ├── cart.rs
    │   └── main.rs
    ├── Cargo.toml
    └── logo.png
    files (lines, ~tokens):
      Cargo.toml  20 ~170
      logo.png  binary, 20480 bytes
      src/cart.rs  410 ~4100

## symbols and find

Functions, methods, classes, structs, enums, traits, impls, interfaces, type
aliases, modules, macros and records, for Rust, Python, JavaScript/TypeScript,
Go, C/C++, Java, C#, Kotlin (other languages: a generic mode). Each symbol has
its kind, `start-end` lines (inclusive) and nesting (methods are indented
under their class or impl):

    == src/cart.rs (3)
      12-40  struct Cart
      42-130  impl Cart
        44-60  fn new
        62-128  fn checkout

`find` matches the pattern against symbol names and file paths
(case-insensitive by default): a symbol counts when its name, its first line
or its file path contains the pattern. `--exact` keeps only symbols whose
name equals the pattern (for a C++ `Foo::bar` also `bar`); use it to find a
definition when the name is also part of other names or paths:

    WRONG  any_txt_parser find parse            (also parser, parse_args, every symbol of src/parse.rs)
    RIGHT  any_txt_parser find parse --exact

    find 'checkout' (exact name): 1 symbol(s)
    src/cart.rs:62-128  fn checkout  (in Cart)

## lines

Numbered lines, inclusive; END defaults to START:

       62|     pub fn checkout(&mut self) -> Result<Order, Error> {
       63|         let total = self.total();

## search and todos

`search` finds a literal, case-sensitive text and returns, with file and line,
the whole trimmed line of the match in code and data files (one hit per line)
and the sentence around each match in prose (`.md`, `.txt`, `.rst`, `.adoc`,
`.org`, `.tex`). `--context line` or `--context sentence` forces one of them:

    any_txt_parser search get_config
    src/app.rs:41: let port = cfg.get_config("port").unwrap_or(80);
    any_txt_parser search "port" README.md --context line

For regexes, context lines or many files use `extract -i DIR -r regex:...`.

`todos` lists lines with a TODO, FIXME, HACK or XXX marker: the marker as a
whole word (no letter, digit or `_` touching it) in capitals, or in any case
when followed by `:`, `(` or `!` or preceded by `@`. `// TODO fix`,
`# todo: x`, `Fixme(bob)`, `todo!()`, `@todo` count; `todos`, `TODO_LIMIT`,
`todo_list`, `hackathon`, `a todo list` do not. `--loose` (MCP `loose: true`)
is the matching of 1.4.0 and earlier: TODO, FIXME or HACK anywhere in a line,
in any case — every `todos` in the docs counts too.

## changes

Compares the project with the last snapshot: `new`, `changed`, `deleted`
files. Without a snapshot every file is new. `--persist` saves the current
state as the snapshot (`.navguard_cache.json` in the root), so the next call
shows only what changed after it — useful between steps of a long task.

## nav — the full navguard command line

`any_txt_parser nav <mode> [--root DIR]` runs the navguard tool; `nav --help`
lists every mode. All of its paths lie inside the root too. Exit codes: 0
success, 1 failure (reason on stderr after `ERROR:`).

`nav --oneshot` (once) and `nav --watch` (again on every change) write three
navigator files into the root, rendered together from one read of every
file, so every pointer is an exact line number:

- `navigator.md` — read it in full. `Root:` (the absolute path, written only
  here), `Updated:` (local time of the scan), `Status: READY` (all three files
  are complete and from the same scan) and the tree. A file's full path is
  Root + the folders above it in the tree; each file points to its line in
  the next file: `cart.rs  deep L41`.
- `navigator_deep.md` — one line per file, in tree order: the path relative
  to Root, bytes, lines, tokens, sha and `deep2 L120-L131`, the lines of its
  symbols in the next file (`-` when none).
- `navigator_deep2.md` — per file the symbols with their line ranges in that
  file; each section names the file and its line in deep:
  `### src/cart.rs  (deep L41)`.

`Updated:` is the same in all three files. Only `--oneshot` and `--watch`
write them; `--check` reports on stdout and writes none. Over MCP the
resource `project://state` says whether they still match the files on disk.

`nav --patch` edits a file; `--dry-run` shows the change and writes nothing:

    any_txt_parser nav --patch replace src/app.rs "port = 80" "port = 8080" --dry-run
    any_txt_parser nav --patch insert src/app.rs 12 "use std::fs;"
    any_txt_parser nav --patch delete src/app.rs 40 52

Line numbers are those of `lines`. The file keeps its line breaks and BOM and
is replaced atomically. Text not found, a line out of range, a path outside
the root, a binary or non-UTF-8 file: exit 1, nothing written.

Other modes: `--check` (report on stdout, one line per item: `T:` workflow
state, `F: path|lines|tokens|sha` per file, `I:` an unfinished verification,
`B:` summary; `--brief` keeps T/I/B, `--diff` lists only changes since the
last snapshot and saves a new one in `.navguard_cache.json`), `--grep`,
`--lines`, `--extract`, `--todos`, `--triangle` (workflow state `[A,B,C]` in
`scripts/.abc_triangle_state.json` of the root, times in ISO 8601 with the
offset), `--validate`, `--build`, `--merkle-verify`, `--version`. A
`navguard.json` in the project root sets `exclude` (folder or file names;
a name containing an entry is skipped) and `timeout_sec`.

## Over MCP

Tools `map`, `symbols`, `find_symbol`, `read_lines`, `search_text`, `todos`,
`changes` take the same arguments as JSON (`root`, `path`, `pattern`, `start`,
`end`, `max_hits`, `context`, `persist`, `output`; `exact` and
`case_sensitive` for `find_symbol`, `loose` for `todos`). `root` must lie inside the
server root (`mcp --root DIR`, see `help mcp`); without it the server root is
used. MCP calls are capped by default: 1000 files for `map`, 200 hits, 2000
lines per `read_lines`. Resources `project://tree` and `project://state`
(see `help mcp`) give the tree and the freshness of the navigator files.
