//! One regression test (or group) per limitation that the 1.0.4 CHANGELOG
//! listed as known and 1.2.0 fixes. Each test reproduces the input of the
//! original finding and fails on 1.1.0.

use any_txt_parser::limits::ExitCode;
use any_txt_parser::render::unescape_anchor_text;
use any_txt_parser::run;
use std::path::Path;
use std::process::Command;

mod support;

use support::Fixture;

/// The decoded data of all blocks, in order.
fn decoded_items(document: &str) -> Vec<String> {
    support::blocks(document)
        .iter()
        .flat_map(support::data_items)
        .map(|item| unescape_anchor_text(&item).into_owned())
        .collect()
}

/// The decoded data of the block of `rule`.
fn rule_items(document: &str, rule: &str) -> Vec<String> {
    let block = support::blocks(document)
        .into_iter()
        .find(|block| block.text.contains(&format!("\nRule: {rule}\n")))
        .unwrap_or_else(|| panic!("no block for rule {rule}"));
    support::data_items(&block)
        .iter()
        .map(|item| unescape_anchor_text(item).into_owned())
        .collect()
}

/// The text of the `## Section` up to the next level-2 heading.
fn section<'a>(document: &'a str, heading: &str) -> &'a str {
    let start = document
        .find(&format!("\n## {heading}\n"))
        .unwrap_or_else(|| panic!("no section {heading}"));
    let rest = &document[start + heading.len() + 5..];
    &rest[..rest.find("\n## ").unwrap_or(rest.len())]
}

