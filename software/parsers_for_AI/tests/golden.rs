use any_txt_parser::limits::{CliOptions, ExitCode};
use any_txt_parser::run;
use std::path::PathBuf;

mod support;

fn tmpdir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("atp_it_{}_{}", name, std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn write(path: &PathBuf, content: impl AsRef<str>) {
    std::fs::write(path, content.as_ref()).unwrap();
}

fn cli(dir: &PathBuf) -> CliOptions {
    CliOptions {
        input: dir.join("in.json").to_string_lossy().to_string(),
        template: dir.join("tpl.yml"),
        output: Some(dir.join("log_any_txt.md")),
        format: None,
        encoding: None,
        dry_run: false,
        verbose: false,
        quiet: true,
        follow_symlinks: false,
        hash: false,
        max_file_size: None,
        max_memory: None,
        max_files: None,
        max_matches: None,
        max_output_size: None,
        max_block_chars: None,
        raw_args: vec!["--input".into(), "in.json".into()],
    }
}

#[test]
fn golden_self_describing_output() {
    let dir = tmpdir("golden");
    write(
        &dir.join("tpl.yml"),
        r#"
version: 1
name: chat
input:
  format: json
rules:
  - id: user_messages
    select:
      type: jsonpath
      expression: "$.messages[?(@.role == 'user')].content"
    output:
      key: user_messages
    transforms:
      - trim
      - drop_empty
    limits:
      max_items: 100
      max_item_chars: 4000
"#,
    );
    write(
        &dir.join("in.json"),
        r#"{"messages": [
  {"role": "user", "content": "  hi  "},
  {"role": "assistant", "content": "hello"},
  {"role": "user", "content": "bye"},
  {"role": "user", "content": "  hi  "}
]}"#,
    );

    let result = run::run(&cli(&dir));
    let code = result.unwrap();
    assert_eq!(code, ExitCode::Success);

    let doc = std::fs::read_to_string(dir.join("log_any_txt.md")).unwrap();

    // fixed header contract (spec 13, 15, 16, 40)
    assert!(doc.starts_with("# any_txt_parser output\n"));
    assert!(doc.contains("Triangle: [0,1,0]"));
    assert!(doc.contains("Generation: 00000001"));
    assert!(doc.contains("## Execution"));
    assert!(doc.contains("## Applied Template"));
    assert!(doc.contains("## Effective Template"));
    assert!(doc.contains("```yaml"));
    assert!(doc.contains("## Result Semantics"));
    assert!(doc.contains("## CLI Arguments"));
    assert!(doc.contains("## Block Index"));
    assert!(doc.contains("### Block Anchors"));
    assert!(doc.contains("## Data Blocks"));
    assert!(doc.ends_with("End of any_txt_parser output\n"));

    // Applied Template carries the rules and limits (spec 10)
    assert!(doc.contains("### Rule: user_messages"));
    assert!(doc.contains("Selector type: jsonpath"));
    assert!(doc.contains("- max_items: 100"));
    assert!(doc.contains("- max_item_chars: 4000"));

    // Block anchor contract (spec 23-25, 64)
    assert!(doc.contains("<!-- BLOCK: B0001 START -->"));
    assert!(doc.contains("<!-- BLOCK: B0001 END -->"));
    assert!(doc.contains("## Block: B0001"));
    assert!(doc.contains("Rule: user_messages"));
    assert!(doc.contains("Output key: user_messages"));
    assert!(doc.contains("Items: 3"));
    assert!(doc.contains("Truncated: no"));
    assert!(doc.contains("Split: no"));
    assert!(doc.contains("Status: success"));

    // data: trim + dedup not on here (stable_deduplicate=false), drop_empty keeps 3 items: hi, bye, hi (with trims)
    let blocks = support::blocks(&doc);
    assert_eq!(blocks.len(), 1);
    assert_eq!(support::data_items(&blocks[0]), vec!["hi", "bye", "hi"]);

    // index carries line ranges (spec 28, 60-61)
    support::assert_index_lines(&doc);
    support::assert_total_characters(&doc);

    // block index comes before the data blocks
    let idx_pos = doc.find("## Block Index").unwrap();
    let data_pos = doc.find("## Data Blocks").unwrap();
    assert!(idx_pos < data_pos);
}

