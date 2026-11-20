//! RENDER: one function per render type, `(input, standard) -> text`.
//! A render that does not fit the input returns `Err(reason)`; the
//! pipeline then falls through (D2) and never crashes.

use crate::detect::level_of;
use crate::json::{J, cmp_num, quote};
use crate::ops::{Effect, field, format_value};
use crate::standard::{Mode, Render, Standard, ValueFormat};
use regex::Regex;
use std::borrow::Cow;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt::Write as _;
use std::sync::OnceLock;

pub struct Ctx<'a> {
    pub std: &'a Standard,
    pub tool: &'a str,
    pub server: &'a str,
    /// Textual payload: the decoded string when the payload is a JSON string.
    pub text: &'a str,
    /// The payload as received (JSON text for structured payloads).
    pub raw: &'a str,
    pub json: Option<&'a J>,
    /// The node at `root` (the whole document without a root).
    pub node: Option<&'a J>,
    /// The primary list after PRE operations.
    pub list: Option<&'a [Cow<'a, J>]>,
    /// The key the list sits under in its envelope, if any.
    pub list_key: Option<&'a str>,
    /// Envelope fields around the list (cursors, totals), always kept.
    pub rest: &'a [(String, &'a J)],
    pub args: &'a J,
    pub keys: Option<&'a [String]>,
}

pub struct Rendered {
    pub text: String,
    pub eff: Effect,
}

pub type RResult = Result<Rendered, String>;

pub fn render(c: &Ctx) -> RResult {
    match c.std.render {
        Render::Passthrough => Ok(Rendered {
            text: c.text.to_string(),
            eff: Effect {
                // A JSON string payload is decoded, so its text is not the
                // document; any other payload is passed on as received.
                shown: match c.json {
                    Some(J::Str(_)) => Vec::new(),
                    _ => vec![String::new()],
                },
                ..Effect::default()
            },
        }),
        Render::Json => json(c),
        Render::Expression => expression(c),
        Render::MatrixTable => matrix(c),
        Render::ObjectsTable => objects(c),
        Render::DictTable => dict(c),
        Render::Template => crate::tpl::render(c),
        Render::Log => log(c),
        Render::NumStats => num_stats(c),
        Render::Tree => tree(c),
        Render::ImageMeta => crate::image::render(c),
    }
}

// ---------------------------------------------------------------- helpers

fn cell(c: &Ctx, key: &str, v: &J, eff: &mut Effect, cut: &mut usize) -> String {
    let mut s = match (v, c.std.format_for(key)) {
        (J::Num(_), Some(f)) => {
            let (s, lossy) = format_value(v, f);
            if lossy {
                eff.lossy = true;
            }
            s
        }
        _ => v.text(),
    };
    if let Some(m) = c.std.raw.max_cell_chars {
        if s.chars().count() > m {
            s = s.chars().take(m).collect::<String>() + "…";
            eff.lossy = true;
            *cut += 1;
        }
    }
    s
}

/// Strings a reader would take for another type: `true`, `null`, `42`,
/// `007`, `-1.5e3` (leading zeros included).
fn looks_literal(s: &str) -> bool {
    if s == "true" || s == "false" || s == "null" {
        return true;
    }
    let body = s.strip_prefix('-').unwrap_or(s);
    let mut parts = body.splitn(2, ['e', 'E']);
    let mantissa = parts.next().unwrap_or("");
    let exp_ok = parts.next().is_none_or(|e| {
        let e = e.strip_prefix(['+', '-']).unwrap_or(e);
        !e.is_empty() && e.bytes().all(|b| b.is_ascii_digit())
    });
    let (int, frac) = mantissa.split_once('.').unwrap_or((mantissa, "0"));
    exp_ok
        && !int.is_empty()
        && !frac.is_empty()
        && int.bytes().all(|b| b.is_ascii_digit())
        && frac.bytes().all(|b| b.is_ascii_digit())
}

