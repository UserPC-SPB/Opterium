//! 1.3.0: commands, the MCP server and `nav` stay inside the project root;
//! `nav --patch` has a dry run and fails loudly; the navigators point at
//! exact lines. Driven through the real binary.

use serde_json::{Value, json};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

static ID: AtomicU64 = AtomicU64::new(0);

/// `<tmp>/<unique>/root` (the project) next to `<tmp>/<unique>/outside.txt`.
struct Layout {
    base: PathBuf,
    root: PathBuf,
}

impl Layout {
    fn new(name: &str) -> Self {
        let base = std::env::temp_dir().join(format!(
            "atp_root_bounds_{}_{}_{}",
            std::process::id(),
            name,
            ID.fetch_add(1, Ordering::Relaxed)
        ));
        let root = base.join("root");
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(
            root.join("src/app.rs"),
            "fn main() {\r\n    let port = 80;\r\n}\r\n",
        )
        .unwrap();
        std::fs::write(root.join("notes.txt"), "one\ntwo\nthree\n").unwrap();
        std::fs::write(base.join("outside.txt"), "secret\n").unwrap();
        Layout { base, root }
    }

    fn outside(&self) -> String {
        self.base.join("outside.txt").to_string_lossy().into_owned()
    }

    fn run_in(&self, dir: &Path, args: &[&str]) -> (i32, String, String) {
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

    fn run(&self, args: &[&str]) -> (i32, String, String) {
        self.run_in(&self.root, args)
    }

    /// One MCP session started in `dir` with `args` after `mcp`.
    fn mcp(&self, dir: &Path, args: &[&str], calls: &[(&str, Value)]) -> Vec<Value> {
        let mut child = Command::new(env!("CARGO_BIN_EXE_any_txt_parser"))
            .arg("mcp")
            .args(args)
            .current_dir(dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        {
            let stdin = child.stdin.as_mut().unwrap();
            writeln!(stdin, "{}", json!({"jsonrpc":"2.0","id":0,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"t","version":"1"}}})).unwrap();
            for (id, (name, arguments)) in calls.iter().enumerate() {
                let request = json!({"jsonrpc":"2.0","id":id + 1,"method":"tools/call","params":{"name":name,"arguments":arguments}});
                writeln!(stdin, "{}", request).unwrap();
            }
        }
        let output = child.wait_with_output().unwrap();
        assert_eq!(output.status.code(), Some(0));
        String::from_utf8(output.stdout)
            .unwrap()
            .lines()
            .skip(1)
            .map(|line| serde_json::from_str(line).unwrap())
            .collect()
    }
}

impl Drop for Layout {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.base);
    }
}

fn text(reply: &Value) -> &str {
    reply["result"]["content"][0]["text"].as_str().unwrap()
}

fn is_error(reply: &Value) -> bool {
    reply["result"]["isError"] == true
}

#[test]
fn commands_refuse_files_outside_the_root() {
    let l = Layout::new("cli");
    let outside = l.outside();
    for path in [outside.as_str(), "../outside.txt"] {
        let (code, stdout, stderr) = l.run(&["lines", path, "1"]);
        assert_eq!(code, 3, "{stdout}{stderr}");
        assert!(stderr.contains("outside the project root"), "{stderr}");
        assert!(!stdout.contains("secret"));
    }
    for args in [
        vec!["symbols", outside.as_str()],
        vec!["search", "secret", outside.as_str()],
    ] {
        let (code, _, stderr) = l.run(&args);
        assert_eq!(code, 3, "{args:?}: {stderr}");
    }
    // With the folder as the root the same file is readable.
    let base = l.base.to_string_lossy().into_owned();
    let (code, stdout, _) = l.run(&["lines", "outside.txt", "1", "--root", &base]);
    assert_eq!(code, 0);
    assert_eq!(stdout, "     1| secret\n");
}

#[test]
fn line_and_token_counts() {
    let l = Layout::new("counts");
    std::fs::write(l.root.join("empty.txt"), "").unwrap();
    std::fs::write(l.root.join("mac.txt"), "a\rb\rc").unwrap();
    let (code, stdout, _) = l.run(&["map"]);
    assert_eq!(code, 0);
    assert!(stdout.contains("  notes.txt  3 ~5\n"), "{stdout}");
    assert!(stdout.contains("  empty.txt  0 ~0\n"), "{stdout}");
    assert!(stdout.contains("  mac.txt  3 ~2\n"), "{stdout}");
    assert!(stdout.contains("  src/app.rs  3 ~"), "{stdout}");
    let (_, stdout, _) = l.run(&["lines", "mac.txt", "2", "3"]);
    assert_eq!(stdout, "     2| b\n     3| c\n");
}

