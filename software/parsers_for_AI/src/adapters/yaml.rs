use super::{Adapter, RawMatch, SourceTree};
use crate::template::{CompiledRule, Format};
use serde::Deserialize;
use serde_json::Value;
use serde_yaml::{Mapping, Value as YamlValue};

pub struct YamlAdapter;

/// Loads every document of a YAML stream (`---` separated) in order, with
/// merge keys expanded, as JSON-like trees. Empty documents are kept here;
/// the caller decides whether to skip them. Each document is deserialized on
/// its own, so serde_yaml's alias repetition and recursion limits apply per
/// document.
fn load_documents(text: &str) -> Result<Vec<Value>, String> {
    let mut documents = Vec::new();
    // The iterator yields its error forever once loading fails, so stop at
    // the first error.
    for (index, document) in serde_yaml::Deserializer::from_str(text).enumerate() {
        let number = index + 1;
        let mut value =
            deserialize_value(document).map_err(|e| yaml_error(number, &e.to_string()))?;
        resolve_merge_keys(&mut value);
        let converted =
            yaml_to_json(&value).ok_or_else(|| "cannot convert YAML document".to_string())?;
        documents.push(converted);
    }
    Ok(documents)
}

/// `serde_yaml::Value::deserialize`, except for integers outside the 64-bit
/// range. serde_yaml passes those up to 128 bits on as `u128` / `i128`,
/// which `Value` rejects, so one such number made the whole file invalid,
/// while longer ones already became floating-point numbers. All of them are
/// floating-point numbers now, as big integers are in JSON sources.
pub(crate) fn deserialize_value<'de, D>(deserializer: D) -> Result<YamlValue, D::Error>
where
    D: serde::Deserializer<'de>,
{
    LenientValue::deserialize(deserializer).map(|value| value.0)
}

struct LenientValue(YamlValue);

impl<'de> Deserialize<'de> for LenientValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer
            .deserialize_any(LenientVisitor)
            .map(LenientValue)
    }
}

/// serde_yaml's own `Value` visitor plus `visit_i128` / `visit_u128`.
struct LenientVisitor;

impl<'de> serde::de::Visitor<'de> for LenientVisitor {
    type Value = YamlValue;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("any YAML value")
    }

    fn visit_bool<E>(self, b: bool) -> Result<YamlValue, E> {
        Ok(YamlValue::Bool(b))
    }

    fn visit_i64<E>(self, i: i64) -> Result<YamlValue, E> {
        Ok(YamlValue::Number(i.into()))
    }

    fn visit_u64<E>(self, u: u64) -> Result<YamlValue, E> {
        Ok(YamlValue::Number(u.into()))
    }

    fn visit_i128<E>(self, i: i128) -> Result<YamlValue, E> {
        Ok(YamlValue::Number((i as f64).into()))
    }

    fn visit_u128<E>(self, u: u128) -> Result<YamlValue, E> {
        Ok(YamlValue::Number((u as f64).into()))
    }

    fn visit_f64<E>(self, f: f64) -> Result<YamlValue, E> {
        Ok(YamlValue::Number(f.into()))
    }

    fn visit_str<E>(self, s: &str) -> Result<YamlValue, E> {
        Ok(YamlValue::String(s.to_owned()))
    }

    fn visit_string<E>(self, s: String) -> Result<YamlValue, E> {
        Ok(YamlValue::String(s))
    }

    fn visit_unit<E>(self) -> Result<YamlValue, E> {
        Ok(YamlValue::Null)
    }

    fn visit_none<E>(self) -> Result<YamlValue, E> {
        Ok(YamlValue::Null)
    }

    fn visit_some<D>(self, deserializer: D) -> Result<YamlValue, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserialize_value(deserializer)
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<YamlValue, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        let mut items = Vec::new();
        while let Some(LenientValue(item)) = seq.next_element()? {
            items.push(item);
        }
        Ok(YamlValue::Sequence(items))
    }

    fn visit_map<A>(self, mut map: A) -> Result<YamlValue, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        use serde::de::Error;
        let mut mapping = Mapping::new();
        while let Some(LenientValue(key)) = map.next_key()? {
            match mapping.entry(key) {
                serde_yaml::mapping::Entry::Occupied(entry) => {
                    // serde_yaml's message for a repeated key.
                    let message = match entry.key() {
                        YamlValue::Null => "duplicate entry with null key".to_string(),
                        YamlValue::Bool(b) => format!("duplicate entry with key `{}`", b),
                        YamlValue::Number(n) => format!("duplicate entry with key {}", n),
                        YamlValue::String(s) => format!("duplicate entry with key {:?}", s),
                        _ => "duplicate entry in YAML map".to_string(),
                    };
                    return Err(A::Error::custom(message));
                }
                serde_yaml::mapping::Entry::Vacant(entry) => {
                    let LenientValue(value) = map.next_value()?;
                    entry.insert(value);
                }
            }
        }
        Ok(YamlValue::Mapping(mapping))
    }

    fn visit_enum<A>(self, data: A) -> Result<YamlValue, A::Error>
    where
        A: serde::de::EnumAccess<'de>,
    {
        use serde::de::{Error, VariantAccess};
        let (tag, contents): (String, _) = data.variant()?;
        if tag.is_empty() {
            return Err(A::Error::custom("empty YAML tag is not allowed"));
        }
        let LenientValue(value) = contents.newtype_variant()?;
        Ok(YamlValue::Tagged(Box::new(
            serde_yaml::value::TaggedValue {
                tag: serde_yaml::value::Tag::new(tag),
                value,
            },
        )))
    }
}

