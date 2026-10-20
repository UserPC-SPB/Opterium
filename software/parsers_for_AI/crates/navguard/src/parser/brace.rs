//! Symbol detection for brace languages (Rust, JS/TS, Go, C/C++, C#, Java,
//! Kotlin) and the generic mode.
//!
//! One pass over the masked lines keeps a stack of open `{` blocks, so each
//! line knows its container (file level, the body of a known symbol, or some
//! other block). Signatures are only recognized at the start of a line that
//! is not inside an open parenthesis of the current block. The range of a
//! symbol runs to the brace closing its body, or to the end of the
//! declaration statement when there is no body.

use super::lexer::LexLine;
use super::{indent, py_class_re, py_def_re, Lang, RawSym, TYPE_KINDS};
use std::collections::HashMap;

/// How a declaration without a body ends.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Style {
    /// Only at `;` (Rust, C/C++, C#, Java).
    Semi,
    /// At `;` or at the end of a line that does not continue the statement
    /// (JS/TS, Go, Kotlin, generic).
    Asi,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum OnDecl {
    Report,
    Skip,
}

struct Candidate {
    name: String,
    kind: &'static str,
    style: Style,
    on_decl: OnDecl,
    /// Range by indentation instead of braces (generic `...:` headers).
    indent_block: bool,
    /// An assignment-like definition (`type X = { .. } | Y;`, `const f = () => {..};`):
    /// braces do not open a body, the symbol ends with its statement.
    statement: bool,
}

fn cand(name: impl Into<String>, kind: &'static str, style: Style, on_decl: OnDecl) -> Candidate {
    Candidate {
        name: name.into(),
        kind,
        style,
        on_decl,
        indent_block: false,
        statement: false,
    }
}

fn statement(mut c: Candidate) -> Candidate {
    c.statement = true;
    c
}

/// The innermost block enclosing a line.
enum Ctx<'a> {
    File,
    Sym { kind: &'static str, name: &'a str },
    Other,
}

struct Block {
    sym: Option<usize>,
    /// `extern "C" {` blocks do not change the container.
    transparent: bool,
    saved_paren: i32,
}

/// Kinds whose bodies may contain C-family function definitions.
const CONTAINER_KINDS: &[&str] = &[
    "class",
    "struct",
    "enum",
    "union",
    "interface",
    "record",
    "object",
    "namespace",
];

/// Words that start statements, never definitions.
const STMT_KEYWORDS: &[&str] = &[
    "return",
    "if",
    "else",
    "for",
    "foreach",
    "while",
    "do",
    "switch",
    "case",
    "try",
    "catch",
    "finally",
    "throw",
    "new",
    "delete",
    "goto",
    "sizeof",
    "typeof",
    "alignof",
    "decltype",
    "using",
    "namespace",
    "yield",
    "await",
    "co_await",
    "co_return",
    "co_yield",
    "static_assert",
    "assert",
    "lock",
    "fixed",
    "checked",
    "unchecked",
    "synchronized",
    "typedef",
    "break",
    "continue",
    "throws",
    "is",
    "as",
    "in",
    "nameof",
    "when",
    "with",
    "elif",
    "import",
    "package",
    "defined",
    "noexcept",
    "__attribute__",
    "__declspec",
    "alignas",
    "function",
    "super",
    "this",
];

