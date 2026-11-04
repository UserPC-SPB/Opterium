//! DETECT: the shape of a raw payload.

use crate::json::J;
use regex::Regex;
use std::sync::OnceLock;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shape {
    /// Empty text, `[]` or `{}`.
    Empty,
    Scalar,
    JsonObj,
    JsonList,
    Matrix2d,
    NumList,
    StrList,
    /// A JSON array mixing element kinds.
    JsonMixed,
    Log,
    Image,
    Text,
}

impl Shape {
    pub fn name(self) -> &'static str {
        match self {
            Shape::Empty => "empty",
            Shape::Scalar => "scalar",
            Shape::JsonObj => "json_obj",
            Shape::JsonList => "json_list",
            Shape::Matrix2d => "matrix_2d",
            Shape::NumList => "num_list",
            Shape::StrList => "str_list",
            Shape::JsonMixed => "json_mixed",
            Shape::Log => "log",
            Shape::Image => "image",
            Shape::Text => "text",
        }
    }
}

/// `text` is the textual payload (the decoded string when the payload is a
/// JSON string); `json` the parsed document, when it is one.
pub fn detect(text: &str, json: Option<&J>) -> Shape {
    match json {
        Some(J::Str(_)) | None => detect_text(text),
        Some(j) => detect_json(j),
    }
}

pub fn detect_json(j: &J) -> Shape {
    match j {
        J::Obj(m) if m.is_empty() => Shape::Empty,
        J::Obj(_) => Shape::JsonObj,
        J::Arr(a) if a.is_empty() => Shape::Empty,
        J::Arr(a) => {
            if a.iter().all(|x| {
                matches!(x, J::Arr(r) if !r.is_empty() && r.iter().all(|c| matches!(c, J::Num(_))))
            }) {
                Shape::Matrix2d
            } else if a.iter().all(|x| matches!(x, J::Num(_))) {
                Shape::NumList
            } else if a.iter().all(|x| matches!(x, J::Str(_))) {
                Shape::StrList
            } else if a.iter().all(|x| matches!(x, J::Obj(_))) {
                Shape::JsonList
            } else {
                Shape::JsonMixed
            }
        }
        J::Str(s) => detect_text(s),
        _ => Shape::Scalar,
    }
}

fn detect_text(text: &str) -> Shape {
    let t = text.trim();
    if t.is_empty() {
        return Shape::Empty;
    }
    if is_image_ref(t) {
        return Shape::Image;
    }
    if !t.contains('\n') && t.chars().count() < 200 {
        return Shape::Scalar;
    }
    if is_log(t) {
        return Shape::Log;
    }
    Shape::Text
}

const IMAGE_EXT: [&str; 6] = [".png", ".jpg", ".jpeg", ".gif", ".webp", ".bmp"];

pub fn is_image_ref(t: &str) -> bool {
    if t.starts_with("data:image/") {
        return true;
    }
    if t.contains('\n') || t.len() > 1024 {
        return false;
    }
    let lower = t.to_ascii_lowercase();
    IMAGE_EXT.iter().any(|e| lower.ends_with(e))
}

fn level_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"(?:^|[\s\[(|<])(TRACE|DEBUG|INFO|NOTICE|WARN|WARNING|ERROR|ERR|FATAL|CRITICAL|SEVERE)(?:[\]):|>\s]|$)|\[(?i:(trace|debug|info|notice|warn|warning|error|err|fatal|critical))\]",
        )
        .expect("level regex")
    })
}

/// The normalized log level a line carries: FATAL, ERROR, WARN, INFO,
/// DEBUG or TRACE.
pub fn level_of(line: &str) -> Option<&'static str> {
    let c = level_re().captures(line)?;
    let raw = c.get(1).or_else(|| c.get(2))?.as_str().to_ascii_uppercase();
    Some(match raw.as_str() {
        "TRACE" => "TRACE",
        "DEBUG" => "DEBUG",
        "INFO" | "NOTICE" => "INFO",
        "WARN" | "WARNING" => "WARN",
        "ERROR" | "ERR" => "ERROR",
        _ => "FATAL",
    })
}

fn is_log(t: &str) -> bool {
    let mut lines = 0usize;
    let mut marked = 0usize;
    for line in t.lines().filter(|l| !l.trim().is_empty()).take(2000) {
        lines += 1;
        if level_of(line).is_some() {
            marked += 1;
        }
    }
    marked >= 3 && marked * 10 >= lines * 4
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shape(s: &str) -> Shape {
        let j = J::parse(s.trim()).ok();
        let text = match &j {
            Some(J::Str(x)) => x.as_str(),
            _ => s,
        };
        detect(text, j.as_ref())
    }

    #[test]
    fn shapes() {
        assert_eq!(shape("5"), Shape::Scalar);
        assert_eq!(shape("hello"), Shape::Scalar);
        assert_eq!(shape("[]"), Shape::Empty);
        assert_eq!(shape("{}"), Shape::Empty);
        assert_eq!(shape(""), Shape::Empty);
        assert_eq!(shape(r#"{"a":1}"#), Shape::JsonObj);
        assert_eq!(shape(r#"[{"a":1},{"a":2}]"#), Shape::JsonList);
        assert_eq!(shape("[[1,2],[3,4]]"), Shape::Matrix2d);
        assert_eq!(shape("[1,2.5,3]"), Shape::NumList);
        assert_eq!(shape(r#"["a/b","c"]"#), Shape::StrList);
        assert_eq!(shape(r#"[1,"a"]"#), Shape::JsonMixed);
        assert_eq!(shape("C:\\shots\\a.PNG"), Shape::Image);
        assert_eq!(shape("data:image/png;base64,AAAA"), Shape::Image);
        let log = "2026-01-01 10:00 INFO start\n2026-01-01 10:01 WARN slow\n[ERROR] boom\nplain\n";
        assert_eq!(shape(log), Shape::Log);
        let prose = "This is an error in prose.\nSecond line that is long enough to not be a scalar at all, really.\nThird.";
        assert_eq!(shape(prose), Shape::Text);
    }

    #[test]
    fn levels() {
        assert_eq!(level_of("[warn] x"), Some("WARN"));
        assert_eq!(level_of("12:00 ERROR: db"), Some("ERROR"));
        assert_eq!(level_of("no ERRORS here"), None);
        assert_eq!(level_of("CRITICAL disk"), Some("FATAL"));
    }
}
