//! The navguard command line (`any_txt_parser nav ...`).
//!
//! Every path a mode reads or writes lies inside the project root
//! (`--root`, default: the current directory; see [`crate::paths`]). Exit
//! codes: 0 success, 1 failure, with the reason on stderr after `ERROR:`.

use crate::api::{self, Project};
use crate::config::Config;
use crate::{
    build_rs, cache, navigator, paths, responder, scanner, text, triangle, validate, watch,
};
use std::fs;
use std::path::Path;

#[derive(Debug, Default, PartialEq)]
struct CliOptions {
    root_override: Option<String>,
    focus_override: Option<String>,
    positional: Vec<String>,
    force_scan: bool,
    timeout_override: Option<u64>,
    dry_run: bool,
}

fn parse_args(args: &[String]) -> Result<CliOptions, String> {
    let mut opts = CliOptions::default();
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--root" => {
                opts.root_override = Some(iter.next().ok_or("--root requires a path")?.clone());
            }
            "--focus" => {
                opts.focus_override = Some(iter.next().ok_or("--focus requires a path")?.clone());
            }
            "--force" => opts.force_scan = true,
            "--dry-run" => opts.dry_run = true,
            "--timeout" => {
                let value = iter.next().ok_or("--timeout requires seconds")?;
                opts.timeout_override = Some(
                    value
                        .parse()
                        .map_err(|_| "--timeout must be a non-negative integer")?,
                );
            }
            _ => opts.positional.push(arg.clone()),
        }
    }
    Ok(opts)
}

const USAGE: &str = "\
Usage: any_txt_parser nav <mode> [options]   (paths inside the project root)
Modes:
  --oneshot                  Scan once, write navigator.md, navigator_deep.md, navigator_deep2.md
  --watch [--focus <path>]   Write the navigators, then rewrite them on every change
  --check [--brief|--diff]   Report to stdout (T:/F:/I:/B: lines); writes no navigator files
  --grep <pattern>           Search the symbol index (class/fn names)
  --lines <path> <start> [end]   Print numbered lines of a file
  --extract <pat> [path]     Search text, print the sentence around each match
  --todos                    Find TODO/FIXME/HACK/XXX markers (whole words) across the project
  --patch replace|insert|delete ...   Edit a file (nav --patch help); --dry-run shows the change
  --set-rules <str>          Write <str> to .navguard_rules in the root
  --triangle <action>        Workflow state: status|step_done|verify_start|verify_complete|verify_fail|reset|log
  --validate                 Validate the anchors listed in anchor_map.md
  --build <plan>             Assemble files from a plan
  --merkle-verify            Verify the cache against the files
Options:
  --root <dir>               Project root (default: current directory); any position
  --force                    Full scan, ignore the cache
  --timeout <secs>           Watch mode timeout (default: 10800 = 3h)
  --brief                    --check: only T:, I: and B: lines
  --diff                     --check: only files changed since the last scan
  --focus <path>             --watch: reuse symbols for changes under path
  --version                  Print the version
Only --oneshot and --watch write navigator.md, navigator_deep.md, navigator_deep2.md;
--check reads (with --diff it saves the scan to .navguard_cache.json for the next diff).
Exit codes: 0 success, 1 failure (reason on stderr after ERROR:).
";

const PATCH_USAGE: &str = "\
Usage: any_txt_parser nav --patch <command> ... [--dry-run] [--root DIR]
  --patch replace <path> <old> <new>      Replace every occurrence of <old> with <new>
  --patch insert <path> <line> <text>     Insert <text> as line <line> (1 = first, last+1 = append)
  --patch delete <path> <start> [<end>]   Delete lines start..end (inclusive)
  --dry-run                               Show the change (- old, + new lines); write nothing
<path> is relative to the root, or absolute inside it. Line numbers are those of
`any_txt_parser lines`. The file keeps its line breaks (CRLF stays CRLF) and its BOM,
and is replaced atomically. Exit 1 when nothing could be changed: text not found,
line out of range, path outside the root, binary or non-UTF-8 file.
";

fn fail(message: impl std::fmt::Display) -> i32 {
    eprintln!("ERROR: {}", message);
    1
}

