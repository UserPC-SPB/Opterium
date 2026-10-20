use crate::adapters::SourceLines;
use crate::collect::{FileResult, FileStatus, RuleResult, RuleStatus};
use crate::template::CompiledTemplate;

#[derive(Debug, Clone)]
pub struct Block {
    pub id: String,
    pub parent_id: Option<String>,
    pub part: Option<usize>,
    pub parts: Option<usize>,
    pub item_index: Option<usize>,
    pub source: String,
    pub format: String,
    pub format_source: String,
    pub rule_id: String,
    pub key: String,
    pub status: &'static str,
    pub file_status: &'static str,
    pub item_count: usize,
    pub chars: usize,
    pub truncated: bool,
    pub split: bool,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
    pub items: Vec<String>,
    pub start_line: usize,
    pub end_line: usize,
}

#[derive(Debug, Clone)]
pub struct Materialized {
    pub blocks: Vec<Block>,
    pub total_items: usize,
    pub total_chars: usize,
    pub truncated: Vec<String>,
    pub errors: Vec<String>,
}

/// One rule result of one source as the document publishes it, before
/// anchor escaping and block splitting: the content of one data block (or
/// of one split group). [`result_entries`] lists them in document order.
#[derive(Debug, Clone)]
pub struct ResultEntry {
    pub source: String,
    pub format: String,
    pub format_source: String,
    pub rule_id: String,
    pub key: String,
    /// Rule status as written in the block (`success`, `no matches found`,
    /// `rule not applicable`, `error`).
    pub status: &'static str,
    pub file_status: &'static str,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
    /// Raw item strings (after `stable_deduplicate`, not anchor-escaped).
    pub items: Vec<String>,
    /// Source lines of each item, parallel to `items` (text rules; the
    /// document does not show them).
    pub item_lines: Vec<Option<SourceLines>>,
    pub truncated: bool,
}

/// The rule results of a run in document order, with the run-level lists
/// the document shows under `## Truncation` and `## Diagnostics`.
#[derive(Debug, Clone)]
pub struct ResultEntries {
    pub entries: Vec<ResultEntry>,
    /// Characters of all items before `stable_deduplicate`: the initial
    /// guess for the document's `Total output characters` fixed point.
    pub total_chars: usize,
    pub truncated: Vec<String>,
    pub errors: Vec<String>,
}

pub fn materialize(
    files: &[FileResult],
    template: &CompiledTemplate,
) -> Result<Materialized, String> {
    let ResultEntries {
        entries: shells,
        total_chars,
        truncated,
        errors,
    } = result_entries(files, template);

    let mut blocks = Vec::new();
    let total_items = shells.iter().map(|s| s.items.len()).sum();
    for (i, mut shell) in shells.into_iter().enumerate() {
        // Escape before splitting: fragments of the escaped item concatenate
        // back to it, and one unescape restores the source text.
        for item in &mut shell.items {
            if let std::borrow::Cow::Owned(escaped) = crate::render::escape_anchor_text(item) {
                *item = escaped;
            }
        }
        let block = Block {
            id: format!("B{:04}", i + 1),
            parent_id: None,
            part: None,
            parts: None,
            item_index: None,
            source: shell.source,
            format: shell.format,
            format_source: shell.format_source,
            rule_id: shell.rule_id,
            key: shell.key,
            status: shell.status,
            file_status: shell.file_status,
            item_count: shell.items.len(),
            chars: shell.items.iter().map(|i| i.chars().count()).sum(),
            truncated: shell.truncated,
            split: false,
            warnings: shell.warnings,
            errors: shell.errors,
            items: shell.items,
            start_line: 0,
            end_line: 0,
        };
        split_block(&mut blocks, block, template)?;
    }

    Ok(Materialized {
        blocks,
        total_items,
        total_chars,
        truncated,
        errors,
    })
}

