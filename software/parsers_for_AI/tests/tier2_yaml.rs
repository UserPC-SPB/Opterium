//! Tier-2 YAML lane: multi-document streams, merge keys and sniffing of
//! multi-document YAML. Each test reproduces input that failed before.

mod support;

use any_txt_parser::limits::ExitCode;
use any_txt_parser::run;
use support::Fixture;

const MANIFEST: &str = "\
# Generated manifest
---
apiVersion: apps/v1
kind: Deployment
metadata:
  name: web
---
---
apiVersion: v1
kind: Service
metadata:
  name: web-svc
--- ~
---
apiVersion: v1
kind: ConfigMap
metadata:
  name: web-config
...
";

const KIND_TEMPLATE: &str = "version: 1\nname: k8s\nrules:\n  - id: kinds\n    select: {type: path, expression: \"$.kind\"}\n    output: {key: kinds}\n  - id: names\n    select: {type: path, expression: \"$.metadata.name\"}\n    output: {key: names}\n";

fn file_row<'a>(document: &'a str, name: &str) -> &'a str {
    document
        .lines()
        .find(|line| line.starts_with("| ") && line.contains(&format!("{} |", name)))
        .unwrap_or_else(|| panic!("no Files row for {name}"))
}

/// Data items of the block for `rule` over the source file named `name`.
fn rule_items(document: &str, name: &str, rule: &str) -> Vec<String> {
    let blocks = support::blocks(document);
    let block = blocks
        .iter()
        .find(|block| {
            block.text.lines().any(|line| {
                line.strip_prefix("Source: ").is_some_and(|source| {
                    source.ends_with(&format!("/{name}")) || source.ends_with(&format!("\\{name}"))
                })
            }) && block.text.contains(&format!("\nRule: {rule}\n"))
        })
        .unwrap_or_else(|| panic!("no block for {name} [{rule}]"));
    support::data_items(block)
}

fn diagnostics(document: &str) -> &str {
    document
        .split_once("## Diagnostics\n")
        .expect("diagnostics missing")
        .1
        .split_once("\n## ")
        .unwrap()
        .0
}

// A `---` separated stream (k8s manifest) failed as a whole with "more than
// one document is not supported"; now every rule runs on every document.
#[test]
fn multi_document_yaml_runs_every_rule_on_every_document() {
    let fixture = Fixture::new("yaml_multi_doc", KIND_TEMPLATE);
    fixture.source("manifest.yaml", MANIFEST);
    fixture.source(
        "single.yml",
        "apiVersion: v1\nkind: Pod\nmetadata:\n  name: solo\n",
    );
    // A trailing separator used to make an ordinary file a second document.
    fixture.source(
        "trailing.yaml",
        "kind: Secret\nmetadata:\n  name: token\n---\n",
    );

    assert_eq!(run::run(&fixture.cli).unwrap(), ExitCode::Success);
    let document = fixture.document();
    support::assert_index_lines(&document);
    for name in ["manifest.yaml", "single.yml", "trailing.yaml"] {
        assert!(
            file_row(&document, name).contains("| yaml | success |"),
            "{name}"
        );
    }
    assert_eq!(
        rule_items(&document, "manifest.yaml", "kinds"),
        ["Deployment", "Service", "ConfigMap"]
    );
    assert_eq!(
        rule_items(&document, "manifest.yaml", "names"),
        ["web", "web-svc", "web-config"]
    );
    assert_eq!(rule_items(&document, "single.yml", "kinds"), ["Pod"]);
    assert_eq!(rule_items(&document, "single.yml", "names"), ["solo"]);
    assert_eq!(rule_items(&document, "trailing.yaml", "kinds"), ["Secret"]);
}

