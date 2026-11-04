use any_txt_parser::collect::{FileResult, FileStatus, RuleResult, RuleStatus};
use any_txt_parser::limits::ExitCode;
use any_txt_parser::materialize;
use any_txt_parser::run;
use any_txt_parser::template::{self, Format};
use std::path::PathBuf;
use std::process::Command;

mod support;

use support::Fixture;

const TEXT_TEMPLATE: &str = "version: 1\nname: text\ninput: {format: text}\nrules:\n  - id: r\n    select: {type: regex, expression: \".+\"}\n    output: {key: values}\n";

const SPLIT_TEMPLATE: &str = "version: 1\nname: split\ninput: {format: json}\noutput:\n  block: {target_chars: 800, max_chars: 1000}\nrules:\n  - id: r1\n    select: {type: jsonpath, expression: \"$.values[*]\"}\n    output: {key: values}\n";

fn source_result(groups: Vec<Vec<String>>) -> FileResult {
    FileResult {
        path: PathBuf::from("source.json"),
        size: 0,
        mtime_iso: "2026-01-01T00:00:00Z".into(),
        hash: None,
        format: Format::Json,
        format_source: "template".into(),
        encoding: "utf-8".into(),
        status: FileStatus::Success,
        error: None,
        rule_results: groups
            .into_iter()
            .enumerate()
            .map(|(index, items)| RuleResult {
                rule_id: format!("r{}", index + 1),
                key: "values".into(),
                status: RuleStatus::Matched,
                error: None,
                items,
                item_lines: Vec::new(),
                truncated_items: 0,
                omitted_items: 0,
                omitted_by_resource: 0,
                truncated_chars_list: Vec::new(),
                truncated_originals: Vec::new(),
                transform_losses: Vec::new(),
            })
            .collect(),
    }
}

fn continuation_item(block: &support::RenderedBlock) -> usize {
    block
        .text
        .lines()
        .find_map(|line| line.strip_prefix("Item continuation: "))
        .expect("original item index missing from a continuation")
        .parse()
        .unwrap()
}

#[test]
fn unicode_long_lines_and_multiline_items_split_without_data_loss() {
    let fixture = Fixture::new("unicode_split", SPLIT_TEMPLATE);
    let original = vec![
        "🦀ёж漢字🙂".repeat(700),
        format!(
            "\n{}\n\n{}\n{}\n",
            "α🦀".repeat(500),
            "добро".repeat(240),
            "末尾".repeat(350)
        ),
    ];
    fixture.source(
        "data.json",
        serde_json::to_vec(&serde_json::json!({"values": original})).unwrap(),
    );
    assert_eq!(run::run(&fixture.cli).unwrap(), ExitCode::Success);
    let first = fixture.document();
    let blocks = support::blocks(&first);
    assert!(blocks.len() > 4);
    assert_eq!(blocks[0].id, "B0001");
    assert!(support::data_items(&blocks[0]).is_empty());
    let part_count = blocks.len() - 1;
    let mut recovered = vec![String::new(), String::new()];
    for (index, block) in blocks[1..].iter().enumerate() {
        assert_eq!(block.id, format!("B0001-{:02}", index + 1));
        assert!(block.text.contains("Parent: B0001\n"));
        assert!(
            block
                .text
                .contains(&format!("Part: {}/{}\n", index + 1, part_count))
        );
        assert!(block.text.chars().count() <= 1000);
        let item_index = continuation_item(block);
        assert!((1..=2).contains(&item_index));
        let chunk = support::data_items(block);
        assert_eq!(chunk.len(), 1);
        recovered[item_index - 1].push_str(&chunk[0]);
    }
    assert_eq!(
        recovered, original,
        "Unicode, empty lines and terminal newlines must survive"
    );
    assert!(blocks[0].text.chars().count() <= 1000);
    support::assert_index_lines(&first);
    support::assert_total_characters(&first);

    assert_eq!(run::run(&fixture.cli).unwrap(), ExitCode::Success);
    let second = fixture.document();
    assert!(first.contains("Generation: 00000001\n"));
    assert!(second.contains("Generation: 00000002\n"));
    assert_eq!(
        support::normalize_volatile(&first),
        support::normalize_volatile(&second)
    );
    support::assert_index_lines(&second);
    support::assert_total_characters(&second);
}

