//! The three navigator files, written together from one scan.
//!
//! * `navigator.md` is read in full: the absolute root, the scan time,
//!   `Status: READY` and the tree. Every file in the tree points to its line
//!   in `navigator_deep.md` (`deep L12`).
//! * `navigator_deep.md`: one line per file, in tree order: the path
//!   relative to the root, size, lines, tokens, sha and the lines of its
//!   symbols in `navigator_deep2.md`.
//! * `navigator_deep2.md`: the symbols of each file with their line ranges,
//!   each section pointing back to the file's line in `navigator_deep.md`.
//!
//! The absolute root is written once, in `navigator.md`; deep and deep2
//! carry paths relative to it. The files are rendered in memory from one
//! cache (every file read once), so every pointer is an exact line number,
//! and only then written: deep2, deep, navigator, each replaced atomically.

use crate::cache::{NavguardCache, SymbolEntry};
use crate::scanner::FileInfo;
use std::collections::BTreeMap;
use std::path::Path;

pub const NAVIGATOR: &str = "navigator.md";
pub const DEEP: &str = "navigator_deep.md";
pub const DEEP2: &str = "navigator_deep2.md";

/// The `Status:` of complete navigators from one scan.
pub const READY: &str = "READY";

/// Longest symbol label shown in deep2, in characters.
const LABEL_CHARS: usize = 60;

/// The rendered navigator files.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Navigators {
    pub navigator: String,
    pub deep: String,
    pub deep2: String,
}

#[derive(Default)]
struct TreeDir {
    dirs: BTreeMap<String, TreeDir>,
    /// File name -> index into the file list.
    files: BTreeMap<String, usize>,
}

impl TreeDir {
    fn from_paths<S: AsRef<str>>(paths: &[S]) -> TreeDir {
        let mut root = TreeDir::default();
        for (index, path) in paths.iter().enumerate() {
            let normalized = path.as_ref().replace('\\', "/");
            let parts: Vec<&str> = normalized
                .split('/')
                .filter(|s| !s.is_empty() && *s != ".")
                .collect();
            let Some((file, dirs)) = parts.split_last() else {
                continue;
            };
            let mut node = &mut root;
            for dir in dirs {
                node = node.dirs.entry((*dir).to_string()).or_default();
            }
            node.files.insert((*file).to_string(), index);
        }
        root
    }

    /// Tree lines (`├──`/`└──`/`│`), directories first, then files, each
    /// sorted by name; directories end with `/`. Each line carries the index
    /// of its file (`None` for a directory).
    fn lines(&self) -> Vec<(String, Option<usize>)> {
        let mut out = Vec::new();
        self.render("", &mut out);
        out
    }

    fn render(&self, prefix: &str, out: &mut Vec<(String, Option<usize>)>) {
        let total = self.dirs.len() + self.files.len();
        let entries = self
            .dirs
            .iter()
            .map(|(name, sub)| (name, Some(sub), None))
            .chain(self.files.iter().map(|(name, &i)| (name, None, Some(i))));
        for (position, (name, sub, file)) in entries.enumerate() {
            let last = position + 1 == total;
            let branch = if last { "└── " } else { "├── " };
            match sub {
                Some(sub) => {
                    out.push((format!("{}{}{}/", prefix, branch, name), None));
                    let child = format!("{}{}", prefix, if last { "    " } else { "│   " });
                    sub.render(&child, out);
                }
                None => out.push((format!("{}{}{}", prefix, branch, name), file)),
            }
        }
    }
}

/// Renders a directory tree of `paths` (relative file paths, `/` or `\`
/// separated; leading `./` and empty segments are ignored). Directories
/// come first, then files, each sorted by name; directories end with `/`.
/// Only directories containing at least one listed file appear.
pub fn build_tree<S: AsRef<str>>(paths: &[S]) -> String {
    TreeDir::from_paths(paths)
        .lines()
        .into_iter()
        .map(|(line, _)| line + "\n")
        .collect()
}

/// `text` safe inside a Markdown table cell.
fn cell(text: &str) -> String {
    text.replace('|', "\\|")
}

fn label(text: &str) -> String {
    if text.chars().count() > LABEL_CHARS {
        format!("{}...", text.chars().take(LABEL_CHARS).collect::<String>())
    } else {
        text.to_string()
    }
}