/// A short decimal for an approximate value: at most 6 decimals.
/// Very large or very small values use exponent notation: printing all
/// digits of an approximation would look exact.
fn approx(x: f64) -> String {
    let a = x.abs();
    if a >= 1e15 || (a != 0.0 && a < 1e-6) {
        return format!("{x:e}");
    }
    let s = format!("{x:.6}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    s.to_string()
}

/// A value on a `key: value` line: multi-line strings and strings that
/// would read as another type (`null`, `42`, `[1,2]`, `{...}`, `"x"`) are
/// quoted, so `total: null` and `total: "null"` stay apart.
fn line_value(v: &J, s: String) -> String {
    match v {
        J::Str(raw)
            if raw.contains(['\n', '\r'])
                || raw.is_empty()
                || raw != raw.trim()
                || looks_literal(raw)
                || raw.starts_with(['[', '{', '"']) =>
        {
            quote(raw)
        }
        _ => s,
    }
}

fn context_lines(c: &Ctx, out: &mut String, eff: &mut Effect, cut: &mut usize) {
    for (k, v) in c.rest {
        let before = *cut;
        let s = cell(c, k, v, eff, cut);
        let exact = *cut == before && !(matches!(v, J::Num(_)) && c.std.format_for(k).is_some());
        if exact {
            eff.shown.push(k.clone());
        }
        let _ = writeln!(out, "{}: {}", k, line_value(v, s));
    }
}

/// Why the built-in table would mislead: a nested array or object prints
/// like a string holding the same JSON, and a repeated key shows only one
/// of its values. `None` when every cell reads back as its value.
fn table_ambiguity(list: &[Cow<J>], builtin: bool) -> Option<&'static str> {
    for x in list {
        let J::Obj(m) = x.as_ref() else { continue };
        let mut seen: HashSet<&str> = HashSet::new();
        for (k, v) in m {
            if !seen.insert(k.as_str()) {
                return Some("an item repeats a key");
            }
            if builtin && matches!(v, J::Arr(_) | J::Obj(_)) {
                return Some("a cell holds a nested array or object");
            }
        }
    }
    None
}

fn cut_note(eff: &mut Effect, cut: usize, c: &Ctx) {
    if cut > 0 {
        let m = c.std.raw.max_cell_chars.unwrap_or(0);
        eff.notes.push(format!("{cut} cells cut to {m} chars"));
    }
}

// ---------------------------------------------------------------- json

fn json(c: &Ctx) -> RResult {
    let mut eff = Effect::default();
    let text = match (c.list, c.list_key) {
        (Some(list), Some(key)) => {
            let mut obj: Vec<(String, J)> = c
                .rest
                .iter()
                .map(|(k, v)| (k.clone(), (*v).clone()))
                .collect();
            obj.push((
                key.to_string(),
                J::Arr(list.iter().map(|x| x.as_ref().clone()).collect()),
            ));
            J::Obj(obj).to_json()
        }
        (Some(list), None) => {
            let mut s = String::from("[");
            for (i, x) in list.iter().enumerate() {
                if i > 0 {
                    s.push(',');
                }
                x.write_json(&mut s);
            }
            s.push(']');
            s
        }
        (None, _) => {
            let node = c.node.ok_or("input is not JSON")?;
            if c.json.is_some_and(|j| std::ptr::eq(j, node)) {
                eff.shown.push(String::new());
            }
            node.to_json()
        }
    };
    if c.list_key.is_some() {
        eff.shown.extend(c.rest.iter().map(|(k, _)| k.clone()));
    }
    eff.lossy = false;
    Ok(Rendered { text, eff })
}

// ---------------------------------------------------------------- expression

#[derive(Debug, Clone)]
pub enum Seg {
    Lit(String),
    Ref {
        path: String,
        fmt: Option<ValueFormat>,
    },
}

/// `{name}` / `{name.path}` / `{name:thousands}`; `{{` and `}}` are braces.
/// Names are data lookups only: no calls, no attributes, no eval (I9).
pub fn parse_format(f: &str) -> Result<Vec<Seg>, String> {
    let mut segs = Vec::new();
    let mut lit = String::new();
    let mut chars = f.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '{' if chars.peek() == Some(&'{') => {
                chars.next();
                lit.push('{');
            }
            '}' if chars.peek() == Some(&'}') => {
                chars.next();
                lit.push('}');
            }
            '{' => {
                let mut inner = String::new();
                loop {
                    match chars.next() {
                        Some('}') => break,
                        Some(x) => inner.push(x),
                        None => return Err(format!("unclosed '{{' in format '{f}'")),
                    }
                }
                let (path, fmt) = match inner.split_once(':') {
                    Some((p, s)) => (p.trim().to_string(), Some(ValueFormat::parse(s)?)),
                    None => (inner.trim().to_string(), None),
                };
                let ok = !path.is_empty()
                    && path
                        .chars()
                        .all(|x| x.is_alphanumeric() || matches!(x, '_' | '-' | '.' | '[' | ']'));
                if !ok {
                    return Err(format!("bad placeholder '{{{inner}}}' in format"));
                }
                if !lit.is_empty() {
                    segs.push(Seg::Lit(std::mem::take(&mut lit)));
                }
                segs.push(Seg::Ref { path, fmt });
            }
            '}' => return Err(format!("single '}}' in format '{f}' (write '}}}}')")),
            x => lit.push(x),
        }
    }
    if !lit.is_empty() {
        segs.push(Seg::Lit(lit));
    }
    Ok(segs)
}