pub(super) fn scan(lang: Lang, lines: &[&str], lexed: &[LexLine]) -> Vec<RawSym> {
    let mut syms: Vec<RawSym> = Vec::new();
    let mut blocks: Vec<Block> = Vec::new();
    let mut body_at: HashMap<(usize, usize), usize> = HashMap::new();
    let mut paren: i32 = 0;
    // Prose in unknown file types has unbalanced parentheses.
    let track_parens = lang != Lang::Generic;
    // Continuation lines of the last signature (`: base(x)`, `where ...`)
    // are never signatures themselves.
    let mut signature_until: Option<usize> = None;

    for i in 0..lines.len() {
        let lx = &lexed[i];
        let in_signature = signature_until.is_some_and(|l| i <= l);
        if lx.code_start
            && !in_signature
            && (paren == 0 || !track_parens)
            && !lines[i].trim().is_empty()
        {
            let ctx = context(&blocks, &syms);
            if let Some(c) = detect(lang, lines, lexed, i, &ctx) {
                if c.indent_block {
                    let end = indent::indent_extent(lines, i);
                    syms.push(RawSym {
                        label: lines[i].trim().to_string(),
                        name: c.name,
                        kind: c.kind,
                        start: i,
                        end,
                    });
                } else {
                    let ext = extent(lexed, i, c.style, c.statement);
                    // Statements (`const f = () => {`) may contain nested
                    // definitions on their following lines.
                    signature_until = Some(match ext.body {
                        Some((line, _)) => line,
                        None if c.statement => i,
                        None => ext.end,
                    });
                    if !(ext.decl && c.on_decl == OnDecl::Skip) {
                        if let Some(pos) = ext.body {
                            body_at.insert(pos, syms.len());
                        }
                        // An unbalanced body would run to the end of the
                        // file; indentation is the better guess.
                        let end = if ext.unclosed {
                            indent::indent_extent(lines, i)
                        } else {
                            ext.end
                        };
                        syms.push(RawSym {
                            label: lines[i].trim().to_string(),
                            name: c.name,
                            kind: c.kind,
                            start: i,
                            end,
                        });
                    }
                }
            }
        }
        for (col, &ch) in lx.masked.iter().enumerate() {
            match ch {
                '(' | '[' => paren += 1,
                ')' | ']' => paren = (paren - 1).max(0),
                '{' => {
                    let sym = body_at.get(&(i, col)).copied();
                    blocks.push(Block {
                        sym,
                        transparent: sym.is_none() && is_extern_block(lines[i]),
                        saved_paren: paren,
                    });
                    paren = 0;
                }
                '}' => {
                    if let Some(b) = blocks.pop() {
                        paren = b.saved_paren;
                    }
                }
                _ => {}
            }
        }
    }
    syms
}

fn is_extern_block(line: &str) -> bool {
    let t = line.trim_start();
    let t = t.strip_prefix("unsafe ").unwrap_or(t).trim_start();
    t.starts_with("extern \"") && !t.contains("fn ")
}

fn context<'a>(blocks: &[Block], syms: &'a [RawSym]) -> Ctx<'a> {
    for b in blocks.iter().rev() {
        if b.transparent {
            continue;
        }
        return match b.sym {
            Some(idx) => Ctx::Sym {
                kind: syms[idx].kind,
                name: &syms[idx].name,
            },
            None => Ctx::Other,
        };
    }
    Ctx::File
}

struct Extent {
    end: usize,
    body: Option<(usize, usize)>,
    decl: bool,
    /// The body never closed (unbalanced braces or a lexer miss).
    unclosed: bool,
}

/// Lines a declaration may spend at bracket depth 0 before its body opens
/// (`where` clauses, return types on their own line, Allman braces).
const MAX_IDLE_SIG_LINES: usize = 8;
/// Upper bound for a signature, including long parameter lists.
const MAX_SIG_LINES: usize = 200;

fn extent(lexed: &[LexLine], start: usize, style: Style, statement: bool) -> Extent {
    let mut paren: i32 = 0;
    let mut brace: i32 = 0;
    let mut body: Option<(usize, usize)> = None;
    let mut idle = 0;
    let decl_at = |end: usize| Extent {
        end,
        body: None,
        decl: true,
        unclosed: false,
    };
    for (l, lx) in lexed.iter().enumerate().skip(start) {
        for (col, &ch) in lx.masked.iter().enumerate() {
            match ch {
                '(' | '[' => paren += 1,
                ')' | ']' => {
                    paren -= 1;
                    if paren < 0 && body.is_none() && brace == 0 {
                        return decl_at(l);
                    }
                }
                '{' => {
                    if !statement && body.is_none() && paren <= 0 && brace == 0 {
                        body = Some((l, col));
                    }
                    brace += 1;
                }
                '}' => {
                    brace -= 1;
                    if brace <= 0 {
                        if body.is_some() {
                            return Extent {
                                end: l,
                                body,
                                decl: false,
                                unclosed: false,
                            };
                        }
                        if brace < 0 {
                            return decl_at(l);
                        }
                    }
                }
                ';' if body.is_none() && paren <= 0 && brace == 0 => return decl_at(l),
                _ => {}
            }
        }
        if body.is_none() {
            if paren <= 0 && brace == 0 {
                if style == Style::Asi && !asi_continues(lexed, l) {
                    return decl_at(l);
                }
                idle += 1;
                if idle > MAX_IDLE_SIG_LINES {
                    return decl_at(start);
                }
            }
            if l - start > MAX_SIG_LINES {
                return decl_at(start);
            }
        }
    }
    match body {
        Some(_) => Extent {
            end: lexed.len().saturating_sub(1),
            body,
            decl: false,
            unclosed: true,
        },
        None => decl_at(start),
    }
}