/// Collects every rule result of `files` in document order (file order,
/// then rule order, then `document.sort`), with items deduplicated by
/// `document.stable_deduplicate`. Shared by [`materialize`] and the
/// in-memory API (`crate::api`).
pub fn result_entries(files: &[FileResult], template: &CompiledTemplate) -> ResultEntries {
    let mut shells: Vec<ResultEntry> = Vec::new();
    let mut total_chars = 0usize;
    let mut truncated = Vec::new();
    let mut errors = Vec::new();

    for file in files {
        if let Some(error) = &file.error {
            errors.push(format!("{}: {}", file.path.display(), error));
        }
        for rr in &file.rule_results {
            // Deduplicating here, before the warnings are written, keeps the
            // truncated item numbers pointing at published items. Sorting
            // below only compares first items, which deduplication keeps.
            let (items, item_lines, mut warnings) = if template.stable_deduplicate {
                deduplicate(rr)
            } else {
                let lines = (0..rr.items.len()).map(|i| line_of(rr, i)).collect();
                (rr.items.clone(), lines, rr.warnings())
            };
            if rr.status == RuleStatus::NotApplicable {
                warnings.extend(not_applicable_reason(&file.format, &rr.rule_id, template));
            }
            if rr.is_truncated() {
                for w in &warnings {
                    truncated.push(format!("{} [{}]: {}", file.path.display(), rr.rule_id, w));
                }
            }
            if rr.status == RuleStatus::Error {
                if let Some(e) = &rr.error {
                    errors.push(format!("{} [{}]: {}", file.path.display(), rr.rule_id, e));
                }
            }
            total_chars += rr.items.iter().map(|i| i.chars().count()).sum::<usize>();
            shells.push(ResultEntry {
                source: file.path.to_string_lossy().to_string(),
                format: file.format.name().to_string(),
                format_source: file.format_source.clone(),
                rule_id: rr.rule_id.clone(),
                key: rr.key.clone(),
                status: rule_status_str(&rr.status),
                file_status: file_status_str(&file.status),
                warnings,
                errors: rr.error.clone().map(|e| vec![e]).unwrap_or_default(),
                items,
                item_lines,
                truncated: rr.is_truncated(),
            });
        }
    }

    if let Some(sort) = &template.sort {
        match sort.as_str() {
            "asc" => shells.sort_by(|a, b| {
                a.items
                    .first()
                    .cmp(&b.items.first())
                    .then_with(|| a.source.cmp(&b.source))
            }),
            "desc" => shells.sort_by(|a, b| {
                b.items
                    .first()
                    .cmp(&a.items.first())
                    .then_with(|| b.source.cmp(&a.source))
            }),
            _ => {}
        }
    }

    ResultEntries {
        entries: shells,
        total_chars,
        truncated,
        errors,
    }
}

/// Why a rule did not run on a source: the source's format does not take
/// the rule's selector type (`when.format` mismatches get no result at all).
fn not_applicable_reason(
    format: &crate::template::Format,
    rule_id: &str,
    template: &CompiledTemplate,
) -> Option<String> {
    let rule = template.rules.iter().find(|r| r.id == rule_id)?;
    if *format == crate::template::Format::Auto {
        return None;
    }
    Some(format!(
        "rule not applicable: selector type {} does not apply to {} files (they take {})",
        rule.selector_type,
        format.name(),
        crate::adapters::adapter_for(format)
            .selector_types()
            .join(", ")
    ))
}

/// The source lines of item `index` of `rr`, if it has them.
fn line_of(rr: &RuleResult, index: usize) -> Option<SourceLines> {
    rr.item_lines.get(index).copied().flatten()
}

