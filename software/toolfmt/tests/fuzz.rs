//! Property-based fuzz harness for the whole pipeline (`run::run`), without
//! extra dependencies.
//!
//! Every case is generated from a deterministic PRNG: random sources in all
//! seven formats (some malformed, empty or not valid UTF-8) and a random
//! template whose selectors are mostly valid for those sources. Each case
//! runs on a worker thread with the production stack size, so a deep
//! recursion shows up as a test failure, and panics are caught per case.
//!
//! Invariants checked for every case:
//! - no panic, no hang; the exit code is plausible for the case (never 4 or
//!   5; 2 only for templates generated as invalid; 6 only with a size cap);
//! - a failed run publishes nothing;
//! - a published document ends with the end marker, `<!-- BLOCK:` appears
//!   only on anchor lines and exactly twice per block, the Block Index and
//!   the character counter are exact, every block fits the effective
//!   `max_block_chars`;
//! - split invariance: the same case with a huge block size yields the same
//!   logical items for every rule result, once fragments are joined by
//!   `Item continuation` and the anchor escaping is decoded;
//! - determinism: a second run gives the same document (volatile fields
//!   aside);
//! - the Effective Template cut from the document compiles and renders back
//!   to the same YAML.
//!
//! Environment:
//! - `ATP_FUZZ_SEED` (decimal or 0x-hex), `ATP_FUZZ_ITERS`: the run;
//! - `ATP_FUZZ_KEEP_GOING=1`: collect failures instead of stopping at the
//!   first one (at most `ATP_FUZZ_MAX_FAILURES`, default 30);
//! - `ATP_FUZZ_FIRST`: the first iteration (to rerun one iteration of a
//!   seed with `ATP_FUZZ_ITERS=1`);
//! - `ATP_FUZZ_TIMEOUT_SECS`: per-run timeout (default 120);
//! - `ATP_FUZZ_WORK_DIR`: where case directories go (default: OS temp dir);
//! - `ATP_FUZZ_KEEP=1`: keep the directories of passing cases too;
//! - `ATP_FUZZ_REPLAY=<dir>`: replay a case saved by a failed run.
//!
//! A failing case is kept under the OS temp dir (the path is printed) as
//! `tpl.yml`, `sources/` and `case.txt`; `ATP_FUZZ_REPLAY` takes that path.
//! If the process aborts (stack overflow, out of memory), the case
//! directory that was not cleaned up is the culprit.

use any_txt_parser::limits::{CliOptions, ExitCode};
use any_txt_parser::render::{ANCHOR_MARK, unescape_anchor_text};
use any_txt_parser::run::{self, RunError};
use any_txt_parser::template::{compile_yaml_str, effective_template_yaml};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{Duration, Instant};

mod support;

const DEFAULT_SEED: u64 = 0x5EED_A7F0_2026;
/// Small enough that a debug `cargo test` stays quick (100 cases took
/// 92-109 s on a busy machine); long campaigns set `ATP_FUZZ_ITERS`.
const DEFAULT_ITERS: u64 = 30;
/// Block size of the differential run: large enough that nothing splits.
const HUGE_BLOCK: u64 = 200_000_000;

// ---------------------------------------------------------------- PRNG

/// splitmix64, plus the case-wide chance (percent) that a text piece is a
/// long run, so that some cases produce items larger than their blocks.
struct Rng(u64, u64);

impl Rng {
    fn new(seed: u64) -> Rng {
        Rng(seed, 10)
    }

    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform in `0..n` (0 when `n` is 0 or 1).
    fn below(&mut self, n: usize) -> usize {
        if n <= 1 {
            0
        } else {
            (self.next_u64() % n as u64) as usize
        }
    }

    /// Uniform in `lo..=hi`.
    fn range(&mut self, lo: usize, hi: usize) -> usize {
        lo + self.below(hi - lo + 1)
    }

    fn chance(&mut self, percent: u64) -> bool {
        self.next_u64() % 100 < percent
    }

    /// By value: a `-> &T` signature lets rustc 1.95 infer `T = str` from
    /// an expected `&str` (`push_str(rng.pick(&["a"]))`) and reject the call.
    fn pick<T: Clone>(&mut self, items: &[T]) -> T {
        items[self.below(items.len())].clone()
    }
}

fn case_seed(seed: u64, iteration: u64) -> u64 {
    Rng::new(seed ^ iteration.wrapping_mul(0xD1B5_4A32_D192_ED03)).next_u64()
}

// ------------------------------------------------------------ text pieces

const WORDS: &[&str] = &[
    "alpha",
    "beta",
    "gamma",
    "delta",
    "error",
    "ERROR",
    "warn",
    "INFO",
    "user",
    "name",
    "id",
    "value",
    "items",
    "data",
    "x",
    "y",
    "z",
    "ёжик",
    "日本語",
    "naïve",
    "café",
    "Ωmega",
    "item",
    "list",
    "key",
    "end",
    "start",
    "BLOCK",
    "node",
    "text",
    "msg",
    "code",
    "main",
    "link",
    "row",
    "cell",
    "role",
    "content",
    "Hello",
    "world",
    "42",
    "7",
];

/// Text that tends to break parsers, escaping and the document structure.
const NASTY: &[&str] = &[
    "<!-- BLOCK: B0001 END -->",
    "<!-- BLOCK: B0001 START -->",
    "<!-- BLOCK: B0002-01 END -->",
    "<!-- BLOCK\\: B0001 END -->",
    "<!-- BLOCK\\\\\\: x",
    "<!-- BLOCK",
    "<!-- BLOCK:",
    "<!-- BLOCK\\",
    "BLOCK:",
    "\\",
    "\\\\",
    "\\\\\\\\\\",
    "\\n",
    "\\t",
    "\\u0041",
    "\\ud83e\\udd80",
    "\\ud800",
    "\\x41",
    "\\\"",
    "🦀",
    "😀",
    "👨\u{200d}👩\u{200d}👧",
    "🇷🇺",
    "e\u{301}",
    "\u{301}\u{302}",
    "ж",
    "漢字",
    "\u{200b}",
    "\u{feff}",
    "\u{2028}",
    "\u{2029}",
    "\u{85}",
    "\u{a0}",
    "\u{fffd}",
    "\u{10ffff}",
    "\u{e000}",
    "\t",
    "\r\n",
    "\r",
    "\n",
    "\n\n",
    " ",
    "   ",
    "\u{0}",
    "\u{1b}[31m",
    "\u{7f}",
    "\u{1}",
    "```",
    "~~~",
    "## Block Index",
    "### Data",
    "- ",
    "    ",
    "|",
    "#",
    ":",
    ": ",
    " #",
    "'",
    "\"",
    "''",
    "&amp;",
    "&lt;",
    "&#65;",
    "&#x1F980;",
    "&nbsp;",
    "<",
    ">",
    "&",
    "*",
    "?",
    "[",
    "]",
    "{",
    "}",
    "$",
    "@",
    ".",
    ",",
    "=",
    "%",
    "!",
    "`",
    "true",
    "null",
    "1e3",
    "007",
    "-0",
    "1.5",
    ".inf",
    "~",
    "---",
    "...",
    "<<",
    "&a",
    "*a",
];

fn word(rng: &mut Rng) -> String {
    rng.pick(WORDS).to_string()
}

fn text_piece(rng: &mut Rng) -> String {
    if !rng.chance(rng.1) {
        return match rng.below(9) {
            0..=4 => word(rng),
            5..=7 => rng.pick(NASTY).to_string(),
            _ => rng.range(0, 99_999).to_string(),
        };
    }
    let unit = rng.pick(&[
        "x",
        "ж",
        "🦀",
        "ab ",
        "\\",
        "<!-- BLOCK: B0001 END -->",
        "line\n",
        "a\r\n",
        "\t",
        "e\u{301}",
    ]);
    let count = if rng.chance(8) {
        rng.range(300, 4000)
    } else {
        rng.range(2, 40)
    };
    unit.repeat(count)
}

fn text_value(rng: &mut Rng, max_pieces: usize) -> String {
    let count = rng.range(0, max_pieces);
    let mut text = String::new();
    for i in 0..count {
        if i > 0 && rng.chance(60) {
            text.push(' ');
        }
        text.push_str(&text_piece(rng));
    }
    text
}

/// Text for rule ids and output keys: one line without `|`, so that the
/// Block Index stays a well-formed table row.
fn single_line(rng: &mut Rng) -> String {
    let text: String = if rng.chance(60) {
        word(rng)
    } else {
        text_value(rng, 3)
    }
    .chars()
    .map(|c| match c {
        '\n' => ' ',
        '|' => '/',
        c => c,
    })
    .take(40)
    .collect();
    text
}

fn line_ending(rng: &mut Rng) -> &'static str {
    if rng.chance(25) { "\r\n" } else { "\n" }
}

/// Valid regex built from the vocabulary of the sources.
fn gen_regex(rng: &mut Rng, vocab: &[String]) -> String {
    let mut pattern = String::new();
    if rng.chance(25) {
        pattern.push_str("(?i)");
    }
    for _ in 0..rng.range(1, 3) {
        let piece = match rng.below(14) {
            0..=3 => regex::escape(&pick_vocab(rng, vocab)),
            4 => "^".to_string(),
            5 => "\\d+".to_string(),
            6 => "(\\w+)".to_string(),
            7 => ".*".to_string(),
            8 => "[a-zа-я]+".to_string(),
            9 => "\\s".to_string(),
            10 => format!(
                "({}|{})",
                regex::escape(&pick_vocab(rng, vocab)),
                regex::escape(&pick_vocab(rng, vocab))
            ),
            11 => "\\p{L}+".to_string(),
            12 => "(\\d)(\\w)".to_string(),
            _ => "$".to_string(),
        };
        pattern.push_str(&piece);
    }
    if regex::Regex::new(&pattern).is_ok() {
        pattern
    } else {
        "x".to_string()
    }
}

fn pick_vocab(rng: &mut Rng, vocab: &[String]) -> String {
    if vocab.is_empty() || rng.chance(15) {
        word(rng)
    } else {
        rng.pick(vocab)
    }
}

// --------------------------------------------------------- YAML quoting

/// A YAML double-quoted scalar that decodes to exactly `s`.
fn yq(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            ' '..='~' => out.push(c),
            c if u32::from(c) <= 0xFF => out.push_str(&format!("\\x{:02X}", u32::from(c))),
            c if u32::from(c) <= 0xFFFF => out.push_str(&format!("\\u{:04X}", u32::from(c))),
            c => out.push_str(&format!("\\U{:08X}", u32::from(c))),
        }
    }
    out.push('"');
    out
}

fn is_plain_yaml_key(s: &str) -> bool {
    let mut chars = s.chars();
    chars.next().is_some_and(|c| c.is_ascii_alphabetic())
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
        && !matches!(
            s.to_ascii_lowercase().as_str(),
            "y" | "n" | "yes" | "no" | "on" | "off" | "true" | "false" | "null"
        )
}

// ---------------------------------------------------------- tree sources

