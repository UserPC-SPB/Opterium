//! Data-returning project navigation for AI tools.
//!
//! Every free function takes the project root, loads `<root>/navguard.json`
//! (defaults when missing or invalid; its `root` field is replaced by the
//! argument) and returns plain data. [`Project`] does the same once and can
//! be reused for several calls.
//!
//! Conventions:
//! * Paths in results are relative to the root and use `/` separators.
//!   Path arguments may be relative to the root or absolute, but must point
//!   inside the root.
//! * Line numbers are 1-based; ranges are inclusive. LF, CRLF and a lone
//!   CR end a line ([`crate::text`]), the same rule everywhere.
//! * Nothing is written to disk unless documented: only
//!   [`FindOptions::use_cache`] and `changes(.., persist = true)` (and
//!   [`Project::update_cache`]) write `<root>/.navguard_cache.json`.
//! * Errors are human-readable strings.
//! * Every result type implements `serde::Serialize`; the JSON field names
//!   are the snake_case Rust field names documented on each type and are
//!   part of the API.

use crate::cache::{self, DiffKind, NavguardCache, SymbolEntry};
use crate::config::Config;
use crate::navigator;
use crate::parser::{self, Anchor};
use crate::scanner::{self, FileInfo};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Symbol cache file name, inside the project root.
pub const CACHE_FILE: &str = ".navguard_cache.json";
/// Configuration file name, inside the project root.
pub const CONFIG_FILE: &str = "navguard.json";

/// Longest snippet returned by [`search_text`], in characters.
pub const SEARCH_SNIPPET_CHARS: usize = 300;
/// Longest line returned by [`todos`], in characters.
pub const TODO_TEXT_CHARS: usize = 120;
/// Markers [`todos`] looks for ([`TodoMatch::Strict`]).
pub const TODO_MARKERS: [&str; 4] = ["TODO", "FIXME", "HACK", "XXX"];

/// A scanned file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FileEntry {
    /// Relative path, `/`-separated.
    pub path: String,
    /// Line count ([`crate::text::line_count`]: an empty file has 0, a
    /// final line break adds none); 0 for binary files.
    pub lines: u64,
    /// Token estimate, a little high on purpose
    /// ([`crate::text::estimate_tokens`]); 0 for binary files.
    pub tokens: u64,
    /// First 8 hex digits of the SHA-256 of the content.
    pub sha: String,
    /// Size in bytes.
    pub bytes: u64,
    /// The file is binary (not text).
    pub binary: bool,
}

impl From<&FileInfo> for FileEntry {
    fn from(f: &FileInfo) -> Self {
        FileEntry {
            path: f.path.clone(),
            lines: f.lines,
            tokens: f.tokens,
            sha: f.sha.clone(),
            bytes: f.bytes,
            binary: f.binary,
        }
    }
}

/// Totals over all scanned files (also when [`ProjectMap::files`] is capped).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MapTotals {
    pub files: usize,
    pub lines: u64,
    pub tokens: u64,
}

/// Options for [`map`].
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct MapOptions {
    /// Return at most this many files (in path order); `None` = all.
    pub max_files: Option<usize>,
}

/// Result of [`map`].
#[derive(Debug, Clone, Serialize)]
pub struct ProjectMap {
    /// The root as given, `/`-separated.
    pub root: String,
    /// Scanned files sorted by path, capped by [`MapOptions::max_files`].
    pub files: Vec<FileEntry>,
    pub totals: MapTotals,
    /// True when `files` (and `tree`) omit files because of the cap.
    pub truncated: bool,
    /// Text tree (`├──`/`└──`/`│`) of exactly the files in `files`;
    /// directories first, `/`-terminated.
    pub tree: String,
}

/// One symbol of a file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Symbol {
    /// Identifier: `new`, `Point`, `Display for Point` (Rust impl),
    /// `Foo::bar` (C++ out-of-line), `[s1-1]` (protocol anchor).
    pub name: String,
    /// Trimmed first line of the definition (what `nav --grep` prints).
    pub label: String,
    /// One of `fn`, `method`, `class`, `struct`, `enum`, `union`, `trait`,
    /// `impl`, `interface`, `type`, `mod`, `namespace`, `macro`, `record`,
    /// `object`, `anchor` (see [`parser::KINDS`]).
    pub kind: String,
    /// First line (1-based).
    pub line_start: usize,
    /// Last line, inclusive.
    pub line_end: usize,
    /// Nesting depth, 0 = top level.
    pub depth: usize,
    /// Name of the enclosing symbol.
    pub parent: Option<String>,
    /// Token estimate of the range.
    pub tokens: usize,
}

