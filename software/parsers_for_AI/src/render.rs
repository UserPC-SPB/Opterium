use crate::limits::CliOptions;
use crate::materialize::{Block, Materialized};
use crate::template::CompiledTemplate;
use std::borrow::Cow;

/// Parser version — re-exported from the crate-level constant so there is a
/// single source of truth (bump it in `lib.rs` only).
pub use crate::PARSER_VERSION;

#[derive(Debug, Clone)]
pub struct FileInfo {
    pub path: String,
    pub size: u64,
    pub mtime: String,
    pub encoding: String,
    pub format: String,
    pub format_source: String,
    pub status: &'static str,
    pub hash: Option<String>,
}

#[derive(Debug, Clone)]
pub struct RunMeta {
    pub generation: String,
    pub started: String,
    pub completed: String,
    pub input_pattern: String,
    pub output_path: String,
    pub files_discovered: usize,
    pub files_processed: usize,
    pub files_success: usize,
    pub files_warning: usize,
    pub files_error: usize,
    pub run_status: &'static str,
    pub format_label: String,
    pub format_source: String,
    pub encoding_label: String,
    pub exit_code: i32,
    pub files: Vec<FileInfo>,
}

pub struct Assembled {
    pub document: String,
    pub blocks: Vec<Block>,
    pub data_chars: usize,
}

pub fn assemble(
    template: &CompiledTemplate,
    cli: &CliOptions,
    meta: &RunMeta,
    mat: &mut Materialized,
) -> Assembled {
    // The decimal character count changes the size of the document itself.
    // Reassemble until that field reaches its fixed point.
    loop {
        let assembled = assemble_once(template, cli, meta, mat);
        let chars = assembled.document.chars().count();
        if chars == mat.total_chars {
            return assembled;
        }
        mat.total_chars = chars;
    }
}

fn assemble_once(
    template: &CompiledTemplate,
    cli: &CliOptions,
    meta: &RunMeta,
    mat: &mut Materialized,
) -> Assembled {
    let (data_part, block_lines) = render_blocks(&mat.blocks);
    // Metadata may quote source text, so it is escaped like data.
    let prelude = escape_anchor_text(&build_prelude(template, cli, meta, mat)).into_owned();
    let index_placeholder = render_index(mat);
    let prelude_lines = count_lines(&prelude) + count_lines(&index_placeholder);
    // `render_blocks` reports positions in block order.
    for (b, (_, s, e)) in mat.blocks.iter_mut().zip(&block_lines) {
        b.start_line = prelude_lines + s;
        b.end_line = prelude_lines + e;
    }
    let index_part = render_index(mat);
    let document = format!(
        "{}{}{}\nEnd of any_txt_parser output\n",
        prelude, index_part, data_part
    );
    Assembled {
        document,
        blocks: mat.blocks.clone(),
        data_chars: data_part.chars().count(),
    }
}

