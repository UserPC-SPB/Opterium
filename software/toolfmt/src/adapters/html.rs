use super::{Adapter, RawMatch, SourceTree};
use crate::template::{CompiledRule, ExtractMode, Format, SelectorCompiled};
use scraper::{ElementRef, Html, Node, Selector};

pub struct HtmlAdapter;

/// Elements a browser does not render: `display: none` in the HTML UA
/// stylesheet, `noscript` (scripting is on) and the fallback content of the
/// replaced elements `iframe`, `video`, `audio` and `canvas`. Their text never
/// appears in the text of an enclosing element.
fn is_hidden(name: &str) -> bool {
    matches!(
        name,
        "audio"
            | "canvas"
            | "datalist"
            | "head"
            | "iframe"
            | "noembed"
            | "noframes"
            | "noscript"
            | "rp"
            | "script"
            | "style"
            | "template"
            | "title"
            | "video"
    )
}

/// Whether an element and its content are left out of the text of an
/// enclosing element: a non-rendered element, or a `form` the parser put
/// straight into a table or row (always empty, and not a line of the table).
fn is_skipped(element: ElementRef<'_>) -> bool {
    let name = element.value().name();
    if name == "form" {
        return element.parent().is_some_and(|parent| {
            matches!(
                parent.value(),
                Node::Element(parent)
                    if matches!(parent.name(), "table" | "tbody" | "tfoot" | "thead" | "tr")
            )
        });
    }
    is_hidden(name)
}

/// Block-level elements: their start and end separate lines of text.
fn is_block(name: &str) -> bool {
    matches!(
        name,
        "address"
            | "article"
            | "aside"
            | "blockquote"
            | "body"
            | "caption"
            | "center"
            | "dd"
            | "details"
            | "dialog"
            | "dir"
            | "div"
            | "dl"
            | "dt"
            | "fieldset"
            | "figcaption"
            | "figure"
            | "footer"
            | "form"
            | "frameset"
            | "h1"
            | "h2"
            | "h3"
            | "h4"
            | "h5"
            | "h6"
            | "header"
            | "hgroup"
            | "hr"
            | "html"
            | "legend"
            | "li"
            | "listing"
            | "main"
            | "menu"
            | "nav"
            | "ol"
            | "optgroup"
            | "option"
            | "p"
            | "plaintext"
            | "pre"
            | "search"
            | "section"
            | "summary"
            | "table"
            | "tbody"
            | "tfoot"
            | "thead"
            | "tr"
            | "ul"
            | "xmp"
    )
}

fn is_cell(name: &str) -> bool {
    matches!(name, "td" | "th")
}

/// Elements whose whitespace is kept as written (`white-space: pre`).
fn preserves_whitespace(name: &str) -> bool {
    matches!(name, "listing" | "plaintext" | "pre" | "textarea" | "xmp")
}

/// HTML "ASCII whitespace"; other spaces (such as U+00A0) are text.
fn is_collapsible_space(ch: char) -> bool {
    matches!(ch, ' ' | '\t' | '\n' | '\x0C' | '\r')
}

/// Whether a cell comes after another cell of its row. Besides cells, the
/// parser only puts non-rendered children into a row (`script`, `style`,
/// `template`, `<input type=hidden>`, an empty `form`), plus comments and
/// whitespace, so everything that is not a cell is passed over. The scan
/// stops at the previous cell: a row costs time linear in its children.
fn follows_cell(cell: ElementRef<'_>) -> bool {
    cell.prev_siblings()
        .any(|node| matches!(node.value(), Node::Element(element) if is_cell(element.name())))
}

/// Text accumulated in document order. A collapsed space or a line break is
/// held back until visible text follows, so no line starts or ends with one.
#[derive(Default)]
struct RenderedText {
    out: String,
    space: bool,
    line_break: bool,
    /// Open `pre`-like elements.
    preserve: usize,
    /// Open table cells.
    cells: usize,
    /// Byte range from the first cell-separating tab to the end of the last.
    cell_tabs: Option<(usize, usize)>,
}

