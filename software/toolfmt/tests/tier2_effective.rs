//! Tier-2 lane "effective": the `## Effective Template` section of the
//! output is a valid template in canonical DSL form. Cut out of the document
//! exactly as written and compiled again, it yields the same effective
//! configuration (spec §3, §9, §11, §66: the output is self-describing).

use any_txt_parser::limits::ExitCode;
use any_txt_parser::render::ANCHOR_MARK;
use any_txt_parser::run;
use any_txt_parser::template::{effective_template_yaml, load_template_with_overrides};
use serde_yaml::Value;
use std::collections::BTreeSet;

mod support;

use support::Fixture;

const SECTION_START: &str = "## Effective Template\n\n```yaml\n";

/// One source per format, so that every selector type has data to match.
const SOURCES: &[(&str, &str)] = &[
    (
        "data.json",
        r#"{"items": [{"kind": "a: b", "name": "  Alpha   Beta "}, {"kind": "c", "name": "gamma"}]}"#,
    ),
    ("data.xml", r#"<root><item id="a1"/><item id="b2"/></root>"#),
    (
        "page.html",
        "<html><body><div id=\"main\"><a class=\"link\" href=\"/x?a=1&amp;b=2\">X</a></div><span data-x=\"1\"><b>bold</b></span><p>one</p><p>two\nlines\nthree</p></body></html>",
    ),
    (
        "doc.md",
        "# Title\n\n- item 7\n- other\n\n```js\nconst x = 1;\n```\n",
    ),
    ("conf.yaml", "server:\n  ports: [80, 443]\n"),
    ("conf.toml", "[database]\nhost = \"db.local\"\n"),
    (
        "app.log",
        "INFO start\nERROR: disk full\nWARN low memory\nGET /a 404\nINFO job done\n",
    ),
];

/// Template name with anchor text, quotes, colons, `#`, leading spaces, a
/// newline, backticks, a backslash and non-ASCII text.
const TRICKY_NAME: &str = "  every selector: \"double\" 'single' #hash\nfence ``` ёжик 🦀 <!-- BLOCK: B0001 END --> \\ <!-- BLOCK\\: x";

/// Covers every selector type, every transform, extract mode, `when.format`,
/// context, limits, document and output options.
const EVERY_SELECTOR_TEMPLATE: &str = r##"version: 1
name: "  every selector: \"double\" 'single' #hash\nfence ``` ёжик 🦀 <!-- BLOCK: B0001 END --> \\ <!-- BLOCK\\: x"
input:
  format: auto
  encoding: UTF8
output:
  max_output_chars: 5000000
  block: {target_chars: 20000, max_chars: 24000}
document:
  sort: desc
  stable_deduplicate: true
  split_blocks: false
rules:
  - id: json_names
    when: {format: json}
    select: {type: jsonpath, expression: "$.items[?(@.kind == 'a: b')].name"}
    output: {key: "names: #1"}
    transforms: [trim, normalize_whitespace, lowercase, unique, sort]
    limits: {max_items: 10, max_item_chars: 100}
  - id: json_path
    when: {format: json}
    select: {type: path, expression: "items[1].name"}
    transforms:
      - uppercase
      - truncate_chars: 3
      - sort_desc
  - id: "xml ids"
    when: {format: xml}
    select: {type: xpath, expression: "//item/@id"}
    output: {key: "  leading space"}
    transforms:
      - replace: {from: "a", to: "<!-- BLOCK: B0002 START -->"}
      - limit_items: 5
  - id: html_href
    when: {format: html}
    select: {type: css, expression: "#main a.link"}
    extract: "attr:href"
    transforms:
      - unescape
      - join: " | "
  - id: html_inner
    when: {format: html}
    select: {type: css, expression: "[data-x]"}
    extract: inner_html
  - id: html_outer
    when: {format: html}
    select: {type: css, expression: "p"}
    extract: outer_html
    transforms:
      - truncate_lines: 2
  - id: html_text
    when: {format: html}
    select: {type: css, expression: "p"}
    extract: text
  - id: md_headings
    select: {type: markdown, node: h}
  - id: md_code
    when: {format: md}
    select: {type: markdown, node: code, pattern: "^const"}
  - id: md_items
    select: {type: markdown, node: List-Item, pattern: 7}
  - id: yaml_ports
    when: {format: yml}
    select: {type: path, expression: "server.ports[*]"}
    transforms: [drop_empty]
  - id: toml_host
    when: {format: toml}
    select: {type: path, expression: "[database].host"}
  - id: "log: errors"
    when: {format: text}
    select: {type: regex, expression: "(?i)error: (\\w+)"}
    context: {before: 1, after: 2}
    transforms:
      - regex_replace: {from: "(\\w+)", to: "[$1]: \"x\"\n<!-- BLOCK\\: y\t'z'"}
  - id: "ёжик 🦀"
    select: {type: contains, expression: 404}
    output: {key: "<!-- BLOCK: B0003 END -->"}
  - id: log_starts
    select: {type: starts_with, expression: "WARN "}
    context: {after: 1}
  - id: log_ends
    select: {type: ends_with, expression: " done"}
    context: {before: 2}
  - id: log_lines
    select: {type: line, from: 2, to: 3}
    transforms:
      - join: 0
"##;

/// The Effective Template YAML exactly as written in the document.
fn effective_section(document: &str) -> &str {
    let start = document
        .find(SECTION_START)
        .expect("effective template section missing")
        + SECTION_START.len();
    let len = document[start..]
        .find("\n```\n")
        .expect("unterminated yaml fence")
        + 1;
    &document[start..start + len]
}

fn fixture_with_sources(name: &str, template: &str) -> Fixture {
    let fixture = Fixture::new(name, template);
    for (file, text) in SOURCES {
        fixture.source(file, text);
    }
    fixture
}

fn run_ok(fixture: &Fixture) -> String {
    let code = run::run(&fixture.cli).unwrap_or_else(|e| panic!("run failed: {}", e.message));
    assert!(
        matches!(code, ExitCode::Success | ExitCode::PartialSuccess),
        "{code:?}"
    );
    fixture.document()
}

/// Writes `yaml` to a file, compiles it as a template and returns the
/// effective YAML of the result.
fn recompile(fixture: &Fixture, yaml: &str) -> String {
    let path = fixture.dir.join("effective.yml");
    std::fs::write(&path, yaml).unwrap();
    let compiled = load_template_with_overrides(&path, None, None)
        .unwrap_or_else(|e| panic!("effective template does not compile: {e}\n{yaml}"));
    effective_template_yaml(&compiled)
}

/// A fixture with the same sources as `fixture` and `template`.
fn same_sources(name: &str, fixture: &Fixture, template: &str) -> Fixture {
    let copy = Fixture::new(name, template);
    for entry in std::fs::read_dir(fixture.dir.join("sources")).unwrap() {
        let entry = entry.unwrap();
        copy.source(
            entry.file_name().to_str().unwrap(),
            std::fs::read(entry.path()).unwrap(),
        );
    }
    copy
}

/// Data blocks without the lines that legitimately differ between runs: the
/// fixture directory (`Source:`) and where a CLI flag became template text
/// (`Format source:`).
fn comparable_blocks(document: &str) -> Vec<String> {
    support::blocks(document)
        .iter()
        .map(|block| {
            block
                .text
                .lines()
                .filter(|line| {
                    !line.starts_with("Source: ") && !line.starts_with("Format source: ")
                })
                .collect::<Vec<_>>()
                .join("\n")
        })
        .collect()
}

/// Runs `fixture`, checks the Effective Template and returns it as written:
/// - it is the emitter's text, unchanged by anchor escaping;
/// - only anchor lines contain the anchor mark;
/// - compiled again it yields the same effective YAML;
/// - used as the template for the same sources it yields the same blocks.
fn assert_round_trip(name: &str, fixture: &Fixture) -> String {
    let document = run_ok(fixture);
    let effective = effective_section(&document).to_string();
    let original = load_template_with_overrides(
        &fixture.cli.template,
        fixture.cli.format.as_deref(),
        fixture.cli.encoding.as_deref(),
    )
    .unwrap();
    if fixture.cli.max_block_chars.is_none() && fixture.cli.max_output_size.is_none() {
        assert_eq!(effective, effective_template_yaml(&original));
    }
    assert!(
        !effective.contains(ANCHOR_MARK),
        "anchor mark in effective template:\n{effective}"
    );
    support::assert_index_lines(&document);
    assert_eq!(recompile(fixture, &effective), effective);

    let rerun = same_sources(&format!("{name}_rerun"), fixture, &effective);
    let rerun_document = run_ok(&rerun);
    assert_eq!(effective_section(&rerun_document), effective);
    assert_eq!(
        comparable_blocks(&rerun_document),
        comparable_blocks(&document)
    );
    effective
}

fn parse(yaml: &str) -> Value {
    serde_yaml::from_str(yaml).unwrap()
}

fn rule<'a>(effective: &'a Value, id: &str) -> &'a Value {
    effective["rules"]
        .as_sequence()
        .unwrap()
        .iter()
        .find(|rule| rule["id"].as_str() == Some(id))
        .unwrap_or_else(|| panic!("rule {id:?} missing"))
}

