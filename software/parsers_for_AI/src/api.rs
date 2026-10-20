//! In-memory extraction API: data back, no template file, no Markdown log.
//!
//! This is the entry point for callers that want the extracted data itself
//! (the short-key CLI, the MCP server): [`extract`] runs the same pipeline
//! as the published document — discovery, decoding, format detection,
//! adapters, transforms, limits, statuses and the run outcome — and returns
//! an [`ExtractResponse`]; [`recon`] looks at files before any rule is
//! written. [`render_text`] / [`render_recon_text`] give a compact plain-text
//! form for stdout, [`to_json`] the JSON form.
//!
//! Rules come from a template file ([`ExtractRequest::template_path`]),
//! inline template text ([`ExtractRequest::template`], YAML or JSON) or a
//! list of [`RuleSpec`]s: shorthand strings `TYPE:EXPR` or rule objects of
//! the template DSL. Every form is compiled by the template compiler, so the
//! validation, defaults and messages are those of a template file.
//!
//! Errors that stop a run before any source is processed are an
//! [`ApiError`] carrying the CLI exit code: `2` invalid template or rule,
//! `3` invalid request or input (missing input, bad format/encoding/SIZE
//! value, no file matched), `6` resource limit. A run that processed its
//! sources is an `Ok` response whose `exit_code` follows the CLI (`0`
//! success, `1` partial, `3` every file failed).
//!
//! Third-party parsers recurse up to their own nesting limits: call these
//! functions on a thread with a large stack, as the CLI does
//! ([`crate::run::run_guarded`], [`crate::run::WORKER_STACK_BYTES`]).

use crate::adapters::{SourceLines, SourceTree, XmlTree, adapter_for};
use crate::confine;
use crate::discovery::{self, DiscoveredFile};
use crate::limits::{ExitCode, ResourceLimits};
use crate::materialize;
use crate::run::{self, RunError};
use crate::template::{self, CompiledTemplate, Format};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_yaml::{Mapping, Value};
use std::fmt;
use std::path::{Path, PathBuf};

/// Selector types of the shorthand form `TYPE:EXPR`, in help order.
pub const SHORTHAND_TYPES: &[&str] = &[
    "contains",
    "starts_with",
    "ends_with",
    "regex",
    "jsonpath",
    "path",
    "xpath",
    "css",
    "line",
    "markdown",
];

/// Request of [`extract`]. Every field except `input` is optional; exactly
/// one of `template_path`, `template` and `rules` gives the rules.
///
/// Deserializes from JSON or YAML with snake_case field names; unknown
/// fields are rejected. SIZE fields (`max_file_size`, `max_memory`) take a
/// number of bytes or a SIZE string (`50m`, `2g`; suffixes b/k/m/g, x1024).
#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct ExtractRequest {
    /// Input file, directory (recursive) or glob pattern (`*`, `**/`).
    pub input: String,
    /// Template file (`version: 1`), as for `-t`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template_path: Option<PathBuf>,
    /// Inline template text: YAML, or JSON of the same structure.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template: Option<String>,
    /// Rules without a template; see [`RuleSpec`].
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub rules: Vec<RuleSpec>,
    /// Forces the format of every file (`json|xml|html|markdown|yaml|toml|text`,
    /// aliases as in the template). `auto` or absent keeps detection
    /// (`input.format` of a template still applies). Reported as format
    /// source `CLI override`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    /// Source encoding, as `--encoding` (default `utf-8`, BOM stripped).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encoding: Option<String>,
    pub follow_symlinks: bool,
    /// Adds the sha256 of every source to its [`FileReport`].
    pub hash: bool,
    /// Max discovered files (default 5000).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_files: Option<u64>,
    /// Max size of one input file (default 200m).
    #[serde(
        deserialize_with = "size_value",
        skip_serializing_if = "Option::is_none"
    )]
    pub max_file_size: Option<String>,
    /// Match cap per rule before transforms (reported as truncation).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_matches: Option<u64>,
    /// Cumulative decoded UTF-8 byte budget across all files.
    #[serde(
        deserialize_with = "size_value",
        skip_serializing_if = "Option::is_none"
    )]
    pub max_memory: Option<String>,
    /// Shorthand rules only: lines of context before a text match.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_before: Option<usize>,
    /// Shorthand rules only: lines of context after a text match.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_after: Option<usize>,
    /// Shorthand rules only: `text`, `attr:NAME`, `inner_html`, `outer_html`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extract: Option<String>,
    /// Shorthand rules only: the transform list, each entry in DSL form as
    /// one YAML/JSON value: `trim`, `unique`, `limit_items: 20`,
    /// `{replace: {from: a, to: b}}`. JSON input may also give an entry as
    /// an object (`{"limit_items": 20}`).
    #[serde(
        deserialize_with = "transform_list",
        skip_serializing_if = "Vec::is_empty"
    )]
    pub transforms: Vec<String>,
    /// Shorthand rules only: `limits.max_items`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_items: Option<u64>,
    /// Shorthand rules only: `limits.max_item_chars`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_item_chars: Option<u64>,
    /// Cap on the characters of all returned items together. When the next
    /// item would exceed it, no further items are returned: the response
    /// has `truncated_output: true`, a note says where it stopped, and the
    /// cut results carry `truncated: true` and a warning. Items are never
    /// cut in the middle. Absent: the template's `output.max_output_chars`
    /// (if any) is used the same way; there is no other cap.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_output_chars: Option<usize>,
    /// Keep results that found nothing (`no matches found`) in the
    /// response. By default they are left out and counted in a note, and a
    /// file is not partial just because one rule found nothing in it; a
    /// rule that found nothing in any file is still a warning.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub include_empty: bool,
    /// Every input file and the template file must lie inside this folder
    /// (after resolving `..`, symlinks and junctions); anything else is an
    /// error (exit 3). Not part of the JSON request: the MCP server sets it
    /// to its root.
    #[serde(skip)]
    pub confine_to: Option<PathBuf>,
}

/// One rule of [`ExtractRequest::rules`].
///
/// # Shorthand: `TYPE:EXPR`
///
/// ```text
/// TYPE         EXPR                                              formats
/// contains     substring of a line                               text
/// starts_with  prefix of a line                                  text
/// ends_with    suffix of a line                                  text
/// regex        regex matched per line; exactly one capture        text
///              group -> the group is the item, else the line
/// jsonpath     JSONPath subset: $.users[*].name,                 json
///              $.items[?(@.price > 10)].id
/// path         plain path: $.server.port, servers[0].name,       json yaml toml
///              [database].host
/// xpath        XPath subset: //item/@id, /root/a/text()          xml
/// css          CSS selector: h2, a[href], table tr               html
/// line         N | N-M | N- | -M  (1-based, inclusive; N- runs   text
///              to the end): the range is ONE item
/// markdown     NODE or NODE:PATTERN; NODE heading|paragraph|     markdown
///              list|list_item|table|table_row|table_cell|
///              code_block|blockquote|link|image (aliases h, p,
///              li, tr, td, code, quote, img); PATTERN a regex
///              the node text must match
/// ```
///
/// - The string is split at its FIRST `:`. TYPE is case-insensitive and may
///   have spaces around it. EXPR is everything after that colon, verbatim:
///   no trimming, quoting or escaping, so `regex:(?i)error|fail`,
///   `contains:key: value`, `jsonpath:$.a[?(@.b == 'x')]` and
///   `xpath://a[@x='1']/@y` are written as they are. `contains: x` looks for
///   ` x` with its space. EXPR must not be empty.
/// - `line` ignores spaces around its numbers; line numbers start at 1.
///   `line:5` is line 5, `line:10-20`, `line:10-` (line 10 to the end),
///   `line:-20` (lines 1-20).
/// - `markdown`: NODE ends at the next `:` (spaces around it ignored);
///   PATTERN after it is verbatim (`markdown:heading:^Install`).
/// - Unknown TYPE, a missing `:`, an empty EXPR and invalid expressions are
///   template errors (exit 2) that name the rule and list the valid types.
/// - A string that starts with `{` (after spaces) is not shorthand but a
///   rule object in YAML/JSON flow form: `{select: {type: regex,
///   expression: 'id=(\d+)'}, transforms: [trim]}`.
/// - The request-level options `context_before`, `context_after`,
///   `extract`, `transforms`, `max_items` and `max_item_chars` apply to
///   every shorthand rule (and to no rule object). Setting one without any
///   shorthand rule is a request error (exit 3).
///
/// A selector type that does not fit a file's format is not an error: the
/// result has status `rule not applicable` (file `warning`), and the
/// response notes which selector types that format takes.
///
/// # Rule object
///
/// The fields of a template rule: `id`, `when: {format}`, `select`,
/// `context: {before, after}`, `output: {key}`, `extract`, `transforms`,
/// `limits: {max_items, max_item_chars}` — see HELP.md.
///
/// # Ids and keys
///
/// A rule without an id gets `r<N>`, N being its 1-based position in
/// `rules` (`r1`, `r2`, ...); the output key defaults to the id. An
/// explicit id that equals a generated one is a duplicate (exit 2).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum RuleSpec {
    /// `TYPE:EXPR`, or a rule object written as YAML/JSON flow text.
    Shorthand(String),
    /// A rule object of the template DSL.
    Object(Mapping),
}

