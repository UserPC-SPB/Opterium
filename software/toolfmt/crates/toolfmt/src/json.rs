//! Order- and number-preserving JSON.
//!
//! Tool outputs are reformatted, never reinterpreted: a number keeps the
//! exact text the upstream wrote (`5.0` stays `5.0`, a 40-digit integer keeps
//! every digit), object keys keep their order, duplicate keys survive. The
//! gateway also forwards call arguments through this type, so a big integer
//! argument reaches the upstream tool unchanged.

use std::cmp::Ordering;
use std::fmt::Write as _;

#[derive(Clone, Debug, PartialEq)]
pub enum J {
    Null,
    Bool(bool),
    /// A number exactly as written in the source (validated JSON grammar).
    Num(String),
    Str(String),
    Arr(Vec<J>),
    Obj(Vec<(String, J)>),
}

const MAX_DEPTH: usize = 512;

impl J {
    pub fn parse(text: &str) -> Result<J, String> {
        let mut p = Parser {
            b: text.as_bytes(),
            s: text,
            i: 0,
            depth: 0,
        };
        p.ws();
        let v = p.value()?;
        p.ws();
        if p.i != p.b.len() {
            return Err(p.err("trailing data"));
        }
        Ok(v)
    }

    /// Parses `text` when it looks like a JSON document (object or array).
    pub fn parse_doc(text: &str) -> Option<J> {
        let t = text.trim();
        if t.starts_with('{') || t.starts_with('[') {
            J::parse(t).ok()
        } else {
            None
        }
    }

    pub fn str(s: impl Into<String>) -> J {
        J::Str(s.into())
    }

    pub fn int<T: Into<i128>>(n: T) -> J {
        J::Num(n.into().to_string())
    }

    pub fn obj() -> J {
        J::Obj(Vec::new())
    }

    /// Builder: sets `k` and returns the object.
    pub fn with(mut self, k: &str, v: J) -> J {
        self.set(k, v);
        self
    }

    pub fn get(&self, k: &str) -> Option<&J> {
        match self {
            J::Obj(m) => m.iter().find(|(kk, _)| kk == k).map(|(_, v)| v),
            _ => None,
        }
    }

    pub fn get_mut(&mut self, k: &str) -> Option<&mut J> {
        match self {
            J::Obj(m) => m.iter_mut().find(|(kk, _)| kk == k).map(|(_, v)| v),
            _ => None,
        }
    }

    pub fn set(&mut self, k: &str, v: J) {
        if let J::Obj(m) = self {
            if let Some(e) = m.iter_mut().find(|(kk, _)| kk == k) {
                e.1 = v;
            } else {
                m.push((k.to_string(), v));
            }
        }
    }

