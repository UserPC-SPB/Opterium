//! Walks a project and describes its files.
//!
//! The walk never follows symlinks or junctions (their target may lie
//! outside the project), skips excluded names and navguard's own output,
//! and every file is read once for all of its numbers.

use crate::text;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileInfo {
    /// Relative path, `/`-separated.
    pub path: String,
    /// Size in bytes.
    pub bytes: u64,
    /// Lines of the text ([`text::line_count`]); 0 for binary files.
    pub lines: u64,
    /// Token estimate of the text ([`text::estimate_tokens`]); 0 for binary
    /// files.
    pub tokens: u64,
    /// First 8 hex digits of the SHA-256 of the bytes.
    pub sha: String,
    /// The file is not text ([`text::is_binary`]).
    pub binary: bool,
}

/// Describes the file at `rel` with content `content`; returns the decoded
/// text too (`None` for binary files).
pub fn describe(rel: &str, content: &[u8]) -> (FileInfo, Option<String>) {
    let mut hasher = Sha256::new();
    hasher.update(content);
    let sha = format!("{:x}", hasher.finalize())[..8].to_string();
    let binary = text::is_binary(content);
    let decoded = (!binary).then(|| text::decode(content));
    let (lines, tokens) = decoded
        .as_deref()
        .map_or((0, 0), |t| (text::line_count(t), text::estimate_tokens(t)));
    (
        FileInfo {
            path: rel.to_string(),
            bytes: content.len() as u64,
            lines,
            tokens,
            sha,
            binary,
        },
        decoded,
    )
}

/// Relative paths (`/`-separated, sorted) of the project's files.
pub fn list_files(root: &str, exclude: &[String]) -> Vec<String> {
    let mut files = Vec::new();
    walk(Path::new(root), Path::new(root), exclude, &mut files);
    files.sort();
    files
}

/// Every project file, described; unreadable files are left out.
pub fn scan_directory(root: &str, exclude: &[String]) -> Vec<FileInfo> {
    list_files(root, exclude)
        .into_iter()
        .filter_map(|rel| {
            let content = fs::read(Path::new(root).join(&rel)).ok()?;
            Some(describe(&rel, &content).0)
        })
        .collect()
}

pub fn is_excluded_path(path: &str, exclude: &[String]) -> bool {
    path.replace('\\', "/")
        .split('/')
        .any(|name| exclude.iter().any(|pattern| name.contains(pattern)))
}

/// Files navguard writes itself: the navigators, the cache, the rules of
/// `--set-rules`, the workflow state and log of `--triangle` and atomic
/// write leftovers. Never part of a scan, whatever `exclude` says.
pub fn is_generated_file(path: &Path) -> bool {
    matches!(
        path.file_name().and_then(|name| name.to_str()),
        Some(
            "navigator.md"
                | "navigator_deep.md"
                | "navigator_deep2.md"
                | ".navguard_cache.json"
                | ".navguard_rules"
                | ".abc_triangle_state.json"
                | ".abc_triangle_log.json"
        )
    ) || path
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.ends_with(crate::write::TEMP_SUFFIX))
}

fn walk(dir: &Path, root: &Path, exclude: &[String], files: &mut Vec<String>) {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        // On Windows a junction is a symlink here too.
        if kind.is_symlink() {
            continue;
        }
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if exclude.iter().any(|e| name.contains(e.as_str())) {
            continue;
        }
        let path = entry.path();
        if kind.is_dir() {
            walk(&path, root, exclude, files);
        } else if kind.is_file() && !is_generated_file(&path) {
            let rel = path.strip_prefix(root).unwrap_or(&path);
            files.push(rel.to_string_lossy().replace('\\', "/"));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scan(dir: &Path, exclude: &[&str]) -> Vec<FileInfo> {
        let exclude: Vec<String> = exclude.iter().map(|s| s.to_string()).collect();
        scan_directory(&dir.to_string_lossy(), &exclude)
    }

    #[test]
    fn finds_files_in_subdirectories() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path();
        fs::write(dir.join("a.py"), "x = 1\n").unwrap();
        fs::write(dir.join("b.rs"), "fn main() {}\n").unwrap();
        fs::create_dir_all(dir.join("sub")).unwrap();
        fs::write(dir.join("sub/c.js"), "let y = 2;\n").unwrap();
        let names: Vec<String> = scan(dir, &[".git", "target"])
            .into_iter()
            .map(|f| f.path)
            .collect();
        assert_eq!(names, ["a.py", "b.rs", "sub/c.js"]);
    }

    #[test]
    fn excludes_names_and_generated_files() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path();
        fs::write(dir.join("main.rs"), "fn main() {}\n").unwrap();
        fs::create_dir_all(dir.join("target")).unwrap();
        fs::write(dir.join("target/out.exe"), "bins").unwrap();
        fs::write(dir.join("navigator.md"), "old").unwrap();
        fs::write(dir.join(".navguard_cache.json"), "{}").unwrap();
        fs::write(dir.join(".navguard_rules"), "rules").unwrap();
        fs::create_dir_all(dir.join("scripts")).unwrap();
        fs::write(dir.join("scripts/.abc_triangle_state.json"), "{}").unwrap();
        fs::write(dir.join("scripts/.abc_triangle_log.json"), "[]").unwrap();
        let files = scan(dir, &["target"]);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, "main.rs");
    }

    #[test]
    fn default_exclude_skips_tool_folders() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path();
        fs::write(dir.join("main.rs"), "fn main() {}\n").unwrap();
        for folder in [
            ".git",
            ".zvec-grep",
            "node_modules",
            "__pycache__",
            "target",
        ] {
            fs::create_dir_all(dir.join(folder)).unwrap();
            fs::write(dir.join(folder).join("CURRENT"), "x\n").unwrap();
        }
        let exclude = crate::config::Config::default().exclude;
        let names: Vec<String> = scan_directory(&dir.to_string_lossy(), &exclude)
            .into_iter()
            .map(|f| f.path)
            .collect();
        assert_eq!(names, ["main.rs"]);
    }

    #[test]
    fn describes_lines_tokens_size_and_sha() {
        let (info, text) = describe("a.txt", b"one\ntwo\nthree\n");
        assert_eq!(info.lines, 3);
        assert_eq!(info.bytes, 14);
        assert_eq!(info.tokens, 5);
        assert_eq!(info.sha, "b6285c57");
        assert!(!info.binary);
        assert_eq!(text.as_deref(), Some("one\ntwo\nthree\n"));
        assert_eq!(describe("e.txt", b"").0.lines, 0);
        assert_eq!(describe("d.txt", b"x\n").0.lines, 1);
        assert_eq!(describe("cr.txt", b"a\rb\rc").0.lines, 3);
        let (bin, text) = describe("x.exe", b"MZ\0\0\x90");
        assert!(bin.binary);
        assert_eq!((bin.lines, bin.tokens, bin.bytes), (0, 0, 5));
        assert!(text.is_none());
    }

    #[cfg(windows)]
    #[test]
    fn junctions_are_not_followed() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("root");
        let outside = tmp.path().join("outside");
        fs::create_dir_all(&root).unwrap();
        fs::create_dir_all(&outside).unwrap();
        fs::write(root.join("in.txt"), "in\n").unwrap();
        fs::write(outside.join("secret.txt"), "secret\n").unwrap();
        let status = std::process::Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(root.join("link"))
            .arg(&outside)
            .output()
            .unwrap();
        assert!(status.status.success(), "mklink /J failed: {:?}", status);
        let names: Vec<String> = scan(&root, &[]).into_iter().map(|f| f.path).collect();
        assert_eq!(names, ["in.txt"]);
    }
}