fn masked_str(lx: &LexLine) -> String {
    lx.masked.iter().collect()
}

/// Whether a statement ending (at bracket depth 0) on line `l` continues on
/// the next line in an ASI language.
fn asi_continues(lexed: &[LexLine], l: usize) -> bool {
    let cur = masked_str(&lexed[l]);
    let cur = cur.trim_end();
    if cur.ends_with("=>") || cur.ends_with(|c: char| ",=|&+*/?:.".contains(c)) {
        return true;
    }
    for next in lexed.iter().skip(l + 1).take(3) {
        let s = masked_str(next);
        let s = s.trim_start();
        if s.is_empty() {
            continue;
        }
        return s.starts_with(|c: char| "{|&.?:,=".contains(c))
            || ["extends", "implements", "where", "throws"]
                .iter()
                .any(|w| starts_with_word(s, w));
    }
    false
}

fn is_ident_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

fn starts_with_word(s: &str, word: &str) -> bool {
    s.strip_prefix(word)
        .is_some_and(|rest| !rest.starts_with(is_ident_char))
}

fn strip_prefix_re<'t>(re: &regex::Regex, t: &'t str) -> &'t str {
    match re.find(t) {
        Some(m) if m.start() == 0 => &t[m.end()..],
        _ => t,
    }
}

fn detect(lang: Lang, lines: &[&str], lexed: &[LexLine], i: usize, ctx: &Ctx) -> Option<Candidate> {
    let t = lines[i].trim();
    match lang {
        Lang::Rust => detect_rust(t, Style::Semi),
        Lang::Js => detect_js(t, lines, lexed, i, ctx),
        Lang::Go => detect_go(t),
        Lang::CLike | Lang::CSharp | Lang::Java => detect_cfamily(lang, t, lexed, i, ctx),
        Lang::Kotlin => detect_kotlin(t),
        Lang::Generic => detect_generic(t),
        Lang::Python => None,
    }
}

// ---------------------------------------------------------------- Rust

fn detect_rust(t: &str, style: Style) -> Option<Candidate> {
    let t = strip_prefix_re(lazy_regex!(r"^(?:#!?\[[^\]]*\]\s*)+"), t);
    const VIS: &str = r"(?:pub(?:\s*\([^)]*\))?\s+)?";
    let fn_re = lazy_regex!(&format!(
        r#"^{VIS}(?:(?:default|const|async|unsafe|safe|extern(?:\s+"[^"]*")?)\s+)*fn\s+(?:r#)?([^\W\d]\w*)"#
    ));
    if let Some(c) = fn_re.captures(t) {
        return Some(cand(&c[1], "fn", style, OnDecl::Report));
    }
    let item_re = lazy_regex!(&format!(
        r"^{VIS}(?:(?:unsafe|auto)\s+)*(struct|enum|union|trait|type|mod)\s+(?:r#)?([^\W\d]\w*)"
    ));
    if let Some(c) = item_re.captures(t) {
        let kind = match &c[1] {
            "struct" => "struct",
            "enum" => "enum",
            "union" => "union",
            "trait" => "trait",
            "type" => "type",
            _ => "mod",
        };
        return Some(cand(&c[2], kind, style, OnDecl::Report));
    }
    if lazy_regex!(r"^(?:unsafe\s+)?impl(?:\s|<|\{|$)").is_match(t) {
        return Some(cand(rust_impl_name(t), "impl", style, OnDecl::Report));
    }
    if let Some(c) = lazy_regex!(r"^macro_rules!\s*([^\W\d]\w*)").captures(t) {
        return Some(cand(&c[1], "macro", style, OnDecl::Report));
    }
    None
}

