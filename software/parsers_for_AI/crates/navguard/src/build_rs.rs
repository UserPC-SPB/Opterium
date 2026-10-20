use crate::paths;
use regex::Regex;
use std::fs;
use std::path::Path;

/// Assembles the files of the plan at `plan_path`. The plan, every source
/// and every target lie inside `root`: the plan path and targets are
/// relative to the root, sources relative to the plan's folder (or to the
/// root when not found there).
pub fn build(root: &Path, plan_path: &str) -> Result<Vec<String>, String> {
    let (plan_full, plan_rel) = paths::existing_file(root, plan_path)?;
    let plan_text = fs::read_to_string(&plan_full)
        .map_err(|e| format!("cannot read '{}': {}", plan_path, e))?;
    let plan_dir = Path::new(&plan_rel)
        .parent()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default();

    let lines = parse_assembly_order(&plan_text)?;
    let mut assembled: Vec<String> = Vec::new();

    for (src_file, anchor_id, target_file) in &lines {
        let beside_plan = Path::new(&plan_dir).join(src_file);
        let (src_full, _) = paths::existing_file(root, &beside_plan.to_string_lossy())
            .or_else(|_| paths::existing_file(root, src_file))?;
        let block = extract_block(&src_full.to_string_lossy(), anchor_id)?;

        let target = paths::writable_file(root, target_file)?;
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("cannot create dir: {}", e))?;
        }

        let existing = fs::read_to_string(&target).unwrap_or_default();
        let new_content = if existing.is_empty() {
            block
        } else {
            format!("{}\n\n{}", existing, block)
        };

        crate::write::atomic_write(&target, new_content.as_bytes())
            .map_err(|e| format!("cannot write '{}': {}", target_file, e))?;

        assembled.push(target_file.clone());
    }

    Ok(assembled)
}

pub fn extract_block(filepath: &str, anchor_id: &str) -> Result<String, String> {
    let mut text =
        fs::read_to_string(filepath).map_err(|e| format!("cannot read '{}': {}", filepath, e))?;

    // Strip UTF-8 BOM
    if text.starts_with('\u{feff}') {
        text = text['\u{feff}'.len_utf8()..].to_string();
    }

    let open = format!(r"\[{}\]", regex::escape(anchor_id));
    let close = format!(r"\[{}end\]", regex::escape(anchor_id));
    let pattern = format!(r"(?s){}\n(.*?)\n{}", open, close);

    let re = Regex::new(&pattern).map_err(|e| format!("regex error: {}", e))?;

    let block = match re.captures(&text) {
        Some(cap) => cap[1].trim().to_string(),
        None => {
            return Err(format!("anchor [{}] not found in {}", anchor_id, filepath));
        }
    };

    let strip_re = Regex::new(r"(?m)^\s*\[s[\w-]+(?:end)?\]\s*$").unwrap();
    let cleaned = strip_re.replace_all(&block, "").trim().to_string();

    Ok(cleaned)
}

fn parse_assembly_order(plan_text: &str) -> Result<Vec<(String, String, String)>, String> {
    // Find "## Assembly Order\n" and capture everything until the next "## " or end of string
    let start_marker = "## Assembly Order\n";
    let start = plan_text
        .find(start_marker)
        .ok_or_else(|| "Assembly Order section not found in plan".to_string())?;
    let start = start + start_marker.len();

    // Find next "## " heading or end of string
    let end = plan_text[start..]
        .find("\n## ")
        .map(|pos| start + pos + 1)
        .unwrap_or(plan_text.len());

    let body = plan_text[start..end].trim();
    let line_re = Regex::new(r"^(\S+)\s+\[([^\]]+)\]\s*→\s*(\S+)").unwrap();

    let mut result: Vec<(String, String, String)> = Vec::new();
    for line in body.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Some(cap) = line_re.captures(trimmed) {
            result.push((cap[1].to_string(), cap[2].to_string(), cap[3].to_string()));
        }
    }

    if result.is_empty() {
        return Err("no valid assembly lines found in Assembly Order section".to_string());
    }

    Ok(result)
}

#[cfg(test)]
mod build_tests {
    use super::*;

    #[test]
    fn test_parse_assembly_order_valid() {
        let text = "\
## Assembly Order
blocks/module.md [s1-1] → src/main.py
blocks/utils.md [s1-2] → src/utils.py
## Next Plan
";
        let result = parse_assembly_order(text).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].0, "blocks/module.md");
        assert_eq!(result[0].1, "s1-1");
        assert_eq!(result[0].2, "src/main.py");
    }

    #[test]
    fn test_parse_assembly_order_missing_section() {
        let text = "# Plan\nno assembly here\n";
        let result = parse_assembly_order(text);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[test]
    fn test_extract_block_found() {
        let dir = tempfile::tempdir().unwrap();
        let filepath = dir.path().join("test.md").to_string_lossy().to_string();
        fs::write(
            &filepath,
            "\
[s1-1]
def hello():
    pass
[s1-1end]
",
        )
        .unwrap();

        let block = extract_block(&filepath, "s1-1").unwrap();
        assert_eq!(block, "def hello():\n    pass");
    }

    #[test]
    fn test_extract_block_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let filepath = dir.path().join("test.md").to_string_lossy().to_string();
        fs::write(&filepath, "no anchors here").unwrap();

        let result = extract_block(&filepath, "s1-99");
        assert!(result.is_err());
    }

    #[test]
    fn test_build_assembles_targets_from_a_plan() {
        let dir = tempfile::tempdir().unwrap();
        let blocks = dir.path().join("blocks");
        fs::create_dir_all(&blocks).unwrap();
        fs::write(blocks.join("a.md"), "[s1]\nfn a() {}\n[s1end]\n").unwrap();
        let target = dir.path().join("out").join("lib.rs");
        let plan = dir.path().join("plan.md");
        fs::write(
            &plan,
            format!(
                "## Assembly Order\nblocks/a.md [s1] → {}\n",
                target.to_string_lossy().replace('\\', "/")
            ),
        )
        .unwrap();
        let built = build(dir.path(), "plan.md").unwrap();
        assert_eq!(built.len(), 1);
        assert_eq!(fs::read_to_string(&target).unwrap(), "fn a() {}");
    }

    #[test]
    fn test_build_refuses_targets_and_sources_outside_the_root() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root");
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("a.md"), "[s1]\nfn a() {}\n[s1end]\n").unwrap();
        fs::write(dir.path().join("outside.md"), "[s1]\nsecret\n[s1end]\n").unwrap();
        let escape = dir.path().join("escaped.rs");
        fs::write(
            root.join("plan.md"),
            format!(
                "## Assembly Order\na.md [s1] → {}\n",
                escape.to_string_lossy().replace('\\', "/")
            ),
        )
        .unwrap();
        let error = build(&root, "plan.md").unwrap_err();
        assert!(error.contains("outside the project root"), "{}", error);
        assert!(!escape.exists());

        fs::write(
            root.join("plan2.md"),
            "## Assembly Order\n../outside.md [s1] → out.rs\n",
        )
        .unwrap();
        let error = build(&root, "plan2.md").unwrap_err();
        assert!(error.contains("outside the project root"), "{}", error);
        assert!(!root.join("out.rs").exists());
    }
}
