//! MCP tools: the parser and navguard modules exposed to AI clients, plus
//! the built-in help. Each handler takes the JSON arguments of a
//! `tools/call` and returns compact text (or JSON with `output: "json"`).

use crate::api;
use crate::mcp::{Resource, Server, Tool, args};
use crate::navigation;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// Default cap on returned item characters for MCP `extract`, so one call
/// cannot flood the model's context; the response says where it stopped.
pub const MCP_DEFAULT_MAX_OUTPUT_CHARS: usize = 50_000;

const INSTRUCTIONS: &str = "any_txt_parser gives you exact data from files and a map of code projects without reading whole files. \
Typical flow: `recon` the files you were given (format, encoding, shape, sample lines) -> `extract` with rules such as \
\"contains:ERROR\", \"jsonpath:$.items[*].id\", \"css:a[href]\" -> read only what you need. For code: `map` -> `symbols` or \
`find_symbol` -> `read_lines`. Resources `project://tree` (the map of the server root) and `project://state` (whether \
navigator.md and its deep files match the files on disk) need no arguments. Call `help` (topics: overview, extract, rules, \
recon, navigation, mcp, reference) for syntax and examples.";

pub fn server() -> Server {
    let mut tools = vec![help_tool(), recon_tool(), extract_tool()];
    tools.extend(crate::tools::navigation_tools());
    Server {
        name: "any_txt_parser",
        version: crate::PARSER_VERSION,
        instructions: INSTRUCTIONS,
        tools,
        resources: resources(),
    }
}

/// The server's resources; both read the server root, which is the working
/// directory (as for tools without `root`; the canonical [`server_root`]
/// is a verbatim `\\?\` path on Windows, where `/` does not separate).
fn resources() -> Vec<Resource> {
    vec![
        Resource {
            uri: "project://tree",
            name: "tree",
            title: "Project tree",
            description: "The tree of the server root and every file with its line count and token estimate \
                          (the map tool without arguments, at most 1000 files, root shown absolute).",
            mime_type: "text/plain",
            reader: || navigation::tree(Path::new(".")),
        },
        Resource {
            uri: "project://state",
            name: "state",
            title: "Project freshness",
            description: "JSON: root, checked (now), totals of the files on disk; navigators.status = fresh \
                          (navigator.md, navigator_deep.md, navigator_deep2.md come from one scan and match every \
                          file), stale (with new/changed/deleted counts and paths), incomplete or missing (run \
                          nav --oneshot); snapshot = time of the changes snapshot; workflow = nav --triangle state.",
            mime_type: "application/json",
            reader: || navigation::state(Path::new(".")),
        },
    ]
}

/// The folder every tool is confined to (canonical); set once by
/// [`serve_stdio`].
static SERVER_ROOT: OnceLock<PathBuf> = OnceLock::new();

/// The server root: every path a tool reads, and every `root` argument,
/// must lie inside it. The working directory when [`serve_stdio`] did not
/// set one (in-process use).
pub fn server_root() -> PathBuf {
    SERVER_ROOT
        .get()
        .cloned()
        .unwrap_or_else(|| PathBuf::from("."))
}

#[derive(clap::Parser, Debug)]
#[command(
    about = "Run as an MCP server on stdio. Every tool works inside the server root: paths and \
             'root' arguments outside it are refused",
    after_help = "Example client entry: {\"command\": \"C:\\\\tools\\\\any_txt_parser.exe\", \"args\": [\"mcp\", \"--root\", \"C:\\\\work\\\\shop\"]}"
)]
struct McpArgs {
    /// Folder the tools may read (default: the working directory). The
    /// server works from there, so relative paths start in it.
    #[arg(long, value_name = "DIR")]
    root: Option<PathBuf>,
}

