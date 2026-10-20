//! The single executable: commands, built-in help, the report mode and the
//! MCP server, driven through the real binary.

use serde_json::{Value, json};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

static ID: AtomicU64 = AtomicU64::new(0);

struct Project {
    dir: PathBuf,
}

impl Project {
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "atp_one_exe_{}_{}_{}",
            std::process::id(),
            name,
            ID.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let project = Project { dir };
        project.file(
            "logs/app.log",
            "2026-10-01 ERROR disk full\n2026-10-01 INFO ok\n2026-10-02 ERROR timeout code=42\n",
        );
        project.file(
            "data/users.json",
            r#"{"users":[{"name":"Ann","active":true},{"name":"Bob","active":false}]}"#,
        );
        project.file(
            "src/cart.rs",
            "pub struct Cart {\n    items: Vec<u32>,\n}\n\nimpl Cart {\n    pub fn new() -> Self {\n        Cart { items: Vec::new() }\n    }\n\n    // TODO: discounts\n    pub fn checkout(&self) -> u32 {\n        self.items.iter().sum()\n    }\n}\n",
        );
        project
    }

    fn file(&self, relative: &str, content: &str) {
        let path = self.dir.join(relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, content).unwrap();
    }

    /// Runs the exe in the project folder.
    fn run(&self, args: &[&str]) -> (i32, String, String) {
        let output = Command::new(env!("CARGO_BIN_EXE_any_txt_parser"))
            .args(args)
            .current_dir(&self.dir)
            .output()
            .unwrap();
        (
            output.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&output.stdout).into_owned(),
            String::from_utf8_lossy(&output.stderr).into_owned(),
        )
    }

    fn files_under(&self, dir: &str) -> Vec<String> {
        walk(&self.dir.join(dir))
    }
}

