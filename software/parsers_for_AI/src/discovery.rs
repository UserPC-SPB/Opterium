use crate::limits::ResourceLimits;
use crate::template::Format;
use std::io::Read;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct DiscoveredFile {
    pub path: PathBuf,
    pub size: u64,
    pub mtime_iso: String,
    pub hash: Option<String>,
}

#[derive(Debug)]
pub struct DiscoveryError {
    pub message: String,
    pub exit: crate::limits::ExitCode,
}

pub fn discover(
    input_pattern: &str,
    output_path: Option<&Path>,
    follow_symlinks: bool,
    limits: &ResourceLimits,
    compute_hash: bool,
) -> Result<Vec<DiscoveredFile>, DiscoveryError> {
    let mut raw_paths: Vec<PathBuf> = Vec::new();
    let p = Path::new(input_pattern);

    let pattern_has_glob =
        input_pattern.contains('*') || input_pattern.contains('?') || input_pattern.contains('[');

    if p.is_dir() {
        let mut entries: Vec<PathBuf> = walk_dir(p, follow_symlinks, 0)
            .map_err(|e| DiscoveryError {
                message: format!("cannot walk directory '{}': {}", p.display(), e),
                exit: crate::limits::ExitCode::InvalidInputOrCli,
            })?
            .into_iter()
            .collect();
        entries.sort();
        raw_paths = entries;
    } else if pattern_has_glob && !p.is_file() {
        let pat = input_pattern.replace('\\', "/");
        let options = glob::MatchOptions {
            case_sensitive: false,
            require_literal_separator: true,
            require_literal_leading_dot: false,
        };
        for entry in glob::glob_with(&pat, options).map_err(|e| DiscoveryError {
            message: format!("invalid glob pattern '{}': {}", input_pattern, e),
            exit: crate::limits::ExitCode::InvalidInputOrCli,
        })? {
            match entry {
                Ok(path) => raw_paths.push(path),
                Err(e) => {
                    return Err(DiscoveryError {
                        message: format!("glob error: {}", e),
                        exit: crate::limits::ExitCode::InvalidInputOrCli,
                    });
                }
            }
        }
        raw_paths.sort();
    } else {
        raw_paths.push(p.to_path_buf());
    }

    let out: Option<PathBuf> =
        output_path.map(|o| o.canonicalize().unwrap_or_else(|_| o.to_path_buf()));

    let mut files = Vec::new();
    for rp in raw_paths {
        if !rp.is_file() {
            continue;
        }
        let canon = rp.canonicalize().unwrap_or_else(|_| rp.clone());
        if let Some(o) = &out {
            if canon == *o {
                continue;
            }
        }
        if !follow_symlinks && rp.ancestors().any(|path| path.is_symlink()) {
            continue;
        }
        let meta = match std::fs::metadata(&rp) {
            Ok(m) => m,
            Err(e) => {
                return Err(DiscoveryError {
                    message: format!("cannot stat '{}': {}", rp.display(), e),
                    exit: crate::limits::ExitCode::InvalidInputOrCli,
                });
            }
        };
        files.push(DiscoveredFile {
            path: rp,
            size: meta.len(),
            mtime_iso: format_systemtime(meta.modified().ok()),
            hash: None,
        });
    }

    if let Some(max) = limits.max_input_files {
        if files.len() as u64 > max {
            return Err(DiscoveryError {
                message: format!(
                    "resource limit: discovered {} files, allowed at most {}",
                    files.len(),
                    max
                ),
                exit: crate::limits::ExitCode::ResourceLimit,
            });
        }
    }
    for f in &files {
        if let Some(max) = limits.max_file_size_bytes {
            if f.size > max {
                return Err(DiscoveryError {
                    message: format!(
                        "resource limit: file '{}' is {} bytes, allowed at most {}",
                        f.path.display(),
                        f.size,
                        max
                    ),
                    exit: crate::limits::ExitCode::ResourceLimit,
                });
            }
        }
    }
    // Validate resource limits before reading source bytes for hashing.
    if compute_hash {
        for f in &mut files {
            f.hash = Some(sha256_hex(&f.path).map_err(|e| DiscoveryError {
                message: format!("cannot hash '{}': {}", f.path.display(), e),
                exit: crate::limits::ExitCode::InvalidInputOrCli,
            })?);
        }
    }
    Ok(files)
}