fn binary(dir: &Path, args: &[&str]) -> (i32, String, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_any_txt_parser"))
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    (
        output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

// 1. `stable_deduplicate` compared values after `max_item_chars`, so
// distinct records with a common start were dropped as duplicates.
#[test]
fn stable_deduplicate_compares_values_before_max_item_chars() {
    let template = "version: 1\nname: t\ninput: {format: text}\ndocument: {stable_deduplicate: true}\nrules:\n  - id: r\n    select: {type: regex, expression: \".+\"}\n    output: {key: values}\n    limits: {max_item_chars: 10}\n";
    let fixture = Fixture::new("dedup_prefix", template);
    fixture.source(
        "a.log",
        "aaaaaaaaaaX\naaaaaaaaaaY\naaaaaaaaaaX\nshort\nshort\n",
    );
    assert_eq!(run::run(&fixture.cli).unwrap(), ExitCode::PartialSuccess);
    let document = fixture.document();
    // X and Y are different records; only the repeated X and `short` are
    // duplicates.
    assert_eq!(
        decoded_items(&document),
        ["aaaaaaaaaa", "aaaaaaaaaa", "short"]
    );
    assert!(
        document.contains(
            "### Warnings\n\n- 2 item(s) truncated to max_item_chars: item 1 (11 -> 10 chars), item 2 (11 -> 10 chars)\n- 1 truncated item(s) dropped as duplicates by stable_deduplicate\n"
        ),
        "{document}"
    );
}

// 2. A lone CR (classic Mac OS) was not a line break for the text adapter,
// and CRLF left `\r` in XML fragments.
#[test]
fn every_line_break_is_one_rule_for_all_formats() {
    let template = "version: 1\nname: t\nrules:\n  - id: errors\n    when: {format: text}\n    select: {type: contains, expression: \"ERROR\"}\n  - id: second_third\n    when: {format: text}\n    select: {type: line, from: 2, to: 3}\n  - id: last\n    when: {format: text}\n    select: {type: ends_with, expression: \"ms\"}\n  - id: element\n    when: {format: xml}\n    select: {type: xpath, expression: \"/r/a\"}\n  - id: inner\n    when: {format: xml}\n    select: {type: xpath, expression: \"/r/a\"}\n    extract: inner_html\n";
    let fixture = Fixture::new("line_breaks", template);
    fixture.source("mac.log", "first\rERROR second\rthird 5 ms\r");
    fixture.source("mixed.log", "one\r\nERROR two\rthree ms\n");
    fixture.source(
        "data.xml",
        "<r>\r\n  <a k=\"1\">\r\n    <b>x</b>\r\r  </a>\r\n</r>\r\n",
    );
    assert_eq!(run::run(&fixture.cli).unwrap(), ExitCode::Success);
    let document = fixture.document();
    assert!(!document.contains('\r'), "{document:?}");
    support::assert_index_lines(&document);
    let blocks = support::blocks(&document);
    let items = |source: &str, rule: &str| -> Vec<String> {
        let block = blocks
            .iter()
            .find(|b| {
                b.text.contains(&format!("Source: {}\n", source))
                    && b.text.contains(&format!("\nRule: {rule}\n"))
            })
            .unwrap_or_else(|| panic!("no block {source} {rule}"));
        support::data_items(block)
    };
    let mac = fixture
        .dir
        .join("sources")
        .join("mac.log")
        .to_string_lossy()
        .into_owned();
    let mixed = fixture
        .dir
        .join("sources")
        .join("mixed.log")
        .to_string_lossy()
        .into_owned();
    let xml = fixture
        .dir
        .join("sources")
        .join("data.xml")
        .to_string_lossy()
        .into_owned();
    assert_eq!(items(&mac, "errors"), ["ERROR second"]);
    assert_eq!(items(&mac, "second_third"), ["ERROR second\nthird 5 ms"]);
    assert_eq!(items(&mac, "last"), ["third 5 ms"]);
    assert_eq!(items(&mixed, "errors"), ["ERROR two"]);
    assert_eq!(items(&mixed, "second_third"), ["ERROR two\nthree ms"]);
    assert_eq!(
        items(&xml, "element"),
        ["<a k=\"1\">\n    <b>x</b>\n\n  </a>"]
    );
    assert_eq!(items(&xml, "inner"), ["\n    <b>x</b>\n\n  "]);

    // recon counts and shows the same lines.
    let (code, stdout, _) = binary(&fixture.dir, &["recon", "sources/mac.log", "--json"]);
    assert_eq!(code, 0, "{stdout}");
    let recon: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(recon["files"][0]["lines"], 3);
    assert_eq!(
        recon["files"][0]["sample"],
        serde_json::json!(["first", "ERROR second", "third 5 ms"])
    );
}

// 3. TOML tables were published with their keys in name order.
#[test]
fn toml_tables_keep_the_source_key_order() {
    let template = "version: 1\nname: t\nrules:\n  - id: pool\n    select: {type: path, expression: \"[database].pool\"}\n  - id: roles\n    select: {type: path, expression: \"$.servers.*.role\"}\n";
    let fixture = Fixture::new("toml_order", template);
    fixture.source(
        "c.toml",
        "[database.pool]\nmin = 2\nmax = 16\n\n[servers.beta]\nrole = \"replica\"\n\n[servers.alpha]\nrole = \"primary\"\n",
    );
    assert_eq!(run::run(&fixture.cli).unwrap(), ExitCode::Success);
    let document = fixture.document();
    assert_eq!(rule_items(&document, "pool"), ["{\"min\":2,\"max\":16}"]);
    assert_eq!(rule_items(&document, "roles"), ["replica", "primary"]);
}

/// Headings, HTML and table rows (cell texts) of the document as a
/// Markdown renderer sees them.
fn rendered_markdown(document: &str) -> (Vec<String>, Vec<String>, Vec<Vec<String>>) {
    use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
    let mut headings = Vec::new();
    let mut html = Vec::new();
    let mut rows = Vec::new();
    let mut heading: Option<String> = None;
    let mut row: Vec<String> = Vec::new();
    let mut cell: Option<String> = None;
    for event in Parser::new_ext(document, Options::ENABLE_TABLES) {
        match event {
            Event::Start(Tag::Heading { .. }) => heading = Some(String::new()),
            Event::End(TagEnd::Heading(_)) => headings.push(heading.take().unwrap()),
            Event::Text(text) | Event::Code(text) => {
                if let Some(h) = &mut heading {
                    h.push_str(&text);
                }
                if let Some(c) = &mut cell {
                    c.push_str(&text);
                }
            }
            Event::Html(text) | Event::InlineHtml(text) => html.push(text.to_string()),
            Event::Start(Tag::TableCell) => cell = Some(String::new()),
            Event::End(TagEnd::TableCell) => row.push(cell.take().unwrap()),
            Event::End(TagEnd::TableRow) | Event::End(TagEnd::TableHead) => {
                rows.push(std::mem::take(&mut row))
            }
            _ => {}
        }
    }
    (headings, html, rows)
}

// 4. Metadata was written without Markdown escaping: a `## x` selector
// became a heading, `<!-- x` opened an HTML comment and `|` split a cell.
#[test]
fn markdown_in_metadata_is_escaped() {
    let template = "version: 1\nname: \"# name\"\ninput: {format: text}\nrules:\n  - id: \"a|b\"\n    select: {type: contains, expression: \"## x\"}\n    output: {key: \"<!-- k\"}\n  - id: \"1. c\"\n    select: {type: contains, expression: \"<!-- y\"}\n    output: {key: \"---\"}\n  - id: d\n    select: {type: regex, expression: \"^> q\"}\n    output: {key: \"- e|f\"}\n";
    let fixture = Fixture::new("md_metadata", template);
    fixture.source("s.log", "a ## x b\na <!-- y b\n> q\n");
    assert_eq!(run::run(&fixture.cli).unwrap(), ExitCode::Success);
    let document = fixture.document();
    let applied = section(&document, "Applied Template");
    for expected in [
        "Selector:\n\\## x\n",
        "Output key:\n\\<!-- k\n",
        "Selector:\n\\<!-- y\n",
        "Output key:\n\\---\n",
        "Selector:\n^> q\n",
        "Output key:\n\\- e|f\n",
        "### Rule: a|b\n",
    ] {
        assert!(applied.contains(expected), "{expected:?} in {applied}");
    }
    assert!(document.contains(r"| a\|b | \<!-- k |"), "{document}");
    assert!(document.contains(r"| d | - e\|f |"), "{document}");

    let (headings, html, rows) = rendered_markdown(&document);
    for unexpected in ["x", "name", "k", "y", "Selector:", "Output key:"] {
        assert!(
            !headings.iter().any(|h| h.trim() == unexpected),
            "metadata became the heading {unexpected:?}: {headings:?}"
        );
    }
    // Only the block anchors are HTML; nothing of the metadata is hidden.
    assert!(
        html.iter().all(|h| h.starts_with("<!-- BLOCK: ")),
        "{html:?}"
    );
    // Every cell keeps its value.
    let index_row = |rule: &str| {
        rows.iter()
            .find(|row| row.len() == 9 && row[2] == rule)
            .unwrap_or_else(|| panic!("no Block Index row for {rule}: {rows:?}"))
    };
    assert_eq!(index_row("a|b")[3], "<!-- k");
    assert_eq!(index_row("1. c")[3], "---");
    assert_eq!(index_row("d")[3], "- e|f");
    // The data keep their text exactly.
    assert_eq!(decoded_items(&document), ["a ## x b", "a <!-- y b", "> q"]);
}

// 5. Self-description gaps.
#[test]
fn split_parent_counts_the_items_of_its_parts() {
    let template = "version: 1\nname: t\ninput: {format: text}\noutput: {block: {target_chars: 900, max_chars: 1000}}\nrules:\n  - id: whole\n    select: {type: line, from: 1, to: 1}\n  - id: many\n    select: {type: regex, expression: \"x\"}\n";
    let fixture = Fixture::new("split_parent", template);
    fixture.source("a.txt", format!("{}\n", "x".repeat(3000)));
    assert_eq!(run::run(&fixture.cli).unwrap(), ExitCode::Success);
    let document = fixture.document();
    support::assert_index_lines(&document);
    let blocks = support::blocks(&document);
    let parent = blocks.iter().find(|b| b.id == "B0001").unwrap();
    let parts = blocks.iter().filter(|b| b.id.starts_with("B0001-")).count();
    assert!(parts > 2, "{document}");
    assert!(parent.text.contains("\nItems: 1\n"), "{}", parent.text);
    assert!(
        parent
            .text
            .contains("### Data\n\n(in the continuation parts)\n"),
        "{}",
        parent.text
    );
    assert!(
        document.contains("| B0001 | ") && document.contains(" | whole | whole | 1 | "),
        "{document}"
    );
}

#[test]
fn applied_template_shows_extract_modes_and_readable_selectors() {
    let template = "version: 1\nname: t\nrules:\n  - id: links\n    when: {format: html}\n    select: {type: css, expression: \"a\"}\n    extract: attr:href\n  - id: titles\n    when: {format: html}\n    select: {type: css, expression: \"h1\"}\n  - id: install\n    when: {format: markdown}\n    select: {type: markdown, node: h, pattern: '^Inst\\w+'}\n  - id: any_heading\n    when: {format: markdown}\n    select: {type: markdown, node: heading}\n  - id: head\n    when: {format: text}\n    select: {type: line, from: 1, to: 2}\n";
    let fixture = Fixture::new("applied_extract", template);
    fixture.source("p.html", "<h1>T</h1><a href=\"/x\">x</a>");
    fixture.source("r.md", "# Install\n");
    fixture.source("n.txt", "a\nb\n");
    assert_eq!(run::run(&fixture.cli).unwrap(), ExitCode::Success);
    let document = fixture.document();
    let applied = section(&document, "Applied Template");
    for expected in [
        "Selector type: css\nSelector:\na\n\nOutput key:\nlinks\n\nExtract: attr:href\nApplies to format: html\n",
        // The default mode is not written, as in Effective Template.
        "Selector type: css\nSelector:\nh1\n\nOutput key:\ntitles\n\nApplies to format: html\nTransforms:\n",
        "Selector type: markdown\nSelector:\nheading:^Inst\\w+\n",
        "Selector type: markdown\nSelector:\nheading\n",
        "Selector type: line\nSelector:\n1-2\n",
    ] {
        assert!(applied.contains(expected), "{expected:?} in {applied}");
    }
    assert!(!applied.contains("pattern="), "{applied}");
}

#[test]
fn files_table_shows_each_file_encoding_and_execution_the_exit_code() {
    let template =
        "version: 1\nname: t\nrules:\n  - id: r\n    select: {type: contains, expression: \"a\"}\n";
    let fixture = Fixture::new("encodings", template);
    fixture.source("bom.txt", b"\xEF\xBB\xBFabc\n");
    fixture.source("plain.txt", "bcd\n");
    assert_eq!(run::run(&fixture.cli).unwrap(), ExitCode::PartialSuccess);
    let document = fixture.document();
    let files = section(&document, "Files");
    assert!(
        files.contains("| Path | Size | Modified | Format | Status | Encoding | Hash |\n|---|---:|---|---|---|---|---|\n"),
        "{files}"
    );
    let row = |name: &str| {
        files
            .lines()
            .find(|line| line.contains(name))
            .unwrap_or_else(|| panic!("no row for {name}"))
            .to_string()
    };
    assert!(
        row("bom.txt").ends_with(" | text | success | utf-8 (BOM stripped) | - |"),
        "{files}"
    );
    assert!(
        row("plain.txt").ends_with(" | text | warning | utf-8 | - |"),
        "{files}"
    );
    let execution = section(&document, "Execution");
    assert!(
        execution.contains("Run status: partial_success\nExit code: 1\n"),
        "{execution}"
    );

    std::fs::remove_file(fixture.dir.join("sources").join("plain.txt")).unwrap();
    assert_eq!(run::run(&fixture.cli).unwrap(), ExitCode::Success);
    assert!(
        fixture
            .document()
            .contains("Run status: success\nExit code: 0\n")
    );
}

#[test]
fn rule_not_applicable_blocks_say_why() {
    let template = "version: 1\nname: t\nrules:\n  - id: links\n    select: {type: css, expression: \"a\"}\n  - id: users\n    select: {type: jsonpath, expression: \"$.users[*]\"}\n";
    let fixture = Fixture::new("not_applicable", template);
    fixture.source("u.json", "{\"users\": [\"ann\"]}");
    assert_eq!(run::run(&fixture.cli).unwrap(), ExitCode::PartialSuccess);
    let document = fixture.document();
    let block = support::blocks(&document)
        .into_iter()
        .find(|b| b.text.contains("\nRule: links\n"))
        .unwrap();
    assert!(
        block.text.contains("Status: rule not applicable\n"),
        "{}",
        block.text
    );
    assert!(
        block.text.contains(
            "### Warnings\n\n- rule not applicable: selector type css does not apply to json files (they take jsonpath, path)\n"
        ),
        "{}",
        block.text
    );
}

// 6. XML with a DOCTYPE and without the .xml extension was sniffed as text.
#[test]
fn doctype_xml_without_extension_is_sniffed_as_xml() {
    let template = "version: 1\nname: t\nrules:\n  - id: value\n    select: {type: xpath, expression: \"/plist/dict/string/text()\"}\n";
    let fixture = Fixture::new("doctype_sniff", template);
    let plist = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE plist [\n  <!ENTITY app \"Demo\">\n]>\n<plist version=\"1.0\"><dict><key>Name</key><string>&app;</string></dict></plist>\n";
    fixture.source("Info", plist);
    fixture.source("settings.conf", plist);
    assert_eq!(run::run(&fixture.cli).unwrap(), ExitCode::Success);
    let document = fixture.document();
    assert_eq!(decoded_items(&document), ["Demo", "Demo"]);
    let files = section(&document, "Files");
    assert_eq!(files.matches(" | xml | ").count(), 2, "{files}");

    // An entity bomb behind a DOCTYPE is XML whose DTD is rejected, not
    // text whose lines are searched.
    let bomb = "<?xml version=\"1.0\"?>\n<!DOCTYPE r [\n<!ENTITY a \"aaaaaaaaaa\">\n<!ENTITY b \"&a;&a;&a;&a;&a;&a;&a;&a;&a;&a;\">\n<!ENTITY c \"&b;&b;&b;&b;&b;&b;&b;&b;&b;&b;\">\n<!ENTITY d \"&c;&c;&c;&c;&c;&c;&c;&c;&c;&c;\">\n<!ENTITY e \"&d;&d;&d;&d;&d;&d;&d;&d;&d;&d;\">\n<!ENTITY f \"&e;&e;&e;&e;&e;&e;&e;&e;&e;&e;\">\n<!ENTITY g \"&f;&f;&f;&f;&f;&f;&f;&f;&f;&f;\">\n]>\n<r>&g;</r>\n";
    std::fs::remove_file(fixture.dir.join("sources").join("Info")).unwrap();
    fixture.source("bomb", bomb);
    assert_eq!(run::run(&fixture.cli).unwrap(), ExitCode::PartialSuccess);
    let document = fixture.document();
    let files = section(&document, "Files");
    assert!(
        files
            .lines()
            .any(|l| l.contains("bomb") && l.contains(" | xml | error | ")),
        "{files}"
    );
    assert!(document.contains("XML DTD rejected"), "{document}");

    let (code, stdout, _) = binary(&fixture.dir, &["recon", "sources/settings.conf", "--json"]);
    assert_eq!(code, 0, "{stdout}");
    let recon: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(recon["files"][0]["format"], "xml");
    assert_eq!(recon["files"][0]["format_source"], "content");
}

// 7. `search` returned the sentence around a match, which cuts code at
// every `.`: a line context is now the default for code files.
#[test]
fn search_shows_whole_lines_in_code_and_sentences_in_prose() {
    let dir = std::env::temp_dir().join(format!("atp_known_search_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::write(
        dir.join("src").join("lib.rs"),
        "fn load() {\n    let port = config.get(\"port\").unwrap_or(80);\n}\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("README.md"),
        "Intro text. Read the port setting first! Then start.\n",
    )
    .unwrap();
    let root = dir.to_string_lossy().into_owned();

    let (code, stdout, stderr) = binary(&dir, &["search", "port", "--root", &root]);
    assert_eq!(code, 0, "{stderr}");
    assert!(
        stdout.contains("src/lib.rs:2: let port = config.get(\"port\").unwrap_or(80);\n"),
        "{stdout}"
    );
    // One hit per line, not per occurrence.
    assert_eq!(stdout.matches("src/lib.rs:").count(), 1, "{stdout}");
    assert!(
        stdout.contains("README.md:1: Read the port setting first!\n"),
        "{stdout}"
    );

    let (code, stdout, _) = binary(
        &dir,
        &[
            "search",
            "port",
            "src/lib.rs",
            "--root",
            &root,
            "--context",
            "sentence",
        ],
    );
    assert_eq!(code, 0);
    assert!(
        stdout.contains("src/lib.rs:2: let port = config."),
        "{stdout}"
    );
    let (code, stdout, _) = binary(
        &dir,
        &[
            "search",
            "port",
            "README.md",
            "--root",
            &root,
            "--context",
            "line",
            "--json",
        ],
    );
    assert_eq!(code, 0);
    let hits: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(
        hits[0]["text"],
        "Intro text. Read the port setting first! Then start."
    );
    let (code, _, stderr) = binary(&dir, &["search", "port", "--context", "word"]);
    assert_eq!(code, 3, "{stderr}");
    let _ = std::fs::remove_dir_all(&dir);
}

// 8. HTML inner_html/outer_html wrote attributes in name order.
#[test]
fn html_fragments_keep_the_source_attribute_order() {
    let template = "version: 1\nname: t\ninput: {format: html}\nrules:\n  - id: outer\n    select: {type: css, expression: \"a\"}\n    extract: outer_html\n  - id: inner\n    select: {type: css, expression: \"p\"}\n    extract: inner_html\n";
    let fixture = Fixture::new("html_attrs", template);
    fixture.source(
        "p.html",
        "<p><a href=\"/x\" id=\"i\" class=\"c\" data-b=\"2\" data-a=\"1\">x</a></p>",
    );
    assert_eq!(run::run(&fixture.cli).unwrap(), ExitCode::Success);
    let document = fixture.document();
    let expected = "<a href=\"/x\" id=\"i\" class=\"c\" data-b=\"2\" data-a=\"1\">x</a>";
    assert_eq!(rule_items(&document, "outer"), [expected]);
    assert_eq!(rule_items(&document, "inner"), [expected]);
}