impl Drop for Project {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

fn walk(dir: &Path) -> Vec<String> {
    let mut out: Vec<String> = std::fs::read_dir(dir)
        .map(|entries| {
            entries
                .flatten()
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default();
    out.sort();
    out
}

#[test]
fn no_arguments_lists_the_commands_and_help_topics_print() {
    let p = Project::new("usage");
    let (code, stdout, stderr) = p.run(&[]);
    assert_eq!(code, 3);
    assert!(stdout.is_empty());
    for command in [
        "extract", "recon", "map", "symbols", "find", "lines", "search", "todos", "changes", "nav",
        "mcp", "help",
    ] {
        assert!(
            stderr.contains(&format!("  {} ", command)),
            "{command} missing from usage"
        );
    }
    for topic in [
        "",
        "overview",
        "extract",
        "rules",
        "recon",
        "navigation",
        "mcp",
        "reference",
        "symbols",
        "topics",
    ] {
        let args: Vec<&str> = if topic.is_empty() {
            vec!["help"]
        } else {
            vec!["help", topic]
        };
        let (code, stdout, _) = p.run(&args);
        assert_eq!(code, 0, "help {topic}");
        assert!(stdout.len() > 300, "help {topic} is too short");
    }
    let (code, _, stderr) = p.run(&["help", "nope"]);
    assert_eq!(code, 3);
    assert!(stderr.contains("unknown help topic"));
    let (code, stdout, _) = p.run(&["--version"]);
    assert_eq!(code, 0);
    assert_eq!(
        stdout.trim(),
        format!("any_txt_parser {}", any_txt_parser::PARSER_VERSION)
    );
}

#[test]
fn every_command_named_in_the_help_exists() {
    let known = any_txt_parser::commands::COMMANDS;
    for topic in any_txt_parser::help::TOPICS {
        for line in topic.text.lines() {
            let Some(rest) = line.trim_start().strip_prefix("any_txt_parser ") else {
                continue;
            };
            let word = rest.split_whitespace().next().unwrap_or("");
            // Options, placeholders and sample output lines ("any_txt_parser 1.5.0: ...").
            if word.is_empty()
                || word.starts_with(['-', '<'])
                || word.starts_with(|c: char| c.is_ascii_digit())
            {
                continue;
            }
            assert!(
                known.iter().any(|(name, _)| *name == word),
                "help topic '{}' uses unknown command '{}': {}",
                topic.name,
                word,
                line
            );
        }
    }
}

#[test]
fn extract_and_recon_from_the_command_line() {
    let p = Project::new("extract");
    let (code, stdout, _) = p.run(&["extract", "-i", ".", "-r", "contains:ERROR"]);
    assert_eq!(code, 0, "{stdout}");
    assert!(stdout.contains("== logs/app.log [text] rule r1 (success, 2 items)\n- L1: 2026-10-01 ERROR disk full\n- L3: 2026-10-02 ERROR timeout code=42\n"));
    assert!(stdout.contains("skipped files it cannot read: 1 json"));

    let (code, stdout, _) = p.run(&[
        "extract",
        "-i",
        "data",
        "-r",
        "jsonpath:$.users[?(@.active == true)].name",
        "--json",
    ]);
    assert_eq!(code, 0);
    let response: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(response["results"][0]["items"], json!(["Ann"]));

    let (code, stdout, stderr) = p.run(&["extract", "-i", "logs", "-r", "nope:x", "--json"]);
    assert_eq!(code, 2);
    assert!(stderr.contains("nope"));
    assert_eq!(
        serde_json::from_str::<Value>(&stdout).unwrap()["exit_code"],
        2
    );

    let (code, stdout, _) = p.run(&["recon", "data", "--sample", "1"]);
    assert_eq!(code, 0);
    assert!(
        stdout.contains("format: json (extension)")
            && stdout.contains("shape: object with 1 key(s): users[2]")
    );
}

#[test]
fn navigation_commands_read_the_project_and_write_nothing() {
    let p = Project::new("nav");
    let before = p.files_under(".");
    let (code, stdout, _) = p.run(&["map"]);
    assert_eq!(code, 0);
    assert!(stdout.starts_with("map .: 3 files"), "{stdout}");
    assert!(stdout.contains("└── src/\n    └── cart.rs\n") || stdout.contains("src/"));
    let (code, stdout, _) = p.run(&["symbols", "src/cart.rs"]);
    assert_eq!(code, 0);
    assert!(stdout.contains("  1-3  struct Cart\n"), "{stdout}");
    assert!(
        stdout.contains("    11-13  method checkout\n") || stdout.contains("11-13"),
        "{stdout}"
    );
    let (code, stdout, _) = p.run(&["find", "CHECKOUT"]);
    assert_eq!(code, 0);
    assert!(stdout.contains("src/cart.rs:11-13"), "{stdout}");
    let (code, stdout, _) = p.run(&["lines", "src/cart.rs", "11", "12"]);
    assert_eq!(code, 0);
    assert_eq!(
        stdout,
        "    11|     pub fn checkout(&self) -> u32 {\n    12|         self.items.iter().sum()\n"
    );
    let (code, stdout, _) = p.run(&["todos", "--json"]);
    assert_eq!(code, 0);
    let todos: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(todos[0]["path"], "src/cart.rs");
    assert_eq!(todos[0]["line"], 10);
    let (code, stdout, _) = p.run(&["changes"]);
    assert_eq!(code, 0);
    assert!(stdout.starts_with("changes: no snapshot yet, 3 file(s)"));
    assert_eq!(
        p.files_under("."),
        before,
        "read-only commands must not write files"
    );

    let (code, _, _) = p.run(&["changes", "--persist"]);
    assert_eq!(code, 0);
    p.file("src/cart.rs", "pub struct Cart;\n");
    let (_, stdout, _) = p.run(&["changes"]);
    assert!(
        stdout.starts_with("changes: 0 new, 1 changed, 0 deleted"),
        "{stdout}"
    );

    let (code, _, stderr) = p.run(&["lines", "src/cart.rs", "99"]);
    assert_eq!(code, 3);
    assert!(
        stderr.contains("out of range") || stderr.contains("99"),
        "{stderr}"
    );
    let (code, _, stderr) = p.run(&["nav"]);
    assert_eq!(code, 3);
    assert!(stderr.contains("give a mode"));
}

#[test]
fn the_report_mode_is_unchanged() {
    let p = Project::new("report");
    p.file(
        "t.yml",
        "version: 1\nname: t\nrules:\n  - id: r\n    select: {type: contains, expression: ERROR}\n",
    );
    let (code, _, stderr) = p.run(&["-i", "logs", "-t", "t.yml", "-o", "out.md", "-q"]);
    assert_eq!(code, 0, "{stderr}");
    let document = std::fs::read_to_string(p.dir.join("out.md")).unwrap();
    assert!(document.starts_with("# any_txt_parser output\n"));
    assert!(document.contains("- 2026-10-02 ERROR timeout code=42\n"));
}

/// Runs one MCP session with the given requests; returns replies by id.
fn mcp_session(p: &Project, requests: &[Value]) -> Vec<Value> {
    let mut child = Command::new(env!("CARGO_BIN_EXE_any_txt_parser"))
        .arg("mcp")
        .current_dir(&p.dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    {
        let stdin = child.stdin.as_mut().unwrap();
        for request in requests {
            writeln!(stdin, "{}", request).unwrap();
        }
    }
    let output = child.wait_with_output().unwrap();
    assert_eq!(output.status.code(), Some(0));
    String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

fn call(id: u64, name: &str, arguments: Value) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "method": "tools/call", "params": {"name": name, "arguments": arguments}})
}

fn text(reply: &Value) -> &str {
    reply["result"]["content"][0]["text"].as_str().unwrap()
}

#[test]
fn mcp_session_lists_and_runs_every_tool() {
    let p = Project::new("mcp");
    let replies = mcp_session(
        &p,
        &[
            json!({"jsonrpc":"2.0","id":0,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"test","version":"1"}}}),
            json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
            json!({"jsonrpc":"2.0","id":1,"method":"tools/list"}),
            call(2, "help", json!({"topic": "rules"})),
            call(3, "recon", json!({"path": "data", "sample_lines": 1})),
            call(
                4,
                "extract",
                json!({"input": ".", "rules": ["contains:ERROR", {"select": {"type": "jsonpath", "expression": "$.users[*].name"}, "when": {"format": "json"}}], "output": "json"}),
            ),
            call(5, "map", json!({})),
            call(6, "symbols", json!({"path": "src/cart.rs"})),
            call(7, "find_symbol", json!({"pattern": "checkout"})),
            call(
                8,
                "read_lines",
                json!({"path": "src/cart.rs", "start": 11, "end": 11}),
            ),
            call(9, "search_text", json!({"pattern": "ERROR disk"})),
            call(10, "todos", json!({})),
            call(11, "changes", json!({})),
            call(
                12,
                "extract",
                json!({"input": "logs", "rules": ["regex:("]}),
            ),
            call(13, "read_lines", json!({"path": "src/cart.rs"})),
            call(14, "extract", json!({"input": "logs", "rulez": []})),
        ],
    );
    assert_eq!(replies.len(), 15);
    assert_eq!(replies[0]["result"]["serverInfo"]["name"], "any_txt_parser");
    assert!(
        replies[0]["result"]["instructions"]
            .as_str()
            .unwrap()
            .contains("recon")
    );
    let names: Vec<&str> = replies[1]["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    assert_eq!(
        names,
        [
            "help",
            "recon",
            "extract",
            "map",
            "symbols",
            "find_symbol",
            "read_lines",
            "search_text",
            "todos",
            "changes"
        ]
    );
    for tool in replies[1]["result"]["tools"].as_array().unwrap() {
        assert_eq!(tool["inputSchema"]["type"], "object", "{}", tool["name"]);
        assert!(tool["description"].as_str().unwrap().len() > 30);
    }
    for reply in &replies[2..12] {
        assert_eq!(reply["result"]["isError"], false, "{}", reply);
    }
    assert!(text(&replies[2]).starts_with("# rules"));
    assert!(text(&replies[3]).contains("shape: object with 1 key(s): users[2]"));
    let extracted: Value = serde_json::from_str(text(&replies[4])).unwrap();
    assert_eq!(extracted["exit_code"], 0, "{}", extracted);
    let items: Vec<&Value> = extracted["results"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|r| r["items"].as_array().unwrap())
        .collect();
    // Files in path order: data/users.json, then logs/app.log.
    assert_eq!(
        items,
        [
            &json!("Ann"),
            &json!("Bob"),
            &json!("2026-10-01 ERROR disk full"),
            &json!("2026-10-02 ERROR timeout code=42")
        ]
    );
    assert!(text(&replies[5]).starts_with("map .: 3 files"));
    assert!(text(&replies[6]).contains("struct Cart"));
    assert!(text(&replies[7]).contains("src/cart.rs:11-13"));
    assert_eq!(
        text(&replies[8]),
        "    11|     pub fn checkout(&self) -> u32 {\n"
    );
    assert!(text(&replies[9]).contains("logs/app.log:1:"));
    assert!(text(&replies[10]).contains("src/cart.rs:10:"));
    assert!(text(&replies[11]).starts_with("changes: no snapshot yet"));
    for (reply, needle) in [
        (&replies[12], "exit code 2"),
        (&replies[13], "start"),
        (&replies[14], "rulez"),
    ] {
        assert_eq!(reply["result"]["isError"], true, "{}", reply);
        assert!(text(reply).contains(needle), "{}", reply);
    }
    assert!(!p.dir.join(".navguard_cache.json").exists());
}

