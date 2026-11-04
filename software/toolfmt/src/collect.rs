use crate::adapters::{Adapter, RawMatch, SourceLines, SourceTree, adapter_for, rule_applicable};
use crate::discovery::DiscoveredFile;
use crate::template::{CompiledRule, CompiledTemplate, Format};
use crate::transform;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileStatus {
    Success,
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuleStatus {
    Matched,
    NoMatch,
    NotApplicable,
    Error,
}

#[derive(Debug, Clone)]
pub struct RuleResult {
    pub rule_id: String,
    pub key: String,
    pub status: RuleStatus,
    pub error: Option<String>,
    pub items: Vec<String>,
    /// Source lines of each item, parallel to `items` (or empty): text
    /// rules only, and `None` for an item that `join` made of several.
    pub item_lines: Vec<Option<SourceLines>>,
    pub truncated_items: usize,
    pub omitted_items: usize,
    pub omitted_by_resource: usize,
    /// `(item number, original chars, kept chars)` for every item cut to
    /// `max_item_chars`; item numbers are 1-based positions in `items`.
    pub truncated_chars_list: Vec<(usize, usize, usize)>,
    /// The full values of the items cut to `max_item_chars`, in the order
    /// of `truncated_chars_list`. `stable_deduplicate` compares these, so
    /// records that only share the kept prefix stay distinct.
    pub truncated_originals: Vec<String>,
    /// Data lost to `truncate_chars`, `truncate_lines` and `limit_items`, one
    /// entry per transform step that lost something, in transform order.
    pub transform_losses: Vec<TransformLoss>,
}

/// Data that one lossy transform step of a rule removed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransformLoss {
    /// The step as listed under `Transforms:` in Applied Template.
    pub transform: String,
    pub kind: LossKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LossKind {
    /// `truncate_chars` / `truncate_lines` cut `count` items. `published`
    /// lists the published items that carry those cuts, each item once, in
    /// item order. Cut items removed later (by `limit_items`, `max_items`,
    /// `unique`, `drop_empty` or `stable_deduplicate`) are counted but not
    /// listed.
    Cut {
        unit: &'static str,
        count: usize,
        published: Vec<CutItem>,
    },
    /// `limit_items` dropped this many items from the end of the list.
    Omitted(usize),
}

/// A published item that one truncating transform step cut.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CutItem {
    /// 1-based position in `items`.
    pub item: usize,
    /// How many cut items it consists of: more than one after `join`.
    pub pieces: usize,
    /// Size before the cut (of the largest piece), in the loss unit.
    pub from: usize,
    /// Size after the cut (of each piece), in the loss unit.
    pub to: usize,
}

impl CutItem {
    fn label(&self, unit: &str) -> String {
        if self.pieces == 1 {
            format!("item {} ({} -> {} {})", self.item, self.from, self.to, unit)
        } else {
            format!(
                "item {} ({} joined items cut to {} {})",
                self.item, self.pieces, self.to, unit
            )
        }
    }
}

impl TransformLoss {
    /// The same loss with published item numbers mapped by `renumber`; cut
    /// items it maps to `None` are no longer published.
    pub fn renumbered(&self, renumber: impl Fn(usize) -> Option<usize>) -> TransformLoss {
        let kind = match &self.kind {
            LossKind::Cut {
                unit,
                count,
                published,
            } => LossKind::Cut {
                unit,
                count: *count,
                published: published
                    .iter()
                    .filter_map(|cut| {
                        renumber(cut.item).map(|item| CutItem {
                            item,
                            ..cut.clone()
                        })
                    })
                    .collect(),
            },
            LossKind::Omitted(count) => LossKind::Omitted(*count),
        };
        TransformLoss {
            transform: self.transform.clone(),
            kind,
        }
    }
}

/// How many truncated items the summary warning names individually.
const TRUNCATION_EXAMPLES: usize = 5;