impl From<&str> for RuleSpec {
    fn from(spec: &str) -> Self {
        RuleSpec::Shorthand(spec.to_string())
    }
}

/// Untagged like the derived form (a string is shorthand, a mapping a rule
/// object), with an error message that says so.
impl<'de> Deserialize<'de> for RuleSpec {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        match Value::deserialize(deserializer)? {
            Value::String(spec) => Ok(RuleSpec::Shorthand(spec)),
            Value::Mapping(rule) => Ok(RuleSpec::Object(rule)),
            other => Err(serde::de::Error::custom(format!(
                "a rule must be a 'TYPE:EXPR' string or a rule object, got {}",
                value_kind(&other)
            ))),
        }
    }
}

/// A SIZE given as a number of bytes or a string.
fn size_value<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Option<String>, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Size {
        Bytes(u64),
        Text(String),
    }
    Ok(
        Option::<Size>::deserialize(deserializer)?.map(|size| match size {
            Size::Bytes(n) => n.to_string(),
            Size::Text(text) => text,
        }),
    )
}

/// Transform entries as strings; a structured entry is kept as its JSON
/// text, which is also a YAML flow value.
fn transform_list<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<String>, D::Error> {
    let entries = Option::<Vec<serde_json::Value>>::deserialize(deserializer)?.unwrap_or_default();
    Ok(entries
        .into_iter()
        .map(|entry| match entry {
            serde_json::Value::String(text) => text,
            other => other.to_string(),
        })
        .collect())
}

fn value_kind(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "a boolean",
        Value::Number(_) => "a number",
        Value::String(_) => "a string",
        Value::Sequence(_) => "a list",
        Value::Mapping(_) => "a mapping",
        Value::Tagged(_) => "a tagged value",
    }
}

/// An error that stopped the request before a response: `code` is the CLI
/// exit code (2 template/rule, 3 request/input, 5 internal, 6 resource
/// limit). Serializes as `{"exit_code": N, "message": "..."}`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApiError {
    pub code: ExitCode,
    pub message: String,
}

impl ApiError {
    pub fn new(code: ExitCode, message: impl Into<String>) -> Self {
        ApiError {
            code,
            message: message.into(),
        }
    }

    fn input(message: impl Into<String>) -> Self {
        ApiError::new(ExitCode::InvalidInputOrCli, message)
    }

    fn template(message: impl Into<String>) -> Self {
        ApiError::new(ExitCode::InvalidTemplate, message)
    }

    /// The process exit code for this error.
    pub fn exit_code(&self) -> i32 {
        self.code.to_i32()
    }
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} (exit {})", self.message, self.exit_code())
    }
}

impl std::error::Error for ApiError {}

impl From<RunError> for ApiError {
    fn from(e: RunError) -> Self {
        ApiError::new(e.code, e.message)
    }
}

impl Serialize for ApiError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut s = serializer.serialize_struct("ApiError", 2)?;
        s.serialize_field("exit_code", &self.exit_code())?;
        s.serialize_field("message", &self.message)?;
        s.end()
    }
}

/// Result of [`extract`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExtractResponse {
    /// Parser version.
    pub version: String,
    /// `success` | `partial_success` | `failure`, as `Run status`.
    pub run_status: String,
    /// The CLI exit code of this run: 0 every file `success`; 1 partial
    /// (some file `warning`/`error`, no rule applies, or the output was
    /// truncated by `max_output_chars`); 3 every file `error`.
    pub exit_code: i32,
    /// Every discovered file, in processing order.
    pub files: Vec<FileReport>,
    /// One entry per rule result of each file, in the order of the
    /// document's data blocks (unsplit; items are raw, not anchor-escaped).
    pub results: Vec<ResultReport>,
    /// File errors (`<path>: <error>`) and rule errors
    /// (`<path> [<rule>]: <error>`), as `## Diagnostics`.
    pub diagnostics: Vec<String>,
    /// Run-level notes: rules that never apply, "No rule applies in this
    /// run", selector types that do not fit a format, output truncation.
    pub notes: Vec<String>,
    /// Whether `max_output_chars` stopped the item list.
    pub truncated_output: bool,
}

/// One input file of an [`ExtractResponse`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FileReport {
    pub path: String,
    pub size: u64,
    pub format: String,
    /// `extension`, `content`, `content fallback`, `template`,
    /// `CLI override` or `auto detection` (file not decoded).
    pub format_source: String,
    /// Encoding used, e.g. `utf-8`, `utf-8 (BOM stripped)`, `utf-16le (BOM)`.
    pub encoding: String,
    /// `success` | `warning` | `error`.
    pub status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hash: Option<String>,
}

/// One rule result of one file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResultReport {
    pub source: String,
    /// Format the file was processed as.
    pub format: String,
    /// Rule id.
    pub rule: String,
    /// Output key (defaults to the rule id).
    pub key: String,
    /// `success` | `no matches found` | `rule not applicable` | `error`.
    pub status: String,
    pub items: Vec<String>,
    /// Text rules (`contains`, `starts_with`, `ends_with`, `regex`,
    /// `line`): the 1-based source line of each item, parallel to `items`:
    /// the matched line (the first line of a `line` range). Left out when
    /// the result has no items or no line numbers (other selector types, or
    /// `join` merged several items into one).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lines: Option<Vec<usize>>,
    /// With `lines`, when an item covers more than one source line (context
    /// lines, a `line` range): `[first, last]` source line of each item,
    /// parallel to `items`. Transforms may have changed the item text since.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spans: Option<Vec<[usize; 2]>>,
    /// Whether a limit, a lossy transform or the output cap removed data.
    pub truncated: bool,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

impl ResultReport {
    /// [`ResultReport::lines`] and [`ResultReport::spans`] of items with
    /// these source lines (`None` = unknown).
    fn locations(
        item_lines: &[Option<SourceLines>],
    ) -> (Option<Vec<usize>>, Option<Vec<[usize; 2]>>) {
        let known: Option<Vec<SourceLines>> = item_lines.iter().copied().collect();
        match known {
            Some(all) if !all.is_empty() => {
                let spans = all
                    .iter()
                    .any(|l| l.first != l.last)
                    .then(|| all.iter().map(|l| [l.first, l.last]).collect());
                (Some(all.iter().map(|l| l.line).collect()), spans)
            }
            _ => (None, None),
        }
    }

    /// Keeps the first `count` items and their line numbers.
    fn truncate_items(&mut self, count: usize) {
        self.items.truncate(count);
        if count == 0 {
            self.lines = None;
            self.spans = None;
        } else {
            if let Some(lines) = &mut self.lines {
                lines.truncate(count);
            }
            if let Some(spans) = &mut self.spans {
                spans.truncate(count);
            }
        }
    }

    /// The source location of item `index`: (matched line, [first, last]).
    fn location(&self, index: usize) -> Option<(usize, [usize; 2])> {
        let line = *self.lines.as_ref()?.get(index)?;
        let span = match &self.spans {
            Some(spans) => *spans.get(index)?,
            None => [line, line],
        };
        Some((line, span))
    }
}

/// Discovers the files of `input`; with `root`, the input must not reach
/// outside it and every file found must lie inside it.
fn discover_confined(
    input: &str,
    root: Option<&Path>,
    follow_symlinks: bool,
    limits: &ResourceLimits,
) -> Result<Vec<DiscoveredFile>, ApiError> {
    if let Some(root) = root {
        confine::check_pattern(input, root).map_err(ApiError::input)?;
    }
    let files = run::discover_inputs(input, None, follow_symlinks, limits)?;
    if let Some(root) = root
        && let Some(f) = files.iter().find(|f| !confine::is_inside(&f.path, root))
    {
        return Err(ApiError::input(confine::outside_message(
            &f.path.to_string_lossy(),
            root,
        )));
    }
    Ok(files)
}

