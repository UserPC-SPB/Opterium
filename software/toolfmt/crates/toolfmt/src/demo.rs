//! `toolfmt demo-server`: a small MCP server with typical output shapes,
//! for trying standards and for the gateway's own tests.

use crate::json::J;
use crate::rpc::{Out, error_obj, text_result};
use std::io::{self, Write};

const TOOLS: &str = r#"[
{"name":"add","description":"Adds two numbers exactly.","inputSchema":{"type":"object","properties":{"a":{"type":"number"},"b":{"type":"number"}},"required":["a","b"]},"annotations":{"readOnlyHint":true}},
{"name":"pow","description":"base^exp as an exact integer.","inputSchema":{"type":"object","properties":{"base":{"type":"integer"},"exp":{"type":"integer"}},"required":["base","exp"]},"annotations":{"readOnlyHint":true}},
{"name":"matrix","description":"Distance matrix |i-j| of size n.","inputSchema":{"type":"object","properties":{"n":{"type":"integer"}}},"annotations":{"readOnlyHint":true}},
{"name":"list_files","description":"Lists files, one page at a time (pretty JSON).","inputSchema":{"type":"object","properties":{"cursor":{"type":"string"}}},"annotations":{"readOnlyHint":true}},
{"name":"log_tail","description":"Last lines of the service log.","inputSchema":{"type":"object","properties":{}},"annotations":{"readOnlyHint":true}},
{"name":"numbers","description":"A list of measurements.","inputSchema":{"type":"object","properties":{}},"annotations":{"readOnlyHint":true}},
{"name":"paths","description":"Changed files.","inputSchema":{"type":"object","properties":{}},"annotations":{"readOnlyHint":true}},
{"name":"search","description":"Searches the docs.","inputSchema":{"type":"object","properties":{"q":{"type":"string"},"_limit":{"type":"integer","description":"a real parameter that happens to start with _"}}},"outputSchema":{"type":"object","properties":{"query":{"type":"string"},"results":{"type":"array"}}},"annotations":{"readOnlyHint":true}},
{"name":"snapshot","description":"A screenshot.","inputSchema":{"type":"object","properties":{}},"annotations":{"readOnlyHint":true}},
{"name":"fail","description":"Always fails.","inputSchema":{"type":"object","properties":{}}},
{"name":"delete_all","description":"Deletes everything (demo: does nothing).","inputSchema":{"type":"object","properties":{}},"annotations":{"destructiveHint":true}}
]"#;

const PNG_1X1: &str = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNkYPhfDwAChwGA60e6kgAAAABJRU5ErkJggg==";

fn big_pow(base: u64, exp: u32) -> String {
    // Little-endian limbs of 10^9.
    let mut limbs: Vec<u64> = vec![1];
    for _ in 0..exp {
        let mut carry = 0u128;
        for l in limbs.iter_mut() {
            let v = u128::from(*l) * u128::from(base) + carry;
            *l = (v % 1_000_000_000) as u64;
            carry = v / 1_000_000_000;
        }
        while carry > 0 {
            limbs.push((carry % 1_000_000_000) as u64);
            carry /= 1_000_000_000;
        }
    }
    let mut s = limbs.last().copied().unwrap_or(0).to_string();
    for l in limbs.iter().rev().skip(1) {
        s.push_str(&format!("{l:09}"));
    }
    s
}

