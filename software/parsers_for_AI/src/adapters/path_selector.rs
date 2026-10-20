use crate::template::PathSegment;
use serde_json::Value;

pub fn parse_path(expr: &str) -> Result<Vec<PathSegment>, String> {
    let mut segs = Vec::new();
    let mut chars: Vec<char> = expr.trim().chars().collect();
    if chars.is_empty() {
        return Err("path must not be empty".to_string());
    }
    if chars[0] == '$' {
        chars.remove(0);
        if chars.starts_with(&['.', '.']) {
            return Err(
                "recursive descent '..' is not supported by path: write the full path (with '*' or '[*]' steps), or use a jsonpath selector for JSON"
                    .to_string(),
            );
        }
        if chars.first() == Some(&'.') {
            chars.remove(0);
        }
    }
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        match c {
            '.' => {
                i += 1;
                let (name, next) = take_name(&chars, i);
                if name.is_empty() {
                    return Err(format!("empty key after '.' at position {}", i));
                }
                segs.push(PathSegment::Key(name));
                i = next;
            }
            '[' => {
                let close = chars[i + 1..].iter().position(|c| *c == ']');
                let close = close.map(|p| p + i + 1);
                match close {
                    None => return Err("unterminated '[' in path".to_string()),
                    Some(ci) => {
                        let inner: String = chars[i + 1..ci].iter().collect();
                        let inner = inner.trim();
                        if inner.is_empty() {
                            return Err("empty '[]' in path".to_string());
                        }
                        if let Some(q) = inner.strip_prefix('\'').and_then(|s| s.strip_suffix('\''))
                        {
                            segs.push(PathSegment::Key(q.to_string()));
                        } else if let Some(q) =
                            inner.strip_prefix('"').and_then(|s| s.strip_suffix('"'))
                        {
                            segs.push(PathSegment::Key(q.to_string()));
                        } else if inner == "*" {
                            segs.push(PathSegment::Key("*".to_string()));
                        } else if inner
                            .starts_with(|c: char| c.is_ascii_digit() || c == '-' || c == '+')
                        {
                            let idx: usize = inner
                                .parse()
                                .map_err(|_| format!("invalid array index '{}' in path", inner))?;
                            segs.push(PathSegment::Index(idx));
                        } else {
                            // A TOML table header: `[database].host`,
                            // `[servers.alpha].ip`. Keys that start with a
                            // digit need quotes: `["2024"]`.
                            for key in inner.split('.') {
                                let key = key.trim();
                                if key.is_empty() || !key.chars().all(is_bare_key_char) {
                                    return Err(format!(
                                        "invalid key '{}' in '[{}]': use an index, '*', a bare key or a quoted key",
                                        key, inner
                                    ));
                                }
                                segs.push(PathSegment::Key(key.to_string()));
                            }
                        }
                        i = ci + 1;
                    }
                }
            }
            other => {
                let (name, next) = take_name(&chars, i);
                if name.is_empty() {
                    return Err(format!("unexpected character '{}' in path", other));
                }
                segs.push(PathSegment::Key(name));
                i = next;
            }
        }
    }
    Ok(segs)
}

fn is_bare_key_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '-' || c > '\u{7f}'
}

fn take_name(chars: &[char], from: usize) -> (String, usize) {
    let mut name = String::new();
    let mut i = from;
    while i < chars.len() {
        let c = chars[i];
        if matches!(c, 'a'..='z' | 'A'..='Z' | '0'..='9' | '_' | '-' | ' ' | '*') || c > '\u{7f}' {
            name.push(c);
            i += 1;
        } else {
            break;
        }
    }
    (name, i)
}

pub fn walk<'a>(value: &'a Value, segs: &[PathSegment]) -> Result<Vec<&'a Value>, String> {
    fn rec<'a>(v: &'a Value, segs: &[PathSegment]) -> Vec<&'a Value> {
        if segs.is_empty() {
            return vec![v];
        }
        let seg = &segs[0];
        let rest = &segs[1..];
        match seg {
            PathSegment::Key(k) if k == "*" => match v {
                Value::Array(a) => a.iter().flat_map(|item| rec(item, rest)).collect(),
                Value::Object(m) => m.values().flat_map(|val| rec(val, rest)).collect(),
                _ => vec![],
            },
            PathSegment::Key(k) => match v {
                Value::Object(m) => match m.get(k) {
                    Some(nv) => rec(nv, rest),
                    None => vec![],
                },
                _ => vec![],
            },
            PathSegment::Index(i) => match v {
                Value::Array(a) => match a.get(*i) {
                    Some(nv) => rec(nv, rest),
                    None => vec![],
                },
                _ => vec![],
            },
        }
    }
    Ok(rec(value, segs))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn path_parse() {
        assert_eq!(
            parse_path("a.b[0].c").unwrap(),
            vec![
                PathSegment::Key("a".into()),
                PathSegment::Key("b".into()),
                PathSegment::Index(0),
                PathSegment::Key("c".into())
            ]
        );
        assert_eq!(
            parse_path("$['a'].b[1]").unwrap(),
            vec![
                PathSegment::Key("a".into()),
                PathSegment::Key("b".into()),
                PathSegment::Index(1)
            ]
        );
        assert!(parse_path("a[").is_err());
        assert!(parse_path("a[0][").is_err());
    }

    #[test]
    fn toml_table_header_paths_and_rejected_forms() {
        let keys = |names: &[&str]| -> Vec<PathSegment> {
            names
                .iter()
                .map(|n| PathSegment::Key(n.to_string()))
                .collect()
        };
        assert_eq!(
            parse_path("[database].host").unwrap(),
            keys(&["database", "host"])
        );
        assert_eq!(
            parse_path("[servers.alpha].ip").unwrap(),
            keys(&["servers", "alpha", "ip"])
        );
        assert_eq!(
            parse_path("[database].ports[*]").unwrap(),
            keys(&["database", "ports", "*"])
        );
        assert_eq!(parse_path("[\"2024\"].x").unwrap(), keys(&["2024", "x"]));
        for index in ["$.a[1]", "$.a[+1]", "a[ 1 ]"] {
            assert_eq!(
                parse_path(index).unwrap(),
                vec![PathSegment::Key("a".into()), PathSegment::Index(1)],
                "{index}"
            );
        }
        for bad in ["[-1]", "[1:2]", "[a b]", "[a..b]", "[a.]", "$..key"] {
            assert!(parse_path(bad).is_err(), "{bad} must be rejected");
        }
    }

    #[test]
    fn path_walk() {
        let v = json!({"a": {"b": [{"c": 1}, {"c": 2}]}});
        let segs = parse_path("a.b[*].c").unwrap();
        assert_eq!(walk(&v, &segs).unwrap(), vec![&json!(1), &json!(2)]);
        let segs = parse_path("a.b[1].c").unwrap();
        assert_eq!(walk(&v, &segs).unwrap(), vec![&json!(2)]);
        let segs = parse_path("a.b").unwrap();
        assert_eq!(walk(&v, &segs).unwrap().len(), 1);
        let segs = parse_path("missing").unwrap();
        assert_eq!(walk(&v, &segs).unwrap(), Vec::<&Value>::new());
    }
}
