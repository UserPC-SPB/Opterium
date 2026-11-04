//! The pipeline: DETECT -> RESOLVE -> PRE -> RENDER -> POST -> HEADER.

use crate::detect::{Shape, detect};
use crate::json::{J, Seg, split_path};
use crate::ops::{Effect, apply_post, apply_pre};
use crate::render::{Ctx, render};
use crate::standard::{
    Loss, PostOp, PreOp, Render, Standard, Store, compile, json_to_yaml, parse_post_list,
    parse_pre_list,
};
use serde_yaml::Value as Y;
use std::borrow::Cow;

/// One-off instructions passed with a call (`_render`, `_pre`, ...).
#[derive(Default, Clone, Debug)]
pub struct Overrides {
    pub render: Option<String>,
    pub pre: Option<Vec<Y>>,
    pub post: Option<Vec<Y>>,
    pub template: Option<String>,
    pub limit: Option<usize>,
    pub keys: Option<Vec<String>>,
}

pub const OVERRIDE_NAMES: [&str; 6] = ["_render", "_pre", "_post", "_template", "_limit", "_keys"];

fn chain(v: &J) -> Option<Vec<Y>> {
    match v {
        J::Arr(_) => match json_to_yaml(v) {
            Y::Sequence(s) => Some(s),
            _ => None,
        },
        J::Str(s) => match serde_yaml::from_str::<Y>(s).ok()? {
            Y::Sequence(seq) => Some(seq),
            other => Some(vec![other]),
        },
        J::Obj(_) => Some(vec![json_to_yaml(v)]),
        _ => None,
    }
}

fn strings(v: &J) -> Option<Vec<String>> {
    match v {
        J::Arr(a) => Some(a.iter().map(J::text).collect()),
        J::Str(s) => Some(
            s.split(',')
                .map(|x| x.trim().to_string())
                .filter(|x| !x.is_empty())
                .collect(),
        ),
        _ => None,
    }
}

impl Overrides {
    pub fn is_empty(&self) -> bool {
        self.render.is_none()
            && self.pre.is_none()
            && self.post.is_none()
            && self.template.is_none()
            && self.limit.is_none()
            && self.keys.is_none()
    }

    fn set(&mut self, name: &str, v: &J) {
        match name {
            "_render" => self.render = v.as_str().map(str::to_string),
            "_pre" => self.pre = chain(v),
            "_post" => self.post = chain(v),
            "_template" => {
                self.template = match v {
                    J::Str(s) => Some(s.clone()),
                    J::Obj(_) => serde_yaml::to_string(&json_to_yaml(v)).ok(),
                    _ => None,
                }
            }
            "_limit" => self.limit = v.text().trim().parse().ok(),
            "_keys" => self.keys = strings(v),
            _ => {}
        }
    }

    /// Takes the override parameters out of call arguments. A name the
    /// upstream tool itself declares is a real parameter and stays.
    pub fn extract(args: &mut J, declared: &[String]) -> Overrides {
        let mut ov = Overrides::default();
        for name in OVERRIDE_NAMES {
            if declared.iter().any(|d| d == name) {
                continue;
            }
            if let Some(v) = args.remove(name) {
                ov.set(name, &v);
            }
        }
        ov
    }

    /// Reads the fields of a `/clean` request body.
    pub fn from_request(body: &J) -> Overrides {
        let mut ov = Overrides::default();
        for name in OVERRIDE_NAMES {
            if let Some(v) = body.get(name).filter(|v| !matches!(v, J::Null)) {
                ov.set(name, v);
            }
        }
        ov
    }
}