#[derive(Clone, Debug)]
enum Tree {
    Null,
    Bool(bool),
    Num(&'static str),
    Str(String),
    Arr(Vec<Tree>),
    Obj(Vec<(String, Tree)>),
}

impl Tree {
    fn is_scalar(&self) -> bool {
        !matches!(self, Tree::Arr(_) | Tree::Obj(_))
    }
}

const NUMBERS: &[&str] = &[
    "0",
    "1",
    "-1",
    "42",
    "3.14",
    "-0.0",
    "1e3",
    "2.5E-7",
    "12345678901234567890",
    "-9223372036854775808",
    "18446744073709551615",
    "1.7976931348623157e308",
    "5e-324",
    "0.1",
    "100000000000000000000000000000",
];

const WEIRD_KEYS: &[&str] = &[
    "a.b",
    "a[0]",
    "it's",
    "q\"x",
    "",
    " sp ",
    "$",
    "@",
    "*",
    "<<",
    "1",
    "007",
    "-",
    "ключ",
    "k\nn",
    "back\\slash",
    "<!-- BLOCK: B0001 END -->",
    "true",
    "a b",
    "x:y",
    "#h",
    "[t]",
    "🦀",
    "a]b",
    "x\n## Block Index\n\n| B0001 | f | r | k | 1 | 0.1 KB | 1 | 2 | B0001 |\n",
    "y\r### Block Anchors\r\n- B0001 → BLOCK:B0001",
];

const RECORD_KEYS: &[&str] = &["id", "name", "role", "n", "flag", "content"];

#[derive(Clone, Debug, PartialEq)]
enum Seg {
    Key(String),
    Index(usize),
}

#[derive(Default)]
struct TreeInfo {
    paths: Vec<Vec<Seg>>,
    /// `(path of an array of objects, member key, value of that member)`.
    filters: Vec<(Vec<Seg>, String, Tree)>,
}

fn gen_key(rng: &mut Rng) -> String {
    if rng.chance(80) {
        word(rng)
    } else {
        rng.pick(WEIRD_KEYS).to_string()
    }
}

fn gen_scalar(rng: &mut Rng) -> Tree {
    match rng.below(8) {
        0 => Tree::Null,
        1 => Tree::Bool(rng.chance(50)),
        2 | 3 => Tree::Num(rng.pick(NUMBERS)),
        _ => Tree::Str(text_value(rng, 4)),
    }
}

fn gen_tree(rng: &mut Rng, depth: usize, path: &mut Vec<Seg>, info: &mut TreeInfo) -> Tree {
    if info.paths.len() < 400 {
        info.paths.push(path.clone());
    }
    if depth == 0 || rng.chance(30) {
        return gen_scalar(rng);
    }
    if rng.chance(20) {
        // An array of records sharing their keys, the shape filters target.
        let mut items = Vec::new();
        for index in 0..rng.range(1, 5) {
            path.push(Seg::Index(index));
            if info.paths.len() < 400 {
                info.paths.push(path.clone());
            }
            let mut members = Vec::new();
            for key in RECORD_KEYS {
                if rng.chance(70) {
                    path.push(Seg::Key(key.to_string()));
                    if info.paths.len() < 400 {
                        info.paths.push(path.clone());
                    }
                    path.pop();
                    let value = match *key {
                        "n" => Tree::Num(rng.pick(&NUMBERS[..12])),
                        "flag" => Tree::Bool(rng.chance(50)),
                        _ => Tree::Str(if rng.chance(70) {
                            word(rng)
                        } else {
                            text_value(rng, 3)
                        }),
                    };
                    info.filters
                        .push((path.clone(), key.to_string(), value.clone()));
                    members.push((key.to_string(), value));
                }
            }
            path.pop();
            items.push(Tree::Obj(members));
        }
        return Tree::Arr(items);
    }
    if rng.chance(45) {
        let mut items = Vec::new();
        for index in 0..rng.range(0, 5) {
            path.push(Seg::Index(index));
            items.push(gen_tree(rng, depth - 1, path, info));
            path.pop();
        }
        Tree::Arr(items)
    } else {
        let mut members: Vec<(String, Tree)> = Vec::new();
        for _ in 0..rng.range(0, 5) {
            let key = gen_key(rng);
            if members.iter().any(|(existing, _)| *existing == key) {
                continue;
            }
            path.push(Seg::Key(key.clone()));
            let value = gen_tree(rng, depth - 1, path, info);
            path.pop();
            members.push((key, value));
        }
        Tree::Obj(members)
    }
}

/// A root object for formats whose documents are tables or mappings.
fn gen_root_object(rng: &mut Rng, info: &mut TreeInfo) -> Tree {
    let mut path = Vec::new();
    loop {
        let depth = rng.range(1, 5);
        let tree = gen_tree(rng, depth, &mut path, info);
        if matches!(tree, Tree::Obj(_)) {
            return tree;
        }
        info.paths.clear();
        info.filters.clear();
    }
}

fn json_string(rng: &mut Rng, s: &str) -> String {
    let escape_unicode = rng.chance(30);
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            '/' if rng.chance(20) => out.push_str("\\/"),
            // A lone surrogate escape: serde_json rejects the file.
            '\u{e000}' if rng.chance(30) => out.push_str("\\ud800"),
            c if u32::from(c) < 0x20 => out.push_str(&format!("\\u{:04x}", u32::from(c))),
            c if escape_unicode && !c.is_ascii() => {
                let mut units = [0u16; 2];
                for unit in c.encode_utf16(&mut units) {
                    out.push_str(&format!("\\u{:04X}", unit));
                }
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn emit_json(rng: &mut Rng, tree: &Tree, out: &mut String, indent: Option<(usize, &str)>) {
    let newline = |out: &mut String, level: usize| {
        if let Some((_, eol)) = indent {
            out.push_str(eol);
            out.push_str(&"  ".repeat(level));
        }
    };
    let level = indent.map_or(0, |(level, _)| level);
    let deeper = indent.map(|(level, eol)| (level + 1, eol));
    match tree {
        Tree::Null => out.push_str("null"),
        Tree::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Tree::Num(n) => out.push_str(n),
        Tree::Str(s) => out.push_str(&json_string(rng, s)),
        Tree::Arr(items) => {
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                newline(out, level + 1);
                emit_json(rng, item, out, deeper);
            }
            if !items.is_empty() {
                newline(out, level);
            }
            out.push(']');
        }
        Tree::Obj(members) => {
            out.push('{');
            for (i, (key, value)) in members.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                newline(out, level + 1);
                out.push_str(&json_string(rng, key));
                out.push_str(if indent.is_some() { ": " } else { ":" });
                emit_json(rng, value, out, deeper);
            }
            if !members.is_empty() {
                newline(out, level);
            }
            out.push('}');
        }
    }
}

fn gen_json(rng: &mut Rng) -> (String, TreeInfo) {
    let mut info = TreeInfo::default();
    if rng.chance(4) {
        // Around serde_json's nesting limit (128).
        let depth = rng.pick(&[126, 127, 128, 129, 300]);
        let text = format!("{}1{}", "[".repeat(depth), "]".repeat(depth));
        return (text, info);
    }
    let mut path = Vec::new();
    let tree = if rng.chance(85) {
        gen_root_object(rng, &mut info)
    } else {
        let depth = rng.range(0, 4);
        gen_tree(rng, depth, &mut path, &mut info)
    };
    let mut out = String::new();
    let indent = if rng.chance(50) {
        Some((0, line_ending(rng)))
    } else {
        None
    };
    emit_json(rng, &tree, &mut out, indent);
    if rng.chance(30) {
        out.push_str(line_ending(rng));
    }
    (out, info)
}

// YAML

fn yaml_key(rng: &mut Rng, key: &str) -> String {
    if is_plain_yaml_key(key) && rng.chance(80) {
        key.to_string()
    } else {
        yq(key)
    }
}

fn yaml_scalar(rng: &mut Rng, tree: &Tree, indent: usize) -> String {
    match tree {
        Tree::Null => rng.pick(&["null", "~", ""]).to_string(),
        Tree::Bool(b) => if *b { "true" } else { "false" }.to_string(),
        Tree::Num(n) if rng.chance(5) => format!("!!str {n}"),
        Tree::Num(n) => n.to_string(),
        Tree::Str(s) if rng.chance(4) => format!("!custom {}", yq(s)),
        Tree::Str(s) => {
            let printable = s.chars().all(|c| {
                matches!(c, ' '..='~')
                    || (u32::from(c) >= 0xA0 && !matches!(c, '\u{2028}' | '\u{2029}' | '\u{feff}'))
            });
            if is_plain_yaml_key(s) && rng.chance(70) {
                s.clone()
            } else if printable && !s.is_empty() && rng.chance(30) {
                format!("'{}'", s.replace('\'', "''"))
            } else if s.contains('\n')
                && !s.starts_with([' ', '\n'])
                && s.chars().all(|c| {
                    c == '\n'
                        || matches!(c, ' '..='~')
                        || u32::from(c) >= 0xA0
                            && !matches!(c, '\u{2028}' | '\u{2029}' | '\u{feff}' | '\u{85}')
                })
                && rng.chance(60)
            {
                let chomp = if s.ends_with('\n') { "+" } else { "-" };
                let style = if rng.chance(70) { "|" } else { ">" };
                let pad = " ".repeat(indent + 2);
                let mut block = format!("{style}{chomp}");
                for line in s.split('\n') {
                    block.push('\n');
                    if !line.is_empty() {
                        block.push_str(&pad);
                        block.push_str(line);
                    }
                }
                if s.ends_with('\n') {
                    // The last split piece is empty; keep chomping exact enough.
                }
                block
            } else {
                yq(s)
            }
        }
        _ => unreachable!("collections are not scalars"),
    }
}

struct YamlState {
    anchors: Vec<(String, bool)>,
    next_anchor: usize,
}

fn yaml_flow(rng: &mut Rng, tree: &Tree) -> String {
    match tree {
        Tree::Arr(items) => format!(
            "[{}]",
            items
                .iter()
                .map(|item| yaml_flow(rng, item))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Tree::Obj(members) => format!(
            "{{{}}}",
            members
                .iter()
                .map(|(k, v)| format!("{}: {}", yq(k), yaml_flow(rng, v)))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Tree::Str(s) => yq(s),
        Tree::Null => "null".to_string(),
        other => yaml_scalar(rng, other, 0),
    }
}

/// Emits `tree` as the value after `key:` or `-` (the caller wrote that).
fn emit_yaml_value(
    rng: &mut Rng,
    tree: &Tree,
    indent: usize,
    out: &mut String,
    state: &mut YamlState,
) {
    if !tree.is_scalar() && !state.anchors.is_empty() && rng.chance(8) {
        let (name, _) = rng.pick(&state.anchors);
        out.push_str(&format!(" *{name}\n"));
        return;
    }
    match tree {
        Tree::Arr(items) if items.is_empty() => out.push_str(" []\n"),
        Tree::Obj(members) if members.is_empty() => out.push_str(" {}\n"),
        Tree::Arr(_) | Tree::Obj(_) if rng.chance(15) => {
            out.push(' ');
            out.push_str(&yaml_flow(rng, tree));
            out.push('\n');
        }
        Tree::Arr(items) => {
            let mut anchored = None;
            if rng.chance(15) {
                let name = format!("a{}", state.next_anchor);
                state.next_anchor += 1;
                out.push_str(&format!(" &{name}"));
                anchored = Some(name);
            }
            out.push('\n');
            for item in items {
                out.push_str(&" ".repeat(indent));
                out.push('-');
                emit_yaml_value(rng, item, indent + 2, out, state);
            }
            // Registered after the children, so no alias refers to an
            // enclosing node.
            if let Some(name) = anchored {
                state.anchors.push((name, false));
            }
        }
        Tree::Obj(members) => {
            let mut anchored = None;
            if rng.chance(20) {
                let name = format!("a{}", state.next_anchor);
                state.next_anchor += 1;
                out.push_str(&format!(" &{name}"));
                anchored = Some(name);
            }
            out.push('\n');
            let maps: Vec<String> = state
                .anchors
                .iter()
                .filter(|(_, is_map)| *is_map)
                .map(|(name, _)| name.clone())
                .collect();
            if !maps.is_empty() && rng.chance(25) {
                out.push_str(&" ".repeat(indent));
                if maps.len() > 1 && rng.chance(40) {
                    out.push_str(&format!("<<: [*{}, *{}]\n", maps[0], maps[maps.len() - 1]));
                } else {
                    out.push_str(&format!("<<: *{}\n", rng.pick(&maps)));
                }
            }
            for (key, value) in members {
                out.push_str(&" ".repeat(indent));
                out.push_str(&yaml_key(rng, key));
                out.push(':');
                emit_yaml_value(rng, value, indent + 2, out, state);
                if rng.below(300) == 0 {
                    // A repeated key: an error that quotes the key path.
                    out.push_str(&" ".repeat(indent));
                    out.push_str(&yq(key));
                    out.push_str(": 1\n");
                }
            }
            if let Some(name) = anchored {
                state.anchors.push((name, true));
            }
        }
        scalar => {
            out.push(' ');
            out.push_str(&yaml_scalar(rng, scalar, indent));
            out.push('\n');
        }
    }
}

fn gen_yaml(rng: &mut Rng) -> (String, TreeInfo) {
    let mut info = TreeInfo::default();
    if rng.chance(4) {
        let depth = rng.pick(&[126, 127, 128, 129, 300]);
        let text = format!("{}1{}\n", "[".repeat(depth), "]".repeat(depth));
        return (text, info);
    }
    let documents = if rng.chance(25) { rng.range(2, 4) } else { 1 };
    let mut out = String::new();
    if rng.chance(20) {
        out.push_str("# generated\n");
    }
    for index in 0..documents {
        if index > 0 || rng.chance(30) {
            out.push_str("---");
            if rng.chance(10) {
                out.push_str(" # doc");
            }
            out.push('\n');
        }
        if rng.chance(10) {
            // An empty document.
            if rng.chance(50) {
                out.push_str("~\n");
            }
            continue;
        }
        let tree = gen_root_object(rng, &mut info);
        let mut state = YamlState {
            anchors: Vec::new(),
            next_anchor: 0,
        };
        let Tree::Obj(members) = &tree else {
            unreachable!()
        };
        for (key, value) in members {
            out.push_str(&yaml_key(rng, key));
            out.push(':');
            emit_yaml_value(rng, value, 2, &mut out, &mut state);
        }
        if members.is_empty() {
            out.push_str("{}\n");
        }
        if rng.chance(15) {
            out.push_str("...\n");
        }
    }
    (out, info)
}

// TOML

fn toml_key(rng: &mut Rng, key: &str) -> String {
    if !key.is_empty()
        && key
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        && rng.chance(85)
    {
        key.to_string()
    } else if !key.contains(['\'', '\n', '\r'])
        && key.chars().all(|c| !c.is_control())
        && rng.chance(30)
    {
        format!("'{key}'")
    } else {
        toml_basic(key)
    }
}

fn toml_basic(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            c if c.is_control() => out.push_str(&format!("\\u{:04X}", u32::from(c))),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn toml_number(n: &str) -> String {
    if n.parse::<i64>().is_ok() {
        n.to_string()
    } else {
        let value: f64 = n.parse().unwrap();
        let text = format!("{value:?}");
        if text.contains(['.', 'e', 'E']) || text.contains("inf") {
            text
        } else {
            format!("{text}.0")
        }
    }
}

fn toml_inline(rng: &mut Rng, tree: &Tree) -> Option<String> {
    Some(match tree {
        Tree::Null => return None,
        Tree::Bool(b) => b.to_string(),
        Tree::Num(n) => toml_number(n),
        Tree::Str(s) => {
            if rng.chance(8) {
                rng.pick(&[
                    "1979-05-27T07:32:00Z",
                    "1979-05-27",
                    "07:32:00",
                    "1979-05-27T00:32:00.999999-07:00",
                ])
                .to_string()
            } else if !s.contains(['\'', '\n', '\r'])
                && s.chars().all(|c| !c.is_control() || c == '\t')
                && rng.chance(25)
            {
                format!("'{s}'")
            } else {
                toml_basic(s)
            }
        }
        Tree::Arr(items) => format!(
            "[{}]",
            items
                .iter()
                .filter_map(|item| toml_inline(rng, item))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Tree::Obj(members) => format!(
            "{{{}}}",
            members
                .iter()
                .filter_map(|(k, v)| toml_inline(rng, v).map(|v| format!(
                    "{} = {}",
                    toml_key(rng, k),
                    v
                )))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    })
}

/// Emits the members of a table: inline values first, then sub-tables and
/// arrays of tables under `header`.
fn emit_toml_table(
    rng: &mut Rng,
    header: &[String],
    members: &[(String, Tree)],
    out: &mut String,
    eol: &str,
) {
    let mut deferred = Vec::new();
    for (key, value) in members {
        let is_table = matches!(value, Tree::Obj(_)) && rng.chance(70);
        let is_table_array = matches!(value, Tree::Arr(items) if !items.is_empty() && items.iter().all(|i| matches!(i, Tree::Obj(_))))
            && rng.chance(70);
        if is_table || is_table_array {
            deferred.push((key, value, is_table_array));
            continue;
        }
        if let Tree::Str(s) = value
            && s.contains('\n')
            && !s.contains("\"\"\"")
            && !s.contains('\\')
            && s.chars()
                .all(|c| !c.is_control() || matches!(c, '\n' | '\t'))
            && rng.chance(30)
        {
            out.push_str(&format!(
                "{} = \"\"\"{}{}\"\"\"{}",
                toml_key(rng, key),
                eol,
                s.replace('\n', eol),
                eol
            ));
            continue;
        }
        if let Some(text) = toml_inline(rng, value) {
            out.push_str(&format!("{} = {}{}", toml_key(rng, key), text, eol));
        }
    }
    for (key, value, is_table_array) in deferred {
        let mut path = header.to_vec();
        path.push(toml_key(rng, key));
        match value {
            Tree::Obj(members) => {
                out.push_str(&format!("{eol}[{}]{eol}", path.join(".")));
                emit_toml_table(rng, &path, members, out, eol);
            }
            Tree::Arr(items) if is_table_array => {
                for item in items {
                    out.push_str(&format!("{eol}[[{}]]{eol}", path.join(".")));
                    if let Tree::Obj(members) = item {
                        emit_toml_table(rng, &path, members, out, eol);
                    }
                }
            }
            _ => {}
        }
    }
}

fn gen_toml(rng: &mut Rng) -> (String, TreeInfo) {
    let mut info = TreeInfo::default();
    let tree = gen_root_object(rng, &mut info);
    let eol = line_ending(rng);
    let mut out = String::new();
    if rng.chance(20) {
        out.push_str(&format!("# config{eol}"));
    }
    if rng.chance(10) {
        out.push_str(&format!("dotted.key.path = 1{eol}"));
    }
    let Tree::Obj(members) = &tree else {
        unreachable!()
    };
    emit_toml_table(rng, &[], members, &mut out, eol);
    (out, info)
}

/// A JSONPath step selecting member `key`.
fn jsonpath_member(rng: &mut Rng, key: &str) -> String {
    let bare = !key.is_empty()
        && key
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-' || u32::from(c) > 0x7f);
    if bare && rng.chance(70) {
        return format!(".{key}");
    }
    let quote = if rng.chance(50) { '\'' } else { '"' };
    let mut out = format!("[{quote}");
    for c in key.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            c if c == quote => {
                out.push('\\');
                out.push(c);
            }
            '\n' => out.push_str("\\n"),
            '\t' if rng.chance(50) => out.push_str("\\t"),
            c if u32::from(c) < 0x20 => out.push_str(&format!("\\u{:04x}", u32::from(c))),
            c if u32::from(c) > 0xFFFF && rng.chance(40) => {
                let mut units = [0u16; 2];
                for unit in c.encode_utf16(&mut units) {
                    out.push_str(&format!("\\u{:04x}", unit));
                }
            }
            c => out.push(c),
        }
    }
    out.push(quote);
    out.push(']');
    out
}

fn jsonpath_literal(rng: &mut Rng, value: &Tree) -> String {
    match value {
        Tree::Null => "null".to_string(),
        Tree::Bool(b) => b.to_string(),
        Tree::Num(n) => n.to_string(),
        Tree::Str(s) => jsonpath_member_quoted(s, if rng.chance(50) { '\'' } else { '"' }),
        _ => "1".to_string(),
    }
}

fn jsonpath_member_quoted(s: &str, quote: char) -> String {
    let mut out = String::new();
    out.push(quote);
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            c if c == quote => {
                out.push('\\');
                out.push(c);
            }
            c if u32::from(c) < 0x20 => out.push_str(&format!("\\u{:04x}", u32::from(c))),
            c => out.push(c),
        }
    }
    out.push(quote);
    out
}

fn jsonpath_for(rng: &mut Rng, info: &TreeInfo) -> String {
    let mut expr = String::from("$");
    if !info.filters.is_empty() && rng.chance(35) {
        let (path, key, value) = rng.pick(&info.filters);
        // The array that holds the records.
        let array = &path[..path.len() - 1];
        expr.push_str(&jsonpath_steps(rng, array));
        let op = rng.pick(&["==", "!=", "<", "<=", ">", ">="]);
        let ws = if rng.chance(30) { " " } else { "" };
        let mut comparisons = vec![format!(
            "@{}{ws}{op}{ws}{}",
            jsonpath_member(rng, &key),
            jsonpath_literal(rng, &value)
        )];
        if rng.chance(30) {
            let (_, other_key, other_value) = rng.pick(&info.filters);
            let joiner = if rng.chance(50) { " && " } else { " || " };
            comparisons.push(joiner.to_string());
            comparisons.push(format!(
                "@{} {} {}",
                jsonpath_member(rng, &other_key),
                rng.pick(&["==", "!=", ">"]),
                jsonpath_literal(rng, &other_value)
            ));
        }
        expr.push_str(&format!("[?({}{ws})]", comparisons.concat()));
        if rng.chance(40) {
            let key = rng.pick(RECORD_KEYS);
            expr.push_str(&jsonpath_member(rng, key));
        }
        return expr;
    }
    let path = if info.paths.is_empty() {
        Vec::new()
    } else {
        rng.pick(&info.paths)
    };
    expr.push_str(&jsonpath_steps(rng, &path));
    if rng.chance(10) {
        expr.push_str(rng.pick(&["..*", ".*", "[*]", "..name", "..id"]));
    }
    if expr == "$" && rng.chance(50) {
        expr.push_str(rng.pick(&["..*", ".*", "[0]", "[0,1]"]));
    }
    expr
}

fn jsonpath_steps(rng: &mut Rng, path: &[Seg]) -> String {
    let mut out = String::new();
    let mut descendant_steps = 0;
    for seg in path {
        match seg {
            Seg::Key(key) => {
                if rng.chance(8) && descendant_steps < 2 {
                    descendant_steps += 1;
                    let member = jsonpath_member(rng, key);
                    match member.strip_prefix('.') {
                        Some(bare) => out.push_str(&format!("..{bare}")),
                        None => out.push_str("..*"),
                    }
                } else if rng.chance(8) {
                    out.push_str(if rng.chance(50) { ".*" } else { "[*]" });
                } else {
                    out.push_str(&jsonpath_member(rng, key));
                }
            }
            Seg::Index(index) => match rng.below(6) {
                0 => out.push_str("[*]"),
                1 => out.push_str(&format!("[{index},{}]", index + 1)),
                2 => out.push_str(&format!("[ {index} ]")),
                3 => out.push_str(".*"),
                _ => out.push_str(&format!("[{index}]")),
            },
        }
    }
    out
}

/// A `path` selector for the node at `path`, or `None` when a key cannot be
/// written in the path syntax.
fn path_selector_for(rng: &mut Rng, path: &[Seg], toml_headers: bool) -> Option<String> {
    let bare = |key: &str| {
        !key.is_empty()
            && key.trim() == key
            && key
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-') || u32::from(c) > 0x7f)
    };
    let mut out = String::new();
    let mut start = 0;
    if toml_headers && rng.chance(40) {
        // `[a.b].c` form for leading bare keys.
        let header: Vec<&str> = path
            .iter()
            .take_while(|seg| matches!(seg, Seg::Key(k) if bare(k) && !k.starts_with(|c: char| c.is_ascii_digit() || c == '-')))
            .map(|seg| match seg {
                Seg::Key(k) => k.as_str(),
                Seg::Index(_) => unreachable!(),
            })
            .take(2)
            .collect();
        if !header.is_empty() {
            out.push_str(&format!("[{}]", header.join(".")));
            start = header.len();
        }
    }
    if out.is_empty() && rng.chance(30) {
        out.push('$');
    }
    for (i, seg) in path.iter().enumerate().skip(start) {
        match seg {
            Seg::Key(key) if key == "*" => return None,
            Seg::Key(key) if bare(key) && rng.chance(80) => {
                if !(out.is_empty() && i == 0) {
                    out.push('.');
                }
                out.push_str(key);
            }
            Seg::Key(key) => {
                if key.contains(']') {
                    return None;
                }
                let quote = if !key.contains('\'') {
                    '\''
                } else if !key.contains('"') {
                    '"'
                } else {
                    return None;
                };
                out.push_str(&format!("[{quote}{key}{quote}]"));
            }
            Seg::Index(index) => {
                if rng.chance(25) {
                    out.push_str("[*]");
                } else {
                    out.push_str(&format!("[{index}]"));
                }
            }
        }
    }
    if out.is_empty() {
        out.push('$');
    }
    if out == "$" && rng.chance(50) {
        out.push_str(".*");
    }
    Some(out)
}

fn tree_selectors(rng: &mut Rng, info: &TreeInfo, jsonpath: bool, toml: bool) -> Vec<Sel> {
    let mut sels = Vec::new();
    for _ in 0..6 {
        if jsonpath && rng.chance(60) {
            sels.push(Sel::new(
                format!(
                    "{{type: jsonpath, expression: {}}}",
                    yq(&jsonpath_for(rng, info))
                ),
                SelKind::Other,
            ));
        } else {
            let path = if info.paths.is_empty() {
                Vec::new()
            } else {
                rng.pick(&info.paths)
            };
            if let Some(expr) = path_selector_for(rng, &path, toml) {
                sels.push(Sel::new(
                    format!("{{type: path, expression: {}}}", yq(&expr)),
                    SelKind::Other,
                ));
            }
        }
    }
    sels
}

// ------------------------------------------------------------------- XML

#[derive(Default)]
struct XmlInfo {
    /// Element names from the root down to every element.
    chains: Vec<Vec<String>>,
    attrs: Vec<(String, String)>,
    texts: Vec<String>,
}

const XML_NAMES: &[&str] = &[
    "item", "a", "b", "entry", "data", "x-y", "_u", "ns.el", "ж", "row",
];
const XML_ATTRS: &[&str] = &["id", "name", "kind", "href", "xml:lang"];

fn xml_text(rng: &mut Rng, s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        match c {
            '&' => out.push_str(if rng.chance(80) { "&amp;" } else { "&#38;" }),
            '<' => out.push_str(if rng.chance(80) { "&lt;" } else { "&#x3C;" }),
            '>' => out.push_str("&gt;"),
            '\t' | '\n' | '\r' => out.push(c),
            c if u32::from(c) < 0x20 || matches!(c, '\u{fffe}' | '\u{ffff}') => out.push('?'),
            c if !c.is_ascii() && rng.chance(5) => out.push_str(&format!("&#x{:X};", u32::from(c))),
            c => out.push(c),
        }
    }
    out
}

fn xml_attr_value(s: &str, quote: char) -> String {
    let mut out = String::new();
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            c if c == quote => out.push_str(if quote == '"' { "&quot;" } else { "&apos;" }),
            '\t' | '\n' | '\r' => out.push(c),
            c if u32::from(c) < 0x20 || matches!(c, '\u{fffe}' | '\u{ffff}') => out.push('?'),
            c => out.push(c),
        }
    }
    out
}

