use crate::template::ExtractMode;
use roxmltree::Node;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum XEnd {
    Element,
    Attr(String),
    Text,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XStep {
    pub name: Option<String>,
    pub wildcard: bool,
    pub descendant: bool,
    pub predicates: Vec<XPredicate>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XPathExpr {
    pub absolute: bool,
    pub steps: Vec<XStep>,
    pub end: XEnd,
    pub end_descendant: bool,
}

pub fn parse(expr: &str) -> Result<XPathExpr, String> {
    let mut p = Parser {
        chars: expr.trim().chars().collect(),
        pos: 0,
    };
    p.parse_expr()
}

struct Parser {
    chars: Vec<char>,
    pos: usize,
}

impl Parser {
    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn starts_with(&self, s: &str) -> bool {
        if self.pos + s.len() > self.chars.len() {
            return false;
        }
        s.chars()
            .enumerate()
            .all(|(i, c)| self.chars[self.pos + i] == c)
    }

    fn parse_expr(&mut self) -> Result<XPathExpr, String> {
        let mut absolute = false;
        let mut first_descendant = false;
        if self.peek() == Some('/') {
            if self.starts_with("//") {
                self.pos += 2;
                first_descendant = true;
            } else {
                self.pos += 1;
                absolute = true;
            }
        }
        let mut steps = Vec::new();
        let mut end = XEnd::Element;
        let mut end_descendant = false;
        loop {
            self.skip_ws();
            let mut descendant = first_descendant;
            first_descendant = false;
            if self.starts_with("//") {
                self.pos += 2;
                descendant = true;
            }
            let (mut step, step_end) = self.parse_step()?;
            step.descendant = descendant;
            if let Some(e) = step_end {
                end = e;
                end_descendant = descendant;
                self.skip_ws();
                if let Some(c) = self.peek() {
                    return Err(format!(
                        "unexpected character '{}' after terminal xpath step at position {}",
                        c, self.pos
                    ));
                }
                break;
            }
            steps.push(step);
            self.skip_ws();
            if self.peek() == Some('/') {
                if !self.starts_with("//") {
                    self.pos += 1;
                }
            } else if self.peek().is_none() {
                break;
            } else {
                return Err(format!(
                    "unexpected character '{}' at position {}",
                    self.peek().unwrap(),
                    self.pos
                ));
            }
        }
        if steps.is_empty() && !end_descendant {
            return Err("empty xpath expression".to_string());
        }
        Ok(XPathExpr {
            absolute,
            steps,
            end,
            end_descendant,
        })
    }

    fn parse_step(&mut self) -> Result<(XStep, Option<XEnd>), String> {
        if self.starts_with("@") {
            self.pos += 1;
            let attr = self.parse_name_q()?;
            if attr.is_empty() {
                return Err("expected attribute name after '@'".to_string());
            }
            return Ok((empty_step(), Some(XEnd::Attr(attr))));
        }
        if self.starts_with("text()") {
            self.pos += 6;
            return Ok((empty_step(), Some(XEnd::Text)));
        }
        let wildcard = if self.peek() == Some('*') {
            self.pos += 1;
            true
        } else {
            false
        };
        let name = if wildcard {
            None
        } else {
            let n = self.parse_name_q()?;
            if n.is_empty() {
                return Err(format!("expected element name at position {}", self.pos));
            }
            Some(n)
        };

        let mut predicates = Vec::new();
        while self.peek() == Some('[') {
            self.pos += 1;
            predicates.push(self.parse_predicate()?);
            self.skip_ws();
            if self.peek() != Some(']') {
                return Err("expected ']' to close predicate".to_string());
            }
            self.pos += 1;
        }

        Ok((
            XStep {
                name,
                wildcard,
                descendant: false,
                predicates,
            },
            None,
        ))
    }

    fn parse_predicate(&mut self) -> Result<XPredicate, String> {
        self.skip_ws();
        if self.peek() == Some('@') {
            self.pos += 1;
            let attr = self.parse_name_q()?;
            if attr.is_empty() {
                return Err("expected attribute name in predicate".to_string());
            }
            self.skip_ws();
            if self.peek() == Some(']') {
                return Ok(XPredicate::Attr(attr, None));
            }
            self.expect_char('=')?;
            self.skip_ws();
            let value = self.parse_quoted()?;
            return Ok(XPredicate::Attr(attr, Some(value)));
        }
        if self.starts_with("text()") {
            self.pos += 6;
            self.skip_ws();
            self.expect_char('=')?;
            self.skip_ws();
            let value = self.parse_quoted()?;
            return Ok(XPredicate::Text(value));
        }
        let digits: String = self.chars[self.pos..]
            .iter()
            .take_while(|c| c.is_ascii_digit())
            .collect();
        if digits.is_empty() {
            return Err("expected predicate: index, @attr='v', or text()='v'".to_string());
        }
        self.pos += digits.len();
        let n: usize = digits
            .parse()
            .map_err(|_| format!("invalid predicate index '{}'", digits))?;
        if n == 0 {
            return Err("predicate index is 1-based; 0 is not allowed".to_string());
        }
        Ok(XPredicate::Index(n))
    }

    fn parse_quoted(&mut self) -> Result<String, String> {
        match self.peek() {
            Some('\'') | Some('"') => {
                let q = self.peek().unwrap();
                self.pos += 1;
                let mut s = String::new();
                while let Some(c) = self.peek() {
                    if c == q {
                        self.pos += 1;
                        return Ok(s);
                    }
                    self.pos += 1;
                    s.push(c);
                }
                Err("unterminated quoted string".to_string())
            }
            other => Err(format!("expected quoted string, got {:?}", other)),
        }
    }

    fn expect_char(&mut self, c: char) -> Result<(), String> {
        self.skip_ws();
        match self.peek() {
            Some(a) if a == c => {
                self.pos += 1;
                Ok(())
            }
            other => Err(format!("expected '{}', got {:?}", c, other)),
        }
    }

    fn skip_ws(&mut self) {
        while self.peek().is_some_and(char::is_whitespace) {
            self.pos += 1;
        }
    }

    fn parse_name_q(&mut self) -> Result<String, String> {
        let mut s = self.parse_name();
        if self.peek() == Some(':') {
            if s.is_empty() {
                return Err("qualified name requires a prefix before ':'".to_string());
            }
            self.pos += 1;
            let local = self.parse_name();
            if local.is_empty() {
                return Err("qualified name requires a local name after ':'".to_string());
            }
            s.push(':');
            s.push_str(&local);
        }
        Ok(s)
    }

    fn parse_name(&mut self) -> String {
        let start = self.pos;
        if !self.peek().is_some_and(|c| c == '_' || c.is_alphabetic()) {
            return String::new();
        }
        self.pos += 1;
        while let Some(c) = self.peek() {
            if c.is_alphanumeric() || matches!(c, '_' | '-' | '.') {
                self.pos += 1;
            } else {
                break;
            }
        }
        self.chars[start..self.pos].iter().collect()
    }
}

fn empty_step() -> XStep {
    XStep {
        name: None,
        wildcard: true,
        descendant: false,
        predicates: Vec::new(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum XPredicate {
    Index(usize),
    Attr(String, Option<String>),
    Text(String),
}

fn namespace_uri<'a>(doc: &'a roxmltree::Document<'_>, prefix: &str) -> Option<&'a str> {
    if prefix == "xml" {
        Some("http://www.w3.org/XML/1998/namespace")
    } else {
        // The DSL has no separate namespace map. Query prefixes use the fixed
        // bindings declared on the source document's root element.
        doc.root_element().lookup_namespace_uri(Some(prefix))
    }
}

pub fn validate_namespace_prefixes(
    expr: &XPathExpr,
    doc: &roxmltree::Document<'_>,
) -> Result<(), String> {
    let mut names = Vec::new();
    for step in &expr.steps {
        if let Some(name) = &step.name {
            names.push(name.as_str());
        }
        for predicate in &step.predicates {
            if let XPredicate::Attr(name, _) = predicate {
                names.push(name.as_str());
            }
        }
    }
    if let XEnd::Attr(name) = &expr.end {
        names.push(name.as_str());
    }
    for name in names {
        if let Some((prefix, _)) = name.split_once(':') {
            if namespace_uri(doc, prefix).is_none() {
                return Err(format!(
                    "unknown xpath namespace prefix '{}': declare it on the XML root element",
                    prefix
                ));
            }
        }
    }
    Ok(())
}

/// Checks the prefix of an `extract: attr:NAME` attribute name against the
/// root element's bindings, like the prefixes inside the expression.
pub fn validate_attribute_prefix(name: &str, doc: &roxmltree::Document<'_>) -> Result<(), String> {
    match name.split_once(':') {
        Some((prefix, _)) if namespace_uri(doc, prefix).is_none() => Err(format!(
            "unknown namespace prefix '{}' in extract 'attr:{}': declare it on the XML root element",
            prefix, name
        )),
        _ => Ok(()),
    }
}

/// Template-time check that `extract` fits what the expression selects:
/// the element modes (`attr:NAME`, `inner_html`, `outer_html`) need an
/// expression that ends at an element, and `attr:` needs a valid name.
pub fn validate_extract(expr: &XPathExpr, extract: &ExtractMode) -> Result<(), String> {
    let selected = match &expr.end {
        XEnd::Element => {
            return match extract {
                ExtractMode::Attr(name) if !is_qualified_name(name) => Err(format!(
                    "extract 'attr:{}' needs an XML attribute name such as attr:id or attr:xml:lang",
                    name
                )),
                _ => Ok(()),
            };
        }
        XEnd::Attr(name) => format!("attribute values (@{})", name),
        XEnd::Text => "text nodes (text())".to_string(),
    };
    let mode = match extract {
        ExtractMode::Text => return Ok(()),
        ExtractMode::Attr(name) => format!("attr:{}", name),
        ExtractMode::InnerHtml => "inner_html".to_string(),
        ExtractMode::OuterHtml => "outer_html".to_string(),
    };
    Err(format!(
        "extract '{}' applies to elements, but the xpath selects {}; end the xpath at an element or remove 'extract'",
        mode, selected
    ))
}

fn is_qualified_name(name: &str) -> bool {
    let mut parser = Parser {
        chars: name.chars().collect(),
        pos: 0,
    };
    parser
        .parse_name_q()
        .is_ok_and(|parsed| !parsed.is_empty() && parser.pos == parser.chars.len())
}

/// Value of an attribute of `node`: unprefixed names match attributes
/// without a namespace, prefixed names use the root element's bindings.
pub fn attribute_value<'a, 'i>(node: Node<'a, 'i>, name: &str) -> Option<&'a str> {
    if let Some((prefix, local)) = name.split_once(':') {
        let uri = namespace_uri(node.document(), prefix)?;
        node.attribute((uri, local))
    } else {
        node.attribute(name)
    }
}

fn child_elements<'a, 'i>(n: Node<'a, 'i>) -> Vec<Node<'a, 'i>> {
    n.children().filter(|c| c.is_element()).collect()
}

/// The namespace of an element; `xmlns=""` means no namespace.
fn element_namespace<'a>(n: Node<'a, '_>) -> Option<&'a str> {
    n.tag_name().namespace().filter(|uri| !uri.is_empty())
}

/// The default namespace (`xmlns="..."`) declared on the root element.
/// Unprefixed element names in a query match elements in this namespace as
/// well as elements without a namespace, so `/project/version` works on a
/// `pom.xml` and `//entry` on an Atom feed.
fn root_default_namespace<'a>(doc: &'a roxmltree::Document<'_>) -> Option<&'a str> {
    doc.root_element()
        .default_namespace()
        .filter(|uri| !uri.is_empty())
}

fn matches_name<'a, 'i>(n: Node<'a, 'i>, step: &XStep, default_namespace: Option<&str>) -> bool {
    if !n.is_element() {
        return false;
    }
    step.wildcard
        || step.name.as_ref().is_some_and(|name| {
            if let Some((prefix, local)) = name.split_once(':') {
                namespace_uri(n.document(), prefix).is_some_and(|uri| {
                    n.tag_name().name() == local && n.tag_name().namespace() == Some(uri)
                })
            } else {
                n.tag_name().name() == name
                    && element_namespace(n).is_none_or(|uri| default_namespace == Some(uri))
            }
        })
}

fn select_children<'a, 'i>(
    parent: Node<'a, 'i>,
    step: &XStep,
    default_namespace: Option<&str>,
) -> Vec<Node<'a, 'i>> {
    let mut nodes: Vec<Node> = child_elements(parent)
        .into_iter()
        .filter(|n| matches_name(*n, step, default_namespace))
        .collect();
    // XPath predicates filter each parent's candidate list in source order.
    for predicate in &step.predicates {
        match predicate {
            XPredicate::Index(index) => {
                nodes = index
                    .checked_sub(1)
                    .and_then(|index| nodes.get(index))
                    .copied()
                    .into_iter()
                    .collect();
            }
            XPredicate::Attr(attr, value) => nodes.retain(|n| match attribute_value(*n, attr) {
                Some(_) if value.is_none() => true,
                Some(actual) => Some(actual) == value.as_deref(),
                None => false,
            }),
            XPredicate::Text(value) => nodes.retain(|n| {
                n.children()
                    .filter(|child| child.is_text())
                    .any(|child| child.text() == Some(value.as_str()))
            }),
        }
    }
    nodes
}

/// One result of an expression, in document order.
#[derive(Debug, Clone, Copy)]
pub enum XMatch<'a, 'i> {
    /// The expression ends at an element.
    Element(Node<'a, 'i>),
    /// The value of an attribute (`@name`) or a text node (`text()`).
    Value(&'a str),
}

pub fn select<'a, 'i>(expr: &XPathExpr, doc: &'a roxmltree::Document<'i>) -> Vec<XMatch<'a, 'i>> {
    let default_namespace = root_default_namespace(doc);
    let mut nodes: Vec<Node> = vec![doc.root()];

    for step in &expr.steps {
        let mut next: Vec<Node> = Vec::new();
        for node in &nodes {
            if step.descendant {
                // `//` expands to descendant-or-self followed by a child step.
                // Evaluate positions independently for every parent's children.
                for parent in node.descendants() {
                    next.extend(select_children(parent, step, default_namespace));
                }
            } else {
                next.extend(select_children(*node, step, default_namespace));
            }
        }
        // Overlapping descendant contexts may visit the same node more than once.
        // roxmltree's Node ordering is document order within the same document.
        next.sort_unstable();
        next.dedup();
        nodes = next;
        if nodes.is_empty() {
            break;
        }
    }

    match &expr.end {
        XEnd::Element => nodes.into_iter().map(XMatch::Element).collect(),
        XEnd::Attr(a) => {
            if expr.end_descendant {
                nodes = nodes.into_iter().flat_map(|n| n.descendants()).collect();
                nodes.sort_unstable();
                nodes.dedup();
            }
            nodes
                .into_iter()
                .filter_map(|n| attribute_value(n, a))
                .map(XMatch::Value)
                .collect()
        }
        XEnd::Text => {
            let mut text_nodes = Vec::new();
            for n in nodes {
                if expr.end_descendant {
                    text_nodes.extend(n.descendants().filter(|child| child.is_text()));
                } else {
                    text_nodes.extend(n.children().filter(|child| child.is_text()));
                }
            }
            text_nodes.sort_unstable();
            text_nodes.dedup();
            text_nodes
                .into_iter()
                .filter_map(|n| n.text())
                .map(XMatch::Value)
                .collect()
        }
    }
}

/// The exact source text of an element, start tag to end tag. Elements
/// that come from an internal entity are quoted from its declaration.
pub fn element_source<'a>(node: Node<'a, '_>) -> Result<&'a str, String> {
    node.document()
        .input_text()
        .get(node.range())
        .ok_or_else(|| {
            format!(
                "source range of element '{}' is not available",
                node.tag_name().name()
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Results as the XML adapter publishes them with the default extract.
    fn evaluate(expr: &XPathExpr, doc: &roxmltree::Document<'_>) -> Vec<String> {
        select(expr, doc)
            .into_iter()
            .map(|selected| match selected {
                XMatch::Element(node) => element_source(node).unwrap().to_string(),
                XMatch::Value(value) => value.to_string(),
            })
            .collect()
    }

    fn doc() -> roxmltree::Document<'static> {
        roxmltree::Document::parse(
            r#"<root>
  <item id="1" kind="a">alpha</item>
  <item id="2" kind="b">beta</item>
  <item id="3" kind="a">gamma</item>
  <sub><item id="4" kind="c">delta</item></sub>
</root>"#,
        )
        .unwrap()
    }

    #[test]
    fn basic() {
        let d = doc();
        assert_eq!(evaluate(&parse("/root/item").unwrap(), &d).len(), 3);
        assert_eq!(evaluate(&parse("/root/item[2]").unwrap(), &d).len(), 1);
        assert_eq!(
            evaluate(&parse("/root/item[@kind='a']").unwrap(), &d).len(),
            2
        );
        assert_eq!(evaluate(&parse("//item").unwrap(), &d).len(), 4);
        assert_eq!(
            evaluate(&parse("/root/item/@id").unwrap(), &d),
            vec!["1", "2", "3"]
        );
        assert_eq!(evaluate(&parse("/root/item/text()").unwrap(), &d).len(), 3);
        assert!(parse("/root/item[0]").is_err());
    }

    #[test]
    fn positions_count_matching_children_for_each_parent() {
        let d = roxmltree::Document::parse(
            r#"<root>
                <other id="skip"/><item id="a"/><other/><item id="b"/>
                <group><other/><item id="c"/><item id="d"/></group>
                <group><item id="e"/><other/><item id="f"/></group>
            </root>"#,
        )
        .unwrap();
        assert_eq!(
            evaluate(&parse("/root/item[1]/@id").unwrap(), &d),
            vec!["a"]
        );
        assert_eq!(
            evaluate(&parse("/root/item[2]/@id").unwrap(), &d),
            vec!["b"]
        );
        assert_eq!(evaluate(&parse("/root/*[2]/@id").unwrap(), &d), vec!["a"]);
        assert_eq!(
            evaluate(&parse("/root/group/item[1]/@id").unwrap(), &d),
            vec!["c", "e"]
        );
        assert_eq!(
            evaluate(&parse("/root/group/item[2]/@id").unwrap(), &d),
            vec!["d", "f"]
        );
    }

    #[test]
    fn predicates_apply_in_source_order() {
        let d = roxmltree::Document::parse(
            r#"<root><item id="a" kind="x"/><item id="b" kind="y"/><item id="c" kind="x"/></root>"#,
        )
        .unwrap();
        assert_eq!(
            evaluate(&parse("/root/item[@kind='x'][2]/@id").unwrap(), &d),
            vec!["c"]
        );
        assert!(evaluate(&parse("/root/item[2][@kind='x']/@id").unwrap(), &d).is_empty());
        assert_eq!(
            evaluate(&parse("/root/item[2][1]/@id").unwrap(), &d),
            vec!["b"]
        );
        assert!(evaluate(&parse("/root/item[1][2]/@id").unwrap(), &d).is_empty());
    }

    #[test]
    fn descendants_keep_document_order_and_per_parent_positions() {
        let d = roxmltree::Document::parse(
            r#"<root id="r"><item id="a"/><branch><item id="b"/><nested><item id="c"/></nested><item id="d"/></branch><item id="e"/></root>"#,
        )
        .unwrap();
        assert_eq!(
            evaluate(&parse("//item/@id").unwrap(), &d),
            vec!["a", "b", "c", "d", "e"]
        );
        assert_eq!(
            evaluate(&parse("/root//item/@id").unwrap(), &d),
            vec!["a", "b", "c", "d", "e"]
        );
        assert_eq!(
            evaluate(&parse("//item[1]/@id").unwrap(), &d),
            vec!["a", "b", "c"]
        );
        assert_eq!(
            evaluate(&parse("//item[2]/@id").unwrap(), &d),
            vec!["d", "e"]
        );
        assert_eq!(evaluate(&parse("//root/@id").unwrap(), &d), vec!["r"]);
        assert_eq!(
            evaluate(&parse("//*//item/@id").unwrap(), &d),
            vec!["a", "b", "c", "d", "e"]
        );
    }

    #[test]
    fn terminal_steps_reject_trailing_input_and_invalid_qualified_names() {
        for expression in [
            "/root/@id/anything",
            "/root/@id[1]",
            "/root/text()junk",
            "/root/text()/name",
            "/root/p:",
            "/root/@p:",
            "/root/@:id",
            "/root/123name",
            "/root/item[@]",
        ] {
            assert!(
                parse(expression).is_err(),
                "accepted invalid xpath {expression}"
            );
        }
        assert!(parse("/root/item[ @kind = 'x' ][ 2 ]/@id").is_ok());
    }

    #[test]
    fn namespace_prefixes_resolve_to_root_bindings_for_elements_and_attributes() {
        let d = roxmltree::Document::parse(
            r#"<root xmlns:a="urn:a" xmlns:alias="urn:a" xmlns:b="urn:b">
                <a:item id="a" a:id="qualified" a:kind="chosen"/>
                <b:item id="b" b:kind="chosen"/>
                <item id="plain"/>
                <branch xmlns:a="urn:other"><a:item id="rebound"/></branch>
            </root>"#,
        )
        .unwrap();
        for expression in ["//a:item/@id", "//alias:item/@id"] {
            let expression = parse(expression).unwrap();
            validate_namespace_prefixes(&expression, &d).unwrap();
            assert_eq!(evaluate(&expression, &d), vec!["a"]);
        }
        assert_eq!(evaluate(&parse("//b:item/@id").unwrap(), &d), vec!["b"]);
        assert_eq!(evaluate(&parse("//item/@id").unwrap(), &d), vec!["plain"]);
        assert_eq!(
            evaluate(&parse("//a:item/@a:id").unwrap(), &d),
            vec!["qualified"]
        );
        assert_eq!(
            evaluate(&parse("/root/*[@a:kind='chosen']/@id").unwrap(), &d),
            vec!["a"]
        );
        let unknown = parse("//missing:item/@id").unwrap();
        assert!(validate_namespace_prefixes(&unknown, &d).is_err());
        assert!(evaluate(&unknown, &d).is_empty());
    }

    #[test]
    fn qualified_root_and_xml_attributes_do_not_match_other_namespaces() {
        let d = roxmltree::Document::parse(
            r#"<a:root xmlns:a="urn:a" xmlns:b="urn:b" xml:lang="ru">
                <a:item id="a"/><b:item id="b"/>
            </a:root>"#,
        )
        .unwrap();
        assert_eq!(
            evaluate(&parse("/a:root/a:item/@id").unwrap(), &d),
            vec!["a"]
        );
        assert!(evaluate(&parse("/b:root/a:item/@id").unwrap(), &d).is_empty());
        assert!(evaluate(&parse("/root/a:item/@id").unwrap(), &d).is_empty());
        let language = parse("/a:root/@xml:lang").unwrap();
        validate_namespace_prefixes(&language, &d).unwrap();
        assert_eq!(evaluate(&language, &d), vec!["ru"]);
    }

    #[test]
    fn direct_and_descendant_terminal_nodes_keep_order_and_exact_text() {
        let d = roxmltree::Document::parse(
            r#"<root id="root"> before <child id="child">inside</child> after <child id="last"/></root>"#,
        )
        .unwrap();
        assert_eq!(
            evaluate(&parse("/root/text()").unwrap(), &d),
            vec![" before ", " after "]
        );
        assert_eq!(
            evaluate(&parse("/root//text()").unwrap(), &d),
            vec![" before ", "inside", " after "]
        );
        assert_eq!(
            evaluate(&parse("/root//@id").unwrap(), &d),
            vec!["root", "child", "last"]
        );
        assert_eq!(
            evaluate(&parse("//@id").unwrap(), &d),
            vec!["root", "child", "last"]
        );
        assert_eq!(
            evaluate(&parse("//*//text()").unwrap(), &d),
            vec![" before ", "inside", " after "]
        );
        assert_eq!(
            evaluate(&parse("/root[text()=' after ']/@id").unwrap(), &d),
            vec!["root"]
        );
    }

    #[test]
    fn selected_subtree_preserves_source_markup_entities_and_comments() {
        let source = r#"<root><p:item xmlns:p='urn:p' p:attr='a&amp;b'> a &lt;b&gt; &amp; c<!--keep--><![CDATA[<raw>]]><?keep yes?></p:item></root>"#;
        let d = roxmltree::Document::parse(source).unwrap();
        let expected = r#"<p:item xmlns:p='urn:p' p:attr='a&amp;b'> a &lt;b&gt; &amp; c<!--keep--><![CDATA[<raw>]]><?keep yes?></p:item>"#;
        assert_eq!(evaluate(&parse("/root/*").unwrap(), &d), vec![expected]);
    }

    #[test]
    fn unprefixed_names_match_the_root_default_namespace() {
        let d = roxmltree::Document::parse(
            r#"<project xmlns="urn:pom" xmlns:x="urn:x" id="p" x:id="qualified">
                <version>1.0</version>
                <x:version>prefixed</x:version>
                <plain xmlns=""><version>none</version></plain>
                <svg xmlns="urn:svg"><version>other</version></svg>
            </project>"#,
        )
        .unwrap();
        assert_eq!(
            evaluate(&parse("/project/version/text()").unwrap(), &d),
            vec!["1.0"]
        );
        // Elements without a namespace keep matching, other defaults do not.
        assert_eq!(
            evaluate(&parse("//version/text()").unwrap(), &d),
            vec!["1.0", "none"]
        );
        assert_eq!(
            evaluate(&parse("//x:version/text()").unwrap(), &d),
            vec!["prefixed"]
        );
        // Unprefixed attributes still mean "no namespace".
        assert_eq!(evaluate(&parse("/project/@id").unwrap(), &d), vec!["p"]);
        assert_eq!(
            evaluate(&parse("/project/@x:id").unwrap(), &d),
            vec!["qualified"]
        );
        assert_eq!(
            evaluate(&parse("/*[@id='p']/version/text()").unwrap(), &d),
            vec!["1.0"]
        );

        // A default namespace declared below the root is not a query default.
        let nested = roxmltree::Document::parse(
            r#"<root><feed xmlns="urn:atom"><entry/></feed><entry/></root>"#,
        )
        .unwrap();
        assert_eq!(
            evaluate(&parse("//entry").unwrap(), &nested),
            vec!["<entry/>"]
        );
    }

    #[test]
    fn extract_must_fit_what_the_expression_selects() {
        let check = |expression: &str, extract: ExtractMode| {
            validate_extract(&parse(expression).unwrap(), &extract)
        };
        for extract in [
            ExtractMode::Text,
            ExtractMode::InnerHtml,
            ExtractMode::OuterHtml,
            ExtractMode::Attr("id".into()),
            ExtractMode::Attr("xml:lang".into()),
        ] {
            assert!(check("//item", extract).is_ok());
        }
        for name in ["", "@id", "a b", "p:", ":id", "1st"] {
            let error = check("//item", ExtractMode::Attr(name.into())).unwrap_err();
            assert!(error.contains("needs an XML attribute name"), "{error}");
        }
        assert!(check("//item/@id", ExtractMode::Text).is_ok());
        assert!(check("//item/text()", ExtractMode::Text).is_ok());
        let error = check("//item/@id", ExtractMode::Attr("id".into())).unwrap_err();
        assert_eq!(
            error,
            "extract 'attr:id' applies to elements, but the xpath selects attribute values (@id); end the xpath at an element or remove 'extract'"
        );
        let error = check("//item//text()", ExtractMode::InnerHtml).unwrap_err();
        assert!(error.contains("selects text nodes (text())"), "{error}");
        assert!(check("//item/@id", ExtractMode::OuterHtml).is_err());
    }
}