impl From<&Anchor> for Symbol {
    fn from(a: &Anchor) -> Self {
        Symbol {
            name: a.name.clone(),
            label: a.label.clone(),
            kind: a.kind.to_string(),
            line_start: a.line_start,
            line_end: a.line_end,
            depth: a.depth,
            parent: a.parent.clone(),
            tokens: a.tokens,
        }
    }
}

impl From<&SymbolEntry> for Symbol {
    fn from(s: &SymbolEntry) -> Self {
        Symbol {
            name: if s.ident.is_empty() {
                s.name.clone()
            } else {
                s.ident.clone()
            },
            label: s.name.clone(),
            kind: s.kind.clone(),
            line_start: s.line,
            line_end: s.line_end.max(s.line),
            depth: s.depth,
            parent: s.parent.clone(),
            tokens: s.tokens,
        }
    }
}

/// Symbols of one file, in line order.
#[derive(Debug, Clone, Serialize)]
pub struct FileSymbols {
    pub path: String,
    pub symbols: Vec<Symbol>,
}

/// A symbol matched by [`find_symbols`]. Serializes flat:
/// `path` plus every [`Symbol`] field.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SymbolHit {
    pub path: String,
    #[serde(flatten)]
    pub symbol: Symbol,
}

/// Options for [`find_symbols`].
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct FindOptions {
    /// Read and update `<root>/.navguard_cache.json` (reusing symbols of
    /// unchanged files) instead of indexing in memory. Writes the cache.
    pub use_cache: bool,
    /// With `use_cache`: ignore the existing cache and rebuild it.
    pub force: bool,
    /// Match case-sensitively (default: case-insensitive).
    pub case_sensitive: bool,
    /// The symbol name must equal the pattern (for a qualified name such
    /// as `Foo::bar`, the part after the last `::` may equal it instead);
    /// label and path do not count. Default: the name, label or path
    /// contains the pattern.
    pub exact: bool,
    /// Return at most this many hits.
    pub max_hits: Option<usize>,
}

/// How [`todos`] recognizes a marker in a line.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TodoMatch {
    /// A whole word of [`TODO_MARKERS`] (letters, digits and `_` do not
    /// touch it) written in capitals, or in any case when followed by `:`,
    /// `(` or `!` or preceded by `@`: `TODO fix`, `todo: x`, `Fixme(bob)`,
    /// `todo!()`, `@todo`. Not `todos`, `TODO_LIMIT`, `hackathon`.
    #[default]
    Strict,
    /// TODO, FIXME or HACK anywhere in the line, in any case (the matching
    /// of 1.4.0 and earlier: `todos` and `todo_hits` count too).
    Loose,
}

impl TodoMatch {
    /// `loose: true` -> [`TodoMatch::Loose`].
    pub fn loose(loose: bool) -> TodoMatch {
        if loose {
            TodoMatch::Loose
        } else {
            TodoMatch::Strict
        }
    }
}

/// One line of a file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NumberedLine {
    /// 1-based line number.
    pub line: usize,
    /// Line content without the line terminator.
    pub text: String,
}

/// What [`search_text`] returns around a match.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SearchContext {
    /// `Sentence` in prose files (Markdown, plain text, reStructuredText,
    /// AsciiDoc, Org, TeX: see [`is_prose_path`]), `Line` in all others.
    #[default]
    Auto,
    /// The trimmed line of the match; one hit per line however many
    /// matches it has. Code is cut at every `.` by sentences.
    Line,
    /// The sentence around each match: from the previous `.`, `!`, `?` or
    /// line start to the next `.`, `!` or `?`.
    Sentence,
}

impl SearchContext {
    /// `auto`, `line`, `sentence`.
    pub fn parse(name: &str) -> Option<SearchContext> {
        match name.to_ascii_lowercase().as_str() {
            "auto" => Some(SearchContext::Auto),
            "line" | "lines" => Some(SearchContext::Line),
            "sentence" | "sentences" => Some(SearchContext::Sentence),
            _ => None,
        }
    }

