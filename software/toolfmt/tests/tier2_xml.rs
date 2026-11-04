//! Tier 2, XML lane: extract modes for selected XML elements, documents with
//! a DOCTYPE (and the entity limits that make them safe), and the root
//! element's default namespace in XPath queries.

use any_txt_parser::limits::ExitCode;
use any_txt_parser::run;
use std::time::{Duration, Instant};

mod support;

use support::Fixture;

/// An `input: {format: xml}` template with one rule per `(id, xpath, extract)`.
fn xml_template(rules: &[(&str, &str, Option<&str>)]) -> String {
    let mut template = String::from("version: 1\nname: xml\ninput: {format: xml}\nrules:\n");
    for (id, expression, extract) in rules {
        template.push_str(&format!(
            "  - id: {id}\n    select: {{type: xpath, expression: \"{expression}\"}}\n    output: {{key: {id}}}\n"
        ));
        if let Some(extract) = extract {
            template.push_str(&format!("    extract: \"{extract}\"\n"));
        }
    }
    template
}

/// Data items of every block, in block order (one block per rule here).
fn items(document: &str) -> Vec<Vec<String>> {
    support::blocks(document)
        .iter()
        .map(support::data_items)
        .collect()
}

fn file_row<'a>(document: &'a str, name: &str) -> &'a str {
    document
        .lines()
        .find(|line| line.starts_with("| ") && line.contains(&format!("{} |", name)))
        .unwrap_or_else(|| panic!("no Files row for {name}"))
}

// Item 1: `extract` was ignored for XML; every mode returned the element's
// outer markup, although the docs promise `attr:NAME` for xml.
#[test]
fn xml_extract_modes_select_attributes_and_exact_inner_markup() {
    let template = xml_template(&[
        ("ids", "//item", Some("attr:id")),
        ("langs", "//item", Some("attr:xml:lang")),
        ("inner", "//item", Some("inner_html")),
        ("outer", "//item", Some("outer_html")),
        ("default", "//item", None),
    ]);
    let fixture = Fixture::new("xml_extract_modes", &template);
    let elements = [
        "<item id=\"a\" xml:lang=\"en\">one <b>two</b> &amp; <![CDATA[<raw>]]></item>",
        "<item>no id</item>",
        "<item id=\"\"/>",
    ];
    fixture.source("data.xml", format!("<root>{}</root>", elements.concat()));

    assert_eq!(run::run(&fixture.cli).unwrap(), ExitCode::Success);
    let document = fixture.document();
    support::assert_index_lines(&document);
    let published = items(&document);
    // An absent attribute yields no item; an empty one is kept, as `/@id`.
    assert_eq!(published[0], vec!["a", ""]);
    assert_eq!(published[1], vec!["en"]);
    assert_eq!(
        published[2],
        vec!["one <b>two</b> &amp; <![CDATA[<raw>]]>", "no id", ""]
    );
    assert_eq!(published[3], elements);
    assert_eq!(published[4], elements, "default extract stays outer markup");
}

#[test]
fn element_extract_modes_on_attribute_or_text_xpaths_are_template_errors() {
    for (expression, extract, selected) in [
        ("//item/@id", "attr:id", "attribute values (@id)"),
        ("//item/@id", "inner_html", "attribute values (@id)"),
        ("//item/text()", "outer_html", "text nodes (text())"),
    ] {
        let template = xml_template(&[("r", expression, Some(extract))]);
        let fixture = Fixture::new("xml_extract_mismatch", &template);
        fixture.source("data.xml", "<root><item id=\"1\">x</item></root>");
        let error = run::run(&fixture.cli).unwrap_err();
        assert_eq!(error.code, ExitCode::InvalidTemplate, "{}", error.message);
        assert!(
            error.message.contains(&format!(
                "rule 'r': extract '{extract}' applies to elements, but the xpath selects {selected}"
            )),
            "{}",
            error.message
        );
    }
    let template = xml_template(&[("r", "//item", Some("attr:"))]);
    let fixture = Fixture::new("xml_extract_empty_attr", &template);
    let error = run::run(&fixture.cli).unwrap_err();
    assert_eq!(error.code, ExitCode::InvalidTemplate);
    assert!(error.message.contains("needs an XML attribute name"));
}

