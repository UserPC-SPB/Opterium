//! upstreams.json: the upstream server registry.
//!
//! Accepts the usual MCP client layout (`{"mcpServers": {...}}`, also
//! `servers`), so an existing client config can be pasted as is. Server
//! order is the file order: on a tool-name collision the first server keeps
//! the bare name, unless the client already saw the names (D6).

use crate::json::J;
use glob::Pattern;
use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Clone, Debug)]
pub struct ServerCfg {
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
    pub cwd: Option<PathBuf>,
    /// Built-in auto mode for this server (lossless layouts only).
    pub auto: bool,
    /// Store the first real outputs of each tool as fixtures.
    pub capture: bool,
    /// Tool-name glob -> family standard, first match wins.
    pub families: Vec<(Pattern, String)>,
    pub timeout: Duration,
    pub init_timeout: Duration,
    /// Minimum time between two restarts of a dead server.
    pub restart_backoff: Duration,
    /// Catalog section of this server's tools (`math/cube`); default: its name.
    pub section: String,
    /// Tool-name glob -> subsection inside `section`, first match wins.
    pub groups: Vec<(Pattern, String)>,
    /// Tool-name glob -> declared price (free text, e.g. "$0.04/call").
    pub prices: Vec<(Pattern, String)>,
}

impl ServerCfg {
    fn first<'a>(list: &'a [(Pattern, String)], tool: &str) -> Option<&'a str> {
        list.iter()
            .find(|(p, _)| p.matches(tool))
            .map(|(_, v)| v.as_str())
    }

    pub fn group_of(&self, tool: &str) -> Option<&str> {
        Self::first(&self.groups, tool)
    }

    pub fn price_of(&self, tool: &str) -> Option<&str> {
        Self::first(&self.prices, tool)
    }
}

/// How the upstream tools are offered to the agent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Expose {
    /// Every tool in `tools/list`.
    All,
    /// Only `catalog` + `call_tool` (+ pinned tools): schemas on demand.
    Catalog,
    /// Catalog once there are more tools than `catalog_threshold`.
    Auto,
}

#[derive(Clone, Debug)]
pub struct Config {
    pub servers: Vec<ServerCfg>,
    pub capture_limit: usize,
    pub audit: bool,
    /// How long `tools/list` waits for slow servers before answering.
    pub startup_wait: Duration,
    pub expose: Expose,
    pub catalog_threshold: usize,
    /// Tools (globs over listed names) kept in `tools/list` in catalog mode.
    pub pin: Vec<Pattern>,
    /// Section path -> description shown in the catalog.
    pub sections: Vec<(String, String)>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            servers: Vec::new(),
            capture_limit: 5,
            audit: true,
            startup_wait: Duration::from_secs(30),
            expose: Expose::Auto,
            catalog_threshold: 30,
            pin: Vec::new(),
            sections: Vec::new(),
        }
    }
}

fn secs(v: Option<&J>, default: u64) -> Duration {
    Duration::from_secs(v.and_then(J::as_u64).unwrap_or(default))
}

fn glob_map(v: Option<&J>, what: &str, server: &str) -> Result<Vec<(Pattern, String)>, String> {
    let mut out = Vec::new();
    if let Some(m) = v.and_then(J::as_obj) {
        for (glob, val) in m {
            let pat = Pattern::new(glob)
                .map_err(|e| format!("server '{server}': {what} glob '{glob}': {e}"))?;
            out.push((pat, val.text()));
        }
    }
    Ok(out)
}

/// A section path: `/`-separated, no empty parts.
fn section_path(s: &str) -> String {
    s.split(['/', '\\'])
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>()
        .join("/")
}