    /// The context used for a file: `Auto` resolved by its path.
    pub fn for_path(self, path: &str) -> SearchContext {
        match self {
            SearchContext::Auto if is_prose_path(path) => SearchContext::Sentence,
            SearchContext::Auto => SearchContext::Line,
            other => other,
        }
    }
}

/// Files whose text is prose, where a sentence is the useful context.
pub fn is_prose_path(path: &str) -> bool {
    let name = path.rsplit(['/', '\\']).next().unwrap_or(path);
    let Some((_, ext)) = name.rsplit_once('.') else {
        return false;
    };
    matches!(
        ext.to_ascii_lowercase().as_str(),
        "md" | "markdown" | "mdx" | "txt" | "text" | "rst" | "adoc" | "asciidoc" | "org" | "tex"
    )
}

/// A text match ([`search_text`]) or a TODO line ([`todos`]).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TextHit {
    pub path: String,
    /// 1-based line where the match starts.
    pub line: usize,
    /// The line or sentence around the match ([`search_text`], see
    /// [`SearchContext`]) or the trimmed line ([`todos`]), cut to
    /// [`SEARCH_SNIPPET_CHARS`] / [`TODO_TEXT_CHARS`].
    pub text: String,
    /// True when `text` was cut.
    pub truncated: bool,
}

/// A file that differs from the cache.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FileChange {
    pub path: String,
    /// `new`, `changed` or `deleted`.
    pub status: String,
    /// Current values; for deleted files the cached values.
    pub lines: u64,
    pub tokens: u64,
    pub sha: String,
}

/// Result of [`changes`].
#[derive(Debug, Clone, Serialize)]
pub struct ChangeReport {
    /// No cache existed: every scanned file is reported as `new`.
    pub cold: bool,
    /// The cache was written (only with `persist`).
    pub persisted: bool,
    /// Why writing the cache failed, if it did.
    pub persist_error: Option<String>,
    /// Number of `new` / `changed` / `deleted` entries in `changes`.
    pub new: usize,
    pub changed: usize,
    pub deleted: usize,
    /// New and changed files in path order, then deleted files in path order.
    pub changes: Vec<FileChange>,
}

/// Most paths listed in [`NavigatorState::paths`].
pub const STATE_MAX_PATHS: usize = 50;

/// Whether the navigator files describe the project as it is now.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NavigatorState {
    /// `fresh`: `navigator.md`, `navigator_deep.md` and `navigator_deep2.md`
    /// come from one scan (same `Updated:`) and list exactly the files on
    /// disk with their current sha; `stale`: they come from one scan, but
    /// files were added, changed or deleted since; `incomplete`: one or two
    /// of them are missing, or their `Updated:` differ; `missing`: none
    /// exists. Anything but `fresh`: run `nav --oneshot`.
    pub status: String,
    /// `Updated:` of `navigator.md` (local time of its scan).
    pub updated: Option<String>,
    /// Files on disk not listed in `navigator_deep.md`.
    pub new: usize,
    /// Listed files whose content changed.
    pub changed: usize,
    /// Listed files that are gone.
    pub deleted: usize,
    /// Paths of the new, changed and deleted files, sorted, at most
    /// [`STATE_MAX_PATHS`].
    pub paths: Vec<String>,
}

/// The workflow state of `nav --triangle`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct WorkflowState {
    /// `[A, B, C]`: `[1,0,0]` step done, `[0,1,0]` verifying, `[0,0,1]` idle
    /// (also when there is no state file).
    pub triangle: [u8; 3],
    pub transitions: u64,
    /// Time of the last transition as written in the state file (empty
    /// when none).
    pub timestamp: String,
}

/// Result of [`state`]: freshness metadata of a project.
#[derive(Debug, Clone, Serialize)]
pub struct ProjectState {
    /// The root, absolute (as `Root:` in `navigator.md`).
    pub root: String,
    /// Local time this state was computed, ISO 8601 with the offset.
    pub checked: String,
    /// Totals of the files on disk now.
    pub totals: MapTotals,
    pub navigators: NavigatorState,
    /// Local time of the snapshot in `.navguard_cache.json` that
    /// [`changes`] compares with; `None` when there is none.
    pub snapshot: Option<String>,
    pub workflow: WorkflowState,
}

/// A project root with its loaded configuration.
#[derive(Debug, Clone)]
pub struct Project {
    cfg: Config,
}