#[test]
fn mcp_server_root_bounds_every_tool() {
    let l = Layout::new("mcp");
    let outside = l.outside();
    let base = l.base.to_string_lossy().into_owned();
    let replies = l.mcp(
        &l.root,
        &[],
        &[
            ("read_lines", json!({"path": outside, "start": 1})),
            ("read_lines", json!({"path": "../outside.txt", "start": 1})),
            ("map", json!({"root": base})),
            ("map", json!({"root": ".."})),
            (
                "extract",
                json!({"input": outside, "rules": ["contains:secret"]}),
            ),
            (
                "extract",
                json!({"input": "../*.txt", "rules": ["contains:secret"]}),
            ),
            (
                "extract",
                json!({"input": "src/*/../../..", "rules": ["contains:secret"]}),
            ),
            ("recon", json!({"path": outside})),
            (
                "extract",
                json!({"input": "notes.txt", "template_path": outside}),
            ),
            // Inside the root everything works.
            ("read_lines", json!({"path": "notes.txt", "start": 2})),
            (
                "extract",
                json!({"input": "*.txt", "rules": ["contains:two"]}),
            ),
            ("map", json!({"root": "src"})),
        ],
    );
    assert_eq!(replies.len(), 12);
    for reply in &replies[..9] {
        assert!(is_error(reply), "{reply}");
        assert!(!text(reply).contains("secret\n"), "{reply}");
        assert!(text(reply).contains("outside"), "{reply}");
    }
    assert_eq!(text(&replies[9]), "     2| two\n");
    assert!(text(&replies[10]).contains("two"), "{}", replies[10]);
    assert!(
        text(&replies[11]).starts_with("map src: 1 files"),
        "{}",
        replies[11]
    );
}

#[test]
fn mcp_root_option_moves_the_server_into_the_project() {
    let l = Layout::new("mcp_root");
    let root = l.root.to_string_lossy().into_owned();
    let replies = l.mcp(
        &l.base,
        &["--root", &root],
        &[
            (
                "read_lines",
                json!({"path": "notes.txt", "start": 1, "end": 1}),
            ),
            ("read_lines", json!({"path": "../outside.txt", "start": 1})),
        ],
    );
    assert_eq!(text(&replies[0]), "     1| one\n");
    assert!(is_error(&replies[1]));
    let (code, _, stderr) = l.run(&["mcp", "--root", "no-such-folder"]);
    assert_eq!(code, 3);
    assert!(stderr.contains("no-such-folder"), "{stderr}");
}

#[cfg(windows)]
#[test]
fn junctions_lead_nowhere() {
    let l = Layout::new("junction");
    let link = l.root.join("link");
    let made = Command::new("cmd")
        .args(["/C", "mklink", "/J"])
        .arg(&link)
        .arg(&l.base)
        .output()
        .unwrap();
    assert!(made.status.success(), "{made:?}");
    let (code, stdout, _) = l.run(&["map"]);
    assert_eq!(code, 0);
    assert!(!stdout.contains("outside.txt"), "{stdout}");
    let (code, _, stderr) = l.run(&["lines", "link/outside.txt", "1"]);
    assert_eq!(code, 3);
    assert!(stderr.contains("outside the project root"), "{stderr}");
    let replies = l.mcp(
        &l.root,
        &[],
        &[
            (
                "read_lines",
                json!({"path": "link/outside.txt", "start": 1}),
            ),
            (
                "extract",
                json!({"input": "link", "rules": ["contains:secret"], "follow_symlinks": true}),
            ),
            ("search_text", json!({"pattern": "secret"})),
        ],
    );
    assert!(is_error(&replies[0]), "{}", replies[0]);
    assert!(is_error(&replies[1]), "{}", replies[1]);
    assert!(text(&replies[2]).contains("0 hit(s)"), "{}", replies[2]);
}