fn transform_names(effective: &Value) -> BTreeSet<String> {
    effective["rules"]
        .as_sequence()
        .unwrap()
        .iter()
        .filter_map(|rule| rule["transforms"].as_sequence())
        .flatten()
        .map(|t| match t {
            Value::String(name) => name.clone(),
            Value::Mapping(m) => m.keys().next().unwrap().as_str().unwrap().to_string(),
            other => panic!("malformed transform {other:?}"),
        })
        .collect()
}

#[test]
fn effective_template_round_trips_for_every_selector_transform_and_option() {
    let fixture = fixture_with_sources("every_selector", EVERY_SELECTOR_TEMPLATE);
    let yaml = assert_round_trip("every_selector", &fixture);
    let effective = parse(&yaml);

    // The template really covers the whole DSL.
    let types: BTreeSet<&str> = effective["rules"]
        .as_sequence()
        .unwrap()
        .iter()
        .map(|rule| rule["select"]["type"].as_str().unwrap())
        .collect();
    assert_eq!(
        types,
        BTreeSet::from([
            "contains",
            "css",
            "ends_with",
            "jsonpath",
            "line",
            "markdown",
            "path",
            "regex",
            "starts_with",
            "xpath"
        ])
    );
    assert_eq!(
        transform_names(&effective),
        [
            "drop_empty",
            "join",
            "limit_items",
            "lowercase",
            "normalize_whitespace",
            "regex_replace",
            "replace",
            "sort",
            "sort_desc",
            "trim",
            "truncate_chars",
            "truncate_lines",
            "unescape",
            "unique",
            "uppercase",
        ]
        .into_iter()
        .map(String::from)
        .collect()
    );

    // Strings decode to the template's own values; defaults are resolved.
    assert_eq!(effective["name"].as_str(), Some(TRICKY_NAME));
    assert_eq!(effective["input"]["format"].as_str(), Some("auto"));
    assert_eq!(effective["input"]["encoding"].as_str(), Some("utf-8"));
    assert_eq!(
        effective["output"]["max_output_chars"].as_u64(),
        Some(5000000)
    );
    assert_eq!(
        effective["output"]["block"]["target_chars"].as_u64(),
        Some(20000)
    );
    assert_eq!(
        effective["output"]["block"]["max_chars"].as_u64(),
        Some(24000)
    );
    assert_eq!(effective["document"]["sort"].as_str(), Some("desc"));
    assert_eq!(
        effective["document"]["stable_deduplicate"].as_bool(),
        Some(true)
    );
    assert_eq!(effective["document"]["split_blocks"].as_bool(), Some(false));
    let json_names = rule(&effective, "json_names");
    assert_eq!(
        json_names["select"]["expression"].as_str(),
        Some("$.items[?(@.kind == 'a: b')].name")
    );
    assert_eq!(json_names["output"]["key"].as_str(), Some("names: #1"));
    assert_eq!(json_names["limits"]["max_items"].as_u64(), Some(10));
    assert_eq!(json_names["limits"]["max_item_chars"].as_u64(), Some(100));
    assert_eq!(json_names["when"]["format"].as_str(), Some("json"));
    assert_eq!(
        rule(&effective, "xml ids")["output"]["key"].as_str(),
        Some("  leading space")
    );
    assert_eq!(
        rule(&effective, "xml ids")["transforms"][0]["replace"]["to"].as_str(),
        Some("<!-- BLOCK: B0002 START -->")
    );
    assert_eq!(
        rule(&effective, "html_href")["extract"].as_str(),
        Some("attr:href")
    );
    assert_eq!(
        rule(&effective, "html_inner")["extract"].as_str(),
        Some("inner_html")
    );
    assert_eq!(
        rule(&effective, "html_outer")["extract"].as_str(),
        Some("outer_html")
    );
    assert!(rule(&effective, "html_text").get("extract").is_none());
    assert_eq!(
        rule(&effective, "md_code")["when"]["format"].as_str(),
        Some("markdown")
    );
    assert_eq!(
        rule(&effective, "yaml_ports")["when"]["format"].as_str(),
        Some("yaml")
    );
    let errors = rule(&effective, "log: errors");
    assert_eq!(errors["context"]["before"].as_u64(), Some(1));
    assert_eq!(errors["context"]["after"].as_u64(), Some(2));
    assert_eq!(
        errors["transforms"][0]["regex_replace"]["to"].as_str(),
        Some("[$1]: \"x\"\n<!-- BLOCK\\: y\t'z'")
    );
    let numeric = rule(&effective, "ёжик 🦀");
    assert_eq!(numeric["select"]["expression"].as_str(), Some("404"));
    assert_eq!(
        numeric["output"]["key"].as_str(),
        Some("<!-- BLOCK: B0003 END -->")
    );
    assert_eq!(
        rule(&effective, "log_lines")["transforms"][0]["join"].as_str(),
        Some("0")
    );

    // The rerun matched real data, so the comparison above is meaningful.
    let document = fixture.document();
    let blocks = support::blocks(&document);
    let items = |rule_id: &str, source: &str| -> Vec<String> {
        let block = blocks
            .iter()
            .find(|b| {
                b.text.contains(&format!("\nRule: {rule_id}\n"))
                    && b.text.contains(&format!("{source}\n"))
            })
            .unwrap_or_else(|| panic!("no block for {rule_id} on {source}"));
        support::data_items(block)
    };
    assert_eq!(items("json_names", "data.json"), vec!["alpha beta"]);
    assert_eq!(
        items("log_lines", "app.log"),
        vec!["ERROR: disk full\nWARN low memory"]
    );
    assert_eq!(items("md_items", "doc.md"), vec!["item 7"]);
    assert_eq!(items("toml_host", "conf.toml"), vec!["db.local"]);
}