impl Project {
    /// Opens `root`: it must be a directory; `<root>/navguard.json` is
    /// loaded and its `root` replaced by `root`.
    pub fn open(root: &Path) -> Result<Project, String> {
        if !root.is_dir() {
            return Err(format!(
                "project root is not a directory: {}",
                posix(&root.to_string_lossy())
            ));
        }
        let mut cfg = Config::load(&root.join(CONFIG_FILE).to_string_lossy());
        cfg.root = root.to_string_lossy().into_owned();
        Ok(Project { cfg })
    }

    /// Uses `cfg` as is (`cfg.root` is the project root; it is not checked).
    pub fn from_config(cfg: Config) -> Project {
        Project { cfg }
    }

    pub fn config(&self) -> &Config {
        &self.cfg
    }

    pub fn root(&self) -> &Path {
        Path::new(&self.cfg.root)
    }

    fn scan(&self) -> Vec<FileInfo> {
        scanner::scan_directory(&self.cfg.root, &self.cfg.exclude)
    }

    /// The text of a scanned file ([`crate::text::decode`]); `None` for
    /// binary and unreadable files.
    fn text_of(&self, rel: &str) -> Option<String> {
        let bytes = fs::read(self.full_path(rel)).ok()?;
        (!crate::text::is_binary(&bytes)).then(|| crate::text::decode(&bytes))
    }

    fn cache_path(&self) -> String {
        self.root().join(CACHE_FILE).to_string_lossy().into_owned()
    }

    fn full_path(&self, rel: &str) -> PathBuf {
        self.root().join(rel)
    }

    /// Resolves a path argument to (full path, root-relative POSIX path);
    /// it must be a file inside the root ([`crate::paths::existing_file`]).
    fn resolve(&self, path: &str) -> Result<(PathBuf, String), String> {
        crate::paths::existing_file(self.root(), path)
    }

    /// The text of the file at `path` (see [`Project::resolve`]).
    fn read_text(&self, path: &str) -> Result<(String, String), String> {
        let (full, rel) = self.resolve(path)?;
        let bytes = fs::read(&full).map_err(|e| format!("cannot read {}: {}", rel, e))?;
        if crate::text::is_binary(&bytes) {
            return Err(format!("{} is a binary file", rel));
        }
        Ok((crate::text::decode(&bytes), rel))
    }

    /// See [`map`].
    pub fn map(&self, opts: &MapOptions) -> Result<ProjectMap, String> {
        let files = self.scan();
        let totals = MapTotals {
            files: files.len(),
            lines: files.iter().map(|f| f.lines).sum(),
            tokens: files.iter().map(|f| f.tokens).sum(),
        };
        let cap = opts.max_files.unwrap_or(usize::MAX);
        let truncated = files.len() > cap;
        let files: Vec<FileEntry> = files.iter().take(cap).map(FileEntry::from).collect();
        let paths: Vec<&str> = files.iter().map(|f| f.path.as_str()).collect();
        Ok(ProjectMap {
            root: posix(&self.cfg.root),
            tree: navigator::build_tree(&paths),
            files,
            totals,
            truncated,
        })
    }

    /// See [`symbols`].
    pub fn symbols(&self, path: Option<&str>) -> Result<Vec<FileSymbols>, String> {
        if let Some(path) = path {
            let (text, rel) = self.read_text(path)?;
            let symbols = parser::parse_source(&text, extension(&rel));
            return Ok(vec![FileSymbols {
                path: rel,
                symbols: symbols.iter().map(Symbol::from).collect(),
            }]);
        }
        Ok(self
            .scan()
            .iter()
            .filter_map(|f| {
                let anchors = parser::parse_source(&self.text_of(&f.path)?, extension(&f.path));
                (!anchors.is_empty()).then(|| FileSymbols {
                    path: f.path.clone(),
                    symbols: anchors.iter().map(Symbol::from).collect(),
                })
            })
            .collect())
    }

