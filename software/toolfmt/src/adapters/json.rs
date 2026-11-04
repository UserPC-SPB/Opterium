//! JSON sources. serde_json parses them, so its error messages and nesting
//! limit apply. It reads a number as an integer when it fits 64 bits and as
//! a double otherwise, and prints the double in its own spelling: `1E16` and
//! `1e16` print as `1e+16`, `1.50` as `1.5`, `-0` as `-0.0`, and a 30-digit
//! integer comes out rounded (`1.2345678901234568e+29`). Items keep every
//! number as the source wrote it: a [`JsonDocument`] remembers the source
//! text of each number whose printed form would differ.

use super::{Adapter, RawMatch, SourceTree};
use crate::template::{CompiledRule, Format};
use serde_json::{Map, Number, Value};
use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::ops::Range;

pub struct JsonAdapter;

pub fn json_value_to_string(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => "<null>".to_string(),
        other => serde_json::to_string(other).unwrap_or_default(),
    }
}

/// A JSON source parsed by serde_json, plus the source text of each number
/// that the value tree would not print as written.
#[derive(Debug)]
pub struct JsonDocument<'a> {
    /// Boxed so that the root keeps its address when the document moves:
    /// `number_text` is keyed by node address, like the node set of
    /// [`crate::jsonpath::evaluate`].
    root: Box<Value>,
    number_text: HashMap<*const Value, &'a str>,
}

impl<'a> JsonDocument<'a> {
    pub fn parse(text: &'a str) -> Result<Self, String> {
        let root: Box<Value> =
            Box::new(serde_json::from_str(text).map_err(|e| format!("invalid JSON: {}", e))?);
        let number_text = if has_reprinted_number(text) {
            NumberWalk::run(text, &root)
        } else {
            HashMap::new()
        };
        Ok(JsonDocument { root, number_text })
    }

    pub fn value(&self) -> &Value {
        &self.root
    }

    /// The item text of `value`, a node of this document: a string as is,
    /// null as `<null>`, anything else as compact JSON with every number as
    /// the source wrote it.
    pub fn item_text(&self, value: &Value) -> String {
        if self.number_text.is_empty()
            || !matches!(value, Value::Number(_) | Value::Array(_) | Value::Object(_))
        {
            return json_value_to_string(value);
        }
        let mut out = String::new();
        self.write(value, &mut out);
        out
    }

    /// serde_json's compact form of `value`, but with source number text.
    fn write(&self, value: &Value, out: &mut String) {
        match value {
            Value::Number(_) => match self.number_text.get(&std::ptr::from_ref(value)) {
                Some(text) => out.push_str(text),
                None => push_json(value, out),
            },
            Value::Array(items) => {
                out.push('[');
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    self.write(item, out);
                }
                out.push(']');
            }
            Value::Object(map) => {
                out.push('{');
                for (i, (key, item)) in map.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    push_json(key, out);
                    out.push(':');
                    self.write(item, out);
                }
                out.push('}');
            }
            other => push_json(other, out),
        }
    }
}

fn push_json<T: serde::Serialize + ?Sized>(value: &T, out: &mut String) {
    out.push_str(&serde_json::to_string(value).unwrap_or_default());
}

/// Whether some number in the valid JSON `text` would not print as written.
fn has_reprinted_number(text: &str) -> bool {
    let bytes = text.as_bytes();
    let mut pos = 0;
    while pos < bytes.len() {
        match bytes[pos] {
            b'"' => pos = string_end(bytes, pos),
            b'-' | b'0'..=b'9' => {
                let end = number_end(bytes, pos);
                if !prints_as_written(&text[pos..end]) {
                    return true;
                }
                pos = end;
            }
            _ => pos += 1,
        }
    }
    false
}

/// Whether serde_json prints the number it reads from `token` as `token`.
fn prints_as_written(token: &str) -> bool {
    // JSON integers have no leading zeros and no `+`, so one that fits 64
    // bits prints as written; `-0` is read as a double (`-0.0`).
    if !token.contains(['.', 'e', 'E'])
        && token != "-0"
        && (token.parse::<u64>().is_ok() || token.parse::<i64>().is_ok())
    {
        return true;
    }
    let Ok(number) = token.parse::<Number>() else {
        return false;
    };
    // serde_json prints a number in at most 25 bytes: no allocation per
    // number, and a longer token cannot be printed as written anyway.
    let mut buffer = [0u8; 32];
    let mut rest = &mut buffer[..];
    if serde_json::to_writer(&mut rest, &number).is_err() {
        return false;
    }
    let unused = rest.len();
    &buffer[..buffer.len() - unused] == token.as_bytes()
}

