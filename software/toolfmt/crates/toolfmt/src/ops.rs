//! PRE operations (structural, on typed items) and POST operations (on the
//! rendered text), plus value formats. Every removal is declared (I3).

use crate::json::{J, cmp};
use crate::standard::{PostOp, PreOp, ValueFormat};
use std::borrow::Cow;
use std::collections::HashSet;

#[derive(Default, Debug)]
pub struct Effect {
    pub lossy: bool,
    /// Declared omissions, appended after the output as `… <note>`.
    pub notes: Vec<String>,
    /// Paths of input fields the render wrote out exactly, value beside its
    /// key (`""` is the whole document). The pipeline trusts only these
    /// when it decides whether a pagination cursor is already visible.
    pub shown: Vec<String>,
}

impl Effect {
    /// Whether the field at `path` (`a.b`) was written out exactly, itself
    /// or as part of a parent written out whole.
    pub fn shows(&self, path: &str) -> bool {
        self.shown.iter().any(|p| {
            p.is_empty()
                || p == path
                || path
                    .strip_prefix(p.as_str())
                    .is_some_and(|t| t.starts_with('.'))
        })
    }

    pub fn lose(&mut self, note: impl Into<String>) {
        self.lossy = true;
        self.notes.push(note.into());
    }
}

/// Value of `key` in an item: a plain key first, then a dotted path.
pub fn field<'a>(item: &'a J, key: &str) -> Option<&'a J> {
    item.get(key).or_else(|| {
        if key.contains('.') || key.contains('[') {
            item.path(key)
        } else {
            None
        }
    })
}