/// `impl<T: Clone> Display for Point<T> where ... {` -> `Display for Point<T>`.
fn rust_impl_name(t: &str) -> String {
    let s = t.strip_prefix("unsafe").map_or(t, str::trim_start);
    let s = s.strip_prefix("impl").unwrap_or(s).trim_start();
    let s = skip_angle_group(s).trim_start();
    let s = &s[..s.find('{').unwrap_or(s.len())];
    let s = match s.find(" where") {
        Some(p) => &s[..p],
        None => s,
    };
    let name = s.trim();
    if name.is_empty() {
        "impl".to_string()
    } else {
        name.to_string()
    }
}

/// Skips a leading balanced `<...>` group (`->` does not close it).
fn skip_angle_group(s: &str) -> &str {
    if !s.starts_with('<') {
        return s;
    }
    let mut depth = 0;
    let mut prev = ' ';
    for (i, c) in s.char_indices() {
        match c {
            '<' => depth += 1,
            '>' if prev != '-' => {
                depth -= 1;
                if depth == 0 {
                    return &s[i + 1..];
                }
            }
            _ => {}
        }
        prev = c;
    }
    s
}

// ---------------------------------------------------------------- JS / TS

const JS_ID: &str = r"[\p{L}_$][\w$]*";

fn detect_js(t: &str, lines: &[&str], lexed: &[LexLine], i: usize, ctx: &Ctx) -> Option<Candidate> {
    let t = strip_prefix_re(lazy_regex!(r"^(?:@[\w.]+(?:\([^)]*\))?\s+)+"), t);
    if let Some(c) = lazy_regex!(&format!(
        r"^(?:export\s+)?(default\s+)?(?:declare\s+)?(?:async\s+)?function\b\s*\*?\s*({JS_ID})?"
    ))
    .captures(t)
    {
        let name = match (c.get(2), c.get(1)) {
            (Some(n), _) => n.as_str(),
            (None, Some(_)) => "default",
            (None, None) => return None,
        };
        return Some(cand(name, "fn", Style::Asi, OnDecl::Report));
    }
    if let Some(c) = lazy_regex!(&format!(
        r"^(?:export\s+)?(default\s+)?(?:declare\s+)?(?:abstract\s+)?class\b\s*({JS_ID})?"
    ))
    .captures(t)
    {
        let name = c
            .get(2)
            .map(|m| m.as_str())
            .filter(|n| *n != "extends" && *n != "implements");
        let name = match (name, c.get(1)) {
            (Some(n), _) => n,
            (None, Some(_)) => "default",
            (None, None) => return None,
        };
        return Some(cand(name, "class", Style::Asi, OnDecl::Report));
    }
    let simple: [(&regex::Regex, &'static str); 4] = [
        (
            lazy_regex!(&format!(
                r"^(?:export\s+)?(?:default\s+)?(?:declare\s+)?interface\s+({JS_ID})"
            )),
            "interface",
        ),
        (
            lazy_regex!(&format!(
                r"^(?:export\s+)?(?:declare\s+)?type\s+({JS_ID})\s*(?:<|=)"
            )),
            "type",
        ),
        (
            lazy_regex!(&format!(
                r"^(?:export\s+)?(?:declare\s+)?(?:const\s+)?enum\s+({JS_ID})"
            )),
            "enum",
        ),
        (
            lazy_regex!(&format!(
                r#"^(?:export\s+)?(?:declare\s+)?(?:namespace|module)\s+({JS_ID}(?:\.{JS_ID})*|"[^"]+"|'[^']+')\s*\{{"#
            )),
            "namespace",
        ),
    ];
    for (re, kind) in simple {
        if let Some(c) = re.captures(t) {
            let name = c[1].trim_matches(['"', '\'']);
            let found = cand(name, kind, Style::Asi, OnDecl::Report);
            return Some(if kind == "type" {
                statement(found)
            } else {
                found
            });
        }
    }
    if let Some(c) = lazy_regex!(&format!(
        r"^(?:export\s+)?(?:default\s+)?(?:const|let|var)\s+({JS_ID})\s*(?::[^=]*)?=\s*(?:async\s+)?(.*)$"
    ))
    .captures(t)
    {
        let rest = c.get(2).map_or("", |m| m.as_str());
        if js_function_value(rest, lines[i], lexed, i) {
            return Some(statement(cand(&c[1], "fn", Style::Asi, OnDecl::Report)));
        }
        return None;
    }
    // File-level `exports.handler = async (event) => {`, `Foo.prototype.bar = function () {`.
    if matches!(ctx, Ctx::File) {
        if let Some(c) = lazy_regex!(&format!(
            r"^({JS_ID}(?:\.{JS_ID})*)\s*=\s*(?:async\s+)?(.*)$"
        ))
        .captures(t)
        {
            let rest = c.get(2).map_or("", |m| m.as_str());
            if js_function_value(rest, lines[i], lexed, i) {
                return Some(statement(cand(&c[1], "fn", Style::Asi, OnDecl::Report)));
            }
        }
    }
    if matches!(ctx, Ctx::Sym { kind: "class", .. }) {
        const MODS: &str = r"(?:(?:public|private|protected|static|readonly|abstract|override|async|declare|accessor)\s+)*";
        if let Some(c) = lazy_regex!(&format!(
            r"^{MODS}(#?{JS_ID})\s*[?!]?\s*(?::[^=]*)?=\s*(?:async\s+)?(?:function\b|(?:\([^)]*\)|{JS_ID})\s*(?::[^=]*)?=>)"
        ))
        .captures(t)
        {
            return Some(statement(cand(&c[1], "fn", Style::Asi, OnDecl::Report)));
        }
        if let Some(c) = lazy_regex!(&format!(
            r#"^{MODS}(?:(?:get|set)\s+)?\*?\s*(#?{JS_ID}|\[[^\]]+\]|"[^"]*"|'[^']*')\s*[?!]?\s*(?:<[^>]*>)?\s*\("#
        ))
        .captures(t)
        {
            let name = &c[1];
            if !STMT_KEYWORDS.contains(&name) {
                return Some(cand(name, "fn", Style::Asi, OnDecl::Report));
            }
        }
    }
    None
}

/// Whether the right-hand side of `const x = <rest>` is a function:
/// `function`, `ident =>`, or `(params) =>` (params may span lines).
fn js_function_value(rest: &str, raw: &str, lexed: &[LexLine], i: usize) -> bool {
    let r = rest.trim_start();
    if starts_with_word(r, "function") {
        return true;
    }
    if lazy_regex!(&format!(r"^{JS_ID}\s*=>")).is_match(r) {
        return true;
    }
    let r = skip_angle_group(r).trim_start();
    if !r.starts_with('(') {
        return false;
    }
    // `rest` runs to the end of the raw line, so its column is known.
    let col = raw.trim_end().chars().count() - r.chars().count();
    let mut depth = 0;
    let mut after = String::new();
    let mut closed_on: Option<usize> = None;
    for (l, lx) in lexed.iter().enumerate().skip(i).take(30) {
        let from = if l == i { col } else { 0 };
        for &ch in lx.masked.iter().skip(from) {
            if closed_on.is_some() {
                after.push(ch);
                continue;
            }
            match ch {
                '(' => depth += 1,
                ')' => {
                    depth -= 1;
                    if depth == 0 {
                        closed_on = Some(l);
                    }
                }
                _ => {}
            }
        }
        after.push(' ');
        if closed_on.is_some_and(|c| l > c) {
            break;
        }
    }
    let a = after.trim_start();
    a.starts_with("=>") || lazy_regex!(r"^:[^=;{]*=>").is_match(a)
}

// ---------------------------------------------------------------- Go

fn detect_go(t: &str) -> Option<Candidate> {
    if let Some(c) = lazy_regex!(r"^func\s*(\([^)]*\))?\s*([^\W\d]\w*)\s*[\[(]").captures(t) {
        let kind = if c.get(1).is_some() { "method" } else { "fn" };
        return Some(cand(&c[2], kind, Style::Asi, OnDecl::Report));
    }
    if let Some(c) = lazy_regex!(r"^type\s+([^\W\d]\w*)(.*)$").captures(t) {
        let rest = c.get(2).map_or("", |m| m.as_str()).trim_start();
        // Skip type parameters `[T any]` (not array lengths like `[4]`).
        let rest = match lazy_regex!(r"^\[[^\]]*\s[^\]]*\]").find(rest) {
            Some(m) => rest[m.end()..].trim_start(),
            None => rest,
        };
        let kind = if starts_with_word(rest, "struct") {
            "struct"
        } else if starts_with_word(rest, "interface") {
            "interface"
        } else {
            "type"
        };
        let found = cand(&c[1], kind, Style::Asi, OnDecl::Report);
        return Some(if kind == "type" {
            statement(found)
        } else {
            found
        });
    }
    None
}

// ---------------------------------------------------------------- Kotlin

const KT_MODS: &str = r"(?:(?:public|private|protected|internal|open|override|abstract|final|inline|noinline|crossinline|suspend|operator|infix|tailrec|external|actual|expect|data|sealed|annotation|enum|inner|value|companion|const|lateinit|fun)\s+)*";

fn detect_kotlin(t: &str) -> Option<Candidate> {
    let t = strip_prefix_re(lazy_regex!(r"^(?:@[\w.:]+(?:\([^)]*\))?\s+)+"), t);
    if let Some(c) = lazy_regex!(&format!(
        r"^{KT_MODS}fun\s+(?:<[^>]*>\s*)?(?:[\w.<>?, ]+?\.)?([^\W\d]\w*|`[^`]+`)\s*\("
    ))
    .captures(t)
    {
        return Some(cand(&c[1], "fn", Style::Asi, OnDecl::Report));
    }
    if let Some(c) = lazy_regex!(&format!(
        r"^({KT_MODS})(class|interface|object|typealias)\s+([^\W\d]\w*)"
    ))
    .captures(t)
    {
        let kind = match &c[2] {
            "class" if starts_with_word(&c[1], "enum") || c[1].contains(" enum ") => "enum",
            "class" => "class",
            "interface" => "interface",
            "object" => "object",
            _ => "type",
        };
        let found = cand(&c[3], kind, Style::Asi, OnDecl::Report);
        return Some(if kind == "type" {
            statement(found)
        } else {
            found
        });
    }
    None
}

// ---------------------------------------------------------------- C family

fn cf_class_re() -> &'static regex::Regex {
    lazy_regex!(
        r"^(?:template\s*<.*>\s*)?(?:(?:public|private|protected|internal|static|final|abstract|sealed|partial|readonly|ref|unsafe|new|file|strictfp|non-sealed|export|inline|typedef)\s+)*(class|struct|interface|enum(?:\s+(?:class|struct))?|record(?:\s+(?:class|struct))?|union|namespace)\b\s*(?:(?:__declspec|alignas|__attribute__)\s*\(\(?[^)]*\)?\)\s*|\[\[[^\]]*\]\]\s*)*([^\W\d]\w*(?:(?:::|\.)[^\W\d]\w*)*)?(.*)$"
    )
}