/// The title and `Updated:` lines; `Root:` only in `navigator.md`.
fn header(title: &str, root: Option<&str>, updated: &str) -> Vec<String> {
    let mut lines = vec![format!("# {} — navguard generated", title), String::new()];
    if let Some(root) = root {
        lines.push(format!("Root: {}", root));
    }
    lines.push(format!("Updated: {}", updated));
    lines
}

/// The value of the `Updated:` line of a navigator file.
pub fn updated_of(text: &str) -> Option<&str> {
    text.lines()
        .take(8)
        .find_map(|line| line.strip_prefix("Updated: "))
}

/// The files listed in a `navigator_deep.md`: (path, sha) of every row.
pub fn deep_rows(text: &str) -> Vec<(String, String)> {
    text.lines()
        .filter_map(|line| {
            // `| path | bytes | lines | tokens | sha | symbols |`; only the
            // path may contain `|` (escaped), so split from the right.
            let inner = line.strip_prefix("| ")?.strip_suffix(" |")?;
            let mut cells = inner.rsplitn(6, " | ");
            let (_symbols, sha, _tokens, _lines, bytes, path) = (
                cells.next()?,
                cells.next()?,
                cells.next()?,
                cells.next()?,
                cells.next()?,
                cells.next()?,
            );
            bytes.parse::<u64>().ok()?;
            Some((path.replace("\\|", "|"), sha.to_string()))
        })
        .collect()
}

fn finish(lines: Vec<String>) -> String {
    let mut text = lines.join("\n");
    text.push('\n');
    text
}

/// Renders the navigators of `cache` for the project at `root` (shown as
/// is), scanned at `updated`.
pub fn render(root: &str, updated: &str, cache: &NavguardCache) -> Navigators {
    let mut paths: Vec<&String> = cache.files.keys().collect();
    paths.sort();
    let files: Vec<FileInfo> = paths.iter().map(|p| cache.files[*p].info(p)).collect();
    let mut symbols: BTreeMap<&str, Vec<&SymbolEntry>> = BTreeMap::new();
    for s in &cache.symbols {
        symbols.entry(s.path.as_str()).or_default().push(s);
    }
    let tree = TreeDir::from_paths(&paths).lines();
    let order: Vec<usize> = tree.iter().filter_map(|(_, file)| *file).collect();

    // navigator_deep.md: its row lines are known before anything is
    // rendered, since every file has exactly one row.
    let mut deep = header("Navigator Deep", None, updated);
    deep.extend([
        String::new(),
        format!(
            "One line per file, in the order of the tree in {}. File: the path relative to Root \
             in {}. Bytes, lines and an estimate of tokens (a little high on purpose); Symbols: \
             the lines of {} with the file's symbols (- when none were found).",
            NAVIGATOR, NAVIGATOR, DEEP2
        ),
        String::new(),
        "| File | Bytes | Lines | Tokens | SHA | Symbols |".to_string(),
        "|------|------:|------:|-------:|-----|---------|".to_string(),
    ]);
    let first_row = deep.len() + 1;
    let mut deep_line = vec![0usize; files.len()];
    for (position, &i) in order.iter().enumerate() {
        deep_line[i] = first_row + position;
    }

    // navigator_deep2.md first: the deep rows point into it.
    let mut deep2 = header("Navigator Deep2", None, updated);
    deep2.extend([
        String::new(),
        format!(
            "Symbols of each file with their lines in that file (inclusive); read them with \
             `any_txt_parser lines <path> <start> <end>`. Each section names the file (path \
             relative to Root in {}) and its line in {}; In is the enclosing symbol.",
            NAVIGATOR, DEEP
        ),
    ]);
    let mut deep2_range = vec![None; files.len()];
    for &i in &order {
        let Some(list) = symbols.get(files[i].path.as_str()) else {
            continue;
        };
        deep2.push(String::new());
        let start = deep2.len() + 1;
        deep2.push(format!("### {}  (deep L{})", files[i].path, deep_line[i]));
        deep2.push(String::new());
        deep2.push("| Symbol | Kind | Lines | Tokens | In |".to_string());
        deep2.push("|--------|------|-------|-------:|----|".to_string());
        for s in list {
            deep2.push(format!(
                "| {} | {} | {}-{} | {} | {} |",
                cell(&label(&s.name)),
                cell(&s.kind),
                s.line,
                s.line_end.max(s.line),
                s.tokens,
                cell(s.parent.as_deref().unwrap_or(""))
            ));
        }
        deep2_range[i] = Some((start, deep2.len()));
    }

    for &i in &order {
        let f = &files[i];
        let (lines, tokens) = if f.binary {
            ("binary".to_string(), "-".to_string())
        } else {
            (f.lines.to_string(), f.tokens.to_string())
        };
        let symbols = match deep2_range[i] {
            Some((a, b)) => format!("deep2 L{}-L{}", a, b),
            None => "-".to_string(),
        };
        deep.push(format!(
            "| {} | {} | {} | {} | {} | {} |",
            cell(&f.path),
            f.bytes,
            lines,
            tokens,
            f.sha,
            symbols
        ));
    }

    let text_files = files.iter().filter(|f| !f.binary);
    let mut navigator = header("Navigator", Some(root), updated);
    navigator.extend([
        format!("Status: {}", READY),
        format!(
            "Files: {} | Lines: {} | Tokens: ~{} | Bytes: {}",
            files.len(),
            text_files.clone().map(|f| f.lines).sum::<u64>(),
            text_files.map(|f| f.tokens).sum::<u64>(),
            files.iter().map(|f| f.bytes).sum::<u64>()
        ),
        String::new(),
        "How to read:".to_string(),
        "- A file's full path is Root + the folders above it in the tree + its name; \
         navigator_deep.md and navigator_deep2.md give paths relative to Root."
            .to_string(),
        format!(
            "- `deep L12` is line 12 of {}: the file's path, bytes, lines, ~tokens, sha and the \
             lines of its symbols in {}.",
            DEEP, DEEP2
        ),
        "- Line numbers are exact for Updated. After edits, run `any_txt_parser nav --oneshot` \
         again."
            .to_string(),
        format!(
            "- Status READY: {}, {} and {} are complete and come from the same scan (same \
             Updated).",
            NAVIGATOR, DEEP, DEEP2
        ),
        String::new(),
        "## TREE".to_string(),
        String::new(),
    ]);
    if tree.is_empty() {
        navigator.push("(no files)".to_string());
    }
    for (line, file) in tree {
        navigator.push(match file {
            Some(i) => format!("{}  deep L{}", line, deep_line[i]),
            None => line,
        });
    }

    Navigators {
        navigator: finish(navigator),
        deep: finish(deep),
        deep2: finish(deep2),
    }
}