struct XmlGen {
    prefixes: Vec<&'static str>,
    entities: Vec<&'static str>,
    info: XmlInfo,
    malformed: bool,
}

impl XmlGen {
    fn element(
        &mut self,
        rng: &mut Rng,
        name: String,
        depth: usize,
        chain: &mut Vec<String>,
        out: &mut String,
    ) {
        chain.push(name.clone());
        if self.info.chains.len() < 300 {
            self.info.chains.push(chain.clone());
        }
        out.push('<');
        out.push_str(&name);
        let mut used = Vec::new();
        for _ in 0..rng.range(0, 3) {
            let mut attr = rng.pick(XML_ATTRS).to_string();
            if rng.chance(15) && !self.prefixes.is_empty() {
                attr = format!("{}:id", rng.pick(&self.prefixes));
            }
            if used.contains(&attr) {
                continue;
            }
            used.push(attr.clone());
            let value = if rng.chance(70) {
                word(rng)
            } else {
                text_value(rng, 2)
            };
            let quote = if rng.chance(50) { '"' } else { '\'' };
            self.info.attrs.push((attr.clone(), value.clone()));
            let mut escaped = xml_attr_value(&value, quote);
            if !self.entities.is_empty() && rng.chance(10) {
                escaped.push_str(&format!("&{};", rng.pick(&["e1", "e2"])));
            }
            out.push_str(&format!(" {attr}={quote}{escaped}{quote}"));
        }
        let children = if depth == 0 { 0 } else { rng.range(0, 4) };
        if children == 0 && rng.chance(40) {
            out.push_str(if rng.chance(50) { "/>" } else { " />" });
            chain.pop();
            return;
        }
        out.push('>');
        for _ in 0..children {
            match rng.below(10) {
                0..=3 => {
                    let child = self.element_name(rng);
                    self.element(rng, child, depth - 1, chain, out);
                }
                4 | 5 => {
                    let text = text_value(rng, 3);
                    self.info.texts.push(text.clone());
                    out.push_str(&xml_text(rng, &text));
                }
                6 => {
                    let comment = word(rng).replace('-', "_");
                    out.push_str(&format!("<!-- {comment} -->"));
                }
                7 => {
                    let data: String = text_value(rng, 2)
                        .replace("]]>", "]] >")
                        .chars()
                        .map(|c| {
                            if (u32::from(c) < 0x20 && !matches!(c, '\t' | '\n' | '\r'))
                                || matches!(c, '\u{fffe}' | '\u{ffff}')
                            {
                                '?'
                            } else {
                                c
                            }
                        })
                        .collect();
                    out.push_str(&format!("<![CDATA[{data}]]>"));
                }
                8 => out.push_str("<?pi some data?>"),
                _ => {
                    if self.entities.is_empty() {
                        out.push_str(rng.pick(&["&amp;", "&#x1F980;", "&#65;", "&quot;"]));
                    } else {
                        out.push_str(&format!("&{};", rng.pick(&self.entities)));
                    }
                }
            }
        }
        if self.malformed && rng.chance(10) {
            // Unclosed element.
        } else {
            out.push_str(&format!("</{name}>"));
        }
        chain.pop();
    }

