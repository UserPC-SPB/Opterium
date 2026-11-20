//! The MCP gateway: one stdio server in front of every upstream server.
//!
//! Tools keep their original names; a name already taken is exposed as
//! `<server>__<tool>` (then `_2`, `_3`, ...). Once the client has seen or
//! used the names they never move to another tool for the life of the
//! process (D6: identity is the pair, nothing becomes unreachable).
//! Overrides are taken out of the arguments before the upstream call.
//! Errors, and tools without a standard, go back exactly as the upstream
//! sent them (D1); a failed standard gives the raw output plus a note (D2).
//!
//! It is a gateway for tools: client cancellations and progress pass
//! through, but resources and prompts are not aggregated and the upstream's
//! own requests to the client (sampling, elicitation) are refused.

use crate::capture::{Audit, Captures, Payload, Source, payload_src, write_atomic, write_baseline};
use crate::catalog;
use crate::config::{Config, Expose, ServerCfg};
use crate::detect::detect;
use crate::json::J;
use crate::meta::{self, clean_text, preview, run_fixture};
use crate::pipeline::{self, Body, Data, Outcome, Overrides, Request};
use crate::rpc::{Out, error_obj, error_text, for_each_line, text_result};
use crate::standard::{Render, Store, compile};
use std::collections::HashMap;
use std::fmt::Write as _;
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering::SeqCst};
use std::sync::{Arc, Condvar, Mutex, RwLock, mpsc};
use std::thread;
use std::time::{Duration, Instant};

/// The newest MCP version toolfmt speaks: offered to every upstream.
pub const PROTOCOL: &str = "2025-06-18";
/// Every version toolfmt speaks, newest first (the set of any_txt_parser's
/// own MCP server). The gateway forwards tool definitions and results
/// without translating them between versions.
pub const PROTOCOLS: [&str; 3] = ["2025-06-18", "2025-03-26", "2024-11-05"];

/// The version to answer a client that asked for `requested`: the same one
/// when toolfmt speaks it, else the newest (the MCP negotiation rule).
pub fn negotiate(requested: Option<&str>) -> &'static str {
    requested
        .and_then(|r| PROTOCOLS.iter().find(|v| **v == r))
        .copied()
        .unwrap_or(PROTOCOL)
}

/// The version an upstream answered to `initialize`, when toolfmt speaks it.
fn check_upstream_protocol(init: &J) -> Result<&'static str, String> {
    let spoken = PROTOCOLS.join(", ");
    match init.get("protocolVersion").and_then(J::as_str) {
        Some(v) => PROTOCOLS.iter().find(|p| **p == v).copied().ok_or_else(|| {
            format!("incompatible protocol version '{v}' (toolfmt speaks {spoken})")
        }),
        None => Err(format!(
            "its initialize answer has no protocolVersion (toolfmt speaks {spoken})"
        )),
    }
}

type Reply = Result<J, J>;

struct Conn {
    server: String,
    stdin: Mutex<ChildStdin>,
    child: Mutex<Child>,
    pending: Mutex<HashMap<u64, mpsc::Sender<Reply>>>,
    next: AtomicU64,
    alive: AtomicBool,
}

impl Conn {
    fn write(&self, msg: &J) -> io::Result<()> {
        let mut s = msg.to_json();
        s.push('\n');
        let mut w = self.stdin.lock().expect("stdin");
        w.write_all(s.as_bytes())?;
        w.flush()
    }

    fn request(&self, method: &str, params: J, timeout: Duration) -> Reply {
        let (id, rx) = self.send(method, params)?;
        self.wait(id, &rx, timeout)
    }

    /// Sends a request; its answer arrives on the receiver.
    fn send(&self, method: &str, params: J) -> Result<(u64, mpsc::Receiver<Reply>), J> {
        if !self.alive.load(SeqCst) {
            return Err(error_obj(
                -32603,
                format!("server '{}' is not running", self.server),
            ));
        }
        let id = self.next.fetch_add(1, SeqCst);
        let (tx, rx) = mpsc::channel();
        self.pending.lock().expect("pending").insert(id, tx);
        let msg = J::obj()
            .with("jsonrpc", J::str("2.0"))
            .with("id", J::int(id))
            .with("method", J::str(method))
            .with("params", params);
        if let Err(e) = self.write(&msg) {
            self.pending.lock().expect("pending").remove(&id);
            return Err(error_obj(
                -32603,
                format!("server '{}': write failed: {e}", self.server),
            ));
        }
        Ok((id, rx))
    }

    /// Cancels request `id` if it is still running: the upstream is told
    /// (`notifications/cancelled`), its waiter is released and a late
    /// answer is dropped.
    fn cancel(&self, id: u64, reason: Option<&str>) {
        let Some(tx) = self.pending.lock().expect("pending").remove(&id) else {
            return;
        };
        let mut params = J::obj().with("requestId", J::int(id));
        if let Some(r) = reason {
            params.set("reason", J::str(r));
        }
        let _ = self.write(
            &J::obj()
                .with("jsonrpc", J::str("2.0"))
                .with("method", J::str("notifications/cancelled"))
                .with("params", params),
        );
        let _ = tx.send(Err(error_obj(-32800, "request cancelled")));
    }

    fn wait(&self, id: u64, rx: &mpsc::Receiver<Reply>, timeout: Duration) -> Reply {
        match rx.recv_timeout(timeout) {
            Ok(r) => r,
            Err(mpsc::RecvTimeoutError::Timeout) => {
                self.cancel(id, Some("timeout"));
                Err(error_obj(
                    -32001,
                    format!(
                        "server '{}' timed out after {}s",
                        self.server,
                        timeout.as_secs()
                    ),
                ))
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => Err(error_obj(
                -32603,
                format!("server '{}' exited", self.server),
            )),
        }
    }

    fn notify(&self, method: &str) {
        let _ = self.write(
            &J::obj()
                .with("jsonrpc", J::str("2.0"))
                .with("method", J::str(method)),
        );
    }

    fn kill(&self) {
        self.alive.store(false, SeqCst);
        let _ = self.child.lock().expect("child").kill();
    }
}

/// Reads the upstream's stdout; `on_note` gets its notifications.
fn read_loop(conn: Arc<Conn>, out: impl BufRead, on_note: impl Fn(&str, J)) {
    for_each_line(out, |line| {
        let t = line.trim();
        if t.is_empty() {
            return;
        }
        let Ok(mut msg) = J::parse(t) else {
            eprintln!("[toolfmt] {}: ignored a non-JSON stdout line", conn.server);
            return;
        };
        let method = msg.get("method").and_then(J::as_str).map(str::to_string);
        let id = msg.remove("id");
        match (method, id) {
            (None, Some(id)) => {
                let Some(n) = id.as_u64() else { return };
                let tx = conn.pending.lock().expect("pending").remove(&n);
                if let Some(tx) = tx {
                    let r = match msg.remove("error") {
                        Some(e) => Err(e),
                        None => Ok(msg.remove("result").unwrap_or(J::Null)),
                    };
                    let _ = tx.send(r);
                }
            }
            (Some(m), Some(id)) => {
                // Requests from the server to its client: answer, never hang
                // it. toolfmt is a gateway for tools: sampling, elicitation
                // and the like are not forwarded to the client.
                let reply = J::obj().with("jsonrpc", J::str("2.0")).with("id", id);
                let reply = match m.as_str() {
                    "ping" => reply.with("result", J::obj()),
                    "roots/list" => reply.with("result", J::obj().with("roots", J::Arr(vec![]))),
                    _ => reply.with(
                        "error",
                        error_obj(-32601, format!("toolfmt does not support '{m}'")),
                    ),
                };
                let _ = conn.write(&reply);
            }
            (Some(m), None) => on_note(&m, msg),
            _ => {}
        }
    });
    // The stream ended: make sure the process is gone too (no orphans).
    conn.kill();
    let _ = conn.child.lock().expect("child").wait();
    let pending: Vec<_> = conn.pending.lock().expect("pending").drain().collect();
    for (_, tx) in pending {
        let _ = tx.send(Err(error_obj(
            -32603,
            format!("server '{}' exited", conn.server),
        )));
    }
}

/// Resolves a command like `npx` to `npx.cmd` on Windows (PATHEXT).
fn resolve_program(cmd: &str) -> PathBuf {
    #[cfg(windows)]
    {
        let p = Path::new(cmd);
        if p.extension().is_none() {
            let exts: Vec<String> = std::env::var("PATHEXT")
                .unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".into())
                .split(';')
                .filter(|e| !e.is_empty())
                .map(|e| e.to_ascii_lowercase())
                .collect();
            let dirs: Vec<PathBuf> = if p.components().count() > 1 {
                vec![PathBuf::new()]
            } else {
                std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default()).collect()
            };
            for d in dirs {
                for e in &exts {
                    let c = d.join(format!("{cmd}{e}"));
                    if c.is_file() {
                        return c;
                    }
                }
            }
        }
    }
    PathBuf::from(cmd)
}

