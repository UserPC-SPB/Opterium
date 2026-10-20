use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub struct TriangleState {
    pub a: u8,
    pub b: u8,
    pub c: u8,
    pub transitions: u64,
    pub timestamp: String,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct TriangleLogEntry {
    pub timestamp: String,
    pub action: String,
    pub from: Vec<u8>,
    pub to: Vec<u8>,
}

/// Transition log used by [`run_triangle_action`], relative to the current
/// directory.
pub const DEFAULT_LOG_PATH: &str = "scripts/.abc_triangle_log.json";

pub fn read_triangle(path: &str) -> TriangleState {
    let path = Path::new(path);
    if !path.exists() {
        return TriangleState {
            c: 1,
            transitions: 0,
            ..Default::default()
        };
    }
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return TriangleState::default(),
    };
    // Notepad and PowerShell 5 write a BOM.
    let content = content.strip_prefix('\u{feff}').unwrap_or(&content);
    let parsed: serde_json::Value = match serde_json::from_str(content) {
        Ok(v) => v,
        Err(_) => return TriangleState::default(),
    };
    TriangleState {
        a: parsed.get("A").and_then(|v| v.as_u64()).unwrap_or(0) as u8,
        b: parsed.get("B").and_then(|v| v.as_u64()).unwrap_or(0) as u8,
        c: parsed.get("C").and_then(|v| v.as_u64()).unwrap_or(0) as u8,
        transitions: parsed
            .get("transitions")
            .and_then(|v| v.as_u64())
            .unwrap_or(0),
        timestamp: parsed
            .get("timestamp")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
    }
}

/// Writes the state; `timestamp` is the current local time in ISO 8601
/// with the offset, like `Updated:` of the navigators.
pub fn write_triangle(path: &str, state: &TriangleState) {
    let json = serde_json::json!({
        "A": state.a,
        "B": state.b,
        "C": state.c,
        "transitions": state.transitions,
        "timestamp": now_iso(),
    });
    if let Some(parent) = Path::new(path).parent() {
        fs::create_dir_all(parent).ok();
    }
    fs::write(path, serde_json::to_string_pretty(&json).unwrap()).ok();
}

pub fn write_triangle_log(path: &str, entry: &TriangleLogEntry) {
    let mut log = read_triangle_log_internal(path);
    log.push(entry.clone());
    if log.len() > 50 {
        log = log[log.len() - 50..].to_vec();
    }
    let json: Vec<serde_json::Value> = log
        .iter()
        .map(|e| {
            serde_json::json!({
                "timestamp": e.timestamp,
                "action": e.action,
                "from": e.from,
                "to": e.to,
            })
        })
        .collect();
    if let Some(parent) = Path::new(path).parent() {
        fs::create_dir_all(parent).ok();
    }
    fs::write(path, serde_json::to_string_pretty(&json).unwrap()).ok();
}

fn read_triangle_log_internal(path: &str) -> Vec<TriangleLogEntry> {
    let path = Path::new(path);
    if !path.exists() {
        return Vec::new();
    }
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    let content = if content.starts_with('\u{feff}') {
        &content['\u{feff}'.len_utf8()..]
    } else {
        &content
    };
    let arr: Vec<serde_json::Value> = match serde_json::from_str(content) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };
    arr.iter()
        .map(|v| TriangleLogEntry {
            timestamp: v
                .get("timestamp")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string(),
            action: v
                .get("action")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string(),
            from: v
                .get("from")
                .and_then(|x| x.as_array())
                .map(|a| {
                    a.iter()
                        .filter_map(|n| n.as_u64().map(|n| n as u8))
                        .collect()
                })
                .unwrap_or_default(),
            to: v
                .get("to")
                .and_then(|x| x.as_array())
                .map(|a| {
                    a.iter()
                        .filter_map(|n| n.as_u64().map(|n| n as u8))
                        .collect()
                })
                .unwrap_or_default(),
        })
        .collect()
}