fn build_prelude(
    template: &CompiledTemplate,
    cli: &CliOptions,
    meta: &RunMeta,
    mat: &Materialized,
) -> String {
    let mut out = String::new();
    out.push_str("# any_txt_parser output\n\n");
    out.push_str("Triangle: [0,1,0]\n");
    out.push_str(&format!("Generation: {}\n", meta.generation));
    out.push_str(&format!("Updated: {}\n\n", meta.completed));
    out.push_str(&format!("Parser version: {}\n\n", PARSER_VERSION));

    out.push_str("## Execution\n\n");
    out.push_str(&format!("Parser version: {}\n", PARSER_VERSION));
    out.push_str(&format!("Template version: {}\n", template.version));
    out.push_str(&format!("Generation: {}\n", meta.generation));
    out.push_str(&format!("Started: {}\n", meta.started));
    out.push_str(&format!("Completed: {}\n\n", meta.completed));
    out.push_str("Input:\n");
    out.push_str(&format!("{}\n\n", md_line(&meta.input_pattern)));
    out.push_str("Output:\n");
    out.push_str(&format!("{}\n\n", md_line(&meta.output_path)));
    out.push_str(&format!("Files discovered: {}\n", meta.files_discovered));
    out.push_str(&format!("Files processed: {}\n", meta.files_processed));
    out.push_str(&format!("Files successful: {}\n", meta.files_success));
    out.push_str(&format!("Files warnings: {}\n", meta.files_warning));
    out.push_str(&format!("Files failed: {}\n", meta.files_error));
    out.push_str(&format!("Total extracted items: {}\n", mat.total_items));
    out.push_str(&format!("Total output characters: {}\n", mat.total_chars));
    out.push_str(&format!("Total output blocks: {}\n", mat.blocks.len()));
    out.push_str(&format!("Effective format: {}\n", meta.format_label));
    out.push_str(&format!("Format source: {}\n", meta.format_source));
    out.push_str(&format!("Encoding: {}\n", meta.encoding_label));
    out.push_str(&format!("Run status: {}\n", meta.run_status));
    out.push_str(&format!("Exit code: {}\n", meta.exit_code));

    if !meta.files.is_empty() {
        out.push_str("\n## Files\n\n");
        // Encoding comes after Status, so `| json | success |` still
        // finds a row.
        out.push_str("| Path | Size | Modified | Format | Status | Encoding | Hash |\n");
        out.push_str("|---|---:|---|---|---|---|---|\n");
        for f in &meta.files {
            out.push_str(&format!(
                "| {} | {} | {} | {} | {} | {} | {} |\n",
                md_cell(&f.path),
                f.size,
                f.mtime,
                f.format,
                f.status,
                md_cell(&f.encoding),
                f.hash.clone().unwrap_or_else(|| "-".to_string())
            ));
        }
    }

    if !mat.errors.is_empty() {
        out.push_str("\n## Diagnostics\n\n");
        for e in &mat.errors {
            out.push_str(&format!("- {}\n", md_line(e)));
        }
    }
    if !mat.truncated.is_empty() {
        out.push_str("\n## Truncation\n\n");
        for t in &mat.truncated {
            out.push_str(&format!("- {}\n", md_line(t)));
        }
    }

    out.push_str("\n## Applied Template\n\n");
    out.push_str(&render_applied_template(template));

    out.push_str("\n## Effective Template\n\n```yaml\n");
    out.push_str(&crate::template::effective_template_yaml(template));
    out.push_str("```\n");

    out.push_str("\n## Result Semantics\n\n");
    out.push_str("This file contains extracted data from the declared source files.\n\n");
    out.push_str("Each data block:\n");
    out.push_str("- belongs to one source file;\n");
    out.push_str("- belongs to one extraction rule;\n");
    out.push_str("- contains only data accepted by that rule;\n");
    out.push_str("- may be transformed or truncated according to the Applied Template;\n");
    out.push_str("- has a stable block anchor.\n\n");
    out.push_str("A block with \"no matches found\" means the source was processed successfully but the rule produced zero results.\n\n");
    if template.rules.iter().any(|r| r.when_format.is_some()) {
        out.push_str("A rule with \"Applies to format\" runs only on source files of that format; other files get no block for it, and their status is not affected.\n\n");
        if no_rule_applies(template) {
            out.push_str("In this run no rule applies to the input format, so nothing was extracted and every source file that was read is a warning (see Diagnostics).\n\n");
        }
    }
    out.push_str(
        "A warning or error means the result may be incomplete for that source or rule.\n",
    );

    out.push_str("\n## CLI Arguments\n\n");
    let args: Vec<String> = cli.raw_args.iter().map(|a| format!("`{}`", a)).collect();
    out.push_str(&args.join(" "));
    out.push('\n');

    out
}