fn detect_cfamily(
    lang: Lang,
    t: &str,
    lexed: &[LexLine],
    i: usize,
    ctx: &Ctx,
) -> Option<Candidate> {
    let managed = lang != Lang::CLike;
    let t = strip_prefix_re(
        lazy_regex!(r"^(?:(?:\[\[[^\]]*\]\]|\[[^\]]*\]|@[\w.]+(?:\([^)]*\))?)\s*)+"),
        t,
    );
    if t.is_empty() || t.starts_with('#') {
        return None;
    }
    if let Some(c) = cf_class_re().captures(t) {
        // Anonymous `struct {` / `namespace {` blocks are not symbols.
        let found = c.get(2)?;
        {
            let mut name = found.as_str();
            let tail = c.get(3).map_or("", |m| m.as_str());
            let head_tail = tail.split(['{', ';']).next().unwrap_or("");
            let mut ok = true;
            if !managed {
                ok = !head_tail.contains(['(', '=', '*']);
                let next = head_tail.trim_start();
                let word: String = next.chars().take_while(|c| is_ident_char(*c)).collect();
                if ok && !word.is_empty() && word != "final" {
                    // `class EXPORT_MACRO Name {` names the class after the macro.
                    let after = next[word.len()..].trim_start();
                    let macro_like = name
                        .chars()
                        .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_');
                    if macro_like
                        && (after.is_empty()
                            || after.starts_with([':', '{'])
                            || starts_with_word(after, "final"))
                    {
                        name = &next[..word.len()];
                    } else {
                        ok = false;
                    }
                }
            }
            if ok {
                let kind = match c[1].split_whitespace().next().unwrap_or("") {
                    "class" => "class",
                    "struct" => "struct",
                    "interface" => "interface",
                    "enum" => "enum",
                    "record" => "record",
                    "union" => "union",
                    _ => "namespace",
                };
                let on_decl = if managed {
                    OnDecl::Report
                } else {
                    OnDecl::Skip
                };
                return Some(cand(name, kind, Style::Semi, on_decl));
            }
        }
        // Not a type definition (`struct foo *make(void) {`): maybe a function.
    }
    let in_type = matches!(ctx, Ctx::Sym { kind, .. } if TYPE_KINDS.contains(kind));
    let eligible = match ctx {
        Ctx::File => true,
        Ctx::Sym { kind, .. } => CONTAINER_KINDS.contains(kind),
        Ctx::Other => false,
    };
    if !eligible {
        return None;
    }
    let prev = if lang == Lang::CLike
        && matches!(
            ctx,
            Ctx::File
                | Ctx::Sym {
                    kind: "namespace",
                    ..
                }
        ) {
        previous_code_line(lexed, i)
    } else {
        None
    };
    let name = c_function_name(t, ctx, prev.as_deref())?;
    // Member declarations (`void f();`) are symbols; file-level prototypes are not.
    let on_decl = if in_type {
        OnDecl::Report
    } else {
        OnDecl::Skip
    };
    Some(cand(name, "fn", Style::Semi, on_decl))
}