/// The first document keeps the single-document message, so errors of
/// ordinary YAML files read exactly as before.
fn yaml_error(document: usize, message: &str) -> String {
    if document == 1 {
        format!("invalid YAML: {}", message)
    } else {
        format!("invalid YAML in document {}: {}", document, message)
    }
}

/// Expands YAML merge keys (`<<: *base`, `<<: [*a, *b]`) in place. Own keys
/// override merged ones, earlier sources override later ones, and a source
/// that itself merges (`c <- b <- a`) is expanded first. Merged keys take
/// the position of the `<<` entry. Unlike `serde_yaml::Value::apply_merge`,
/// this resolves such chains and keeps key order; like it, values are moved,
/// not copied, so the tree never grows beyond what alias expansion (bounded
/// by serde_yaml's repetition limit) already produced.
///
/// A `<<` whose value is not a mapping or a list of mappings (`<<: 5`, or
/// data such as `"<<": shift-left`: serde_yaml does not keep quotes) stays
/// an ordinary key, as it was before merge keys were expanded.
fn resolve_merge_keys(value: &mut YamlValue) {
    match value {
        YamlValue::Mapping(mapping) => {
            for child in mapping.values_mut() {
                resolve_merge_keys(child);
            }
            if mapping.get("<<").is_some_and(is_merge_value) {
                let entries = std::mem::take(mapping);
                *mapping = merge_entries(entries);
            }
        }
        YamlValue::Sequence(items) => items.iter_mut().for_each(resolve_merge_keys),
        YamlValue::Tagged(tagged) => resolve_merge_keys(&mut tagged.value),
        _ => {}
    }
}

/// A mapping or a list of mappings (an empty list merges nothing).
fn is_merge_value(value: &YamlValue) -> bool {
    match untag_ref(value) {
        YamlValue::Mapping(_) => true,
        YamlValue::Sequence(items) => items.iter().all(|item| untag_ref(item).is_mapping()),
        _ => false,
    }
}

/// Called only when the `<<` value passed `is_merge_value`.
fn merge_entries(entries: Mapping) -> Mapping {
    let mut merged = Mapping::with_capacity(entries.len());
    for (key, value) in entries {
        if matches!(&key, YamlValue::String(s) if s == "<<") {
            for source in merge_sources(value) {
                for (k, v) in source {
                    merged.entry(k).or_insert(v);
                }
            }
        } else {
            // Replaces a merged value but keeps its position.
            merged.insert(key, value);
        }
    }
    merged
}