// Item 2: any DOCTYPE made the file "invalid XML: XML with DTD detected".
#[test]
fn documents_with_a_doctype_parse_and_expand_internal_entities() {
    let template = xml_template(&[
        ("statements", "//select/@id", None),
        ("db_url", "/config/db/@url", None),
        ("db_host", "/config/db/text()", None),
        ("plist_keys", "/plist/dict/key/text()", None),
    ]);
    let fixture = Fixture::new("xml_doctype", &template);
    fixture.source(
        "a_mapper.xml",
        concat!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\" ?>\n",
            "<!DOCTYPE mapper PUBLIC \"-//mybatis.org//DTD Mapper 3.0//EN\"\n",
            "  \"http://mybatis.org/dtd/mybatis-3-mapper.dtd\">\n",
            "<mapper namespace=\"org.example.UserMapper\">\n",
            "  <select id=\"findUser\">SELECT * FROM users WHERE id = #{id}</select>\n",
            "  <select id=\"listUsers\">SELECT * FROM users</select>\n",
            "</mapper>\n"
        ),
    );
    fixture.source(
        "b_config.xml",
        concat!(
            "<!DOCTYPE config [\n",
            "  <!ENTITY host \"db.local\">\n",
            "  <!ENTITY url \"jdbc://&host;/app\">\n",
            "]>\n",
            "<config><db url=\"&url;\">&host;</db></config>\n"
        ),
    );
    fixture.source(
        "c_info.xml",
        concat!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n",
            "<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" ",
            "\"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n",
            "<plist version=\"1.0\"><dict><key>CFBundleName</key><string>Demo</string>",
            "<key>CFBundleVersion</key><string>7</string></dict></plist>\n"
        ),
    );

    // Each rule matches in one file only, so the other files report
    // "no matches found" warnings, but no file fails to parse.
    assert_eq!(run::run(&fixture.cli).unwrap(), ExitCode::PartialSuccess);
    let document = fixture.document();
    assert!(!document.contains("DTD"), "no DTD error expected");
    for name in ["a_mapper.xml", "b_config.xml", "c_info.xml"] {
        assert!(!file_row(&document, name).contains("| error |"), "{name}");
    }
    let published: Vec<Vec<String>> = items(&document)
        .into_iter()
        .filter(|items| !items.is_empty())
        .collect();
    assert_eq!(
        published,
        vec![
            vec!["findUser".to_string(), "listUsers".to_string()],
            vec!["jdbc://db.local/app".to_string()],
            vec!["db.local".to_string()],
            vec!["CFBundleName".to_string(), "CFBundleVersion".to_string()],
        ]
    );
}

// Item 2, safety: with DTDs allowed, roxmltree would expand these itself.
// Unchecked it needed 38 s for the quadratic case (release build), built
// trees deeper than the 128-level limit, and produced backwards element
// ranges (a slicing panic) for unbalanced entity markup.
#[test]
fn entity_bombs_and_unsafe_entity_markup_are_per_file_errors() {
    let template = xml_template(&[("titles", "//title/text()", None)]);
    let fixture = Fixture::new("xml_entity_bombs", &template);
    let mut laughs = String::from("<!DOCTYPE r [<!ENTITY lol0 \"lol\">");
    for level in 1..=9 {
        let refs = format!("&lol{};", level - 1).repeat(10);
        laughs.push_str(&format!("<!ENTITY lol{level} \"{refs}\">"));
    }
    laughs.push_str("]><r><title>&lol9;</title></r>");
    fixture.source("laughs.xml", &laughs);
    fixture.source(
        "quadratic.xml",
        format!(
            "<!DOCTYPE r [<!ENTITY big \"{}\">]><r><title>{}</title></r>",
            "x".repeat(100_000),
            "&big;".repeat(1000)
        ),
    );
    fixture.source(
        "merge.xml",
        format!(
            "<!DOCTYPE r [<!ENTITY e \"x\">]><r><title>{}</title></r>",
            "&e;".repeat(200_000)
        ),
    );
    fixture.source(
        "recursive.xml",
        "<!DOCTYPE r [<!ENTITY a \"&b;\"><!ENTITY b \"&a;\">]><r><title>&a;</title></r>",
    );
    fixture.source(
        "deep.xml",
        format!(
            "<!DOCTYPE r [<!ENTITY deep \"{}x{}\">]><r>{}&deep;{}<title>t</title></r>",
            "<a>".repeat(100),
            "</a>".repeat(100),
            "<b>".repeat(100),
            "</b>".repeat(100)
        ),
    );
    fixture.source(
        "unbalanced.xml",
        "<!DOCTYPE r [<!ENTITY c \"<b/></a>\">]><r><a>&c;</r>",
    );
    fixture.source("good.xml", "<r><title>still published</title></r>");

    let started = Instant::now();
    assert_eq!(run::run(&fixture.cli).unwrap(), ExitCode::PartialSuccess);
    assert!(started.elapsed() < Duration::from_secs(20));
    let document = fixture.document();
    for (name, message) in [
        (
            "laughs.xml",
            "XML DTD rejected: entity references expand to more than 8388608 bytes (entity expansion bomb)",
        ),
        (
            "quadratic.xml",
            "XML DTD rejected: entity references expand to more than 8388608 bytes",
        ),
        (
            "merge.xml",
            "XML DTD rejected: entity references would make the parser copy more than 1073741824 bytes of text (quadratic entity blowup)",
        ),
        (
            "recursive.xml",
            "XML DTD rejected: entity 'a' is recursive (its expansion refers back to it)",
        ),
        (
            "deep.xml",
            "XML nesting depth exceeds the limit of 128 elements (entity 'deep' adds 100 levels of elements)",
        ),
        (
            "unbalanced.xml",
            "XML DTD rejected: entity 'c' contains markup that is not well-formed (each element must start and end inside the same entity)",
        ),
    ] {
        assert!(
            file_row(&document, name).contains("| xml | error |"),
            "{name}"
        );
        assert!(document.contains(message), "{name}: {message}");
    }
    assert!(file_row(&document, "good.xml").contains("| xml | success |"));
    let published: Vec<Vec<String>> = items(&document)
        .into_iter()
        .filter(|items| !items.is_empty())
        .collect();
    assert_eq!(published, vec![vec!["still published".to_string()]]);
}

