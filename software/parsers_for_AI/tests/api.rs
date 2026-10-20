//! The in-memory API (`any_txt_parser::api`): shorthand and object rules,
//! template sources, error paths with exit codes, the output cap, recon,
//! the text renderings, and equivalence with the published document.

mod support;

use any_txt_parser::api::{
    self, ApiError, ExtractRequest, ExtractResponse, FileReport, ReconRequest, ResultReport,
    RuleSpec,
};
use any_txt_parser::limits::ExitCode;
use any_txt_parser::render::unescape_anchor_text;
use any_txt_parser::run;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static DIR_ID: AtomicU64 = AtomicU64::new(0);

/// A temporary directory of source files, removed on drop.
struct Dir(PathBuf);

impl Dir {
    fn new(name: &str) -> Dir {
        let dir = std::env::temp_dir().join(format!(
            "atp_api_{}_{}_{}",
            std::process::id(),
            name,
            DIR_ID.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Dir(dir)
    }

    fn file(&self, name: &str, bytes: impl AsRef<[u8]>) -> String {
        let path = self.0.join(name);
        std::fs::write(&path, bytes).unwrap();
        path.to_string_lossy().into_owned()
    }

    fn path(&self) -> String {
        self.0.to_string_lossy().into_owned()
    }
}

impl Drop for Dir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// A request that keeps empty results, so tests see every (file, rule)
/// status; `folds_empty_results_by_default` covers the default.
fn rules(input: &str, rules: &[&str]) -> ExtractRequest {
    ExtractRequest {
        input: input.to_string(),
        rules: rules.iter().map(|r| RuleSpec::from(*r)).collect(),
        include_empty: true,
        ..ExtractRequest::default()
    }
}

fn ok(req: &ExtractRequest) -> ExtractResponse {
    api::extract(req).unwrap_or_else(|e| panic!("extract failed: {e}"))
}

fn fail(req: &ExtractRequest) -> ApiError {
    match api::extract(req) {
        Ok(resp) => panic!("expected an error, got {}", api::to_json(&resp)),
        Err(e) => e,
    }
}

/// Items of the only result.
fn items(req: &ExtractRequest) -> Vec<String> {
    let resp = ok(req);
    assert_eq!(resp.results.len(), 1, "{}", api::render_text(&resp));
    resp.results[0].items.clone()
}

fn strings(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| s.to_string()).collect()
}

const LOG: &str = "2026-10-01 INFO start\n2026-10-01 ERROR disk: full | retry\nkey: value here\n2026-10-02 WARN slow code=17 done\n2026-10-02 ERROR timeout code=42\n";

#[test]
fn shorthand_text_types_take_their_expression_verbatim() {
    let dir = Dir::new("text");
    let log = dir.file("app.log", LOG);
    assert_eq!(
        items(&rules(&log, &["contains:ERROR"])),
        strings(&[
            "2026-10-01 ERROR disk: full | retry",
            "2026-10-02 ERROR timeout code=42"
        ])
    );
    // Colons, pipes and spaces after the first colon belong to EXPR.
    assert_eq!(
        items(&rules(&log, &["contains:disk: full | retry"])),
        strings(&["2026-10-01 ERROR disk: full | retry"])
    );
    assert_eq!(
        items(&rules(&log, &["contains:key: value"])),
        strings(&["key: value here"])
    );
    assert_eq!(
        items(&rules(&log, &["contains: here"])),
        strings(&["key: value here"])
    );
    assert!(items(&rules(&log, &["contains: key"])).is_empty());
    assert_eq!(
        items(&rules(&log, &["starts_with:2026-10-02"])),
        strings(&[
            "2026-10-02 WARN slow code=17 done",
            "2026-10-02 ERROR timeout code=42"
        ])
    );
    assert_eq!(
        items(&rules(&log, &["ends_with:done"])),
        strings(&["2026-10-02 WARN slow code=17 done"])
    );
    // Regex alternation and a colon inside the regex.
    assert_eq!(
        items(&rules(&log, &["regex:(?i)warn|disk: f"])),
        strings(&[
            "2026-10-01 ERROR disk: full | retry",
            "2026-10-02 WARN slow code=17 done"
        ])
    );
    // One capture group: the group is the item.
    assert_eq!(
        items(&rules(&log, &["regex:code=(\\d+)"])),
        strings(&["17", "42"])
    );
    // Type names are case-insensitive and may have spaces around them.
    assert_eq!(items(&rules(&log, &[" REGEX :^key"])).len(), 1);
}

#[test]
fn shorthand_line_ranges() {
    let dir = Dir::new("lines");
    let log = dir.file("a.txt", "l1\nl2\nl3\nl4\nl5\n");
    for (spec, expected) in [
        ("line:2", "l2"),
        ("line:2-3", "l2\nl3"),
        ("line: 4 - ", "l4\nl5"),
        ("line:-2", "l1\nl2"),
        ("line:5-9", "l5"),
    ] {
        assert_eq!(items(&rules(&log, &[spec])), strings(&[expected]), "{spec}");
    }
    let resp = ok(&rules(&log, &["line:9-"]));
    assert_eq!(resp.results[0].status, "no matches found");
    for bad in ["line:0", "line:a-b", "line:-", "line:"] {
        let e = fail(&rules(&log, &[bad]));
        assert_eq!(e.code, ExitCode::InvalidTemplate, "{bad}");
        assert!(e.message.contains("line range"), "{bad}: {e}");
    }
    // The compiler's own validation still applies.
    let e = fail(&rules(&log, &["line:5-2"]));
    assert_eq!(e.code, ExitCode::InvalidTemplate);
    assert!(
        e.message.contains("select.to must be >= select.from"),
        "{e}"
    );
}

#[test]
fn shorthand_structured_types() {
    let dir = Dir::new("structured");
    let json = dir.file(
        "users.json",
        r#"{"users": [{"name": "Ann", "role": "admin"}, {"name": "Bob", "role": "user"}], "server": {"port": 80}}"#,
    );
    assert_eq!(
        items(&rules(&json, &["jsonpath:$.users[*].name"])),
        strings(&["Ann", "Bob"])
    );
    assert_eq!(
        items(&rules(
            &json,
            &["jsonpath:$.users[?(@.role == 'admin')].name"]
        )),
        strings(&["Ann"])
    );
    assert_eq!(
        items(&rules(&json, &["path:$.server.port"])),
        strings(&["80"])
    );

    let yaml = dir.file("cfg.yaml", "server:\n  port: 8080\n  hosts: [a, b]\n");
    assert_eq!(
        items(&rules(&yaml, &["path:server.port"])),
        strings(&["8080"])
    );
    assert_eq!(
        items(&rules(&yaml, &["path:$.server.hosts[*]"])),
        strings(&["a", "b"])
    );

    let toml = dir.file("cfg.toml", "[database]\nhost = \"db.local\"\n");
    assert_eq!(
        items(&rules(&toml, &["path:[database].host"])),
        strings(&["db.local"])
    );

    let xml = dir.file(
        "cat.xml",
        "<catalog><item id='a1' type='book'><t>X</t></item><item id='b2'><t>Y</t></item></catalog>",
    );
    assert_eq!(
        items(&rules(&xml, &["xpath://item/@id"])),
        strings(&["a1", "b2"])
    );
    assert_eq!(
        items(&rules(&xml, &["xpath://item[@type='book']/t/text()"])),
        strings(&["X"])
    );

    let html = dir.file(
        "page.html",
        "<html><head><title>T</title></head><body><h2>One</h2><h2>Two</h2><a href='/x'>x</a></body></html>",
    );
    assert_eq!(items(&rules(&html, &["css:h2"])), strings(&["One", "Two"]));
    let mut attr = rules(&html, &["css:a[href]"]);
    attr.extract = Some("attr:href".into());
    assert_eq!(items(&attr), strings(&["/x"]));

    let md = dir.file(
        "notes.md",
        "# Title\n\nText.\n\n## Install\n\n- step one\n- step two\n\n## Usage\n",
    );
    assert_eq!(
        items(&rules(&md, &["markdown:heading"])),
        strings(&["Title", "Install", "Usage"])
    );
    assert_eq!(
        items(&rules(&md, &["markdown: h :^Ins|^Us"])),
        strings(&["Install", "Usage"])
    );
    assert_eq!(
        items(&rules(&md, &["markdown:li"])),
        strings(&["step one", "step two"])
    );
}

#[test]
fn shorthand_select_mappings() {
    let select = |spec: &str| {
        serde_yaml::to_string(&api::shorthand_select(spec).unwrap())
            .unwrap()
            .trim_end()
            .replace('\n', "; ")
    };
    assert_eq!(select("regex:a|b"), "type: regex; expression: a|b");
    assert_eq!(select("Contains::x"), "type: contains; expression: :x");
    assert_eq!(
        select("xpath://a[@x='1']/@y"),
        "type: xpath; expression: //a[@x='1']/@y"
    );
    assert_eq!(select("line:3"), "type: line; from: 3; to: 3");
    assert_eq!(
        select("line:3-"),
        format!("type: line; from: 3; to: {}", u64::MAX)
    );
    assert_eq!(select("markdown:code"), "type: markdown; node: code");
    assert_eq!(
        select("markdown:heading:^a: b"),
        "type: markdown; node: heading; pattern: '^a: b'"
    );
    assert_eq!(api::SHORTHAND_TYPES.len(), 10);
    for kind in api::SHORTHAND_TYPES {
        let spec = match *kind {
            "line" => "line:1".to_string(),
            "markdown" => "markdown:p".to_string(),
            other => format!("{other}:x"),
        };
        assert!(api::shorthand_select(&spec).is_ok(), "{spec}");
    }
}

#[test]
fn shorthand_options_apply_to_every_shorthand_rule() {
    let dir = Dir::new("options");
    let log = dir.file(
        "a.log",
        "a\n  ERROR one  \nb\n  ERROR one  \nERROR two\nc\n",
    );
    let mut req = rules(&log, &["contains:ERROR", "contains:two"]);
    req.transforms = strings(&["trim", "unique"]);
    let resp = ok(&req);
    assert_eq!(resp.results[0].items, strings(&["ERROR one", "ERROR two"]));
    assert_eq!(resp.results[1].items, strings(&["ERROR two"]));
    assert_eq!(resp.results[0].rule, "r1");
    assert_eq!(resp.results[1].rule, "r2");
    assert_eq!(resp.results[1].key, "r2");

    let mut req = rules(&log, &["contains:two"]);
    req.context_before = Some(1);
    req.context_after = Some(1);
    assert_eq!(items(&req), strings(&["  ERROR one  \nERROR two\nc"]));

    let mut req = rules(&log, &["contains:ERROR"]);
    req.transforms = strings(&[
        "trim",
        "limit_items: 2",
        r#"{"replace": {"from": "ERROR", "to": "E"}}"#,
    ]);
    let resp = ok(&req);
    assert_eq!(resp.results[0].items, strings(&["E one", "E one"]));
    assert!(resp.results[0].truncated);
    assert_eq!(
        resp.results[0].warnings,
        strings(&["1 item(s) omitted by limit_items(2)"])
    );
    assert_eq!(resp.exit_code, 1);

    let mut req = rules(&log, &["contains:ERROR"]);
    req.max_items = Some(1);
    req.max_item_chars = Some(4);
    let resp = ok(&req);
    assert_eq!(resp.results[0].items, strings(&["  ER"]));
    assert_eq!(
        resp.results[0].warnings,
        strings(&[
            "1 item(s) truncated to max_item_chars: item 1 (13 -> 4 chars)",
            "2 item(s) omitted after max_items limit"
        ])
    );

    // Request JSON: transforms as strings or objects, sizes as numbers.
    let json = format!(
        r#"{{"input": {}, "rules": ["contains:ERROR"], "transforms": ["trim", {{"limit_items": 1}}], "max_file_size": 4096, "max_memory": "1m"}}"#,
        serde_json::Value::String(log.clone())
    );
    let req: ExtractRequest = serde_json::from_str(&json).unwrap();
    assert_eq!(req.transforms, strings(&["trim", r#"{"limit_items":1}"#]));
    assert_eq!(req.max_file_size.as_deref(), Some("4096"));
    assert_eq!(items(&req), strings(&["ERROR one"]));
}

#[test]
fn object_rules_from_json_yaml_and_flow_text() {
    let dir = Dir::new("objects");
    let log = dir.file("a.log", "id=7 ok\nid=8 fail\nnothing\n");
    let json = format!(
        r#"{{"input": {}, "rules": [
            {{"select": {{"type": "regex", "expression": "id=(\\d+)"}}, "transforms": ["trim", {{"limit_items": 5}}]}},
            {{"id": "fails", "select": {{"type": "contains", "expression": "fail"}}, "output": {{"key": "failed"}}}},
            "starts_with:no"
        ]}}"#,
        serde_json::Value::String(log.clone())
    );
    let req: ExtractRequest = serde_json::from_str(&json).unwrap();
    assert!(matches!(req.rules[0], RuleSpec::Object(_)));
    let resp = ok(&req);
    let summary: Vec<(&str, &str, Vec<String>)> = resp
        .results
        .iter()
        .map(|r| (r.rule.as_str(), r.key.as_str(), r.items.clone()))
        .collect();
    assert_eq!(
        summary,
        vec![
            ("r1", "r1", strings(&["7", "8"])),
            ("fails", "failed", strings(&["id=8 fail"])),
            ("r3", "r3", strings(&["nothing"])),
        ]
    );

    let yaml = format!(
        "input: '{}'\nrules:\n  - select: {{type: contains, expression: ok}}\n    when: {{format: text}}\n  - \"{{select: {{type: ends_with, expression: fail}}, output: {{key: tail}}}}\"\n",
        log.replace('\'', "''")
    );
    let req: ExtractRequest = serde_yaml::from_str(&yaml).unwrap();
    assert!(matches!(&req.rules[1], RuleSpec::Shorthand(s) if s.starts_with('{')));
    let resp = ok(&req);
    assert_eq!(resp.results[0].items, strings(&["id=7 ok"]));
    assert_eq!(resp.results[1].key, "tail");
    assert_eq!(resp.results[1].items, strings(&["id=8 fail"]));

    // A rule that is neither a string nor a mapping.
    let bad = r#"{"input": "x", "rules": [42]}"#;
    let e = serde_json::from_str::<ExtractRequest>(bad).unwrap_err();
    assert!(
        e.to_string()
            .contains("'TYPE:EXPR' string or a rule object"),
        "{e}"
    );
    // Unknown request fields are rejected.
    let e = serde_json::from_str::<ExtractRequest>(r#"{"input": "x", "rulez": []}"#).unwrap_err();
    assert!(e.to_string().contains("unknown field `rulez`"), "{e}");
}

#[test]
fn template_path_and_inline_template_text() {
    let dir = Dir::new("templates");
    let log = dir.file("a.log", "ERROR x\nok\n");
    let yaml = "version: 1\nname: t\nrules:\n  - id: e\n    select: {type: contains, expression: ERROR}\n    output: {key: errors}\n";
    let tpl = dir.file("tpl.yml", yaml);
    let req = ExtractRequest {
        input: log.clone(),
        template_path: Some(PathBuf::from(&tpl)),
        ..ExtractRequest::default()
    };
    let from_file = ok(&req);
    assert_eq!(from_file.results[0].key, "errors");
    assert_eq!(from_file.results[0].items, strings(&["ERROR x"]));

    let req = ExtractRequest {
        input: log.clone(),
        template: Some(yaml.to_string()),
        ..ExtractRequest::default()
    };
    assert_eq!(ok(&req), from_file);

    // JSON text, indented with tabs (not valid YAML indentation).
    let json = "{\n\t\"version\": 1,\n\t\"name\": \"t\",\n\t\"rules\": [\n\t\t{\"id\": \"e\", \"select\": {\"type\": \"contains\", \"expression\": \"ERROR\"}, \"output\": {\"key\": \"errors\"}}\n\t]\n}";
    let req = ExtractRequest {
        input: log.clone(),
        template: Some(json.to_string()),
        ..ExtractRequest::default()
    };
    assert_eq!(ok(&req), from_file);

    // The request format overrides the template, as --format does.
    let req = ExtractRequest {
        input: log,
        template: Some(yaml.replace("rules:", "input: {format: json}\nrules:")),
        format: Some("text".into()),
        ..ExtractRequest::default()
    };
    let resp = ok(&req);
    assert_eq!(resp.files[0].format_source, "CLI override");
    assert_eq!(resp.results[0].items, strings(&["ERROR x"]));
}

#[test]
fn errors_carry_exit_codes() {
    let dir = Dir::new("errors");
    let log = dir.file("a.log", "x\n");
    let code = |req: &ExtractRequest| fail(req).code;

    // Rules: exit 2.
    let e = fail(&rules(&log, &["grep:x"]));
    assert_eq!(e.code, ExitCode::InvalidTemplate);
    assert_eq!(
        e.message,
        "rule 'r1': unknown rule type 'grep' in 'grep:x': valid types are contains, starts_with, ends_with, regex, jsonpath, path, xpath, css, line, markdown"
    );
    assert_eq!(e.exit_code(), 2);
    let e = fail(&rules(&log, &["contains:x", "ERROR"]));
    assert!(
        e.message
            .starts_with("rule 'r2': invalid rule 'ERROR': expected TYPE:EXPR"),
        "{e}"
    );
    let e = fail(&rules(&log, &["regex:"]));
    assert!(e.message.contains("empty expression in 'regex:'"), "{e}");
    let e = fail(&rules(&log, &["regex:(unclosed"]));
    assert_eq!(e.code, ExitCode::InvalidTemplate);
    assert!(e.message.starts_with("rule 'r1': invalid regex:"), "{e}");
    let e = fail(&rules(&log, &["jsonpath:$.a[?(@.x)]"]));
    assert!(e.message.starts_with("rule 'r1': invalid jsonpath:"), "{e}");
    let e = fail(&rules(&log, &["markdown:nope"]));
    assert!(
        e.message.contains("unknown markdown node type 'nope'"),
        "{e}"
    );
    let mut req = rules(&log, &["contains:x"]);
    req.transforms = strings(&["trimm"]);
    assert_eq!(fail(&req).message, "rule 'r1': unknown transform 'trimm'");
    req.transforms = strings(&["limit_items: [1"]);
    assert!(
        fail(&req)
            .message
            .starts_with("invalid transform 'limit_items: [1'")
    );
    req.transforms = strings(&["limit_items: x"]);
    assert_eq!(
        fail(&req).message,
        "rule 'r1': limit_items requires an integer parameter"
    );
    let mut req = rules(&log, &["contains:x"]);
    req.max_items = Some(0);
    assert_eq!(fail(&req).message, "rule 'r1': max_items must be positive");
    let mut req = rules(&log, &["contains:x"]);
    req.extract = Some("html".into());
    assert!(fail(&req).message.contains("unknown extract mode 'html'"));
    // Object rule errors keep the compiler's message, without a location in
    // the generated YAML.
    let e = fail(&rules(&log, &["{selct: {type: contains, expression: x}}"]));
    assert_eq!(e.code, ExitCode::InvalidTemplate);
    assert!(e.message.contains("unknown field `selct`"), "{e}");
    assert!(!e.message.contains(" at line "), "{e}");
    let e = fail(&rules(
        &log,
        &[
            "contains:x",
            "{id: r1, select: {type: contains, expression: y}}",
        ],
    ));
    assert_eq!(e.message, "duplicate rule id 'r1'");
    let e = fail(&rules(&log, &["{select: [1"]));
    assert!(e.message.starts_with("rule #1: invalid rule object"), "{e}");
    let e = fail(&ExtractRequest {
        input: log.clone(),
        template: Some("version: 1\nname: t\n".into()),
        ..ExtractRequest::default()
    });
    assert_eq!(
        (e.code, e.message.as_str()),
        (ExitCode::InvalidTemplate, "template has no 'rules' section")
    );

    // Request and input: exit 3.
    let e = fail(&rules("", &["contains:x"]));
    assert_eq!(e.code, ExitCode::InvalidInputOrCli);
    assert!(e.message.starts_with("missing required 'input'"), "{e}");
    assert_eq!(e.to_string(), format!("{} (exit 3)", e.message));
    assert_eq!(
        api::to_json(&e),
        format!(
            r#"{{"exit_code":3,"message":{}}}"#,
            serde_json::Value::String(e.message.clone())
        )
    );
    let e = fail(&rules(&log, &[]));
    assert_eq!(
        e.message,
        "give exactly one of template_path, template or rules (got none)"
    );
    let mut req = rules(&log, &["contains:x"]);
    req.template = Some("x".into());
    assert_eq!(
        fail(&req).message,
        "give exactly one of template_path, template or rules (got template, rules)"
    );
    let req = ExtractRequest {
        input: log.clone(),
        template_path: Some(dir.0.join("missing.yml")),
        ..ExtractRequest::default()
    };
    assert_eq!(code(&req), ExitCode::InvalidInputOrCli);
    assert!(fail(&req).message.starts_with("template file not found"));
    let missing = dir.0.join("none_*.log").to_string_lossy().into_owned();
    let e = fail(&rules(&missing, &["contains:x"]));
    assert_eq!(e.code, ExitCode::InvalidInputOrCli);
    assert!(e.message.starts_with("no input files matched"), "{e}");
    for (field, set) in [
        (
            "format",
            (|r: &mut ExtractRequest| r.format = Some("csv".into())) as fn(&mut ExtractRequest),
        ),
        ("encoding", |r| r.encoding = Some("koi8-r".into())),
        ("max_file_size", |r| {
            r.max_file_size = Some("12 parsecs".into())
        }),
        ("max_memory", |r| r.max_memory = Some("-1".into())),
        ("max_files", |r| r.max_files = Some(0)),
        ("max_matches", |r| r.max_matches = Some(0)),
        ("max_output_chars", |r| r.max_output_chars = Some(0)),
    ] {
        let mut req = rules(&log, &["contains:x"]);
        set(&mut req);
        let e = fail(&req);
        assert_eq!(e.code, ExitCode::InvalidInputOrCli, "{field}: {e}");
    }
    // Shorthand options without a shorthand rule.
    let mut req = rules(&log, &["{select: {type: contains, expression: x}}"]);
    req.context_after = Some(2);
    req.max_items = Some(3);
    let e = fail(&req);
    assert_eq!(e.code, ExitCode::InvalidInputOrCli);
    assert!(
        e.message
            .starts_with("context_after, max_items apply only to shorthand rules"),
        "{e}"
    );

    // Resource limits: exit 6.
    dir.file("b.log", "y\n");
    let mut req = rules(&dir.path(), &["contains:x"]);
    req.max_files = Some(1);
    assert_eq!(code(&req), ExitCode::ResourceLimit);
    let mut req = rules(&log, &["contains:x"]);
    req.max_file_size = Some("1".into());
    assert_eq!(code(&req), ExitCode::ResourceLimit);
    let mut req = rules(&dir.path(), &["contains:x"]);
    req.max_memory = Some("3".into());
    assert_eq!(code(&req), ExitCode::ResourceLimit);
}

#[test]
fn statuses_diagnostics_and_notes_follow_the_cli() {
    let dir = Dir::new("statuses");
    dir.file("a.json", r#"{"a": [1, 2]}"#);
    dir.file("b.json", "{not json");
    dir.file("c.log", "plain\n");
    let mut req = rules(&dir.path(), &["jsonpath:$.a[*]"]);
    req.hash = true;
    let resp = ok(&req);
    assert_eq!(
        (resp.run_status.as_str(), resp.exit_code),
        ("partial_success", 1)
    );
    let files: Vec<(&str, &str)> = resp
        .files
        .iter()
        .map(|f| (f.format.as_str(), f.status.as_str()))
        .collect();
    // The text file is skipped by the jsonpath shorthand rule, not warned.
    assert_eq!(
        files,
        vec![("json", "success"), ("json", "error"), ("text", "success")]
    );
    assert!(
        resp.files[1]
            .error
            .as_deref()
            .unwrap()
            .starts_with("invalid JSON")
    );
    assert!(
        resp.files
            .iter()
            .all(|f| f.hash.as_ref().is_some_and(|h| h.len() == 64))
    );
    assert_eq!(resp.diagnostics.len(), 1);
    assert!(resp.diagnostics[0].contains("b.json: invalid JSON"));
    let statuses: Vec<&str> = resp.results.iter().map(|r| r.status.as_str()).collect();
    assert_eq!(statuses, vec!["success"]);
    assert_eq!(
        resp.notes,
        strings(&["rule 'r1' (jsonpath) skipped files it cannot read: 1 text"])
    );

    // Every file failed: a response with exit 3, like the CLI.
    let bad = Dir::new("all_fail");
    bad.file("x.json", "[");
    let resp = ok(&rules(&bad.path(), &["jsonpath:$"]));
    assert_eq!((resp.run_status.as_str(), resp.exit_code), ("failure", 3));
    assert!(resp.results.is_empty());

    // No rule applies: the CLI's run-level note.
    let resp = ok(&ExtractRequest {
        input: dir.path(),
        format: Some("text".into()),
        rules: vec![RuleSpec::from(
            "{when: {format: json}, select: {type: path, expression: a}}",
        )],
        ..ExtractRequest::default()
    });
    assert_eq!(resp.exit_code, 1);
    assert!(resp.results.is_empty());
    assert!(
        resp.notes
            .iter()
            .any(|n| n.starts_with("Rule 'r1' never applies"))
    );
    assert!(
        resp.notes
            .iter()
            .any(|n| n.starts_with("No rule applies in this run"))
    );
    assert!(resp.files.iter().all(|f| f.status == "warning"));

    // Encoding override and BOM reporting.
    let enc = Dir::new("encoding");
    let mut utf16 = vec![0xFF, 0xFE];
    for unit in "ERROR ж\n".encode_utf16() {
        utf16.extend(unit.to_le_bytes());
    }
    let path = enc.file("u16.log", &utf16);
    let mut req = rules(&path, &["contains:ERROR"]);
    req.encoding = Some("UTF-16LE".into());
    let resp = ok(&req);
    assert_eq!(resp.files[0].encoding, "utf-16le (BOM)");
    assert_eq!(resp.results[0].items, strings(&["ERROR ж"]));
    let resp = ok(&rules(&path, &["contains:ERROR"]));
    assert_eq!(resp.files[0].status, "error");
    assert!(
        resp.files[0]
            .error
            .as_deref()
            .unwrap()
            .contains("invalid UTF-8")
    );
}

#[test]
fn output_cap_stops_adding_items_and_says_where() {
    let dir = Dir::new("cap");
    dir.file("a.log", "aaaa\nbbbb\ncccc\n");
    dir.file("b.log", "dddd\neeee\n");
    let mut req = rules(&dir.path(), &["contains:"]);
    let e = fail(&req);
    assert!(e.message.contains("empty expression"));
    req.rules = vec![RuleSpec::from("regex:.")];
    let full = ok(&req);
    assert_eq!(full.exit_code, 0);
    assert!(!full.truncated_output);

    req.max_output_chars = Some(10);
    let resp = ok(&req);
    assert!(resp.truncated_output);
    assert_eq!(
        (resp.run_status.as_str(), resp.exit_code),
        ("partial_success", 1)
    );
    assert_eq!(resp.results[0].items, strings(&["aaaa", "bbbb"]));
    assert!(resp.results[0].truncated);
    assert_eq!(
        resp.results[0].warnings,
        strings(&[
            "2 of 3 item(s) returned: the output cap max_output_chars (10 characters) was reached"
        ])
    );
    assert!(resp.results[1].items.is_empty());
    assert_eq!(
        resp.results[1].warnings,
        strings(&[
            "0 of 2 item(s) returned: the output cap max_output_chars (10 characters) was reached"
        ])
    );
    assert_eq!(resp.notes.len(), 1);
    let note = &resp.notes[0];
    assert!(
        note.starts_with(
            "output truncated by max_output_chars 10: stopped in result 1 of 2 (source '"
        ),
        "{note}"
    );
    assert!(
        note.ends_with("after 2 of 3 item(s); 3 item(s) not returned in total"),
        "{note}"
    );
    // Files keep their own status: the cap only limits what is returned.
    assert!(resp.files.iter().all(|f| f.status == "success"));
    assert!(
        api::render_text(&resp)
            .lines()
            .next()
            .unwrap()
            .ends_with("; output truncated")
    );

    // Exactly at the cap: nothing is cut.
    req.max_output_chars = Some(20);
    let resp = ok(&req);
    assert!(!resp.truncated_output);
    assert_eq!(resp, full);

    // The template's output.max_output_chars caps the API the same way.
    let resp = ok(&ExtractRequest {
        input: dir.path(),
        template: Some(
            "version: 1\nname: t\noutput: {max_output_chars: 6}\nrules:\n  - id: all\n    select: {type: regex, expression: '.'}\n".into(),
        ),
        ..ExtractRequest::default()
    });
    assert!(resp.truncated_output);
    assert_eq!(resp.results[0].items, strings(&["aaaa"]));
    assert!(resp.notes[0].starts_with("output truncated by template output.max_output_chars 6:"));
}

#[test]
fn render_text_format() {
    let file = |path: &str, status: &str, error: Option<&str>| FileReport {
        path: path.into(),
        size: 10,
        format: "text".into(),
        format_source: "extension".into(),
        encoding: "utf-8".into(),
        status: status.into(),
        error: error.map(str::to_string),
        hash: None,
    };
    let result = |rule: &str, key: &str, status: &str, items: &[&str]| ResultReport {
        source: "logs/app.log".into(),
        format: "text".into(),
        rule: rule.into(),
        key: key.into(),
        status: status.into(),
        items: strings(items),
        lines: None,
        spans: None,
        truncated: false,
        warnings: Vec::new(),
        errors: Vec::new(),
    };
    let mut second = result("r2", "codes", "error", &[]);
    second.truncated = true;
    second.errors = strings(&["bad\nthing"]);
    second.warnings = strings(&["1 item(s) omitted after max_items limit"]);
    let resp = ExtractResponse {
        version: "9.9.9".into(),
        run_status: "partial_success".into(),
        exit_code: 1,
        files: vec![
            file("logs/app.log", "warning", None),
            file("logs/bad.json", "error", Some("invalid JSON: x")),
        ],
        results: vec![
            result(
                "r1",
                "r1",
                "success",
                &["2026 ERROR disk", "multi\nline\n", ""],
            ),
            result("r3", "r3", "success", &["- dash"]),
            second,
        ],
        diagnostics: strings(&["logs/bad.json: invalid JSON: x"]),
        notes: strings(&["a\r\nnote"]),
        truncated_output: false,
    };
    let expected = concat!(
        "any_txt_parser 9.9.9: partial_success, exit 1; files 2 (success 0, warning 1, error 1); results 3; items 4\n",
        "diagnostic: logs/bad.json: invalid JSON: x\n",
        "note: a\\r\\nnote\n",
        "== logs/app.log [text] rule r1 (success, 3 items)\n",
        "- 2026 ERROR disk\n",
        "- multi\n",
        "    line\n",
        "    \n",
        "- \n",
        "== logs/app.log [text] rule r3 (success, 1 item)\n",
        "- - dash\n",
        "== logs/app.log [text] rule r2 key codes (error, 0 items, truncated)\n",
        "error: bad\\nthing\n",
        "warning: 1 item(s) omitted after max_items limit\n",
        "== end: partial_success, exit 1\n",
    );
    assert_eq!(api::render_text(&resp), expected);
    assert_eq!(
        decode_text_items(expected),
        vec![
            strings(&["2026 ERROR disk", "multi\nline\n", ""]),
            strings(&["- dash"]),
            Vec::new(),
        ]
    );

    // JSON: compact, and it reads back to the same response.
    let json = api::to_json(&resp);
    assert!(!json.contains('\n'));
    assert!(json.starts_with(r#"{"version":"9.9.9","run_status":"partial_success","exit_code":1,"files":[{"path":"logs/app.log","#));
    assert!(!json.contains("\"hash\""), "absent hash is omitted");
    assert_eq!(
        serde_json::from_str::<ExtractResponse>(&json).unwrap(),
        resp
    );
    assert_eq!(
        serde_json::from_str::<ExtractResponse>(&api::to_json_pretty(&resp)).unwrap(),
        resp
    );
}

/// Items per result from `render_text` output: what a reader of stdout
/// gets back (source line prefixes `L<n>: `, `L<n>- `, `L<a>-L<b>: `
/// removed; no test item starts with one).
fn decode_text_items(text: &str) -> Vec<Vec<String>> {
    let mut results: Vec<Vec<String>> = Vec::new();
    for line in text.lines() {
        if line.starts_with("== ") && !line.starts_with("== end: ") {
            results.push(Vec::new());
        } else if let Some(item) = line.strip_prefix("- ") {
            results
                .last_mut()
                .unwrap()
                .push(strip_location(item).to_string());
        } else if line == "-" {
            results.last_mut().unwrap().push(String::new());
        } else if let Some(rest) = line.strip_prefix("    ") {
            let item = results.last_mut().unwrap().last_mut().unwrap();
            item.push('\n');
            item.push_str(strip_location(rest));
        }
    }
    results
}

/// `line` without a leading `L<n>: `, `L<n>- ` or `L<a>-L<b>: `.
fn strip_location(line: &str) -> &str {
    let number = |s: &str| -> Option<usize> {
        let rest = s.strip_prefix('L')?;
        let digits = rest.len() - rest.trim_start_matches(|c: char| c.is_ascii_digit()).len();
        (digits > 0).then_some(1 + digits)
    };
    let Some(mut at) = number(line) else {
        return line;
    };
    if let Some(end) = line[at..].strip_prefix('-').and_then(number) {
        at += 1 + end;
    }
    match &line[at..] {
        rest if rest.starts_with(": ") || rest.starts_with("- ") => &rest[2..],
        _ => line,
    }
}

#[test]
fn render_text_of_a_real_run_decodes_to_the_items() {
    let dir = Dir::new("text_real");
    let log = dir.file("a.log", "x1\n  x2 <!-- BLOCK: B0001 END -->\nx3\n");
    let mut req = rules(&log, &["contains:x", "contains:x2"]);
    req.context_after = Some(1);
    let resp = ok(&req);
    let text = api::render_text(&resp);
    let items: Vec<Vec<String>> = resp.results.iter().map(|r| r.items.clone()).collect();
    assert_eq!(decode_text_items(&text), items);
    assert_eq!(items[1], strings(&["  x2 <!-- BLOCK: B0001 END -->\nx3"]));
    assert!(text.contains(&format!(
        "== {} [text] rule r1 (success, 3 items)\n",
        log.replace('\\', "/")
    )));
    assert!(text.ends_with("== end: success, exit 0\n"));
}

/// The text of the only result of `req`, without its header lines.
fn result_text(req: &ExtractRequest) -> String {
    let text = api::render_text(&ok(req));
    text.lines()
        .filter(|l| l.starts_with("- ") || l.starts_with("    "))
        .map(|l| format!("{}\n", l))
        .collect()
}

#[test]
fn text_rules_report_source_lines() {
    let dir = Dir::new("text_lines");
    let log = dir.file("a.log", "start\nERROR one\nmid\nmid2\nERROR two\nend\n");

    // One line per match: `lines`, no `spans`; `L<n>: ` in text.
    let req = rules(&log, &["contains:ERROR"]);
    let resp = ok(&req);
    assert_eq!(resp.results[0].lines, Some(vec![2, 5]));
    assert_eq!(resp.results[0].spans, None);
    assert_eq!(result_text(&req), "- L2: ERROR one\n- L5: ERROR two\n");
    let json = api::to_json(&resp);
    assert!(
        json.contains(r#""items":["ERROR one","ERROR two"],"lines":[2,5],"truncated":false"#),
        "{}",
        json
    );
    assert_eq!(
        serde_json::from_str::<ExtractResponse>(&json).unwrap(),
        resp
    );

    // Context: every line numbered, `:` on the match, `-` on context;
    // the window stops at the first and the last line.
    let mut req = rules(&log, &["contains:ERROR"]);
    req.context_before = Some(1);
    req.context_after = Some(1);
    let resp = ok(&req);
    assert_eq!(resp.results[0].lines, Some(vec![2, 5]));
    assert_eq!(resp.results[0].spans, Some(vec![[1, 3], [4, 6]]));
    assert_eq!(
        result_text(&req),
        "- L1- start\n    L2: ERROR one\n    L3- mid\n- L4- mid2\n    L5: ERROR two\n    L6- end\n"
    );
    req.context_before = Some(3);
    req.context_after = Some(0);
    assert_eq!(ok(&req).results[0].spans, Some(vec![[1, 2], [2, 5]]));

    // A regex group is the item; its line is the line it was found on.
    let req = rules(
        &log,
        &[r"regex:ERROR (\w+)", "starts_with:mid", "ends_with:two"],
    );
    let resp = ok(&req);
    let lines: Vec<_> = resp.results.iter().map(|r| r.lines.clone()).collect();
    assert_eq!(lines, [Some(vec![2, 5]), Some(vec![3, 4]), Some(vec![5])]);
    assert!(result_text(&rules(&log, &[r"regex:ERROR (\w+)"])).starts_with("- L2: one\n"));

    // A line range is one item: its first line is the "match".
    let req = rules(&log, &["line:2-3"]);
    let resp = ok(&req);
    assert_eq!(resp.results[0].lines, Some(vec![2]));
    assert_eq!(resp.results[0].spans, Some(vec![[2, 3]]));
    assert_eq!(result_text(&req), "- L2: ERROR one\n    L3- mid\n");

    // Transforms that reorder or drop items keep each item's own line.
    let dup = dir.file("dup.log", "b x\na x\nb x\n");
    let mut req = rules(&dup, &["contains:x"]);
    req.transforms = strings(&["unique", "sort"]);
    let resp = ok(&req);
    assert_eq!(resp.results[0].items, strings(&["a x", "b x"]));
    assert_eq!(resp.results[0].lines, Some(vec![2, 1]));

    // A transform that changes the line count: only the first line is
    // marked, with the source lines of the item.
    let mut req = rules(&log, &["contains:ERROR"]);
    req.context_after = Some(1);
    req.transforms = strings(&["truncate_lines: 1"]);
    assert_eq!(
        result_text(&req),
        "- L2-L3: ERROR one\n- L5-L6: ERROR two\n"
    );

    // `join` merges items: no line numbers at all.
    let mut req = rules(&log, &["contains:ERROR"]);
    req.transforms = strings(&["join: ','"]);
    let resp = ok(&req);
    assert_eq!(resp.results[0].items, strings(&["ERROR one,ERROR two"]));
    assert_eq!(
        (&resp.results[0].lines, &resp.results[0].spans),
        (&None, &None)
    );
    assert_eq!(result_text(&req), "- ERROR one,ERROR two\n");

    // The output cap keeps the numbers of the items it keeps.
    let mut req = rules(&log, &["contains:ERROR"]);
    req.context_after = Some(1);
    req.max_output_chars = Some(15);
    let resp = ok(&req);
    assert_eq!(resp.results[0].items.len(), 1);
    assert_eq!(resp.results[0].lines, Some(vec![2]));
    assert_eq!(resp.results[0].spans, Some(vec![[2, 3]]));
    req.max_output_chars = Some(1);
    let resp = ok(&req);
    assert!(resp.results[0].items.is_empty());
    assert_eq!(
        (&resp.results[0].lines, &resp.results[0].spans),
        (&None, &None)
    );

    // Other selector types have no line numbers; JSON leaves the keys out.
    let data = dir.file("d.json", r#"{"a": [1, 2]}"#);
    let resp = ok(&rules(&data, &["jsonpath:$.a[*]"]));
    assert_eq!(resp.results[0].lines, None);
    let json = api::to_json(&resp);
    assert!(
        !json.contains("\"lines\"") && !json.contains("\"spans\""),
        "{}",
        json
    );
    assert_eq!(
        result_text(&rules(&data, &["jsonpath:$.a[*]"])),
        "- 1\n- 2\n"
    );
}

#[test]
fn stable_deduplicate_keeps_the_lines_of_the_kept_items() {
    let dir = Dir::new("dedup_lines");
    let log = dir.file("a.log", "x 1\ny\nx 1\nx 2\n");
    let req = ExtractRequest {
        input: log,
        template: Some(
            "version: 1\nname: dedup\ndocument: {stable_deduplicate: true}\nrules:\n  - id: r\n    select: {type: contains, expression: x}\n"
                .to_string(),
        ),
        ..ExtractRequest::default()
    };
    let resp = ok(&req);
    assert_eq!(resp.results[0].items, strings(&["x 1", "x 2"]));
    assert_eq!(resp.results[0].lines, Some(vec![1, 4]));
}

fn utf16(text: &str, little_endian: bool) -> Vec<u8> {
    let mut bytes = if little_endian {
        vec![0xFF, 0xFE]
    } else {
        vec![0xFE, 0xFF]
    };
    for unit in text.encode_utf16() {
        bytes.extend(if little_endian {
            unit.to_le_bytes()
        } else {
            unit.to_be_bytes()
        });
    }
    bytes
}

fn recon_of(path: &str) -> api::ReconFile {
    let resp = api::recon(&ReconRequest {
        path: path.to_string(),
        ..ReconRequest::default()
    })
    .unwrap();
    assert_eq!(resp.files.len(), 1);
    resp.files.into_iter().next().unwrap()
}

#[test]
fn recon_reports_format_shape_and_sample() {
    let dir = Dir::new("recon_formats");
    let f = recon_of(&dir.file(
        "users.json",
        "{\"users\": [\n  {\"id\": 1, \"name\": \"Ann\"},\n  {\"id\": 2, \"name\": \"Bob\"}],\n \"total\": 2, \"meta\": {\"v\": 1}}\n",
    ));
    assert_eq!(
        (f.format.as_str(), f.format_source.as_str()),
        ("json", "extension")
    );
    assert_eq!(f.encoding.as_deref(), Some("utf-8"));
    assert_eq!(f.selectors, strings(&["jsonpath", "path"]));
    assert_eq!(f.lines, Some(4));
    assert_eq!(f.sample[1], "  {\"id\": 1, \"name\": \"Ann\"},");
    assert_eq!(
        f.shape,
        strings(&["object with 3 key(s): users[2], total, meta{1}"])
    );

    let f = recon_of(&dir.file("list", "[{\"id\": 1, \"tags\": [\"a\"]}, {\"id\": 2}]"));
    assert_eq!(
        (f.format.as_str(), f.format_source.as_str()),
        ("json", "content")
    );
    assert_eq!(
        f.shape,
        strings(&["list of 2 item(s); item [0]: object with 2 key(s): id, tags[1]"])
    );

    let f = recon_of(&dir.file(
        "k8s.yaml",
        "apiVersion: v1\nkind: Service\n---\napiVersion: apps/v1\nkind: Deployment\nspec: {replicas: 2}\n",
    ));
    assert_eq!(f.selectors, strings(&["path"]));
    assert_eq!(
        f.shape,
        strings(&[
            "2 YAML documents",
            "document 1: mapping with 2 key(s): apiVersion, kind",
            "document 2: mapping with 3 key(s): apiVersion, kind, spec{1}",
        ])
    );

    let f = recon_of(&dir.file(
        "cfg.toml",
        "title = \"x\"\n[database]\nhost = \"db\"\nport = 5432\n",
    ));
    assert_eq!(f.format, "toml");
    assert!(
        f.shape[0].starts_with("table with 2 key(s): "),
        "{:?}",
        f.shape
    );
    assert!(f.shape[0].contains("database{2}") && f.shape[0].contains("title"));

    let f = recon_of(&dir.file(
        "pom.xml",
        "<project xmlns=\"http://maven.apache.org/POM/4.0.0\" xmlns:x=\"urn:x\" x:a=\"1\"><modelVersion>4.0.0</modelVersion><dependency/><dependency/><x:ext/></project>",
    ));
    assert_eq!(f.selectors, strings(&["xpath"]));
    assert_eq!(
        f.shape,
        strings(&[
            "root element <project> in namespace http://maven.apache.org/POM/4.0.0",
            "namespaces: xmlns=http://maven.apache.org/POM/4.0.0, xmlns:x=urn:x",
            "root attributes: @x:a",
            "child elements: modelVersion, dependency x2, x:ext",
        ])
    );
    let f = recon_of(&dir.file("broken.xml", "<a><b></a>"));
    assert_eq!(f.format, "xml");
    assert!(f.shape[0].starts_with("invalid XML"), "{:?}", f.shape);

    let f = recon_of(&dir.file(
        "page.html",
        "<!doctype html><html><head><title> Shop\n Page </title></head><body><h1>Catalog</h1><h2>Books</h2><ul><li><a href='/a'>A</a></li><li>B</li></ul><table><tr><td>1</td></tr></table></body></html>",
    ));
    assert_eq!(f.selectors, strings(&["css"]));
    assert_eq!(
        f.shape,
        strings(&[
            "title: Shop Page",
            "h1: Catalog",
            "h2: Books",
            "elements: 1 table, 1 ul, 2 li, 1 a",
        ])
    );

    let f = recon_of(&dir.file(
        "notes.md",
        "# Guide\n\n## Install `tool`\n\n```sh\nrun\n```\n\n- [link](http://x)\n- two\n\n### Deep\n",
    ));
    assert_eq!(f.selectors, strings(&["markdown"]));
    assert_eq!(
        f.shape,
        strings(&[
            "# Guide",
            "## Install tool",
            "### Deep",
            "nodes: 3 heading, 1 code_block, 2 list_item, 1 link",
        ])
    );

    let long = format!("{}\n", "z".repeat(310));
    let f = recon_of(&dir.file("app.log", format!("a\r\n\nb\0c\n{long}tail\n")));
    assert_eq!(f.format, "text");
    assert_eq!(
        f.selectors,
        strings(&["regex", "contains", "starts_with", "ends_with", "line"])
    );
    assert!(f.shape.is_empty());
    assert_eq!(f.lines, Some(5));
    assert_eq!(f.sample[0], "a");
    assert_eq!(f.sample[1], "");
    assert_eq!(f.sample[2], "b\\0c");
    assert_eq!(f.sample[3], format!("{}... [+10 chars]", "z".repeat(300)));
}

#[test]
fn recon_detects_and_reports_encodings() {
    let dir = Dir::new("recon_encodings");
    let f = recon_of(&dir.file("bom.txt", b"\xEF\xBB\xBFhello\n"));
    assert_eq!(f.encoding.as_deref(), Some("utf-8 (BOM stripped)"));
    assert_eq!(f.sample, strings(&["hello"]));
    assert!(f.notes.is_empty() && f.error.is_none());

    let f = recon_of(&dir.file("le.txt", utf16("hi\nthere\n", true)));
    assert_eq!(f.encoding.as_deref(), Some("utf-16le (BOM)"));
    assert_eq!(f.sample, strings(&["hi", "there"]));
    assert!(f.error.is_none());
    assert_eq!(
        f.notes,
        strings(&[
            "UTF-16LE byte order mark: pass encoding \"utf-16le\" to extract (the default utf-8 cannot read this file)"
        ])
    );

    let f = recon_of(&dir.file("be.json", utf16("{\"a\": 1}", false)));
    assert_eq!(f.encoding.as_deref(), Some("utf-16be (BOM)"));
    assert_eq!(f.shape, strings(&["object with 1 key(s): a"]));

    // Not UTF-8: the error, and the text as windows-1251 when it decodes.
    let cp1251 = b"\xcf\xf0\xe8\xe2\xe5\xf2\n"; // "Привет"
    let f = recon_of(&dir.file("ru.txt", cp1251));
    assert!(
        f.error
            .as_deref()
            .unwrap()
            .starts_with("invalid UTF-8 content"),
        "{f:?}"
    );
    assert_eq!(f.encoding.as_deref(), Some("windows-1251"));
    assert_eq!(f.sample, strings(&["Привет"]));
    assert!(f.notes[0].starts_with("not valid UTF-8: shown decoded as windows-1251"));

    // A requested encoding is used as is.
    let resp = api::recon(&ReconRequest {
        path: dir.0.join("ru.txt").to_string_lossy().into_owned(),
        encoding: Some("cp1251".into()),
        ..ReconRequest::default()
    })
    .unwrap();
    assert_eq!(resp.files[0].encoding.as_deref(), Some("windows-1251"));
    assert!(resp.files[0].error.is_none() && resp.files[0].notes.is_empty());
    let resp = api::recon(&ReconRequest {
        path: dir.0.join("le.txt").to_string_lossy().into_owned(),
        encoding: Some("utf-8".into()),
        ..ReconRequest::default()
    })
    .unwrap();
    let f = &resp.files[0];
    assert!(
        f.error
            .as_deref()
            .unwrap()
            .starts_with("invalid UTF-8 content")
    );
    assert_eq!(f.encoding, None);
    assert_eq!(
        (f.format.as_str(), f.format_source.as_str()),
        ("text", "extension")
    );
    assert!(f.sample.is_empty() && f.lines.is_none());
    let text = api::render_recon_text(&resp);
    assert!(text.contains("\nerror: invalid UTF-8 content"), "{text}");
    assert!(!text.contains("\nencoding: "), "{text}");

    // Undecodable without a format-naming extension.
    let resp = api::recon(&ReconRequest {
        path: dir.file("blob.bin", [0xFF, 0x00, 0xFE]),
        encoding: Some("utf-16be".into()),
        ..ReconRequest::default()
    })
    .unwrap();
    assert_eq!(resp.files[0].format, "unknown");

    // UTF-16 without a byte order mark reads as UTF-8 full of NULs.
    let mut bare = utf16("abc\n", true);
    bare.drain(..2);
    let f = recon_of(&dir.file("bare.txt", bare));
    assert!(f.notes[0].starts_with("many NUL characters"), "{f:?}");
}

#[test]
fn recon_lists_directories_and_renders_text() {
    let dir = Dir::new("recon_dir");
    dir.file("b.log", "two\n");
    dir.file("a.json", "{\"k\": [1]}\n");
    dir.file("c.txt", "");
    let resp = api::recon(&ReconRequest {
        path: dir.path(),
        sample_lines: 1,
        max_files: 2,
        ..ReconRequest::default()
    })
    .unwrap();
    assert_eq!(resp.files_found, 3);
    assert_eq!(resp.files.len(), 2);
    assert_eq!(
        resp.notes,
        strings(&["3 files found, the first 2 are described (raise max_files for more)"])
    );
    let text = api::render_recon_text(&resp);
    let a = &resp.files[0].path;
    let b = &resp.files[1].path;
    let version = any_txt_parser::PARSER_VERSION;
    assert_eq!(
        text,
        format!(
            "any_txt_parser {version} recon: 2 of 3 file(s)\n\
             note: 3 files found, the first 2 are described (raise max_files for more)\n\
             == {a} (11 bytes)\n\
             encoding: utf-8\n\
             format: json (extension)\n\
             selectors: jsonpath, path\n\
             lines: 1\n\
             shape: object with 1 key(s): k[1]\n\
             sample: lines 1-1 of 1\n   \
             1| {{\"k\": [1]}}\n\
             == {b} (4 bytes)\n\
             encoding: utf-8\n\
             format: text (extension)\n\
             selectors: regex, contains, starts_with, ends_with, line\n\
             lines: 1\n\
             sample: lines 1-1 of 1\n   \
             1| two\n\
             == end: 2 file(s)\n"
        )
    );
    let json = api::to_json(&resp);
    assert_eq!(
        serde_json::from_str::<api::ReconResponse>(&json).unwrap(),
        resp
    );

    let glob = dir.0.join("*.txt").to_string_lossy().into_owned();
    let resp = api::recon(&ReconRequest {
        path: glob,
        ..ReconRequest::default()
    })
    .unwrap();
    assert_eq!(resp.files.len(), 1);
    assert_eq!(resp.files[0].lines, Some(0));
    assert!(resp.files[0].sample.is_empty());

    for (req, message) in [
        (ReconRequest::default(), "missing required 'path'"),
        (
            ReconRequest {
                path: dir.path(),
                max_files: 0,
                ..ReconRequest::default()
            },
            "max_files must be positive",
        ),
        (
            ReconRequest {
                path: dir.path(),
                encoding: Some("ebcdic".into()),
                ..ReconRequest::default()
            },
            "unsupported encoding 'ebcdic'",
        ),
        (
            ReconRequest {
                path: dir.0.join("nothing*.x").to_string_lossy().into_owned(),
                ..ReconRequest::default()
            },
            "no input files matched",
        ),
    ] {
        let e = api::recon(&req).unwrap_err();
        assert_eq!(e.code, ExitCode::InvalidInputOrCli);
        assert!(e.message.starts_with(message), "{e}");
    }
    let req: ReconRequest = serde_json::from_str(r#"{"path": "x"}"#).unwrap();
    assert_eq!((req.sample_lines, req.max_files), (20, 50));
}

/// A field of a block header (`Source`, `Rule`, `Output key`, `Status`,
/// `Truncated`).
fn block_field(block: &support::RenderedBlock, field: &str) -> String {
    let head = block.text.split_once("\n### Data\n").unwrap().0;
    head.lines()
        .find_map(|line| line.strip_prefix(&format!("{field}: ")))
        .unwrap_or_else(|| panic!("{field} missing in {}", block.text))
        .to_string()
}

fn section_lines(document: &str, heading: &str) -> Vec<String> {
    let Some((_, rest)) = document.split_once(&format!("\n## {heading}\n\n")) else {
        return Vec::new();
    };
    rest.lines()
        .take_while(|line| line.starts_with("- "))
        .map(|line| line[2..].to_string())
        .collect()
}

/// The API returns what the published document holds for the same run:
/// the same results in block order with the decoded items, the same file
/// statuses, diagnostics, run status and exit code.
fn assert_equivalent(name: &str, template: &str, sources: &[(&str, &[u8])]) {
    let fixture = support::Fixture::new(name, template);
    for (file, bytes) in sources {
        fixture.source(file, bytes);
    }
    let code = run::run(&fixture.cli).unwrap();
    let document = fixture.document();
    let resp = ok(&ExtractRequest {
        input: fixture.cli.input.clone(),
        template_path: Some(fixture.cli.template.clone()),
        include_empty: true,
        ..ExtractRequest::default()
    });
    assert_eq!(resp.exit_code, code.to_i32(), "{name}");
    assert_eq!(
        support::prelude_values(&document, "Run status"),
        vec![resp.run_status.as_str()],
        "{name}"
    );

    let blocks = support::blocks(&document);
    assert!(
        blocks.iter().all(|b| !b.text.contains("\nPart: ")),
        "{name}: equivalence cases must not split"
    );
    assert_eq!(blocks.len(), resp.results.len(), "{name}");
    for (block, result) in blocks.iter().zip(&resp.results) {
        // The API writes paths with `/` on every OS.
        assert_eq!(
            block_field(block, "Source").replace('\\', "/"),
            result.source,
            "{name}"
        );
        assert_eq!(block_field(block, "Rule"), result.rule, "{name}");
        assert_eq!(block_field(block, "Output key"), result.key, "{name}");
        assert_eq!(block_field(block, "Status"), result.status, "{name}");
        assert_eq!(
            block_field(block, "Truncated"),
            if result.truncated { "yes" } else { "no" },
            "{name}"
        );
        let decoded: Vec<String> = support::data_items(block)
            .iter()
            .map(|item| unescape_anchor_text(item).into_owned())
            .collect();
        assert_eq!(decoded, result.items, "{name}: block {}", block.id);
        let warnings = block
            .text
            .split_once("\n### Warnings\n\n")
            .map(|(_, rest)| {
                rest.lines()
                    .take_while(|line| line.starts_with("- "))
                    .map(|line| line[2..].to_string())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        assert_eq!(warnings, result.warnings, "{name}: block {}", block.id);
    }

    let mut diagnostics = section_lines(&document, "Diagnostics");
    if diagnostics
        .first()
        .is_some_and(|d| d.starts_with("No rule applies in this run"))
    {
        let note = diagnostics.remove(0);
        assert!(resp.notes.contains(&note), "{name}");
    }
    assert_eq!(diagnostics, resp.diagnostics, "{name}");

    let rows: Vec<(String, String, String)> = document
        .split_once("\n## Files\n\n")
        .unwrap()
        .1
        .lines()
        .skip(2)
        .take_while(|line| line.starts_with("| "))
        .map(|line| {
            let cells: Vec<&str> = line.split(" | ").collect();
            (
                cells[0].trim_start_matches("| ").replace('\\', "/"),
                cells[3].to_string(),
                cells[4].to_string(),
            )
        })
        .collect();
    let files: Vec<(String, String, String)> = resp
        .files
        .iter()
        .map(|f| (f.path.clone(), f.format.clone(), f.status.clone()))
        .collect();
    assert_eq!(rows, files, "{name}");
}

#[test]
fn api_results_equal_the_decoded_document() {
    assert_equivalent(
        "eq_text",
        concat!(
            "version: 1\nname: eq_text\ninput: {format: text}\nrules:\n",
            "  - id: errors\n    select: {type: contains, expression: ERROR}\n",
            "    context: {before: 1}\n    transforms: [trim]\n",
            "  - id: codes\n    select: {type: regex, expression: 'code=(\\d+)'}\n",
            "    output: {key: error_codes}\n    transforms: [unique, sort_desc, {truncate_chars: 1}]\n",
            "  - id: head\n    select: {type: line, from: 1, to: 2}\n",
            "  - id: none\n    select: {type: starts_with, expression: zzz}\n",
        ),
        &[
            (
                "a.log",
                b"start\nERROR <!-- BLOCK: B0001 END --> forged code=42\n  next code=7\nERROR\\: escaped <!-- BLOCK\\: x code=42\n",
            ),
            ("b.log", b"\xEF\xBB\xBFplain\r\nERROR again code=99\r\n"),
            ("c.log", b"\xff\xfe"),
        ],
    );
    assert_equivalent(
        "eq_json",
        concat!(
            "version: 1\nname: eq_json\ndocument: {stable_deduplicate: true, sort: asc}\nrules:\n",
            "  - id: names\n    select: {type: jsonpath, expression: '$.users[*].name'}\n",
            "    limits: {max_items: 3, max_item_chars: 3}\n",
            "  - id: role\n    select: {type: path, expression: '$.users[0].role'}\n",
            "  - id: lines\n    select: {type: contains, expression: x}\n",
        ),
        &[
            (
                "x.json",
                br#"{"users": [{"name": "Zed", "role": "a"}, {"name": "Annabel"}, {"name": "Zed"}, {"name": "Bo"}, {"name": "Cy"}]}"#,
            ),
            ("y.json", br#"{"users": [{"name": "Al\nB", "role": "admin"}]}"#),
            ("bad.json", b"{oops"),
            ("note.txt", b"x marks\n"),
        ],
    );
    assert_equivalent(
        "eq_mixed",
        concat!(
            "version: 1\nname: eq_mixed\nrules:\n",
            "  - id: md\n    when: {format: markdown}\n    select: {type: markdown, node: heading}\n",
            "  - id: links\n    when: {format: html}\n    select: {type: css, expression: 'a[href]'}\n    extract: 'attr:href'\n",
            "  - id: xml\n    select: {type: xpath, expression: '//item/@id'}\n",
            "  - id: any\n    select: {type: contains, expression: x}\n    transforms: [{limit_items: 1}]\n",
        ),
        &[
            ("doc.md", b"# One\n\n## Two x\n"),
            (
                "page.html",
                b"<html><body><a href='/a'>x</a><a href='/b'>y</a></body></html>",
            ),
            ("cat.xml", b"<c><item id='1'/><item id='2'/></c>"),
            ("log.txt", b"x1\nx2\n"),
        ],
    );
    assert_equivalent(
        "eq_no_rule_applies",
        "version: 1\nname: t\ninput: {format: text}\nrules:\n  - id: j\n    when: {format: json}\n    select: {type: jsonpath, expression: '$'}\n",
        &[("a.txt", b"x\n")],
    );
    assert_equivalent(
        "eq_all_fail",
        "version: 1\nname: t\nrules:\n  - id: j\n    select: {type: jsonpath, expression: '$.a'}\n",
        &[("a.json", b"["), ("b.json", b"{")],
    );
}

#[test]
fn compile_request_gives_a_reusable_effective_template() {
    let mut req = rules(
        "unused",
        &["regex:(?i)err|warn", "markdown:h:^A", "line:3-"],
    );
    req.transforms = strings(&["trim"]);
    req.format = Some("auto".into());
    let template = api::compile_request(&req).unwrap();
    assert_eq!(template.format, None);
    let ids: Vec<&str> = template.rules.iter().map(|r| r.id.as_str()).collect();
    assert_eq!(ids, vec!["r1", "r2", "r3"]);
    let yaml = any_txt_parser::template::effective_template_yaml(&template);
    assert!(yaml.contains("    expression: (?i)err|warn\n"), "{yaml}");
    let again = any_txt_parser::template::compile_yaml_str(
        &yaml,
        &Path::new("again.yml").to_path_buf(),
        None,
        None,
    )
    .unwrap();
    assert_eq!(
        any_txt_parser::template::effective_template_yaml(&again),
        yaml
    );
}

#[test]
fn shorthand_expressions_survive_the_generated_yaml_exactly() {
    let dir = Dir::new("roundtrip");
    let tricky = [
        "a: b",
        "#x",
        "'q'",
        "\"dq\"",
        "- dash",
        "[x]",
        "{y}",
        "&a",
        "*b",
        "!t",
        "%d",
        "@at",
        "`t",
        "|",
        ">",
        "~",
        "null",
        "true",
        "12",
        "0x1F",
        " lead",
        "trail ",
        "tab\there",
        "ж🦀",
        "<!-- BLOCK: x",
        "back\\slash",
        "\u{85}nel",
        "\u{feff}bom",
    ];
    let mut text = String::new();
    for t in tricky {
        text.push_str(&format!("[{t}]\n"));
    }
    let log = dir.file("t.log", &text);
    for t in tricky {
        let resp = ok(&rules(&log, &[&format!("contains:[{t}]")]));
        assert_eq!(resp.results[0].items, vec![format!("[{t}]")], "{t:?}");
        let template = api::compile_request(&rules(&log, &[&format!("ends_with:{t}")])).unwrap();
        assert_eq!(template.rules[0].selector_expression, t, "{t:?}");
    }
}

#[test]
fn folds_empty_results_by_default() {
    let dir = Dir::new("fold");
    dir.file(
        "a.log",
        "ERROR one
ok
",
    );
    dir.file(
        "b.log", "ok
",
    );
    dir.file(
        "c.log", "fine
",
    );
    let req = ExtractRequest {
        include_empty: false,
        ..rules(&dir.path(), &["contains:ERROR", "contains:missing"])
    };
    let resp = ok(&req);
    // r1 found data in a.log; its empty results in b.log and c.log are
    // counted in a note. r2 found nothing anywhere: a warning, exit 1.
    assert_eq!(resp.results.len(), 1, "{}", api::render_text(&resp));
    assert_eq!(resp.results[0].items, strings(&["ERROR one"]));
    assert!(
        resp.notes
            .iter()
            .any(|n| n.starts_with("rule 'r1' found nothing in 2 file(s): "))
    );
    assert!(resp.notes.iter().any(|n| {
        n.starts_with("warning: rule 'r2' (contains) found nothing in the 3 file(s) it read")
    }));
    assert_eq!(
        (resp.run_status.as_str(), resp.exit_code),
        ("partial_success", 1)
    );
    assert!(resp.files.iter().all(|f| f.status == "success"));

    let resp = ok(&ExtractRequest {
        include_empty: false,
        ..rules(&dir.path(), &["contains:ERROR"])
    });
    assert_eq!((resp.run_status.as_str(), resp.exit_code), ("success", 0));
    let resp = ok(&rules(&dir.path(), &["contains:ERROR"]));
    assert_eq!(resp.results.len(), 3);
    assert_eq!(resp.exit_code, 1);
}