    fn element_name(&mut self, rng: &mut Rng) -> String {
        let local = rng.pick(XML_NAMES).to_string();
        if !self.prefixes.is_empty() && rng.chance(20) {
            format!("{}:{}", rng.pick(&self.prefixes), local)
        } else if self.malformed && rng.chance(5) {
            format!("undeclared:{local}")
        } else {
            local
        }
    }
}

fn gen_xml(rng: &mut Rng) -> (String, XmlInfo) {
    if rng.chance(4) {
        let depth = rng.pick(&[127, 128, 129, 400]);
        let text = format!("{}{}", "<a>".repeat(depth), "</a>".repeat(depth));
        return (text, XmlInfo::default());
    }
    let mut generator = XmlGen {
        prefixes: Vec::new(),
        entities: Vec::new(),
        info: XmlInfo::default(),
        malformed: rng.chance(8),
    };
    let eol = line_ending(rng);
    let mut out = String::new();
    if rng.chance(30) {
        out.push_str(&format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?>{eol}"));
    }
    if rng.chance(15) {
        out.push_str(&format!("<!-- prolog -->{eol}"));
    }
    if rng.chance(20) {
        let value = xml_attr_value(&text_value(rng, 2), '"').replace(['%', '&'], "_");
        out.push_str("<!DOCTYPE root [");
        out.push_str(eol);
        out.push_str(&format!("  <!ENTITY e1 \"{value}\">{eol}"));
        out.push_str(&format!("  <!ENTITY e2 'two &e1; x'>{eol}"));
        out.push_str(&format!("  <!ENTITY m \"<b id='m'>m &e1;</b>\">{eol}"));
        out.push_str(&format!(
            "  <!ELEMENT root ANY>{eol}  <!ATTLIST item id CDATA #IMPLIED>{eol}"
        ));
        if rng.chance(20) {
            out.push_str(&format!(
                "  <!ENTITY ext SYSTEM 'file:///etc/hostname'>{eol}"
            ));
        }
        out.push_str(&format!("  <!-- in dtd ]> -->{eol}]>{eol}"));
        generator.entities = vec!["e1", "e2", "m"];
        if rng.chance(10) {
            generator.entities.push("ext");
        }
    }
    let mut root = String::from("<root");
    if rng.chance(30) {
        root.push_str(" xmlns=\"urn:default\"");
    }
    if rng.chance(50) {
        root.push_str(" xmlns:p=\"urn:p\"");
        generator.prefixes.push("p");
    }
    if rng.chance(25) {
        root.push_str(" xmlns:q='urn:q'");
        generator.prefixes.push("q");
    }
    if rng.chance(30) {
        root.push_str(" id='r'");
        generator.info.attrs.push(("id".into(), "r".into()));
    }
    root.push('>');
    out.push_str(&root);
    let mut chain = vec!["root".to_string()];
    generator.info.chains.push(chain.clone());
    for _ in 0..rng.range(0, 5) {
        out.push_str(if rng.chance(50) { eol } else { "" });
        let name = generator.element_name(rng);
        let depth = rng.range(0, 4);
        generator.element(rng, name, depth, &mut chain, &mut out);
    }
    out.push_str(eol);
    out.push_str("</root>");
    if generator.malformed {
        match rng.below(4) {
            0 => out.push_str("<root/>"),
            1 => out.push('&'),
            2 => out = out.replacen("</root>", "</rot>", 1),
            _ => out.push_str("<!-- unterminated"),
        }
    }
    if rng.chance(50) {
        out.push_str(eol);
    }
    (out, generator.info)
}

fn xpath_name_test(rng: &mut Rng, name: &str) -> String {
    if rng.chance(10) {
        "*".to_string()
    } else {
        name.to_string()
    }
}

fn xpath_predicate(rng: &mut Rng, info: &XmlInfo) -> String {
    let quoted = |value: &str| -> Option<String> {
        if !value.contains('\'') {
            Some(format!("'{value}'"))
        } else if !value.contains('"') {
            Some(format!("\"{value}\""))
        } else {
            None
        }
    };
    match rng.below(4) {
        0 => format!("[{}]", rng.range(1, 3)),
        1 if !info.attrs.is_empty() => format!("[@{}]", rng.pick(&info.attrs).0),
        2 if !info.attrs.is_empty() => {
            let (name, value) = rng.pick(&info.attrs);
            match quoted(&value) {
                Some(v) => format!("[ @{name} = {v} ]"),
                None => "[1]".to_string(),
            }
        }
        3 if !info.texts.is_empty() => match quoted(&rng.pick(&info.texts)) {
            Some(v) => format!("[text()={v}]"),
            None => "[2]".to_string(),
        },
        _ => "[1]".to_string(),
    }
}

fn xpath_for(rng: &mut Rng, info: &XmlInfo) -> (String, SelKind) {
    let chain = if info.chains.is_empty() {
        vec!["root".to_string()]
    } else {
        rng.pick(&info.chains)
    };
    let mut expr = String::new();
    if rng.chance(55) {
        for name in &chain {
            expr.push('/');
            expr.push_str(&xpath_name_test(rng, name));
            if rng.chance(15) {
                expr.push_str(&xpath_predicate(rng, info));
            }
        }
    } else {
        let last = chain.len().saturating_sub(rng.range(1, 2));
        for (i, name) in chain.iter().enumerate().skip(last) {
            expr.push_str(if i == last || rng.chance(30) {
                "//"
            } else {
                "/"
            });
            expr.push_str(&xpath_name_test(rng, name));
            if rng.chance(15) {
                expr.push_str(&xpath_predicate(rng, info));
            }
        }
    }
    let attr = info
        .attrs
        .first()
        .map(|(name, _)| name.clone())
        .unwrap_or_else(|| "id".to_string());
    match rng.below(10) {
        0 | 1 => (
            format!(
                "{expr}/@{}",
                if rng.chance(50) {
                    attr
                } else {
                    rng.pick(XML_ATTRS).to_string()
                }
            ),
            SelKind::XPathValue,
        ),
        2 => (format!("{expr}/text()"), SelKind::XPathValue),
        3 => (format!("{expr}//text()"), SelKind::XPathValue),
        4 if rng.chance(30) => ("//@id".to_string(), SelKind::XPathValue),
        _ => (expr, SelKind::XPathElement),
    }
}

fn xml_selectors(rng: &mut Rng, info: &XmlInfo) -> Vec<Sel> {
    (0..6)
        .map(|_| {
            let (expr, kind) = xpath_for(rng, info);
            Sel::new(format!("{{type: xpath, expression: {}}}", yq(&expr)), kind)
        })
        .collect()
}

// ------------------------------------------------------------------ HTML

#[derive(Default)]
struct HtmlInfo {
    tags: Vec<String>,
    classes: Vec<String>,
    ids: Vec<String>,
}

const HTML_TAGS: &[&str] = &[
    "div",
    "p",
    "span",
    "b",
    "i",
    "a",
    "ul",
    "ol",
    "li",
    "table",
    "tr",
    "td",
    "th",
    "pre",
    "code",
    "br",
    "hr",
    "h1",
    "h2",
    "img",
    "script",
    "style",
    "template",
    "section",
    "form",
    "input",
    "textarea",
    "noscript",
    "iframe",
    "video",
    "em",
    "sub",
    "blockquote",
    "dl",
    "dt",
    "dd",
];
const HTML_CLASSES: &[&str] = &["x", "y", "main", "link", "c-1", "note", "ж"];

fn html_text(rng: &mut Rng, s: &str, malformed: bool) -> String {
    let mut out = String::new();
    for c in s.chars() {
        match c {
            '&' if !malformed || rng.chance(80) => out.push_str("&amp;"),
            '<' if !malformed || rng.chance(80) => out.push_str("&lt;"),
            c => out.push(c),
        }
    }
    if rng.chance(10) {
        out.push_str(rng.pick(&["&nbsp;", "&eacute;", "&#x1F980;", "&copy", "&bogus;"]));
    }
    out
}

fn gen_html_element(
    rng: &mut Rng,
    depth: usize,
    malformed: bool,
    info: &mut HtmlInfo,
    out: &mut String,
) {
    let tag = rng.pick(HTML_TAGS);
    info.tags.push(tag.to_string());
    out.push('<');
    out.push_str(tag);
    if rng.chance(40) {
        let class = rng.pick(HTML_CLASSES).to_string();
        info.classes.push(class.clone());
        if rng.chance(30) {
            out.push_str(&format!(" class=\"{class} y\""));
            info.classes.push("y".to_string());
        } else if malformed && rng.chance(30) {
            out.push_str(&format!(" class={class}"));
        } else {
            out.push_str(&format!(" class='{class}'"));
        }
    }
    if rng.chance(20) {
        let id = format!("i{}", rng.range(0, 9));
        info.ids.push(id.clone());
        out.push_str(&format!(" id=\"{id}\""));
    }
    if rng.chance(25) {
        let value = text_value(rng, 2).replace('"', "&quot;");
        out.push_str(&format!(
            " {}=\"{}\"",
            rng.pick(&["href", "src", "alt", "title", "data-x"]),
            value
        ));
    }
    out.push('>');
    if matches!(tag, "br" | "hr" | "img" | "input") {
        return;
    }
    if matches!(tag, "script" | "style" | "textarea") {
        let body = text_value(rng, 3).replace("</", "< /");
        out.push_str(&body);
        out.push_str(&format!("</{tag}>"));
        return;
    }
    let children = if depth == 0 { 0 } else { rng.range(0, 4) };
    for _ in 0..children {
        if rng.chance(55) {
            gen_html_element(rng, depth - 1, malformed, info, out);
        } else if rng.chance(15) {
            out.push_str("<!-- c -->");
        } else {
            let text = text_value(rng, 3);
            out.push_str(&html_text(rng, &text, malformed));
        }
    }
    if malformed && rng.chance(25) {
        return;
    }
    out.push_str(&format!("</{tag}>"));
    if malformed && rng.chance(10) {
        out.push_str("</div></p>");
    }
}

fn gen_html(rng: &mut Rng) -> (String, HtmlInfo) {
    let mut info = HtmlInfo::default();
    let malformed = rng.chance(25);
    let mut out = String::new();
    if rng.chance(60) {
        out.push_str("<!DOCTYPE html>\n");
    }
    let document = rng.chance(70);
    if document {
        out.push_str("<html><head><title>");
        let title = word(rng);
        out.push_str(&html_text(rng, &title, false));
        out.push_str(
            "</title><style>p { color: red }</style><script>var a = 1 < 2;</script></head><body>\n",
        );
    }
    if rng.chance(30) {
        out.push_str("<table><thead><tr><th>h1</th><th>h2</th></tr></thead><tbody>");
        for _ in 0..rng.range(1, 3) {
            out.push_str("<tr>");
            for _ in 0..rng.range(0, 3) {
                out.push_str("<td>");
                let text = text_value(rng, 2);
                out.push_str(&html_text(rng, &text, malformed));
                out.push_str("</td>");
            }
            out.push_str("</tr>");
        }
        out.push_str("</tbody></table>\n");
        info.tags
            .extend(["table", "tr", "td", "th"].map(String::from));
    }
    for _ in 0..rng.range(0, 5) {
        let depth = rng.range(0, 4);
        gen_html_element(rng, depth, malformed, &mut info, &mut out);
        out.push('\n');
    }
    if document && !(malformed && rng.chance(50)) {
        out.push_str("</body></html>\n");
    }
    (out, info)
}

fn css_for(rng: &mut Rng, info: &HtmlInfo) -> String {
    let tag = |rng: &mut Rng| {
        if info.tags.is_empty() {
            "p".to_string()
        } else {
            rng.pick(&info.tags)
        }
    };
    let class = |rng: &mut Rng| {
        if info.classes.is_empty() {
            "x".to_string()
        } else {
            rng.pick(&info.classes)
        }
    };
    let candidate = match rng.below(16) {
        0..=3 => tag(rng),
        4 => format!(".{}", class(rng)),
        5 => match info.ids.first() {
            Some(id) => format!("#{id}"),
            None => "#i1".to_string(),
        },
        6 => format!("{}.{}", tag(rng), class(rng)),
        7 => "[data-x]".to_string(),
        8 => "a[href]".to_string(),
        9 => format!("{} > {}", tag(rng), tag(rng)),
        10 => format!("{} {}", tag(rng), tag(rng)),
        11 => format!("{}:nth-child({})", tag(rng), rng.range(1, 3)),
        12 => format!(":not({})", tag(rng)),
        13 => "*".to_string(),
        14 => format!("{}, {}", tag(rng), tag(rng)),
        _ => format!("{} + {}", tag(rng), tag(rng)),
    };
    if scraper::Selector::parse(&candidate).is_ok() {
        candidate
    } else {
        "p".to_string()
    }
}

fn html_selectors(rng: &mut Rng, info: &HtmlInfo) -> Vec<Sel> {
    (0..6)
        .map(|_| {
            Sel::new(
                format!("{{type: css, expression: {}}}", yq(&css_for(rng, info))),
                SelKind::Css,
            )
        })
        .collect()
}

// -------------------------------------------------------------- Markdown

fn md_inline(rng: &mut Rng) -> String {
    let mut out = String::new();
    for i in 0..rng.range(1, 5) {
        if i > 0 {
            out.push(' ');
        }
        match rng.below(12) {
            0 => out.push_str(&format!("*{}*", word(rng))),
            1 => out.push_str(&format!("**{}**", word(rng))),
            2 => out.push_str(&format!("`{}`", word(rng))),
            3 => out.push_str(&format!(
                "[{}](https://e.test/{} \"t\")",
                word(rng),
                word(rng)
            )),
            4 => out.push_str(&format!("![{}](img/{}.png)", word(rng), rng.range(0, 9))),
            5 => out.push_str("<span>inline</span>"),
            6 => out.push_str("<https://auto.test>"),
            7 => out.push_str("\\*not em\\*"),
            8 => out.push_str("~~strike~~"),
            _ => out.push_str(&text_value(rng, 2).replace(['\n', '\r'], " ")),
        }
    }
    out
}

fn gen_markdown(rng: &mut Rng) -> (String, Vec<String>) {
    let eol = line_ending(rng);
    let mut out = String::new();
    let mut vocab = Vec::new();
    if rng.chance(20) {
        out.push_str(&format!(
            "---{eol}title: {}{eol}tags: [a, b]{eol}---{eol}",
            word(rng)
        ));
    }
    if rng.chance(3) {
        // Deep nesting.
        let depth = rng.range(30, 120);
        for level in 0..depth {
            out.push_str(&format!("{} quote {level}{eol}", ">".repeat(level + 1)));
        }
        for level in 0..depth {
            out.push_str(&format!("{}- item {level}{eol}", "  ".repeat(level)));
        }
    }
    for _ in 0..rng.range(1, 8) {
        let w = word(rng);
        vocab.push(w.clone());
        match rng.below(10) {
            0 => out.push_str(&format!(
                "{} {} {}{eol}",
                "#".repeat(rng.range(1, 6)),
                w,
                md_inline(rng)
            )),
            1 => out.push_str(&format!(
                "{w} heading{eol}{}{eol}",
                if rng.chance(50) { "===" } else { "---" }
            )),
            2 => {
                let marker = rng.pick(&["-", "*", "+", "1.", "2)"]);
                for i in 0..rng.range(1, 4) {
                    let level = rng.range(0, 2).min(i);
                    let task = if rng.chance(20) { "[x] " } else { "" };
                    out.push_str(&format!(
                        "{}{marker} {task}{}{eol}",
                        "  ".repeat(level * 2),
                        md_inline(rng)
                    ));
                }
            }
            3 => {
                let columns = rng.range(1, 3);
                out.push_str(&format!(
                    "| {} |{eol}",
                    vec![w.as_str(); columns].join(" | ")
                ));
                out.push_str(&format!("|{}|{eol}", vec!["---"; columns].join("|")));
                for _ in 0..rng.range(0, 3) {
                    let cells: Vec<String> = (0..rng.range(0, columns + 1))
                        .map(|_| md_inline(rng).replace('|', "\\|"))
                        .collect();
                    out.push_str(&format!("| {} |{eol}", cells.join(" | ")));
                }
            }
            4 => {
                let fence = if rng.chance(70) { "```" } else { "~~~" };
                out.push_str(&format!("{fence}{}{eol}", rng.pick(&["", "rust", "json"])));
                out.push_str(&text_value(rng, 4).replace(fence, "''"));
                out.push_str(eol);
                if rng.chance(90) {
                    out.push_str(fence);
                    out.push_str(eol);
                }
            }
            5 => {
                for level in 1..=rng.range(1, 3) {
                    out.push_str(&format!("{} {}{eol}", ">".repeat(level), md_inline(rng)));
                }
            }
            6 => out.push_str(&format!(
                "<div class=\"x\">{eol}{}{eol}</div>{eol}",
                word(rng)
            )),
            7 => out.push_str(&format!("{}{eol}", rng.pick(&["---", "***", "___"]))),
            8 => out.push_str(&format!("    indented code {w}{eol}")),
            _ => out.push_str(&format!("{}  {eol}{}{eol}", md_inline(rng), md_inline(rng))),
        }
        out.push_str(eol);
    }
    (out, vocab)
}

const MD_NODES: &[&str] = &[
    "heading",
    "h",
    "paragraph",
    "p",
    "list",
    "list_item",
    "li",
    "List-Item",
    "item",
    "table",
    "table_row",
    "tr",
    "row",
    "table_cell",
    "td",
    "cell",
    "code_block",
    "code",
    "codeblock",
    "blockquote",
    "quote",
    "link",
    "image",
    "img",
];

fn markdown_selectors(rng: &mut Rng, vocab: &[String]) -> Vec<Sel> {
    (0..6)
        .map(|_| {
            let node = rng.pick(MD_NODES);
            let yaml = if rng.chance(40) {
                format!(
                    "{{type: markdown, node: {}, pattern: {}}}",
                    yq(node),
                    yq(&gen_regex(rng, vocab))
                )
            } else {
                format!("{{type: markdown, node: {}}}", yq(node))
            };
            Sel::new(yaml, SelKind::Other)
        })
        .collect()
}

// ------------------------------------------------------------------ text

fn gen_text(rng: &mut Rng) -> (String, Vec<String>) {
    let mut out = String::new();
    let mut vocab = Vec::new();
    for i in 0..rng.range(0, 30) {
        let level = rng.pick(&["INFO", "WARN", "ERROR", "DEBUG", "error:", "Error"]);
        let msg = if rng.chance(5) {
            // A very long line.
            text_piece(rng).repeat(rng.range(50, 400))
        } else {
            text_value(rng, 4)
        };
        let w = word(rng);
        vocab.push(w.clone());
        vocab.push(level.to_string());
        let line = match rng.below(4) {
            0 => format!(
                "2026-10-{:02} 12:00:{:02} {level} {w} {msg}",
                rng.range(1, 28),
                i % 60
            ),
            1 => format!("{level}\t{w}\t{msg}"),
            2 => format!("<{}>{w}: {msg}", rng.range(0, 191)),
            _ => msg,
        };
        out.push_str(&line);
        out.push_str(rng.pick(&["\n", "\n", "\n", "\r\n", "\r", "\u{2028}", "\n\n"]));
    }
    if rng.chance(30) {
        out.push_str("no trailing newline");
    }
    (out, vocab)
}

fn text_selectors(rng: &mut Rng, vocab: &[String]) -> Vec<Sel> {
    (0..6)
        .map(|_| {
            let yaml = match rng.below(5) {
                0 | 1 => format!(
                    "{{type: regex, expression: {}}}",
                    yq(&gen_regex(rng, vocab))
                ),
                2 => format!(
                    "{{type: contains, expression: {}}}",
                    yq(&pick_vocab(rng, vocab))
                ),
                3 => format!(
                    "{{type: {}, expression: {}}}",
                    rng.pick(&["starts_with", "ends_with"]),
                    yq(&pick_vocab(rng, vocab))
                ),
                _ => {
                    let from = rng.range(0, 10);
                    format!(
                        "{{type: line, from: {from}, to: {}}}",
                        from + rng.range(0, 10)
                    )
                }
            };
            Sel::new(yaml, SelKind::Other)
        })
        .collect()
}

// --------------------------------------------------------------- sources

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Fmt {
    Json,
    Yaml,
    Toml,
    Xml,
    Html,
    Markdown,
    Text,
}

fn any_format(rng: &mut Rng) -> Fmt {
    rng.pick(&Fmt::ALL)
}

/// A name of some format, as `Format::parse` accepts it.
fn any_alias(rng: &mut Rng) -> &'static str {
    let format = any_format(rng);
    rng.pick(format.aliases())
}

