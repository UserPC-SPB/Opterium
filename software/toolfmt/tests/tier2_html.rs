mod support;

// Tier 2, lane "html": `extract: text` reads an element the way a browser
// renders it, skips elements that are not rendered, and an HTML match with
// an empty value is an item like in the other adapters.

use any_txt_parser::limits::ExitCode;
use any_txt_parser::run;

use support::Fixture;

const PAGE: &str = "<!DOCTYPE html>
<html>
<head>
  <title>Water facts</title>
  <style>p { color: red; }</style>
  <script>var tracking = 'head script';</script>
  <script type=\"application/ld+json\">{\"@type\": \"Article\",
    \"name\": \"Water\"}</script>
</head>
<body>
  <article>
    <h1>H<sub>2</sub>O</h1>
    <p>Hello <b>wor</b>ld, <a href=\"/w\">water</a>
       is <em>wet</em>.</p>
    <ul>
      <li>Boils at <strong>100</strong>&nbsp;°C</li>
      <li>Freezes at <span>0</span> °C</li>
    </ul>
    <p>Line one<br>Line two</p>
  </article>
  <noscript>Please enable JavaScript</noscript>
  <template><p>template text</p></template>
  <script>document.write('body script');</script>
</body>
</html>
";

const ARTICLE: &str =
    "H2O\nHello world, water is wet.\nBoils at 100\u{a0}°C\nFreezes at 0 °C\nLine one\nLine two";

fn css_rule(id: &str, expression: &str, extra: &str) -> String {
    format!(
        "  - id: {id}\n    select: {{type: css, expression: \"{expression}\"}}\n    output: {{key: {id}}}\n{extra}"
    )
}

fn template(rules: &[String]) -> String {
    format!(
        "version: 1\nname: html_lane\ninput: {{format: html}}\nrules:\n{}",
        rules.concat()
    )
}

/// Runs the template on one HTML file; returns the decoded items of every
/// block (one block per rule here) and the document.
fn run_page(name: &str, template: &str, html: &str) -> (Vec<Vec<String>>, String) {
    let fixture = Fixture::new(name, template);
    fixture.source("page.html", html);
    assert_eq!(run::run(&fixture.cli).unwrap(), ExitCode::Success);
    let document = fixture.document();
    support::assert_index_lines(&document);
    support::assert_total_characters(&document);
    let items = support::blocks(&document)
        .iter()
        .map(support::data_items)
        .collect();
    (items, document)
}

/// The value of a `Field: value` header line in the block of `rule`.
fn block_field(document: &str, rule: &str, field: &str) -> String {
    let blocks = support::blocks(document);
    let block = blocks
        .iter()
        .find(|block| block.text.contains(&format!("\nRule: {rule}\n")))
        .unwrap_or_else(|| panic!("no block for rule {rule}"));
    block
        .text
        .lines()
        .find_map(|line| line.strip_prefix(field))
        .unwrap_or_else(|| panic!("no {field} in block of rule {rule}"))
        .to_string()
}

#[test]
fn inline_markup_joins_words_and_block_boundaries_end_lines() {
    let template = template(&[
        css_rule("heading", "h1", ""),
        css_rule("paragraphs", "article p", ""),
        css_rule("items", "li", ""),
        css_rule("article", "article", ""),
        css_rule(
            "flat",
            "article",
            "    transforms: [normalize_whitespace]\n",
        ),
    ]);
    let (items, document) = run_page("inline_block", &template, PAGE);
    assert_eq!(items[0], ["H2O"]);
    assert_eq!(
        items[1],
        ["Hello world, water is wet.", "Line one\nLine two"]
    );
    assert_eq!(items[2], ["Boils at 100\u{a0}°C", "Freezes at 0 °C"]);
    assert_eq!(items[3], [ARTICLE]);
    assert_eq!(
        items[4],
        ["H2O Hello world, water is wet. Boils at 100 °C Freezes at 0 °C Line one Line two"]
    );
    // A multi-line item is published as continuation lines of one item.
    assert!(document.contains("- H2O\n    Hello world, water is wet.\n    Boils at"));
}