#[test]
fn repeated_oversized_groups_restart_parts_and_preserve_raw_chunks() {
    let template = template::compile_yaml_str(
        &format!("{SPLIT_TEMPLATE}  - id: r2\n    select: {{type: jsonpath, expression: \"$.values[*]\"}}\n    output: {{key: values}}\n"),
        &PathBuf::from("template.yml"),
        None,
        None,
    )
    .unwrap();
    let originals = vec![
        vec![
            "甲🙂".repeat(1400),
            format!("{}\n\n{}\n", "ёж".repeat(700), "終".repeat(1100)),
        ],
        vec![
            "β🦀".repeat(1200),
            format!("{}\n{}\n", "Ж".repeat(1400), "末".repeat(1000)),
        ],
    ];
    let files = vec![source_result(originals.clone())];
    let materialized = materialize::materialize(&files, &template).unwrap();
    assert_eq!(materialized.total_items, 4);
    assert_eq!(
        materialized.total_chars,
        originals
            .iter()
            .flatten()
            .map(|item| item.chars().count())
            .sum::<usize>()
    );
    let parents: Vec<_> = materialized
        .blocks
        .iter()
        .filter(|block| block.parent_id.is_none())
        .collect();
    assert_eq!(parents.len(), 2);
    assert_eq!(parents[0].id, "B0001");
    assert_eq!(parents[1].id, "B0002");
    for (group_index, parent) in parents.iter().enumerate() {
        assert!(parent.items.is_empty());
        let parts: Vec<_> = materialized
            .blocks
            .iter()
            .filter(|block| block.parent_id.as_deref() == Some(parent.id.as_str()))
            .collect();
        assert!(parts.len() > 2);
        assert_eq!(parent.parts, Some(parts.len()));
        let mut recovered = vec![String::new(), String::new()];
        for (part_index, part) in parts.iter().enumerate() {
            assert_eq!(part.id, format!("{}-{:02}", parent.id, part_index + 1));
            assert_eq!(part.part, Some(part_index + 1));
            assert_eq!(part.parts, Some(parts.len()));
            assert_eq!(part.items.len(), 1);
            assert!(part.chars <= 1000);
            recovered[part.item_index.unwrap() - 1].push_str(&part.items[0]);
        }
        assert_eq!(recovered, originals[group_index]);
    }
}

#[test]
fn block_cap_too_small_for_metadata_fails_without_publishing() {
    let mut fixture = Fixture::new("tiny_block", TEXT_TEMPLATE);
    fixture.source("small.txt", "x\n");
    let old = fixture.seed_output();
    fixture.cli.max_block_chars = Some(64);
    let error = run::run(&fixture.cli).unwrap_err();
    assert_eq!(error.code, ExitCode::ResourceLimit);
    assert_eq!(error.code.to_i32(), 6);
    assert!(error.message.contains("max_block_chars"));
    fixture.assert_output_unchanged(&old);
}

#[test]
fn full_document_limit_and_cli_precedence_preserve_previous_output() {
    let limited_template =
        TEXT_TEMPLATE.replace("rules:\n", "output: {max_output_chars: 1000}\nrules:\n");
    let mut fixture = Fixture::new("output_limit", &limited_template);
    fixture.source("small.txt", "🦀\n");
    let old = fixture.seed_output();
    // The extracted data is one character, but the complete document exceeds 1000.
    let error = run::run(&fixture.cli).unwrap_err();
    assert_eq!(error.code, ExitCode::ResourceLimit);
    assert!(error.message.contains("max_output_chars"));
    fixture.assert_output_unchanged(&old);

    fixture.cli.max_output_size = Some("100000".into());
    assert_eq!(run::run(&fixture.cli).unwrap(), ExitCode::Success);
    let published = fixture.document();
    assert!(published.contains("max_output_chars: 100000\n"));
    assert_eq!(
        support::data_items(&support::blocks(&published)[0]),
        vec!["🦀"]
    );
    support::assert_total_characters(&published);

    let generous_template =
        limited_template.replace("max_output_chars: 1000}", "max_output_chars: 100000}");
    std::fs::write(&fixture.cli.template, generous_template).unwrap();
    fixture.cli.max_output_size = Some("1000".into());
    let error = run::run(&fixture.cli).unwrap_err();
    assert_eq!(error.code, ExitCode::ResourceLimit);
    fixture.assert_output_unchanged(published.as_bytes());
}

