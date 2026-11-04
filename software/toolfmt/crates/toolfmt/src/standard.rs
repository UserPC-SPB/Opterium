//! Standards: the YAML instructions that say how one tool's output is shown.

use crate::json::J;
use any_txt_parser::template::{CompiledTemplate, compile_yaml_str};
use regex::{Regex, RegexBuilder};
use serde::Deserialize;
use serde_yaml::Value as Y;
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

#[derive(Deserialize, Default, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct RawStandard {
    pub version: Option<u32>,
    pub name: Option<String>,
    pub server: Option<String>,
    pub description: Option<String>,
    pub loss: Option<String>,
    pub render: Option<String>,
    pub format: Option<String>,
    pub mode: Option<String>,
    pub root: Option<String>,
    pub columns: Option<Vec<String>>,
    pub rows: Option<usize>,
    pub max_rows: Option<usize>,
    pub max_cols: Option<usize>,
    pub max_items: Option<usize>,
    pub max_cell_chars: Option<usize>,
    pub sample: Option<usize>,
    pub formats: Option<BTreeMap<String, String>>,
    pub input: Option<Y>,
    pub rules: Option<Y>,
    pub pre: Option<Vec<Y>>,
    pub post: Option<Vec<Y>>,
    /// Accepted for revision-2 files; family routing lives in upstreams.json.
    pub family: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Render {
    Passthrough,
    Json,
    Expression,
    MatrixTable,
    ObjectsTable,
    DictTable,
    Template,
    Log,
    NumStats,
    Tree,
    ImageMeta,
}

impl Render {
    pub fn parse(s: &str) -> Result<Render, String> {
        Ok(match s.trim() {
            "passthrough" => Render::Passthrough,
            "json" => Render::Json,
            "expression" => Render::Expression,
            "matrix_table" => Render::MatrixTable,
            "objects_table" => Render::ObjectsTable,
            "dict_table" => Render::DictTable,
            "template" => Render::Template,
            "log" => Render::Log,
            "num_stats" => Render::NumStats,
            "tree" => Render::Tree,
            "image_meta" => Render::ImageMeta,
            "image_verdict" | "image_downscale" => {
                return Err(format!(
                    "render '{s}' is planned and not available in this build"
                ));
            }
            other => {
                return Err(format!(
                    "unknown render '{other}' (expected passthrough, json, expression, matrix_table, objects_table, dict_table, template, log, num_stats, tree, image_meta)"
                ));
            }
        })
    }