fn walk_dir(dir: &Path, follow_symlinks: bool, _depth: usize) -> std::io::Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    for entry in walkdir::WalkDir::new(dir).follow_links(follow_symlinks) {
        let entry = entry.map_err(std::io::Error::from)?;
        if entry.file_type().is_file() {
            out.push(entry.into_path());
        }
    }
    Ok(out)
}

pub fn format_systemtime(t: Option<std::time::SystemTime>) -> String {
    match t {
        Some(st) => {
            let dt: chrono::DateTime<chrono::Utc> = st.into();
            dt.format("%Y-%m-%dT%H:%M:%SZ").to_string()
        }
        None => "unknown".to_string(),
    }
}

pub struct Decoded {
    pub text: String,
    pub encoding: String,
    pub hash: Option<String>,
}

#[derive(Debug)]
pub struct ReadError {
    pub message: String,
    pub resource_limit: bool,
}

impl ReadError {
    fn input(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            resource_limit: false,
        }
    }
    fn limit(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            resource_limit: true,
        }
    }
}

pub fn read_decoded(
    path: &Path,
    encoding: &str,
    max_bytes: Option<u64>,
    compute_hash: bool,
) -> Result<Decoded, ReadError> {
    let mut f = std::fs::File::open(path)
        .map_err(|e| ReadError::input(format!("cannot open '{}': {}", path.display(), e)))?;
    if let Some(max) = max_bytes {
        let meta = f
            .metadata()
            .map_err(|e| ReadError::input(format!("cannot stat: {}", e)))?;
        if meta.len() > max {
            return Err(ReadError::limit(format!(
                "resource limit: file is {} bytes, allowed at most {}",
                meta.len(),
                max
            )));
        }
    }
    let mut bytes = Vec::new();
    // Bound the read itself too: the source may grow after metadata was read.
    let read = if let Some(max) = max_bytes {
        (&mut f).take(max.saturating_add(1)).read_to_end(&mut bytes)
    } else {
        f.read_to_end(&mut bytes)
    };
    read.map_err(|e| ReadError::input(format!("cannot read '{}': {}", path.display(), e)))?;
    if max_bytes.is_some_and(|max| bytes.len() as u64 > max) {
        return Err(ReadError::limit(format!(
            "resource limit: '{}' grew beyond the file-size limit",
            path.display()
        )));
    }
    let mut decoded = decode(&bytes, encoding).map_err(ReadError::input)?;
    if compute_hash {
        use sha2::{Digest, Sha256};
        decoded.hash = Some(format!("{:x}", Sha256::digest(&bytes)));
    }
    Ok(decoded)
}

/// Decodes `bytes` and normalizes line breaks (see
/// [`normalize_line_breaks`]), so every adapter, line number and fragment
/// sees the same lines.
pub fn decode(bytes: &[u8], encoding: &str) -> Result<Decoded, String> {
    let mut decoded = decode_text(bytes, encoding)?;
    decoded.text = normalize_line_breaks(decoded.text);
    Ok(decoded)
}

/// The one line-break rule of the parser: LF, CRLF and a lone CR (classic
/// Mac OS) each end a line, as in XML, YAML, HTML and CommonMark. Decoded
/// text gets LF for all three before format detection, so `line:` ranges,
/// context windows, recon samples and XML, HTML or Markdown source
/// fragments count and contain the same lines, and the published document
/// has no CR from a source's line endings. A CR inside a value (JSON
/// `"\r"`, XML `&#13;`) is data and is kept.
pub fn normalize_line_breaks(text: String) -> String {
    if !text.contains('\r') {
        return text;
    }
    let mut out = String::with_capacity(text.len());
    let mut rest = text.as_str();
    while let Some(cr) = rest.find('\r') {
        out.push_str(&rest[..cr]);
        out.push('\n');
        rest = &rest[cr + 1..];
        if let Some(after) = rest.strip_prefix('\n') {
            rest = after;
        }
    }
    out.push_str(rest);
    out
}

