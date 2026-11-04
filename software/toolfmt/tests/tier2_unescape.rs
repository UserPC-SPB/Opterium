//! Tier 2, lane "unescape": the `unescape` item transform must decode valid
//! escapes, combine UTF-16 surrogate pairs and keep every invalid sequence
//! literally instead of dropping characters.

mod support;

use any_txt_parser::limits::ExitCode;
use any_txt_parser::render::unescape_anchor_text;
use any_txt_parser::run;
use support::Fixture;

/// Concatenates the decoded data items of all blocks in document order.
fn all_items(document: &str) -> Vec<String> {
    support::blocks(document)
        .iter()
        .flat_map(support::data_items)
        .map(|item| unescape_anchor_text(&item).into_owned())
        .collect()
}

#[test]
fn unescape_transform_decodes_pairs_and_keeps_invalid_escapes_in_text_sources() {
    let template = "version: 1\nname: unescape\ninput: {format: text}\nrules:\n  - id: r\n    select: {type: regex, expression: \".+\"}\n    output: {key: values}\n    transforms: [unescape]\n";
    let fixture = Fixture::new("unescape_text", template);
    let lines = [
        r"crab \ud83e\udd80 and \uD83D\uDE00",
        r"lone \ud800 and \udd80 stay",
        r"bad \uZZZZ \u12 \u+041 tail \u",
        r#"quote \"x\" tab\tend \u0041"#,
        r"multi\nline",
        r"path C:\Users\b\f\x41 \/ \0",
        r"trailing \",
    ];
    fixture.source("a.log", lines.join("\n"));

    assert_eq!(run::run(&fixture.cli).unwrap(), ExitCode::Success);
    let document = fixture.document();
    support::assert_index_lines(&document);
    support::assert_total_characters(&document);
    assert_eq!(
        all_items(&document),
        vec![
            "crab 🦀 and 😀".to_string(),
            r"lone \ud800 and \udd80 stay".to_string(),
            r"bad \uZZZZ \u12 \u+041 tail \u".to_string(),
            "quote \"x\" tab\tend A".to_string(),
            "multi\nline".to_string(),
            r"path C:\Users\b\f\x41 \/ \0".to_string(),
            r"trailing \".to_string(),
        ]
    );
}

#[test]
fn unescape_transform_decodes_double_escaped_json_strings() {
    // The JSON parser removes one level of escaping; `unescape` the second.
    let template = "version: 1\nname: unescape_json\ninput: {format: json}\nrules:\n  - id: r\n    select: {type: jsonpath, expression: \"$.values[*]\"}\n    output: {key: values}\n    transforms: [unescape]\n";
    let fixture = Fixture::new("unescape_json", template);
    fixture.source(
        "a.json",
        r#"{"values": ["\\ud83e\\udd80", "\\ud83e", "\\u00e9\\n\\u12"]}"#,
    );

    assert_eq!(run::run(&fixture.cli).unwrap(), ExitCode::Success);
    let document = fixture.document();
    support::assert_index_lines(&document);
    assert_eq!(
        all_items(&document),
        vec![
            "🦀".to_string(),
            r"\ud83e".to_string(),
            "é\n\\u12".to_string(),
        ]
    );
}