#[test]
fn regex_replace_line_and_markdown_selectors_are_written_in_dsl_form() {
    let template = "version: 1\nname: dsl_form\nrules:\n  - id: lines\n    select: {type: line, from: 1, to: 2}\n    transforms:\n      - regex_replace: {from: \"^(\\\\w+)\", to: \"<$1>\"}\n  - id: headings\n    select: {type: markdown, node: heading, pattern: \"^T\"}\n  - id: any_heading\n    select: {type: markdown, node: heading}\n";
    let fixture = fixture_with_sources("dsl_form", template);
    let yaml = assert_round_trip("dsl_form", &fixture);
    let effective = parse(&yaml);

    let lines = rule(&effective, "lines");
    assert_eq!(lines["select"]["from"].as_u64(), Some(1));
    assert_eq!(lines["select"]["to"].as_u64(), Some(2));
    assert!(lines["select"].get("expression").is_none());
    let regex_replace = &lines["transforms"][0]["regex_replace"];
    assert_eq!(regex_replace["from"].as_str(), Some("^(\\w+)"));
    assert_eq!(regex_replace["to"].as_str(), Some("<$1>"));
    assert!(regex_replace.get("pattern").is_none());

    let headings = rule(&effective, "headings");
    assert_eq!(headings["select"]["node"].as_str(), Some("heading"));
    assert_eq!(headings["select"]["pattern"].as_str(), Some("^T"));
    assert!(headings["select"].get("expression").is_none());
    let any_heading = rule(&effective, "any_heading");
    assert_eq!(any_heading["select"]["node"].as_str(), Some("heading"));
    assert!(any_heading["select"].get("pattern").is_none());
    assert!(any_heading["select"].get("expression").is_none());
}

