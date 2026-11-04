//! Golden snapshots of the complete published document.
//!
//! Every directory under `tests/snapshots/` is one case:
//! - `template.yml`: the template, passed as `-t template.yml`;
//! - `sources/`: the input tree, passed as `-i sources`;
//! - `args.txt` (optional): extra CLI arguments, one per line; blank lines
//!   and lines starting with `#` are skipped. An `-i`, `-t` or `-o` there
//!   replaces the default one. `<CASE>` in an argument becomes the absolute
//!   path of the temporary case directory (with native separators), for
//!   cases about absolute paths;
//! - `expected.md`: the normalized document (`-o out.md` unless `args.txt`
//!   names another output);
//! - `expected_exit.txt`: the exit code.
//!
//! The case is copied to a fresh temporary directory and the real binary
//! runs there, by default with relative paths. Only volatile parts are
//! normalized, see [`normalize`]: timestamps, the generation and the parser
//! version (each only when well-formed, and checked for consistency on the
//! raw document), the Modified column, the temporary case directory (`<CASE>`)
//! and path separators of the case's own files. Placeholders differ in
//! length from what they replace, so the numbers the document reports about
//! its own size (`Total output characters`, the Size column of the Block
//! Index) are checked on the raw document and then recomputed for the
//! normalized one; snapshots stay valid documents and do not change when the
//! version string or the temp directory changes length.
//!
//! Files are compared byte for byte: `.gitattributes` keeps
//! `tests/snapshots/**` free of line-ending conversion, because some sources
//! contain CRLF or a lone CR on purpose.
//!
//! `UPDATE_SNAPSHOTS=1 cargo test --test snapshots` rewrites the expected
//! files; review every changed `expected.md` before committing it.
//!
//! Known issues frozen by these snapshots (regenerate the snapshots once they
//! are fixed) are described in the `template.yml` of the case ("Known issue
//! frozen here").

mod support;

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

/// One `#[test]` per case directory, plus the list of all of them.
macro_rules! snapshot_cases {
    ($($case:ident),* $(,)?) => {
        $(
            #[test]
            fn $case() {
                check_case(stringify!($case));
            }
        )*
        const CASES: &[&str] = &[$(stringify!($case)),*];
    };
}

snapshot_cases![
    absolute_paths,
    all_files_fail,
    anchor_text,
    cli_format_override,
    dedup_sort,
    hash_glob_input,
    html_extract_modes,
    json_jsonpath_filters,
    markdown_nodes,
    mixed_when_format,
    no_matches_not_applicable,
    partial_unparsable,
    split_hard_line,
    split_unicode_items,
    text_logs_context,
    toml_tables,
    truncation_limits,
    xml_namespaces_dtd,
    yaml_multidoc_merge,
];

/// Placeholder for the temporary directory a case runs in.
const CASE_DIR: &str = "<CASE>";

fn cases_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("snapshots")
}

#[test]
fn every_case_directory_has_a_test() {
    let mut directories: Vec<String> = std::fs::read_dir(cases_root())
        .expect("tests/snapshots is missing")
        .map(|entry| entry.unwrap())
        .filter(|entry| entry.file_type().unwrap().is_dir())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    directories.sort();
    let mut listed: Vec<String> = CASES.iter().map(|case| case.to_string()).collect();
    listed.sort();
    assert_eq!(
        directories, listed,
        "list every directory of tests/snapshots in snapshot_cases!"
    );
}