fn decode_text(bytes: &[u8], encoding: &str) -> Result<Decoded, String> {
    let enc = encoding.to_ascii_lowercase();
    match enc.as_str() {
        "utf-8" | "utf8" => {
            if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
                let text = std::str::from_utf8(&bytes[3..])
                    .map_err(|e| format!("invalid UTF-8 content: {}", e))?;
                Ok(Decoded {
                    text: text.to_string(),
                    encoding: "utf-8 (BOM stripped)".to_string(),
                    hash: None,
                })
            } else {
                let text = std::str::from_utf8(bytes)
                    .map_err(|e| format!("invalid UTF-8 content: {}", e))?;
                Ok(Decoded {
                    text: text.to_string(),
                    encoding: "utf-8".to_string(),
                    hash: None,
                })
            }
        }
        "utf-8-sig" => {
            let start = if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
                3
            } else {
                0
            };
            let text = std::str::from_utf8(&bytes[start..])
                .map_err(|e| format!("invalid UTF-8 content: {}", e))?;
            Ok(Decoded {
                text: text.to_string(),
                encoding: "utf-8-sig".to_string(),
                hash: None,
            })
        }
        "utf-16" | "utf-16le" => {
            let (text, used) = decode_utf16(bytes, encoding_rs::UTF_16LE)?;
            Ok(Decoded {
                text,
                encoding: used,
                hash: None,
            })
        }
        "utf-16be" => {
            let (text, used) = decode_utf16(bytes, encoding_rs::UTF_16BE)?;
            Ok(Decoded {
                text,
                encoding: used,
                hash: None,
            })
        }
        "windows-1251" | "cp1251" => {
            let (text, _, errors) = encoding_rs::WINDOWS_1251.decode(bytes);
            if errors {
                return Err("invalid windows-1251 content".to_string());
            }
            Ok(Decoded {
                text: text.to_string(),
                encoding: "windows-1251".to_string(),
                hash: None,
            })
        }
        other => Err(format!("unsupported encoding '{}'", other)),
    }
}

fn decode_utf16(
    bytes: &[u8],
    enc: &'static encoding_rs::Encoding,
) -> Result<(String, String), String> {
    let used = if enc == encoding_rs::UTF_16LE {
        "utf-16le"
    } else {
        "utf-16be"
    };
    let bom = bytes.starts_with(&[0xFF, 0xFE]) || bytes.starts_with(&[0xFE, 0xFF]);
    if bom && ((bytes[0] == 0xFF) != (enc == encoding_rs::UTF_16LE)) {
        return Err(format!(
            "UTF-16 BOM contradicts requested encoding {}",
            used
        ));
    }
    let source = if bom { &bytes[2..] } else { bytes };
    let (text, errors) = enc.decode_without_bom_handling(source);
    if errors {
        return Err(format!("invalid {} content", used));
    }
    Ok((
        text.to_string(),
        if bom {
            format!("{} (BOM)", used)
        } else {
            used.to_string()
        },
    ))
}

/// Extension-only format hint without reading file content.
/// Returns the format `detect_format` would assign purely by extension, or
/// `None` when the extension is not recognized (a real run would then fall
/// back to content sniffing). Used by `--dry-run` reporting.
pub fn extension_hint(path: &Path) -> Option<Format> {
    let name = path.to_string_lossy().to_string();
    match Format::from_extension(&name) {
        Some(f) if f != Format::Text || is_plain_ext(&name) => Some(f),
        _ => None,
    }
}

pub fn detect_format(path: &Path, text: &str) -> (Format, String) {
    if let Some(f) = Format::from_extension(&path.to_string_lossy()) {
        if f != Format::Text || is_plain_ext(&path.to_string_lossy()) {
            return (f, "extension".to_string());
        }
    }
    let sniffed = sniff(text);
    if let Some(f) = sniffed {
        return (f, "content".to_string());
    }
    (Format::Text, "content fallback".to_string())
}