    pub fn name(self) -> &'static str {
        match self {
            Render::Passthrough => "passthrough",
            Render::Json => "json",
            Render::Expression => "expression",
            Render::MatrixTable => "matrix_table",
            Render::ObjectsTable => "objects_table",
            Render::DictTable => "dict_table",
            Render::Template => "template",
            Render::Log => "log",
            Render::NumStats => "num_stats",
            Render::Tree => "tree",
            Render::ImageMeta => "image_meta",
        }
    }

    /// Renders whose output is a table, where line cuts would break rows.
    pub fn is_table(self) -> bool {
        matches!(
            self,
            Render::MatrixTable | Render::ObjectsTable | Render::DictTable
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Loss {
    Lossless,
    Lossy,
    Interpretive,
}

impl Loss {
    pub fn parse(s: &str) -> Result<Loss, String> {
        match s.trim() {
            "lossless" => Ok(Loss::Lossless),
            "lossy" => Ok(Loss::Lossy),
            "interpretive" => Ok(Loss::Interpretive),
            o => Err(format!(
                "unknown loss '{o}' (expected lossless, lossy, interpretive)"
            )),
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Loss::Lossless => "lossless",
            Loss::Lossy => "lossy",
            Loss::Interpretive => "interpretive",
        }
    }
}

#[derive(Clone, Debug)]
pub enum PreOp {
    Unique,
    Sort { desc: bool, by: Option<String> },
    Limit(usize),
    Select(Vec<String>),
}

#[derive(Clone, Debug)]
pub enum PostOp {
    Join(String),
    Replace {
        re: Regex,
        to: String,
        literal: bool,
    },
    TruncateChars(usize),
    TruncateLines(usize),
    Mask(Vec<(Regex, String)>),
    DropLines(Regex),
    KeepLines(Regex),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValueFormat {
    Thousands,
    Bytes,
    Fixed(usize),
}

impl ValueFormat {
    pub fn parse(s: &str) -> Result<ValueFormat, String> {
        let s = s.trim();
        if s == "thousands" {
            return Ok(ValueFormat::Thousands);
        }
        if s == "bytes" {
            return Ok(ValueFormat::Bytes);
        }
        if let Some(n) = s.strip_prefix("fixed:") {
            return n
                .trim()
                .parse()
                .map(ValueFormat::Fixed)
                .map_err(|_| format!("bad value format '{s}'"));
        }
        Err(format!(
            "unknown value format '{s}' (expected thousands, bytes, fixed:N)"
        ))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Toon,
    Markdown,
    Plain,
    Lines,
}

#[derive(Clone, Debug)]
pub struct Standard {
    pub raw: RawStandard,
    pub render: Render,
    pub mode: Mode,
    pub declared: Loss,
    pub pre: Vec<PreOp>,
    pub post: Vec<PostOp>,
    pub formats: Vec<(String, ValueFormat)>,
    pub template: Option<CompiledTemplate>,
    /// Where the standard came from: a file path, `inline`, `builtin`.
    pub origin: String,
}

impl Standard {
    pub fn builtin(render: Render) -> Standard {
        Standard {
            raw: RawStandard::default(),
            render,
            mode: default_mode(render),
            declared: Loss::Lossless,
            pre: Vec::new(),
            post: Vec::new(),
            formats: Vec::new(),
            template: None,
            origin: "builtin".into(),
        }
    }

    pub fn format_for(&self, key: &str) -> Option<ValueFormat> {
        self.formats
            .iter()
            .find(|(k, _)| k == key)
            .or_else(|| self.formats.iter().find(|(k, _)| k == "*"))
            .map(|(_, f)| *f)
    }

    /// Replaces the render (a `_render` override), keeping the chains.
    pub fn set_render(&mut self, render: Render, tool: &str) -> Result<(), String> {
        self.render = render;
        self.mode = parse_mode(render, self.raw.mode.as_deref()).unwrap_or(default_mode(render));
        if render == Render::Template && self.template.is_none() {
            self.template = Some(compile_template(&self.raw, tool, &self.origin)?);
        }
        if render == Render::Expression && self.raw.format.is_none() {
            return Err("render expression needs 'format'".into());
        }
        check_post_for_render(&self.post, render)
    }
}

fn default_mode(render: Render) -> Mode {
    match render {
        Render::ObjectsTable => Mode::Toon,
        Render::MatrixTable => Mode::Plain,
        _ => Mode::Lines,
    }
}

fn parse_mode(render: Render, mode: Option<&str>) -> Result<Mode, String> {
    let Some(m) = mode else {
        return Ok(default_mode(render));
    };
    let allowed: &[(&str, Mode)] = match render {
        Render::ObjectsTable => &[("toon", Mode::Toon), ("markdown", Mode::Markdown)],
        Render::MatrixTable => &[("plain", Mode::Plain), ("markdown", Mode::Markdown)],
        Render::DictTable => &[("lines", Mode::Lines), ("markdown", Mode::Markdown)],
        _ => &[],
    };
    allowed
        .iter()
        .find(|(n, _)| *n == m.trim())
        .map(|(_, v)| *v)
        .ok_or_else(|| {
            if allowed.is_empty() {
                format!("render {} takes no 'mode'", render.name())
            } else {
                let names: Vec<&str> = allowed.iter().map(|(n, _)| *n).collect();
                format!(
                    "mode '{m}' is not valid for {} (expected {})",
                    render.name(),
                    names.join(" or ")
                )
            }
        })
}

/// Compiles a standard from YAML. `tool` names the template for the parser.
pub fn compile(yaml: &str, origin: &str, tool: &str) -> Result<Standard, String> {
    let raw: RawStandard = serde_yaml::from_str(yaml).map_err(|e| format!("YAML: {e}"))?;
    compile_raw(raw, origin, tool)
}

pub fn compile_raw(raw: RawStandard, origin: &str, tool: &str) -> Result<Standard, String> {
    if let Some(v) = raw.version {
        if v != 2 && v != 3 {
            return Err(format!(
                "unsupported version {v} (expected 3; 2 is accepted)"
            ));
        }
    }
    let render = Render::parse(raw.render.as_deref().unwrap_or("passthrough"))?;
    let mode = parse_mode(render, raw.mode.as_deref())?;
    let declared = match raw.loss.as_deref() {
        Some(l) => Loss::parse(l)?,
        None => Loss::Lossless,
    };
    let pre = match &raw.pre {
        Some(list) => parse_pre_list(list)?,
        None => Vec::new(),
    };
    let post = match &raw.post {
        Some(list) => parse_post_list(list)?,
        None => Vec::new(),
    };
    check_post_for_render(&post, render)?;
    let mut formats = Vec::new();
    if let Some(f) = &raw.formats {
        for (k, v) in f {
            formats.push((k.clone(), ValueFormat::parse(v)?));
        }
    }
    if render == Render::Expression {
        let f = raw
            .format
            .as_deref()
            .ok_or("render expression needs 'format'")?;
        crate::render::parse_format(f)?;
    }
    if let Some(r) = &raw.root {
        if crate::json::split_path(r).is_none() {
            return Err(format!("bad root path '{r}'"));
        }
    }
    let template = if render == Render::Template {
        Some(compile_template(&raw, tool, origin)?)
    } else {
        None
    };
    Ok(Standard {
        raw,
        render,
        mode,
        declared,
        pre,
        post,
        formats,
        template,
        origin: origin.to_string(),
    })
}

fn compile_template(
    raw: &RawStandard,
    tool: &str,
    origin: &str,
) -> Result<CompiledTemplate, String> {
    let rules = raw.rules.clone().ok_or("render template needs 'rules'")?;
    let mut m = serde_yaml::Mapping::new();
    m.insert(Y::from("version"), Y::from(1));
    let name = raw.name.clone().unwrap_or_else(|| tool.to_string());
    m.insert(
        Y::from("name"),
        Y::from(if name.trim().is_empty() {
            "tool".to_string()
        } else {
            name
        }),
    );
    if let Some(input) = &raw.input {
        m.insert(Y::from("input"), input.clone());
    }
    m.insert(Y::from("rules"), rules);
    let text = serde_yaml::to_string(&Y::Mapping(m)).map_err(|e| e.to_string())?;
    compile_yaml_str(&text, &PathBuf::from(origin), None, None)
        .map_err(|e| format!("template: {}", e.message))
}

fn check_post_for_render(post: &[PostOp], render: Render) -> Result<(), String> {
    if !render.is_table() {
        return Ok(());
    }
    for op in post {
        match op {
            PostOp::TruncateChars(_) => {
                return Err(format!(
                    "truncate_chars would cut {} rows mid-row (I7): use max_cell_chars",
                    render.name()
                ));
            }
            PostOp::TruncateLines(_) => {
                return Err(format!(
                    "truncate_lines would cut {} mid-table (I7): use rows / max_rows or pre limit_items",
                    render.name()
                ));
            }
            _ => {}
        }
    }
    Ok(())
}

/// A chain entry is `name` or `{name: value}`.
fn entry(y: &Y) -> Result<(String, Y), String> {
    match y {
        Y::String(s) => Ok((s.trim().to_string(), Y::Bool(true))),
        Y::Mapping(m) if m.len() == 1 => {
            let (k, v) = m.iter().next().expect("one entry");
            let k = k.as_str().ok_or("operation name must be a string")?;
            Ok((k.trim().to_string(), v.clone()))
        }
        _ => Err("each operation is `name` or `{name: value}`".into()),
    }
}

const PRE_NAMES: [&str; 5] = ["unique", "sort", "sort_desc", "limit_items", "select"];
const POST_NAMES: [&str; 9] = [
    "join",
    "replace",
    "truncate_chars",
    "truncate_lines",
    "mask",
    "drop_lines",
    "keep_lines",
    "thousands",
    "bytes",
];

fn yaml_usize(v: &Y, op: &str) -> Result<usize, String> {
    v.as_u64()
        .map(|n| n as usize)
        .ok_or_else(|| format!("{op} needs a whole number"))
}

fn yaml_strings(v: &Y, op: &str) -> Result<Vec<String>, String> {
    match v {
        Y::String(s) => Ok(s
            .split(',')
            .map(|x| x.trim().to_string())
            .filter(|x| !x.is_empty())
            .collect()),
        Y::Sequence(seq) => seq
            .iter()
            .map(|x| {
                x.as_str()
                    .map(str::to_string)
                    .ok_or_else(|| format!("{op} takes strings"))
            })
            .collect(),
        _ => Err(format!("{op} takes a string or a list of strings")),
    }
}

pub fn parse_pre_list(list: &[Y]) -> Result<Vec<PreOp>, String> {
    list.iter().map(parse_pre).collect()
}

pub fn parse_post_list(list: &[Y]) -> Result<Vec<PostOp>, String> {
    list.iter().map(parse_post).collect()
}

fn parse_pre(y: &Y) -> Result<PreOp, String> {
    let (name, v) = entry(y)?;
    let sort_key = |v: &Y| match v {
        Y::Bool(_) | Y::Null => Ok(None),
        Y::String(s) => Ok(Some(s.clone())),
        Y::Mapping(m) => Ok(m
            .get(Y::from("by"))
            .and_then(|b| b.as_str())
            .map(str::to_string)),
        _ => Err("sort takes true, a key, or {by: key}".to_string()),
    };
    match name.as_str() {
        "unique" => Ok(PreOp::Unique),
        "sort" => Ok(PreOp::Sort {
            desc: false,
            by: sort_key(&v)?,
        }),
        "sort_desc" => Ok(PreOp::Sort {
            desc: true,
            by: sort_key(&v)?,
        }),
        "limit_items" => Ok(PreOp::Limit(yaml_usize(&v, "limit_items")?)),
        "select" => Ok(PreOp::Select(yaml_strings(&v, "select")?)),
        n if POST_NAMES.contains(&n) => Err(format!(
            "'{n}' is a POST (text) operation, not valid in pre (D7)"
        )),
        n => Err(format!(
            "unknown pre operation '{n}' (expected {})",
            PRE_NAMES.join(", ")
        )),
    }
}

fn build_regex(p: &str) -> Result<Regex, String> {
    RegexBuilder::new(p)
        .size_limit(1 << 20)
        .build()
        .map_err(|e| format!("regex '{p}': {e}"))
}

fn parse_post(y: &Y) -> Result<PostOp, String> {
    let (name, v) = entry(y)?;
    match name.as_str() {
        "join" => Ok(PostOp::Join(match &v {
            Y::Bool(true) => " ".into(),
            Y::String(s) => s.clone(),
            _ => return Err("join takes a separator string".into()),
        })),
        "replace" => {
            let m = v.as_mapping().ok_or("replace takes {from, to, regex?}")?;
            let from = m
                .get(Y::from("from"))
                .and_then(|x| x.as_str())
                .ok_or("replace needs 'from'")?;
            let to = m
                .get(Y::from("to"))
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string();
            let is_re = m
                .get(Y::from("regex"))
                .and_then(|x| x.as_bool())
                .unwrap_or(false);
            let re = build_regex(&if is_re {
                from.to_string()
            } else {
                regex::escape(from)
            })?;
            Ok(PostOp::Replace {
                re,
                to,
                literal: !is_re,
            })
        }
        "truncate_chars" => Ok(PostOp::TruncateChars(yaml_usize(&v, "truncate_chars")?)),
        "truncate_lines" => Ok(PostOp::TruncateLines(yaml_usize(&v, "truncate_lines")?)),
        "mask" => Ok(PostOp::Mask(mask_patterns(&v)?)),
        "drop_lines" => Ok(PostOp::DropLines(build_regex(
            v.as_str().ok_or("drop_lines takes a regex")?,
        )?)),
        "keep_lines" => Ok(PostOp::KeepLines(build_regex(
            v.as_str().ok_or("keep_lines takes a regex")?,
        )?)),
        "thousands" | "bytes" => Err(format!(
            "'{name}' formats values, not text: use `formats: {{<key>: {name}}}` (or \"*\" for every value)"
        )),
        n if PRE_NAMES.contains(&n) => Err(format!(
            "'{n}' is a PRE (structural) operation, not valid in post (D7)"
        )),
        n => Err(format!(
            "unknown post operation '{n}' (expected join, replace, truncate_chars, truncate_lines, mask, drop_lines, keep_lines)"
        )),
    }
}

/// Built-in secret patterns. A `keep` group survives the mask.
pub const MASKS: [(&str, &str); 8] = [
    (
        "private_key",
        r"-----BEGIN [A-Z ]*PRIVATE KEY-----[\s\S]*?-----END [A-Z ]*PRIVATE KEY-----",
    ),
    (
        "jwt",
        r"\beyJ[A-Za-z0-9_-]{8,}\.[A-Za-z0-9_-]{8,}\.[A-Za-z0-9_-]{8,}",
    ),
    (
        "bearer",
        r"(?P<keep>(?i:bearer)\s+)[A-Za-z0-9._~+/-]{16,}=*",
    ),
    (
        "api_key",
        r"\b(?:sk|pk|rk)-[A-Za-z0-9_-]{16,}|\bgh[pousr]_[A-Za-z0-9]{20,}|\bAKIA[0-9A-Z]{16}\b|\bxox[baprs]-[A-Za-z0-9-]{10,}|\bAIza[0-9A-Za-z_-]{30,}",
    ),
    (
        "password",
        r#"(?P<keep>(?i:password|passwd|pwd)\s*[=:]\s*["']?)[^\s"',;]{3,}"#,
    ),
    (
        "token",
        r#"(?P<keep>(?i:token|secret|api[_-]?key|access[_-]?key|client[_-]?secret)["']?\s*[=:]\s*["']?)[A-Za-z0-9._~+/-]{8,}"#,
    ),
    ("aws", r"\bAKIA[0-9A-Z]{16}\b"),
    (
        "email",
        r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}\b",
    ),
];

pub fn builtin_masks(include_email: bool) -> Vec<(Regex, String)> {
    MASKS
        .iter()
        .filter(|(n, _)| include_email || *n != "email")
        .map(|(_, p)| mask_entry(p).expect("builtin mask"))
        .collect()
}

fn mask_entry(p: &str) -> Result<(Regex, String), String> {
    let re = build_regex(p)?;
    let rep = if re.capture_names().any(|n| n == Some("keep")) {
        "${keep}***".to_string()
    } else {
        "***".to_string()
    };
    Ok((re, rep))
}

fn mask_patterns(v: &Y) -> Result<Vec<(Regex, String)>, String> {
    match v {
        Y::Bool(true) => Ok(builtin_masks(false)),
        _ => {
            let mut out = Vec::new();
            for name in yaml_strings(v, "mask")? {
                match MASKS.iter().find(|(n, _)| *n == name) {
                    Some((_, p)) => out.push(mask_entry(p)?),
                    None => out.push(mask_entry(&name)?),
                }
            }
            Ok(out)
        }
    }
}

/// Converts a JSON chain (from a `_pre` / `_post` override) to YAML values.
pub fn json_to_yaml(j: &J) -> Y {
    match j {
        J::Null => Y::Null,
        J::Bool(b) => Y::Bool(*b),
        J::Num(n) => serde_yaml::from_str(n).unwrap_or(Y::String(n.clone())),
        J::Str(s) => Y::String(s.clone()),
        J::Arr(a) => Y::Sequence(a.iter().map(json_to_yaml).collect()),
        J::Obj(m) => {
            let mut map = serde_yaml::Mapping::new();
            for (k, v) in m {
                map.insert(Y::String(k.clone()), json_to_yaml(v));
            }
            Y::Mapping(map)
        }
    }
}

/// File-name stem of a standard: `<server>.<tool>`, unsafe characters as `_`.
pub fn stem(server: &str, tool: &str) -> String {
    format!("{}.{}", sanitize(server), sanitize(tool))
}

pub fn sanitize(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

type Cached = (Option<SystemTime>, Arc<Result<Standard, String>>);

/// The standards directory with a modification-time cache: an edited file
/// applies from the next call, an unchanged one is not re-read.
pub struct Store {
    pub home: PathBuf,
    cache: Mutex<HashMap<PathBuf, Cached>>,
}

impl Store {
    pub fn new(home: PathBuf) -> Store {
        Store {
            home,
            cache: Mutex::new(HashMap::new()),
        }
    }

    pub fn standards_dir(&self) -> PathBuf {
        self.home.join("standards")
    }

    pub fn fixtures_dir(&self) -> PathBuf {
        self.home.join("fixtures")
    }

    pub fn tool_file(&self, server: &str, tool: &str) -> PathBuf {
        self.standards_dir()
            .join(format!("{}.yml", stem(server, tool)))
    }

    fn existing(base: PathBuf) -> Option<PathBuf> {
        if base.is_file() {
            return Some(base);
        }
        let alt = base.with_extension("yaml");
        alt.is_file().then_some(alt)
    }

    pub fn tool(&self, server: &str, tool: &str) -> Option<Arc<Result<Standard, String>>> {
        let path = Self::existing(self.tool_file(server, tool))?;
        Some(self.load(&path, tool))
    }

    pub fn family(&self, family: &str, tool: &str) -> Option<Arc<Result<Standard, String>>> {
        let base = self
            .standards_dir()
            .join("family")
            .join(format!("{}.yml", sanitize(family)));
        let path = Self::existing(base)?;
        Some(self.load(&path, tool))
    }

    pub fn load(&self, path: &Path, tool: &str) -> Arc<Result<Standard, String>> {
        let mtime = std::fs::metadata(path).and_then(|m| m.modified()).ok();
        // Family standards are shared by tools; the template name is cosmetic.
        let key = path.to_path_buf();
        if let Some((t, s)) = self.cache.lock().expect("cache").get(&key) {
            if *t == mtime && mtime.is_some() {
                return s.clone();
            }
        }
        let origin = path.display().to_string();
        let res = std::fs::read_to_string(path)
            .map_err(|e| format!("cannot read: {e}"))
            .and_then(|text| compile(&text, &origin, tool));
        let res = Arc::new(res);
        self.cache
            .lock()
            .expect("cache")
            .insert(key, (mtime, res.clone()));
        res
    }

    pub fn invalidate(&self, path: &Path) {
        self.cache.lock().expect("cache").remove(path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phases_are_checked() {
        let e = compile("render: log\npre: [join: ', ']", "t", "t").unwrap_err();
        assert!(e.contains("POST"), "{e}");
        let e = compile("render: log\npost: [unique]", "t", "t").unwrap_err();
        assert!(e.contains("PRE"), "{e}");
        let e = compile("render: objects_table\npost: [truncate_lines: 5]", "t", "t").unwrap_err();
        assert!(e.contains("I7"), "{e}");
        let e = compile("post: [thousands: true]", "t", "t").unwrap_err();
        assert!(e.contains("formats"), "{e}");
    }

    #[test]
    fn rev2_file_is_accepted() {
        let s = compile(
            "version: 2\nname: add\nserver: cube_v5\nloss: lossless\nrender: expression\nformat: \"{a} + {b} = {result}\"\nfamily: cube_arith\n",
            "t",
            "add",
        )
        .unwrap();
        assert_eq!(s.render, Render::Expression);
    }

    #[test]
    fn unknown_fields_are_rejected() {
        assert!(compile("render: log\nmax_itemz: 3", "t", "t").is_err());
    }

    #[test]
    fn masks_compile() {
        assert_eq!(builtin_masks(true).len(), MASKS.len());
    }
}