#[test]
fn generation_increments_and_repeatable() {
    let dir = tmpdir("gen");
    write(
        &dir.join("tpl.yml"),
        "version: 1\nname: t\ninput: {format: text}\nrules:\n  - id: r\n    select: {type: regex, expression: \"x\"}\n    output: {key: k}\n",
    );
    write(&dir.join("in.txt"), "x\nx\ny\n");

    let mut c = cli(&dir);
    c.input = dir.join("in.txt").to_string_lossy().to_string();

    run::run(&c).unwrap();
    let first = std::fs::read_to_string(dir.join("log_any_txt.md")).unwrap();
    assert!(first.contains("Generation: 00000001"));

    run::run(&c).unwrap();
    let second = std::fs::read_to_string(dir.join("log_any_txt.md")).unwrap();
    assert!(second.contains("Generation: 00000002"));

    // deterministic: same data aside from volatile fields
    support::assert_total_characters(&first);
    support::assert_total_characters(&second);
    support::assert_index_lines(&first);
    support::assert_index_lines(&second);
    let s1 = support::normalize_volatile(&first);
    let s2 = support::normalize_volatile(&second);
    assert_eq!(
        s1, s2,
        "output must be deterministic apart from volatile fields"
    );
}

#[test]
fn no_matches_vs_not_applicable_vs_error() {
    let dir = tmpdir("status");
    write(
        &dir.join("tpl.yml"),
        r#"
version: 1
name: t
input: {format: text}
rules:
  - id: nm
    select: {type: contains, expression: "zzz"}
    output: {key: k1}
  - id: na
    select: {type: css, expression: "p"}
    output: {key: k2}
"#,
    );
    write(&dir.join("in.txt"), "hello world\n");

    let mut c = cli(&dir);
    c.input = dir.join("in.txt").to_string_lossy().to_string();

    run::run(&c).unwrap();
    let doc = std::fs::read_to_string(dir.join("log_any_txt.md")).unwrap();

    // no matches found (spec 37) vs rule not applicable (spec 69)
    assert!(doc.contains("no matches found"));
    assert!(doc.contains("rule not applicable"));
    assert!(doc.contains("Block: B0001"));
    assert!(doc.contains("Block: B0002"));
    assert!(doc.contains("File status: warning"));
    assert!(doc.contains("Status: no matches found\n") || doc.contains("no matches found\n"));
}

#[test]
fn split_oversized_item() {
    let dir = tmpdir("split");
    write(
        &dir.join("tpl.yml"),
        r#"
version: 1
name: t
input: {format: text}
output:
  block:
    target_chars: 800
    max_chars: 1000
rules:
  - id: big
    select: {type: line, from: 1, to: 200}
    output: {key: k}
"#,
    );
    let big = (0..200)
        .map(|i| format!("line {}", i))
        .collect::<Vec<_>>()
        .join("\n");
    write(&dir.join("in.txt"), &big);

    let mut c = cli(&dir);
    c.input = dir.join("in.txt").to_string_lossy().to_string();

    run::run(&c).unwrap();
    let doc = std::fs::read_to_string(dir.join("log_any_txt.md")).unwrap();

    // split: yes + parts (spec 33-34)
    assert!(doc.contains("Split: yes"));
    assert!(doc.contains("Parent: B0001"));
    assert!(doc.contains("Part: 1/"));
    let blocks = support::blocks(&doc);
    assert_eq!(blocks[0].id, "B0001");
    assert!(support::data_items(&blocks[0]).is_empty());
    let recovered: String = blocks[1..].iter().flat_map(support::data_items).collect();
    assert_eq!(recovered, big, "splitting must preserve every newline");
    for block in &blocks {
        assert!(
            block.text.chars().count() <= 1000,
            "{} exceeds the full block cap",
            block.id
        );
    }
    support::assert_index_lines(&doc);
    support::assert_total_characters(&doc);
}

#[test]
fn invalid_template_does_not_start() {
    let dir = tmpdir("bad_tpl");
    write(
        &dir.join("tpl.yml"),
        "version: 99\nname: t\ninput: {format: text}\nrules:\n  - id: r\n    select: {type: regex, expression: \"x\"}\n    output: {key: k}\n",
    );
    write(&dir.join("in.txt"), "x\n");
    let mut c = cli(&dir);
    c.input = dir.join("in.txt").to_string_lossy().to_string();

    let r = run::run(&c);
    assert!(r.is_err());
    assert_eq!(r.unwrap_err().code, ExitCode::InvalidTemplate);
    assert!(!dir.join("log_any_txt.md").exists());
}

