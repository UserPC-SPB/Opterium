//! Paths inside the project root.
//!
//! Every path navguard reads or writes for a command must lie inside the
//! project root: relative paths start at the root, absolute paths are
//! allowed when they point inside it. "Inside" is decided on the resolved
//! path (`..`, symlinks and junctions followed by the file system), so a
//! link in the project that points elsewhere is outside.

use std::fs;
use std::path::{Path, PathBuf};

/// `path` relative to `root` (absolute paths stay as they are).
pub fn join(root: &Path, path: &str) -> PathBuf {
    let p = Path::new(path);
    if p.is_absolute() {
        p.to_path_buf()
    } else {
        root.join(p)
    }
}

/// True when `path` exists and lies inside `root` (or is the root).
pub fn is_inside(path: &Path, root: &Path) -> bool {
    match (fs::canonicalize(path), fs::canonicalize(root)) {
        (Ok(path), Ok(root)) => path.starts_with(root),
        _ => false,
    }
}

/// The error for `path` outside `root` (shown absolute).
pub fn outside(path: &str, root: &Path) -> String {
    format!(
        "path is outside the project root {}: {}",
        display_root(root),
        path
    )
}

/// An existing file inside `root`: (path as joined, `/`-separated path
/// relative to the root).
pub fn existing_file(root: &Path, path: &str) -> Result<(PathBuf, String), String> {
    if path.trim().is_empty() {
        return Err("path must not be empty".to_string());
    }
    let full = join(root, path);
    let canon = fs::canonicalize(&full).map_err(|_| format!("file not found: {}", path))?;
    if !canon.is_file() {
        return Err(format!("not a file: {}", path));
    }
    let canon_root = fs::canonicalize(root)
        .map_err(|e| format!("cannot resolve project root {}: {}", root.display(), e))?;
    let rel = canon
        .strip_prefix(&canon_root)
        .map_err(|_| outside(path, root))?;
    Ok((full, rel.to_string_lossy().replace('\\', "/")))
}

/// A file to write inside `root`; it may not exist yet. Its nearest
/// existing ancestor must lie inside the root, and the part that does not
/// exist yet must not contain `..`. An existing path must be a file.
pub fn writable_file(root: &Path, path: &str) -> Result<PathBuf, String> {
    if path.trim().is_empty() {
        return Err("path must not be empty".to_string());
    }
    let full = join(root, path);
    if full.exists() {
        if !full.is_file() {
            return Err(format!("not a file: {}", path));
        }
        if !is_inside(&full, root) {
            return Err(outside(path, root));
        }
        return Ok(full);
    }
    // Climb to the nearest existing ancestor. file_name() is None for a
    // path ending in `..`, so a missing part never contains `..`.
    let mut existing = full.as_path();
    while !existing.exists() {
        match (existing.parent(), existing.file_name()) {
            (Some(parent), Some(_)) => existing = parent,
            _ => return Err(outside(path, root)),
        }
    }
    let existing = if existing.as_os_str().is_empty() {
        Path::new(".")
    } else {
        existing
    };
    if !is_inside(existing, root) {
        return Err(outside(path, root));
    }
    Ok(full)
}

/// The absolute form of `root` for display: resolved, without the `\\?\`
/// prefix Windows adds to resolved paths.
pub fn display_root(root: &Path) -> String {
    let resolved = fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());
    let text = resolved.to_string_lossy();
    if let Some(unc) = text.strip_prefix(r"\\?\UNC\") {
        format!(r"\\{}", unc)
    } else if let Some(local) = text.strip_prefix(r"\\?\") {
        local.to_string()
    } else {
        text.into_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layout() -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root");
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(root.join("src/a.rs"), "fn a() {}\n").unwrap();
        fs::write(dir.path().join("outside.txt"), "secret\n").unwrap();
        (dir, root)
    }

    #[test]
    fn existing_files_must_be_inside() {
        let (dir, root) = layout();
        let (_, rel) = existing_file(&root, "src/a.rs").unwrap();
        assert_eq!(rel, "src/a.rs");
        let abs = root.join("src/a.rs");
        assert!(existing_file(&root, &abs.to_string_lossy()).is_ok());
        let err = existing_file(&root, "../outside.txt").unwrap_err();
        assert!(err.contains("outside the project root"), "{}", err);
        let out = dir.path().join("outside.txt");
        assert!(existing_file(&root, &out.to_string_lossy()).is_err());
        assert!(existing_file(&root, "src/missing.rs")
            .unwrap_err()
            .contains("not found"));
        assert!(existing_file(&root, "src")
            .unwrap_err()
            .contains("not a file"));
    }

    #[test]
    fn writable_files_must_be_inside() {
        let (dir, root) = layout();
        assert!(writable_file(&root, "src/a.rs").is_ok());
        assert!(writable_file(&root, "new/dir/file.txt").is_ok());
        assert!(writable_file(&root, "../outside.txt").is_err());
        assert!(writable_file(&root, "../new.txt").is_err());
        assert!(writable_file(&root, "new/../../x.txt").is_err());
        let out = dir.path().join("elsewhere/x.txt");
        assert!(writable_file(&root, &out.to_string_lossy()).is_err());
        assert!(writable_file(&root, "src").is_err());
    }

    #[test]
    fn display_root_is_absolute_without_verbatim_prefix() {
        let (_dir, root) = layout();
        let shown = display_root(&root);
        assert!(Path::new(&shown).is_absolute(), "{}", shown);
        assert!(!shown.starts_with(r"\\?\"), "{}", shown);
        assert!(shown.ends_with("root"), "{}", shown);
    }
}