/// Line numbers in extract results, `find --exact` and whole-word `todos`,
/// from the command line and over MCP.
#[test]
fn extract_lines_exact_find_and_strict_todos() {
    let p = Project::new("coding");
    p.file(
        "docs/notes.md",
        "# Notes\n\nList todos here; see todo_hits and TODO_TEXT_CHARS.\n",
    );

    // extract: `L<n>:` in text, `lines` / `spans` in JSON.
    let (code, stdout, _) = p.run(&[
        "extract",
        "-i",
        "logs",
        "-r",
        "contains:ERROR",
        "--context",
        "1",
    ]);
    assert_eq!(code, 0, "{stdout}");
    assert!(
        stdout.contains("- L1: 2026-10-01 ERROR disk full\n    L2- 2026-10-01 INFO ok\n- L2- 2026-10-01 INFO ok\n    L3: 2026-10-02 ERROR timeout code=42\n"),
        "{stdout}"
    );
    let (code, stdout, _) = p.run(&[
        "extract",
        "-i",
        "logs",
        "-r",
        "contains:ERROR",
        "--context",
        "1",
        "--json",
    ]);
    assert_eq!(code, 0, "{stdout}");
    let response: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(response["results"][0]["lines"], json!([1, 3]));
    assert_eq!(response["results"][0]["spans"], json!([[1, 2], [2, 3]]));
    assert_eq!(
        response["results"][0]["items"][0],
        "2026-10-01 ERROR disk full\n2026-10-01 INFO ok"
    );

    // find: every symbol of src/cart.rs contains "cart" in its path;
    // --exact keeps the ones named Cart.
    let (code, stdout, _) = p.run(&["find", "cart"]);
    assert_eq!(code, 0);
    assert!(stdout.starts_with("find 'cart': 4 symbol(s)\n"), "{stdout}");
    let (code, stdout, _) = p.run(&["find", "cart", "--exact"]);
    assert_eq!(code, 0);
    assert_eq!(
        stdout,
        "find 'cart' (exact name): 2 symbol(s)\nsrc/cart.rs:1-3  struct Cart\nsrc/cart.rs:5-14  impl Cart\n"
    );
    let (code, stdout, _) = p.run(&["find", "check", "--exact", "--json"]);
    assert_eq!(code, 0);
    assert_eq!(stdout.trim(), "[]");

    // todos: whole-word markers only; --loose is the old substring match.
    let (code, stdout, _) = p.run(&["todos"]);
    assert_eq!(code, 0);
    assert_eq!(
        stdout,
        "todos: 1 hit(s)\nsrc/cart.rs:10: // TODO: discounts\n"
    );
    let (code, stdout, _) = p.run(&["todos", "--loose"]);
    assert_eq!(code, 0);
    assert!(stdout.starts_with("todos: 2 hit(s)\n"), "{stdout}");
    assert!(
        stdout.contains("docs/notes.md:3: List todos here"),
        "{stdout}"
    );

    // The same over MCP.
    let replies = mcp_session(
        &p,
        &[
            json!({"jsonrpc":"2.0","id":0,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"test","version":"1"}}}),
            json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
            call(
                1,
                "find_symbol",
                json!({"pattern": "cart", "exact": true, "output": "json"}),
            ),
            call(2, "todos", json!({})),
            call(3, "todos", json!({"loose": true})),
            call(
                4,
                "extract",
                json!({"input": "logs", "rules": ["regex:code=(\\d+)"], "output": "json"}),
            ),
            call(5, "find_symbol", json!({"pattern": "cart", "exact": "yes"})),
            json!({"jsonrpc":"2.0","id":6,"method":"tools/list"}),
        ],
    );
    assert_eq!(replies.len(), 7);
    let hits: Value = serde_json::from_str(text(&replies[1])).unwrap();
    let names: Vec<&str> = hits
        .as_array()
        .unwrap()
        .iter()
        .map(|h| h["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["Cart", "Cart"]);
    assert_eq!(
        text(&replies[2]),
        "todos: 1 hit(s)\nsrc/cart.rs:10: // TODO: discounts\n"
    );
    assert!(text(&replies[3]).starts_with("todos: 2 hit(s)\n"));
    let extracted: Value = serde_json::from_str(text(&replies[4])).unwrap();
    assert_eq!(extracted["results"][0]["items"], json!(["42"]));
    assert_eq!(extracted["results"][0]["lines"], json!([3]));
    assert_eq!(replies[5]["result"]["isError"], true, "{}", replies[5]);
    let schema = |name: &str| {
        replies[6]["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .find(|t| t["name"] == name)
            .unwrap()["inputSchema"]["properties"]
            .clone()
    };
    assert_eq!(schema("find_symbol")["exact"]["type"], "boolean");
    assert_eq!(schema("todos")["loose"]["type"], "boolean");
}