impl RuleResult {
    pub fn is_truncated(&self) -> bool {
        self.truncated_items > 0
            || self.omitted_items > 0
            || self.omitted_by_resource > 0
            || !self.truncated_chars_list.is_empty()
            || !self.transform_losses.is_empty()
    }

    pub fn warnings(&self) -> Vec<String> {
        let mut w = limit_warnings(
            &self.truncated_chars_list,
            self.omitted_items,
            self.omitted_by_resource,
        );
        w.extend(transform_loss_warnings(&self.transform_losses));
        w
    }
}

/// `item 1 (86 -> 20 chars), …` for the first few cut items, then
/// `, and N more`.
fn cut_examples<T>(cuts: &[T], label: impl Fn(&T) -> String) -> String {
    let examples: Vec<String> = cuts.iter().take(TRUNCATION_EXAMPLES).map(label).collect();
    let mut line = examples.join(", ");
    let more = cuts.len() - examples.len();
    if more > 0 {
        line.push_str(&format!(", and {} more", more));
    }
    line
}

/// One line per lossy transform step, however many items it affected.
pub fn transform_loss_warnings(losses: &[TransformLoss]) -> Vec<String> {
    losses
        .iter()
        .map(|loss| match &loss.kind {
            LossKind::Cut {
                unit,
                count,
                published,
            } => {
                let mut line = format!("{} item(s) truncated by {}", count, loss.transform);
                if !published.is_empty() {
                    line.push_str(": ");
                    line.push_str(&cut_examples(published, |cut| cut.label(unit)));
                }
                let shown: usize = published.iter().map(|cut| cut.pieces).sum();
                let unpublished = count.saturating_sub(shown);
                if unpublished > 0 {
                    line.push_str(&format!("; {} not published", unpublished));
                }
                line
            }
            LossKind::Omitted(count) => {
                format!("{} item(s) omitted by {}", count, loss.transform)
            }
        })
        .collect()
}

/// One line per kind of loss, however many items it affected, so the
/// warnings of a large result stay small enough for its block.
pub fn limit_warnings(
    truncated: &[(usize, usize, usize)],
    omitted_items: usize,
    omitted_by_resource: usize,
) -> Vec<String> {
    let mut w = Vec::new();
    if !truncated.is_empty() {
        w.push(format!(
            "{} item(s) truncated to max_item_chars: {}",
            truncated.len(),
            cut_examples(truncated, |(item, from, to)| format!(
                "item {} ({} -> {} chars)",
                item, from, to
            ))
        ));
    }
    if omitted_items > 0 {
        w.push(format!(
            "{} item(s) omitted after max_items limit",
            omitted_items
        ));
    }
    if omitted_by_resource > 0 {
        w.push(format!(
            "{} match(es) omitted after max_matches_per_rule resource limit",
            omitted_by_resource
        ));
    }
    w
}

#[derive(Debug, Clone)]
pub struct FileResult {
    pub path: PathBuf,
    pub size: u64,
    pub mtime_iso: String,
    pub hash: Option<String>,
    pub format: Format,
    pub format_source: String,
    pub encoding: String,
    pub status: FileStatus,
    pub error: Option<String>,
    pub rule_results: Vec<RuleResult>,
}

pub fn process_file(
    file: &DiscoveredFile,
    template: &CompiledTemplate,
    text: &str,
    encoding: &str,
    format: Format,
    format_source: String,
    max_matches_per_rule: Option<u64>,
) -> FileResult {
    let adapter = adapter_for(&format);
    let parsed = adapter.parse(text);

    let mut rule_results = Vec::new();

    let tree = match parsed {
        Err(e) => {
            return FileResult {
                path: file.path.clone(),
                size: file.size,
                mtime_iso: file.mtime_iso.clone(),
                hash: file.hash.clone(),
                format,
                format_source,
                encoding: encoding.to_string(),
                status: FileStatus::Error,
                error: Some(e),
                rule_results,
            };
        }
        Ok(t) => t,
    };

    for rule in &template.rules {
        // `when.format` is a declared filter, not a mismatch: a rule meant
        // for another format gets no result here and leaves the status alone.
        if !selected_for_format(rule, &format) {
            continue;
        }
        rule_results.push(process_rule(
            adapter.as_ref(),
            &tree,
            rule,
            max_matches_per_rule,
        ));
    }
    let file_status = file_status(&rule_results);

    FileResult {
        path: file.path.clone(),
        size: file.size,
        mtime_iso: file.mtime_iso.clone(),
        hash: file.hash.clone(),
        format,
        format_source,
        encoding: encoding.to_string(),
        status: file_status,
        error: None,
        rule_results,
    }
}

