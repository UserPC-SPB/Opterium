use super::{Adapter, RawMatch, SourceTree};
use crate::template::{CompiledRule, Format, MarkdownNodeType, SelectorCompiled};
use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};

#[derive(Debug)]
struct MdBlock {
    kind: MarkdownNodeType,
    text: String,
}

struct Frame {
    index: usize,
    kind: MarkdownNodeType,
    buffer: String,
    cells: Vec<String>,
    url: Option<String>,
    block_ended: bool,
}

impl Frame {
    fn append_inline(&mut self, text: &str) {
        if self.block_ended && !text.is_empty() {
            self.newline();
            self.block_ended = false;
        }
        self.buffer.push_str(text);
    }

    fn newline(&mut self) {
        if !self.buffer.is_empty() && !self.buffer.ends_with('\n') {
            self.buffer.push('\n');
        }
    }

    fn append_block(&mut self, text: &str) {
        self.newline();
        self.buffer.push_str(text);
        self.block_ended = true;
    }

    fn text(&self) -> String {
        match self.kind {
            MarkdownNodeType::TableRow => self.cells.join(" | "),
            MarkdownNodeType::Table => self.buffer.clone(),
            // pulldown-cmark includes the line ending before the closing fence.
            // Strip that one ending, preserving significant trailing spaces.
            MarkdownNodeType::CodeBlock => self
                .buffer
                .strip_suffix('\n')
                .unwrap_or(&self.buffer)
                .to_string(),
            MarkdownNodeType::Link | MarkdownNodeType::Image => {
                let label = self.buffer.trim();
                let url = self.url.as_deref().unwrap_or_default();
                if label.is_empty() {
                    url.to_string()
                } else if url.is_empty() || label == url {
                    label.to_string()
                } else {
                    format!("{} ({})", label, url)
                }
            }
            _ => self.buffer.trim().to_string(),
        }
    }
}

pub struct MarkdownAdapter;

fn start_node(tag: &Tag<'_>) -> Option<(MarkdownNodeType, Option<String>)> {
    use MarkdownNodeType::*;
    let kind = match tag {
        Tag::Heading { .. } => Heading,
        Tag::Paragraph => Paragraph,
        Tag::List(_) => List,
        Tag::Item => ListItem,
        Tag::Table(_) => Table,
        Tag::TableHead | Tag::TableRow => TableRow,
        Tag::TableCell => TableCell,
        Tag::CodeBlock(_) => CodeBlock,
        Tag::BlockQuote(_) => Blockquote,
        Tag::Link { dest_url, .. } => return Some((Link, Some(dest_url.to_string()))),
        Tag::Image { dest_url, .. } => return Some((Image, Some(dest_url.to_string()))),
        _ => return None,
    };
    Some((kind, None))
}

fn end_node(tag: TagEnd) -> Option<MarkdownNodeType> {
    use MarkdownNodeType::*;
    Some(match tag {
        TagEnd::Heading(_) => Heading,
        TagEnd::Paragraph => Paragraph,
        TagEnd::List(_) => List,
        TagEnd::Item => ListItem,
        TagEnd::Table => Table,
        TagEnd::TableHead | TagEnd::TableRow => TableRow,
        TagEnd::TableCell => TableCell,
        TagEnd::CodeBlock => CodeBlock,
        TagEnd::BlockQuote(_) => Blockquote,
        TagEnd::Link => Link,
        TagEnd::Image => Image,
        _ => return None,
    })
}