/// Runs the rules of `req` over its input and returns the data.
///
/// Errors before any source is processed are [`ApiError`]s (see the module
/// docs); per-file problems are file statuses and `diagnostics` of an `Ok`
/// response.
pub fn extract(req: &ExtractRequest) -> Result<ExtractResponse, ApiError> {
    if req.input.trim().is_empty() {
        return Err(ApiError::input(
            "missing required 'input': a file, a directory or a glob pattern",
        ));
    }
    validate_request_limits(req)?;
    if let (Some(root), Some(path)) = (&req.confine_to, &req.template_path)
        && path.exists()
        && !confine::is_inside(path, root)
    {
        return Err(ApiError::input(confine::outside_message(
            &path.to_string_lossy(),
            root,
        )));
    }
    let template = compile_request(req)?;
    let limits = run::resource_limits(
        &template,
        req.max_file_size.as_deref(),
        req.max_memory.as_deref(),
        req.max_files,
        req.max_matches,
    );
    let files = discover_confined(
        &req.input,
        req.confine_to.as_deref(),
        req.follow_symlinks,
        &limits,
    )?;
    let collection = run::collect_sources(&files, &template, &limits, req.hash, false)?;
    let entries = materialize::result_entries(&collection.file_results, &template);
    let (run_status, exit_code) = run::run_outcome(
        collection.files_success,
        collection.files_warning,
        collection.files_error,
    );

    let mut notes = run::never_applying_rules(&template);
    notes.extend(collection.nothing_applies.clone());
    notes.extend(not_applicable_notes(&entries.entries, &template));
    let (skip_notes, unread_rules) = skipped_format_notes(&collection.file_results, &template);
    notes.extend(skip_notes);
    let (run_status, exit_code) = if unread_rules && exit_code == ExitCode::Success {
        ("partial_success", ExitCode::PartialSuccess)
    } else {
        (run_status, exit_code)
    };

    let mut response = ExtractResponse {
        version: crate::PARSER_VERSION.to_string(),
        run_status: run_status.to_string(),
        exit_code: exit_code.to_i32(),
        files: collection
            .file_results
            .iter()
            .map(|f| FileReport {
                path: display_path(&f.path.to_string_lossy()),
                size: f.size,
                format: f.format.name().to_string(),
                format_source: f.format_source.clone(),
                encoding: f.encoding.clone(),
                status: materialize::file_status_str(&f.status).to_string(),
                error: f.error.clone(),
                hash: f.hash.clone(),
            })
            .collect(),
        results: entries
            .entries
            .into_iter()
            .map(|e| {
                let (lines, spans) = ResultReport::locations(&e.item_lines);
                ResultReport {
                    source: display_path(&e.source),
                    format: e.format,
                    rule: e.rule_id,
                    key: e.key,
                    status: e.status.to_string(),
                    items: e.items,
                    lines,
                    spans,
                    truncated: e.truncated,
                    warnings: e.warnings,
                    errors: e.errors,
                }
            })
            .collect(),
        diagnostics: entries.errors,
        notes,
        truncated_output: false,
    };
    if !req.include_empty {
        fold_empty_results(&mut response, &template);
    }

    let cap = match req.max_output_chars {
        Some(cap) => Some((cap, "max_output_chars")),
        None => template.max_output_chars.map(|cap| {
            (
                usize::try_from(cap).unwrap_or(usize::MAX),
                "template output.max_output_chars",
            )
        }),
    };
    if let Some((cap, origin)) = cap {
        apply_output_cap(&mut response, cap, origin);
    }
    Ok(response)
}

/// Leaves out results that found nothing, notes them per rule, and judges
/// statuses the way a search over many files needs: a file is not partial
/// because one rule found nothing in it, but a rule that found nothing in
/// any file it read makes the run partial.
fn fold_empty_results(response: &mut ExtractResponse, template: &CompiledTemplate) {
    let is_empty = |r: &ResultReport| {
        r.status == "no matches found"
            && r.items.is_empty()
            && r.warnings.is_empty()
            && r.errors.is_empty()
    };
    let mut rule_found_nothing = false;
    for rule in &template.rules {
        let ours = response.results.iter().filter(|r| r.rule == rule.id);
        let (empty, with_data): (Vec<&ResultReport>, Vec<&ResultReport>) =
            ours.partition(|r| is_empty(r));
        if empty.is_empty() {
            continue;
        }
        let names: Vec<&str> = empty.iter().take(5).map(|r| r.source.as_str()).collect();
        let more = if empty.len() > names.len() {
            format!(", and {} more", empty.len() - names.len())
        } else {
            String::new()
        };
        if with_data.iter().any(|r| !r.items.is_empty()) {
            response.notes.push(format!(
                "rule '{}' found nothing in {} file(s): {}{}",
                rule.id,
                empty.len(),
                names.join(", "),
                more
            ));
        } else if with_data.is_empty() {
            rule_found_nothing = true;
            response.notes.push(format!(
                "warning: rule '{}' ({}) found nothing in the {} file(s) it read: {}{}",
                rule.id,
                rule.selector_type,
                empty.len(),
                names.join(", "),
                more
            ));
        }
    }
    let before = response.results.len();
    response.results.retain(|r| !is_empty(r));
    if response.results.len() == before && !rule_found_nothing {
        return;
    }

    // File statuses from what is left: problems that remain are warnings.
    let (mut success, mut warning, mut error) = (0usize, 0usize, 0usize);
    for file in &mut response.files {
        if file.status != "error" {
            let partial = response.results.iter().any(|r| {
                r.source == file.path
                    && (r.status != "success"
                        || r.truncated
                        || !r.warnings.is_empty()
                        || !r.errors.is_empty())
            });
            file.status = if partial { "warning" } else { "success" }.to_string();
        }
        match file.status.as_str() {
            "success" => success += 1,
            "warning" => warning += 1,
            _ => error += 1,
        }
    }
    let (status, code) = run::run_outcome(success, warning, error);
    let (status, code) = if code == ExitCode::Success
        && (rule_found_nothing
            || response
                .notes
                .iter()
                .any(|n| n.starts_with("warning: ") || n.starts_with("No rule applies")))
    {
        ("partial_success", ExitCode::PartialSuccess)
    } else {
        (status, code)
    };
    response.run_status = status.to_string();
    response.exit_code = code.to_i32();
}

/// Paths in API output use `/`, which every OS (including Windows) accepts
/// back as input.
fn display_path(path: &str) -> String {
    let path = path.replace('\\', "/");
    match path.strip_prefix("./") {
        Some(rest) if !rest.is_empty() => rest.to_string(),
        _ => path,
    }
}

