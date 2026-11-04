// Each test crate uses a different subset of these helpers.
#![allow(dead_code)]

use any_txt_parser::limits::CliOptions;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug)]
pub struct RenderedBlock {
    pub id: String,
    pub text: String,
}

pub fn blocks(document: &str) -> Vec<RenderedBlock> {
    let mut result = Vec::new();
    let mut current: Option<RenderedBlock> = None;
    for line in document.split_inclusive('\n') {
        if let Some(id) = line
            .strip_prefix("<!-- BLOCK: ")
            .and_then(|line| line.strip_suffix(" START -->\n"))
        {
            assert!(current.is_none(), "nested block anchor");
            current = Some(RenderedBlock {
                id: id.to_string(),
                text: String::new(),
            });
        }
        if let Some(block) = &mut current {
            block.text.push_str(line);
            if line == format!("<!-- BLOCK: {} END -->\n", block.id) {
                result.push(current.take().unwrap());
            }
        }
    }
    assert!(current.is_none(), "unterminated block anchor");
    result
}

pub fn data_items(block: &RenderedBlock) -> Vec<String> {
    let data = block
        .text
        .split_once("\n### Data\n\n")
        .expect("data section missing")
        .1
        .split_once("\n<!-- BLOCK: ")
        .expect("end anchor missing")
        .0;
    let mut items: Vec<String> = Vec::new();
    for line in data.lines() {
        if line.starts_with("### Errors") || line.starts_with("### Warnings") {
            break;
        }
        if let Some(value) = line.strip_prefix("- ") {
            items.push(value.to_string());
        } else if let Some(value) = line.strip_prefix("    ") {
            let item = items.last_mut().expect("continuation without an item");
            item.push('\n');
            item.push_str(value);
        } else {
            assert!(
                line.is_empty()
                    || matches!(
                        line,
                        "(no data)" | "(error)" | "no matches found" | "rule not applicable"
                    )
                    || line.starts_with("(in the continuation part"),
                "unexpected data line: {line:?}"
            );
        }
    }
    items
}

pub fn assert_total_characters(document: &str) {
    let reported: usize = document
        .lines()
        .find_map(|line| line.strip_prefix("Total output characters: "))
        .expect("character counter missing")
        .parse()
        .unwrap();
    assert_eq!(reported, document.chars().count());
}

/// The cells of a Markdown table row: split at every `|` that is not
/// escaped as `\|`, trimmed, with the empty cells before the first and
/// after the last `|`.
pub fn table_cells(row: &str) -> Vec<String> {
    let mut cells = vec![String::new()];
    let mut chars = row.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\\' if chars.peek() == Some(&'|') => {
                cells.last_mut().unwrap().push_str("\\|");
                chars.next();
            }
            '|' => cells.push(String::new()),
            c => cells.last_mut().unwrap().push(c),
        }
    }
    cells.iter().map(|c| c.trim().to_string()).collect()
}

pub fn assert_index_lines(document: &str) {
    let lines: Vec<&str> = document.lines().collect();
    let rendered = blocks(document);
    let index = document
        .split_once("## Block Index\n")
        .expect("block index missing")
        .1
        .split_once("### Block Anchors\n")
        .expect("block anchors missing")
        .0;
    let block_row = regex::Regex::new(r"^\| B[0-9]+(?:-[0-9]+)? \|").unwrap();
    let rows: Vec<Vec<String>> = index
        .lines()
        .filter(|line| block_row.is_match(line))
        .map(table_cells)
        .collect();
    assert_eq!(rows.len(), rendered.len(), "each block needs one index row");
    for (row, block) in rows.iter().zip(&rendered) {
        assert_eq!(row.len(), 11, "unexpected index columns");
        assert_eq!(row[1], block.id);
        assert_eq!(row[9], block.id);
        let size = block.text.chars().count() * 10 / 1024;
        assert_eq!(row[6], format!("{}.{} KB", size / 10, size % 10));
        let start: usize = row[7].parse().expect("numeric start line required");
        let end: usize = row[8].parse().expect("numeric end line required");
        assert!(start > 0 && start < end);
        assert_eq!(
            lines[start - 1],
            format!("<!-- BLOCK: {} START -->", block.id)
        );
        assert_eq!(lines[end - 1], format!("<!-- BLOCK: {} END -->", block.id));
        assert_eq!(block.text.lines().count(), end - start + 1);
        assert!(document.contains(&format!(
            "- {} → BLOCK:{} (lines {}-{})",
            block.id, block.id, start, end
        )));
    }
}

pub fn normalize_volatile(document: &str) -> String {
    document
        .split_inclusive('\n')
        .map(|line| {
            for field in ["Generation", "Updated", "Started", "Completed"] {
                if line.starts_with(&format!("{field}: ")) {
                    return format!("{field}: <volatile>\n");
                }
            }
            line.to_string()
        })
        .collect()
}