fn parse_blocks(text: &str) -> Vec<MdBlock> {
    let opts = Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TASKLISTS;
    let mut stack: Vec<Frame> = Vec::new();
    let mut blocks: Vec<MdBlock> = Vec::new();

    for event in Parser::new_ext(text, opts) {
        match event {
            Event::Start(tag) => {
                if let Some((kind, url)) = start_node(&tag) {
                    let index = blocks.len();
                    // Reserve at START rather than END: nested matches retain
                    // source order, with a parent preceding its descendants.
                    blocks.push(MdBlock {
                        kind: kind.clone(),
                        text: String::new(),
                    });
                    stack.push(Frame {
                        index,
                        kind,
                        buffer: String::new(),
                        cells: Vec::new(),
                        url,
                        block_ended: false,
                    });
                }
            }
            Event::End(tag) => {
                let Some(kind) = end_node(tag) else {
                    continue;
                };
                let frame = stack.pop().expect("balanced Markdown events");
                debug_assert_eq!(frame.kind, kind);
                let value = frame.text();
                blocks[frame.index].text = value.clone();
                if let Some(parent) = stack.last_mut() {
                    match kind {
                        MarkdownNodeType::TableCell => parent.cells.push(value),
                        MarkdownNodeType::Link | MarkdownNodeType::Image => {
                            parent.append_inline(&value);
                        }
                        _ => parent.append_block(&value),
                    }
                }
            }
            Event::Text(value)
            | Event::Code(value)
            | Event::Html(value)
            | Event::InlineHtml(value) => {
                if let Some(parent) = stack.last_mut() {
                    parent.append_inline(&value);
                }
            }
            Event::SoftBreak | Event::HardBreak => {
                if let Some(parent) = stack.last_mut() {
                    parent.append_inline("\n");
                }
            }
            Event::TaskListMarker(checked) => {
                if let Some(parent) = stack.last_mut() {
                    parent.append_inline(if checked { "[x] " } else { "[ ] " });
                }
            }
            Event::Rule => {
                if let Some(parent) = stack.last_mut() {
                    parent.append_block("---");
                }
            }
            _ => {}
        }
    }
    blocks
}

impl Adapter for MarkdownAdapter {
    fn format(&self) -> Format {
        Format::Markdown
    }

