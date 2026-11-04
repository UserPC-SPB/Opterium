use crate::parser::{parse_source, Anchor};
use crate::scanner::{self, FileInfo};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FileCache {
    pub sha: String,
    pub lines: u64,
    pub tokens: u64,
    pub chunk_hashes: Vec<String>,
    pub chunk_size: usize,
    /// Size in bytes (0 in caches written before 1.3.0).
    #[serde(default)]
    pub bytes: u64,
    /// The file is binary (see [`crate::text::is_binary`]).
    #[serde(default)]
    pub binary: bool,
}

impl FileCache {
    /// The scan description of this entry.
    pub fn info(&self, path: &str) -> FileInfo {
        FileInfo {
            path: path.to_string(),
            bytes: self.bytes,
            lines: self.lines,
            tokens: self.tokens,
            sha: self.sha.clone(),
            binary: self.binary,
        }
    }
}

/// One indexed symbol. `name` holds the label (the trimmed first line of the
/// definition) for compatibility with caches written before the symbol
/// version 2; the other fields were added in version 2.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SymbolEntry {
    pub path: String,
    pub name: String,
    pub line: usize,
    /// The identifier (`new`, `Point`, `[s1-1]`).
    #[serde(default)]
    pub ident: String,
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub line_end: usize,
    #[serde(default)]
    pub depth: usize,
    #[serde(default)]
    pub parent: Option<String>,
    #[serde(default)]
    pub tokens: usize,
}

/// Version of the symbol indexer that produced `NavguardCache::symbols`.
/// Symbols from another version are never reused for unchanged files.
/// 3: line breaks normalized before indexing, byte-based token estimates.
pub const SYMBOLS_VERSION: u32 = 3;

/// Converts parser output for the file at `path` (relative) into cache entries.
pub fn symbol_entries(path: &str, anchors: &[Anchor]) -> Vec<SymbolEntry> {
    anchors
        .iter()
        .map(|a| SymbolEntry {
            path: path.to_string(),
            name: a.label.clone(),
            line: a.line_start,
            ident: a.name.clone(),
            kind: a.kind.to_string(),
            line_end: a.line_end,
            depth: a.depth,
            parent: a.parent.clone(),
            tokens: a.tokens,
        })
        .collect()
}