/// Runs one navguard command; `args` are the arguments after `nav`;
/// `version` is what `nav --version` prints. Returns the exit code.
pub fn run(args: &[String], version: &str) -> i32 {
    let opts = match parse_args(args) {
        Ok(opts) => opts,
        Err(error) => return fail(error),
    };
    let mode = opts
        .positional
        .first()
        .map(|s| s.as_str())
        .unwrap_or("--watch");
    if matches!(mode, "--help" | "-h" | "help") {
        print!("{}", USAGE);
        return 0;
    }
    if matches!(mode, "--version" | "-V") {
        println!("{}", version);
        return 0;
    }
    let config_path = Path::new(opts.root_override.as_deref().unwrap_or(".")).join("navguard.json");
    let mut cfg = Config::load(&config_path.to_string_lossy());
    if let Some(ref r) = opts.root_override {
        cfg.root = r.clone();
    }
    if let Some(t) = opts.timeout_override {
        cfg.timeout_sec = t;
    }
    if !Path::new(&cfg.root).is_dir() {
        return fail(format!("project root is not a directory: {}", cfg.root));
    }
    let arg = |i: usize| opts.positional.get(i).map(|s| s.as_str());

    match mode {
        "--watch" => run_watch(&cfg, opts.force_scan, opts.focus_override.as_deref()),
        "--oneshot" => {
            let current = update_cache(&cfg, opts.force_scan);
            write_navigators(&cfg, &current)
        }
        "--check" => {
            let brief = opts.positional.iter().any(|a| a == "--brief");
            if opts.positional.iter().any(|a| a == "--diff") {
                return run_check_diff(&cfg);
            }
            let response = responder::run_check(&cfg);
            if brief {
                for line in response.output.lines() {
                    if line.starts_with("T:") || line.starts_with("I:") || line.starts_with("B:") {
                        println!("{}", line);
                    }
                }
            } else {
                println!("{}", response.output);
            }
            0
        }
        "--grep" => match arg(1) {
            Some(pattern) => run_grep(&cfg, pattern, opts.force_scan),
            None => fail("--grep requires a pattern"),
        },
        "--set-rules" => match arg(1) {
            Some(rules) => {
                let path = Path::new(&cfg.root).join(".navguard_rules");
                match fs::write(&path, rules) {
                    Ok(()) => {
                        println!("B: rules|written|{}", path.display());
                        0
                    }
                    Err(e) => fail(format!("cannot write '{}': {}", path.display(), e)),
                }
            }
            None => fail("--set-rules requires a value"),
        },
        "--lines" => {
            let (Some(path), Some(start)) = (arg(1), arg(2)) else {
                return fail("--lines <path> <start> [<end>]");
            };
            let Ok(start) = start.parse::<usize>() else {
                return fail("start must be a number");
            };
            let end = match arg(3).map(str::parse::<usize>) {
                None => None,
                Some(Ok(end)) => Some(end),
                Some(Err(_)) => return fail("end must be a number"),
            };
            run_lines(&cfg, path, start, end)
        }
        "--extract" => match arg(1) {
            Some(pattern) => run_extract(&cfg, pattern, arg(2)),
            None => fail("--extract <pattern> [<path>]"),
        },
        "--todos" => run_todos(&cfg),
        "--patch" => run_patch(&cfg, &opts.positional[1..], opts.dry_run),
        "--triangle" => match arg(1) {
            Some(action) => run_triangle(&cfg, action),
            None => fail(
                "--triangle requires action (status|step_done|verify_start|verify_complete|verify_fail|reset|log)",
            ),
        },
        "--validate" => {
            let map_path = project_path(&cfg, "anchor_map.md");
            let errors = validate::validate(Path::new(&cfg.root), &map_path);
            if errors.is_empty() {
                println!("B: validate|ok|all anchors valid");
                0
            } else {
                for e in &errors {
                    eprintln!("  {}", e);
                }
                println!("B: validate|{} errors", errors.len());
                1
            }
        }
        "--build" => match arg(1) {
            Some(plan) => match build_rs::build(Path::new(&cfg.root), plan) {
                Ok(files) => {
                    for f in &files {
                        println!("B: build|assembled|{}", f);
                    }
                    println!("B: build|ok|{} files assembled", files.len());
                    0
                }
                Err(e) => fail(format!("build failed: {}", e)),
            },
            None => fail("--build requires plan path"),
        },
        "--merkle-verify" => {
            let cache_path = project_path(&cfg, api::CACHE_FILE);
            match cache::load_cache(&cache_path) {
                Some(cache) => {
                    let errors = cache::verify_cache_integrity(&cfg.root, &cache);
                    if errors.is_empty() {
                        println!("B: merkle-verify|ok|all files match cache");
                        0
                    } else {
                        for e in &errors {
                            eprintln!("  {}", e);
                        }
                        println!("B: merkle-verify|{} errors", errors.len());
                        1
                    }
                }
                None => {
                    println!("B: merkle-verify|no cache found");
                    1
                }
            }
        }
        _ => {
            eprintln!("ERROR: unknown argument '{}'", mode);
            eprint!("{}", USAGE);
            1
        }
    }
}