/// `2026-10-19T15:04:05+03:00` (before 1.4.0: Unix seconds).
fn now_iso() -> String {
    crate::navigator::now()
}

fn log_transition(log_path: &str, action: &str, from: (u8, u8, u8), to: (u8, u8, u8)) {
    write_triangle_log(
        log_path,
        &TriangleLogEntry {
            timestamp: now_iso(),
            action: action.to_string(),
            from: vec![from.0, from.1, from.2],
            to: vec![to.0, to.1, to.2],
        },
    );
}

/// Runs a triangle action on the state file `state_path`, logging
/// transitions to [`DEFAULT_LOG_PATH`].
pub fn run_triangle_action(state_path: &str, action: &str) -> String {
    run_triangle_action_with_log(state_path, DEFAULT_LOG_PATH, action)
}

/// [`run_triangle_action`] with an explicit transition log path.
pub fn run_triangle_action_with_log(state_path: &str, log_path: &str, action: &str) -> String {
    match action {
        "status" => {
            let state = read_triangle(state_path);
            format!(
                "B: triangle|status|[{},{},{}]|transitions:{}",
                state.a, state.b, state.c, state.transitions
            )
        }
        "step_done" => {
            let state = read_triangle(state_path);
            if state.c != 1 {
                return format!(
                    "B: triangle|error|cannot step_done from [{},{},{}] (1 must be at C)",
                    state.a, state.b, state.c
                );
            }
            let from = (state.a, state.b, state.c);
            let new = TriangleState {
                a: 1,
                b: 0,
                c: 0,
                transitions: state.transitions + 1,
                timestamp: now_iso(),
            };
            log_transition(log_path, "step_done", from, (1, 0, 0));
            write_triangle(state_path, &new);
            format!(
                "B: triangle|step_done|[{},{},{}] -> [1,0,0]",
                from.0, from.1, from.2
            )
        }
        "verify_start" => {
            let state = read_triangle(state_path);
            if state.a != 1 {
                return format!(
                    "B: triangle|error|cannot verify_start from [{},{},{}] (1 must be at A)",
                    state.a, state.b, state.c
                );
            }
            let from = (state.a, state.b, state.c);
            let new = TriangleState {
                a: 0,
                b: 1,
                c: 0,
                transitions: state.transitions + 1,
                timestamp: now_iso(),
            };
            log_transition(log_path, "verify_start", from, (0, 1, 0));
            write_triangle(state_path, &new);
            format!(
                "B: triangle|verify_start|[{},{},{}] -> [0,1,0]",
                from.0, from.1, from.2
            )
        }
        "verify_complete" => {
            let state = read_triangle(state_path);
            if state.b != 1 {
                return format!(
                    "B: triangle|error|cannot verify_complete from [{},{},{}] (1 must be at B)",
                    state.a, state.b, state.c
                );
            }
            let from = (state.a, state.b, state.c);
            let new = TriangleState {
                a: 0,
                b: 0,
                c: 1,
                transitions: state.transitions + 1,
                timestamp: now_iso(),
            };
            log_transition(log_path, "verify_complete", from, (0, 0, 1));
            write_triangle(state_path, &new);
            format!(
                "B: triangle|verify_complete|[{},{},{}] -> [0,0,1]",
                from.0, from.1, from.2
            )
        }
        "verify_fail" => {
            let state = read_triangle(state_path);
            if state.b != 1 {
                return format!(
                    "B: triangle|error|cannot verify_fail from [{},{},{}] (1 must be at B)",
                    state.a, state.b, state.c
                );
            }
            let from = (state.a, state.b, state.c);
            let new = TriangleState {
                a: 0,
                b: 0,
                c: 1,
                transitions: state.transitions + 1,
                timestamp: now_iso(),
            };
            log_transition(log_path, "verify_fail", from, (0, 0, 1));
            write_triangle(state_path, &new);
            format!(
                "B: triangle|verify_fail|[{},{},{}] -> [0,0,1]",
                from.0, from.1, from.2
            )
        }
        "reset" => {
            let state = read_triangle(state_path);
            let from = (state.a, state.b, state.c);
            let new = TriangleState {
                a: 0,
                b: 0,
                c: 1,
                transitions: state.transitions + 1,
                timestamp: now_iso(),
            };
            log_transition(log_path, "reset", from, (0, 0, 1));
            write_triangle(state_path, &new);
            format!(
                "B: triangle|reset|[{},{},{}] -> [0,0,1]",
                from.0, from.1, from.2
            )
        }
        "log" => {
            let entries = read_triangle_log_internal(log_path);
            if entries.is_empty() {
                return "B: triangle|log|no entries".to_string();
            }
            let mut out = String::new();
            for e in &entries {
                out.push_str(&format!(
                    "B: triangle|log|{} {} [{:?}] -> [{:?}]\n",
                    e.timestamp, e.action, e.from, e.to
                ));
            }
            out.trim().to_string()
        }
        _ => format!("B: triangle|error|unknown action '{}'", action),
    }
}