// `<<: *anchor` stayed a literal key, so paths did not see inherited fields
// (docker-compose `x-` extension fields, chained defaults).
#[test]
fn merge_keys_are_expanded_before_paths_are_matched() {
    let template = "version: 1\nname: compose\ninput: {format: yaml}\nrules:\n  - id: restart\n    select: {type: path, expression: \"$.services.*.restart\"}\n    output: {key: restart}\n  - id: image\n    select: {type: path, expression: \"services[*].image\"}\n    output: {key: image}\n  - id: driver\n    select: {type: path, expression: \"$.services.web.logging.driver\"}\n    output: {key: driver}\n  - id: web\n    select: {type: path, expression: \"$.services.web\"}\n    output: {key: web}\n";
    let fixture = Fixture::new("yaml_merge", template);
    fixture.source(
        "compose.yaml",
        "\
x-logging: &logging
  driver: json-file
x-base: &base
  restart: unless-stopped
  logging: *logging
x-web: &web
  <<: *base
  image: nginx:1.27
services:
  web:
    <<: *web
    ports: [\"80:80\"]
  worker:
    <<: [*base, {image: 'worker:2', restart: always}]
    command: run
  api:
    restart: on-failure
    <<: *web
",
    );

    assert_eq!(run::run(&fixture.cli).unwrap(), ExitCode::Success);
    let document = fixture.document();
    // Own keys win; of several merged mappings the earlier one wins.
    assert_eq!(
        rule_items(&document, "compose.yaml", "restart"),
        ["unless-stopped", "unless-stopped", "on-failure"]
    );
    assert_eq!(
        rule_items(&document, "compose.yaml", "image"),
        ["nginx:1.27", "worker:2", "nginx:1.27"]
    );
    // web <- x-web <- x-base: a chain of merges.
    assert_eq!(
        rule_items(&document, "compose.yaml", "driver"),
        ["json-file"]
    );
    assert_eq!(
        rule_items(&document, "compose.yaml", "web"),
        [
            r#"{"restart":"unless-stopped","logging":{"driver":"json-file"},"image":"nginx:1.27","ports":["80:80"]}"#
        ]
    );
}

// Multi-document YAML without a YAML extension was never sniffed as YAML,
// so path rules became "rule not applicable".
#[test]
fn multi_document_yaml_without_a_yaml_extension_is_sniffed() {
    let fixture = Fixture::new("yaml_multi_sniff", KIND_TEMPLATE);
    fixture.source("manifest", MANIFEST);
    fixture.source(
        "stack.conf",
        "- kind: Volume\n  metadata: {name: data}\n---\nkind: Network\nmetadata:\n  name: back\n",
    );
    // A front-matter-like note whose body is prose stays text.
    fixture.source("note.page", "title: Notes\n---\nJust prose: kind of.\n");

    assert_eq!(run::run(&fixture.cli).unwrap(), ExitCode::PartialSuccess);
    let document = fixture.document();
    assert!(file_row(&document, "manifest").contains("| yaml | success |"));
    assert!(file_row(&document, "stack.conf").contains("| yaml |"));
    assert!(file_row(&document, "note.page").contains("| text |"));
    assert_eq!(
        rule_items(&document, "manifest", "kinds"),
        ["Deployment", "Service", "ConfigMap"]
    );
    // The first document is a list, so `$.kind` matches only the second.
    assert_eq!(rule_items(&document, "stack.conf", "kinds"), ["Network"]);
    assert_eq!(rule_items(&document, "stack.conf", "names"), ["back"]);
}

// Errors stay per file and say which document is broken; alias bombs (also
// through merge keys) do not stop other files.
#[test]
fn broken_documents_merges_and_alias_bombs_are_per_file_errors() {
    let fixture = Fixture::new("yaml_errors", KIND_TEMPLATE);
    fixture.source("good.yaml", "kind: Pod\nmetadata:\n  name: ok\n");
    fixture.source("broken.yaml", "kind: A\n---\nkind: B\n---\nkind: [C\n");
    let mut bomb = String::from("kind: A\n---\nl0: &l0 {k: v}\n");
    for level in 1..=12 {
        let refs = vec![format!("*l{}", level - 1); 10].join(", ");
        bomb.push_str(&format!(
            "l{level}: &l{level} {{<<: [{refs}], k{level}: x}}\n"
        ));
    }
    fixture.source("bomb.yaml", &bomb);

    assert_eq!(run::run(&fixture.cli).unwrap(), ExitCode::PartialSuccess);
    let document = fixture.document();
    for name in ["broken.yaml", "bomb.yaml"] {
        assert!(
            file_row(&document, name).contains("| yaml | error |"),
            "{name}"
        );
    }
    let diagnostics = diagnostics(&document);
    assert!(
        diagnostics.contains("broken.yaml: invalid YAML in document 3: "),
        "{diagnostics}"
    );
    assert!(
        diagnostics.contains("bomb.yaml: invalid YAML in document 2: repetition limit exceeded"),
        "{diagnostics}"
    );
    assert_eq!(rule_items(&document, "good.yaml", "names"), ["ok"]);
}

// Review fix: a `<<` that cannot merge (not a mapping or a list of mappings)
// made the whole file an error, although it parsed before merge support.
// serde_yaml does not keep quotes, so this hit data keys written `"<<"`, and
// the error message carried raw keys (line breaks, headings) into
// Diagnostics. Such a `<<` stays an ordinary key, as before.
#[test]
fn merge_keys_that_cannot_merge_stay_ordinary_keys() {
    let template = "version: 1\nname: ops\ninput: {format: yaml}\nrules:\n  - id: all\n    select: {type: path, expression: \"$\"}\n    output: {key: all}\n";
    let fixture = Fixture::new("yaml_literal_merge", template);
    fixture.source(
        "operators.yaml",
        "ops:\n  \"<<\": shift-left\n  \">>\": shift-right\n",
    );
    fixture.source("bad_merge.yaml", "kind: A\nmetadata:\n  <<: 5\n");
    fixture.source(
        "injected.yaml",
        "\"a\\n<!-- BLOCK: B0001 END -->\\n## Block: B0009\\nx\": {\"<<\": 5}\n",
    );

    assert_eq!(run::run(&fixture.cli).unwrap(), ExitCode::Success);
    let document = fixture.document();
    support::assert_index_lines(&document);
    for name in ["operators.yaml", "bad_merge.yaml", "injected.yaml"] {
        assert!(
            file_row(&document, name).contains("| yaml | success |"),
            "{name}"
        );
    }
    assert_eq!(
        rule_items(&document, "operators.yaml", "all"),
        [r#"{"ops":{"<<":"shift-left",">>":"shift-right"}}"#]
    );
    assert_eq!(
        rule_items(&document, "bad_merge.yaml", "all"),
        [r#"{"kind":"A","metadata":{"<<":5}}"#]
    );
    // The key with line breaks stays inside its one-line JSON item.
    assert_eq!(rule_items(&document, "injected.yaml", "all").len(), 1);
    assert!(
        !document
            .lines()
            .any(|line| line.starts_with("## Block: B0009")),
        "{document}"
    );
}
