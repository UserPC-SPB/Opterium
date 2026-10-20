use super::{Adapter, RawMatch, SourceTree, XmlTree};
use crate::template::{CompiledRule, ExtractMode, Format, SelectorCompiled};
use crate::xpath::{self, XMatch};
use roxmltree::Node;

pub struct XmlAdapter;

/// Text published for one selected element. The default (`text`) and
/// `outer_html` quote the element's exact source; `inner_html` quotes the
/// source between its start and end tags; `attr:NAME` is the attribute
/// value exactly as `/@NAME` would select it, and no item when absent.
fn element_value<'a>(node: Node<'a, '_>, extract: &ExtractMode) -> Result<Option<&'a str>, String> {
    Ok(match extract {
        ExtractMode::Text | ExtractMode::OuterHtml => Some(xpath::element_source(node)?),
        ExtractMode::InnerHtml => Some(inner_source(node)?),
        ExtractMode::Attr(name) => xpath::attribute_value(node, name),
    })
}

fn inner_source<'a>(node: Node<'a, '_>) -> Result<&'a str, String> {
    let outer = xpath::element_source(node)?;
    let (content_start, self_closing) = super::skip_tag(outer.as_bytes(), 1);
    if self_closing {
        return Ok("");
    }
    // An end tag `</name>` contains no `<`, so the last one starts it.
    match outer.rfind('<') {
        Some(content_end) if content_end >= content_start => Ok(&outer[content_start..content_end]),
        _ => Err(format!(
            "cannot locate the content of element '{}' in its source",
            node.tag_name().name()
        )),
    }
}

impl Adapter for XmlAdapter {
    fn format(&self) -> Format {
        Format::Xml
    }

    fn parse<'a>(&self, text: &'a str) -> Result<SourceTree<'a>, String> {
        XmlTree::parse(text).map(SourceTree::Xml)
    }

    fn selector_types(&self) -> &'static [&'static str] {
        &["xpath"]
    }

    fn extract(&self, tree: &SourceTree<'_>, rule: &CompiledRule) -> Result<Vec<RawMatch>, String> {
        let doc = match tree {
            SourceTree::Xml(t) => &t.doc,
            _ => return Err("xml adapter received non-xml tree".to_string()),
        };
        let expr = match &rule.selector {
            SelectorCompiled::XPath(expr) => expr,
            _ => return Err("xml adapter requires an xpath selector".to_string()),
        };
        // Templates are checked when compiled; rules built in code are not.
        xpath::validate_extract(expr, &rule.extract)?;
        xpath::validate_namespace_prefixes(expr, doc)?;
        if let ExtractMode::Attr(name) = &rule.extract {
            xpath::validate_attribute_prefix(name, doc)?;
        }
        let mut matches = Vec::new();
        for selected in xpath::select(expr, doc) {
            let value = match selected {
                XMatch::Element(node) => element_value(node, &rule.extract)?,
                XMatch::Value(value) => Some(value),
            };
            if let Some(value) = value {
                matches.push(RawMatch {
                    value: value.to_string(),
                    lines: None,
                });
            }
        }
        Ok(matches)
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
input: {{format: xml}}
rules:
  - id: r
    select: {{type: xpath, expression: "{}"}}
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
    fn xml_extract() {
        let a = XmlAdapter;
        let tree = a
            .parse(r#"<root><item a="1">x</item><item a="2">y</item></root>"#)
            .unwrap();
        let matches = a.extract(&tree, &rule("/root/item/@a")).unwrap();
        assert_eq!(
            matches.iter().map(|m| m.value.as_str()).collect::<Vec<_>>(),
            vec!["1", "2"]
        );
        let matches = a.extract(&tree, &rule("/root/item/text()")).unwrap();
        assert_eq!(
            matches.iter().map(|m| m.value.as_str()).collect::<Vec<_>>(),
            vec!["x", "y"]
        );
        let matches = a.extract(&tree, &rule("/root/item[1]")).unwrap();
        assert_eq!(matches.len(), 1);
        assert!(matches[0].value.contains("a=\"1\""));
    }

    fn values(tree: &SourceTree<'_>, expr: &str, extract: ExtractMode) -> Vec<String> {
        let mut rule = rule(expr);
        rule.extract = extract;
        XmlAdapter
            .extract(tree, &rule)
            .unwrap()
            .into_iter()
            .map(|m| m.value)
            .collect()
    }

    #[test]
    fn extract_modes_apply_to_selected_elements() {
        let source = concat!(
            "<root xmlns:p='urn:p' xml:lang='ru'>",
            "<item id='1' p:id='q' empty=''>a &amp; <b k=\"x>y\">bold</b><!--c--><![CDATA[<raw>]]></item>",
            "<item id='2'/>",
            "<item>  spaced  </item >",
            "</root>"
        );
        let tree = XmlAdapter.parse(source).unwrap();
        let attr = |name: &str| ExtractMode::Attr(name.to_string());
        assert_eq!(values(&tree, "//item", attr("id")), vec!["1", "2"]);
        assert_eq!(values(&tree, "//item", attr("p:id")), vec!["q"]);
        assert_eq!(values(&tree, "//item", attr("empty")), vec![""]);
        assert_eq!(values(&tree, "/root", attr("xml:lang")), vec!["ru"]);
        assert_eq!(
            values(&tree, "//item", ExtractMode::InnerHtml),
            vec![
                "a &amp; <b k=\"x>y\">bold</b><!--c--><![CDATA[<raw>]]>",
                "",
                "  spaced  "
            ]
        );
        let outer = values(&tree, "//item", ExtractMode::OuterHtml);
        assert_eq!(outer, values(&tree, "//item", ExtractMode::Text));
        assert_eq!(outer[1], "<item id='2'/>");
        assert_eq!(outer[2], "<item>  spaced  </item >");

        let mut wrong = rule("//item/@id");
        wrong.extract = attr("id");
        let error = XmlAdapter.extract(&tree, &wrong).unwrap_err();
        assert!(error.contains("applies to elements"), "{error}");
        let mut unknown = rule("//item");
        unknown.extract = attr("q:id");
        let error = XmlAdapter.extract(&tree, &unknown).unwrap_err();
        assert!(error.contains("unknown namespace prefix 'q'"), "{error}");
    }

    #[test]
    fn unknown_namespace_prefix_is_an_extraction_error() {
        let adapter = XmlAdapter;
        let tree = adapter.parse("<root><item>value</item></root>").unwrap();
        let error = adapter
            .extract(&tree, &rule("/root/unknown:item"))
            .unwrap_err();
        assert!(error.contains("unknown xpath namespace prefix 'unknown'"));
    }
}