fn sort_symbols(symbols: &mut [SymbolEntry]) {
    symbols.sort_by(|a, b| (&a.path, a.line, &a.name).cmp(&(&b.path, b.line, &b.name)));
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MerkleNode {
    pub hash: String,
    pub start: usize,
    pub end: usize,
    pub left: Option<Box<MerkleNode>>,
    pub right: Option<Box<MerkleNode>>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NavguardCache {
    pub updated: u64,
    pub files: HashMap<String, FileCache>,
    pub symbols: Vec<SymbolEntry>,
    pub merkle_root: String,
    /// [`SYMBOLS_VERSION`] of `symbols`; 0 for caches written before it existed.
    #[serde(default)]
    pub symbols_version: u32,
}

use sha2::{Digest, Sha256};

#[derive(Debug, Clone, PartialEq)]
pub enum DiffKind {
    New,
    Changed,
    Deleted,
}

#[derive(Debug, Clone)]
pub struct DiffEntry {
    pub kind: DiffKind,
    pub path: String,
    pub lines: u64,
    pub tokens: u64,
    pub sha: String,
}

pub const CHUNK_SIZE: usize = 4096;

/// Compare every content chunk, including changes in the middle of a file.
/// Missing or unreadable files are handled by the caller.
pub fn quick_check(file_path: &Path, cached: &FileCache) -> bool {
    let content = match fs::read(file_path) {
        Ok(c) => c,
        Err(_) => return false, // файл удалён
    };

    cached.chunk_size == 0
        || compute_chunk_hashes(&content, cached.chunk_size) != cached.chunk_hashes
}

pub fn compute_chunk_hashes(content: &[u8], chunk_size: usize) -> Vec<String> {
    content
        .chunks(chunk_size.max(1))
        .map(|chunk| {
            let mut hasher = Sha256::new();
            hasher.update(chunk);
            format!("{:x}", hasher.finalize())[..16].to_string()
        })
        .collect()
}

pub fn build_merkle_tree(chunks: &[String]) -> Option<String> {
    if chunks.is_empty() {
        return None;
    }
    let mut level = chunks.to_vec();
    while level.len() > 1 {
        let mut next = Vec::new();
        for pair in level.chunks(2) {
            let combined = if pair.len() == 2 {
                format!("{}{}", pair[0], pair[1])
            } else {
                pair[0].clone()
            };
            let mut hasher = Sha256::new();
            hasher.update(combined.as_bytes());
            next.push(format!("{:x}", hasher.finalize())[..16].to_string());
        }
        level = next;
    }
    level.first().cloned()
}

/// File paths and boundaries are part of the tree, so renames and empty files
/// change the root. Sort paths independently of filesystem/HashMap order.
pub fn cache_merkle_root(files: &HashMap<String, FileCache>) -> String {
    let mut paths: Vec<_> = files.keys().collect();
    paths.sort();
    let leaves: Vec<_> = paths
        .into_iter()
        .map(|path| {
            let file = &files[path];
            let mut hasher = Sha256::new();
            hasher.update((path.len() as u64).to_le_bytes());
            hasher.update(path.as_bytes());
            hasher.update((file.chunk_hashes.len() as u64).to_le_bytes());
            for chunk in &file.chunk_hashes {
                hasher.update(chunk.as_bytes());
            }
            format!("{:x}", hasher.finalize())[..16].to_string()
        })
        .collect();
    build_merkle_tree(&leaves).unwrap_or_default()
}

pub fn load_cache(path: &str) -> Option<NavguardCache> {
    let p = Path::new(path);
    if !p.exists() {
        return None;
    }
    let content = fs::read_to_string(p).ok()?;
    let content = if content.starts_with('\u{feff}') {
        &content['\u{feff}'.len_utf8()..]
    } else {
        &content
    };
    serde_json::from_str(content).ok()
}

/// Writes the cache, reporting errors on stderr (`cache: write error: ...`).
pub fn save_cache(path: &str, cache: &NavguardCache) {
    if let Err(e) = try_save_cache(path, cache) {
        eprintln!("cache: {}", e);
    }
}

/// Writes the cache; the error reads `write error: ...` or `serialize error: ...`.
pub fn try_save_cache(path: &str, cache: &NavguardCache) -> Result<(), String> {
    if let Some(parent) = Path::new(path).parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).ok();
        }
    }
    let json = serde_json::to_string(cache).map_err(|e| format!("serialize error: {}", e))?;
    fs::write(path, &json).map_err(|e| format!("write error: {}", e))
}

pub fn verify_file_integrity(file_path: &Path, cached: &FileCache) -> bool {
    let content = match fs::read(file_path) {
        Ok(c) => c,
        Err(_) => return false,
    };
    cached.chunk_size > 0
        && compute_chunk_hashes(&content, cached.chunk_size) == cached.chunk_hashes
}

pub fn build_cache(root: &str, exclude: &[String]) -> NavguardCache {
    scan_into_cache(root, exclude, None)
}

/// Symbols of the file at `rel` with (normalized) text `text`.
fn symbols_of(rel: &str, text: &str) -> Vec<SymbolEntry> {
    let ext = Path::new(rel)
        .extension()
        .map(|e| e.to_string_lossy().into_owned())
        .unwrap_or_default();
    symbol_entries(rel, &parse_source(text, &ext))
}

