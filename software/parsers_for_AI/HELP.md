# any_txt_parser v1.5.0 — quick reference

Deterministic CLI extractor for AI use: reads files (JSON, YAML, TOML, XML, HTML, Markdown, plain text),
extracts data per a YAML template, and publishes a self-describing Markdown log.
Single portable exe, no dependencies. Strict exit codes, atomic publish, deterministic output.

## Commands (1.1): rules on the command line, project navigation, MCP

Since 1.1 the same exe also answers short commands on stdout — no template file, no report to read back.
Full built-in documentation with examples: `any_txt_parser help [overview|extract|rules|recon|navigation|mcp]`.
Navigation commands read only inside `--root` (default: the current folder), the MCP server only inside its
root (`mcp --root DIR`); `..`, other absolute paths and links pointing outside are refused (since 1.3).

    any_txt_parser recon logs/                                   # format, encoding, shape, first lines
    any_txt_parser extract -i logs/ -r contains:ERROR --context 1  # rules TYPE:EXPR, repeatable -r, --json
    any_txt_parser extract -i data.json -r "jsonpath:$.users[*].email"
    any_txt_parser map --root DIR | symbols FILE | find NAME [--exact] | lines FILE A B | search TEXT | todos | changes
    any_txt_parser search TEXT [FILE] [--context auto|line|sentence]  # auto: the line in code, the sentence in prose
    any_txt_parser nav --oneshot                                 # navigator.md -> deep -> deep2, exact line pointers
    any_txt_parser mcp --root DIR                                # MCP server on stdio (10 tools, 2 resources)

Shorthand rule types: contains, starts_with, ends_with, regex, jsonpath, path, xpath, css, line (`line:10-20`),
markdown (`markdown:heading:^Install`). Items of text rules carry their source line: `- L12: text`, context
lines `L11- ` (`help extract`); `--json` adds `lines[]` (and `spans[]` for multi-line items) parallel to `items`.
`find --exact` matches the whole symbol name. `todos` counts TODO/FIXME/HACK/XXX as whole words (not `todos`,
`TODO_LIMIT`); `todos --loose` is the old substring match. The rest of this page is the template/report mode reference.

## Invocation

    any_txt_parser -i <file|dir|glob> -t <template.yml> -o out.md [options]

- `-i` input file, directory (recursive), or glob pattern (`*`, `**/`). The `-o` file is never scanned.
- `-t` YAML template (DSL below). `-o` output Markdown file — required (missing → exit 3).
- Precedence: **CLI > template > default**. Overrides are recorded in the output (`Format source: CLI override`).

| Option | Effect |
|---|---|
| `--format FMT` | Force format for ALL files: `json\|xml\|html\|markdown\|yaml\|toml\|text` (aliases: md, htm, yml, txt/log/plain). Not `auto`. Independent of file extension. Invalid value → exit 3 |
| `--encoding ENC` | `utf-8` (default; BOM auto-stripped), `utf-8-sig`, `utf-16le`, `utf-16` (=LE), `utf-16be`, `windows-1251`/`cp1251`. Invalid value → exit 3 |
| `--dry-run` | Validate template and files, print a plan with per-file format hints. Publishes nothing. Exit 0 (or 2/3 on error) |
| `-v` / `-q` | Diagnostics to stderr / suppress the final message |
| `--hash` | Add sha256 of each source file to the output |
| `--max-files N` | Max discovered files (default 5000) |
| `--max-file-size SIZE` | Max size of one input file (default 200m; SIZE: bare bytes or k/m/g ×1024, e.g. 50m) |
| `--max-matches N` | Match cap per rule (before transforms); cut matches are reported as truncation |
| `--max-memory SIZE` | Cumulative decoded UTF-8 byte budget across ALL files (default unlimited); sources are processed one at a time. This is not a process RSS limit |
| `--max-output-size SIZE` | Document character limit (default: from template, else unlimited) |
| `--max-block-chars N` | Hard cap including block metadata and anchors (default 32000); oversized items are split. A cap too small for metadata fails with exit 6 |
| `--follow-symlinks` | Traverse symlinks (default: skipped) |

