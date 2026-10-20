//! The command line of the single executable. `any_txt_parser <command> ...`
//! runs one module tool and prints its result on stdout; anything that is not
//! a command is the original `-i/-t/-o` report mode (see `run`).

use crate::api;
use crate::limits::ExitCode;
use clap::Parser;
use std::path::PathBuf;

/// Commands and their one-line purpose, as shown by `help` and on errors.
pub const COMMANDS: &[(&str, &str)] = &[
    ("extract", "pull matching data out of files (parser)"),
    (
        "recon",
        "inspect files: format, encoding, shape, sample lines (parser)",
    ),
    (
        "map",
        "project tree and files with line/token estimates (navguard)",
    ),
    (
        "symbols",
        "functions, classes, ... with line ranges (navguard)",
    ),
    ("find", "find symbols by name (navguard)"),
    ("lines", "print a numbered line range of a file (navguard)"),
    (
        "search",
        "text search: the line in code, the sentence in prose (navguard)",
    ),
    ("todos", "TODO / FIXME / HACK / XXX marker lines (navguard)"),
    (
        "changes",
        "files new / changed / deleted since the last snapshot (navguard)",
    ),
    (
        "nav",
        "the full navguard command line (watch, oneshot, check, patch, ...)",
    ),
    ("mcp", "run as an MCP server on stdio"),
    ("help", "built-in documentation: help [topic]"),
];

/// Runs `args` (without the program name) when they start with a command.
/// `None` means the arguments are for the report mode.
pub fn dispatch(args: &[String]) -> Option<i32> {
    let (command, rest) = args.split_first()?;
    Some(match command.as_str() {
        "extract" => extract(rest),
        "recon" => recon(rest),
        "help" | "--help-topics" => help(rest),
        "mcp" => crate::tools::serve_stdio(rest),
        "nav" if rest.is_empty() => {
            // The standalone tool watched the current folder for hours when
            // started without arguments; here a mode is required.
            eprintln!(
                "any_txt_parser nav: give a mode, e.g. nav --oneshot --root DIR, nav --check, nav --watch --timeout 600 \
                 (all modes: any_txt_parser help navigation)"
            );
            ExitCode::InvalidInputOrCli.to_i32()
        }
        "nav" => navguard::cli::run(rest, &format!("any_txt_parser {}", crate::PARSER_VERSION)),
        "map" | "symbols" | "find" | "lines" | "search" | "todos" | "changes" => {
            crate::tools::navigation_command(command, rest)
        }
        _ => return None,
    })
}

/// Text printed when the program runs without arguments.
pub fn usage() -> String {
    let mut out = format!(
        "any_txt_parser {} - data extraction and project navigation for AI agents\n\n\
         Usage: any_txt_parser <command> [options]   (any_txt_parser help <command|topic>)\n\n",
        crate::PARSER_VERSION
    );
    for (name, about) in COMMANDS {
        out.push_str(&format!("  {:<9} {}\n", name, about));
    }
    out.push_str(
        "\nReport mode (self-describing Markdown file): any_txt_parser -i <input> -t <template.yml> -o <out.md>\n\
         Start with: any_txt_parser help\n",
    );
    out
}

/// Parses command arguments with clap; `Err` carries the exit code after
/// clap printed its message (0 for --help, 3 for usage errors).
pub(crate) fn parse<T: Parser>(command: &str, args: &[String]) -> Result<T, i32> {
    let argv = std::iter::once(format!("any_txt_parser {}", command)).chain(args.iter().cloned());
    T::try_parse_from(argv).map_err(|error| {
        let code = match error.kind() {
            clap::error::ErrorKind::DisplayHelp | clap::error::ErrorKind::DisplayVersion => 0,
            _ => ExitCode::InvalidInputOrCli.to_i32(),
        };
        let _ = error.print();
        code
    })
}

