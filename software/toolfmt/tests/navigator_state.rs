//! 1.4.0: the answers to the verification report on 1.3.0. Scans skip
//! service folders and navguard's own state files; deep and deep2 give
//! relative paths and only navigator.md the absolute root; `Status: READY`
//! instead of a second `[A,B,C]`; ISO timestamps in the workflow state;
//! `nav --check` writes nothing; `nav --version`; MCP resources
//! `project://tree` and `project://state`. Driven through the real binary.

use serde_json::{Value, json};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

static ID: AtomicU64 = AtomicU64::new(0);

/// A project with two files of the same name, an empty file, a file
/// without a final line break and the folder of another tool.
struct Fixture {
    root: PathBuf,
}

impl Fixture {
    fn new(name: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "atp_navigator_state_{}_{}_{}",
            std::process::id(),
            name,
            ID.fetch_add(1, Ordering::Relaxed)
        ));
        let write = |rel: &str, content: &str| {
            let path = root.join(rel);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, content).unwrap();
        };
        write("a/dispatch.md", "# A\n\nfirst\n");
        write("b/dispatch.md", "# B\n\nsecond\nthird\n");
        write("src/lib.rs", "pub fn run() {\n    helper();\n}\n");
        write("empty.txt", "");
        write("tail.txt", "one\ntwo\nthree");
        write(".zvec-grep/CURRENT", "MANIFEST-000001\n");
        write(".zvec-grep/000001.sst", "index\n");
        Fixture { root }
    }

    fn run(&self, args: &[&str]) -> (i32, String, String) {
        let output = Command::new(env!("CARGO_BIN_EXE_any_txt_parser"))
            .args(args)
            .current_dir(&self.root)
            .output()
            .unwrap();
        (
            output.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&output.stdout).into_owned(),
            String::from_utf8_lossy(&output.stderr).into_owned(),
        )
    }

    fn ok(&self, args: &[&str]) -> String {
        let (code, stdout, stderr) = self.run(args);
        assert_eq!(code, 0, "{args:?}: {stdout}{stderr}");
        stdout
    }

    fn read(&self, rel: &str) -> String {
        std::fs::read_to_string(self.root.join(rel)).unwrap()
    }

    /// One MCP session (`mcp --root <root>`, started elsewhere) sending
    /// `requests` after initialize; returns every reply, initialize first.
    fn mcp(&self, requests: &[Value]) -> Vec<Value> {
        let mut child = Command::new(env!("CARGO_BIN_EXE_any_txt_parser"))
            .arg("mcp")
            .arg("--root")
            .arg(&self.root)
            .current_dir(std::env::temp_dir())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        {
            let stdin = child.stdin.as_mut().unwrap();
            writeln!(stdin, "{}", json!({"jsonrpc":"2.0","id":0,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"t","version":"1"}}})).unwrap();
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
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

fn read_resource(id: u64, uri: &str) -> Value {
    json!({"jsonrpc":"2.0","id":id,"method":"resources/read","params":{"uri":uri}})
}

fn resource_text(reply: &Value) -> &str {
    reply["result"]["contents"][0]["text"].as_str().unwrap()
}

const PROJECT_FILES: [&str; 5] = [
    "a/dispatch.md",
    "b/dispatch.md",
    "empty.txt",
    "src/lib.rs",
    "tail.txt",
];

#[test]
fn scans_skip_tool_folders_and_navguard_state_files() {
    let f = Fixture::new("scan");
    let status = f.ok(&["nav", "--triangle", "step_done"]);
    assert!(status.contains("[0,0,1] -> [1,0,0]"), "{status}");
    assert!(f.root.join("scripts/.abc_triangle_state.json").is_file());
    assert!(f.root.join("scripts/.abc_triangle_log.json").is_file());
    f.ok(&["nav", "--set-rules", "rules"]);

    let map = f.ok(&["map"]);
    assert!(map.starts_with("map .: 5 files, 13 lines,"), "{map}");
    for hidden in [".zvec-grep", ".abc_triangle", "scripts", ".navguard_rules"] {
        assert!(!map.contains(hidden), "{hidden} in {map}");
    }
    for file in PROJECT_FILES {
        assert!(
            map.contains(&format!("  {file}  ")),
            "{file} missing: {map}"
        );
    }
    f.ok(&["nav", "--oneshot"]);
    let nav = f.read("navigator.md");
    assert!(nav.contains("\nFiles: 5 | Lines: 13 |"), "{nav}");
    assert!(!nav.contains(".zvec-grep") && !nav.contains("scripts/"));
}