/// The template in prose. Values are written one line each and escaped
/// for Markdown (see [`md_inline`], [`md_line`]); `## Effective Template`
/// holds them exactly.
pub fn render_applied_template(template: &CompiledTemplate) -> String {
    let mut out = String::new();
    out.push_str(&format!("Template name: {}\n", md_inline(&template.name)));
    out.push_str(&format!("Template version: {}\n", template.version));
    out.push_str(&format!(
        "Template source: {}\n\n",
        md_inline(&template.source_path.display().to_string())
    ));
    out.push_str(&format!(
        "Input format: {}\n",
        template
            .format
            .as_ref()
            .map(|f| f.name().to_string())
            .unwrap_or_else(|| "auto".to_string())
    ));
    out.push_str(&format!("Format source: {}\n", template.format_source));
    out.push_str(&format!("Encoding: {}\n", template.encoding));
    out.push_str(&format!(
        "Encoding source: {}\n\n",
        template.encoding_source
    ));
    out.push_str("Rules:\n\n");

    for r in &template.rules {
        out.push_str(&format!("### Rule: {}\n\n", md_inline(&r.id)));
        out.push_str(&format!("Selector type: {}\n", r.selector_type));
        out.push_str("Selector:\n");
        out.push_str(&format!("{}\n\n", md_line(&r.selector_expression)));
        out.push_str("Output key:\n");
        out.push_str(&format!("{}\n\n", md_line(&r.output_key)));
        // As in Effective Template, the default `text` is not written.
        if let Some(mode) = extract_name(&r.extract) {
            out.push_str(&format!("Extract: {}\n", md_inline(&mode)));
        }
        if let Some(wf) = &r.when_format {
            out.push_str(&format!("Applies to format: {}", wf.name()));
            if rule_never_applies(template, r) {
                out.push_str(&format!(
                    " (never applies in this run: every file is processed as {}, format source: {})",
                    template.format.as_ref().map_or("auto", |f| f.name()),
                    template.format_source
                ));
            }
            out.push('\n');
        }
        if r.context.before > 0 || r.context.after > 0 {
            out.push_str(&format!(
                "Context: before={}, after={}\n",
                r.context.before, r.context.after
            ));
        }
        out.push_str("Transforms:\n");
        if r.transforms.is_empty() && r.result_transforms.is_empty() {
            out.push_str("- none\n");
        } else {
            for step in &r.transform_order {
                let name = match step {
                    crate::template::TransformStep::Item(i) => {
                        item_transform_name(&r.transforms[*i])
                    }
                    crate::template::TransformStep::Result(i) => {
                        result_transform_name(&r.result_transforms[*i])
                    }
                };
                out.push_str(&format!("- {}\n", md_line(&name)));
            }
        }
        out.push_str("Limits:\n");
        match (r.max_items, r.max_item_chars) {
            (None, None) => out.push_str("- none\n"),
            _ => {
                if let Some(mi) = r.max_items {
                    out.push_str(&format!("- max_items: {}\n", mi));
                }
                if let Some(mc) = r.max_item_chars {
                    out.push_str(&format!("- max_item_chars: {}\n", mc));
                }
            }
        }
        out.push('\n');
    }
    out
}

/// Whether the format fixed for every file (by `--format` or the template's
/// `input.format`) is not the one the rule's `when.format` selects, so the
/// rule runs on no file at all.
pub fn rule_never_applies(
    template: &CompiledTemplate,
    rule: &crate::template::CompiledRule,
) -> bool {
    match &template.format {
        Some(forced) => !crate::collect::selected_for_format(rule, forced),
        None => false,
    }
}

/// Whether the fixed input format excludes every rule of the template, so
/// the run extracts nothing by construction.
pub fn no_rule_applies(template: &CompiledTemplate) -> bool {
    !template.rules.is_empty()
        && template
            .rules
            .iter()
            .all(|rule| rule_never_applies(template, rule))
}

/// `extract` as a template writes it; `None` for the default `text`.
fn extract_name(mode: &crate::template::ExtractMode) -> Option<String> {
    use crate::template::ExtractMode;
    match mode {
        ExtractMode::Text => None,
        ExtractMode::Attr(name) => Some(format!("attr:{}", name)),
        ExtractMode::InnerHtml => Some("inner_html".to_string()),
        ExtractMode::OuterHtml => Some("outer_html".to_string()),
    }
}

pub(crate) fn item_transform_name(t: &crate::template::ItemTransform) -> String {
    match t {
        crate::template::ItemTransform::Trim => "trim".to_string(),
        crate::template::ItemTransform::NormalizeWhitespace => "normalize_whitespace".to_string(),
        crate::template::ItemTransform::Lowercase => "lowercase".to_string(),
        crate::template::ItemTransform::Uppercase => "uppercase".to_string(),
        crate::template::ItemTransform::Unescape => "unescape".to_string(),
        crate::template::ItemTransform::TruncateChars(n) => format!("truncate_chars({})", n),
        crate::template::ItemTransform::TruncateLines(n) => format!("truncate_lines({})", n),
        crate::template::ItemTransform::Replace { from, to } => {
            format!("replace({} -> {})", from, to)
        }
        crate::template::ItemTransform::RegexReplace {
            pattern,
            replacement,
        } => {
            format!("regex_replace({} -> {})", pattern.as_str(), replacement)
        }
    }
}

