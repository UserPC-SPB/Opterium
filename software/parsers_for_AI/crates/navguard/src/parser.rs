//! Heuristic symbol indexer.
//!
//! Finds definitions (functions, methods, types, modules, ...) and protocol
//! anchors (`[sX]` ... `[sXend]`) in a source file without building an AST.
//!
//! * The language is chosen from the file extension: Rust, Python,
//!   JavaScript/TypeScript, Go, C/C++, C#, Java and Kotlin have dedicated
//!   rules; every other file uses a generic mode that recognizes the keyword
//!   forms of all of them (the pre-1.1 behaviour, now with nesting).
//! * Comments and string literals are masked first ([`lexer`]), so braces
//!   inside `"{"` or `// }` do not break symbol ranges.
//! * Brace languages end a symbol at its matching closing brace (or at the
//!   `;` / end of statement for declarations without a body); Python and
//!   generic `...:` blocks end by indentation. Multi-line signatures are
//!   supported.
//! * Nested symbols are reported with `depth` and `parent`, derived from
//!   range containment; a `fn` whose parent is a type-like symbol (class,
//!   impl, trait, ...) is reported as `method`.

/// Compiles a regex once per call site (textually visible in the child
/// modules declared below).
macro_rules! lazy_regex {
    ($re:expr $(,)?) => {{
        static RE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
        RE.get_or_init(|| regex::Regex::new($re).expect("valid symbol regex"))
    }};
}

mod brace;
mod indent;
mod lexer;

use regex::Regex;
use std::fs;
use std::path::Path;

/// One symbol found in a file.
#[derive(Debug, Clone)]
pub struct Anchor {
    /// The trimmed first line of the definition (for protocol anchors the
    /// tag itself, e.g. `[s1-1]`). This is what `--grep` prints.
    pub label: String,
    /// The symbol's identifier: `new`, `Point`, `Display for Point` (impl),
    /// `Foo::bar` (C++ out-of-line definition), `[s1-1]` (protocol anchor).
    pub name: String,
    /// One of [`KINDS`].
    pub kind: &'static str,
    /// 1-based first line.
    pub line_start: usize,
    /// 1-based last line, inclusive (closing brace, last body line, or the
    /// closing protocol tag).
    pub line_end: usize,
    /// Token estimate of the range's text ([`crate::text::estimate_tokens`]).
    pub tokens: usize,
    /// Nesting depth, 0 = top level.
    pub depth: usize,
    /// Name of the enclosing symbol, if any.
    pub parent: Option<String>,
    pub anchor_type: AnchorType,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AnchorType {
    Code,
    Protocol,
}

/// Every value [`Anchor::kind`] can take.
pub const KINDS: &[&str] = &[
    "fn",
    "method",
    "class",
    "struct",
    "enum",
    "union",
    "trait",
    "impl",
    "interface",
    "type",
    "mod",
    "namespace",
    "macro",
    "record",
    "object",
    "anchor",
];

/// Kinds whose direct `fn` children are reported as `method`.
pub(crate) const TYPE_KINDS: &[&str] = &[
    "class",
    "struct",
    "enum",
    "union",
    "trait",
    "impl",
    "interface",
    "record",
    "object",
];

/// Language rules selected from the file extension.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Lang {
    Rust,
    Python,
    Js,
    Go,
    /// C and C++.
    CLike,
    CSharp,
    Java,
    Kotlin,
    Generic,
}

impl Lang {
    pub(crate) fn from_extension(ext: &str) -> Lang {
        match ext.to_ascii_lowercase().as_str() {
            "rs" => Lang::Rust,
            "py" | "pyw" | "pyi" => Lang::Python,
            "js" | "jsx" | "mjs" | "cjs" | "ts" | "tsx" | "mts" | "cts" => Lang::Js,
            "go" => Lang::Go,
            "c" | "h" | "cpp" | "cc" | "cxx" | "c++" | "hpp" | "hh" | "hxx" | "h++" | "ipp"
            | "inl" | "tpp" | "cu" | "cuh" | "ino" => Lang::CLike,
            "cs" => Lang::CSharp,
            "java" => Lang::Java,
            "kt" | "kts" => Lang::Kotlin,
            _ => Lang::Generic,
        }
    }

    fn syntax(self) -> lexer::Syntax {
        match self {
            Lang::Rust => lexer::Syntax::Rust,
            Lang::Python => lexer::Syntax::Python,
            Lang::Js => lexer::Syntax::Js,
            Lang::Go => lexer::Syntax::Go,
            Lang::CLike => lexer::Syntax::CLike,
            Lang::CSharp => lexer::Syntax::CSharp,
            Lang::Java => lexer::Syntax::Java,
            Lang::Kotlin => lexer::Syntax::Kotlin,
            Lang::Generic => lexer::Syntax::Plain,
        }
    }
}

/// A symbol as found by a language scanner (0-based, inclusive lines).
pub(crate) struct RawSym {
    pub label: String,
    pub name: String,
    pub kind: &'static str,
    pub start: usize,
    pub end: usize,
}

