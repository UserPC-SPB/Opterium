//! The gateway as an MCP proxy: names that never move to another tool,
//! cancellation and progress, protocol versions, the shape of a formatted
//! result, one restart for many callers, and what saving a standard reports.

mod common;
use common::{Client, home, text};
use std::time::{Duration, Instant};
use toolfmt::json::J;

/// One test-mode server, a timeout long enough to cancel by hand.
const T: &str = r#"{"expose":"all","mcpServers":{
  "t":{"command":{exe},"args":["demo-server","--test-modes"],"timeout":60}}}"#;

fn start(name: &str, cfg: &str) -> (Client, std::path::PathBuf) {
    let dir = home(name, cfg);
    let mut c = Client::start(&dir);
    c.request("initialize", r#"{"protocolVersion":"2025-06-18"}"#);
    (c, dir)
}

fn served_by(r: &J) -> String {
    r.get("_meta")
        .and_then(|m| m.get("server"))
        .map(J::text)
        .unwrap_or_default()
}

#[test]
fn names_never_move_to_a_server_that_starts_later() {
    // "early" comes first in the file but is up only after the client has
    // listed the tools: the bare names are already the other server's.
    let (mut c, dir) = start(
        "late-names",
        r#"{"expose":"all","startup_wait":1,"mcpServers":{
          "early":{"command":{exe},"args":["demo-server","--tag","early","--init-delay-ms","3000"]},
          "late":{"command":{exe},"args":["demo-server","--tag","late"]}}}"#,
    );
    let names = c.tool_names();
    assert!(names.iter().any(|n| n == "delete_all"), "{names:?}");
    assert!(!names.iter().any(|n| n.starts_with("early__")), "{names:?}");
    // A destructive tool: the call goes where the listed name pointed...
    assert_eq!(served_by(&c.call("delete_all", "{}")), "late");
    assert!(
        c.saw("notifications/tools/list_changed", 30),
        "the slow server never came up"
    );
    let names = c.tool_names();
    assert!(names.iter().any(|n| n == "early__delete_all"), "{names:?}");
    assert!(names.iter().any(|n| n == "delete_all"), "{names:?}");
    // ...and keeps going there after the earlier server appeared.
    assert_eq!(served_by(&c.call("delete_all", "{}")), "late");
    assert_eq!(served_by(&c.call("early__delete_all", "{}")), "early");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_client_cancellation_reaches_the_upstream() {
    let (mut c, dir) = start("cancel", T);
    c.tool_names();
    let id = c.send(
        "tools/call",
        r#"{"name":"sleep","arguments":{"ms":30000},"_meta":{"progressToken":"p-1"}}"#,
    );
    // Progress passes through with the client's own token; it also says
    // the upstream is now busy with the request.
    let p = c
        .note("notifications/progress", 30)
        .expect("no progress notification");
    let params = p.get("params").unwrap();
    assert_eq!(
        params.get("progressToken").map(J::text).as_deref(),
        Some("p-1")
    );
    c.notify(
        "notifications/cancelled",
        &format!(r#"{{"requestId":{id},"reason":"user pressed stop"}}"#),
    );
    let t0 = Instant::now();
    let t = text(&c.call("cancels", "{}"));
    // The upstream stopped at once (it would sleep 30 s) and knows why.
    assert!(t.contains("(user pressed stop)"), "{t}");
    assert!(t0.elapsed() < Duration::from_secs(10), "{:?}", t0.elapsed());
    // A cancelled request gets no answer (MCP).
    assert!(!c.got(id, 1), "the cancelled request was answered");
    // Cancelling what is already answered is ignored.
    let done = c.send("tools/call", r#"{"name":"add","arguments":{"a":1,"b":2}}"#);
    assert_eq!(text(c.wait(done).get("result").unwrap()), "3");
    c.notify(
        "notifications/cancelled",
        &format!(r#"{{"requestId":{done}}}"#),
    );
    assert_eq!(text(&c.call("add", r#"{"a":2,"b":2}"#)), "4");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn protocol_versions_are_negotiated() {
    let dir = home(
        "protocol",
        r#"{"expose":"all","mcpServers":{
          "ok":{"command":{exe},"args":["demo-server","--protocol","2025-03-26"]},
          "odd":{"command":{exe},"args":["demo-server","--protocol","1999-01-01"]}}}"#,
    );
    let mut c = Client::start(&dir);
    let version = |r: &J| {
        r.get("result")
            .and_then(|r| r.get("protocolVersion"))
            .map(J::text)
            .unwrap_or_default()
    };
    // A version toolfmt does not speak gets its newest one.
    let r = c.request("initialize", r#"{"protocolVersion":"2099-01-01"}"#);
    assert_eq!(version(&r), "2025-06-18");
    let r = c.request("initialize", r#"{"protocolVersion":"2024-11-05"}"#);
    assert_eq!(version(&r), "2024-11-05");
    // An upstream answering a version toolfmt does not speak is dropped,
    // with the reason in the overview.
    let names = c.tool_names();
    assert!(names.iter().any(|n| n == "add"), "{names:?}");
    assert!(!names.iter().any(|n| n.starts_with("odd__")), "{names:?}");
    let t = text(&c.call("formatter_inspect", "{}"));
    assert!(
        t.contains("server ok: 11 tools, protocol 2025-03-26"),
        "{t}"
    );
    assert!(
        t.contains("server odd: failed: incompatible protocol version '1999-01-01'"),
        "{t}"
    );
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn many_callers_restart_a_dead_server_once() {
    let log = std::env::temp_dir().join(format!("toolfmt-starts-{}.log", std::process::id()));
    let _ = std::fs::remove_file(&log);
    let cfg = format!(
        r#"{{"expose":"all","mcpServers":{{"t":{{"command":{{exe}},"restart_backoff":0,
          "args":["demo-server","--test-modes","--start-log",{}]}}}}}}"#,
        J::str(log.display().to_string()).to_json()
    );
    let (mut c, dir) = start("restart-once", &cfg);
    c.tool_names();
    let r = c.call_raw("crash", "{}");
    assert!(r.get("error").is_some(), "{}", r.to_json());
    let ids: Vec<u64> = (0..24)
        .map(|i| {
            c.send(
                "tools/call",
                &format!(r#"{{"name":"add","arguments":{{"a":{i},"b":1}}}}"#),
            )
        })
        .collect();
    for (i, id) in ids.into_iter().enumerate() {
        let r = c.wait(id);
        let res = r.get("result").unwrap_or_else(|| panic!("{}", r.to_json()));
        assert_eq!(text(res), (i + 1).to_string());
    }
    let starts = std::fs::read_to_string(&log).unwrap();
    assert_eq!(
        starts.lines().count(),
        2,
        "one start, one restart:\n{starts}"
    );
    let _ = std::fs::remove_file(&log);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_formatted_result_keeps_what_the_text_did_not_replace() {
    let dir = home("mixed", T);
    std::fs::create_dir_all(dir.join("standards")).unwrap();
    std::fs::write(
        dir.join("standards").join("t.mixed.yml"),
        "render: objects_table\nroot: items\n",
    )
    .unwrap();
    let mut c = Client::start(&dir);
    c.request("initialize", r#"{"protocolVersion":"2025-06-18"}"#);
    let r = c.call("mixed", "{}");
    let blocks = r.get("content").unwrap().as_arr().unwrap();
    let kinds: Vec<String> = blocks
        .iter()
        .map(|b| b.get("type").unwrap().text())
        .collect();
    assert_eq!(
        kinds,
        ["text", "text", "image", "resource_link"],
        "{}",
        r.to_json()
    );
    // The summary was not part of the payload: it stays, first, annotated.
    assert_eq!(blocks[0].get("text").unwrap().text(), "2 items found");
    assert_eq!(
        blocks[0].get("annotations").unwrap().to_json(),
        r#"{"audience":["user"]}"#
    );
    // The JSON copy of structuredContent became the formatted text, in its
    // place and with its annotations.
    let t = blocks[1].get("text").unwrap().text();
    assert!(t.starts_with("[objects_table"), "{t}");
    assert!(t.contains("1,alpha") && t.contains("2,beta"), "{t}");
    assert_eq!(
        blocks[1].get("annotations").unwrap().to_json(),
        r#"{"audience":["assistant"]}"#
    );
    assert!(r.get("structuredContent").is_none());
    assert_eq!(r.get("x_trace").map(J::text).as_deref(), Some("trace-1"));
    assert!(r.get("_meta").and_then(|m| m.get("echo_args")).is_some());
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_standard_that_falls_back_is_saved_only_with_force() {
    let (mut c, dir) = start("save-force", T);
    assert_eq!(text(&c.call("add", r#"{"a":2,"b":3}"#)), "5");
    let save = |c: &mut Client, extra: &str| {
        c.call(
            "formatter_save_standard",
            &format!(r#"{{"tool":"add","yaml":"render: matrix_table"{extra}}}"#),
        )
    };
    let file = dir.join("standards").join("t.add.yml");
    let r = save(&mut c, "");
    let t = text(&r);
    assert_eq!(r.get("isError").and_then(J::as_bool), Some(true), "{t}");
    assert!(
        t.starts_with("not saved: it fell back to raw output on capture #1 of 1."),
        "{t}"
    );
    assert!(t.contains("FELL BACK TO RAW"), "{t}");
    assert!(!file.exists());
    let t = text(&save(&mut c, r#","dry_run":true"#));
    assert!(t.contains("Saving would be refused"), "{t}");
    assert!(!file.exists());
    let t = text(&save(&mut c, r#","force":true"#));
    assert!(t.starts_with("saved "), "{t}");
    assert!(
        t.contains("Saved with force=true although it fell back"),
        "{t}"
    );
    assert!(file.is_file());
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_baseline_that_cannot_be_written_is_reported() {
    let (mut c, dir) = start("save-baseline", T);
    assert_eq!(text(&c.call("add", r#"{"a":2,"b":3}"#)), "5");
    let fixtures = dir.join("fixtures").join("t.add");
    assert!(fixtures.join("01.json").is_file());
    // The temporary file of the baseline write is in the way.
    std::fs::create_dir_all(fixtures.join("01.tmp")).unwrap();
    let yaml = J::str("render: expression\nformat: \"{a} + {b} = {result}\"\n").to_json();
    let r = c.call(
        "formatter_save_standard",
        &format!(r#"{{"tool":"add","yaml":{yaml}}}"#),
    );
    let t = text(&r);
    assert!(t.starts_with("saved "), "{t}");
    assert!(
        t.contains("Baselines recorded in 0 of 1 captures; FAILED to write: #1 ("),
        "{t}"
    );
    assert!(t.contains("2 + 3 = 5"), "{t}");
    let _ = std::fs::remove_dir_all(dir);
}