fn call(name: &str, args: &J) -> Result<J, String> {
    Ok(match name {
        "add" => {
            let a = args.get("a").ok_or("a is required")?;
            let b = args.get("b").ok_or("b is required")?;
            let sum = match (a.text().parse::<i128>(), b.text().parse::<i128>()) {
                (Ok(x), Ok(y)) => x.checked_add(y).map(|s| s.to_string()),
                _ => None,
            }
            .unwrap_or_else(|| {
                let x: f64 = a.text().parse().unwrap_or(0.0);
                let y: f64 = b.text().parse().unwrap_or(0.0);
                (x + y).to_string()
            });
            text_result(sum, false)
        }
        "pow" => {
            let base = args
                .get("base")
                .and_then(J::as_u64)
                .unwrap_or(2)
                .min(1_000_000_000);
            let exp = args.get("exp").and_then(J::as_u64).unwrap_or(10).min(2000) as u32;
            text_result(big_pow(base, exp), false)
        }
        "matrix" => {
            let n = args.get("n").and_then(J::as_u64).unwrap_or(4).min(200) as i64;
            let rows: Vec<J> = (0..n)
                .map(|i| J::Arr((0..n).map(|j| J::int((i - j).abs())).collect()))
                .collect();
            text_result(J::Arr(rows).to_json(), false)
        }
        "list_files" => {
            let page2 = args.get("cursor").and_then(J::as_str) == Some("page-2");
            let start = if page2 { 12 } else { 0 };
            let items: Vec<J> = (start..start + 12)
                .map(|i| {
                    J::obj()
                        .with("path", J::Str(format!("src/module_{i:02}.rs")))
                        .with("size", J::int(1000 + i * 1537))
                        .with(
                            "modified",
                            J::Str(format!("2026-11-{:02}T10:{:02}:00Z", 1 + i % 4, i)),
                        )
                        .with("kind", J::str("file"))
                })
                .collect();
            let mut doc = J::obj()
                .with("items", J::Arr(items))
                .with("total", J::int(24));
            doc.set(
                "next_cursor",
                if page2 { J::Null } else { J::str("page-2") },
            );
            text_result(doc.to_pretty(), false)
        }
        "log_tail" => {
            let mut lines = Vec::new();
            for i in 0..30 {
                let t = format!("2026-11-04 10:{:02}:{:02}", i / 6, (i * 7) % 60);
                let l = match i {
                    4 | 17 => format!("{t} ERROR db timeout after 30s"),
                    9 | 12 | 21 => format!("{t} WARN slow query 120ms"),
                    25 => format!("{t} ERROR disk full on /var"),
                    _ => format!("{t} INFO request {i} served in {}ms", 10 + i),
                };
                lines.push(l);
            }
            text_result(lines.join("\n"), false)
        }
        "numbers" => {
            let digits = [3, 1, 4, 1, 5, 9, 2, 6, 5, 3, 5, 8, 9, 7, 9, 3, 2, 3, 8, 4];
            text_result(
                J::Arr(digits.iter().map(|&d| J::int(d)).collect()).to_json(),
                false,
            )
        }
        "paths" => {
            let p = [
                "src/lib.rs",
                "src/main.rs",
                "src/render/mod.rs",
                "src/render/table.rs",
                "tests/gateway.rs",
                "README.md",
                "Cargo.toml",
            ];
            text_result(
                J::Arr(p.iter().map(|s| J::str(*s)).collect()).to_json(),
                false,
            )
        }
        "search" => {
            let q = args.get("q").and_then(J::as_str).unwrap_or("").to_string();
            let results: Vec<J> = (1..=3)
                .map(|i| {
                    J::obj()
                        .with("title", J::Str(format!("{q} guide part {i}")))
                        .with("url", J::Str(format!("https://docs.example/{q}/{i}")))
                        .with("score", J::Num(format!("0.{}", 10 - i)))
                })
                .collect();
            let sc = J::obj()
                .with("query", J::Str(q))
                .with("results", J::Arr(results));
            text_result(sc.to_json(), false).with("structuredContent", sc)
        }
        "snapshot" => J::obj().with(
            "content",
            J::Arr(vec![
                J::obj()
                    .with("type", J::str("image"))
                    .with("mimeType", J::str("image/png"))
                    .with("data", J::str(PNG_1X1)),
            ]),
        ),
        "fail" => text_result("boom: invalid input", true),
        "delete_all" => text_result("deleted 0 items (demo)", false),
        other => return Err(format!("unknown tool '{other}'")),
    })
}