pub fn apply_pre<'a>(ops: &[PreOp], items: &mut Vec<Cow<'a, J>>, eff: &mut Effect) {
    for op in ops {
        match op {
            PreOp::Unique => {
                let mut seen = HashSet::new();
                let before = items.len();
                items.retain(|x| seen.insert(x.to_json()));
                let gone = before - items.len();
                if gone > 0 {
                    eff.lose(format!("{gone} duplicate items removed"));
                }
            }
            PreOp::Sort { desc, by } => {
                let key = |x: &J| -> J {
                    match by {
                        Some(k) => field(x, k).cloned().unwrap_or(J::Null),
                        None => x.clone(),
                    }
                };
                if by.is_some() {
                    let mut keyed: Vec<(J, Cow<'a, J>)> =
                        items.drain(..).map(|x| (key(&x), x)).collect();
                    keyed.sort_by(|a, b| {
                        let o = cmp(&a.0, &b.0);
                        if *desc { o.reverse() } else { o }
                    });
                    items.extend(keyed.into_iter().map(|(_, x)| x));
                } else {
                    items.sort_by(|a, b| {
                        let o = cmp(a, b);
                        if *desc { o.reverse() } else { o }
                    });
                }
            }
            PreOp::Limit(n) => {
                if items.len() > *n {
                    let gone = items.len() - n;
                    items.truncate(*n);
                    eff.lose(format!("{gone} more items (limit_items {n})"));
                }
            }
            PreOp::Select(keys) => {
                let mut dropped = false;
                for x in items.iter_mut() {
                    if let J::Obj(m) = x.as_ref() {
                        let mut out = Vec::with_capacity(keys.len());
                        for k in keys {
                            if let Some(v) = field(x, k) {
                                out.push((k.clone(), v.clone()));
                            }
                        }
                        let covered = m.iter().all(|(k, _)| keys.iter().any(|s| s == k));
                        if !covered {
                            dropped = true;
                        }
                        *x = Cow::Owned(J::Obj(out));
                    }
                }
                if dropped {
                    eff.lossy = true;
                }
            }
        }
    }
}

pub fn apply_post(ops: &[PostOp], mut text: String, eff: &mut Effect) -> String {
    for op in ops {
        text = match op {
            PostOp::Join(sep) => text.lines().collect::<Vec<_>>().join(sep),
            PostOp::Replace { re, to, literal } => {
                if *literal {
                    re.replace_all(&text, regex::NoExpand(to)).into_owned()
                } else {
                    re.replace_all(&text, to.as_str()).into_owned()
                }
            }
            PostOp::TruncateChars(n) => {
                let mut cut = 0usize;
                let out: Vec<String> = text
                    .lines()
                    .map(|l| {
                        if l.chars().count() > *n {
                            cut += 1;
                            let mut s: String = l.chars().take(*n).collect();
                            s.push('…');
                            s
                        } else {
                            l.to_string()
                        }
                    })
                    .collect();
                if cut > 0 {
                    eff.lose(format!("{cut} lines cut to {n} chars"));
                }
                out.join("\n")
            }
            PostOp::TruncateLines(n) => {
                let total = text.lines().count();
                if total > *n {
                    eff.lose(format!("{} more lines", total - n));
                    text.lines().take(*n).collect::<Vec<_>>().join("\n")
                } else {
                    text
                }
            }
            PostOp::Mask(list) => {
                let mut t = text;
                for (re, rep) in list {
                    if re.is_match(&t) {
                        eff.lossy = true;
                        t = re.replace_all(&t, rep.as_str()).into_owned();
                    }
                }
                t
            }
            PostOp::DropLines(re) => {
                let before = text.lines().count();
                let out: Vec<&str> = text.lines().filter(|l| !re.is_match(l)).collect();
                let gone = before - out.len();
                if gone > 0 {
                    eff.lose(format!("{gone} lines dropped (drop_lines)"));
                }
                out.join("\n")
            }
            PostOp::KeepLines(re) => {
                let before = text.lines().count();
                let out: Vec<&str> = text.lines().filter(|l| re.is_match(l)).collect();
                let gone = before - out.len();
                if gone > 0 {
                    eff.lose(format!("{gone} lines dropped (keep_lines)"));
                }
                out.join("\n")
            }
        };
    }
    text
}

/// Formats a number; returns the text and whether the format rounded.
pub fn format_value(v: &J, f: ValueFormat) -> (String, bool) {
    let J::Num(n) = v else {
        return (v.text(), false);
    };
    match f {
        ValueFormat::Thousands => (thousands(n), false),
        ValueFormat::Bytes => bytes(n),
        ValueFormat::Fixed(d) => match n.parse::<f64>() {
            Ok(x) => {
                let s = format!("{x:.d$}");
                let exact = s.parse::<f64>().map(|y| y == x).unwrap_or(false);
                (s, !exact)
            }
            Err(_) => (n.clone(), false),
        },
    }
}

pub fn thousands(n: &str) -> String {
    if n.contains(['e', 'E']) {
        return n.to_string();
    }
    let (sign, rest) = n.strip_prefix('-').map_or(("", n), |r| ("-", r));
    let (int, frac) = match rest.find('.') {
        Some(i) => (&rest[..i], &rest[i..]),
        None => (rest, ""),
    };
    let mut out = String::with_capacity(n.len() + int.len() / 3);
    out.push_str(sign);
    for (i, c) in int.chars().enumerate() {
        if i > 0 && (int.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    out.push_str(frac);
    out
}

fn bytes(n: &str) -> (String, bool) {
    let Ok(x) = n.parse::<f64>() else {
        return (n.to_string(), false);
    };
    const UNITS: [&str; 6] = ["B", "KB", "MB", "GB", "TB", "PB"];
    let mut unit = 0;
    let mut v = x.abs();
    while v >= 1024.0 && unit < UNITS.len() - 1 {
        v /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        return (format!("{n} B"), false);
    }
    let shown = (v * 10.0).round() / 10.0;
    let lossy = shown * 1024f64.powi(unit as i32) != x.abs();
    let mut s = format!("{shown:.1}");
    if s.ends_with(".0") {
        s.truncate(s.len() - 2);
    }
    let sign = if x < 0.0 { "-" } else { "" };
    (format!("{sign}{s} {}", UNITS[unit]), lossy)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::standard::parse_post_list;

    fn items(s: &str) -> Vec<J> {
        J::parse(s).unwrap().as_arr().unwrap().clone()
    }

    #[test]
    fn pre_chain() {
        let src = items(r#"[{"n":3,"x":1},{"n":1,"x":2},{"n":3,"x":1},{"n":2,"x":3}]"#);
        let mut list: Vec<Cow<J>> = src.iter().map(Cow::Borrowed).collect();
        let mut eff = Effect::default();
        apply_pre(
            &[
                PreOp::Unique,
                PreOp::Sort {
                    desc: true,
                    by: Some("n".into()),
                },
                PreOp::Limit(2),
                PreOp::Select(vec!["n".into()]),
            ],
            &mut list,
            &mut eff,
        );
        let out: Vec<String> = list.iter().map(|x| x.to_json()).collect();
        assert_eq!(out, vec![r#"{"n":3}"#, r#"{"n":2}"#]);
        assert!(eff.lossy);
        assert_eq!(eff.notes.len(), 2);
    }

    #[test]
    fn post_chain() {
        let ops = parse_post_list(
            &serde_yaml::from_str::<Vec<serde_yaml::Value>>(
                "[{drop_lines: '^#'}, {truncate_chars: 3}, {join: ' | '}]",
            )
            .unwrap(),
        )
        .unwrap();
        let mut eff = Effect::default();
        let out = apply_post(&ops, "# c\nabcdef\nab".into(), &mut eff);
        assert_eq!(out, "abc… | ab");
        assert!(eff.lossy);
    }

    #[test]
    fn value_formats() {
        assert_eq!(thousands("1234567"), "1,234,567");
        assert_eq!(thousands("-1234.5"), "-1,234.5");
        assert_eq!(thousands("123"), "123");
        assert_eq!(bytes("1536"), ("1.5 KB".into(), false));
        assert_eq!(bytes("1024"), ("1 KB".into(), false));
        assert_eq!(bytes("512"), ("512 B".into(), false));
        assert!(bytes("1500000").1);
    }

    #[test]
    fn masks_keep_the_key() {
        let ops = parse_post_list(
            &serde_yaml::from_str::<Vec<serde_yaml::Value>>("[mask: true]").unwrap(),
        )
        .unwrap();
        let mut eff = Effect::default();
        let out = apply_post(
            &ops,
            "password=hunter22 key sk-abcdefghijklmnopqrstu".into(),
            &mut eff,
        );
        assert_eq!(out, "password=*** key ***");
    }
}
