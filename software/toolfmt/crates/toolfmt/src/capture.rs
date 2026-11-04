//! Capture of real outputs (fixtures), payload extraction from MCP results,
//! and the audit log. Everything is written inside the home directory only
//! (I4): `fixtures/`, `audit.jsonl`.

use crate::json::J;
use crate::standard::{builtin_masks, stem};
use regex::Regex;
use std::borrow::Cow;
use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

/// The payload to format, taken from an MCP `tools/call` result.
pub enum Payload<'a> {
    Text(Cow<'a, str>),
    Json(Cow<'a, J>),
    None,
}

/// What a payload was made of: the parts of the result a formatted text
/// replaces. Everything else in the result is kept.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    /// `structuredContent`.
    Structured,
    /// These text blocks (indexes into `content`), every one of them.
    Texts(Vec<usize>),
    /// This image block, as a data URI.
    Image(usize),
    None,
}

/// `structuredContent` when present (typed, no text parsing), otherwise the
/// text blocks, otherwise an image block as a data URI.
pub fn payload(result: &J) -> Payload<'_> {
    payload_src(result).0
}

/// [`payload`] and the parts of the result it was taken from.
pub fn payload_src(result: &J) -> (Payload<'_>, Source) {
    if let Some(sc) = result
        .get("structuredContent")
        .filter(|v| !matches!(v, J::Null))
    {
        return (Payload::Json(Cow::Borrowed(sc)), Source::Structured);
    }
    let blocks = result.get("content").and_then(J::as_arr);
    let Some(blocks) = blocks else {
        return (Payload::None, Source::None);
    };
    let (idx, texts): (Vec<usize>, Vec<&str>) = blocks
        .iter()
        .enumerate()
        .filter(|(_, b)| b.get("type").and_then(J::as_str) == Some("text"))
        .filter_map(|(i, b)| Some((i, b.get("text").and_then(J::as_str)?)))
        .unzip();
    match texts.len() {
        1 => (Payload::Text(Cow::Borrowed(texts[0])), Source::Texts(idx)),
        0 => blocks
            .iter()
            .enumerate()
            .find(|(_, b)| b.get("type").and_then(J::as_str) == Some("image"))
            .and_then(|(i, b)| {
                let mime = b.get("mimeType").and_then(J::as_str).unwrap_or("image/png");
                let data = b.get("data").and_then(J::as_str)?;
                Some((
                    Payload::Text(Cow::Owned(format!("data:{mime};base64,{data}"))),
                    Source::Image(i),
                ))
            })
            .unwrap_or((Payload::None, Source::None)),
        _ => {
            let docs: Option<Vec<J>> = texts.iter().map(|t| J::parse_doc(t)).collect();
            let pl = match docs {
                Some(d) => Payload::Json(Cow::Owned(J::Arr(d))),
                None => Payload::Text(Cow::Owned(texts.join("\n"))),
            };
            (pl, Source::Texts(idx))
        }
    }
}

pub fn mask_json(j: &mut J, masks: &[(Regex, String)]) {
    match j {
        J::Str(s) => {
            for (re, rep) in masks {
                if re.is_match(s) {
                    *s = re.replace_all(s, rep.as_str()).into_owned();
                }
            }
        }
        J::Arr(a) => a.iter_mut().for_each(|x| mask_json(x, masks)),
        J::Obj(m) => {
            for (k, v) in m.iter_mut() {
                let lower = k.to_ascii_lowercase();
                // Pagination tokens are cursors, not secrets (I8).
                let cursor_like = ["page", "next", "cursor", "continuation"]
                    .iter()
                    .any(|s| lower.contains(s));
                let secret_key = !cursor_like
                    && [
                        "password",
                        "passwd",
                        "secret",
                        "token",
                        "api_key",
                        "apikey",
                        "authorization",
                    ]
                    .iter()
                    .any(|s| lower.contains(s));
                if secret_key && matches!(v, J::Str(_)) {
                    *v = J::str("***");
                } else {
                    mask_json(v, masks);
                }
            }
        }
        _ => {}
    }
}

pub fn fnv64(s: &str) -> String {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.bytes() {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x100000001b3);
    }
    format!("{h:016x}")
}

pub fn now_iso() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0) as i64;
    let days = secs.div_euclid(86400);
    let rem = secs.rem_euclid(86400);
    // Civil date from days since 1970-01-01 (Howard Hinnant's algorithm).
    let z = days + 719468;
    let era = z.div_euclid(146097);
    let doe = z.rem_euclid(146097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z",
        rem / 3600,
        rem % 3600 / 60,
        rem % 60
    )
}