/// Prints an API error: always on stderr, and as a JSON object on stdout
/// when the caller asked for JSON.
fn report_error(error: &api::ApiError, json: bool) -> i32 {
    eprintln!("any_txt_parser: {}", error.message);
    if json {
        println!("{}", api::to_json(error));
    }
    error.exit_code()
}

#[derive(Parser, Debug)]
#[command(
    about = "Pull matching data out of files and print it (text, or JSON with --json)",
    after_help = "Examples (more: any_txt_parser help extract, help rules):\n  \
        any_txt_parser extract -i logs/app.log -r contains:ERROR --context-after 1\n  \
        any_txt_parser extract -i data.json -r 'jsonpath:$.users[?(@.active == true)].email' --json\n  \
        any_txt_parser extract -i page.html -r 'css:a[href]' --extract attr:href --transform unique\n  \
        any_txt_parser extract -i Cargo.toml -r 'path:[package].version'"
)]
struct ExtractArgs {
    /// File, directory (recursive) or glob pattern.
    #[arg(short = 'i', long = "input", value_name = "PATH")]
    input: String,
    /// Rule, repeatable: TYPE:EXPR shorthand or a {...} rule object (help rules).
    #[arg(short = 'r', long = "rule", value_name = "RULE")]
    rules: Vec<String>,
    /// A YAML template file instead of -r rules.
    #[arg(short = 't', long = "template", value_name = "FILE")]
    template: Option<PathBuf>,
    /// Force one format: json|xml|html|markdown|yaml|toml|text (default: detect per file).
    #[arg(long, value_name = "FMT")]
    format: Option<String>,
    /// Source encoding: utf-8 (default), utf-8-sig, utf-16le, utf-16be, windows-1251.
    #[arg(long, value_name = "ENC")]
    encoding: Option<String>,
    /// Neighbour lines before and after each text match (-r rules).
    #[arg(long, value_name = "N")]
    context: Option<usize>,
    /// Neighbour lines before each text match (-r rules).
    #[arg(long = "context-before", value_name = "N")]
    context_before: Option<usize>,
    /// Neighbour lines after each text match (-r rules).
    #[arg(long = "context-after", value_name = "N")]
    context_after: Option<usize>,
    /// text | attr:NAME | inner_html | outer_html (html/xml; -r rules).
    #[arg(long, value_name = "MODE")]
    extract: Option<String>,
    /// Transform, repeatable, in order: trim, unique, sort, 'limit_items: 20', ... (-r rules).
    #[arg(long = "transform", value_name = "T")]
    transforms: Vec<String>,
    /// Keep at most N items per rule and file (-r rules).
    #[arg(long = "max-items", value_name = "N")]
    max_items: Option<u64>,
    /// Cut each item to N characters (-r rules).
    #[arg(long = "max-item-chars", value_name = "N")]
    max_item_chars: Option<u64>,
    /// Cap on the characters of all returned items; the output says where it stopped.
    #[arg(long = "max-output-chars", value_name = "N")]
    max_output_chars: Option<usize>,
    /// Max discovered files (default 5000).
    #[arg(long = "max-files", value_name = "N")]
    max_files: Option<u64>,
    /// Max size of one file, e.g. 50m (default 200m).
    #[arg(long = "max-file-size", value_name = "SIZE")]
    max_file_size: Option<String>,
    /// Match cap per rule, before transforms.
    #[arg(long = "max-matches", value_name = "N")]
    max_matches: Option<u64>,
    /// Cumulative decoded-text budget across all files, e.g. 1g.
    #[arg(long = "max-memory", value_name = "SIZE")]
    max_memory: Option<String>,
    /// Add the sha256 of each source file.
    #[arg(long)]
    hash: bool,
    /// Traverse symlinks during discovery.
    #[arg(long = "follow-symlinks")]
    follow_symlinks: bool,
    /// Also list results that found nothing (left out and counted by default).
    #[arg(long = "include-empty")]
    include_empty: bool,
    /// Print JSON instead of text.
    #[arg(long)]
    json: bool,
}