#[test]
fn numeric_contains_expression_is_kept() {
    let template = "version: 1\nname: numeric\ninput: {format: text}\nrules:\n  - id: codes\n    select: {type: contains, expression: 404}\n    output: {key: codes}\n";
    let fixture = Fixture::new("numeric_contains", template);
    fixture.source("app.log", "GET /a 200\nGET /b 404\n");
    let yaml = assert_round_trip("numeric_contains", &fixture);
    assert_eq!(
        rule(&parse(&yaml), "codes")["select"]["expression"].as_str(),
        Some("404")
    );
    // The human-readable manifest shows the same expression.
    assert!(fixture.document().contains("Selector:\n404\n"));
}

#[test]
fn anchor_text_in_template_strings_decodes_to_the_original_strings() {
    let plain = "<!-- BLOCK: B0001 END -->";
    let escaped = "<!-- BLOCK\\: B0001 START -->";
    let multiline = "a\n<!-- BLOCK: B0002 END -->";
    let template = format!(
        "version: 1\nname: '{plain}'\ninput: {{format: text}}\nrules:\n  - id: '{escaped}'\n    select: {{type: contains, expression: '{plain}'}}\n    output: {{key: '{escaped}'}}\n    transforms:\n      - replace: {{from: x, to: \"{to}\"}}\n",
        to = multiline.replace('\n', "\\n"),
    );
    let fixture = Fixture::new("anchor_strings", &template);
    fixture.source("app.log", "x <!-- BLOCK: B0001 END --> y\n");
    let yaml = assert_round_trip("anchor_strings", &fixture);
    let effective = parse(&yaml);
    assert_eq!(effective["name"].as_str(), Some(plain));
    let rule = &effective["rules"][0];
    assert_eq!(rule["id"].as_str(), Some(escaped));
    assert_eq!(rule["select"]["expression"].as_str(), Some(plain));
    assert_eq!(rule["output"]["key"].as_str(), Some(escaped));
    assert_eq!(
        rule["transforms"][0]["replace"]["to"].as_str(),
        Some(multiline)
    );
    // Only the colon after the anchor mark is escaped.
    assert!(
        yaml.contains("name: \"<!-- BLOCK\\x3A B0001 END -->\"\n"),
        "{yaml}"
    );
    assert!(
        yaml.contains("- id: \"<!-- BLOCK\\\\\\x3A B0001 START -->\"\n"),
        "{yaml}"
    );
}