/// Extra tools for failure tests, only with `--test-modes`.
const TEST_TOOLS: &str = r#"[
{"name":"crash","description":"Exits the server without answering.","inputSchema":{"type":"object","properties":{}}},
{"name":"sleep","description":"Answers after ms milliseconds.","inputSchema":{"type":"object","properties":{"ms":{"type":"integer"}},"required":["ms"]}},
{"name":"noise","description":"Writes junk to stdout (a banner, invalid UTF-8, a stray response) before answering.","inputSchema":{"type":"object","properties":{}}},
{"name":"ask_client","description":"Sends roots/list and sampling/createMessage to its client and reports the answers.","inputSchema":{"type":"object","properties":{}}},
{"name":"grow","description":"Adds a tool and sends notifications/tools/list_changed.","inputSchema":{"type":"object","properties":{}}},
{"name":"big","description":"A JSON list of n objects.","inputSchema":{"type":"object","properties":{"n":{"type":"integer"}},"required":["n"]},"annotations":{"readOnlyHint":true}}
]"#;

fn big(n: u64) -> J {
    J::Arr(
        (0..n.min(2_000_000))
            .map(|i| {
                J::obj()
                    .with("id", J::int(i))
                    .with("path", J::Str(format!("src/dir{}/file_{i}.rs", i % 50)))
                    .with("size", J::int(i * 37))
                    .with(
                        "owner",
                        J::obj().with("login", J::Str(format!("user{}", i % 7))),
                    )
            })
            .collect(),
    )
}

