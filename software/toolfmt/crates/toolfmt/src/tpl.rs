//! template: any_txt_parser rules run in memory on the payload.

use crate::ops::Effect;
use crate::render::{Ctx, RResult, Rendered};
use any_txt_parser::collect::{FileStatus, RuleStatus, process_file};
use any_txt_parser::discovery::DiscoveredFile;
use any_txt_parser::template::Format;
use std::borrow::Cow;
use std::fmt::Write as _;
use std::path::PathBuf;

pub fn render(c: &Ctx) -> RResult {
    let base = c.std.template.as_ref().ok_or("template has no rules")?;
    let filtered;
    let tpl = match c.keys {
        Some(keys) if !keys.is_empty() => {
            let mut t = base.clone();
            t.rules
                .retain(|r| keys.iter().any(|k| *k == r.id || *k == r.output_key));
            if t.rules.is_empty() {
                return Err(format!("keys [{}] match no rule", keys.join(",")));
            }
            filtered = t;
            &filtered
        }
        _ => base,
    };
    let text: Cow<str> = match c.json {
        Some(crate::json::J::Str(_)) => Cow::Borrowed(c.text),
        Some(_) if c.raw.trim_start().starts_with(['{', '[']) => Cow::Borrowed(c.raw),
        Some(j) => Cow::Owned(j.to_json()),
        None => Cow::Borrowed(c.text),
    };
    let format = match &tpl.format {
        Some(f) => f.clone(),
        None if c.json.is_some_and(|j| !matches!(j, crate::json::J::Str(_))) => Format::Json,
        None => Format::Text,
    };
    let file = DiscoveredFile {
        path: PathBuf::from(format!("{}.{}", c.server, c.tool)),
        size: text.len() as u64,
        mtime_iso: String::new(),
        hash: None,
    };
    let fr = process_file(&file, tpl, &text, "utf-8", format, "toolfmt".into(), None);
    if fr.status == FileStatus::Error {
        return Err(fr.error.unwrap_or_else(|| "template failed".into()));
    }
    let mut eff = Effect::default();
    let mut out = String::new();
    let mut matched = 0usize;
    for rr in &fr.rule_results {
        let line_start = out.is_empty();
        if !line_start {
            out.push('\n');
        }
        match rr.status {
            RuleStatus::Matched => {
                matched += 1;
                if rr.items.len() == 1 && !rr.items[0].contains('\n') {
                    let _ = write!(out, "{}: {}", rr.key, rr.items[0]);
                } else {
                    let _ = write!(out, "{}[{}]:", rr.key, rr.items.len());
                    for it in &rr.items {
                        let _ = write!(out, "\n- {}", it.replace('\n', "\n  "));
                    }
                }
                let omitted = rr.omitted_items + rr.omitted_by_resource;
                if omitted > 0 {
                    eff.lose(format!("{}: {omitted} more items", rr.key));
                }
                if rr.truncated_items > 0 || !rr.transform_losses.is_empty() {
                    eff.lossy = true;
                }
            }
            RuleStatus::NoMatch => {
                let _ = write!(out, "{}: (no match)", rr.key);
            }
            RuleStatus::Error => {
                let _ = write!(
                    out,
                    "{}: (error: {})",
                    rr.key,
                    rr.error.as_deref().unwrap_or("unknown")
                );
            }
            RuleStatus::NotApplicable => {
                if !line_start {
                    out.pop();
                }
            }
        }
    }
    if matched == 0 {
        return Err("no rule matched".into());
    }
    // A template extracts by design: what it leaves out is not shown.
    eff.lossy = true;
    Ok(Rendered { text: out, eff })
}

#[cfg(test)]
mod tests {
    use crate::json::J;
    use crate::pipeline::{Data, Overrides, Request, run};
    use crate::standard::compile;

    /// The parser keeps the source text of JSON numbers (I11), so a big
    /// integer is no longer rounded by a template rule.
    #[test]
    fn template_items_keep_number_text() {
        let standard = compile(
            concat!(
                "version: 3\nrender: template\ninput: {format: json}\nrules:\n",
                "  - id: n\n    select: {type: jsonpath, expression: \"$.n\"}\n",
                "  - id: rest\n    select: {type: jsonpath, expression: \"$.rest\"}\n",
            ),
            "t",
            "t",
        )
        .unwrap();
        let raw = r#"{"n": 123456789012345678901234567890, "rest": [5.0, 1e16, -0]}"#;
        let args = J::Obj(Vec::new());
        let request = Request {
            server: "s",
            tool: "t",
            args: &args,
            data: Data::Text(raw),
            keys: None,
            ov: Overrides::default(),
            auto: false,
            family: None,
        };
        let text = run(None, &request, Some(&standard)).agent_text(raw);
        assert!(
            text.ends_with("\nn: 123456789012345678901234567890\nrest: [5.0,1e16,-0]"),
            "{text}"
        );
        assert!(!text.contains("rounded"), "{text}");
    }
}
