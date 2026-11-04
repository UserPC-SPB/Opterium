//! File content as text: one decoding, line and token rule for everything
//! navguard reports (map, navigators, symbols, lines, search, todos).

/// Bytes looked at to tell binary files from text.
const BINARY_SNIFF_BYTES: usize = 8192;

/// True when `bytes` look binary: a NUL byte in the first 8 KiB that is not
/// part of UTF-16 text with a byte order mark.
pub fn is_binary(bytes: &[u8]) -> bool {
    if utf16_bom(bytes).is_some() {
        return false;
    }
    bytes.iter().take(BINARY_SNIFF_BYTES).any(|&b| b == 0)
}

fn utf16_bom(bytes: &[u8]) -> Option<bool> {
    match bytes {
        [0xFF, 0xFE, ..] => Some(true),
        [0xFE, 0xFF, ..] => Some(false),
        _ => None,
    }
}

/// The text of a file: UTF-16 with a byte order mark is decoded as such,
/// everything else as UTF-8 (a BOM is dropped, invalid bytes become U+FFFD).
/// Line breaks are normalized: CRLF and a lone CR become LF.
pub fn decode(bytes: &[u8]) -> String {
    let text = match utf16_bom(bytes) {
        Some(little_endian) => {
            let units: Vec<u16> = bytes[2..]
                .chunks(2)
                .map(|pair| match (pair, little_endian) {
                    ([lo, hi], true) => u16::from_le_bytes([*lo, *hi]),
                    ([hi, lo], false) => u16::from_be_bytes([*hi, *lo]),
                    _ => 0xFFFD,
                })
                .collect();
            String::from_utf16_lossy(&units)
        }
        None => {
            let bytes = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(bytes);
            String::from_utf8_lossy(bytes).into_owned()
        }
    };
    normalize_line_breaks(&text)
}

/// `text` with CRLF and lone CR turned into LF.
pub fn normalize_line_breaks(text: &str) -> String {
    if !text.contains('\r') {
        return text.to_string();
    }
    text.replace("\r\n", "\n").replace('\r', "\n")
}

/// Lines of normalized `text`: an empty text has none, a last line without
/// a line break counts, a final line break does not start another line.
pub fn line_count(text: &str) -> u64 {
    if text.is_empty() {
        return 0;
    }
    let breaks = text.bytes().filter(|&b| b == b'\n').count() as u64;
    if text.ends_with('\n') {
        breaks
    } else {
        breaks + 1
    }
}

/// A token estimate meant to be a little high for every common tokenizer:
/// one token per 3 bytes of UTF-8 (ASCII 1/3 per character, Cyrillic and
/// other 2-byte characters 2/3, CJK 1), 2 per 4-byte character (emoji),
/// rounded up.
pub fn estimate_tokens(text: &str) -> u64 {
    let thirds: u64 = text
        .chars()
        .map(|c| match c.len_utf8() {
            4 => 6,
            n => n as u64,
        })
        .sum();
    thirds.div_ceil(3)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_counts_follow_line_breaks() {
        assert_eq!(line_count(""), 0);
        assert_eq!(line_count("x"), 1);
        assert_eq!(line_count("x\n"), 1);
        assert_eq!(line_count("one\ntwo\nthree\n"), 3);
        assert_eq!(line_count("one\ntwo\nthree"), 3);
        assert_eq!(line_count("\n"), 1);
        assert_eq!(line_count("a\n\nb"), 3);
        assert_eq!(line_count(&decode(b"a\r\nb\r\n")), 2);
        assert_eq!(line_count(&decode(b"a\rb\rc")), 3);
        assert_eq!(line_count(&decode(b"a\r\rb")), 3);
    }

    #[test]
    fn decoding() {
        assert_eq!(decode(b"\xEF\xBB\xBFhi\r\n"), "hi\n");
        assert_eq!(decode(b"\xFF\xFEh\0i\0\r\0\n\0"), "hi\n");
        assert_eq!(decode(b"\xFE\xFF\0h\0i"), "hi");
        assert_eq!(decode(b"a\xFFb"), "a\u{FFFD}b");
    }

    #[test]
    fn binary_files() {
        assert!(is_binary(b"MZ\x90\0\x03"));
        assert!(!is_binary(b"plain text\n"));
        assert!(!is_binary(b"\xFF\xFEh\0i\0"));
        assert!(!is_binary(b""));
    }

    #[test]
    fn token_estimates_err_high() {
        assert_eq!(estimate_tokens(""), 0);
        assert_eq!(estimate_tokens("abc"), 1);
        assert_eq!(estimate_tokens("abcd"), 2);
        // 6 Cyrillic letters = 12 bytes.
        assert_eq!(estimate_tokens("привет"), 4);
        assert_eq!(estimate_tokens("日本"), 2);
        assert_eq!(estimate_tokens("😀"), 2);
        let code = "fn main() {\n    helper();\n}\n";
        assert_eq!(estimate_tokens(code), 10);
    }
}
