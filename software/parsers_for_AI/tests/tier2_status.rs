//! Statuses, truncation reporting and exit codes (tier-2 review, lane
//! "status"). Each test reproduces an input that was misreported before.

mod support;

use any_txt_parser::limits::ExitCode;
use any_txt_parser::run;
use std::process::Command;
use support::Fixture;

/// The block produced by `rule` for the source named `source`.
fn block_of(document: &str, source: &str, rule: &str) -> support::RenderedBlock {
    support::blocks(document)
        .into_iter()
        .find(|b| {
            b.text.contains(&format!("Rule: {}\n", rule))
                && b.text
                    .lines()
                    .any(|line| line.starts_with("Source: ") && line.ends_with(source))
        })
        .unwrap_or_else(|| panic!("no block for {source} / {rule}"))
}

fn warnings(block: &support::RenderedBlock) -> Vec<String> {
    match block.text.split_once("\n### Warnings\n\n") {
        None => Vec::new(),
        Some((_, rest)) => rest
            .lines()
            .map_while(|line| line.strip_prefix("- "))
            .map(str::to_string)
            .collect(),
    }
}

fn file_row<'a>(document: &'a str, name: &str) -> &'a str {
    document
        .lines()
        .find(|line| line.starts_with("| ") && line.contains(&format!("{} |", name)))
        .unwrap_or_else(|| panic!("no Files row for {name}"))
}

fn section<'a>(document: &'a str, heading: &str) -> &'a str {
    document
        .split_once(&format!("\n## {}\n\n", heading))
        .unwrap_or_else(|| panic!("section {heading} missing"))
        .1
        .split_once("\n## ")
        .unwrap()
        .0
}

fn run_binary(fixture: &Fixture, extra: &[&str]) -> (Option<i32>, String) {
    let (code, _, stderr) = run_binary_output(fixture, extra);
    (code, stderr)
}