    /// See [`find_symbols`].
    pub fn find_symbols(
        &self,
        pattern: &str,
        opts: &FindOptions,
    ) -> Result<Vec<SymbolHit>, String> {
        let needle = if opts.case_sensitive {
            pattern.to_string()
        } else {
            pattern.to_lowercase()
        };
        let matches = |hay: &str| {
            if opts.case_sensitive {
                hay.contains(&needle)
            } else {
                hay.to_lowercase().contains(&needle)
            }
        };
        let equals = |name: &str| {
            let last = name.rsplit("::").next().unwrap_or(name);
            if opts.case_sensitive {
                name == pattern || last == pattern
            } else {
                name.to_lowercase() == needle || last.to_lowercase() == needle
            }
        };
        let limit = opts.max_hits.unwrap_or(usize::MAX);
        let mut hits = Vec::new();
        let mut push = |path: &str, symbol: Symbol| -> bool {
            let found = if opts.exact {
                equals(&symbol.name)
            } else {
                matches(&symbol.name) || matches(&symbol.label) || matches(path)
            };
            if found {
                hits.push(SymbolHit {
                    path: path.to_string(),
                    symbol,
                });
            }
            hits.len() < limit
        };
        if opts.use_cache {
            let cache = self.update_cache(opts.force);
            for entry in &cache.symbols {
                if !push(&entry.path, Symbol::from(entry)) {
                    break;
                }
            }
        } else {
            'files: for f in self.scan() {
                let Some(text) = self.text_of(&f.path) else {
                    continue;
                };
                for anchor in parser::parse_source(&text, extension(&f.path)) {
                    if !push(&f.path, Symbol::from(&anchor)) {
                        break 'files;
                    }
                }
            }
        }
        hits.truncate(limit);
        Ok(hits)
    }

    /// See [`read_lines`].
    pub fn read_lines(
        &self,
        path: &str,
        start: usize,
        end: Option<usize>,
    ) -> Result<Vec<NumberedLine>, String> {
        let (content, rel) = self.read_text(path)?;
        select_lines(&content, start, end).map_err(|e| match e {
            LineRangeError::StartOutOfRange { start: 0, .. } => {
                "start line must be 1 or greater (line numbers are 1-based)".to_string()
            }
            LineRangeError::StartOutOfRange { start, len } => format!(
                "start line {} is out of range: {} has {} line(s)",
                start, rel, len
            ),
            LineRangeError::EndBeforeStart { start, end } => {
                format!("end line {} is before start line {}", end, start)
            }
        })
    }

    /// See [`search_text`].
    pub fn search_text(
        &self,
        pattern: &str,
        path: Option<&str>,
        max_hits: Option<usize>,
        context: SearchContext,
    ) -> Result<Vec<TextHit>, String> {
        if pattern.is_empty() {
            return Err("search pattern must not be empty".to_string());
        }
        let limit = max_hits.unwrap_or(usize::MAX);
        let mut hits = Vec::new();
        let mut collect = |rel: &str, content: &str| -> bool {
            let found = match context.for_path(rel) {
                SearchContext::Sentence => sentence_hits(content, pattern),
                _ => line_hits(content, pattern),
            };
            for (line, text, truncated) in found {
                if hits.len() >= limit {
                    return false;
                }
                hits.push(TextHit {
                    path: rel.to_string(),
                    line,
                    text,
                    truncated,
                });
            }
            hits.len() < limit
        };
        match path {
            Some(path) => {
                let (content, rel) = self.read_text(path)?;
                collect(&rel, &content);
            }
            None => {
                for f in self.scan() {
                    // Binary files are skipped.
                    if let Some(content) = self.text_of(&f.path) {
                        if !collect(&f.path, &content) {
                            break;
                        }
                    }
                }
            }
        }
        Ok(hits)
    }

    /// See [`todos`].
    pub fn todos(&self, mode: TodoMatch) -> Result<Vec<TextHit>, String> {
        let mut hits = Vec::new();
        for f in self.scan() {
            if let Some(content) = self.text_of(&f.path) {
                for (line, text, truncated) in todo_hits(&content, mode) {
                    hits.push(TextHit {
                        path: f.path.clone(),
                        line,
                        text,
                        truncated,
                    });
                }
            }
        }
        Ok(hits)
    }

    /// See [`changes`].
    pub fn changes(&self, persist: bool) -> Result<ChangeReport, String> {
        let cache_path = self.cache_path();
        let cached = cache::load_cache(&cache_path);
        let files = self.scan();
        let changes: Vec<FileChange> = match &cached {
            Some(old) => {
                let mut diffs = cache::diff_scan(&files, old);
                // diff_scan lists deleted files last, in hash-map order.
                let first_deleted = diffs
                    .iter()
                    .position(|d| d.kind == DiffKind::Deleted)
                    .unwrap_or(diffs.len());
                diffs[first_deleted..].sort_by(|a, b| a.path.cmp(&b.path));
                diffs
                    .into_iter()
                    .map(|d| FileChange {
                        status: match d.kind {
                            DiffKind::New => "new",
                            DiffKind::Changed => "changed",
                            DiffKind::Deleted => "deleted",
                        }
                        .to_string(),
                        path: d.path,
                        lines: d.lines,
                        tokens: d.tokens,
                        sha: d.sha,
                    })
                    .collect()
            }
            None => files
                .iter()
                .map(|f| FileChange {
                    path: f.path.clone(),
                    status: "new".to_string(),
                    lines: f.lines,
                    tokens: f.tokens,
                    sha: f.sha.clone(),
                })
                .collect(),
        };
        let count = |status: &str| changes.iter().filter(|c| c.status == status).count();
        let mut report = ChangeReport {
            cold: cached.is_none(),
            persisted: false,
            persist_error: None,
            new: count("new"),
            changed: count("changed"),
            deleted: count("deleted"),
            changes,
        };
        if persist {
            let updated = match &cached {
                Some(old) => cache::smart_update_cache(&self.cfg.root, old, &self.cfg.exclude),
                None => cache::build_cache(&self.cfg.root, &self.cfg.exclude),
            };
            match cache::try_save_cache(&cache_path, &updated) {
                Ok(()) => report.persisted = true,
                Err(e) => report.persist_error = Some(e),
            }
        }
        Ok(report)
    }

    /// See [`state`].
    pub fn state(&self) -> Result<ProjectState, String> {
        let files = self.scan();
        let read = |name: &str| fs::read_to_string(self.root().join(name)).ok();
        let texts = [
            read(navigator::NAVIGATOR),
            read(navigator::DEEP),
            read(navigator::DEEP2),
        ];
        let triangle = crate::triangle::read_triangle(
            &self.full_path(&self.cfg.triangle_path).to_string_lossy(),
        );
        Ok(ProjectState {
            root: crate::paths::display_root(self.root()),
            checked: navigator::now(),
            totals: MapTotals {
                files: files.len(),
                lines: files.iter().map(|f| f.lines).sum(),
                tokens: files.iter().map(|f| f.tokens).sum(),
            },
            navigators: navigator_state(&files, &texts),
            snapshot: cache::load_cache(&self.cache_path())
                .map(|cache| navigator::local_time(cache.updated)),
            workflow: WorkflowState {
                triangle: [triangle.a, triangle.b, triangle.c],
                transitions: triangle.transitions,
                timestamp: triangle.timestamp,
            },
        })
    }

    /// Loads `<root>/.navguard_cache.json` (unless `force`), reconciles it
    /// with the files on disk (reusing symbols of unchanged files), writes
    /// it back and returns it. This is what the CLI's watch/oneshot/grep do.
    pub fn update_cache(&self, force: bool) -> NavguardCache {
        let cache_path = self.cache_path();
        let cached = if force {
            None
        } else {
            cache::load_cache(&cache_path)
        };
        let updated = match cached {
            Some(old) => cache::smart_update_cache(&self.cfg.root, &old, &self.cfg.exclude),
            None => cache::build_cache(&self.cfg.root, &self.cfg.exclude),
        };
        cache::save_cache(&cache_path, &updated);
        updated
    }
}

