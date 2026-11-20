//! `formatter_tools: false`: a working agent is not offered the formatter.

mod common;
use common::{Client, home, text};

const CFG: &str = r#"{"expose":"catalog","formatter_tools":false,"mcpServers":{
  "d":{"command":{exe},"args":["demo-server"]}}}"#;

#[test]
fn formatter_tools_can_be_left_out() {
    let dir = home("no-formatter", CFG);
    let mut c = Client::start(&dir);
    let init = c.request("initialize", r#"{"protocolVersion":"2025-06-18"}"#);
    let instructions = init
        .get("result")
        .and_then(|r| r.get("instructions"))
        .map(|i| i.text())
        .unwrap_or_default();
    assert!(instructions.contains("call_tool"), "{instructions}");
    assert!(!instructions.contains("formatter"), "{instructions}");
    let names = c.tool_names();
    assert_eq!(names, ["catalog", "call_tool"], "{names:?}");
    let err = c.call_raw("formatter_inspect", "{}").to_json();
    assert!(err.contains("unknown tool 'formatter_inspect'"), "{err}");
    let via = text(&c.call("call_tool", r#"{"tool":"formatter_save_standard","args":{"tool":"add"}}"#));
    assert!(via.starts_with("unknown tool 'formatter_save_standard'"), "{via}");
    // The upstream tools still work through the catalog.
    let sum = text(&c.call("call_tool", r#"{"tool":"add","args":{"a":2,"b":3}}"#));
    assert_eq!(sum, "5");
    let _ = std::fs::remove_dir_all(dir);
}
