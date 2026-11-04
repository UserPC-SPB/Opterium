use crate::jsonpath::JsonPathExpr;
use crate::xpath::XPathExpr;
use serde::Deserialize;
use std::fmt;
use std::path::PathBuf;

pub const TEMPLATE_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Format {
    Json,
    Xml,
    Html,
    Markdown,
    Yaml,
    Toml,
    Text,
    Auto,
}

impl Format {
    pub fn parse(s: &str) -> Option<Format> {
        match s.to_ascii_lowercase().as_str() {
            "json" => Some(Format::Json),
            "xml" => Some(Format::Xml),
            "html" | "htm" => Some(Format::Html),
            "markdown" | "md" => Some(Format::Markdown),
            "yaml" | "yml" => Some(Format::Yaml),
            "toml" => Some(Format::Toml),
            "text" | "txt" | "log" | "plain" => Some(Format::Text),
            "auto" => Some(Format::Auto),
            _ => None,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Format::Json => "json",
            Format::Xml => "xml",
            Format::Html => "html",
            Format::Markdown => "markdown",
            Format::Yaml => "yaml",
            Format::Toml => "toml",
            Format::Text => "text",
            Format::Auto => "auto",
        }
    }

    /// Format named by the file extension. A name without an extension
    /// (`json`, `.yml`) and the `auto` pseudo-format name nothing: such
    /// files are sniffed by content.
    pub fn from_extension(path: &str) -> Option<Format> {
        let ext = std::path::Path::new(path).extension()?.to_str()?;
        match Format::parse(ext)? {
            Format::Auto => None,
            format => Some(format),
        }
    }
}