/// Runs the demo server. `test_modes` adds the failure tools; `page_size`
/// splits `tools/list` into pages of that many tools.
pub fn serve(test_modes: bool, page_size: usize) -> i32 {
    let out = Out::stdout();
    let mut tools = J::parse(TOOLS)
        .expect("demo tools")
        .as_arr()
        .cloned()
        .unwrap_or_default();
    if test_modes {
        tools.extend(
            J::parse(TEST_TOOLS)
                .expect("test tools")
                .as_arr()
                .cloned()
                .unwrap_or_default(),
        );
    }
    let (tx, rx) = std::sync::mpsc::channel::<J>();
    std::thread::spawn(move || {
        crate::rpc::for_each_line(io::stdin().lock(), |line| {
            if let Ok(m) = J::parse(line.trim()) {
                let _ = tx.send(m);
            }
        });
    });
    let mut backlog: std::collections::VecDeque<J> = std::collections::VecDeque::new();
    loop {
        let mut msg = match backlog.pop_front() {
            Some(m) => m,
            None => match rx.recv() {
                Ok(m) => m,
                Err(_) => break,
            },
        };
        let Some(id) = msg.remove("id") else { continue };
        let method = msg
            .get("method")
            .and_then(J::as_str)
            .unwrap_or("")
            .to_string();
        let params = msg.remove("params").unwrap_or_else(J::obj);
        match method.as_str() {
            "initialize" => out.respond(
                &id,
                J::obj()
                    .with(
                        "protocolVersion",
                        params
                            .get("protocolVersion")
                            .cloned()
                            .unwrap_or(J::str("2025-06-18")),
                    )
                    .with("capabilities", J::obj().with("tools", J::obj()))
                    .with(
                        "serverInfo",
                        J::obj()
                            .with("name", J::str("toolfmt-demo"))
                            .with("version", J::str(crate::VERSION)),
                    ),
            ),
            "ping" => out.respond(&id, J::obj()),
            "tools/list" => {
                let start: usize = params
                    .get("cursor")
                    .and_then(J::as_str)
                    .and_then(|c| c.strip_prefix('p'))
                    .and_then(|c| c.parse().ok())
                    .unwrap_or(0);
                let end = if page_size == 0 {
                    tools.len()
                } else {
                    (start + page_size).min(tools.len())
                };
                let mut r = J::obj().with("tools", J::Arr(tools[start.min(end)..end].to_vec()));
                if end < tools.len() {
                    r.set("nextCursor", J::Str(format!("p{end}")));
                }
                out.respond(&id, r);
            }
            "tools/call" => {
                let name = params.get("name").and_then(J::as_str).unwrap_or("");
                let empty = J::obj();
                let args = params.get("arguments").unwrap_or(&empty);
                if args.get("_render").is_some() {
                    out.respond(&id, text_result("override leaked to upstream", true));
                    continue;
                }
                if test_modes {
                    match name {
                        "crash" => std::process::exit(3),
                        "sleep" => {
                            let ms = args.get("ms").and_then(J::as_u64).unwrap_or(0);
                            std::thread::sleep(std::time::Duration::from_millis(ms));
                            out.respond(&id, text_result(format!("slept {ms} ms"), false));
                            continue;
                        }
                        "noise" => {
                            let mut so = io::stdout().lock();
                            let _ = so.write_all(b"banner: demo server ready\n");
                            let _ = so.write_all(b"\xff\xfe broken \xc3 utf-8\n");
                            let _ = so
                                .write_all(b"{\"jsonrpc\":\"2.0\",\"id\":999999,\"result\":{}}\n");
                            let _ = so.write_all(b"{\"jsonrpc\":\"2.0\",\"method\":\"notifications/message\",\"params\":{\"level\":\"info\",\"data\":\"x\"}}\n");
                            let _ = so.flush();
                            drop(so);
                            out.respond(&id, text_result("quiet now", false));
                            continue;
                        }
                        "ask_client" => {
                            for (rid, method) in
                                [("s1", "roots/list"), ("s2", "sampling/createMessage")]
                            {
                                out.send(
                                    &J::obj()
                                        .with("jsonrpc", J::str("2.0"))
                                        .with("id", J::str(rid))
                                        .with("method", J::str(method))
                                        .with("params", J::obj()),
                                );
                            }
                            let mut answers = std::collections::BTreeMap::new();
                            while answers.len() < 2 {
                                let Ok(m) = rx.recv() else { break };
                                match m.get("id").and_then(J::as_str) {
                                    Some(k @ ("s1" | "s2")) if m.get("method").is_none() => {
                                        let v = m
                                            .get("result")
                                            .or_else(|| m.get("error"))
                                            .cloned()
                                            .unwrap_or(J::Null);
                                        answers.insert(k.to_string(), v);
                                    }
                                    _ => backlog.push_back(m),
                                }
                            }
                            let text = format!(
                                "roots: {}; sampling: {}",
                                answers.get("s1").map(J::to_json).unwrap_or_default(),
                                answers.get("s2").map(J::to_json).unwrap_or_default()
                            );
                            out.respond(&id, text_result(text, false));
                            continue;
                        }
                        "grow" => {
                            if !tools
                                .iter()
                                .any(|t| t.get("name").and_then(J::as_str) == Some("added_tool"))
                            {
                                tools.push(J::parse(r#"{"name":"added_tool","description":"Appeared at runtime.","inputSchema":{"type":"object","properties":{}}}"#).expect("tool"));
                            }
                            out.respond(&id, text_result("grown", false));
                            out.notify("notifications/tools/list_changed");
                            continue;
                        }
                        "big" => {
                            let n = args.get("n").and_then(J::as_u64).unwrap_or(1000);
                            out.respond(&id, text_result(big(n).to_json(), false));
                            continue;
                        }
                        "added_tool" => {
                            out.respond(&id, text_result("added tool works", false));
                            continue;
                        }
                        _ => {}
                    }
                }
                match call(name, args) {
                    Ok(r) => out.respond(
                        &id,
                        r.with("_meta", J::obj().with("echo_args", args.clone())),
                    ),
                    Err(e) => out.error(&id, error_obj(-32602, e)),
                }
            }
            m => out.error(&id, error_obj(-32601, format!("method '{m}' not found"))),
        }
    }
    0
}