impl RenderedText {
    fn push_text(&mut self, text: &str) {
        for ch in text.chars() {
            if self.preserve == 0 && is_collapsible_space(ch) {
                self.space = true;
            } else {
                self.flush();
                self.out.push(ch);
            }
        }
    }

    fn flush(&mut self) {
        if self.line_break {
            self.out.push('\n');
        } else if self.space && !matches!(self.out.chars().last(), None | Some('\n' | '\t')) {
            self.out.push(' ');
        }
        self.line_break = false;
        self.space = false;
    }

    /// Start or end of a block. Inside a table cell it is only a word break,
    /// so that every table row stays on one line.
    fn block_boundary(&mut self) {
        if self.cells > 0 {
            self.space = true;
        } else {
            self.space = false;
            if !self.out.is_empty() && !self.out.ends_with('\n') {
                self.line_break = true;
            }
        }
    }

    /// `<br>`: one more line, even right after a block.
    fn hard_break(&mut self) {
        self.separator('\n');
    }

    /// A cell after another cell of the same row: a tab, even when the cells
    /// before it are empty. Cells of a table nested in a cell are only
    /// separated by a word break.
    fn cell_separator(&mut self) {
        if self.separator('\t') {
            let end = self.out.len();
            let first = self.cell_tabs.map_or(end - 1, |(first, _)| first);
            self.cell_tabs = Some((first, end));
        }
    }

    /// Emits a separator that is never collapsed, after any pending line
    /// break. Inside a table cell it is a word break instead; returns whether
    /// the separator itself was emitted.
    fn separator(&mut self, separator: char) -> bool {
        if self.cells > 0 {
            self.space = true;
            return false;
        }
        self.space = false;
        if self.line_break {
            self.out.push('\n');
            self.line_break = false;
        }
        self.out.push(separator);
        true
    }

    /// The trimmed text. Cell-separating tabs at either end are kept, so
    /// that leading and trailing empty cells still count as columns.
    fn finish(self) -> String {
        let out = self.out;
        let mut start = out.len() - out.trim_start().len();
        let mut end = out.trim_end().len();
        if let Some((first, last)) = self.cell_tabs {
            start = start.min(first);
            end = end.max(last);
        }
        out[start..end.max(start)].to_string()
    }

    /// Returns whether the element's children are part of the text.
    fn open(&mut self, element: ElementRef<'_>) -> bool {
        if is_skipped(element) {
            return false;
        }
        let name = element.value().name();
        if name == "br" {
            self.hard_break();
        } else if is_cell(name) {
            if follows_cell(element) {
                self.cell_separator();
            }
            self.cells += 1;
        } else if is_block(name) {
            self.block_boundary();
        }
        if preserves_whitespace(name) {
            self.preserve += 1;
        }
        true
    }

    fn close(&mut self, element: ElementRef<'_>) {
        if is_skipped(element) {
            return;
        }
        let name = element.value().name();
        if is_cell(name) {
            self.cells -= 1;
        } else if is_block(name) {
            self.block_boundary();
        }
        if preserves_whitespace(name) {
            self.preserve -= 1;
        }
    }
}

