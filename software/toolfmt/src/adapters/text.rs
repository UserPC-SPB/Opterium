use super::{Adapter, RawMatch, SourceLines, SourceTree};
use crate::template::{CompiledRule, Format, SelectorCompiled};

pub struct TextAdapter;

/// Indexes of the first and last line of the context window around `idx`.
fn context_window(lines: &[&str], idx: usize, ctx: &crate::template::Context) -> (usize, usize) {
    let before_max = ctx.before.min(idx);
    let after_max = ctx.after.min(lines.len().saturating_sub(idx + 1));
    (idx - before_max, idx + after_max)
}

impl Adapter for TextAdapter {
    fn format(&self) -> Format {
        Format::Text
    }

    fn parse<'a>(&self, text: &'a str) -> Result<SourceTree<'a>, String> {
        Ok(SourceTree::Text(text))
    }

    fn selector_types(&self) -> &'static [&'static str] {
        &["regex", "contains", "starts_with", "ends_with", "line"]
    }

    fn extract(&self, tree: &SourceTree<'_>, rule: &CompiledRule) -> Result<Vec<RawMatch>, String> {
        let text = match tree {
            SourceTree::Text(t) => t,
            _ => return Err("text adapter received non-text tree".to_string()),
        };
        let lines: Vec<&str> = text.lines().collect();
        let raw: Vec<usize> = match &rule.selector {
            SelectorCompiled::Regex(re) => lines
                .iter()
                .enumerate()
                .filter(|(_, l)| re.is_match(l))
                .map(|(i, _)| i)
                .collect(),
            SelectorCompiled::Contains(needle) => lines
                .iter()
                .enumerate()
                .filter(|(_, l)| l.contains(needle.as_str()))
                .map(|(i, _)| i)
                .collect(),
            SelectorCompiled::StartsWith(prefix) => lines
                .iter()
                .enumerate()
                .filter(|(_, l)| l.starts_with(prefix.as_str()))
                .map(|(i, _)| i)
                .collect(),
            SelectorCompiled::EndsWith(suffix) => lines
                .iter()
                .enumerate()
                .filter(|(_, l)| l.ends_with(suffix.as_str()))
                .map(|(i, _)| i)
                .collect(),
            SelectorCompiled::LineRange { from, to } => {
                let from = (*from as usize).saturating_sub(1).min(lines.len());
                let to = (*to as usize).min(lines.len());
                if from >= to { Vec::new() } else { vec![from] }
            }
            _ => return Err(
                "text adapter requires a regex, contains, starts_with, ends_with or line selector"
                    .to_string(),
            ),
        };

        let mut out = Vec::new();
        match &rule.selector {
            SelectorCompiled::Regex(re) => {
                let single_capture = re.captures_len() == 2;
                for idx in raw {
                    let line = lines[idx];
                    let value = if single_capture {
                        re.captures(line)
                            .and_then(|c| c.get(1))
                            .map(|m| m.as_str().to_string())
                            .unwrap_or_else(|| line.to_string())
                    } else {
                        line.to_string()
                    };
                    push_with_context(&mut out, &lines, idx, rule, value);
                }
            }
            _ => {
                for idx in raw {
                    if let SelectorCompiled::LineRange { from, to } = &rule.selector {
                        let from = (*from as usize).saturating_sub(1).min(lines.len());
                        let to = (*to as usize).min(lines.len());
                        if from < to {
                            out.push(RawMatch {
                                value: lines[from..to].join("\n"),
                                lines: Some(SourceLines {
                                    line: from + 1,
                                    first: from + 1,
                                    last: to,
                                }),
                            });
                        }
                    } else {
                        push_with_context(&mut out, &lines, idx, rule, lines[idx].to_string());
                    }
                }
            }
        }
        Ok(out)
    }
}

fn push_with_context(
    out: &mut Vec<RawMatch>,
    lines: &[&str],
    idx: usize,
    rule: &CompiledRule,
    value: String,
) {
    if rule.context.before == 0 && rule.context.after == 0 {
        out.push(RawMatch {
            value,
            lines: Some(SourceLines {
                line: idx + 1,
                first: idx + 1,
                last: idx + 1,
            }),
        });
    } else {
        let (first, last) = context_window(lines, idx, &rule.context);
        out.push(RawMatch {
            value: lines[first..=last].join("\n"),
            lines: Some(SourceLines {
                line: idx + 1,
                first: first + 1,
                last: last + 1,
            }),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule_yaml(sel_yaml: &str) -> CompiledRule {
        let t = crate::template::compile_yaml_str(
            &format!(
                r#"
version: 1
name: t
input: {{format: text}}
rules:
  - id: r
    select:
{}
"#,
                sel_yaml
            ),
            &std::path::PathBuf::from("t.yml"),
            None,
            None,
        )
        .unwrap();
        t.rules[0].clone()
    }

    #[test]
    fn regex_lines() {
        let a = TextAdapter;
        let tree = a.parse("ok\nERROR: boom\nerror: meh\nfine\n").unwrap();
        let r = rule_yaml("      type: regex\n      expression: \"(?i)error\"");
        let out = a.extract(&tree, &r).unwrap();
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].value, "ERROR: boom");
        let at = |line| {
            Some(SourceLines {
                line,
                first: line,
                last: line,
            })
        };
        assert_eq!((out[0].lines, out[1].lines), (at(2), at(3)));
    }

    #[test]
    fn context_window() {
        let a = TextAdapter;
        let tree = a.parse("l1\nl2\nERROR\nl4\nl5\n").unwrap();
        let mut r = rule_yaml("      type: contains\n      expression: \"ERROR\"");
        r.context = crate::template::Context {
            before: 1,
            after: 1,
        };
        let out = a.extract(&tree, &r).unwrap();
        assert_eq!(out[0].value, "l2\nERROR\nl4");
        let window = |line, first, last| Some(SourceLines { line, first, last });
        assert_eq!(out[0].lines, window(3, 2, 4));
        // The window stops at the first and the last line.
        r.context = crate::template::Context {
            before: 5,
            after: 5,
        };
        let out = a.extract(&tree, &r).unwrap();
        assert_eq!(out[0].lines, window(3, 1, 5));
    }

    #[test]
    fn line_range() {
        let a = TextAdapter;
        let tree = a.parse("a\nb\nc\nd\ne\n").unwrap();
        let r = rule_yaml("      type: line\n      from: 2\n      to: 4");
        let out = a.extract(&tree, &r).unwrap();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].value, "b\nc\nd");
        assert_eq!(
            out[0].lines,
            Some(SourceLines {
                line: 2,
                first: 2,
                last: 4
            })
        );
    }
}