/// Notes for rules that skip formats they cannot read: how many files of
/// each format they skipped, and a warning note for a rule that read no
/// file at all (`true` in the second value).
fn skipped_format_notes(
    files: &[crate::collect::FileResult],
    template: &CompiledTemplate,
) -> (Vec<String>, bool) {
    let parsed: Vec<&crate::collect::FileResult> =
        files.iter().filter(|f| f.error.is_none()).collect();
    let mut notes = Vec::new();
    let mut unread = false;
    for rule in template.rules.iter().filter(|r| r.skip_unreadable_formats) {
        let mut skipped: Vec<(&'static str, usize)> = Vec::new();
        let mut read = 0usize;
        for file in &parsed {
            if crate::collect::format_reads_selector(&file.format, &rule.selector_type) {
                read += 1;
            } else if let Some(entry) = skipped.iter_mut().find(|(f, _)| *f == file.format.name()) {
                entry.1 += 1;
            } else {
                skipped.push((file.format.name(), 1));
            }
        }
        if skipped.is_empty() {
            continue;
        }
        let counts: Vec<String> = skipped
            .iter()
            .map(|(f, n)| format!("{} {}", n, f))
            .collect();
        if read == 0 {
            unread = true;
            notes.push(format!(
                "warning: rule '{}' ({}) read no file: every input file has a format it cannot read ({}); use a selector for that format or --format",
                rule.id,
                rule.selector_type,
                counts.join(", ")
            ));
        } else {
            notes.push(format!(
                "rule '{}' ({}) skipped files it cannot read: {}",
                rule.id,
                rule.selector_type,
                counts.join(", ")
            ));
        }
    }
    (notes, unread)
}

/// One note per (rule, format) whose results are `rule not applicable`
/// because the selector type does not fit the format.
fn not_applicable_notes(
    entries: &[materialize::ResultEntry],
    template: &CompiledTemplate,
) -> Vec<String> {
    let mut seen: Vec<(&str, &str)> = Vec::new();
    let mut notes = Vec::new();
    for e in entries {
        if e.status != "rule not applicable" || seen.contains(&(&e.rule_id, &e.format)) {
            continue;
        }
        seen.push((&e.rule_id, &e.format));
        let Some(rule) = template.rules.iter().find(|r| r.id == e.rule_id) else {
            continue;
        };
        let Some(format) = Format::parse(&e.format).filter(|f| *f != Format::Auto) else {
            continue;
        };
        notes.push(format!(
            "rule '{}' ({}) does not apply to {} files: {} takes {}",
            e.rule_id,
            rule.selector_type,
            e.format,
            e.format,
            adapter_for(&format).selector_types().join(", ")
        ));
    }
    notes
}

/// Keeps whole items, in result order, while their characters fit in `cap`.
fn apply_output_cap(response: &mut ExtractResponse, cap: usize, origin: &str) {
    let mut used = 0usize;
    // (result position, kept items, items of that result)
    let mut stop: Option<(usize, usize, usize)> = None;
    let mut omitted = 0usize;
    let total = response.results.len();
    for (index, result) in response.results.iter_mut().enumerate() {
        let count = result.items.len();
        if count == 0 {
            continue;
        }
        let kept = if stop.is_some() {
            0
        } else {
            let mut kept = 0;
            for item in &result.items {
                let chars = item.chars().count();
                if used.saturating_add(chars) > cap {
                    break;
                }
                used += chars;
                kept += 1;
            }
            kept
        };
        if kept == count {
            continue;
        }
        if stop.is_none() {
            stop = Some((index, kept, count));
        }
        omitted += count - kept;
        result.truncate_items(kept);
        result.truncated = true;
        result.warnings.push(format!(
            "{} of {} item(s) returned: the output cap {} ({} characters) was reached",
            kept, count, origin, cap
        ));
    }
    let Some((index, kept, count)) = stop else {
        return;
    };
    let at = &response.results[index];
    response.truncated_output = true;
    response.notes.push(format!(
        "output truncated by {} {}: stopped in result {} of {} (source '{}', rule '{}') after {} of {} item(s); {} item(s) not returned in total",
        origin,
        cap,
        index + 1,
        total,
        at.source,
        at.rule,
        kept,
        count,
        omitted
    ));
    if response.exit_code == ExitCode::Success.to_i32() {
        response.exit_code = ExitCode::PartialSuccess.to_i32();
        response.run_status = "partial_success".to_string();
    }
}

/// Request values that must fail with exit 3 when malformed.
fn validate_request_limits(req: &ExtractRequest) -> Result<(), ApiError> {
    for (name, value) in [
        ("max_files", req.max_files),
        ("max_matches", req.max_matches),
    ] {
        if value == Some(0) {
            return Err(ApiError::input(format!("{} must be positive", name)));
        }
    }
    if req.max_output_chars == Some(0) {
        return Err(ApiError::input("max_output_chars must be positive"));
    }
    for (name, value) in [
        ("max_file_size", &req.max_file_size),
        ("max_memory", &req.max_memory),
    ] {
        if let Some(v) = value
            && crate::limits::parse_size(v).is_none()
        {
            return Err(ApiError::input(format!(
                "invalid {} value '{}': expected a number of bytes or a SIZE with suffix b/k/m/g (x1024, case-insensitive)",
                name, v
            )));
        }
    }
    Ok(())
}

/// The `format` override of a request: `None` for absent or `auto`.
fn format_override(req: &ExtractRequest) -> Result<Option<String>, ApiError> {
    let Some(value) = &req.format else {
        return Ok(None);
    };
    match Format::parse(value.trim()) {
        None => Err(ApiError::input(format!(
            "invalid format '{}': expected one of json|xml|html|markdown|yaml|toml|text|auto",
            value
        ))),
        Some(Format::Auto) => Ok(None),
        Some(format) => Ok(Some(format.name().to_string())),
    }
}

/// Compiles the rules of `req` (template file, inline template or rule
/// list) with its `format` / `encoding` overrides into the template the run
/// uses. This is the validation [`extract`] performs before reading input;
/// `template::effective_template_yaml` of the result is a reusable template.
pub fn compile_request(req: &ExtractRequest) -> Result<CompiledTemplate, ApiError> {
    let given: Vec<&str> = [
        ("template_path", req.template_path.is_some()),
        ("template", req.template.is_some()),
        ("rules", !req.rules.is_empty()),
    ]
    .into_iter()
    .filter_map(|(name, set)| set.then_some(name))
    .collect();
    if given.len() != 1 {
        return Err(ApiError::input(format!(
            "give exactly one of template_path, template or rules (got {})",
            if given.is_empty() {
                "none".to_string()
            } else {
                given.join(", ")
            }
        )));
    }
    let format = format_override(req)?;
    let encoding = match &req.encoding {
        Some(e) => Some(template::validate_encoding(e).map_err(|e| ApiError::input(e.message))?),
        None => None,
    };
    let options = shorthand_option_names(req);
    if !options.is_empty() && !req.rules.iter().any(is_shorthand) {
        return Err(ApiError::input(format!(
            "{} apply only to shorthand rules (TYPE:EXPR); set them in the rule object or template instead",
            options.join(", ")
        )));
    }

    if let Some(path) = &req.template_path {
        if !path.is_file() {
            return Err(ApiError::input(format!(
                "template file not found: '{}'",
                path.display()
            )));
        }
        return template::load_template_with_overrides(
            path,
            format.as_deref(),
            encoding.as_deref(),
        )
        .map_err(|e| ApiError::template(e.message));
    }
    if let Some(text) = &req.template {
        return template::compile_yaml_str(
            &inline_template_yaml(text),
            &PathBuf::from("<inline template>"),
            format,
            encoding,
        )
        .map_err(|e| ApiError::template(e.message));
    }

    let shared = shorthand_fields(req)?;
    let mut rules = Vec::with_capacity(req.rules.len());
    for (index, spec) in req.rules.iter().enumerate() {
        let default_id = format!("r{}", index + 1);
        let rule = match spec {
            RuleSpec::Object(rule) => with_default_id(rule.clone(), &default_id),
            RuleSpec::Shorthand(text) if !is_shorthand(spec) => {
                match serde_yaml::from_str::<Value>(text) {
                    Ok(Value::Mapping(rule)) => with_default_id(rule, &default_id),
                    Ok(other) => {
                        return Err(ApiError::template(format!(
                            "rule #{}: a rule object must be a mapping, got {}",
                            index + 1,
                            value_kind(&other)
                        )));
                    }
                    Err(e) => {
                        return Err(ApiError::template(format!(
                            "rule #{}: invalid rule object '{}': {}",
                            index + 1,
                            text,
                            e
                        )));
                    }
                }
            }
            RuleSpec::Shorthand(text) => {
                let select = shorthand_select(text)
                    .map_err(|m| ApiError::template(format!("rule '{}': {}", default_id, m)))?;
                let mut rule = Mapping::new();
                rule.insert("id".into(), default_id.into());
                rule.insert("select".into(), Value::Mapping(select));
                for (key, value) in &shared {
                    rule.insert((*key).into(), value.clone());
                }
                rule
            }
        };
        rules.push(Value::Mapping(rule));
    }
    let mut root = Mapping::new();
    root.insert(
        "version".into(),
        number(u64::from(template::TEMPLATE_VERSION)),
    );
    root.insert("name".into(), "rules".into());
    root.insert("rules".into(), Value::Sequence(rules));
    let text = serde_yaml::to_string(&Value::Mapping(root)).map_err(|e| {
        ApiError::new(
            ExitCode::InternalFatal,
            format!("internal error: cannot write the generated template: {}", e),
        )
    })?;
    let mut template =
        template::compile_yaml_str(&text, &PathBuf::from("<rules>"), format, encoding)
            .map_err(|e| ApiError::template(strip_generated_location(&e.message)))?;
    // A shorthand rule asks for its kind of data wherever it exists: files
    // whose format cannot hold it are skipped, not reported as warnings.
    for (rule, spec) in template.rules.iter_mut().zip(&req.rules) {
        rule.skip_unreadable_formats = is_shorthand(spec);
    }
    Ok(template)
}

fn number(n: u64) -> Value {
    Value::Number(n.into())
}

/// Whether the rule is a `TYPE:EXPR` string (not an object in flow form).
fn is_shorthand(spec: &RuleSpec) -> bool {
    matches!(spec, RuleSpec::Shorthand(text) if !text.trim_start().starts_with('{'))
}

fn with_default_id(rule: Mapping, default_id: &str) -> Mapping {
    if rule.contains_key("id") {
        return rule;
    }
    let mut out = Mapping::new();
    out.insert("id".into(), default_id.into());
    for (key, value) in rule {
        out.insert(key, value);
    }
    out
}

/// The request fields of the shorthand-level options that are set.
fn shorthand_option_names(req: &ExtractRequest) -> Vec<&'static str> {
    [
        ("context_before", req.context_before.is_some()),
        ("context_after", req.context_after.is_some()),
        ("extract", req.extract.is_some()),
        ("transforms", !req.transforms.is_empty()),
        ("max_items", req.max_items.is_some()),
        ("max_item_chars", req.max_item_chars.is_some()),
    ]
    .into_iter()
    .filter_map(|(name, set)| set.then_some(name))
    .collect()
}