/// Placeholder for values that change between runs of the same input.
pub const VOLATILE: &str = "<volatile>";

/// Placeholder for the parser version, so a version bump does not change
/// snapshots.
pub const VERSION: &str = "<version>";

/// `Generation: NNNNNNNN`.
const GENERATION_FORMAT: &str = r"^[0-9]{8}$";
/// `2026-10-18T10:00:00Z`, the format of `Updated`, `Started`, `Completed`
/// and the Modified column.
const TIMESTAMP_FORMAT: &str = r"^[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}Z$";
/// `x.y.z`, optionally with a SemVer pre-release or build suffix.
const VERSION_FORMAT: &str = r"^[0-9]+\.[0-9]+\.[0-9]+(?:[-+][0-9A-Za-z.+-]+)?$";

fn has_format(value: &str, format: &str) -> bool {
    regex::Regex::new(format).unwrap().is_match(value)
}

/// Snapshot form of a published document: only the parts that change
/// between runs of the same input become placeholders. Before `## Applied
/// Template` (the header and `## Execution`) these are the `Generation`,
/// `Updated`, `Started` and `Completed` lines and the `Parser version`
/// lines; in the `## Files` table, the Modified column. A value is replaced
/// only when it has the expected format (eight digits, `YYYY-MM-DDTHH:MM:SSZ`,
/// `x.y.z`), so a malformed one stays in the text and fails the comparison.
/// Every other byte, including line endings, is kept.
pub fn normalize_snapshot(document: &str) -> String {
    let mut in_prelude = true;
    let mut in_files = false;
    let mut out = String::with_capacity(document.len());
    for line in document.split_inclusive('\n') {
        if let Some(heading) = line.strip_prefix("## ") {
            let heading = heading.trim_end_matches('\n');
            in_files = heading == "Files";
            if heading == "Applied Template" {
                in_prelude = false;
            }
            out.push_str(line);
            continue;
        }
        if in_prelude && let Some(replaced) = normalize_prelude_line(line) {
            out.push_str(&replaced);
            continue;
        }
        if in_files && let Some(row) = normalize_files_row(line) {
            out.push_str(&row);
            continue;
        }
        out.push_str(line);
    }
    out
}

/// A volatile header/Execution line with its value replaced, or `None` when
/// the line is not one of them or its value is malformed.
fn normalize_prelude_line(line: &str) -> Option<String> {
    let (field, value) = line.strip_suffix('\n')?.split_once(": ")?;
    let (format, placeholder) = match field {
        "Parser version" => (VERSION_FORMAT, VERSION),
        "Generation" => (GENERATION_FORMAT, VOLATILE),
        "Updated" | "Started" | "Completed" => (TIMESTAMP_FORMAT, VOLATILE),
        _ => return None,
    };
    has_format(value, format).then(|| format!("{field}: {placeholder}\n"))
}

/// `| path | size | modified | format | status | encoding | hash |` with a
/// well-formed Modified value replaced. Columns are taken from the right,
/// because only the path may contain ` | ` (as ` \| `). `None` for the
/// header, the separator, other lines and a malformed Modified value.
fn normalize_files_row(line: &str) -> Option<String> {
    if line.starts_with("| Path | Size | Modified |") {
        return None;
    }
    let body = line.strip_prefix("| ")?.strip_suffix(" |\n")?;
    let mut columns: Vec<&str> = body.rsplitn(7, " | ").collect();
    // Reversed order: hash, encoding, status, format, modified, size, path.
    if columns.len() != 7 || !has_format(columns[4], TIMESTAMP_FORMAT) {
        return None;
    }
    columns[4] = VOLATILE;
    columns.reverse();
    Some(format!("| {} |\n", columns.join(" | ")))
}

/// Values of the `field: value` lines of the header and `## Execution`
/// (everything before `## Files` or `## Applied Template`).
pub fn prelude_values<'a>(document: &'a str, field: &str) -> Vec<&'a str> {
    let prefix = format!("{field}: ");
    document
        .lines()
        .take_while(|line| *line != "## Files" && *line != "## Applied Template")
        .filter_map(|line| line.strip_prefix(&prefix))
        .collect()
}

/// What [`normalize_snapshot`] hides must still be consistent in the raw
/// document: the header and `## Execution` agree on `Generation` and
/// `Parser version`, the version is the crate's, `Updated` is the
/// completion time and `Started` is not after `Completed` (the timestamps
/// have a fixed width, so they compare as strings).
pub fn assert_volatile_fields_consistent(document: &str) {
    let single = |field: &str, count: usize| {
        let values = prelude_values(document, field);
        assert_eq!(values.len(), count, "expected {count} '{field}:' line(s)");
        assert!(
            values.iter().all(|value| *value == values[0]),
            "'{field}' differs between the header and Execution: {values:?}"
        );
        values[0]
    };
    assert_eq!(
        single("Parser version", 2),
        any_txt_parser::PARSER_VERSION,
        "document reports another parser version"
    );
    single("Generation", 2);
    let started = single("Started", 1);
    let completed = single("Completed", 1);
    assert_eq!(single("Updated", 1), completed, "Updated is not Completed");
    assert!(
        started <= completed,
        "Started {started} is after Completed {completed}"
    );
}