pub(crate) fn result_transform_name(t: &crate::template::ResultTransform) -> String {
    match t {
        crate::template::ResultTransform::DropEmpty => "drop_empty".to_string(),
        crate::template::ResultTransform::Unique => "unique".to_string(),
        crate::template::ResultTransform::SortAsc => "sort".to_string(),
        crate::template::ResultTransform::SortDesc => "sort_desc".to_string(),
        crate::template::ResultTransform::LimitItems(n) => format!("limit_items({})", n),
        crate::template::ResultTransform::Join(sep) => format!("join({})", sep),
    }
}

fn render_index(mat: &Materialized) -> String {
    let mut out = String::new();
    out.push_str("\n## Block Index\n\n");
    out.push_str("| Block | Source | Rule | Key | Items | Size | Start | End | Anc |\n");
    out.push_str("|---|---|---|---|---:|---:|---:|---:|---|\n");
    for b in &mat.blocks {
        out.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} |\n",
            b.id,
            md_cell(&b.source),
            md_cell(&b.rule_id),
            md_cell(&b.key),
            b.item_count,
            kb(b.chars),
            if b.start_line > 0 {
                b.start_line.to_string()
            } else {
                "-".to_string()
            },
            if b.end_line > 0 {
                b.end_line.to_string()
            } else {
                "-".to_string()
            },
            b.id
        ));
    }
    out.push_str("\n### Block Anchors\n\n");
    for b in &mat.blocks {
        out.push_str(&format!("- {} → BLOCK:{}", b.id, b.id));
        if b.start_line > 0 {
            out.push_str(&format!(" (lines {}-{})", b.start_line, b.end_line));
        }
        if let Some(p) = &b.parent_id {
            out.push_str(&format!(
                " (part {}/{} of {})",
                b.part.unwrap_or(0),
                b.parts.unwrap_or(1),
                p
            ));
        } else if let Some(n) = b.parts {
            if n > 0 && b.split {
                out.push_str(&format!(" (parent; continuation parts: {})", n));
            }
        }
        out.push('\n');
    }
    escape_anchor_text(&out).into_owned()
}

fn kb(chars: usize) -> String {
    let kb10 = chars * 10 / 1024;
    format!("{}.{} KB", kb10 / 10, kb10 % 10)
}

fn render_blocks(blocks: &[Block]) -> (String, Vec<(String, usize, usize)>) {
    let mut out = String::new();
    out.push_str("\n## Data Blocks\n\n");
    let mut lines = count_lines(&out);
    let mut positions = Vec::new();
    for b in blocks {
        let start_line = lines + 1;
        let rendered = render_block_document(b);
        out.push_str(&rendered);
        lines += count_lines(&rendered);
        positions.push((b.id.clone(), start_line, lines));
    }
    (out, positions)
}

/// Items are stored already escaped (see `materialize`); block metadata is
/// escaped here.
pub(crate) fn render_block_document(b: &Block) -> String {
    let mut out = start_anchor(&b.id);
    out.push_str(&escape_anchor_text(&render_block_head(b, b.item_count)));
    if b.items.is_empty() {
        out.push_str(empty_data_text(b));
    } else {
        for item in &b.items {
            out.push_str(&render_item(item));
        }
    }
    out.push_str(&escape_anchor_text(&render_block_tail(b)));
    out.push_str(&end_anchor(&b.id));
    out
}

/// Exact `render_block_document(b).chars().count()` for `b` holding
/// `item_count` items whose `rendered_item_chars` add up to `items_chars`.
/// Only the metadata is rendered, so block splitting stays linear.
pub(crate) fn block_document_chars(b: &Block, item_count: usize, items_chars: usize) -> usize {
    let data = if item_count == 0 {
        empty_data_text(b).chars().count()
    } else {
        items_chars
    };
    start_anchor(&b.id).chars().count()
        + escape_anchor_text(&render_block_head(b, item_count))
            .chars()
            .count()
        + data
        + escape_anchor_text(&render_block_tail(b)).chars().count()
        + end_anchor(&b.id).chars().count()
}

