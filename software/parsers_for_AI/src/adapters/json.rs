use super::{Adapter, RawMatch, SourceTree};
use crate::template::{CompiledRule, Format};
use serde_json::Value;

pub struct JsonAdapter;

pub fn json_value_to_string(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => "<null>".to_string(),
        other => serde_json::to_string(other).unwrap_or_default(),
    }
}

impl Adapter for JsonAdapter {
    fn format(&self) -> Format {
        Format::Json
    }

    fn parse<'a>(&self, text: &'a str) -> Result<SourceTree<'a>, String> {
        let v: Value = serde_json::from_str(text).map_err(|e| format!("invalid JSON: {}", e))?;
        Ok(SourceTree::Json(v))
    }

    fn selector_types(&self) -> &'static [&'static str] {
        &["jsonpath", "path"]
    }

    fn extract(&self, tree: &SourceTree<'_>, rule: &CompiledRule) -> Result<Vec<RawMatch>, String> {
        let doc = match tree {
            SourceTree::Json(v) => v,
            _ => return Err("json adapter received non-json tree".to_string()),
        };
        let matches = super::match_value(&rule.selector, doc)?;
        Ok(matches
            .into_iter()
            .map(|m| RawMatch {
                value: json_value_to_string(m),
                lines: None,
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