fn expression(c: &Ctx) -> RResult {
    let f = c
        .std
        .raw
        .format
        .as_deref()
        .ok_or("expression needs format")?;
    let segs = parse_format(f)?;
    let owned;
    let result: &J = match c.json {
        Some(j) => j,
        None => {
            owned = J::Str(c.text.trim().to_string());
            &owned
        }
    };
    let mut eff = Effect::default();
    let mut out = String::new();
    for s in &segs {
        match s {
            Seg::Lit(l) => out.push_str(l),
            Seg::Ref { path, fmt } => {
                let split = path.find(['.', '[']).unwrap_or(path.len());
                let (head, tail) = path.split_at(split);
                let base = if head == "result" {
                    Some(result)
                } else {
                    c.args.get(head)
                };
                let v = match base {
                    Some(b) if tail.is_empty() => Some(b),
                    Some(b) => b.path(tail),
                    None => None,
                }
                .ok_or_else(|| format!("placeholder {{{path}}} has no value"))?;
                match fmt {
                    Some(f) => {
                        let (s, lossy) = format_value(v, *f);
                        eff.lossy |= lossy;
                        out.push_str(&s);
                    }
                    None => {
                        if head == "result" && c.json.is_some() {
                            eff.shown.push(tail.trim_start_matches('.').to_string());
                        }
                        out.push_str(&v.text())
                    }
                }
            }
        }
    }
    Ok(Rendered { text: out, eff })
}

// ---------------------------------------------------------------- tables

fn union_keys(list: &[Cow<J>]) -> Vec<String> {
    let mut seen: HashSet<&str> = HashSet::new();
    let mut cols = Vec::new();
    for x in list {
        if let J::Obj(m) = x.as_ref() {
            for (k, _) in m {
                if seen.insert(k.as_str()) {
                    cols.push(k.clone());
                }
            }
        }
    }
    cols
}

fn toon_needs_quote(s: &str) -> bool {
    s.is_empty()
        || s.starts_with(' ')
        || s.ends_with(' ')
        || s.contains([',', '"', '\n', '\r', '\t', '\\'])
}

fn toon_cell(v: &J, s: String) -> String {
    let as_text = matches!(v, J::Str(_)) && looks_literal(&s);
    if toon_needs_quote(&s) || as_text {
        quote(&s)
    } else {
        s
    }
}

fn toon_name(s: &str) -> String {
    if s.is_empty() || s.contains([',', '"', ' ', ':', '{', '}', '[', ']', '\n']) {
        quote(s)
    } else {
        s.to_string()
    }
}

fn md_escape(s: &str) -> String {
    s.replace('|', "\\|")
        .replace("\r\n", "<br>")
        .replace('\n', "<br>")
}