/// Exact character count of `render_item(item)`: the `- ` marker, the text,
/// four indentation spaces after every newline and the final newline.
pub(crate) fn rendered_item_chars(item: &str) -> usize {
    let (chars, newlines) = item.chars().fold((0, 0), |(chars, newlines), ch| {
        (chars + 1, newlines + usize::from(ch == '\n'))
    });
    chars + 4 * newlines + 3
}

fn start_anchor(id: &str) -> String {
    format!("<!-- BLOCK: {} START -->\n", id)
}

fn end_anchor(id: &str) -> String {
    format!("\n<!-- BLOCK: {} END -->\n", id)
}

fn empty_data_text(b: &Block) -> &'static str {
    if b.split && b.part.is_none() {
        // The parent of a split group: `Items` counts what its parts hold.
        return "(in the continuation parts)\n";
    }
    match b.status {
        "no matches found" => "no matches found\n",
        "rule not applicable" => "rule not applicable\n",
        "error" => "(error)\n",
        _ => "(no data)\n",
    }
}

fn render_block_head(b: &Block, item_count: usize) -> String {
    let mut out = String::new();
    out.push_str(&format!("## Block: {}\n\n", b.id));
    if let Some(p) = &b.parent_id {
        out.push_str(&format!("Parent: {}\n", p));
    }
    if b.parts.is_some() && b.part.is_some() {
        out.push_str(&format!("Part: {}/{}\n", b.part.unwrap(), b.parts.unwrap()));
    } else if b.parts.is_some() && b.part.is_none() {
        out.push_str(&format!(
            "Parent block: continuation parts follow ({})\n",
            b.parts.unwrap()
        ));
    }
    if let Some(index) = b.item_index {
        out.push_str(&format!("Item continuation: {}\n", index));
    }
    out.push_str(&format!("Source: {}\n", md_inline(&b.source)));
    out.push_str(&format!("Format: {}\n", b.format));
    out.push_str(&format!("Format source: {}\n", b.format_source));
    out.push_str("\n");
    out.push_str(&format!("Rule: {}\n", md_inline(&b.rule_id)));
    out.push_str(&format!("Output key: {}\n", md_inline(&b.key)));
    out.push_str("\n");
    out.push_str(&format!("Status: {}\n", b.status));
    if b.file_status != "success" {
        out.push_str(&format!("File status: {}\n", b.file_status));
    }
    out.push_str(&format!("Items: {}\n", item_count));
    out.push_str(&format!("Truncated: {}\n", yesno(b.truncated)));
    out.push_str(&format!("Split: {}\n", yesno(b.split)));
    out.push_str("\n### Data\n\n");
    out
}

fn render_block_tail(b: &Block) -> String {
    let mut out = String::new();
    if !b.errors.is_empty() {
        out.push_str("\n### Errors\n\n");
        for e in &b.errors {
            out.push_str(&format!("- {}\n", md_line(e)));
        }
    }
    if !b.warnings.is_empty() {
        out.push_str("\n### Warnings\n\n");
        for w in &b.warnings {
            out.push_str(&format!("- {}\n", md_line(w)));
        }
    }
    out
}

fn render_item(item: &str) -> String {
    if !item.contains('\n') {
        return format!("- {}\n", item);
    }
    let mut lines = item.split('\n');
    let mut out = String::new();
    if let Some(first) = lines.next() {
        out.push_str(&format!("- {}\n", first));
        for rest in lines {
            out.push_str("    ");
            out.push_str(rest);
            out.push('\n');
        }
    }
    out
}