/// `any_txt_parser mcp [--root DIR]`.
pub fn serve_stdio(args: &[String]) -> i32 {
    let args: McpArgs = match crate::commands::parse("mcp", args) {
        Ok(args) => args,
        Err(code) => return code,
    };
    if let Some(root) = &args.root
        && let Err(e) = std::env::set_current_dir(root)
    {
        eprintln!(
            "any_txt_parser mcp: cannot use --root {}: {}",
            root.display(),
            e
        );
        return crate::limits::ExitCode::InvalidInputOrCli.to_i32();
    }
    match crate::confine::canonical(Path::new(".")) {
        Some(root) => {
            let _ = SERVER_ROOT.set(root);
        }
        None => {
            eprintln!("any_txt_parser mcp: cannot resolve the working directory");
            return crate::limits::ExitCode::InvalidInputOrCli.to_i32();
        }
    }
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    server().serve(stdin.lock(), stdout.lock())
}

fn output_json(arguments: &Value) -> Result<bool, String> {
    match args::optional_string(arguments, "output")?.as_deref() {
        None | Some("text") => Ok(false),
        Some("json") => Ok(true),
        Some(other) => Err(format!(
            "argument 'output' must be \"text\" or \"json\", got \"{}\"",
            other
        )),
    }
}

/// The arguments without the presentation-only keys, for strict decoding.
fn request_fields(arguments: &Value, extra: &[&str]) -> Value {
    let mut map = arguments.as_object().cloned().unwrap_or_default();
    for key in extra {
        map.remove(*key);
    }
    Value::Object(map)
}

fn api_error(error: api::ApiError) -> String {
    format!("{} (exit code {})", error.message, error.exit_code())
}

const OUTPUT_PROPERTY: &str = "output";

fn output_schema() -> Value {
    json!({"type": "string", "enum": ["text", "json"], "description": "text (default, compact) or json"})
}

fn help_tool() -> Tool {
    Tool {
        name: "help",
        title: "Help",
        description: "Built-in documentation with examples. Topics: overview, extract, rules, recon, navigation, mcp, reference.",
        input_schema: json!({
            "type": "object",
            "properties": {
                "topic": {"type": "string", "description": "overview (default), extract, rules, recon, navigation, mcp or reference"}
            }
        }),
        handler: |arguments| {
            let topic = args::optional_string(arguments, "topic")?;
            crate::help::topic(topic.as_deref()).map(str::to_string)
        },
    }
}

fn recon_tool() -> Tool {
    Tool {
        name: "recon",
        title: "Inspect files",
        description: "Describe files before extracting: size, encoding, detected format and why, which rule types apply, \
line count, a shape hint (top-level keys, XML root, HTML headings, Markdown headings) and the first lines. Extracts nothing.",
        input_schema: json!({
            "type": "object",
            "properties": {
                "path": {"type": "string", "description": "file, directory (recursive) or glob"},
                "sample_lines": {"type": "integer", "minimum": 0, "description": "lines shown per file (default 20)"},
                "encoding": {"type": "string", "description": "decode with this encoding instead of detecting"},
                "max_files": {"type": "integer", "minimum": 1, "description": "files described (default 50)"},
                "output": output_schema()
            },
            "required": ["path"]
        }),
        handler: |arguments| {
            let json = output_json(arguments)?;
            let mut request: api::ReconRequest =
                serde_json::from_value(request_fields(arguments, &[OUTPUT_PROPERTY]))
                    .map_err(|e| format!("invalid recon arguments: {}", e))?;
            request.confine_to = Some(server_root());
            let response = api::recon(&request).map_err(api_error)?;
            Ok(if json {
                api::to_json(&response)
            } else {
                api::render_recon_text(&response)
            })
        },
    }
}

