# recon — look at files before writing rules

Describes files without extracting anything, so you can pick the right rule
without loading the files into your context.

    any_txt_parser recon <file|dir|glob> [--sample N] [--encoding ENC] [--max-files N] [--json]

## Examples

    any_txt_parser recon logs/
    any_txt_parser recon data/export.json --sample 5
    any_txt_parser recon "docs/**/*.md" --sample 0 --json

## What you get per file (first 50 files, --max-files changes it)

- size and line count;
- encoding: `utf-8`, `utf-8 (BOM stripped)`, `utf-16le (BOM)`, ...;
- format as `format: auto` would detect it, and why: `extension`,
  `content` (sniffed) or `content fallback`;
- `selectors`: the rule types that format takes;
- `shape`: JSON/YAML/TOML top-level keys (`users[3]` = list of 3, `meta{2}` =
  mapping of 2) or list length and the first item's keys; YAML documents in a
  multi-document file; XML root element, namespaces and child elements; HTML
  title and h1-h3; Markdown headings and node counts; or the parse error of
  the detected format;
- the first N lines (default 20), numbered so you can ask for `line:A-B`, each
  cut to 300 characters.

Encodings: without `--encoding`, a UTF-16 BOM is detected (a note says to pass
`--encoding utf-16le` to extract); text that is not valid UTF-8 is reported as
an error and shown decoded as windows-1251 when that works, with a note.

Exit 0, or 3 for a bad option or when no file matched.

## Example output

    any_txt_parser 1.5.0 recon: 2 file(s)
    == logs/app.log (79 bytes)
    encoding: utf-8
    format: text (extension)
    selectors: regex, contains, starts_with, ends_with, line
    lines: 3
    sample: lines 1-2 of 3
       1| 2026-10-01 ERROR disk full
       2| 2026-10-01 INFO ok
    == data/users.json (70 bytes)
    encoding: utf-8
    format: json (extension)
    selectors: jsonpath, path
    lines: 1
    shape: object with 1 key(s): users[2]
    sample: lines 1-1 of 1
       1| {"users":[{"name":"Ann","active":true},{"name":"Bob","active":false}]}
    == end: 2 file(s)