/// Whether the rule runs on files of `format`: its `when.format` condition
/// admits the format and, for rules that skip unreadable formats, the
/// format's adapter takes the rule's selector type.
pub fn selected_for_format(rule: &CompiledRule, format: &Format) -> bool {
    rule.when_format
        .as_ref()
        .is_none_or(|wanted| wanted == format)
        && (!rule.skip_unreadable_formats || format_reads_selector(format, &rule.selector_type))
}

/// Whether files of `format` take selectors of `selector_type`.
pub fn format_reads_selector(format: &Format, selector_type: &str) -> bool {
    *format != Format::Auto
        && adapter_for(format)
            .selector_types()
            .contains(&selector_type)
}

/// Status of a parsed source from the results of its rules.
/// `error` means the source yielded nothing: every rule that ran failed
/// (a rule not applicable to the format did not run, so it does not count).
/// A failed rule next to working ones, an empty or inapplicable rule, and
/// any truncation make the result partial: `warning`.
fn file_status(results: &[RuleResult]) -> FileStatus {
    let mut ran = results
        .iter()
        .filter(|r| r.status != RuleStatus::NotApplicable)
        .peekable();
    if ran.peek().is_some() && ran.all(|r| r.status == RuleStatus::Error) {
        FileStatus::Error
    } else if results
        .iter()
        .any(|r| r.status != RuleStatus::Matched || r.is_truncated())
    {
        FileStatus::Warning
    } else {
        FileStatus::Success
    }
}

/// A collected value, its source lines and the transform cuts it carries:
/// `(index into the rule's losses, size before, size after)`.
struct Tracked {
    value: String,
    lines: Option<SourceLines>,
    cuts: Vec<(usize, usize, usize)>,
}

impl transform::ResultItem for Tracked {
    fn text(&self) -> &str {
        &self.value
    }

    fn join(items: Vec<Self>, separator: &str) -> Self {
        // A joined item has one place in the source only if it is one item.
        let lines = match items.as_slice() {
            [only] => only.lines,
            _ => None,
        };
        let mut cuts = Vec::new();
        let mut values = Vec::with_capacity(items.len());
        for item in items {
            values.push(item.value);
            cuts.extend(item.cuts);
        }
        Tracked {
            value: values.join(separator),
            lines,
            cuts,
        }
    }
}

/// Runs the rule's transform list in written order, recording what
/// `truncate_chars`, `truncate_lines` and `limit_items` remove.
fn run_transforms(rule: &CompiledRule, raw: Vec<RawMatch>) -> (Vec<Tracked>, Vec<TransformLoss>) {
    let mut items: Vec<Tracked> = raw
        .into_iter()
        .map(|m| Tracked {
            value: m.value,
            lines: m.lines,
            cuts: Vec::new(),
        })
        .collect();
    let mut losses = Vec::new();
    for step in &rule.transform_order {
        match step {
            crate::template::TransformStep::Item(i) => {
                let t = &rule.transforms[*i];
                let mut loss = None;
                for item in &mut items {
                    if let Some((from, to)) = transform::item_cut(t, &item.value) {
                        let index = *loss.get_or_insert_with(|| {
                            losses.push(TransformLoss {
                                transform: crate::render::item_transform_name(t),
                                kind: LossKind::Cut {
                                    unit: match t {
                                        crate::template::ItemTransform::TruncateLines(_) => "lines",
                                        _ => "chars",
                                    },
                                    count: 0,
                                    published: Vec::new(),
                                },
                            });
                            losses.len() - 1
                        });
                        if let LossKind::Cut { count, .. } = &mut losses[index].kind {
                            *count += 1;
                        }
                        item.cuts.push((index, from, to));
                    }
                    let value = std::mem::take(&mut item.value);
                    item.value = transform::apply_item_transforms(value, std::slice::from_ref(t));
                }
            }
            crate::template::TransformStep::Result(i) => {
                let t = &rule.result_transforms[*i];
                let before = items.len();
                items = transform::apply_result_transforms(items, std::slice::from_ref(t));
                if matches!(t, crate::template::ResultTransform::LimitItems(_))
                    && items.len() < before
                {
                    losses.push(TransformLoss {
                        transform: crate::render::result_transform_name(t),
                        kind: LossKind::Omitted(before - items.len()),
                    });
                }
            }
        }
    }
    (items, losses)
}