fn objects(c: &Ctx) -> RResult {
    let list = c.list.ok_or("input has no list of objects")?;
    if !list.iter().all(|x| matches!(x.as_ref(), J::Obj(_))) {
        return Err("items are not all objects".into());
    }
    if let Some(why) = table_ambiguity(list, c.std.origin == "builtin") {
        return Err(why.into());
    }
    let mut eff = Effect::default();
    let mut cut = 0usize;
    let explicit = c.std.raw.columns.clone();
    let cols = explicit.clone().unwrap_or_else(|| union_keys(list));
    if explicit.is_some() {
        let dropped = list.iter().any(|x| {
            x.as_obj()
                .map(|m| m.iter().any(|(k, _)| !cols.iter().any(|c| c == k)))
                .unwrap_or(false)
        });
        if dropped {
            eff.lossy = true;
        }
    }
    let cap = c.std.raw.rows.or(c.std.raw.max_rows).unwrap_or(usize::MAX);
    let shown = &list[..list.len().min(cap)];
    let mut out = String::new();
    context_lines(c, &mut out, &mut eff, &mut cut);
    match c.std.mode {
        Mode::Markdown => {
            out.push('|');
            for k in &cols {
                let _ = write!(out, " {} |", md_escape(k));
            }
            out.push_str("\n|");
            for _ in &cols {
                out.push_str("---|");
            }
            for x in shown {
                out.push_str("\n|");
                for k in &cols {
                    let s = match field(x, k) {
                        Some(J::Str(s)) if s.is_empty() => "\"\"".to_string(),
                        Some(v) => md_escape(&cell(c, k, v, &mut eff, &mut cut)),
                        None => String::new(),
                    };
                    let _ = write!(out, " {s} |");
                }
            }
        }
        _ => {
            let names: Vec<String> = cols.iter().map(|k| toon_name(k)).collect();
            let _ = write!(
                out,
                "{}[{}]{{{}}}:",
                c.list_key.map(toon_name).unwrap_or_default(),
                shown.len(),
                names.join(",")
            );
            for x in shown {
                out.push_str("\n  ");
                for (i, k) in cols.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    if let Some(v) = field(x, k) {
                        let s = cell(c, k, v, &mut eff, &mut cut);
                        out.push_str(&toon_cell(v, s));
                    }
                }
            }
        }
    }
    if list.len() > shown.len() {
        eff.lose(format!("{} more rows", list.len() - shown.len()));
    }
    cut_note(&mut eff, cut, c);
    Ok(Rendered { text: out, eff })
}

fn matrix(c: &Ctx) -> RResult {
    let rows = c.list.ok_or("input is not a 2D list")?;
    let mut grid: Vec<&Vec<J>> = Vec::with_capacity(rows.len());
    for r in rows {
        match r.as_ref() {
            J::Arr(a) if a.iter().all(J::is_scalar) => grid.push(a),
            _ => return Err("rows are not lists of scalars".into()),
        }
    }
    let nr = grid.len();
    let nc = grid.iter().map(|r| r.len()).max().unwrap_or(0);
    let sr = nr.min(c.std.raw.max_rows.unwrap_or(usize::MAX));
    let sc = nc.min(c.std.raw.max_cols.unwrap_or(usize::MAX));
    let mut eff = Effect::default();
    let mut cut = 0usize;
    let mut out = String::new();
    context_lines(c, &mut out, &mut eff, &mut cut);
    match c.std.mode {
        Mode::Markdown => {
            out.push_str("| |");
            for j in 0..sc {
                let _ = write!(out, " {j} |");
            }
            out.push_str("\n|---|");
            for _ in 0..sc {
                out.push_str("---|");
            }
            for (i, r) in grid.iter().take(sr).enumerate() {
                let _ = write!(out, "\n| {i} |");
                for v in r.iter().take(sc) {
                    let _ = write!(
                        out,
                        " {} |",
                        md_escape(&cell(c, "*", v, &mut eff, &mut cut))
                    );
                }
            }
        }
        _ => {
            let _ = write!(out, "{nr}x{nc}");
            for r in grid.iter().take(sr) {
                out.push('\n');
                for (j, v) in r.iter().take(sc).enumerate() {
                    if j > 0 {
                        out.push(' ');
                    }
                    let s = cell(c, "*", v, &mut eff, &mut cut);
                    if matches!(v, J::Str(_)) && (s.contains(' ') || s.is_empty()) {
                        out.push_str(&quote(&s));
                    } else {
                        out.push_str(&s);
                    }
                }
            }
        }
    }
    if nr > sr {
        eff.lose(format!("{} more rows", nr - sr));
    }
    if nc > sc {
        eff.lose(format!("{} more columns", nc - sc));
    }
    cut_note(&mut eff, cut, c);
    Ok(Rendered { text: out, eff })
}