#[test]
fn resource_limit_max_matches() {
    let dir = tmpdir("matches");
    write(
        &dir.join("tpl.yml"),
        "version: 1\nname: t\ninput: {format: text}\nrules:\n  - id: r\n    select: {type: regex, expression: \"x\"}\n    output: {key: k}\n",
    );
    write(
        &dir.join("in.txt"),
        (0..50).map(|_| "x").collect::<Vec<_>>().join("\n"),
    );
    let mut c = cli(&dir);
    c.input = dir.join("in.txt").to_string_lossy().to_string();
    c.max_matches = Some(5);

    run::run(&c).unwrap();
    let doc = std::fs::read_to_string(dir.join("log_any_txt.md")).unwrap();
    assert!(doc.contains("45 match(es) omitted after max_matches_per_rule"));
    assert!(doc.contains("Items: 5"));
}

#[test]
fn hash_flag_embeds_sha256() {
    let dir = tmpdir("hash");
    write(
        &dir.join("tpl.yml"),
        "version: 1\nname: t\ninput: {format: text}\nrules:\n  - id: r\n    select: {type: contains, expression: \"x\"}\n    output: {key: k}\n",
    );
    write(&dir.join("in.txt"), "x\n");
    let mut c = cli(&dir);
    c.input = dir.join("in.txt").to_string_lossy().to_string();
    c.hash = true;

    run::run(&c).unwrap();
    let doc = std::fs::read_to_string(dir.join("log_any_txt.md")).unwrap();
    // sha256("x\n") is known
    assert!(doc.contains("## Files"));
    assert!(doc.contains("in.txt | 2 |"));
    assert!(doc.contains("73cb3858a687a8494ca3323053016282f3dad39d42cf62ca4e79dda2aac7d9ac"));
}

#[test]
fn stable_deduplicate_preserves_first() {
    let dir = tmpdir("dedup");
    write(
        &dir.join("tpl.yml"),
        "version: 1\nname: t\ninput: {format: text}\ndocument:\n  stable_deduplicate: true\nrules:\n  - id: r\n    select: {type: regex, expression: \"L[0-9]\"}\n    output: {key: k}\n",
    );
    write(&dir.join("in.txt"), "L1\nL2\nL1\nL3\nL2\nL4\n");
    let mut c = cli(&dir);
    c.input = dir.join("in.txt").to_string_lossy().to_string();

    run::run(&c).unwrap();
    let doc = std::fs::read_to_string(dir.join("log_any_txt.md")).unwrap();
    let l1_pos = doc.find("- L1").unwrap();
    let l3_pos = doc.find("- L3").unwrap();
    let l4_pos = doc.find("- L4").unwrap();
    assert!(l1_pos < l3_pos && l3_pos < l4_pos);
    assert!(doc.matches("- L2").count() == 1);
    assert!(doc.matches("- L1").count() == 1);
    assert!(doc.contains("Items: 4"));
}

#[test]
fn dry_run_publishes_nothing() {
    let dir = tmpdir("dry");
    write(
        &dir.join("tpl.yml"),
        "version: 1\nname: t\ninput: {format: text}\nrules:\n  - id: r\n    select: {type: contains, expression: \"x\"}\n    output: {key: k}\n",
    );
    write(&dir.join("in.txt"), "x\n");
    let mut c = cli(&dir);
    c.input = dir.join("in.txt").to_string_lossy().to_string();
    c.dry_run = true;

    let result = run::run(&c).unwrap();
    assert_eq!(result, ExitCode::Success);
    assert!(!dir.join("log_any_txt.md").exists());
}

// ---------------------------------------------------------------------------
// CLI override tests (--format / --encoding must override template & detection)
// ---------------------------------------------------------------------------

fn block_of(doc: &str, id: &str) -> String {
    let start = format!("<!-- BLOCK: {} START -->", id);
    let end = format!("<!-- BLOCK: {} END -->", id);
    let s = doc.find(&start).expect("block start missing");
    let e = doc.find(&end).expect("block end missing");
    doc[s..e].to_string()
}