enum Phase {
    Starting,
    Ready,
    Failed(String),
}

struct UpState {
    phase: Phase,
    conn: Option<Arc<Conn>>,
    tools: Vec<J>,
    last_start: Option<Instant>,
    /// First sentence of the server's `instructions`: its catalog description.
    about: Option<String>,
    /// The MCP version the server answered.
    protocol: Option<&'static str>,
}

struct Upstream {
    cfg: ServerCfg,
    st: Mutex<UpState>,
    cv: Condvar,
}

/// Whether the caller may restart a dead server, decided and claimed in one
/// step under the state lock: of many callers that find it dead at once,
/// exactly one gets `true` (the phase is `Starting` from then on), the
/// others wait for it.
fn claim_restart(st: &mut UpState, backoff: Duration) -> bool {
    let may = !matches!(st.phase, Phase::Starting)
        && st.last_start.is_none_or(|t| t.elapsed() >= backoff);
    if may {
        st.phase = Phase::Starting;
        st.last_start = Some(Instant::now());
    }
    may
}

/// What `connect` gives: the connection, the tools, the description, the
/// protocol version.
type Connected = (Arc<Conn>, Vec<J>, Option<String>, &'static str);

/// Tools named by the gateway itself.
const RESERVED: [&str; 4] = [meta::INSPECT, meta::SAVE, catalog::CATALOG, catalog::CALL];

/// The first free name for `tool` of `server`: the tool's own name, else
/// `<server>__<tool>`, else that with `_2`, `_3`, ... Never none (D6).
fn free_name(taken: impl Fn(&str) -> bool, server: &str, tool: &str) -> String {
    let free = |c: &str| !taken(c) && !RESERVED.contains(&c);
    if free(tool) {
        return tool.to_string();
    }
    let base = format!("{server}__{tool}");
    if free(&base) {
        return base;
    }
    (2u64..)
        .map(|n| format!("{base}_{n}"))
        .find(|c| free(c))
        .expect("a free name")
}

/// Exposed name <-> (upstream index, tool name).
#[derive(Default)]
struct Names {
    /// Set when the client first sees or uses the names (`tools/list`, a
    /// call, the catalog). Before that names follow the file order afresh
    /// on every rebuild; from then on a name given is kept for the life of
    /// the process, also while its server is down or stops listing the
    /// tool, so it can never come to mean another tool.
    fixed: bool,
    by_key: HashMap<(usize, String), String>,
    owner: HashMap<String, (usize, String)>,
}

/// A client request served in its own thread, until it is answered.
#[derive(Default)]
struct Flight {
    /// The client cancelled it: no answer is sent (MCP cancellation).
    cancelled: bool,
    reason: Option<String>,
    /// The upstream request serving it, once sent.
    upstream: Option<(Arc<Conn>, u64)>,
}

#[derive(Clone)]
struct Route {
    exposed: String,
    up: usize,
    tool: String,
    def: J,
    declared: Vec<String>,
    read_only: bool,
    destructive: bool,
}

#[derive(Default)]
struct Routes {
    list: Vec<Route>,
    by_name: HashMap<String, usize>,
}

/// Per `server.tool`, this session plus what the audit log remembers.
#[derive(Default, Clone)]
struct Stat {
    calls: u64,
    errors: u64,
    raw: u64,
    out: u64,
    ms: u64,
    used: String,
}

fn stat_key(server: &str, tool: &str) -> String {
    format!("{server}\u{1f}{tool}")
}

/// Totals per tool from `audit.jsonl` (and its rotated predecessor).
fn seed_stats(home: &Path) -> HashMap<String, Stat> {
    let mut map: HashMap<String, Stat> = HashMap::new();
    for f in ["audit.1.jsonl", "audit.jsonl"] {
        let Ok(text) = std::fs::read_to_string(home.join(f)) else {
            continue;
        };
        for line in text.lines() {
            let Ok(j) = J::parse(line) else { continue };
            let (Some(s), Some(t)) = (
                j.get("server").and_then(J::as_str),
                j.get("tool").and_then(J::as_str),
            ) else {
                continue;
            };
            let e = map.entry(stat_key(s, t)).or_default();
            let used = j.get("used").map(J::text).unwrap_or_default();
            if used == "error" {
                e.errors += 1;
                continue;
            }
            e.calls += 1;
            e.raw += j.get("raw").and_then(J::as_u64).unwrap_or(0);
            e.out += j.get("out").and_then(J::as_u64).unwrap_or(0);
            e.ms += j.get("ms").and_then(J::as_u64).unwrap_or(0);
            e.used = used;
        }
    }
    map
}

pub struct Gateway {
    cfg: Config,
    store: Store,
    caps: Captures,
    audit: Option<Audit>,
    out: Out,
    ups: Vec<Upstream>,
    routes: RwLock<Routes>,
    names: Mutex<Names>,
    /// Client requests in progress, by their id as JSON text.
    inflight: Mutex<HashMap<String, Flight>>,
    stats: Mutex<HashMap<String, Stat>>,
    listed: AtomicBool,
    warned: Mutex<std::collections::HashSet<String>>,
}

fn tool_text(r: Result<String, String>) -> J {
    match r {
        Ok(t) => text_result(t, false),
        Err(e) => text_result(e, true),
    }
}

fn human(n: u64) -> String {
    if n >= 10_000 {
        format!("{:.1}k", n as f64 / 1000.0)
    } else {
        n.to_string()
    }
}

impl Gateway {
    pub fn new(cfg: Config, home: PathBuf) -> Arc<Gateway> {
        let ups = cfg
            .servers
            .iter()
            .map(|s| Upstream {
                cfg: s.clone(),
                st: Mutex::new(UpState {
                    phase: Phase::Starting,
                    conn: None,
                    tools: Vec::new(),
                    last_start: None,
                    about: None,
                    protocol: None,
                }),
                cv: Condvar::new(),
            })
            .collect();
        let store = Store::new(home.clone());
        Arc::new(Gateway {
            caps: Captures::new(store.fixtures_dir(), cfg.capture_limit),
            audit: cfg.audit.then(|| Audit::new(&home)),
            store,
            out: Out::stdout(),
            ups,
            routes: RwLock::new(Routes::default()),
            names: Mutex::new(Names::default()),
            inflight: Mutex::new(HashMap::new()),
            stats: Mutex::new(seed_stats(&home)),
            listed: AtomicBool::new(false),
            warned: Mutex::new(Default::default()),
            cfg,
        })
    }

    fn connect(self: &Arc<Self>, idx: usize) -> Result<Connected, String> {
        let cfg = &self.ups[idx].cfg;
        let mut cmd = Command::new(resolve_program(&cfg.command));
        cmd.args(&cfg.args)
            .envs(cfg.env.iter().map(|(k, v)| (k.as_str(), v.as_str())))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit());
        if let Some(c) = &cfg.cwd {
            cmd.current_dir(c);
        }
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x0800_0000);
        }
        let mut child = cmd
            .spawn()
            .map_err(|e| format!("cannot start '{}': {e}", cfg.command))?;
        let stdin = child.stdin.take().ok_or("no stdin")?;
        let stdout = child.stdout.take().ok_or("no stdout")?;
        let conn = Arc::new(Conn {
            server: cfg.name.clone(),
            stdin: Mutex::new(stdin),
            child: Mutex::new(child),
            pending: Mutex::new(HashMap::new()),
            next: AtomicU64::new(1),
            alive: AtomicBool::new(true),
        });
        let weak = Arc::downgrade(self);
        let reader_conn = conn.clone();
        thread::spawn(move || {
            read_loop(reader_conn, BufReader::new(stdout), move |method, msg| {
                let Some(gw) = weak.upgrade() else { return };
                match method {
                    "notifications/tools/list_changed" => {
                        thread::spawn(move || gw.refresh_tools(idx));
                    }
                    // The token is the client's own (passed on with the call).
                    "notifications/progress" => gw.out.send(&msg),
                    _ => {}
                }
            })
        });
        let init = J::obj()
            .with("protocolVersion", J::str(PROTOCOL))
            .with("capabilities", J::obj())
            .with(
                "clientInfo",
                J::obj()
                    .with("name", J::str("toolfmt"))
                    .with("version", J::str(crate::VERSION)),
            );
        let init = match conn.request("initialize", init, cfg.init_timeout) {
            Ok(r) => r,
            Err(e) => {
                conn.kill();
                return Err(error_text(&e));
            }
        };
        // A version toolfmt does not speak ends the connection (MCP: the
        // client should disconnect), with the reason in the overview.
        let protocol = match check_upstream_protocol(&init) {
            Ok(v) => v,
            Err(e) => {
                conn.kill();
                return Err(e);
            }
        };
        let about = init
            .get("instructions")
            .and_then(J::as_str)
            .map(|s| crate::catalog::short(s, 100))
            .filter(|s| !s.is_empty());
        conn.notify("notifications/initialized");
        match list_tools(&conn, cfg.init_timeout) {
            Ok(t) => Ok((conn, t, about, protocol)),
            Err(e) => {
                conn.kill();
                Err(error_text(&e))
            }
        }
    }

    /// First start of a server.
    fn start_upstream(self: &Arc<Self>, idx: usize) {
        {
            let mut st = self.ups[idx].st.lock().expect("state");
            st.phase = Phase::Starting;
            st.last_start = Some(Instant::now());
        }
        self.finish_start(idx);
    }

    /// Connects a server whose phase the caller set to `Starting` (under the
    /// state lock, so only one thread ever starts it), then publishes it.
    fn finish_start(self: &Arc<Self>, idx: usize) {
        let up = &self.ups[idx];
        let res = self.connect(idx);
        {
            let mut st = up.st.lock().expect("state");
            match res {
                Ok((conn, tools, about, protocol)) => {
                    st.phase = Phase::Ready;
                    if let Some(old) = st.conn.replace(conn) {
                        old.kill();
                    }
                    st.tools = tools;
                    st.about = about;
                    st.protocol = Some(protocol);
                }
                Err(e) => {
                    eprintln!("[toolfmt] server '{}' failed to start: {e}", up.cfg.name);
                    st.phase = Phase::Failed(e);
                    if let Some(old) = st.conn.take() {
                        old.kill();
                    }
                }
            }
        }
        up.cv.notify_all();
        self.rebuild_routes();
    }

    fn refresh_tools(self: &Arc<Self>, idx: usize) {
        let up = &self.ups[idx];
        let conn = up.st.lock().expect("state").conn.clone();
        if let Some(conn) = conn {
            if let Ok(tools) = list_tools(&conn, up.cfg.init_timeout) {
                up.st.lock().expect("state").tools = tools;
                self.rebuild_routes();
            }
        }
    }

    fn ensure_conn(self: &Arc<Self>, idx: usize) -> Result<Arc<Conn>, J> {
        let up = &self.ups[idx];
        let deadline = Instant::now() + up.cfg.init_timeout + Duration::from_secs(5);
        let mut st = up.st.lock().expect("state");
        while matches!(st.phase, Phase::Starting) {
            let now = Instant::now();
            if now >= deadline {
                break;
            }
            st = up.cv.wait_timeout(st, deadline - now).expect("state").0;
        }
        if let Some(c) = &st.conn {
            if c.alive.load(SeqCst) {
                return Ok(c.clone());
            }
        }
        let restart = claim_restart(&mut st, up.cfg.restart_backoff);
        drop(st);
        if restart {
            self.finish_start(idx);
        }
        let st = up.st.lock().expect("state");
        match (&st.conn, &st.phase) {
            (Some(c), _) if c.alive.load(SeqCst) => Ok(c.clone()),
            (_, Phase::Failed(e)) => Err(error_obj(
                -32603,
                format!("server '{}' is not available: {e}", up.cfg.name),
            )),
            _ => Err(error_obj(
                -32603,
                format!("server '{}' is not available", up.cfg.name),
            )),
        }
    }

    /// Recomputes the routes from the servers that are ready. A tool keeps
    /// the name it was given once names are fixed; a new tool gets the
    /// first free one (file order decides only among tools named together).
    fn rebuild_routes(&self) {
        let mut names = self.names.lock().expect("names");
        let names = &mut *names;
        let mut given: HashMap<(usize, String), String> = HashMap::new();
        let mut r = Routes::default();
        for (i, up) in self.ups.iter().enumerate() {
            let st = up.st.lock().expect("state");
            if !matches!(st.phase, Phase::Ready) {
                continue;
            }
            for t in &st.tools {
                let Some(name) = t.get("name").and_then(J::as_str) else {
                    continue;
                };
                let key = (i, name.to_string());
                let exposed = match names.by_key.get(&key).or_else(|| given.get(&key)) {
                    Some(e) => e.clone(),
                    None => {
                        let e = free_name(
                            |c| names.owner.contains_key(c) || r.by_name.contains_key(c),
                            &up.cfg.name,
                            name,
                        );
                        given.insert(key, e.clone());
                        e
                    }
                };
                if r.by_name.contains_key(&exposed) {
                    // The server listed the same tool twice.
                    continue;
                }
                let declared = t
                    .get("inputSchema")
                    .and_then(|s| s.get("properties"))
                    .and_then(J::as_obj)
                    .map(|m| m.iter().map(|(k, _)| k.clone()).collect())
                    .unwrap_or_default();
                let hint = |k: &str| {
                    t.get("annotations")
                        .and_then(|a| a.get(k))
                        .and_then(J::as_bool)
                        .unwrap_or(false)
                };
                r.by_name.insert(exposed.clone(), r.list.len());
                r.list.push(Route {
                    exposed,
                    up: i,
                    tool: name.to_string(),
                    def: t.clone(),
                    declared,
                    read_only: hint("readOnlyHint"),
                    destructive: hint("destructiveHint"),
                });
            }
        }
        if names.fixed {
            for (k, e) in given {
                names.owner.insert(e.clone(), k.clone());
                names.by_key.insert(k, e);
            }
        }
        let changed = {
            let old = self.routes.read().expect("routes");
            old.list.len() != r.list.len()
                || old
                    .list
                    .iter()
                    .zip(&r.list)
                    .any(|(a, b)| a.exposed != b.exposed || a.def != b.def)
        };
        if changed {
            let mut warned = self.warned.lock().expect("warned");
            for x in &r.list {
                if x.exposed != x.tool && warned.insert(x.exposed.clone()) {
                    eprintln!(
                        "[toolfmt] tool '{}' of server '{}' collides with another tool; exposed as '{}'",
                        x.tool, self.ups[x.up].cfg.name, x.exposed
                    );
                }
            }
        }
        *self.routes.write().expect("routes") = r;
        if changed && self.listed.load(SeqCst) {
            self.out.notify("notifications/tools/list_changed");
        }
    }

    fn dispatch(self: &Arc<Self>, mut msg: J) {
        let method = msg.get("method").and_then(J::as_str).map(str::to_string);
        let id = msg.remove("id");
        let params = msg.remove("params").unwrap_or_else(J::obj);
        let Some(method) = method else {
            // A response: toolfmt sends no requests to its client.
            return;
        };
        let Some(id) = id else {
            if method == "notifications/cancelled" {
                self.cancel(&params);
            }
            return;
        };
        match method.as_str() {
            "initialize" => {
                let v = negotiate(params.get("protocolVersion").and_then(J::as_str));
                self.out.respond(
                    &id,
                    J::obj()
                        .with("protocolVersion", J::str(v))
                        .with(
                            "capabilities",
                            J::obj().with("tools", J::obj().with("listChanged", J::Bool(true))),
                        )
                        .with(
                            "serverInfo",
                            J::obj()
                                .with("name", J::str("toolfmt"))
                                .with("version", J::str(crate::VERSION)),
                        )
                        .with("instructions", J::str(meta::instructions(self.cfg.formatter_tools))),
                );
            }
            "ping" | "logging/setLevel" => self.out.respond(&id, J::obj()),
            "tools/list" => {
                self.take_off(&id);
                let g = self.clone();
                thread::spawn(move || g.tools_list(id));
            }
            "tools/call" => {
                // Registered before the thread starts: a cancellation sent
                // right after the request is never lost.
                self.take_off(&id);
                let g = self.clone();
                thread::spawn(move || g.tools_call(id, params));
            }
            "resources/list" => self
                .out
                .respond(&id, J::obj().with("resources", J::Arr(vec![]))),
            "resources/templates/list" => self
                .out
                .respond(&id, J::obj().with("resourceTemplates", J::Arr(vec![]))),
            "prompts/list" => self
                .out
                .respond(&id, J::obj().with("prompts", J::Arr(vec![]))),
            m => self
                .out
                .error(&id, error_obj(-32601, format!("method '{m}' not found"))),
        }
    }

    fn take_off(&self, id: &J) {
        self.inflight
            .lock()
            .expect("inflight")
            .insert(id.to_json(), Flight::default());
    }

    /// Ends a client request; false when the client cancelled it, and then
    /// no answer may be sent.
    fn land(&self, id: &J) -> bool {
        self.inflight
            .lock()
            .expect("inflight")
            .remove(&id.to_json())
            .is_none_or(|f| !f.cancelled)
    }

    fn answer(&self, id: &J, result: J) {
        if self.land(id) {
            self.out.respond(id, result);
        }
    }

    fn fail(&self, id: &J, err: J) {
        if self.land(id) {
            self.out.error(id, err);
        }
    }

    /// `notifications/cancelled` from the client: the request is marked so
    /// it gets no answer, and the upstream request serving it is cancelled
    /// too. A request already answered or unknown is ignored (MCP).
    fn cancel(&self, params: &J) {
        let Some(rid) = params.get("requestId") else {
            return;
        };
        let reason = params.get("reason").and_then(J::as_str).map(str::to_string);
        let upstream = {
            let mut fl = self.inflight.lock().expect("inflight");
            let Some(f) = fl.get_mut(&rid.to_json()) else {
                return;
            };
            f.cancelled = true;
            f.reason = reason.clone();
            f.upstream.clone()
        };
        if let Some((conn, uid)) = upstream {
            conn.cancel(uid, reason.as_deref());
        }
    }

    /// Records the upstream request serving client request `id`; if the
    /// client cancelled meanwhile, cancels it right away. False then.
    fn link(&self, id: &J, conn: &Arc<Conn>, uid: u64) -> bool {
        let cancelled = {
            let mut fl = self.inflight.lock().expect("inflight");
            match fl.get_mut(&id.to_json()) {
                Some(f) if f.cancelled => Some(f.reason.clone()),
                Some(f) => {
                    f.upstream = Some((conn.clone(), uid));
                    None
                }
                None => None,
            }
        };
        match cancelled {
            Some(reason) => {
                conn.cancel(uid, reason.as_deref());
                false
            }
            None => true,
        }
    }

    fn cancelled(&self, id: &J) -> bool {
        self.inflight
            .lock()
            .expect("inflight")
            .get(&id.to_json())
            .is_some_and(|f| f.cancelled)
    }

    /// Waits (bounded) for servers still starting, then makes sure the
    /// routes reflect every server that is ready by now. From here on the
    /// names are fixed: the client is about to see or use them.
    fn wait_startup(&self) {
        let deadline = Instant::now() + self.cfg.startup_wait;
        for up in &self.ups {
            let mut st = up.st.lock().expect("state");
            while matches!(st.phase, Phase::Starting) {
                let now = Instant::now();
                if now >= deadline {
                    break;
                }
                st = up.cv.wait_timeout(st, deadline - now).expect("state").0;
            }
        }
        self.names.lock().expect("names").fixed = true;
        self.rebuild_routes();
    }

    fn names_fixed(&self) -> bool {
        self.names.lock().expect("names").fixed
    }

    /// The server a name was given to, while the name has no route (its
    /// server is down or stopped listing the tool).
    fn bound_server(&self, name: &str) -> Option<usize> {
        self.names
            .lock()
            .expect("names")
            .owner
            .get(name)
            .map(|k| k.0)
    }

    /// Whether the upstream tools go behind `catalog` + `call_tool`.
    fn catalog_mode(&self, tools: usize) -> bool {
        match self.cfg.expose {
            Expose::All => false,
            Expose::Catalog => true,
            Expose::Auto => tools > self.cfg.catalog_threshold,
        }
    }

    fn tools_list(&self, id: J) {
        self.wait_startup();
        self.listed.store(true, SeqCst);
        let routes = self.routes.read().expect("routes");
        let catalog = self.catalog_mode(routes.list.len());
        let mut tools = Vec::with_capacity(if catalog { 8 } else { routes.list.len() + 2 });
        for r in &routes.list {
            if catalog && !self.cfg.pin.iter().any(|p| p.matches(&r.exposed)) {
                continue;
            }
            let mut d = r.def.clone();
            d.set("name", J::str(&r.exposed));
            // Formatted results are text: an output schema would be a lie.
            d.remove("outputSchema");
            tools.push(d);
        }
        drop(routes);
        if catalog {
            tools.extend(catalog::defs());
        }
        if self.cfg.formatter_tools {
            tools.extend(meta::defs());
        }
        self.answer(&id, J::obj().with("tools", J::Arr(tools)));
    }

    fn entries(&self) -> Vec<catalog::Entry> {
        let routes = self.routes.read().expect("routes");
        routes
            .list
            .iter()
            .map(|r| {
                let cfg = &self.ups[r.up].cfg;
                let mut section: Vec<String> = cfg.section.split('/').map(str::to_string).collect();
                if let Some(g) = cfg.group_of(&r.tool) {
                    section.extend(g.split('/').map(str::to_string));
                }
                catalog::Entry {
                    name: r.exposed.clone(),
                    server: cfg.name.clone(),
                    section,
                    def: r.def.clone(),
                    read_only: r.read_only,
                    destructive: r.destructive,
                }
            })
            .collect()
    }

    fn section_desc(&self, path: &str) -> Option<String> {
        if let Some((_, d)) = self.cfg.sections.iter().find(|(p, _)| p == path) {
            return Some(d.clone());
        }
        self.ups
            .iter()
            .find(|u| u.cfg.section == path)
            .and_then(|u| u.st.lock().expect("state").about.clone())
    }

    fn card(&self, e: &catalog::Entry) -> catalog::Card {
        let tool = e.def.get("name").and_then(J::as_str).unwrap_or(&e.name);
        let standard = match self.store.tool(&e.server, tool).as_deref() {
            Some(Ok(s)) => Some(s.render.name().to_string()),
            Some(Err(_)) => Some("invalid standard: raw output".into()),
            None => self
                .cfg
                .family_of(&e.server, tool)
                .and_then(|f| self.store.family(&f, tool))
                .and_then(|a| match a.as_ref() {
                    Ok(s) => Some(format!("{}, family", s.render.name())),
                    Err(_) => None,
                }),
        };
        let st = self
            .stats
            .lock()
            .expect("stats")
            .get(&stat_key(&e.server, tool))
            .cloned()
            .unwrap_or_default();
        let example = self.caps.list(&e.server, tool).first().map(|f| {
            let a = f.args().to_json();
            if a.chars().count() > 200 {
                a.chars().take(200).collect::<String>() + "…"
            } else {
                a
            }
        });
        catalog::Card {
            standard,
            calls: st.calls,
            errors: st.errors,
            raw: st.raw,
            out: st.out,
            ms: st.ms,
            price: self
                .cfg
                .server(&e.server)
                .and_then(|c| c.price_of(tool))
                .map(str::to_string),
            example,
        }
    }

    fn catalog(&self, args: &J) -> Result<String, String> {
        let entries = self.entries();
        let names: Vec<String> = match args.get("tools") {
            Some(J::Arr(a)) => a.iter().map(J::text).collect(),
            Some(J::Str(s)) => s
                .split(',')
                .map(|x| x.trim().to_string())
                .filter(|x| !x.is_empty())
                .collect(),
            _ => Vec::new(),
        };
        if !names.is_empty() {
            return Ok(catalog::cards(&entries, &names, &|e| self.card(e)));
        }
        if let Some(q) = args
            .get("query")
            .and_then(J::as_str)
            .filter(|q| !q.trim().is_empty())
        {
            return Ok(catalog::search(&entries, q));
        }
        catalog::browse(&entries, args.get("path").and_then(J::as_str), &|p| {
            self.section_desc(p)
        })
    }

    fn tools_call(self: &Arc<Self>, id: J, mut params: J) {
        let mut name = params
            .get("name")
            .and_then(J::as_str)
            .unwrap_or("")
            .to_string();
        let mut args = params
            .remove("arguments")
            .filter(|a| matches!(a, J::Obj(_)))
            .unwrap_or_else(J::obj);
        let via_call = name == catalog::CALL;
        if via_call {
            let Some(inner) = args.get("tool").and_then(J::as_str).map(str::to_string) else {
                return self.answer(
                    &id,
                    text_result("call_tool needs 'tool' (a name from catalog)", true),
                );
            };
            args = match args.remove("args") {
                Some(a @ J::Obj(_)) => a,
                // Some agents pass the arguments as a JSON string.
                Some(J::Str(s)) => J::parse(&s)
                    .ok()
                    .filter(|a| matches!(a, J::Obj(_)))
                    .unwrap_or_else(J::obj),
                _ => J::obj(),
            };
            name = inner;
        }
        if name == catalog::CATALOG {
            self.wait_startup();
            return self.answer(&id, tool_text(self.catalog(&args)));
        }
        if name == meta::INSPECT && self.cfg.formatter_tools {
            self.wait_startup();
            return self.answer(&id, tool_text(self.inspect(&args)));
        }
        if name == meta::SAVE && self.cfg.formatter_tools {
            self.wait_startup();
            return self.answer(&id, tool_text(self.save(&args)));
        }
        let route = match self.route_of(&name) {
            Ok(r) => r,
            Err(None) if via_call => {
                let hint = catalog::search(&self.entries(), &name);
                return self.answer(
                    &id,
                    text_result(format!("unknown tool '{name}'. {hint}"), true),
                );
            }
            Err(None) => {
                return self.fail(&id, error_obj(-32602, format!("unknown tool '{name}'")));
            }
            Err(Some(e)) => return self.fail(&id, e),
        };
        let ov = Overrides::extract(&mut args, &route.declared);
        if via_call {
            // No client checked these arguments against the schema: say what
            // is missing before the upstream fails less helpfully.
            let schema = route.def.get("inputSchema");
            let missing = catalog::missing(schema, &args);
            if !missing.is_empty() {
                let (sig, _) = catalog::signature(schema);
                return self.answer(
                    &id,
                    text_result(
                        format!(
                            "missing required {}: call {}({sig})",
                            missing.join(", "),
                            route.exposed
                        ),
                        true,
                    ),
                );
            }
        }
        let conn = match self.ensure_conn(route.up) {
            Ok(c) => c,
            Err(e) => return self.fail(&id, e),
        };
        if self.cancelled(&id) {
            // Cancelled before it reached the upstream: never sent at all.
            self.land(&id);
            return;
        }
        let timeout = self.ups[route.up].cfg.timeout;
        let mut call = J::obj()
            .with("name", J::str(&route.tool))
            .with("arguments", args.clone());
        // A progress token is the client's own: the upstream's progress
        // notifications go back to the client as they are.
        if let Some(m) = params.remove("_meta") {
            call.set("_meta", m);
        }
        let t0 = Instant::now();
        let (uid, rx) = match conn.send("tools/call", call) {
            Ok(x) => x,
            Err(e) => return self.fail(&id, e),
        };
        if !self.link(&id, &conn, uid) {
            self.land(&id);
            return;
        }
        match conn.wait(uid, &rx, timeout) {
            Err(e) => self.fail(&id, e),
            Ok(result) => {
                let r = self.process(&route, &args, ov, result, t0);
                self.answer(&id, r);
            }
        }
    }

    /// The route of a tool name. `None`: no such name. A name the client
    /// was given whose server is down or stopped listing the tool stays
    /// that tool's: the server is restarted if the backoff allows, and the
    /// call never reaches another tool.
    fn route_of(self: &Arc<Self>, name: &str) -> Result<Route, Option<J>> {
        let lookup = || {
            let r = self.routes.read().expect("routes");
            r.by_name.get(name).map(|&i| r.list[i].clone())
        };
        // First use before any tools/list: name by the file order of every
        // server that comes up in time, then fix the names.
        let waited = !self.names_fixed();
        if waited {
            self.wait_startup();
        }
        if let Some(r) = lookup() {
            return Ok(r);
        }
        if !waited {
            // A call can arrive before slow servers finished starting.
            self.wait_startup();
            if let Some(r) = lookup() {
                return Ok(r);
            }
        }
        let up = self.bound_server(name).ok_or(None)?;
        self.ensure_conn(up).map_err(Some)?;
        lookup().ok_or_else(|| {
            Some(error_obj(
                -32602,
                format!(
                    "tool '{name}' is no longer offered by server '{}'",
                    self.ups[up].cfg.name
                ),
            ))
        })
    }

    fn process(&self, route: &Route, args: &J, ov: Overrides, result: J, t0: Instant) -> J {
        let cfg = &self.ups[route.up].cfg;
        let server = cfg.name.as_str();
        if result.get("isError").and_then(J::as_bool) == Some(true) {
            self.account(route, "error", 0, 0, t0);
            return result;
        }
        if cfg.capture {
            self.caps.maybe_capture(server, &route.tool, args, &result);
        }
        let (outcome, src): (Option<Outcome>, Source) = {
            let (pl, src) = payload_src(&result);
            let data = match &pl {
                Payload::Text(t) => Some(Data::Text(t)),
                Payload::Json(j) => Some(Data::Json(j)),
                Payload::None => None,
            };
            let outcome = data.map(|data| {
                let req = Request {
                    server,
                    tool: &route.tool,
                    args,
                    data,
                    keys: None,
                    ov,
                    auto: cfg.auto,
                    family: self.cfg.family_of(server, &route.tool),
                };
                pipeline::run(Some(&self.store), &req, None)
            });
            (outcome, src)
        };
        let Some(out) = outcome else {
            self.account(route, "passthrough", 0, 0, t0);
            return result;
        };
        self.account(
            route,
            &out.used,
            out.raw_chars as u64,
            out.out_chars as u64,
            t0,
        );
        let header = out.header.clone().unwrap_or_default();
        match out.body {
            Body::Untouched => result,
            Body::Fallback(_) => {
                let mut result = result;
                match result.get_mut("content") {
                    Some(J::Arr(blocks)) => blocks.insert(0, text_block(header)),
                    _ => result.set("content", J::Arr(vec![text_block(header)])),
                }
                result
            }
            Body::Formatted(text) => {
                let image_meta = out.used.starts_with(Render::ImageMeta.name());
                formatted_result(result, &src, format!("{header}\n{text}"), image_meta)
            }
        }
    }

    fn account(&self, route: &Route, used: &str, raw: u64, out: u64, t0: Instant) {
        let server = &self.ups[route.up].cfg.name;
        let ms = t0.elapsed().as_millis() as u64;
        {
            let mut s = self.stats.lock().expect("stats");
            let e = s.entry(stat_key(server, &route.tool)).or_default();
            if used == "error" {
                e.errors += 1;
            } else {
                e.calls += 1;
                e.raw += raw;
                e.out += out;
                e.ms += ms;
                e.used = used.to_string();
            }
        }
        if let Some(a) = &self.audit {
            a.record(
                &J::obj()
                    .with("ts", J::Str(crate::capture::now_iso()))
                    .with("server", J::str(server))
                    .with("tool", J::str(&route.tool))
                    .with("used", J::str(used))
                    .with("raw", J::int(raw))
                    .with("out", J::int(out))
                    .with("ms", J::int(ms)),
            );
        }
    }

    /// `tool` as listed, `server.tool` or `server__tool`.
    fn resolve(&self, name: &str) -> Result<(String, String, Option<Route>), String> {
        {
            let routes = self.routes.read().expect("routes");
            if let Some(&i) = routes.by_name.get(name) {
                let r = routes.list[i].clone();
                return Ok((self.ups[r.up].cfg.name.clone(), r.tool.clone(), Some(r)));
            }
        }
        // A listed name whose server is down right now. (Lock order:
        // names before routes, as in rebuild_routes.)
        if let Some((up, tool)) = self.names.lock().expect("names").owner.get(name) {
            return Ok((self.ups[*up].cfg.name.clone(), tool.clone(), None));
        }
        let routes = self.routes.read().expect("routes");
        for sep in [".", "__"] {
            if let Some((s, t)) = name.split_once(sep) {
                if self.cfg.server(s).is_some() {
                    let r = routes
                        .list
                        .iter()
                        .find(|r| self.ups[r.up].cfg.name == s && r.tool == t)
                        .cloned();
                    return Ok((s.to_string(), t.to_string(), r));
                }
            }
        }
        Err(format!(
            "unknown tool '{name}' (use the listed name or server.tool)"
        ))
    }

    fn inspect(&self, args: &J) -> Result<String, String> {
        if args.get("topic").and_then(J::as_str) == Some("schema") {
            return Ok(meta::REFERENCE.to_string());
        }
        match args.get("tool").and_then(J::as_str) {
            Some(t) => self.inspect_tool(t, args.get("capture").and_then(J::as_u64)),
            None => Ok(self.overview()),
        }
    }

    fn overview(&self) -> String {
        let mut out = format!(
            "toolfmt {} | home {}\n",
            crate::VERSION,
            self.store.home.display()
        );
        for up in &self.ups {
            let st = up.st.lock().expect("state");
            let state = match &st.phase {
                Phase::Starting => "starting".to_string(),
                Phase::Ready => format!(
                    "{} tools, protocol {}",
                    st.tools.len(),
                    st.protocol.unwrap_or("?")
                ),
                Phase::Failed(e) => format!("failed: {e}"),
            };
            let _ = writeln!(out, "server {}: {state}", up.cfg.name);
        }
        let stats = self.stats.lock().expect("stats").clone();
        let (calls, raw, outc) = stats
            .values()
            .fold((0, 0, 0), |a, s| (a.0 + s.calls, a.1 + s.raw, a.2 + s.out));
        if raw > 0 {
            let saved = 100.0 * (1.0 - outc as f64 / raw as f64);
            let _ = writeln!(
                out,
                "measured (audit log): {calls} calls, {} -> {} chars ({saved:.0}% saved)",
                human(raw),
                human(outc)
            );
        }
        let routes = self.routes.read().expect("routes");
        let _ = writeln!(
            out,
            "tools offered as: {}",
            if self.catalog_mode(routes.list.len()) {
                "catalog + call_tool (schemas on demand)"
            } else {
                "a flat list"
            }
        );
        let mut rows = Vec::new();
        let mut idle_ro = Vec::new();
        let mut idle = 0usize;
        for r in &routes.list {
            let server = &self.ups[r.up].cfg.name;
            let has_std = self.store.tool(server, &r.tool).is_some();
            let caps = self.caps.count(server, &r.tool);
            let st = stats.get(&stat_key(server, &r.tool));
            if !has_std && caps == 0 && st.is_none() {
                idle += 1;
                if r.read_only {
                    idle_ro.push(r.exposed.clone());
                }
                continue;
            }
            let saved = st
                .filter(|s| s.raw > 0)
                .map(|s| format!("{:.0}%", 100.0 * (1.0 - s.out as f64 / s.raw as f64)))
                .unwrap_or_default();
            rows.push(format!(
                "  {},{},{},{},{},{}",
                r.exposed,
                server,
                if has_std { "yes" } else { "no" },
                caps,
                st.map_or(0, |s| s.calls),
                saved
            ));
        }
        let _ = write!(
            out,
            "[{}]{{tool,server,standard,captures,calls,saved}}:",
            rows.len()
        );
        for row in rows {
            out.push('\n');
            out.push_str(&row);
        }
        if idle > 0 {
            let _ = write!(out, "\n{idle} more tools not called yet");
            if !idle_ro.is_empty() {
                let shown: Vec<&str> = idle_ro.iter().take(20).map(String::as_str).collect();
                let _ = write!(
                    out,
                    "; read-only ones are safe to call once to capture: {}",
                    shown.join(", ")
                );
                if idle_ro.len() > 20 {
                    let _ = write!(out, " … {} more", idle_ro.len() - 20);
                }
            }
        }
        out.push_str(
            "\nnext: formatter_inspect tool=<name> -> formatter_save_standard tool=<name> yaml=...",
        );
        out
    }

    fn inspect_tool(&self, name: &str, capture: Option<u64>) -> Result<String, String> {
        let (server, tool, route) = self.resolve(name)?;
        let mut out = String::new();
        let _ = write!(out, "tool {tool} (server {server}");
        if let Some(r) = &route {
            if r.exposed != tool {
                let _ = write!(out, ", listed as {}", r.exposed);
            }
            if r.read_only {
                out.push_str(", read-only");
            }
        }
        out.push_str(")\n");
        if let Some(r) = &route {
            if let Some(d) = r.def.get("description").and_then(J::as_str) {
                let d: String = d.chars().take(300).collect();
                let _ = writeln!(out, "description: {}", d.replace('\n', " "));
            }
            if !r.declared.is_empty() {
                let _ = writeln!(out, "params: {}", r.declared.join(", "));
            }
        }
        let path = self.store.tool_file(&server, &tool);
        let current = self.store.tool(&server, &tool);
        match &current {
            Some(_) => {
                let file = if path.is_file() {
                    path.clone()
                } else {
                    path.with_extension("yaml")
                };
                let yaml = std::fs::read_to_string(&file).unwrap_or_default();
                let _ = writeln!(out, "standard {}:\n{}", file.display(), yaml.trim_end());
                if let Some(Err(e)) = current.as_deref() {
                    let _ = writeln!(out, "standard is INVALID: {e}");
                }
            }
            None => {
                let fam = self.cfg.family_of(&server, &tool);
                match fam {
                    Some(f) => {
                        let _ = writeln!(
                            out,
                            "standard: none (family '{f}' applies if standards/family/{f}.yml exists)"
                        );
                    }
                    None => out.push_str("standard: none (output passes through untouched)\n"),
                }
            }
        }
        let caps = self.caps.list(&server, &tool);
        if caps.is_empty() {
            out.push_str("captures: none yet. Call the tool once (a real call): its output is captured, then write the standard against it.");
            return Ok(out);
        }
        let _ = writeln!(out, "captures ({}):", caps.len());
        for (i, f) in caps.iter().enumerate() {
            let pl = f.payload();
            let (shape, chars) = match &pl {
                Payload::Text(t) => {
                    let j = crate::json::J::parse_doc(t);
                    (detect(t, j.as_ref()).name(), t.chars().count())
                }
                Payload::Json(j) => (
                    crate::detect::detect_json(j).name(),
                    j.to_json().chars().count(),
                ),
                Payload::None => ("none", 0),
            };
            let args = f.args().to_json();
            let args: String = args.chars().take(120).collect();
            let _ = write!(out, "  #{} {shape}, {chars} chars, args {args}", i + 1);
            if let Some((out_text, used)) = (current.is_some())
                .then(|| run_fixture(Some(&self.store), Some(&self.cfg), f, None))
                .flatten()
                .map(|(o, raw)| (o.agent_text(&raw), o.used))
            {
                let first = out_text
                    .lines()
                    .next()
                    .unwrap_or("")
                    .chars()
                    .take(100)
                    .collect::<String>();
                let _ = write!(out, "\n     now: {used} | {first}");
            }
            out.push('\n');
        }
        let n = capture.map(|c| c as usize).unwrap_or(caps.len());
        let f = caps
            .get(n.saturating_sub(1))
            .ok_or_else(|| format!("no capture #{n} (there are {})", caps.len()))?;
        let raw = match f.payload() {
            Payload::Text(t) => t.into_owned(),
            Payload::Json(j) => j.to_json(),
            Payload::None => String::from("(no payload)"),
        };
        let max = if capture.is_some() { 60_000 } else { 6_000 };
        let _ = write!(out, "capture #{n} payload:\n{}", preview(&raw, max));
        if raw.chars().count() > max {
            let _ = write!(
                out,
                " (formatter_inspect tool={name} capture={n} shows more)"
            );
        }
        Ok(out)
    }

    fn save(&self, args: &J) -> Result<String, String> {
        let name = args
            .get("tool")
            .and_then(J::as_str)
            .ok_or("'tool' is required")?;
        let (server, tool, _) = self.resolve(name)?;
        let path = self.store.tool_file(&server, &tool);
        if args.get("delete").and_then(J::as_bool) == Some(true) {
            let mut removed = false;
            for p in [path.clone(), path.with_extension("yaml")] {
                if p.is_file() {
                    std::fs::remove_file(&p)
                        .map_err(|e| format!("cannot delete {}: {e}", p.display()))?;
                    self.store.invalidate(&p);
                    removed = true;
                }
            }
            return Ok(if removed {
                format!(
                    "deleted the standard of {server}.{tool}: its output passes through untouched again"
                )
            } else {
                format!("{server}.{tool} has no standard")
            });
        }
        let yaml = args
            .get("yaml")
            .and_then(J::as_str)
            .ok_or("'yaml' is required (formatter_inspect topic=schema)")?;
        let std = compile(yaml, &path.display().to_string(), &tool)
            .map_err(|e| format!("invalid standard: {e}"))?;
        let caps = self.caps.list(&server, &tool);
        if caps.is_empty() {
            return Err(format!(
                "no captured output of {server}.{tool} yet: call the tool once first, then write the standard against that real output (D3)"
            ));
        }
        let dry = args.get("dry_run").and_then(J::as_bool) == Some(true);
        let force = args.get("force").and_then(J::as_bool) == Some(true);
        let mut report = String::new();
        let mut budget = 6_000usize;
        let mut rows = Vec::new();
        let mut fell_back = Vec::new();
        let mut empty = Vec::new();
        for (i, f) in caps.iter().enumerate() {
            let Some((o, raw)) = run_fixture(Some(&self.store), Some(&self.cfg), f, Some(&std))
            else {
                let _ = writeln!(report, "#{} has no payload: not replayed\n", i + 1);
                empty.push(format!("#{}", i + 1));
                continue;
            };
            let agent = o.agent_text(&raw);
            let _ = write!(
                report,
                "#{} {} -> {} chars [{}]",
                i + 1,
                o.raw_chars,
                o.out_chars,
                o.used
            );
            if matches!(o.body, Body::Fallback(_)) {
                report.push_str(" FELL BACK TO RAW");
                fell_back.push(format!("#{}", i + 1));
            }
            report.push('\n');
            let p = preview(&agent, budget.clamp(200, 3_000));
            budget = budget.saturating_sub(p.chars().count());
            report.push_str(&p);
            report.push_str("\n\n");
            rows.push((i, clean_text(&o, &raw), o.used));
        }
        let report = report.trim_end();
        // A standard that does not work on the real outputs it was written
        // against is not saved unless asked for explicitly.
        let problem = if rows.is_empty() {
            Some("no capture has a payload to replay it on".to_string())
        } else if !fell_back.is_empty() {
            Some(format!(
                "it fell back to raw output on capture {} of {}",
                fell_back.join(", "),
                caps.len()
            ))
        } else {
            None
        };
        if dry {
            let verdict = match &problem {
                Some(p) => format!(" Saving would be refused: {p} (force=true saves anyway)."),
                None => String::new(),
            };
            return Ok(format!(
                "dry run, not saved.{verdict} Replay on {} captures:\n{report}",
                caps.len()
            ));
        }
        if let Some(p) = &problem {
            if !force {
                return Err(format!(
                    "not saved: {p}. Fix the standard, or pass force=true to save it anyway. Replay on {} captures:\n{report}",
                    caps.len()
                ));
            }
        }
        write_atomic(&path, yaml).map_err(|e| format!("cannot write {}: {e}", path.display()))?;
        self.store.invalidate(&path);
        let mut failed = Vec::new();
        for (i, text, used) in &rows {
            if let Err(e) = write_baseline(&caps[*i], text, used) {
                failed.push(format!("#{} ({}: {e})", i + 1, caps[*i].path.display()));
            }
        }
        let mut head = format!(
            "saved {} (applies from the next call). Baselines recorded in {} of {} captures",
            path.display(),
            rows.len() - failed.len(),
            caps.len()
        );
        if !empty.is_empty() {
            let _ = write!(head, "; no payload: {}", empty.join(", "));
        }
        if !failed.is_empty() {
            let _ = write!(head, "; FAILED to write: {}", failed.join(", "));
        }
        head.push('.');
        if let Some(p) = &problem {
            let _ = write!(head, " Saved with force=true although {p}.");
        }
        Ok(format!("{head} Replay:\n{report}"))
    }

    fn shutdown(&self) {
        for up in &self.ups {
            if let Some(c) = &up.st.lock().expect("state").conn {
                c.kill();
            }
        }
    }
}

