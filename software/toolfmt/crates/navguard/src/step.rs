use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

/// Single entry in step.json history
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StepEntry {
    pub file: String,
    pub step: [i32; 3],
    pub ts: u64,
}

/// The full step.json structure
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StepJson {
    pub history: Vec<StepEntry>,
}

/// Default history location, relative to the current directory. The
/// functions without an `_at` suffix use it; the `_at` variants take an
/// explicit path.
pub const STEP_JSON_PATH: &str = "scripts/.navguard_step.json";
const HISTORY_LIMIT: usize = 64;
const CYCLE_THRESHOLD: usize = 3;
const SESSION_TIMEOUT_SECS: u64 = 60;

/// Current timestamp in seconds since UNIX_EPOCH
fn now_ts() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn default_path() -> &'static Path {
    Path::new(STEP_JSON_PATH)
}

/// Load step.json from disk, return its history.
/// If the file doesn't exist or can't be parsed, return an empty Vec.
pub fn load_step_json() -> Vec<StepEntry> {
    load_step_json_at(default_path())
}

/// [`load_step_json`] for the history file at `path`.
pub fn load_step_json_at(path: &Path) -> Vec<StepEntry> {
    if !path.exists() {
        return Vec::new();
    }
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    let parsed: StepJson = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };
    parsed.history
}

/// Save entries to step.json on disk.
pub fn save_step_json(entries: Vec<StepEntry>) {
    save_step_json_at(default_path(), entries)
}

/// [`save_step_json`] for the history file at `path` (parent directories
/// are created).
pub fn save_step_json_at(path: &Path, entries: Vec<StepEntry>) {
    let step_json = StepJson { history: entries };
    let content = match serde_json::to_string_pretty(&step_json) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("navguard: error serializing step.json: {}", e);
            return;
        }
    };
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            let _ = fs::create_dir_all(parent);
        }
    }
    if let Err(e) = fs::write(path, content) {
        eprintln!("navguard: error writing step.json: {}", e);
    }
}

/// Add a single new entry to step.json (load, append, save).
pub fn add_step_entry(file: &str, step: [i32; 3]) {
    add_step_entry_at(default_path(), file, step)
}

/// [`add_step_entry`] for the history file at `path`.
pub fn add_step_entry_at(path: &Path, file: &str, step: [i32; 3]) {
    let mut history = load_step_json_at(path);
    history.push(StepEntry {
        file: file.to_string(),
        step,
        ts: now_ts(),
    });
    // Keep only the last HISTORY_LIMIT entries
    if history.len() > HISTORY_LIMIT {
        history.drain(..history.len() - HISTORY_LIMIT);
    }
    save_step_json_at(path, history);
}

/// Return files that are currently "active" - i.e. their last recorded step
/// is [1,0,0] or [0,1,0].
pub fn get_active_files() -> Vec<String> {
    get_active_files_at(default_path())
}

/// [`get_active_files`] for the history file at `path`.
pub fn get_active_files_at(path: &Path) -> Vec<String> {
    let history = load_step_json_at(path);
    // Build a map: file -> latest step
    let mut latest: HashMap<String, [i32; 3]> = HashMap::new();
    for entry in &history {
        latest.insert(entry.file.clone(), entry.step);
    }
    let mut active: Vec<String> = latest
        .into_iter()
        .filter(|(_, step)| *step == [1, 0, 0] || *step == [0, 1, 0])
        .map(|(file, _)| file)
        .collect();
    active.sort();
    active
}

/// Detect cycles: if any file appears 3+ times in the last 64 records,
/// return the file path of the first such file. Returns None if no cycle.
pub fn detect_cycle() -> Option<String> {
    detect_cycle_at(default_path())
}

/// [`detect_cycle`] for the history file at `path`.
pub fn detect_cycle_at(path: &Path) -> Option<String> {
    let history = load_step_json_at(path);
    if history.len() < CYCLE_THRESHOLD {
        return None;
    }
    // Count occurrences in the last 64 records
    let limit = std::cmp::min(history.len(), HISTORY_LIMIT);
    let recent = &history[history.len() - limit..];
    let mut counts: HashMap<String, usize> = HashMap::new();
    for entry in recent {
        *counts.entry(entry.file.clone()).or_insert(0) += 1;
    }
    counts
        .into_iter()
        .find(|(_, count)| *count >= CYCLE_THRESHOLD)
        .map(|(file, _)| file)
}

/// Check if the last recorded step indicates an interrupted session.
/// A session is considered broken if:
/// - There is at least one entry in history,
/// - The last entry's step is [1,0,0] or [0,1,0],
/// - And the time since the last entry exceeds SESSION_TIMEOUT_SECS (60s).
pub fn is_session_broken() -> bool {
    is_session_broken_at(default_path())
}

/// [`is_session_broken`] for the history file at `path`.
pub fn is_session_broken_at(path: &Path) -> bool {
    let history = load_step_json_at(path);
    let Some(last) = history.last() else {
        return false;
    };
    let is_active_step = last.step == [1, 0, 0] || last.step == [0, 1, 0];
    if !is_active_step {
        return false;
    }
    let elapsed = now_ts().saturating_sub(last.ts);
    elapsed > SESSION_TIMEOUT_SECS
}

/// Log a session-broken event and synchronize the file.
/// This returns true if a broken session was detected and logged.
pub fn check_and_log_broken_session() -> bool {
    check_and_log_broken_session_at(default_path())
}

