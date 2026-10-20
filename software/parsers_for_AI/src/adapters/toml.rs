use super::{Adapter, RawMatch, SourceTree};
use crate::template::{CompiledRule, Format};
use serde_json::Value;

pub struct TomlAdapter;

fn toml_to_json(v: &toml::Value) -> Value {
    match v {
        toml::Value::String(s) => Value::String(s.clone()),
        toml::Value::Integer(i) => Value::Number((*i).into()),
        toml::Value::Float(f) => serde_json::Number::from_f64(*f)
            .map(Value::Number)
            .unwrap_or(Value::Null),
        toml::Value::Boolean(b) => Value::Bool(*b),
        toml::Value::Datetime(d) => Value::String(d.to_string()),
        toml::Value::Array(a) => Value::Array(a.iter().map(toml_to_json).collect()),
        toml::Value::Table(t) => {
            let mut obj = serde_json::Map::new();
            for (k, v) in t {
                obj.insert(k.clone(), toml_to_json(v));
            }
            Value::Object(obj)
        }
    }
}

impl Adapter for TomlAdapter {
    fn format(&self) -> Format {
        Format::Toml
    }

    fn parse<'a>(&self, text: &'a str) -> Result<SourceTree<'a>, String> {
        let t: toml::Value = text
            .parse()
            .map_err(|e: toml::de::Error| format!("invalid TOML: {}", e))?;
        Ok(SourceTree::Json(toml_to_json(&t)))
    }

    fn selector_types(&self) -> &'static [&'static str] {
        &["path"]
    }

    fn extract(&self, tree: &SourceTree<'_>, rule: &CompiledRule) -> Result<Vec<RawMatch>, String> {
        let doc = match tree {
            SourceTree::Json(v) => v,
            _ => return Err("toml adapter received non-json tree".to_string()),
        };
        let matches = super::match_value(&rule.selector, doc)?;
        Ok(matches
            .into_iter()
            .map(|m| RawMatch {
                value: crate::adapters::json::json_value_to_string(m),
                lines: None,
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toml_path() {
        let a = TomlAdapter;
        let tree = a.parse("title = \"x\"\n[server]\nport = 8080\n").unwrap();
        let t = crate::template::compile_yaml_str(
            r#"
version: 1
name: t
input: {format: toml}
rules:
  - id: r
    select: {type: path, expression: "server.port"}
"#,
            &std::path::PathBuf::from("t.yml"),
            None,
            None,
        )
        .unwrap();
        let matches = a.extract(&tree, &t.rules[0]).unwrap();
        assert_eq!(matches[0].value, "8080");
    }
}