#[test]
fn max_memory_counts_decoded_utf8_bytes_across_multiple_files() {
    let mut fixture = Fixture::new("memory", TEXT_TEMPLATE);
    fixture.source("a.txt", "α");
    fixture.source("b.txt", "🦀");
    let old = fixture.seed_output();
    // Both files fit separately (2 and 4 bytes); the run needs 6 bytes.
    fixture.cli.max_memory = Some("4".into());
    let error = run::run(&fixture.cli).unwrap_err();
    assert_eq!(error.code, ExitCode::ResourceLimit);
    assert!(error.message.contains("across the run"));
    fixture.assert_output_unchanged(&old);

    fixture.cli.max_memory = Some("6".into());
    assert_eq!(run::run(&fixture.cli).unwrap(), ExitCode::Success);
    let document = fixture.document();
    let blocks = support::blocks(&document);
    assert_eq!(blocks.len(), 2);
    assert_eq!(support::data_items(&blocks[0]), vec!["α"]);
    assert_eq!(support::data_items(&blocks[1]), vec!["🦀"]);
    assert!(document.contains("Files processed: 2\n"));
}

#[test]
fn max_memory_uses_utf8_expansion_after_legacy_decoding() {
    let mut fixture = Fixture::new("decoded_memory", TEXT_TEMPLATE);
    fixture.source("a.txt", [0xdf_u8]); // Windows-1251 Я: one source byte, two UTF-8 bytes.
    fixture.source("b.txt", [0xdf_u8]);
    fixture.cli.encoding = Some("windows-1251".into());
    fixture.cli.max_memory = Some("3".into());
    let old = fixture.seed_output();
    assert_eq!(
        run::run(&fixture.cli).unwrap_err().code,
        ExitCode::ResourceLimit
    );
    fixture.assert_output_unchanged(&old);
    fixture.cli.max_memory = Some("4".into());
    assert_eq!(run::run(&fixture.cli).unwrap(), ExitCode::Success);
    let document = fixture.document();
    let blocks = support::blocks(&document);
    assert_eq!(support::data_items(&blocks[0]), vec!["Я"]);
    assert_eq!(support::data_items(&blocks[1]), vec!["Я"]);
}

#[test]
fn invalid_utf8_file_does_not_prevent_later_good_file_processing() {
    let fixture = Fixture::new("mixed_decode", TEXT_TEMPLATE);
    fixture.source("a_bad.txt", [b'f', 0xff, b'g']);
    fixture.source("z_good.txt", "успех 🦀\n");
    assert_eq!(run::run(&fixture.cli).unwrap(), ExitCode::PartialSuccess);
    let document = fixture.document();
    assert!(document.contains("Files processed: 2\n"));
    assert!(document.contains("Files successful: 1\n"));
    assert!(document.contains("Files failed: 1\n"));
    assert!(document.contains("## Diagnostics\n"));
    assert!(document.lines().any(|line| line.starts_with("| ")
        && line.contains("a_bad.txt |")
        && line.contains("| error |")));
    assert!(document.lines().any(|line| line.starts_with("| ")
        && line.contains("z_good.txt |")
        && line.contains("| success |")));
    let blocks = support::blocks(&document);
    assert_eq!(blocks.len(), 1);
    assert_eq!(support::data_items(&blocks[0]), vec!["успех 🦀"]);
    support::assert_index_lines(&document);
    support::assert_total_characters(&document);
}

