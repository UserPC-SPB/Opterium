use std::fs;
use std::path::Path;

/// Settings from `navguard.json` in the project root. Missing or invalid
/// files give the defaults; unknown keys (such as `token_coeff`, used
/// before 1.3.0 when tokens were estimated per line) are ignored.
#[derive(Debug, Clone)]
pub struct Config {
    pub root: String,
    pub exclude: Vec<String>,
    pub timeout_sec: u64,
    pub triangle_path: String,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            root: ".".to_string(),
            exclude: vec![
                ".git".to_string(),
                "__pycache__".to_string(),
                "node_modules".to_string(),
                "target".to_string(),
                // The index of the zvec-grep search tool.
                ".zvec-grep".to_string(),
                ".navguard_cache.json".to_string(),
                "navigator.md".to_string(),
                "navigator_deep.md".to_string(),
                "navigator_deep2.md".to_string(),
            ],
            timeout_sec: 10800,
            triangle_path: "scripts/.abc_triangle_state.json".to_string(),
        }
    }
}

impl Config {
    pub fn load(path: &str) -> Self {
        let config_path = Path::new(path);
        if !config_path.exists() {
            return Config::default();
        }

        let content = match fs::read_to_string(config_path) {
            Ok(c) => c,
            Err(_) => return Config::default(),
        };
        // Notepad and PowerShell 5 write a BOM.
        let content = content.strip_prefix('\u{feff}').unwrap_or(&content);

        let parsed: serde_json::Value = match serde_json::from_str(content) {
            Ok(v) => v,
            Err(_) => return Config::default(),
        };

        let mut cfg = Config::default();

        if let Some(obj) = parsed.as_object() {
            if let Some(v) = obj.get("root").and_then(|x| x.as_str()) {
                cfg.root = v.to_string();
            }
            if let Some(v) = obj.get("timeout_sec").and_then(|x| x.as_u64()) {
                cfg.timeout_sec = v;
            }
            if let Some(v) = obj.get("triangle_path").and_then(|x| x.as_str()) {
                cfg.triangle_path = v.to_string();
            }
            if let Some(arr) = obj.get("exclude").and_then(|x| x.as_array()) {
                cfg.exclude = arr
                    .iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect();
            }
        }

        cfg
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_a_config_written_with_a_bom() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("navguard.json");
        fs::write(
            &path,
            "\u{feff}{\"exclude\": [\"build\"], \"timeout_sec\": 5}",
        )
        .unwrap();
        let cfg = Config::load(&path.to_string_lossy());
        assert_eq!(cfg.exclude, ["build"]);
        assert_eq!(cfg.timeout_sec, 5);
    }
}