fn text_block(text: String) -> J {
    J::obj()
        .with("type", J::str("text"))
        .with("text", J::Str(text))
}

/// The result with a formatted `text`. The text takes the place of exactly
/// what the payload was made of (`src`): structuredContent and the text
/// blocks that mirror it, all the text blocks it joined, or, for
/// image_meta, the one image it described. It goes where the first of them
/// was; every other block keeps its place and every other field of the
/// result (`_meta`, `isError`, extensions) stays. Fields all the replaced
/// blocks share (`annotations`, `_meta`) move to the new block.
fn formatted_result(mut result: J, src: &Source, text: String, image_meta: bool) -> J {
    let blocks = match result.get_mut("content") {
        Some(J::Arr(b)) => std::mem::take(b),
        _ => Vec::new(),
    };
    let replaced: Vec<usize> = match src {
        Source::Texts(ix) => ix.clone(),
        Source::Image(i) if image_meta => vec![*i],
        Source::Structured => {
            // The serialized copy MCP asks servers to add; any other text
            // is information of its own and stays.
            let sc = result.get("structuredContent");
            blocks
                .iter()
                .enumerate()
                .filter(|(_, b)| b.get("type").and_then(J::as_str) == Some("text"))
                .filter(|(_, b)| {
                    b.get("text")
                        .and_then(J::as_str)
                        .and_then(J::parse_doc)
                        .as_ref()
                        == sc
                })
                .map(|(i, _)| i)
                .collect()
        }
        _ => Vec::new(),
    };
    let mut block = text_block(text);
    if let Some((&first, rest)) = replaced.split_first() {
        if let Some(J::Obj(fields)) = blocks.get(first) {
            for (k, v) in fields {
                let content_field = matches!(k.as_str(), "type" | "text" | "data" | "mimeType");
                if !content_field && rest.iter().all(|&i| blocks[i].get(k) == Some(v)) {
                    block.set(k, v.clone());
                }
            }
        }
    }
    // A described-but-kept image (not image_meta) gets its text in front.
    let at = match src {
        Source::Image(i) => *i,
        _ => replaced.first().copied().unwrap_or(0),
    };
    let mut block = Some(block);
    let mut content = Vec::with_capacity(blocks.len() + 1);
    for (i, b) in blocks.into_iter().enumerate() {
        if i == at {
            content.extend(block.take());
        }
        if !replaced.contains(&i) {
            content.push(b);
        }
    }
    content.extend(block.take());
    if *src == Source::Structured {
        result.remove("structuredContent");
    }
    result.set("content", J::Arr(content));
    result
}

