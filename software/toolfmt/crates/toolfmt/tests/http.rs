//! `toolfmt serve`: the revision-2 HTTP contract on a free local port.

mod common;
use common::EXE;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::process::{Child, Command, Stdio};
use toolfmt::json::J;

struct Server {
    child: Child,
    port: u16,
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
    }
}

fn start(name: &str) -> Server {
    let home = std::env::temp_dir().join(format!("toolfmt-http-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(home.join("standards")).unwrap();
    std::fs::write(
        home.join("standards").join("s.list.yml"),
        "render: objects_table\ncolumns: [id]\n",
    )
    .unwrap();
    let mut child = Command::new(EXE)
        .args(["serve", "--port", "0", "--home"])
        .arg(&home)
        .stderr(Stdio::piped())
        .stdout(Stdio::null())
        .spawn()
        .unwrap();
    let mut line = String::new();
    BufReader::new(child.stderr.take().unwrap())
        .read_line(&mut line)
        .unwrap();
    let port = line
        .split("127.0.0.1:")
        .nth(1)
        .and_then(|r| r.split(' ').next())
        .and_then(|p| p.parse().ok())
        .unwrap_or_else(|| panic!("no port in {line:?}"));
    Server { child, port }
}

/// Sends a raw HTTP request; returns (status, body).
fn http(port: u16, req: &str) -> (u16, String) {
    let mut s = TcpStream::connect(("127.0.0.1", port)).unwrap();
    s.write_all(req.as_bytes()).unwrap();
    let mut resp = String::new();
    s.read_to_string(&mut resp).unwrap();
    let status = resp
        .split(' ')
        .nth(1)
        .and_then(|c| c.parse().ok())
        .unwrap_or(0);
    let body = resp
        .split_once("\r\n\r\n")
        .map(|(_, b)| b.to_string())
        .unwrap_or_default();
    (status, body)
}

fn post(port: u16, body: &str, extra: &str) -> (u16, String) {
    http(
        port,
        &format!(
            "POST /clean HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\n{extra}Content-Length: {}\r\n\r\n{body}",
            body.len()
        ),
    )
}

#[test]
fn clean_contract_over_http() {
    let srv = start("clean");
    let (st, body) = http(srv.port, "GET /health HTTP/1.1\r\nHost: localhost\r\n\r\n");
    assert_eq!(st, 200);
    assert!(body.contains(r#""ok":true"#), "{body}");

    // A standard from the home directory.
    let req = J::obj()
        .with("server", J::str("s"))
        .with("tool", J::str("list"))
        .with("data", J::str(r#"[{"id":1,"x":2},{"id":2,"x":3}]"#))
        .to_json();
    let (st, body) = post(srv.port, &req, "");
    assert_eq!(st, 200, "{body}");
    let r = J::parse(&body).unwrap();
    assert_eq!(r.get("used").unwrap().text(), "objects_table:list");
    assert_eq!(r.get("text").unwrap().text(), "[2]{id}:\n  1\n  2");
    assert_eq!(r.get("loss").unwrap().text(), "lossy");

    // No standard: the data comes back as it was.
    let req = r#"{"server":"s","tool":"other","data":"plain text"}"#;
    let r = J::parse(&post(srv.port, req, "").1).unwrap();
    assert_eq!(r.get("used").unwrap().text(), "passthrough");
    assert_eq!(r.get("text").unwrap().text(), "plain text");

    // A body of several megabytes, UTF-8 included.
    let big: Vec<J> = (0..50_000)
        .map(|i| {
            J::obj()
                .with("id", J::int(i))
                .with("name", J::Str(format!("имя {i}")))
        })
        .collect();
    let req = J::obj()
        .with("server", J::str("s"))
        .with("tool", J::str("list"))
        .with("data", J::Str(J::Arr(big).to_json()))
        .with("_pre", J::parse(r#"[{"limit_items":2}]"#).unwrap())
        .to_json();
    assert!(req.len() > 1_000_000);
    let (st, body) = post(srv.port, &req, "");
    assert_eq!(st, 200);
    assert!(body.contains("49998 more items"), "{body}");
}

#[test]
fn bad_requests_are_refused() {
    let srv = start("bad");
    let ok = r#"{"server":"s","tool":"t","data":"x"}"#;
    // Browsers send Origin: refused (a web page must not drive the formatter).
    assert_eq!(
        post(srv.port, ok, "Origin: https://evil.example\r\n").0,
        403
    );
    assert_eq!(post(srv.port, "{not json", "").0, 400);
    assert_eq!(
        http(srv.port, "GET /nope HTTP/1.1\r\nHost: localhost\r\n\r\n").0,
        404
    );
    // Still serving after all of that, also in parallel.
    let threads: Vec<_> = (0..16)
        .map(|_| {
            let p = srv.port;
            std::thread::spawn(move || post(p, ok, "").0)
        })
        .collect();
    for t in threads {
        assert_eq!(t.join().unwrap(), 200);
    }
}