/// `document.stable_deduplicate` for one rule result: keeps first
/// occurrences (with their source lines) and renumbers the truncation
/// report to the kept items. Items are compared by their value before
/// `max_item_chars`: two records that only share the kept prefix are both
/// published. Truncated items that turned out to be duplicates are counted
/// apart.
fn deduplicate(rr: &RuleResult) -> (Vec<String>, Vec<Option<SourceLines>>, Vec<String>) {
    // Full values of the cut items, by their 1-based number.
    let originals: std::collections::HashMap<usize, &str> = rr
        .truncated_chars_list
        .iter()
        .zip(&rr.truncated_originals)
        .map(|(&(number, _, _), original)| (number, original.as_str()))
        .collect();
    let mut seen = std::collections::HashSet::new();
    let mut items = Vec::new();
    let mut item_lines = Vec::new();
    // New 1-based number of every original item; 0 for a removed duplicate.
    let mut numbers = Vec::with_capacity(rr.items.len());
    for (index, item) in rr.items.iter().enumerate() {
        let full = originals.get(&(index + 1)).copied().unwrap_or(item);
        if seen.insert(full) {
            items.push(item.clone());
            item_lines.push(line_of(rr, index));
            numbers.push(items.len());
        } else {
            numbers.push(0);
        }
    }
    let mut kept = Vec::new();
    let mut removed = 0usize;
    for &(number, from, to) in &rr.truncated_chars_list {
        match numbers.get(number.wrapping_sub(1)) {
            Some(&new) if new > 0 => kept.push((new, from, to)),
            _ => removed += 1,
        }
    }
    let mut warnings =
        crate::collect::limit_warnings(&kept, rr.omitted_items, rr.omitted_by_resource);
    if removed > 0 {
        warnings.push(format!(
            "{} truncated item(s) dropped as duplicates by stable_deduplicate",
            removed
        ));
    }
    // Transform cuts on removed duplicates stay counted as not published.
    let losses: Vec<_> = rr
        .transform_losses
        .iter()
        .map(|loss| {
            loss.renumbered(|number| {
                numbers
                    .get(number.wrapping_sub(1))
                    .copied()
                    .filter(|&n| n > 0)
            })
        })
        .collect();
    warnings.extend(crate::collect::transform_loss_warnings(&losses));
    (items, item_lines, warnings)
}

fn refresh_block(block: &mut Block) {
    // The parent of a split group keeps the item count of the whole rule
    // result; its items are in the continuation parts.
    if !(block.split && block.part.is_none()) {
        block.item_count = block.items.len();
    }
    block.chars = crate::render::render_block_document(block).chars().count();
}

fn limit_error(block: &Block, max: usize) -> String {
    format!(
        "block {} exceeds max_block_chars: {} characters, limit {} (including anchors and metadata)",
        block.id, block.chars, max
    )
}

fn split_block(
    out: &mut Vec<Block>,
    mut block: Block,
    template: &CompiledTemplate,
) -> Result<(), String> {
    let max = usize::try_from(template.block.max_chars).unwrap_or(usize::MAX);
    let target = usize::try_from(template.block.target_chars).unwrap_or(usize::MAX);
    refresh_block(&mut block);
    if block.chars <= max {
        out.push(block);
        return Ok(());
    }
    if !template.split_blocks || block.items.is_empty() {
        return Err(limit_error(&block, max));
    }

    let items = std::mem::take(&mut block.items);
    let item_chars: Vec<usize> = items
        .iter()
        .map(|item| crate::render::rendered_item_chars(item))
        .collect();
    block.split = true;
    let mut reserved_parts = 1usize;
    // Part-number widths affect available data space. Start with one digit,
    // then repartition only if the actual count needs a wider reservation.
    let parts = loop {
        block.parts = Some(reserved_parts);
        refresh_block(&mut block);
        if block.chars > max {
            return Err(limit_error(&block, max));
        }
        let mut prototype = block.clone();
        prototype.id = format!("{}-{:02}", block.id, reserved_parts);
        prototype.parent_id = Some(block.id.clone());
        prototype.part = Some(reserved_parts);
        // Warnings describe the whole rule result; the parent carries them.
        prototype.warnings.clear();
        let mut current = prototype.clone();
        let mut current_chars = 0usize;
        let mut parts = Vec::new();

        for (item_index, item) in items.iter().enumerate() {
            // Sizes come from the renderer's own accounting, so each item
            // costs only a metadata render instead of a full part render.
            // Reserved metadata widths shrink when final part numbers are
            // assigned.
            let size_with = |count: usize, chars: usize| {
                crate::render::block_document_chars(&prototype, count, chars)
            };
            if size_with(1, item_chars[item_index]) > max {
                if !current.items.is_empty() {
                    parts.push(current);
                    current = prototype.clone();
                    current_chars = 0;
                }
                let mut single = prototype.clone();
                single.item_index = Some(item_index + 1);
                single.items = vec![String::new()];
                refresh_block(&mut single);
                let budget = max
                    .checked_sub(single.chars.saturating_sub(3))
                    .ok_or_else(|| limit_error(&single, max))?;
                for fragment in split_item_parts(item, budget)? {
                    let mut part = single.clone();
                    part.items = vec![fragment];
                    refresh_block(&mut part);
                    if part.chars > max {
                        return Err(limit_error(&part, max));
                    }
                    parts.push(part);
                }
            } else {
                let count = current.items.len() + 1;
                let size = size_with(count, current_chars + item_chars[item_index]);
                if count > 1 && (size > target || size > max) {
                    parts.push(current);
                    current = prototype.clone();
                    current_chars = 0;
                }
                current.items.push(item.clone());
                current_chars += item_chars[item_index];
            }
        }
        if !current.items.is_empty() {
            parts.push(current);
        }
        if parts.len() > reserved_parts {
            reserved_parts = parts.len();
            continue;
        }
        break parts;
    };
    block.parts = Some(parts.len());
    refresh_block(&mut block);
    out.push(block);
    let count = parts.len();
    for (i, mut part) in parts.into_iter().enumerate() {
        part.id = format!("{}-{:02}", part.parent_id.as_deref().unwrap(), i + 1);
        part.part = Some(i + 1);
        part.parts = Some(count);
        refresh_block(&mut part);
        if part.chars > max {
            return Err(limit_error(&part, max));
        }
        out.push(part);
    }
    Ok(())
}