fn extract(args: &[String]) -> i32 {
    let args: ExtractArgs = match parse("extract", args) {
        Ok(args) => args,
        Err(code) => return code,
    };
    let request = api::ExtractRequest {
        input: args.input,
        template_path: args.template,
        rules: args
            .rules
            .into_iter()
            .map(api::RuleSpec::Shorthand)
            .collect(),
        format: args.format,
        encoding: args.encoding,
        follow_symlinks: args.follow_symlinks,
        hash: args.hash,
        max_files: args.max_files,
        max_file_size: args.max_file_size,
        max_matches: args.max_matches,
        max_memory: args.max_memory,
        context_before: args.context_before.or(args.context),
        context_after: args.context_after.or(args.context),
        extract: args.extract,
        transforms: args.transforms,
        max_items: args.max_items,
        max_item_chars: args.max_item_chars,
        max_output_chars: args.max_output_chars,
        include_empty: args.include_empty,
        ..Default::default()
    };
    match api::extract(&request) {
        Ok(response) => {
            if args.json {
                println!("{}", api::to_json(&response));
            } else {
                print!("{}", api::render_text(&response));
            }
            response.exit_code
        }
        Err(error) => report_error(&error, args.json),
    }
}

#[derive(Parser, Debug)]
#[command(
    about = "Inspect files before extracting: format, encoding, shape and sample lines",
    after_help = "Examples:\n  any_txt_parser recon logs/\n  any_txt_parser recon data.json --sample 5 --json"
)]
struct ReconArgs {
    /// File, directory (recursive) or glob pattern.
    #[arg(value_name = "PATH", required_unless_present = "input")]
    path: Option<String>,
    /// Same as PATH.
    #[arg(
        short = 'i',
        long = "input",
        value_name = "PATH",
        conflicts_with = "path"
    )]
    input: Option<String>,
    /// Sample lines per file (default 20).
    #[arg(long = "sample", value_name = "N")]
    sample_lines: Option<usize>,
    /// Decode with this encoding instead of detecting.
    #[arg(long, value_name = "ENC")]
    encoding: Option<String>,
    /// Report at most N files (default 50).
    #[arg(long = "max-files", value_name = "N")]
    max_files: Option<usize>,
    /// Print JSON instead of text.
    #[arg(long)]
    json: bool,
}

fn recon(args: &[String]) -> i32 {
    let args: ReconArgs = match parse("recon", args) {
        Ok(args) => args,
        Err(code) => return code,
    };
    let mut request = api::ReconRequest {
        path: args.path.or(args.input).unwrap_or_default(),
        encoding: args.encoding,
        ..Default::default()
    };
    if let Some(n) = args.sample_lines {
        request.sample_lines = n;
    }
    if let Some(n) = args.max_files {
        request.max_files = n;
    }
    match api::recon(&request) {
        Ok(response) => {
            if args.json {
                println!("{}", api::to_json(&response));
            } else {
                print!("{}", api::render_recon_text(&response));
            }
            0
        }
        Err(error) => report_error(&error, args.json),
    }
}

fn help(args: &[String]) -> i32 {
    let topic = args.first().map(String::as_str);
    // `help extract` etc.: a command name is also a topic, or maps to one.
    let topic = match topic {
        Some("map" | "symbols" | "find" | "lines" | "search" | "todos" | "changes" | "nav") => {
            Some("navigation")
        }
        Some("topics" | "--help" | "-h") => {
            print!("{}", crate::help::index());
            return 0;
        }
        other => other,
    };
    match crate::help::topic(topic) {
        Ok(text) => {
            print!("{}", text);
            if !text.ends_with('\n') {
                println!();
            }
            0
        }
        Err(message) => {
            eprintln!("any_txt_parser: {}", message);
            ExitCode::InvalidInputOrCli.to_i32()
        }
    }
}