fn list_tools(conn: &Conn, timeout: Duration) -> Result<Vec<J>, J> {
    let mut all = Vec::new();
    let mut cursor: Option<J> = None;
    for _ in 0..100 {
        let params = match &cursor {
            Some(c) => J::obj().with("cursor", c.clone()),
            None => J::obj(),
        };
        let mut r = conn.request("tools/list", params, timeout)?;
        if let Some(J::Arr(t)) = r.remove("tools") {
            all.extend(t);
        }
        match r.remove("nextCursor") {
            Some(c @ J::Str(_)) => cursor = Some(c),
            _ => break,
        }
    }
    Ok(all)
}

/// Runs the gateway on stdin/stdout until the client closes stdin.
pub fn serve(cfg: Config, home: &Path) -> i32 {
    let gw = Gateway::new(cfg, home.to_path_buf());
    for i in 0..gw.ups.len() {
        let g = gw.clone();
        thread::spawn(move || g.start_upstream(i));
    }
    for_each_line(io::stdin().lock(), |line| {
        let line = line.trim();
        if line.is_empty() {
            return;
        }
        match J::parse(line) {
            Ok(J::Arr(batch)) => batch.into_iter().for_each(|m| gw.dispatch(m)),
            Ok(m) => gw.dispatch(m),
            Err(e) => gw.out.error(&J::Null, error_obj(-32700, e)),
        }
    });
    gw.shutdown();
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A gateway over servers that are never started: their state is set
    /// by hand.
    fn gw(servers: &[&str]) -> Arc<Gateway> {
        let list: Vec<String> = servers
            .iter()
            .map(|s| format!(r#""{s}":{{"command":"none"}}"#))
            .collect();
        let text = format!(r#"{{"audit":false,"mcpServers":{{{}}}}}"#, list.join(","));
        let cfg = Config::parse(&text, Path::new(".")).unwrap();
        Gateway::new(cfg, std::env::temp_dir().join("toolfmt-unit-names"))
    }

    fn ready(g: &Gateway, up: usize, tools: &[&str]) {
        let mut st = g.ups[up].st.lock().unwrap();
        st.phase = Phase::Ready;
        st.tools = tools
            .iter()
            .map(|t| J::obj().with("name", J::str(*t)))
            .collect();
    }

    fn down(g: &Gateway, up: usize) {
        g.ups[up].st.lock().unwrap().phase = Phase::Failed("down".into());
    }

    /// `server.tool` behind a listed name.
    fn target(g: &Gateway, name: &str) -> Option<String> {
        let r = g.routes.read().unwrap();
        r.by_name
            .get(name)
            .map(|&i| format!("{}.{}", g.ups[r.list[i].up].cfg.name, r.list[i].tool))
    }

    fn fix(g: &Gateway) {
        g.names.lock().unwrap().fixed = true;
        g.rebuild_routes();
    }

    #[test]
    fn free_name_never_gives_up() {
        let taken = ["t", "s__t", "s__t_2"];
        assert_eq!(free_name(|c| taken.contains(&c), "s", "u"), "u");
        assert_eq!(free_name(|c| taken.contains(&c), "s", "t"), "s__t_3");
        assert_eq!(free_name(|_| false, "s", "catalog"), "s__catalog");
        assert_eq!(
            free_name(|_| false, "s", meta::SAVE),
            "s__formatter_save_standard"
        );
    }

    #[test]
    fn before_names_are_fixed_the_file_order_decides() {
        let g = gw(&["a", "b"]);
        ready(&g, 1, &["foo"]);
        g.rebuild_routes();
        assert_eq!(target(&g, "foo").as_deref(), Some("b.foo"));
        // Nobody saw the names yet: the first server takes the bare one.
        ready(&g, 0, &["foo"]);
        g.rebuild_routes();
        assert_eq!(target(&g, "foo").as_deref(), Some("a.foo"));
        assert_eq!(target(&g, "b__foo").as_deref(), Some("b.foo"));
    }

    #[test]
    fn a_fixed_name_never_moves_to_another_tool() {
        let g = gw(&["a", "b"]);
        ready(&g, 1, &["foo", "delete_all"]);
        fix(&g);
        assert_eq!(target(&g, "delete_all").as_deref(), Some("b.delete_all"));
        // The earlier server starts late: it gets the prefixed names, the
        // names the client already has keep pointing where they did.
        ready(&g, 0, &["foo", "delete_all"]);
        g.rebuild_routes();
        assert_eq!(target(&g, "delete_all").as_deref(), Some("b.delete_all"));
        assert_eq!(target(&g, "a__delete_all").as_deref(), Some("a.delete_all"));
        assert_eq!(target(&g, "foo").as_deref(), Some("b.foo"));
        // b goes down: its names stay reserved, no route, nothing stolen.
        down(&g, 1);
        g.rebuild_routes();
        assert_eq!(target(&g, "delete_all"), None);
        assert_eq!(g.bound_server("delete_all"), Some(1));
        assert_eq!(target(&g, "a__delete_all").as_deref(), Some("a.delete_all"));
        // b comes back: the same names as before.
        ready(&g, 1, &["foo", "delete_all"]);
        g.rebuild_routes();
        assert_eq!(target(&g, "delete_all").as_deref(), Some("b.delete_all"));
        assert_eq!(target(&g, "a__foo").as_deref(), Some("a.foo"));
        // A tool b stops listing keeps its name reserved too.
        ready(&g, 1, &["delete_all"]);
        ready(&g, 0, &["foo", "delete_all", "bar"]);
        g.rebuild_routes();
        assert_eq!(target(&g, "foo"), None);
        assert_eq!(g.bound_server("foo"), Some(1));
        assert_eq!(target(&g, "bar").as_deref(), Some("a.bar"));
    }

    #[test]
    fn every_collision_is_resolved() {
        let g = gw(&["c", "a", "b", "s"]);
        ready(&g, 0, &["t"]);
        ready(&g, 1, &["b__t"]);
        ready(&g, 2, &["t", "t"]);
        ready(&g, 3, &["catalog", "formatter_inspect", "call_tool"]);
        fix(&g);
        assert_eq!(target(&g, "t").as_deref(), Some("c.t"));
        assert_eq!(target(&g, "b__t").as_deref(), Some("a.b__t"));
        // Both the bare and the prefixed name taken: a suffix, not a drop.
        assert_eq!(target(&g, "b__t_2").as_deref(), Some("b.t"));
        assert_eq!(target(&g, "s__catalog").as_deref(), Some("s.catalog"));
        assert_eq!(
            target(&g, "s__formatter_inspect").as_deref(),
            Some("s.formatter_inspect")
        );
        assert_eq!(target(&g, "s__call_tool").as_deref(), Some("s.call_tool"));
        // A tool listed twice by its server is one route.
        assert_eq!(g.routes.read().unwrap().list.len(), 6);
    }

    fn j(s: &str) -> J {
        J::parse(s).unwrap()
    }

    fn content(r: &J) -> String {
        r.get("content").unwrap().to_json()
    }

    #[test]
    fn formatting_replaces_every_text_block_and_keeps_the_rest() {
        let r = j(r#"{"content":[
            {"type":"text","text":"a","annotations":{"audience":["assistant"]},"_meta":{"k":1}},
            {"type":"image","mimeType":"image/png","data":"AA=="},
            {"type":"text","text":"b","annotations":{"audience":["assistant"]},"_meta":{"k":2}},
            {"type":"resource_link","uri":"file:///x","name":"x"}],
            "isError":false,"x_ext":1,"_meta":{"m":1}}"#);
        let (pl, src) = payload_src(&r);
        assert!(matches!(pl, Payload::Text(t) if t == "a\nb"));
        assert_eq!(src, Source::Texts(vec![0, 2]));
        let out = formatted_result(r.clone(), &src, "[x]\nAB".into(), false);
        assert_eq!(
            content(&out),
            r#"[{"type":"text","text":"[x]\nAB","annotations":{"audience":["assistant"]}},{"type":"image","mimeType":"image/png","data":"AA=="},{"type":"resource_link","uri":"file:///x","name":"x"}]"#
        );
        // Every other field of the result is kept, in its place.
        let keys: Vec<&str> = out
            .as_obj()
            .unwrap()
            .iter()
            .map(|(k, _)| k.as_str())
            .collect();
        assert_eq!(keys, ["content", "isError", "x_ext", "_meta"]);
    }

    #[test]
    fn annotations_the_text_blocks_disagree_on_are_not_invented() {
        let r = j(r#"{"content":[
            {"type":"text","text":"1","annotations":{"audience":["user"]}},
            {"type":"text","text":"2","annotations":{"audience":["assistant"]}}]}"#);
        let (_, src) = payload_src(&r);
        let out = formatted_result(r.clone(), &src, "t".into(), false);
        assert_eq!(content(&out), r#"[{"type":"text","text":"t"}]"#);
    }

    #[test]
    fn structured_content_takes_its_copy_along_but_not_other_text() {
        let r = j(r#"{"content":[
            {"type":"text","text":"2 items found"},
            {"type":"text","text":"{\n  \"items\": [1, 2]\n}","annotations":{"priority":1}}],
            "structuredContent":{"items":[1,2]},"_meta":{"m":1}}"#);
        let (_, src) = payload_src(&r);
        assert_eq!(src, Source::Structured);
        let out = formatted_result(r.clone(), &src, "t".into(), false);
        assert_eq!(
            content(&out),
            r#"[{"type":"text","text":"2 items found"},{"type":"text","text":"t","annotations":{"priority":1}}]"#
        );
        assert!(out.get("structuredContent").is_none());
        assert!(out.get("_meta").is_some());
        // No copy at all: the text goes first, the summary stays.
        let r = j(r#"{"content":[{"type":"text","text":"done"}],"structuredContent":{"a":1}}"#);
        let out = formatted_result(r, &Source::Structured, "t".into(), false);
        assert_eq!(
            content(&out),
            r#"[{"type":"text","text":"t"},{"type":"text","text":"done"}]"#
        );
        let r = j(r#"{"structuredContent":{"a":1}}"#);
        let out = formatted_result(r, &Source::Structured, "t".into(), false);
        assert_eq!(content(&out), r#"[{"type":"text","text":"t"}]"#);
    }

    #[test]
    fn image_meta_drops_only_the_image_it_described() {
        let r = j(r#"{"content":[
            {"type":"image","mimeType":"image/png","data":"AA=="},
            {"type":"image","mimeType":"image/gif","data":"BB=="}]}"#);
        let (_, src) = payload_src(&r);
        assert_eq!(src, Source::Image(0));
        let out = formatted_result(r.clone(), &src, "m".into(), true);
        assert_eq!(
            content(&out),
            r#"[{"type":"text","text":"m"},{"type":"image","mimeType":"image/gif","data":"BB=="}]"#
        );
        // Any other render on an image keeps the pixels.
        let out = formatted_result(r, &src, "m".into(), false);
        assert_eq!(out.get("content").unwrap().as_arr().unwrap().len(), 3);
    }

    #[test]
    fn only_one_caller_claims_a_restart() {
        let g = gw(&["a"]);
        let mut st = g.ups[0].st.lock().unwrap();
        st.phase = Phase::Ready;
        st.last_start = Some(Instant::now() - Duration::from_secs(60));
        // The first caller to find the server dead claims the restart; the
        // next one, even with no backoff at all, sees it starting.
        assert!(claim_restart(&mut st, Duration::ZERO));
        assert!(matches!(st.phase, Phase::Starting));
        assert!(!claim_restart(&mut st, Duration::ZERO));
        // After a failed start the backoff decides.
        st.phase = Phase::Failed("x".into());
        assert!(!claim_restart(&mut st, Duration::from_secs(600)));
        assert!(claim_restart(&mut st, Duration::ZERO));
    }

    #[test]
    fn protocol_versions() {
        assert_eq!(negotiate(Some("2025-03-26")), "2025-03-26");
        assert_eq!(negotiate(Some("2024-11-05")), "2024-11-05");
        assert_eq!(negotiate(Some("2099-01-01")), PROTOCOL);
        assert_eq!(negotiate(None), PROTOCOL);
        assert_eq!(
            check_upstream_protocol(&j(r#"{"protocolVersion":"2025-03-26"}"#)),
            Ok("2025-03-26")
        );
        let e = check_upstream_protocol(&j(r#"{"protocolVersion":"1999-01-01"}"#)).unwrap_err();
        assert!(
            e.contains("incompatible protocol version '1999-01-01'"),
            "{e}"
        );
        assert!(check_upstream_protocol(&j("{}")).is_err());
    }
}