    pub fn remove(&mut self, k: &str) -> Option<J> {
        match self {
            J::Obj(m) => {
                let pos = m.iter().position(|(kk, _)| kk == k)?;
                Some(m.remove(pos).1)
            }
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            J::Str(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_arr(&self) -> Option<&Vec<J>> {
        match self {
            J::Arr(a) => Some(a),
            _ => None,
        }
    }

    pub fn as_obj(&self) -> Option<&Vec<(String, J)>> {
        match self {
            J::Obj(m) => Some(m),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            J::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_u64(&self) -> Option<u64> {
        match self {
            J::Num(s) => s.parse().ok(),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            J::Num(s) => s.parse().ok(),
            _ => None,
        }
    }

    pub fn is_scalar(&self) -> bool {
        !matches!(self, J::Arr(_) | J::Obj(_))
    }

    pub fn to_json(&self) -> String {
        let mut out = String::new();
        self.write_json(&mut out);
        out
    }

    pub fn write_json(&self, out: &mut String) {
        match self {
            J::Null => out.push_str("null"),
            J::Bool(true) => out.push_str("true"),
            J::Bool(false) => out.push_str("false"),
            J::Num(n) => out.push_str(n),
            J::Str(s) => write_str(s, out),
            J::Arr(a) => {
                out.push('[');
                for (i, v) in a.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    v.write_json(out);
                }
                out.push(']');
            }
            J::Obj(m) => {
                out.push('{');
                for (i, (k, v)) in m.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    write_str(k, out);
                    out.push(':');
                    v.write_json(out);
                }
                out.push('}');
            }
        }
    }

    /// Writes compact JSON straight to `w`: a big message is never built as
    /// one string first.
    pub fn write_io(&self, w: &mut dyn std::io::Write) -> std::io::Result<()> {
        match self {
            J::Null => w.write_all(b"null"),
            J::Bool(true) => w.write_all(b"true"),
            J::Bool(false) => w.write_all(b"false"),
            J::Num(n) => w.write_all(n.as_bytes()),
            J::Str(s) => write_str_io(s, w),
            J::Arr(a) => {
                w.write_all(b"[")?;
                for (i, v) in a.iter().enumerate() {
                    if i > 0 {
                        w.write_all(b",")?;
                    }
                    v.write_io(w)?;
                }
                w.write_all(b"]")
            }
            J::Obj(m) => {
                w.write_all(b"{")?;
                for (i, (k, v)) in m.iter().enumerate() {
                    if i > 0 {
                        w.write_all(b",")?;
                    }
                    write_str_io(k, w)?;
                    w.write_all(b":")?;
                    v.write_io(w)?;
                }
                w.write_all(b"}")
            }
        }
    }

    /// The length in chars of `to_json()`, without building it.
    pub fn json_chars(&self) -> usize {
        fn str_chars(s: &str) -> usize {
            2 + s
                .chars()
                .map(|c| match c {
                    '"' | '\\' | '\n' | '\r' | '\t' | '\u{08}' | '\u{0c}' => 2,
                    c if (c as u32) < 0x20 => 6,
                    _ => 1,
                })
                .sum::<usize>()
        }
        match self {
            J::Null => 4,
            J::Bool(true) => 4,
            J::Bool(false) => 5,
            J::Num(n) => n.len(),
            J::Str(s) => str_chars(s),
            J::Arr(a) => 2 + a.len().saturating_sub(1) + a.iter().map(J::json_chars).sum::<usize>(),
            J::Obj(m) => {
                2 + m.len().saturating_sub(1)
                    + m.iter()
                        .map(|(k, v)| str_chars(k) + 1 + v.json_chars())
                        .sum::<usize>()
            }
        }
    }

    /// Indented JSON (two spaces), as many tools emit it.
    pub fn to_pretty(&self) -> String {
        fn go(v: &J, ind: usize, out: &mut String) {
            let pad = |n: usize, out: &mut String| out.extend(std::iter::repeat_n(' ', n));
            match v {
                J::Arr(a) if !a.is_empty() => {
                    out.push_str("[\n");
                    for (i, x) in a.iter().enumerate() {
                        pad(ind + 2, out);
                        go(x, ind + 2, out);
                        out.push_str(if i + 1 < a.len() { ",\n" } else { "\n" });
                    }
                    pad(ind, out);
                    out.push(']');
                }
                J::Obj(m) if !m.is_empty() => {
                    out.push_str("{\n");
                    for (i, (k, x)) in m.iter().enumerate() {
                        pad(ind + 2, out);
                        write_str(k, out);
                        out.push_str(": ");
                        go(x, ind + 2, out);
                        out.push_str(if i + 1 < m.len() { ",\n" } else { "\n" });
                    }
                    pad(ind, out);
                    out.push('}');
                }
                other => other.write_json(out),
            }
        }
        let mut out = String::new();
        go(self, 0, &mut out);
        out
    }

    /// Plain text of a value: strings unquoted, numbers as written,
    /// containers as compact JSON.
    pub fn text(&self) -> String {
        match self {
            J::Str(s) => s.clone(),
            J::Num(n) => n.clone(),
            other => other.to_json(),
        }
    }

    /// Looks up a path: `a.b`, `$.a.b`, `items[0].name`, `$[2]`.
    pub fn path(&self, path: &str) -> Option<&J> {
        let mut cur = self;
        for seg in split_path(path)? {
            cur = match seg {
                Seg::Key(k) => cur.get(k)?,
                Seg::Index(i) => cur.as_arr()?.get(i)?,
            };
        }
        Some(cur)
    }

    fn rank(&self) -> u8 {
        match self {
            J::Null => 0,
            J::Bool(_) => 1,
            J::Num(_) => 2,
            J::Str(_) => 3,
            J::Arr(_) => 4,
            J::Obj(_) => 5,
        }
    }
}

pub enum Seg<'a> {
    Key(&'a str),
    Index(usize),
}

/// Splits a path into segments; `None` when it is malformed.
pub fn split_path(path: &str) -> Option<Vec<Seg<'_>>> {
    let p = path.trim();
    let p = p.strip_prefix('$').unwrap_or(p);
    let p = p.strip_prefix('.').unwrap_or(p);
    let mut segs = Vec::new();
    if p.is_empty() {
        return Some(segs);
    }
    for part in p.split('.') {
        let (key, mut rest) = match part.find('[') {
            Some(i) => (&part[..i], &part[i..]),
            None => (part, ""),
        };
        if !key.is_empty() {
            segs.push(Seg::Key(key));
        } else if rest.is_empty() {
            return None;
        }
        while !rest.is_empty() {
            let close = rest.find(']')?;
            let idx: usize = rest[1..close].trim().parse().ok()?;
            segs.push(Seg::Index(idx));
            rest = &rest[close + 1..];
            if !rest.is_empty() && !rest.starts_with('[') {
                return None;
            }
        }
    }
    Some(segs)
}

pub fn write_str(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0c}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

fn write_str_io(s: &str, w: &mut dyn std::io::Write) -> std::io::Result<()> {
    w.write_all(b"\"")?;
    let bytes = s.as_bytes();
    let mut start = 0;
    for (i, &b) in bytes.iter().enumerate() {
        let esc: Option<&[u8]> = match b {
            b'"' => Some(b"\\\""),
            b'\\' => Some(b"\\\\"),
            b'\n' => Some(b"\\n"),
            b'\r' => Some(b"\\r"),
            b'\t' => Some(b"\\t"),
            0x08 => Some(b"\\b"),
            0x0c => Some(b"\\f"),
            _ => None,
        };
        if esc.is_some() || b < 0x20 {
            w.write_all(&bytes[start..i])?;
            match esc {
                Some(e) => w.write_all(e)?,
                None => write!(w, "\\u{:04x}", b)?,
            }
            start = i + 1;
        }
    }
    w.write_all(&bytes[start..])?;
    w.write_all(b"\"")
}

pub fn quote(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    write_str(s, &mut out);
    out
}

/// Total order used by `sort`: null < bool < number < string < array <
/// object. Integers compare exactly at any length; strings by code point.
pub fn cmp(a: &J, b: &J) -> Ordering {
    match (a, b) {
        (J::Bool(x), J::Bool(y)) => x.cmp(y),
        (J::Num(x), J::Num(y)) => cmp_num(x, y),
        (J::Str(x), J::Str(y)) => x.cmp(y),
        (J::Arr(x), J::Arr(y)) => {
            for (p, q) in x.iter().zip(y.iter()) {
                let o = cmp(p, q);
                if o != Ordering::Equal {
                    return o;
                }
            }
            x.len().cmp(&y.len())
        }
        (J::Obj(_), J::Obj(_)) => a.to_json().cmp(&b.to_json()),
        _ => a.rank().cmp(&b.rank()),
    }
}

fn is_int(s: &str) -> bool {
    !s.contains(['.', 'e', 'E'])
}

pub fn cmp_num(x: &str, y: &str) -> Ordering {
    if is_int(x) && is_int(y) {
        let (nx, dx) = x.strip_prefix('-').map_or((false, x), |d| (true, d));
        let (ny, dy) = y.strip_prefix('-').map_or((false, y), |d| (true, d));
        let zero = |d: &str| d.bytes().all(|b| b == b'0');
        let (nx, ny) = (nx && !zero(dx), ny && !zero(dy));
        let mag = dx.len().cmp(&dy.len()).then_with(|| dx.cmp(dy));
        return match (nx, ny) {
            (false, false) => mag,
            (true, true) => mag.reverse(),
            (true, false) => Ordering::Less,
            (false, true) => Ordering::Greater,
        };
    }
    let fx: f64 = x.parse().unwrap_or(0.0);
    let fy: f64 = y.parse().unwrap_or(0.0);
    fx.partial_cmp(&fy).unwrap_or(Ordering::Equal)
}

struct Parser<'a> {
    b: &'a [u8],
    s: &'a str,
    i: usize,
    depth: usize,
}

impl Parser<'_> {
    fn peek(&self) -> Option<u8> {
        self.b.get(self.i).copied()
    }

