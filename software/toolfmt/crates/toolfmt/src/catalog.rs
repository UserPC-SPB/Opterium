//! The catalog: the tools of every server as a tree of sections, a search,
//! and tool cards. The agent sees two small tools instead of every schema
//! in every request, and pays for a schema only when it needs the tool.

use crate::json::J;
use std::collections::HashMap;
use std::fmt::Write as _;

pub const CATALOG: &str = "catalog";
pub const CALL: &str = "call_tool";

/// A section with more direct tools than this is split by name prefix.
const MAX_DIRECT: usize = 20;
const SEARCH_LIMIT: usize = 15;

pub fn defs() -> Vec<J> {
    let catalog = r#"{"name":"catalog","description":"toolfmt: every tool of the connected servers as a tree. No args: sections as `name subsections_tools`. path: one section. query: find tools by words. tools: cards with call signature, params, output format, measured size and time, price, example. Then call with call_tool.","inputSchema":{"type":"object","properties":{"path":{"type":"string","description":"section path, e.g. math/vectors"},"query":{"type":"string","description":"words to search in tool names and descriptions"},"tools":{"type":"array","items":{"type":"string"},"description":"tool names to describe in full"}}},"annotations":{"readOnlyHint":true}}"#;
    let call = r#"{"name":"call_tool","description":"toolfmt: call a tool from the catalog. tool: its name; args: its arguments (formatting overrides like _render are allowed).","inputSchema":{"type":"object","properties":{"tool":{"type":"string"},"args":{"type":"object"}},"required":["tool"]}}"#;
    vec![
        J::parse(catalog).expect("catalog def"),
        J::parse(call).expect("call def"),
    ]
}

pub struct Entry {
    /// The name the tool is listed and called by.
    pub name: String,
    pub server: String,
    pub section: Vec<String>,
    pub def: J,
    pub read_only: bool,
    pub destructive: bool,
}

/// What the gateway knows about a tool beyond its definition.
#[derive(Default)]
pub struct Card {
    pub standard: Option<String>,
    pub calls: u64,
    pub errors: u64,
    pub raw: u64,
    pub out: u64,
    pub ms: u64,
    pub price: Option<String>,
    pub example: Option<String>,
}

impl Entry {
    fn description(&self) -> &str {
        self.def
            .get("description")
            .and_then(J::as_str)
            .unwrap_or("")
    }

    fn flags(&self) -> &'static str {
        if self.destructive {
            " (!)"
        } else if self.read_only {
            " (ro)"
        } else {
            ""
        }
    }

    fn path(&self) -> String {
        self.section.join("/")
    }
}

/// First sentence of the first line, at most `max` chars.
pub fn short(desc: &str, max: usize) -> String {
    let line = desc
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .unwrap_or("");
    let sentence = match line.find(". ") {
        Some(i) => &line[..=i],
        None => line,
    };
    if sentence.chars().count() > max {
        let mut s: String = sentence.chars().take(max).collect();
        s.push('…');
        s
    } else {
        sentence.to_string()
    }
}

#[derive(Default)]
struct Node {
    name: String,
    children: Vec<Node>,
    tools: Vec<usize>,
}

impl Node {
    fn child(&mut self, name: &str) -> &mut Node {
        let i = match self.children.iter().position(|c| c.name == name) {
            Some(i) => i,
            None => {
                self.children.push(Node {
                    name: name.to_string(),
                    ..Node::default()
                });
                self.children.len() - 1
            }
        };
        &mut self.children[i]
    }

    fn total(&self) -> usize {
        self.tools.len() + self.children.iter().map(Node::total).sum::<usize>()
    }

    fn find(&self, path: &[&str]) -> Option<&Node> {
        match path.split_first() {
            None => Some(self),
            Some((head, rest)) => self.children.iter().find(|c| c.name == *head)?.find(rest),
        }
    }
}

fn prefix(name: &str) -> Option<&str> {
    let i = name.find(['_', '-', '.'])?;
    (i > 0).then(|| &name[..i])
}

