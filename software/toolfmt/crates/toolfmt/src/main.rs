use clap::{Parser, Subcommand};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use toolfmt::config::Config;
use toolfmt::json::J;
use toolfmt::pipeline::{self, Data, Overrides, Request};
use toolfmt::standard::{Store, compile};

#[derive(Parser)]
#[command(
    name = "toolfmt",
    version,
    about = "Universal tool-output formatter and MCP gateway: each tool's output reaches the agent in the format chosen once, per tool, in a standard"
)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(clap::Args, Clone)]
struct HomeArgs {
    /// Directory with standards/, fixtures/ and upstreams.json
    /// (default: $TOOLFMT_HOME, else the directory of --config, else the exe's directory)
    #[arg(long)]
    home: Option<PathBuf>,
    /// Upstream registry (default: <home>/upstreams.json)
    #[arg(long)]
    config: Option<PathBuf>,
}

#[derive(Subcommand)]
enum Cmd {
    /// Run the MCP gateway on stdin/stdout (register this in the MCP client)
    Gateway {
        #[command(flatten)]
        home: HomeArgs,
    },
    /// Serve the revision-2 HTTP contract: POST /clean on 127.0.0.1
    Serve {
        #[command(flatten)]
        home: HomeArgs,
        #[arg(long, default_value_t = 8787)]
        port: u16,
        /// Enable built-in auto mode for every request
        #[arg(long)]
        auto: bool,
    },
    /// Read one /clean request (JSON) on stdin, print the response JSON
    Clean {
        #[command(flatten)]
        home: HomeArgs,
        #[arg(long)]
        auto: bool,
    },
    /// Format one payload (FILE or stdin) and print what the agent would see
    Format {
        #[command(flatten)]
        home: HomeArgs,
        #[arg(long, default_value = "")]
        server: String,
        #[arg(long, default_value = "")]
        tool: String,
        /// Call arguments as JSON (for expression standards)
        #[arg(long)]
        args: Option<String>,
        /// Use this standard file instead of the lookup
        #[arg(long)]
        standard: Option<PathBuf>,
        #[arg(long)]
        auto: bool,
        file: Option<PathBuf>,
    },
    /// Replay every fixture against its baseline (exit 1 on any difference)
    Replay {
        #[command(flatten)]
        home: HomeArgs,
        /// Record the current output as the new baselines
        #[arg(long)]
        update: bool,
    },
    /// Validate standards (all of <home>/standards, or the given files)
    Check {
        #[command(flatten)]
        home: HomeArgs,
        files: Vec<PathBuf>,
    },
    /// A demo MCP server with typical output shapes (for trying standards)
    DemoServer {
        /// Add failure tools for tests: crash, sleep, noise, ask_client, grow, big
        #[arg(long)]
        test_modes: bool,
        /// Split tools/list into pages of this many tools (0: one page)
        #[arg(long, default_value_t = 0)]
        page_size: usize,
    },
}

fn resolve_home(h: &HomeArgs) -> PathBuf {
    if let Some(p) = &h.home {
        return p.clone();
    }
    if let Some(p) = std::env::var_os("TOOLFMT_HOME") {
        return PathBuf::from(p);
    }
    if let Some(c) = &h.config {
        if let Some(d) = c.parent().filter(|d| !d.as_os_str().is_empty()) {
            return d.to_path_buf();
        }
        return PathBuf::from(".");
    }
    std::env::current_exe()
        .ok()
        .and_then(|e| e.parent().map(Path::to_path_buf))
        .unwrap_or_else(|| PathBuf::from("."))
}

fn load_config(h: &HomeArgs, home: &Path, required: bool) -> Result<Option<Config>, String> {
    let path = h
        .config
        .clone()
        .unwrap_or_else(|| home.join("upstreams.json"));
    if !path.is_file() {
        if required {
            return Err(format!(
                "no upstream registry at {} (create it: {{\"mcpServers\": {{\"name\": {{\"command\": ..., \"args\": [...]}}}}}})",
                path.display()
            ));
        }
        return Ok(None);
    }
    Config::load(&path).map(Some)
}

fn read_input(file: Option<&Path>) -> Result<String, String> {
    match file {
        Some(p) => std::fs::read_to_string(p).map_err(|e| format!("{}: {e}", p.display())),
        None => {
            let mut s = String::new();
            std::io::stdin()
                .read_to_string(&mut s)
                .map_err(|e| e.to_string())?;
            Ok(s)
        }
    }
}

