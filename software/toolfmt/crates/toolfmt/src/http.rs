//! `toolfmt serve`: the revision-2 HTTP contract (`POST /clean`) on
//! localhost, a drop-in for the Python formatter behind mcp_gateway.py.

use crate::config::Config;
use crate::json::J;
use crate::pipeline::clean;
use crate::standard::Store;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::sync::Arc;
use std::thread;

const MAX_BODY: usize = 64 << 20;

pub fn serve(port: u16, home: &Path, cfg: Option<Config>, auto: bool) -> Result<(), String> {
    let listener = TcpListener::bind(("127.0.0.1", port))
        .map_err(|e| format!("bind 127.0.0.1:{port}: {e}"))?;
    // Port 0 asks the system for a free port; the line names the real one.
    let port = listener.local_addr().map(|a| a.port()).unwrap_or(port);
    eprintln!(
        "[toolfmt] serving POST /clean on http://127.0.0.1:{port} (home {})",
        home.display()
    );
    let store = Arc::new(Store::new(home.to_path_buf()));
    let cfg = Arc::new(cfg);
    for stream in listener.incoming().flatten() {
        let store = store.clone();
        let cfg = cfg.clone();
        thread::spawn(move || {
            let _ = handle(stream, &store, cfg.as_ref().as_ref(), auto);
        });
    }
    Ok(())
}

fn respond(s: &mut TcpStream, code: u16, body: &str) -> std::io::Result<()> {
    let reason = match code {
        200 => "OK",
        400 => "Bad Request",
        403 => "Forbidden",
        404 => "Not Found",
        413 => "Payload Too Large",
        _ => "Error",
    };
    write!(
        s,
        "HTTP/1.1 {code} {reason}\r\nContent-Type: application/json; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )?;
    s.flush()
}

fn handle(
    mut stream: TcpStream,
    store: &Store,
    cfg: Option<&Config>,
    auto: bool,
) -> std::io::Result<()> {
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut line = String::new();
    reader.read_line(&mut line)?;
    let mut parts = line.split_whitespace();
    let method = parts.next().unwrap_or("").to_string();
    let path = parts.next().unwrap_or("").to_string();
    let mut length = 0usize;
    let mut origin = false;
    loop {
        let mut h = String::new();
        if reader.read_line(&mut h)? == 0 || h.trim().is_empty() {
            break;
        }
        if let Some((k, v)) = h.split_once(':') {
            let k = k.trim().to_ascii_lowercase();
            if k == "content-length" {
                length = v.trim().parse().unwrap_or(0);
            } else if k == "origin" {
                origin = true;
            }
        }
    }
    let err = |m: &str| J::obj().with("error", J::str(m)).to_json();
    match (method.as_str(), path.as_str()) {
        ("GET", "/health") => respond(
            &mut stream,
            200,
            &J::obj()
                .with("ok", J::Bool(true))
                .with("version", J::str(crate::VERSION))
                .to_json(),
        ),
        ("POST", "/clean") => {
            // Browsers send Origin on cross-site requests; local clients do not.
            if origin {
                return respond(&mut stream, 403, &err("cross-origin requests are refused"));
            }
            if length > MAX_BODY {
                return respond(&mut stream, 413, &err("body too large"));
            }
            let mut body = vec![0u8; length];
            reader.read_exact(&mut body)?;
            let Ok(text) = String::from_utf8(body) else {
                return respond(&mut stream, 400, &err("body is not UTF-8"));
            };
            let req = match J::parse(&text) {
                Ok(j) => j,
                Err(e) => return respond(&mut stream, 400, &err(&e)),
            };
            let server = req.get("server").and_then(J::as_str).unwrap_or("");
            let tool = req.get("tool").and_then(J::as_str).unwrap_or("");
            let family = cfg.and_then(|c| c.family_of(server, tool));
            let auto = cfg
                .and_then(|c| c.server(server))
                .map_or(auto, |s| s.auto || auto);
            let resp = clean(Some(store), &req, auto, family);
            respond(&mut stream, 200, &resp.to_json())
        }
        _ => respond(
            &mut stream,
            404,
            &err("not found: POST /clean, GET /health"),
        ),
    }
}