fn run_watch(cfg: &Config, force_scan: bool, focus: Option<&str>) -> i32 {
    // Start collecting events before the initial scan to avoid a gap.
    let watcher = match watch::FileWatcher::new(&cfg.root) {
        Ok(watcher) => watcher,
        Err(e) => return fail(e),
    };
    let current = update_cache(cfg, force_scan);
    let code = write_navigators(cfg, &current);
    if code != 0 {
        return code;
    }
    let mut last_activity = std::time::Instant::now();
    let timeout = std::time::Duration::from_secs(cfg.timeout_sec);
    loop {
        if last_activity.elapsed() > timeout {
            println!("B: watch|timeout|{}s elapsed, exiting", cfg.timeout_sec);
            return 0;
        }
        let mut changed =
            match watcher.wait_for_changes_for(timeout.saturating_sub(last_activity.elapsed())) {
                Some(files) => files,
                None => continue,
            };
        changed.retain(|path| !scanner::is_excluded_path(path, &cfg.exclude));
        if changed.is_empty() {
            continue;
        }
        last_activity = std::time::Instant::now();
        // Events outside focus require rebuilding all symbols.
        let needs_full = focus.is_some_and(|focus| {
            let focus = focus.replace('\\', "/");
            changed.iter().any(|c| !Path::new(c).starts_with(&focus))
        });
        // Reconcile the whole file set, including directory events.
        let current = update_cache(cfg, needs_full);
        let code = write_navigators(cfg, &current);
        if code != 0 {
            return code;
        }
    }
}

fn run_check_diff(cfg: &Config) -> i32 {
    let triangle = triangle::read_triangle(&project_path(cfg, &cfg.triangle_path));
    // Compares with the cache and saves the current scan for the next diff.
    let report = match Project::from_config(cfg.clone()).changes(true) {
        Ok(report) => report,
        Err(e) => return fail(e),
    };

    println!("T: {},{},{}", triangle.a, triangle.b, triangle.c);

    if report.cold {
        for f in &report.changes {
            println!("F: {}|{}|{}|{}", f.path, f.lines, f.tokens, f.sha);
        }
        println!("B: diff|cold (no previous cache)");
    } else {
        for d in &report.changes {
            println!(
                "F: {}|{}|{}|{}|({})",
                d.path, d.lines, d.tokens, d.sha, d.status
            );
        }
        if report.changes.is_empty() {
            println!("B: diff|no changes");
        } else {
            println!(
                "B: diff|{} changed, {} new, {}",
                report.changed, report.new, report.deleted
            );
        }
    }
    if let Some(error) = &report.persist_error {
        eprintln!("cache: {}", error);
    }

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    println!("B: sync|{}", now);
    0
}

fn run_grep(cfg: &Config, pattern: &str, force_scan: bool) -> i32 {
    let options = api::FindOptions {
        use_cache: true,
        force: force_scan,
        case_sensitive: true,
        exact: false,
        max_hits: None,
    };
    let hits = match Project::from_config(cfg.clone()).find_symbols(pattern, &options) {
        Ok(hits) => hits,
        Err(e) => return fail(e),
    };
    for hit in &hits {
        println!(
            "F: {}|{}|{}",
            hit.path, hit.symbol.line_start, hit.symbol.label
        );
    }
    if hits.is_empty() {
        println!("B: grep|no matches for '{}'", pattern);
    } else {
        println!("B: grep|{} matches for '{}'", hits.len(), pattern);
    }
    0
}

fn run_lines(cfg: &Config, path: &str, start: usize, end: Option<usize>) -> i32 {
    match Project::from_config(cfg.clone()).read_lines(path, start, end) {
        Ok(lines) => {
            for line in lines {
                println!("{:>6}|{}", line.line, line.text);
            }
            0
        }
        Err(e) => fail(e),
    }
}