/// Scanned files with line/token/sha data, a capped file list and a text
/// tree built from that list. Reads only.
pub fn map(root: &Path, opts: &MapOptions) -> Result<ProjectMap, String> {
    Project::open(root)?.map(opts)
}

/// Symbols of one file (`path` relative to the root or absolute; the file
/// may be excluded by the configuration) or, with `None`, of every scanned
/// file that has at least one symbol. Reads only.
pub fn symbols(root: &Path, path: Option<&str>) -> Result<Vec<FileSymbols>, String> {
    Project::open(root)?.symbols(path)
}

/// Symbols whose name, label or file path contains `pattern`, or with
/// [`FindOptions::exact`] whose name equals it (case-insensitive unless
/// [`FindOptions::case_sensitive`]; an empty pattern matches every symbol,
/// or none when exact), ordered by path and line. Indexes in memory unless
/// [`FindOptions::use_cache`].
pub fn find_symbols(
    root: &Path,
    pattern: &str,
    opts: &FindOptions,
) -> Result<Vec<SymbolHit>, String> {
    Project::open(root)?.find_symbols(pattern, opts)
}

/// Lines `start..=end` of a file (`end: None` = only `start`; an `end` past
/// the last line is clamped). Errors when `start` is 0 or past the end of
/// the file, or `end < start`. Reads only.
pub fn read_lines(
    root: &Path,
    path: &str,
    start: usize,
    end: Option<usize>,
) -> Result<Vec<NumberedLine>, String> {
    Project::open(root)?.read_lines(path, start, end)
}