fn previous_code_line(lexed: &[LexLine], i: usize) -> Option<String> {
    lexed[..i]
        .iter()
        .rev()
        .map(|l| masked_str(l).trim().to_string())
        .find(|s| !s.is_empty())
}

fn head_chars_ok(pre: &str) -> bool {
    pre.chars()
        .all(|c| is_ident_char(c) || c.is_whitespace() || ":<>,*&[].?$".contains(c))
}

fn has_stmt_keyword(pre: &str) -> bool {
    pre.split(|c: char| !is_ident_char(c))
        .any(|w| STMT_KEYWORDS.contains(&w))
}

/// Byte index where the trailing identifier (optionally `~`-prefixed) of `s` starts.
fn trailing_ident_start(s: &str) -> Option<usize> {
    let mut start = s.len();
    for (i, c) in s.char_indices().rev() {
        if is_ident_char(c) {
            start = i;
        } else {
            break;
        }
    }
    if start == s.len() || s[start..].starts_with(|c: char| c.is_ascii_digit()) {
        return None;
    }
    if s[..start].ends_with('~') {
        start -= 1;
    }
    Some(start)
}

/// Strips a trailing balanced `<...>` group.
fn strip_trailing_generic(s: &str) -> Option<&str> {
    let mut depth = 0;
    for (i, c) in s.char_indices().rev() {
        match c {
            '>' => depth += 1,
            '<' => {
                depth -= 1;
                if depth == 0 {
                    return Some(s[..i].trim_end());
                }
            }
            _ => {}
        }
    }
    None
}