fn dict(c: &Ctx) -> RResult {
    let Some(J::Obj(m)) = c.node else {
        return Err("input is not a JSON object".into());
    };
    let mut eff = Effect::default();
    let mut cut = 0usize;
    let cap = c.std.raw.max_rows.unwrap_or(usize::MAX);
    let mut out = String::new();
    if c.std.mode == Mode::Markdown {
        out.push_str("| key | value |\n|---|---|");
        for (k, v) in m.iter().take(cap) {
            let s = cell(c, k, v, &mut eff, &mut cut);
            let _ = write!(out, "\n| {} | {} |", md_escape(k), md_escape(&s));
        }
    } else {
        for (i, (k, v)) in m.iter().take(cap).enumerate() {
            if i > 0 {
                out.push('\n');
            }
            let s = cell(c, k, v, &mut eff, &mut cut);
            let key = if k.contains([':', '\n']) || k.is_empty() {
                quote(k)
            } else {
                k.clone()
            };
            let _ = write!(out, "{}: {}", key, line_value(v, s));
        }
    }
    if m.len() > cap {
        eff.lose(format!("{} more keys", m.len() - cap));
    }
    cut_note(&mut eff, cut, c);
    Ok(Rendered { text: out, eff })
}

// ---------------------------------------------------------------- log

fn stamp_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"^\s*[\[(]?(?:\d{2,4}[-/.]\d{1,2}[-/.]\d{1,2}(?:[T ]\d{1,2}:\d{2}(?::\d{2})?(?:[.,]\d+)?)?(?:Z|[+-]\d{2}:?\d{2})?|\d{1,2}:\d{2}:\d{2}(?:[.,]\d+)?)[\])]?\s*",
        )
        .expect("stamp regex")
    })
}

const LEVELS: [&str; 6] = ["FATAL", "ERROR", "WARN", "INFO", "DEBUG", "TRACE"];

struct Uniq {
    order: Vec<(String, usize)>,
    index: HashMap<String, usize>,
}

impl Uniq {
    fn new() -> Uniq {
        Uniq {
            order: Vec::new(),
            index: HashMap::new(),
        }
    }
    fn add(&mut self, msg: String) {
        match self.index.get(&msg) {
            Some(&i) => self.order[i].1 += 1,
            None => {
                self.index.insert(msg.clone(), self.order.len());
                self.order.push((msg, 1));
            }
        }
    }
}

fn log(c: &Ctx) -> RResult {
    let mut counts = [0usize; 6];
    let mut errors = Uniq::new();
    let mut warns = Uniq::new();
    let mut total = 0usize;
    for line in c.text.lines() {
        if line.trim().is_empty() {
            continue;
        }
        total += 1;
        let Some(level) = level_of(line) else {
            continue;
        };
        let i = LEVELS.iter().position(|l| *l == level).unwrap_or(3);
        counts[i] += 1;
        if i <= 2 {
            let mut msg = stamp_re().replace(line, "").trim().to_string();
            if msg.chars().count() > 300 {
                msg = msg.chars().take(300).collect::<String>() + "…";
            }
            if i <= 1 {
                errors.add(msg);
            } else {
                warns.add(msg);
            }
        }
    }
    let marked: usize = counts.iter().sum();
    if marked == 0 {
        return Err("no log levels found".into());
    }
    let mut eff = Effect::default();
    let mut out = format!("{total} lines:");
    let mut first = true;
    for (i, n) in counts.iter().enumerate() {
        if *n > 0 {
            let _ = write!(out, "{} {} {}", if first { "" } else { "," }, LEVELS[i], n);
            first = false;
        }
    }
    let cap = c.std.raw.max_items.unwrap_or(10);
    for (title, u) in [("errors", &errors), ("warnings", &warns)] {
        if u.order.is_empty() {
            continue;
        }
        let _ = write!(out, "\n{title} ({} unique):", u.order.len());
        for (msg, k) in u.order.iter().take(cap) {
            let _ = write!(out, "\n  {msg}");
            if *k > 1 {
                let _ = write!(out, " ×{k}");
                eff.lossy = true;
            }
        }
        if u.order.len() > cap {
            eff.lose(format!("{} more unique {title}", u.order.len() - cap));
        }
    }
    if total > counts[0] + counts[1] + counts[2] {
        eff.lossy = true;
    }
    Ok(Rendered { text: out, eff })
}

// ---------------------------------------------------------------- num_stats

fn gcd(mut a: i128, mut b: i128) -> i128 {
    a = a.abs();
    b = b.abs();
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a
}