/// Text written into one line of the document (a diagnostic, a table cell,
/// a block header field, a value in Applied Template): `\n` and `\r` are
/// written as those escapes. Parser messages quote source text (serde_yaml
/// copies map keys into its error paths, the TOML parser shows the offending
/// line), and template values may contain line breaks; either would start
/// lines of its own, such as a forged `## Block Index` with table rows.
/// Data items keep their line breaks: their lines are indented.
fn one_line(text: &str) -> Cow<'_, str> {
    if !text.contains(['\n', '\r']) {
        return Cow::Borrowed(text);
    }
    let mut out = String::with_capacity(text.len() + 8);
    for ch in text.chars() {
        match ch {
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            ch => out.push(ch),
        }
    }
    Cow::Owned(out)
}

/// Metadata inside a line of the document (after a label such as `Rule: `,
/// in a heading): written on one line ([`one_line`]), with every `<!--`
/// as `\<!--`, so a Markdown renderer does not take the text up to the
/// next `-->` (a block anchor) for a comment and hide it. Data items are
/// never escaped. Anchor escaping ([`escape_anchor_text`]) runs on top and
/// still finds every `<!-- BLOCK…:` run.
fn md_inline(text: &str) -> Cow<'_, str> {
    let line = one_line(text);
    if !line.contains("<!--") {
        return line;
    }
    Cow::Owned(line.replace("<!--", "\\<!--"))
}

/// Metadata that begins a line or a list item of its own (`Selector:` and
/// `Output key:` values, `Input:`, list items): [`md_inline`], plus a `\`
/// before a leading character that would start a Markdown block there
/// (`## x` a heading, `> x` a quote, `- x` or `1. x` a list, `---` a rule
/// or setext underline, a code fence, HTML, a table delimiter row, a link
/// reference definition). The backslash is a Markdown escape: a renderer
/// shows the value as written.
fn md_line(text: &str) -> Cow<'_, str> {
    let line = md_inline(text);
    let body = line.trim_start_matches([' ', '\t']);
    let at = line.len() - body.len();
    let Some(first) = body.chars().next() else {
        return line;
    };
    let ends_marker = |rest: &str| rest.is_empty() || rest.starts_with([' ', '\t']);
    // `---`, `***`, `___`, `===`: a thematic break or a setext underline.
    let only_marks = |marks: &[char]| {
        marks.contains(&first) && body.chars().all(|c| matches!(c, ' ' | '\t') || c == first)
    };
    let digits = body.chars().take_while(char::is_ascii_digit).count();
    let escape = match first {
        '#' => {
            let hashes = body.chars().take_while(|c| *c == '#').count();
            hashes <= 6 && ends_marker(&body[hashes..])
        }
        '>' | '<' | '|' => true,
        '-' | '+' | '*' if ends_marker(&body[1..]) => true,
        '`' => body.starts_with("```"),
        '~' => body.starts_with("~~~"),
        '[' => body
            .find("]:")
            .is_some_and(|end| end > 1 && !body[1..end].contains(']')),
        _ => false,
    } || only_marks(&['-', '*', '_', '='])
        || (body.contains('|')
            && body.contains('-')
            && body
                .chars()
                .all(|c| matches!(c, '|' | '-' | ':' | ' ' | '\t')));
    if escape {
        let mut out = String::with_capacity(line.len() + 1);
        out.push_str(&line[..at]);
        out.push('\\');
        out.push_str(body);
        return Cow::Owned(out);
    }
    // `1. x` / `1) x`: an ordered list; escape the delimiter.
    if (1..=9).contains(&digits)
        && body[digits..].starts_with(['.', ')'])
        && ends_marker(&body[digits + 1..])
    {
        let split = at + digits;
        return Cow::Owned(format!("{}\\{}", &line[..split], &line[split..]));
    }
    line
}

/// A table cell: [`md_inline`], with `|` as `\|` so the cell keeps its
/// value instead of splitting the row.
fn md_cell(text: &str) -> Cow<'_, str> {
    let cell = md_inline(text);
    if !cell.contains('|') {
        return cell;
    }
    Cow::Owned(cell.replace('|', "\\|"))
}

fn yesno(b: bool) -> &'static str {
    if b { "yes" } else { "no" }
}

fn count_lines(s: &str) -> usize {
    s.bytes().filter(|b| *b == b'\n').count()
}

/// Text that every block anchor starts with.
pub const ANCHOR_MARK: &str = "<!-- BLOCK:";

