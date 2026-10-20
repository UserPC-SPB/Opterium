use crate::template::{ItemTransform, ResultTransform};

pub fn apply_item_transforms(value: String, transforms: &[ItemTransform]) -> String {
    let mut v = value;
    for t in transforms {
        v = apply_one(t, v);
    }
    v
}

fn apply_one(t: &ItemTransform, v: String) -> String {
    match t {
        ItemTransform::Trim => v.trim().to_string(),
        ItemTransform::NormalizeWhitespace => {
            let mut out = String::with_capacity(v.len());
            let mut prev_ws = false;
            for c in v.chars() {
                if c.is_whitespace() {
                    if !prev_ws {
                        out.push(' ');
                    }
                    prev_ws = true;
                } else {
                    out.push(c);
                    prev_ws = false;
                }
            }
            let out = out.trim().to_string();
            out
        }
        ItemTransform::Lowercase => v.to_lowercase(),
        ItemTransform::Uppercase => v.to_uppercase(),
        ItemTransform::Unescape => unescape(&v),
        ItemTransform::TruncateChars(max) => {
            if v.chars().count() > *max {
                v.chars().take(*max).collect()
            } else {
                v
            }
        }
        ItemTransform::TruncateLines(max) => {
            let lines: Vec<&str> = v.lines().collect();
            if lines.len() > *max {
                lines[..*max].join("\n")
            } else {
                v
            }
        }
        ItemTransform::Replace { from, to } => v.replace(from, to),
        ItemTransform::RegexReplace {
            pattern,
            replacement,
        } => pattern.replace_all(&v, replacement.as_str()).into_owned(),
    }
}

/// Decodes backslash escapes for the `unescape` item transform.
///
/// Supported escapes:
/// - `\n`, `\t`, `\r`, `\"`, `\'`, `\\`;
/// - `\uXXXX` with exactly four ASCII hex digits in any case. A UTF-16
///   surrogate pair written as two adjacent escapes (`\ud83e\udd80`)
///   decodes to one character (`🦀`).
///
/// Everything else is kept literally, so no input character is lost:
/// unsupported escapes (`\/`, `\b`, `\f`, `\0`, `\x41`, `\U0041`, `\u{41}`,
/// ...), a trailing `\`, a `\u` without four hex digits (the text after it
/// is scanned again), and lone or unpaired surrogates (`\ud800`). Decoded
/// characters are never re-read as the start of another escape.
fn unescape(v: &str) -> String {
    let mut out = String::with_capacity(v.len());
    let mut rest = v;
    while let Some(pos) = rest.find('\\') {
        out.push_str(&rest[..pos]);
        let escape = &rest[pos..];
        let (decoded, len) = decode_escape(escape);
        match decoded {
            Some(c) => out.push(c),
            None => out.push_str(&escape[..len]),
        }
        rest = &escape[len..];
    }
    out.push_str(rest);
    out
}

/// Decodes the escape at the start of `s`, which begins with `\`. Returns
/// the decoded character (`None`: keep the covered text literally) and the
/// number of bytes the escape covers.
fn decode_escape(s: &str) -> (Option<char>, usize) {
    let Some(next) = s[1..].chars().next() else {
        return (None, 1);
    };
    let decoded = match next {
        'n' => '\n',
        't' => '\t',
        'r' => '\r',
        '"' => '"',
        '\'' => '\'',
        '\\' => '\\',
        'u' => return decode_unicode_escape(s),
        _ => return (None, 1 + next.len_utf8()),
    };
    (Some(decoded), 2)
}

/// Length in bytes of one `\uXXXX` escape.
const UNICODE_ESCAPE_LEN: usize = 6;

fn decode_unicode_escape(s: &str) -> (Option<char>, usize) {
    let Some(unit) = utf16_unit(s) else {
        // Keep only `\u`: what follows is scanned again.
        return (None, 2);
    };
    if let Some(c) = char::from_u32(u32::from(unit)) {
        return (Some(c), UNICODE_ESCAPE_LEN);
    }
    // A surrogate decodes only as a high + low pair.
    let low = utf16_unit(&s[UNICODE_ESCAPE_LEN..]);
    if let Some(Ok(c)) = low.and_then(|low| char::decode_utf16([unit, low]).next()) {
        return (Some(c), 2 * UNICODE_ESCAPE_LEN);
    }
    (None, UNICODE_ESCAPE_LEN)
}

/// Parses a `\uXXXX` escape with exactly four ASCII hex digits at the start
/// of `s`.
fn utf16_unit(s: &str) -> Option<u16> {
    let hex = s.strip_prefix("\\u")?.get(..4)?;
    if !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    u16::from_str_radix(hex, 16).ok()
}

/// What a truncating item transform would cut from `value`:
/// `(size before, size after)` in the transform's unit (characters for
/// `truncate_chars`, lines for `truncate_lines`). `None` when nothing would
/// be lost, including every transform that does not truncate.
pub fn item_cut(t: &ItemTransform, value: &str) -> Option<(usize, usize)> {
    let (size, max) = match t {
        ItemTransform::TruncateChars(max) => (value.chars().count(), *max),
        ItemTransform::TruncateLines(max) => (value.lines().count(), *max),
        _ => return None,
    };
    (size > max).then_some((size, max))
}

