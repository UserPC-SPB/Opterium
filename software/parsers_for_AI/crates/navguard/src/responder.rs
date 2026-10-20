use crate::config::Config;
use crate::scanner::{scan_directory, FileInfo};
use crate::triangle::{read_triangle, read_triangle_log, TriangleState};
use std::path::Path;

#[allow(dead_code)]
pub struct CheckResponse {
    pub output: String,
    pub triangle: TriangleState,
    pub files: Vec<FileInfo>,
}

pub fn run_check(config: &Config) -> CheckResponse {
    // Resolve triangle path
    let rel_triangle = if config.root == "." {
        config.triangle_path.clone()
    } else {
        format!("{}/{}", config.root, config.triangle_path)
    };
    let triangle = read_triangle(&rel_triangle);

    // Resolve log path (same dir as state file)
    let log_path = log_path_from_triangle(&rel_triangle);

    // Scan files
    let files = scan_directory(&config.root, &config.exclude);

    // Build prefix output
    let mut out = String::new();
    out.push_str(&format!(
        "T: {},{},{}\n",
        triangle.a, triangle.b, triangle.c
    ));

    for f in &files {
        out.push_str(&format!(
            "F: {}|{}|{}|{}\n",
            f.path, f.lines, f.tokens, f.sha
        ));
    }

    // Detect incomplete transitions
    let log = read_triangle_log(&log_path);
    let incomplete = detect_incomplete(&log);
    for inc in &incomplete {
        out.push_str(&format!("I: {}|{}\n", inc.index, inc.reason));
    }

    out.push_str(&format!("B: sync|{}\n", chrono_now()));

    CheckResponse {
        output: out,
        triangle,
        files,
    }
}

struct IncompleteEntry {
    index: usize,
    reason: String,
}

fn detect_incomplete(log: &[crate::triangle::TriangleLogEntry]) -> Vec<IncompleteEntry> {
    // Walk log; each verify_start should be followed by verify_complete/verify_fail.
    // Track unmatched verify_start indices.
    let mut unmatched: Vec<usize> = Vec::new();
    let mut matched: Vec<bool> = vec![false; log.len()];

    for (i, entry) in log.iter().enumerate() {
        match entry.action.as_str() {
            "verify_start" => {
                unmatched.push(i);
            }
            "verify_complete" | "verify_fail" => {
                // Match the most recent unmatched verify_start
                if let Some(start_idx) = unmatched.pop() {
                    matched[start_idx] = true;
                }
            }
            _ => {}
        }
    }

    // unmatched now holds indices of verify_start without a subsequent verify_complete/verify_fail
    unmatched
        .into_iter()
        .map(|i| {
            let ts = &log[i].timestamp;
            let idx = i;
            IncompleteEntry {
                index: idx,
                reason: format!("verify_start at {} never completed", ts),
            }
        })
        .collect()
}

pub fn log_path_from_triangle(triangle_path: &str) -> String {
    let p = Path::new(triangle_path);
    let parent = p.parent().unwrap_or(Path::new("."));
    parent
        .join(".abc_triangle_log.json")
        .to_string_lossy()
        .to_string()
}

fn chrono_now() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    format!("{}", now.as_secs())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::triangle::TriangleLogEntry;

    fn make_entry(timestamp: &str, action: &str) -> TriangleLogEntry {
        TriangleLogEntry {
            timestamp: timestamp.to_string(),
            action: action.to_string(),
            from: vec![],
            to: vec![],
        }
    }

    #[test]
    fn test_detect_incomplete_empty_log() {
        let log: Vec<TriangleLogEntry> = vec![];
        let inc = detect_incomplete(&log);
        assert!(inc.is_empty());
    }

    #[test]
    fn test_detect_incomplete_all_matched() {
        let log = vec![
            make_entry("t1", "step_done"),
            make_entry("t2", "verify_start"),
            make_entry("t3", "verify_complete"),
            make_entry("t4", "step_done"),
            make_entry("t5", "verify_start"),
            make_entry("t6", "verify_fail"),
        ];
        let inc = detect_incomplete(&log);
        assert!(inc.is_empty());
    }

    #[test]
    fn test_detect_incomplete_unmatched_verify_start() {
        let log = vec![
            make_entry("t1", "step_done"),
            make_entry("t2", "verify_start"),
            make_entry("t3", "verify_complete"),
            make_entry("t4", "step_done"),
            make_entry("t5", "verify_start"), // unmatched
        ];
        let inc = detect_incomplete(&log);
        assert_eq!(inc.len(), 1);
        assert_eq!(inc[0].index, 4);
    }

    #[test]
    fn test_detect_incomplete_multiple_unmatched() {
        let log = vec![
            make_entry("t1", "verify_start"),
            make_entry("t2", "step_done"),
            make_entry("t3", "verify_start"), // unmatched
        ];
        let inc = detect_incomplete(&log);
        assert_eq!(inc.len(), 2);
    }

    #[test]
    fn test_log_path_from_triangle() {
        let log = log_path_from_triangle("scripts/.abc_triangle_state.json");
        assert!(log.contains(".abc_triangle_log.json"));
        assert!(log.contains("scripts"));
    }

    #[test]
    fn test_check_output_format() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("main.rs"), "fn main() {}\n").unwrap();
        let cfg = Config {
            root: dir.path().to_string_lossy().into_owned(),
            ..Config::default()
        };
        let resp = run_check(&cfg);
        assert!(resp.output.starts_with("T: 0,0,1\n"));
        assert!(resp.output.contains("F: main.rs|1|5|"));
        assert!(resp.output.contains("B: sync|"));
        assert_eq!(resp.files.len(), 1);
    }

    #[test]
    fn test_check_reports_incomplete_transitions_from_the_root() {
        let dir = tempfile::tempdir().unwrap();
        let scripts = dir.path().join("scripts");
        std::fs::create_dir_all(&scripts).unwrap();
        std::fs::write(
            scripts.join(".abc_triangle_state.json"),
            r#"{"A": 0, "B": 1, "C": 0, "transitions": 2, "timestamp": ""}"#,
        )
        .unwrap();
        std::fs::write(
            scripts.join(".abc_triangle_log.json"),
            r#"[{"timestamp":"t1","action":"verify_start","from":[1,0,0],"to":[0,1,0]}]"#,
        )
        .unwrap();
        let cfg = Config {
            root: dir.path().to_string_lossy().into_owned(),
            exclude: vec!["scripts".to_string()],
            ..Config::default()
        };
        let resp = run_check(&cfg);
        assert!(resp.output.starts_with("T: 0,1,0\n"));
        assert!(resp
            .output
            .contains("I: 0|verify_start at t1 never completed\n"));
        assert!(resp.files.is_empty());
    }
}