/// A crowded section is split by tool-name prefix (`cubint_add`,
/// `cubint_mul` -> `cubint`) when that gives at least two groups.
fn autogroup(n: &mut Node, entries: &[Entry]) {
    for c in &mut n.children {
        autogroup(c, entries);
    }
    if n.tools.len() <= MAX_DIRECT {
        return;
    }
    let mut counts: HashMap<&str, usize> = HashMap::new();
    for &t in &n.tools {
        if let Some(p) = prefix(&entries[t].name) {
            *counts.entry(p).or_default() += 1;
        }
    }
    let groups: Vec<&str> = counts
        .iter()
        .filter(|(_, c)| **c >= 3)
        .map(|(p, _)| *p)
        .collect();
    if groups.len() < 2 {
        return;
    }
    let tools = std::mem::take(&mut n.tools);
    for t in tools {
        match prefix(&entries[t].name).filter(|p| groups.contains(p)) {
            Some(p) => {
                let p = p.to_string();
                n.child(&p).tools.push(t);
            }
            None => n.tools.push(t),
        }
    }
}

fn build(entries: &[Entry]) -> Node {
    let mut root = Node::default();
    for (i, e) in entries.iter().enumerate() {
        let mut n = &mut root;
        for seg in &e.section {
            n = n.child(seg);
        }
        n.tools.push(i);
    }
    autogroup(&mut root, entries);
    root
}

fn tool_line(out: &mut String, e: &Entry, indent: &str) {
    let d = short(e.description(), 100);
    let _ = write!(out, "\n{indent}{}{}", e.name, e.flags());
    if !d.is_empty() {
        let _ = write!(out, " — {d}");
    }
}

/// The tree at `path` (root without one). `desc` gives section descriptions.
pub fn browse(
    entries: &[Entry],
    path: Option<&str>,
    desc: &dyn Fn(&str) -> Option<String>,
) -> Result<String, String> {
    let root = build(entries);
    let segs: Vec<&str> = path
        .unwrap_or("")
        .split('/')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();
    let node = root.find(&segs).ok_or_else(|| {
        let names: Vec<&str> = root.children.iter().map(|c| c.name.as_str()).collect();
        format!(
            "no section '{}'. Sections: {}",
            segs.join("/"),
            names.join(", ")
        )
    })?;
    let base = segs.join("/");
    let mut out = if segs.is_empty() {
        format!(
            "catalog: {} sections, {} tools (as name subsections_tools — description)",
            node.children.len(),
            node.total()
        )
    } else {
        let mut h = format!("{base} {}_{}", node.children.len(), node.total());
        if let Some(d) = desc(&base) {
            let _ = write!(h, " — {d}");
        }
        h
    };
    for c in &node.children {
        let full = if base.is_empty() {
            c.name.clone()
        } else {
            format!("{base}/{}", c.name)
        };
        let _ = write!(out, "\n  {} {}_{}", c.name, c.children.len(), c.total());
        if let Some(d) = desc(&full) {
            let _ = write!(out, " — {d}");
        }
    }
    if !node.tools.is_empty() {
        let _ = write!(out, "\ntools ({}):", node.tools.len());
        for &t in &node.tools {
            tool_line(&mut out, &entries[t], "  ");
        }
    }
    if segs.is_empty() {
        out.push_str("\nnext: catalog path=<section> | query=<words> | tools=[names] -> call_tool tool=<name> args={...}");
    }
    Ok(out)
}

fn words(q: &str) -> Vec<String> {
    q.split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.chars().count() >= 2)
        .map(str::to_lowercase)
        .collect()
}

/// Tools ranked by the words of `query` in name, description and section.
pub fn search(entries: &[Entry], query: &str) -> String {
    let ws = words(query);
    if ws.is_empty() {
        return "query has no words (two letters or more)".into();
    }
    let mut hits: Vec<(usize, usize)> = Vec::new();
    for (i, e) in entries.iter().enumerate() {
        let name = e.name.to_lowercase();
        let desc = e.description().to_lowercase();
        let path = e.path().to_lowercase();
        let mut score = 0;
        let mut matched = 0;
        for w in &ws {
            let s = 3 * usize::from(name.contains(w.as_str()))
                + usize::from(desc.contains(w.as_str()))
                + usize::from(path.contains(w.as_str()));
            if s > 0 {
                matched += 1;
            }
            score += s;
        }
        if score > 0 {
            if matched == ws.len() {
                score += 2;
            }
            hits.push((score, i));
        }
    }
    if hits.is_empty() {
        return format!("no tool matches \"{query}\". Browse: catalog");
    }
    // Ties keep catalog order: the first server's tool before a namespaced twin.
    hits.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    let mut out = format!("{} tools match \"{query}\"", hits.len());
    if hits.len() > SEARCH_LIMIT {
        let _ = write!(out, " (best {SEARCH_LIMIT})");
    }
    out.push(':');
    for &(_, i) in hits.iter().take(SEARCH_LIMIT) {
        let e = &entries[i];
        tool_line(&mut out, e, "  ");
        let _ = write!(out, " [{}]", e.path());
    }
    out
}

