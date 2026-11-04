//! End to end: agent -> toolfmt gateway -> two demo upstream servers.

use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use toolfmt::json::J;

const EXE: &str = env!("CARGO_BIN_EXE_toolfmt");

struct Client {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    next: u64,
}

impl Client {
    fn start(home: &PathBuf) -> Client {
        let mut child = Command::new(EXE)
            .args(["gateway", "--home"])
            .arg(home)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("start gateway");
        let stdin = child.stdin.take().unwrap();
        let stdout = BufReader::new(child.stdout.take().unwrap());
        Client {
            child,
            stdin,
            stdout,
            next: 1,
        }
    }

    fn request(&mut self, method: &str, params: &str) -> J {
        let id = self.next;
        self.next += 1;
        writeln!(
            self.stdin,
            r#"{{"jsonrpc":"2.0","id":{id},"method":"{method}","params":{params}}}"#
        )
        .unwrap();
        self.stdin.flush().unwrap();
        loop {
            let mut line = String::new();
            assert!(
                self.stdout.read_line(&mut line).unwrap() > 0,
                "gateway closed stdout"
            );
            let msg = J::parse(line.trim()).expect("JSON line");
            if msg.get("id").and_then(J::as_u64) == Some(id) {
                return msg;
            }
        }
    }

    fn call(&mut self, tool: &str, args: &str) -> J {
        let r = self.request(
            "tools/call",
            &format!(r#"{{"name":"{tool}","arguments":{args}}}"#),
        );
        r.get("result")
            .cloned()
            .unwrap_or_else(|| panic!("no result: {}", r.to_json()))
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        let _ = self.child.kill();
    }
}

fn text(result: &J) -> String {
    result
        .get("content")
        .and_then(J::as_arr)
        .and_then(|c| c.first())
        .and_then(|b| b.get("text"))
        .map(J::text)
        .unwrap_or_default()
}

fn home(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("toolfmt-test-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let exe = J::str(EXE).to_json();
    std::fs::write(
        dir.join("upstreams.json"),
        format!(
            r#"{{"mcpServers":{{"alpha":{{"command":{exe},"args":["demo-server"]}},"beta":{{"command":{exe},"args":["demo-server"],"capture":false}}}}}}"#
        ),
    )
    .unwrap();
    dir
}

#[test]
fn gateway_end_to_end() {
    let dir = home("e2e");
    let mut c = Client::start(&dir);
    let init = c.request("initialize", r#"{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"t","version":"1"}}"#);
    assert!(
        init.get("result")
            .and_then(|r| r.get("instructions"))
            .is_some()
    );

    // tools/list: original names, collision namespaced, outputSchema removed.
    let list = c.request("tools/list", "{}");
    let tools = list
        .get("result")
        .unwrap()
        .get("tools")
        .unwrap()
        .as_arr()
        .unwrap()
        .clone();
    let names: Vec<String> = tools
        .iter()
        .map(|t| t.get("name").unwrap().text())
        .collect();
    for n in [
        "add",
        "search",
        "beta__search",
        "beta__add",
        "formatter_inspect",
        "formatter_save_standard",
    ] {
        assert!(names.iter().any(|x| x == n), "missing {n} in {names:?}");
    }
    assert!(tools.iter().all(|t| t.get("outputSchema").is_none()));

    // No standard: the result comes back exactly; big integers pass both ways.
    let r = c.call("add", r#"{"a":123456789012345678901234567889,"b":1}"#);
    assert_eq!(text(&r), "123456789012345678901234567890");
    assert_eq!(
        r.get("_meta").unwrap().get("echo_args").unwrap().to_json(),
        r#"{"a":123456789012345678901234567889,"b":1}"#
    );

    let raw = c.call("list_files", "{}");
    let raw_text = text(&raw);
    assert!(raw_text.starts_with("{\n  \"items\""), "{raw_text}");
    assert!(
        dir.join("fixtures")
            .join("alpha.list_files")
            .join("01.json")
            .is_file()
    );
    // Same output again: not captured twice.
    c.call("list_files", "{}");
    assert!(
        !dir.join("fixtures")
            .join("alpha.list_files")
            .join("02.json")
            .exists()
    );

    // Standards are refused without a captured output (D3)...
    let r = c.call(
        "formatter_save_standard",
        r#"{"tool":"numbers","yaml":"render: num_stats"}"#,
    );
    assert_eq!(r.get("isError").and_then(J::as_bool), Some(true));
    // ...and invalid ones are refused with the reason.
    let r = c.call(
        "formatter_save_standard",
        r#"{"tool":"list_files","yaml":"render: objects_table\npost: [truncate_lines: 2]"}"#,
    );
    assert!(text(&r).contains("I7"), "{}", text(&r));

    // Save a standard against the capture; the replay shows the new look.
    let yaml =
        J::str("render: objects_table\ncolumns: [path, size]\nformats: {size: bytes}\n").to_json();
    let r = c.call(
        "formatter_save_standard",
        &format!(r#"{{"tool":"list_files","yaml":{yaml}}}"#),
    );
    let report = text(&r);
    assert!(report.starts_with("saved"), "{report}");
    assert!(report.contains("items[12]{path,size}:"), "{report}");
    assert!(dir.join("standards").join("alpha.list_files.yml").is_file());

    let r = c.call("list_files", "{}");
    let t = text(&r);
    assert!(t.starts_with("[objects_table:lossy]\n"), "{t}");
    assert!(t.contains("next_cursor: page-2"), "cursor kept: {t}");
    assert!(t.contains("src/module_00.rs,1000 B"), "{t}");
    assert!(
        t.len() * 2 < raw_text.len(),
        "{} vs {}",
        t.len(),
        raw_text.len()
    );

    // Per-call override; a real `_limit` parameter of the tool is not stripped.
    let r = c.call(
        "search",
        r#"{"q":"mcp","_limit":2,"_render":"json","_pre":[{"select":["title"]}]}"#,
    );
    let t = text(&r);
    assert!(t.starts_with("[json"), "{t}");
    assert!(r.get("structuredContent").is_none());
    assert_eq!(
        r.get("_meta").unwrap().get("echo_args").unwrap().to_json(),
        r#"{"q":"mcp","_limit":2}"#
    );

    // Upstream errors are never formatted.
    let r = c.call("fail", "{}");
    assert_eq!(r.get("isError").and_then(J::as_bool), Some(true));
    assert_eq!(text(&r), "boom: invalid input");

    // Images: metadata instead of pixels.
    let r = c.call("snapshot", r#"{"_render":"image_meta"}"#);
    let blocks = r.get("content").unwrap().as_arr().unwrap();
    assert_eq!(blocks.len(), 1);
    assert!(text(&r).contains("image: png 1x1"), "{}", text(&r));

    // A broken standard: raw output plus a note, never a lost output (D2).
    std::fs::write(
        dir.join("standards").join("alpha.numbers.yml"),
        "render: matrix_table\n",
    )
    .unwrap();
    let r = c.call("numbers", "{}");
    let blocks = r.get("content").unwrap().as_arr().unwrap();
    assert!(
        text(&r).starts_with("[fallback:raw] matrix_table:"),
        "{}",
        text(&r)
    );
    assert_eq!(
        blocks[1].get("text").unwrap().text(),
        "[3,1,4,1,5,9,2,6,5,3,5,8,9,7,9,3,2,3,8,4]"
    );

    // Overview.
    let r = c.call("formatter_inspect", "{}");
    let t = text(&r);
    assert!(t.contains("list_files,alpha,yes,1"), "{t}");
    let r = c.call("formatter_inspect", r#"{"tool":"list_files"}"#);
    assert!(text(&r).contains("capture #1 payload:"), "{}", text(&r));

    // The second server's colliding tool works and is not captured (capture: false).
    let r = c.call("beta__add", r#"{"a":2,"b":3}"#);
    assert_eq!(text(&r), "5");
    assert!(!dir.join("fixtures").join("beta.add").exists());

    // Unknown tool: a JSON-RPC error, not a crash.
    let r = c.request("tools/call", r#"{"name":"nope","arguments":{}}"#);
    assert!(r.get("error").is_some());
    drop(c);

    // The captures became a replayable corpus with baselines.
    let out = Command::new(EXE)
        .args(["replay", "--home"])
        .arg(&dir)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("replay: 1/1 green"), "{stdout}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn catalog_mode_end_to_end() {
    let dir = std::env::temp_dir().join(format!("toolfmt-test-catalog-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let exe = J::str(EXE).to_json();
    std::fs::write(
        dir.join("upstreams.json"),
        format!(
            r#"{{"expose":"catalog","pin":["add"],
               "sections":{{"lab":"demo tools","lab/files":"files and logs"}},
               "mcpServers":{{
                 "alpha":{{"command":{exe},"args":["demo-server"],"section":"lab",
                           "groups":{{"list_*":"files","log_*":"files","snapshot":"eyes"}},
                           "prices":{{"snapshot":"$0.01/call"}},"price":"free"}},
                 "beta":{{"command":{exe},"args":["demo-server"],"section":"mirror"}}}}}}"#
        ),
    )
    .unwrap();
    let mut c = Client::start(&dir);
    c.request("initialize", r#"{"protocolVersion":"2025-06-18"}"#);

    // Only the pinned tool, the catalog pair and the formatter tools are listed.
    let list = c.request("tools/list", "{}");
    let tools = list
        .get("result")
        .unwrap()
        .get("tools")
        .unwrap()
        .as_arr()
        .unwrap()
        .clone();
    let mut names: Vec<String> = tools
        .iter()
        .map(|t| t.get("name").unwrap().text())
        .collect();
    names.sort();
    assert_eq!(
        names,
        [
            "add",
            "call_tool",
            "catalog",
            "formatter_inspect",
            "formatter_save_standard"
        ]
    );

    let t = text(&c.call("catalog", "{}"));
    assert!(t.starts_with("catalog: 2 sections, 22 tools"), "{t}");
    assert!(t.contains("\n  lab 2_11 — demo tools"), "{t}");
    assert!(t.contains("\n  mirror 0_11"), "{t}");
    let t = text(&c.call("catalog", r#"{"path":"lab/files"}"#));
    assert!(t.starts_with("lab/files 0_2 — files and logs"), "{t}");
    assert!(t.contains("list_files (ro) — Lists files"), "{t}");
    let t = text(&c.call("catalog", r#"{"query":"screenshot"}"#));
    assert!(
        t.lines()
            .nth(1)
            .unwrap()
            .trim_start()
            .starts_with("snapshot"),
        "{t}"
    );

    // call_tool goes through the same path: untouched output, capture, stats.
    let r = c.call("call_tool", r#"{"tool":"list_files","args":{}}"#);
    assert!(text(&r).starts_with("{\n  \"items\""), "{}", text(&r));
    assert!(
        dir.join("fixtures")
            .join("alpha.list_files")
            .join("01.json")
            .is_file()
    );
    // Arguments given as a JSON string work too.
    let r = c.call("call_tool", r#"{"tool":"add","args":"{\"a\":2,\"b\":3}"}"#);
    assert_eq!(text(&r), "5");

    let t = text(&c.call("catalog", r#"{"tools":["list_files","snapshot","nope"]}"#));
    assert!(t.contains("list_files (ro) [lab/files]"), "{t}");
    assert!(t.contains("call: list_files(cursor?: string)"), "{t}");
    assert!(t.contains("measured: 1 calls, avg"), "{t}");
    assert!(
        t.contains("price: free") && t.contains("price: $0.01/call"),
        "{t}"
    );
    assert!(t.contains("example: {}"), "{t}");
    assert!(t.contains("nope: unknown tool."), "{t}");

    // Missing required arguments are reported with the signature.
    let r = c.call("call_tool", r#"{"tool":"add","args":{"a":1}}"#);
    assert_eq!(r.get("isError").and_then(J::as_bool), Some(true));
    assert_eq!(
        text(&r),
        "missing required b: call add(a: number, b: number)"
    );
    let r = c.call("call_tool", r#"{"tool":"snapshott"}"#);
    assert!(
        text(&r).starts_with("unknown tool 'snapshott'."),
        "{}",
        text(&r)
    );

    // Collisions keep working behind the catalog.
    let r = c.call("call_tool", r#"{"tool":"beta__add","args":{"a":1,"b":1}}"#);
    assert_eq!(text(&r), "2");
    let t = text(&c.call("formatter_inspect", "{}"));
    assert!(t.contains("tools offered as: catalog + call_tool"), "{t}");
    drop(c);

    // A new gateway remembers the measurements (audit log).
    let mut c = Client::start(&dir);
    c.request("initialize", "{}");
    let t = text(&c.call("catalog", r#"{"tools":["list_files"]}"#));
    assert!(t.contains("measured: 1 calls"), "{t}");
    drop(c);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn clean_contract_on_stdin() {
    let mut child = Command::new(EXE)
        .args(["clean", "--home"])
        .arg(std::env::temp_dir().join("toolfmt-no-such-home"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(br#"{"server":"s","tool":"t","data":"[[1,2,3],[4,5,6],[7,8,9],[10,11,12]]","_render":"matrix_table"}"#)
        .unwrap();
    let out = child.wait_with_output().unwrap();
    let resp = J::parse(String::from_utf8_lossy(&out.stdout).trim()).unwrap();
    assert_eq!(resp.get("used").unwrap().text(), "matrix_table:t");
    assert_eq!(
        resp.get("text").unwrap().text(),
        "4x3\n1 2 3\n4 5 6\n7 8 9\n10 11 12"
    );
}