#[test]
fn scripts_styles_templates_and_head_are_not_page_text() {
    let template = template(&[
        css_rule("page", "html", ""),
        css_rule("body", "body", ""),
        css_rule("json_ld", "script[type='application/ld+json']", ""),
        css_rule("title", "title", ""),
        css_rule("noscript", "noscript", ""),
    ]);
    let (items, document) = run_page("hidden", &template, PAGE);
    assert_eq!(items[0], [ARTICLE]);
    assert_eq!(items[1], [ARTICLE]);
    let data: String = support::blocks(&document)[..2]
        .iter()
        .flat_map(support::data_items)
        .collect();
    for hidden in [
        "Water facts",
        "color: red",
        "tracking",
        "@type",
        "Please enable",
        "template text",
        "document.write",
    ] {
        assert!(!data.contains(hidden), "{hidden:?} leaked into page text");
    }
    // Selected directly, a non-rendered element yields its raw text.
    assert_eq!(
        items[2],
        ["{\"@type\": \"Article\",\n    \"name\": \"Water\"}"]
    );
    assert_eq!(items[3], ["Water facts"]);
    assert_eq!(items[4], ["Please enable JavaScript"]);
}

const TABLE: &str = "<html><body>
<table>
  <tr><th></th><th>Q1</th><th>Q2</th></tr>
  <tr><th>Revenue</th><td>10</td><td></td></tr>
  <tr><th>Cost</th><td> </td><td>7</td></tr>
</table>
<img src=\"a.png\" alt=\"\"><img src=\"b.png\"><img src=\"c.png\" alt=\" Cat \">
<ul><li>one</li><li></li></ul>
</body></html>
";

#[test]
fn table_rows_are_tab_separated_lines_that_keep_empty_cells() {
    let template = template(&[css_rule("table", "table", ""), css_rule("rows", "tr", "")]);
    let (items, _) = run_page("table_text", &template, TABLE);
    assert_eq!(items[0], ["\tQ1\tQ2\nRevenue\t10\t\nCost\t\t7"]);
    assert_eq!(items[1], ["\tQ1\tQ2", "Revenue\t10\t", "Cost\t\t7"]);
}

// Review fix: hidden inputs and forms the parser puts into rows, fallback
// content of media elements and directly selected non-rendered containers.
const FORM_PAGE: &str = "<html><body>
<table>
  <tr><th>Name</th>
      <input type=\"hidden\" name=\"id\" value=\"7\">
      <th>Age</th></tr>
  <tr><td>Bob</td><form action=\"/edit\"><td>42</td></form></tr>
</table>
<p>Watch: <video src=\"a.mp4\">Your browser does not support video.</video></p>
<template id=\"row\"><p>One</p><p>Two</p></template>
<datalist id=\"sizes\"><option>S</option><option>M</option></datalist>
</body></html>
";

#[test]
fn non_rendered_markup_neither_breaks_table_rows_nor_glues_words() {
    let template = template(&[
        css_rule("rows", "tr", ""),
        css_rule("table", "table", ""),
        css_rule("watch", "body > p", ""),
        css_rule("fallback", "video", ""),
        css_rule("row_template", "template", ""),
        css_rule("sizes", "datalist", ""),
    ]);
    let (items, _) = run_page("non_rendered", &template, FORM_PAGE);
    assert_eq!(items[0], ["Name\tAge", "Bob\t42"]);
    assert_eq!(items[1], ["Name\tAge\nBob\t42"]);
    assert_eq!(items[2], ["Watch:"]);
    assert_eq!(items[3], ["Your browser does not support video."]);
    assert_eq!(items[4], ["One\nTwo"]);
    assert_eq!(items[5], ["S\nM"]);
}

#[test]
fn empty_matches_are_items_and_drop_empty_removes_them() {
    let template = template(&[
        css_rule("cells", "td", ""),
        css_rule("cells_kept", "td", "    transforms: [drop_empty]\n"),
        css_rule("headers", "th", ""),
        css_rule("cell_html", "td", "    extract: inner_html\n"),
        css_rule("alts", "img", "    extract: attr:alt\n"),
        css_rule("sources", "img", "    extract: attr:src\n"),
        css_rule("list", "li", ""),
    ]);
    let (items, document) = run_page("empty_items", &template, TABLE);
    assert_eq!(items[0], ["10", "", "", "7"]);
    assert_eq!(block_field(&document, "cells", "Items: "), "4");
    assert_eq!(items[1], ["10", "7"]);
    assert_eq!(block_field(&document, "cells_kept", "Items: "), "2");
    assert_eq!(items[2], ["", "Q1", "Q2", "Revenue", "Cost"]);
    assert_eq!(items[3], ["10", "", " ", "7"]);
    // An empty attribute is a value; a missing attribute is not a match.
    assert_eq!(items[4], ["", "Cat"]);
    assert_eq!(items[5], ["a.png", "b.png", "c.png"]);
    assert_eq!(items[6], ["one", ""]);
    assert_eq!(block_field(&document, "list", "Status: "), "success");
}