/// The text of an element as a browser renders it (an `innerText`-like
/// reading, without CSS):
/// - inline elements (`b`, `span`, `a`, `sub`, unknown tags, ...) add
///   nothing: `H<sub>2</sub>O` is `H2O`;
/// - runs of HTML whitespace collapse to one space, except inside
///   `pre`, `textarea`, `listing`, `plaintext` and `xmp`;
/// - the start and end of a block element (`p`, `div`, `li`, `tr`, `h1`,
///   ...) and every `<br>` end a line; consecutive block boundaries make one
///   line break, so empty lines come only from `<br>` and from line breaks
///   written inside `pre`-like elements;
/// - `td`/`th` cells of one row are separated by a tab, also after an empty
///   cell; inside a cell, block boundaries, `<br>` and nested cells are a
///   single space (line breaks written inside `pre`-like elements are kept);
/// - elements that are not rendered (`script`, `style`, `noscript`,
///   `template`, `head`, `title`, the fallback content of `iframe`, `video`,
///   `audio`, `canvas`, ...) are skipped, and so is a `form` placed directly
///   in a table;
/// - if the matched element is itself not rendered, it is read as if it
///   were: with only text inside (`script`, `style`, `title`, `noscript`,
///   ...) its raw text is returned, trimmed; with child elements (`template`,
///   `datalist`, ...) its children follow the rules above;
/// - the result is trimmed, except for cell-separating tabs.
fn element_text(e: ElementRef) -> String {
    let name = e.value().name();
    // A `template` keeps its content in a document fragment child, hence a
    // look at all descendants rather than the children.
    if is_hidden(name)
        && !e
            .descendants()
            .skip(1)
            .any(|node| node.value().is_element())
    {
        return e.text().collect::<String>().trim().to_string();
    }
    let mut text = RenderedText {
        preserve: usize::from(preserves_whitespace(name)),
        cells: usize::from(is_cell(name)),
        ..RenderedText::default()
    };
    // Iterative walk: deep documents must not exhaust the stack.
    let mut next = e.first_child();
    while let Some(node) = next {
        let descend = match node.value() {
            Node::Text(t) => {
                text.push_text(t);
                false
            }
            Node::Element(_) => ElementRef::wrap(node).is_some_and(|element| text.open(element)),
            // The content of a selected `template` (nested ones are skipped).
            Node::Fragment => true,
            _ => false,
        };
        next = node.first_child().filter(|_| descend);
        let mut current = node;
        while next.is_none() {
            if let Some(element) = ElementRef::wrap(current) {
                text.close(element);
            }
            next = current.next_sibling();
            if next.is_some() {
                break;
            }
            match current.parent() {
                Some(parent) if parent.id() != e.id() => current = parent,
                _ => break,
            }
        }
    }
    text.finish()
}

const HTML_NAMESPACE: &str = "http://www.w3.org/1999/xhtml";

/// One open element of [`HtmlWriter`].
struct OpenElement {
    /// Local name of an element in the HTML namespace.
    html_name: Option<String>,
    /// A void element: nothing of its content is written.
    ignore_children: bool,
}

/// html5ever's HTML serializer as scraper calls it (scripting disabled).
/// Attributes are written in source order: scraper's `deterministic`
/// feature keeps them in an insertion-ordered map (without it, a hash map
/// whose order differs from one parse to the next).
struct HtmlWriter {
    out: String,
    stack: Vec<OpenElement>,
}

impl HtmlWriter {
    fn escape(&mut self, text: &str, attribute: bool) {
        for ch in text.chars() {
            match ch {
                '&' => self.out.push_str("&amp;"),
                '\u{a0}' => self.out.push_str("&nbsp;"),
                '"' if attribute => self.out.push_str("&quot;"),
                '<' if !attribute => self.out.push_str("&lt;"),
                '>' if !attribute => self.out.push_str("&gt;"),
                ch => self.out.push(ch),
            }
        }
    }

    fn open(&mut self, node: &Node) {
        match node {
            Node::Doctype(doctype) => {
                self.out.push_str("<!DOCTYPE ");
                self.out.push_str(doctype.name());
                self.out.push('>');
            }
            Node::Comment(comment) => {
                self.out.push_str("<!--");
                self.out.push_str(comment);
                self.out.push_str("-->");
            }
            Node::Text(text) => {
                let raw = self
                    .stack
                    .last()
                    .and_then(|e| e.html_name.as_deref())
                    .is_some_and(|name| {
                        matches!(
                            name,
                            "style"
                                | "script"
                                | "xmp"
                                | "iframe"
                                | "noembed"
                                | "noframes"
                                | "plaintext"
                        )
                    });
                if raw {
                    self.out.push_str(text);
                } else {
                    self.escape(text, false);
                }
            }
            Node::Element(element) => self.start_element(element),
            _ => {}
        }
    }