fn run_extract(cfg: &Config, pattern: &str, specific_path: Option<&str>) -> i32 {
    let root = Path::new(&cfg.root);
    let files: Vec<(std::path::PathBuf, String)> = match specific_path {
        Some(p) => match paths::existing_file(root, p) {
            Ok(file) => vec![file],
            Err(e) => return fail(e),
        },
        None => scanner::list_files(&cfg.root, &cfg.exclude)
            .into_iter()
            .map(|rel| (root.join(&rel), rel))
            .collect(),
    };

    let mut total = 0usize;
    for (full, rel) in &files {
        let content = match fs::read(full) {
            Ok(bytes) if !text::is_binary(&bytes) => text::decode(&bytes),
            _ => continue,
        };
        for (line, snippet, truncated) in api::sentence_hits(&content, pattern) {
            let ellipsis = if truncated { "..." } else { "" };
            println!("M: {}|{}|{}{}", rel, line, snippet, ellipsis);
            total += 1;
        }
    }

    if total == 0 {
        println!("B: extract|no matches for '{}'", pattern);
    } else {
        println!("B: extract|{} matches for '{}'", total, pattern);
    }
    0
}

fn run_todos(cfg: &Config) -> i32 {
    let hits = match Project::from_config(cfg.clone()).todos(api::TodoMatch::Strict) {
        Ok(hits) => hits,
        Err(e) => return fail(e),
    };
    for hit in &hits {
        println!("M: {}|{}|{}", hit.path, hit.line, hit.text);
    }
    if hits.is_empty() {
        println!("B: todos|none found");
    } else {
        println!("B: todos|{} items", hits.len());
    }
    0
}

fn run_triangle(cfg: &Config, action: &str) -> i32 {
    // The state file and its transition log live in the project root.
    let state = project_path(cfg, &cfg.triangle_path);
    let log = responder::log_path_from_triangle(&state);
    let result = triangle::run_triangle_action_with_log(&state, &log, action);
    println!("{}", result);
    if result.contains("|error|") {
        1
    } else {
        0
    }
}

/// A file edit computed in memory.
#[derive(Debug)]
struct Patch {
    full: std::path::PathBuf,
    rel: String,
    bom: bool,
    before: String,
    after: String,
    /// `replaced 2 occurrence(s) in src/a.rs`.
    summary: String,
}

/// One line of `text` with its line break (`""` for a last line without
/// one). LF, CRLF and a lone CR end a line, as in `lines`.
fn split_keep_ends(text: &str) -> Vec<(&str, &str)> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut start = 0;
    let mut i = 0;
    while i < bytes.len() {
        let end = match bytes[i] {
            b'\n' => Some(i + 1),
            b'\r' if bytes.get(i + 1) == Some(&b'\n') => Some(i + 2),
            b'\r' => Some(i + 1),
            _ => None,
        };
        match end {
            Some(end) => {
                out.push((&text[start..i], &text[i..end]));
                start = end;
                i = end;
            }
            None => i += 1,
        }
    }
    if start < text.len() {
        out.push((&text[start..], ""));
    }
    out
}

/// The line break of the first line of `text`, `\n` when it has none.
fn line_break_of(text: &str) -> &'static str {
    match split_keep_ends(text).first().map(|(_, end)| *end) {
        Some("\r\n") => "\r\n",
        Some("\r") => "\r",
        _ => "\n",
    }
}

/// Reads the file at `path` for editing: it must be inside the root and
/// UTF-8 text. Returns (full path, relative path, had a BOM, text).
fn read_for_patch(
    root: &Path,
    path: &str,
) -> Result<(std::path::PathBuf, String, bool, String), String> {
    let (full, rel) = paths::existing_file(root, path)?;
    let bytes = fs::read(&full).map_err(|e| format!("cannot read '{}': {}", rel, e))?;
    if text::is_binary(&bytes) {
        return Err(format!("cannot patch {}: binary file", rel));
    }
    let (bom, body) = match bytes.strip_prefix(b"\xEF\xBB\xBF") {
        Some(body) => (true, body),
        None => (false, &bytes[..]),
    };
    let content = String::from_utf8(body.to_vec())
        .map_err(|_| format!("cannot patch {}: not UTF-8 text", rel))?;
    Ok((full, rel, bom, content))
}