/// Recognizes `type name(` / `Qualified::name(` / constructors and returns
/// the (qualified) name.
fn c_function_name(t: &str, ctx: &Ctx, prev: Option<&str>) -> Option<String> {
    let first = t.chars().next()?;
    if !(first.is_alphabetic() || first == '_' || first == '~' || t.starts_with("::")) {
        return None;
    }
    let paren = t.find('(')?;
    let head = t[..paren].trim_end();
    if head.is_empty() {
        return None;
    }

    // Operator overloads: `bool operator==(`, `Foo& Foo::operator=(`, `operator()(`.
    if let Some(k) = find_word(head, "operator") {
        let op = head[k..].trim();
        let op = if op == "operator" {
            "operator()".to_string()
        } else {
            op.split_whitespace().collect::<Vec<_>>().join(" ")
        };
        let before = head[..k].trim_end();
        let (qual, pre) = split_qualifier(before);
        if !head_chars_ok(pre) || has_stmt_keyword(pre) {
            return None;
        }
        return Some(format!("{}{}", qual, op));
    }

    let mut h = head;
    if h.ends_with('>') {
        h = strip_trailing_generic(h)?;
    }
    let name_start = trailing_ident_start(h)?;
    let ident = h[name_start..].trim_start_matches('~');
    if STMT_KEYWORDS.contains(&ident) {
        return None;
    }
    let (qual, pre) = split_qualifier(&h[..name_start]);
    let name = format!("{}{}", qual, &h[name_start..]);
    if !head_chars_ok(pre) || has_stmt_keyword(pre) {
        return None;
    }
    if pre.is_empty() {
        let qualified = qual.contains("::");
        let ok = match ctx {
            Ctx::File
            | Ctx::Sym {
                kind: "namespace", ..
            } => qualified || prev.is_some_and(is_return_type_line),
            Ctx::Sym { name: owner, .. } => ident == *owner,
            Ctx::Other => false,
        };
        if !ok {
            return None;
        }
    }
    Some(name)
}

