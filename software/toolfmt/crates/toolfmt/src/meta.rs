//! What the gateway tells the agent: initialize instructions, the two
//! meta-tools, the standard reference; and fixture replay shared with the CLI.

use crate::capture::{Fixture, Payload};
use crate::config::Config;
use crate::json::J;
use crate::pipeline::{self, Body, Data, Outcome, Overrides, Request};
use crate::standard::{Standard, Store};

pub const INSPECT: &str = "formatter_inspect";
pub const SAVE: &str = "formatter_save_standard";

pub const INSTRUCTIONS: &str = "toolfmt gateway. If a `catalog` tool is listed, the servers' tools are behind it: browse or search with catalog, read a tool's card (catalog tools=[name]), then call it with call_tool. A tool's output is shown as the tool returned it until a standard exists for it; to make outputs compact, formatter_inspect tool=<name> shows its captured real outputs and formatter_save_standard saves a YAML standard (formatter_inspect topic=schema explains the format).";

pub fn defs() -> Vec<J> {
    let inspect = r#"{"name":"formatter_inspect","description":"toolfmt: how tool outputs are formatted. No args: every tool with its standard, captured outputs and savings. tool: one tool's captured real outputs and current standard. topic=schema: how to write a standard.","inputSchema":{"type":"object","properties":{"tool":{"type":"string","description":"tool name as listed, or server.tool"},"capture":{"type":"integer","description":"show this capture in full"},"topic":{"type":"string","enum":["schema"]}}},"annotations":{"readOnlyHint":true}}"#;
    let save = r#"{"name":"formatter_save_standard","description":"toolfmt: save a YAML standard for a tool. It is first replayed on the tool's captured real outputs and the result shows how each looks now. dry_run: test only. delete: remove the standard.","inputSchema":{"type":"object","properties":{"tool":{"type":"string","description":"tool name as listed, or server.tool"},"yaml":{"type":"string","description":"the standard (formatter_inspect topic=schema)"},"dry_run":{"type":"boolean"},"delete":{"type":"boolean"}},"required":["tool"]}}"#;
    vec![
        J::parse(inspect).expect("inspect def"),
        J::parse(save).expect("save def"),
    ]
}

pub const REFERENCE: &str = r#"STANDARD (YAML, file standards/<server>.<tool>.yml; write it against a captured real output)
render: passthrough | json | expression | matrix_table | objects_table | dict_table | template | log | num_stats | tree | image_meta
loss: lossless | lossy | interpretive   (optional; the header shows max(declared, what actually happened))
root: data.items            path to the part to render (fields beside it, e.g. cursors and totals, stay as "key: value" lines)

render params
  expression    format: "{a} + {b} = {result}"   refs to call args and {result}; paths {result.x}; {x:thousands|bytes|fixed:2}; {{ }} = braces
  objects_table mode: toon (default) | markdown; columns: [id, user.login]; rows: 20; max_cell_chars: 80
  matrix_table  mode: plain (default) | markdown; max_rows; max_cols
  dict_table    mode: lines (default) | markdown; max_rows
  json          compact JSON of root/list after pre (projection that stays JSON)
  log           max_items: 10 (level counts + unique errors/warnings with ×N)
  num_stats     sample: 5 (count/min/max/sum/mean; exact fraction for integers)
  tree          max_items (paths grouped by directory)
  template      input: {format: json|text|xml|html|markdown|yaml|toml}; rules: any_txt_parser rules
                  - id: title
                    select: {type: jsonpath, expression: "$.items[*].title"}   # also regex, line, contains, xpath, css, markdown
                    transforms: [trim, unique]
                    limits: {max_items: 20}
                call with _keys: [title] to get only some rules
  formats: {size: bytes, count: thousands, "*": fixed:2}   value formats for table/dict cells

pre (structural, on items before render, in order)
  - unique
  - sort: name          (or sort: true; sort_desc: size)
  - limit_items: 50
  - select: [id, name]

post (text, after render, in order; not truncate_* on tables: use rows/max_cell_chars)
  - drop_lines: "^\\s*$"      - keep_lines: "ERROR|WARN"
  - replace: {from: "x", to: "y", regex: false}
  - truncate_chars: 200 (per line)   - truncate_lines: 40
  - join: ", "
  - mask: true | [api_key, token, password, jwt, bearer, private_key, email]

per-call overrides (in the tool arguments, removed before the upstream call):
  _render, _pre, _post, _template (inline YAML), _limit, _keys

Cuts are always declared ("… N more rows"); pagination cursors are never dropped.
Example:
  render: objects_table
  root: items
  columns: [path, size]
  formats: {size: bytes}
  pre: [{sort_desc: size}, {limit_items: 30}]
"#;

/// Runs one fixture through the pipeline. `std` replaces the lookup.
pub fn run_fixture(
    store: Option<&Store>,
    cfg: Option<&Config>,
    f: &Fixture,
    std: Option<&Standard>,
) -> Option<(Outcome, String)> {
    let server = f.field("server").and_then(J::as_str).unwrap_or("");
    let tool = f.field("tool").and_then(J::as_str).unwrap_or("");
    let pl = f.payload();
    let owned;
    let (data, raw): (Data, &str) = match &pl {
        Payload::Text(t) => (Data::Text(t), t),
        Payload::Json(j) => {
            owned = j.to_json();
            (Data::Json(j), owned.as_str())
        }
        Payload::None => return None,
    };
    let ov = match f.doc.get("request") {
        Some(r) => Overrides::from_request(r),
        None => Overrides::from_request(&f.doc),
    };
    let keys = f.field("keys").and_then(|k| match k {
        J::Arr(a) => Some(a.iter().map(J::text).collect()),
        _ => None,
    });
    let auto = f
        .field("auto")
        .and_then(J::as_bool)
        .unwrap_or_else(|| cfg.is_some_and(|c| c.auto_for(server)));
    let req = Request {
        server,
        tool,
        args: f.args(),
        data,
        keys,
        ov,
        auto,
        family: f
            .field("family")
            .and_then(J::as_str)
            .map(str::to_string)
            .or_else(|| cfg.and_then(|c| c.family_of(server, tool))),
    };
    let out = pipeline::run(store, &req, std);
    Some((out, raw.to_string()))
}

/// The baseline text of an outcome: the formatted text, or the raw payload.
pub fn clean_text(out: &Outcome, raw: &str) -> String {
    match &out.body {
        Body::Formatted(t) => t.clone(),
        _ => raw.to_string(),
    }
}

/// `text` cut at a line boundary to about `max` chars, cut declared.
pub fn preview(text: &str, max: usize) -> String {
    let total = text.chars().count();
    if total <= max {
        return text.to_string();
    }
    let mut out = String::new();
    let mut n = 0usize;
    for line in text.lines() {
        let len = line.chars().count() + 1;
        if n + len > max {
            if n == 0 {
                out.extend(line.chars().take(max));
            }
            break;
        }
        out.push_str(line);
        out.push('\n');
        n += len;
    }
    let shown = out.chars().count();
    format!(
        "{}\n… {} more chars",
        out.trim_end(),
        total.saturating_sub(shown)
    )
}

pub fn kb(n: u64) -> String {
    if n >= 1 << 20 {
        format!("{:.1} MB", n as f64 / (1u64 << 20) as f64)
    } else if n >= 1024 {
        format!("{:.1} KB", n as f64 / 1024.0)
    } else {
        format!("{n} B")
    }
}