fn type_of(p: &J) -> String {
    if let Some(J::Arr(vals)) = p.get("enum") {
        let mut v: Vec<String> = vals.iter().take(6).map(J::to_json).collect();
        if vals.len() > 6 {
            v.push("…".into());
        }
        return v.join("|");
    }
    if let Some(c) = p.get("const") {
        return c.to_json();
    }
    match p.get("type") {
        Some(J::Str(t)) => match t.as_str() {
            "array" => format!(
                "{}[]",
                p.get("items").map(type_of).unwrap_or_else(|| "any".into())
            ),
            "object" => match p.get("properties").and_then(J::as_obj) {
                Some(m) if !m.is_empty() => {
                    let req = required(p);
                    let mut names: Vec<String> = m
                        .iter()
                        .take(8)
                        .map(|(k, _)| {
                            if req.contains(k) {
                                k.clone()
                            } else {
                                format!("{k}?")
                            }
                        })
                        .collect();
                    if m.len() > 8 {
                        names.push("…".into());
                    }
                    format!("{{{}}}", names.join(", "))
                }
                _ => "object".into(),
            },
            other => other.to_string(),
        },
        Some(J::Arr(ts)) => ts.iter().map(J::text).collect::<Vec<_>>().join("|"),
        _ => {
            for k in ["anyOf", "oneOf"] {
                if let Some(J::Arr(alts)) = p.get(k) {
                    return alts.iter().map(type_of).collect::<Vec<_>>().join("|");
                }
            }
            if let Some(r) = p.get("$ref").and_then(J::as_str) {
                return r.rsplit('/').next().unwrap_or(r).to_string();
            }
            "any".into()
        }
    }
}

fn required(schema: &J) -> Vec<String> {
    schema
        .get("required")
        .and_then(J::as_arr)
        .map(|a| a.iter().map(J::text).collect())
        .unwrap_or_default()
}

/// Names of required parameters missing from `args`.
pub fn missing(schema: Option<&J>, args: &J) -> Vec<String> {
    let Some(s) = schema else { return Vec::new() };
    required(s)
        .into_iter()
        .filter(|k| args.get(k).is_none())
        .collect()
}

/// `a: number, b?: string = "x"` and one line per documented parameter.
pub fn signature(schema: Option<&J>) -> (String, Vec<String>) {
    let Some(s) = schema else {
        return (String::new(), Vec::new());
    };
    let req = required(s);
    let mut parts = Vec::new();
    let mut lines = Vec::new();
    if let Some(props) = s.get("properties").and_then(J::as_obj) {
        for (k, p) in props {
            let opt = if req.contains(k) { "" } else { "?" };
            let mut part = format!("{k}{opt}: {}", type_of(p));
            if let Some(d) = p.get("default") {
                let _ = write!(part, " = {}", d.to_json());
            }
            parts.push(part);
            if let Some(d) = p.get("description").and_then(J::as_str) {
                lines.push(format!("  {k}: {}", short(d, 160)));
            }
        }
    }
    (parts.join(", "), lines)
}

fn find<'a>(entries: &'a [Entry], name: &str) -> Option<&'a Entry> {
    entries
        .iter()
        .find(|e| e.name == name)
        .or_else(|| entries.iter().find(|e| e.name.eq_ignore_ascii_case(name)))
        .or_else(|| {
            let (s, t) = name.split_once('.').or_else(|| name.split_once("__"))?;
            entries
                .iter()
                .find(|e| e.server == s && e.def.get("name").and_then(J::as_str) == Some(t))
        })
}

