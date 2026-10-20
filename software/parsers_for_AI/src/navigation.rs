//! Project navigation commands and MCP tools on top of the navguard module:
//! map, symbols, find, lines, search, todos, changes. Each returns compact
//! text for a model to read, or JSON.

use navguard::api as nav;
use serde::Serialize;
use std::path::{Path, PathBuf};

/// Default caps for MCP calls, so one call cannot flood the context; the
/// text says when a list was cut.
pub const MCP_MAX_FILES: usize = 1000;
pub const MCP_MAX_HITS: usize = 200;
pub const MCP_MAX_LINES: usize = 2000;

/// The project root for a call: `root` when given, otherwise the current
/// directory. Every path a command reads must lie inside it.
pub fn resolve_root(root: Option<&str>) -> PathBuf {
    match root.filter(|r| !r.trim().is_empty()) {
        Some(root) => PathBuf::from(root),
        None => PathBuf::from("."),
    }
}

/// The project root for an MCP call: `root` (relative to the working
/// directory) or the working directory; it must lie inside `boundary`, the
/// server root.
pub fn confined_root(root: Option<&str>, boundary: &Path) -> Result<PathBuf, String> {
    let resolved = resolve_root(root);
    if !resolved.is_dir() {
        return Err(format!(
            "project root is not a directory: {}",
            resolved.display()
        ));
    }
    if !crate::confine::is_inside(&resolved, boundary) {
        return Err(format!(
            "root is outside the server root {}: {} (start the server with mcp --root DIR to serve another folder)",
            navguard::paths::display_root(boundary),
            resolved.display()
        ));
    }
    Ok(resolved)
}

fn json<T: Serialize>(value: &T) -> String {
    serde_json::to_string(value).unwrap_or_else(|e| format!("{{\"error\":\"{}\"}}", e))
}

fn cut_note(shown: usize, total: usize, what: &str, option: &str) -> String {
    if shown < total {
        format!(
            "note: showing {} of {} {}; raise {} to see more\n",
            shown, total, what, option
        )
    } else {
        String::new()
    }
}

pub fn map(root: &Path, max_files: Option<usize>, as_json: bool) -> Result<String, String> {
    let map = nav::map(root, &nav::MapOptions { max_files })?;
    if as_json {
        return Ok(json(&map));
    }
    Ok(map_text(&map.root, &map, "max_files"))
}

/// The MCP resource `project://tree`: the map of `root` (shown absolute),
/// at most [`MCP_MAX_FILES`] files.
pub fn tree(root: &Path) -> Result<String, String> {
    let map = nav::map(
        root,
        &nav::MapOptions {
            max_files: Some(MCP_MAX_FILES),
        },
    )?;
    Ok(map_text(
        &navguard::paths::display_root(root),
        &map,
        "max_files of the map tool",
    ))
}

/// The MCP resource `project://state`: [`nav::state`] as JSON.
pub fn state(root: &Path) -> Result<String, String> {
    Ok(json(&nav::state(root)?))
}

/// The text of `map`; `cap_option` names what raises the file cap.
fn map_text(shown_root: &str, map: &nav::ProjectMap, cap_option: &str) -> String {
    let mut out = format!(
        "map {}: {} files, {} lines, ~{} tokens\n",
        shown_root, map.totals.files, map.totals.lines, map.totals.tokens
    );
    out.push_str(&cut_note(
        map.files.len(),
        map.totals.files,
        "files",
        cap_option,
    ));
    out.push_str(&map.tree);
    if !map.tree.ends_with('\n') {
        out.push('\n');
    }
    out.push_str("files (lines, ~tokens):\n");
    for f in &map.files {
        if f.binary {
            out.push_str(&format!("  {}  binary, {} bytes\n", f.path, f.bytes));
        } else {
            out.push_str(&format!("  {}  {} ~{}\n", f.path, f.lines, f.tokens));
        }
    }
    out
}

pub fn symbols(root: &Path, path: Option<&str>, as_json: bool) -> Result<String, String> {
    let files = nav::symbols(root, path)?;
    if as_json {
        return Ok(json(&files));
    }
    let total: usize = files.iter().map(|f| f.symbols.len()).sum();
    let mut out = format!("symbols: {} in {} file(s)\n", total, files.len());
    for file in &files {
        out.push_str(&format!("== {} ({})\n", file.path, file.symbols.len()));
        for s in &file.symbols {
            out.push_str(&format!(
                "  {}{}-{}  {} {}\n",
                "  ".repeat(s.depth),
                s.line_start,
                s.line_end,
                s.kind,
                s.name
            ));
        }
    }
    Ok(out)
}

