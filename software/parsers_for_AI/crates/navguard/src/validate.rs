use regex::Regex;
use std::fs;
use std::path::Path;

/// Checks that every anchor declared in the anchor map at `anchor_map_path`
/// opens and closes in its `File:`. Files are relative to `root` and must
/// lie inside it.
pub fn validate(root: &Path, anchor_map_path: &str) -> Vec<String> {
    let mut errors: Vec<String> = Vec::new();

    let map_text = match fs::read_to_string(anchor_map_path) {
        Ok(c) => c,
        Err(e) => {
            errors.push(format!("cannot read '{}': {}", anchor_map_path, e));
            return errors;
        }
    };

    let anchor_re = Regex::new(r"Anchor:\s*\[([^\]]+)\]").unwrap();
    let file_re = Regex::new(r"(?ms)##\s*\[([^\]]+)\].*?File:\s*(\S+)").unwrap();

    let declared: Vec<String> = anchor_re
        .captures_iter(&map_text)
        .map(|c| c[1].to_string())
        .collect();

    let file_map: std::collections::HashMap<String, String> = file_re
        .captures_iter(&map_text)
        .map(|c| (c[1].to_string(), c[2].to_string()))
        .collect();

    if declared.is_empty() {
        errors.push("no anchors found in anchor_map.md".to_string());
        return errors;
    }

    for anchor_id in &declared {
        let filepath = match file_map.get(anchor_id) {
            Some(f) => f,
            None => {
                errors.push(format!("no File field for [{}]", anchor_id));
                continue;
            }
        };

        let code = match crate::paths::existing_file(root, filepath)
            .and_then(|(full, _)| fs::read_to_string(full).map_err(|e| e.to_string()))
        {
            Ok(c) => c,
            Err(e) => {
                errors.push(format!("{} (anchor [{}])", e, anchor_id));
                continue;
            }
        };

        let open = format!("[{}]", anchor_id);
        let close = format!("[{}end]", anchor_id);

        if !code.contains(&open) {
            errors.push(format!("anchor [{}] missing in {}", anchor_id, filepath));
        }
        if !code.contains(&close) {
            errors.push(format!(
                "closing anchor [{}end] missing in {}",
                anchor_id, filepath
            ));
        }
    }

    errors
}
