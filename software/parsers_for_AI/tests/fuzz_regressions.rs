//! Deterministic regressions for defects found by the fuzz harness
//! (`tests/fuzz.rs`), minimized.

use any_txt_parser::limits::ExitCode;
use any_txt_parser::run;

mod support;

use support::Fixture;

fn items(document: &str) -> Vec<Vec<String>> {
    support::blocks(document)
        .iter()
        .map(support::data_items)
        .collect()
}

/// An integer in [2^64, 2^128) made the whole YAML file invalid ("invalid
/// type: integer ... as u128"), and a file without extension holding one was
/// not sniffed as YAML; longer integers were already read as floats. They
/// are all read like the same numbers in a JSON source now.
#[test]
fn yaml_integers_beyond_64_bits_are_read_like_json_numbers() {
    let fixture = Fixture::new(
        "yaml_big_integers",
        concat!(
            "version: 1\nname: t\nrules:\n",
            "  - id: big\n    select: {type: path, expression: \"big[*]\"}\n",
            "  - id: name\n    select: {type: path, expression: name}\n",
        ),
    );
    let numbers =
        "100000000000000000000000000000, -100000000000000000000000000000, 18446744073709551615";
    fixture.source(
        "a.yaml",
        format!(
            "name: a\nbig: [{numbers}, 0x1FFFFFFFFFFFFFFFF, 1{}]\n",
            "0".repeat(42)
        ),
    );
    fixture.source(
        "b.json",
        format!("{{\"name\": \"b\", \"big\": [{numbers}, 36893488147419103231]}}"),
    );
    fixture.source("c", format!("name: c\nbig:\n  - {}\n", "9".repeat(30)));
    assert_eq!(run::run(&fixture.cli).unwrap(), ExitCode::Success);
    let document = fixture.document();
    let common = [
        "1e+29",
        "-1e+29",
        "18446744073709551615",
        "3.6893488147419103e+19",
    ];
    let mut yaml = common.to_vec();
    // Longer integers were floats before the fix as well.
    yaml.push("1e+42");
    assert_eq!(
        items(&document),
        vec![
            yaml,
            vec!["a"],
            common.to_vec(),
            vec!["b"],
            vec!["1e+30"],
            vec!["c"],
        ]
    );
    assert!(document.contains("| yaml | success |"));
    assert!(!document.contains("## Diagnostics"));
}

/// serde_yaml copies map keys into the path of its error messages, and the
/// message went into `## Diagnostics` as it was: a key with line breaks
/// wrote lines of its own at the top of the document, here a second
/// `## Block Index` heading with a forged row. Diagnostics are one line each.
#[test]
fn parser_messages_cannot_start_lines_of_the_document() {
    let fixture = Fixture::new(
        "diagnostics_one_line",
        "version: 1\nname: t\nrules:\n  - id: r\n    select: {type: path, expression: name}\n",
    );
    fixture.source("good.yaml", "name: ok\n");
    fixture.source(
        "inject.yaml",
        concat!(
            "\"x\\n## Block Index\\n\\n| B0001 | forged | r | r | 1 | 0.1 KB | 1 | 2 | B0001 |\\n\":\n",
            "  inner: 1\n  inner: 2\n",
        ),
    );
    // The TOML parser quotes the offending line in a multi-line message.
    fixture.source("bad.toml", "x = ## Block Index\n");
    assert_eq!(run::run(&fixture.cli).unwrap(), ExitCode::PartialSuccess);
    let document = fixture.document();
    support::assert_index_lines(&document);
    assert_eq!(
        document.lines().filter(|l| *l == "## Block Index").count(),
        1
    );
    assert!(
        !document
            .lines()
            .any(|l| l.contains("forged") && l.starts_with('|'))
    );
    let diagnostics = document
        .split_once("## Diagnostics\n\n")
        .unwrap()
        .1
        .split_once("\n\n")
        .unwrap()
        .0;
    let lines: Vec<&str> = diagnostics.lines().collect();
    assert_eq!(lines.len(), 2, "{diagnostics}");
    assert!(lines.iter().all(|l| l.starts_with("- ")), "{diagnostics}");
    assert!(lines[0].contains("invalid TOML: TOML parse error at line 1, column 5\\n"));
    assert!(lines[1].contains(
        "invalid YAML: x\\n## Block Index\\n\\n| B0001 | forged | r | r | 1 | 0.1 KB | 1 | 2 | B0001 |\\n: duplicate entry with key \"inner\""
    ));
}