fn run(cli: Cli) -> Result<ExitCode, String> {
    match cli.cmd {
        Cmd::Gateway { home } => {
            let dir = resolve_home(&home);
            let cfg = load_config(&home, &dir, true)?.unwrap_or_default();
            let code = toolfmt::gateway::serve(cfg, &dir);
            // Threads may still wait on upstreams; the client has gone.
            std::process::exit(code);
        }
        Cmd::Serve { home, port, auto } => {
            let dir = resolve_home(&home);
            let cfg = load_config(&home, &dir, false)?;
            toolfmt::http::serve(port, &dir, cfg, auto)?;
            Ok(ExitCode::SUCCESS)
        }
        Cmd::Clean { home, auto } => {
            let dir = resolve_home(&home);
            let cfg = load_config(&home, &dir, false)?;
            let body = J::parse(read_input(None)?.trim())?;
            let server = body.get("server").and_then(J::as_str).unwrap_or("");
            let tool = body.get("tool").and_then(J::as_str).unwrap_or("");
            let family = cfg.as_ref().and_then(|c| c.family_of(server, tool));
            let auto = auto || cfg.as_ref().is_some_and(|c| c.auto_for(server));
            let store = Store::new(dir);
            println!(
                "{}",
                pipeline::clean(Some(&store), &body, auto, family).to_json()
            );
            Ok(ExitCode::SUCCESS)
        }
        Cmd::Format {
            home,
            server,
            tool,
            args,
            standard,
            auto,
            file,
        } => {
            let dir = resolve_home(&home);
            let cfg = load_config(&home, &dir, false)?;
            let data = read_input(file.as_deref())?;
            let args = match args {
                Some(a) => J::parse(&a)?,
                None => J::obj(),
            };
            let fixed = match &standard {
                Some(p) => {
                    let text =
                        std::fs::read_to_string(p).map_err(|e| format!("{}: {e}", p.display()))?;
                    Some(compile(&text, &p.display().to_string(), &tool)?)
                }
                None => None,
            };
            let store = Store::new(dir);
            let req = Request {
                server: &server,
                tool: &tool,
                args: &args,
                data: Data::Text(&data),
                keys: None,
                ov: Overrides::default(),
                auto: auto || cfg.as_ref().is_some_and(|c| c.auto_for(&server)),
                family: cfg.as_ref().and_then(|c| c.family_of(&server, &tool)),
            };
            let out = pipeline::run(Some(&store), &req, fixed.as_ref());
            println!("{}", out.agent_text(&data));
            eprintln!(
                "[{}] {} -> {} chars ({})",
                out.used,
                out.raw_chars,
                out.out_chars,
                out.loss.name()
            );
            Ok(ExitCode::SUCCESS)
        }
        Cmd::Replay { home, update } => {
            let dir = resolve_home(&home);
            let cfg = load_config(&home, &dir, false)?;
            let rep = toolfmt::replay::replay(&dir, cfg.as_ref(), update);
            for f in &rep.failed {
                println!("FAIL {f}");
            }
            let total = rep.green + rep.failed.len();
            println!(
                "replay: {}/{} green{}{}",
                rep.green,
                total,
                if rep.unbaselined > 0 {
                    format!(", {} without baseline", rep.unbaselined)
                } else {
                    String::new()
                },
                if rep.updated > 0 {
                    format!(", {} baselines recorded", rep.updated)
                } else {
                    String::new()
                },
            );
            Ok(if rep.failed.is_empty() {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            })
        }
        Cmd::Check { home, files } => {
            let dir = resolve_home(&home);
            let files = if files.is_empty() {
                let mut v = Vec::new();
                for d in [dir.join("standards"), dir.join("standards").join("family")] {
                    if let Ok(rd) = std::fs::read_dir(&d) {
                        for e in rd.flatten() {
                            let p = e.path();
                            if p.extension().is_some_and(|x| x == "yml" || x == "yaml") {
                                v.push(p);
                            }
                        }
                    }
                }
                v.sort();
                v
            } else {
                files
            };
            let mut bad = 0;
            for p in &files {
                let tool = p
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .and_then(|s| s.split_once('.').map(|(_, t)| t.to_string()))
                    .unwrap_or_default();
                let res = std::fs::read_to_string(p)
                    .map_err(|e| e.to_string())
                    .and_then(|t| compile(&t, &p.display().to_string(), &tool));
                match res {
                    Ok(s) => println!("ok   {} ({})", p.display(), s.render.name()),
                    Err(e) => {
                        bad += 1;
                        println!("FAIL {}: {e}", p.display());
                    }
                }
            }
            println!("check: {} standards, {bad} invalid", files.len());
            Ok(if bad == 0 {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            })
        }
        Cmd::DemoServer {
            test_modes,
            page_size,
        } => Ok(ExitCode::from(
            toolfmt::demo::serve(test_modes, page_size) as u8
        )),
    }
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("toolfmt: {e}");
            ExitCode::from(2)
        }
    }
}