fn process_rule(
    adapter: &dyn Adapter,
    tree: &SourceTree<'_>,
    rule: &CompiledRule,
    max_matches_per_rule: Option<u64>,
) -> RuleResult {
    let base = RuleResult {
        rule_id: rule.id.clone(),
        key: rule.output_key.clone(),
        status: RuleStatus::NotApplicable,
        error: None,
        items: Vec::new(),
        item_lines: Vec::new(),
        truncated_items: 0,
        omitted_items: 0,
        omitted_by_resource: 0,
        truncated_chars_list: Vec::new(),
        truncated_originals: Vec::new(),
        transform_losses: Vec::new(),
    };

    if !rule_applicable(adapter, rule) {
        return base;
    }

    let raw: Vec<RawMatch> = match adapter.extract(tree, rule) {
        Ok(m) => m,
        Err(e) => {
            return RuleResult {
                status: RuleStatus::Error,
                error: Some(e),
                ..base
            };
        }
    };

    let raw_total = raw.len();
    let max_raw = if let Some(limit) = max_matches_per_rule {
        if (raw_total as u64) > limit {
            raw[..limit as usize].to_vec()
        } else {
            raw
        }
    } else {
        raw
    };
    let omitted_by_resource = match max_matches_per_rule {
        Some(limit) => raw_total.saturating_sub(limit as usize),
        None => 0,
    };

    let (mut tracked, mut transform_losses) = run_transforms(rule, max_raw);
    let mut rr = RuleResult {
        status: RuleStatus::Matched,
        ..base
    };

    // Drop surplus items first, so truncation is reported only for items
    // that are actually published.
    if let Some(max) = rule.max_items {
        let max = max as usize;
        if tracked.len() > max {
            rr.omitted_items = tracked.len() - max;
            tracked.truncate(max);
        }
    }

    let mut items = Vec::with_capacity(tracked.len());
    let mut item_lines = Vec::with_capacity(tracked.len());
    for (index, item) in tracked.into_iter().enumerate() {
        let number = index + 1;
        for (loss, from, to) in item.cuts {
            if let LossKind::Cut { published, .. } = &mut transform_losses[loss].kind {
                // Items are visited in order, so the cuts a joined item
                // carries for one step merge into that step's last entry.
                match published.last_mut() {
                    Some(last) if last.item == number => {
                        last.pieces += 1;
                        last.from = last.from.max(from);
                    }
                    _ => published.push(CutItem {
                        item: number,
                        pieces: 1,
                        from,
                        to,
                    }),
                }
            }
        }
        items.push(item.value);
        item_lines.push(item.lines);
    }
    rr.transform_losses = transform_losses;

    let mut truncated_items = 0usize;
    let mut truncated_chars_list = Vec::new();
    let mut truncated_originals = Vec::new();
    if let Some(max) = rule.max_item_chars {
        let max = max as usize;
        for (index, v) in items.iter_mut().enumerate() {
            let chars = v.chars().count();
            if chars > max {
                truncated_chars_list.push((index + 1, chars, max));
                truncated_items += 1;
                let kept = v.chars().take(max).collect();
                truncated_originals.push(std::mem::replace(v, kept));
            }
        }
    }

    rr.items = items;
    rr.item_lines = item_lines;
    rr.truncated_items = truncated_items;
    rr.omitted_by_resource = omitted_by_resource;
    rr.truncated_chars_list = truncated_chars_list;
    rr.truncated_originals = truncated_originals;
    if rr.items.is_empty() {
        rr.status = RuleStatus::NoMatch;
    }
    rr
}

