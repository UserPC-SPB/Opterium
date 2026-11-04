//! Upstream failure modes through the gateway: none may cost the agent its
//! connection or its other servers.

mod common;
use common::{Client, home, text};
use std::time::Instant;
use toolfmt::json::J;

const CFG: &str = r#"{"expose":"all","mcpServers":{
  "t":{"command":{exe},"args":["demo-server","--test-modes"],"timeout":2,"restart_backoff":0,"init_timeout":20},
  "paged":{"command":{exe},"args":["demo-server","--page-size","3"],"capture":false}}}"#;

fn start(name: &str) -> (Client, std::path::PathBuf) {
    let dir = home(name, CFG);
    let mut c = Client::start(&dir);
    c.request("initialize", r#"{"protocolVersion":"2025-06-18"}"#);
    (c, dir)
}

#[test]
fn paged_tools_list_is_read_to_the_end() {
    let (mut c, dir) = start("paged");
    let names = c.tool_names();
    // 11 demo tools + 6 test tools on t; 11 on paged (pages of 3), namespaced.
    assert!(names.iter().any(|n| n == "paged__delete_all"), "{names:?}");
    assert_eq!(
        names.iter().filter(|n| n.starts_with("paged__")).count(),
        11,
        "{names:?}"
    );
    assert!(names.iter().any(|n| n == "big"));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn junk_on_upstream_stdout_is_skipped() {
    let (mut c, dir) = start("noise");
    assert_eq!(text(&c.call("noise", "{}")), "quiet now");
    // The same connection still works after a banner, broken UTF-8 and a
    // response to a request nobody sent.
    assert_eq!(text(&c.call("add", r#"{"a":2,"b":3}"#)), "5");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn broken_client_input_is_survived() {
    let (mut c, dir) = start("client-utf8");
    c.send_raw(b"\xff\xfe not json at all\n");
    c.send_raw(
        b"{\"jsonrpc\":\"2.0\",\"id\":77,\"method\":\"ping\",\"params\":{\"x\":\"\xc3\"}}\n",
    );
    let pong = c.wait(77);
    assert!(pong.get("result").is_some(), "{}", pong.to_json());
    assert_eq!(text(&c.call("add", r#"{"a":1,"b":1}"#)), "2");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn requests_from_the_server_are_answered() {
    let (mut c, dir) = start("ask");
    let t = text(&c.call("ask_client", "{}"));
    assert!(
        t.starts_with(r#"roots: {"roots":[]}; sampling: {"code":-32601"#),
        "{t}"
    );
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_timeout_is_an_error_not_a_hang() {
    let (mut c, dir) = start("timeout");
    let t0 = Instant::now();
    let r = c.call_raw("sleep", r#"{"ms":4000}"#);
    let e = r.get("error").unwrap_or_else(|| panic!("{}", r.to_json()));
    assert_eq!(e.get("code").map(J::text).as_deref(), Some("-32001"));
    assert!(t0.elapsed().as_secs_f64() < 3.5, "{:?}", t0.elapsed());
    // The late answer is dropped; the server keeps serving.
    assert_eq!(text(&c.call("add", r#"{"a":4,"b":5}"#)), "9");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_crashed_server_is_restarted() {
    let (mut c, dir) = start("crash");
    let r = c.call_raw("crash", "{}");
    let e = r.get("error").unwrap_or_else(|| panic!("{}", r.to_json()));
    assert!(
        e.get("message").unwrap().text().contains("exited"),
        "{}",
        r.to_json()
    );
    // Next call starts the server again.
    assert_eq!(text(&c.call("add", r#"{"a":20,"b":22}"#)), "42");
    // The other server never noticed.
    assert_eq!(text(&c.call("paged__add", r#"{"a":1,"b":2}"#)), "3");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn tool_list_changes_are_followed() {
    let (mut c, dir) = start("grow");
    assert!(!c.tool_names().iter().any(|n| n == "added_tool"));
    assert_eq!(text(&c.call("grow", "{}")), "grown");
    assert!(
        c.saw("notifications/tools/list_changed", 20),
        "no list_changed downstream"
    );
    assert!(c.tool_names().iter().any(|n| n == "added_tool"));
    assert_eq!(text(&c.call("added_tool", "{}")), "added tool works");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn many_parallel_calls_each_get_their_own_answer() {
    let (mut c, dir) = start("parallel");
    c.tool_names();
    let ids: Vec<(u64, u64)> = (0..120u64)
        .map(|i| {
            let tool = if i % 2 == 0 { "add" } else { "paged__add" };
            (
                c.send(
                    "tools/call",
                    &format!(r#"{{"name":"{tool}","arguments":{{"a":{i},"b":1000}}}}"#),
                ),
                i,
            )
        })
        .collect();
    for (id, i) in ids.into_iter().rev() {
        let r = c.wait(id);
        let t = text(r.get("result").unwrap());
        assert_eq!(t, (i + 1000).to_string(), "request {i}");
    }
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_big_output_is_formatted_through_the_gateway() {
    let (mut c, dir) = start("big");
    c.tool_names();
    std::fs::create_dir_all(dir.join("standards")).unwrap();
    std::fs::write(
        dir.join("standards").join("t.big.yml"),
        "render: objects_table\ncolumns: [id, size, owner.login]\nformats: {size: bytes}\npre: [{sort_desc: size}, {limit_items: 5}]\n",
    )
    .unwrap();
    let t0 = Instant::now();
    let r = c.call("big", r#"{"n":100000}"#);
    let t = text(&r);
    // 99999 * 37 bytes = 3.53 MB; 99999 % 7 = 4.
    assert!(
        t.starts_with("[objects_table:lossy]\n[5]{id,size,owner.login}:\n  99999,3.5 MB,user4"),
        "{t}"
    );
    assert!(t.ends_with("… 99995 more items (limit_items 5)"), "{t}");
    eprintln!("100k objects through the gateway: {:?}", t0.elapsed());
    // Bigger than the capture limit (2 MB): not stored as a fixture.
    assert!(!dir.join("fixtures").join("t.big").exists());
    let _ = std::fs::remove_dir_all(dir);
}