/// The rule fields the shorthand-level options give every shorthand rule:
/// `context`, `extract`, `transforms`, `limits`.
fn shorthand_fields(req: &ExtractRequest) -> Result<Vec<(&'static str, Value)>, ApiError> {
    let mut fields = Vec::new();
    if req.context_before.is_some() || req.context_after.is_some() {
        let mut context = Mapping::new();
        let lines = |n: Option<usize>| number(n.unwrap_or(0) as u64);
        context.insert("before".into(), lines(req.context_before));
        context.insert("after".into(), lines(req.context_after));
        fields.push(("context", Value::Mapping(context)));
    }
    if let Some(extract) = &req.extract {
        fields.push(("extract", Value::String(extract.clone())));
    }
    if !req.transforms.is_empty() {
        let list = req
            .transforms
            .iter()
            .map(|text| parse_transform(text))
            .collect::<Result<Vec<_>, _>>()?;
        fields.push(("transforms", Value::Sequence(list)));
    }
    if req.max_items.is_some() || req.max_item_chars.is_some() {
        let mut limits = Mapping::new();
        if let Some(n) = req.max_items {
            limits.insert("max_items".into(), number(n));
        }
        if let Some(n) = req.max_item_chars {
            limits.insert("max_item_chars".into(), number(n));
        }
        fields.push(("limits", Value::Mapping(limits)));
    }
    Ok(fields)
}

/// One transform in DSL form: a YAML/JSON value such as `trim`,
/// `limit_items: 20` or `{replace: {from: a, to: b}}`.
fn parse_transform(text: &str) -> Result<Value, ApiError> {
    match serde_yaml::from_str::<Value>(text) {
        Ok(Value::Null) => Err(ApiError::template(format!(
            "empty transform '{}': expected a name (trim) or NAME: PARAM (limit_items: 20)",
            text
        ))),
        Ok(value) => Ok(value),
        Err(e) => Err(ApiError::template(format!(
            "invalid transform '{}': {} (expected a name such as trim, or NAME: PARAM such as limit_items: 20)",
            text, e
        ))),
    }
}

/// Inline template text as YAML: JSON is converted first, because YAML
/// does not accept every JSON text (tab indentation).
fn inline_template_yaml(text: &str) -> String {
    if text.trim_start().starts_with('{')
        && let Ok(value) = serde_json::from_str::<serde_json::Value>(text)
        && let Ok(yaml) = serde_yaml::to_string(&value)
    {
        return yaml;
    }
    text.to_string()
}

/// Drops the ` at line N column M` that points into the generated YAML.
fn strip_generated_location(message: &str) -> String {
    let location = regex::Regex::new(r" at line \d+ column \d+$").expect("valid regex");
    location.replace(message, "").into_owned()
}

/// The `select` mapping of a shorthand rule `TYPE:EXPR` (see [`RuleSpec`]),
/// e.g. `regex:a|b` -> `{type: regex, expression: "a|b"}`, `line:10-` ->
/// `{type: line, from: 10, to: u64::MAX}`. The selector itself is validated
/// when the template compiles.
pub fn shorthand_select(spec: &str) -> Result<Mapping, String> {
    let valid = SHORTHAND_TYPES.join(", ");
    let Some((kind, expr)) = spec.split_once(':') else {
        return Err(format!(
            "invalid rule '{}': expected TYPE:EXPR with TYPE one of {} (e.g. contains:ERROR)",
            spec, valid
        ));
    };
    let kind = kind.trim().to_ascii_lowercase();
    if !SHORTHAND_TYPES.contains(&kind.as_str()) {
        return Err(format!(
            "unknown rule type '{}' in '{}': valid types are {}",
            kind, spec, valid
        ));
    }
    let mut select = Mapping::new();
    select.insert("type".into(), kind.clone().into());
    match kind.as_str() {
        "line" => {
            let (from, to) = parse_line_range(expr).ok_or_else(|| {
                format!(
                    "invalid line range '{}' in '{}': expected N, N-M, N- or -M (1-based, inclusive)",
                    expr, spec
                )
            })?;
            select.insert("from".into(), number(from));
            select.insert("to".into(), number(to));
        }
        "markdown" => {
            let (node, pattern) = match expr.split_once(':') {
                Some((node, pattern)) => (node.trim(), Some(pattern)),
                None => (expr.trim(), None),
            };
            if node.is_empty() {
                return Err(format!(
                    "missing markdown node in '{}': expected markdown:NODE or markdown:NODE:PATTERN (NODE e.g. heading, paragraph, list_item, table_row, code_block, link)",
                    spec
                ));
            }
            select.insert("node".into(), node.into());
            if let Some(pattern) = pattern {
                if pattern.is_empty() {
                    return Err(format!("empty markdown pattern in '{}'", spec));
                }
                select.insert("pattern".into(), pattern.into());
            }
        }
        _ => {
            if expr.is_empty() {
                return Err(format!(
                    "empty expression in '{}': expected {}:EXPR",
                    spec, kind
                ));
            }
            select.insert("expression".into(), expr.into());
        }
    }
    Ok(select)
}

/// `N`, `N-M`, `N-` (to the end) or `-M` (from line 1); numbers >= 1.
fn parse_line_range(expr: &str) -> Option<(u64, u64)> {
    let number = |s: &str| -> Option<u64> {
        let s = s.trim();
        if s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        s.parse().ok().filter(|n| *n >= 1)
    };
    match expr.split_once('-') {
        None => number(expr).map(|n| (n, n)),
        Some((from, to)) => match (from.trim().is_empty(), to.trim().is_empty()) {
            (true, true) => None,
            (true, false) => Some((1, number(to)?)),
            (false, true) => Some((number(from)?, u64::MAX)),
            (false, false) => Some((number(from)?, number(to)?)),
        },
    }
}

/// Plain-text form of an [`ExtractResponse`] for an AI reading stdout.
///
/// ```text
/// any_txt_parser 1.0.4: partial_success, exit 1; files 2 (success 1, warning 0, error 1); results 2; items 3
/// diagnostic: logs/bad.json: invalid JSON: expected value at line 1 column 1
/// note: rule 'r2' (jsonpath) does not apply to text files: text takes regex, contains, starts_with, ends_with, line
/// == logs/app.log [text] rule r1 (success, 2 items)
/// - L12: 2026-10-01 ERROR disk full
/// - L40: 2026-10-01 ERROR timeout
///     L41- caused by: upstream
/// == logs/app.log [text] rule r2 (rule not applicable, 0 items)
/// == end: partial_success, exit 1
/// ```
///
/// - Line 1: version, run status, exit code, file counts by status,
///   number of results and items, `; output truncated` when
///   `max_output_chars` stopped the list.
/// - Then `diagnostic: ` lines (file and rule errors) and `note: ` lines.
/// - Per result, in document order, a header
///   `== <source> [<format>] rule <id>[ key <key>] (<status>, N item(s)[, truncated])`
///   (` key` only when the key differs from the id), its items, then
///   `error: ` and `warning: ` lines.
/// - An item is `- ` plus its first line; every further line of the same
///   item is indented by four spaces (`    `). Items are raw (no
///   escaping), so every line that is not `- `/`    ` is metadata.
/// - Items with source lines ([`ResultReport::lines`], text rules) carry
///   them in front of the raw text, grep style: `L<n>: ` on the matched
///   line (the first line of a `line` range), `L<n>- ` on its other lines
///   (context lines, the rest of the range), every line of the item
///   numbered. When a transform changed the number of lines of an item,
///   only its first line is marked, with the source lines it came from:
///   `L<n>: ` or `L<first>-L<last>: `.
/// - Metadata values are on one line: a line break in them is written as
///   `\n` (`\r` as `\r`).
/// - Last line: `== end: <run status>, exit <code>`.
pub fn render_text(resp: &ExtractResponse) -> String {
    let count = |status: &str| resp.files.iter().filter(|f| f.status == status).count();
    let items: usize = resp.results.iter().map(|r| r.items.len()).sum();
    let mut out = format!(
        "any_txt_parser {}: {}, exit {}; files {} (success {}, warning {}, error {}); results {}; items {}{}\n",
        resp.version,
        resp.run_status,
        resp.exit_code,
        resp.files.len(),
        count("success"),
        count("warning"),
        count("error"),
        resp.results.len(),
        items,
        if resp.truncated_output {
            "; output truncated"
        } else {
            ""
        }
    );
    for d in &resp.diagnostics {
        out.push_str(&format!("diagnostic: {}\n", one_line(d)));
    }
    for n in &resp.notes {
        out.push_str(&format!("note: {}\n", one_line(n)));
    }
    for r in &resp.results {
        out.push_str(&format!(
            "== {} [{}] rule {}",
            one_line(&r.source),
            r.format,
            one_line(&r.rule)
        ));
        if r.key != r.rule {
            out.push_str(&format!(" key {}", one_line(&r.key)));
        }
        out.push_str(&format!(
            " ({}, {}{})\n",
            r.status,
            plural(r.items.len(), "item"),
            if r.truncated { ", truncated" } else { "" }
        ));
        for (index, item) in r.items.iter().enumerate() {
            push_item(&mut out, item, r.location(index));
        }
        for e in &r.errors {
            out.push_str(&format!("error: {}\n", one_line(e)));
        }
        for w in &r.warnings {
            out.push_str(&format!("warning: {}\n", one_line(w)));
        }
    }
    out.push_str(&format!(
        "== end: {}, exit {}\n",
        resp.run_status, resp.exit_code
    ));
    out
}