/// Reads every project file once and builds the cache from that one read:
/// size, lines, tokens, sha, chunk hashes and symbols agree with each
/// other. With `old`, symbols of files whose chunks did not change are
/// reused instead of indexing the file again.
fn scan_into_cache(root: &str, exclude: &[String], old: Option<&NavguardCache>) -> NavguardCache {
    // A cache from the older, unordered tree format, or symbols produced by
    // another indexer version, may be stale.
    let reusable = old.filter(|cache| {
        cache.merkle_root == cache_merkle_root(&cache.files)
            && cache.symbols_version == SYMBOLS_VERSION
    });
    let mut cached_symbols: HashMap<&str, Vec<&SymbolEntry>> = HashMap::new();
    if let Some(cache) = reusable {
        for s in &cache.symbols {
            cached_symbols.entry(s.path.as_str()).or_default().push(s);
        }
    }
    let mut file_map = HashMap::new();
    let mut symbols = Vec::new();
    for rel in scanner::list_files(root, exclude) {
        let Ok(content) = fs::read(Path::new(root).join(&rel)) else {
            continue;
        };
        let (info, text) = scanner::describe(&rel, &content);
        let chunk_hashes = compute_chunk_hashes(&content, CHUNK_SIZE);
        let unchanged = reusable.is_some_and(|cache| {
            cache
                .files
                .get(&rel)
                .is_some_and(|old| old.chunk_size == CHUNK_SIZE && old.chunk_hashes == chunk_hashes)
        });
        if unchanged {
            if let Some(reused) = cached_symbols.get(rel.as_str()) {
                symbols.extend(reused.iter().map(|s| (*s).clone()));
            }
        } else if let Some(text) = &text {
            symbols.extend(symbols_of(&rel, text));
        }
        file_map.insert(
            rel,
            FileCache {
                sha: info.sha,
                lines: info.lines,
                tokens: info.tokens,
                chunk_hashes,
                chunk_size: CHUNK_SIZE,
                bytes: info.bytes,
                binary: info.binary,
            },
        );
    }
    sort_symbols(&mut symbols);
    let merkle_root = cache_merkle_root(&file_map);
    let updated = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    NavguardCache {
        updated,
        files: file_map,
        symbols,
        merkle_root,
        symbols_version: SYMBOLS_VERSION,
    }
}

pub fn verify_cache_integrity(root: &str, cache: &NavguardCache) -> Vec<String> {
    let mut errors = Vec::new();
    for (rel_path, cached) in &cache.files {
        let full_path = Path::new(root).join(rel_path);
        if !full_path.exists() {
            errors.push(format!("missing file: {}", rel_path));
            continue;
        }
        let content = match fs::read(&full_path) {
            Ok(c) => c,
            Err(e) => {
                errors.push(format!("cannot read {}: {}", rel_path, e));
                continue;
            }
        };
        let chunk_hashes = compute_chunk_hashes(&content, cached.chunk_size);
        if chunk_hashes != cached.chunk_hashes {
            errors.push(format!("content changed: {}", rel_path));
        }
    }
    errors
}

#[cfg(test)]
mod cache_fresh_tests {
    use super::*;
    use std::io::Write;

    fn temp_dir() -> tempfile::TempDir {
        tempfile::tempdir().unwrap()
    }

    fn make_file(dir: &std::path::Path, name: &str, data: &[u8]) -> std::path::PathBuf {
        let p = dir.join(name);
        let mut f = std::fs::File::create(&p).unwrap();
        f.write_all(data).unwrap();
        p
    }

    fn make_cache(root: &str, exclude: &[String]) -> NavguardCache {
        build_cache(root, exclude)
    }

    #[test]
    fn test_is_cache_fresh_no_changes() {
        let d = temp_dir();
        make_file(
            d.path(),
            "a.rs",
            b"fn main() {}
",
        );
        make_file(
            d.path(),
            "b.py",
            b"x = 1
",
        );
        let exclude: Vec<String> = vec![];
        let cache = make_cache(d.path().to_str().unwrap(), &exclude);
        assert!(
            is_cache_fresh(d.path().to_str().unwrap(), &cache, &exclude),
            "cache should be fresh when no changes"
        );
    }

    #[test]
    fn test_is_cache_fresh_file_modified() {
        let d = temp_dir();
        make_file(
            d.path(),
            "a.rs",
            b"fn main() {}
",
        );
        let exclude: Vec<String> = vec![];
        let cache = make_cache(d.path().to_str().unwrap(), &exclude);

        // Modify file
        make_file(
            d.path(),
            "a.rs",
            b"fn main() { let x = 1; }
",
        );

        assert!(
            !is_cache_fresh(d.path().to_str().unwrap(), &cache, &exclude),
            "cache should NOT be fresh after file modification"
        );
    }

