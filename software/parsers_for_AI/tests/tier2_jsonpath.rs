//! Tier 2, lane "jsonpath": RFC 9535 semantics of the supported JSONPath
//! subset. Each test reproduces a query that returned wrong data (or failed
//! to compile) before the fix, run end to end through the published document.

mod support;

use any_txt_parser::limits::ExitCode;
use any_txt_parser::run;
use support::Fixture;

/// One jsonpath rule. The expression is embedded as a JSON string, which is
/// also a valid YAML double-quoted scalar, so backslashes reach the parser
/// exactly as written in the Rust source.
fn template(expression: &str) -> String {
    format!(
        "version: 1\nname: jsonpath\ninput: {{format: json}}\nrules:\n  - id: r\n    select: {{type: jsonpath, expression: {}}}\n    output: {{key: values}}\n",
        serde_json::to_string(expression).unwrap()
    )
}

/// Runs `expression` over one JSON source and returns the exit code and the
/// published items of the single block.
fn select(name: &str, json: &str, expression: &str) -> (ExitCode, Vec<String>) {
    let fixture = Fixture::new(name, &template(expression));
    fixture.source("data.json", json);
    let code = run::run(&fixture.cli)
        .unwrap_or_else(|e| panic!("{expression}: run failed ({:?}): {}", e.code, e.message));
    let document = fixture.document();
    support::assert_index_lines(&document);
    let blocks = support::blocks(&document);
    assert_eq!(blocks.len(), 1, "{expression}");
    (code, support::data_items(&blocks[0]))
}

fn assert_selects(name: &str, json: &str, expression: &str, expected: &[&str]) {
    let (code, items) = select(name, json, expression);
    assert_eq!(items, expected, "{expression}");
    assert_eq!(code, ExitCode::Success, "{expression}");
}

fn assert_template_error(name: &str, expression: &str, expected_message: &str) {
    let fixture = Fixture::new(name, &template(expression));
    fixture.source("data.json", r#"{"a": [{"v": 1}], "x": 1}"#);
    let previous = fixture.seed_output();
    let error = run::run(&fixture.cli)
        .err()
        .unwrap_or_else(|| panic!("{expression} must be rejected at compile time"));
    assert_eq!(error.code, ExitCode::InvalidTemplate, "{expression}");
    assert!(
        error.message.contains("invalid jsonpath") && error.message.contains(expected_message),
        "{expression}: {}",
        error.message
    );
    fixture.assert_output_unchanged(&previous);
}

const RECORDS: &str = r#"{"records": [
    {"id": "a", "role": "user"},
    {"id": "b"},
    {"id": "c", "role": "assistant"},
    {"id": "d", "role": null}
]}"#;

// Item 1: `!=` against a missing field silently dropped every record without
// the field. RFC 9535: Nothing != value is true; ordering is false.
#[test]
fn not_equal_keeps_records_where_the_field_is_missing() {
    assert_selects(
        "ne_missing",
        RECORDS,
        "$.records[?(@.role != 'user')].id",
        &["b", "c", "d"],
    );
    assert_selects(
        "ne_missing_reversed",
        RECORDS,
        "$.records[?('user' != @.role)].id",
        &["b", "c", "d"],
    );
    assert_selects(
        "ne_null",
        RECORDS,
        "$.records[?(@.role != null)].id",
        &["a", "b", "c"],
    );
    // Unchanged: == and the ordering operators are false against Nothing.
    assert_selects(
        "eq_missing",
        RECORDS,
        "$.records[?(@.role == 'user')].id",
        &["a"],
    );
    assert_selects(
        "lt_missing",
        RECORDS,
        "$.records[?(@.role < 'z')].id",
        &["a", "c"],
    );
    assert_selects(
        "ge_missing",
        RECORDS,
        "$.records[?(@.role >= 'a')].id",
        &["a", "c"],
    );
}

// Item 2: filters ignored objects. RFC 9535: a filter selects the matching
// children of arrays and of objects (member values, here in source order).
#[test]
fn filter_selects_matching_member_values_of_an_object() {
    let json = r#"{"services": {
        "api": {"port": 8080, "enabled": true},
        "db": {"port": 5432, "enabled": false},
        "cache": {"port": 6379, "enabled": true}
    }, "limits": {"cpu": 2, "memory": 512, "disk": 40}}"#;
    assert_selects(
        "object_filter",
        json,
        "$.services[?(@.enabled == true)].port",
        &["8080", "6379"],
    );
    assert_selects(
        "object_filter_scalars",
        json,
        "$.limits[?(@ > 10)]",
        &["512", "40"],
    );
}

// Item 3: decimal and exponent literals failed to parse. They now compare by
// exact numeric value against integers and floats in the source.
#[test]
fn decimal_and_exponent_literals_compare_exactly() {
    let json = r#"{"readings": [
        {"id": "r1", "t": 1.25},
        {"id": "r2", "t": 1.5},
        {"id": "r3", "t": 2},
        {"id": "r4", "t": 1000},
        {"id": "r5", "t": 0.025},
        {"id": "r6", "t": -2.25},
        {"id": "r7", "t": 9007199254740993}
    ]}"#;
    let cases: &[(&str, &[&str])] = &[
        ("@.t > 1.25", &["r2", "r3", "r4", "r7"]),
        ("@.t == 2.0", &["r3"]),
        ("@.t == 1e3", &["r4"]),
        ("@.t == 1E+3", &["r4"]),
        ("@.t == 2.5E-2", &["r5"]),
        ("@.t < -2", &["r6"]),
        ("@.t == -2.25", &["r6"]),
        // 2^53 + 1 is not rounded to the float 2^53.
        ("@.t > 9007199254740992.0", &["r7"]),
    ];
    for (index, (filter, expected)) in cases.iter().enumerate() {
        assert_selects(
            &format!("numbers_{index}"),
            json,
            &format!("$.readings[?({filter})].id"),
            expected,
        );
    }
}

