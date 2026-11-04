//! The MCP gateway: one stdio server in front of every upstream server.
//!
//! Tools keep their original names; a name already taken is exposed as
//! `<server>__<tool>` (D6: identity is the pair, nothing becomes
//! unreachable). Overrides are taken out of the arguments before the
//! upstream call. Errors, and tools without a standard, go back exactly as
//! the upstream sent them (D1); a failed standard gives the raw output plus
//! a note (D2).

use crate::capture::{Audit, Captures, Payload, payload, write_atomic, write_baseline};
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

pub const PROTOCOL: &str = "2025-06-18";

struct Conn {
    server: String,
    stdin: Mutex<ChildStdin>,
    child: Mutex<Child>,
    pending: Mutex<HashMap<u64, mpsc::Sender<Result<J, J>>>>,
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

    fn request(&self, method: &str, params: J, timeout: Duration) -> Result<J, J> {
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
        match rx.recv_timeout(timeout) {
            Ok(r) => r,
            Err(mpsc::RecvTimeoutError::Timeout) => {
                self.pending.lock().expect("pending").remove(&id);
                let _ = self.write(
                    &J::obj()
                        .with("jsonrpc", J::str("2.0"))
                        .with("method", J::str("notifications/cancelled"))
                        .with(
                            "params",
                            J::obj()
                                .with("requestId", J::int(id))
                                .with("reason", J::str("timeout")),
                        ),
                );
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

fn read_loop(conn: Arc<Conn>, out: impl BufRead, on_list_changed: impl Fn()) {
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
                // Requests from the server to its client: answer, never hang it.
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
            (Some(m), None) if m == "notifications/tools/list_changed" => on_list_changed(),
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
}

struct Upstream {
    cfg: ServerCfg,
    st: Mutex<UpState>,
    cv: Condvar,
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
            stats: Mutex::new(seed_stats(&home)),
            listed: AtomicBool::new(false),
            warned: Mutex::new(Default::default()),
            cfg,
        })
    }

    #[allow(clippy::type_complexity)]
    fn connect(
        self: &Arc<Self>,
        idx: usize,
    ) -> Result<(Arc<Conn>, Vec<J>, Option<String>), String> {
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
            read_loop(reader_conn, BufReader::new(stdout), move || {
                if let Some(gw) = weak.upgrade() {
                    thread::spawn(move || gw.refresh_tools(idx));
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
        let about = match conn.request("initialize", init, cfg.init_timeout) {
            Ok(r) => r
                .get("instructions")
                .and_then(J::as_str)
                .map(|s| crate::catalog::short(s, 100))
                .filter(|s| !s.is_empty()),
            Err(e) => {
                conn.kill();
                return Err(error_text(&e));
            }
        };
        conn.notify("notifications/initialized");
        match list_tools(&conn, cfg.init_timeout) {
            Ok(t) => Ok((conn, t, about)),
            Err(e) => {
                conn.kill();
                Err(error_text(&e))
            }
        }
    }

    fn start_upstream(self: &Arc<Self>, idx: usize) {
        let up = &self.ups[idx];
        {
            let mut st = up.st.lock().expect("state");
            st.phase = Phase::Starting;
            st.last_start = Some(Instant::now());
        }
        let res = self.connect(idx);
        {
            let mut st = up.st.lock().expect("state");
            match res {
                Ok((conn, tools, about)) => {
                    st.phase = Phase::Ready;
                    st.conn = Some(conn);
                    st.tools = tools;
                    st.about = about;
                }
                Err(e) => {
                    eprintln!("[toolfmt] server '{}' failed to start: {e}", up.cfg.name);
                    st.phase = Phase::Failed(e);
                    st.conn = None;
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
        let can_restart = !matches!(st.phase, Phase::Starting)
            && st
                .last_start
                .is_none_or(|t| t.elapsed() >= up.cfg.restart_backoff);
        drop(st);
        if can_restart {
            self.start_upstream(idx);
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

    fn rebuild_routes(&self) {
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
                let mut exposed = name.to_string();
                if r.by_name.contains_key(&exposed)
                    || [meta::INSPECT, meta::SAVE, catalog::CATALOG, catalog::CALL]
                        .contains(&exposed.as_str())
                {
                    exposed = format!("{}__{}", up.cfg.name, name);
                }
                if r.by_name.contains_key(&exposed) {
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
        let (Some(method), Some(id)) = (method, id) else {
            return;
        };
        match method.as_str() {
            "initialize" => {
                let v = params
                    .get("protocolVersion")
                    .and_then(J::as_str)
                    .unwrap_or(PROTOCOL)
                    .to_string();
                self.out.respond(
                    &id,
                    J::obj()
                        .with("protocolVersion", J::Str(v))
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
                        .with("instructions", J::str(meta::INSTRUCTIONS)),
                );
            }
            "ping" | "logging/setLevel" => self.out.respond(&id, J::obj()),
            "tools/list" => {
                let g = self.clone();
                thread::spawn(move || g.tools_list(id));
            }
            "tools/call" => {
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

    /// Waits (bounded) for servers still starting, then makes sure the
    /// routes reflect every server that is ready by now.
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
        self.rebuild_routes();
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
        tools.extend(meta::defs());
        self.out.respond(&id, J::obj().with("tools", J::Arr(tools)));
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
                return self.out.respond(
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
            return self.out.respond(&id, tool_text(self.catalog(&args)));
        }
        if name == meta::INSPECT {
            self.wait_startup();
            return self.out.respond(&id, tool_text(self.inspect(&args)));
        }
        if name == meta::SAVE {
            self.wait_startup();
            return self.out.respond(&id, tool_text(self.save(&args)));
        }
        let lookup = || {
            let r = self.routes.read().expect("routes");
            r.by_name.get(&name).map(|&i| r.list[i].clone())
        };
        let route = lookup().or_else(|| {
            // A call can arrive before slow servers finished starting.
            self.wait_startup();
            lookup()
        });
        let Some(route) = route else {
            if via_call {
                let hint = catalog::search(&self.entries(), &name);
                return self.out.respond(
                    &id,
                    text_result(format!("unknown tool '{name}'. {hint}"), true),
                );
            }
            return self
                .out
                .error(&id, error_obj(-32602, format!("unknown tool '{name}'")));
        };
        let ov = Overrides::extract(&mut args, &route.declared);
        if via_call {
            // No client checked these arguments against the schema: say what
            // is missing before the upstream fails less helpfully.
            let schema = route.def.get("inputSchema");
            let missing = catalog::missing(schema, &args);
            if !missing.is_empty() {
                let (sig, _) = catalog::signature(schema);
                return self.out.respond(
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
            Err(e) => return self.out.error(&id, e),
        };
        let timeout = self.ups[route.up].cfg.timeout;
        let call = J::obj()
            .with("name", J::str(&route.tool))
            .with("arguments", args.clone());
        let t0 = Instant::now();
        match conn.request("tools/call", call, timeout) {
            Err(e) => self.out.error(&id, e),
            Ok(result) => {
                let r = self.process(&route, &args, ov, result, t0);
                self.out.respond(&id, r);
            }
        }
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
        let outcome: Option<Outcome> = {
            let pl = payload(&result);
            let data = match &pl {
                Payload::Text(t) => Some(Data::Text(t)),
                Payload::Json(j) => Some(Data::Json(j)),
                Payload::None => None,
            };
            data.map(|data| {
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
            })
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
        let mut result = result;
        let blocks = match result.remove("content") {
            Some(J::Arr(b)) => b,
            _ => Vec::new(),
        };
        let header = out.header.clone().unwrap_or_default();
        match out.body {
            Body::Untouched => {
                result.set("content", J::Arr(blocks));
                result
            }
            Body::Fallback(_) => {
                let mut content = vec![text_block(header)];
                content.extend(blocks);
                result.set("content", J::Arr(content));
                result
            }
            Body::Formatted(text) => {
                let image = out.used.starts_with(Render::ImageMeta.name());
                let mut content = vec![text_block(format!("{header}\n{text}"))];
                for b in blocks {
                    let kind = b.get("type").and_then(J::as_str).unwrap_or("");
                    if kind != "text" && !(image && kind == "image") {
                        content.push(b);
                    }
                }
                let mut r = J::obj().with("content", J::Arr(content));
                if let Some(m) = result.remove("_meta") {
                    r.set("_meta", m);
                }
                r
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
        let routes = self.routes.read().expect("routes");
        if let Some(&i) = routes.by_name.get(name) {
            let r = routes.list[i].clone();
            return Ok((self.ups[r.up].cfg.name.clone(), r.tool.clone(), Some(r)));
        }
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
                Phase::Ready => format!("{} tools", st.tools.len()),
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
        let mut report = String::new();
        let mut budget = 6_000usize;
        let mut rows = Vec::new();
        for (i, f) in caps.iter().enumerate() {
            let Some((o, raw)) = run_fixture(Some(&self.store), Some(&self.cfg), f, Some(&std))
            else {
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
            }
            report.push('\n');
            let p = preview(&agent, budget.clamp(200, 3_000));
            budget = budget.saturating_sub(p.chars().count());
            report.push_str(&p);
            report.push_str("\n\n");
            rows.push((i, clean_text(&o, &raw), o.used));
        }
        if dry {
            report.insert_str(
                0,
                &format!("dry run, not saved. Replay on {} captures:\n", caps.len()),
            );
            return Ok(report.trim_end().to_string());
        }
        write_atomic(&path, yaml).map_err(|e| format!("cannot write {}: {e}", path.display()))?;
        self.store.invalidate(&path);
        for (i, text, used) in rows {
            let _ = write_baseline(&caps[i], &text, &used);
        }
        report.insert_str(
            0,
            &format!(
                "saved {} (applies from the next call; baselines recorded in {} captures). Replay:\n",
                path.display(),
                caps.len()
            ),
        );
        Ok(report.trim_end().to_string())
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
