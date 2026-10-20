//! Writing files so that a reader never sees a half-written one.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Suffix of the temporary files; scans and the watcher ignore them.
pub const TEMP_SUFFIX: &str = ".navguard-tmp";

/// The temporary file used while writing `path`.
fn temp_path(path: &Path) -> PathBuf {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    path.with_file_name(format!(".{}{}", name, TEMP_SUFFIX))
}

/// Writes `content` to a temporary file next to `path` and renames it over
/// `path`, so `path` holds either the old or the new content.
pub fn atomic_write(path: &Path, content: &[u8]) -> io::Result<()> {
    let temp = temp_path(path);
    let written = fs::write(&temp, content).and_then(|_| fs::rename(&temp, path));
    if written.is_err() {
        let _ = fs::remove_file(&temp);
    }
    written
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replaces_content_and_leaves_no_temporary_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.txt");
        fs::write(&path, "old").unwrap();
        atomic_write(&path, b"new").unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "new");
        assert!(!temp_path(&path).exists());
        let names: Vec<_> = fs::read_dir(dir.path()).unwrap().collect();
        assert_eq!(names.len(), 1);
    }

    #[test]
    fn temporary_files_are_generated_files() {
        let temp = temp_path(Path::new("src/main.rs"));
        assert!(crate::scanner::is_generated_file(&temp));
    }
}