// Also checked: escapes in quoted names and string literals were kept as
// literal backslashes, so `\\` and `\uXXXX` matched the wrong key (or none)
// and `\'` / `\"` broke the query.
#[test]
fn string_escapes_in_names_and_literals_are_decoded() {
    let json = r#"{
        "it's": "apostrophe",
        "a\\b": "one backslash",
        "a\\\\b": "two backslashes",
        "A": "unicode",
        "say \"hi\"": "quote",
        "😀": "emoji",
        "people": [{"id": "p1", "name": "O'Brien"}, {"id": "p2", "name": "C:\\new"}]
    }"#;
    let cases: &[(&str, &str)] = &[
        (r"$['it\'s']", "apostrophe"),
        (r#"$["it's"]"#, "apostrophe"),
        (r"$['a\\b']", "one backslash"),
        (r"$['a\\\\b']", "two backslashes"),
        (r"$['\u0041']", "unicode"),
        (r#"$["say \"hi\""]"#, "quote"),
        (r"$['\uD83D\uDE00']", "emoji"),
        (r"$.people[?(@.name == 'O\'Brien')].id", "p1"),
        (r"$.people[?(@.name == 'C:\\new')].id", "p2"),
        (r"$.people[?(@['\u006eame'] == 'O\'Brien')].id", "p1"),
    ];
    for (index, (expression, expected)) in cases.iter().enumerate() {
        assert_selects(&format!("escapes_{index}"), json, expression, &[expected]);
    }
}

#[test]
fn invalid_escapes_and_number_literals_are_template_errors() {
    assert_template_error("bad_escape", r"$['\x41']", "invalid escape '\\x'");
    assert_template_error(
        "bad_escape_in_filter",
        r"$.a[?(@.v == 'C:\data')]",
        "invalid escape '\\d'",
    );
    assert_template_error(
        "wrong_quote_escape",
        r#"$["it\'s"]"#,
        "only allowed inside single-quoted strings",
    );
    assert_template_error("lone_surrogate", r"$['\uD83D']", "unpaired surrogate");
    assert_template_error("short_unicode", r"$['\u41']", "expected 4 hex digits");
    assert_template_error(
        "unterminated",
        r"$.a[?(@.v == 'x\')]",
        "unterminated string literal",
    );
    assert_template_error("trailing_dot", "$.a[?(@.v == 1.)]", "invalid number '1.'");
    assert_template_error("bare_exponent", "$.a[?(@.v == 1e)]", "invalid number '1e'");
    assert_template_error("huge_number", "$.a[?(@.v == 1e400)]", "out of range");
}

// Review fix: literals the JSON number representation cannot hold were
// silently rounded. `-9223372036854775809` became -2^63 and matched
// i64::MIN; `1e-400` became 0, so `@ < 1e-400` dropped 0 (0 < 1e-400).
// They are now template errors, like the overflow `1e400`.
#[test]
fn number_literals_that_would_be_silently_rounded_are_template_errors() {
    assert_template_error(
        "below_i64_min",
        "$.a[?(@.v == -9223372036854775809)]",
        "integer '-9223372036854775809' at position 13 is outside the exact integer range",
    );
    assert_template_error(
        "above_u64_max",
        "$.a[?(@.v > 18446744073709551616)]",
        "outside the exact integer range [-2^63, 2^64)",
    );
    assert_template_error(
        "underflow",
        "$.a[?(@.v < 1e-400)]",
        "number '1e-400' at position 12 is out of range: it is too small and would round to 0",
    );
    // The boundaries of the exact integer domain and explicit doubles work.
    let json = r#"{"n": [
        {"id": "zero", "v": 0},
        {"id": "min", "v": -9223372036854775808},
        {"id": "max", "v": 18446744073709551615},
        {"id": "tiny", "v": 5e-324}
    ]}"#;
    let cases: &[(&str, &[&str])] = &[
        ("@.v == -9223372036854775808", &["min"]),
        ("@.v == 18446744073709551615", &["max"]),
        ("@.v < -9223372036854775807", &["min"]),
        ("@.v > 18446744073709551614", &["max"]),
        ("@.v > 0 && @.v < 1e-300", &["tiny"]),
        ("@.v == 0e-400", &["zero"]),
        ("@.v < 1e20", &["zero", "min", "max", "tiny"]),
    ];
    for (index, (filter, expected)) in cases.iter().enumerate() {
        assert_selects(
            &format!("number_bounds_{index}"),
            json,
            &format!("$.n[?({filter})].id"),
            expected,
        );
    }
}

// RFC 9535 equality is structural for arrays and objects; previously two
// equal arrays compared unequal and `!=` selected them.
#[test]
fn equality_of_arrays_and_objects_is_structural() {
    let json = r#"{"pairs": [
        {"id": "same-array", "left": [1, 2], "right": [1, 2]},
        {"id": "other-order", "left": [1, 2], "right": [2, 1]},
        {"id": "same-object", "left": {"k": 1, "j": 2}, "right": {"j": 2.0, "k": 1}},
        {"id": "strings", "left": "x", "right": "x"}
    ]}"#;
    assert_selects(
        "deep_eq",
        json,
        "$.pairs[?(@.left == @.right)].id",
        &["same-array", "same-object", "strings"],
    );
    assert_selects(
        "deep_ne",
        json,
        "$.pairs[?(@.left != @.right)].id",
        &["other-order"],
    );
}