fn patch_replace(root: &Path, path: &str, old: &str, new: &str) -> Result<Patch, String> {
    if old.is_empty() {
        return Err("the text to replace must not be empty".to_string());
    }
    let (full, rel, bom, before) = read_for_patch(root, path)?;
    let mut old = old.to_string();
    let mut new = new.to_string();
    if !before.contains(&old) {
        // A multi-line <old> typed with LF still matches a CRLF file.
        let crlf = |s: &str| text::normalize_line_breaks(s).replace('\n', "\r\n");
        if before.contains("\r\n") && old.contains('\n') && before.contains(&crlf(&old)) {
            old = crlf(&old);
            new = crlf(&new);
        } else {
            return Err(format!("'{}' not found in {}; nothing changed", old, rel));
        }
    }
    let count = before.matches(&old).count();
    let after = before.replace(&old, &new);
    Ok(Patch {
        summary: format!("replaced {} occurrence(s) in {}", count, rel),
        full,
        rel,
        bom,
        before,
        after,
    })
}

fn patch_insert(root: &Path, path: &str, line: usize, inserted: &str) -> Result<Patch, String> {
    let (full, rel, bom, before) = read_for_patch(root, path)?;
    let mut lines = split_keep_ends(&before);
    if line == 0 || line > lines.len() + 1 {
        return Err(format!(
            "line {} out of range: {} has {} line(s), insert at 1-{}",
            line,
            rel,
            lines.len(),
            lines.len() + 1
        ));
    }
    let eol = line_break_of(&before);
    let block = text::normalize_line_breaks(inserted).replace('\n', eol);
    let count = split_keep_ends(&block).len().max(1);
    let mut after = String::new();
    let appending = line == lines.len() + 1;
    if appending {
        // The old last line needs a line break before the new one; the new
        // last line keeps the file's choice of a final line break.
        let ends_with_break = lines.last().is_none_or(|(_, end)| !end.is_empty());
        if let Some(last) = lines.last_mut() {
            last.1 = if last.1.is_empty() { eol } else { last.1 };
        }
        for (text, end) in &lines {
            after.push_str(text);
            after.push_str(end);
        }
        after.push_str(&block);
        if ends_with_break || lines.is_empty() {
            after.push_str(eol);
        }
    } else {
        for (index, (text, end)) in lines.iter().enumerate() {
            if index + 1 == line {
                after.push_str(&block);
                after.push_str(eol);
            }
            after.push_str(text);
            after.push_str(end);
        }
    }
    Ok(Patch {
        summary: format!("inserted {} line(s) at line {} in {}", count, line, rel),
        full,
        rel,
        bom,
        before,
        after,
    })
}

fn patch_delete(
    root: &Path,
    path: &str,
    start: usize,
    end: Option<usize>,
) -> Result<Patch, String> {
    let (full, rel, bom, before) = read_for_patch(root, path)?;
    let lines = split_keep_ends(&before);
    if start == 0 || start > lines.len() {
        return Err(format!(
            "start line {} out of range: {} has {} line(s)",
            start,
            rel,
            lines.len()
        ));
    }
    let end = end.unwrap_or(start);
    if end < start {
        return Err(format!("end line {} is before start line {}", end, start));
    }
    let end = end.min(lines.len());
    let mut kept: Vec<(&str, &str)> = lines[..start - 1].to_vec();
    kept.extend_from_slice(&lines[end..]);
    // Deleting the last line keeps the file's choice of a final line break.
    if end == lines.len() && lines[end - 1].1.is_empty() {
        if let Some(last) = kept.last_mut() {
            last.1 = "";
        }
    }
    let after: String = kept.iter().flat_map(|(t, e)| [*t, *e]).collect();
    Ok(Patch {
        summary: format!(
            "deleted {} line(s) ({}-{}) from {}",
            end - start + 1,
            start,
            end,
            rel
        ),
        full,
        rel,
        bom,
        before,
        after,
    })
}

/// Lines of the change from `before` to `after`: the changed region with
/// `-` (old) and `+` (new) lines and their numbers, at most `cap` of each.
fn preview(before: &str, after: &str, cap: usize) -> String {
    let a = text::normalize_line_breaks(before);
    let b = text::normalize_line_breaks(after);
    let a: Vec<&str> = a.lines().collect();
    let b: Vec<&str> = b.lines().collect();
    let prefix = a.iter().zip(&b).take_while(|(x, y)| x == y).count();
    let suffix = a[prefix..]
        .iter()
        .rev()
        .zip(b[prefix..].iter().rev())
        .take_while(|(x, y)| x == y)
        .count();
    let removed = &a[prefix..a.len() - suffix];
    let added = &b[prefix..b.len() - suffix];
    let mut out = String::new();
    for (sign, lines) in [("-", removed), ("+", added)] {
        for (i, line) in lines.iter().enumerate().take(cap) {
            out.push_str(&format!("{} {:>6}| {}\n", sign, prefix + i + 1, line));
        }
        if lines.len() > cap {
            out.push_str(&format!(
                "{} ... {} more line(s)\n",
                sign,
                lines.len() - cap
            ));
        }
    }
    out
}

