use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct ResourceLimits {
    pub max_input_files: Option<u64>,
    pub max_file_size_bytes: Option<u64>,
    pub max_memory_bytes: Option<u64>,
    pub max_output_chars: Option<u64>,
    pub max_block_chars: u64,
    pub target_block_chars: u64,
    pub max_matches_per_rule: Option<u64>,
}

/// Deepest XML element nesting accepted before parsing. The XML parser is
/// recursive, so deeper input would overflow the stack instead of failing as
/// a per-file error. The JSON, YAML and TOML parsers have their own limits
/// (127, 128 and 80 levels).
pub const MAX_XML_NESTING_DEPTH: usize = 128;

impl Default for ResourceLimits {
    fn default() -> Self {
        ResourceLimits {
            max_input_files: Some(5000),
            max_file_size_bytes: Some(200 * 1024 * 1024),
            max_memory_bytes: None,
            max_output_chars: None,
            max_block_chars: 32_000,
            target_block_chars: 28_000,
            max_matches_per_rule: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SizeUnit {
    Bytes,
    KiB,
    MiB,
    GiB,
}

pub fn parse_size(value: &str) -> Option<u64> {
    let v = value.trim();
    if v.is_empty() {
        return None;
    }
    let (num_part, unit) = split_number(v);
    let num: u64 = num_part.parse().ok()?;
    let mult: u64 = match unit.to_ascii_lowercase().as_str() {
        "" | "b" => 1,
        "k" | "kb" | "kib" => 1024,
        "m" | "mb" | "mib" => 1024 * 1024,
        "g" | "gb" | "gib" => 1024 * 1024 * 1024,
        _ => return None,
    };
    num.checked_mul(mult)
}

fn split_number(v: &str) -> (&str, &str) {
    let idx = v.find(|c: char| !c.is_ascii_digit()).unwrap_or(v.len());
    (&v[..idx], v[idx..].trim())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitCode {
    Success = 0,
    PartialSuccess = 1,
    InvalidTemplate = 2,
    InvalidInputOrCli = 3,
    OutputFailure = 4,
    InternalFatal = 5,
    ResourceLimit = 6,
}

impl ExitCode {
    pub fn to_i32(self) -> i32 {
        self as i32
    }
}

#[derive(Debug, Clone)]
pub struct CliOptions {
    pub input: String,
    pub template: PathBuf,
    pub output: Option<PathBuf>,
    pub format: Option<String>,
    pub encoding: Option<String>,
    pub dry_run: bool,
    pub verbose: bool,
    pub quiet: bool,
    pub follow_symlinks: bool,
    pub hash: bool,
    pub max_file_size: Option<String>,
    pub max_memory: Option<String>,
    pub max_files: Option<u64>,
    pub max_matches: Option<u64>,
    pub max_output_size: Option<String>,
    pub max_block_chars: Option<u64>,
    pub raw_args: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn size_parsing() {
        assert_eq!(parse_size("100"), Some(100));
        assert_eq!(parse_size("100KB"), Some(102400));
        assert_eq!(parse_size("2MB"), Some(2 * 1024 * 1024));
        assert_eq!(parse_size("1GiB"), Some(1024 * 1024 * 1024));
        assert_eq!(parse_size("abc"), None);
        assert_eq!(parse_size(""), None);
    }
}