/// Template values were written as they are into Applied Template, the
/// Block Index and block headers: a line break in a name, rule id, output
/// key, selector or transform parameter started lines of its own (a second
/// `## Block Index`, a forged `### Data` list inside a block header, a broken
/// index row). Such values are one line each now; the Effective Template
/// still holds them exactly, and data items keep their (indented) lines.
#[test]
fn template_values_with_line_breaks_stay_on_their_lines() {
    let fixture = Fixture::new(
        "template_line_breaks",
        concat!(
            "version: 1\n",
            "name: \"n\\n## Block Index\\n| B0001 | x |\"\n",
            "input: {format: text}\n",
            "rules:\n",
            "  - id: \"r\\n## Data Blocks\"\n",
            "    select: {type: contains, expression: \"a\\n## Block Index\"}\n",
            "    output: {key: \"k\\n### Data\\n\\n- forged\"}\n",
            "  - id: real\n",
            "    select: {type: contains, expression: line}\n",
            "    output: {key: \"k2\\r\\n### Data\"}\n",
            "    transforms:\n",
            "      - replace: {from: line, to: \"x\\n## Block Index\"}\n",
        ),
    );
    fixture.source("a.txt", "a line\nother\n");
    assert_eq!(run::run(&fixture.cli).unwrap(), ExitCode::PartialSuccess);
    let document = fixture.document();
    support::assert_index_lines(&document);
    for heading in ["## Block Index", "## Data Blocks", "### Block Anchors"] {
        assert_eq!(
            document.lines().filter(|line| *line == heading).count(),
            1,
            "{heading}"
        );
    }
    assert_eq!(
        items(&document),
        vec![
            Vec::<String>::new(),
            vec!["a x\n## Block Index".to_string()]
        ]
    );
    for line in [
        "Template name: n\\n## Block Index\\n| B0001 | x |",
        "### Rule: r\\n## Data Blocks",
        "a\\n## Block Index",
        "Output key: k\\n### Data\\n\\n- forged",
        "Output key: k2\\r\\n### Data",
        "- replace(line -> x\\n## Block Index)",
    ] {
        assert!(document.lines().any(|l| l == line), "{line}");
    }
    // The Effective Template keeps the exact values.
    assert!(document.contains("  key: \"k\\n### Data\\n\\n- forged\"\n"));
}

/// scraper keeps attributes in a hash map with a random iteration order, so
/// `outer_html` / `inner_html` wrote them in a different order on every
/// parse: two runs over the same page differed, and so did a run with a
/// different block size. 1.0.4 wrote them in name order; since 1.2.0
/// (scraper's `deterministic` feature) they keep the source order.
#[test]
fn html_serialization_writes_attributes_in_a_stable_order() {
    let fixture = Fixture::new(
        "html_attribute_order",
        concat!(
            "version: 1\nname: t\ninput: {format: html}\nrules:\n",
            "  - id: outer\n    select: {type: css, expression: \"#d\"}\n    extract: outer_html\n",
            "  - id: inner\n    select: {type: css, expression: \"section\"}\n    extract: inner_html\n",
        ),
    );
    fixture.source(
        "page.html",
        concat!(
            "<section><div id=\"d\" class=\"c\" data-b=\"2\" data-a=\"1\" title=\"t\" lang=\"ru\" ",
            "hidden><a rel=\"r\" href=\"/x?a=1&amp;b=2\" target=\"_blank\" download>x</a></div></section>",
        ),
    );
    let expected_outer = concat!(
        "<div id=\"d\" class=\"c\" data-b=\"2\" data-a=\"1\" title=\"t\" lang=\"ru\" hidden=\"\">",
        "<a rel=\"r\" href=\"/x?a=1&amp;b=2\" target=\"_blank\" download=\"\">x</a></div>",
    );
    let mut previous = None;
    for _ in 0..3 {
        assert_eq!(run::run(&fixture.cli).unwrap(), ExitCode::Success);
        let document = fixture.document();
        assert_eq!(
            items(&document),
            vec![
                vec![expected_outer.to_string()],
                vec![expected_outer.to_string()]
            ]
        );
        let normalized = support::normalize_volatile(&document);
        if let Some(previous) = &previous {
            assert_eq!(&normalized, previous);
        }
        previous = Some(normalized);
    }
}