#[test]
fn normalization_touches_only_volatile_parts() {
    let document = concat!(
        "# any_txt_parser output\n\n",
        "Triangle: [0,1,0]\n",
        "Generation: 00000007\n",
        "Updated: 2026-10-18T10:00:00Z\n\n",
        "Parser version: 9.8.7\n\n",
        "## Execution\n\n",
        "Parser version: 10.0.0-rc.1\n",
        "Started: 2026-10-18T09:59:59Z\n",
        "Completed: 2026-10-18T10:00:00Z\n",
        "Input:\n",
        "<CASEDIR>\\sources\n",
        "\n## Files\n\n",
        "| Path | Size | Modified | Format | Status | Encoding | Hash |\n",
        "|---|---:|---|---|---|---|---|\n",
        "| sources\\a \\| b.json | 12 | 2026-10-18T09:00:00Z | json | success | utf-8 | - |\n",
        "\n## Applied Template\n\n",
        "Selector:\n",
        "Generation: 1\n",
        "- sources\\a | b.json\r\n",
    );
    let case_dir = Path::new("<CASEDIR>");
    let sources = vec![PathBuf::from("sources").join("a | b.json")];
    let expected = concat!(
        "# any_txt_parser output\n\n",
        "Triangle: [0,1,0]\n",
        "Generation: <volatile>\n",
        "Updated: <volatile>\n\n",
        "Parser version: <version>\n\n",
        "## Execution\n\n",
        "Parser version: <version>\n",
        "Started: <volatile>\n",
        "Completed: <volatile>\n",
        "Input:\n",
        "<CASE>/sources\n",
        "\n## Files\n\n",
        "| Path | Size | Modified | Format | Status | Encoding | Hash |\n",
        "|---|---:|---|---|---|---|---|\n",
        // A table cell writes `|` as `\|`: only list lines name the file
        // as it is, so only they get `/`.
        "| sources\\a \\| b.json | 12 | <volatile> | json | success | utf-8 | - |\n",
        "\n## Applied Template\n\n",
        "Selector:\n",
        "Generation: 1\n",
        "- sources/a | b.json\r\n",
    );
    let normalized = normalize(document, case_dir, &sources);
    if cfg!(windows) {
        assert_eq!(normalized, expected);
    } else {
        // Backslashes are file name characters there, not separators.
        assert_eq!(
            normalized,
            expected
                .replace("<CASE>/sources", "<CASE>\\sources")
                .replace("sources/a | b", "sources\\a | b")
        );
    }
}

#[test]
fn normalization_keeps_malformed_volatile_values() {
    // Each value has the wrong format, so it stays and fails the comparison.
    let document = concat!(
        "Generation: 7\n",
        "Updated: 2026-10-18 10:00:00\n",
        "Parser version: \n",
        "Started: 2026-10-18T10:00:00+03:00\n",
        "Completed: unknown\n",
        "Parser version: 1.0\n",
        "\n## Files\n\n",
        "| Path | Size | Modified | Format | Status | Encoding | Hash |\n",
        "|---|---:|---|---|---|---|---|\n",
        "| sources/a.json | 12 | unknown | json | success | utf-8 | - |\n",
        "| sources/b.json | 12 | 2026-10-18T09:00:00Z | json | success | utf-8 | - |\n",
    );
    let expected = document.replace("| 2026-10-18T09:00:00Z |", "| <volatile> |");
    assert_eq!(support::normalize_snapshot(document), expected);
}

#[test]
fn recount_makes_size_claims_match_the_text() {
    let document = concat!(
        "Total output characters: 1\n",
        "\n## Block Index\n\n",
        "| Block | Source | Rule | Key | Items | Size | Start | End | Anc |\n",
        "|---|---|---|---|---:|---:|---:|---:|---|\n",
        "| B0001 | a | r | k | 1 | 9.9 KB | 15 | 17 | B0001 |\n",
        "\n### Block Anchors\n\n",
        "- B0001 → BLOCK:B0001 (lines 15-17)\n",
        "\n## Data Blocks\n\n",
        "<!-- BLOCK: B0001 START -->\n",
        "x\n",
        "<!-- BLOCK: B0001 END -->\n",
    );
    let recounted = support::recount_sizes(document);
    assert_eq!(
        recounted,
        document
            .replace("characters: 1\n", "characters: 337\n")
            .replace("| 9.9 KB |", "| 0.0 KB |")
    );
    support::assert_total_characters(&recounted);
    support::assert_index_lines(&recounted);
    // Padding moves the count across powers of ten, where its own width
    // changes.
    for padding in [0, 1, 9_960, 9_966, 9_967, 9_970, 99_960, 99_966] {
        let padded = format!("Total output characters: 123\n{}", "й".repeat(padding));
        support::assert_total_characters(&support::recount_sizes(&padded));
    }
}