fn is_plain_ext(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower.ends_with(".txt") || lower.ends_with(".log")
}

fn sniff(text: &str) -> Option<Format> {
    let t = text.trim_start_matches(['\u{feff}', '\r', '\n', '\t', ' ']);
    if t.is_empty() {
        return None;
    }
    let first = t.chars().next().unwrap();
    if first == '{' || first == '[' {
        if serde_json::from_str::<serde_json::Value>(t).is_ok() {
            return Some(Format::Json);
        }
        // `[table]` may still start TOML; other bracketed text is not JSON.
        if first == '{' {
            return None;
        }
    } else if first == '<' {
        let lower = t.chars().take(512).collect::<String>().to_ascii_lowercase();
        if lower.starts_with("<!doctype html") || lower.starts_with("<html") {
            return Some(Format::Html);
        }
        // Too deep for the parser: XML only if every tag pairs up, so the
        // adapter reports the depth limit, while `<34>` syslog lines or
        // `<nick>` chat logs stay text.
        let too_deep =
            crate::adapters::check_xml_depth(t, crate::limits::MAX_XML_NESTING_DEPTH).is_err();
        if too_deep {
            return crate::adapters::xml_markup_is_balanced(t).then_some(Format::Xml);
        }
        // Parsed the way the XML adapter parses it, DOCTYPE included (plist,
        // MyBatis mappers). A DTD the adapter rejects (an entity bomb) still
        // makes the file XML, so it fails with that reason instead of being
        // searched line by line as text.
        return match crate::adapters::XmlTree::parse(t) {
            Ok(_) => Some(Format::Xml),
            Err(e) if e.starts_with(crate::adapters::xml_dtd::REJECTED) => Some(Format::Xml),
            Err(_) => None,
        };
    }
    // TOML is strict enough that a successful parse is a reliable signal.
    if t.contains('=')
        && let Ok(toml::Value::Table(table)) = t.parse::<toml::Value>()
        && !table.is_empty()
    {
        return Some(Format::Toml);
    }
    // Almost any text with ": " is valid YAML (log lines, prose, mail
    // headers), so YAML also needs a config-like shape. The cheap first-line
    // check keeps large logs from being parsed as YAML at all.
    if (t.contains(": ") || t.starts_with("---"))
        && first_line_looks_like_yaml(t)
        && looks_like_yaml_document(t)
    {
        return Some(Format::Yaml);
    }
    None
}

/// `name`, `apiVersion`, `server.port`: a letter or `_`, then letters,
/// digits or `_-./`. No spaces, so `2026-10-01 12:00:00 ERROR` is not one.
fn is_identifier_key(key: &str) -> bool {
    let mut chars = key.chars();
    chars
        .next()
        .is_some_and(|first| first.is_alphabetic() || first == '_')
        && chars.all(|c| c.is_alphanumeric() || matches!(c, '_' | '-' | '.' | '/'))
}

/// The first line that is not blank or a comment is `---`, a `- ` list item
/// or `key:` with an identifier key.
fn first_line_looks_like_yaml(text: &str) -> bool {
    let Some(line) = text
        .lines()
        .map(str::trim_end)
        .find(|line| !line.trim().is_empty() && !line.trim_start().starts_with('#'))
    else {
        return false;
    };
    if line == "---" || line.starts_with("--- ") || line == "-" || line.starts_with("- ") {
        return true;
    }
    line.split_once(':').is_some_and(|(key, rest)| {
        is_identifier_key(key) && rest.chars().next().is_none_or(char::is_whitespace)
    })
}