#[test]
fn selectors_keep_exact_results_through_collection_and_rendering() {
    let json_template = "version: 1\nname: filters\ninput: {format: json}\nrules:\n  - id: r\n    select: {type: jsonpath, expression: \"$.records[?(@.role == 'user' || @.role == 'assistant' && @.active == true)].id\"}\n    output: {key: ids}\n";
    let fixture = Fixture::new("json_filter", json_template);
    fixture.source("data.json", r#"{"records":[{"id":"u","role":"user","active":false},{"id":"a0","role":"assistant","active":false},{"id":"a1","role":"assistant","active":true}]}"#);
    assert_eq!(run::run(&fixture.cli).unwrap(), ExitCode::Success);
    assert_eq!(
        support::data_items(&support::blocks(&fixture.document())[0]),
        vec!["u", "a1"]
    );

    let xml_template = "version: 1\nname: positions\ninput: {format: xml}\nrules:\n  - id: r\n    select: {type: xpath, expression: \"//item[2]/@id\"}\n    output: {key: ids}\n";
    let fixture = Fixture::new("xml_position", xml_template);
    fixture.source("data.xml", r#"<root><group><other/><item id="a"/><item id="b"/></group><item id="c"/><other/><item id="d"/></root>"#);
    assert_eq!(run::run(&fixture.cli).unwrap(), ExitCode::Success);
    assert_eq!(
        support::data_items(&support::blocks(&fixture.document())[0]),
        vec!["b", "d"]
    );
}

#[test]
fn mixed_transform_order_changes_results_and_effective_template() {
    let template = "version: 1\nname: transform_order\ninput: {format: text}\nrules:\n  - id: unique_first\n    select: {type: regex, expression: \".+\"}\n    output: {key: first}\n    transforms: [unique, lowercase]\n  - id: lowercase_first\n    select: {type: regex, expression: \".+\"}\n    output: {key: second}\n    transforms: [lowercase, unique]\n";
    let fixture = Fixture::new("transform_order", template);
    fixture.source("data.txt", "A\na\n");
    assert_eq!(run::run(&fixture.cli).unwrap(), ExitCode::Success);
    let document = fixture.document();
    let blocks = support::blocks(&document);
    assert_eq!(blocks.len(), 2);
    assert_eq!(support::data_items(&blocks[0]), vec!["a", "a"]);
    assert_eq!(support::data_items(&blocks[1]), vec!["a"]);
    let effective = document
        .split_once("## Effective Template\n\n```yaml\n")
        .unwrap()
        .1
        .split_once("```\n")
        .unwrap()
        .0;
    let effective: serde_yaml::Value = serde_yaml::from_str(effective).unwrap();
    let rules = effective["rules"].as_sequence().unwrap();
    let transforms = |rule: &serde_yaml::Value| -> Vec<String> {
        rule["transforms"]
            .as_sequence()
            .unwrap()
            .iter()
            .map(|value| value.as_str().unwrap().to_string())
            .collect()
    };
    assert_eq!(transforms(&rules[0]), vec!["unique", "lowercase"]);
    assert_eq!(transforms(&rules[1]), vec!["lowercase", "unique"]);
    support::assert_total_characters(&document);
}

#[test]
fn binary_cli_parse_errors_exit3_and_help_remains_successful() {
    let binary = env!("CARGO_BIN_EXE_any_txt_parser");
    for args in [
        vec![],
        vec!["--unknown-flag"],
        vec![
            "--input",
            "input.txt",
            "--template",
            "tpl.yml",
            "--max-block-chars",
            "bogus",
        ],
    ] {
        let output = Command::new(binary).args(&args).output().unwrap();
        assert_eq!(output.status.code(), Some(3), "arguments: {args:?}");
        assert!(!output.stderr.is_empty());
    }
    for flag in ["--help", "--version"] {
        let output = Command::new(binary).arg(flag).output().unwrap();
        assert!(
            output.status.success(),
            "{flag} must remain a successful informational request"
        );
        assert!(!output.stdout.is_empty());
    }
}