/// Full cards for the named tools.
pub fn cards(entries: &[Entry], names: &[String], card: &dyn Fn(&Entry) -> Card) -> String {
    let mut out = String::new();
    for name in names {
        if !out.is_empty() {
            out.push_str("\n\n");
        }
        let Some(e) = find(entries, name) else {
            let _ = write!(out, "{name}: unknown tool. {}", search(entries, name));
            continue;
        };
        let c = card(e);
        let _ = write!(out, "{}{} [{}]", e.name, e.flags(), e.path());
        let d = e.description().trim();
        if !d.is_empty() {
            let d: String = d.chars().take(600).collect();
            let _ = write!(out, "\n{}", d.replace("\n\n", "\n"));
        }
        let (sig, lines) = signature(e.def.get("inputSchema"));
        let _ = write!(out, "\ncall: {}({sig})", e.name);
        for l in lines {
            out.push('\n');
            out.push_str(&l);
        }
        match &c.standard {
            Some(r) => {
                let _ = write!(out, "\noutput: formatted by standard ({r})");
            }
            None => out.push_str("\noutput: as the tool returns it (no standard)"),
        }
        if c.calls > 0 {
            let n = c.calls;
            let _ = write!(
                out,
                "\nmeasured: {n} calls, avg {} -> {} chars, avg {} ms",
                c.raw / n,
                c.out / n,
                c.ms / n
            );
            if c.errors > 0 {
                let _ = write!(out, ", {} errors", c.errors);
            }
        } else {
            out.push_str("\nmeasured: not called yet");
        }
        if let Some(p) = &c.price {
            let _ = write!(out, "\nprice: {p}");
        }
        if let Some(x) = &c.example {
            let _ = write!(out, "\nexample: {x}");
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(name: &str, section: &str, desc: &str, schema: &str) -> Entry {
        Entry {
            name: name.into(),
            server: "s".into(),
            section: section.split('/').map(str::to_string).collect(),
            def: J::parse(&format!(
                r#"{{"name":"{name}","description":"{desc}","inputSchema":{schema}}}"#
            ))
            .unwrap(),
            read_only: false,
            destructive: false,
        }
    }

    fn corpus() -> Vec<Entry> {
        let mut v = Vec::new();
        for i in 0..12 {
            v.push(entry(
                &format!("cubint_op{i}"),
                "math",
                "Exact integer operation.",
                "{}",
            ));
        }
        for i in 0..12 {
            v.push(entry(
                &format!("vec_op{i}"),
                "math",
                "Vector operation. More text.",
                "{}",
            ));
        }
        v.push(entry("norm", "math", "Euclidean norm of a vector.", "{}"));
        v.push(entry(
            "shot",
            "eyes",
            "Takes a screenshot.",
            r#"{"type":"object","properties":{"x":{"type":"integer","description":"left edge"},"fmt":{"enum":["png","jpg"],"default":"png"},"tags":{"type":"array","items":{"type":"string"}}},"required":["x"]}"#,
        ));
        v
    }

    #[test]
    fn tree_counts_and_autogroups() {
        let e = corpus();
        let root = browse(&e, None, &|p| (p == "math").then(|| "numbers".to_string())).unwrap();
        assert!(root.starts_with("catalog: 2 sections, 26 tools"), "{root}");
        assert!(root.contains("\n  math 2_25 — numbers"), "{root}");
        assert!(root.contains("\n  eyes 0_1"), "{root}");
        let math = browse(&e, Some("math"), &|_| None).unwrap();
        assert!(math.contains("\n  cubint 0_12"), "{math}");
        assert!(
            math.contains("\ntools (1):\n  norm — Euclidean norm of a vector."),
            "{math}"
        );
        let sub = browse(&e, Some("math/vec"), &|_| None).unwrap();
        assert!(sub.contains("vec_op3 — Vector operation."), "{sub}");
        assert!(browse(&e, Some("nope"), &|_| None).is_err());
    }

    #[test]
    fn search_ranks_name_matches_first() {
        let e = corpus();
        let s = search(&e, "vector norm");
        let first = s.lines().nth(1).unwrap();
        assert!(first.trim_start().starts_with("norm"), "{s}");
        assert!(s.starts_with("13 tools match"), "{s}");
        assert!(search(&e, "op").contains("(best 15)"));
    }

    #[test]
    fn card_has_signature_and_measures() {
        let e = corpus();
        let t = cards(&e, &["shot".into()], &|_| Card {
            calls: 4,
            raw: 4000,
            out: 400,
            ms: 80,
            price: Some("free".into()),
            example: Some(r#"{"x":1}"#.into()),
            ..Card::default()
        });
        assert!(
            t.contains(r#"call: shot(x: integer, fmt?: "png"|"jpg" = "png", tags?: string[])"#),
            "{t}"
        );
        assert!(t.contains("  x: left edge"), "{t}");
        assert!(
            t.contains("measured: 4 calls, avg 1000 -> 100 chars, avg 20 ms"),
            "{t}"
        );
        assert!(
            t.contains("price: free") && t.contains(r#"example: {"x":1}"#),
            "{t}"
        );
        let schema = e.last().unwrap().def.get("inputSchema");
        assert_eq!(missing(schema, &J::obj()), vec!["x".to_string()]);
    }
}
