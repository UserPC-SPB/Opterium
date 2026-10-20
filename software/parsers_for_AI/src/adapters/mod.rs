pub mod html;
pub mod json;
pub mod markdown;
pub mod path_selector;
pub mod text;
pub mod toml;
pub mod xml;
pub mod xml_dtd;
pub mod yaml;

use crate::template::{CompiledRule, Format, SelectorCompiled};

#[derive(Debug)]
pub enum SourceTree<'a> {
    Json(serde_json::Value),
    /// The non-empty documents of a multi-document YAML stream, in order.
    JsonDocuments(Vec<serde_json::Value>),
    Xml(XmlTree<'a>),
    Html(scraper::Html),
    Text(&'a str),
}

#[derive(Debug)]
pub struct XmlTree<'a> {
    pub source: &'a str,
    pub doc: roxmltree::Document<'a>,
}

impl<'a> XmlTree<'a> {
    pub fn parse(text: &'a str) -> Result<Self, String> {
        let depth_limit = crate::limits::MAX_XML_NESTING_DEPTH;
        check_xml_depth(text, depth_limit)?;
        let doc = match roxmltree::Document::parse(text) {
            // The prolog has a DOCTYPE. Its internal entities are expanded by
            // roxmltree itself, so their cost and markup are bounded first.
            Err(roxmltree::Error::DtdDetected) => {
                xml_dtd::check_doctype(text, depth_limit)?;
                let options = roxmltree::ParsingOptions {
                    allow_dtd: true,
                    ..roxmltree::ParsingOptions::default()
                };
                roxmltree::Document::parse_with_options(text, options)
            }
            parsed => parsed,
        }
        .map_err(|e| format!("invalid XML: {}", e))?;
        Ok(XmlTree { source: text, doc })
    }
}

/// Rejects XML nested deeper than `limit` elements without recursion, before
/// the recursive parser sees it. Depth is exact for well-formed XML;
/// malformed input that passes is rejected by the parser itself.
pub fn check_xml_depth(text: &str, limit: usize) -> Result<(), String> {
    let mut depth = 0usize;
    let within_limit = scan_xml_tags(text, |tag| {
        match tag {
            XmlTag::Open(_) => depth += 1,
            XmlTag::Close(_) => depth = depth.saturating_sub(1),
        }
        depth <= limit
    });
    if within_limit {
        Ok(())
    } else {
        Err(format!(
            "XML nesting depth exceeds the limit of {} elements",
            limit
        ))
    }
}

/// Whether the text is one element whose start and end tags all pair up,
/// checked without recursion and without a depth limit. Used to tell deep XML
/// from text that merely starts with `<` (`<34>` syslog, `<nick>` chat logs).
pub fn xml_markup_is_balanced(text: &str) -> bool {
    let mut open: Vec<&[u8]> = Vec::new();
    let mut roots = 0usize;
    let paired = scan_xml_tags(text, |tag| match tag {
        XmlTag::Open(name) => {
            if open.is_empty() {
                roots += 1;
            }
            open.push(name);
            roots == 1
        }
        XmlTag::Close(name) => open.pop() == Some(name),
    });
    paired && open.is_empty() && roots == 1
}