pub fn read_triangle_log(path: &str) -> Vec<TriangleLogEntry> {
    read_triangle_log_internal(path)
}

#[allow(dead_code)]
fn status_string(state: &TriangleState) -> &'static str {
    if state.c == 1 {
        "IDLE"
    } else if state.a == 1 {
        "AI WORKING"
    } else if state.b == 1 {
        "HERMES READING"
    } else {
        "UNKNOWN"
    }
}

#[cfg(test)]
mod triangle_tests {
    use super::*;

    /// State and log paths inside a fresh temporary directory.
    struct Paths {
        _dir: tempfile::TempDir,
        state: String,
        log: String,
    }

    fn paths_with_state(content: Option<&str>) -> Paths {
        let dir = tempfile::tempdir().unwrap();
        let scripts = dir.path().join("scripts");
        let state = scripts.join(".abc_triangle_state.json");
        if let Some(content) = content {
            fs::create_dir_all(&scripts).unwrap();
            fs::write(&state, content).unwrap();
        }
        Paths {
            state: state.to_string_lossy().into_owned(),
            log: scripts
                .join(".abc_triangle_log.json")
                .to_string_lossy()
                .into_owned(),
            _dir: dir,
        }
    }

    const IDLE: &str = r#"{"A": 0, "B": 0, "C": 1, "transitions": 0, "timestamp": ""}"#;

    #[test]
    fn test_read_triangle_default_when_missing() {
        let p = paths_with_state(None);
        let state = read_triangle(&p.state);
        assert_eq!((state.a, state.b, state.c), (0, 0, 1));
    }

    #[test]
    fn test_read_triangle_parses_valid() {
        let p = paths_with_state(Some(
            r#"{"A": 1, "B": 0, "C": 0, "transitions": 5, "timestamp": "2026-01-01"}"#,
        ));
        let state = read_triangle(&p.state);
        assert_eq!(state.a, 1);
        assert_eq!(state.transitions, 5);
        assert_eq!(state.timestamp, "2026-01-01");
    }

    #[test]
    fn test_read_triangle_accepts_a_bom() {
        let p = paths_with_state(Some(
            "\u{feff}{\"A\": 0, \"B\": 1, \"C\": 0, \"transitions\": 3, \"timestamp\": \"\"}",
        ));
        let state = read_triangle(&p.state);
        assert_eq!((state.a, state.b, state.c, state.transitions), (0, 1, 0, 3));
    }

    #[test]
    fn test_read_triangle_log_empty_when_missing() {
        let p = paths_with_state(None);
        assert!(read_triangle_log(&p.log).is_empty());
    }