## Template (YAML)

    version: 1
    name: template_name            # required, non-empty
    input:                         # optional
      format: auto                 # json|xml|html|markdown|yaml|toml|text|auto
      encoding: utf-8
    output:                        # optional
      max_output_chars: 100000     # hard limit for the whole document (characters)
      block: {target_chars: 28000, max_chars: 32000}
    document:                      # optional
      sort: none                   # none|asc|desc — block sorting
      stable_deduplicate: false    # drop duplicate items (first kept; compared before max_item_chars)
      split_blocks: true           # split oversized blocks into numbered parts
    rules:                         # required, >= 1, unique ids
      - id: r1
        when: {format: json}       # optional filter: rule runs only on files of this format
        select:                    # required — see table below
          type: contains
          expression: "ERROR"
        context: {before: 1, after: 1}   # text selectors only: neighboring lines
        output: {key: errors}      # default key = rule id
        extract: text              # text | attr:NAME | inner_html | outer_html (xml/html)
        transforms: [trim]         # see below; order matters
        limits: {max_items: 10, max_item_chars: 500}

**Selectors by format.** A wrong type for the format is NOT an error — the rule gets status
`rule not applicable` (warning; the block's Warnings name the selector types the format takes), output still
publishes. `when.format` is a filter instead: files of other formats
get no block and no warning (a mixed-format run can exit 0). A `when.format` that differs from a fixed format
(`--format` or `input.format`) is not an error: the rule never applies (stated in Applied Template, `--dry-run`,
`-v`); if no rule applies at all, every file read is `warning` (exit 1), noted in `Diagnostics` and on stderr.

| Format | Selectors |
|---|---|
| json | `jsonpath` `$.users[*].name`, filters `[?(@.active == true)]`; `path` `$.a.b[0]` |
| yaml / toml | `path` `$.settings.timeout` (toml: `[database].host`) |
| xml | `xpath` `/catalog/item/@id`, `//item[@type='book']/title/text()` |
| html | `css` `h2`, `a[href]` (+ `extract: attr:href`) |
| markdown | `markdown` node `heading\|paragraph\|list\|list_item\|table\|table_row\|table_cell\|code_block\|blockquote\|link\|image` (aliases: h, p, li, tr, td, code, quote, img; + optional `pattern` regex) |
| text | line-based: `regex`, `contains`, `starts_with`, `ends_with`, `line {from: 1, to: 5}`. One capture group in a regex → the group is the item, else the whole line |

**HTML `extract: text`** reads an element like a browser's innerText (no CSS): inline tags add nothing
(`H<sub>2</sub>O` → `H2O`); whitespace runs collapse to one space, except in `pre`, `textarea`, `xmp`, `listing`,
`plaintext`; block elements (`p`, `div`, `li`, `tr`, `h1`…) end lines, each `<br>` adds one (empty lines come only
from `<br>` and from blank lines inside those preformatted elements); cells of a row are TAB-separated (empty cells
keep their TAB; inside a cell blocks and `<br>` are a space); trimmed. Skipped as not rendered: `script`, `style`,
`noscript`, `template`, `datalist`, `head`/`title`, fallback content of `iframe`, `video`, `audio`, `canvas`.
Selected directly, a text-only one yields its trimmed source text (`script[type='application/ld+json']`, `title`,
`style`, `noscript`); one with child elements (`template`, `datalist`) is rendered by the rules above; `head` → empty.
`attr:NAME` is trimmed. Empty values (text, `inner_html`, `alt=""`) are items, as in the other adapters (add
`drop_empty`); a missing attribute yields no item. One-line values: `normalize_whitespace`.

**Transforms.** The entire list executes in its written order, including mixed item/result operations. Rule limits apply afterwards. Item (per match): `trim`, `normalize_whitespace`, `lowercase`, `uppercase`,
`unescape`, `truncate_chars N`, `truncate_lines N`, `replace {from, to}`, `regex_replace {from, to}`.
Result (item list): `drop_empty`, `unique`, `sort`, `sort_desc`, `limit_items N`, `join "SEP"`.
`unescape` decodes only `\n \t \r \" \' \\` and `\uXXXX` (exactly 4 hex digits; a surrogate pair `\uD83E\uDD80`
→ one character). Everything else stays literally: `\/ \b \0 \x41 \u{41}`, a trailing `\`, `\u12`, lone surrogates.
`truncate_chars`, `truncate_lines` and `limit_items` that cut something count as truncation (`Truncated: yes`,
file `warning`, one warning line per step), e.g. `5 item(s) omitted by limit_items(10)`,
`3 item(s) truncated by truncate_chars(20): item 1 (86 -> 20 chars), item 4 (31 -> 20 chars); 1 not published`.
Item numbers are positions of published items. Items cut first and then glued by `join` show as
`item 1 (3 joined items cut to 20 chars)`; cutting an already joined value is the plain `item 1 (30 -> 20 chars)`.
`not published` = cut items removed later (`limit_items`, `max_items`, `unique`, `drop_empty`, `stable_deduplicate`).

JSONPath is a supported subset: no slices, negative indices or parentheses in filters; a filter needs a comparison
(no bare `[?(@.x)]`); `&&` binds before `||`. Filters follow RFC 9535: they select matching array elements and
object member values (`$.services[?(@.enabled == true)].port`). A missing field is "Nothing": `@.x != 1` is true
when `x` is absent, `==` `<` `<=` `>` `>=` are false (two missing fields are equal); `<=`/`>=` = `<`/`>` or `==`;
`==` on arrays and objects is structural. Numbers (`1`, `-2.25`, `1e3`, `2.5E-2`) parse like JSON: integers in
[-2^63, 2^64) exact, others IEEE doubles, compared exactly; `1.`, `.5`, `+1`, `1e400`, `1e-400` and integers
outside that range are exit 2 (write `1e20` for a rounded double). Quoted names and strings decode `\\ \/ \b \f \n \r \t \uXXXX` and
the escaped own quote (`'O\'Brien'`, `"say \"hi\""`); other escapes are exit 2, so a literal backslash is `\\`
(key `C:\new` → `$['C:\\new']`; `$['C:\new']` silently looks for a newline). In a YAML double-quoted
`expression` double every `\` again; YAML single quotes keep it (write `'` there as `''`).
`path` also takes TOML table headers: `[database].host`, `[servers.alpha].ip`; keys starting with a digit
need quotes (`["2024"]`). `$..key`, negative indices and slices are rejected (exit 2): write the full path
(`*` / `[*]` steps), or use `jsonpath` for JSON. `path` does not decode escapes: `['a\b']` is the key `a\b`.
YAML streams: a file of several `---` separated documents (k8s manifests) is one source; each rule runs on every
document, matches are concatenated in document order (`$.kind` over 3 documents → 3 items). Empty (`~`) documents
are skipped; anchors do not cross documents; an error in document N ≥ 2 reads `invalid YAML in document N: …`
and fails the file. Merge keys are expanded before matching (`<<: *base`, `<<: [*a, *b]`): own keys win, earlier
sources beat later ones, chains resolve, merged keys take the place of `<<`. A `<<` whose value is not a mapping
or a list of mappings (`<<: 5`) stays an ordinary key; quotes do not matter (`"<<": {a: 1}` is merged).
Duplicate keys: YAML/TOML → per-file error; JSON → the last value wins silently. Objects, mappings and TOML
tables keep the source key order (`{"min":2,"max":16}`, `$.servers.*` walks tables in file order).
XPath is a supported subset; query prefixes use namespace declarations on the XML root; unknown prefixes produce
a rule error. Unprefixed names match elements without a namespace and, when the root declares `xmlns="..."`
(pom.xml, Atom, SVG), elements of that default namespace: `/project/version/text()` works on a pom.xml. A default
namespace declared below the root is not matched; unprefixed attributes match only attributes without a namespace.
XML `extract` on an xpath ending at an element: `text` (default) and `outer_html` = the element's exact source;
`inner_html` = the exact source between its tags (`<a/>` → empty item); `attr:NAME` = what `/@NAME` gives (no
trim, empty kept, absent → no item; `attr:xml:lang`, root prefixes work). After `@attr`/`text()` other modes: exit 2.
XML with a DOCTYPE uses only its internal subset: external DTDs and entities are never loaded (`&nbsp;` from the
XHTML DTD → file error `unknown entity reference`). A file is rejected before parsing (file error
`XML DTD rejected: ...`) if its entities expand to more than 8 MiB, would make the parser copy more than 1 GiB of
text, need more than 64 Mi entity lookups or compare more than 1 GiB of entity names, nest deeper than 10 levels,
are recursive, or (used in content) contain unbalanced markup.
XML nested deeper than 128 elements, counting elements that come from entities, is a per-file error (the JSON,
YAML and TOML parsers stop at 127, 128 and 80). Markdown selection includes nested nodes in source order.
Integers beyond 64 bits are read as floats in JSON and YAML (`1e+29`); TOML rejects them (file error).
HTML `inner_html`/`outer_html` re-serialize the parsed element with attributes in source order (stable across runs).
**Line breaks:** LF, CRLF and a lone CR (classic Mac) each end a line, in every format. Decoded text gets LF for
all three before format detection, so line numbers, `line` ranges, context lines, recon samples and XML/HTML/
Markdown source fragments count the same lines, and no CR of a source's line endings reaches the output. A CR
that is part of a value (JSON `"\r"`, XML `&#13;`) is data and stays.

## Format detection (`format: auto`)
1. Extension: `.json .xml .html/.htm .md/.markdown .yaml/.yml .toml .txt/.log` (=text). Unknown, none
   (a file named `json`) or `.auto` → step 2.
2. Sniffing: valid JSON → json; `<!doctype html`/`<html` → html; valid XML → xml (XML too deep to parse
   counts only if all its tags pair up, so `<34>` syslog or `<nick>` chat logs stay text; XML with a DOCTYPE
   is parsed like the XML adapter parses it, internal subset included — a plist or MyBatis file without `.xml`
   is xml, and one whose DTD is rejected (entity bomb) is xml with that file error); parses as TOML with at least one key (may start with
   `[table]`) → toml; YAML only if the text has `: ` or starts with `---`, its first line is `---`, `- …` or `key:`,
   and every non-empty `---` document (`#`-only ones are empty) is a config shape — a top-level mapping whose keys
   are all identifier-like (`name`, `apiVersion`, `server.port`: no spaces) or a list of mappings. Log lines such
   as `2026-10-01 12:00:00 ERROR: x`, `Error:` headings and PEM files stay text; short `Word: text` notes
   (`Subject: …`) and front matter followed only by `#` headings still look like YAML.
3. Fallback: text. If sniffing guesses wrong, set `--format` or `input.format`.

## Exit codes
`0` success — every file `success` · `1` partial — some files warning/error but not all `error` (also: no rule
applies), output published · `2` invalid template · `3` invalid CLI/input (bad `--format`/`--encoding`/SIZE,
missing `-o`, no files found), or every input file is `error` — then the output IS published (`Run status: failure`,
errors in `Diagnostics`; stderr says `no input file could be processed`, even with `-q`) · `4` publish failure (or
failed internal output check — a bug) · `5` internal error — a bug, including any panic; report the stderr text ·
`6` resource limit exceeded. A CLI error publishes nothing and leaves an older output untouched: tell it from the
all-failed case by stderr or a changed `Generation`, not by the file's `Run status`.

## Output file
Starts with `# any_txt_parser output`, then:
- `Triangle: [0,1,0]` — "done" marker; `Generation: NNNNNNNN` — increments on every run (freshness check); `Updated: <UTC>`.
- Sections: `Execution` (incl. `Run status`, `Exit code`) · `Files` (per-file Format/Status/Encoding, the
  encoding the file was decoded with: `utf-8 (BOM stripped)`, `utf-16le (BOM)`, ...) · `Diagnostics` (file/rule errors;
  a run-level note when no rule applies) · `Applied Template` · `Effective Template` (actual settings incl. CLI
  overrides, as a reusable template) · `Result Semantics` · `CLI Arguments` · `Block Index` (block → line range).
- Metadata values (paths, rule ids, keys, selectors, messages) are written on one line: a line break inside one
  appears as `
` (`` as ``). Effective Template keeps the exact values.
- Metadata is escaped for Markdown, so a renderer shows it as written: every `<!--` is `\<!--`; a value that
  starts a line and would start a block there (`## x`, `> x`, `- x`, `1. x`, `---`, a code fence, `<...`) gets a
  `\` before that mark (`Selector:` `\## x`); `|` in table cells is `\|`. Data items are never escaped.
  Applied Template writes a selector as the shorthand does (`heading:^Install`, `10-20`) and `Extract:` when set.
- Data blocks are wrapped in `<!-- BLOCK: B0001 START -->` … `END -->`; block header carries
  `Source/Format/Format source/Rule/Status/Items`, then a `### Data` list.
- The document always ends with `End of any_txt_parser output`.
- `<!-- BLOCK:` appears only on anchor lines. In data and metadata, every `<!-- BLOCK` followed by
  `\`s and `:` gets one extra `\` (`<!-- BLOCK: x` → `<!-- BLOCK\: x`); remove one `\` from each such
  run to decode. Substring search for anchors is therefore safe.
- Split groups have a parent and numbered parts. The parent holds no data (`(in the continuation parts)`); its
  `Items:` counts the items of the whole rule result. `Item continuation: N` identifies fragments of original item N (1-based); concatenate their item strings in part order without adding separators, then decode.
- Warnings appear once per rule result: in the block (the parent of a split group) and in `## Truncation`.
  Items cut by `max_item_chars` (applied after `max_items`) are summarized in one line naming the first five
  item numbers, counted after `stable_deduplicate` (which compares the values before the cut, so records that
  only share the kept prefix both stay). Lossy transforms add one line per step after those lines.
- `Effective Template` is a valid template in canonical DSL form (`line {from, to}`, `markdown {node, pattern}`,
  `regex_replace {from, to}`, canonical format/node/encoding names, resolved defaults): save it as `.yml` and pass
  it to `-t` unchanged. Anchor escaping does not apply to it: a string with anchor text is double-quoted, its `:`
  written as `\x3A`, so plain YAML parsing restores the original.

**Reading it (for AI):** check the exit code first (also `Exit code:` under `Execution`), then the `Files` table; locate blocks by anchors or
Block Index lines — do not re-parse prose. Statuses: `success` = complete · `warning` = a rule found nothing /
was not applicable / failed next to working rules / was truncated by a limit or a lossy transform, or no rule
applies in the run (data may be partial) · `error` = nothing extracted: the file failed to read, decode or parse,
or every rule that ran on it failed (not-applicable rules do not count). `Run status`: `success` (exit 0) ·
`partial_success` (exit 1) · `failure` = every file `error` (exit 3). "rule not applicable" = selector type does
not match the format (e.g. `contains` on json) — fix format or selector; it is not a crash.

## Minimal examples
```text
# ERROR lines from any log file
-i app.log -t tpl.yml      # rule: select {type: starts_with, expression: "ERROR"}
# JSON field extraction
-i data.json -t tpl.yml    # rule: select {type: jsonpath, expression: "$.users[*].name"}
# force text processing for ANY extension (.md, .json, …)
-i notes.md -t tpl.yml --format text
```

## Guarantees
Deterministic for unchanged sources, paths, template and CLI except generation and execution timestamps.
Output/block limits count Unicode characters in the FULL rendered document/block. CLI output limit
overrides the template. Exceeding a hard limit fails with exit 6 and preserves the previous output;
`split_blocks: false` also fails if a block exceeds its cap. Atomic publish preserves the previous file
on failure. Invalid source encoding is a per-file error; other files still produce results (exit 1), and if
every file fails the run exits 3 with the errors published.