/// `0.2 KB`: the block size format of the Block Index.
fn kb(chars: usize) -> String {
    let tenths = chars * 10 / 1024;
    format!("{}.{} KB", tenths / 10, tenths % 10)
}

/// The document with what it reports about its own size recomputed for its
/// current text: the Size column of the Block Index and `Total output
/// characters`. A snapshot runner checks both on the raw document
/// ([`assert_index_lines`], [`assert_total_characters`]) and calls this
/// after replacing volatile values: placeholders differ in length from the
/// values they replace (parser version, absolute paths), and without the
/// recount every version of another length or temp directory would change
/// these numbers. The result again passes both checks.
pub fn recount_sizes(document: &str) -> String {
    let sizes: std::collections::HashMap<String, String> = blocks(document)
        .iter()
        .map(|block| (block.id.clone(), kb(block.text.chars().count())))
        .collect();
    let mut in_index = false;
    let mut total_line = None;
    let mut out = String::with_capacity(document.len());
    for line in document.split_inclusive('\n') {
        if line.starts_with("## ") || line.starts_with("### ") {
            in_index = line == "## Block Index\n";
        }
        if in_index && let Some(row) = resize_index_row(line, &sizes) {
            out.push_str(&row);
            continue;
        }
        if total_line.is_none() && line.starts_with("Total output characters: ") {
            total_line = Some(out.len());
        }
        out.push_str(line);
    }
    let Some(start) = total_line else {
        return out;
    };
    let prefix = "Total output characters: ";
    let digits_start = start + prefix.len();
    let digits_len = out[digits_start..]
        .bytes()
        .take_while(u8::is_ascii_digit)
        .count();
    if digits_len == 0 || !out[digits_start + digits_len..].starts_with('\n') {
        return out;
    }
    // The count includes its own digits: find n = rest + digits(n).
    let rest = out.chars().count() - digits_len;
    let mut width = 1;
    let total = loop {
        let candidate = rest + width;
        if candidate.to_string().len() == width {
            break candidate;
        }
        width += 1;
    };
    out.replace_range(digits_start..digits_start + digits_len, &total.to_string());
    out
}

/// A Block Index row with its Size column (the fourth from the right) set
/// to the size of the block it names; `None` for other lines.
fn resize_index_row(
    line: &str,
    sizes: &std::collections::HashMap<String, String>,
) -> Option<String> {
    let body = line.strip_prefix("| ")?.strip_suffix(" |\n")?;
    let (id, _) = body.split_once(" | ")?;
    let size = sizes.get(id)?;
    let mut columns: Vec<&str> = body.rsplitn(5, " | ").collect();
    if columns.len() != 5 {
        return None;
    }
    // Reversed order: anchor, end, start, size, the rest.
    columns[3] = size;
    columns.reverse();
    Some(format!("| {} |\n", columns.join(" | ")))
}

static FIXTURE_ID: AtomicU64 = AtomicU64::new(0);

pub struct Fixture {
    pub dir: PathBuf,
    pub cli: CliOptions,
}

impl Fixture {
    pub fn new(name: &str, template: &str) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "atp_stabilization_{}_{}_{}",
            std::process::id(),
            name,
            FIXTURE_ID.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(dir.join("sources")).unwrap();
        std::fs::write(dir.join("tpl.yml"), template).unwrap();
        let cli = CliOptions {
            input: dir.join("sources").to_string_lossy().into_owned(),
            template: dir.join("tpl.yml"),
            output: Some(dir.join("out.md")),
            format: None,
            encoding: None,
            dry_run: false,
            verbose: false,
            quiet: true,
            follow_symlinks: false,
            hash: false,
            max_file_size: None,
            max_memory: None,
            max_files: None,
            max_matches: None,
            max_output_size: None,
            max_block_chars: None,
            raw_args: vec!["--input".into(), "sources".into()],
        };
        Self { dir, cli }
    }

    pub fn source(&self, name: &str, bytes: impl AsRef<[u8]>) {
        std::fs::write(self.dir.join("sources").join(name), bytes).unwrap();
    }

    pub fn document(&self) -> String {
        std::fs::read_to_string(self.cli.output.as_ref().unwrap()).unwrap()
    }

    pub fn seed_output(&self) -> Vec<u8> {
        let old = b"Existing output must survive a failed run.\nGeneration: 00000009\n";
        std::fs::write(self.cli.output.as_ref().unwrap(), old).unwrap();
        old.to_vec()
    }

    pub fn assert_output_unchanged(&self, previous: &[u8]) {
        assert_eq!(
            std::fs::read(self.cli.output.as_ref().unwrap()).unwrap(),
            previous
        );
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}