#[test]
fn cli_format_override_applies_to_any_extension() {
    // .md file would auto-detect as markdown (contains selector -> not applicable);
    // forced --format text must make the rule work regardless of extension.
    let dir = tmpdir("cli_fmt");
    write(
        &dir.join("tpl.yml"),
        "version: 1\nname: t\ninput: {format: auto}\nrules:\n  - id: r\n    select: {type: contains, expression: \"ERROR\"}\n    output: {key: errors}\n",
    );
    write(
        &dir.join("in.md"),
        "Заголовок\nЭто простой текст.\nЕщё строка с ERROR: что-то сломалось.\n",
    );

    let mut c = cli(&dir);
    c.input = dir.join("in.md").to_string_lossy().to_string();
    c.format = Some("text".into());

    let code = run::run(&c).unwrap();
    assert_eq!(code, ExitCode::Success);

    let doc = std::fs::read_to_string(dir.join("log_any_txt.md")).unwrap();
    assert!(doc.contains("Effective format: text"));
    assert!(doc.contains("Format source: CLI override"));
    let block = block_of(&doc, "B0001");
    assert!(block.contains("Format: text"));
    assert!(block.contains("Status: success"));
    assert!(block.contains("Items: 1"));
    assert!(doc.contains("Ещё строка с ERROR: что-то сломалось."));
}

#[test]
fn cli_format_overrides_template_format() {
    // Template says json, CLI forces text: the .json file must be processed by
    // the text adapter (line extracted verbatim), not parsed as JSON.
    let dir = tmpdir("cli_fmt_tpl");
    write(
        &dir.join("tpl.yml"),
        "version: 1\nname: t\ninput: {format: json}\nrules:\n  - id: r\n    select: {type: contains, expression: \"key\"}\n    output: {key: k}\n",
    );
    write(
        &dir.join("in.json"),
        "{\"key\": \"value\", \"number\": 42}\n",
    );

    let mut c = cli(&dir);
    c.input = dir.join("in.json").to_string_lossy().to_string();
    c.format = Some("text".into());

    let code = run::run(&c).unwrap();
    assert_eq!(code, ExitCode::Success);

    let doc = std::fs::read_to_string(dir.join("log_any_txt.md")).unwrap();
    assert!(doc.contains("Format source: CLI override"));
    let block = block_of(&doc, "B0001");
    assert!(block.contains("Status: success"));
    assert!(block.contains("- {\"key\": \"value\", \"number\": 42}"));
}

#[test]
fn cli_format_invalid_rejected_exit3() {
    let dir = tmpdir("cli_fmt_bad");
    write(
        &dir.join("tpl.yml"),
        "version: 1\nname: t\ninput: {format: auto}\nrules:\n  - id: r\n    select: {type: contains, expression: \"x\"}\n    output: {key: k}\n",
    );
    write(&dir.join("in.txt"), "x\n");

    let mut c = cli(&dir);
    c.input = dir.join("in.txt").to_string_lossy().to_string();
    c.format = Some("bogus".into());

    let r = run::run(&c);
    assert!(r.is_err());
    let e = r.unwrap_err();
    assert_eq!(e.code, ExitCode::InvalidInputOrCli);
    assert!(e.message.contains("--format"));
    assert!(!dir.join("log_any_txt.md").exists());
}

#[test]
fn cli_format_auto_rejected_exit3() {
    let dir = tmpdir("cli_fmt_auto");
    write(
        &dir.join("tpl.yml"),
        "version: 1\nname: t\ninput: {format: auto}\nrules:\n  - id: r\n    select: {type: contains, expression: \"x\"}\n    output: {key: k}\n",
    );
    write(&dir.join("in.txt"), "x\n");

    let mut c = cli(&dir);
    c.input = dir.join("in.txt").to_string_lossy().to_string();
    c.format = Some("auto".into());

    let r = run::run(&c);
    assert!(r.is_err());
    let e = r.unwrap_err();
    assert_eq!(e.code, ExitCode::InvalidInputOrCli);
    assert!(e.message.contains("auto"));
}