impl Fmt {
    const ALL: [Fmt; 7] = [
        Fmt::Json,
        Fmt::Yaml,
        Fmt::Toml,
        Fmt::Xml,
        Fmt::Html,
        Fmt::Markdown,
        Fmt::Text,
    ];

    fn extensions(self) -> &'static [&'static str] {
        match self {
            Fmt::Json => &["json"],
            Fmt::Yaml => &["yaml", "yml"],
            Fmt::Toml => &["toml"],
            Fmt::Xml => &["xml"],
            Fmt::Html => &["html", "htm"],
            Fmt::Markdown => &["md", "markdown"],
            Fmt::Text => &["txt", "log"],
        }
    }

    /// Names `Format::parse` accepts for this format.
    fn aliases(self) -> &'static [&'static str] {
        match self {
            Fmt::Json => &["json", "JSON"],
            Fmt::Yaml => &["yaml", "yml"],
            Fmt::Toml => &["toml"],
            Fmt::Xml => &["xml"],
            Fmt::Html => &["html", "htm"],
            Fmt::Markdown => &["markdown", "md"],
            Fmt::Text => &["text", "txt", "log", "plain"],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SelKind {
    /// An xpath that ends at elements: every extract mode applies.
    XPathElement,
    /// An xpath that ends at attributes or text: only `extract: text`.
    XPathValue,
    Css,
    Other,
}

#[derive(Clone, Debug)]
struct Sel {
    /// The `select` mapping in flow style.
    yaml: String,
    kind: SelKind,
}

impl Sel {
    fn new(yaml: String, kind: SelKind) -> Sel {
        Sel { yaml, kind }
    }
}

struct Source {
    name: String,
    bytes: Vec<u8>,
    format: Fmt,
    selectors: Vec<Sel>,
}

/// Damages the text: the result is usually not valid in its format.
fn mutate_text(rng: &mut Rng, text: &mut String) {
    let boundaries: Vec<usize> = text
        .char_indices()
        .map(|(i, _)| i)
        .chain([text.len()])
        .collect();
    let at = rng.pick(&boundaries);
    match rng.below(4) {
        0 => text.truncate(at),
        1 => {
            let piece = rng.pick(NASTY).to_string();
            text.insert_str(at, &piece);
        }
        2 => {
            let end = rng.pick(&boundaries);
            let (from, to) = (at.min(end), at.max(end));
            let slice = text[from..to].to_string();
            text.insert_str(to, &slice);
        }
        _ => {
            if let Some(c) = text[at..].chars().next() {
                text.replace_range(at..at + c.len_utf8(), "");
            }
        }
    }
}

fn encode_source(rng: &mut Rng, text: &str, encoding: &str) -> Vec<u8> {
    match encoding {
        "utf-16le" | "utf-16" | "utf-16be" => {
            let big = encoding == "utf-16be";
            let mut bytes = Vec::new();
            if rng.chance(40) {
                bytes.extend(if big { [0xFE, 0xFF] } else { [0xFF, 0xFE] });
            }
            for unit in text.encode_utf16() {
                bytes.extend(if big {
                    unit.to_be_bytes()
                } else {
                    unit.to_le_bytes()
                });
            }
            bytes
        }
        "windows-1251" | "cp1251" => encoding_rs::WINDOWS_1251.encode(text).0.into_owned(),
        _ => {
            let mut bytes = Vec::new();
            if rng.chance(8) || (encoding == "utf-8-sig" && rng.chance(60)) {
                bytes.extend([0xEF, 0xBB, 0xBF]);
            }
            bytes.extend(text.as_bytes());
            if rng.chance(4) && !bytes.is_empty() {
                // Not valid UTF-8.
                let at = rng.below(bytes.len());
                let bad: &[u8] = rng.pick(&[
                    &[0xFF][..],
                    &[0xC3][..],
                    &[0xC0, 0x80][..],
                    &[0xED, 0xA0, 0x80][..],
                ]);
                let tail = bytes.split_off(at);
                bytes.extend(bad);
                bytes.extend(tail);
            }
            bytes
        }
    }
}

fn gen_source(rng: &mut Rng, index: usize, encoding: &str) -> Source {
    let format = rng.pick(&Fmt::ALL);
    let (mut text, selectors) = match format {
        Fmt::Json => {
            let (text, info) = gen_json(rng);
            let selectors = tree_selectors(rng, &info, true, false);
            (text, selectors)
        }
        Fmt::Yaml => {
            let (text, info) = gen_yaml(rng);
            let selectors = tree_selectors(rng, &info, false, false);
            (text, selectors)
        }
        Fmt::Toml => {
            let (text, info) = gen_toml(rng);
            let selectors = tree_selectors(rng, &info, false, true);
            (text, selectors)
        }
        Fmt::Xml => {
            let (text, info) = gen_xml(rng);
            let selectors = xml_selectors(rng, &info);
            (text, selectors)
        }
        Fmt::Html => {
            let (text, info) = gen_html(rng);
            let selectors = html_selectors(rng, &info);
            (text, selectors)
        }
        Fmt::Markdown => {
            let (text, vocab) = gen_markdown(rng);
            let selectors = markdown_selectors(rng, &vocab);
            (text, selectors)
        }
        Fmt::Text => {
            let (text, vocab) = gen_text(rng);
            let selectors = text_selectors(rng, &vocab);
            (text, selectors)
        }
    };
    if rng.chance(10) {
        mutate_text(rng, &mut text);
    }
    if rng.chance(4) {
        text.clear();
    }
    let extension = match rng.below(20) {
        0 => "dat".to_string(),
        1 => String::new(),
        2 => {
            let other = any_format(rng);
            rng.pick(other.extensions()).to_string()
        }
        _ => rng.pick(format.extensions()).to_string(),
    };
    let name = if extension.is_empty() {
        format!("s{index}")
    } else {
        format!("s{index}.{extension}")
    };
    Source {
        name,
        bytes: encode_source(rng, &text, encoding),
        format,
        selectors,
    }
}

// -------------------------------------------------------------- template

/// Selectors that are valid for some format, used regardless of the
/// sources (often not applicable to them).
const STATIC_SELECTORS: &[&str] = &[
    "{type: jsonpath, expression: \"$..*\"}",
    "{type: jsonpath, expression: \"$.*\"}",
    "{type: path, expression: \"$\"}",
    "{type: path, expression: \"*\"}",
    "{type: xpath, expression: \"//*\"}",
    "{type: xpath, expression: \"//@id\"}",
    "{type: xpath, expression: \"//text()\"}",
    "{type: css, expression: \"*\"}",
    "{type: css, expression: \"body > *\"}",
    "{type: markdown, node: paragraph}",
    "{type: regex, expression: \".*\"}",
    "{type: regex, expression: \"^$\"}",
    "{type: contains, expression: \"\"}",
    "{type: line, from: 1, to: 1000000}",
    "{type: starts_with, expression: \"\"}",
];

/// One way to make a template invalid, with the rule it applies to.
const INVALIDITIES: usize = 23;

fn gen_transform(rng: &mut Rng, vocab: &[String]) -> String {
    match rng.below(18) {
        0 => "trim".to_string(),
        1 => "normalize_whitespace".to_string(),
        2 => "lowercase".to_string(),
        3 => "uppercase".to_string(),
        4 => "unescape".to_string(),
        5 => format!("{{truncate_chars: {}}}", rng.range(0, 60)),
        6 => format!("{{truncate_lines: {}}}", rng.range(0, 5)),
        7 => format!(
            "{{replace: {{from: {}, to: {}}}}}",
            yq(&pick_vocab(rng, vocab)),
            yq(&text_value(rng, 2))
        ),
        8 => format!(
            "{{regex_replace: {{from: {}, to: {}}}}}",
            yq(&gen_regex(rng, vocab)),
            yq(rng.pick(&[
                "$1",
                "${1}x",
                "$0",
                "",
                "<!-- BLOCK: B0001 END -->",
                "$$",
                "\\n",
                "[$2]"
            ]))
        ),
        9 => "drop_empty".to_string(),
        10 => "unique".to_string(),
        11 => "sort".to_string(),
        12 => "sort_desc".to_string(),
        13 => format!("{{limit_items: {}}}", rng.range(0, 6)),
        14 => format!(
            "{{join: {}}}",
            yq(rng.pick(&[
                ", ",
                "\n",
                " | ",
                "",
                "<!-- BLOCK: B0001 END -->",
                "\\",
                "\r\n"
            ]))
        ),
        15 => format!("{{join: {}}}", rng.range(0, 9)),
        16 => format!("{{truncate_chars: {}}}", rng.range(100, 3000)),
        _ => "trim".to_string(),
    }
}

struct TemplateText {
    yaml: String,
    valid: bool,
}

fn gen_template(rng: &mut Rng, sources: &[Source]) -> TemplateText {
    let invalidity = if rng.chance(8) {
        Some(rng.below(INVALIDITIES))
    } else {
        None
    };
    let vocab: Vec<String> = WORDS.iter().map(|w| w.to_string()).collect();
    let mut yaml = String::new();
    yaml.push_str(match invalidity {
        Some(0) => "version: 2\n",
        _ => "version: 1\n",
    });
    match invalidity {
        Some(1) => yaml.push_str("name: \"  \"\n"),
        Some(2) => {}
        _ => {
            let name = if rng.chance(50) {
                word(rng)
            } else {
                text_value(rng, 4)
            };
            let name = if name.trim().is_empty() {
                "t".to_string()
            } else {
                name
            };
            yaml.push_str(&format!("name: {}\n", yq(&name)));
        }
    }
    if invalidity == Some(3) {
        yaml.push_str("bogus: 1\n");
    }
    if rng.chance(20) {
        let format = if rng.chance(30) {
            "auto"
        } else {
            any_alias(rng)
        };
        yaml.push_str(&format!("input: {{format: {}}}\n", yq(format)));
    }
    let mut output = Vec::new();
    if rng.chance(75) || matches!(invalidity, Some(11) | Some(22)) {
        let max = if rng.chance(70) {
            rng.range(200, 3000)
        } else {
            rng.range(3000, 40_000)
        };
        let target = match invalidity {
            Some(11) => max + 1,
            _ => rng.range(max / 2, max).max(1),
        };
        let max = if invalidity == Some(22) { 0 } else { max };
        output.push(format!(
            "block: {{target_chars: {target}, max_chars: {max}}}"
        ));
    }
    if rng.chance(8) {
        output.push(format!("max_output_chars: {}", rng.range(2_000, 2_000_000)));
    }
    if !output.is_empty() {
        yaml.push_str(&format!("output: {{{}}}\n", output.join(", ")));
    }
    if rng.chance(35) || invalidity == Some(13) {
        let sort = match invalidity {
            Some(13) => "random",
            _ => rng.pick(&["none", "asc", "desc"]),
        };
        yaml.push_str(&format!(
            "document: {{sort: {sort}, stable_deduplicate: {}, split_blocks: {}}}\n",
            rng.chance(40),
            !rng.chance(10)
        ));
    }
    if invalidity == Some(4) {
        yaml.push_str("rules: []\n");
        return TemplateText { yaml, valid: false };
    }
    yaml.push_str("rules:\n");
    let rule_count = if rng.chance(5) {
        rng.range(6, 12)
    } else {
        rng.range(1, 5)
    };
    let invalid_rule = rng.below(rule_count);
    let mut first_id = String::new();
    for index in 0..rule_count {
        let broken = if index == invalid_rule {
            invalidity
        } else {
            None
        };
        let id = if broken == Some(5) && index > 0 {
            first_id.clone()
        } else {
            let base = single_line(rng);
            format!("{base}#{index}")
        };
        if index == 0 {
            first_id = id.clone();
        }
        let source = if sources.is_empty() {
            None
        } else {
            Some(&sources[rng.below(sources.len())])
        };
        let mut fields = vec![format!("id: {}", yq(&id))];
        let when_format = match broken {
            Some(14) => Some("auto"),
            _ if rng.chance(20) => {
                let format = source.map_or(Fmt::Json, |s| s.format);
                Some(rng.pick(format.aliases()))
            }
            _ if rng.chance(5) => Some(any_alias(rng)),
            _ => None,
        };
        if let Some(format) = when_format {
            fields.push(format!("when: {{format: {}}}", yq(format)));
        }
        let mut selector = match source {
            Some(source) if !source.selectors.is_empty() && rng.chance(85) => {
                rng.pick(&source.selectors)
            }
            _ => {
                let yaml = rng.pick(STATIC_SELECTORS);
                let kind = if yaml.contains("xpath") {
                    if yaml.contains('@') || yaml.contains("text()") {
                        SelKind::XPathValue
                    } else {
                        SelKind::XPathElement
                    }
                } else if yaml.contains("css") {
                    SelKind::Css
                } else {
                    SelKind::Other
                };
                Sel::new(yaml.to_string(), kind)
            }
        };
        match broken {
            Some(7) => {
                selector = Sel::new(
                    "{type: regex, expression: \"(unclosed\"}".into(),
                    SelKind::Other,
                )
            }
            Some(8) => {
                selector = Sel::new(
                    "{type: jsonpath, expression: \"$.a[\"}".into(),
                    SelKind::Other,
                )
            }
            Some(9) => {
                selector = Sel::new(
                    "{type: xpath, expression: \"//a[0]\"}".into(),
                    SelKind::Other,
                )
            }
            Some(10) => {
                selector = Sel::new("{type: css, expression: \"div >\"}".into(), SelKind::Other)
            }
            Some(15) => {
                selector = Sel::new("{type: jq, expression: \".a\"}".into(), SelKind::Other)
            }
            Some(16) => selector = Sel::new("{type: line, from: 5, to: 2}".into(), SelKind::Other),
            Some(19) => selector = Sel::new("{expression: \"x\"}".into(), SelKind::Other),
            Some(20) => {
                selector = Sel::new(
                    "{type: contains, expression: \"x\", flags: i}".into(),
                    SelKind::Other,
                )
            }
            Some(21) => {
                selector = Sel::new(
                    "{type: xpath, expression: \"//a/@id\"}".into(),
                    SelKind::XPathValue,
                )
            }
            _ => {}
        }
        fields.push(format!("select: {}", selector.yaml));
        if rng.chance(15) {
            fields.push(format!(
                "context: {{before: {}, after: {}}}",
                rng.range(0, 3),
                rng.range(0, 3)
            ));
        }
        if rng.chance(50) {
            fields.push(format!("output: {{key: {}}}", yq(&single_line(rng))));
        }
        let extract = match broken {
            Some(17) => Some("html".to_string()),
            Some(21) => Some("inner_html".to_string()),
            _ => match selector.kind {
                SelKind::XPathElement | SelKind::Css if rng.chance(40) => Some(
                    rng.pick(&[
                        "text",
                        "inner_html",
                        "outer_html",
                        "attr:id",
                        "attr:name",
                        "attr:p:id",
                        "attr:xml:lang",
                        "attr:missing",
                    ])
                    .to_string(),
                ),
                SelKind::Css if rng.chance(10) => Some("attr:href".to_string()),
                SelKind::XPathValue if rng.chance(10) => Some("text".to_string()),
                SelKind::Other if rng.chance(5) => Some("inner_html".to_string()),
                _ => None,
            },
        };
        if let Some(extract) = extract {
            fields.push(format!("extract: {}", yq(&extract)));
        }
        let mut transforms = Vec::new();
        if rng.chance(55) {
            for _ in 0..rng.range(1, 6) {
                transforms.push(gen_transform(rng, &vocab));
            }
        }
        match broken {
            Some(6) => transforms.push("frobnicate".to_string()),
            Some(18) => transforms.push("{truncate_chars: \"x\"}".to_string()),
            Some(12) if transforms.is_empty() => {
                transforms.push("{regex_replace: {from: \"[\", to: \"\"}}".to_string())
            }
            _ => {}
        }
        if !transforms.is_empty() {
            fields.push(format!("transforms: [{}]", transforms.join(", ")));
        }
        let mut limits = Vec::new();
        if rng.chance(20) || broken == Some(12) {
            if broken == Some(12) {
                limits.push("max_items: 0".to_string());
            } else if rng.chance(60) {
                limits.push(format!("max_items: {}", rng.range(1, 6)));
            }
            if rng.chance(50) {
                limits.push(format!("max_item_chars: {}", rng.range(1, 300)));
            }
        }
        if !limits.is_empty() {
            fields.push(format!("limits: {{{}}}", limits.join(", ")));
        }
        yaml.push_str(&format!("  - {{{}}}\n", fields.join(", ")));
    }
    // Invalidity 5 needs a second rule to duplicate the first id.
    let valid = match invalidity {
        None => true,
        Some(5) if rule_count == 1 => true,
        Some(5) if invalid_rule == 0 => true,
        _ => false,
    };
    TemplateText { yaml, valid }
}

// ------------------------------------------------------------------ cases

#[derive(Clone, Debug, Default)]
struct Overrides {
    format: Option<String>,
    encoding: Option<String>,
    max_block_chars: Option<u64>,
    max_matches: Option<u64>,
    max_output_size: Option<String>,
}

#[derive(Clone, Debug)]
struct Case {
    template: String,
    template_valid: bool,
    sources: Vec<(String, Vec<u8>)>,
    cli: Overrides,
}

const ENCODINGS: &[&str] = &[
    "utf-8",
    "UTF8",
    "utf-8-sig",
    "utf-16le",
    "utf-16",
    "utf-16be",
    "windows-1251",
    "cp1251",
];

fn gen_case(rng: &mut Rng) -> Case {
    // Some cases are mostly long runs: items larger than their blocks.
    rng.1 = if rng.chance(15) { 45 } else { 10 };
    let encoding = if rng.chance(8) {
        Some(rng.pick(ENCODINGS).to_string())
    } else {
        None
    };
    let effective_encoding = encoding
        .clone()
        .unwrap_or_else(|| "utf-8".to_string())
        .to_ascii_lowercase();
    let mut sources = Vec::new();
    for index in 0..rng.range(1, 5) {
        sources.push(gen_source(rng, index, &effective_encoding));
    }
    let mut template = gen_template(rng, &sources);
    if let Some(encoding) = &encoding {
        // `input:` may already be present; a second `input:` key is a
        // duplicate key, so splice the encoding into the template text.
        if let Some(at) = template.yaml.find("input: {") {
            template.yaml.insert_str(
                at + "input: {".len(),
                &format!("encoding: {}, ", yq(encoding)),
            );
        } else {
            template.yaml.insert_str(
                "version: 1\n".len().min(template.yaml.len()),
                &format!("input: {{encoding: {}}}\n", yq(encoding)),
            );
        }
    }
    let mut cli = Overrides::default();
    if rng.chance(8) {
        cli.format = Some(any_alias(rng).to_string());
    }
    if rng.chance(12) {
        cli.max_block_chars = Some(rng.range(200, 5000) as u64);
    }
    if rng.chance(10) {
        cli.max_matches = Some(rng.range(1, 5) as u64);
    }
    if rng.chance(3) {
        cli.max_output_size = Some(rng.pick(&["50k", "2m", "100000", "8K"]).to_string());
    }
    Case {
        template: template.yaml,
        template_valid: template.valid,
        sources: sources.into_iter().map(|s| (s.name, s.bytes)).collect(),
        cli,
    }
}

impl Case {
    fn write(&self, dir: &Path) {
        let _ = std::fs::remove_dir_all(dir);
        std::fs::create_dir_all(dir.join("sources")).unwrap();
        std::fs::write(dir.join("tpl.yml"), &self.template).unwrap();
        for (name, bytes) in &self.sources {
            std::fs::write(dir.join("sources").join(name), bytes).unwrap();
        }
        let mut meta = format!("template_valid={}\n", self.template_valid);
        let fields = [
            ("format", self.cli.format.clone()),
            ("encoding", self.cli.encoding.clone()),
            (
                "max_block_chars",
                self.cli.max_block_chars.map(|v| v.to_string()),
            ),
            ("max_matches", self.cli.max_matches.map(|v| v.to_string())),
            ("max_output_size", self.cli.max_output_size.clone()),
        ];
        for (key, value) in fields {
            if let Some(value) = value {
                meta.push_str(&format!("{key}={value}\n"));
            }
        }
        std::fs::write(dir.join("case.txt"), meta).unwrap();
    }

    fn read(dir: &Path) -> Case {
        let template = std::fs::read_to_string(dir.join("tpl.yml")).expect("tpl.yml");
        let mut sources = Vec::new();
        let mut entries: Vec<_> = std::fs::read_dir(dir.join("sources"))
            .expect("sources/")
            .map(|e| e.unwrap().path())
            .collect();
        entries.sort();
        for path in entries {
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            sources.push((name, std::fs::read(&path).unwrap()));
        }
        let meta = std::fs::read_to_string(dir.join("case.txt")).unwrap_or_default();
        let mut case = Case {
            template,
            template_valid: true,
            sources,
            cli: Overrides::default(),
        };
        for line in meta.lines() {
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            let value = value.to_string();
            match key {
                "template_valid" => case.template_valid = value == "true",
                "format" => case.cli.format = Some(value),
                "encoding" => case.cli.encoding = Some(value),
                "max_block_chars" => case.cli.max_block_chars = value.parse().ok(),
                "max_matches" => case.cli.max_matches = value.parse().ok(),
                "max_output_size" => case.cli.max_output_size = Some(value),
                _ => {}
            }
        }
        case
    }

    fn describe(&self) -> String {
        let mut text = format!(
            "template (valid by construction: {}):\n{}\ncli: {:?}\n",
            self.template_valid, self.template, self.cli
        );
        for (name, bytes) in &self.sources {
            let preview: String = String::from_utf8_lossy(bytes).chars().take(1500).collect();
            text.push_str(&format!(
                "source {name} ({} bytes): {preview:?}\n",
                bytes.len()
            ));
        }
        text
    }
}

fn cli_for(dir: &Path, case: &Case, output: &str, max_block_chars: Option<u64>) -> CliOptions {
    CliOptions {
        input: dir.join("sources").to_string_lossy().into_owned(),
        template: dir.join("tpl.yml"),
        output: Some(dir.join(output)),
        format: case.cli.format.clone(),
        encoding: case.cli.encoding.clone(),
        dry_run: false,
        verbose: false,
        quiet: true,
        follow_symlinks: false,
        hash: false,
        max_file_size: None,
        max_memory: None,
        max_files: None,
        max_matches: case.cli.max_matches,
        max_output_size: case.cli.max_output_size.clone(),
        max_block_chars,
        raw_args: vec!["--input".into(), "sources".into()],
    }
}

// ------------------------------------------------------------------ runner

enum Outcome {
    Returned(Result<ExitCode, RunError>),
    Panicked(String),
    TimedOut,
}

fn panic_message(payload: &(dyn std::any::Any + Send)) -> String {
    if let Some(s) = payload.downcast_ref::<&str>() {
        s.to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "non-string panic payload".to_string()
    }
}

/// Runs the pipeline like `run::run_guarded` does: on a worker thread with
/// the production stack, so deep recursion fails here as it would in the
/// binary. A panic is caught and reported.
fn run_isolated(cli: CliOptions, timeout: Duration) -> (Outcome, Duration) {
    let (sender, receiver) = mpsc::channel();
    let started = Instant::now();
    std::thread::Builder::new()
        .name("fuzz-run".into())
        .stack_size(run::WORKER_STACK_BYTES)
        .spawn(move || {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| run::run(&cli)));
            let _ = sender.send(result.map_err(|payload| panic_message(payload.as_ref())));
        })
        .expect("cannot start the worker thread");
    let outcome = match receiver.recv_timeout(timeout) {
        Ok(Ok(result)) => Outcome::Returned(result),
        Ok(Err(message)) => Outcome::Panicked(message),
        Err(mpsc::RecvTimeoutError::Timeout) => Outcome::TimedOut,
        Err(mpsc::RecvTimeoutError::Disconnected) => Outcome::Panicked("worker thread died".into()),
    };
    (outcome, started.elapsed())
}