fn apply_patch(patch: Result<Patch, String>, dry_run: bool) -> i32 {
    let patch = match patch {
        Ok(patch) => patch,
        Err(e) => return fail(format!("patch: {}", e)),
    };
    if dry_run {
        println!(
            "B: patch|dry-run|would have {}; nothing written",
            patch.summary
        );
        print!("{}", preview(&patch.before, &patch.after, 40));
        return 0;
    }
    let mut bytes = Vec::with_capacity(patch.after.len() + 3);
    if patch.bom {
        bytes.extend_from_slice(b"\xEF\xBB\xBF");
    }
    bytes.extend_from_slice(patch.after.as_bytes());
    if let Err(e) = crate::write::atomic_write(&patch.full, &bytes) {
        return fail(format!("cannot write '{}': {}", patch.rel, e));
    }
    println!("B: patch|{}", patch.summary);
    0
}

fn run_patch(cfg: &Config, args: &[String], dry_run: bool) -> i32 {
    let root = Path::new(&cfg.root);
    let arg = |i: usize| args.get(i).map(|s| s.as_str());
    let number = |i: usize, what: &str| -> Result<usize, String> {
        arg(i)
            .ok_or_else(|| format!("{} is missing (nav --patch help)", what))?
            .parse()
            .map_err(|_| format!("{} must be a number", what))
    };
    let patch = match arg(0) {
        Some("help" | "--help" | "-h") => {
            print!("{}", PATCH_USAGE);
            return 0;
        }
        Some("replace") => match (arg(1), arg(2), arg(3)) {
            (Some(path), Some(old), Some(new)) => patch_replace(root, path, old, new),
            _ => Err("usage: --patch replace <path> <old> <new>".to_string()),
        },
        Some("insert") => match (arg(1), number(2, "line"), arg(3)) {
            (Some(path), Ok(line), Some(text)) => patch_insert(root, path, line, text),
            (_, Err(e), _) if arg(2).is_some() => Err(e),
            _ => Err("usage: --patch insert <path> <line> <text>".to_string()),
        },
        Some("delete") => match (arg(1), number(2, "start")) {
            (Some(path), Ok(start)) => match arg(3).map(str::parse::<usize>) {
                None => patch_delete(root, path, start, None),
                Some(Ok(end)) => patch_delete(root, path, start, Some(end)),
                Some(Err(_)) => Err("end must be a number".to_string()),
            },
            (_, Err(e)) if arg(2).is_some() => Err(e),
            _ => Err("usage: --patch delete <path> <start> [<end>]".to_string()),
        },
        Some(other) => Err(format!(
            "unknown patch command '{}' (replace, insert, delete; nav --patch help)",
            other
        )),
        None => Err("--patch requires replace, insert or delete (nav --patch help)".to_string()),
    };
    apply_patch(patch, dry_run)
}

fn project_path(cfg: &Config, path: &str) -> String {
    Path::new(&cfg.root)
        .join(path)
        .to_string_lossy()
        .into_owned()
}

fn update_cache(cfg: &Config, force_scan: bool) -> cache::NavguardCache {
    Project::from_config(cfg.clone()).update_cache(force_scan)
}

fn write_navigators(cfg: &Config, cache: &cache::NavguardCache) -> i32 {
    match navigator::write(Path::new(&cfg.root), cache) {
        Ok(()) => 0,
        Err(e) => fail(e),
    }
}

#[cfg(test)]
mod cli_root_tests {
    use super::*;