/// `- first line`, then each further line indented by four spaces; with a
/// source location `(matched line, [first, last])`, the line numbers in
/// front of the text (see [`render_text`]).
fn push_item(out: &mut String, item: &str, at: Option<(usize, [usize; 2])>) {
    let lines: Vec<&str> = item.split('\n').collect();
    let numbers: Vec<String> = match at {
        None => Vec::new(),
        Some((matched, [first, last])) if (first..=last).count() == lines.len() => (first..=last)
            .map(|n| format!("L{}{} ", n, if n == matched { ':' } else { '-' }))
            .collect(),
        Some((matched, [first, last])) if first == last => vec![format!("L{}: ", matched)],
        Some((_, [first, last])) => vec![format!("L{}-L{}: ", first, last)],
    };
    for (index, line) in lines.iter().enumerate() {
        out.push_str(if index == 0 { "- " } else { "    " });
        if let Some(number) = numbers.get(index) {
            out.push_str(number);
        }
        out.push_str(line);
        out.push('\n');
    }
}

fn plural(n: usize, word: &str) -> String {
    if n == 1 {
        format!("1 {}", word)
    } else {
        format!("{} {}s", n, word)
    }
}

/// `\n` and `\r` written as escapes, so the value stays on its line.
fn one_line(text: &str) -> String {
    text.replace('\r', "\\r").replace('\n', "\\n")
}

/// JSON text of any API value (requests, responses, [`ApiError`]),
/// compact: one line, no insignificant whitespace — the form for MCP
/// results and machine readers. See [`to_json_pretty`] for people.
pub fn to_json<T: Serialize + ?Sized>(value: &T) -> String {
    serde_json::to_string(value).unwrap_or_else(|e| serialization_failure(&e))
}

/// Indented JSON text (two spaces) of any API value.
pub fn to_json_pretty<T: Serialize + ?Sized>(value: &T) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|e| serialization_failure(&e))
}

fn serialization_failure(e: &serde_json::Error) -> String {
    to_json(&ApiError::new(
        ExitCode::InternalFatal,
        format!("internal error: cannot serialize the result: {}", e),
    ))
}

/// Request of [`recon`].
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct ReconRequest {
    /// File, directory (recursive) or glob pattern.
    pub path: String,
    /// Lines of each file to show (default 20; 0 = none).
    pub sample_lines: usize,
    /// Decode with this encoding instead of detecting one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encoding: Option<String>,
    /// Files to describe (default 50); further files are only counted.
    pub max_files: usize,
    /// As [`ExtractRequest::confine_to`].
    #[serde(skip)]
    pub confine_to: Option<PathBuf>,
}

impl Default for ReconRequest {
    fn default() -> Self {
        ReconRequest {
            path: String::new(),
            sample_lines: 20,
            encoding: None,
            max_files: 50,
            confine_to: None,
        }
    }
}

/// Result of [`recon`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReconResponse {
    pub version: String,
    /// Files discovered (more than `files.len()` when `max_files` cut).
    pub files_found: usize,
    pub files: Vec<ReconFile>,
    pub notes: Vec<String>,
}

/// What [`recon`] found out about one file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReconFile {
    pub path: String,
    pub size: u64,
    /// Encoding the text was decoded with, as [`FileReport::encoding`]
    /// (`utf-8`, `utf-8 (BOM stripped)`, `utf-16le (BOM)`, ...); `None`
    /// when the file could not be decoded.
    pub encoding: Option<String>,
    /// Why the file cannot be read with the requested encoding (or the
    /// default `utf-8`). A file may have both an error and a sample decoded
    /// with a guessed encoding (see `notes`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// The format `format: auto` gives this file (`unknown` when it could
    /// not be decoded and its extension names no format).
    pub format: String,
    /// `extension`, `content`, `content fallback`, or why none was found.
    pub format_source: String,
    /// Selector types for that format.
    pub selectors: Vec<String>,
    /// Line count of the decoded text.
    pub lines: Option<usize>,
    /// The first `sample_lines` lines, each cut to 300 characters
    /// (`... [+N chars]` marks a cut); control characters other than TAB
    /// are shown as `\r`, `\0`, `\xNN`.
    pub sample: Vec<String>,
    /// Structure hints: top-level keys (`users[3]` a list of 3, `meta{2}`
    /// a mapping of 2), root element and namespaces, title and headings,
    /// or the parse error of the detected format.
    pub shape: Vec<String>,
    /// Encoding hints and other remarks.
    pub notes: Vec<String>,
}

/// Longest sample line shown, in characters.
const SAMPLE_LINE_CHARS: usize = 300;
/// Largest file recon reads.
const RECON_MAX_BYTES: u64 = 200 * 1024 * 1024;
/// Keys / children / headings listed in a shape hint.
const SHAPE_LIST: usize = 20;

/// Describes the files at `req.path` — encoding, detected format, line
/// count, a sample and a structure hint — so that rules can be written
/// without reading the files. Nothing is extracted; per-file problems are
/// reported in the file entry. Errors: missing path, bad encoding or
/// `max_files: 0` (exit 3), no file matched (exit 3).
pub fn recon(req: &ReconRequest) -> Result<ReconResponse, ApiError> {
    if req.path.trim().is_empty() {
        return Err(ApiError::input(
            "missing required 'path': a file, a directory or a glob pattern",
        ));
    }
    if req.max_files == 0 {
        return Err(ApiError::input("max_files must be positive"));
    }
    let encoding = match &req.encoding {
        Some(e) => Some(template::validate_encoding(e).map_err(|e| ApiError::input(e.message))?),
        None => None,
    };
    let limits = ResourceLimits {
        max_input_files: None,
        max_file_size_bytes: None,
        ..ResourceLimits::default()
    };
    let files = discover_confined(&req.path, req.confine_to.as_deref(), false, &limits)?;
    let mut notes = Vec::new();
    if files.len() > req.max_files {
        notes.push(format!(
            "{} files found, the first {} are described (raise max_files for more)",
            files.len(),
            req.max_files
        ));
    }
    Ok(ReconResponse {
        version: crate::PARSER_VERSION.to_string(),
        files_found: files.len(),
        files: files
            .iter()
            .take(req.max_files)
            .map(|f| recon_file(f, encoding.as_deref(), req.sample_lines))
            .collect(),
        notes,
    })
}