// Item 2, review fixes for the DTD limits. Lookups were counted as table
// comparisons, but equal-length names are compared byte by byte: long names
// passed the check and roxmltree then spent seconds per file (17 s for a
// 128 MB one). The text-merge estimate charged every '<' of an entity as a
// merge, rejecting entities made of elements only, and references inside
// comments of an entity made it "recursive" even where they stay text.
#[test]
fn dtd_limits_count_compared_name_bytes_and_only_real_text_merges() {
    let template = xml_template(&[("titles", "//title/text()", None)]);
    let fixture = Fixture::new("xml_dtd_cost_model", &template);
    let name = |index: usize| format!("{}{index:06}", "n".repeat(994));
    let mut names = String::from("<!DOCTYPE r [");
    for index in 0..2000 {
        names.push_str(&format!("<!ENTITY {} ''>", name(index)));
    }
    let reference = format!("&{};", name(1999));
    names.push_str(&format!(
        "]><r><title>{}</title></r>",
        reference.repeat(2000)
    ));
    fixture.source("names.xml", &names);
    fixture.source(
        "markup.xml",
        format!(
            "<!DOCTYPE r [<!ENTITY m \"{}\">]><r><title>elements</title>{}</r>",
            "<a/>".repeat(1000),
            "<b>&m;</b>".repeat(400)
        ),
    );
    let comment = "<!DOCTYPE r [<!ENTITY a \"<!-- &a; -->text\">]>";
    fixture.source("comment.xml", format!("{comment}<r><title>&a;</title></r>"));
    // In an attribute value roxmltree does expand it, so there it recurses.
    fixture.source(
        "comment_attr.xml",
        format!("{comment}<r v=\"&a;\"><title>attr</title></r>"),
    );
    // And an expansion hidden in a comment still counts in attributes.
    let hidden = format!(
        "<!DOCTYPE r [<!ENTITY big \"{}\"><!ENTITY h \"<!--{}-->\">]>",
        "x".repeat(100_000),
        "&big;".repeat(100)
    );
    fixture.source(
        "hidden_attr.xml",
        format!("{hidden}<r v=\"&h;\"><title>hidden</title></r>"),
    );

    let started = Instant::now();
    assert_eq!(run::run(&fixture.cli).unwrap(), ExitCode::PartialSuccess);
    assert!(started.elapsed() < Duration::from_secs(20));
    let document = fixture.document();
    for (name, message) in [
        (
            "names.xml",
            "XML DTD rejected: entity references need entity lookups that compare more than 1073741824 bytes of entity names",
        ),
        (
            "comment_attr.xml",
            "XML DTD rejected: entity 'a' is recursive (its expansion refers back to it)",
        ),
        (
            "hidden_attr.xml",
            "XML DTD rejected: entity references expand to more than 8388608 bytes (entity expansion bomb)",
        ),
    ] {
        assert!(
            file_row(&document, name).contains("| xml | error |"),
            "{name}"
        );
        assert!(document.contains(message), "{name}: {message}");
    }
    for name in ["markup.xml", "comment.xml"] {
        assert!(
            file_row(&document, name).contains("| xml | success |"),
            "{name}"
        );
    }
    let mut published = items(&document).concat();
    published.sort();
    assert_eq!(published, vec!["elements", "text"]);
}