    fn start_element(&mut self, element: &scraper::node::Element) {
        let local: &str = &element.name.local;
        let html = &*element.name.ns == HTML_NAMESPACE;
        let html_name = html.then(|| local.to_string());
        if self.stack.last().is_some_and(|e| e.ignore_children) {
            self.stack.push(OpenElement {
                html_name,
                ignore_children: true,
            });
            return;
        }
        let attributes: Vec<(String, &str)> = element
            .attrs
            .iter()
            .map(|(name, value)| {
                let prefix = match &*name.ns {
                    "" => "",
                    "http://www.w3.org/XML/1998/namespace" => "xml:",
                    "http://www.w3.org/2000/xmlns/" if &*name.local == "xmlns" => "",
                    "http://www.w3.org/2000/xmlns/" => "xmlns:",
                    "http://www.w3.org/1999/xlink" => "xlink:",
                    _ => "unknown_namespace:",
                };
                (format!("{}{}", prefix, &*name.local), &**value)
            })
            .collect();
        self.out.push('<');
        self.out.push_str(local);
        for (name, value) in attributes {
            self.out.push(' ');
            self.out.push_str(&name);
            self.out.push_str("=\"");
            self.escape(value, true);
            self.out.push('"');
        }
        self.out.push('>');
        let ignore_children = html
            && matches!(
                local,
                "area"
                    | "base"
                    | "basefont"
                    | "bgsound"
                    | "br"
                    | "col"
                    | "embed"
                    | "frame"
                    | "hr"
                    | "img"
                    | "input"
                    | "keygen"
                    | "link"
                    | "meta"
                    | "param"
                    | "source"
                    | "track"
                    | "wbr"
            );
        self.stack.push(OpenElement {
            html_name,
            ignore_children,
        });
    }

    fn close(&mut self, node: &Node) {
        if let Node::Element(element) = node
            && self.stack.pop().is_some_and(|e| !e.ignore_children)
        {
            self.out.push_str("</");
            self.out.push_str(&element.name.local);
            self.out.push('>');
        }
    }
}

/// `e.html()` (with `include_self`) or `e.inner_html()`, deterministically:
/// see [`HtmlWriter`]. The walk is iterative, like [`element_text`].
fn serialize_html(e: ElementRef<'_>, include_self: bool) -> String {
    let mut writer = HtmlWriter {
        out: String::new(),
        stack: vec![OpenElement {
            html_name: None,
            ignore_children: false,
        }],
    };
    let mut next = if include_self {
        Some(*e)
    } else {
        e.first_child()
    };
    while let Some(node) = next {
        writer.open(node.value());
        next = node.first_child();
        let mut current = node;
        while next.is_none() {
            writer.close(current.value());
            if current.id() == e.id() {
                break;
            }
            next = current.next_sibling();
            if next.is_some() {
                break;
            }
            match current.parent() {
                Some(parent) if parent.id() != e.id() || include_self => current = parent,
                _ => break,
            }
        }
    }
    writer.out
}

/// The value a match yields, or `None` when it has none (a missing
/// attribute). Empty values are kept, as in the other adapters; templates
/// remove them with `drop_empty`.
fn extract_value(e: ElementRef, mode: &ExtractMode) -> Option<String> {
    match mode {
        ExtractMode::Text => Some(element_text(e)),
        ExtractMode::Attr(name) => e.value().attr(name).map(|v| v.trim().to_string()),
        ExtractMode::InnerHtml => Some(serialize_html(e, false)),
        ExtractMode::OuterHtml => Some(serialize_html(e, true)),
    }
}

impl Adapter for HtmlAdapter {
    fn format(&self) -> Format {
        Format::Html
    }