fn recon_file(f: &DiscoveredFile, encoding: Option<&str>, sample_lines: usize) -> ReconFile {
    let mut file = ReconFile {
        path: f.path.to_string_lossy().into_owned(),
        size: f.size,
        encoding: None,
        error: None,
        format: String::new(),
        format_source: String::new(),
        selectors: Vec::new(),
        lines: None,
        sample: Vec::new(),
        shape: Vec::new(),
        notes: Vec::new(),
    };
    let decoded = if f.size > RECON_MAX_BYTES {
        file.error = Some(format!(
            "file is {} bytes, recon reads at most {} bytes",
            f.size, RECON_MAX_BYTES
        ));
        None
    } else {
        match std::fs::read(&f.path) {
            Ok(bytes) => decode_for_recon(&bytes, encoding, &mut file),
            Err(e) => {
                file.error = Some(format!("cannot read '{}': {}", f.path.display(), e));
                None
            }
        }
    };
    let Some(decoded) = decoded else {
        match discovery::extension_hint(&f.path) {
            Some(format) => {
                file.format = format.name().to_string();
                file.format_source = "extension".to_string();
                file.selectors = selector_types(&format);
            }
            None => {
                file.format = "unknown".to_string();
                file.format_source = "not detected: the file could not be decoded".to_string();
            }
        }
        return file;
    };
    let text = decoded.text;
    file.encoding = Some(decoded.encoding);
    let (format, source) = discovery::detect_format(&f.path, &text);
    file.format = format.name().to_string();
    file.format_source = source;
    file.selectors = selector_types(&format);
    file.lines = Some(text.lines().count());
    file.sample = text.lines().take(sample_lines).map(sample_line).collect();
    file.shape = shape_hint(&format, &text);
    let nul = text.chars().filter(|c| *c == '\0').count();
    if nul > 0 && nul * 10 >= text.chars().count() {
        file.notes.push(
            "many NUL characters: this may be UTF-16 without a byte order mark (try encoding \"utf-16le\") or a binary file"
                .to_string(),
        );
    }
    file
}

/// Decodes with the requested encoding, or detects one: a UTF-16 byte
/// order mark selects UTF-16, otherwise UTF-8 (a UTF-8 BOM is stripped);
/// text that is not UTF-8 is shown as windows-1251 when it decodes as such.
fn decode_for_recon(
    bytes: &[u8],
    requested: Option<&str>,
    file: &mut ReconFile,
) -> Option<discovery::Decoded> {
    if let Some(encoding) = requested {
        return match discovery::decode(bytes, encoding) {
            Ok(decoded) => Some(decoded),
            Err(e) => {
                file.error = Some(e);
                None
            }
        };
    }
    let bom = if bytes.starts_with(&[0xFF, 0xFE]) {
        Some("utf-16le")
    } else if bytes.starts_with(&[0xFE, 0xFF]) {
        Some("utf-16be")
    } else {
        None
    };
    if let Some(encoding) = bom {
        return match discovery::decode(bytes, encoding) {
            Ok(decoded) => {
                file.notes.push(format!(
                    "{} byte order mark: pass encoding \"{}\" to extract (the default utf-8 cannot read this file)",
                    encoding.to_ascii_uppercase(),
                    encoding
                ));
                Some(decoded)
            }
            Err(e) => {
                file.error = Some(e);
                None
            }
        };
    }
    match discovery::decode(bytes, "utf-8") {
        Ok(decoded) => Some(decoded),
        Err(e) => {
            file.error = Some(e);
            match discovery::decode(bytes, "windows-1251") {
                Ok(decoded) => {
                    file.notes.push(
                        "not valid UTF-8: shown decoded as windows-1251; if the sample reads correctly, pass encoding \"windows-1251\" to extract"
                            .to_string(),
                    );
                    Some(decoded)
                }
                Err(_) => None,
            }
        }
    }
}

fn selector_types(format: &Format) -> Vec<String> {
    if *format == Format::Auto {
        return Vec::new();
    }
    adapter_for(format)
        .selector_types()
        .iter()
        .map(|s| s.to_string())
        .collect()
}

fn sample_line(line: &str) -> String {
    let mut out = String::new();
    let mut chars = line.chars();
    for c in chars.by_ref().take(SAMPLE_LINE_CHARS) {
        match c {
            '\r' => out.push_str("\\r"),
            '\0' => out.push_str("\\0"),
            '\t' => out.push('\t'),
            c if c.is_control() && (c as u32) < 0x100 => {
                out.push_str(&format!("\\x{:02X}", c as u32));
            }
            c => out.push(c),
        }
    }
    let rest = chars.count();
    if rest > 0 {
        out.push_str(&format!("... [+{} chars]", rest));
    }
    out
}

fn shape_hint(format: &Format, text: &str) -> Vec<String> {
    match format {
        Format::Json | Format::Yaml | Format::Toml => {
            let adapter = adapter_for(format);
            let noun = match format {
                Format::Json => "object",
                Format::Yaml => "mapping",
                _ => "table",
            };
            match adapter.parse(text) {
                Ok(SourceTree::Json(value)) => vec![describe_value(&value, noun)],
                Ok(SourceTree::JsonDocuments(documents)) => {
                    let mut out = vec![format!("{} YAML documents", documents.len())];
                    for (i, document) in documents.iter().take(5).enumerate() {
                        out.push(format!(
                            "document {}: {}",
                            i + 1,
                            describe_value(document, noun)
                        ));
                    }
                    if documents.len() > 5 {
                        out.push(format!("... and {} more document(s)", documents.len() - 5));
                    }
                    out
                }
                Ok(_) => Vec::new(),
                Err(e) => vec![one_line(&e)],
            }
        }
        Format::Xml => xml_shape(text),
        Format::Html => html_shape(text),
        Format::Markdown => markdown_shape(text),
        Format::Text | Format::Auto => Vec::new(),
    }
}

/// `object with 3 key(s): name, users[2], meta{1}` / `array of 2 item(s);
/// item [0]: object with 2 key(s): id, name`.
fn describe_value(value: &serde_json::Value, noun: &str) -> String {
    use serde_json::Value as J;
    match value {
        J::Object(map) => format!(
            "{} with {} key(s): {}",
            noun,
            map.len(),
            limited_list(
                map.iter()
                    .map(|(k, v)| format!("{}{}", one_line(k), size_marker(v)))
            )
        ),
        J::Array(items) => {
            let mut out = format!("list of {} item(s)", items.len());
            if let Some(first) = items.first() {
                out.push_str("; item [0]: ");
                out.push_str(&match first {
                    J::Object(map) => format!(
                        "{} with {} key(s): {}",
                        noun,
                        map.len(),
                        limited_list(map.iter().map(|(k, v)| format!(
                            "{}{}",
                            one_line(k),
                            size_marker(v)
                        )))
                    ),
                    J::Array(inner) => format!("list of {} item(s)", inner.len()),
                    other => scalar_kind(other).to_string(),
                });
            }
            out
        }
        other => format!("a single {}", scalar_kind(other)),
    }
}

fn size_marker(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Object(map) => format!("{{{}}}", map.len()),
        serde_json::Value::Array(items) => format!("[{}]", items.len()),
        _ => String::new(),
    }
}

fn scalar_kind(value: &serde_json::Value) -> &'static str {
    match value {
        serde_json::Value::String(_) => "string",
        serde_json::Value::Number(_) => "number",
        serde_json::Value::Bool(_) => "boolean",
        serde_json::Value::Null => "null",
        serde_json::Value::Array(_) => "list",
        serde_json::Value::Object(_) => "mapping",
    }
}

/// The first [`SHAPE_LIST`] entries, then `... (+N more)`.
fn limited_list(entries: impl Iterator<Item = String>) -> String {
    let all: Vec<String> = entries.collect();
    let mut out = all
        .iter()
        .take(SHAPE_LIST)
        .cloned()
        .collect::<Vec<_>>()
        .join(", ");
    if all.len() > SHAPE_LIST {
        out.push_str(&format!(", ... (+{} more)", all.len() - SHAPE_LIST));
    }
    out
}

fn xml_shape(text: &str) -> Vec<String> {
    let tree = match XmlTree::parse(text) {
        Ok(tree) => tree,
        Err(e) => return vec![one_line(&e)],
    };
    let root = tree.doc.root_element();
    let mut out = Vec::new();
    let mut line = format!("root element <{}>", qualified_name(root));
    if let Some(ns) = root.tag_name().namespace() {
        line.push_str(&format!(" in namespace {}", ns));
    }
    out.push(line);
    let declared: Vec<String> = root
        .namespaces()
        .filter(|ns| ns.name() != Some("xml"))
        .map(|ns| match ns.name() {
            Some(prefix) => format!("xmlns:{}={}", prefix, ns.uri()),
            None => format!("xmlns={}", ns.uri()),
        })
        .collect();
    if !declared.is_empty() {
        out.push(format!("namespaces: {}", declared.join(", ")));
    }
    let attributes: Vec<String> = root
        .attributes()
        .map(
            |a| match a.namespace().and_then(|uri| root.lookup_prefix(uri)) {
                Some(prefix) if !prefix.is_empty() => format!("@{}:{}", prefix, a.name()),
                _ => format!("@{}", a.name()),
            },
        )
        .collect();
    if !attributes.is_empty() {
        out.push(format!(
            "root attributes: {}",
            limited_list(attributes.into_iter())
        ));
    }
    let mut children: Vec<(String, usize)> = Vec::new();
    for child in root.children().filter(|n| n.is_element()) {
        let name = qualified_name(child);
        match children.iter_mut().find(|(n, _)| *n == name) {
            Some((_, count)) => *count += 1,
            None => children.push((name, 1)),
        }
    }
    if !children.is_empty() {
        out.push(format!(
            "child elements: {}",
            limited_list(children.into_iter().map(|(name, count)| {
                if count == 1 {
                    name
                } else {
                    format!("{} x{}", name, count)
                }
            }))
        ));
    }
    out
}