    fn parse<'a>(&self, text: &'a str) -> Result<SourceTree<'a>, String> {
        Ok(SourceTree::Text(text))
    }

    fn selector_types(&self) -> &'static [&'static str] {
        &["markdown"]
    }

    fn extract(&self, tree: &SourceTree<'_>, rule: &CompiledRule) -> Result<Vec<RawMatch>, String> {
        let text = match tree {
            SourceTree::Text(text) => text,
            _ => return Err("markdown adapter received non-text tree".to_string()),
        };
        let (node_type, pattern) = match &rule.selector {
            SelectorCompiled::Markdown { node, pattern } => (node, pattern),
            _ => return Err("markdown adapter requires a markdown selector".to_string()),
        };
        Ok(parse_blocks(text)
            .into_iter()
            .filter(|block| &block.kind == node_type)
            .filter(|block| pattern.as_ref().is_none_or(|re| re.is_match(&block.text)))
            .map(|block| RawMatch {
                value: block.text,
                lines: None,
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule(node: &str) -> CompiledRule {
        let t = crate::template::compile_yaml_str(
            &format!(
                r#"
version: 1
name: t
input: {{format: markdown}}
rules:
  - id: r
    select: {{type: markdown, node: "{}"}}
    output: {{key: k}}
"#,
                node
            ),
            &std::path::PathBuf::from("t.yml"),
            None,
            None,
        )
        .unwrap();
        t.rules[0].clone()
    }

    #[test]
    fn headings_and_code() {
        let a = MarkdownAdapter;
        let md = "# Title\nSome text\n```\ncode block\n```\n## Sub\nsecond para\n";
        let tree = a.parse(md).unwrap();
        let h = a.extract(&tree, &rule("heading")).unwrap();
        assert_eq!(h.len(), 2);
        assert_eq!(h[0].value, "Title");
        assert_eq!(h[1].value, "Sub");
        let c = a.extract(&tree, &rule("code_block")).unwrap();
        assert_eq!(c.len(), 1);
        assert_eq!(c[0].value, "code block");
    }

    #[test]
    fn tables() {
        let a = MarkdownAdapter;
        let md = "| a | b |\n|---|---|\n| 1 | 2 |\n| 3 | 4 |\n";
        let tree = a.parse(md).unwrap();
        let t = a.extract(&tree, &rule("table")).unwrap();
        assert_eq!(t.len(), 1);
        assert_eq!(t[0].value, "a | b\n1 | 2\n3 | 4");
    }

    #[test]
    fn lists() {
        let a = MarkdownAdapter;
        let md = "- one\n- two\n\nparagraph\n";
        let tree = a.parse(md).unwrap();
        let l = a.extract(&tree, &rule("list_item")).unwrap();
        assert_eq!(l.len(), 2);
        assert_eq!(l[0].value, "one");
        assert_eq!(l[1].value, "two");
    }

    fn values(markdown: &str, node: &str) -> Vec<String> {
        let adapter = MarkdownAdapter;
        let tree = adapter.parse(markdown).unwrap();
        adapter
            .extract(&tree, &rule(node))
            .unwrap()
            .into_iter()
            .map(|matched| matched.value)
            .collect()
    }

    #[test]
    fn nested_lists_quotes_and_paragraphs_remain_selectable_in_source_order() {
        let markdown =
            "- outer\n  - inner\n- last\n\n> first quote\n>\n> > inner quote\n>\n> final quote\n";
        assert_eq!(
            values(markdown, "list_item"),
            vec!["outer\ninner", "inner", "last"]
        );
        assert_eq!(
            values(markdown, "list"),
            vec!["outer\ninner\nlast", "inner"]
        );
        assert_eq!(
            values(markdown, "blockquote"),
            vec!["first quote\ninner quote\nfinal quote", "inner quote"]
        );
        assert_eq!(
            values(markdown, "paragraph"),
            vec!["first quote", "inner quote", "final quote"]
        );
    }

    #[test]
    fn nested_links_and_images_keep_their_own_urls_and_inline_spacing() {
        let markdown = "# Read [docs](https://docs) now\n\nbefore [![alt](https://image)](https://target) after ![second](https://second).\n";
        assert_eq!(
            values(markdown, "heading"),
            vec!["Read docs (https://docs) now"]
        );
        assert_eq!(
            values(markdown, "link"),
            vec![
                "docs (https://docs)",
                "alt (https://image) (https://target)"
            ]
        );
        assert_eq!(
            values(markdown, "image"),
            vec!["alt (https://image)", "second (https://second)"]
        );
        assert_eq!(
            values(markdown, "paragraph"),
            vec!["before alt (https://image) (https://target) after second (https://second)."]
        );
        assert_eq!(
            values("<https://example.test>", "link"),
            vec!["https://example.test"]
        );
    }

    #[test]
    fn table_rows_cells_and_inline_nodes_preserve_empty_cells_and_literal_pipes() {
        let markdown = "| first | second |\n|---|---|\n| a\\|b | [link](https://cell) |\n| | ![image](https://img) |\n";
        assert_eq!(
            values(markdown, "table_cell"),
            vec![
                "first",
                "second",
                "a|b",
                "link (https://cell)",
                "",
                "image (https://img)"
            ]
        );
        assert_eq!(
            values(markdown, "table_row"),
            vec![
                "first | second",
                "a|b | link (https://cell)",
                " | image (https://img)"
            ]
        );
        assert_eq!(
            values(markdown, "table"),
            vec!["first | second\na|b | link (https://cell)\n | image (https://img)"]
        );
        assert_eq!(values(markdown, "link"), vec!["link (https://cell)"]);
        assert_eq!(values(markdown, "image"), vec!["image (https://img)"]);
    }

    #[test]
    fn nested_code_blocks_preserve_trailing_spaces_and_pattern_filters_nodes() {
        assert_eq!(
            values("> ~~~\n> code  \n> ~~~\n", "code_block"),
            vec!["code  "]
        );
        let mut selected = rule("link");
        if let SelectorCompiled::Markdown { pattern, .. } = &mut selected.selector {
            *pattern = Some(regex::Regex::new("keep").unwrap());
        }
        let adapter = MarkdownAdapter;
        let tree = adapter
            .parse("- [skip](https://skip)\n- [keep](https://keep)\n")
            .unwrap();
        let output = adapter.extract(&tree, &selected).unwrap();
        assert_eq!(output.len(), 1);
        assert_eq!(output[0].value, "keep (https://keep)");
    }
}
