# rules — how to say what to extract

## Shorthand: TYPE:EXPR

The rule is split at the FIRST `:`. EXPR is the rest, taken verbatim — no
trimming, quoting or escaping — so `regex:(?i)error|fail`,
`contains:key: value` and `xpath://a[@x='1']/@y` work as written
(`contains: x` searches for " x", with the space).

| TYPE | EXPR | reads |
|---|---|---|
| `contains` | text inside a line | text |
| `starts_with` | text at the start of a line | text |
| `ends_with` | text at the end of a line | text |
| `regex` | regex per line; exactly one capture group → the group, else the whole line | text |
| `jsonpath` | `$.users[*].name`, `$.items[?(@.price > 10)].id`, `$..id` | json |
| `path` | `$.server.port`, `servers[0].name`, `[database].host` (TOML table) | json, yaml, toml |
| `xpath` | `//item/@id`, `/root/a/text()`, `//item[@type='book'][1]` | xml |
| `css` | `h2`, `a[href]`, `table tr`, `div.note > p` | html |
| `line` | `5`, `10-20`, `10-` (to the end), `-20` (from 1); the range is ONE item | text |
| `markdown` | `NODE` or `NODE:PATTERN` (regex on the node text) | markdown |

Markdown nodes: heading, paragraph, list, list_item, table, table_row,
table_cell, code_block, blockquote, link, image (aliases h, p, li, tr, td,
code, quote, img).

A shorthand rule reads only files whose format holds its kind of data and
skips the others (a note counts them). To read a file differently, force the
format: `--format text` reads JSON, HTML, ... as plain lines.

## Rule objects

The full form is a template rule. Over MCP pass it as a JSON object; on the
command line pass YAML/JSON flow text starting with `{`:

    {"id": "links", "select": {"type": "css", "expression": "a[href]"},
     "extract": "attr:href", "transforms": ["unique"], "limits": {"max_items": 50}}

    -r '{select: {type: regex, expression: "code=(\d+)"}, transforms: [unique, sort]}'

Fields: `id` · `when: {format}` · `select: {type, expression}` (markdown:
`{type: markdown, node, pattern}`; line: `{type: line, from, to}`) ·
`context: {before, after}` · `output: {key}` · `extract` · `transforms: [...]`
· `limits: {max_items, max_item_chars}`. Without `id` a rule is r<N> by
position; the key defaults to the id. Rule objects do not skip formats: a
selector that does not fit a file's format gives `rule not applicable` for it
(a warning that names the selector types that format takes). Use
`when: {format: html}` to limit an object rule to a format.

## extract modes (html, xml)

- `text` (default): HTML text the way a browser reads it (inline tags join
  words, block tags separate them, script/style are left out); for an xpath
  that ends at an element, the element's exact XML source.
- `attr:NAME`: the attribute value (`attr:href`, `attr:xml:lang`).
- `inner_html` / `outer_html`: the markup inside / including the element.

## Transforms (in the order written)

Per item: `trim`, `normalize_whitespace`, `lowercase`, `uppercase`, `unescape`,
`truncate_chars: N`, `truncate_lines: N`, `replace: {from: a, to: b}`,
`regex_replace: {from: '(\d+)', to: '<$1>'}`.

On the list: `drop_empty`, `unique`, `sort`, `sort_desc`, `limit_items: N`,
`join: ", "`.

Transforms that drop or cut data (`truncate_*`, `limit_items`) mark the
result truncated and say what was cut — the output never loses data silently.
Limits run after transforms: `max_items`, then `max_item_chars`.

## Examples

    contains:ERROR
    regex:^(\d{4}-\d{2}-\d{2}) .*FATAL
    regex:user=(\w+)
    jsonpath:$.orders[?(@.total > 100 && @.status != 'paid')].id
    path:$.services.*.image
    path:[tool.poetry.dependencies]
    xpath://dependency/artifactId/text()
    css:meta[name=description]          (with --extract attr:content)
    line:1-40
    markdown:code_block
    markdown:heading:(?i)^install