/// Rule id, xpath expression and the values it must publish.
type ExpectedRule = (&'static str, &'static str, Vec<&'static str>);

// Item 3: elements in a root default namespace (pom.xml, Atom, SVG, XHTML)
// matched no unprefixed name test: silent "no matches found".
#[test]
fn unprefixed_xpath_names_match_the_root_default_namespace() {
    let cases: [(&str, &str, Vec<ExpectedRule>); 4] = [
        (
            "pom.xml",
            concat!(
                "<project xmlns=\"http://maven.apache.org/POM/4.0.0\" ",
                "xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\" ",
                "xsi:schemaLocation=\"http://maven.apache.org/POM/4.0.0 pom.xsd\">\n",
                "  <groupId>org.example</groupId><version>1.2.3</version>\n",
                "  <dependencies><dependency><artifactId>junit</artifactId></dependency></dependencies>\n",
                "</project>\n"
            ),
            vec![
                ("version", "/project/version/text()", vec!["1.2.3"]),
                ("deps", "//dependency/artifactId/text()", vec!["junit"]),
                (
                    "schema",
                    "/project/@xsi:schemaLocation",
                    vec!["http://maven.apache.org/POM/4.0.0 pom.xsd"],
                ),
            ],
        ),
        (
            "feed.xml",
            concat!(
                "<feed xmlns=\"http://www.w3.org/2005/Atom\"><title>Feed</title>",
                "<entry><title>E1</title><link href=\"http://a\"/></entry>",
                "<entry><title>E2</title></entry></feed>"
            ),
            vec![
                ("entries", "//entry/title/text()", vec!["E1", "E2"]),
                ("links", "/feed/entry/link/@href", vec!["http://a"]),
            ],
        ),
        (
            "icon.xml",
            concat!(
                "<svg xmlns=\"http://www.w3.org/2000/svg\" xmlns:xlink=\"http://www.w3.org/1999/xlink\">",
                "<g id=\"layer\"><rect width=\"10\"/><use xlink:href=\"#a\"/></g></svg>"
            ),
            vec![
                ("widths", "/svg/g[@id='layer']/rect/@width", vec!["10"]),
                ("hrefs", "//use/@xlink:href", vec!["#a"]),
            ],
        ),
        (
            "page.xml",
            concat!(
                "<!DOCTYPE html PUBLIC \"-//W3C//DTD XHTML 1.0 Strict//EN\" ",
                "\"http://www.w3.org/TR/xhtml1/DTD/xhtml1-strict.dtd\">\n",
                "<html xmlns=\"http://www.w3.org/1999/xhtml\"><head><title>T</title></head>",
                "<body><p>Hello</p><div xmlns=\"\"><p>plain</p></div>",
                "<math xmlns=\"http://www.w3.org/1998/Math/MathML\"><p>other</p></math></body></html>"
            ),
            vec![
                ("title", "/html/head/title/text()", vec!["T"]),
                // No-namespace elements still match; other namespaces do not.
                ("paragraphs", "//p/text()", vec!["Hello", "plain"]),
            ],
        ),
    ];
    for (file, source, rules) in cases {
        let spec: Vec<(&str, &str, Option<&str>)> = rules
            .iter()
            .map(|(id, expression, _)| (*id, *expression, None))
            .collect();
        let fixture = Fixture::new("xml_default_namespace", &xml_template(&spec));
        fixture.source(file, source);
        assert_eq!(run::run(&fixture.cli).unwrap(), ExitCode::Success, "{file}");
        let expected: Vec<Vec<String>> = rules
            .iter()
            .map(|(_, _, values)| values.iter().map(|v| v.to_string()).collect())
            .collect();
        assert_eq!(items(&fixture.document()), expected, "{file}");
    }
}
