//! A test client for the gateway: every wait has a deadline, so a hung
//! gateway fails the test instead of hanging it.
#![allow(dead_code)]

use std::collections::VecDeque;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{Receiver, RecvTimeoutError, channel};
use std::time::{Duration, Instant};
use toolfmt::json::J;

pub const EXE: &str = env!("CARGO_BIN_EXE_toolfmt");
const WAIT: Duration = Duration::from_secs(60);

pub struct Client {
    child: Child,
    stdin: ChildStdin,
    rx: Receiver<J>,
    next: u64,
    /// Messages read while waiting for something else.
    stash: VecDeque<J>,
}

impl Client {
    pub fn start(home: &Path) -> Client {
        let mut child = Command::new(EXE)
            .args(["gateway", "--home"])
            .arg(home)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("start gateway");
        let stdin = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();
        let (tx, rx) = channel();
        std::thread::spawn(move || {
            toolfmt::rpc::for_each_line(std::io::BufReader::new(stdout), |line| {
                if let Ok(m) = J::parse(line.trim()) {
                    let _ = tx.send(m);
                }
            });
        });
        Client {
            child,
            stdin,
            rx,
            next: 1,
            stash: VecDeque::new(),
        }
    }

    pub fn send_raw(&mut self, bytes: &[u8]) {
        self.stdin.write_all(bytes).unwrap();
        self.stdin.flush().unwrap();
    }

    /// Sends a request and returns its id without waiting.
    pub fn send(&mut self, method: &str, params: &str) -> u64 {
        let id = self.next;
        self.next += 1;
        let line =
            format!(r#"{{"jsonrpc":"2.0","id":{id},"method":"{method}","params":{params}}}"#);
        self.send_raw(format!("{line}\n").as_bytes());
        id
    }

    fn next_msg(&mut self, deadline: Instant) -> Option<J> {
        let left = deadline.saturating_duration_since(Instant::now());
        match self.rx.recv_timeout(left) {
            Ok(m) => Some(m),
            Err(RecvTimeoutError::Timeout) => None,
            Err(RecvTimeoutError::Disconnected) => panic!("gateway closed stdout"),
        }
    }

    /// Waits for the response to `id`.
    pub fn wait(&mut self, id: u64) -> J {
        if let Some(i) = self
            .stash
            .iter()
            .position(|m| m.get("id").and_then(J::as_u64) == Some(id))
        {
            return self.stash.remove(i).unwrap();
        }
        let deadline = Instant::now() + WAIT;
        loop {
            let m = self
                .next_msg(deadline)
                .unwrap_or_else(|| panic!("no response to request {id} within {WAIT:?}"));
            if m.get("id").and_then(J::as_u64) == Some(id) && m.get("method").is_none() {
                return m;
            }
            self.stash.push_back(m);
        }
    }

    /// Waits up to `secs` for a notification `method`.
    pub fn saw(&mut self, method: &str, secs: u64) -> bool {
        self.note(method, secs).is_some()
    }

    /// Waits up to `secs` for a notification `method` and returns it.
    pub fn note(&mut self, method: &str, secs: u64) -> Option<J> {
        let is =
            |m: &J| m.get("method").and_then(J::as_str) == Some(method) && m.get("id").is_none();
        self.find(is, secs)
    }

    /// Whether the response to `id` arrives within `secs`.
    pub fn got(&mut self, id: u64, secs: u64) -> bool {
        let is = |m: &J| m.get("id").and_then(J::as_u64) == Some(id) && m.get("method").is_none();
        self.find(is, secs).is_some()
    }

    fn find(&mut self, is: impl Fn(&J) -> bool, secs: u64) -> Option<J> {
        if let Some(m) = self.stash.iter().find(|m| is(m)) {
            return Some(m.clone());
        }
        let deadline = Instant::now() + Duration::from_secs(secs);
        while let Some(m) = self.next_msg(deadline) {
            let hit = is(&m);
            self.stash.push_back(m.clone());
            if hit {
                return Some(m);
            }
        }
        None
    }

    /// Sends a notification (no id, no answer).
    pub fn notify(&mut self, method: &str, params: &str) {
        let line = format!(r#"{{"jsonrpc":"2.0","method":"{method}","params":{params}}}"#);
        self.send_raw(format!("{line}\n").as_bytes());
    }

    pub fn request(&mut self, method: &str, params: &str) -> J {
        let id = self.send(method, params);
        self.wait(id)
    }

    /// `tools/call`; the whole response (result or error).
    pub fn call_raw(&mut self, tool: &str, args: &str) -> J {
        self.request(
            "tools/call",
            &format!(r#"{{"name":"{tool}","arguments":{args}}}"#),
        )
    }

    pub fn call(&mut self, tool: &str, args: &str) -> J {
        let r = self.call_raw(tool, args);
        r.get("result")
            .cloned()
            .unwrap_or_else(|| panic!("no result: {}", r.to_json()))
    }

    pub fn tool_names(&mut self) -> Vec<String> {
        let list = self.request("tools/list", "{}");
        list.get("result")
            .and_then(|r| r.get("tools"))
            .and_then(J::as_arr)
            .map(|a| {
                a.iter()
                    .map(|t| t.get("name").map(J::text).unwrap_or_default())
                    .collect()
            })
            .unwrap_or_default()
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        let _ = self.child.kill();
    }
}

pub fn text(result: &J) -> String {
    result
        .get("content")
        .and_then(J::as_arr)
        .and_then(|c| c.first())
        .and_then(|b| b.get("text"))
        .map(J::text)
        .unwrap_or_default()
}

/// A fresh home directory with `upstreams.json`; `{exe}` in the config is
/// replaced by the toolfmt binary as a JSON string.
pub fn home(name: &str, config: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("toolfmt-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let cfg = config.replace("{exe}", &J::str(EXE).to_json());
    std::fs::write(dir.join("upstreams.json"), cfg).unwrap();
    dir
}