/// [`check_and_log_broken_session`] for the history file at `path`.
pub fn check_and_log_broken_session_at(path: &Path) -> bool {
    if !is_session_broken_at(path) {
        return false;
    }
    let history = load_step_json_at(path);
    if let Some(last) = history.last() {
        eprintln!(
            "navguard: session broken for file '{}' (last step: {:?}, {}s ago)",
            last.file,
            last.step,
            now_ts().saturating_sub(last.ts)
        );
    }
    // Synchronize: save the current state to ensure file is up-to-date
    save_step_json_at(path, history);
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// A history path inside a fresh temporary directory (the file itself
    /// does not exist yet; `scripts/` is created on first save).
    fn temp_history() -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("scripts").join(".navguard_step.json");
        (dir, path)
    }

    fn entry(file: &str, step: [i32; 3], ts: u64) -> StepEntry {
        StepEntry {
            file: file.to_string(),
            step,
            ts,
        }
    }

    #[test]
    fn test_load_empty() {
        let (_dir, path) = temp_history();
        assert!(load_step_json_at(&path).is_empty());
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "not json").unwrap();
        assert!(load_step_json_at(&path).is_empty());
    }

    #[test]
    fn test_load_and_save_roundtrip() {
        let (_dir, path) = temp_history();
        let entries = vec![
            entry("/src/main.rs", [1, 0, 0], 1710000001),
            entry("/src/lib.rs", [0, 1, 0], 1710000002),
        ];
        save_step_json_at(&path, entries.clone());
        assert_eq!(load_step_json_at(&path), entries);
    }

    #[test]
    fn test_add_step_entry() {
        let (_dir, path) = temp_history();
        add_step_entry_at(&path, "/src/main.rs", [1, 0, 0]);
        let history = load_step_json_at(&path);
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].file, "/src/main.rs");
        assert_eq!(history[0].step, [1, 0, 0]);
    }

    #[test]
    fn test_history_limit() {
        let (_dir, path) = temp_history();
        for i in 0..100 {
            add_step_entry_at(&path, &format!("/src/file{}.rs", i), [1, 0, 0]);
        }
        let history = load_step_json_at(&path);
        assert_eq!(history.len(), HISTORY_LIMIT);
        assert_eq!(history.last().unwrap().file, "/src/file99.rs");
    }

    #[test]
    fn test_get_active_files() {
        let (_dir, path) = temp_history();
        save_step_json_at(
            &path,
            vec![
                entry("/src/main.rs", [1, 0, 0], 1710000001),
                entry("/src/lib.rs", [0, 1, 0], 1710000002),
                entry("/src/other.rs", [1, 1, 0], 1710000003),
            ],
        );
        assert_eq!(get_active_files_at(&path), ["/src/lib.rs", "/src/main.rs"]);
    }

    #[test]
    fn test_detect_cycle() {
        let (_dir, path) = temp_history();
        for _ in 0..3 {
            add_step_entry_at(&path, "/src/main.rs", [1, 0, 0]);
        }
        assert_eq!(detect_cycle_at(&path).as_deref(), Some("/src/main.rs"));
    }

    #[test]
    fn test_detect_no_cycle() {
        let (_dir, path) = temp_history();
        for i in 0..3 {
            add_step_entry_at(&path, &format!("/src/file{}.rs", i), [1, 0, 0]);
        }
        assert!(detect_cycle_at(&path).is_none());
    }

    #[test]
    fn test_detect_cycle_threshold() {
        let (_dir, path) = temp_history();
        add_step_entry_at(&path, "/src/main.rs", [1, 0, 0]);
        add_step_entry_at(&path, "/src/main.rs", [1, 0, 0]);
        assert!(detect_cycle_at(&path).is_none());
    }

    #[test]
    fn test_session_broken_old() {
        let (_dir, path) = temp_history();
        let old_ts = now_ts() - SESSION_TIMEOUT_SECS - 10;
        save_step_json_at(&path, vec![entry("/src/main.rs", [1, 0, 0], old_ts)]);
        assert!(is_session_broken_at(&path));
    }

    #[test]
    fn test_session_not_broken_recent() {
        let (_dir, path) = temp_history();
        save_step_json_at(&path, vec![entry("/src/main.rs", [1, 0, 0], now_ts() - 10)]);
        assert!(!is_session_broken_at(&path));
    }

    #[test]
    fn test_session_not_broken_non_active_step() {
        let (_dir, path) = temp_history();
        let old_ts = now_ts() - SESSION_TIMEOUT_SECS - 10;
        save_step_json_at(&path, vec![entry("/src/main.rs", [1, 1, 0], old_ts)]);
        assert!(!is_session_broken_at(&path));
        assert!(!is_session_broken_at(&path.with_file_name("missing.json")));
    }

    #[test]
    fn test_check_and_log_broken_session() {
        let (_dir, path) = temp_history();
        let old_ts = now_ts() - SESSION_TIMEOUT_SECS - 10;
        save_step_json_at(&path, vec![entry("/src/main.rs", [1, 0, 0], old_ts)]);
        assert!(check_and_log_broken_session_at(&path));
        assert_eq!(load_step_json_at(&path).len(), 1);
    }

    #[test]
    fn test_check_session_not_broken() {
        let (_dir, path) = temp_history();
        save_step_json_at(&path, vec![entry("/src/main.rs", [1, 0, 0], now_ts() - 10)]);
        assert!(!check_and_log_broken_session_at(&path));
    }

    #[test]
    fn default_functions_use_the_relative_default_path() {
        assert_eq!(default_path(), Path::new("scripts/.navguard_step.json"));
    }
}