fn num_stats(c: &Ctx) -> RResult {
    let list = c.list.ok_or("input is not a list of numbers")?;
    let mut nums: Vec<&str> = Vec::with_capacity(list.len());
    for x in list {
        match x.as_ref() {
            J::Num(n) => nums.push(n),
            _ => return Err("items are not all numbers".into()),
        }
    }
    let n = nums.len();
    let mut eff = Effect::default();
    if n == 0 {
        return Ok(Rendered {
            text: "count 0".into(),
            eff,
        });
    }
    let min = nums
        .iter()
        .copied()
        .min_by(|a, b| cmp_num(a, b))
        .unwrap_or("");
    let max = nums
        .iter()
        .copied()
        .max_by(|a, b| cmp_num(a, b))
        .unwrap_or("");
    let ints: Option<Vec<i128>> = nums.iter().map(|s| s.parse::<i128>().ok()).collect();
    let exact_sum = ints
        .as_ref()
        .and_then(|v| v.iter().try_fold(0i128, |a, &x| a.checked_add(x)));
    let (sum, mean) = match exact_sum {
        Some(s) => {
            let cnt = n as i128;
            let mean = if s % cnt == 0 {
                (s / cnt).to_string()
            } else {
                let g = gcd(s, cnt);
                format!("{}/{} (≈{})", s / g, cnt / g, approx(s as f64 / n as f64))
            };
            (s.to_string(), mean)
        }
        None => {
            let s: f64 = nums.iter().map(|x| x.parse::<f64>().unwrap_or(0.0)).sum();
            // Integers too large for an exact sum, or floats: approximate.
            let sum = if ints.is_some() {
                format!("≈{}", approx(s))
            } else {
                approx(s)
            };
            (sum, format!("≈{}", approx(s / n as f64)))
        }
    };
    let k = c.std.raw.sample.unwrap_or(5);
    let mut out = format!("count {n} | min {min} | max {max} | sum {sum} | mean {mean}");
    if n <= k {
        let _ = write!(out, "\nvalues: {}", nums.join(", "));
    } else {
        let _ = write!(out, "\nsample: {} ({k} of {n})", nums[..k].join(", "));
        eff.lossy = true;
    }
    Ok(Rendered { text: out, eff })
}

// ---------------------------------------------------------------- tree

fn tree(c: &Ctx) -> RResult {
    let list = c.list.ok_or("input is not a list of paths")?;
    let mut paths: Vec<&str> = Vec::with_capacity(list.len());
    for x in list {
        match x.as_ref() {
            J::Str(s) => paths.push(s),
            _ => return Err("items are not all strings".into()),
        }
    }
    let sep = if paths.iter().any(|p| p.contains('/')) {
        '/'
    } else if paths.iter().any(|p| p.contains('\\')) {
        '\\'
    } else {
        '/'
    };
    let split = |p: &str| -> (Vec<String>, String) {
        let is_dir = p.ends_with(['/', '\\']);
        let mut segs: Vec<String> = p
            .trim_end_matches(['/', '\\'])
            .split(['/', '\\'])
            .map(str::to_string)
            .collect();
        let mut name = segs.pop().unwrap_or_default();
        if is_dir {
            name.push(sep);
        }
        (segs, name)
    };
    let parsed: Vec<(Vec<String>, String)> = paths.iter().map(|p| split(p)).collect();
    let mut prefix: Vec<String> = parsed.first().map(|p| p.0.clone()).unwrap_or_default();
    for (dir, _) in &parsed {
        let same = prefix
            .iter()
            .zip(dir.iter())
            .take_while(|(a, b)| a == b)
            .count();
        prefix.truncate(same);
    }
    let mut groups: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let s = sep.to_string();
    for (dir, name) in parsed {
        let rel = dir[prefix.len()..].join(&s);
        groups.entry(rel).or_default().push(name);
    }
    let mut eff = Effect::default();
    let mut out = if prefix.is_empty() {
        format!("{} paths", paths.len())
    } else {
        format!("{}{} ({} paths)", prefix.join(&s), sep, paths.len())
    };
    let cap = c.std.raw.max_items.unwrap_or(usize::MAX);
    let total = groups.len();
    for (rel, mut names) in groups.into_iter().take(cap) {
        names.sort();
        let names: Vec<String> = names
            .into_iter()
            .map(|n| {
                if n.contains(", ") || n != n.trim() {
                    quote(&n)
                } else {
                    n
                }
            })
            .collect();
        if rel.is_empty() {
            let _ = write!(out, "\n.: {}", names.join(", "));
        } else {
            let _ = write!(out, "\n{rel}{sep}: {}", names.join(", "));
        }
    }
    if total > cap {
        eff.lose(format!("{} more directories", total - cap));
    }
    Ok(Rendered { text: out, eff })
}