    #[test]
    fn test_is_cache_fresh_new_file_added() {
        let d = temp_dir();
        make_file(
            d.path(),
            "a.rs",
            b"fn main() {}
",
        );
        let exclude: Vec<String> = vec![];
        let cache = make_cache(d.path().to_str().unwrap(), &exclude);

        // Add new file
        make_file(
            d.path(),
            "b.rs",
            b"fn helper() {}
",
        );

        assert!(
            !is_cache_fresh(d.path().to_str().unwrap(), &cache, &exclude),
            "cache should NOT be fresh when new file added"
        );
    }

    #[test]
    fn test_is_cache_fresh_file_deleted() {
        let d = temp_dir();
        let p = make_file(
            d.path(),
            "a.rs",
            b"fn main() {}
",
        );
        let exclude: Vec<String> = vec![];
        let cache = make_cache(d.path().to_str().unwrap(), &exclude);

        // Delete file
        std::fs::remove_file(&p).unwrap();

        assert!(
            !is_cache_fresh(d.path().to_str().unwrap(), &cache, &exclude),
            "cache should NOT be fresh when file deleted"
        );
    }

    #[test]
    fn test_is_cache_age_fresh_recent() {
        let cache = NavguardCache {
            updated: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
                - 60, // 1 minute ago
            files: HashMap::new(),
            symbols: Vec::new(),
            merkle_root: String::new(),
            symbols_version: 0,
        };
        assert!(
            is_cache_age_fresh(&cache, 3600),
            "1 min old cache should be fresh with 1h timeout"
        );
    }

    #[test]
    fn test_is_cache_age_fresh_old() {
        let cache = NavguardCache {
            updated: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
                - 7200, // 2 hours ago
            files: HashMap::new(),
            symbols: Vec::new(),
            merkle_root: String::new(),
            symbols_version: 0,
        };
        assert!(
            !is_cache_age_fresh(&cache, 3600),
            "2h old cache should NOT be fresh with 1h timeout"
        );
    }

    #[test]
    fn test_is_cache_age_fresh_zero_updated() {
        let cache = NavguardCache {
            updated: 0,
            files: HashMap::new(),
            symbols: Vec::new(),
            merkle_root: String::new(),
            symbols_version: 0,
        };
        assert!(
            !is_cache_age_fresh(&cache, 3600),
            "cache with updated=0 should not be fresh"
        );
    }

    #[test]
    fn test_compute_current_merkle_root_deterministic() {
        let d = temp_dir();
        make_file(
            d.path(),
            "a.rs",
            b"fn main() {}
",
        );
        let exclude: Vec<String> = vec![];
        let root1 = compute_current_merkle_root(d.path().to_str().unwrap(), &exclude);
        let root2 = compute_current_merkle_root(d.path().to_str().unwrap(), &exclude);
        assert_eq!(root1, root2, "merkle root should be deterministic");
        assert!(!root1.is_empty(), "merkle root should not be empty");
    }
}

/// Проверяет все файлы из кэша через quick_check.
/// Возвращает список изменённых файлов. Пустой = ничего не изменилось.
pub fn check_tree(root: &str, cache: &NavguardCache) -> Vec<String> {
    let mut changed = Vec::new();
    for (rel_path, cached) in &cache.files {
        let full_path = Path::new(root).join(rel_path);
        if !full_path.exists() {
            changed.push(rel_path.clone()); // удалён — тоже «изменился»
            continue;
        }
        if quick_check(&full_path, cached) {
            changed.push(rel_path.clone());
        }
    }
    changed
}