/// `exact`: the symbol name equals `pattern` (see `FindOptions::exact`)
/// instead of name, label or path containing it.
pub fn find(
    root: &Path,
    pattern: &str,
    case_sensitive: bool,
    exact: bool,
    max_hits: Option<usize>,
    as_json: bool,
) -> Result<String, String> {
    // One more than asked, to know whether the list was cut.
    let options = nav::FindOptions {
        case_sensitive,
        exact,
        max_hits: max_hits.map(|n| n + 1),
        ..Default::default()
    };
    let mut hits = nav::find_symbols(root, pattern, &options)?;
    let cut = max_hits.is_some_and(|n| hits.len() > n);
    if let Some(n) = max_hits {
        hits.truncate(n);
    }
    if as_json {
        return Ok(json(&hits));
    }
    let mut out = format!(
        "find '{}'{}: {} symbol(s){}\n",
        pattern,
        if exact { " (exact name)" } else { "" },
        hits.len(),
        if cut {
            " (more exist; raise max_hits)"
        } else {
            ""
        }
    );
    for h in &hits {
        out.push_str(&format!(
            "{}:{}-{}  {} {}{}\n",
            h.path,
            h.symbol.line_start,
            h.symbol.line_end,
            h.symbol.kind,
            h.symbol.name,
            h.symbol
                .parent
                .as_deref()
                .map(|p| format!("  (in {})", p))
                .unwrap_or_default()
        ));
    }
    Ok(out)
}

pub fn lines(
    root: &Path,
    path: &str,
    start: usize,
    end: Option<usize>,
    max_lines: Option<usize>,
    as_json: bool,
) -> Result<String, String> {
    let end = match (end, max_lines) {
        (Some(e), Some(max)) if e >= start && e - start + 1 > max => Some(start + max - 1),
        (end, _) => end,
    };
    let lines = nav::read_lines(root, path, start, end)?;
    if as_json {
        return Ok(json(&lines));
    }
    let mut out = String::new();
    for l in &lines {
        out.push_str(&format!("{:>6}| {}\n", l.line, l.text));
    }
    Ok(out)
}

/// `context`: `auto` (default: the line in code, the sentence in prose),
/// `line` or `sentence`.
pub fn search(
    root: &Path,
    pattern: &str,
    path: Option<&str>,
    max_hits: Option<usize>,
    context: Option<&str>,
    as_json: bool,
) -> Result<String, String> {
    let context = match context {
        None => nav::SearchContext::Auto,
        Some(name) => nav::SearchContext::parse(name).ok_or_else(|| {
            format!(
                "invalid search context '{}': expected auto, line or sentence",
                name
            )
        })?,
    };
    let mut hits = nav::search_text(root, pattern, path, max_hits.map(|n| n + 1), context)?;
    let cut = max_hits.is_some_and(|n| hits.len() > n);
    if let Some(n) = max_hits {
        hits.truncate(n);
    }
    if as_json {
        return Ok(json(&hits));
    }
    Ok(text_hits(&format!("search '{}'", pattern), &hits, cut))
}

/// `loose`: the old substring match (`nav::TodoMatch::Loose`).
pub fn todos(
    root: &Path,
    loose: bool,
    max_hits: Option<usize>,
    as_json: bool,
) -> Result<String, String> {
    let mut hits = nav::todos(root, nav::TodoMatch::loose(loose))?;
    let total = hits.len();
    if let Some(n) = max_hits {
        hits.truncate(n);
    }
    if as_json {
        return Ok(json(&hits));
    }
    Ok(text_hits("todos", &hits, hits.len() < total))
}

fn text_hits(title: &str, hits: &[nav::TextHit], cut: bool) -> String {
    let mut out = format!(
        "{}: {} hit(s){}\n",
        title,
        hits.len(),
        if cut {
            " (more exist; raise max_hits)"
        } else {
            ""
        }
    );
    for h in hits {
        out.push_str(&format!(
            "{}:{}: {}{}\n",
            h.path,
            h.line,
            h.text.replace('\n', " "),
            if h.truncated { " ..." } else { "" }
        ));
    }
    out
}

pub fn changes(root: &Path, persist: bool, as_json: bool) -> Result<String, String> {
    let report = nav::changes(root, persist)?;
    if as_json {
        return Ok(json(&report));
    }
    let mut out = if report.cold {
        format!(
            "changes: no snapshot yet, {} file(s) listed as new{}\n",
            report.new,
            if report.persisted {
                "; snapshot saved"
            } else {
                "; pass persist to save a snapshot"
            }
        )
    } else {
        format!(
            "changes: {} new, {} changed, {} deleted{}\n",
            report.new,
            report.changed,
            report.deleted,
            if report.persisted {
                "; snapshot updated"
            } else {
                ""
            }
        )
    };
    if let Some(error) = &report.persist_error {
        out.push_str(&format!("warning: snapshot not saved: {}\n", error));
    }
    for c in &report.changes {
        out.push_str(&format!("{:<8} {}  {} lines\n", c.status, c.path, c.lines));
    }
    Ok(out)
}