    #[test]
    fn test_read_triangle_log_parses() {
        let p = paths_with_state(None);
        fs::create_dir_all(Path::new(&p.log).parent().unwrap()).unwrap();
        fs::write(
            &p.log,
            "\u{feff}[
                {\"timestamp\":\"t1\",\"action\":\"verify_start\",\"from\":[1,0,0],\"to\":[0,1,0]},
                {\"timestamp\":\"t2\",\"action\":\"verify_complete\",\"from\":[0,1,0],\"to\":[0,0,1]}
            ]",
        )
        .unwrap();
        let log = read_triangle_log(&p.log);
        assert_eq!(log.len(), 2);
        assert_eq!(log[0].action, "verify_start");
        assert_eq!(log[1].action, "verify_complete");
        assert_eq!(log[1].to, [0, 0, 1]);
    }

    #[test]
    fn test_step_done_transitions() {
        let p = paths_with_state(Some(IDLE));
        let r = run_triangle_action_with_log(&p.state, &p.log, "step_done");
        assert!(r.contains("[0,0,1] -> [1,0,0]"));
        let state = read_triangle(&p.state);
        assert_eq!((state.a, state.c), (1, 0));
        let log = read_triangle_log(&p.log);
        assert_eq!(log.len(), 1);
        // ISO 8601 with the offset, like the navigators: 2026-10-19T15:04:05+03:00.
        for timestamp in [&state.timestamp, &log[0].timestamp] {
            assert!(
                chrono::DateTime::parse_from_rfc3339(timestamp).is_ok(),
                "{}",
                timestamp
            );
        }
    }

    #[test]
    fn test_verify_complete_flow() {
        let p = paths_with_state(Some(IDLE));
        run_triangle_action_with_log(&p.state, &p.log, "step_done");
        run_triangle_action_with_log(&p.state, &p.log, "verify_start");
        let r = run_triangle_action_with_log(&p.state, &p.log, "verify_complete");
        assert!(r.contains("[0,1,0] -> [0,0,1]"));
        let state = read_triangle(&p.state);
        assert_eq!(state.c, 1);
        assert_eq!(state.transitions, 3);
    }

    #[test]
    fn test_verify_fail_flow() {
        let p = paths_with_state(Some(IDLE));
        run_triangle_action_with_log(&p.state, &p.log, "step_done");
        run_triangle_action_with_log(&p.state, &p.log, "verify_start");
        let r = run_triangle_action_with_log(&p.state, &p.log, "verify_fail");
        assert!(r.contains("[0,1,0] -> [0,0,1]"));
        assert!(r.contains("verify_fail"));
        assert_eq!(read_triangle(&p.state).c, 1);
    }

    #[test]
    fn test_invalid_transitions() {
        let p = paths_with_state(Some(IDLE));
        let r = run_triangle_action_with_log(&p.state, &p.log, "verify_start");
        assert!(r.contains("error"));
        assert!(r.contains("must be at A"));
        let r = run_triangle_action_with_log(&p.state, &p.log, "bogus");
        assert_eq!(r, "B: triangle|error|unknown action 'bogus'");
        assert!(read_triangle_log(&p.log).is_empty());
    }

    #[test]
    fn test_reset() {
        let p = paths_with_state(Some(
            r#"{"A": 1, "B": 0, "C": 0, "transitions": 5, "timestamp": ""}"#,
        ));
        let r = run_triangle_action_with_log(&p.state, &p.log, "reset");
        assert!(r.contains("-> [0,0,1]"));
        let state = read_triangle(&p.state);
        assert_eq!((state.a, state.c, state.transitions), (0, 1, 6));
    }

    #[test]
    fn test_log_action() {
        let p = paths_with_state(Some(IDLE));
        assert_eq!(
            run_triangle_action_with_log(&p.state, &p.log, "log"),
            "B: triangle|log|no entries"
        );
        run_triangle_action_with_log(&p.state, &p.log, "step_done");
        let log_output = run_triangle_action_with_log(&p.state, &p.log, "log");
        assert!(log_output.contains("step_done"));
        assert_eq!(log_output.lines().count(), 1);
    }

    #[test]
    fn test_status_reads_state() {
        let p = paths_with_state(Some(IDLE));
        assert_eq!(
            run_triangle_action_with_log(&p.state, &p.log, "status"),
            "B: triangle|status|[0,0,1]|transitions:0"
        );
    }
}