#[test]
fn nav_patch_stays_inside_and_fails_loudly() {
    let l = Layout::new("patch");
    let outside = l.outside();
    let (code, _, stderr) = l.run(&["nav", "--patch", "replace", &outside, "secret", "x"]);
    assert_eq!(code, 1);
    assert!(stderr.contains("outside the project root"), "{stderr}");
    assert_eq!(std::fs::read_to_string(&outside).unwrap(), "secret\n");

    let (code, _, stderr) = l.run(&["nav", "--patch", "replace", "notes.txt", "NOPE", "x"]);
    assert_eq!(code, 1);
    assert!(stderr.contains("not found"), "{stderr}");

    let (code, stdout, _) = l.run(&[
        "nav",
        "--patch",
        "replace",
        "src/app.rs",
        "port = 80",
        "port = 8080",
        "--dry-run",
    ]);
    assert_eq!(code, 0);
    assert!(stdout.contains("dry-run"), "{stdout}");
    assert!(
        stdout.contains("-      2|     let port = 80;\n+      2|     let port = 8080;\n"),
        "{stdout}"
    );
    let source = l.root.join("src/app.rs");
    assert_eq!(
        std::fs::read_to_string(&source).unwrap(),
        "fn main() {\r\n    let port = 80;\r\n}\r\n"
    );

    let (code, _, _) = l.run(&[
        "nav",
        "--patch",
        "insert",
        "src/app.rs",
        "3",
        "    run(port);",
    ]);
    assert_eq!(code, 0);
    assert_eq!(
        std::fs::read_to_string(&source).unwrap(),
        "fn main() {\r\n    let port = 80;\r\n    run(port);\r\n}\r\n"
    );
    let (code, _, stderr) = l.run(&["nav", "--patch", "delete", "src/app.rs", "9"]);
    assert_eq!(code, 1);
    assert!(stderr.contains("out of range"), "{stderr}");
}

#[test]
fn help_flags_print_help() {
    let l = Layout::new("help");
    let (code, stdout, _) = l.run(&["help", "--help"]);
    assert_eq!(code, 0);
    assert!(stdout.contains("navigation"), "{stdout}");
    let (code, stdout, _) = l.run(&["nav", "--help"]);
    assert_eq!(code, 0);
    assert!(stdout.contains("--oneshot"), "{stdout}");
    let (code, stdout, _) = l.run(&["nav", "--patch", "help"]);
    assert_eq!(code, 0);
    assert!(stdout.contains("--dry-run"), "{stdout}");
}

#[test]
fn nav_oneshot_writes_linked_navigators() {
    let l = Layout::new("navigators");
    let (code, _, stderr) = l.run(&["nav", "--oneshot"]);
    assert_eq!(code, 0, "{stderr}");
    let read = |name: &str| std::fs::read_to_string(l.root.join(name)).unwrap();
    let (nav, deep, deep2) = (
        read("navigator.md"),
        read("navigator_deep.md"),
        read("navigator_deep2.md"),
    );
    let root_line = nav.lines().find(|l| l.starts_with("Root: ")).unwrap();
    assert!(Path::new(&root_line[6..]).is_absolute(), "{root_line}");
    assert!(root_line.ends_with("root"), "{root_line}");
    let updated = nav.lines().find(|l| l.starts_with("Updated: ")).unwrap();
    assert!(deep.contains(updated) && deep2.contains(updated));
    assert!(nav.contains("\nStatus: READY\n"));
    // The absolute root only in navigator.md.
    assert!(!deep.contains("Root: ") && !deep2.contains("Root: "));
    // app.rs -> its deep row -> its deep2 section.
    let tree_line = nav.lines().find(|l| l.contains("app.rs  deep L")).unwrap();
    let n: usize = tree_line.rsplit("deep L").next().unwrap().parse().unwrap();
    let row = deep.lines().nth(n - 1).unwrap();
    assert!(row.starts_with("| src/app.rs | "), "{row}");
    let range = row.rsplit("deep2 L").next().unwrap().trim_end_matches(" |");
    let start: usize = range.split("-L").next().unwrap().parse().unwrap();
    assert_eq!(
        deep2.lines().nth(start - 1).unwrap(),
        format!("### src/app.rs  (deep L{n})")
    );
    assert!(deep2.contains("| fn main() { | fn | 1-3 |"), "{deep2}");
}