/// The end of the string literal whose opening quote is at `start`.
fn string_end(bytes: &[u8], start: usize) -> usize {
    let mut pos = start + 1;
    while pos < bytes.len() {
        match bytes[pos] {
            b'"' => return pos + 1,
            b'\\' => pos += 2,
            _ => pos += 1,
        }
    }
    bytes.len()
}

/// The end of the number literal that starts at `start`.
fn number_end(bytes: &[u8], start: usize) -> usize {
    bytes[start..]
        .iter()
        .position(|b| !matches!(b, b'0'..=b'9' | b'-' | b'+' | b'.' | b'e' | b'E'))
        .map_or(bytes.len(), |len| start + len)
}

/// Reads valid JSON text alongside the tree serde_json parsed from it and
/// collects, by node address, the source text of the numbers that do not
/// print as written. Its recursion is bounded by serde_json's nesting limit.
struct NumberWalk<'a> {
    text: &'a str,
    pos: usize,
    /// In source order. Entries from a member that serde_json replaced with
    /// a later member of the same key are cleared to a null address.
    found: Vec<(*const Value, &'a str)>,
}

impl<'a> NumberWalk<'a> {
    fn run(text: &'a str, root: &Value) -> HashMap<*const Value, &'a str> {
        let mut walk = NumberWalk {
            text,
            pos: 0,
            found: Vec::new(),
        };
        walk.value(Some(root));
        walk.found
            .into_iter()
            .filter(|(node, _)| !node.is_null())
            .collect()
    }

    /// The byte at the current position, 0 at the end (JSON has no NUL
    /// outside strings).
    fn peek(&self) -> u8 {
        self.text.as_bytes().get(self.pos).copied().unwrap_or(0)
    }

    fn skip_whitespace(&mut self) {
        while matches!(self.peek(), b' ' | b'\t' | b'\n' | b'\r') {
            self.pos += 1;
        }
    }

    /// The next byte after whitespace, consumed.
    fn next_byte(&mut self) -> u8 {
        self.skip_whitespace();
        let byte = self.peek();
        if byte != 0 {
            self.pos += 1;
        }
        byte
    }

    /// The value at the current position. `node` is its node in the tree:
    /// `None` within a member that serde_json replaced by a later one of the
    /// same key, when that member's value has another shape.
    fn value(&mut self, node: Option<&Value>) {
        self.skip_whitespace();
        match self.peek() {
            b'{' => self.object(node.and_then(Value::as_object)),
            b'[' => self.array(node.and_then(Value::as_array)),
            b'"' => self.pos = string_end(self.text.as_bytes(), self.pos),
            b'-' | b'0'..=b'9' => {
                let start = self.pos;
                self.pos = number_end(self.text.as_bytes(), start);
                let token = &self.text[start..self.pos];
                if let Some(node @ Value::Number(_)) = node
                    && !prints_as_written(token)
                {
                    self.found.push((std::ptr::from_ref(node), token));
                }
            }
            b't' | b'n' => self.pos += 4,
            b'f' => self.pos += 5,
            _ => {}
        }
    }

    fn array(&mut self, items: Option<&Vec<Value>>) {
        self.pos += 1;
        self.skip_whitespace();
        if self.peek() == b']' {
            self.pos += 1;
            return;
        }
        for index in 0.. {
            self.value(items.and_then(|items| items.get(index)));
            if self.next_byte() != b',' {
                return;
            }
        }
    }

    fn object(&mut self, map: Option<&Map<String, Value>>) {
        self.pos += 1;
        self.skip_whitespace();
        if self.peek() == b'}' {
            self.pos += 1;
            return;
        }
        // Each member's key and the range of `found` its value added.
        let mut members: Vec<(Cow<'a, str>, Range<usize>)> = Vec::new();
        loop {
            self.skip_whitespace();
            let key = self.key();
            self.next_byte(); // `:`
            let start = self.found.len();
            self.value(map.and_then(|map| map.get(key.as_ref())));
            if map.is_some() {
                members.push((key, start..self.found.len()));
            }
            if self.next_byte() != b',' {
                break;
            }
        }
        // serde_json keeps the last value of a repeated key (at the place
        // of its first member); the earlier members' numbers are not in
        // the tree.
        if let Some(map) = map
            && map.len() < members.len()
        {
            let mut later = HashSet::new();
            for (key, range) in members.iter().rev() {
                if !later.insert(key.as_ref()) {
                    for entry in &mut self.found[range.clone()] {
                        entry.0 = std::ptr::null();
                    }
                }
            }
        }
    }

    /// The object key at the current position, decoded.
    fn key(&mut self) -> Cow<'a, str> {
        let start = self.pos;
        self.pos = string_end(self.text.as_bytes(), start);
        let literal = &self.text[start..self.pos];
        let inner = literal
            .get(1..literal.len().saturating_sub(1))
            .unwrap_or_default();
        if inner.contains('\\') {
            Cow::Owned(serde_json::from_str(literal).unwrap_or_default())
        } else {
            Cow::Borrowed(inner)
        }
    }
}

impl Adapter for JsonAdapter {
    fn format(&self) -> Format {
        Format::Json
    }