fn extract_tool() -> Tool {
    Tool {
        name: "extract",
        title: "Extract data",
        description: "Extract exactly the data you need from files (json, yaml, toml, xml, html, markdown, text/logs). \
Give `input` and `rules`; each rule is a shorthand string TYPE:EXPR (contains, starts_with, ends_with, regex, jsonpath, \
path, xpath, css, line, markdown) or a rule object. Returns per file and rule the matched items, statuses and what was cut; \
items of text rules carry their 1-based source line (`L12: ` in text, `lines` in JSON). \
Returned item text is capped (max_output_chars, default 50000) and the response says where it stopped. See help topic 'rules'.",
        input_schema: json!({
            "type": "object",
            "properties": {
                "input": {"type": "string", "description": "file, directory (recursive) or glob"},
                "rules": {
                    "type": "array",
                    "description": "rules r1, r2, ... in order",
                    "items": {"oneOf": [
                        {"type": "string", "description": "TYPE:EXPR, e.g. contains:ERROR, regex:code=(\\d+), jsonpath:$.users[*].name, path:[database].host, xpath://item/@id, css:a[href], line:10-20, markdown:heading"},
                        {"type": "object", "description": "a template rule: {select: {type, expression}, extract, transforms, limits, context, when, output, id}"}
                    ]}
                },
                "template": {"type": "string", "description": "a whole template as YAML or JSON text, instead of rules"},
                "template_path": {"type": "string", "description": "a template file, instead of rules"},
                "format": {"type": "string", "description": "force json|xml|html|markdown|yaml|toml|text for every file"},
                "encoding": {"type": "string", "description": "utf-8 (default), utf-8-sig, utf-16le, utf-16be, windows-1251"},
                "context_before": {"type": "integer", "minimum": 0, "description": "text rules: lines before each match"},
                "context_after": {"type": "integer", "minimum": 0, "description": "text rules: lines after each match"},
                "extract": {"type": "string", "description": "html/xml: text | attr:NAME | inner_html | outer_html"},
                "transforms": {"type": "array", "items": {}, "description": "in order, e.g. [\"trim\", \"unique\", \"limit_items: 20\"]"},
                "max_items": {"type": "integer", "minimum": 1},
                "max_item_chars": {"type": "integer", "minimum": 1},
                "max_output_chars": {"type": "integer", "minimum": 1, "description": "cap on returned item characters (default 50000)"},
                "include_empty": {"type": "boolean", "description": "also list file/rule results that found nothing (default: left out, counted in a note)"},
                "max_files": {"type": "integer", "minimum": 1},
                "max_file_size": {"type": "string", "description": "e.g. 50m"},
                "max_matches": {"type": "integer", "minimum": 1},
                "max_memory": {"type": "string"},
                "follow_symlinks": {"type": "boolean"},
                "hash": {"type": "boolean"},
                "output": output_schema()
            },
            "required": ["input"]
        }),
        handler: |arguments| {
            let json = output_json(arguments)?;
            let mut request: api::ExtractRequest =
                serde_json::from_value(request_fields(arguments, &[OUTPUT_PROPERTY]))
                    .map_err(|e| format!("invalid extract arguments: {}", e))?;
            if request.max_output_chars.is_none() {
                request.max_output_chars = Some(MCP_DEFAULT_MAX_OUTPUT_CHARS);
            }
            request.confine_to = Some(server_root());
            let response = api::extract(&request).map_err(api_error)?;
            Ok(if json {
                api::to_json(&response)
            } else {
                api::render_text(&response)
            })
        },
    }
}

fn root_schema() -> Value {
    json!({"type": "string", "description": "project root inside the server root (default: the server root); paths are relative to it"})
}

fn root_arg(a: &Value) -> Result<PathBuf, String> {
    navigation::confined_root(args::optional_string(a, "root")?.as_deref(), &server_root())
}

fn cap(a: &Value, name: &str, default: usize) -> Result<usize, String> {
    Ok(args::optional_u64(a, name)?.map_or(default, |n| n as usize))
}