    #[test]
    fn parses_force_timeout_and_root_in_any_position() {
        let args: Vec<String> = [
            "--force",
            "--watch",
            "--root",
            "project",
            "--timeout",
            "2",
            "--focus",
            "src",
            "--dry-run",
        ]
        .into_iter()
        .map(String::from)
        .collect();
        let options = parse_args(&args).unwrap();
        assert!(options.force_scan);
        assert!(options.dry_run);
        assert_eq!(options.timeout_override, Some(2));
        assert_eq!(options.root_override.as_deref(), Some("project"));
        assert_eq!(options.focus_override.as_deref(), Some("src"));
        assert_eq!(options.positional, ["--watch"]);
        assert!(parse_args(&["--timeout".to_string()]).is_err());
        assert!(parse_args(&["--timeout".to_string(), "invalid".to_string()]).is_err());
        assert!(parse_args(&["--root".to_string()]).is_err());
    }

    #[test]
    fn cache_and_all_navigators_use_the_selected_root() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = Config {
            root: dir.path().to_string_lossy().into_owned(),
            exclude: vec![],
            ..Config::default()
        };
        fs::write(dir.path().join("sample.rs"), "fn sample() {}\n").unwrap();
        let current = update_cache(&cfg, false);
        assert_eq!(write_navigators(&cfg, &current), 0);
        for name in [
            ".navguard_cache.json",
            "navigator.md",
            "navigator_deep.md",
            "navigator_deep2.md",
        ] {
            assert!(dir.path().join(name).is_file(), "missing {} in root", name);
        }
        let deep2 = fs::read_to_string(dir.path().join("navigator_deep2.md")).unwrap();
        assert!(deep2.contains("fn sample()"));
        let cached = cache::load_cache(&project_path(&cfg, ".navguard_cache.json")).unwrap();
        assert_eq!(cached.files.len(), 1);
        assert!(cache::is_cache_fresh(&cfg.root, &cached, &cfg.exclude));
    }

    #[test]
    fn navigator_tree_comes_from_the_scanned_files() {
        let dir = tempfile::tempdir().unwrap();
        for sub in ["src/deep", "target", "empty"] {
            fs::create_dir_all(dir.path().join(sub)).unwrap();
        }
        fs::write(dir.path().join("src/deep/a.rs"), "fn a() {}\n").unwrap();
        fs::write(dir.path().join("src/b.rs"), "fn b() {}\n").unwrap();
        fs::write(dir.path().join("target/out.rs"), "fn skipped() {}\n").unwrap();
        let cfg = Config {
            root: dir.path().to_string_lossy().into_owned(),
            ..Config::default()
        };
        let current = update_cache(&cfg, false);
        write_navigators(&cfg, &current);
        let nav = fs::read_to_string(dir.path().join("navigator.md")).unwrap();
        assert!(
            nav.ends_with("## TREE\n\n└── src/\n    ├── deep/\n    │   └── a.rs  deep L9\n    └── b.rs  deep L10\n"),
            "{}",
            nav
        );
    }

    #[test]
    fn force_rebuilds_a_preexisting_symbol_index() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = Config {
            root: dir.path().to_string_lossy().into_owned(),
            ..Config::default()
        };
        fs::write(dir.path().join("sample.rs"), "fn sample() {}\n").unwrap();
        let mut stale = update_cache(&cfg, false);
        stale.symbols.clear();
        cache::save_cache(&project_path(&cfg, ".navguard_cache.json"), &stale);
        let rebuilt = update_cache(&cfg, true);
        assert!(rebuilt
            .symbols
            .iter()
            .any(|symbol| symbol.name.contains("sample")));
    }
}

#[cfg(test)]
mod patch_tests {
    use super::*;