/// The document with the volatile parts replaced: the fields
/// [`support::normalize_snapshot`] covers (timestamps, generation, parser
/// version, Modified column), the temporary case directory, and on Windows
/// the path separators inside the paths of the copied case files. The size
/// claims are then recomputed for the result, see
/// [`support::recount_sizes`].
fn normalize(document: &str, case_dir: &Path, files: &[PathBuf]) -> String {
    let mut text = support::normalize_snapshot(document);
    let native = case_dir.to_string_lossy();
    for spelling in [native.to_string(), native.replace('\\', "/")] {
        text = text.replace(&spelling, CASE_DIR);
    }
    if cfg!(windows) {
        text = text.replace(&format!("{CASE_DIR}\\"), &format!("{CASE_DIR}/"));
        // `sources\sub\a.json` (walkdir joins with `\`; a glob may mix both
        // separators) becomes `sources/sub/a.json`. Only paths of files the
        // case really has are rewritten, so escaped anchor text and data with
        // backslashes stay as they are.
        for file in files {
            let parts: Vec<String> = file
                .components()
                .map(|part| regex::escape(&part.as_os_str().to_string_lossy()))
                .collect();
            if parts.len() < 2 {
                continue;
            }
            let pattern = regex::Regex::new(&parts.join(r"[\\/]")).unwrap();
            let portable = file
                .components()
                .map(|part| part.as_os_str().to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join("/");
            text = pattern
                .replace_all(&text, regex::NoExpand(&portable))
                .into_owned();
        }
    }
    support::recount_sizes(&text)
}

static RUN_ID: AtomicU64 = AtomicU64::new(0);

fn check_case(case: &str) {
    let source = cases_root().join(case);
    assert!(
        source.is_dir(),
        "missing case directory {}",
        source.display()
    );
    let work = std::env::temp_dir().join(format!(
        "atp_snapshots_{}_{}_{}",
        std::process::id(),
        case,
        RUN_ID.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&work);
    std::fs::create_dir_all(&work).unwrap();
    std::fs::copy(source.join("template.yml"), work.join("template.yml"))
        .unwrap_or_else(|e| panic!("{case}: template.yml: {e}"));
    if source.join("sources").is_dir() {
        copy_tree(&source.join("sources"), &work.join("sources"));
    }
    let mut files = Vec::new();
    list_files(&work, Path::new(""), &mut files);
    files.sort();

    let args = case_args(&source, &work);
    let output = Command::new(env!("CARGO_BIN_EXE_any_txt_parser"))
        .args(&args)
        .current_dir(&work)
        .output()
        .expect("cannot start the any_txt_parser binary");
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let code = output
        .status
        .code()
        .unwrap_or_else(|| panic!("{case}: terminated by a signal\n{stderr}"));
    let published = work.join(output_arg(&args));
    let raw = std::fs::read_to_string(&published).unwrap_or_else(|e| {
        panic!(
            "{case}: no document at {} (exit {code}): {e}\nstderr:\n{stderr}",
            published.display()
        )
    });
    // The document's own claims about itself hold before normalization, and
    // what normalization hides is consistent.
    support::assert_total_characters(&raw);
    support::assert_index_lines(&raw);
    support::assert_volatile_fields_consistent(&raw);
    assert_eq!(
        support::prelude_values(&raw, "Generation")[0],
        "00000001",
        "{case}: the first document in a fresh directory has generation 1"
    );
    let actual = normalize(&raw, &work, &files);
    support::assert_total_characters(&actual);
    support::assert_index_lines(&actual);
    let actual_exit = format!("{code}\n");

    let expected_path = source.join("expected.md");
    let exit_path = source.join("expected_exit.txt");
    if std::env::var_os("UPDATE_SNAPSHOTS").is_some_and(|value| value == "1") {
        std::fs::write(&expected_path, &actual).unwrap();
        std::fs::write(&exit_path, &actual_exit).unwrap();
        let _ = std::fs::remove_dir_all(&work);
        eprintln!("{case}: snapshot updated (exit {code})");
        return;
    }

    let expected = std::fs::read(&expected_path)
        .map(|bytes| String::from_utf8(bytes).expect("expected.md is not UTF-8"))
        .unwrap_or_else(|e| {
            panic!(
                "{case}: {}: {e}; run with UPDATE_SNAPSHOTS=1 to create it",
                expected_path.display()
            )
        });
    let expected_exit = std::fs::read_to_string(&exit_path)
        .unwrap_or_else(|e| panic!("{case}: {}: {e}", exit_path.display()));
    let mut problems = Vec::new();
    if expected_exit.trim() != actual_exit.trim() {
        problems.push(format!(
            "exit code: expected {}, got {code}",
            expected_exit.trim()
        ));
    }
    if expected != actual {
        let kept = work.join("actual.md");
        std::fs::write(&kept, &actual).unwrap();
        problems.push(format!(
            "document differs from {} (normalized actual kept at {}):\n{}",
            expected_path.display(),
            kept.display(),
            diff(&expected, &actual)
        ));
    }
    if problems.is_empty() {
        let _ = std::fs::remove_dir_all(&work);
        return;
    }
    panic!(
        "snapshot '{case}' failed (args: {args:?}); UPDATE_SNAPSHOTS=1 rewrites it\n{}\nstderr:\n{stderr}",
        problems.join("\n")
    );
}

/// `-i sources -t template.yml -o out.md` followed by `args.txt`; an `-i`,
/// `-t` or `-o` in `args.txt` replaces the default, and `<CASE>` in an
/// argument becomes the case directory `work`.
fn case_args(case: &Path, work: &Path) -> Vec<String> {
    let extra: Vec<String> = match std::fs::read_to_string(case.join("args.txt")) {
        Ok(text) => text
            .lines()
            .map(|line| line.strip_suffix('\r').unwrap_or(line))
            .filter(|line| !line.trim().is_empty() && !line.starts_with('#'))
            .map(|line| expand_case_dir(line, work))
            .collect(),
        Err(_) => Vec::new(),
    };
    let given = |short: &str, long: &str| {
        extra
            .iter()
            .any(|arg| arg == short || arg == long || arg.starts_with(&format!("{long}=")))
    };
    let mut args = Vec::new();
    for (short, long, value) in [
        ("-i", "--input", "sources"),
        ("-t", "--template", "template.yml"),
        ("-o", "--output", "out.md"),
    ] {
        if !given(short, long) {
            args.push(short.to_string());
            args.push(value.to_string());
        }
    }
    args.extend(extra);
    args
}

/// `arg` with `<CASE>` replaced by `work`; on Windows such an argument gets
/// native separators throughout (`<CASE>/sources` → `C:\...\sources`).
fn expand_case_dir(arg: &str, work: &Path) -> String {
    if !arg.contains(CASE_DIR) {
        return arg.to_string();
    }
    let expanded = arg.replace(CASE_DIR, &work.to_string_lossy());
    if cfg!(windows) {
        expanded.replace('/', "\\")
    } else {
        expanded
    }
}

/// The value of the last `-o`/`--output` argument.
fn output_arg(args: &[String]) -> &str {
    let mut output = "out.md";
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        if arg == "-o" || arg == "--output" {
            if let Some(value) = iter.next() {
                output = value;
            }
        } else if let Some(value) = arg.strip_prefix("--output=") {
            output = value;
        }
    }
    output
}

fn copy_tree(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), &target).unwrap();
        }
    }
}