fn find_word(s: &str, word: &str) -> Option<usize> {
    let mut from = 0;
    while let Some(p) = s[from..].find(word) {
        let at = from + p;
        let before_ok = !s[..at].ends_with(is_ident_char);
        let after_ok = !s[at + word.len()..].starts_with(is_ident_char);
        if before_ok && after_ok {
            return Some(at);
        }
        from = at + word.len();
    }
    None
}

/// Splits `pre Qual::Inner<T>::` into (`Qual::Inner<T>::`, `pre`).
fn split_qualifier(s: &str) -> (&str, &str) {
    let mut q = s.len();
    loop {
        let before = &s[..q];
        let sep = if before.ends_with("::") {
            2
        } else if before.ends_with('.') {
            1
        } else {
            break;
        };
        let mut b = &before[..before.len() - sep];
        if b.ends_with('>') {
            match strip_trailing_generic(b) {
                Some(stripped) => b = stripped,
                None => break,
            }
        }
        match trailing_ident_start(b) {
            Some(start) => q = start,
            None => {
                // Global scope `::name`.
                if b.trim().is_empty() {
                    q = b.len();
                }
                break;
            }
        }
    }
    (&s[q..], s[..q].trim())
}

/// GNU style puts the return type on its own line: `static int\nfoo (void)`.
fn is_return_type_line(prev: &str) -> bool {
    prev.starts_with(|c: char| c.is_alphabetic() || c == '_')
        && prev
            .chars()
            .all(|c| is_ident_char(c) || c.is_whitespace() || "*&:<>,".contains(c))
        && !prev.ends_with(',')
        && !has_stmt_keyword(prev)
}

// ---------------------------------------------------------------- generic

/// Unknown file types: keyword forms of all supported languages. Python-like
/// headers ending with `:` get indentation ranges.
fn detect_generic(t: &str) -> Option<Candidate> {
    let py = py_def_re()
        .captures(t)
        .map(|c| (c[1].to_string(), "fn"))
        .or_else(|| {
            py_class_re()
                .captures(t)
                .map(|c| (c[1].to_string(), "class"))
        });
    if let Some((name, kind)) = &py {
        let code = t.split(" #").next().unwrap_or(t).trim_end();
        if code.ends_with(':') {
            let mut c = cand(name.clone(), kind, Style::Asi, OnDecl::Report);
            c.indent_block = true;
            return Some(c);
        }
    }
    // Type-like keywords also start English sentences ("type of ...");
    // outside known languages they must look like code.
    let code_shaped = t.ends_with(['{', ';', '(', '=', '}']) || t.contains(" = ");
    if let Some(c) = detect_rust(t, Style::Asi).or_else(|| detect_go(t)) {
        if matches!(c.kind, "fn" | "method" | "macro") || code_shaped {
            return Some(c);
        }
    }
    if let Some(c) = lazy_regex!(&format!(
        r"^(?:export\s+)?(?:default\s+)?(?:async\s+)?function\b\s*\*?\s*({JS_ID})"
    ))
    .captures(t)
    {
        return Some(cand(&c[1], "fn", Style::Asi, OnDecl::Report));
    }
    if let Some(c) = cf_class_re().captures(t) {
        if let (Some(name), true) = (c.get(2), code_shaped) {
            let kind = match c[1].split_whitespace().next().unwrap_or("") {
                "class" => "class",
                "struct" => "struct",
                "interface" => "interface",
                "enum" => "enum",
                "record" => "record",
                "union" => "union",
                _ => "namespace",
            };
            return Some(cand(name.as_str(), kind, Style::Asi, OnDecl::Report));
        }
    }
    // `def name` / `class Name` without a colon (pre-1.1 generic behaviour).
    py.map(|(name, kind)| cand(name, kind, Style::Asi, OnDecl::Report))
}