    fn root_with(name: &str, content: &[u8]) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("root")).unwrap();
        fs::write(dir.path().join("root").join(name), content).unwrap();
        dir
    }

    fn apply_ok(patch: Result<Patch, String>) {
        assert_eq!(apply_patch(Ok(patch.unwrap()), false), 0);
    }

    fn read(dir: &tempfile::TempDir, name: &str) -> String {
        fs::read_to_string(dir.path().join("root").join(name)).unwrap()
    }

    #[test]
    fn splits_lines_with_their_breaks() {
        assert_eq!(
            split_keep_ends("a\r\nb\rc\nd"),
            [("a", "\r\n"), ("b", "\r"), ("c", "\n"), ("d", "")]
        );
        assert!(split_keep_ends("").is_empty());
        assert_eq!(split_keep_ends("\n"), [("", "\n")]);
    }

    #[test]
    fn replace_keeps_crlf_and_bom_and_fails_when_missing() {
        let dir = root_with("a.txt", b"\xEF\xBB\xBFone\r\ntwo\r\nthree\r\n");
        let root = dir.path().join("root");
        apply_ok(patch_replace(&root, "a.txt", "one\ntwo", "1\n2"));
        let bytes = fs::read(root.join("a.txt")).unwrap();
        assert_eq!(bytes, b"\xEF\xBB\xBF1\r\n2\r\nthree\r\n");
        let error = patch_replace(&root, "a.txt", "NOPE", "x").unwrap_err();
        assert!(error.contains("not found"), "{}", error);
        assert!(patch_replace(&root, "a.txt", "", "x").is_err());
        assert_eq!(
            apply_patch(patch_replace(&root, "a.txt", "NOPE", "x"), false),
            1
        );
    }

    #[test]
    fn insert_and_delete_keep_line_breaks() {
        let dir = root_with("a.txt", b"a\r\nb\r\nc");
        let root = dir.path().join("root");
        apply_ok(patch_insert(&root, "a.txt", 2, "x"));
        assert_eq!(read(&dir, "a.txt"), "a\r\nx\r\nb\r\nc");
        apply_ok(patch_insert(&root, "a.txt", 5, "end"));
        assert_eq!(read(&dir, "a.txt"), "a\r\nx\r\nb\r\nc\r\nend");
        apply_ok(patch_delete(&root, "a.txt", 5, None));
        assert_eq!(read(&dir, "a.txt"), "a\r\nx\r\nb\r\nc");
        apply_ok(patch_delete(&root, "a.txt", 1, Some(2)));
        assert_eq!(read(&dir, "a.txt"), "b\r\nc");
        assert!(patch_insert(&root, "a.txt", 0, "x").is_err());
        assert!(patch_insert(&root, "a.txt", 4, "x").is_err());
        assert!(patch_delete(&root, "a.txt", 3, None).is_err());
        assert!(patch_delete(&root, "a.txt", 2, Some(1)).is_err());

        let dir = root_with("b.txt", b"one\ntwo\n");
        let root = dir.path().join("root");
        apply_ok(patch_insert(&root, "b.txt", 3, "three"));
        assert_eq!(read(&dir, "b.txt"), "one\ntwo\nthree\n");
        apply_ok(patch_delete(&root, "b.txt", 3, None));
        assert_eq!(read(&dir, "b.txt"), "one\ntwo\n");
        let dir = root_with("c.txt", b"");
        let root = dir.path().join("root");
        apply_ok(patch_insert(&root, "c.txt", 1, "first"));
        assert_eq!(read(&dir, "c.txt"), "first\n");
    }

    #[test]
    fn lone_cr_lines_are_numbered_like_lines() {
        let dir = root_with("mac.txt", b"a\rb\rc\r");
        let root = dir.path().join("root");
        apply_ok(patch_delete(&root, "mac.txt", 2, None));
        assert_eq!(read(&dir, "mac.txt"), "a\rc\r");
    }

    #[test]
    fn dry_run_writes_nothing_and_previews() {
        let dir = root_with("a.txt", b"one\ntwo\nthree\n");
        let root = dir.path().join("root");
        let patch = patch_replace(&root, "a.txt", "two", "2").unwrap();
        assert_eq!(
            preview(&patch.before, &patch.after, 40),
            "-      2| two\n+      2| 2\n"
        );
        assert_eq!(apply_patch(Ok(patch), true), 0);
        assert_eq!(read(&dir, "a.txt"), "one\ntwo\nthree\n");
    }

    #[test]
    fn refuses_paths_outside_the_root_and_binary_files() {
        let dir = root_with("a.txt", b"x\n");
        fs::write(dir.path().join("outside.txt"), "secret\n").unwrap();
        let root = dir.path().join("root");
        let outside = dir.path().join("outside.txt");
        for path in [
            "../outside.txt".to_string(),
            outside.to_string_lossy().into_owned(),
        ] {
            let error = patch_replace(&root, &path, "secret", "changed").unwrap_err();
            assert!(error.contains("outside the project root"), "{}", error);
        }
        assert_eq!(fs::read_to_string(&outside).unwrap(), "secret\n");
        fs::write(root.join("b.bin"), b"\0\x01").unwrap();
        assert!(patch_insert(&root, "b.bin", 1, "x")
            .unwrap_err()
            .contains("binary"));
        fs::write(root.join("w.txt"), b"caf\xE9\n").unwrap();
        assert!(patch_insert(&root, "w.txt", 1, "x")
            .unwrap_err()
            .contains("UTF-8"));
    }
}