enum XmlTag<'a> {
    Open(&'a [u8]),
    Close(&'a [u8]),
}

/// Calls `visit` for every start tag (self-closing ones excluded) and end tag
/// in document order, skipping comments, CDATA, processing instructions and
/// declarations. Returns false as soon as `visit` does.
fn scan_xml_tags<'a>(text: &'a str, mut visit: impl FnMut(XmlTag<'a>) -> bool) -> bool {
    let bytes = text.as_bytes();
    let mut i = 0;
    while let Some(offset) = bytes[i..].iter().position(|b| *b == b'<') {
        i += offset;
        let rest = &bytes[i..];
        i = if rest.starts_with(b"<!--") {
            skip_past(bytes, i + 4, b"-->")
        } else if rest.starts_with(b"<![CDATA[") {
            skip_past(bytes, i + 9, b"]]>")
        } else if rest.starts_with(b"<?") {
            skip_past(bytes, i + 2, b"?>")
        } else if rest.starts_with(b"<!") {
            skip_declaration(bytes, i + 2)
        } else if rest.starts_with(b"</") {
            if !visit(XmlTag::Close(tag_name(bytes, i + 2))) {
                return false;
            }
            skip_tag(bytes, i + 2).0
        } else {
            let (end, self_closing) = skip_tag(bytes, i + 1);
            if !self_closing && !visit(XmlTag::Open(tag_name(bytes, i + 1))) {
                return false;
            }
            end
        };
    }
    true
}

fn tag_name(bytes: &[u8], from: usize) -> &[u8] {
    let rest = &bytes[from..];
    let len = rest
        .iter()
        .position(|b| b.is_ascii_whitespace() || *b == b'/' || *b == b'>')
        .unwrap_or(rest.len());
    &rest[..len]
}

fn skip_past(bytes: &[u8], from: usize, terminator: &[u8]) -> usize {
    bytes
        .get(from..)
        .and_then(|rest| rest.windows(terminator.len()).position(|w| w == terminator))
        .map_or(bytes.len(), |p| from + p + terminator.len())
}

/// Returns the index after the tag's closing `>` and whether it was `/>`.
fn skip_tag(bytes: &[u8], from: usize) -> (usize, bool) {
    let mut quote = None;
    for (offset, &b) in bytes.iter().enumerate().skip(from) {
        match quote {
            Some(q) if b == q => quote = None,
            Some(_) => {}
            None if b == b'"' || b == b'\'' => quote = Some(b),
            None if b == b'>' => return (offset + 1, offset > from && bytes[offset - 1] == b'/'),
            None => {}
        }
    }
    (bytes.len(), false)
}

/// Skips `<!DOCTYPE …>` and similar declarations, including a bracketed
/// internal subset with its own quoted strings and comments.
fn skip_declaration(bytes: &[u8], from: usize) -> usize {
    let mut quote = None;
    let mut brackets = 0usize;
    let mut i = from;
    while i < bytes.len() {
        let b = bytes[i];
        match quote {
            Some(q) if b == q => quote = None,
            Some(_) => {}
            None if bytes[i..].starts_with(b"<!--") => {
                i = skip_past(bytes, i + 4, b"-->");
                continue;
            }
            None if b == b'"' || b == b'\'' => quote = Some(b),
            None if b == b'[' => brackets += 1,
            None if b == b']' => brackets = brackets.saturating_sub(1),
            None if b == b'>' && brackets == 0 => return i + 1,
            None => {}
        }
        i += 1;
    }
    bytes.len()
}

#[derive(Debug, Clone)]
pub struct RawMatch {
    pub value: String,
    /// Where the value is in the source: text rules only.
    pub lines: Option<SourceLines>,
}

/// Where an item of a text rule comes from (1-based source lines).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceLines {
    /// The matched line; the first line of a `line` range.
    pub line: usize,
    /// First and last source line of the item: with its context lines, or
    /// the whole `line` range.
    pub first: usize,
    pub last: usize,
}

pub trait Adapter: Send + Sync {
    fn format(&self) -> Format;
    fn parse<'a>(&self, text: &'a str) -> Result<SourceTree<'a>, String>;
    fn selector_types(&self) -> &'static [&'static str];
    fn extract(&self, tree: &SourceTree<'_>, rule: &CompiledRule) -> Result<Vec<RawMatch>, String>;
}

pub fn adapter_for(format: &Format) -> Box<dyn Adapter> {
    match format {
        Format::Json => Box::new(json::JsonAdapter),
        Format::Xml => Box::new(xml::XmlAdapter),
        Format::Html => Box::new(html::HtmlAdapter),
        Format::Markdown => Box::new(markdown::MarkdownAdapter),
        Format::Yaml => Box::new(yaml::YamlAdapter),
        Format::Toml => Box::new(toml::TomlAdapter),
        Format::Text => Box::new(text::TextAdapter),
        Format::Auto => unreachable!("auto is resolved before adapter dispatch"),
    }
}