/// The Merkle root of the files on disk now (see [`cache_merkle_root`]).
pub fn compute_current_merkle_root(root: &str, exclude: &[String]) -> String {
    let mut file_map = HashMap::new();
    for rel in scanner::list_files(root, exclude) {
        let Ok(content) = fs::read(Path::new(root).join(&rel)) else {
            continue;
        };
        let (info, _) = scanner::describe(&rel, &content);
        file_map.insert(
            rel,
            FileCache {
                chunk_hashes: compute_chunk_hashes(&content, CHUNK_SIZE),
                chunk_size: CHUNK_SIZE,
                sha: info.sha,
                lines: info.lines,
                tokens: info.tokens,
                bytes: info.bytes,
                binary: info.binary,
            },
        );
    }
    cache_merkle_root(&file_map)
}

/// True when the files on disk have the cache's Merkle root (nothing
/// changed, appeared or disappeared).
pub fn is_cache_fresh(root: &str, cache: &NavguardCache, exclude: &[String]) -> bool {
    compute_current_merkle_root(root, exclude) == cache.merkle_root
}

/// Быстрая проверка по времени: если кэш не старше timeout_sec.
/// Если cache.updated == 0, всегда возвращает false.
pub fn is_cache_age_fresh(cache: &NavguardCache, timeout_sec: u64) -> bool {
    if cache.updated == 0 {
        return false;
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    now - cache.updated < timeout_sec
}

/// Rescans the project (each file read once), reusing symbols only for
/// files whose chunks did not change. Deleted and excluded files drop out.
pub fn smart_update_cache(root: &str, cache: &NavguardCache, exclude: &[String]) -> NavguardCache {
    scan_into_cache(root, exclude, Some(cache))
}

pub fn diff_scan(current: &[FileInfo], cache: &NavguardCache) -> Vec<DiffEntry> {
    let mut result = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for f in current {
        seen.insert(f.path.clone());
        if let Some(cached) = cache.files.get(&f.path) {
            if cached.sha != f.sha {
                result.push(DiffEntry {
                    kind: DiffKind::Changed,
                    path: f.path.clone(),
                    lines: f.lines,
                    tokens: f.tokens,
                    sha: f.sha.clone(),
                });
            }
        } else {
            result.push(DiffEntry {
                kind: DiffKind::New,
                path: f.path.clone(),
                lines: f.lines,
                tokens: f.tokens,
                sha: f.sha.clone(),
            });
        }
    }

    for (path, cached) in &cache.files {
        if !seen.contains(path) {
            result.push(DiffEntry {
                kind: DiffKind::Deleted,
                path: path.clone(),
                lines: cached.lines,
                tokens: cached.tokens,
                sha: cached.sha.clone(),
            });
        }
    }

    result
}

#[cfg(test)]
mod quick_check_tests {
    use super::*;
    use std::io::Write;

    fn temp_dir() -> tempfile::TempDir {
        tempfile::tempdir().unwrap()
    }

    fn make_file(dir: &std::path::Path, name: &str, data: &[u8]) -> std::path::PathBuf {
        let p = dir.join(name);
        let mut f = std::fs::File::create(&p).unwrap();
        f.write_all(data).unwrap();
        p
    }

    fn cache_for(data: &[u8]) -> FileCache {
        let chunks = compute_chunk_hashes(data, CHUNK_SIZE);
        FileCache {
            sha: "test".to_string(),
            lines: 10,
            tokens: 10,
            chunk_hashes: chunks,
            chunk_size: CHUNK_SIZE,
            bytes: data.len() as u64,
            binary: false,
        }
    }

    #[test]
    fn test_quick_check_unchanged() {
        let d = temp_dir();
        let data = vec![0xAB; 10000]; // > 8KB
        let p = make_file(d.path(), "f.txt", &data);
        let fc = cache_for(&data);
        assert!(
            !quick_check(&p, &fc),
            "file should not be changed without modifications"
        );
    }

    #[test]
    fn test_quick_check_first_changed() {
        let d = temp_dir();
        let data = vec![0xAB; 10000];
        let p = make_file(d.path(), "f.txt", &data);
        // Меняем first 4KB
        let mut modified = data.clone();
        modified[0] = 0xCD;
        make_file(d.path(), "f.txt", &modified);
        let fc = cache_for(&data);
        assert!(quick_check(&p, &fc), "first 4KB change should be detected");
    }

    #[test]
    fn test_quick_check_last_changed() {
        let d = temp_dir();
        let data = vec![0xAB; 10000];
        let p = make_file(d.path(), "f.txt", &data);
        // Меняем last 4KB
        let mut modified = data.clone();
        let last_idx = modified.len() - 1;
        modified[last_idx] = 0xCD;
        make_file(d.path(), "f.txt", &modified);
        let fc = cache_for(&data);
        assert!(quick_check(&p, &fc), "last 4KB change should be detected");
    }

    #[test]
    fn test_quick_check_middle_changed() {
        let d = temp_dir();
        let data = vec![0xAB; 10000];
        let p = make_file(d.path(), "f.txt", &data);
        // A middle edit must be detected even when first/last chunks match.
        let mut modified = data.clone();
        modified[5000] = 0xCD;
        make_file(d.path(), "f.txt", &modified);
        let fc = cache_for(&data);
        assert!(quick_check(&p, &fc), "middle change should be detected");
    }

    #[test]
    fn test_quick_check_deleted() {
        let d = temp_dir();
        let data = vec![0xAB; 1000];
        let p = make_file(d.path(), "f.txt", &data);
        std::fs::remove_file(&p).unwrap();
        let fc = cache_for(&data);
        assert!(
            !quick_check(&p, &fc),
            "deleted file = false (not changed, but deleted)"
        );
    }

    #[test]
    fn test_check_tree_no_changes() {
        let d = temp_dir();
        let data = vec![0xAB; 5000];
        make_file(d.path(), "f.txt", &data);
        // Создаём кэш вручную
        let mut cache = NavguardCache {
            updated: 0,
            files: HashMap::new(),
            symbols: Vec::new(),
            merkle_root: "test".to_string(),
            symbols_version: 0,
        };
        cache.files.insert("f.txt".to_string(), cache_for(&data));
        let changed = check_tree(d.path().to_str().unwrap(), &cache);
        assert!(changed.is_empty(), "no changes should return empty");
    }

    #[test]
    fn test_check_tree_one_change() {
        let d = temp_dir();
        let data = vec![0xAB; 5000];
        make_file(d.path(), "f.txt", &data);
        let mut cache = NavguardCache {
            updated: 0,
            files: HashMap::new(),
            symbols: Vec::new(),
            merkle_root: "test".to_string(),
            symbols_version: 0,
        };
        cache.files.insert("f.txt".to_string(), cache_for(&data));
        // Меняем файл
        make_file(d.path(), "f.txt", &vec![0xCD; 5000]);
        let changed = check_tree(d.path().to_str().unwrap(), &cache);
        assert_eq!(changed.len(), 1, "one changed file");
        assert_eq!(changed[0], "f.txt");
    }
}
// changed

#[cfg(test)]
mod reconciliation_tests {
    use super::*;

    #[test]
    fn discovers_new_and_empty_files_without_other_changes() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_str().unwrap();
        fs::write(dir.path().join("old.rs"), "fn old() {}\n").unwrap();
        let original = build_cache(root, &[]);
        fs::write(dir.path().join("new.rs"), "fn added() {}\n").unwrap();
        fs::write(dir.path().join("empty.txt"), "").unwrap();
        let updated = smart_update_cache(root, &original, &[]);
        assert_eq!(updated.files.len(), 3);
        assert!(updated.symbols.iter().any(|s| s.name.contains("added")));
        assert_ne!(updated.merkle_root, original.merkle_root);
        assert!(is_cache_fresh(root, &updated, &[]));
    }

    #[test]
    fn refreshes_middle_edits_and_removes_deleted_symbols() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_str().unwrap();
        let before = format!(
            "{}\nfn before() {{}}\n{}",
            " ".repeat(CHUNK_SIZE),
            " ".repeat(CHUNK_SIZE)
        );
        fs::write(dir.path().join("middle.rs"), &before).unwrap();
        fs::write(dir.path().join("deleted.rs"), "fn deleted() {}\n").unwrap();
        let original = build_cache(root, &[]);
        fs::write(
            dir.path().join("middle.rs"),
            before.replace("before", "after_"),
        )
        .unwrap();
        fs::remove_file(dir.path().join("deleted.rs")).unwrap();
        let updated = smart_update_cache(root, &original, &[]);
        assert_eq!(updated.files.len(), 1);
        assert!(updated.symbols.iter().any(|s| s.name.contains("after_")));
        assert!(!updated
            .symbols
            .iter()
            .any(|s| s.name.contains("before") || s.path == "deleted.rs"));
        assert_ne!(
            updated.files["middle.rs"].chunk_hashes,
            original.files["middle.rs"].chunk_hashes
        );
        assert!(is_cache_fresh(root, &updated, &[]));
    }

    #[test]
    fn merkle_root_is_stable_across_map_order_and_tracks_renames() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_str().unwrap();
        fs::write(dir.path().join("z.rs"), "fn z() {}\n").unwrap();
        fs::write(dir.path().join("a.rs"), "fn a() {}\n").unwrap();
        fs::write(dir.path().join("empty.txt"), "").unwrap();
        let cache = build_cache(root, &[]);
        let mut reverse = HashMap::new();
        for name in ["z.rs", "empty.txt", "a.rs"] {
            reverse.insert(name.to_string(), cache.files[name].clone());
        }
        assert_eq!(cache_merkle_root(&reverse), cache.merkle_root);
        assert_eq!(
            smart_update_cache(root, &cache, &[]).merkle_root,
            cache.merkle_root
        );
        fs::rename(dir.path().join("empty.txt"), dir.path().join("renamed.txt")).unwrap();
        assert!(!is_cache_fresh(root, &cache, &[]));
        let renamed = smart_update_cache(root, &cache, &[]);
        assert!(!renamed.files.contains_key("empty.txt"));
        assert!(renamed.files.contains_key("renamed.txt"));
        assert!(is_cache_fresh(root, &renamed, &[]));
    }

    #[test]
    fn excludes_removed_paths_and_repairs_legacy_symbol_indexes() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_str().unwrap();
        fs::write(dir.path().join("keep.rs"), "fn keep() {}\n").unwrap();
        fs::write(dir.path().join("ignored.rs"), "fn ignored() {}\n").unwrap();
        let mut legacy = build_cache(root, &[]);
        legacy.merkle_root = "legacy-unordered-root".to_string();
        legacy.symbols.clear();
        let updated = smart_update_cache(root, &legacy, &["ignored.rs".to_string()]);
        assert_eq!(updated.files.len(), 1);
        assert!(updated.symbols.iter().any(|s| s.name.contains("keep")));
        assert!(!updated.symbols.iter().any(|s| s.path == "ignored.rs"));
    }

    #[test]
    fn rebuilds_symbols_written_by_an_older_indexer() {
        // A pre-version-2 cache file: no symbol details, no version.
        let v1 = r#"{"updated":1,"files":{},"symbols":[{"path":"a.rs","name":"impl A {","line":1}],"merkle_root":""}"#;
        let parsed: NavguardCache = serde_json::from_str(v1).unwrap();
        assert_eq!(parsed.symbols_version, 0);
        assert_eq!(parsed.symbols[0].ident, "");

        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_str().unwrap();
        fs::write(dir.path().join("lib.rs"), "impl A {\n    fn b() {}\n}\n").unwrap();
        let mut old = build_cache(root, &[]);
        old.symbols.retain(|s| s.line == 1);
        old.symbols_version = 0;
        let updated = smart_update_cache(root, &old, &[]);
        assert_eq!(updated.symbols_version, SYMBOLS_VERSION);
        assert_eq!(updated.symbols.len(), 2);
        let b = &updated.symbols[1];
        assert_eq!(
            (
                b.ident.as_str(),
                b.kind.as_str(),
                b.line,
                b.line_end,
                b.depth
            ),
            ("b", "method", 2, 2, 1)
        );
        assert_eq!(b.parent.as_deref(), Some("A"));
    }
}