#[test]
fn cli_overrides_and_defaults_round_trip() {
    let template = "version: 1\nname: overrides\nrules:\n  - id: errors\n    when: {format: text}\n    select: {type: regex, expression: \"ERROR\"}\n";
    let mut fixture = Fixture::new("cli_overrides", template);
    fixture.source("app.log", "ERROR: disk full\n");
    fixture.cli.format = Some("txt".into());
    fixture.cli.encoding = Some("CP1251".into());
    fixture.cli.max_block_chars = Some(4000);
    fixture.cli.max_output_size = Some("2M".into());
    let effective = parse(&assert_round_trip("cli_overrides", &fixture));
    assert_eq!(effective["input"]["format"].as_str(), Some("text"));
    assert_eq!(effective["input"]["encoding"].as_str(), Some("cp1251"));
    assert_eq!(
        effective["output"]["block"]["max_chars"].as_u64(),
        Some(4000)
    );
    assert_eq!(
        effective["output"]["block"]["target_chars"].as_u64(),
        Some(3500)
    );
    assert_eq!(
        effective["output"]["max_output_chars"].as_u64(),
        Some(2 * 1024 * 1024)
    );

    let fixture = Fixture::new("defaults", template);
    fixture.source("app.log", "ERROR: disk full\n");
    let yaml = assert_round_trip("defaults", &fixture);
    assert_eq!(
        yaml,
        "version: 1\nname: overrides\ninput:\n  format: auto\n  encoding: utf-8\noutput:\n  block:\n    target_chars: 28000\n    max_chars: 32000\ndocument:\n  stable_deduplicate: false\n  sort: none\n  split_blocks: true\nrules:\n- id: errors\n  when:\n    format: text\n  select:\n    type: regex\n    expression: ERROR\n  output:\n    key: errors\n"
    );
}