    fn parse<'a>(&self, text: &'a str) -> Result<SourceTree<'a>, String> {
        JsonDocument::parse(text).map(SourceTree::JsonSource)
    }

    fn selector_types(&self) -> &'static [&'static str] {
        &["jsonpath", "path"]
    }

    fn extract(&self, tree: &SourceTree<'_>, rule: &CompiledRule) -> Result<Vec<RawMatch>, String> {
        let doc = match tree {
            SourceTree::JsonSource(doc) => doc,
            _ => return Err("json adapter received non-json tree".to_string()),
        };
        let matches = super::match_value(&rule.selector, doc.value())?;
        Ok(matches
            .into_iter()
            .map(|m| RawMatch {
                value: doc.item_text(m),
                lines: None,
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule(selector: &str, expression: &str) -> CompiledRule {
        let template = crate::template::compile_yaml_str(
            &format!(
                "version: 1\nname: t\ninput: {{format: json}}\nrules:\n  - id: r\n    select: {{type: {selector}, expression: '{expression}'}}\n"
            ),
            &std::path::PathBuf::from("t.yml"),
            None,
            None,
        )
        .unwrap();
        template.rules[0].clone()
    }

    fn items(text: &str, expression: &str) -> Vec<String> {
        let tree = JsonAdapter.parse(text).unwrap();
        JsonAdapter
            .extract(&tree, &rule("jsonpath", expression))
            .unwrap()
            .into_iter()
            .map(|m| m.value)
            .collect()
    }

    fn document(text: &str) -> JsonDocument<'_> {
        match JsonAdapter.parse(text).unwrap() {
            SourceTree::JsonSource(doc) => doc,
            other => panic!("not a JSON document: {other:?}"),
        }
    }

    #[test]
    fn extract_subtree() {
        let a = JsonAdapter;
        let tree = a.parse(r#"{"a": {"b": [1, 2]}, "s": "x"}"#).unwrap();
        let rule = crate::template::compile_yaml_str(
            r#"
version: 1
name: t
input: {format: json}
rules:
  - id: r
    select: {type: jsonpath, expression: "$.a"}
    output: {key: k}
"#,
            &std::path::PathBuf::from("t.yml"),
            None,
            None,
        )
        .unwrap();
        let rule = &rule.rules[0];
        let out = a.extract(&tree, rule).unwrap();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].value, r#"{"b":[1,2]}"#);
    }

    #[test]
    fn scalar_values() {
        assert_eq!(json_value_to_string(&Value::Bool(true)), "true");
        assert_eq!(json_value_to_string(&Value::from(3)), "3");
        assert_eq!(json_value_to_string(&Value::Null), "<null>");
    }

    /// serde_json reads a 30-digit integer as a rounded double and reprints
    /// other numbers in its own spelling; items keep the source text.
    #[test]
    fn numbers_keep_their_source_text() {
        let big = r#"{"n":123456789012345678901234567890,"f":5.0}"#;
        // Before: 1.2345678901234568e+29.
        assert_eq!(items(big, "$.n"), ["123456789012345678901234567890"]);
        assert_eq!(items(big, "$.f"), ["5.0"]);
        assert_eq!(items(big, "$"), [big]);
        let tree = JsonAdapter.parse(big).unwrap();
        let path = JsonAdapter.extract(&tree, &rule("path", "n")).unwrap();
        assert_eq!(path[0].value, "123456789012345678901234567890");

        let numbers = [
            "123456789012345678901234567890",
            "-123456789012345678901234567890",
            "18446744073709551616",
            "18446744073709551615",
            "-9223372036854775808",
            "42",
            "0",
            "-0",
            "5.0",
            "1.50",
            "0.1",
            "1e16",
            "1E16",
            "1e+16",
            "1.0e-7",
            "2.5E-3",
        ];
        let source = format!("{{\"n\": [{}]}}", numbers.join(", "));
        assert_eq!(items(&source, "$.n[*]"), numbers);
        assert_eq!(items(&source, "$.n"), [format!("[{}]", numbers.join(","))]);
        assert_eq!(
            items(&source, "$"),
            [format!("{{\"n\":[{}]}}", numbers.join(","))]
        );
        // A document that is a single number.
        assert_eq!(
            items(" 123456789012345678901234567890 ", "$"),
            ["123456789012345678901234567890"]
        );
    }

    #[test]
    fn documents_without_reprinted_numbers_skip_the_walk() {
        let doc = document(r#"{"a": [1, -2, 5.0, 0.1, 18446744073709551615], "s": "1E16 \" -0"}"#);
        assert!(doc.number_text.is_empty());
        for reprinted in ["1E2", "1.50", "-0", "1e16", "100000000000000000000"] {
            assert!(!prints_as_written(reprinted), "{reprinted}");
            assert!(has_reprinted_number(&format!("[{reprinted}]")));
        }
        assert!(!has_reprinted_number(r#"{"s": "1E2", "t": "\"1.50\\"}"#));
    }

    /// serde_json keeps the last member of a repeated key at the place of the
    /// first; numbers of the replaced members must not leak into the items.
    #[test]
    fn repeated_keys_keep_the_text_of_the_kept_member() {
        assert_eq!(
            items(r#"{"a":1E2,"b":[1.50],"a":2E2}"#, "$"),
            [r#"{"a":2E2,"b":[1.50]}"#]
        );
        assert_eq!(items(r#"{"a":[1E1,2E1],"a":[5,6]}"#, "$.a"), ["[5,6]"]);
        assert_eq!(items(r#"{"a":[1E1],"a":[2.50]}"#, "$.a"), ["[2.50]"]);
        assert_eq!(
            items(r#"{"a":[1E2,3E2],"a":{"x":1.50}}"#, "$"),
            [r#"{"a":{"x":1.50}}"#]
        );
        assert_eq!(
            items(
                r#"{"a":{"x":1E1,"x":2E1},"a":7E0,"b":{"y":1E3,"y":2}}"#,
                "$"
            ),
            [r#"{"a":7E0,"b":{"y":2}}"#]
        );
        // Keys compare decoded: "a" + backslash-u0062 is "ab".
        let escaped_ab = format!("a{}u0062", char::from(92));
        assert_eq!(
            items(
                &format!(r#"{{"ab":1E2,"{escaped_ab}":3E2,"c\"d":4E2}}"#),
                "$"
            ),
            [r#"{"ab":3E2,"c\"d":4E2}"#]
        );
    }

    /// With reprinted numbers in the document, subtrees are written by
    /// `JsonDocument::write`; everything but numbers matches serde_json.
    #[test]
    fn exact_items_match_serde_json_apart_from_numbers() {
        let sub = r#"{"s":"q\"\\ \n\u0001 é 😀 </","e":[],"o":{},"t":true,"f":false,"z":null,"n":[1,-2,0.5],"k\n":"v"}"#;
        let source = format!(r#"{{"x": 1E2, "sub": {sub}}}"#);
        let doc = document(&source);
        assert_eq!(doc.number_text.len(), 1);
        let node = &doc.value()["sub"];
        assert_eq!(doc.item_text(node), serde_json::to_string(node).unwrap());
        assert_eq!(doc.item_text(&node["s"]), "q\"\\ \n\u{1} é 😀 </");
        assert_eq!(doc.item_text(&node["z"]), "<null>");
        assert_eq!(items(&source, "$.x"), ["1E2"]);
    }

    #[test]
    fn filters_still_compare_numeric_values() {
        let source = r#"[{"n":1.50,"id":"a"},{"n":0.5E0,"id":"b"},{"n":1E1,"id":"c"}]"#;
        assert_eq!(items(source, "$[?(@.n > 1)].n"), ["1.50", "1E1"]);
        assert_eq!(items(source, "$[?(@.n == 0.5)].id"), ["b"]);
    }

    #[test]
    fn errors_are_serde_json_errors() {
        for (source, message) in [
            (r#"{"n": 1e400}"#, "number out of range"),
            (r#"{"n": 1E2"#, "EOF while parsing an object"),
            (r#"{"n": 01}"#, "invalid number"),
            (r#"{"n": "\ud800"}"#, "hex escape"),
        ] {
            let error = JsonAdapter.parse(source).unwrap_err();
            assert!(error.starts_with("invalid JSON: "), "{error}");
            assert!(error.contains(message), "{source}: {error}");
        }
    }
}
