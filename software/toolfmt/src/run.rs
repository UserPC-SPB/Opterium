use crate::collect::{FileStatus, process_file};
use crate::discovery::{self, DiscoveredFile};
use crate::limits::{CliOptions, ExitCode, ResourceLimits};
use crate::materialize;
use crate::publish;
use crate::render::{self, RunMeta};
use crate::template::{CompiledTemplate, Format, load_template_with_overrides, validate_encoding};

pub fn execute(cli: CliOptions) -> i32 {
    match run(&cli) {
        Err(RunError { code, message }) => {
            eprintln!("any_txt_parser: {}", message);
            code.to_i32()
        }
        Ok(code) => code.to_i32(),
    }
}

/// Stack for the worker thread. Third-party parsers (toml_edit, serde_json,
/// serde_yaml) recurse up to their own nesting limits; debug builds need far
/// more than the 1 MiB Windows main-thread stack for that.
pub const WORKER_STACK_BYTES: usize = 64 * 1024 * 1024;

/// Runs `job` on a worker thread with a large stack. A panic becomes exit
/// code 5 (internal fatal error) instead of the runtime's 101.
pub fn run_guarded<F>(job: F) -> i32
where
    F: FnOnce() -> i32 + Send + 'static,
{
    let worker = std::thread::Builder::new()
        .name("any_txt_parser".to_string())
        .stack_size(WORKER_STACK_BYTES)
        .spawn(job);
    match worker {
        Ok(handle) => handle
            .join()
            .unwrap_or_else(|_| ExitCode::InternalFatal.to_i32()),
        Err(e) => {
            eprintln!(
                "any_txt_parser: internal error: cannot start worker thread: {}",
                e
            );
            ExitCode::InternalFatal.to_i32()
        }
    }
}

#[derive(Debug)]
pub struct RunError {
    pub code: ExitCode,
    pub message: String,
}

fn err(code: ExitCode, message: impl Into<String>) -> RunError {
    RunError {
        code,
        message: message.into(),
    }
}