/// Case-sensitive substring search in one file or in all scanned UTF-8
/// files, returning the line or the sentence around each match (see
/// [`SearchContext`]). Reads only.
pub fn search_text(
    root: &Path,
    pattern: &str,
    path: Option<&str>,
    max_hits: Option<usize>,
    context: SearchContext,
) -> Result<Vec<TextHit>, String> {
    Project::open(root)?.search_text(pattern, path, max_hits, context)
}

/// Lines with a TODO, FIXME, HACK or XXX marker in scanned files (see
/// [`TodoMatch`]). Reads only.
pub fn todos(root: &Path, mode: TodoMatch) -> Result<Vec<TextHit>, String> {
    Project::open(root)?.todos(mode)
}

/// New, changed (by content hash) and deleted files compared with
/// `<root>/.navguard_cache.json`. Writes the updated cache only when
/// `persist` is true.
pub fn changes(root: &Path, persist: bool) -> Result<ChangeReport, String> {
    Project::open(root)?.changes(persist)
}

/// Freshness of the project: file totals, whether the navigator files
/// match the files on disk ([`NavigatorState`]), the time of the
/// [`changes`] snapshot and the `nav --triangle` workflow state. Reads only.
pub fn state(root: &Path) -> Result<ProjectState, String> {
    Project::open(root)?.state()
}

/// Compares the navigator texts (navigator, deep, deep2; `None` = missing)
/// with the scanned `files`.
fn navigator_state(files: &[FileInfo], texts: &[Option<String>; 3]) -> NavigatorState {
    let updated: Vec<Option<&str>> = texts
        .iter()
        .map(|t| t.as_deref().and_then(navigator::updated_of))
        .collect();
    let mut state = NavigatorState {
        status: "missing".to_string(),
        updated: updated[0].map(String::from),
        new: 0,
        changed: 0,
        deleted: 0,
        paths: Vec::new(),
    };
    if texts.iter().all(Option::is_none) {
        return state;
    }
    if updated.iter().any(|u| u.is_none() || *u != updated[0]) {
        state.status = "incomplete".to_string();
        return state;
    }
    let listed: std::collections::HashMap<String, String> =
        navigator::deep_rows(texts[1].as_deref().unwrap_or_default())
            .into_iter()
            .collect();
    let mut paths = Vec::new();
    for f in files {
        match listed.get(&f.path) {
            None => state.new += 1,
            Some(sha) if *sha != f.sha => state.changed += 1,
            Some(_) => continue,
        }
        paths.push(f.path.clone());
    }
    let on_disk: std::collections::HashSet<&str> = files.iter().map(|f| f.path.as_str()).collect();
    for path in listed.keys().filter(|p| !on_disk.contains(p.as_str())) {
        state.deleted += 1;
        paths.push(path.clone());
    }
    paths.sort();
    paths.truncate(STATE_MAX_PATHS);
    state.paths = paths;
    state.status = if state.new + state.changed + state.deleted == 0 {
        "fresh"
    } else {
        "stale"
    }
    .to_string();
    state
}

fn posix(path: &str) -> String {
    path.replace('\\', "/")
}

/// The extension of `path` without the dot (empty when none).
fn extension(path: &str) -> &str {
    let name = path.rsplit('/').next().unwrap_or(path);
    name.rsplit_once('.').map_or("", |(_, ext)| ext)
}

/// Why [`select_lines`] rejected a range.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum LineRangeError {
    StartOutOfRange { start: usize, len: usize },
    EndBeforeStart { start: usize, end: usize },
}