/// Ordinary one-line strings keep the quoting the Effective Template had
/// before it got its own emitter (serde_yaml's): text that only YAML 1.1
/// reads as a number or boolean stays plain, while document markers,
/// leading-zero digits and real YAML 1.2 values are quoted.
#[test]
fn ordinary_strings_keep_the_previous_quoting() {
    let template = r#"version: 1
name: "2024 report"
input: {format: text}
rules:
  - id: "2fa_codes"
    select: {type: contains, expression: "1_000"}
    output: {key: "off"}
  - id: "yes"
    select: {type: contains, expression: ".5x"}
  - id: "y"
    select: {type: starts_with, expression: "---x"}
  - id: "n"
    select: {type: ends_with, expression: "...y"}
  - id: "1C export"
    select: {type: regex, expression: "012"}
    output: {key: "on"}
  - id: "12:30"
    select: {type: contains, expression: "-1x"}
    output: {key: "true"}
"#;
    let fixture = Fixture::new("previous_quoting", template);
    fixture.source("app.log", "1_000 .5x\n---x ...y\n012 -1x\n");
    let yaml = assert_round_trip("previous_quoting", &fixture);
    assert_eq!(
        yaml,
        concat!(
            "version: 1\nname: 2024 report\ninput:\n  format: text\n  encoding: utf-8\n",
            "output:\n  block:\n    target_chars: 28000\n    max_chars: 32000\n",
            "document:\n  stable_deduplicate: false\n  sort: none\n  split_blocks: true\n",
            "rules:\n",
            "- id: 2fa_codes\n  select:\n    type: contains\n    expression: 1_000\n",
            "  output:\n    key: off\n",
            "- id: yes\n  select:\n    type: contains\n    expression: .5x\n",
            "  output:\n    key: yes\n",
            "- id: y\n  select:\n    type: starts_with\n    expression: '---x'\n",
            "  output:\n    key: y\n",
            "- id: n\n  select:\n    type: ends_with\n    expression: '...y'\n",
            "  output:\n    key: n\n",
            "- id: 1C export\n  select:\n    type: regex\n    expression: '012'\n",
            "  output:\n    key: on\n",
            "- id: 12:30\n  select:\n    type: contains\n    expression: -1x\n",
            "  output:\n    key: 'true'\n",
        )
    );
}
