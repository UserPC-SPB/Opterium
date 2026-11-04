//! JSON-RPC over newline-delimited stdio (the MCP stdio transport).

use crate::json::J;
use std::io::{BufRead, Write};
use std::sync::Mutex;

pub fn error_obj(code: i64, msg: impl Into<String>) -> J {
    J::obj()
        .with("code", J::int(code))
        .with("message", J::Str(msg.into()))
}

pub fn error_text(e: &J) -> String {
    e.get("message").map(J::text).unwrap_or_else(|| e.to_json())
}

/// A text-only tool result.
pub fn text_result(text: impl Into<String>, is_error: bool) -> J {
    let block = J::obj()
        .with("type", J::str("text"))
        .with("text", J::Str(text.into()));
    let r = J::obj().with("content", J::Arr(vec![block]));
    if is_error {
        r.with("isError", J::Bool(true))
    } else {
        r
    }
}

/// Calls `f` for every line of `r` until end of stream. A line that is not
/// valid UTF-8 (a server logging in a legacy code page, say) is decoded with
/// replacement characters instead of ending the stream: one bad line must
/// never cost the whole connection.
pub fn for_each_line(mut r: impl BufRead, mut f: impl FnMut(&str)) {
    let mut buf = Vec::new();
    loop {
        buf.clear();
        match r.read_until(b'\n', &mut buf) {
            Ok(0) => break,
            Ok(_) => f(&String::from_utf8_lossy(&buf)),
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(_) => break,
        }
        // One huge message must not keep its buffer alive for the session.
        if buf.capacity() > 1 << 20 {
            buf = Vec::new();
        }
    }
}

/// One writer shared by all threads: each message is one line.
pub struct Out {
    w: Mutex<Box<dyn Write + Send>>,
}

impl Out {
    pub fn stdout() -> Out {
        Out {
            w: Mutex::new(Box::new(std::io::stdout())),
        }
    }

    /// Streams the message through a buffer: no copy of a big result.
    pub fn send(&self, msg: &J) {
        let mut w = self.w.lock().expect("stdout");
        let mut bw = std::io::BufWriter::with_capacity(64 << 10, &mut *w);
        let _ = msg.write_io(&mut bw);
        let _ = bw.write_all(b"\n");
        let _ = bw.flush();
        drop(bw);
        let _ = w.flush();
    }

    pub fn respond(&self, id: &J, result: J) {
        self.send(
            &J::obj()
                .with("jsonrpc", J::str("2.0"))
                .with("id", id.clone())
                .with("result", result),
        );
    }

    pub fn error(&self, id: &J, err: J) {
        self.send(
            &J::obj()
                .with("jsonrpc", J::str("2.0"))
                .with("id", id.clone())
                .with("error", err),
        );
    }

    pub fn notify(&self, method: &str) {
        self.send(
            &J::obj()
                .with("jsonrpc", J::str("2.0"))
                .with("method", J::str(method)),
        );
    }
}