/// Makes text safe to place between block anchors. Every `<!-- BLOCK` that
/// is followed by a run of zero or more `\` and then `:` gets one more `\`,
/// so `<!-- BLOCK:` itself only ever appears in anchor lines written by the
/// renderer. [`unescape_anchor_text`] restores the original text.
pub fn escape_anchor_text(text: &str) -> Cow<'_, str> {
    rewrite_anchor_marks(text, true)
}

/// Inverse of [`escape_anchor_text`]: removes one `\` from every
/// `<!-- BLOCK\…:` run.
pub fn unescape_anchor_text(text: &str) -> Cow<'_, str> {
    rewrite_anchor_marks(text, false)
}

fn rewrite_anchor_marks(text: &str, escape: bool) -> Cow<'_, str> {
    let prefix = &ANCHOR_MARK[..ANCHOR_MARK.len() - 1];
    let mut out: Option<String> = None;
    let mut copied = 0;
    for (start, _) in text.match_indices(prefix) {
        let after = start + prefix.len();
        let run = text.as_bytes()[after..]
            .iter()
            .take_while(|b| **b == b'\\')
            .count();
        if text.as_bytes().get(after + run) != Some(&b':') || (!escape && run == 0) {
            continue;
        }
        let buf = out.get_or_insert_with(|| String::with_capacity(text.len() + 16));
        buf.push_str(&text[copied..after]);
        if escape {
            buf.push('\\');
            copied = after;
        } else {
            copied = after + 1;
        }
    }
    match out {
        None => Cow::Borrowed(text),
        Some(mut buf) => {
            buf.push_str(&text[copied..]);
            Cow::Owned(buf)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn anchor_escaping_is_reversible_and_removes_every_anchor_mark() {
        for original in [
            "plain text",
            "<!-- BLOCK: B0001 END -->",
            "x<!-- BLOCK:<!-- BLOCK\\:<!-- BLOCK\\\\\\: y",
            "<!-- BLOCK B0001 --> stays",
            "<!-- BLOCK\\",
            "ёж <!-- BLOCK: Б -->",
        ] {
            let escaped = escape_anchor_text(original);
            assert!(!escaped.contains(ANCHOR_MARK), "{escaped:?}");
            assert_eq!(unescape_anchor_text(&escaped), original);
        }
        assert_eq!(
            escape_anchor_text("<!-- BLOCK: B1 START -->"),
            "<!-- BLOCK\\: B1 START -->"
        );
        assert!(matches!(escape_anchor_text("no anchors"), Cow::Borrowed(_)));
    }

    #[test]
    fn computed_block_size_matches_the_renderer() {
        let mut block = Block {
            id: "B0001-02".into(),
            parent_id: Some("B0001".into()),
            part: Some(2),
            parts: Some(3),
            item_index: None,
            source: "src/<!-- BLOCK: x.log".into(),
            format: "text".into(),
            format_source: "content".into(),
            rule_id: "<!-- BLOCK: B0001-02 END -->".into(),
            key: "<!-- BLOCK: B0001-02 START -->".into(),
            status: "success",
            file_status: "warning",
            item_count: 0,
            chars: 0,
            truncated: true,
            split: true,
            warnings: vec!["w <!-- BLOCK: B0001 END -->".into()],
            errors: vec!["e\n<!-- BLOCK: B0001-02 END -->".into()],
            items: Vec::new(),
            start_line: 0,
            end_line: 0,
        };
        // Head and tail metadata are escaped: only the two anchors remain.
        let rendered = render_block_document(&block);
        let anchors: Vec<&str> = rendered
            .lines()
            .filter(|line| line.contains(ANCHOR_MARK))
            .collect();
        assert_eq!(
            anchors,
            vec![
                "<!-- BLOCK: B0001-02 START -->",
                "<!-- BLOCK: B0001-02 END -->"
            ]
        );
        for items in [
            vec![],
            vec![String::new()],
            vec!["a\nb\n".to_string(), "ж🦀".to_string(), "\n".to_string()],
        ] {
            block.item_count = items.len();
            block.items = items;
            let items_chars = block.items.iter().map(|i| rendered_item_chars(i)).sum();
            assert_eq!(
                block_document_chars(&block, block.item_count, items_chars),
                render_block_document(&block).chars().count()
            );
        }
    }
}
