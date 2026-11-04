//! Keeping file access inside a project root.
//!
//! A path is inside a root when, with `.`/`..`, symlinks and junctions
//! resolved by the file system, it lies under the root. A link inside the
//! root that points outside it is therefore outside.

use std::path::{Component, Path, PathBuf};

/// `path` with `..`, symlinks and junctions resolved; `None` when it does
/// not exist.
pub fn canonical(path: &Path) -> Option<PathBuf> {
    std::fs::canonicalize(path).ok()
}

/// True when `path` exists and lies inside `root` (or is the root).
pub fn is_inside(path: &Path, root: &Path) -> bool {
    match (canonical(path), canonical(root)) {
        (Some(path), Some(root)) => path.starts_with(root),
        _ => false,
    }
}

/// Checks an input pattern (file, directory or glob) before anything is
/// read, so that a pattern cannot make discovery walk outside `root`: its
/// folder part (the components before the first one with `*`, `?` or `[`)
/// must lie inside `root` when it exists, and no `..` may follow a
/// wildcard. Every file found must still be checked with [`is_inside`].
pub fn check_pattern(pattern: &str, root: &Path) -> Result<(), String> {
    let normalized = pattern.replace('\\', "/");
    let path = Path::new(&normalized);
    let mut base = PathBuf::new();
    let mut in_glob = false;
    for component in path.components() {
        let text = component.as_os_str().to_string_lossy();
        if !in_glob && text.contains(['*', '?', '[']) {
            in_glob = true;
        }
        if in_glob {
            if component == Component::ParentDir {
                return Err(format!(
                    "'..' after a wildcard is not allowed: it could reach outside the project root ({})",
                    pattern
                ));
            }
        } else {
            base.push(component);
        }
    }
    if base.as_os_str().is_empty() {
        base.push(".");
    }
    // A base that does not exist matches nothing; discovery reports that.
    if canonical(&base).is_some() && !is_inside(&base, root) {
        return Err(outside_message(pattern, root));
    }
    Ok(())
}

/// The error for a path outside the root (shown absolute).
pub fn outside_message(path: &str, root: &Path) -> String {
    format!(
        "path is outside the project root {}: {}",
        navguard::paths::display_root(root),
        path
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A scratch folder removed on drop.
    struct Scratch(PathBuf);

    impl Scratch {
        fn new(name: &str) -> Scratch {
            let dir =
                std::env::temp_dir().join(format!("atp_confine_{}_{}", name, std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(dir.join("root")).unwrap();
            Scratch(dir)
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn inside_and_outside() {
        let dir = Scratch::new("inside");
        let root = dir.0.join("root");
        std::fs::create_dir_all(root.join("sub")).unwrap();
        std::fs::write(root.join("sub/a.txt"), "a").unwrap();
        std::fs::write(dir.0.join("out.txt"), "o").unwrap();
        assert!(is_inside(&root.join("sub/a.txt"), &root));
        assert!(is_inside(&root, &root));
        assert!(!is_inside(&root.join("../out.txt"), &root));
        assert!(!is_inside(&dir.0.join("out.txt"), &root));
        assert!(!is_inside(&root.join("missing.txt"), &root));
    }

    #[test]
    fn patterns() {
        let dir = Scratch::new("patterns");
        let root = dir.0.join("root");
        std::fs::create_dir_all(root.join("src")).unwrap();
        let in_root = |p: &str| root.join(p).to_string_lossy().into_owned();
        assert!(check_pattern(&in_root("src/**/*.rs"), &root).is_ok());
        assert!(check_pattern(&in_root("*.md"), &root).is_ok());
        assert!(check_pattern(&in_root("missing/*.md"), &root).is_ok());
        assert!(check_pattern(&in_root("../*"), &root).is_err());
        assert!(check_pattern(&in_root("src/*/../../.."), &root).is_err());
        let outside = dir.0.join("*.txt");
        assert!(check_pattern(&outside.to_string_lossy(), &root).is_err());
    }
}