pub fn rule_applicable(adapter: &dyn Adapter, rule: &CompiledRule) -> bool {
    rule.when_format
        .as_ref()
        .is_none_or(|format| *format == adapter.format())
        && adapter
            .selector_types()
            .contains(&rule.selector_type.as_str())
}

pub fn match_value<'a>(
    sel: &SelectorCompiled,
    value: &'a serde_json::Value,
) -> Result<Vec<&'a serde_json::Value>, String> {
    match sel {
        SelectorCompiled::JsonPath(expr) => Ok(crate::jsonpath::evaluate(expr, value)),
        SelectorCompiled::Path(segs) => path_selector::walk(value, segs),
        _ => Err("selector is not valid for json-like trees".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_condition_and_selector_must_both_match() {
        let template = crate::template::compile_yaml_str(
            "version: 1\nname: t\nrules:\n  - id: r\n    when: {format: yaml}\n    select: {type: path, expression: name}\n",
            &std::path::PathBuf::from("t.yml"),
            None,
            None,
        )
        .unwrap();
        let mut rule = template.rules[0].clone();
        assert!(rule_applicable(&yaml::YamlAdapter, &rule));
        assert!(!rule_applicable(&json::JsonAdapter, &rule));
        assert!(!rule_applicable(&toml::TomlAdapter, &rule));
        rule.when_format = None;
        assert!(rule_applicable(&json::JsonAdapter, &rule));
        assert!(!rule_applicable(&text::TextAdapter, &rule));
    }

    #[test]
    fn xml_depth_limit_is_exact_and_ignores_markup_that_is_not_an_element() {
        let nested = |depth: usize| format!("{}{}", "<a>".repeat(depth), "</a>".repeat(depth));
        assert!(check_xml_depth(&nested(4), 4).is_ok());
        assert!(check_xml_depth(&nested(5), 4).is_err());
        let decorated = concat!(
            "<?xml version=\"1.0\"?><!-- <a><a><a> -->",
            "<!DOCTYPE r [ <!ENTITY e \"<a><a>\"> <!-- ]> <a> --> ]>",
            "<r k='<a>' j=\"/>\"><x/><y/><![CDATA[<a><a><a>]]><?pi <a><a>?><z></z></r>"
        );
        assert!(check_xml_depth(decorated, 2).is_ok());
        assert!(check_xml_depth(decorated, 1).is_err());
        assert!(xml_markup_is_balanced(decorated));
        assert!(xml_markup_is_balanced(&nested(100_000)));
        for not_xml in [
            "<34>Oct 11 host su: failed\n<34>Oct 11 host su: failed\n",
            "<alice> hi\n<bob> hello\n",
            "<div>a<br>b</div>",
            "<a></a><b></b>",
            "<a><b></a></b>",
            "<a>",
        ] {
            assert!(!xml_markup_is_balanced(not_xml), "{not_xml}");
        }
        // Deep enough to overflow the recursive parser: a per-file error now.
        let deep = nested(100_000);
        let error = adapter_for(&Format::Xml).parse(&deep).unwrap_err();
        assert!(error.contains("nesting depth"), "{error}");
        assert!(adapter_for(&Format::Xml).parse(&nested(128)).is_ok());
    }

    #[test]
    fn dynamic_xml_adapter_borrows_the_callers_buffer() {
        let adapter = adapter_for(&Format::Xml);
        let mut source = String::from("<root>value</root>");
        {
            let tree = adapter.parse(&source).unwrap();
            let SourceTree::Xml(xml) = tree else {
                panic!("XML adapter returned a different tree");
            };
            assert_eq!(xml.source.as_ptr(), source.as_ptr());
            assert_eq!(xml.doc.root_element().text(), Some("value"));
        }
        source.clear();
        source.push_str("<next>changed</next>");
        let tree = adapter.parse(&source).unwrap();
        let SourceTree::Xml(xml) = tree else {
            panic!("XML adapter returned a different tree");
        };
        assert_eq!(xml.doc.root_element().text(), Some("changed"));
        assert!(adapter.parse("<invalid>").is_err());
    }
}
