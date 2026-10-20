use any_txt_parser::limits::CliOptions;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(
    name = "any_txt_parser",
    version = any_txt_parser::PARSER_VERSION,
    about = "Data extraction and project navigation for AI agents. This is the report mode: source -> template -> self-describing Markdown file",
    after_help = "Commands: extract, recon, map, symbols, find, lines, search, todos, changes, nav, mcp, help.
Start with: any_txt_parser help"
)]
struct Cli {
    #[arg(
        short = 'i',
        long = "input",
        required = true,
        value_name = "PATH",
        help = "input file, directory (recursive), or glob pattern (* and **/); the -o output file is never scanned"
    )]
    input: String,

    #[arg(
        short = 't',
        long = "template",
        required = true,
        value_name = "FILE",
        help = "YAML template (version: 1) with extraction rules - see HELP.md"
    )]
    template: std::path::PathBuf,

    #[arg(
        short = 'o',
        long = "output",
        value_name = "FILE",
        help = "output Markdown file (required at runtime; missing -o -> exit 3)"
    )]
    output: Option<std::path::PathBuf>,

    #[arg(
        long = "format",
        value_name = "FMT",
        help = "force format for all input files: json|xml|html|markdown|yaml|toml|text (not 'auto'); overrides template input.format and extension detection"
    )]
    format: Option<String>,

    #[arg(
        long = "encoding",
        value_name = "ENC",
        help = "utf-8, utf-8-sig, utf-16le, utf-16 (=LE), utf-16be, windows-1251/cp1251; overrides template input.encoding"
    )]
    encoding: Option<String>,

    #[arg(
        long = "dry-run",
        help = "validate template and inputs, print a plan with per-file format hints; publishes nothing; exit 0"
    )]
    dry_run: bool,

    #[arg(
        short = 'v',
        long = "verbose",
        help = "diagnostics to stderr (versions, files, formats, counters)"
    )]
    verbose: bool,

    #[arg(
        short = 'q',
        long = "quiet",
        help = "suppress the final stderr message"
    )]
    quiet: bool,

    #[arg(
        long = "follow-symlinks",
        help = "traverse symlinks during discovery (default: skipped)"
    )]
    follow_symlinks: bool,

    #[arg(long = "hash", help = "include sha256 of each source in the output")]
    hash: bool,

    #[arg(
        long = "max-file-size",
        value_name = "SIZE",
        help = "max size of one input file (default 200m; SIZE: bare bytes or k/m/g x1024, e.g. 50m)"
    )]
    max_file_size: Option<String>,

    #[arg(
        long = "max-memory",
        value_name = "SIZE",
        help = "cumulative decoded UTF-8 byte budget across the run (default: unlimited; not a process RSS limit)"
    )]
    max_memory: Option<String>,

    #[arg(
        long = "max-files",
        value_name = "N",
        help = "max number of discovered files (default 5000)"
    )]
    max_files: Option<u64>,

    #[arg(
        long = "max-matches",
        value_name = "N",
        help = "cap matches per rule (resource limit)"
    )]
    max_matches: Option<u64>,

    #[arg(
        long = "max-output-size",
        value_name = "SIZE",
        help = "max final document size in characters (default: unlimited)"
    )]
    max_output_size: Option<String>,

    #[arg(
        long = "max-block-chars",
        value_name = "N",
        help = "hard per-block character cap (default 32000); oversized items are split"
    )]
    max_block_chars: Option<u64>,
}

fn main() {
    // Installed first and everything runs guarded, so any panic, even while
    // parsing arguments, exits with 5.
    std::panic::set_hook(Box::new(|info| {
        eprintln!(
            "any_txt_parser: internal error (exit 5), please report it: {}",
            info
        );
    }));
    let code = any_txt_parser::run::run_guarded(run_cli);
    // `exit` skips destructors; flush anything still buffered (dry-run plan).
    let _ = std::io::Write::flush(&mut std::io::stdout());
    std::process::exit(code);
}

fn run_cli() -> i32 {
    // `args_os`: an argument that is not valid Unicode must reach clap (exit
    // 3, or accepted as a path) instead of panicking in `std::env::args`.
    let raw_args: Vec<String> = std::env::args_os()
        .skip(1)
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect();
    if raw_args.is_empty() {
        eprint!("{}", any_txt_parser::commands::usage());
        return any_txt_parser::limits::ExitCode::InvalidInputOrCli.to_i32();
    }
    if let Some(code) = any_txt_parser::commands::dispatch(&raw_args) {
        return code;
    }
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(error) => {
            let code = if matches!(
                error.kind(),
                clap::error::ErrorKind::DisplayHelp | clap::error::ErrorKind::DisplayVersion
            ) {
                0
            } else {
                3
            };
            let _ = error.print();
            return code;
        }
    };
    let options = CliOptions {
        input: cli.input,
        template: cli.template,
        output: cli.output,
        format: cli.format,
        encoding: cli.encoding,
        dry_run: cli.dry_run,
        verbose: cli.verbose,
        quiet: cli.quiet,
        follow_symlinks: cli.follow_symlinks,
        hash: cli.hash,
        max_file_size: cli.max_file_size,
        max_memory: cli.max_memory,
        max_files: cli.max_files,
        max_matches: cli.max_matches,
        max_output_size: cli.max_output_size,
        max_block_chars: cli.max_block_chars,
        raw_args,
    };
    any_txt_parser::run::execute(options)
}