/// Paths of all files under `root`, relative to it.
fn list_files(root: &Path, relative: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(root.join(relative)).unwrap() {
        let entry = entry.unwrap();
        let path = relative.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            list_files(root, &path, out);
        } else {
            out.push(path);
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Edit {
    Same,
    Removed,
    Added,
}

/// Line edit script from `old` to `new`: the common prefix and suffix, and
/// between them a longest common subsequence of lines, removals before
/// additions. A middle part too large for the quadratic table is shown as
/// removed and added as a whole.
fn edit_script(old: &[&str], new: &[&str]) -> Vec<Edit> {
    const MAX_TABLE: usize = 16_000_000;
    let prefix = old.iter().zip(new).take_while(|(a, b)| a == b).count();
    let suffix = old[prefix..]
        .iter()
        .rev()
        .zip(new[prefix..].iter().rev())
        .take_while(|(a, b)| a == b)
        .count();
    let a = &old[prefix..old.len() - suffix];
    let b = &new[prefix..new.len() - suffix];
    let mut script = vec![Edit::Same; prefix];
    if (a.len() + 1).saturating_mul(b.len() + 1) > MAX_TABLE {
        script.extend(std::iter::repeat_n(Edit::Removed, a.len()));
        script.extend(std::iter::repeat_n(Edit::Added, b.len()));
    } else {
        // common[i * width + j]: length of the LCS of a[i..] and b[j..].
        let width = b.len() + 1;
        let mut common = vec![0u32; (a.len() + 1) * width];
        for i in (0..a.len()).rev() {
            for j in (0..b.len()).rev() {
                common[i * width + j] = if a[i] == b[j] {
                    common[(i + 1) * width + j + 1] + 1
                } else {
                    common[(i + 1) * width + j].max(common[i * width + j + 1])
                };
            }
        }
        let (mut i, mut j) = (0, 0);
        while i < a.len() || j < b.len() {
            if i < a.len() && j < b.len() && a[i] == b[j] {
                script.push(Edit::Same);
                i += 1;
                j += 1;
            } else if j == b.len()
                || (i < a.len() && common[(i + 1) * width + j] >= common[i * width + j + 1])
            {
                script.push(Edit::Removed);
                i += 1;
            } else {
                script.push(Edit::Added);
                j += 1;
            }
        }
    }
    script.extend(std::iter::repeat_n(Edit::Same, suffix));
    script
}

/// Unified diff of two texts by lines: hunks with three lines of context
/// (hunks whose context would touch are merged), at most eight hunks and 60
/// lines per hunk.
fn diff(expected: &str, actual: &str) -> String {
    const CONTEXT: usize = 3;
    const MAX_HUNKS: usize = 8;
    const MAX_HUNK_LINES: usize = 60;
    let old: Vec<&str> = expected.split_inclusive('\n').collect();
    let new: Vec<&str> = actual.split_inclusive('\n').collect();
    let script = edit_script(&old, &new);
    // positions[k]: old and new lines before script[k].
    let mut positions = Vec::with_capacity(script.len() + 1);
    let (mut o, mut n) = (0, 0);
    for edit in &script {
        positions.push((o, n));
        match edit {
            Edit::Same => (o, n) = (o + 1, n + 1),
            Edit::Removed => o += 1,
            Edit::Added => n += 1,
        }
    }
    positions.push((o, n));
    // Script ranges (first and last edit) of the hunks.
    let mut hunks: Vec<(usize, usize)> = Vec::new();
    for (k, _) in script
        .iter()
        .enumerate()
        .filter(|(_, edit)| **edit != Edit::Same)
    {
        match hunks.last_mut() {
            Some((_, last)) if k - *last <= 2 * CONTEXT + 1 => *last = k,
            _ => hunks.push((k, k)),
        }
    }
    let mut out = String::from("--- expected.md\n+++ actual\n");
    for (number, &(first, last)) in hunks.iter().enumerate() {
        if number == MAX_HUNKS {
            out.push_str(&format!(
                "... {} more hunk(s) not shown\n",
                hunks.len() - MAX_HUNKS
            ));
            break;
        }
        let start = first.saturating_sub(CONTEXT);
        let end = (last + 1 + CONTEXT).min(script.len());
        let (old_start, new_start) = positions[start];
        let (old_end, new_end) = positions[end];
        out.push_str(&format!(
            "@@ -{} +{} @@\n",
            hunk_range(old_start, old_end - old_start),
            hunk_range(new_start, new_end - new_start)
        ));
        for k in start..end {
            if k - start == MAX_HUNK_LINES {
                out.push_str(&format!("... {} more line(s) in this hunk\n", end - k));
                break;
            }
            let (o, n) = positions[k];
            let (mark, line) = match script[k] {
                Edit::Same => (' ', old[o]),
                Edit::Removed => ('-', old[o]),
                Edit::Added => ('+', new[n]),
            };
            out.push(mark);
            out.push_str(&show_line(line));
            out.push('\n');
        }
    }
    out
}

/// `start,count` of a hunk header, 1-based; an empty range names the line
/// before it, as in GNU diff.
fn hunk_range(start: usize, count: usize) -> String {
    if count == 0 {
        format!("{start},0")
    } else {
        format!("{},{count}", start + 1)
    }
}

/// A line without its `\n`, with carriage returns and tabs made visible and
/// a missing final newline marked.
fn show_line(line: &str) -> String {
    let (body, newline) = match line.strip_suffix('\n') {
        Some(body) => (body, true),
        None => (line, false),
    };
    let mut shown = body.replace('\r', "\\r").replace('\t', "\\t");
    if !newline {
        shown.push_str(" [no newline at end]");
    }
    shown
}

#[test]
fn diff_shows_changed_lines_with_context() {
    let expected = "a\nb\nc\nd\ne\nf\ng\n";
    let actual = "a\nb\nc\nD\r\ne\nf\ng";
    assert_eq!(
        diff(expected, actual),
        concat!(
            "--- expected.md\n+++ actual\n@@ -1,7 +1,7 @@\n",
            " a\n b\n c\n",
            "-d\n+D\\r\n",
            " e\n f\n",
            "-g\n+g [no newline at end]\n",
        )
    );
    assert_eq!(
        diff("x\ny\n", "x\nz\ny\n"),
        "--- expected.md\n+++ actual\n@@ -1,2 +1,3 @@\n x\n+z\n y\n"
    );
    assert_eq!(
        diff("x\nz\n", "x\n"),
        "--- expected.md\n+++ actual\n@@ -1,2 +1,1 @@\n x\n-z\n"
    );
    assert_eq!(
        diff("", "x\n"),
        "--- expected.md\n+++ actual\n@@ -0,0 +1,1 @@\n+x\n"
    );
    // Only three lines of trailing context.
    assert_eq!(
        diff("1\n2\n3\n4\n5\n6\n7\n", "1\nX\n3\n4\n5\n6\n7\n"),
        "--- expected.md\n+++ actual\n@@ -1,5 +1,5 @@\n 1\n-2\n+X\n 3\n 4\n 5\n"
    );
}

/// Lines `1` to `count`, with the given line numbers replaced.
fn numbered(count: usize, changes: &[(usize, &str)]) -> String {
    (1..=count)
        .map(|n| match changes.iter().find(|(line, _)| *line == n) {
            Some((_, text)) => format!("{text}\n"),
            None => format!("{n}\n"),
        })
        .collect()
}

#[test]
fn diff_shows_distant_changes_as_separate_hunks() {
    // Like a changed `Total output characters` line near the top and the
    // real change far below it: both are shown, the lines between are not.
    let expected = numbered(20, &[]);
    assert_eq!(
        diff(&expected, &numbered(20, &[(2, "X"), (18, "Y")])),
        concat!(
            "--- expected.md\n+++ actual\n",
            "@@ -1,5 +1,5 @@\n 1\n-2\n+X\n 3\n 4\n 5\n",
            "@@ -15,6 +15,6 @@\n 15\n 16\n 17\n-18\n+Y\n 19\n 20\n",
        )
    );
    // An inserted line does not turn the rest of the text into a change.
    let shifted = expected.replace("\n10\n", "\n10\nnew\n");
    assert_eq!(
        diff(&expected, &shifted),
        "--- expected.md\n+++ actual\n@@ -8,6 +8,7 @@\n 8\n 9\n 10\n+new\n 11\n 12\n 13\n"
    );
    // Hunks whose context would touch are merged: six unchanged lines
    // between two changes still form one hunk, seven do not.
    let hunks = |changes: &[(usize, &str)]| {
        diff(&expected, &numbered(20, changes))
            .matches("@@ -")
            .count()
    };
    assert_eq!(hunks(&[(4, "A"), (11, "B")]), 1);
    assert_eq!(hunks(&[(4, "A"), (12, "B")]), 2);
}

#[test]
fn diff_limits_the_number_of_hunks() {
    let texts: Vec<String> = (1..=20).map(|k| format!("changed {k}")).collect();
    let changes: Vec<(usize, &str)> = texts
        .iter()
        .enumerate()
        .map(|(k, text)| ((k + 1) * 10, text.as_str()))
        .collect();
    let shown = diff(&numbered(200, &[]), &numbered(200, &changes));
    assert_eq!(shown.matches("@@ -").count(), 8);
    assert!(
        shown.ends_with("... 12 more hunk(s) not shown\n"),
        "{shown}"
    );
}