/// A YAML stream with at least one non-empty document, where every
/// non-empty document has a config shape (`---` separated manifests count
/// when all their documents do).
fn looks_like_yaml_document(text: &str) -> bool {
    use serde_yaml::Value;
    let mut config_documents = 0usize;
    // The iterator yields its error forever once loading fails, so stop at
    // the first document that is not a config.
    for document in serde_yaml::Deserializer::from_str(text) {
        // Read the way the YAML adapter reads it (integers beyond 64 bits).
        match crate::adapters::yaml::deserialize_value(document) {
            Ok(Value::Null) => {}
            Ok(value) if has_config_shape(&value) => config_documents += 1,
            _ => return false,
        }
    }
    config_documents > 0
}

/// A top level that is a mapping with identifier-like keys, or a non-empty
/// sequence of mappings.
fn has_config_shape(value: &serde_yaml::Value) -> bool {
    use serde_yaml::Value;
    match value {
        Value::Mapping(map) => {
            !map.is_empty()
                && map
                    .keys()
                    .all(|k| k.as_str().is_some_and(is_identifier_key))
        }
        Value::Sequence(items) => !items.is_empty() && items.iter().all(|item| item.is_mapping()),
        _ => false,
    }
}

pub fn sha256_hex(path: &Path) -> Result<String, std::io::Error> {
    use sha2::{Digest, Sha256};
    let mut f = std::fs::File::open(path)?;
    let mut h = Sha256::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let count = f.read(&mut buf)?;
        if count == 0 {
            break;
        }
        h.update(&buf[..count]);
    }
    Ok(format!("{:x}", h.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sniff_json_xml() {
        assert_eq!(sniff("{\"a\": 1}"), Some(Format::Json));
        assert_eq!(sniff("<root><a/></root>"), Some(Format::Xml));
        assert_eq!(sniff("<!DOCTYPE html><html></html>"), Some(Format::Html));
        assert_eq!(sniff("key: value\nother: 2"), Some(Format::Yaml));
        assert_eq!(sniff("a = 1\nb = \"x\""), Some(Format::Toml));
        assert_eq!(sniff("just text"), None);
    }

    #[test]
    fn sniffing_requires_a_config_shape_for_yaml_and_prefers_strict_toml() {
        assert_eq!(sniff("2026-10-01 12:00:00 ERROR: disk full\n"), None);
        assert_eq!(sniff("Subject line: hello there\nSee also: x\n"), None);
        assert_eq!(sniff("-----BEGIN CERTIFICATE-----\nMIIB\n"), None);
        assert_eq!(sniff("- first\n- second\n"), None);
        assert_eq!(sniff("# Note: tuned\nport = 8080\n"), Some(Format::Toml));
        assert_eq!(sniff("apiVersion: v1\nkind: Pod\n"), Some(Format::Yaml));
        assert_eq!(sniff("---\nserver.port: 80\n"), Some(Format::Yaml));
        assert_eq!(sniff("- name: a\n  hosts: all\n"), Some(Format::Yaml));
        let deep = format!("{}{}", "<a>".repeat(5000), "</a>".repeat(5000));
        assert_eq!(sniff(&deep), Some(Format::Xml));
        // Starts with `<` and has many unclosed "tags", but is not XML.
        let syslog = "<34>Oct 11 22:14:15 host su: root login failed\n".repeat(200);
        assert_eq!(sniff(&syslog), None);
        assert_eq!(sniff(&"<alice> hello\n".repeat(200)), None);
        // Labels without ": " stay text, as before the YAML shape check.
        assert_eq!(sniff("Error:\n  file not found\n"), None);
        assert_eq!(sniff("Usage:\n  tool [options] <file>\n"), None);
        // A TOML file may start with a table header.
        assert_eq!(sniff("[database]\nhost = \"db\"\n"), Some(Format::Toml));
        assert_eq!(sniff("[2026-10-01] ERROR: x = 1\n"), None);
    }

    #[test]
    fn multi_document_yaml_is_sniffed_when_every_document_is_a_config() {
        let manifest =
            "apiVersion: v1\nkind: Service\n---\napiVersion: apps/v1\nkind: Deployment\n";
        assert_eq!(sniff(manifest), Some(Format::Yaml));
        assert_eq!(
            sniff("---\nkind: A\n---\n\n--- ~\n---\n- name: b\n...\n"),
            Some(Format::Yaml)
        );
        assert_eq!(sniff("name: a\n---\n"), Some(Format::Yaml));
        // A document of only `#` comment lines is empty (helm renders empty
        // templates as `# Source:` documents), so a front-matter note whose
        // body is only `#` headings is yaml too (documented: set --format).
        for text in [
            "apiVersion: v1\nkind: A\n---\n# Source: chart/templates/empty.yaml\n",
            "---\ntitle: Notes\n---\n# Heading\n## Sub\n",
        ] {
            assert_eq!(sniff(text), Some(Format::Yaml), "{text:?}");
        }
        // One document without a config shape keeps the whole file text.
        for text in [
            "title: Notes\n---\nJust prose here.\n",
            "name: a\n---\n2026-10-01 12:00:00 ERROR: x\n",
            "name: a\n---\nSubject line: hello there\n",
            "name: a\n---\n- first\n- second\n",
            "name: a\n---\nbad: [\n",
            "---\n---\n",
            // The first-line trigger is unchanged.
            "Release notes: v2\n---\nname: a\n",
        ] {
            assert_eq!(sniff(text), None, "{text:?}");
        }
    }

    #[test]
    fn auto_and_extensionless_names_are_never_extension_formats() {
        for name in [
            "settings.auto",
            "SETTINGS.AUTO",
            "json",
            "yml",
            ".json",
            "dir.json/file",
        ] {
            assert_eq!(extension_hint(Path::new(name)), None, "{name}");
            let (format, source) = detect_format(Path::new(name), "plain words");
            assert_eq!(
                (format, source.as_str()),
                (Format::Text, "content fallback")
            );
        }
    }

    #[test]
    fn every_line_break_becomes_lf_when_decoding() {
        for (raw, expected) in [
            ("a\r\nb\rc\nd", "a\nb\nc\nd"),
            ("\r\r\n\n\r", "\n\n\n\n"),
            ("no breaks", "no breaks"),
            ("ж\rё\r\n", "ж\nё\n"),
        ] {
            assert_eq!(decode(raw.as_bytes(), "utf-8").unwrap().text, expected);
        }
        let utf16: Vec<u8> = "x\ry"
            .encode_utf16()
            .flat_map(|unit| unit.to_le_bytes())
            .collect();
        assert_eq!(decode(&utf16, "utf-16le").unwrap().text, "x\ny");
    }

    #[test]
    fn utf16_decode() {
        let bytes = [0xFF, 0xFE, 0x68, 0x00, 0x69, 0x00];
        let d = decode(&bytes, "utf-16le").unwrap();
        assert_eq!(d.text, "hi");
    }

    #[test]
    fn malformed_utf16_is_an_error_instead_of_silent_replacement() {
        assert!(decode(&[0x00], "utf-16le").is_err());
        assert!(decode(&[0x00, 0xD8], "utf-16le").is_err());
        assert!(decode(&[0xFE, 0xFF, 0, 0x61], "utf-16le").is_err());
    }

    #[test]
    fn sniffing_unicode_xml_does_not_slice_inside_a_character() {
        let text = format!("<root>{}</root>", "ж".repeat(500));
        assert_eq!(sniff(&text), Some(Format::Xml));
    }

    #[test]
    fn extension_hint_matches_extension_branch_of_detect() {
        // Known extensions -> final format (same as detect_format's first branch)
        assert_eq!(
            extension_hint(Path::new("a/b/test.md")),
            Some(Format::Markdown)
        );
        assert_eq!(extension_hint(Path::new("x.json")), Some(Format::Json));
        assert_eq!(extension_hint(Path::new("cfg.YAML")), Some(Format::Yaml));
        assert_eq!(extension_hint(Path::new("notes.txt")), Some(Format::Text));
        assert_eq!(extension_hint(Path::new("app.log")), Some(Format::Text));
        // Unknown / missing extensions -> sniffing decides at run time
        assert_eq!(extension_hint(Path::new("data.unknownext")), None);
        assert_eq!(extension_hint(Path::new("noext")), None);
    }
}