fn qualified_name(node: roxmltree::Node<'_, '_>) -> String {
    let name = node.tag_name();
    match name.namespace().and_then(|uri| node.lookup_prefix(uri)) {
        Some(prefix) if !prefix.is_empty() => format!("{}:{}", prefix, name.name()),
        _ => name.name().to_string(),
    }
}

fn html_shape(text: &str) -> Vec<String> {
    let doc = scraper::Html::parse_document(text);
    let select = |selector: &str| scraper::Selector::parse(selector).expect("valid selector");
    let squash = |s: String| s.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut out = Vec::new();
    if let Some(title) = doc.select(&select("title")).next() {
        out.push(format!("title: {}", squash(title.text().collect())));
    }
    let headings: Vec<_> = doc.select(&select("h1, h2, h3")).collect();
    for heading in headings.iter().take(SHAPE_LIST / 2) {
        out.push(format!(
            "{}: {}",
            heading.value().name(),
            squash(heading.text().collect())
        ));
    }
    if headings.len() > SHAPE_LIST / 2 {
        out.push(format!(
            "... and {} more h1-h3 heading(s)",
            headings.len() - SHAPE_LIST / 2
        ));
    }
    let counts: Vec<String> = [
        "table", "form", "ul", "ol", "li", "a", "img", "pre", "script",
    ]
    .iter()
    .filter_map(|tag| {
        let n = doc.select(&select(tag)).count();
        (n > 0).then(|| format!("{} {}", n, tag))
    })
    .collect();
    if !counts.is_empty() {
        out.push(format!("elements: {}", counts.join(", ")));
    }
    out
}

fn markdown_shape(text: &str) -> Vec<String> {
    use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
    let options =
        Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TASKLISTS;
    let mut headings: Vec<(usize, String)> = Vec::new();
    let mut current: Option<(usize, String)> = None;
    let (mut code, mut tables, mut items, mut links, mut images) = (0, 0, 0, 0, 0);
    for event in Parser::new_ext(text, options) {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                current = Some((level as usize, String::new()))
            }
            Event::End(TagEnd::Heading(_)) => headings.extend(current.take()),
            Event::Text(t) | Event::Code(t) => {
                if let Some((_, heading)) = &mut current {
                    heading.push_str(&t);
                }
            }
            Event::Start(Tag::CodeBlock(_)) => code += 1,
            Event::Start(Tag::Table(_)) => tables += 1,
            Event::Start(Tag::Item) => items += 1,
            Event::Start(Tag::Link { .. }) => links += 1,
            Event::Start(Tag::Image { .. }) => images += 1,
            _ => {}
        }
    }
    let mut out: Vec<String> = headings
        .iter()
        .take(SHAPE_LIST)
        .map(|(level, heading)| format!("{} {}", "#".repeat(*level), one_line(heading)))
        .collect();
    if headings.len() > SHAPE_LIST {
        out.push(format!(
            "... and {} more heading(s)",
            headings.len() - SHAPE_LIST
        ));
    }
    let counts: Vec<String> = [
        (headings.len(), "heading"),
        (code, "code_block"),
        (tables, "table"),
        (items, "list_item"),
        (links, "link"),
        (images, "image"),
    ]
    .iter()
    .filter(|(n, _)| *n > 0)
    .map(|(n, name)| format!("{} {}", n, name))
    .collect();
    if !counts.is_empty() {
        out.push(format!("nodes: {}", counts.join(", ")));
    }
    out
}

/// Plain-text form of a [`ReconResponse`].
///
/// ```text
/// any_txt_parser 1.0.4 recon: 1 file(s)
/// == data/users.json (64 bytes)
/// encoding: utf-8
/// format: json (extension)
/// selectors: jsonpath, path
/// lines: 4
/// shape: object with 2 key(s): users[2], total
/// sample: lines 1-4 of 4
///    1| {"users": [
///    2|   {"id": 1, "name": "Ann"},
///    3|   {"id": 2, "name": "Bob"}],
///    4|  "total": 2}
/// == end: 1 file(s)
/// ```
///
/// - Line 1: version and file count (`N of M file(s)` when `max_files`
///   cut the list), then `note: ` lines.
/// - Per file a `== <path> (<size> bytes)` header and `key: value` lines:
///   `error`, `encoding`, `format` (with its source), `selectors`, `lines`,
///   `shape` (one line per hint), `note`, then `sample: lines A-B of N`
///   with each sample line as `<number>| <text>`.
/// - Last line: `== end: N file(s)`.
pub fn render_recon_text(resp: &ReconResponse) -> String {
    let shown = resp.files.len();
    let mut out = if shown < resp.files_found {
        format!(
            "any_txt_parser {} recon: {} of {} file(s)\n",
            resp.version, shown, resp.files_found
        )
    } else {
        format!("any_txt_parser {} recon: {} file(s)\n", resp.version, shown)
    };
    for note in &resp.notes {
        out.push_str(&format!("note: {}\n", one_line(note)));
    }
    for f in &resp.files {
        out.push_str(&format!("== {} ({} bytes)\n", one_line(&f.path), f.size));
        if let Some(error) = &f.error {
            out.push_str(&format!("error: {}\n", one_line(error)));
        }
        if let Some(encoding) = &f.encoding {
            out.push_str(&format!("encoding: {}\n", encoding));
        }
        out.push_str(&format!("format: {} ({})\n", f.format, f.format_source));
        if !f.selectors.is_empty() {
            out.push_str(&format!("selectors: {}\n", f.selectors.join(", ")));
        }
        if let Some(lines) = f.lines {
            out.push_str(&format!("lines: {}\n", lines));
        }
        for shape in &f.shape {
            out.push_str(&format!("shape: {}\n", one_line(shape)));
        }
        for note in &f.notes {
            out.push_str(&format!("note: {}\n", one_line(note)));
        }
        if !f.sample.is_empty() {
            let last = f.sample.len();
            out.push_str(&format!(
                "sample: lines 1-{} of {}\n",
                last,
                f.lines.unwrap_or(last)
            ));
            let width = last.to_string().len().max(4);
            for (i, line) in f.sample.iter().enumerate() {
                if line.is_empty() {
                    out.push_str(&format!("{:>width$}|\n", i + 1));
                } else {
                    out.push_str(&format!("{:>width$}| {}\n", i + 1, line));
                }
            }
        }
    }
    out.push_str(&format!("== end: {} file(s)\n", shown));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_ranges() {
        assert_eq!(parse_line_range("5"), Some((5, 5)));
        assert_eq!(parse_line_range(" 10 - 20 "), Some((10, 20)));
        assert_eq!(parse_line_range("10-"), Some((10, u64::MAX)));
        assert_eq!(parse_line_range("-3"), Some((1, 3)));
        for bad in ["", "-", "0", "a", "1-b", "1-2-3", "+1", "0-4"] {
            assert_eq!(parse_line_range(bad), None, "{bad:?}");
        }
    }

    #[test]
    fn generated_locations_are_dropped() {
        assert_eq!(
            strip_generated_location(
                "YAML syntax error: rules[0]: unknown field `x` at line 3 column 5"
            ),
            "YAML syntax error: rules[0]: unknown field `x`"
        );
        assert_eq!(strip_generated_location("rule 'r1': bad"), "rule 'r1': bad");
    }

    #[test]
    fn sample_lines_are_cut_and_escaped() {
        assert_eq!(sample_line("a\tb\r\0\u{1}"), "a\tb\\r\\0\\x01");
        let long = "x".repeat(SAMPLE_LINE_CHARS + 7);
        assert_eq!(
            sample_line(&long),
            format!("{}... [+7 chars]", "x".repeat(SAMPLE_LINE_CHARS))
        );
    }
}
