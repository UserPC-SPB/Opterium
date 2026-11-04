//! image_meta: format, size and weight of an image; never its pixels.

use crate::ops::Effect;
use crate::render::{Ctx, RResult, Rendered};
use std::io::Read;

pub fn render(c: &Ctx) -> RResult {
    let t = c.text.trim();
    let (bytes, total, label) = if let Some(rest) = t.strip_prefix("data:") {
        let (_, b64) = rest.split_once(',').ok_or("data URI without ','")?;
        let bytes = decode_base64(b64).ok_or("invalid base64 image data")?;
        let n = bytes.len() as u64;
        (bytes, n, String::from("inline"))
    } else {
        let meta = std::fs::metadata(t).map_err(|e| format!("image file: {e}"))?;
        let mut head = Vec::new();
        std::fs::File::open(t)
            .and_then(|f| f.take(1 << 20).read_to_end(&mut head))
            .map_err(|e| format!("image file: {e}"))?;
        (head, meta.len(), t.to_string())
    };
    let (fmt, w, h) = probe(&bytes).ok_or("unknown image format")?;
    // Pixels are never given to the agent: metadata only.
    let eff = Effect {
        lossy: true,
        notes: Vec::new(),
    };
    Ok(Rendered {
        text: format!("image: {fmt} {w}x{h}, {} ({label})", human(total)),
        eff,
    })
}

fn human(n: u64) -> String {
    let (v, u) = if n >= 1 << 20 {
        (n as f64 / (1u64 << 20) as f64, "MB")
    } else if n >= 1 << 10 {
        (n as f64 / 1024.0, "KB")
    } else {
        return format!("{n} B");
    };
    format!("{v:.1} {u}")
}

fn be16(b: &[u8], i: usize) -> Option<u32> {
    Some(u32::from(*b.get(i)?) << 8 | u32::from(*b.get(i + 1)?))
}
fn le16(b: &[u8], i: usize) -> Option<u32> {
    Some(u32::from(*b.get(i)?) | u32::from(*b.get(i + 1)?) << 8)
}
fn le24(b: &[u8], i: usize) -> Option<u32> {
    Some(le16(b, i)? | u32::from(*b.get(i + 2)?) << 16)
}
fn be32(b: &[u8], i: usize) -> Option<u32> {
    Some(be16(b, i)? << 16 | be16(b, i + 2)?)
}

/// Format, width and height from the image header.
pub fn probe(b: &[u8]) -> Option<(&'static str, u32, u32)> {
    if b.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Some(("png", be32(b, 16)?, be32(b, 20)?));
    }
    if b.starts_with(b"GIF87a") || b.starts_with(b"GIF89a") {
        return Some(("gif", le16(b, 6)?, le16(b, 8)?));
    }
    if b.starts_with(b"BM") {
        let w = i32::from_le_bytes(b.get(18..22)?.try_into().ok()?);
        let h = i32::from_le_bytes(b.get(22..26)?.try_into().ok()?);
        return Some(("bmp", w.unsigned_abs(), h.unsigned_abs()));
    }
    if b.starts_with(b"RIFF") && b.get(8..12)? == b"WEBP" {
        let chunk = b.get(12..16)?;
        return match chunk {
            b"VP8 " => Some(("webp", le16(b, 26)? & 0x3fff, le16(b, 28)? & 0x3fff)),
            b"VP8L" => {
                let (b0, b1, b2, b3) = (
                    u32::from(*b.get(21)?),
                    u32::from(*b.get(22)?),
                    u32::from(*b.get(23)?),
                    u32::from(*b.get(24)?),
                );
                Some((
                    "webp",
                    1 + (b0 | (b1 & 0x3f) << 8),
                    1 + (b1 >> 6 | b2 << 2 | (b3 & 0x0f) << 10),
                ))
            }
            b"VP8X" => Some(("webp", 1 + le24(b, 24)?, 1 + le24(b, 27)?)),
            _ => None,
        };
    }
    if b.starts_with(&[0xff, 0xd8]) {
        let mut i = 2;
        while i + 3 < b.len() {
            if b[i] != 0xff {
                i += 1;
                continue;
            }
            let m = b[i + 1];
            if m == 0xff {
                i += 1;
                continue;
            }
            if m == 0x01 || (0xd0..=0xd9).contains(&m) {
                i += 2;
                continue;
            }
            let len = be16(b, i + 2)? as usize;
            if (0xc0..=0xcf).contains(&m) && m != 0xc4 && m != 0xc8 && m != 0xcc {
                return Some(("jpeg", be16(b, i + 7)?, be16(b, i + 5)?));
            }
            i += 2 + len;
        }
        return None;
    }
    None
}

pub fn decode_base64(s: &str) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(s.len() * 3 / 4);
    let mut acc = 0u32;
    let mut bits = 0u32;
    for ch in s.bytes() {
        let v = match ch {
            b'A'..=b'Z' => ch - b'A',
            b'a'..=b'z' => ch - b'a' + 26,
            b'0'..=b'9' => ch - b'0' + 52,
            b'+' | b'-' => 62,
            b'/' | b'_' => 63,
            b'=' => break,
            b' ' | b'\n' | b'\r' | b'\t' => continue,
            _ => return None,
        };
        acc = acc << 6 | u32::from(v);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
            acc &= (1 << bits) - 1;
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn png_gif_bmp() {
        let mut png = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR".to_vec();
        png.extend_from_slice(&640u32.to_be_bytes());
        png.extend_from_slice(&480u32.to_be_bytes());
        assert_eq!(probe(&png), Some(("png", 640, 480)));
        let gif = b"GIF89a\x20\x03\x58\x02";
        assert_eq!(probe(gif), Some(("gif", 800, 600)));
    }

    #[test]
    fn jpeg_sof() {
        let jpg = [
            0xff, 0xd8, 0xff, 0xe0, 0, 4, 0, 0, 0xff, 0xc0, 0, 11, 8, 0x01, 0xe0, 0x02, 0x80, 3,
        ];
        assert_eq!(probe(&jpg), Some(("jpeg", 640, 480)));
    }

    #[test]
    fn base64() {
        assert_eq!(decode_base64("aGVsbG8=").unwrap(), b"hello");
        assert_eq!(decode_base64("aGVsbG8").unwrap(), b"hello");
    }
}