/// Exit code, stdout and stderr of the real binary.
fn run_binary_output(fixture: &Fixture, extra: &[&str]) -> (Option<i32>, String, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_any_txt_parser"))
        .arg("-i")
        .arg(&fixture.cli.input)
        .arg("-t")
        .arg(&fixture.cli.template)
        .arg("-o")
        .arg(fixture.cli.output.as_ref().unwrap())
        .arg("-q")
        .args(extra)
        .output()
        .unwrap();
    (
        output.status.code(),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

// Defect 1: truncate_chars, truncate_lines and limit_items lost data while
// the block said "Truncated: no" and the run exited 0.
#[test]
fn lossy_transforms_are_reported_as_truncation() {
    let template = r#"version: 1
name: lossy
input: {format: json}
rules:
  - id: chars
    select: {type: jsonpath, expression: "$.values[*]"}
    output: {key: chars}
    transforms: [{truncate_chars: 5}]
  - id: lines
    select: {type: jsonpath, expression: "$.values[*]"}
    output: {key: lines}
    transforms: [{truncate_lines: 2}]
  - id: limited
    select: {type: jsonpath, expression: "$.values[*]"}
    output: {key: limited}
    transforms: [{limit_items: 1}]
  - id: lossless
    select: {type: jsonpath, expression: "$.values[*]"}
    output: {key: lossless}
    transforms: [{truncate_chars: 100}, {truncate_lines: 10}, {limit_items: 10}]
"#;
    let fixture = Fixture::new("lossy_transforms", template);
    fixture.source(
        "data.json",
        r#"{"values": ["a\nb\nc", "short", "1234567890"]}"#,
    );

    assert_eq!(run::run(&fixture.cli).unwrap(), ExitCode::PartialSuccess);
    let document = fixture.document();
    support::assert_index_lines(&document);
    support::assert_total_characters(&document);

    let chars = block_of(&document, "data.json", "chars");
    assert!(chars.text.contains("Truncated: yes\n"));
    assert_eq!(
        warnings(&chars),
        vec!["1 item(s) truncated by truncate_chars(5): item 3 (10 -> 5 chars)"]
    );
    assert_eq!(
        support::data_items(&chars),
        vec!["a\nb\nc", "short", "12345"]
    );

    let lines = block_of(&document, "data.json", "lines");
    assert!(lines.text.contains("Truncated: yes\n"));
    assert_eq!(
        warnings(&lines),
        vec!["1 item(s) truncated by truncate_lines(2): item 1 (3 -> 2 lines)"]
    );
    assert_eq!(support::data_items(&lines)[0], "a\nb");

    let limited = block_of(&document, "data.json", "limited");
    assert!(limited.text.contains("Truncated: yes\n"));
    assert_eq!(
        warnings(&limited),
        vec!["2 item(s) omitted by limit_items(1)"]
    );

    // Limits that cut nothing stay silent.
    let lossless = block_of(&document, "data.json", "lossless");
    assert!(lossless.text.contains("Truncated: no\n"));
    assert!(warnings(&lossless).is_empty());

    // Same precedent as max_item_chars: the file is a warning, the run
    // partial, and the Data Contract lists every loss.
    assert!(file_row(&document, "data.json").contains("| json | warning |"));
    assert!(document.contains("Run status: partial_success\n"));
    let truncation = section(&document, "Truncation");
    for rule in ["chars", "lines", "limited"] {
        assert!(
            truncation.contains(&format!("data.json [{}]: ", rule)),
            "{truncation}"
        );
    }
    assert!(!truncation.contains("[lossless]"));
}

#[test]
fn transform_cuts_name_published_items_after_later_transforms_and_limits() {
    let template = r#"version: 1
name: tracked
input: {format: text}
rules:
  - id: reordered
    select: {type: regex, expression: ".+"}
    output: {key: reordered}
    transforms: [{truncate_chars: 3}, unique, sort_desc]
    limits: {max_items: 2}
  - id: joined
    select: {type: regex, expression: ".+"}
    output: {key: joined}
    transforms: [{truncate_chars: 3}, drop_empty, {join: "|"}]
  - id: joined_then_cut
    select: {type: regex, expression: ".+"}
    output: {key: joined_then_cut}
    transforms: [{truncate_chars: 3}, {limit_items: 4}, {join: "|"}, {truncate_chars: 4}]
"#;
    let fixture = Fixture::new("tracked_cuts", template);
    fixture.source("a.log", "zzzzzzzz\na\nbbbbbbbb\na\ncccccccc\n");

    assert_eq!(run::run(&fixture.cli).unwrap(), ExitCode::PartialSuccess);
    let document = fixture.document();

    // zzz, a, bbb, a, ccc -> unique -> sort_desc: zzz, ccc, bbb, a -> max_items 2.
    let reordered = block_of(&document, "a.log", "reordered");
    assert_eq!(support::data_items(&reordered), vec!["zzz", "ccc"]);
    assert_eq!(
        warnings(&reordered),
        vec![
            "2 item(s) omitted after max_items limit",
            "3 item(s) truncated by truncate_chars(3): item 1 (8 -> 3 chars), item 2 (8 -> 3 chars); 1 not published",
        ]
    );

    // A joined item is named once, with the number of cut pieces in it.
    let joined = block_of(&document, "a.log", "joined");
    assert_eq!(support::data_items(&joined), vec!["zzz|a|bbb|a|ccc"]);
    assert_eq!(
        warnings(&joined),
        vec!["3 item(s) truncated by truncate_chars(3): item 1 (3 joined items cut to 3 chars)"]
    );

    // zzz, a, bbb, a, ccc -> limit_items 4 -> "zzz|a|bbb|a" (11) -> 4 chars.
    let joined_then_cut = block_of(&document, "a.log", "joined_then_cut");
    assert_eq!(support::data_items(&joined_then_cut), vec!["zzz|"]);
    assert_eq!(
        warnings(&joined_then_cut),
        vec![
            "3 item(s) truncated by truncate_chars(3): item 1 (2 joined items cut to 3 chars); 1 not published",
            "1 item(s) omitted by limit_items(4)",
            "1 item(s) truncated by truncate_chars(4): item 1 (11 -> 4 chars)",
        ]
    );
    for block in [&joined, &joined_then_cut] {
        let line = &warnings(block)[0];
        assert_eq!(line.matches("item 1 (").count(), 1, "{line}");
    }
}

#[test]
fn transform_cut_numbers_follow_stable_deduplicate_and_stay_compact() {
    let template = r#"version: 1
name: dedup
input: {format: text}
document: {stable_deduplicate: true}
rules:
  - id: r
    select: {type: regex, expression: "^[a-z].*"}
    output: {key: values}
    transforms: [{truncate_chars: 3}]
  - id: many
    select: {type: starts_with, expression: "LONG"}
    output: {key: many}
    transforms: [{truncate_chars: 6}]
"#;
    let fixture = Fixture::new("dedup_cuts", template);
    let mut lines = vec!["abcX".to_string(), "abcY".to_string(), "zz".to_string()];
    lines.extend((0..1500).map(|i| format!("LONG line {i:04}")));
    fixture.source("a.log", lines.join("\n"));

    assert_eq!(run::run(&fixture.cli).unwrap(), ExitCode::PartialSuccess);
    let document = fixture.document();
    let r = block_of(&document, "a.log", "r");
    assert_eq!(support::data_items(&r), vec!["abc", "zz"]);
    assert_eq!(
        warnings(&r),
        vec!["2 item(s) truncated by truncate_chars(3): item 1 (4 -> 3 chars); 1 not published"]
    );
    // 1500 cut items that deduplicate to one: one line, not 1500.
    let many = block_of(&document, "a.log", "many");
    assert_eq!(support::data_items(&many), vec!["LONG l"]);
    assert_eq!(
        warnings(&many),
        vec![
            "1500 item(s) truncated by truncate_chars(6): item 1 (14 -> 6 chars); 1499 not published"
        ]
    );
}

const MIXED_TEMPLATE: &str = r#"version: 1
name: mixed
rules:
  - id: json_names
    when: {format: json}
    select: {type: jsonpath, expression: "$.name"}
    output: {key: names}
  - id: log_errors
    when: {format: text}
    select: {type: contains, expression: "ERROR"}
    output: {key: errors}
"#;

// Defect 2a: every file whose format a rule's when.format excluded became
// "warning" with a "rule not applicable" block, so a multi-format template
// could never exit 0 (spec §68).
#[test]
fn when_format_filters_rules_without_blocks_or_warnings() {
    let fixture = Fixture::new("when_format_mixed", MIXED_TEMPLATE);
    fixture.source("a.json", r#"{"name": "alpha"}"#);
    fixture.source("b.log", "ok\nERROR disk full\n");

    assert_eq!(run::run(&fixture.cli).unwrap(), ExitCode::Success);
    let document = fixture.document();
    let blocks = support::blocks(&document);
    assert_eq!(
        blocks.len(),
        2,
        "one block per applicable (file, rule) pair"
    );
    assert_eq!(
        support::data_items(&block_of(&document, "a.json", "json_names")),
        vec!["alpha"]
    );
    assert_eq!(
        support::data_items(&block_of(&document, "b.log", "log_errors")),
        vec!["ERROR disk full"]
    );
    assert!(!document.contains("rule not applicable\n"));
    assert!(!document.contains("File status:"));
    assert!(file_row(&document, "a.json").contains("| json | success |"));
    assert!(file_row(&document, "b.log").contains("| text | success |"));
    assert!(document.contains("Run status: success\n"));
    assert!(document.contains("Applies to format: json\n"));
    assert!(document.contains("Applies to format: text\n"));
    assert!(section(&document, "Result Semantics").contains(
        "A rule with \"Applies to format\" runs only on source files of that format; other files get no block for it, and their status is not affected.\n"
    ));
    support::assert_index_lines(&document);
    support::assert_total_characters(&document);

    // Without when.format, a selector that does not fit the format is still
    // "rule not applicable" and a warning (spec §69).
    let template = MIXED_TEMPLATE.replace("    when: {format: text}\n", "");
    let fixture = Fixture::new("selector_mismatch", &template);
    fixture.source("a.json", r#"{"name": "alpha"}"#);
    fixture.source("b.log", "ERROR\n");
    assert_eq!(run::run(&fixture.cli).unwrap(), ExitCode::PartialSuccess);
    let document = fixture.document();
    let mismatch = block_of(&document, "a.json", "log_errors");
    assert!(mismatch.text.contains("Status: rule not applicable\n"));
    assert!(mismatch.text.contains("File status: warning\n"));
    assert!(file_row(&document, "a.json").contains("| json | warning |"));
    assert!(file_row(&document, "b.log").contains("| text | success |"));
}

// Defect 2b: --format that differs from a rule's when.format rejected the
// template with exit 2, although the template itself is valid.
#[test]
fn cli_format_contradicting_when_format_skips_the_rule_instead_of_exit_2() {
    let mut fixture = Fixture::new("when_format_cli", MIXED_TEMPLATE);
    fixture.source("notes.md", "intro\nERROR in notes\n");
    fixture.cli.format = Some("text".into());

    assert_eq!(run::run(&fixture.cli).unwrap(), ExitCode::Success);
    let document = fixture.document();
    let blocks = support::blocks(&document);
    assert_eq!(blocks.len(), 1);
    assert_eq!(
        support::data_items(&block_of(&document, "notes.md", "log_errors")),
        vec!["ERROR in notes"]
    );
    let applied = section(&document, "Applied Template");
    assert!(applied.contains(
        "Applies to format: json (never applies in this run: every file is processed as text, format source: CLI override)\n"
    ), "{applied}");
    assert!(applied.contains("Applies to format: text\n"));

    // The same through the binary, with a template of the documented shape:
    // input.format fixed to json, rule restricted to json.
    let template = "version: 1\nname: users\ninput: {format: json}\nrules:\n  - id: active_names\n    when: {format: json}\n    select: {type: jsonpath, expression: \"$.name\"}\n    output: {key: names}\n  - id: lines\n    select: {type: contains, expression: \"name\"}\n    output: {key: lines}\n";
    let fixture = Fixture::new("when_format_cli_bin", template);
    fixture.source("in.json", "{\"name\": \"alpha\"}\n");
    let (code, stderr) = run_binary(&fixture, &["--format", "text"]);
    assert_eq!(code, Some(0), "{stderr}");
    let document = fixture.document();
    assert_eq!(support::blocks(&document).len(), 1);
    assert!(document.contains(
        "Applies to format: json (never applies in this run: every file is processed as text, format source: CLI override)\n"
    ));
    // The dry-run plan validates the same combination and names the rule.
    let (code, stdout, stderr) = run_binary_output(&fixture, &["--format", "text", "--dry-run"]);
    assert_eq!(code, Some(0), "{stderr}");
    assert!(
        stdout.contains(
            "Rule 'active_names' never applies: when.format json does not match the input format text (format source: CLI override)\n"
        ),
        "{stdout}"
    );
    assert!(!stdout.contains("No rule applies"), "{stdout}");
}

/// The YAML published under `## Effective Template`.
fn effective_template(document: &str) -> &str {
    section(document, "Effective Template")
        .strip_prefix("```yaml\n")
        .and_then(|yaml| yaml.split_once("```\n"))
        .map(|(yaml, _)| yaml)
        .expect("Effective Template is not a yaml code block")
}

// Review I1: a --format run publishes an Effective Template with
// `input.format: text` next to `when.format: json`; reusing it as a template
// must not fail with exit 2 (spec §9, §11; HELP: reuse the Effective
// Template YAML as a template).
#[test]
fn effective_template_of_a_format_override_run_is_reusable() {
    let mut fixture = Fixture::new("effective_reuse_source", MIXED_TEMPLATE);
    fixture.source("notes.md", "intro\nERROR in notes\n");
    fixture.cli.format = Some("text".into());
    assert_eq!(run::run(&fixture.cli).unwrap(), ExitCode::Success);
    let document = fixture.document();
    let yaml = effective_template(&document);
    assert!(yaml.contains("format: text\n"), "{yaml}");
    assert!(yaml.contains("format: json\n"), "{yaml}");

    for cli_format in [None, Some("text")] {
        let mut reuse = Fixture::new("effective_reuse", yaml);
        reuse.source("notes.md", "intro\nERROR in notes\n");
        reuse.cli.format = cli_format.map(str::to_string);
        assert_eq!(
            run::run(&reuse.cli).map_err(|e| e.message),
            Ok(ExitCode::Success),
            "--format {cli_format:?}"
        );
        let again = reuse.document();
        assert_eq!(
            support::data_items(&block_of(&again, "notes.md", "log_errors")),
            vec!["ERROR in notes"]
        );
        assert_eq!(support::blocks(&again).len(), 1);
        let source = cli_format.map_or("template", |_| "CLI override");
        assert!(again.contains(&format!(
            "Applies to format: json (never applies in this run: every file is processed as text, format source: {source})\n"
        )));
        // The reused Effective Template describes the same configuration.
        assert_eq!(effective_template(&again), yaml);
    }

    // The same shape written by hand: a rule restricted to a format other
    // than the template's own input.format never applies and says so.
    let template = "version: 1\nname: users\ninput: {format: json}\nrules:\n  - id: log_only\n    when: {format: text}\n    select: {type: contains, expression: \"name\"}\n    output: {key: lines}\n  - id: names\n    select: {type: jsonpath, expression: \"$.name\"}\n    output: {key: names}\n";
    let fixture = Fixture::new("when_format_template", template);
    fixture.source("in.json", "{\"name\": \"alpha\"}\n");
    assert_eq!(run::run(&fixture.cli).unwrap(), ExitCode::Success);
    let document = fixture.document();
    assert_eq!(support::blocks(&document).len(), 1);
    assert!(document.contains(
        "Applies to format: text (never applies in this run: every file is processed as json, format source: template)\n"
    ));
    let (code, stdout, stderr) = run_binary_output(&fixture, &["--dry-run"]);
    assert_eq!(code, Some(0), "{stderr}");
    assert!(
        stdout.contains(
            "Rule 'log_only' never applies: when.format text does not match the input format json (format source: template)\n"
        ),
        "{stdout}"
    );
}

/// Exit code and stderr of the real binary run without `-q`.
fn run_binary_loud(fixture: &Fixture, extra: &[&str]) -> (Option<i32>, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_any_txt_parser"))
        .arg("-i")
        .arg(&fixture.cli.input)
        .arg("-t")
        .arg(&fixture.cli.template)
        .arg("-o")
        .arg(fixture.cli.output.as_ref().unwrap())
        .args(extra)
        .output()
        .unwrap();
    (
        output.status.code(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

// Review I3: when the fixed input format excludes every rule by its
// when.format, nothing can be extracted; that run must not report success
// with exit 0 and an empty document (spec §65, §69).
#[test]
fn a_run_in_which_no_rule_applies_is_not_a_success() {
    const NOTE: &str =
        "No rule applies in this run: every rule's when.format differs from the input format text";
    let json_only = "version: 1\nname: json_only\nrules:\n  - id: names\n    when: {format: json}\n    select: {type: jsonpath, expression: \"$.name\"}\n    output: {key: names}\n";
    let mut fixture = Fixture::new("no_rule_applies", json_only);
    fixture.source("a.json", r#"{"name": "alpha"}"#);
    fixture.source("b.log", "plain\n");
    fixture.cli.format = Some("text".into());
    assert_eq!(run::run(&fixture.cli).unwrap(), ExitCode::PartialSuccess);
    let document = fixture.document();
    assert!(support::blocks(&document).is_empty());
    assert!(document.contains("Run status: partial_success\n"));
    assert!(document.contains("Files warnings: 2\n"));
    assert!(file_row(&document, "a.json").contains("| text | warning |"));
    assert!(file_row(&document, "b.log").contains("| text | warning |"));
    assert!(
        section(&document, "Diagnostics").contains(&format!(
            "- {NOTE} (format source: CLI override); nothing was extracted\n"
        )),
        "{document}"
    );
    assert!(section(&document, "Result Semantics").contains(
        "In this run no rule applies to the input format, so nothing was extracted and every source file that was read is a warning (see Diagnostics).\n"
    ));
    support::assert_index_lines(&document);

    let (code, stderr) = run_binary_loud(&fixture, &["--format", "text"]);
    assert_eq!(code, Some(1), "{stderr}");
    assert!(
        stderr.contains(&format!("any_txt_parser: warning: {NOTE}")),
        "{stderr}"
    );
    let (_, stdout, _) = run_binary_output(&fixture, &["--format", "text", "--dry-run"]);
    assert!(stdout.contains(NOTE), "{stdout}");

    // The same when the template fixes the format itself; a file that fails
    // to read stays an error.
    let fixed = json_only.replace("rules:\n", "input: {format: text}\nrules:\n");
    let fixture = Fixture::new("no_rule_applies_template", &fixed);
    fixture.source("a.log", "plain\n");
    fixture.source("bad.log", b"plain \xff\n");
    assert_eq!(run::run(&fixture.cli).unwrap(), ExitCode::PartialSuccess);
    let document = fixture.document();
    assert!(file_row(&document, "a.log").contains("| text | warning |"));
    assert!(file_row(&document, "bad.log").contains("| error |"));
    assert!(section(&document, "Diagnostics").contains(&format!(
        "- {NOTE} (format source: template); nothing was extracted\n"
    )));

    // One rule that still applies keeps the run a plain success.
    let mixed = format!(
        "{json_only}  - id: lines\n    select: {{type: contains, expression: \"plain\"}}\n    output: {{key: lines}}\n"
    );
    let mut fixture = Fixture::new("some_rule_applies", &mixed);
    fixture.source("b.log", "plain\n");
    fixture.cli.format = Some("text".into());
    assert_eq!(run::run(&fixture.cli).unwrap(), ExitCode::Success);
    let document = fixture.document();
    assert!(!document.contains("No rule applies"));
    assert!(!document.contains("no rule applies"));
    assert!(file_row(&document, "b.log").contains("| text | success |"));
}

const JSON_TEMPLATE: &str = "version: 1\nname: json\ninput: {format: json}\nrules:\n  - id: names\n    select: {type: jsonpath, expression: \"$.name\"}\n    output: {key: names}\n";

// Defect 3a: one failed file made the run "failure" although other files
// succeeded; a run in which every file failed exited 1 like a partial one.
#[test]
fn run_status_and_exit_code_agree() {
    let fixture = Fixture::new("one_failed", JSON_TEMPLATE);
    fixture.source("bad.json", "{not json");
    fixture.source("good.json", r#"{"name": "alpha"}"#);
    assert_eq!(run::run(&fixture.cli).unwrap(), ExitCode::PartialSuccess);
    let document = fixture.document();
    assert!(document.contains("Run status: partial_success\n"));
    assert!(document.contains("Files failed: 1\n"));

    let fixture = Fixture::new("all_failed", JSON_TEMPLATE);
    fixture.source("bad1.json", "{not json");
    fixture.source("bad2.json", "[1,");
    let previous = fixture.seed_output();
    assert_eq!(run::run(&fixture.cli).unwrap(), ExitCode::InvalidInputOrCli);
    // The failed run still publishes a document that says why.
    let document = fixture.document();
    assert_ne!(document.as_bytes(), previous.as_slice());
    assert!(document.contains("Run status: failure\n"));
    assert!(document.contains("Files failed: 2\n"));
    let diagnostics = section(&document, "Diagnostics");
    assert!(diagnostics.contains("bad1.json: "), "{diagnostics}");
    assert!(diagnostics.contains("bad2.json: "), "{diagnostics}");
    assert!(support::blocks(&document).is_empty());

    let (code, stderr) = run_binary(&fixture, &[]);
    assert_eq!(code, Some(3), "{stderr}");
    assert!(
        stderr.contains("no input file could be processed"),
        "{stderr}"
    );
    // Seeded generation 9, then the in-process run, then the binary run.
    assert!(fixture.document().contains("Generation: 00000011\n"));
}

const XML_TEMPLATE: &str = "version: 1\nname: xml\ninput: {format: xml}\nrules:\n  - id: ids\n    select: {type: xpath, expression: \"/root/item/@id\"}\n    output: {key: ids}\n  - id: prefixed\n    select: {type: xpath, expression: \"/root/unknown:item\"}\n    output: {key: prefixed}\n";

// Defect 3b: a failing rule marked its whole file "error" (and the run
// "failure") although the other rules of that file published data.
#[test]
fn rule_error_next_to_working_rules_is_a_file_warning() {
    let fixture = Fixture::new("rule_error", XML_TEMPLATE);
    fixture.source("data.xml", r#"<root><item id="a"/><item id="b"/></root>"#);
    assert_eq!(run::run(&fixture.cli).unwrap(), ExitCode::PartialSuccess);
    let document = fixture.document();
    assert!(file_row(&document, "data.xml").contains("| xml | warning |"));
    assert!(document.contains("Files warnings: 1\n"));
    assert!(document.contains("Files failed: 0\n"));
    assert!(document.contains("Run status: partial_success\n"));
    let ids = block_of(&document, "data.xml", "ids");
    assert_eq!(support::data_items(&ids), vec!["a", "b"]);
    assert!(ids.text.contains("File status: warning\n"));
    let prefixed = block_of(&document, "data.xml", "prefixed");
    assert!(prefixed.text.contains("Status: error\n"));
    assert!(
        prefixed
            .text
            .contains("unknown xpath namespace prefix 'unknown'")
    );
    assert!(section(&document, "Diagnostics").contains("data.xml [prefixed]: "));

    // When every rule of the only file fails, nothing was extracted at all.
    let only_failing = XML_TEMPLATE.replace(
        "  - id: ids\n    select: {type: xpath, expression: \"/root/item/@id\"}\n    output: {key: ids}\n",
        "",
    );
    let fixture = Fixture::new("rule_error_only", &only_failing);
    fixture.source("data.xml", r#"<root><item id="a"/></root>"#);
    assert_eq!(run::run(&fixture.cli).unwrap(), ExitCode::InvalidInputOrCli);
    let document = fixture.document();
    assert!(file_row(&document, "data.xml").contains("| xml | error |"));
    assert!(document.contains("Run status: failure\n"));
}

// Review I2: rules that are not applicable to the format did not run, so a
// file whose every rule that ran failed yielded nothing: `error`, and with
// no other file the run is a failure (exit 3), not partial_success.
#[test]
fn inapplicable_rules_do_not_turn_a_failed_file_into_a_warning() {
    let template = "version: 1\nname: xml\ninput: {format: xml}\nrules:\n  - id: prefixed\n    select: {type: xpath, expression: \"//zz:item\"}\n    output: {key: prefixed}\n  - id: css\n    select: {type: css, expression: \"item\"}\n    output: {key: css}\n";
    let fixture = Fixture::new("error_and_not_applicable", template);
    fixture.source("a.xml", r#"<root><item id="1">x</item></root>"#);
    assert_eq!(run::run(&fixture.cli).unwrap(), ExitCode::InvalidInputOrCli);
    let document = fixture.document();
    assert!(
        file_row(&document, "a.xml").contains("| xml | error |"),
        "{document}"
    );
    assert!(document.contains("Files failed: 1\n"));
    assert!(document.contains("Run status: failure\n"));
    let prefixed = block_of(&document, "a.xml", "prefixed");
    assert!(prefixed.text.contains("Status: error\n"));
    let css = block_of(&document, "a.xml", "css");
    assert!(css.text.contains("Status: rule not applicable\n"));
    assert!(css.text.contains("File status: error\n"));

    // Next to a file that yields data, the run is partial.
    fixture.source(
        "b.xml",
        "<root xmlns:zz=\"urn:z\"><zz:item>y</zz:item></root>",
    );
    assert_eq!(run::run(&fixture.cli).unwrap(), ExitCode::PartialSuccess);
    let document = fixture.document();
    assert!(file_row(&document, "a.xml").contains("| xml | error |"));
    assert!(file_row(&document, "b.xml").contains("| xml | warning |"));
    assert!(document.contains("Run status: partial_success\n"));
}