/// Navigation tools (navguard module).
pub fn navigation_tools() -> Vec<Tool> {
    vec![
        Tool {
            name: "map",
            title: "Project map",
            description: "Project tree and every file with line count and token estimate (skips .git, target, node_modules, __pycache__, .zvec-grep and navguard's own files; a navguard.json in the root can change that). Start here to see what a project contains.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "root": root_schema(),
                    "max_files": {"type": "integer", "minimum": 1, "description": "files listed (default 1000); totals always cover all"},
                    "output": output_schema()
                }
            }),
            handler: |a| {
                let json = output_json(a)?;
                let root = root_arg(a)?;
                let max = cap(a, "max_files", navigation::MCP_MAX_FILES)?;
                navigation::map(&root, Some(max), json)
            },
        },
        Tool {
            name: "symbols",
            title: "Symbols with line ranges",
            description: "Functions, methods, classes, structs, traits, interfaces, ... of one file (path) or of the whole project, with start-end lines, nesting and kind. Use the ranges with read_lines.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "root": root_schema(),
                    "path": {"type": "string", "description": "a file (relative to root or absolute); omit for the whole project"},
                    "output": output_schema()
                }
            }),
            handler: |a| {
                let json = output_json(a)?;
                let path = args::optional_string(a, "path")?;
                let root = root_arg(a)?;
                navigation::symbols(&root, path.as_deref(), json)
            },
        },
        Tool {
            name: "find_symbol",
            title: "Find symbols",
            description: "Symbols whose name or file path contains the pattern (case-insensitive by default), with file and line range. exact: true returns only symbols named exactly pattern (find a definition, not every name that contains it).",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "root": root_schema(),
                    "pattern": {"type": "string"},
                    "case_sensitive": {"type": "boolean"},
                    "exact": {"type": "boolean", "description": "the symbol name equals pattern (for Foo::bar also bar); label and path do not count (default false: name, label or path contains pattern)"},
                    "max_hits": {"type": "integer", "minimum": 1, "description": "default 200"},
                    "output": output_schema()
                },
                "required": ["pattern"]
            }),
            handler: |a| {
                let json = output_json(a)?;
                let root = root_arg(a)?;
                let pattern = args::string(a, "pattern")?;
                let case = args::optional_bool(a, "case_sensitive")?.unwrap_or(false);
                let exact = args::optional_bool(a, "exact")?.unwrap_or(false);
                let max = cap(a, "max_hits", navigation::MCP_MAX_HITS)?;
                navigation::find(&root, &pattern, case, exact, Some(max), json)
            },
        },
        Tool {
            name: "read_lines",
            title: "Read lines",
            description: "Numbered lines start..end (1-based, inclusive) of a file; end defaults to start. At most 2000 lines per call.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "root": root_schema(),
                    "path": {"type": "string"},
                    "start": {"type": "integer", "minimum": 1},
                    "end": {"type": "integer", "minimum": 1},
                    "output": output_schema()
                },
                "required": ["path", "start"]
            }),
            handler: |a| {
                let json = output_json(a)?;
                let path = args::string(a, "path")?;
                let root = root_arg(a)?;
                let start = args::optional_u64(a, "start")?
                    .ok_or("missing required argument 'start'")?
                    as usize;
                let end = args::optional_u64(a, "end")?.map(|n| n as usize);
                navigation::lines(
                    &root,
                    &path,
                    start,
                    end,
                    Some(navigation::MCP_MAX_LINES),
                    json,
                )
            },
        },
        Tool {
            name: "search_text",
            title: "Search text",
            description: "Case-sensitive text search over the project (or one file), returning file, line and the text around each match: the whole line in code and data files (one hit per line), the sentence in prose (.md, .txt, .rst, ...).",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "root": root_schema(),
                    "pattern": {"type": "string"},
                    "path": {"type": "string", "description": "search only this file"},
                    "max_hits": {"type": "integer", "minimum": 1, "description": "default 200"},
                    "context": {"type": "string", "enum": ["auto", "line", "sentence"], "description": "text around a match: auto (default: line in code, sentence in prose), line or sentence"},
                    "output": output_schema()
                },
                "required": ["pattern"]
            }),
            handler: |a| {
                let json = output_json(a)?;
                let path = args::optional_string(a, "path")?;
                let root = root_arg(a)?;
                let pattern = args::string(a, "pattern")?;
                let max = cap(a, "max_hits", navigation::MCP_MAX_HITS)?;
                let context = args::optional_string(a, "context")?;
                navigation::search(
                    &root,
                    &pattern,
                    path.as_deref(),
                    Some(max),
                    context.as_deref(),
                    json,
                )
            },
        },
        Tool {
            name: "todos",
            title: "TODO list",
            description: "TODO, FIXME, HACK and XXX marker lines across the project: the marker as a whole word in capitals, or in any case followed by : ( ! or after @ (todo: x, @todo, todo!()); words such as todos or TODO_LIMIT do not count.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "root": root_schema(),
                    "loose": {"type": "boolean", "description": "old matching: TODO, FIXME or HACK anywhere in a line, any case (noisy: counts todos, todo_list)"},
                    "max_hits": {"type": "integer", "minimum": 1, "description": "default 200"},
                    "output": output_schema()
                }
            }),
            handler: |a| {
                let json = output_json(a)?;
                let root = root_arg(a)?;
                let loose = args::optional_bool(a, "loose")?.unwrap_or(false);
                let max = cap(a, "max_hits", navigation::MCP_MAX_HITS)?;
                navigation::todos(&root, loose, Some(max), json)
            },
        },
        Tool {
            name: "changes",
            title: "Changes since snapshot",
            description: "Files new, changed or deleted since the last snapshot of the project. With persist: true the current state becomes the new snapshot (.navguard_cache.json in the root); without it nothing is written.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "root": root_schema(),
                    "persist": {"type": "boolean", "description": "save the current state as the snapshot (default false)"},
                    "output": output_schema()
                }
            }),
            handler: |a| {
                let json = output_json(a)?;
                let root = root_arg(a)?;
                let persist = args::optional_bool(a, "persist")?.unwrap_or(false);
                navigation::changes(&root, persist, json)
            },
        },
    ]
}