#[cfg(test)]
mod tests {
    use super::*;

    fn result(status: RuleStatus) -> RuleResult {
        RuleResult {
            rule_id: "r".into(),
            key: "k".into(),
            status,
            error: None,
            items: Vec::new(),
            item_lines: Vec::new(),
            truncated_items: 0,
            omitted_items: 0,
            omitted_by_resource: 0,
            truncated_chars_list: Vec::new(),
            truncated_originals: Vec::new(),
            transform_losses: Vec::new(),
        }
    }

    #[test]
    fn file_is_an_error_only_when_every_rule_that_ran_failed() {
        use RuleStatus::*;
        for (statuses, expected) in [
            (vec![], FileStatus::Success),
            (vec![Matched, Matched], FileStatus::Success),
            (vec![Matched, NoMatch], FileStatus::Warning),
            (vec![NotApplicable], FileStatus::Warning),
            (vec![NotApplicable, NotApplicable], FileStatus::Warning),
            (vec![Matched, Error], FileStatus::Warning),
            (vec![NoMatch, Error], FileStatus::Warning),
            (vec![Error, Error], FileStatus::Error),
            // An inapplicable rule did not run: it extracted nothing either.
            (vec![Error, NotApplicable], FileStatus::Error),
            (vec![NotApplicable, Error, NotApplicable], FileStatus::Error),
        ] {
            let results: Vec<_> = statuses.iter().cloned().map(result).collect();
            assert_eq!(file_status(&results), expected, "{statuses:?}");
        }
        let mut truncated = result(Matched);
        truncated.transform_losses.push(TransformLoss {
            transform: "limit_items(1)".into(),
            kind: LossKind::Omitted(2),
        });
        assert!(truncated.is_truncated());
        assert_eq!(file_status(&[truncated]), FileStatus::Warning);
    }

    #[test]
    fn transform_loss_warnings_stay_one_line_per_step() {
        let cut = TransformLoss {
            transform: "truncate_lines(2)".into(),
            kind: LossKind::Cut {
                unit: "lines",
                count: 9,
                published: (1..=7)
                    .map(|item| CutItem {
                        item: item * 2,
                        pieces: 1,
                        from: 5,
                        to: 2,
                    })
                    .collect(),
            },
        };
        assert_eq!(
            transform_loss_warnings(std::slice::from_ref(&cut)),
            vec![
                "9 item(s) truncated by truncate_lines(2): item 2 (5 -> 2 lines), item 4 (5 -> 2 lines), item 6 (5 -> 2 lines), item 8 (5 -> 2 lines), item 10 (5 -> 2 lines), and 2 more; 2 not published"
            ]
        );
        let renumbered = cut.renumbered(|item| (item <= 4).then_some(item / 2));
        assert_eq!(
            transform_loss_warnings(&[renumbered]),
            vec![
                "9 item(s) truncated by truncate_lines(2): item 1 (5 -> 2 lines), item 2 (5 -> 2 lines); 7 not published"
            ]
        );
        // A joined item names its cut pieces once, and they count as shown.
        let joined = TransformLoss {
            transform: "truncate_chars(3)".into(),
            kind: LossKind::Cut {
                unit: "chars",
                count: 5,
                published: vec![CutItem {
                    item: 1,
                    pieces: 4,
                    from: 8,
                    to: 3,
                }],
            },
        };
        assert_eq!(
            transform_loss_warnings(&[joined]),
            vec![
                "5 item(s) truncated by truncate_chars(3): item 1 (4 joined items cut to 3 chars); 1 not published"
            ]
        );
    }
}