#[test]
fn cli_encoding_override_applies() {
    // File is windows-1251; template has no encoding (default utf-8 would fail
    // to decode). --encoding must override and the Cyrillic line must be found.
    let dir = tmpdir("cli_enc");
    write(
        &dir.join("tpl.yml"),
        "version: 1\nname: t\ninput: {format: text}\nrules:\n  - id: r\n    select: {type: contains, expression: \"ОШИБКА\"}\n    output: {key: k}\n",
    );
    let (cow, _, _) = encoding_rs::WINDOWS_1251.encode("в логе обнаружена ОШИБКА\n");
    std::fs::write(dir.join("in.txt"), &*cow).unwrap();

    let mut c = cli(&dir);
    c.input = dir.join("in.txt").to_string_lossy().to_string();
    c.encoding = Some("windows-1251".into());

    let code = run::run(&c).unwrap();
    assert_eq!(code, ExitCode::Success);

    let doc = std::fs::read_to_string(dir.join("log_any_txt.md")).unwrap();
    assert!(doc.contains("Encoding source: CLI override"));
    assert!(doc.contains("в логе обнаружена ОШИБКА"));
}

#[test]
fn cli_encoding_invalid_rejected_exit3() {
    let dir = tmpdir("cli_enc_bad");
    write(
        &dir.join("tpl.yml"),
        "version: 1\nname: t\ninput: {format: auto}\nrules:\n  - id: r\n    select: {type: contains, expression: \"x\"}\n    output: {key: k}\n",
    );
    write(&dir.join("in.txt"), "x\n");

    let mut c = cli(&dir);
    c.input = dir.join("in.txt").to_string_lossy().to_string();
    c.encoding = Some("utf-9".into());

    let r = run::run(&c);
    assert!(r.is_err());
    let e = r.unwrap_err();
    assert_eq!(e.code, ExitCode::InvalidInputOrCli);
    assert!(e.message.contains("encoding"));
}

#[test]
fn invalid_size_string_rejected_exit3() {
    // Previously a bad SIZE value was silently ignored (limit disabled).
    let dir = tmpdir("bad_size");
    write(
        &dir.join("tpl.yml"),
        "version: 1\nname: t\ninput: {format: auto}\nrules:\n  - id: r\n    select: {type: contains, expression: \"x\"}\n    output: {key: k}\n",
    );
    write(&dir.join("in.txt"), "x\n");

    let mut c = cli(&dir);
    c.input = dir.join("in.txt").to_string_lossy().to_string();
    c.max_file_size = Some("abc".into());

    let r = run::run(&c);
    assert!(r.is_err());
    let e = r.unwrap_err();
    assert_eq!(e.code, ExitCode::InvalidInputOrCli);
    assert!(e.message.contains("--max-file-size"));
}

#[test]
fn dry_run_rejects_bad_format_before_anything() {
    let dir = tmpdir("dry_bad");
    write(
        &dir.join("tpl.yml"),
        "version: 1\nname: t\ninput: {format: auto}\nrules:\n  - id: r\n    select: {type: contains, expression: \"x\"}\n    output: {key: k}\n",
    );
    write(&dir.join("in.txt"), "x\n");

    let mut c = cli(&dir);
    c.input = dir.join("in.txt").to_string_lossy().to_string();
    c.dry_run = true;
    c.format = Some("bogus".into());

    let r = run::run(&c);
    assert!(r.is_err());
    assert_eq!(r.unwrap_err().code, ExitCode::InvalidInputOrCli);
    assert!(!dir.join("log_any_txt.md").exists());
}

/// The report-mode example of the README: `examples/errors.yml` on
/// `examples/example.log`.
#[test]
fn readme_example_finds_the_error_with_its_context() {
    let dir = tmpdir("readme_example");
    let examples = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples");
    let mut c = cli(&dir);
    c.input = examples.join("example.log").to_string_lossy().to_string();
    c.template = examples.join("errors.yml");

    assert_eq!(run::run(&c).unwrap(), ExitCode::Success);
    let doc = std::fs::read_to_string(dir.join("log_any_txt.md")).unwrap();
    let blocks = support::blocks(&doc);
    assert_eq!(blocks.len(), 1);
    assert_eq!(
        support::data_items(&blocks[0]),
        [
            "INFO Application started\nERROR Could not open settings.json\nINFO Falling back to defaults"
        ]
    );
}