#[derive(clap::Parser, Debug)]
#[command(about = "Project tree and files with line counts and token estimates")]
struct MapArgs {
    /// Project root (default: current directory).
    #[arg(long)]
    root: Option<String>,
    /// List at most N files (totals still cover all).
    #[arg(long = "max-files", value_name = "N")]
    max_files: Option<usize>,
    /// Print JSON instead of text.
    #[arg(long)]
    json: bool,
}

#[derive(clap::Parser, Debug)]
#[command(about = "Functions, classes, ... with line ranges, of one file or the whole project")]
struct SymbolsArgs {
    /// A file; omit for the whole project.
    path: Option<String>,
    /// Project root (default: current directory); PATH must lie inside it.
    #[arg(long)]
    root: Option<String>,
    /// Print JSON instead of text.
    #[arg(long)]
    json: bool,
}

#[derive(clap::Parser, Debug)]
#[command(
    about = "Find symbols whose name or path contains PATTERN (case-insensitive); --exact: named PATTERN"
)]
struct FindArgs {
    pattern: String,
    /// Project root (default: current directory).
    #[arg(long)]
    root: Option<String>,
    #[arg(long = "case-sensitive")]
    case_sensitive: bool,
    /// The symbol name equals PATTERN (for Foo::bar also bar); label and path do not count.
    #[arg(long)]
    exact: bool,
    #[arg(long = "max-hits", value_name = "N")]
    max_hits: Option<usize>,
    /// Print JSON instead of text.
    #[arg(long)]
    json: bool,
}

#[derive(clap::Parser, Debug)]
#[command(about = "Print lines START..END (1-based, inclusive) of a file")]
struct LinesArgs {
    path: String,
    start: usize,
    end: Option<usize>,
    /// Project root (default: current directory); PATH must lie inside it.
    #[arg(long)]
    root: Option<String>,
    /// Print JSON instead of text.
    #[arg(long)]
    json: bool,
}