#[test]
fn deep_files_carry_relative_paths_and_the_root_is_written_once() {
    let f = Fixture::new("paths");
    f.ok(&["nav", "--oneshot"]);
    let (nav, deep, deep2) = (
        f.read("navigator.md"),
        f.read("navigator_deep.md"),
        f.read("navigator_deep2.md"),
    );
    let root_lines = [&nav, &deep, &deep2]
        .iter()
        .flat_map(|text| text.lines())
        .filter(|line| line.starts_with("Root: "))
        .count();
    assert_eq!(root_lines, 1);
    assert!(nav.contains("\nStatus: READY\n"), "{nav}");
    assert!(
        !nav.contains("Triangle") && !nav.contains("[0,1,0]"),
        "{nav}"
    );
    // Same name in two folders: the rows differ by path.
    let a = deep
        .lines()
        .find(|l| l.starts_with("| a/dispatch.md | "))
        .unwrap();
    let b = deep
        .lines()
        .find(|l| l.starts_with("| b/dispatch.md | "))
        .unwrap();
    assert_ne!(a, b);
    // Every tree pointer lands on the row of its own file.
    for line in nav.lines().filter(|l| l.contains("  deep L")) {
        let n: usize = line.rsplit("deep L").next().unwrap().parse().unwrap();
        let name = line.trim_start_matches(['│', '├', '└', '─', ' ']);
        let name = name.split("  deep L").next().unwrap();
        let row = deep.lines().nth(n - 1).unwrap();
        let path = row[2..].split(" | ").next().unwrap();
        assert!(path.rsplit('/').next() == Some(name), "{line} -> {row}");
    }
    assert!(deep2.contains("\n### src/lib.rs  (deep L"), "{deep2}");
}

#[test]
fn workflow_state_times_are_iso_8601() {
    let f = Fixture::new("triangle");
    f.ok(&["nav", "--triangle", "step_done"]);
    let state: Value = serde_json::from_str(&f.read("scripts/.abc_triangle_state.json")).unwrap();
    let log: Value = serde_json::from_str(&f.read("scripts/.abc_triangle_log.json")).unwrap();
    for time in [&state["timestamp"], &log[0]["timestamp"]] {
        let time = time.as_str().unwrap();
        assert!(
            chrono::DateTime::parse_from_rfc3339(time).is_ok(),
            "not ISO 8601: {time}"
        );
    }
}

#[test]
fn check_writes_no_navigators_and_nav_has_a_version() {
    let f = Fixture::new("check");
    let report = f.ok(&["nav", "--check"]);
    assert!(report.starts_with("T: 0,0,1\n"), "{report}");
    assert!(report.contains("F: a/dispatch.md|3|"), "{report}");
    for name in ["navigator.md", "navigator_deep.md", "navigator_deep2.md"] {
        assert!(!f.root.join(name).exists(), "--check wrote {name}");
    }
    let help = f.ok(&["nav", "--help"]);
    assert!(help.contains("writes no navigator files"), "{help}");

    let version = f.ok(&["--version"]);
    assert!(version.starts_with("any_txt_parser "), "{version}");
    assert_eq!(f.ok(&["nav", "--version"]), version);
    assert_eq!(f.ok(&["nav", "--root", ".", "-V"]), version);
}

#[test]
fn mcp_serves_tree_and_state_resources() {
    let f = Fixture::new("mcp");
    let replies = f.mcp(&[
        json!({"jsonrpc":"2.0","id":1,"method":"resources/list"}),
        read_resource(2, "project://tree"),
        read_resource(3, "project://state"),
        read_resource(4, "project://nope"),
    ]);
    assert_eq!(
        replies[0]["result"]["capabilities"]["resources"]["listChanged"],
        false
    );
    let uris: Vec<&str> = replies[1]["result"]["resources"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["uri"].as_str().unwrap())
        .collect();
    assert_eq!(uris, ["project://tree", "project://state"]);

    let tree = resource_text(&replies[2]);
    assert_eq!(
        replies[2]["result"]["contents"][0]["mimeType"],
        "text/plain"
    );
    let header = tree.lines().next().unwrap();
    assert!(header.contains(": 5 files, 13 lines, ~"), "{header}");
    let shown = header["map ".len()..].split(": 5 files").next().unwrap();
    assert!(Path::new(shown).is_absolute(), "{header}");
    assert!(!tree.contains(".zvec-grep"), "{tree}");
    for file in PROJECT_FILES {
        assert!(tree.contains(&format!("  {file}  ")), "{file}: {tree}");
    }

    let state: Value = serde_json::from_str(resource_text(&replies[3])).unwrap();
    assert_eq!(state["navigators"]["status"], "missing");
    assert_eq!(state["totals"]["files"], 5);
    assert_eq!(state["workflow"]["triangle"], json!([0, 0, 1]));
    assert!(state["snapshot"].is_null());
    assert_eq!(replies[4]["error"]["code"], -32002);

    // After nav --oneshot the navigators are fresh; after an edit, stale.
    f.ok(&["nav", "--oneshot"]);
    let replies = f.mcp(&[read_resource(1, "project://state")]);
    let state: Value = serde_json::from_str(resource_text(&replies[1])).unwrap();
    assert_eq!(state["navigators"]["status"], "fresh", "{state}");
    let nav = f.read("navigator.md");
    let updated = nav
        .lines()
        .find_map(|l| l.strip_prefix("Updated: "))
        .unwrap();
    assert_eq!(state["navigators"]["updated"], updated);
    assert!(state["snapshot"].is_string());

    std::fs::write(f.root.join("b/dispatch.md"), "# B\n\nchanged\n").unwrap();
    std::fs::write(f.root.join("new.txt"), "new\n").unwrap();
    let replies = f.mcp(&[read_resource(1, "project://state")]);
    let state: Value = serde_json::from_str(resource_text(&replies[1])).unwrap();
    let navigators = &state["navigators"];
    assert_eq!(navigators["status"], "stale", "{state}");
    assert_eq!(
        (
            &navigators["new"],
            &navigators["changed"],
            &navigators["deleted"]
        ),
        (&json!(1), &json!(1), &json!(0))
    );
    assert_eq!(navigators["paths"], json!(["b/dispatch.md", "new.txt"]));
}