pub(crate) fn py_def_re() -> &'static Regex {
    lazy_regex!(r"^(?:async\s+)?def\s+([^\W\d]\w*)")
}

pub(crate) fn py_class_re() -> &'static Regex {
    lazy_regex!(r"^class\s+([^\W\d]\w*)")
}

/// Parses the file at `path` (decoded as [`crate::text::decode`]);
/// unreadable and binary files have no symbols.
pub fn parse_file(path: &str) -> Vec<Anchor> {
    let content = match fs::read(path) {
        Ok(bytes) if !crate::text::is_binary(&bytes) => crate::text::decode(&bytes),
        _ => return Vec::new(),
    };
    let ext = Path::new(path)
        .extension()
        .map(|e| e.to_string_lossy().into_owned())
        .unwrap_or_default();
    parse_source(&content, &ext)
}

/// Parses `content` with the rules for files with extension `extension`
/// (without the dot, case-insensitive; unknown extensions use the generic
/// mode). Symbols are returned in order of their first line. CRLF and a
/// lone CR end a line, as LF does.
pub fn parse_source(content: &str, extension: &str) -> Vec<Anchor> {
    let content = content.strip_prefix('\u{feff}').unwrap_or(content);
    let content = crate::text::normalize_line_breaks(content);
    let lines: Vec<&str> = content.lines().collect();
    let lang = Lang::from_extension(extension);
    let lexed = lexer::lex(&lines, lang.syntax());
    let code = match lang {
        Lang::Python => indent::scan_python(&lines, &lexed),
        _ => brace::scan(lang, &lines, &lexed),
    };
    finalize(code, protocol_anchors(&lines), &lines)
}

/// Finds `[sX]` ... `[sXend]` pairs (each tag alone on its line). Unmatched
/// tags are ignored.
fn protocol_anchors(lines: &[&str]) -> Vec<RawSym> {
    let open_re = lazy_regex!(r"^\[(s[\w-]+)\]$");
    let mut out = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        let Some(caps) = open_re.captures(line.trim()) else {
            continue;
        };
        let id = &caps[1];
        if id.ends_with("end") {
            continue;
        }
        let close = format!("[{}end]", id);
        if let Some(j) = (i + 1..lines.len()).find(|&j| lines[j].trim() == close) {
            let label = format!("[{}]", id);
            out.push(RawSym {
                label: label.clone(),
                name: label,
                kind: "anchor",
                start: i,
                end: j,
            });
        }
    }
    out
}

/// Token estimate of `lines` joined by line breaks.
fn range_tokens(lines: &[&str]) -> usize {
    crate::text::estimate_tokens(&lines.join("\n")) as usize
}

/// Sorts symbols, clamps code symbols to the protocol anchor they start in,
/// derives depth/parent from range containment and computes token estimates.
fn finalize(mut code: Vec<RawSym>, protocol: Vec<RawSym>, lines: &[&str]) -> Vec<Anchor> {
    for sym in &mut code {
        let innermost = protocol
            .iter()
            .filter(|a| a.start < sym.start && sym.start < a.end)
            .min_by_key(|a| a.end - a.start);
        if let Some(anchor) = innermost {
            sym.end = sym.end.min(anchor.end - 1);
        }
    }

    let mut items: Vec<(RawSym, bool)> = code
        .into_iter()
        .map(|s| (s, false))
        .chain(protocol.into_iter().map(|s| (s, true)))
        .collect();
    items.sort_by(|a, b| {
        a.0.start
            .cmp(&b.0.start)
            .then(b.0.end.cmp(&a.0.end))
            .then(b.1.cmp(&a.1))
    });

    let mut out: Vec<Anchor> = Vec::with_capacity(items.len());
    let mut ends: Vec<usize> = Vec::with_capacity(items.len());
    let mut stack: Vec<usize> = Vec::new();
    for (sym, is_protocol) in items {
        while let Some(&top) = stack.last() {
            if ends[top] >= sym.end {
                break;
            }
            stack.pop();
        }
        let parent = stack.last().copied();
        let mut kind = sym.kind;
        if kind == "fn" && parent.is_some_and(|p| TYPE_KINDS.contains(&out[p].kind)) {
            kind = "method";
        }
        let end = sym.end.min(lines.len().saturating_sub(1)).max(sym.start);
        // A protocol anchor's estimate excludes its closing tag line.
        let counted = if is_protocol {
            &lines[sym.start..end]
        } else {
            &lines[sym.start..=end]
        };
        out.push(Anchor {
            label: sym.label,
            name: sym.name,
            kind,
            line_start: sym.start + 1,
            line_end: end + 1,
            tokens: range_tokens(counted),
            depth: stack.len(),
            parent: parent.map(|p| out[p].name.clone()),
            anchor_type: if is_protocol {
                AnchorType::Protocol
            } else {
                AnchorType::Code
            },
        });
        ends.push(end);
        stack.push(out.len() - 1);
    }
    out
}

#[cfg(test)]
mod tests;