pub enum Data<'a> {
    Text(&'a str),
    Json(&'a J),
}

pub struct Request<'a> {
    pub server: &'a str,
    pub tool: &'a str,
    pub args: &'a J,
    pub data: Data<'a>,
    pub keys: Option<Vec<String>>,
    pub ov: Overrides,
    /// Built-in auto mode (off by default: D1).
    pub auto: bool,
    pub family: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Body {
    /// No instruction: the upstream output goes out exactly as received.
    Untouched,
    /// An instruction failed: raw output plus this note (D2).
    Fallback(String),
    Formatted(String),
}

#[derive(Debug, Clone)]
pub struct Outcome {
    pub body: Body,
    /// What happened, in the revision-2 vocabulary (`objects_table:<tool>`).
    pub used: String,
    /// First line shown to the agent; `None` for untouched output.
    pub header: Option<String>,
    pub loss: Loss,
    pub raw_chars: usize,
    pub out_chars: usize,
}

impl Outcome {
    /// The text the agent receives, given the raw payload.
    pub fn agent_text(&self, raw: &str) -> String {
        match (&self.body, &self.header) {
            (Body::Formatted(t), Some(h)) => format!("{h}\n{t}"),
            (Body::Formatted(t), None) => t.clone(),
            (Body::Fallback(_), Some(h)) => format!("{h}\n{raw}"),
            _ => raw.to_string(),
        }
    }
}

fn parse_any(t: &str) -> Option<J> {
    let s = t.trim();
    let first = s.bytes().next()?;
    if matches!(
        first,
        b'{' | b'[' | b'"' | b'-' | b'0'..=b'9' | b't' | b'f' | b'n'
    ) {
        J::parse(s).ok()
    } else {
        None
    }
}

/// Built-in auto mode: lossless layouts only, and only when they help.
fn builtin_render(shape: Shape, json: Option<&J>, raw_chars: usize) -> Option<Render> {
    let pretty = || json.is_some_and(|j| raw_chars * 20 > j.to_json().chars().count() * 23);
    match shape {
        Shape::Matrix2d => Some(Render::MatrixTable),
        Shape::JsonList => Some(Render::ObjectsTable),
        Shape::JsonObj => {
            let envelope = json.and_then(J::as_obj).is_some_and(|m| {
                m.iter().any(|(_, v)| {
                    matches!(v, J::Arr(a) if !a.is_empty() && a.iter().all(|x| matches!(x, J::Obj(_))))
                })
            });
            if envelope {
                Some(Render::ObjectsTable)
            } else if pretty() {
                Some(Render::Json)
            } else {
                None
            }
        }
        Shape::NumList | Shape::StrList | Shape::JsonMixed if pretty() => Some(Render::Json),
        _ => None,
    }
}

fn apply_overrides(s: &mut Standard, ov: &Overrides, tool: &str) -> Result<(), String> {
    if let Some(r) = &ov.render {
        s.set_render(Render::parse(r)?, tool)?;
    }
    if let Some(pre) = &ov.pre {
        s.pre = parse_pre_list(pre).map_err(|e| format!("_pre: {e}"))?;
    }
    if let Some(post) = &ov.post {
        s.post = parse_post_list(post).map_err(|e| format!("_post: {e}"))?;
        s.set_render(s.render, tool)?;
    }
    if let Some(n) = ov.limit {
        match s.render {
            Render::ObjectsTable
            | Render::MatrixTable
            | Render::NumStats
            | Render::Tree
            | Render::Json => s.pre.push(PreOp::Limit(n)),
            Render::DictTable => s.raw.max_rows = Some(n),
            _ => s.post.push(PostOp::TruncateLines(n)),
        }
    }
    Ok(())
}

const CURSOR_KEYS: [&str; 14] = [
    "cursor",
    "nextcursor",
    "next",
    "nextpage",
    "nextpagetoken",
    "pagetoken",
    "nexttoken",
    "continuation",
    "continuationtoken",
    "hasmore",
    "hasnextpage",
    "endcursor",
    "nextlink",
    "odatanextlink",
];
const CURSOR_HOLDERS: [&str; 7] = [
    "meta",
    "pagination",
    "pageinfo",
    "links",
    "paging",
    "page",
    "metadata",
];

fn norm(k: &str) -> String {
    k.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn live_cursor(v: &J) -> bool {
    match v {
        J::Str(s) => !s.is_empty(),
        J::Num(_) | J::Bool(true) => true,
        _ => false,
    }
}

/// Pagination cursors of a document (I8): top level and one level down in
/// the usual holders (`meta`, `pagination`, `pageInfo`, ...).
pub fn cursors(j: &J) -> Vec<(String, &J)> {
    let mut out = Vec::new();
    let Some(m) = j.as_obj() else { return out };
    for (k, v) in m {
        let n = norm(k);
        if CURSOR_KEYS.contains(&n.as_str()) && live_cursor(v) {
            out.push((k.clone(), v));
        } else if CURSOR_HOLDERS.contains(&n.as_str()) {
            if let Some(inner) = v.as_obj() {
                for (k2, v2) in inner {
                    if CURSOR_KEYS.contains(&norm(k2).as_str()) && live_cursor(v2) {
                        out.push((format!("{k}.{k2}"), v2));
                    }
                }
            }
        }
    }
    out
}

/// Walks `root`, collecting the fields beside the path (they stay in the
/// output: cursors, totals). Returns the node and the key it sits under.
fn descend<'a>(
    j: &'a J,
    path: &str,
    rest: &mut Vec<(String, &'a J)>,
) -> Result<(&'a J, Option<String>), String> {
    let segs = split_path(path).ok_or_else(|| format!("bad root '{path}'"))?;
    let mut cur = j;
    let mut prefix = String::new();
    let mut key = None;
    for seg in segs {
        match seg {
            Seg::Key(k) => {
                let m = cur
                    .as_obj()
                    .ok_or_else(|| format!("root '{path}': '{k}' is not inside an object"))?;
                for (kk, vv) in m {
                    if kk != k {
                        rest.push((format!("{prefix}{kk}"), vv));
                    }
                }
                cur = cur
                    .get(k)
                    .ok_or_else(|| format!("root '{path}': no key '{k}'"))?;
                prefix = format!("{prefix}{k}.");
                key = Some(k.to_string());
            }
            Seg::Index(i) => {
                cur = cur
                    .as_arr()
                    .and_then(|a| a.get(i))
                    .ok_or_else(|| format!("root '{path}': no index {i}"))?;
                key = None;
            }
        }
    }
    Ok((cur, key))
}

/// The list inside an envelope object: the only array field, or the
/// longest array of objects.
fn auto_root<'a>(
    m: &'a [(String, J)],
    prefix: &str,
    rest: &mut Vec<(String, &'a J)>,
) -> Option<(&'a Vec<J>, String)> {
    let arrays: Vec<usize> = (0..m.len())
        .filter(|&i| matches!(m[i].1, J::Arr(_)))
        .collect();
    let pick = if arrays.len() == 1 {
        arrays[0]
    } else {
        *arrays
            .iter()
            .filter(|&&i| matches!(&m[i].1, J::Arr(a) if a.iter().all(|x| matches!(x, J::Obj(_)))))
            .max_by_key(|&&i| m[i].1.as_arr().map_or(0, Vec::len))?
    };
    for (i, (k, v)) in m.iter().enumerate() {
        if i != pick {
            rest.push((format!("{prefix}{k}"), v));
        }
    }
    Some((m[pick].1.as_arr()?, m[pick].0.clone()))
}

struct Input<'a> {
    raw: &'a str,
    text: &'a str,
    json: Option<&'a J>,
}

fn execute(s: &Standard, inp: &Input, req: &Request) -> Result<(String, Effect), String> {
    let mut rest: Vec<(String, &J)> = Vec::new();
    let mut list_key: Option<String> = None;
    let node: Option<&J> = match (&s.raw.root, inp.json) {
        (Some(r), Some(j)) => {
            let (n, k) = descend(j, r, &mut rest)?;
            list_key = k;
            Some(n)
        }
        (Some(r), None) => return Err(format!("root '{r}': input is not JSON")),
        (None, j) => j,
    };
    let wants_list = matches!(
        s.render,
        Render::ObjectsTable | Render::MatrixTable | Render::NumStats | Render::Tree | Render::Json
    );
    let list_src: Option<&Vec<J>> = match node {
        Some(J::Arr(a)) => Some(a),
        Some(J::Obj(m)) if wants_list && s.render != Render::Json => {
            let prefix = s.raw.root.as_deref().map(|r| {
                let r = r.trim().trim_start_matches('$').trim_start_matches('.');
                if r.is_empty() {
                    String::new()
                } else {
                    format!("{r}.")
                }
            });
            let before = rest.len();
            match auto_root(m, prefix.as_deref().unwrap_or(""), &mut rest) {
                Some((a, k)) => {
                    list_key = Some(k);
                    Some(a)
                }
                None => {
                    rest.truncate(before);
                    None
                }
            }
        }
        _ => None,
    };
    let mut items: Vec<Cow<J>> = list_src
        .map(|a| a.iter().map(Cow::Borrowed).collect())
        .unwrap_or_default();
    let mut eff = Effect::default();
    if list_src.is_some() {
        apply_pre(&s.pre, &mut items, &mut eff);
    }
    let keys = req.keys.as_deref().or(req.ov.keys.as_deref());
    let ctx = Ctx {
        std: s,
        tool: req.tool,
        server: req.server,
        text: inp.text,
        raw: inp.raw,
        json: inp.json,
        node,
        list: list_src.map(|_| items.as_slice()),
        list_key: list_key.as_deref(),
        rest: &rest,
        args: req.args,
        keys,
    };
    let r = render(&ctx)?;
    eff.lossy |= r.eff.lossy;
    eff.notes.extend(r.eff.notes);
    let mut text = apply_post(&s.post, r.text, &mut eff);
    for n in &eff.notes {
        text.push_str("\n… ");
        text.push_str(n);
    }
    if let Some(j) = inp.json {
        for (k, v) in cursors(j) {
            let vt = v.text();
            if !(text.contains(&vt) && text.contains(k.rsplit('.').next().unwrap_or(&k))) {
                text.push_str(&format!("\ncursor: {k}={vt}"));
            }
        }
    }
    Ok((text, eff))
}

fn header(s: &Standard, builtin: bool, keys: Option<&[String]>, loss: Loss) -> String {
    let base = if builtin {
        format!("builtin:{}", s.render.name())
    } else {
        match keys {
            Some(k) if s.render == Render::Template && !k.is_empty() => {
                format!("template[{}]", k.join(","))
            }
            _ => s.render.name().to_string(),
        }
    };
    match loss {
        Loss::Lossless => format!("[{base}]"),
        l => format!("[{base}:{}]", l.name()),
    }
}

/// Runs the pipeline. `fixed` replaces the standard lookup (used when a
/// standard is tested before it is saved).
pub fn run(store: Option<&Store>, req: &Request, fixed: Option<&Standard>) -> Outcome {
    // Most tools have no standard: then the payload is not even parsed (D1
    // costs nothing, also for a 30 MB output).
    let modifies = req.ov.template.is_some()
        || req.ov.render.is_some()
        || req.ov.pre.is_some()
        || req.ov.post.is_some()
        || req.ov.limit.is_some();
    if fixed.is_none() && !modifies && !req.auto {
        let has_tool = store.is_some_and(|st| st.tool(req.server, req.tool).is_some());
        let has_family = match (store, &req.family) {
            (Some(st), Some(f)) => st.family(f, req.tool).is_some(),
            _ => false,
        };
        if !has_tool && !has_family {
            let chars = match req.data {
                Data::Text(t) => t.chars().count(),
                Data::Json(j) => j.json_chars(),
            };
            return Outcome {
                body: Body::Untouched,
                used: "passthrough".into(),
                header: None,
                loss: Loss::Lossless,
                raw_chars: chars,
                out_chars: chars,
            };
        }
    }
    let owned_json;
    let owned_raw;
    let (raw, json): (&str, Option<&J>) = match req.data {
        Data::Text(t) => {
            owned_json = parse_any(t);
            (t, owned_json.as_ref())
        }
        Data::Json(j) => {
            owned_raw = j.to_json();
            (owned_raw.as_str(), Some(j))
        }
    };
    let text = match json {
        Some(J::Str(s)) => s.as_str(),
        _ => raw,
    };
    let inp = Input { raw, text, json };
    let shape = detect(text, json);
    let raw_chars = raw.chars().count();

    let untouched = || Outcome {
        body: Body::Untouched,
        used: "passthrough".into(),
        header: None,
        loss: Loss::Lossless,
        raw_chars,
        out_chars: raw_chars,
    };
    let fallback = |reason: String| {
        let h = format!("[fallback:raw] {reason}");
        Outcome {
            out_chars: raw_chars + h.chars().count() + 1,
            body: Body::Fallback(reason),
            used: "fallback:raw".into(),
            header: Some(h),
            loss: Loss::Lossless,
            raw_chars,
        }
    };

    // RESOLVE: override > tool standard > family standard > builtin > none.
    let inline = match &req.ov.template {
        Some(t) => match compile(t, "inline", req.tool) {
            Ok(s) => Some(s),
            Err(e) => return fallback(format!("_template: {e}")),
        },
        None => None,
    };
    let looked_up = inline.is_none() && fixed.is_none();
    let tool_arc = if looked_up {
        store.and_then(|st| st.tool(req.server, req.tool))
    } else {
        None
    };
    let fam_arc = if looked_up && tool_arc.is_none() {
        match (store, &req.family) {
            (Some(st), Some(f)) => st.family(f, req.tool),
            _ => None,
        }
    } else {
        None
    };
    let from_file = match tool_arc.as_deref().or(fam_arc.as_deref()) {
        Some(Ok(s)) => Some(s),
        Some(Err(e)) => {
            let which = if tool_arc.is_some() {
                "standard"
            } else {
                "family standard"
            };
            return fallback(format!("{which}: {e}"));
        }
        None => None,
    };
    let base: Option<&Standard> = inline.as_ref().or(fixed).or(from_file);
    let has_ov = req.ov.render.is_some()
        || req.ov.pre.is_some()
        || req.ov.post.is_some()
        || req.ov.limit.is_some();
    let mut builtin = false;
    let s: Cow<Standard> = match (base, has_ov) {
        (Some(b), false) => Cow::Borrowed(b),
        (b, true) => {
            let mut s = b.cloned().unwrap_or_else(|| {
                Standard::builtin(
                    builtin_render(shape, json, raw_chars).unwrap_or(Render::Passthrough),
                )
            });
            if let Err(e) = apply_overrides(&mut s, &req.ov, req.tool) {
                return fallback(e);
            }
            Cow::Owned(s)
        }
        (None, false) => match builtin_render(shape, json, raw_chars) {
            Some(r) if req.auto => {
                builtin = true;
                Cow::Owned(Standard::builtin(r))
            }
            _ => return untouched(),
        },
    };

    let keys = req.keys.as_deref().or(req.ov.keys.as_deref());
    let done = |s: &Standard, builtin: bool, text: String, eff: Effect| -> Outcome {
        let loss = s.declared.max(if eff.lossy {
            Loss::Lossy
        } else {
            Loss::Lossless
        });
        let h = header(s, builtin, keys, loss);
        let used = if builtin {
            format!("builtin:{}->{}", shape.name(), s.render.name())
        } else if s.render == Render::Template && keys.is_some_and(|k| !k.is_empty()) {
            format!(
                "template:{}[{}]",
                req.tool,
                keys.unwrap_or_default().join(",")
            )
        } else {
            format!("{}:{}", s.render.name(), req.tool)
        };
        Outcome {
            out_chars: h.chars().count() + 1 + text.chars().count(),
            body: Body::Formatted(text),
            used,
            header: Some(h),
            loss,
            raw_chars,
        }
    };

    match execute(&s, &inp, req) {
        Ok((text, eff)) => {
            let out = done(&s, builtin, text, eff);
            if builtin && out.out_chars >= raw_chars {
                return untouched();
            }
            out
        }
        Err(reason) => {
            if !builtin && req.auto {
                if let Some(r) = builtin_render(shape, json, raw_chars) {
                    if r != s.render {
                        let b = Standard::builtin(r);
                        if let Ok((text, eff)) = execute(&b, &inp, req) {
                            let out = done(&b, true, text, eff);
                            if out.out_chars < raw_chars {
                                return out;
                            }
                        }
                    }
                }
            }
            if builtin {
                return untouched();
            }
            fallback(format!("{}: {reason}", s.render.name()))
        }
    }
}

/// The `/clean` contract: request body in, response body out.
pub fn clean(store: Option<&Store>, body: &J, auto: bool, family: Option<String>) -> J {
    let server = body.get("server").and_then(J::as_str).unwrap_or("");
    let tool = body.get("tool").and_then(J::as_str).unwrap_or("");
    let empty = J::obj();
    let args = body
        .get("args")
        .filter(|a| matches!(a, J::Obj(_)))
        .unwrap_or(&empty);
    let data_owned;
    let data = match body.get("data") {
        Some(J::Str(s)) => s.as_str(),
        Some(J::Null) | None => "",
        Some(other) => {
            data_owned = other.to_json();
            data_owned.as_str()
        }
    };
    let keys = body.get("keys").and_then(|k| match k {
        J::Null => None,
        other => strings(other),
    });
    let auto = body.get("auto").and_then(J::as_bool).unwrap_or(auto);
    let family = body
        .get("family")
        .and_then(J::as_str)
        .map(str::to_string)
        .or(family);
    let req = Request {
        server,
        tool,
        args,
        data: Data::Text(data),
        keys,
        ov: Overrides::from_request(body),
        auto,
        family,
    };
    let out = run(store, &req, None);
    let text = match &out.body {
        Body::Formatted(t) => t.clone(),
        _ => data.to_string(),
    };
    J::obj()
        .with("text", J::Str(text))
        .with("used", J::Str(out.used.clone()))
        .with("loss", J::str(out.loss.name()))
        .with("header", out.header.clone().map(J::Str).unwrap_or(J::Null))
        .with("raw_chars", J::int(out.raw_chars as i64))
        .with("out_chars", J::int(out.out_chars as i64))
}
