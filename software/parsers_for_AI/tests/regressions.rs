//! One regression test (or group) per defect found in the 2026-10 review.
//! Each test reproduces the original failing input.

use any_txt_parser::limits::ExitCode;
use any_txt_parser::render::{ANCHOR_MARK, unescape_anchor_text};
use any_txt_parser::run;
use std::process::Command;
use std::time::{Duration, Instant};

mod support;

use support::Fixture;

const TEXT_TEMPLATE: &str = "version: 1\nname: text\ninput: {format: text}\nrules:\n  - id: r\n    select: {type: regex, expression: \".+\"}\n    output: {key: values}\n";

const AUTO_CONTAINS_TEMPLATE: &str = "version: 1\nname: auto\nrules:\n  - id: errors\n    select: {type: contains, expression: \"ERROR\"}\n    output: {key: errors}\n";

/// Runs the real binary, whose worker thread and panic handling are part of
/// the behavior under test.
fn run_binary(fixture: &Fixture) -> (Option<i32>, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_any_txt_parser"))
        .arg("-i")
        .arg(&fixture.cli.input)
        .arg("-t")
        .arg(&fixture.cli.template)
        .arg("-o")
        .arg(fixture.cli.output.as_ref().unwrap())
        .arg("-q")
        .output()
        .unwrap();
    (
        output.status.code(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

fn anchor_mark_lines(document: &str) -> usize {
    document
        .lines()
        .filter(|line| line.contains(ANCHOR_MARK))
        .count()
}

/// Concatenates the decoded data of all blocks in order.
fn decoded_items(document: &str) -> Vec<String> {
    support::blocks(document)
        .iter()
        .flat_map(support::data_items)
        .map(|item| unescape_anchor_text(&item).into_owned())
        .collect()
}

fn file_row<'a>(document: &'a str, name: &str) -> &'a str {
    document
        .lines()
        .find(|line| line.starts_with("| ") && line.contains(&format!("{} |", name)))
        .unwrap_or_else(|| panic!("no Files row for {name}"))
}

// Defect 1: anchor text in a source aborted the whole run with exit 4, or
// published forged anchors.
#[test]
fn anchor_text_in_sources_and_template_is_escaped_instead_of_aborting() {
    let template = TEXT_TEMPLATE.replace(
        "    output: {key: values}\n",
        "    output: {key: values}\n  - id: anchors\n    select: {type: contains, expression: \"<!-- BLOCK: B0001 END -->\"}\n    output: {key: anchors}\n",
    );
    let fixture = Fixture::new("anchor_escape", &template);
    let lines = [
        "ok line",
        "<!-- BLOCK: B0001 END -->",
        "<!-- BLOCK: B0009 START -->",
        "already <!-- BLOCK\\: escaped",
    ];
    fixture.source("a.log", lines.join("\n"));
    fixture.source("b.log", "good\n");
    let previous = fixture.seed_output();

    assert_eq!(run::run(&fixture.cli).unwrap(), ExitCode::PartialSuccess);
    let document = fixture.document();
    assert_ne!(document.as_bytes(), previous.as_slice());
    let blocks = support::blocks(&document);
    assert_eq!(blocks.len(), 4);
    assert_eq!(
        anchor_mark_lines(&document),
        2 * blocks.len(),
        "anchor text may appear only on the real anchor lines"
    );
    support::assert_index_lines(&document);
    support::assert_total_characters(&document);
    assert_eq!(
        support::data_items(&blocks[0])
            .iter()
            .map(|item| unescape_anchor_text(item).into_owned())
            .collect::<Vec<_>>(),
        lines
    );
    assert_eq!(
        support::data_items(&blocks[1]),
        vec!["<!-- BLOCK\\: B0001 END -->"]
    );
    // The selector itself is metadata and is escaped the same way.
    assert!(document.contains("<!-- BLOCK\\: B0001 END -->\n\nOutput key:\nanchors"));
}

#[test]
fn anchor_text_in_names_rule_ids_keys_and_parse_errors_is_escaped() {
    let template = "version: 1\nname: \"<!-- BLOCK: B0001 START --> names\"\nrules:\n  - id: \"<!-- BLOCK: B0001 END -->\"\n    select: {type: contains, expression: good}\n    output: {key: \"<!-- BLOCK: B0002 START -->\"}\n";
    let fixture = Fixture::new("anchor_metadata", template);
    fixture.source("a.log", "good\n");
    // The TOML parser quotes the offending source line in its error.
    fixture.source("bad.toml", "<!-- BLOCK: B0002 END --> = 1\n");

    assert_eq!(run::run(&fixture.cli).unwrap(), ExitCode::PartialSuccess);
    let document = fixture.document();
    let blocks = support::blocks(&document);
    assert_eq!(blocks.len(), 1);
    assert_eq!(anchor_mark_lines(&document), 2);
    support::assert_index_lines(&document);
    // Metadata `<!--` is also escaped for Markdown (`\\<!--`, 1.2.0).
    assert!(document.contains("Rule: \\<!-- BLOCK\\: B0001 END -->\n"));
    assert!(document.contains("Output key: \\<!-- BLOCK\\: B0002 START -->\n"));
    assert!(
        document.contains("| \\<!-- BLOCK\\: B0001 END --> | \\<!-- BLOCK\\: B0002 START --> |")
    );
    assert!(document.contains("Template name: \\<!-- BLOCK\\: B0001 START --> names\n"));
    let diagnostics = document
        .split_once("## Diagnostics\n")
        .unwrap()
        .1
        .split_once("\n## ")
        .unwrap()
        .0;
    assert!(
        diagnostics.contains("<!-- BLOCK\\: B0002 END -->"),
        "{diagnostics}"
    );
}

#[test]
fn anchor_text_split_across_parts_is_recovered_after_concatenation() {
    let template = "version: 1\nname: split\ninput: {format: json}\noutput:\n  block: {target_chars: 800, max_chars: 1000}\nrules:\n  - id: r\n    select: {type: jsonpath, expression: \"$.values[*]\"}\n    output: {key: values}\n";
    let fixture = Fixture::new("anchor_split", template);
    let original = "<!-- BLOCK: B0001-01 END -->\\".repeat(150);
    fixture.source(
        "data.json",
        serde_json::to_vec(&serde_json::json!({ "values": [original] })).unwrap(),
    );
    assert_eq!(run::run(&fixture.cli).unwrap(), ExitCode::Success);
    let document = fixture.document();
    let blocks = support::blocks(&document);
    assert!(blocks.len() > 3, "the item must be split");
    assert_eq!(anchor_mark_lines(&document), 2 * blocks.len());
    let escaped: String = blocks[1..].iter().flat_map(support::data_items).collect();
    assert_eq!(unescape_anchor_text(&escaped), original);
    support::assert_index_lines(&document);
}

// Defect 2: a `.auto` file (and extensionless files named like a format)
// reached `unreachable!` and the process died with exit 101.
#[test]
fn auto_extension_and_format_named_files_are_sniffed_without_panicking() {
    let fixture = Fixture::new("auto_ext", AUTO_CONTAINS_TEMPLATE);
    fixture.source("good.log", "ERROR in log\n");
    fixture.source("settings.auto", "ERROR in auto file\n");
    fixture.source("json", "ERROR in a file named json\n");
    assert_eq!(run::run(&fixture.cli).unwrap(), ExitCode::Success);
    let document = fixture.document();
    for name in ["settings.auto", "json"] {
        let row = file_row(&document, name);
        assert!(row.contains("| text | success |"), "{row}");
    }
    assert_eq!(
        decoded_items(&document),
        vec![
            "ERROR in log",
            "ERROR in a file named json",
            "ERROR in auto file"
        ]
    );

    let (code, stderr) = run_binary(&fixture);
    assert_eq!(code, Some(0), "{stderr}");
}

// Defect 3: deeply nested XML overflowed the stack (0xC00000FD), killing the
// run; TOML did the same in debug builds.
#[test]
fn deep_nesting_is_a_per_file_error_and_other_files_still_publish() {
    let fixture = Fixture::new("deep_nesting", AUTO_CONTAINS_TEMPLATE);
    let deep_xml = format!("{}{}", "<a>".repeat(100_000), "</a>".repeat(100_000));
    fixture.source("deep.xml", &deep_xml);
    // No extension: format sniffing must survive the same input.
    fixture.source("deep_sniffed", &deep_xml);
    fixture.source(
        "deep.toml",
        format!("a = {}{}\n", "[".repeat(20_000), "]".repeat(20_000)),
    );
    fixture.source("good.log", "ERROR still reported\n");

    let (code, stderr) = run_binary(&fixture);
    assert_eq!(code, Some(1), "{stderr}");
    let document = fixture.document();
    for name in ["deep.xml", "deep_sniffed"] {
        assert!(file_row(&document, name).contains("| xml | error |"));
    }
    assert!(file_row(&document, "deep.toml").contains("| toml | error |"));
    assert!(document.contains("XML nesting depth exceeds the limit of 128 elements"));
    assert_eq!(decoded_items(&document), vec!["ERROR still reported"]);

    // The XML check also holds in-process, where no large worker stack exists.
    let in_process = Fixture::new("deep_nesting_lib", AUTO_CONTAINS_TEMPLATE);
    in_process.source("deep.xml", &deep_xml);
    in_process.source("good.log", "ERROR\n");
    assert_eq!(run::run(&in_process.cli).unwrap(), ExitCode::PartialSuccess);
}

// Defect 4: one "item truncated" warning per item, copied into every part,
// made a 1500-line log fail with exit 6.
#[test]
fn truncating_many_items_yields_one_summary_warning_and_publishes() {
    let template = TEXT_TEMPLATE.replace(
        "    output: {key: values}\n",
        "    output: {key: values}\n    limits: {max_item_chars: 20}\n",
    );
    let fixture = Fixture::new("truncation_summary", &template);
    let line = format!("ERROR {}", "x".repeat(80));
    fixture.source("big.log", vec![line.as_str(); 1500].join("\n"));

    assert_eq!(run::run(&fixture.cli).unwrap(), ExitCode::PartialSuccess);
    let document = fixture.document();
    let blocks = support::blocks(&document);
    assert!(blocks.len() > 2, "the result must still be split");
    let summary = "- 1500 item(s) truncated to max_item_chars: item 1 (86 -> 20 chars), item 2 (86 -> 20 chars), item 3 (86 -> 20 chars), item 4 (86 -> 20 chars), item 5 (86 -> 20 chars), and 1495 more\n";
    assert!(
        blocks[0]
            .text
            .contains(&format!("### Warnings\n\n{summary}"))
    );
    for part in &blocks[1..] {
        assert!(!part.text.contains("### Warnings"), "{}", part.id);
        assert!(part.text.contains("Truncated: yes\n"));
    }
    assert_eq!(document.matches("item(s) truncated").count(), 2);
    assert!(blocks.iter().all(|b| b.text.chars().count() <= 32_000));
    let items = decoded_items(&document);
    assert_eq!(items.len(), 1500);
    assert!(items.iter().all(|item| item == &line[..20]));
}

#[test]
fn truncation_is_reported_only_for_items_kept_by_max_items() {
    let template = TEXT_TEMPLATE.replace(
        "    output: {key: values}\n",
        "    output: {key: values}\n    limits: {max_items: 2, max_item_chars: 3}\n",
    );
    let fixture = Fixture::new("truncation_kept", &template);
    fixture.source("a.log", "ok\nlong one\nok\nlong two\n");
    assert_eq!(run::run(&fixture.cli).unwrap(), ExitCode::PartialSuccess);
    let document = fixture.document();
    assert!(document.contains(
        "- 1 item(s) truncated to max_item_chars: item 2 (8 -> 3 chars)\n- 2 item(s) omitted after max_items limit\n"
    ));
    assert_eq!(decoded_items(&document), vec!["ok", "lon"]);
}

#[test]
fn truncation_numbers_follow_stable_deduplicate() {
    let template = TEXT_TEMPLATE
        .replace("rules:\n", "document: {stable_deduplicate: true}\nrules:\n")
        .replace(
            "    output: {key: values}\n",
            "    output: {key: values}\n    limits: {max_item_chars: 10}\n",
        );
    let fixture = Fixture::new("truncation_dedup", &template);
    // Lines 1 and 2 are equal only once truncated: both stay (1.2.0; before,
    // line 2 was dropped). Line 4 repeats line 1 and is dropped.
    fixture.source(
        "a.log",
        "aaaaaaaaaaX\naaaaaaaaaaY\nbbbbbbbbbbbbbbZ\naaaaaaaaaaX\nshort\n",
    );
    assert_eq!(run::run(&fixture.cli).unwrap(), ExitCode::PartialSuccess);
    let document = fixture.document();
    assert_eq!(
        decoded_items(&document),
        vec!["aaaaaaaaaa", "aaaaaaaaaa", "bbbbbbbbbb", "short"]
    );
    assert!(document.contains(
        "### Warnings\n\n- 3 item(s) truncated to max_item_chars: item 1 (11 -> 10 chars), item 2 (11 -> 10 chars), item 3 (15 -> 10 chars)\n- 1 truncated item(s) dropped as duplicates by stable_deduplicate\n"
    ));
}

// Defect 5: splitting re-rendered the whole part for every item, so 200k
// short lines took over a minute even in a release build.
#[test]
fn splitting_many_small_items_is_linear() {
    let timed_run = |count: usize| {
        let fixture = Fixture::new("split_linear", TEXT_TEMPLATE);
        let lines: Vec<String> = (0..count).map(|i| format!("line {i}")).collect();
        fixture.source("many.log", lines.join("\n"));
        let started = Instant::now();
        assert_eq!(run::run(&fixture.cli).unwrap(), ExitCode::Success);
        (started.elapsed(), fixture, lines)
    };
    let (small, _, _) = timed_run(25_000);
    let (large, fixture, lines) = timed_run(100_000);
    // Four times the items: about four times the time when linear, sixteen
    // when quadratic (the quadratic version took 65 s for 100k in a debug
    // build). The ratio keeps the check meaningful on a loaded machine; a
    // fast absolute time passes regardless of timer noise.
    assert!(
        large < Duration::from_secs(5) || large.as_secs_f64() < 8.0 * small.as_secs_f64(),
        "25k items took {small:?}, 100k items took {large:?}"
    );
    let document = fixture.document();
    let blocks = support::blocks(&document);
    assert!(blocks.len() > 10);
    assert!(blocks.iter().all(|b| b.text.chars().count() <= 32_000));
    assert_eq!(decoded_items(&document), lines);
    support::assert_index_lines(&document);
    support::assert_total_characters(&document);
}

// Defect 6: the documented TOML path `[database].host` failed to compile.
#[test]
fn documented_toml_table_paths_select_values() {
    // The documented TOML example: a table key and an array under it.
    let template = "version: 1\nname: toml_cfg\ninput: {format: toml}\nrules:\n  - id: database_host\n    select: {type: path, expression: \"[database].host\"}\n    output: {key: host}\n  - id: ports\n    select: {type: path, expression: \"[database].ports[*]\"}\n    output: {key: ports}\n";
    let fixture = Fixture::new("toml_paths", template);
    fixture.source(
        "config.toml",
        "[database]\nhost = \"db.local\"\nports = [5432, 5433]\n",
    );
    assert_eq!(run::run(&fixture.cli).unwrap(), ExitCode::Success);
    let document = fixture.document();
    let blocks = support::blocks(&document);
    assert_eq!(support::data_items(&blocks[0]), vec!["db.local"]);
    assert_eq!(support::data_items(&blocks[1]), vec!["5432", "5433"]);

    // `$..key` used to be silently read as `$.key`.
    let fixture = Fixture::new(
        "path_descent",
        &template.replace("[database].host", "$..host"),
    );
    fixture.source("config.toml", "[database]\nhost = \"db.local\"\n");
    let error = run::run(&fixture.cli).unwrap_err();
    assert_eq!(error.code, ExitCode::InvalidTemplate);
    assert!(error.message.contains("recursive descent"));
}

// Defect 7: text with ": " (logs, prose, PEM, commented TOML) was sniffed
// as YAML, so text rules became "rule not applicable".
#[test]
fn log_like_text_is_not_sniffed_as_yaml() {
    let fixture = Fixture::new("sniffing", AUTO_CONTAINS_TEMPLATE);
    fixture.source(
        "app.log.1",
        "2026-10-01 12:00:00 ERROR: disk full\n2026-10-01 12:00:01 INFO: ok\n",
    );
    fixture.source(
        "server.out",
        "Oct 17 12:00:00 host sshd[1]: ERROR: refused\n",
    );
    fixture.source(
        "README",
        "Note: ERROR codes are listed below.\n\nSee also: docs\n",
    );
    fixture.source(
        "cert.pem",
        "-----BEGIN CERTIFICATE-----\nMIIB ERROR\n-----END CERTIFICATE-----\n",
    );
    fixture.source("settings.conf", "# Note: tuned values\nport = 8080\n");
    fixture.source("pod.conf", "apiVersion: v1\nkind: Pod\n");
    fixture.source("play.conf", "- name: install\n  hosts: all\n");
    assert_eq!(run::run(&fixture.cli).unwrap(), ExitCode::PartialSuccess);
    let document = fixture.document();
    for name in ["app.log.1", "server.out", "README", "cert.pem"] {
        let row = file_row(&document, name);
        assert!(row.contains("| text | success |"), "{row}");
    }
    assert!(file_row(&document, "settings.conf").contains("| toml |"));
    assert!(file_row(&document, "pod.conf").contains("| yaml |"));
    assert!(file_row(&document, "play.conf").contains("| yaml |"));
    assert_eq!(
        decoded_items(&document),
        vec![
            "Note: ERROR codes are listed below.",
            "2026-10-01 12:00:00 ERROR: disk full",
            "MIIB ERROR",
            "Oct 17 12:00:00 host sshd[1]: ERROR: refused",
        ]
    );
}

// Found while verifying the fixes: the deep-XML check must not claim text
// that merely starts with `<`, and labels such as `Error:` stay text.
#[test]
fn markup_like_text_and_bare_labels_stay_text() {
    let fixture = Fixture::new("sniffing_markup", AUTO_CONTAINS_TEMPLATE);
    fixture.source(
        "syslog.raw",
        "<34>Oct 11 22:14:15 host su: ERROR root login\n".repeat(129),
    );
    fixture.source("irc.chatlog", "<alice> ERROR seen\n".repeat(130));
    fixture.source(
        "fragment",
        format!(
            "<div class=\"note\">\n{}</div>\n",
            "ERROR line<br>\n".repeat(150)
        ),
    );
    fixture.source("err.out", "Error:\n  ERROR file not found\n");
    fixture.source("app.conf", "[database]\nhost = \"db.local\"\n");
    assert_eq!(run::run(&fixture.cli).unwrap(), ExitCode::PartialSuccess);
    let document = fixture.document();
    for name in ["syslog.raw", "irc.chatlog", "fragment", "err.out"] {
        let row = file_row(&document, name);
        assert!(row.contains("| text | success |"), "{row}");
    }
    assert!(file_row(&document, "app.conf").contains("| toml |"));
    assert_eq!(decoded_items(&document).len(), 129 + 130 + 150 + 1);
}