const TIME_FORMAT: &str = "%Y-%m-%dT%H:%M:%S%:z";

/// The current local time, `2026-10-19T15:04:05+03:00`.
pub fn now() -> String {
    chrono::Local::now().format(TIME_FORMAT).to_string()
}

/// Unix seconds as local time in the format of [`now`].
pub fn local_time(unix_seconds: u64) -> String {
    use chrono::TimeZone;
    i64::try_from(unix_seconds)
        .ok()
        .and_then(|secs| chrono::Local.timestamp_opt(secs, 0).single())
        .map_or_else(
            || unix_seconds.to_string(),
            |time| time.format(TIME_FORMAT).to_string(),
        )
}

/// Renders the navigators of `cache` and writes them into `root`: deep2,
/// deep, then navigator, each replaced atomically.
pub fn write(root: &Path, cache: &NavguardCache) -> Result<(), String> {
    let navigators = render(&crate::paths::display_root(root), &now(), cache);
    for (name, text) in [
        (DEEP2, &navigators.deep2),
        (DEEP, &navigators.deep),
        (NAVIGATOR, &navigators.navigator),
    ] {
        let path = root.join(name);
        crate::write::atomic_write(&path, text.as_bytes())
            .map_err(|e| format!("cannot write '{}': {}", path.display(), e))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn tree_lists_directories_first_and_closes_subtrees() {
        let tree = build_tree(&[
            "src/parser/lexer.rs",
            "README.md",
            "src/main.rs",
            "src/parser.rs",
            "Cargo.toml",
            "docs/guide.md",
        ]);
        let expected = "\
├── docs/
│   └── guide.md
├── src/
│   ├── parser/
│   │   └── lexer.rs
│   ├── main.rs
│   └── parser.rs
├── Cargo.toml
└── README.md
";
        assert_eq!(tree, expected);
    }

    #[test]
    fn tree_siblings_after_a_last_subtree_have_no_dangling_connectors() {
        let tree = build_tree(&["a/b/c/deep.rs", "a/z.rs", "b/x.rs"]);
        let expected = "\
├── a/
│   ├── b/
│   │   └── c/
│   │       └── deep.rs
│   └── z.rs
└── b/
    └── x.rs
";
        assert_eq!(tree, expected);
        for line in tree.lines() {
            assert!(!line.trim_end().ends_with('│'), "dangling: {:?}", line);
        }
    }

    #[test]
    fn tree_normalizes_windows_paths_and_is_deterministic() {
        let windows = build_tree(&[".\\src\\lib.rs", "src\\api.rs", "src/api.rs", "top.txt"]);
        let posix = build_tree(&["top.txt", "src/lib.rs", "./src/api.rs"]);
        assert_eq!(windows, posix);
        assert_eq!(
            windows,
            "├── src/\n│   ├── api.rs\n│   └── lib.rs\n└── top.txt\n"
        );
        assert!(!windows.contains('\\'));
        assert_eq!(build_tree::<&str>(&[]), "");
    }

    fn project() -> (tempfile::TempDir, NavguardCache) {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        fs::create_dir_all(root.join("src")).unwrap();
        fs::create_dir_all(root.join("lib")).unwrap();
        fs::write(
            root.join("src/main.rs"),
            "fn main() {\r\n    helper();\r\n}\r\n\r\nfn helper() {}\r\n",
        )
        .unwrap();
        fs::write(
            root.join("lib/main.rs"),
            "struct A;\nimpl A {\n    fn b(&self) {}\n}\n",
        )
        .unwrap();
        fs::write(root.join("notes.txt"), "one\ntwo\nthree\n").unwrap();
        fs::write(root.join("empty.txt"), "").unwrap();
        fs::write(root.join("logo.bin"), b"\x89PNG\0\0\x01").unwrap();
        let cache = crate::cache::build_cache(&root.to_string_lossy(), &[]);
        (dir, cache)
    }

    fn line(text: &str, n: usize) -> &str {
        text.lines()
            .nth(n - 1)
            .unwrap_or_else(|| panic!("no line {}", n))
    }

    /// The number after `marker` in `text` (`deep L12` -> 12).
    fn number_after(text: &str, marker: &str) -> usize {
        let start = text.find(marker).unwrap() + marker.len();
        text[start..]
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect::<String>()
            .parse()
            .unwrap()
    }

    #[test]
    fn every_pointer_lands_on_its_file() {
        let (_dir, cache) = project();
        let nav = render(r"C:\work\shop", "2026-10-19T15:04:05+03:00", &cache);
        for file in ["main.rs", "notes.txt", "empty.txt", "logo.bin"] {
            for tree_line in nav
                .navigator
                .lines()
                .filter(|l| l.contains(&format!(" {}  deep L", file)))
            {
                let row = line(&nav.deep, number_after(tree_line, "deep L"));
                let path = &row[2..row.find(" | ").unwrap()];
                assert!(
                    path == file || path.ends_with(&format!("/{}", file)),
                    "{} -> {}",
                    tree_line,
                    row
                );
                if let Some(at) = row.find("deep2 L") {
                    let start = number_after(&row[at..], "deep2 L");
                    let end = number_after(&row[at..], "-L");
                    let section = line(&nav.deep2, start);
                    assert!(
                        section.starts_with(&format!("### {}  (deep L", path)),
                        "{}",
                        section
                    );
                    let back = number_after(section, "(deep L");
                    assert_eq!(line(&nav.deep, back), row);
                    assert!(line(&nav.deep2, end).starts_with("| "), "end of section");
                    assert!(nav.deep2.lines().nth(end).is_none_or(|l| l.is_empty()));
                }
            }
        }
        // Two files named main.rs: two distinct rows, two distinct sections.
        let pointers: Vec<usize> = nav
            .navigator
            .lines()
            .filter(|l| l.contains(" main.rs  deep L"))
            .map(|l| number_after(l, "deep L"))
            .collect();
        assert_eq!(pointers.len(), 2);
        assert_ne!(pointers[0], pointers[1]);
    }

    #[test]
    fn header_paths_counts_and_rows() {
        let (_dir, cache) = project();
        let nav = render(r"C:\work\shop", "2026-10-19T15:04:05+03:00", &cache);
        assert!(nav.navigator.contains(
            "\nRoot: C:\\work\\shop\nUpdated: 2026-10-19T15:04:05+03:00\nStatus: READY\n"
        ));
        // The absolute root once; deep and deep2 have relative paths.
        for text in [&nav.deep, &nav.deep2] {
            assert!(text.contains("\nUpdated: 2026-10-19T15:04:05+03:00\n"));
            assert!(!text.contains("Root: ") && !text.contains("shop"));
        }
        assert!(!nav.navigator.contains("[0,1,0]") && !nav.navigator.contains("Triangle"));
        assert!(nav.navigator.contains("\nFiles: 5 | Lines: 12 | Tokens: ~"));
        assert!(nav.deep.contains("| src/main.rs | ") && nav.deep.contains("| lib/main.rs | "));
        assert!(nav.deep2.contains("\n### src/main.rs  (deep L"));
        assert!(nav.deep2.contains("\n### lib/main.rs  (deep L"));
        assert_eq!(nav.navigator.matches("notes.txt").count(), 1);
        assert_eq!(updated_of(&nav.deep), Some("2026-10-19T15:04:05+03:00"));
        let rows = deep_rows(&nav.deep);
        assert_eq!(rows.len(), 5);
        assert!(rows.contains(&("notes.txt".to_string(), "b6285c57".to_string())));
        assert!(nav
            .deep
            .contains("| notes.txt | 14 | 3 | 5 | b6285c57 | - |"));
        assert!(nav
            .deep
            .contains("| empty.txt | 0 | 0 | 0 | e3b0c442 | - |"));
        assert!(nav.deep.contains("| logo.bin | 7 | binary | - |"));
        // CRLF source: 5 lines, symbol ranges in those lines.
        assert!(nav.deep2.contains("| fn main() { | fn | 1-3 |"));
        assert!(nav.deep2.contains("| fn helper() {} | fn | 5-5 |"));
        assert!(nav
            .deep2
            .contains("| fn b(&self) {} | method | 3-3 | 6 | A |"));
        assert!(!nav.deep2.contains("anchor_map"));
    }

    #[test]
    fn labels_are_cut_by_character_and_pipes_escaped() {
        let dir = tempfile::tempdir().unwrap();
        let long = format!("fn {}() {{}}", "я".repeat(80));
        fs::write(dir.path().join("unicode.rs"), &long).unwrap();
        fs::write(dir.path().join("pipe.rs"), "fn f() { let g = |x| x; }\n").unwrap();
        let cache = crate::cache::build_cache(&dir.path().to_string_lossy(), &[]);
        let nav = render("root", "now", &cache);
        let short: String = long.chars().take(LABEL_CHARS).collect();
        assert!(nav.deep2.contains(&format!("| {}... |", short)));
        assert!(nav.deep2.contains("\\|x\\|"));
        // A `|` in a path (possible outside Windows) is escaped in the row.
        assert_eq!(
            deep_rows("| File | Bytes |\n|--|--:|\n| a \\| b.txt | 2 | 1 | 1 | 0123abcd | - |\n"),
            [("a | b.txt".to_string(), "0123abcd".to_string())]
        );
    }

    #[test]
    fn empty_project() {
        let dir = tempfile::tempdir().unwrap();
        let cache = crate::cache::build_cache(&dir.path().to_string_lossy(), &[]);
        let nav = render("root", "now", &cache);
        assert!(nav.navigator.ends_with("## TREE\n\n(no files)\n"));
        assert!(nav
            .navigator
            .contains("Files: 0 | Lines: 0 | Tokens: ~0 | Bytes: 0"));
    }

    #[test]
    fn writes_all_three_files_into_the_root() {
        let (dir, cache) = project();
        write(dir.path(), &cache).unwrap();
        let updated: Vec<String> = [NAVIGATOR, DEEP, DEEP2]
            .iter()
            .map(|name| {
                let text = fs::read_to_string(dir.path().join(name)).unwrap();
                text.lines()
                    .find(|l| l.starts_with("Updated: "))
                    .unwrap()
                    .to_string()
            })
            .collect();
        assert!(updated.iter().all(|u| u == &updated[0]));
        let root_line = fs::read_to_string(dir.path().join(NAVIGATOR)).unwrap();
        let root = root_line.lines().find(|l| l.starts_with("Root: ")).unwrap();
        assert!(Path::new(&root["Root: ".len()..]).is_absolute(), "{}", root);
    }
}