/// Runs `check`, turning an assertion panic into an error.
fn catching<T>(what: &str, check: impl FnOnce() -> T) -> Result<T, String> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(check))
        .map_err(|payload| format!("{what}: {}", panic_message(payload.as_ref())))
}

// ---------------------------------------------------------------- checks

#[derive(Default)]
struct Stats {
    cases: u64,
    codes: std::collections::BTreeMap<String, u64>,
    published: u64,
    blocks: u64,
    split_groups: u64,
    compared_groups: u64,
    /// `format:status` of every source file of a published document.
    files: std::collections::BTreeMap<String, u64>,
    /// Status of every block.
    statuses: std::collections::BTreeMap<String, u64>,
    slowest: Vec<(Duration, String)>,
}

impl Stats {
    fn record_time(&mut self, elapsed: Duration, label: &str) {
        self.slowest.push((elapsed, label.to_string()));
        self.slowest.sort_by_key(|entry| std::cmp::Reverse(entry.0));
        self.slowest.truncate(5);
    }
}

fn code_name(result: &Result<ExitCode, RunError>) -> String {
    match result {
        Ok(code) => format!("Ok({code:?})"),
        Err(e) => format!("Err({:?})", e.code),
    }
}

fn returned(outcome: Outcome, what: &str) -> Result<Result<ExitCode, RunError>, String> {
    match outcome {
        Outcome::Returned(result) => Ok(result),
        Outcome::Panicked(message) => Err(format!("{what}: panic: {message}")),
        Outcome::TimedOut => Err(format!("{what}: timed out")),
    }
}