/// Lines `start..=min(end, len)` of `content` (`end: None` = `start`).
pub(crate) fn select_lines(
    content: &str,
    start: usize,
    end: Option<usize>,
) -> Result<Vec<NumberedLine>, LineRangeError> {
    let lines: Vec<&str> = content.lines().collect();
    if start == 0 || start > lines.len() {
        return Err(LineRangeError::StartOutOfRange {
            start,
            len: lines.len(),
        });
    }
    let end = end.unwrap_or(start);
    if end < start {
        return Err(LineRangeError::EndBeforeStart { start, end });
    }
    Ok((start..=end.min(lines.len()))
        .map(|n| NumberedLine {
            line: n,
            text: lines[n - 1].to_string(),
        })
        .collect())
}

/// Sentence-context matches of `pattern` in `content`:
/// (1-based line of the match, snippet, snippet was cut). An empty pattern
/// has no matches.
pub(crate) fn sentence_hits(content: &str, pattern: &str) -> Vec<(usize, String, bool)> {
    let mut out = Vec::new();
    if pattern.is_empty() {
        return out;
    }
    let is_boundary = |c: char| c == '.' || c == '!' || c == '?';
    let mut search_pos = 0usize;
    while let Some(match_pos) = content[search_pos..].find(pattern) {
        let abs_pos = search_pos + match_pos;
        let sentence_start = content[..abs_pos]
            .rfind(|c: char| is_boundary(c) || c == '\n')
            .map(|p| p + 1)
            .unwrap_or(0);
        let after_match = abs_pos + pattern.len();
        let sentence_end = content[after_match..]
            .find(is_boundary)
            .map(|p| after_match + p + 1)
            .unwrap_or(content.len());
        let sentence = content[sentence_start..sentence_end].trim();
        if !sentence.is_empty() {
            let line = content[..abs_pos].matches('\n').count() + 1;
            let short: String = sentence.chars().take(SEARCH_SNIPPET_CHARS).collect();
            let truncated = short.len() < sentence.len();
            out.push((line, short, truncated));
        }
        search_pos = after_match;
    }
    out
}

/// Lines of `content` containing `pattern`, once each: (1-based line,
/// trimmed line cut to [`SEARCH_SNIPPET_CHARS`], line was cut).
pub(crate) fn line_hits(content: &str, pattern: &str) -> Vec<(usize, String, bool)> {
    if pattern.is_empty() {
        return Vec::new();
    }
    content
        .lines()
        .enumerate()
        .filter(|(_, line)| line.contains(pattern))
        .map(|(i, line)| {
            let trimmed = line.trim();
            let short: String = trimmed.chars().take(SEARCH_SNIPPET_CHARS).collect();
            let truncated = short.len() < trimmed.len();
            (i + 1, short, truncated)
        })
        .collect()
}

/// Marker lines of `content` (see [`TodoMatch`]): (1-based line, trimmed
/// text cut to [`TODO_TEXT_CHARS`], text was cut).
pub(crate) fn todo_hits(content: &str, mode: TodoMatch) -> Vec<(usize, String, bool)> {
    content
        .lines()
        .enumerate()
        .filter_map(|(i, line)| {
            let trimmed = line.trim();
            if !has_todo_marker(trimmed, mode) {
                return None;
            }
            let short: String = trimmed.chars().take(TODO_TEXT_CHARS).collect();
            let truncated = short.len() < trimmed.len();
            Some((i + 1, short, truncated))
        })
        .collect()
}

/// Whether `line` has a TODO marker by the rules of `mode`.
pub(crate) fn has_todo_marker(line: &str, mode: TodoMatch) -> bool {
    if mode == TodoMatch::Loose {
        let upper = line.to_uppercase();
        return upper.contains("TODO") || upper.contains("FIXME") || upper.contains("HACK");
    }
    let is_word = |c: Option<char>| c.is_some_and(|c| c.is_alphanumeric() || c == '_');
    let bytes = line.as_bytes();
    TODO_MARKERS.iter().any(|marker| {
        let n = marker.len();
        (0..(bytes.len() + 1).saturating_sub(n)).any(|i| {
            let word = &bytes[i..i + n];
            if !word.eq_ignore_ascii_case(marker.as_bytes()) {
                return false;
            }
            // The match is ASCII, so `i` and `i + n` are char boundaries.
            let before = line[..i].chars().next_back();
            let after = line[i + n..].chars().next();
            !is_word(before)
                && !is_word(after)
                && (word == marker.as_bytes()
                    || before == Some('@')
                    || matches!(after, Some(':' | '(' | '!')))
        })
    })
}

#[cfg(test)]
mod tests;