impl fmt::Display for Format {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MarkdownNodeType {
    Heading,
    Paragraph,
    List,
    ListItem,
    Table,
    TableRow,
    TableCell,
    CodeBlock,
    Blockquote,
    Link,
    Image,
}

impl MarkdownNodeType {
    pub fn parse(s: &str) -> Option<MarkdownNodeType> {
        match s
            .to_ascii_lowercase()
            .replace('-', "")
            .replace('_', "")
            .replace(' ', "")
            .as_str()
        {
            "heading" | "h" => Some(MarkdownNodeType::Heading),
            "paragraph" | "p" => Some(MarkdownNodeType::Paragraph),
            "list" => Some(MarkdownNodeType::List),
            "listitem" | "item" | "li" => Some(MarkdownNodeType::ListItem),
            "table" => Some(MarkdownNodeType::Table),
            "tablerow" | "row" | "tr" => Some(MarkdownNodeType::TableRow),
            "tablecell" | "cell" | "td" => Some(MarkdownNodeType::TableCell),
            "codeblock" | "code" => Some(MarkdownNodeType::CodeBlock),
            "blockquote" | "quote" => Some(MarkdownNodeType::Blockquote),
            "link" => Some(MarkdownNodeType::Link),
            "image" | "img" => Some(MarkdownNodeType::Image),
            _ => None,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            MarkdownNodeType::Heading => "heading",
            MarkdownNodeType::Paragraph => "paragraph",
            MarkdownNodeType::List => "list",
            MarkdownNodeType::ListItem => "list_item",
            MarkdownNodeType::Table => "table",
            MarkdownNodeType::TableRow => "table_row",
            MarkdownNodeType::TableCell => "table_cell",
            MarkdownNodeType::CodeBlock => "code_block",
            MarkdownNodeType::Blockquote => "blockquote",
            MarkdownNodeType::Link => "link",
            MarkdownNodeType::Image => "image",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExtractMode {
    Text,
    Attr(String),
    InnerHtml,
    OuterHtml,
}

#[derive(Debug, Clone)]
pub enum ItemTransform {
    Trim,
    NormalizeWhitespace,
    Lowercase,
    Uppercase,
    Unescape,
    TruncateChars(usize),
    TruncateLines(usize),
    Replace {
        from: String,
        to: String,
    },
    RegexReplace {
        pattern: regex::Regex,
        replacement: String,
    },
}

#[derive(Debug, Clone)]
pub enum ResultTransform {
    DropEmpty,
    Unique,
    SortAsc,
    SortDesc,
    LimitItems(usize),
    Join(String),
}

#[derive(Debug, Clone)]
pub enum TransformStep {
    Item(usize),
    Result(usize),
}

#[derive(Debug, Clone)]
pub enum SelectorCompiled {
    JsonPath(JsonPathExpr),
    XPath(XPathExpr),
    Css(String),
    Markdown {
        node: MarkdownNodeType,
        pattern: Option<regex::Regex>,
    },
    Path(Vec<PathSegment>),
    Regex(regex::Regex),
    Contains(String),
    StartsWith(String),
    EndsWith(String),
    LineRange {
        from: u64,
        to: u64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathSegment {
    Key(String),
    Index(usize),
}

#[derive(Debug, Clone)]
pub struct Context {
    pub before: usize,
    pub after: usize,
}

#[derive(Debug, Clone)]
pub struct CompiledRule {
    pub id: String,
    pub when_format: Option<Format>,
    pub selector: SelectorCompiled,
    pub selector_type: String,
    pub selector_expression: String,
    pub context: Context,
    pub output_key: String,
    pub extract: ExtractMode,
    pub transforms: Vec<ItemTransform>,
    pub result_transforms: Vec<ResultTransform>,
    pub transform_order: Vec<TransformStep>,
    pub max_items: Option<u64>,
    pub max_item_chars: Option<u64>,
    /// Run only on files whose format reads this selector type; other files
    /// are skipped like a `when.format` mismatch (no block, no warning).
    /// Set for command-line shorthand rules (`api`), never by templates.
    pub skip_unreadable_formats: bool,
}

#[derive(Debug, Clone)]
pub struct BlockConfig {
    pub target_chars: u64,
    pub max_chars: u64,
}

#[derive(Debug, Clone)]
pub struct CompiledTemplate {
    pub version: u32,
    pub name: String,
    pub source_path: PathBuf,
    pub format: Option<Format>,
    pub format_source: String,
    pub encoding: String,
    pub encoding_source: String,
    pub encoding_override: bool,
    pub rules: Vec<CompiledRule>,
    pub block: BlockConfig,
    pub max_output_chars: Option<u64>,
    pub stable_deduplicate: bool,
    pub sort: Option<String>,
    pub split_blocks: bool,
}

#[derive(Debug)]
pub struct TemplateError {
    pub message: String,
}

impl TemplateError {
    fn new(message: impl Into<String>) -> TemplateError {
        TemplateError {
            message: message.into(),
        }
    }
}

impl fmt::Display for TemplateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "template error: {}", self.message)
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawTemplate {
    version: Option<u32>,
    name: Option<String>,
    input: Option<RawInput>,
    output: Option<RawOutput>,
    document: Option<RawDocument>,
    rules: Option<Vec<RawRule>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawInput {
    format: Option<String>,
    encoding: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawOutput {
    max_output_chars: Option<u64>,
    block: Option<RawBlock>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawBlock {
    target_chars: Option<u64>,
    max_chars: Option<u64>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawDocument {
    sort: Option<String>,
    stable_deduplicate: Option<bool>,
    limit_total: Option<u64>,
    split_blocks: Option<bool>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawRule {
    id: Option<String>,
    when: Option<RawWhen>,
    select: serde_yaml::Value,
    context: Option<RawContext>,
    output: Option<RawRuleOutput>,
    extract: Option<String>,
    transforms: Option<Vec<serde_yaml::Value>>,
    limits: Option<RawLimits>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawWhen {
    format: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawContext {
    before: Option<usize>,
    after: Option<usize>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawRuleOutput {
    key: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawLimits {
    max_items: Option<u64>,
    max_item_chars: Option<u64>,
}

pub fn load_template(path: &PathBuf) -> Result<CompiledTemplate, TemplateError> {
    load_template_with_overrides(path, None, None)
}

/// Load and compile a template file, applying optional CLI overrides for
/// `input.format` / `input.encoding`. Precedence (spec §67):
/// CLI override > template value > auto/default.
///
/// The CLI values are validated here as well; the CLI entry point validates
/// them up front so that bad flag values map to exit code 3
/// (InvalidInputOrCli) instead of 2 (InvalidTemplate).
pub fn load_template_with_overrides(
    path: &PathBuf,
    cli_format: Option<&str>,
    cli_encoding: Option<&str>,
) -> Result<CompiledTemplate, TemplateError> {
    let text = std::fs::read_to_string(path).map_err(|e| {
        TemplateError::new(format!("cannot read template '{}': {}", path.display(), e))
    })?;
    compile_yaml_str(
        &text,
        path,
        cli_format.map(str::to_string),
        cli_encoding.map(str::to_string),
    )
}

pub fn compile_yaml_str(
    text: &str,
    path: &PathBuf,
    cli_format: Option<String>,
    cli_encoding: Option<String>,
) -> Result<CompiledTemplate, TemplateError> {
    let raw: RawTemplate = serde_yaml::from_str(text)
        .map_err(|e| TemplateError::new(format!("YAML syntax error: {}", e)))?;
    compile_raw(raw, path, cli_format, cli_encoding)
}

pub fn compile_raw(
    raw: RawTemplate,
    path: &PathBuf,
    cli_format: Option<String>,
    cli_encoding: Option<String>,
) -> Result<CompiledTemplate, TemplateError> {
    let version = raw.version.ok_or_else(|| {
        TemplateError::new(
            "missing required field 'version'. Template must declare its DSL version.",
        )
    })?;
    if version != TEMPLATE_VERSION {
        return Err(TemplateError::new(format!(
            "unsupported template version {}: this parser supports version {}",
            version, TEMPLATE_VERSION
        )));
    }

    let name = raw
        .name
        .clone()
        .filter(|n| !n.trim().is_empty())
        .ok_or_else(|| TemplateError::new("missing required field 'name'"))?;

    let raw_format = raw.input.as_ref().and_then(|i| i.format.clone());
    let raw_encoding = raw.input.as_ref().and_then(|i| i.encoding.clone());

    let (format, format_source) = if let Some(cf) = cli_format {
        let f = Format::parse(&cf)
            .ok_or_else(|| TemplateError::new(format!("unknown CLI format '{}'", cf)))?;
        if f == Format::Auto {
            return Err(TemplateError::new("CLI --format cannot be 'auto'"));
        }
        (Some(f), "CLI override".to_string())
    } else if let Some(tf) = raw_format {
        let f = Format::parse(&tf)
            .ok_or_else(|| TemplateError::new(format!("unknown input format '{}'", tf)))?;
        if f == Format::Auto {
            (None, "auto detection".to_string())
        } else {
            (Some(f), "template".to_string())
        }
    } else {
        (None, "auto detection".to_string())
    };

    let (encoding, encoding_source, encoding_override) = if let Some(ce) = cli_encoding {
        // CLI override wins over the template value (spec §67).
        (validate_encoding(&ce)?, "CLI override".to_string(), true)
    } else if let Some(te) = raw_encoding {
        (validate_encoding(&te)?, "template".to_string(), false)
    } else {
        ("utf-8".to_string(), "default".to_string(), false)
    };

    let mut block = BlockConfig {
        target_chars: 28_000,
        max_chars: 32_000,
    };
    if let Some(o) = &raw.output {
        if let Some(b) = &o.block {
            if let Some(t) = b.target_chars {
                block.target_chars = t;
            }
            if let Some(m) = b.max_chars {
                block.max_chars = m;
            }
        }
    }
    if block.target_chars == 0 || block.max_chars == 0 {
        return Err(TemplateError::new("block sizes must be positive"));
    }
    if block.target_chars > block.max_chars {
        return Err(TemplateError::new(
            "output.block.target_chars must not exceed output.block.max_chars",
        ));
    }

    let max_output_chars = raw
        .output
        .as_ref()
        .and_then(|o| o.max_output_chars)
        .or_else(|| raw.document.as_ref().and_then(|d| d.limit_total));

    let stable_deduplicate = raw
        .document
        .as_ref()
        .and_then(|d| d.stable_deduplicate)
        .unwrap_or(false);

    let sort = match raw.document.as_ref().and_then(|d| d.sort.clone()) {
        Some(s) => {
            if s != "none" && s != "asc" && s != "desc" {
                return Err(TemplateError::new(format!(
                    "document.sort must be one of: none, asc, desc (got '{}')",
                    s
                )));
            }
            Some(s)
        }
        None => None,
    };

    let split_blocks = raw
        .document
        .as_ref()
        .and_then(|d| d.split_blocks)
        .unwrap_or(true);

    let rules_raw = raw
        .rules
        .ok_or_else(|| TemplateError::new("template has no 'rules' section"))?;
    if rules_raw.is_empty() {
        return Err(TemplateError::new("template has no rules"));
    }

    let mut rules = Vec::new();
    let mut seen_ids = std::collections::HashSet::new();
    for (i, r) in rules_raw.into_iter().enumerate() {
        let rule = compile_rule(r, i)?;
        if !seen_ids.insert(rule.id.clone()) {
            return Err(TemplateError::new(format!(
                "duplicate rule id '{}'",
                rule.id
            )));
        }
        rules.push(rule);
    }

    Ok(CompiledTemplate {
        version,
        name,
        source_path: path.clone(),
        format,
        format_source,
        encoding,
        encoding_source,
        encoding_override,
        rules,
        block,
        max_output_chars,
        stable_deduplicate,
        sort,
        split_blocks,
    })
}

/// Validate an encoding name (case-insensitive). Returns the normalized
/// lowercase name. Used both for template `input.encoding` and for the
/// CLI `--encoding` override.
pub fn validate_encoding(e: &str) -> Result<String, TemplateError> {
    let norm = e.to_ascii_lowercase();
    // Canonical aliases: only `utf8` is folded into its canonical spelling so
    // that the recorded `Encoding:` field is stable; other names are kept as
    // typed (lowercased) because each is already unambiguous.
    if norm == "utf8" {
        return Ok("utf-8".to_string());
    }
    match norm.as_str() {
        "utf-8" | "utf-8-sig" | "utf-16le" | "utf-16" | "utf-16be" | "windows-1251" | "cp1251" => {
            Ok(norm)
        }
        other => Err(TemplateError::new(format!(
            "unsupported encoding '{}'",
            other
        ))),
    }
}

fn compile_rule(r: RawRule, index: usize) -> Result<CompiledRule, TemplateError> {
    let id =
        r.id.clone()
            .filter(|s| !s.trim().is_empty())
            .ok_or_else(|| {
                TemplateError::new(format!(
                    "rule #{} is missing required field 'id'",
                    index + 1
                ))
            })?;

    let when_format = match r.when {
        Some(w) => match w.format {
            Some(f) => {
                let fmt = Format::parse(&f).ok_or_else(|| {
                    TemplateError::new(format!("rule '{}': unknown when.format '{}'", id, f))
                })?;
                if fmt == Format::Auto {
                    return Err(TemplateError::new(format!(
                        "rule '{}': when.format cannot be 'auto'",
                        id
                    )));
                }
                Some(fmt)
            }
            None => None,
        },
        None => None,
    };
    // A `when.format` other than a fixed input format is not an error: the
    // rule never applies in such a run, which Applied Template states
    // (render::rule_never_applies). The Effective Template of a `--format`
    // run has exactly this shape and must stay reusable as a template.

    let (selector, selector_type, selector_expression) = compile_selector(&r.select, &id)?;

    let context = match r.context {
        Some(c) => Context {
            before: c.before.unwrap_or(0),
            after: c.after.unwrap_or(0),
        },
        None => Context {
            before: 0,
            after: 0,
        },
    };

    let output_key = match r.output {
        Some(o) => o.key,
        None => id.clone(),
    };

    let extract = match r.extract {
        Some(e) => parse_extract(&e, &id)?,
        None => ExtractMode::Text,
    };
    if let SelectorCompiled::XPath(expr) = &selector {
        crate::xpath::validate_extract(expr, &extract)
            .map_err(|e| TemplateError::new(format!("rule '{}': {}", id, e)))?;
    }

    let mut transforms = Vec::new();
    let mut result_transforms = Vec::new();
    let mut transform_order = Vec::new();
    if let Some(tlist) = r.transforms {
        for t in tlist {
            let (tname, tparam) = name_and_param(&t);
            let tname = tname.ok_or_else(|| {
                TemplateError::new(format!(
                    "rule '{}': malformed transform entry (must be a name or a single-key mapping)",
                    id
                ))
            })?;
            let parsed_item = parse_item_transform(&tname, &tparam, &id)?;
            let parsed_result = parse_result_transform(&tname, &tparam, &id)?;
            match (parsed_item, parsed_result) {
                (Some(it), _) => {
                    transform_order.push(TransformStep::Item(transforms.len()));
                    transforms.push(it);
                }
                (None, Some(rt)) => {
                    transform_order.push(TransformStep::Result(result_transforms.len()));
                    result_transforms.push(rt);
                }
                (None, None) => {
                    return Err(TemplateError::new(format!(
                        "rule '{}': unknown transform '{}'",
                        id, tname
                    )));
                }
            }
        }
    }

    let limits = r.limits;
    let max_items = match &limits {
        Some(l) => l.max_items,
        None => None,
    };
    let max_item_chars = match &limits {
        Some(l) => l.max_item_chars,
        None => None,
    };
    if let Some(li) = max_items {
        if li == 0 {
            return Err(TemplateError::new(format!(
                "rule '{}': max_items must be positive",
                id
            )));
        }
    }
    if let Some(lc) = max_item_chars {
        if lc == 0 {
            return Err(TemplateError::new(format!(
                "rule '{}': max_item_chars must be positive",
                id
            )));
        }
    }

    Ok(CompiledRule {
        id,
        when_format,
        selector,
        selector_type,
        selector_expression,
        context,
        output_key,
        extract,
        transforms,
        result_transforms,
        transform_order,
        max_items,
        max_item_chars,
        skip_unreadable_formats: false,
    })
}

fn parse_extract(e: &str, rule_id: &str) -> Result<ExtractMode, TemplateError> {
    match e {
        "text" => Ok(ExtractMode::Text),
        "inner_html" => Ok(ExtractMode::InnerHtml),
        "outer_html" => Ok(ExtractMode::OuterHtml),
        other => {
            if let Some(attr) = other.strip_prefix("attr:") {
                Ok(ExtractMode::Attr(attr.to_string()))
            } else {
                Err(TemplateError::new(format!(
                    "rule '{}': unknown extract mode '{}' (expected text, attr:NAME, inner_html, outer_html)",
                    rule_id, other
                )))
            }
        }
    }
}

fn name_and_param(v: &serde_yaml::Value) -> (Option<String>, Option<serde_yaml::Value>) {
    match v {
        serde_yaml::Value::String(s) => (Some(s.clone()), None),
        serde_yaml::Value::Mapping(m) => {
            if m.len() == 1 {
                if let Some((k, val)) = m.iter().next() {
                    if let serde_yaml::Value::String(name) = k {
                        return (Some(name.clone()), Some(val.clone()));
                    }
                }
            }
            (None, None)
        }
        _ => (None, None),
    }
}

fn parse_item_transform(
    name: &str,
    param: &Option<serde_yaml::Value>,
    rule_id: &str,
) -> Result<Option<ItemTransform>, TemplateError> {
    let it = match name {
        "trim" => ItemTransform::Trim,
        "normalize_whitespace" => ItemTransform::NormalizeWhitespace,
        "lowercase" => ItemTransform::Lowercase,
        "uppercase" => ItemTransform::Uppercase,
        "unescape" => ItemTransform::Unescape,
        "truncate_chars" => ItemTransform::TruncateChars(usize_param(name, param, rule_id)?),
        "truncate_lines" => ItemTransform::TruncateLines(usize_param(name, param, rule_id)?),
        "replace" => {
            let (from, to) = pair_param(name, param, rule_id)?;
            ItemTransform::Replace { from, to }
        }
        "regex_replace" => {
            let (pattern, replacement) = pair_param(name, param, rule_id)?;
            let re = regex::Regex::new(&pattern).map_err(|e| {
                TemplateError::new(format!(
                    "rule '{}': invalid regex in regex_replace: {}",
                    rule_id, e
                ))
            })?;
            ItemTransform::RegexReplace {
                pattern: re,
                replacement,
            }
        }
        _ => return Ok(None),
    };
    Ok(Some(it))
}

fn parse_result_transform(
    name: &str,
    param: &Option<serde_yaml::Value>,
    rule_id: &str,
) -> Result<Option<ResultTransform>, TemplateError> {
    let rt = match name {
        "drop_empty" => ResultTransform::DropEmpty,
        "unique" => ResultTransform::Unique,
        "sort" => ResultTransform::SortAsc,
        "sort_desc" => ResultTransform::SortDesc,
        "limit_items" => ResultTransform::LimitItems(usize_param(name, param, rule_id)?),
        "join" => {
            let sep = match param {
                Some(serde_yaml::Value::String(s)) => s.clone(),
                Some(serde_yaml::Value::Number(n)) => n.to_string(),
                _ => {
                    return Err(TemplateError::new(format!(
                        "rule '{}': join requires a string separator",
                        rule_id
                    )));
                }
            };
            ResultTransform::Join(sep)
        }
        _ => return Ok(None),
    };
    Ok(Some(rt))
}

fn usize_param(
    name: &str,
    param: &Option<serde_yaml::Value>,
    rule_id: &str,
) -> Result<usize, TemplateError> {
    match param {
        Some(serde_yaml::Value::Number(n)) => n.as_u64().map(|v| v as usize).ok_or_else(|| {
            TemplateError::new(format!(
                "rule '{}': {} requires a non-negative integer",
                rule_id, name
            ))
        }),
        _ => Err(TemplateError::new(format!(
            "rule '{}': {} requires an integer parameter",
            rule_id, name
        ))),
    }
}

fn pair_param(
    name: &str,
    param: &Option<serde_yaml::Value>,
    rule_id: &str,
) -> Result<(String, String), TemplateError> {
    match param {
        Some(serde_yaml::Value::Mapping(m)) => {
            let get = |k: &str| -> Result<String, TemplateError> {
                m.iter()
                    .find_map(|(k2, v2)| match (k2, v2) {
                        (serde_yaml::Value::String(s), serde_yaml::Value::String(v2)) if s == k => {
                            Some(Ok(v2.clone()))
                        }
                        (serde_yaml::Value::String(s), _) if s == k => {
                            Some(Err(TemplateError::new(format!(
                                "rule '{}': '{}' in {} must be a string",
                                rule_id, k, name
                            ))))
                        }
                        _ => None,
                    })
                    .ok_or_else(|| {
                        TemplateError::new(format!("rule '{}': {} requires '{}'", rule_id, name, k))
                    })?
            };
            let from = get("from")?;
            let to = get("to")?;
            Ok((from, to))
        }
        _ => Err(TemplateError::new(format!(
            "rule '{}': {} requires a mapping {{from: ..., to: ...}}",
            rule_id, name
        ))),
    }
}

fn compile_selector(
    sel: &serde_yaml::Value,
    rule_id: &str,
) -> Result<(SelectorCompiled, String, String), TemplateError> {
    let map = match sel {
        serde_yaml::Value::Mapping(m) => m,
        _ => {
            return Err(TemplateError::new(format!(
                "rule '{}': select must be a mapping with 'type' and expression fields",
                rule_id
            )));
        }
    };
    let get = |k: &str| -> Option<serde_yaml::Value> {
        map.iter().find_map(|(k2, v2)| match k2 {
            serde_yaml::Value::String(s) if s == k => Some(v2.clone()),
            _ => None,
        })
    };
    let sel_type = match get("type") {
        Some(serde_yaml::Value::String(s)) => s,
        _ => {
            return Err(TemplateError::new(format!(
                "rule '{}': select.type is required",
                rule_id
            )));
        }
    };
    let unknown: Vec<String> = map
        .iter()
        .filter_map(|(k2, _)| match k2 {
            serde_yaml::Value::String(s)
                if s != "type"
                    && s != "expression"
                    && s != "node"
                    && s != "pattern"
                    && s != "from"
                    && s != "to" =>
            {
                Some(s.clone())
            }
            _ => None,
        })
        .collect();
    if !unknown.is_empty() {
        return Err(TemplateError::new(format!(
            "rule '{}': unknown select properties: {}",
            rule_id,
            unknown.join(", ")
        )));
    }

    let expression = |rule_id: &str| -> Result<String, TemplateError> {
        match get("expression") {
            Some(serde_yaml::Value::String(s)) => Ok(s),
            Some(serde_yaml::Value::Number(n)) => Ok(n.to_string()),
            _ => Err(TemplateError::new(format!(
                "rule '{}': select.expression is required",
                rule_id
            ))),
        }
    };

    let selector = match sel_type.as_str() {
        "jsonpath" => {
            let expr = expression(rule_id)?;
            let parsed = crate::jsonpath::parse(&expr).map_err(|e| {
                TemplateError::new(format!("rule '{}': invalid jsonpath: {}", rule_id, e))
            })?;
            (SelectorCompiled::JsonPath(parsed), sel_type, expr)
        }
        "xpath" => {
            let expr = expression(rule_id)?;
            let parsed = crate::xpath::parse(&expr).map_err(|e| {
                TemplateError::new(format!("rule '{}': invalid xpath: {}", rule_id, e))
            })?;
            (SelectorCompiled::XPath(parsed), sel_type, expr)
        }
        "css" => {
            let expr = expression(rule_id)?;
            if expr.is_empty() {
                return Err(TemplateError::new(format!(
                    "rule '{}': css selector must not be empty",
                    rule_id
                )));
            }
            scraper::Selector::parse(&expr).map_err(|e| {
                TemplateError::new(format!("rule '{}': invalid css selector: {}", rule_id, e))
            })?;
            (SelectorCompiled::Css(expr.clone()), sel_type, expr)
        }
        "markdown" => {
            let node_name = match get("node") {
                Some(serde_yaml::Value::String(s)) => s,
                _ => {
                    return Err(TemplateError::new(format!(
                        "rule '{}': markdown selector requires select.node",
                        rule_id
                    )));
                }
            };
            let node = MarkdownNodeType::parse(&node_name).ok_or_else(|| {
                TemplateError::new(format!(
                    "rule '{}': unknown markdown node type '{}'",
                    rule_id, node_name
                ))
            })?;
            let pattern = match get("pattern") {
                Some(serde_yaml::Value::String(s)) => Some(regex::Regex::new(&s).map_err(|e| {
                    TemplateError::new(format!("rule '{}': invalid pattern regex: {}", rule_id, e))
                })?),
                Some(serde_yaml::Value::Number(n)) => {
                    Some(regex::Regex::new(&n.to_string()).map_err(|e| {
                        TemplateError::new(format!(
                            "rule '{}': invalid pattern regex: {}",
                            rule_id, e
                        ))
                    })?)
                }
                Some(_) => {
                    return Err(TemplateError::new(format!(
                        "rule '{}': select.pattern must be a string",
                        rule_id
                    )));
                }
                None => None,
            };
            // As the shorthand rule `markdown:NODE[:PATTERN]` writes it,
            // with the canonical node name.
            let expr = match &pattern {
                Some(p) => format!("{}:{}", node.name(), p.as_str()),
                None => node.name().to_string(),
            };
            (SelectorCompiled::Markdown { node, pattern }, sel_type, expr)
        }
        "path" => {
            let expr = expression(rule_id)?;
            let segments = crate::adapters::path_selector::parse_path(&expr).map_err(|e| {
                TemplateError::new(format!("rule '{}': invalid path: {}", rule_id, e))
            })?;
            (SelectorCompiled::Path(segments), sel_type, expr)
        }
        "regex" => {
            let expr = expression(rule_id)?;
            let re = regex::Regex::new(&expr).map_err(|e| {
                TemplateError::new(format!("rule '{}': invalid regex: {}", rule_id, e))
            })?;
            (SelectorCompiled::Regex(re), sel_type, expr)
        }
        "contains" => {
            let expr = expression(rule_id)?;
            (SelectorCompiled::Contains(expr.clone()), sel_type, expr)
        }
        "starts_with" => {
            let expr = expression(rule_id)?;
            (SelectorCompiled::StartsWith(expr.clone()), sel_type, expr)
        }
        "ends_with" => {
            let expr = expression(rule_id)?;
            (SelectorCompiled::EndsWith(expr.clone()), sel_type, expr)
        }
        "line" => {
            let from = match get("from") {
                Some(serde_yaml::Value::Number(n)) => n.as_u64().ok_or_else(|| {
                    TemplateError::new(format!(
                        "rule '{}': select.from must be a non-negative integer",
                        rule_id
                    ))
                })?,
                _ => {
                    return Err(TemplateError::new(format!(
                        "rule '{}': line selector requires select.from",
                        rule_id
                    )));
                }
            };
            let to = match get("to") {
                Some(serde_yaml::Value::Number(n)) => n.as_u64().ok_or_else(|| {
                    TemplateError::new(format!(
                        "rule '{}': select.to must be a non-negative integer",
                        rule_id
                    ))
                })?,
                _ => {
                    return Err(TemplateError::new(format!(
                        "rule '{}': line selector requires select.to",
                        rule_id
                    )));
                }
            };
            if to < from {
                return Err(TemplateError::new(format!(
                    "rule '{}': select.to must be >= select.from",
                    rule_id
                )));
            }
            // As the shorthand rule `line:FROM-TO` writes it.
            let expr = format!("{}-{}", from, to);
            (SelectorCompiled::LineRange { from, to }, sel_type, expr)
        }
        other => {
            return Err(TemplateError::new(format!(
                "rule '{}': unknown selector type '{}' (supported: jsonpath, xpath, css, markdown, path, regex, contains, starts_with, ends_with, line)",
                rule_id, other
            )));
        }
    };
    Ok(selector)
}

/// Canonical DSL text of the effective configuration, shown in the
/// `## Effective Template` section. It is itself a valid template: compiling
/// it yields the same effective configuration (and therefore this same
/// text). Selectors and transforms use their real DSL fields, and strings
/// are written so that the text never contains the anchor mark (see
/// `yaml_string`); `render` therefore embeds it unchanged.
pub fn effective_template_yaml(t: &CompiledTemplate) -> String {
    let mut output = vec![(
        "block",
        Yaml::Map(vec![
            ("target_chars", Yaml::Int(t.block.target_chars)),
            ("max_chars", Yaml::Int(t.block.max_chars)),
        ]),
    )];
    if let Some(mo) = t.max_output_chars {
        output.push(("max_output_chars", Yaml::Int(mo)));
    }
    let root = vec![
        ("version", Yaml::Int(u64::from(t.version))),
        ("name", Yaml::str(&t.name)),
        (
            "input",
            Yaml::Map(vec![
                (
                    "format",
                    Yaml::str(t.format.as_ref().map_or("auto", Format::name)),
                ),
                ("encoding", Yaml::str(&t.encoding)),
            ]),
        ),
        ("output", Yaml::Map(output)),
        (
            "document",
            Yaml::Map(vec![
                ("stable_deduplicate", Yaml::Bool(t.stable_deduplicate)),
                ("sort", Yaml::str(t.sort.as_deref().unwrap_or("none"))),
                ("split_blocks", Yaml::Bool(t.split_blocks)),
            ]),
        ),
        (
            "rules",
            Yaml::Seq(t.rules.iter().map(rule_to_yaml).collect()),
        ),
    ];
    let mut out = String::new();
    write_yaml_map(&mut out, &root, 0, false);
    out
}

/// The subset of YAML the effective template needs.
enum Yaml {
    Str(String),
    Int(u64),
    Bool(bool),
    Map(Vec<(&'static str, Yaml)>),
    Seq(Vec<Yaml>),
}

impl Yaml {
    fn str(s: &str) -> Yaml {
        Yaml::Str(s.to_string())
    }

    /// Single-entry mapping: the form of a transform that takes a parameter.
    fn entry(key: &'static str, value: Yaml) -> Yaml {
        Yaml::Map(vec![(key, value)])
    }
}

fn rule_to_yaml(r: &CompiledRule) -> Yaml {
    let mut m = vec![("id", Yaml::str(&r.id))];
    if let Some(wf) = &r.when_format {
        m.push(("when", Yaml::entry("format", Yaml::str(wf.name()))));
    }
    m.push(("select", Yaml::Map(selector_to_yaml(r))));
    m.push(("output", Yaml::entry("key", Yaml::str(&r.output_key))));
    if r.context.before > 0 || r.context.after > 0 {
        m.push((
            "context",
            Yaml::Map(vec![
                ("before", Yaml::Int(r.context.before as u64)),
                ("after", Yaml::Int(r.context.after as u64)),
            ]),
        ));
    }
    match &r.extract {
        ExtractMode::Text => {}
        ExtractMode::Attr(a) => m.push(("extract", Yaml::Str(format!("attr:{}", a)))),
        ExtractMode::InnerHtml => m.push(("extract", Yaml::str("inner_html"))),
        ExtractMode::OuterHtml => m.push(("extract", Yaml::str("outer_html"))),
    }
    let transforms: Vec<Yaml> = r
        .transform_order
        .iter()
        .map(|step| match step {
            TransformStep::Item(i) => item_transform_to_yaml(&r.transforms[*i]),
            TransformStep::Result(i) => result_transform_to_yaml(&r.result_transforms[*i]),
        })
        .collect();
    if !transforms.is_empty() {
        m.push(("transforms", Yaml::Seq(transforms)));
    }
    let mut limits = Vec::new();
    if let Some(mi) = r.max_items {
        limits.push(("max_items", Yaml::Int(mi)));
    }
    if let Some(mc) = r.max_item_chars {
        limits.push(("max_item_chars", Yaml::Int(mc)));
    }
    if !limits.is_empty() {
        m.push(("limits", Yaml::Map(limits)));
    }
    Yaml::Map(m)
}

/// `select` in the fields `compile_selector` reads for the selector type.
fn selector_to_yaml(r: &CompiledRule) -> Vec<(&'static str, Yaml)> {
    let mut m = vec![("type", Yaml::str(&r.selector_type))];
    match &r.selector {
        SelectorCompiled::Markdown { node, pattern } => {
            m.push(("node", Yaml::str(node.name())));
            if let Some(p) = pattern {
                m.push(("pattern", Yaml::str(p.as_str())));
            }
        }
        SelectorCompiled::LineRange { from, to } => {
            m.push(("from", Yaml::Int(*from)));
            m.push(("to", Yaml::Int(*to)));
        }
        _ => m.push(("expression", Yaml::str(&r.selector_expression))),
    }
    m
}

fn from_to(from: &str, to: &str) -> Yaml {
    Yaml::Map(vec![("from", Yaml::str(from)), ("to", Yaml::str(to))])
}

fn item_transform_to_yaml(t: &ItemTransform) -> Yaml {
    match t {
        ItemTransform::Trim => Yaml::str("trim"),
        ItemTransform::NormalizeWhitespace => Yaml::str("normalize_whitespace"),
        ItemTransform::Lowercase => Yaml::str("lowercase"),
        ItemTransform::Uppercase => Yaml::str("uppercase"),
        ItemTransform::Unescape => Yaml::str("unescape"),
        ItemTransform::TruncateChars(n) => Yaml::entry("truncate_chars", Yaml::Int(*n as u64)),
        ItemTransform::TruncateLines(n) => Yaml::entry("truncate_lines", Yaml::Int(*n as u64)),
        ItemTransform::Replace { from, to } => Yaml::entry("replace", from_to(from, to)),
        ItemTransform::RegexReplace {
            pattern,
            replacement,
        } => Yaml::entry("regex_replace", from_to(pattern.as_str(), replacement)),
    }
}

fn result_transform_to_yaml(t: &ResultTransform) -> Yaml {
    match t {
        ResultTransform::DropEmpty => Yaml::str("drop_empty"),
        ResultTransform::Unique => Yaml::str("unique"),
        ResultTransform::SortAsc => Yaml::str("sort"),
        ResultTransform::SortDesc => Yaml::str("sort_desc"),
        ResultTransform::LimitItems(n) => Yaml::entry("limit_items", Yaml::Int(*n as u64)),
        ResultTransform::Join(sep) => Yaml::entry("join", Yaml::str(sep)),
    }
}

/// Block-style mapping at `indent`. With `inline_first` the first key
/// continues the current line (after a sequence dash).
fn write_yaml_map(out: &mut String, entries: &[(&str, Yaml)], indent: usize, inline_first: bool) {
    for (i, (key, value)) in entries.iter().enumerate() {
        if i > 0 || !inline_first {
            push_indent(out, indent);
        }
        out.push_str(key);
        out.push(':');
        match value {
            Yaml::Map(m) if !m.is_empty() => {
                out.push('\n');
                write_yaml_map(out, m, indent + 2, false);
            }
            // Sequences under a key are not indented (serde_yaml's style).
            Yaml::Seq(items) if !items.is_empty() => {
                out.push('\n');
                write_yaml_seq(out, items, indent);
            }
            scalar => {
                out.push(' ');
                write_yaml_scalar(out, scalar);
                out.push('\n');
            }
        }
    }
}

fn write_yaml_seq(out: &mut String, items: &[Yaml], indent: usize) {
    for item in items {
        push_indent(out, indent);
        out.push('-');
        match item {
            Yaml::Map(m) if !m.is_empty() => {
                out.push(' ');
                write_yaml_map(out, m, indent + 2, true);
            }
            Yaml::Seq(nested) if !nested.is_empty() => {
                out.push('\n');
                write_yaml_seq(out, nested, indent + 2);
            }
            scalar => {
                out.push(' ');
                write_yaml_scalar(out, scalar);
                out.push('\n');
            }
        }
    }
}

fn write_yaml_scalar(out: &mut String, value: &Yaml) {
    match value {
        Yaml::Str(s) => out.push_str(&yaml_string(s)),
        Yaml::Int(n) => out.push_str(&n.to_string()),
        Yaml::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Yaml::Map(_) => out.push_str("{}"),
        Yaml::Seq(_) => out.push_str("[]"),
    }
}

fn push_indent(out: &mut String, indent: usize) {
    out.extend(std::iter::repeat_n(' ', indent));
}

/// YAML scalar that decodes to exactly `s` and never contains the anchor
/// mark — `<!-- BLOCK`, any run of `\`, then `:` — which
/// `render::escape_anchor_text` would otherwise rewrite inside the output
/// document, changing or invalidating the YAML.
///
/// One-line printable text keeps the style `serde_yaml` gives it (plain or
/// single-quoted, see `serde_yaml_scalar`); the rest — line breaks, control
/// characters and anchor marks, whose `:` is written as `\x3A` — is
/// double-quoted with escapes.
fn yaml_string(s: &str) -> String {
    let anchor_colons = anchor_mark_colons(s);
    if anchor_colons.is_empty()
        && s.chars().all(is_inline_yaml_char)
        && let Some(scalar) = serde_yaml_scalar(s)
    {
        return scalar;
    }
    let mut out = String::with_capacity(s.len() + 8);
    out.push('"');
    for (at, c) in s.char_indices() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            ':' if anchor_colons.binary_search(&at).is_ok() => out.push_str("\\x3A"),
            c if is_inline_yaml_char(c) => out.push(c),
            c if u32::from(c) <= 0xFF => out.push_str(&format!("\\x{:02X}", u32::from(c))),
            c if u32::from(c) <= 0xFFFF => out.push_str(&format!("\\u{:04X}", u32::from(c))),
            c => out.push_str(&format!("\\U{:08X}", u32::from(c))),
        }
    }
    out.push('"');
    out
}

/// Byte offsets of the `:` that ends each anchor mark in `s` (the same
/// sequence `render::escape_anchor_text` rewrites).
fn anchor_mark_colons(s: &str) -> Vec<usize> {
    let mark = crate::render::ANCHOR_MARK;
    let prefix = &mark[..mark.len() - 1];
    let bytes = s.as_bytes();
    s.match_indices(prefix)
        .filter_map(|(start, _)| {
            let after = start + prefix.len();
            let run = bytes[after..].iter().take_while(|b| **b == b'\\').count();
            (bytes.get(after + run) == Some(&b':')).then_some(after + run)
        })
        .collect()
}

/// A printable YAML character that may be written verbatim inside a
/// one-line scalar: not a tab, line break (YAML 1.1 also breaks lines at
/// U+0085, U+2028 and U+2029), byte order mark or other non-printable.
fn is_inline_yaml_char(c: char) -> bool {
    matches!(c, ' '..='~')
        || (c >= '\u{A0}'
            && !matches!(
                c,
                '\u{2028}' | '\u{2029}' | '\u{FEFF}' | '\u{FFFE}' | '\u{FFFF}'
            ))
}

/// `s` as `serde_yaml` writes it as a block mapping value — the text the
/// effective template had before it got its own emitter, so ordinary
/// templates are shown exactly as before. For one-line printable text that
/// is plain style, or single-quoted where plain would read back differently
/// (`'42'`, `'true'`, `'012'`, `'---x'`, `'a: b'`, `' lead'`). `None` if the
/// result is not a one-line plain or single-quoted scalar.
fn serde_yaml_scalar(s: &str) -> Option<String> {
    let doc = serde_yaml::to_string(&std::collections::BTreeMap::from([("k", s)])).ok()?;
    let scalar = doc.strip_prefix("k: ")?.strip_suffix('\n')?;
    (!scalar.is_empty() && !scalar.contains('\n') && !scalar.starts_with('"'))
        .then(|| scalar.to_string())
}

#[cfg(test)]
mod effective_template_tests {
    use super::*;

    fn compile(text: &str) -> CompiledTemplate {
        compile_yaml_str(text, &PathBuf::from("tpl.yml"), None, None).unwrap()
    }

    fn assert_scalar_round_trips(s: &str) {
        let scalar = yaml_string(s);
        for doc in [format!("k: {}\n", scalar), format!("- {}\n", scalar)] {
            let value: serde_yaml::Value = serde_yaml::from_str(&doc)
                .unwrap_or_else(|e| panic!("{s:?} as {doc:?} does not parse: {e}"));
            let decoded = match &value {
                serde_yaml::Value::Mapping(m) => m.get("k"),
                serde_yaml::Value::Sequence(items) => items.first(),
                _ => None,
            };
            assert_eq!(
                decoded,
                Some(&serde_yaml::Value::String(s.to_string())),
                "{s:?} as {doc:?}"
            );
            assert!(
                matches!(
                    crate::render::escape_anchor_text(&doc),
                    std::borrow::Cow::Borrowed(_)
                ),
                "anchor mark left in {doc:?}"
            );
            assert!(!doc.contains("\n```"), "fence break in {doc:?}");
        }
    }

    #[test]
    fn strings_round_trip_through_yaml_and_never_contain_anchor_marks() {
        for s in [
            "",
            " ",
            "plain",
            "a: b",
            "a:b",
            "trailing:",
            "#comment",
            "a #b",
            "a#b",
            " lead",
            "trail ",
            "line\nbreak",
            "\n",
            "tab\there",
            "cr\r\n",
            "'single'",
            "\"double\"",
            "it's",
            "back\\slash",
            "true",
            "False",
            "null",
            "~",
            "yes",
            "off",
            "12",
            "-3",
            "+4",
            ".5",
            "1.5e3",
            "0x1F",
            "1_000",
            ".inf",
            "-.nan",
            "---",
            "...",
            "- item",
            "-x",
            "? key",
            "[a, b]",
            "{a: b}",
            "$.a[0]",
            "*",
            "&anchor x",
            "!tag x",
            "%directive",
            "@at",
            "`tick",
            "a```",
            "```",
            "|",
            ">",
            "=",
            "<<",
            "ёжик 🦀 日本語",
            "\u{a0}nbsp\u{a0}",
            "nel\u{85}",
            "ls\u{2028}ps\u{2029}",
            "\u{feff}bom",
            "nul\u{0}del\u{7f}",
            "\u{1b}[31m",
            "<!-- BLOCK: B0001 START -->",
            "<!-- BLOCK\\: B0001 END -->",
            "<!-- BLOCK\\\\\\:",
            "x<!-- BLOCK:<!-- BLOCK\\:<!-- BLOCK",
            "<!-- BLOCK B0001 -->",
            "<!-- BLOCK\\",
            "<!-- BLOCK: a\n<!-- BLOCK: b",
            "'<!-- BLOCK: q'",
        ] {
            assert_scalar_round_trips(s);
        }
        // Every string of up to two characters from a set of YAML and
        // anchor-relevant characters.
        let alphabet: Vec<char> =
            " \t\n\r:#-?,[]{}&*!|>'\"%@`\\a0.~+=<\u{85}\u{2028}\u{feff}\u{0}ж"
                .chars()
                .collect();
        for a in &alphabet {
            assert_scalar_round_trips(&a.to_string());
            for b in &alphabet {
                assert_scalar_round_trips(&format!("{a}{b}"));
                assert_scalar_round_trips(&format!("<!-- BLOCK{a}{b}"));
            }
        }
    }

    #[test]
    fn plain_style_is_kept_for_ordinary_text() {
        for s in [
            "user_messages",
            "$.messages[?(@.role == 'user')].content",
            "(?i)(error|failed|timeout)",
            "//item[@id='a']",
            "div.content > p",
            "attr:href",
            "utf-8",
            "ёжик",
        ] {
            assert_eq!(yaml_string(s), s);
        }
        assert_eq!(yaml_string("a: b"), "'a: b'");
        assert_eq!(yaml_string("it's: x"), "'it''s: x'");
        assert_eq!(yaml_string("42"), "'42'");
        // The same style serde_yaml chose when it wrote the whole section:
        // text that only YAML 1.1 would read as a number or boolean stays
        // plain, document markers and leading-zero digits are quoted.
        for s in [
            "2024 report",
            "2fa_codes",
            "1C export",
            "1_000",
            "12:30",
            "-1x",
            ".5x",
            "y",
            "n",
            "yes",
            "on",
            "off",
        ] {
            assert_eq!(yaml_string(s), s);
        }
        for (s, quoted) in [
            ("---x", "'---x'"),
            ("...y", "'...y'"),
            ("012", "'012'"),
            ("-7", "'-7'"),
            ("1.5", "'1.5'"),
            ("true", "'true'"),
            ("null", "'null'"),
            ("~", "'~'"),
            ("", "''"),
            (" lead", "' lead'"),
        ] {
            assert_eq!(yaml_string(s), quoted);
        }
        assert_eq!(
            yaml_string("<!-- BLOCK: x"),
            "\"<!-- BLOCK\\x3A x\"",
            "anchor colon is escaped"
        );
        assert_eq!(
            yaml_string("<!-- BLOCK\\: x\n"),
            "\"<!-- BLOCK\\\\\\x3A x\\n\""
        );
    }

    #[test]
    fn selectors_and_transforms_use_their_dsl_fields() {
        let t = compile(concat!(
            "version: 1\nname: t\nrules:\n",
            "  - id: lines\n    select: {type: line, from: 2, to: 4}\n",
            "  - id: md\n    select: {type: markdown, node: li, pattern: '^x'}\n",
            "  - id: md_any\n    select: {type: markdown, node: heading}\n",
            "  - id: num\n    select: {type: contains, expression: 42}\n",
            "    transforms:\n      - regex_replace: {from: '(\\d+)', to: '<$1>'}\n",
            "      - join: 7\n",
        ));
        let yaml = effective_template_yaml(&t);
        for expected in [
            "- id: lines\n  select:\n    type: line\n    from: 2\n    to: 4\n",
            "- id: md\n  select:\n    type: markdown\n    node: list_item\n    pattern: ^x\n",
            "- id: md_any\n  select:\n    type: markdown\n    node: heading\n  output:",
            "    type: contains\n    expression: '42'\n",
            "  transforms:\n  - regex_replace:\n      from: (\\d+)\n      to: <$1>\n  - join: '7'\n",
        ] {
            assert!(yaml.contains(expected), "{expected:?} not in:\n{yaml}");
        }
        assert_eq!(effective_template_yaml(&compile(&yaml)), yaml);
    }

    #[test]
    fn canonical_names_parse_back_to_the_same_value() {
        use MarkdownNodeType as M;
        for node in [
            M::Heading,
            M::Paragraph,
            M::List,
            M::ListItem,
            M::Table,
            M::TableRow,
            M::TableCell,
            M::CodeBlock,
            M::Blockquote,
            M::Link,
            M::Image,
        ] {
            assert_eq!(MarkdownNodeType::parse(node.name()), Some(node));
        }
        for format in [
            Format::Json,
            Format::Xml,
            Format::Html,
            Format::Markdown,
            Format::Yaml,
            Format::Toml,
            Format::Text,
            Format::Auto,
        ] {
            assert_eq!(Format::parse(format.name()), Some(format));
        }
    }
}

#[cfg(test)]
mod cli_override_tests {
    use super::*;

    const MIN_TPL: &str = r#"
version: 1
name: t
input:
  format: auto
rules:
  - id: r
    select: {type: contains, expression: "x"}
    output: {key: k}
"#;

    fn compile(
        text: &str,
        cli_format: Option<&str>,
        cli_encoding: Option<&str>,
    ) -> Result<CompiledTemplate, TemplateError> {
        compile_yaml_str(
            text,
            &PathBuf::from("tpl.yml"),
            cli_format.map(str::to_string),
            cli_encoding.map(str::to_string),
        )
    }

    #[test]
    fn cli_format_overrides_template_auto() {
        let t = compile(MIN_TPL, Some("text"), None).unwrap();
        assert_eq!(t.format, Some(Format::Text));
        assert_eq!(t.format_source, "CLI override");
    }

    #[test]
    fn cli_format_overrides_template_fixed() {
        // Template explicitly declares json; CLI still wins (spec §67).
        let tpl = MIN_TPL.replace("format: auto", "format: json");
        let t = compile(&tpl, Some("text"), None).unwrap();
        assert_eq!(t.format, Some(Format::Text));
        assert_eq!(t.format_source, "CLI override");
    }

    #[test]
    fn when_format_may_differ_from_a_fixed_input_format() {
        let rules = "rules:\n  - id: r\n    when: {format: json}\n    select: {type: jsonpath, expression: \"$.a\"}\n";
        for input in ["", "input: {format: auto}\n", "input: {format: json}\n"] {
            let tpl = format!("version: 1\nname: t\n{input}{rules}");
            let t = compile(&tpl, Some("text"), None).unwrap();
            assert_eq!(t.format, Some(Format::Text));
            assert_eq!(t.rules[0].when_format, Some(Format::Json));
            // The Effective Template of that run compiles back to the same
            // configuration: input.format text, rule restricted to json.
            let again = compile(&effective_template_yaml(&t), None, None).unwrap();
            assert_eq!(again.format, Some(Format::Text));
            assert_eq!(again.format_source, "template");
            assert_eq!(again.rules[0].when_format, Some(Format::Json));
        }
        // A rule that never applies under the template's own input.format is
        // reported in Applied Template, not rejected.
        let tpl = format!("version: 1\nname: t\ninput: {{format: text}}\n{rules}");
        for cli in [None, Some("text"), Some("json")] {
            let t = compile(&tpl, cli, None).unwrap();
            assert_eq!(t.rules[0].when_format, Some(Format::Json));
        }
    }

    #[test]
    fn cli_format_invalid_rejected() {
        let e = compile(MIN_TPL, Some("bogus"), None).unwrap_err();
        assert!(e.message.contains("unknown CLI format 'bogus'"));
    }

    #[test]
    fn cli_format_auto_rejected() {
        let e = compile(MIN_TPL, Some("auto"), None).unwrap_err();
        assert!(e.message.contains("cannot be 'auto'"));
    }

    #[test]
    fn no_cli_keeps_template_value() {
        let t = compile(MIN_TPL, None, None).unwrap();
        assert_eq!(t.format, None); // auto -> per-file detection at run time
        assert_eq!(t.format_source, "auto detection");
    }

    #[test]
    fn cli_encoding_overrides_and_normalizes() {
        let t = compile(MIN_TPL, None, Some("Windows-1251")).unwrap();
        assert_eq!(t.encoding, "windows-1251");
        assert_eq!(t.encoding_source, "CLI override");
    }

    #[test]
    fn cli_encoding_invalid_rejected() {
        let e = compile(MIN_TPL, None, Some("utf-9")).unwrap_err();
        assert!(e.message.contains("unsupported encoding 'utf-9'"));
    }

    #[test]
    fn validate_encoding_accepts_aliases() {
        for (alias, norm) in [
            ("UTF-8", "utf-8"),
            ("utf8", "utf-8"),
            ("CP1251", "cp1251"),
            ("windows-1251", "windows-1251"),
            ("utf-16", "utf-16"),
        ] {
            assert_eq!(validate_encoding(alias).unwrap(), norm);
        }
        assert!(validate_encoding("ascii").is_err());
    }
}