/// An item that result transforms can filter, reorder and join while it
/// carries data of its own along.
pub trait ResultItem: Sized {
    fn text(&self) -> &str;
    fn join(items: Vec<Self>, separator: &str) -> Self;
}

impl ResultItem for String {
    fn text(&self) -> &str {
        self
    }

    fn join(items: Vec<Self>, separator: &str) -> Self {
        items.join(separator)
    }
}

pub fn apply_result_transforms<T: ResultItem>(
    items: Vec<T>,
    transforms: &[ResultTransform],
) -> Vec<T> {
    let mut out = items;
    for t in transforms {
        match t {
            ResultTransform::DropEmpty => {
                out.retain(|s| !s.text().trim().is_empty());
            }
            ResultTransform::Unique => {
                let mut seen = std::collections::HashSet::new();
                out.retain(|s| seen.insert(s.text().to_owned()));
            }
            ResultTransform::SortAsc => out.sort_by(|a, b| a.text().cmp(b.text())),
            ResultTransform::SortDesc => out.sort_by(|a, b| b.text().cmp(a.text())),
            ResultTransform::LimitItems(n) => {
                out.truncate(*n);
            }
            ResultTransform::Join(sep) => {
                let joined = T::join(out, sep);
                out = vec![joined];
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::template::ItemTransform::*;

    #[test]
    fn item_ops() {
        assert_eq!(apply_one(&Trim, "  hi  ".into()), "hi");
        assert_eq!(
            apply_one(&NormalizeWhitespace, "a\t b\n c ".into()),
            "a b c"
        );
        assert_eq!(
            apply_one(
                &Replace {
                    from: "a".into(),
                    to: "X".into()
                },
                "banana".into()
            ),
            "bXnXnX"
        );
        assert_eq!(apply_one(&TruncateChars(2), "hello".into()), "he");
        assert_eq!(apply_one(&TruncateLines(2), "a\nb\nc".into()), "a\nb");
        assert_eq!(apply_one(&Unescape, "a\\n\\t\\u0041".into()), "a\n\tA");
    }

    fn unescape_str(s: &str) -> String {
        apply_one(&Unescape, s.to_string())
    }

    #[test]
    fn unescape_decodes_every_supported_simple_escape() {
        let cases = [
            (r"\n", "\n"),
            (r"\t", "\t"),
            (r"\r", "\r"),
            (r#"\""#, "\""),
            (r"\'", "'"),
            (r"\\", "\\"),
            (r#"a\nb\tc\rd\"e\'f\\g"#, "a\nb\tc\rd\"e'f\\g"),
            // `\\` is consumed as a pair, so the `n` after it stays literal.
            (r"\\n", "\\n"),
            (r"\\\n", "\\\n"),
            (r"\\\\", "\\\\"),
        ];
        for (input, expected) in cases {
            assert_eq!(unescape_str(input), expected, "input {input:?}");
        }
    }

    #[test]
    fn unescape_decodes_valid_unicode_escapes() {
        let cases = [
            (r"\u0041", "A"),
            (r"\u00e9", "é"),
            (r"\u00E9", "é"),
            (r"\u00c9t\u00e9", "Été"),
            (r"\u043f\u0440\u0438\u0432\u0435\u0442", "привет"),
            (r"\u4e2d", "中"),
            (r"\uffff", "\u{ffff}"),
            (r"\u0000", "\0"),
            (r"\u005c", "\\"),
            // The decoded backslash is output, not re-read as an escape.
            (r"\u005cn", "\\n"),
            (r"\u00411", "A1"),
            (r"x\u0041y", "xAy"),
        ];
        for (input, expected) in cases {
            assert_eq!(unescape_str(input), expected, "input {input:?}");
        }
    }

    #[test]
    fn unescape_combines_utf16_surrogate_pairs() {
        let cases = [
            (r"\ud83e\udd80", "🦀"),
            (r"\uD83E\uDD80", "🦀"),
            (r"\uD83e\udD80", "🦀"),
            (r"\ud83d\ude00", "😀"),
            (r"\ud800\udc00", "\u{10000}"),
            (r"\udbff\udfff", "\u{10ffff}"),
            (r"crab: \ud83e\udd80!", "crab: 🦀!"),
            (r"\ud83e\udd80\ud83e\udd80", "🦀🦀"),
            // A lone high surrogate before a pair stays literal; the pair decodes.
            (r"\ud83e\ud83e\udd80", r"\ud83e🦀"),
        ];
        for (input, expected) in cases {
            assert_eq!(unescape_str(input), expected, "input {input:?}");
        }
    }

    #[test]
    fn unescape_keeps_lone_and_mismatched_surrogates_literally() {
        let cases = [
            r"\ud800",
            r"\uD800",
            r"\udbff",
            r"\udc00",
            r"\uDFFF",
            r"\udd80\ud83e",
            r"a\ud83eb",
            r"\ud83e\\udd80",
            r"\ud83e\u12",
            r"\ud83e\uZZZZ",
            r"\ud83e\u",
            r"\ud83e\",
            r"\ud83e\n",
        ];
        let expected = [
            r"\ud800".to_string(),
            r"\uD800".to_string(),
            r"\udbff".to_string(),
            r"\udc00".to_string(),
            r"\uDFFF".to_string(),
            r"\udd80\ud83e".to_string(),
            r"a\ud83eb".to_string(),
            // `\\` decodes to one backslash; the rest is plain text.
            r"\ud83e\udd80".to_string(),
            r"\ud83e\u12".to_string(),
            r"\ud83e\uZZZZ".to_string(),
            r"\ud83e\u".to_string(),
            r"\ud83e\".to_string(),
            "\\ud83e\n".to_string(),
        ];
        for (input, expected) in cases.iter().zip(expected) {
            assert_eq!(unescape_str(input), expected, "input {input:?}");
        }
        // A high surrogate followed by a valid non-surrogate escape: the
        // surrogate stays literal and the second escape still decodes.
        assert_eq!(unescape_str(r"\ud83e\u0041"), r"\ud83eA");
    }

    #[test]
    fn unescape_keeps_invalid_unicode_escapes_literally() {
        let cases = [
            r"\u",
            r"\u1",
            r"\u12",
            r"\u123",
            r"\uZZZZ",
            r"\u12G4",
            r"\uGGGG",
            r"\u+041",
            r"\u-041",
            r"\u 041",
            r"\u{41}",
            r"\u{1F980}",
            r"\u０４１",
            r"\u12é4",
            r"\U0041",
            r"x\u",
            r"\u12 rest",
        ];
        for input in cases {
            assert_eq!(unescape_str(input), input, "input {input:?}");
        }
        // Text after an invalid `\u` is scanned again, so escapes in it decode.
        assert_eq!(unescape_str(r"\u12\n"), "\\u12\n");
        assert_eq!(unescape_str(r"\u\u0041"), r"\uA");
        assert_eq!(unescape_str(r"\u12\u0041"), r"\u12A");
        assert_eq!(unescape_str(r"\u004\u0041"), r"\u004A");
    }

    #[test]
    fn unescape_keeps_unsupported_escapes_and_trailing_backslash_literally() {
        let cases = [
            r"\",
            r"abc\",
            r"\/",
            r"http:\/\/example.com",
            r"\b",
            r"\f",
            r"\0",
            r"\x41",
            r"\a",
            r"\v",
            r"\e",
            r"\N",
            r"\T",
            r"\ ",
            r"\é",
            r"\🦀",
            r"C:\Users\data",
        ];
        for input in cases {
            assert_eq!(unescape_str(input), input, "input {input:?}");
        }
        // An unsupported escape does not hide the escape that follows it.
        assert_eq!(unescape_str(r"\q\n"), "\\q\n");
        assert_eq!(unescape_str(r"\x\u0041"), r"\xA");
    }

    #[test]
    fn unescape_never_drops_characters() {
        // Without any supported escape the text must come back unchanged.
        let inputs = [
            "",
            "plain text",
            "юникод 🦀 中文",
            r"\uZZZZ\u12\u\ud800\x\udfff\",
            r"\u0\u00\u000",
            "\\\u{0}\\",
        ];
        for input in inputs {
            assert_eq!(unescape_str(input), input, "input {input:?}");
        }
        // Mixed: only the valid pair decodes, every invalid piece survives.
        assert_eq!(
            unescape_str(r"\uZZZZ\u12\u\ud800\udc00x\udfff\"),
            "\\uZZZZ\\u12\\u\u{10000}x\\udfff\\"
        );
    }

    #[test]
    fn item_cut_predicts_exactly_what_truncation_removes() {
        for (t, value, cut) in [
            (TruncateChars(2), "hello", Some((5, 2))),
            (TruncateChars(5), "hello", None),
            (TruncateChars(1), "ёж", Some((2, 1))),
            (TruncateLines(2), "a\nb\nc", Some((3, 2))),
            (TruncateLines(2), "a\nb\n", None),
            (TruncateLines(0), "a", Some((1, 0))),
        ] {
            assert_eq!(item_cut(&t, value), cut, "{t:?} on {value:?}");
            let changed = apply_one(&t, value.to_string()) != value;
            assert_eq!(cut.is_some(), changed, "{t:?} on {value:?}");
        }
        // Transforms that only rewrite text never count as a cut.
        assert_eq!(item_cut(&Trim, "  long  "), None);
    }

    #[test]
    fn result_ops() {
        let items = vec![
            "b".to_string(),
            "a".to_string(),
            "b".to_string(),
            "".to_string(),
        ];
        let out = apply_result_transforms(
            items,
            &[
                ResultTransform::DropEmpty,
                ResultTransform::Unique,
                ResultTransform::SortAsc,
            ],
        );
        assert_eq!(out, vec!["a", "b"]);
        let joined = apply_result_transforms(
            vec!["x".to_string(), "y".to_string()],
            &[ResultTransform::Join("|".into())],
        );
        assert_eq!(joined, vec!["x|y"]);
    }
}
