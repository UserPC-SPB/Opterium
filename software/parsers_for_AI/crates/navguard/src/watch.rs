use notify::{Config, Event, PollWatcher, RecursiveMode, Watcher};
use std::path::Path;
use std::sync::mpsc;
use std::time::Duration;

pub struct FileWatcher {
    rx: mpsc::Receiver<Event>,
    _watcher: PollWatcher,
    root: String,
}

impl FileWatcher {
    pub fn new(root: &str) -> Result<Self, String> {
        let (tx, rx) = mpsc::channel();
        let root_path = std::fs::canonicalize(root)
            .map_err(|e| format!("Failed to resolve watch path: {}", e))?;

        let mut watcher = PollWatcher::new(
            move |res| {
                if let Ok(event) = res {
                    let _ = tx.send(event);
                }
            },
            Config::default()
                .with_poll_interval(Duration::from_secs(2))
                .with_compare_contents(true),
        )
        .map_err(|e| format!("Failed to create watcher: {}", e))?;

        watcher
            .watch(&root_path, RecursiveMode::Recursive)
            .map_err(|e| format!("Failed to watch path: {}", e))?;

        Ok(FileWatcher {
            rx,
            _watcher: watcher,
            root: root_path.to_string_lossy().into_owned(),
        })
    }

    pub fn wait_for_event(&self) -> Option<Event> {
        match self.rx.recv_timeout(Duration::from_secs(10)) {
            Ok(event) => Some(event),
            Err(mpsc::RecvTimeoutError::Timeout) => None,
            Err(mpsc::RecvTimeoutError::Disconnected) => None,
        }
    }

    pub fn wait_for_changes(&self) -> Option<Vec<String>> {
        self.wait_for_changes_for(Duration::from_secs(10))
    }

    pub fn wait_for_changes_for(&self, timeout: Duration) -> Option<Vec<String>> {
        let mut changed_files = Vec::new();
        let deadline = std::time::Instant::now() + timeout.min(Duration::from_secs(10));

        while std::time::Instant::now() < deadline {
            match self.rx.recv_timeout(Duration::from_millis(100)) {
                Ok(event) => {
                    for path in &event.paths {
                        if let Ok(rel) = path.strip_prefix(&self.root) {
                            let rel_str = rel.to_string_lossy().replace('\\', "/");
                            // Skip navigator files themselves to avoid loops
                            if !is_navigator_file(&rel_str) {
                                changed_files.push(rel_str);
                            }
                        }
                    }
                    // Collect more events for a bit
                    continue;
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    if !changed_files.is_empty() {
                        break;
                    }
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }

        if changed_files.is_empty() {
            None
        } else {
            // Deduplicate
            changed_files.sort();
            changed_files.dedup();
            Some(changed_files)
        }
    }
}

pub fn extract_changed_files(event: &Event, root: &str) -> Vec<String> {
    let mut files = Vec::new();
    for path in &event.paths {
        if let Ok(rel) = path.strip_prefix(root) {
            let rel_str = rel.to_string_lossy().replace('\\', "/");
            files.push(rel_str);
        }
    }
    files
}

pub fn is_navigator_file(path: &str) -> bool {
    crate::scanner::is_generated_file(Path::new(path))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ignores_generated_outputs_but_watches_project_scripts_and_config() {
        assert!(is_navigator_file("navigator.md"));
        assert!(is_navigator_file("sub/navigator_deep2.md"));
        assert!(is_navigator_file(".navguard_cache.json"));
        assert!(!is_navigator_file("scripts/build.py"));
        assert!(!is_navigator_file("Cargo.toml"));
        assert!(!is_navigator_file("navigator_helpers.rs"));
        assert!(!is_navigator_file("navguard.json"));
    }

    #[test]
    fn starts_with_an_existing_root_and_respects_a_zero_wait() {
        let dir = tempfile::tempdir().unwrap();
        let watcher = FileWatcher::new(dir.path().to_str().unwrap()).unwrap();
        let expected = std::fs::canonicalize(dir.path()).unwrap();
        assert_eq!(Path::new(&watcher.root), expected);
        assert!(watcher.wait_for_changes_for(Duration::ZERO).is_none());
    }
}