impl Config {
    pub fn load(path: &Path) -> Result<Config, String> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
        let base = path.parent().unwrap_or(Path::new("."));
        Config::parse(&text, base).map_err(|e| format!("{}: {e}", path.display()))
    }

    pub fn parse(text: &str, base: &Path) -> Result<Config, String> {
        let j = J::parse(text.trim_start_matches('\u{feff}'))?;
        let mut cfg = Config::default();
        let defaults = j.get("defaults");
        let d_bool = |k: &str, dv: bool| {
            defaults
                .and_then(|d| d.get(k))
                .and_then(J::as_bool)
                .or_else(|| j.get(k).and_then(J::as_bool))
                .unwrap_or(dv)
        };
        let auto_all = d_bool("auto", false);
        let capture_all = d_bool("capture", true);
        cfg.audit = d_bool("audit", true);
        if let Some(n) = j.get("capture_limit").and_then(J::as_u64) {
            cfg.capture_limit = n as usize;
        }
        cfg.startup_wait = secs(j.get("startup_wait"), 30);
        cfg.expose = match j.get("expose").and_then(J::as_str) {
            None | Some("auto") => Expose::Auto,
            Some("all") => Expose::All,
            Some("catalog") => Expose::Catalog,
            Some(o) => return Err(format!("expose '{o}': expected all, catalog or auto")),
        };
        if let Some(n) = j.get("catalog_threshold").and_then(J::as_u64) {
            cfg.catalog_threshold = n as usize;
        }
        if let Some(a) = j.get("pin").and_then(J::as_arr) {
            for p in a {
                let t = p.text();
                cfg.pin
                    .push(Pattern::new(&t).map_err(|e| format!("pin '{t}': {e}"))?);
            }
        }
        if let Some(m) = j.get("sections").and_then(J::as_obj) {
            for (k, v) in m {
                cfg.sections.push((section_path(k), v.text()));
            }
        }
        let servers = j
            .get("mcpServers")
            .or_else(|| j.get("servers"))
            .and_then(J::as_obj)
            .ok_or("expected an object 'mcpServers' (or 'servers')")?;
        for (name, s) in servers {
            if s.get("disabled").and_then(J::as_bool).unwrap_or(false) {
                continue;
            }
            if name.contains("__") {
                return Err(format!("server name '{name}' must not contain '__'"));
            }
            let command = s
                .get("command")
                .and_then(J::as_str)
                .ok_or_else(|| {
                    format!(
                        "server '{name}': 'command' is required (only stdio servers are supported)"
                    )
                })?
                .to_string();
            let args = s
                .get("args")
                .and_then(J::as_arr)
                .map(|a| a.iter().map(J::text).collect())
                .unwrap_or_default();
            let env = s
                .get("env")
                .and_then(J::as_obj)
                .map(|m| m.iter().map(|(k, v)| (k.clone(), v.text())).collect())
                .unwrap_or_default();
            let cwd = s.get("cwd").and_then(J::as_str).map(|c| {
                let p = PathBuf::from(c);
                if p.is_absolute() { p } else { base.join(p) }
            });
            let mut families = Vec::new();
            if let Some(f) = s.get("families").and_then(J::as_obj) {
                for (glob, fam) in f {
                    let pat = Pattern::new(glob)
                        .map_err(|e| format!("server '{name}': family glob '{glob}': {e}"))?;
                    families.push((pat, fam.text()));
                }
            }
            if let Some(f) = s.get("family").and_then(J::as_str) {
                families.push((Pattern::new("*").expect("glob"), f.to_string()));
            }
            let groups = glob_map(s.get("groups"), "group", name)?
                .into_iter()
                .map(|(p, g)| (p, section_path(&g)))
                .collect();
            let mut prices = glob_map(s.get("prices"), "price", name)?;
            if let Some(p) = s.get("price") {
                prices.push((Pattern::new("*").expect("glob"), p.text()));
            }
            let section = s
                .get("section")
                .and_then(J::as_str)
                .map(section_path)
                .filter(|p| !p.is_empty())
                .unwrap_or_else(|| name.clone());
            cfg.servers.push(ServerCfg {
                name: name.clone(),
                command,
                args,
                env,
                cwd,
                auto: s.get("auto").and_then(J::as_bool).unwrap_or(auto_all),
                capture: s.get("capture").and_then(J::as_bool).unwrap_or(capture_all),
                families,
                timeout: secs(s.get("timeout"), 600),
                init_timeout: secs(s.get("init_timeout"), 60),
                restart_backoff: secs(s.get("restart_backoff"), 10),
                section,
                groups,
                prices,
            });
        }
        Ok(cfg)
    }

    pub fn server(&self, name: &str) -> Option<&ServerCfg> {
        self.servers.iter().find(|s| s.name == name)
    }

    pub fn family_of(&self, server: &str, tool: &str) -> Option<String> {
        self.server(server)?
            .families
            .iter()
            .find(|(p, _)| p.matches(tool))
            .map(|(_, f)| f.clone())
    }

    pub fn auto_for(&self, server: &str) -> bool {
        self.server(server).is_some_and(|s| s.auto)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_client_layout_in_order() {
        let cfg = Config::parse(
            r#"{"mcpServers":{
                "zeta":{"command":"z","args":["-x",1],"families":{"cubint_*":"cube_arith"}},
                "alpha":{"command":"a","auto":true,"env":{"K":"V"}},
                "off":{"command":"o","disabled":true}}}"#,
            Path::new("."),
        )
        .unwrap();
        let names: Vec<&str> = cfg.servers.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, ["zeta", "alpha"]);
        assert_eq!(cfg.servers[0].args, ["-x", "1"]);
        assert_eq!(
            cfg.family_of("zeta", "cubint_add").as_deref(),
            Some("cube_arith")
        );
        assert_eq!(cfg.family_of("zeta", "vec_norm"), None);
        assert!(cfg.auto_for("alpha") && !cfg.auto_for("zeta"));
    }
}