/// Writes `text` to `path` through a temporary file, so a reader never
/// sees half a file.
pub fn write_atomic(path: &Path, text: &str) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, text)?;
    fs::rename(&tmp, path)
}

pub const MAX_CAPTURE_BYTES: usize = 2 << 20;

pub struct Captures {
    dir: PathBuf,
    limit: usize,
    masks: Vec<(Regex, String)>,
    hashes: Mutex<HashMap<String, Vec<String>>>,
}

pub struct Fixture {
    pub path: PathBuf,
    pub doc: J,
}

impl Fixture {
    pub fn args(&self) -> &J {
        static EMPTY: std::sync::OnceLock<J> = std::sync::OnceLock::new();
        self.doc
            .get("args")
            .or_else(|| self.doc.get("request").and_then(|r| r.get("args")))
            .unwrap_or_else(|| EMPTY.get_or_init(J::obj))
    }

    /// The field holding the request (flat, or under `request`).
    pub fn field(&self, k: &str) -> Option<&J> {
        self.doc
            .get(k)
            .or_else(|| self.doc.get("request").and_then(|r| r.get(k)))
    }

    pub fn payload(&self) -> Payload<'_> {
        if let Some(r) = self.doc.get("result") {
            return payload(r);
        }
        match self.field("data") {
            Some(J::Str(s)) => Payload::Text(Cow::Borrowed(s)),
            Some(J::Null) | None => Payload::None,
            Some(j) => Payload::Json(Cow::Borrowed(j)),
        }
    }

    pub fn baseline(&self) -> Option<(&str, &str)> {
        let b = self
            .doc
            .get("baseline")
            .or_else(|| self.doc.get("expected"))?;
        Some((b.get("text")?.as_str()?, b.get("used")?.as_str()?))
    }
}

/// All fixture files under `dir`, sorted by path.
pub fn walk_fixtures(dir: &Path) -> Vec<Fixture> {
    let mut files = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(rd) = fs::read_dir(&d) else { continue };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().is_some_and(|x| x == "json") {
                files.push(p);
            }
        }
    }
    files.sort();
    files
        .into_iter()
        .filter_map(|p| {
            let text = fs::read_to_string(&p).ok()?;
            let doc = J::parse(text.trim_start_matches('\u{feff}')).ok()?;
            Some(Fixture { path: p, doc })
        })
        .collect()
}

impl Captures {
    pub fn new(fixtures: PathBuf, limit: usize) -> Captures {
        Captures {
            dir: fixtures,
            limit,
            masks: builtin_masks(false),
            hashes: Mutex::new(HashMap::new()),
        }
    }

    pub fn tool_dir(&self, server: &str, tool: &str) -> PathBuf {
        self.dir.join(stem(server, tool))
    }

    pub fn list(&self, server: &str, tool: &str) -> Vec<Fixture> {
        walk_fixtures(&self.tool_dir(server, tool))
    }

    pub fn count(&self, server: &str, tool: &str) -> usize {
        let mut map = self.hashes.lock().expect("hashes");
        self.known(&mut map, server, tool).len()
    }

    fn known<'m>(
        &self,
        map: &'m mut HashMap<String, Vec<String>>,
        server: &str,
        tool: &str,
    ) -> &'m mut Vec<String> {
        map.entry(stem(server, tool)).or_insert_with(|| {
            self.list(server, tool)
                .iter()
                .map(|f| {
                    f.doc
                        .get("hash")
                        .map(J::text)
                        .unwrap_or_else(|| fnv64(&f.path.display().to_string()))
                })
                .collect()
        })
    }

    /// Stores a successful result as a fixture while the tool has fewer than
    /// `limit` distinct captures. Secrets are masked before writing. One
    /// lock covers check and write: parallel calls never share a file.
    pub fn maybe_capture(&self, server: &str, tool: &str, args: &J, result: &J) -> Option<PathBuf> {
        if self.limit == 0 {
            return None;
        }
        let mut map = self.hashes.lock().expect("hashes");
        let known = self.known(&mut map, server, tool);
        if known.len() >= self.limit {
            return None;
        }
        // Size first: a huge output is neither copied nor serialized.
        if result.json_chars() > MAX_CAPTURE_BYTES {
            return None;
        }
        let mut res = result.clone();
        mask_json(&mut res, &self.masks);
        let body = res.to_json();
        if body.len() > MAX_CAPTURE_BYTES {
            return None;
        }
        let hash = fnv64(&body);
        if known.contains(&hash) {
            return None;
        }
        let mut a = args.clone();
        mask_json(&mut a, &self.masks);
        let doc = J::obj()
            .with("server", J::str(server))
            .with("tool", J::str(tool))
            .with("captured", J::Str(now_iso()))
            .with("hash", J::Str(hash.clone()))
            .with("args", a)
            .with("result", res);
        let dir = self.tool_dir(server, tool);
        let mut n = known.len() + 1;
        let mut path = dir.join(format!("{n:02}.json"));
        while path.exists() {
            n += 1;
            path = dir.join(format!("{n:02}.json"));
        }
        write_atomic(&path, &doc.to_json()).ok()?;
        known.push(hash);
        Some(path)
    }
}