#[derive(clap::Parser, Debug)]
#[command(
    about = "Case-sensitive text search: the line around each match in code, the sentence in prose"
)]
struct SearchArgs {
    pattern: String,
    /// Search only this file.
    path: Option<String>,
    /// Project root (default: current directory).
    #[arg(long)]
    root: Option<String>,
    #[arg(long = "max-hits", value_name = "N")]
    max_hits: Option<usize>,
    /// Text around a match: auto (the line in code and data files, the
    /// sentence in .md/.txt/.rst/... prose), line or sentence.
    #[arg(long, value_name = "MODE", value_parser = ["auto", "line", "sentence"])]
    context: Option<String>,
    /// Print JSON instead of text.
    #[arg(long)]
    json: bool,
}

#[derive(clap::Parser, Debug)]
#[command(
    about = "TODO / FIXME / HACK / XXX marker lines across the project (whole words: not todos, TODO_LIMIT)"
)]
struct TodosArgs {
    /// Project root (default: current directory).
    #[arg(long)]
    root: Option<String>,
    /// Old matching: TODO, FIXME or HACK anywhere in a line, any case (counts todos, todo_list).
    #[arg(long)]
    loose: bool,
    #[arg(long = "max-hits", value_name = "N")]
    max_hits: Option<usize>,
    /// Print JSON instead of text.
    #[arg(long)]
    json: bool,
}

#[derive(clap::Parser, Debug)]
#[command(
    about = "Files new / changed / deleted since the last snapshot (--persist saves a new one)"
)]
struct ChangesArgs {
    /// Project root (default: current directory).
    #[arg(long)]
    root: Option<String>,
    /// Save the current state as the snapshot (.navguard_cache.json in the root).
    #[arg(long)]
    persist: bool,
    /// Print JSON instead of text.
    #[arg(long)]
    json: bool,
}

/// `any_txt_parser map|symbols|find|lines|search|todos|changes ...`.
pub fn navigation_command(command: &str, args: &[String]) -> i32 {
    use crate::commands::parse;
    use navigation::resolve_root;
    let result = match command {
        "map" => parse::<MapArgs>(command, args)
            .map(|a| navigation::map(&resolve_root(a.root.as_deref()), a.max_files, a.json)),
        "symbols" => parse::<SymbolsArgs>(command, args).map(|a| {
            let root = resolve_root(a.root.as_deref());
            navigation::symbols(&root, a.path.as_deref(), a.json)
        }),
        "find" => parse::<FindArgs>(command, args).map(|a| {
            let root = resolve_root(a.root.as_deref());
            navigation::find(
                &root,
                &a.pattern,
                a.case_sensitive,
                a.exact,
                a.max_hits,
                a.json,
            )
        }),
        "lines" => parse::<LinesArgs>(command, args).map(|a| {
            let root = resolve_root(a.root.as_deref());
            navigation::lines(&root, &a.path, a.start, a.end, None, a.json)
        }),
        "search" => parse::<SearchArgs>(command, args).map(|a| {
            let root = resolve_root(a.root.as_deref());
            navigation::search(
                &root,
                &a.pattern,
                a.path.as_deref(),
                a.max_hits,
                a.context.as_deref(),
                a.json,
            )
        }),
        "todos" => parse::<TodosArgs>(command, args).map(|a| {
            let root = resolve_root(a.root.as_deref());
            navigation::todos(&root, a.loose, a.max_hits, a.json)
        }),
        "changes" => parse::<ChangesArgs>(command, args)
            .map(|a| navigation::changes(&resolve_root(a.root.as_deref()), a.persist, a.json)),
        other => {
            eprintln!("any_txt_parser: unknown command '{}'", other);
            return crate::limits::ExitCode::InvalidInputOrCli.to_i32();
        }
    };
    match result {
        Err(code) => code,
        Ok(Ok(text)) => {
            print!("{}", text);
            0
        }
        Ok(Err(message)) => {
            eprintln!("any_txt_parser: {}", message);
            crate::limits::ExitCode::InvalidInputOrCli.to_i32()
        }
    }
}
