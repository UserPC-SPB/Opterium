# extract — pull data out of files

Get exactly the lines, values or elements you need from one file, a folder or
a glob, printed on stdout. No template file, no report to read back.

    any_txt_parser extract -i <file|dir|glob> -r RULE [-r RULE ...] [options]
    any_txt_parser extract -i <path> -t template.yml        (a template instead of -r)

`-r` is repeatable; rules are named r1, r2, ... in order. A rule is a
shorthand `TYPE:EXPR` such as `contains:ERROR` or a rule object — see
`any_txt_parser help rules`.

## Examples

    any_txt_parser extract -i logs/app.log -r contains:ERROR --context-after 1
    any_txt_parser extract -i "logs/*.log" -r "regex:code=(\d+)" -r "regex:(?i)timeout|refused" --transform unique
    any_txt_parser extract -i users.json -r "jsonpath:$.users[?(@.active == true)].email"
    any_txt_parser extract -i k8s/ --format yaml -r path:spec.replicas
    any_txt_parser extract -i Cargo.toml -r "path:[package].version"
    any_txt_parser extract -i pom.xml -r "xpath:/project/dependencies/dependency/artifactId/text()"
    any_txt_parser extract -i page.html -r "css:a[href]" --extract attr:href --transform unique
    any_txt_parser extract -i README.md -r "markdown:heading:^Install" -r line:1-5
    any_txt_parser extract -i project/ -r contains:TODO --json

A folder may mix formats: a shorthand rule only reads the files whose format
can hold its kind of data (`contains` reads text files, `jsonpath` JSON, `css`
HTML, ...) and skips the others; a note says how many files it skipped.

Files where a rule found nothing are left out and counted in a note
(`rule 'r1' found nothing in 37 file(s): ...`), so a search over a big folder
prints only the hits. A rule that found nothing in any file it read is a
warning (exit 1). `--include-empty` lists every file/rule result.

## Options

    -i, --input PATH          file, directory (recursive) or glob
    -r, --rule RULE           shorthand TYPE:EXPR or a {...} rule object (repeatable)
    -t, --template FILE       a YAML template instead of -r (see: help reference)
    --format FMT              json|xml|html|markdown|yaml|toml|text for every file (default: detect per file)
    --encoding ENC            utf-8 (default, BOM stripped), utf-8-sig, utf-16le, utf-16be, windows-1251
    --context N               N lines before and after each text match (numbered L<n>-)
    --context-before N, --context-after N
    --extract MODE            html/xml: text | attr:NAME | inner_html | outer_html
    --transform T             repeatable, applied in order: trim, unique, sort, "limit_items: 20", ...
    --max-items N             keep at most N items per rule and file
    --max-item-chars N        cut each item to N characters
    --max-output-chars N      cap on all returned item text; the output says where it stopped
    --max-files N  --max-file-size SIZE  --max-matches N  --max-memory SIZE
    --hash                    sha256 of each source file
    --follow-symlinks
    --include-empty           also list file/rule results that found nothing
    --json                    JSON instead of text

`--context*`, `--extract`, `--transform`, `--max-items`, `--max-item-chars`
apply to every shorthand `-r` rule; rule objects carry their own settings.

## Output (text)

    any_txt_parser 1.5.0: success, exit 0; files 2 (success 2, warning 0, error 0); results 1; items 2
    note: rule 'r1' (contains) skipped files it cannot read: 1 json
    == logs/app.log [text] rule r1 (success, 2 items)
    - L1: 2026-10-01 ERROR disk full
    - L3: 2026-10-02 ERROR timeout code=42
    == end: success, exit 0

- Line 1: run status, exit code, file and item counts.
- `diagnostic:` lines: files that could not be read or parsed.
- `note:` lines: skipped formats, rules that read nothing, output caps.
- Per result: `== <file> [<format>] rule <id> (<status>, N items)`, then the
  items as `- first line`; continuation lines of a multi-line item are
  indented by 4 spaces; then `warning:` / `error:` lines of that result.
- Last line: `== end: <status>, exit <code>`.

Items of text rules (`contains`, `starts_with`, `ends_with`, `regex`, `line`)
start with their 1-based source line, grep style, so you can jump to it
(`lines`, `nav --patch`): `L<n>: ` on the matched line, `L<n>- ` on context
lines and on the other lines of a `line:A-B` range; every line of the item
is numbered. The prefix is not part of the item (JSON has the item alone).
With `--context 1`:

    - L41- fn load() {
        L42:     panic!("ERROR: no config");
        L43- }

If a transform changed the number of lines of an item (`truncate_lines`,
`--max-item-chars`, `trim`, ...), only its first line is marked, with the
source lines it came from: `- L41-L43: fn load() {`. `join` merges items:
no line numbers. Other selector types (`jsonpath`, `path`, `xpath`, `css`,
`markdown`) have none.

`--json` prints one JSON object: `version`, `run_status`, `exit_code`,
`files[]` (path, size, format, format_source, encoding, status, error?, hash?),
`results[]` (source, format, rule, key, status, items[], lines[]?, spans[]?,
truncated, warnings[], errors[]), `diagnostics[]`, `notes[]`,
`truncated_output`. On an error before any file is read, `--json` prints
`{"exit_code": N, "message": "..."}`.

`lines` (text rules only) is parallel to `items`: the source line of each
item's match (the first line of a `line` range). `spans` is there when an item
covers several lines (context, a `line` range): `[first, last]` per item.

    "items": ["2026-10-01 ERROR disk full\n2026-10-01 INFO ok"], "lines": [1], "spans": [[1, 2]]

## Status and exit codes

- `success` / exit 0: every file read, every rule found data somewhere,
  nothing cut.
- `partial_success` / exit 1: a rule found nothing in any file, data was cut
  (limits, transforms, output cap), a file failed, or a rule read no file.
- exit 2: a rule or template is invalid (the message says what to fix).
- exit 3: bad option or input, no file matched, or every file failed.
- exit 6: a resource limit (files, size, memory) was exceeded.

Shell quoting: put rules in quotes when they contain spaces, `$`, `|`, `*`,
`[`, `(`, `>` — single quotes in bash and PowerShell, double quotes in cmd.exe.
Over MCP there is no quoting: rules are plain JSON strings.