/// Records the baseline (`text`, `used`) of a fixture in place.
pub fn write_baseline(f: &Fixture, text: &str, used: &str) -> std::io::Result<()> {
    let mut doc = f.doc.clone();
    doc.remove("expected");
    doc.set(
        "baseline",
        J::obj()
            .with("used", J::str(used))
            .with("text", J::str(text)),
    );
    write_atomic(&f.path, &doc.to_json())
}

pub struct Audit {
    path: PathBuf,
    file: Mutex<Option<fs::File>>,
}

const AUDIT_MAX: u64 = 10 << 20;

impl Audit {
    pub fn new(home: &Path) -> Audit {
        Audit {
            path: home.join("audit.jsonl"),
            file: Mutex::new(None),
        }
    }

    pub fn record(&self, entry: &J) {
        let mut guard = self.file.lock().expect("audit");
        if guard.is_none() {
            if fs::metadata(&self.path)
                .map(|m| m.len() > AUDIT_MAX)
                .unwrap_or(false)
            {
                let _ = fs::rename(&self.path, self.path.with_extension("1.jsonl"));
            }
            if let Some(dir) = self.path.parent() {
                let _ = fs::create_dir_all(dir);
            }
            *guard = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&self.path)
                .ok();
        }
        if let Some(f) = guard.as_mut() {
            let mut line = entry.to_json();
            line.push('\n');
            let _ = f.write_all(line.as_bytes());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payload_prefers_structured_content() {
        let r = J::parse(
            r#"{"content":[{"type":"text","text":"{\"a\":1}"}],"structuredContent":{"a":1}}"#,
        )
        .unwrap();
        assert!(matches!(payload(&r), Payload::Json(_)));
        let r = J::parse(r#"{"content":[{"type":"text","text":"hi"}]}"#).unwrap();
        assert!(matches!(payload(&r), Payload::Text(t) if t == "hi"));
        let r = J::parse(
            r#"{"content":[{"type":"text","text":"[1]"},{"type":"text","text":"{\"b\":2}"}]}"#,
        )
        .unwrap();
        assert!(matches!(payload(&r), Payload::Json(j) if j.to_json() == r#"[[1],{"b":2}]"#));
        let r = J::parse(r#"{"content":[{"type":"image","mimeType":"image/gif","data":"R0lG"}]}"#)
            .unwrap();
        assert!(matches!(payload(&r), Payload::Text(t) if t == "data:image/gif;base64,R0lG"));
    }

    #[test]
    fn masking() {
        let mut j =
            J::parse(r#"{"api_key":"abc","note":"Bearer abcdefghijklmnopqrstuvwxyz","n":1}"#)
                .unwrap();
        mask_json(&mut j, &builtin_masks(false));
        assert_eq!(
            j.to_json(),
            r#"{"api_key":"***","note":"Bearer ***","n":1}"#
        );
    }

    #[test]
    fn parallel_captures_never_share_a_file() {
        let dir = std::env::temp_dir().join(format!("toolfmt-cap-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let caps = std::sync::Arc::new(Captures::new(dir.clone(), 5));
        let threads: Vec<_> = (0..8)
            .map(|i| {
                let c = caps.clone();
                std::thread::spawn(move || {
                    let r = J::parse(&format!(
                        r#"{{"content":[{{"type":"text","text":"out {i}"}}]}}"#
                    ))
                    .unwrap();
                    c.maybe_capture("s", "t", &J::obj(), &r);
                })
            })
            .collect();
        threads.into_iter().for_each(|t| t.join().unwrap());
        assert_eq!(walk_fixtures(&dir.join("s.t")).len(), 5);
        assert_eq!(caps.count("s", "t"), 5);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn iso_time_shape() {
        let t = now_iso();
        assert_eq!(t.len(), 20);
        assert!(t.starts_with("20") && t.ends_with('Z'));
    }
}