fn started_iso() -> String {
    chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

pub fn run(cli: &CliOptions) -> Result<ExitCode, RunError> {
    let started = started_iso();

    if cli.verbose && !cli.quiet {
        eprintln!("any_txt_parser v{}", render::PARSER_VERSION);
    }

    if !cli.template.is_file() {
        return Err(err(
            ExitCode::InvalidInputOrCli,
            format!("template file not found: '{}'", cli.template.display()),
        ));
    }

    // Validate CLI overrides up front so malformed flag values fail with exit
    // code 3 (InvalidInputOrCli) before any file work is done.
    validate_cli_overrides(cli)?;

    let template = load_template_with_overrides(
        &cli.template,
        cli.format.as_deref(),
        cli.encoding.as_deref(),
    )
    .map_err(|e| err(ExitCode::InvalidTemplate, e.message))?;
    if cli.verbose && !cli.quiet {
        eprintln!("template '{}' v{} valid", template.name, template.version);
        for note in never_applying_rules(&template) {
            eprintln!("  {}", note);
        }
    }

    let mut effective = template;
    // `--format` / `--encoding` are already baked into `effective` at compile
    // time (precedence: CLI > template > default); only block sizing is left.
    apply_cli_overrides(&mut effective, cli);

    let limits = build_limits(cli, &effective);

    let out_path = cli
        .output
        .clone()
        .ok_or_else(|| err(ExitCode::InvalidInputOrCli, "missing required --output"))?;

    let files = discover_inputs(&cli.input, Some(&out_path), cli.follow_symlinks, &limits)?;
    if cli.verbose && !cli.quiet {
        eprintln!("discovered {} file(s)", files.len());
    }

    if cli.dry_run {
        print_dry_run(&effective, &files, &limits);
        return Ok(ExitCode::Success);
    }

    let generation = next_generation(&out_path);
    let Collection {
        file_results,
        files_success,
        files_warning,
        files_error,
        nothing_applies,
    } = collect_sources(
        &files,
        &effective,
        &limits,
        cli.hash,
        cli.verbose && !cli.quiet,
    )?;

    let mut mat = materialize::materialize(&file_results, &effective)
        .map_err(|message| err(ExitCode::ResourceLimit, message))?;
    if let Some(note) = &nothing_applies {
        mat.errors.insert(0, note.clone());
    }

    let (run_status, exit_code) = run_outcome(files_success, files_warning, files_error);

    let completed = started_iso();
    let meta = RunMeta {
        generation: generation.clone(),
        started: started.clone(),
        completed: completed.clone(),
        input_pattern: cli.input.clone(),
        output_path: out_path.to_string_lossy().to_string(),
        files_discovered: files.len(),
        files_processed: file_results.len(),
        files_success,
        files_warning,
        files_error,
        run_status,
        format_label: effective
            .format
            .as_ref()
            .map(|f| f.name().to_string())
            .unwrap_or_else(|| "auto".to_string()),
        format_source: effective.format_source.clone(),
        encoding_label: effective.encoding.clone(),
        exit_code: exit_code.to_i32(),
        files: file_results
            .iter()
            .map(|f| render::FileInfo {
                path: f.path.to_string_lossy().to_string(),
                size: f.size,
                mtime: f.mtime_iso.clone(),
                encoding: f.encoding.clone(),
                format: f.format.name().to_string(),
                format_source: f.format_source.clone(),
                status: match f.status {
                    FileStatus::Success => "success",
                    FileStatus::Warning => "warning",
                    FileStatus::Error => "error",
                },
                hash: f.hash.clone(),
            })
            .collect(),
    };

    if cli.verbose && !cli.quiet {
        eprintln!(
            "collected {} item(s) from {} file(s), materializing...",
            mat.total_items,
            file_results.len()
        );
    }

    let assembled = render::assemble(&effective, cli, &meta, &mut mat);
    if let Some(max) = limits.max_output_chars {
        if assembled.document.chars().count() as u64 > max {
            return Err(err(
                ExitCode::ResourceLimit,
                format!(
                    "output exceeds max_output_chars: {} characters, limit {} (including metadata)",
                    assembled.document.chars().count(),
                    max
                ),
            ));
        }
    }

    validate(&assembled.document, &mat.blocks).map_err(|m| err(ExitCode::OutputFailure, m))?;

    publish::atomic_write(&out_path, &assembled.document)
        .map_err(|e| err(ExitCode::OutputFailure, e.message))?;

    if !cli.quiet {
        eprintln!(
            "published '{}': generation {}, blocks {}, chars {}",
            out_path.display(),
            meta.generation,
            mat.blocks.len(),
            assembled.document.chars().count()
        );
        eprintln!("triangle: [0,1,0] READY");
        if let Some(note) = &nothing_applies {
            eprintln!("any_txt_parser: warning: {}", note);
        }
    }
    if run_status == "failure" {
        eprintln!(
            "any_txt_parser: no input file could be processed ({} of {} failed); the errors are listed under Diagnostics in '{}'",
            files_error,
            file_results.len(),
            out_path.display()
        );
    }
    Ok(exit_code)
}

/// Discovers the input files of a run (file, directory or glob). An empty
/// result is an input error (exit 3). `exclude` is a path never scanned (the
/// published document).
pub(crate) fn discover_inputs(
    input: &str,
    exclude: Option<&std::path::Path>,
    follow_symlinks: bool,
    limits: &ResourceLimits,
) -> Result<Vec<DiscoveredFile>, RunError> {
    let files =
        discovery::discover(input, exclude, follow_symlinks, limits, false).map_err(|e| {
            RunError {
                code: e.exit,
                message: e.message,
            }
        })?;
    if files.is_empty() {
        return Err(err(
            ExitCode::InvalidInputOrCli,
            format!("no input files matched '{}'", input),
        ));
    }
    Ok(files)
}

/// Every source of a run read, decoded, detected and processed by the
/// template, with the file status counters `run_outcome` takes.
pub(crate) struct Collection {
    pub file_results: Vec<crate::collect::FileResult>,
    pub files_success: usize,
    pub files_warning: usize,
    pub files_error: usize,
    /// The run-level diagnostic when no rule can apply at all.
    pub nothing_applies: Option<String>,
}

/// The collection phase shared by the published document (`run`) and the
/// in-memory API (`crate::api`): sources are processed one at a time, in
/// discovery order. A resource limit aborts the whole run (exit 6); a source
/// that cannot be read or decoded becomes a file error. With `verbose`, the
/// per-file format and errors go to stderr.
pub(crate) fn collect_sources(
    files: &[DiscoveredFile],
    effective: &CompiledTemplate,
    limits: &ResourceLimits,
    hash: bool,
    verbose: bool,
) -> Result<Collection, RunError> {
    let nothing_applies = no_rule_applies_note(effective);

    let mut file_results = Vec::new();
    let mut files_success = 0usize;
    let mut files_warning = 0usize;
    let mut files_error = 0usize;
    let mut decoded_bytes = 0u64;
    // Process one source at a time. The decoded UTF-8 budget is cumulative
    // across the run, rather than being reset for each source.
    for f in files {
        let decoded = match discovery::read_decoded(
            &f.path,
            &effective.encoding,
            limits.max_file_size_bytes,
            hash,
        ) {
            Ok(d) => d,
            Err(e) if e.resource_limit => {
                return Err(err(ExitCode::ResourceLimit, e.message));
            }
            Err(e) => {
                files_error += 1;
                file_results.push(crate::collect::FileResult {
                    path: f.path.clone(),
                    size: f.size,
                    mtime_iso: f.mtime_iso.clone(),
                    hash: f.hash.clone(),
                    format: effective
                        .format
                        .clone()
                        .or_else(|| discovery::extension_hint(&f.path))
                        .unwrap_or(Format::Text),
                    format_source: effective.format_source.clone(),
                    encoding: effective.encoding.clone(),
                    status: FileStatus::Error,
                    error: Some(e.message),
                    rule_results: Vec::new(),
                });
                continue;
            }
        };
        decoded_bytes = decoded_bytes
            .checked_add(decoded.text.len() as u64)
            .ok_or_else(|| err(ExitCode::ResourceLimit, "decoded-text byte count overflow"))?;
        if let Some(max_mem) = limits.max_memory_bytes {
            if decoded_bytes > max_mem {
                return Err(err(
                    ExitCode::ResourceLimit,
                    format!(
                        "decoded-text budget exceeded at '{}': {} bytes across the run, limit {}",
                        f.path.display(),
                        decoded_bytes,
                        max_mem
                    ),
                ));
            }
        }
        let (fmt, src) = match &effective.format {
            Some(fmt) => (fmt.clone(), effective.format_source.clone()),
            None => discovery::detect_format(&f.path, &decoded.text),
        };
        if verbose {
            eprintln!("  {} -> format {} ({})", f.path.display(), fmt.name(), src);
        }
        let mut source = f.clone();
        source.hash = decoded.hash.clone();
        let mut fr = process_file(
            &source,
            effective,
            &decoded.text,
            &decoded.encoding,
            fmt,
            src,
            limits.max_matches_per_rule,
        );
        if nothing_applies.is_some() && fr.status == FileStatus::Success {
            // No rule could run on this file, nor on any other (see the
            // run-level diagnostic): the result is empty, not complete.
            fr.status = FileStatus::Warning;
        }
        match fr.status {
            FileStatus::Success => files_success += 1,
            FileStatus::Warning => files_warning += 1,
            FileStatus::Error => files_error += 1,
        }
        if let Some(e) = &fr.error {
            if verbose {
                eprintln!("  {}: ERROR {}", fr.path.display(), e);
            }
        }
        file_results.push(fr);
    }
    Ok(Collection {
        file_results,
        files_success,
        files_warning,
        files_error,
        nothing_applies,
    })
}

/// One note per rule whose `when.format` differs from the input format
/// fixed for every file (by `--format` or `input.format`): such rules are
/// skipped for the whole run.
pub(crate) fn never_applying_rules(template: &CompiledTemplate) -> Vec<String> {
    template
        .rules
        .iter()
        .filter(|rule| render::rule_never_applies(template, rule))
        .map(|rule| {
            format!(
                "Rule '{}' never applies: when.format {} does not match the input format {} (format source: {})",
                rule.id,
                rule.when_format.as_ref().map_or("-", |f| f.name()),
                template.format.as_ref().map_or("-", |f| f.name()),
                template.format_source
            )
        })
        .collect()
}

/// The run-level diagnostic when not a single rule can apply, because the
/// fixed input format excludes every rule by its `when.format`. Such a run
/// extracts nothing by construction, so it must not look like a success.
pub(crate) fn no_rule_applies_note(template: &CompiledTemplate) -> Option<String> {
    render::no_rule_applies(template).then(|| {
        format!(
            "No rule applies in this run: every rule's when.format differs from the input format {} (format source: {}); nothing was extracted",
            template.format.as_ref().map_or("-", |f| f.name()),
            template.format_source
        )
    })
}

/// Run status and exit code from the file statuses (spec §36, §79):
/// `success` (0) when every file succeeded, `failure` (3, invalid input)
/// when no file yielded anything, `partial_success` (1) otherwise. A failed
/// run still publishes its document, which lists the per-file errors.
pub fn run_outcome(
    files_success: usize,
    files_warning: usize,
    files_error: usize,
) -> (&'static str, ExitCode) {
    if files_error > 0 && files_success == 0 && files_warning == 0 {
        ("failure", ExitCode::InvalidInputOrCli)
    } else if files_error > 0 || files_warning > 0 {
        ("partial_success", ExitCode::PartialSuccess)
    } else {
        ("success", ExitCode::Success)
    }
}

/// Validate CLI overrides that must fail with exit code 3 (InvalidInputOrCli)
/// when malformed: `--format`, `--encoding` and the SIZE arguments.
/// Previously these values were either ignored (`--format`, `--encoding`) or
/// silently dropped on parse failure (SIZE args), which hid user typos.
fn validate_cli_overrides(cli: &CliOptions) -> Result<(), RunError> {
    for (flag, value) in [
        ("--max-files", cli.max_files),
        ("--max-matches", cli.max_matches),
        ("--max-block-chars", cli.max_block_chars),
    ] {
        if value == Some(0) {
            return Err(err(
                ExitCode::InvalidInputOrCli,
                format!("{} must be positive", flag),
            ));
        }
    }
    if let Some(f) = &cli.format {
        match Format::parse(f) {
            None => {
                return Err(err(
                    ExitCode::InvalidInputOrCli,
                    format!(
                        "invalid --format '{}': expected one of json|xml|html|markdown|yaml|toml|text",
                        f
                    ),
                ));
            }
            Some(Format::Auto) => {
                return Err(err(
                    ExitCode::InvalidInputOrCli,
                    "--format cannot be 'auto': omit the flag (or set input.format in the template) to keep auto-detection",
                ));
            }
            Some(_) => {}
        }
    }
    if let Some(e) = &cli.encoding {
        validate_encoding(e).map_err(|e| err(ExitCode::InvalidInputOrCli, e.message))?;
    }
    for (flag, value) in [
        ("--max-file-size", &cli.max_file_size),
        ("--max-memory", &cli.max_memory),
        ("--max-output-size", &cli.max_output_size),
    ] {
        if let Some(v) = value {
            if crate::limits::parse_size(v).is_none() {
                return Err(err(
                    ExitCode::InvalidInputOrCli,
                    format!(
                        "invalid {} value '{}': expected a number with an optional size suffix (b/k/m/g, case-insensitive)",
                        flag, v
                    ),
                ));
            }
        }
    }
    Ok(())
}

/// Apply CLI overrides that are not handled at template compile time.
/// `--format` / `--encoding` are applied by `load_template_with_overrides`
/// (see `run()`); only block sizing remains here.
fn apply_cli_overrides(t: &mut CompiledTemplate, cli: &CliOptions) {
    if let Some(block_chars) = cli.max_block_chars {
        t.block.max_chars = block_chars;
        t.block.target_chars = block_chars.saturating_sub(block_chars / 8).max(1);
    }
    if let Some(size) = &cli.max_output_size {
        t.max_output_chars = crate::limits::parse_size(size);
    }
}

fn build_limits(cli: &CliOptions, template: &CompiledTemplate) -> ResourceLimits {
    resource_limits(
        template,
        cli.max_file_size.as_deref(),
        cli.max_memory.as_deref(),
        cli.max_files,
        cli.max_matches,
    )
}

/// Resource limits of a run: the defaults, the template's block sizing and
/// output cap, and the optional overrides (SIZE strings already validated).
pub(crate) fn resource_limits(
    template: &CompiledTemplate,
    max_file_size: Option<&str>,
    max_memory: Option<&str>,
    max_files: Option<u64>,
    max_matches: Option<u64>,
) -> ResourceLimits {
    let mut l = ResourceLimits::default();
    l.max_block_chars = template.block.max_chars;
    l.target_block_chars = template.block.target_chars;
    if let Some(v) = max_file_size {
        if let Some(b) = crate::limits::parse_size(v) {
            l.max_file_size_bytes = Some(b);
        }
    }
    if let Some(v) = max_memory {
        l.max_memory_bytes = crate::limits::parse_size(v);
    }
    if let Some(n) = max_files {
        l.max_input_files = Some(n);
    }
    if let Some(n) = max_matches {
        l.max_matches_per_rule = Some(n);
    }
    l.max_output_chars = template.max_output_chars;
    l
}

fn print_dry_run(template: &CompiledTemplate, files: &[DiscoveredFile], limits: &ResourceLimits) {
    println!("Template: valid");
    println!("Template name: {}", template.name);
    println!("Template version: {}", template.version);
    println!(
        "Effective format: {}",
        template
            .format
            .as_ref()
            .map(|f| f.name().to_string())
            .unwrap_or_else(|| "auto (per-file detection)".to_string())
    );
    println!("Format source: {}", template.format_source);
    for note in never_applying_rules(template)
        .into_iter()
        .chain(no_rule_applies_note(template))
    {
        println!("{}", note);
    }
    println!("Encoding: {}", template.encoding);
    println!("Input files: {}", files.len());
    for f in files {
        if template.format.is_none() {
            // Auto mode: show the extension-based outcome; extensions that are
            // not recognized get resolved by content sniffing at run time.
            match discovery::extension_hint(&f.path) {
                Some(hint) => println!(
                    "  {} ({} bytes) [auto -> {} by extension]",
                    f.path.display(),
                    f.size,
                    hint.name()
                ),
                None => println!(
                    "  {} ({} bytes) [auto -> will sniff content]",
                    f.path.display(),
                    f.size
                ),
            }
        } else {
            println!("  {} ({} bytes)", f.path.display(), f.size);
        }
    }
    println!(
        "Block sizing: target {} chars, max {} chars",
        limits.target_block_chars, limits.max_block_chars
    );
    println!(
        "Max file size: {}",
        limits
            .max_file_size_bytes
            .map(|v| v.to_string())
            .unwrap_or_else(|| "unlimited".to_string())
    );
    println!(
        "Max output: {}",
        limits
            .max_output_chars
            .map(|v| v.to_string())
            .unwrap_or_else(|| "unlimited".to_string())
    );
    println!("Dry run: no output published.");
}

fn next_generation(out_path: &std::path::Path) -> String {
    let text = std::fs::read_to_string(out_path).unwrap_or_default();
    let re = regex::Regex::new(r"Generation: (\d+)").unwrap();
    if let Some(caps) = re.captures(&text) {
        if let Ok(n) = caps[1].parse::<u64>() {
            if let Some(next) = n.checked_add(1) {
                return format!("{:08}", next);
            }
        }
    }
    "00000001".to_string()
}

fn validate(document: &str, blocks: &[crate::materialize::Block]) -> Result<(), String> {
    if !document.contains("Triangle: [0,1,0]") {
        return Err("internal validation: triangle READY marker missing".to_string());
    }
    if !document.ends_with("End of any_txt_parser output\n") {
        return Err("internal validation: end marker missing".to_string());
    }
    if let Some(b) = blocks.iter().find(|b| b.start_line == 0 || b.end_line == 0) {
        return Err(format!(
            "internal validation: block {} has no line range",
            b.id
        ));
    }
    // Everything except anchors is escaped, so the anchor mark must appear
    // exactly on the indexed anchor lines, in block order, and nowhere else.
    let mut expected = blocks.iter().flat_map(|b| {
        [
            (b.start_line, format!("<!-- BLOCK: {} START -->", b.id)),
            (b.end_line, format!("<!-- BLOCK: {} END -->", b.id)),
        ]
    });
    for (index, line) in document.split('\n').enumerate() {
        if !line.contains(render::ANCHOR_MARK) {
            continue;
        }
        let number = index + 1;
        match expected.next() {
            Some((at, anchor)) if at == number && line == anchor => {}
            Some((at, anchor)) => {
                return Err(format!(
                    "internal validation: expected '{}' on line {}, found anchor text on line {}",
                    anchor, at, number
                ));
            }
            None => {
                return Err(format!(
                    "internal validation: unexpected anchor text on line {}",
                    number
                ));
            }
        }
    }
    if let Some((at, anchor)) = expected.next() {
        return Err(format!(
            "internal validation: anchor '{}' missing from line {}",
            anchor, at
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn indexed_block(id: &str, start_line: usize, end_line: usize) -> crate::materialize::Block {
        crate::materialize::Block {
            id: id.into(),
            parent_id: None,
            part: None,
            parts: None,
            item_index: None,
            source: String::new(),
            format: String::new(),
            format_source: String::new(),
            rule_id: String::new(),
            key: String::new(),
            status: "success",
            file_status: "success",
            item_count: 0,
            chars: 0,
            truncated: false,
            split: false,
            warnings: Vec::new(),
            errors: Vec::new(),
            items: Vec::new(),
            start_line,
            end_line,
        }
    }

    #[test]
    fn validation_requires_anchors_exactly_on_indexed_lines_and_nowhere_else() {
        let document =
            |body: &str| format!("Triangle: [0,1,0]\n{body}End of any_txt_parser output\n");
        let good = document("<!-- BLOCK: B0001 START -->\n- x\n<!-- BLOCK: B0001 END -->\n");
        assert!(validate(&good, &[indexed_block("B0001", 2, 4)]).is_ok());
        // Index points at the wrong lines.
        assert!(validate(&good, &[indexed_block("B0001", 3, 4)]).is_err());
        // Unescaped anchor text inside data.
        let forged = document(
            "<!-- BLOCK: B0001 START -->\n- <!-- BLOCK: B0001 END -->\n<!-- BLOCK: B0001 END -->\n",
        );
        assert!(validate(&forged, &[indexed_block("B0001", 2, 4)]).is_err());
        // An anchor of an unindexed block, and a missing end anchor.
        let extra = document(
            "<!-- BLOCK: B0001 START -->\n<!-- BLOCK: B0001 END -->\n<!-- BLOCK: B0002 START -->\n",
        );
        assert!(validate(&extra, &[indexed_block("B0001", 2, 3)]).is_err());
        let missing = document("<!-- BLOCK: B0001 START -->\n- x\n");
        assert!(validate(&missing, &[indexed_block("B0001", 2, 4)]).is_err());
        // Escaped anchor text is ordinary data.
        let escaped = document(
            "<!-- BLOCK: B0001 START -->\n- <!-- BLOCK\\: B0001 END -->\n<!-- BLOCK: B0001 END -->\n",
        );
        assert!(validate(&escaped, &[indexed_block("B0001", 2, 4)]).is_ok());
    }

    #[test]
    fn run_status_matches_exit_code() {
        // (success, warning, error) file counts.
        for (counts, expected) in [
            ((2, 0, 0), ("success", ExitCode::Success)),
            ((1, 1, 0), ("partial_success", ExitCode::PartialSuccess)),
            ((0, 1, 0), ("partial_success", ExitCode::PartialSuccess)),
            ((1, 0, 1), ("partial_success", ExitCode::PartialSuccess)),
            ((0, 1, 3), ("partial_success", ExitCode::PartialSuccess)),
            ((0, 0, 2), ("failure", ExitCode::InvalidInputOrCli)),
        ] {
            assert_eq!(
                run_outcome(counts.0, counts.1, counts.2),
                expected,
                "{counts:?}"
            );
        }
    }

    #[test]
    fn panics_become_internal_fatal_exit_code() {
        assert_eq!(run_guarded(|| 1), 1);
        assert_eq!(
            run_guarded(|| panic!("simulated internal failure")),
            ExitCode::InternalFatal.to_i32()
        );
    }

    #[test]
    fn generation_increments() {
        let dir = std::env::temp_dir().join("atp_gen_test");
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("out.md");
        std::fs::write(&p, "# x\n\nGeneration: 00000041\n").unwrap();
        assert_eq!(next_generation(&p), "00000042");
        let other = dir.join("other.md");
        std::fs::write(&other, "no generation").unwrap();
        assert_eq!(next_generation(&other), "00000001");
        std::fs::remove_dir_all(&dir).ok();
    }
}