    fn parse<'a>(&self, text: &'a str) -> Result<SourceTree<'a>, String> {
        Ok(SourceTree::Html(Html::parse_document(text)))
    }

    fn selector_types(&self) -> &'static [&'static str] {
        &["css"]
    }

    fn extract(&self, tree: &SourceTree<'_>, rule: &CompiledRule) -> Result<Vec<RawMatch>, String> {
        let doc = match tree {
            SourceTree::Html(h) => h,
            _ => return Err("html adapter received non-html tree".to_string()),
        };
        let expr = match &rule.selector {
            SelectorCompiled::Css(s) => s,
            _ => return Err("html adapter requires a css selector".to_string()),
        };
        let sel =
            Selector::parse(expr).map_err(|e| format!("invalid css selector '{}': {}", expr, e))?;
        Ok(doc
            .select(&sel)
            .filter_map(|e| extract_value(e, &rule.extract))
            .map(|value| RawMatch { value, lines: None })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule(expr: &str) -> CompiledRule {
        let t = crate::template::compile_yaml_str(
            &format!(
                r#"
version: 1
name: t
input: {{format: html}}
rules:
  - id: r
    select: {{type: css, expression: "{}"}}
    output: {{key: k}}
"#,
                expr
            ),
            &std::path::PathBuf::from("t.yml"),
            None,
            None,
        )
        .unwrap();
        t.rules[0].clone()
    }

    #[test]
    fn html_extract() {
        let a = HtmlAdapter;
        let tree = a
            .parse(r#"<html><body><h1>Title</h1><p class="x">Hello</p><p class="y">World</p></body></html>"#)
            .unwrap();
        let matches = a.extract(&tree, &rule("p.x")).unwrap();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].value, "Hello");
        let matches = a.extract(&tree, &rule("p")).unwrap();
        assert_eq!(matches.len(), 2);
    }

    #[test]
    fn attr_extract() {
        let a = HtmlAdapter;
        let mut r = rule("h1");
        r.extract = ExtractMode::Attr("class".into());
        let tree = a
            .parse(r#"<html><body><h1 class="ch">Title</h1></body></html>"#)
            .unwrap();
        let matches = a.extract(&tree, &r).unwrap();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].value, "ch");
    }

    fn values(html: &str, expr: &str, extract: ExtractMode) -> Vec<String> {
        let adapter = HtmlAdapter;
        let tree = adapter.parse(html).unwrap();
        let mut r = rule(expr);
        r.extract = extract;
        let matches = adapter.extract(&tree, &r).unwrap();
        matches.into_iter().map(|m| m.value).collect()
    }

    fn texts(html: &str, expr: &str) -> Vec<String> {
        values(html, expr, ExtractMode::Text)
    }

    #[test]
    fn inline_elements_add_no_separator_and_single_text_nodes_are_unchanged() {
        assert_eq!(texts("<p>H<sub>2</sub>O</p>", "p"), ["H2O"]);
        assert_eq!(texts("<p>Hello <b>wor</b>ld</p>", "p"), ["Hello world"]);
        assert_eq!(
            texts(
                "<p><a href='#'>x</a><span>y</span><code>z</code> <em>e</em>m<x-tag>!</x-tag></p>",
                "p"
            ),
            ["xyz em!"]
        );
        assert_eq!(texts("<p>  Hello world  </p>", "p"), ["Hello world"]);
        assert_eq!(texts("<h1>Title</h1>", "h1"), ["Title"]);
        assert_eq!(
            texts("<p>caf&eacute; &amp; &lt;tag&gt;</p>", "p"),
            ["café & <tag>"]
        );
        // U+00A0 is text, not collapsible whitespace (the outer trim still applies).
        assert_eq!(texts("<p>a&nbsp;&nbsp;b</p>", "p"), ["a\u{a0}\u{a0}b"]);
    }

    #[test]
    fn whitespace_collapses_and_blocks_end_lines() {
        let html = "<div>\n  <p>First\n     paragraph</p>\n  <p>Second</p>\n</div>";
        assert_eq!(texts(html, "div"), ["First paragraph\nSecond"]);
        let list = "<ul>\n <li>One <i>1</i></li>\n <li>Two</li>\n</ul>";
        assert_eq!(texts(list, "ul"), ["One 1\nTwo"]);
        assert_eq!(texts(list, "li"), ["One 1", "Two"]);
        // Nested and adjacent blocks give a single line break, never a blank line.
        assert_eq!(
            texts(
                "<section><div><div><h2>Head</h2></div></div><div>Body</div></section>",
                "section"
            ),
            ["Head\nBody"]
        );
        assert_eq!(texts("<div>a<hr>b</div>", "div"), ["a\nb"]);
        assert_eq!(
            texts("<div>inline <div>block</div> tail</div>", "body > div"),
            ["inline\nblock\ntail"]
        );
    }

    #[test]
    fn br_breaks_lines_and_repeated_br_leave_an_empty_line() {
        assert_eq!(texts("<p>a<br>b</p>", "p"), ["a\nb"]);
        assert_eq!(texts("<p>a <br> b</p>", "p"), ["a\nb"]);
        assert_eq!(texts("<p>a<br><br>b</p>", "p"), ["a\n\nb"]);
        assert_eq!(texts("<div><p>a<br></p><p>b</p></div>", "div"), ["a\nb"]);
        assert_eq!(texts("<div><p>a</p><br><p>b</p></div>", "div"), ["a\n\nb"]);
        assert_eq!(texts("<p><br>a<br></p>", "p"), ["a"]);
    }

    #[test]
    fn table_cells_are_tab_separated_and_rows_are_lines() {
        let table = "<table>\n<tr><th>Name</th> <th>Age</th></tr>\n\
                     <tr><td> Bob </td><td><p>4</p><p>2</p></td></tr>\n\
                     <tr><td></td><td>x<br>y</td></tr>\n</table>";
        assert_eq!(texts(table, "table"), ["Name\tAge\nBob\t4 2\n\tx y"]);
        assert_eq!(texts(table, "tr"), ["Name\tAge", "Bob\t4 2", "\tx y"]);
        assert_eq!(texts(table, "td"), ["Bob", "4 2", "", "x y"]);
        // Leading and trailing empty cells keep their tabs; nothing else does.
        let sparse = "<table><tr><td>&nbsp;</td><td><br></td><td>m</td><td> </td><td></td></tr>\
                      <tr><td></td><td></td></tr></table>";
        assert_eq!(texts(sparse, "tr"), ["\t\tm\t\t", "\t"]);
        assert_eq!(texts(sparse, "table"), ["\t\tm\t\t\n\t"]);
        let nested = "<table><tr><td>a<table><tr><td>b</td><td>c</td></tr></table></td>\
                      <td>d</td></tr></table>";
        assert_eq!(texts(nested, "body > table > tbody > tr"), ["a b c\td"]);
    }

    #[test]
    fn preformatted_text_keeps_its_whitespace() {
        let html = "<div>x<pre>  a\n    b  </pre>y <textarea>p  q\nr</textarea></div>";
        assert_eq!(texts(html, "div"), ["x\n  a\n    b  \ny p  q\nr"]);
        assert_eq!(texts(html, "pre"), ["a\n    b"]);
        assert_eq!(texts("<pre><b>k</b>  <i>v</i></pre>", "pre"), ["k  v"]);
        // Line breaks written inside `pre` are kept, also inside a table cell.
        let cell = "<table><tr><td><pre>a\n b</pre></td><td>c</td></tr></table>";
        assert_eq!(texts(cell, "tr"), ["a\n b\tc"]);
    }

    #[test]
    fn non_rendered_children_of_a_row_neither_join_nor_split_its_cells() {
        let input = "<table><tr><td>Name</td><input type=\"hidden\" name=\"id\" value=\"7\">\
                     <td>Bob</td></tr></table>";
        assert_eq!(texts(input, "tr"), ["Name\tBob"]);
        assert_eq!(texts(input, "table"), ["Name\tBob"]);
        let spaced =
            "<table>\n<tr>\n<td>Name</td>\n<input type=hidden>\n<td>Bob</td>\n</tr>\n</table>";
        assert_eq!(texts(spaced, "tr"), ["Name\tBob"]);
        // The parser puts the form into the row, empty, between the cells.
        let form = "<table><tr><td>a</td><form><td>b</td></form><script>x()</script>\
                    <td>c</td></tr><tr><td>d</td></tr></table>";
        assert_eq!(texts(form, "tr"), ["a\tb\tc", "d"]);
        assert_eq!(texts(form, "table"), ["a\tb\tc\nd"]);
        assert_eq!(texts(form, "form"), [""]);
        // Outside a table a form is still a block.
        assert_eq!(texts("<div>a<form>b</form>c</div>", "div"), ["a\nb\nc"]);
    }

    #[test]
    fn a_selected_non_rendered_element_is_read_as_if_rendered() {
        let template = "<template><p>One</p><p>Two</p></template>";
        assert_eq!(texts(template, "template"), ["One\nTwo"]);
        assert_eq!(
            texts("<template>Hello <b>wor</b>ld</template>", "template"),
            ["Hello world"]
        );
        let datalist = "<datalist><option>A</option><option>B</option></datalist>";
        assert_eq!(texts(datalist, "datalist"), ["A\nB"]);
        // Nothing inside head is rendered; its parts are selected on their own.
        let head = "<head><title>T</title><meta charset=utf-8><style>s{}</style></head>";
        assert_eq!(texts(head, "head"), [""]);
        assert_eq!(texts(head, "title"), ["T"]);
        // Only text inside: the raw text, trimmed.
        let script = "<script>\n  if (a) {\n    b();\n  }\n</script>";
        assert_eq!(texts(script, "script"), ["if (a) {\n    b();\n  }"]);
    }

    #[test]
    fn fallback_content_of_media_and_canvas_is_not_text() {
        let video = "<p>Watch:<video src='a.mp4'>Your browser does not support video.</video></p>";
        assert_eq!(texts(video, "p"), ["Watch:"]);
        assert_eq!(
            texts(video, "video"),
            ["Your browser does not support video."]
        );
        let media = "<div>a <audio><source src='a.ogg'>aud</audio><canvas>cv</canvas> b</div>";
        assert_eq!(texts(media, "div"), ["a b"]);
        assert_eq!(texts(media, "audio"), ["aud"]);
    }

    #[test]
    fn hidden_elements_are_skipped_unless_selected() {
        let html = "<html><head><title>T</title><style>p{}</style>\
                    <script>var head;</script></head>\
                    <body><p>One</p><script type='application/ld+json'>{\n  \"a\": 1\n}</script>\
                    <style>.x{}</style><noscript>Enable JS</noscript>\
                    <template><p>tpl</p></template><p>Two<script>x()</script>!</p>\
                    <iframe>fallback</iframe><ruby>漢<rp>(</rp><rt>kan</rt><rp>)</rp></ruby></body></html>";
        assert_eq!(texts(html, "html"), ["One\nTwo!\n漢kan"]);
        assert_eq!(texts(html, "body"), ["One\nTwo!\n漢kan"]);
        assert_eq!(
            texts(html, "script[type='application/ld+json']"),
            ["{\n  \"a\": 1\n}"]
        );
        assert_eq!(texts(html, "title"), ["T"]);
        assert_eq!(texts(html, "style"), ["p{}", ".x{}"]);
        assert_eq!(texts(html, "noscript"), ["Enable JS"]);
        assert_eq!(texts(html, "template"), ["tpl"]);
    }

    #[test]
    fn empty_values_are_kept_and_missing_attributes_are_not_matches() {
        let html = "<ul><li>a</li><li></li><li> \n </li><li><span></span></li></ul>\
                    <img alt='' src='x.png'><img src='y.png'><img alt=' z '>";
        assert_eq!(texts(html, "li"), ["a", "", "", ""]);
        assert_eq!(
            values(html, "li", ExtractMode::InnerHtml),
            ["a", "", " \n ", "<span></span>"]
        );
        assert_eq!(
            values(html, "img", ExtractMode::Attr("alt".into())),
            ["", "z"]
        );
        assert_eq!(
            values(html, "img", ExtractMode::Attr("src".into())),
            ["x.png", "y.png"]
        );
        assert_eq!(values(html, "li", ExtractMode::OuterHtml)[1], "<li></li>");
    }

    #[test]
    fn serialization_matches_scraper_and_keeps_the_source_attribute_order() {
        // scraper's own output (`deterministic` feature: attributes in
        // source order) must be reproduced exactly.
        let documents = [
            "<!DOCTYPE html><html><head><title>T &amp; t</title><style>p > a { x: '<' }</style>\
             <script>if (a < b && c) {}</script></head><body><p class=\"x\">a&nbsp;b &lt;c&gt; \"q\" &amp;</p>\
             <br><img src=\"a.png\"><hr><input value='\"v\" &amp; &lt;'><!-- note --><template><p>t</p></template>\
             <noscript><b>n</b> &lt;</noscript><textarea>x &lt; y</textarea><pre>\n  pre</pre>\
             <svg xmlns:xlink=\"http://www.w3.org/1999/xlink\"><a xlink:href=\"#h\"><text>s</text></a></svg>\
             <math><mi xml:lang=\"ru\">x</mi></math><iframe>raw &lt;</iframe><xmp>a < b</xmp>\
             <table><tr><td>1</td></tr></table><ul><li>one<li>two</ul></body></html>",
            "<div><p>unclosed<span>inline</div></p><a href='x'><a>nested</a></a>",
            "plain text only",
            "<p></p><p>",
            "<p b=\"2\" a=\"1\" c=\"3\" a=\"dup\"><i z=\"\" y>t</i></p>",
        ];
        for document in documents {
            let html = Html::parse_document(document);
            let all = Selector::parse("*").unwrap();
            for element in html.select(&all) {
                assert_eq!(serialize_html(element, true), element.html(), "{document}");
                assert_eq!(
                    serialize_html(element, false),
                    element.inner_html(),
                    "{document}"
                );
            }
        }
        let html = Html::parse_document(
            "<div id=\"d\" class=\"c\" data-b=\"2\" data-a=\"1\" title='t\"' lang=\"ru\" hidden>\
             <svg viewBox=\"0 0 1 1\" xmlns:xlink=\"http://www.w3.org/1999/xlink\" xml:lang=\"en\">\
             <use xlink:href=\"#i\" y=\"2\" x=\"1\"/></svg></div>",
        );
        let div = html
            .select(&Selector::parse("div").unwrap())
            .next()
            .unwrap();
        let expected = "<div id=\"d\" class=\"c\" data-b=\"2\" data-a=\"1\" title=\"t&quot;\" lang=\"ru\" hidden=\"\">\
            <svg viewBox=\"0 0 1 1\" xmlns:xlink=\"http://www.w3.org/1999/xlink\" xml:lang=\"en\">\
            <use xlink:href=\"#i\" y=\"2\" x=\"1\"></use></svg></div>";
        assert_eq!(serialize_html(div, true), expected);
        assert_eq!(
            values(&div.html(), "div", ExtractMode::OuterHtml),
            [expected]
        );
    }

    #[test]
    fn deeply_nested_markup_keeps_its_text() {
        let depth = 2_000;
        let html = format!(
            "{}deep{}",
            "<div><span>".repeat(depth),
            "</span></div>".repeat(depth)
        );
        let text = texts(&html, "body");
        assert_eq!(text, ["deep"]);
    }
}