/// Whether a resource-limit failure is plausible for the template: a cap on
/// the output size, no splitting, or a block cap small enough that block
/// metadata alone may not fit.
fn limit_plausible(case: &Case, dir: &Path) -> bool {
    let Ok(text) = std::fs::read_to_string(dir.join("tpl.yml")) else {
        return true;
    };
    let Ok(template) = compile_yaml_str(&text, &dir.join("tpl.yml"), None, None) else {
        return true;
    };
    let max_chars = case.cli.max_block_chars.unwrap_or(template.block.max_chars);
    case.cli.max_output_size.is_some()
        || template.max_output_chars.is_some()
        || !template.split_blocks
        || max_chars < 2500
}

const EFFECTIVE_START: &str = "\n## Effective Template\n\n```yaml\n";

fn effective_section(document: &str) -> Result<&str, String> {
    let start = document
        .find(EFFECTIVE_START)
        .ok_or("Effective Template section missing")?
        + EFFECTIVE_START.len();
    let len = document[start..]
        .find("\n```\n")
        .ok_or("unterminated Effective Template fence")?
        + 1;
    Ok(&document[start..start + len])
}

/// The Effective Template cut from the document compiles and renders back
/// to the same text. Returns the effective block cap.
fn check_effective(document: &str, dir: &Path) -> Result<u64, String> {
    let cut = effective_section(document)?;
    let decoded = unescape_anchor_text(cut);
    let template =
        compile_yaml_str(&decoded, &dir.join("effective.yml"), None, None).map_err(|e| {
            format!(
                "Effective Template does not compile: {}\n{decoded}",
                e.message
            )
        })?;
    let again = effective_template_yaml(&template);
    if again != decoded {
        return Err(format!(
            "Effective Template does not render back to itself:\n--- document\n{decoded}\n--- re-rendered\n{again}"
        ));
    }
    Ok(template.block.max_chars)
}

/// The data items of a rendered block, exactly as published (still
/// escaped): `- ` starts an item, four spaces continue it; the first other
/// line ends the data.
fn exact_items(block: &support::RenderedBlock) -> Result<Vec<String>, String> {
    let data = block
        .text
        .split_once("\n### Data\n\n")
        .ok_or_else(|| format!("block {}: data section missing", block.id))?
        .1;
    let mut items: Vec<String> = Vec::new();
    for line in data.split('\n') {
        if let Some(value) = line.strip_prefix("- ") {
            items.push(value.to_string());
        } else if let Some(value) = line.strip_prefix("    ") {
            let item = items
                .last_mut()
                .ok_or_else(|| format!("block {}: continuation line without an item", block.id))?;
            item.push('\n');
            item.push_str(value);
        } else {
            break;
        }
    }
    Ok(items)
}

fn head_field<'a>(block: &'a support::RenderedBlock, field: &str) -> Option<&'a str> {
    let head = block.text.split_once("\n### Data\n\n")?.0;
    head.split('\n').find_map(|line| line.strip_prefix(field))
}

/// The text after the data: `### Errors` and `### Warnings`. Item lines
/// start with `- ` or four spaces, so these headings are never data.
fn block_tail(block: &support::RenderedBlock) -> String {
    let data = block
        .text
        .split_once("\n### Data\n\n")
        .map_or("", |(_, d)| d);
    let start = ["\n### Errors\n", "\n### Warnings\n"]
        .iter()
        .filter_map(|heading| data.find(heading))
        .min();
    match start {
        Some(start) => data[start..].to_string(),
        None => String::new(),
    }
}

/// One rule result: a top-level block and its continuation parts.
#[derive(Debug, PartialEq)]
struct Group {
    id: String,
    source: String,
    rule: String,
    status: String,
    tail: String,
    items: Vec<String>,
}

/// Rule results of a document, with split items joined back and decoded.
fn groups(blocks: &[support::RenderedBlock]) -> Result<Vec<Group>, String> {
    let mut groups: Vec<Group> = Vec::new();
    // `(item continuation, text)` of the current group.
    let mut pieces: Vec<(Option<usize>, String)> = Vec::new();
    let finish = |groups: &mut Vec<Group>, pieces: &mut Vec<(Option<usize>, String)>| {
        if let Some(group) = groups.last_mut() {
            group.items = pieces
                .drain(..)
                .map(|(_, text)| unescape_anchor_text(&text).into_owned())
                .collect();
        }
    };
    for block in blocks {
        let items = exact_items(block)?;
        if let Some(parent) = head_field(block, "Parent: ") {
            let group = groups
                .last()
                .ok_or_else(|| format!("part {} before any parent", block.id))?;
            if group.id != parent {
                return Err(format!(
                    "part {} follows {}, not its parent {parent}",
                    block.id, group.id
                ));
            }
            match head_field(block, "Item continuation: ") {
                Some(index) => {
                    let index: usize = index
                        .parse()
                        .map_err(|_| format!("block {}: bad Item continuation", block.id))?;
                    if items.len() != 1 {
                        return Err(format!(
                            "continuation part {} has {} items",
                            block.id,
                            items.len()
                        ));
                    }
                    match pieces.last_mut() {
                        Some((Some(last), text)) if *last == index => text.push_str(&items[0]),
                        _ => pieces.push((Some(index), items[0].clone())),
                    }
                }
                None => pieces.extend(items.into_iter().map(|item| (None, item))),
            }
        } else {
            finish(&mut groups, &mut pieces);
            groups.push(Group {
                id: block.id.clone(),
                source: head_field(block, "Source: ")
                    .unwrap_or_default()
                    .to_string(),
                rule: head_field(block, "Rule: ").unwrap_or_default().to_string(),
                status: head_field(block, "Status: ")
                    .unwrap_or_default()
                    .to_string(),
                tail: block_tail(block),
                items: Vec::new(),
            });
            pieces.extend(items.into_iter().map(|item| (None, item)));
        }
    }
    finish(&mut groups, &mut pieces);
    Ok(groups)
}