// Return exact substrings: concatenation restores the original item, including
// every newline. A newline adds four indentation characters in the renderer.
fn split_item_parts(item: &str, budget: usize) -> Result<Vec<String>, String> {
    let mut parts = Vec::new();
    let mut remaining = item;
    while !remaining.is_empty() {
        let mut rendered_chars = 3usize;
        let mut end = 0;
        let mut last_line_end = None;
        for (offset, ch) in remaining.char_indices() {
            let cost = if ch == '\n' { 5 } else { 1 };
            if rendered_chars.saturating_add(cost) > budget {
                break;
            }
            rendered_chars += cost;
            end = offset + ch.len_utf8();
            if ch == '\n' {
                last_line_end = Some(end);
            }
        }
        if end == 0 {
            return Err(
                "max_block_chars is too small for continuation metadata and data".to_string(),
            );
        }
        if end < remaining.len() {
            if let Some(line_end) = last_line_end {
                end = line_end;
            }
        }
        parts.push(remaining[..end].to_string());
        remaining = &remaining[end..];
    }
    Ok(parts)
}

pub fn rule_status_str(rule_status: &RuleStatus) -> &'static str {
    match rule_status {
        RuleStatus::Matched => "success",
        RuleStatus::NoMatch => "no matches found",
        RuleStatus::NotApplicable => "rule not applicable",
        RuleStatus::Error => "error",
    }
}

pub fn file_status_str(file_status: &FileStatus) -> &'static str {
    match file_status {
        FileStatus::Error => "error",
        FileStatus::Warning => "warning",
        FileStatus::Success => "success",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn small_caps_use_actual_part_number_widths_without_losing_data() {
        let template = crate::template::compile_yaml_str(
            "version: 1\nname: t\ninput: {format: text}\noutput: {block: {target_chars: 300, max_chars: 300}}\nrules:\n  - id: r\n    select: {type: line, from: 1, to: 1}\n    output: {key: k}\n",
            &std::path::PathBuf::from("t.yml"), None, None).unwrap();
        let source = crate::discovery::DiscoveredFile {
            path: "s".into(),
            size: 0,
            mtime_iso: String::new(),
            hash: None,
        };
        for length in [150, 5000] {
            let text = "x".repeat(length);
            let result = crate::collect::process_file(
                &source,
                &template,
                &text,
                "utf-8",
                crate::template::Format::Text,
                "template".into(),
                None,
            );
            let materialized = materialize(&[result], &template).unwrap();
            assert!(materialized.blocks.iter().all(|block| block.chars <= 300));
            let fragments: Vec<_> = materialized
                .blocks
                .iter()
                .filter(|b| b.item_index == Some(1))
                .collect();
            assert_eq!(
                fragments
                    .iter()
                    .flat_map(|b| b.items.iter())
                    .cloned()
                    .collect::<String>(),
                text
            );
            for (i, fragment) in fragments.iter().enumerate() {
                assert_eq!(fragment.part, Some(i + 1));
                assert_eq!(fragment.parts, Some(fragments.len()));
            }
        }
    }
}