    fn ws(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            self.i += 1;
        }
    }

    fn err(&self, m: &str) -> String {
        format!("invalid JSON: {} at byte {}", m, self.i)
    }

    fn value(&mut self) -> Result<J, String> {
        match self.peek() {
            None => Err(self.err("unexpected end")),
            Some(b'{') => self.object(),
            Some(b'[') => self.array(),
            Some(b'"') => Ok(J::Str(self.string()?)),
            Some(b't') => self.lit("true", J::Bool(true)),
            Some(b'f') => self.lit("false", J::Bool(false)),
            Some(b'n') => self.lit("null", J::Null),
            Some(b'-' | b'0'..=b'9') => self.number(),
            Some(_) => Err(self.err("unexpected character")),
        }
    }

    fn lit(&mut self, word: &str, v: J) -> Result<J, String> {
        if self.b[self.i..].starts_with(word.as_bytes()) {
            self.i += word.len();
            Ok(v)
        } else {
            Err(self.err("invalid literal"))
        }
    }

    fn digits(&mut self) -> usize {
        let start = self.i;
        while matches!(self.peek(), Some(b'0'..=b'9')) {
            self.i += 1;
        }
        self.i - start
    }

    fn number(&mut self) -> Result<J, String> {
        let start = self.i;
        if self.peek() == Some(b'-') {
            self.i += 1;
        }
        match self.peek() {
            Some(b'0') => self.i += 1,
            Some(b'1'..=b'9') => {
                self.digits();
            }
            _ => return Err(self.err("invalid number")),
        }
        if self.peek() == Some(b'.') {
            self.i += 1;
            if self.digits() == 0 {
                return Err(self.err("invalid number"));
            }
        }
        if matches!(self.peek(), Some(b'e' | b'E')) {
            self.i += 1;
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.i += 1;
            }
            if self.digits() == 0 {
                return Err(self.err("invalid number"));
            }
        }
        Ok(J::Num(self.s[start..self.i].to_string()))
    }

    fn hex4(&mut self) -> Result<u32, String> {
        let h = self
            .s
            .get(self.i..self.i + 4)
            .ok_or_else(|| self.err("short \\u escape"))?;
        let v = u32::from_str_radix(h, 16).map_err(|_| self.err("bad \\u escape"))?;
        self.i += 4;
        Ok(v)
    }

    fn string(&mut self) -> Result<String, String> {
        self.i += 1;
        let mut out = String::new();
        loop {
            let run = self.i;
            while let Some(c) = self.peek() {
                if c == b'"' || c == b'\\' {
                    break;
                }
                self.i += 1;
            }
            // '"' and '\\' are ASCII, so the run ends on a char boundary.
            out.push_str(&self.s[run..self.i]);
            match self.peek() {
                None => return Err(self.err("unterminated string")),
                Some(b'"') => {
                    self.i += 1;
                    return Ok(out);
                }
                Some(_) => {
                    self.i += 1;
                    let c = self.peek().ok_or_else(|| self.err("unterminated escape"))?;
                    self.i += 1;
                    match c {
                        b'"' => out.push('"'),
                        b'\\' => out.push('\\'),
                        b'/' => out.push('/'),
                        b'b' => out.push('\u{08}'),
                        b'f' => out.push('\u{0c}'),
                        b'n' => out.push('\n'),
                        b'r' => out.push('\r'),
                        b't' => out.push('\t'),
                        b'u' => {
                            let hi = self.hex4()?;
                            if (0xD800..0xDC00).contains(&hi) {
                                if self.b[self.i..].starts_with(b"\\u") {
                                    self.i += 2;
                                    let lo = self.hex4()?;
                                    if (0xDC00..0xE000).contains(&lo) {
                                        let cp = 0x10000 + ((hi - 0xD800) << 10) + (lo - 0xDC00);
                                        out.push(char::from_u32(cp).unwrap_or('\u{FFFD}'));
                                    } else {
                                        out.push('\u{FFFD}');
                                        out.push(char::from_u32(lo).unwrap_or('\u{FFFD}'));
                                    }
                                } else {
                                    out.push('\u{FFFD}');
                                }
                            } else {
                                out.push(char::from_u32(hi).unwrap_or('\u{FFFD}'));
                            }
                        }
                        _ => return Err(self.err("invalid escape")),
                    }
                }
            }
        }
    }

    fn enter(&mut self) -> Result<(), String> {
        self.depth += 1;
        if self.depth > MAX_DEPTH {
            return Err(self.err("nesting too deep"));
        }
        self.i += 1;
        self.ws();
        Ok(())
    }

    fn object(&mut self) -> Result<J, String> {
        self.enter()?;
        let mut m = Vec::new();
        if self.peek() == Some(b'}') {
            self.i += 1;
            self.depth -= 1;
            return Ok(J::Obj(m));
        }
        loop {
            self.ws();
            if self.peek() != Some(b'"') {
                return Err(self.err("expected key"));
            }
            let k = self.string()?;
            self.ws();
            if self.peek() != Some(b':') {
                return Err(self.err("expected ':'"));
            }
            self.i += 1;
            self.ws();
            let v = self.value()?;
            m.push((k, v));
            self.ws();
            match self.peek() {
                Some(b',') => self.i += 1,
                Some(b'}') => {
                    self.i += 1;
                    break;
                }
                _ => return Err(self.err("expected ',' or '}'")),
            }
        }
        self.depth -= 1;
        Ok(J::Obj(m))
    }

    fn array(&mut self) -> Result<J, String> {
        self.enter()?;
        let mut a = Vec::new();
        if self.peek() == Some(b']') {
            self.i += 1;
            self.depth -= 1;
            return Ok(J::Arr(a));
        }
        loop {
            self.ws();
            a.push(self.value()?);
            self.ws();
            match self.peek() {
                Some(b',') => self.i += 1,
                Some(b']') => {
                    self.i += 1;
                    break;
                }
                _ => return Err(self.err("expected ',' or ']'")),
            }
        }
        self.depth -= 1;
        Ok(J::Arr(a))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_keep_their_text() {
        let j = J::parse(r#"{"a":5.0,"b":123456789012345678901234567890,"c":1e16}"#).unwrap();
        assert_eq!(
            j.to_json(),
            r#"{"a":5.0,"b":123456789012345678901234567890,"c":1e16}"#
        );
    }

    #[test]
    fn streamed_and_counted_like_to_json() {
        let j = J::parse(r#"{"a":"x\"y\\z\n\u0001é😀","b":[1,2.5,null,true,false,{}],"c":""}"#)
            .unwrap();
        let mut out = Vec::new();
        j.write_io(&mut out).unwrap();
        assert_eq!(String::from_utf8(out).unwrap(), j.to_json());
        assert_eq!(j.json_chars(), j.to_json().chars().count());
    }

    #[test]
    fn keys_keep_order_and_duplicates() {
        let j = J::parse(r#"{"z":1,"a":2,"z":3}"#).unwrap();
        assert_eq!(j.to_json(), r#"{"z":1,"a":2,"z":3}"#);
    }

    #[test]
    fn strings_round_trip() {
        let src = r#"["a\"b\\c\né😀/", "\t"]"#;
        let j = J::parse(src).unwrap();
        assert_eq!(j.as_arr().unwrap()[0].as_str().unwrap(), "a\"b\\c\né😀/");
        assert_eq!(J::parse(&j.to_json()).unwrap(), j);
    }

    #[test]
    fn rejects_garbage() {
        for bad in ["{", "[1,]", "01", "1.", "{\"a\" 1}", "[1] x", "tru"] {
            assert!(J::parse(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn deep_nesting_is_an_error_not_a_crash() {
        let s = "[".repeat(10_000);
        assert!(J::parse(&s).is_err());
    }

    #[test]
    fn exact_integer_order() {
        assert_eq!(
            cmp_num(
                "123456789012345678901234567891",
                "123456789012345678901234567890"
            ),
            Ordering::Greater
        );
        assert_eq!(cmp_num("-5", "3"), Ordering::Less);
        assert_eq!(cmp_num("-10", "-9"), Ordering::Less);
        assert_eq!(cmp_num("2.5", "10"), Ordering::Less);
    }

    #[test]
    fn paths() {
        let j = J::parse(r#"{"data":{"items":[{"n":1},{"n":2}]}}"#).unwrap();
        assert_eq!(j.path("$.data.items[1].n").unwrap().text(), "2");
        assert_eq!(j.path("data.items").unwrap().as_arr().unwrap().len(), 2);
        assert!(j.path("data.missing").is_none());
    }
}