/// Structural invariants of a published document.
fn check_document(
    document: &str,
    code: ExitCode,
    dir: &Path,
    stats: &mut Stats,
) -> Result<Vec<support::RenderedBlock>, String> {
    if !document.ends_with("\nEnd of any_txt_parser output\n") {
        return Err("document does not end with the end marker".into());
    }
    if !document.contains("\nTriangle: [0,1,0]\n") {
        return Err("Triangle marker missing".into());
    }
    let anchor =
        regex::Regex::new(r"^<!-- BLOCK: (B[0-9]{4,}(?:-[0-9]{2,})?) (START|END) -->$").unwrap();
    let mut anchor_lines = 0usize;
    for (number, line) in document.split('\n').enumerate() {
        if line.contains(ANCHOR_MARK) {
            if !anchor.is_match(line) {
                return Err(format!(
                    "anchor mark outside an anchor line {}: {line:?}",
                    number + 1
                ));
            }
            anchor_lines += 1;
        }
    }
    if document.matches(ANCHOR_MARK).count() != anchor_lines {
        return Err("anchor mark repeated within a line".into());
    }
    // Applied Template writes each template value on a line of its own (by
    // design), so a value such as `## Block Index` reads like a heading
    // there; no other text may.
    let outside_applied = match (
        document.find("\n## Applied Template\n"),
        document.find(EFFECTIVE_START),
    ) {
        (Some(start), Some(end)) if start < end => {
            format!("{}{}", &document[..start], &document[end..])
        }
        _ => return Err("Applied or Effective Template section missing".into()),
    };
    for heading in ["## Block Index", "### Block Anchors", "## Data Blocks"] {
        let count = outside_applied
            .split('\n')
            .filter(|line| *line == heading)
            .count();
        if count != 1 {
            return Err(format!("{count} lines {heading:?}"));
        }
    }
    let blocks = catching("support::blocks", || support::blocks(document))?;
    if anchor_lines != 2 * blocks.len() {
        return Err(format!(
            "{anchor_lines} anchor lines for {} blocks",
            blocks.len()
        ));
    }
    catching("support::assert_index_lines", || {
        support::assert_index_lines(document)
    })?;
    catching("support::assert_total_characters", || {
        support::assert_total_characters(document)
    })?;
    let max_block = check_effective(document, dir)?;
    let rule_results = groups(&blocks)?;
    for block in &blocks {
        let chars = block.text.chars().count() as u64;
        if chars > max_block {
            return Err(format!(
                "block {} has {chars} characters, cap {max_block}",
                block.id
            ));
        }
        let mut rendered = exact_items(block)?.len();
        // A split parent holds no data and counts the items of its parts,
        // fragments of one item as one.
        if head_field(block, "Parent block: continuation parts follow").is_some() {
            if rendered != 0 {
                return Err(format!("split parent {} renders data", block.id));
            }
            rendered = rule_results
                .iter()
                .find(|group| group.id == block.id)
                .map_or(0, |group| group.items.len());
        }
        let declared: usize = head_field(block, "Items: ")
            .and_then(|n| n.parse().ok())
            .ok_or_else(|| format!("block {}: Items field missing", block.id))?;
        if declared != rendered {
            return Err(format!(
                "block {}: Items: {declared}, but {rendered} items rendered",
                block.id
            ));
        }
    }
    let run_status = document
        .lines()
        .find_map(|line| line.strip_prefix("Run status: "))
        .ok_or("Run status missing")?;
    let expected = match code {
        ExitCode::Success => "success",
        ExitCode::PartialSuccess => "partial_success",
        _ => "failure",
    };
    if run_status != expected {
        return Err(format!("Run status {run_status} with exit code {code:?}"));
    }
    stats.published += 1;
    stats.blocks += blocks.len() as u64;
    if let Some(files) = document.split_once("\n## Files\n\n") {
        for row in files
            .1
            .lines()
            .skip(2)
            .take_while(|line| line.starts_with('|'))
        {
            let cells: Vec<&str> = row.split('|').map(str::trim).collect();
            if cells.len() == 9 {
                *stats
                    .files
                    .entry(format!("{}:{}", cells[4], cells[5]))
                    .or_default() += 1;
            }
        }
    }
    for block in &blocks {
        let status = head_field(block, "Status: ").unwrap_or("?");
        *stats.statuses.entry(status.to_string()).or_default() += 1;
    }
    Ok(blocks)
}

fn check_case(
    case: &Case,
    dir: &Path,
    label: &str,
    timeout: Duration,
    stats: &mut Stats,
) -> Result<(), String> {
    stats.cases += 1;
    let cli = cli_for(dir, case, "out.md", case.cli.max_block_chars);
    let (outcome, elapsed) = run_isolated(cli.clone(), timeout);
    stats.record_time(elapsed, label);
    let first = returned(outcome, "run")?;
    *stats.codes.entry(code_name(&first)).or_default() += 1;
    let output = dir.join("out.md");

    let published = match &first {
        Err(RunError { code, message }) => {
            if output.exists() {
                return Err(format!("failed run ({code:?}) published a document"));
            }
            match code {
                ExitCode::InvalidTemplate if case.template_valid => {
                    return Err(format!("a valid template was rejected: {message}"));
                }
                ExitCode::InvalidTemplate => {}
                ExitCode::ResourceLimit if limit_plausible(case, dir) => {}
                other => return Err(format!("implausible failure {other:?}: {message}")),
            }
            None
        }
        Ok(code) => {
            if !case.template_valid {
                return Err(format!("an invalid template was accepted (exit {code:?})"));
            }
            if !matches!(
                code,
                ExitCode::Success | ExitCode::PartialSuccess | ExitCode::InvalidInputOrCli
            ) {
                return Err(format!("implausible exit code {code:?}"));
            }
            let document = std::fs::read_to_string(&output)
                .map_err(|e| format!("cannot read the output: {e}"))?;
            let blocks = check_document(&document, *code, dir, stats)?;
            Some((*code, document, blocks))
        }
    };

    // Determinism: the same case again gives the same document.
    let (outcome, elapsed) = run_isolated(cli, timeout);
    stats.record_time(elapsed, label);
    let second = returned(outcome, "second run")?;
    if code_name(&second) != code_name(&first) {
        return Err(format!(
            "second run: {} after {}",
            code_name(&second),
            code_name(&first)
        ));
    }
    if let (Some((_, document, _)), Ok(_)) = (&published, &second) {
        let again = std::fs::read_to_string(&output).map_err(|e| e.to_string())?;
        let (a, b) = (
            support::normalize_volatile(document),
            support::normalize_volatile(&again),
        );
        if a != b {
            let line = a
                .lines()
                .zip(b.lines())
                .position(|(x, y)| x != y)
                .unwrap_or(0);
            return Err(format!(
                "second run differs at line {}: {:?} vs {:?}",
                line + 1,
                a.lines().nth(line),
                b.lines().nth(line)
            ));
        }
    }

    // Split invariance: nothing splits with a huge block cap.
    if matches!(
        &first,
        Err(RunError {
            code: ExitCode::InvalidTemplate,
            ..
        })
    ) {
        return Ok(());
    }
    let big_cli = cli_for(dir, case, "out_big.md", Some(HUGE_BLOCK));
    let (outcome, elapsed) = run_isolated(big_cli, timeout);
    stats.record_time(elapsed, label);
    let big = returned(outcome, "huge-block run")?;
    let big_document = match &big {
        Ok(code) => {
            let document =
                std::fs::read_to_string(dir.join("out_big.md")).map_err(|e| e.to_string())?;
            let blocks = check_document(&document, *code, dir, stats)?;
            Some((*code, blocks))
        }
        Err(RunError {
            code: ExitCode::ResourceLimit,
            message,
        }) => {
            let output_capped = case.cli.max_output_size.is_some()
                || compile_yaml_str(&case.template, &dir.join("tpl.yml"), None, None)
                    .is_ok_and(|t| t.max_output_chars.is_some());
            if !output_capped {
                return Err(format!("huge-block run hit a resource limit: {message}"));
            }
            None
        }
        Err(RunError { code, message }) => {
            return Err(format!("huge-block run failed with {code:?}: {message}"));
        }
    };
    if let (Some((code, _, blocks)), Some((big_code, big_blocks))) = (&published, &big_document) {
        if code != big_code {
            return Err(format!(
                "exit {code:?}, but {big_code:?} with a huge block cap"
            ));
        }
        let split = groups(blocks)?;
        let whole = groups(big_blocks)?;
        if big_blocks
            .iter()
            .any(|b| head_field(b, "Parent: ").is_some())
        {
            return Err("blocks split under a huge block cap".into());
        }
        if split.len() != whole.len() {
            return Err(format!(
                "{} rule results, but {} with a huge block cap",
                split.len(),
                whole.len()
            ));
        }
        for (a, b) in split.iter().zip(&whole) {
            if a != b {
                return Err(format!(
                    "rule result {} differs from the unsplit run:\n split: {:?}\n whole: {:?}",
                    a.id, a, b
                ));
            }
        }
        stats.compared_groups += split.len() as u64;
        stats.split_groups += blocks
            .iter()
            .filter(|b| {
                head_field(b, "Parent: ").is_none() && head_field(b, "Split: ") == Some("yes")
            })
            .count() as u64;
    }
    Ok(())
}

// ------------------------------------------------------------------ tests

fn env_u64(name: &str) -> Option<u64> {
    let value = std::env::var(name).ok()?;
    let value = value.trim();
    match value.strip_prefix("0x") {
        Some(hex) => u64::from_str_radix(hex, 16).ok(),
        None => value.parse().ok(),
    }
}

/// Where case directories go: `ATP_FUZZ_WORK_DIR`, else the OS temp dir.
fn work_dir() -> PathBuf {
    std::env::var_os("ATP_FUZZ_WORK_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
}

fn timeout() -> Duration {
    Duration::from_secs(env_u64("ATP_FUZZ_TIMEOUT_SECS").unwrap_or(120))
}

#[test]
fn fuzz_pipeline() {
    if std::env::var_os("ATP_FUZZ_REPLAY").is_some() {
        return;
    }
    let seed = env_u64("ATP_FUZZ_SEED").unwrap_or(DEFAULT_SEED);
    let iterations = env_u64("ATP_FUZZ_ITERS").unwrap_or(DEFAULT_ITERS);
    let keep_going = std::env::var("ATP_FUZZ_KEEP_GOING").is_ok_and(|v| v == "1");
    let max_failures = env_u64("ATP_FUZZ_MAX_FAILURES").unwrap_or(30);
    let keep_cases = std::env::var("ATP_FUZZ_KEEP").is_ok_and(|v| v == "1");
    let first = env_u64("ATP_FUZZ_FIRST").unwrap_or(0);
    let root = work_dir().join(format!("atp_fuzz_{}_{seed:x}", std::process::id()));
    let mut stats = Stats::default();
    let mut failures = Vec::new();
    let started = Instant::now();
    for iteration in first..first + iterations {
        let mut rng = Rng::new(case_seed(seed, iteration));
        let case = gen_case(&mut rng);
        let dir = root.join(format!("case_{iteration}"));
        case.write(&dir);
        let label = format!("seed {seed:#x} iteration {iteration}");
        match check_case(&case, &dir, &label, timeout(), &mut stats) {
            Ok(()) => {
                if !keep_cases {
                    let _ = std::fs::remove_dir_all(&dir);
                }
            }
            Err(reason) => {
                std::fs::write(dir.join("failure.txt"), &reason).unwrap();
                eprintln!(
                    "\n=== FUZZ FAILURE: {label}\nreason: {reason}\nreplay: ATP_FUZZ_REPLAY={}\n{}",
                    dir.display(),
                    case.describe()
                );
                failures.push(format!("{label}: {}", reason.lines().next().unwrap_or("")));
                if !keep_going || failures.len() as u64 >= max_failures {
                    break;
                }
            }
        }
    }
    eprintln!(
        "fuzz: seed {seed:#x}, {} cases in {:.1?}; exit codes {:?}; {} documents, {} blocks, {} split groups, {} rule results compared",
        stats.cases,
        started.elapsed(),
        stats.codes,
        stats.published,
        stats.blocks,
        stats.split_groups,
        stats.compared_groups
    );
    eprintln!("  files by format:status: {:?}", stats.files);
    eprintln!("  blocks by status: {:?}", stats.statuses);
    for (elapsed, label) in &stats.slowest {
        eprintln!("  slowest run: {elapsed:.2?} ({label})");
    }
    if failures.is_empty() {
        if !keep_cases {
            let _ = std::fs::remove_dir_all(&root);
        }
    } else {
        panic!(
            "{} fuzz failure(s):\n{}",
            failures.len(),
            failures.join("\n")
        );
    }
}

/// The generators mostly produce sources their adapter parses, so most cases
/// reach extraction rather than stopping at a parse error.
#[test]
fn fuzz_generators_mostly_produce_valid_sources() {
    use any_txt_parser::adapters::adapter_for;
    use any_txt_parser::template::Format;
    let mut rng = Rng::new(env_u64("ATP_FUZZ_SEED").unwrap_or(DEFAULT_SEED));
    let mut parsed = std::collections::BTreeMap::<String, (u32, u32)>::new();
    let mut samples = Vec::new();
    for index in 0..1400 {
        let source = gen_source(&mut rng, index, "utf-8");
        let format = match source.format {
            Fmt::Json => Format::Json,
            Fmt::Yaml => Format::Yaml,
            Fmt::Toml => Format::Toml,
            Fmt::Xml => Format::Xml,
            Fmt::Html => Format::Html,
            Fmt::Markdown => Format::Markdown,
            Fmt::Text => Format::Text,
        };
        let entry = parsed.entry(format.name().to_string()).or_default();
        entry.1 += 1;
        let Ok(text) = std::str::from_utf8(&source.bytes) else {
            continue;
        };
        let text = text.strip_prefix('\u{feff}').unwrap_or(text);
        match adapter_for(&format).parse(text) {
            Ok(_) => entry.0 += 1,
            Err(error) => {
                if std::env::var_os("ATP_FUZZ_SHOW_PARSE_ERRORS").is_some() {
                    samples.push(format!("{}: {error}\n{text:?}", format.name()));
                }
            }
        }
    }
    for sample in samples.iter().take(40) {
        eprintln!("{}\n", sample.chars().take(1200).collect::<String>());
    }
    eprintln!("parsed / generated per format: {parsed:?}");
    for (format, (ok, total)) in &parsed {
        assert!(
            *ok * 100 >= *total * 60,
            "{format}: only {ok} of {total} generated sources parse"
        );
    }
}

#[test]
fn fuzz_replay() {
    let Some(dir) = std::env::var_os("ATP_FUZZ_REPLAY") else {
        return;
    };
    let saved = PathBuf::from(dir);
    let case = Case::read(&saved);
    let work = work_dir().join(format!("atp_fuzz_replay_{}", std::process::id()));
    case.write(&work);
    eprintln!("{}", case.describe());
    let mut stats = Stats::default();
    let result = check_case(&case, &work, "replay", timeout(), &mut stats);
    eprintln!(
        "replay: exit codes {:?}, work dir {}",
        stats.codes,
        work.display()
    );
    if let Err(reason) = result {
        panic!("replayed case fails: {reason}");
    }
    let _ = std::fs::remove_dir_all(&work);
}