fn merge_sources(value: YamlValue) -> Vec<Mapping> {
    match untag(value) {
        YamlValue::Mapping(source) => vec![source],
        YamlValue::Sequence(items) => items
            .into_iter()
            .filter_map(|item| match untag(item) {
                YamlValue::Mapping(source) => Some(source),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

fn untag(mut value: YamlValue) -> YamlValue {
    while let YamlValue::Tagged(tagged) = value {
        value = tagged.value;
    }
    value
}

fn untag_ref(mut value: &YamlValue) -> &YamlValue {
    while let YamlValue::Tagged(tagged) = value {
        value = &tagged.value;
    }
    value
}

fn yaml_to_json(v: &serde_yaml::Value) -> Option<Value> {
    Some(match v {
        serde_yaml::Value::Null => Value::Null,
        serde_yaml::Value::Bool(b) => Value::Bool(*b),
        serde_yaml::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Value::Number(i.into())
            } else if let Some(u) = n.as_u64() {
                Value::Number(u.into())
            } else if let Some(f) = n.as_f64() {
                serde_json::Number::from_f64(f)
                    .map(Value::Number)
                    .unwrap_or(Value::Null)
            } else {
                Value::Null
            }
        }
        serde_yaml::Value::String(s) => Value::String(s.clone()),
        serde_yaml::Value::Sequence(seq) => {
            Value::Array(seq.iter().filter_map(yaml_to_json).collect())
        }
        serde_yaml::Value::Mapping(map) => {
            let mut obj = serde_json::Map::new();
            for (k, val) in map {
                let key = match k {
                    serde_yaml::Value::String(s) => s.clone(),
                    other => yaml_key_string(other),
                };
                if let Some(jv) = yaml_to_json(val) {
                    obj.insert(key, jv);
                }
            }
            Value::Object(obj)
        }
        serde_yaml::Value::Tagged(t) => yaml_to_json(&t.value)?,
    })
}

fn yaml_key_string(other: &serde_yaml::Value) -> String {
    serde_yaml::to_string(other)
        .map(|s| s.trim().to_string())
        .unwrap_or_default()
}

impl Adapter for YamlAdapter {
    fn format(&self) -> Format {
        Format::Yaml
    }

    /// A single-document stream (including an empty file) gives the same
    /// tree as before multi-document support. In a stream of several
    /// documents, empty (null) documents are skipped.
    fn parse<'a>(&self, text: &'a str) -> Result<SourceTree<'a>, String> {
        let mut documents = load_documents(text)?;
        if documents.len() == 1 {
            return Ok(SourceTree::Json(documents.remove(0)));
        }
        documents.retain(|document| !document.is_null());
        Ok(SourceTree::JsonDocuments(documents))
    }

    fn selector_types(&self) -> &'static [&'static str] {
        &["path"]
    }

    /// Runs the rule on every document and concatenates the matches in
    /// document order.
    fn extract(&self, tree: &SourceTree<'_>, rule: &CompiledRule) -> Result<Vec<RawMatch>, String> {
        let documents = match tree {
            SourceTree::Json(v) => std::slice::from_ref(v),
            SourceTree::JsonDocuments(documents) => documents.as_slice(),
            _ => return Err("yaml adapter received non-json tree".to_string()),
        };
        let mut raw = Vec::new();
        for document in documents {
            let matches = super::match_value(&rule.selector, document)?;
            raw.extend(matches.into_iter().map(|m| RawMatch {
                value: crate::adapters::json::json_value_to_string(m),
                lines: None,
            }));
        }
        Ok(raw)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn yaml_path() {
        let a = YamlAdapter;
        let tree = a
            .parse("root:\n  items:\n    - name: a\n    - name: b\n")
            .unwrap();
        let t = crate::template::compile_yaml_str(
            r#"
version: 1
name: t
input: {format: yaml}
rules:
  - id: r
    select: {type: path, expression: "root.items[*].name"}
"#,
            &std::path::PathBuf::from("t.yml"),
            None,
            None,
        )
        .unwrap();
        let matches = a.extract(&tree, &t.rules[0]).unwrap();
        assert_eq!(
            matches.iter().map(|m| m.value.as_str()).collect::<Vec<_>>(),
            vec!["a", "b"]
        );
    }

    fn path_values(text: &str, expression: &str) -> Result<Vec<String>, String> {
        let template = crate::template::compile_yaml_str(
            &format!(
                "version: 1\nname: t\ninput: {{format: yaml}}\nrules:\n  - id: r\n    select: {{type: path, expression: \"{}\"}}\n",
                expression
            ),
            &std::path::PathBuf::from("t.yml"),
            None,
            None,
        )
        .unwrap();
        let tree = YamlAdapter.parse(text)?;
        let matches = YamlAdapter.extract(&tree, &template.rules[0])?;
        Ok(matches.into_iter().map(|m| m.value).collect())
    }

    fn single_tree(text: &str) -> Value {
        match YamlAdapter.parse(text).unwrap() {
            SourceTree::Json(value) => value,
            other => panic!("{text:?} gave {other:?}"),
        }
    }

    #[test]
    fn single_document_streams_keep_their_tree() {
        for text in ["", "# only a comment\n", "---\n", "--- ~\n"] {
            assert_eq!(single_tree(text), Value::Null, "{text:?}");
        }
        assert_eq!(path_values("", "$").unwrap(), vec!["<null>"]);
        assert_eq!(single_tree("---\na: 1\n...\n"), serde_json::json!({"a": 1}));
        // Parse errors of ordinary files read as before.
        let error = YamlAdapter.parse("a: [1\n").unwrap_err();
        assert!(error.starts_with("invalid YAML: "), "{error}");
    }

    #[test]
    fn every_document_of_a_stream_is_searched_in_order() {
        let stream = "# manifest\n---\nkind: Deployment\n---\n---\nkind: Service\n--- ~\n---\nkind: ConfigMap\n...\n";
        assert_eq!(
            path_values(stream, "$.kind").unwrap(),
            ["Deployment", "Service", "ConfigMap"]
        );
        // Empty documents are skipped, not reported as `<null>`.
        assert_eq!(
            path_values(stream, "$").unwrap(),
            [
                r#"{"kind":"Deployment"}"#,
                r#"{"kind":"Service"}"#,
                r#"{"kind":"ConfigMap"}"#
            ]
        );
        assert_eq!(path_values("a: 1\n---\n", "a").unwrap(), ["1"]);
        assert!(path_values("---\n---\n", "$").unwrap().is_empty());
        let SourceTree::JsonDocuments(documents) = YamlAdapter.parse("---\n---\n").unwrap() else {
            panic!("two documents expected");
        };
        assert!(documents.is_empty());
    }

    #[test]
    fn errors_name_the_document_after_the_first() {
        let error = YamlAdapter.parse("a: [1\n---\nb: 2\n").unwrap_err();
        assert!(error.starts_with("invalid YAML: "), "{error}");
        let error = YamlAdapter
            .parse("a: 1\n---\nb: 2\n---\nc: [\n")
            .unwrap_err();
        assert!(error.starts_with("invalid YAML in document 3: "), "{error}");
        // Anchors belong to their own document.
        let error = YamlAdapter.parse("a: &x 1\n---\nb: *x\n").unwrap_err();
        assert!(
            error.starts_with("invalid YAML in document 2: unknown anchor"),
            "{error}"
        );
    }

    #[test]
    fn merge_keys_follow_yaml_precedence_and_keep_key_order() {
        let text = "\
defaults: &defaults
  adapter: postgres
  host: localhost
  pool: 5
tuned: &tuned
  <<: *defaults
  pool: 20
development:
  <<: *defaults
  host: dev.local
  database: dev
production:
  database: prod
  <<: [*tuned, {host: prod.local, ssl: true}]
nested:
  - name: item
    <<: {name: ignored, extra: 1}
";
        let tree = single_tree(text);
        let json = |key: &str| serde_json::to_string(&tree[key]).unwrap();
        assert_eq!(
            json("tuned"),
            r#"{"adapter":"postgres","host":"localhost","pool":20}"#
        );
        // An own key after `<<` overrides the merged value in its place.
        assert_eq!(
            json("development"),
            r#"{"adapter":"postgres","host":"dev.local","pool":5,"database":"dev"}"#
        );
        // `tuned` brings what it merged itself; earlier sources win.
        assert_eq!(
            json("production"),
            r#"{"database":"prod","adapter":"postgres","host":"localhost","pool":20,"ssl":true}"#
        );
        assert_eq!(json("nested"), r#"[{"name":"item","extra":1}]"#);
        assert!(!serde_json::to_string(&tree).unwrap().contains("<<"));
        assert_eq!(
            path_values(text, "production.adapter").unwrap(),
            ["postgres"]
        );
    }

    #[test]
    fn merge_keys_that_cannot_merge_stay_ordinary_keys() {
        // Not a mapping or a list of mappings: the key stays as it was before
        // merge keys were expanded, including data keys written `"<<"`.
        for (text, expected) in [
            ("svc:\n  web:\n    <<: 5\n", r#"{"svc":{"web":{"<<":5}}}"#),
            ("<<: ~\n", r#"{"<<":null}"#),
            (
                "ops:\n  \"<<\": shift-left\n  \">>\": shift-right\n",
                r#"{"ops":{"<<":"shift-left",">>":"shift-right"}}"#,
            ),
            (
                "list:\n  - a: 1\n    <<: [{x: 1}, [y]]\n",
                r#"{"list":[{"a":1,"<<":[{"x":1},["y"]]}]}"#,
            ),
            (
                "base: &b 7\nkey:\n  <<: *b\n  own: 1\n",
                r#"{"base":7,"key":{"<<":7,"own":1}}"#,
            ),
            // A literal `<<` of a merged mapping is inherited as data.
            (
                "a: &a {<<: x, k: 1}\nb: {<<: *a}\n",
                r#"{"a":{"<<":"x","k":1},"b":{"<<":"x","k":1}}"#,
            ),
            // No message carries a key with line breaks into Diagnostics.
            (
                "\"a\\n## Injected\\nx\": {\"<<\": 5}\n",
                r#"{"a\n## Injected\nx":{"<<":5}}"#,
            ),
        ] {
            let tree = serde_json::to_string(&single_tree(text)).unwrap();
            assert_eq!(tree, expected, "{text:?}");
        }
        assert_eq!(
            path_values("a: 1\n---\nm: {<<: [1]}\n", "$.m").unwrap(),
            [r#"{"<<":[1]}"#]
        );
    }

    #[test]
    fn alias_expansion_stays_bounded_with_merge_keys() {
        let bomb = |merge: bool| {
            let mut text = String::from("l0: &l0 {k: v}\n");
            for level in 1..=12 {
                let refs = vec![format!("*l{}", level - 1); 10].join(", ");
                if merge {
                    text.push_str(&format!(
                        "l{level}: &l{level} {{<<: [{refs}], k{level}: x}}\n"
                    ));
                } else {
                    text.push_str(&format!("l{level}: &l{level} [{refs}]\n"));
                }
            }
            text
        };
        for merge in [false, true] {
            let error = YamlAdapter.parse(&bomb(merge)).unwrap_err();
            assert!(error.contains("repetition limit exceeded"), "{error}");
            let error = YamlAdapter
                .parse(&format!("a: 1\n---\n{}", bomb(merge)))
                .unwrap_err();
            assert!(
                error.starts_with("invalid YAML in document 2: repetition limit exceeded"),
                "{error}"
            );
        }
    }
}
