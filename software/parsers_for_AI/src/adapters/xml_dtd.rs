//! Safety checks for XML with a DOCTYPE, run before roxmltree parses it.
//!
//! roxmltree expands internal entities itself. Its loop detector stops
//! recursive entities and billion-laughs nesting, but not one large entity
//! referenced many times, the quadratic cost of merging many expansions into
//! one text node, slow searches of its entity table, or element markup inside
//! entities: such markup nests deeper than the literal depth pre-scan can
//! see, and an unbalanced entity yields element ranges that point backwards.
//! This module mirrors roxmltree's DOCTYPE grammar, bounds the work parsing
//! would do and rejects the file before roxmltree runs. External DTDs and
//! external entities are never loaded; roxmltree reports references to them
//! as unknown entities.

use super::{skip_past, skip_tag, tag_name};
use std::collections::HashMap;

/// Longest chain of entity references inside one reference (roxmltree's own
/// limit, so deeper chains could never parse anyway).
pub const MAX_ENTITY_NESTING: u32 = 10;
/// Replacement text all entity references of one file may produce together.
pub const MAX_ENTITY_EXPANSION_BYTES: u64 = 8 * 1024 * 1024;
/// Text the parser may copy while it merges pieces of text (literal text,
/// CDATA, entity text) into one text node.
pub const MAX_ENTITY_COPY_BYTES: u64 = 1024 * 1024 * 1024;
/// Entity-table declarations the parser may compare names with while it
/// resolves references.
pub const MAX_ENTITY_LOOKUPS: u64 = 64 * 1024 * 1024;
/// Name bytes those comparisons may read: names of equal length are
/// compared byte by byte, so long names make each comparison expensive.
pub const MAX_ENTITY_LOOKUP_BYTES: u64 = 1024 * 1024 * 1024;

/// Start of every error message of [`check_doctype`].
pub const REJECTED: &str = "XML DTD rejected";

/// Checks a document whose prolog has a DOCTYPE: entity expansion must stay
/// within the limits above, every entity used must contain well-formed
/// markup, and elements, including those from entities, must not nest deeper
/// than `depth_limit`.
pub fn check_doctype(text: &str, depth_limit: usize) -> Result<(), String> {
    let doctype = find_doctype(text.as_bytes())
        .ok_or_else(|| format!("{}: unsupported markup before the DOCTYPE", REJECTED))?;
    let (entities, body) = parse_doctype(text, doctype)?;
    Analysis::new(entities).scan_body(text, body, depth_limit)
}

#[derive(Debug, Clone, Copy)]
struct Entity<'t> {
    name: &'t str,
    value: &'t str,
}

fn is_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\r')
}

fn skip_spaces(bytes: &[u8], mut i: usize) -> usize {
    while i < bytes.len() && is_space(bytes[i]) {
        i += 1;
    }
    i
}

/// Like `skip_spaces`, but at least one space is required.
fn spaces(bytes: &[u8], i: usize) -> Option<usize> {
    let end = skip_spaces(bytes, i);
    (end > i).then_some(end)
}

/// Skips a name: everything up to a space, `[` or `>`. On input roxmltree
/// accepts this ends exactly where its name grammar ends.
fn skip_name(bytes: &[u8], mut i: usize) -> usize {
    while i < bytes.len() && !is_space(bytes[i]) && bytes[i] != b'[' && bytes[i] != b'>' {
        i += 1;
    }
    i
}

/// Index after the first `terminator` at or after `from`.
fn find_past(bytes: &[u8], from: usize, terminator: &[u8]) -> Option<usize> {
    bytes
        .get(from..)?
        .windows(terminator.len())
        .position(|w| w == terminator)
        .map(|p| from + p + terminator.len())
}

/// Like `find_past`, but ignores terminators inside quoted values.
fn find_past_unquoted(bytes: &[u8], from: usize, terminator: &[u8]) -> Option<usize> {
    let mut quote = None;
    for (i, &b) in bytes.iter().enumerate().skip(from) {
        match quote {
            Some(q) if b == q => quote = None,
            Some(_) => {}
            None if b == b'"' || b == b'\'' => quote = Some(b),
            None if bytes[i..].starts_with(terminator) => return Some(i + terminator.len()),
            None => {}
        }
    }
    None
}

fn position(text: &str, at: usize) -> String {
    let before = &text.as_bytes()[..at.min(text.len())];
    let line_start = before
        .iter()
        .rposition(|&b| b == b'\n')
        .map_or(0, |p| p + 1);
    let line = before.iter().filter(|&&b| b == b'\n').count() + 1;
    let column = String::from_utf8_lossy(&before[line_start..])
        .chars()
        .count()
        + 1;
    format!("{}:{}", line, column)
}

/// Where `<!DOCTYPE` starts after the XML declaration, comments and
/// processing instructions, read the way roxmltree reads the prolog.
fn find_doctype(bytes: &[u8]) -> Option<usize> {
    const BOM: &[u8] = b"\xEF\xBB\xBF";
    let mut i = if bytes.starts_with(BOM) { BOM.len() } else { 0 };
    if bytes[i..].starts_with(b"<?xml ") {
        i = find_past_unquoted(bytes, i + 6, b"?>")?;
    }
    loop {
        i = skip_spaces(bytes, i);
        let rest = &bytes[i..];
        i = if rest.starts_with(b"<!--") {
            find_past(bytes, i + 4, b"-->")?
        } else if rest.starts_with(b"<?") {
            find_past(bytes, i + 2, b"?>")?
        } else if rest.starts_with(b"<!DOCTYPE") {
            return Some(i);
        } else {
            return None;
        };
    }
}

/// Skips an optional `SYSTEM "uri"` or `PUBLIC "id" "uri"`.
fn skip_external_id(bytes: &[u8], i: usize) -> Option<usize> {
    let rest = &bytes[i..];
    let literals = if rest.starts_with(b"SYSTEM") {
        1
    } else if rest.starts_with(b"PUBLIC") {
        2
    } else {
        return Some(i);
    };
    let mut i = i + 6;
    for _ in 0..literals {
        i = spaces(bytes, i)?;
        let quote = *bytes.get(i).filter(|b| matches!(b, b'"' | b'\''))?;
        i = find_past(bytes, i + 1, &[quote])?;
    }
    Some(i)
}

/// Mirrors roxmltree's DOCTYPE parser. Returns the entities it registers, in
/// declaration order (general and parameter entities share one table there
/// and the first declaration of a name wins), and the index after the
/// DOCTYPE. Anything roxmltree would not accept is rejected here as well.
fn parse_doctype(text: &str, start: usize) -> Result<(Vec<Entity<'_>>, usize), String> {
    let bytes = text.as_bytes();
    let malformed = |at: usize| {
        format!(
            "{}: malformed or unsupported DOCTYPE at {}",
            REJECTED,
            position(text, at)
        )
    };
    let mut i = spaces(bytes, start + b"<!DOCTYPE".len()).ok_or_else(|| malformed(start))?;
    i = skip_spaces(bytes, skip_name(bytes, i));
    i = skip_external_id(bytes, i).ok_or_else(|| malformed(i))?;
    i = skip_spaces(bytes, i);
    match bytes.get(i) {
        Some(b'>') => return Ok((Vec::new(), i + 1)),
        Some(b'[') => i += 1,
        _ => return Err(malformed(i)),
    }
    let mut entities = Vec::new();
    loop {
        i = skip_spaces(bytes, i);
        let rest = &bytes[i..];
        let next = if rest.starts_with(b"<!ENTITY") {
            parse_entity(text, i, &mut entities)
        } else if rest.starts_with(b"<!--") {
            find_past(bytes, i + 4, b"-->")
        } else if rest.starts_with(b"<?") {
            find_past(bytes, i + 2, b"?>")
        } else if rest.starts_with(b"]") {
            let end = skip_spaces(bytes, i + 1);
            if bytes.get(end) == Some(&b'>') {
                return Ok((entities, end + 1));
            }
            None
        } else if rest.starts_with(b"<!ELEMENT")
            || rest.starts_with(b"<!ATTLIST")
            || rest.starts_with(b"<!NOTATION")
        {
            // roxmltree skips these up to the first `>`, quoted or not.
            find_past(bytes, i, b">")
        } else {
            None
        };
        i = next.ok_or_else(|| malformed(i))?;
    }
}

/// Mirrors roxmltree's `<!ENTITY …>` parser and returns the index after it.
/// Internal entities are recorded; external ones are never registered.
fn parse_entity<'t>(text: &'t str, start: usize, entities: &mut Vec<Entity<'t>>) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut i = spaces(bytes, start + b"<!ENTITY".len())?;
    let general = bytes.get(i) != Some(&b'%');
    if !general {
        i = spaces(bytes, i + 1)?;
    }
    let name_start = i;
    while i < bytes.len() && !is_space(bytes[i]) {
        i += 1;
    }
    let name = &text[name_start..i];
    i = spaces(bytes, i)?;
    let rest = &bytes[i..];
    match *rest.first()? {
        quote @ (b'"' | b'\'') => {
            let end = find_past(bytes, i + 1, &[quote])?;
            entities.push(Entity {
                name,
                value: &text[i + 1..end - 1],
            });
            i = end;
        }
        _ if rest.starts_with(b"SYSTEM") || rest.starts_with(b"PUBLIC") => {
            i = skip_external_id(bytes, i)?;
            if general {
                let after = skip_spaces(bytes, i);
                if bytes[after..].starts_with(b"NDATA") {
                    i = skip_name(bytes, spaces(bytes, after + 5)?);
                }
            }
        }
        _ => return None,
    }
    i = skip_spaces(bytes, i);
    (bytes.get(i) == Some(&b'>')).then_some(i + 1)
}

/// `&name;` references that need the entity table (character references and
/// the five predefined entities do not), with their byte offsets.
fn references(text: &str) -> impl Iterator<Item = (usize, &str)> + '_ {
    text.match_indices('&').filter_map(move |(at, _)| {
        let rest = &text[at + 1..];
        let len = rest
            .find(|c: char| matches!(c, ';' | '&' | '<' | '"' | '\'') || c.is_ascii_whitespace())?;
        let name = &rest[..len];
        let needs_table = rest[len..].starts_with(';')
            && !name.is_empty()
            && !name.starts_with('#')
            && !matches!(name, "lt" | "gt" | "amp" | "apos" | "quot");
        needs_table.then_some((at, name))
    })
}

/// The quoted attribute values of a start tag (`tag` runs from `<` to `>`).
fn attribute_values(tag: &str) -> impl Iterator<Item = &str> + '_ {
    let bytes = tag.as_bytes();
    let mut i = 0;
    std::iter::from_fn(move || {
        let open = i + bytes[i..].iter().position(|&b| b == b'"' || b == b'\'')?;
        let close = bytes[open + 1..]
            .iter()
            .position(|&b| b == bytes[open])
            .map_or(bytes.len(), |p| open + 1 + p);
        i = (close + 1).min(bytes.len());
        Some(&tag[open + 1..close])
    })
}

fn add(total: &mut u64, amount: u64) {
    *total = total.saturating_add(amount);
}

/// Where replacement text is parsed. The two differ in which references
/// inside it roxmltree expands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Context {
    /// In content the text is parsed as markup: references in its text and
    /// attribute values are expanded, those inside comments, CDATA and
    /// processing instructions are not.
    Content,
    /// In an attribute value the text is appended to the value as is and
    /// every reference in it is expanded, markup or not.
    Attribute,
}

/// Work searching roxmltree's entity table does: it compares the name with
/// the declarations in order until the first one with that name, and names
/// of equal length byte by byte.
#[derive(Debug, Clone, Copy, Default)]
struct Lookups {
    /// Declarations compared.
    entries: u64,
    /// Name bytes compared (an upper bound: comparison stops at the first
    /// differing byte).
    bytes: u64,
}

impl Lookups {
    fn add(&mut self, other: Lookups) {
        add(&mut self.entries, other.entries);
        add(&mut self.bytes, other.bytes);
    }
}

/// How parsing a stretch of content grows the parser's text nodes.
/// roxmltree adds every piece of text (literal text between references, a
/// CDATA section, the text of an expansion) to the open text node by copying
/// the whole node, so one node built from many pieces costs quadratic time.
/// Elements, comments and processing instructions close the node; CDATA and
/// entity references do not. The fields are upper bounds; a stretch is
/// described independently of the node open where it starts, so an entity's
/// description is computed once and reused for every reference to it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct TextNodes {
    /// Text added before the first markup that closes the node: it extends
    /// the node open where the stretch starts.
    head_len: u64,
    /// Pieces added before that markup. Each also copies the node that was
    /// open where the stretch starts.
    head_pieces: u64,
    /// Bytes those pieces copy, not counting the node open at the start.
    head_copies: u64,
    /// Whether markup in the stretch closes the node.
    closed: bool,
    /// Bytes copied after the first markup that closes the node.
    rest_copies: u64,
    /// Length of the node left open at the end, after that markup.
    tail_len: u64,
}

impl TextNodes {
    /// One piece of `len` bytes; nothing for empty literal text.
    fn literal(&mut self, len: usize) {
        if len > 0 {
            self.piece(len as u64);
        }
    }

    fn piece(&mut self, len: u64) {
        if self.closed {
            add(&mut self.tail_len, len);
            add(&mut self.rest_copies, self.tail_len);
        } else {
            add(&mut self.head_len, len);
            add(&mut self.head_pieces, 1);
            add(&mut self.head_copies, self.head_len);
        }
    }

    /// Markup that closes the open text node.
    fn close(&mut self) {
        self.closed = true;
        self.tail_len = 0;
    }

    /// An expansion described by `inner` parsed at this point.
    fn splice(&mut self, inner: &TextNodes) {
        if self.closed {
            add(
                &mut self.rest_copies,
                inner.head_pieces.saturating_mul(self.tail_len),
            );
            add(&mut self.rest_copies, inner.head_copies);
        } else {
            add(
                &mut self.head_copies,
                inner.head_pieces.saturating_mul(self.head_len),
            );
            add(&mut self.head_copies, inner.head_copies);
            add(&mut self.head_pieces, inner.head_pieces);
        }
        if inner.closed {
            if !self.closed {
                add(&mut self.head_len, inner.head_len);
            }
            self.closed = true;
            add(&mut self.rest_copies, inner.rest_copies);
            self.tail_len = inner.tail_len;
        } else if self.closed {
            add(&mut self.tail_len, inner.head_len);
        } else {
            add(&mut self.head_len, inner.head_len);
        }
    }

    /// Bytes copied in all when the stretch starts without an open node.
    fn copies(&self) -> u64 {
        self.head_copies.saturating_add(self.rest_copies)
    }
}

/// Upper bounds for what parsing one reference to an entity costs.
#[derive(Debug, Clone, Copy, Default)]
struct Cost {
    /// Replacement text produced, nested references included.
    bytes: u64,
    /// Table searches for the nested references.
    lookups: Lookups,
    /// How the expansion grows text nodes (in content only).
    text: TextNodes,
    /// Element nesting the expansion adds below the reference (in content
    /// only).
    depth: usize,
}

#[derive(Debug, Clone, Copy)]
enum Memo {
    Visiting,
    Done(Cost),
}

#[derive(Debug, Default)]
struct Totals {
    bytes: u64,
    copies: u64,
    lookups: Lookups,
}

impl Totals {
    fn check(&self) -> Result<(), String> {
        if self.bytes > MAX_ENTITY_EXPANSION_BYTES {
            Err(format!(
                "{}: entity references expand to more than {} bytes (entity expansion bomb)",
                REJECTED, MAX_ENTITY_EXPANSION_BYTES
            ))
        } else if self.copies > MAX_ENTITY_COPY_BYTES {
            Err(format!(
                "{}: entity references would make the parser copy more than {} bytes of text (quadratic entity blowup)",
                REJECTED, MAX_ENTITY_COPY_BYTES
            ))
        } else if self.lookups.entries > MAX_ENTITY_LOOKUPS {
            Err(format!(
                "{}: entity references need more than {} entity lookups",
                REJECTED, MAX_ENTITY_LOOKUPS
            ))
        } else if self.lookups.bytes > MAX_ENTITY_LOOKUP_BYTES {
            Err(format!(
                "{}: entity references need entity lookups that compare more than {} bytes of entity names",
                REJECTED, MAX_ENTITY_LOOKUP_BYTES
            ))
        } else {
            Ok(())
        }
    }
}

struct Analysis<'t> {
    entities: Vec<Entity<'t>>,
    /// Index of the first declaration of each name, which roxmltree's
    /// linear table search finds.
    first: HashMap<&'t str, usize>,
    /// What finding each declaration costs.
    found: Vec<Lookups>,
    /// Declarations per name length, for searches that find nothing.
    lengths: HashMap<usize, u64>,
    /// Costs of the entities used so far, per context.
    memo: HashMap<(usize, Context), Memo>,
    totals: Totals,
}

impl<'t> Analysis<'t> {
    fn new(entities: Vec<Entity<'t>>) -> Self {
        let mut first = HashMap::new();
        let mut lengths: HashMap<usize, u64> = HashMap::new();
        let mut found = Vec::with_capacity(entities.len());
        for (index, entity) in entities.iter().enumerate() {
            first.entry(entity.name).or_insert(index);
            let same_length = lengths.entry(entity.name.len()).or_insert(0);
            *same_length += 1;
            found.push(Lookups {
                entries: index as u64 + 1,
                bytes: same_length.saturating_mul(entity.name.len() as u64),
            });
        }
        Analysis {
            entities,
            first,
            found,
            lengths,
            memo: HashMap::new(),
            totals: Totals::default(),
        }
    }

    /// The declaration roxmltree finds for `name`, and what searching costs.
    fn lookup(&self, name: &str) -> (Option<usize>, Lookups) {
        match self.first.get(name) {
            Some(&index) => (Some(index), self.found[index]),
            None => {
                let same_length = self.lengths.get(&name.len()).copied().unwrap_or(0);
                let searched = Lookups {
                    entries: self.entities.len() as u64,
                    bytes: same_length.saturating_mul(name.len() as u64),
                };
                (None, searched)
            }
        }
    }

    /// Accounts for one reference in the document body. `None`: the entity
    /// is not declared, so roxmltree stops there with an "unknown entity
    /// reference" error and parses nothing after it.
    fn resolve(&mut self, name: &str, context: Context) -> Result<Option<Cost>, String> {
        let (index, searched) = self.lookup(name);
        self.totals.lookups.add(searched);
        let Some(index) = index else {
            return Ok(None);
        };
        let cost = self.cost(index, context, 1)?;
        add(&mut self.totals.bytes, cost.bytes);
        self.totals.lookups.add(cost.lookups);
        Ok(Some(cost))
    }

    /// Accounts for a reference inside the value of an entity at `level`
    /// whose cost `outer` is being computed.
    fn nested(
        &mut self,
        name: &str,
        context: Context,
        level: u32,
        outer: &mut Cost,
    ) -> Result<Option<Cost>, String> {
        let (index, searched) = self.lookup(name);
        outer.lookups.add(searched);
        let Some(index) = index else {
            return Ok(None);
        };
        let inner = self.cost(index, context, level + 1)?;
        add(&mut outer.bytes, inner.bytes);
        outer.lookups.add(inner.lookups);
        Ok(Some(inner))
    }

    fn cost(&mut self, index: usize, context: Context, level: u32) -> Result<Cost, String> {
        let name = self.entities[index].name;
        match self.memo.get(&(index, context)) {
            Some(Memo::Done(cost)) => return Ok(*cost),
            Some(Memo::Visiting) => {
                return Err(format!(
                    "{}: entity '{}' is recursive (its expansion refers back to it)",
                    REJECTED, name
                ));
            }
            None => {}
        }
        if level > MAX_ENTITY_NESTING {
            return Err(format!(
                "{}: entity references nest deeper than {} levels at entity '{}'",
                REJECTED, MAX_ENTITY_NESTING, name
            ));
        }
        self.memo.insert((index, context), Memo::Visiting);
        let cost = match context {
            Context::Content => self.content_cost(index, level)?,
            Context::Attribute => self.attribute_cost(index, level)?,
        };
        self.memo.insert((index, context), Memo::Done(cost));
        Ok(cost)
    }

    /// In an attribute value the replacement text is plain text, so markup
    /// in it does not matter, but every reference in it is expanded.
    fn attribute_cost(&mut self, index: usize, level: u32) -> Result<Cost, String> {
        let value = self.entities[index].value;
        let mut cost = Cost {
            bytes: value.len() as u64,
            ..Cost::default()
        };
        for (_, reference) in references(value) {
            self.nested(reference, Context::Attribute, level, &mut cost)?;
        }
        Ok(cost)
    }

    /// Walks the value the way roxmltree parses it as content: checks that
    /// every element in it also ends in it, and accounts for its text pieces,
    /// the element depth it adds and the references it expands.
    fn content_cost(&mut self, index: usize, level: u32) -> Result<Cost, String> {
        let Entity { name, value } = self.entities[index];
        let bytes = value.as_bytes();
        let not_well_formed = || {
            format!(
                "{}: entity '{}' contains markup that is not well-formed (each element must start and end inside the same entity)",
                REJECTED, name
            )
        };
        let mut cost = Cost {
            bytes: value.len() as u64,
            ..Cost::default()
        };
        let mut open: Vec<&[u8]> = Vec::new();
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] != b'<' {
                let end = bytes[i..]
                    .iter()
                    .position(|&b| b == b'<')
                    .map_or(bytes.len(), |p| i + p);
                let segment = &value[i..end];
                let mut literal_from = 0;
                for (at, reference) in references(segment) {
                    cost.text.literal(at - literal_from);
                    literal_from = at + reference.len() + 2;
                    if let Some(inner) =
                        self.nested(reference, Context::Content, level, &mut cost)?
                    {
                        cost.text.splice(&inner.text);
                        cost.depth = cost.depth.max(open.len() + inner.depth);
                    }
                }
                cost.text.literal(segment.len() - literal_from);
                i = end;
                continue;
            }
            let rest = &bytes[i..];
            let next = if rest.starts_with(b"<!--") {
                cost.text.close();
                find_past(bytes, i + 4, b"-->")
            } else if rest.starts_with(b"<![CDATA[") {
                let end = find_past(bytes, i + 9, b"]]>");
                if let Some(end) = end {
                    cost.text.piece((end - i - b"<![CDATA[]]>".len()) as u64);
                }
                end
            } else if rest.starts_with(b"<?") {
                cost.text.close();
                find_past(bytes, i + 2, b"?>")
            } else if rest.starts_with(b"<!") {
                None
            } else if rest.starts_with(b"</") {
                cost.text.close();
                let closed = tag_name(bytes, i + 2);
                (open.pop() == Some(closed))
                    .then(|| find_past(bytes, i, b">"))
                    .flatten()
            } else {
                cost.text.close();
                let (end, self_closing) = skip_tag(bytes, i + 1);
                if bytes[..end].ends_with(b">") {
                    for attribute in attribute_values(&value[i..end]) {
                        for (_, reference) in references(attribute) {
                            self.nested(reference, Context::Attribute, level, &mut cost)?;
                        }
                    }
                    if !self_closing {
                        open.push(tag_name(bytes, i + 1));
                        cost.depth = cost.depth.max(open.len());
                    }
                    Some(end)
                } else {
                    None
                }
            };
            i = next.ok_or_else(not_well_formed)?;
        }
        if open.is_empty() {
            Ok(cost)
        } else {
            Err(not_well_formed())
        }
    }

    /// Walks the content after the DOCTYPE like roxmltree's tokenizer and
    /// accounts for every entity reference in text and attribute values, up
    /// to the first undeclared one.
    fn scan_body(mut self, text: &str, start: usize, depth_limit: usize) -> Result<(), String> {
        let bytes = text.as_bytes();
        let too_deep = || {
            format!(
                "XML nesting depth exceeds the limit of {} elements",
                depth_limit
            )
        };
        let mut nodes = TextNodes::default();
        let mut depth = 0usize;
        let mut i = start;
        while i < bytes.len() {
            let rest = &bytes[i..];
            if rest[0] != b'<' {
                let end = rest
                    .iter()
                    .position(|&b| b == b'<')
                    .map_or(bytes.len(), |p| i + p);
                let segment = &text[i..end];
                let mut literal_from = 0;
                for (at, name) in references(segment) {
                    nodes.literal(at - literal_from);
                    literal_from = at + name.len() + 2;
                    let Some(cost) = self.resolve(name, Context::Content)? else {
                        return Ok(());
                    };
                    if depth + cost.depth > depth_limit {
                        return Err(format!(
                            "{} (entity '{}' adds {} levels of elements)",
                            too_deep(),
                            name,
                            cost.depth
                        ));
                    }
                    nodes.splice(&cost.text);
                    self.totals.copies = nodes.copies();
                    self.totals.check()?;
                }
                nodes.literal(segment.len() - literal_from);
                i = end;
            } else if rest.starts_with(b"<![CDATA[") {
                // CDATA text joins the text node around it.
                let end = skip_past(bytes, i + 9, b"]]>");
                nodes.piece((end - i) as u64);
                i = end;
            } else if rest.starts_with(b"<!--") {
                nodes.close();
                i = skip_past(bytes, i + 4, b"-->");
            } else if rest.starts_with(b"<?") {
                nodes.close();
                i = skip_past(bytes, i + 2, b"?>");
            } else if rest.starts_with(b"<!") {
                // roxmltree fails on any other declaration here; nothing
                // after it is parsed.
                break;
            } else if rest.starts_with(b"</") {
                nodes.close();
                depth = depth.saturating_sub(1);
                i = skip_tag(bytes, i + 2).0;
            } else {
                nodes.close();
                let (end, self_closing) = skip_tag(bytes, i + 1);
                for attribute in attribute_values(&text[i..end]) {
                    for (_, name) in references(attribute) {
                        if self.resolve(name, Context::Attribute)?.is_none() {
                            return Ok(());
                        }
                    }
                }
                if !self_closing {
                    depth += 1;
                    if depth > depth_limit {
                        return Err(too_deep());
                    }
                }
                i = end;
            }
            self.totals.copies = nodes.copies();
            self.totals.check()?;
        }
        self.totals.copies = nodes.copies();
        self.totals.check()
    }
}

#[cfg(test)]
mod tests {
    use crate::adapters::XmlTree;
    use crate::limits::MAX_XML_NESTING_DEPTH;
    use std::time::{Duration, Instant};

    fn parse_error(text: &str) -> String {
        XmlTree::parse(text).map(|_| ()).unwrap_err()
    }

    fn billion_laughs(reference: &str) -> String {
        let mut dtd = String::from("<!DOCTYPE r [<!ENTITY lol0 \"lol\">");
        for level in 1..=9 {
            let refs = format!("&lol{};", level - 1).repeat(10);
            dtd.push_str(&format!("<!ENTITY lol{level} \"{refs}\">"));
        }
        format!("{dtd}]>{}", reference.replace("REF", "&lol9;"))
    }

    #[test]
    fn doctypes_without_entities_parse_like_before() {
        let plist = concat!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n",
            "<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" ",
            "\"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n",
            "<plist version=\"1.0\"><dict><key>Name</key><string>App</string></dict></plist>\n"
        );
        let tree = XmlTree::parse(plist).unwrap();
        assert_eq!(tree.doc.root_element().tag_name().name(), "plist");
        let declarations = concat!(
            "<!DOCTYPE r SYSTEM 'r.dtd' [\n",
            "  <!ELEMENT r (#PCDATA)> <!ATTLIST r id CDATA #IMPLIED>\n",
            "  <!-- a comment ]> --> <?pi ]> ?> <!NOTATION gif SYSTEM 'gif'>\n",
            "  <!ENTITY logo SYSTEM 'logo.gif' NDATA gif> <!ENTITY % pe 'x'>\n",
            "]><r id='1'>text</r>"
        );
        assert_eq!(
            XmlTree::parse(declarations)
                .unwrap()
                .doc
                .root_element()
                .text(),
            Some("text")
        );
    }

    #[test]
    fn internal_entities_expand_in_text_and_attributes() {
        let text = concat!(
            "<!DOCTYPE r [<!ENTITY ver '1.2'><!ENTITY name \"app &ver;\">",
            "<!ENTITY ver 'ignored: the first declaration wins'>",
            "<!ENTITY item '<item id=\"&ver;\">&name;</item>'>]>",
            "<r v='&name;'>&name; &amp; &#65;&item;</r>"
        );
        let tree = XmlTree::parse(text).unwrap();
        let root = tree.doc.root_element();
        assert_eq!(root.attribute("v"), Some("app 1.2"));
        assert_eq!(root.text(), Some("app 1.2 & A"));
        let item = root.children().find(|n| n.is_element()).unwrap();
        assert_eq!(item.attribute("id"), Some("1.2"));
        assert_eq!(item.text(), Some("app 1.2"));
        // roxmltree keeps parameter entities in the same table, so the
        // checks must count them as general entities too.
        let parameter = "<!DOCTYPE r [<!ENTITY % pe 'val'>]><r>&pe;</r>";
        let tree = XmlTree::parse(parameter).unwrap();
        assert_eq!(tree.doc.root_element().text(), Some("val"));
        // External entities are never loaded: roxmltree reports them.
        let external = "<!DOCTYPE r [<!ENTITY x SYSTEM 'file:///etc/passwd'>]><r>&x;</r>";
        assert!(parse_error(external).contains("unknown entity reference 'x'"));
    }

    #[test]
    fn entity_bombs_are_rejected_before_expansion() {
        let started = Instant::now();
        for (document, expected) in [
            (billion_laughs("<r>REF</r>"), "more than 8388608 bytes"),
            (billion_laughs("<r a='REF'/>"), "more than 8388608 bytes"),
            (
                // One large entity referenced many times (quadratic blowup):
                // unchecked, roxmltree needed 38 s for this in a release build.
                format!(
                    "<!DOCTYPE r [<!ENTITY big '{}'>]><r>{}</r>",
                    "x".repeat(100_000),
                    "&big;".repeat(1000)
                ),
                "more than 8388608 bytes",
            ),
            (
                // roxmltree also resolves `&name;` to parameter entities.
                format!(
                    "<!DOCTYPE r [<!ENTITY % big '{}'>]><r>{}</r>",
                    "x".repeat(100_000),
                    "&big;".repeat(1000)
                ),
                "more than 8388608 bytes",
            ),
            (
                // Little text, but every reference re-copies the growing node.
                format!(
                    "<!DOCTYPE r [<!ENTITY e 'x'>]><r>{}</r>",
                    "&e;".repeat(1_000_000)
                ),
                "copy more than 1073741824 bytes",
            ),
            (
                "<!DOCTYPE r [<!ENTITY a '&b;'><!ENTITY b '&a;'>]><r>&a;</r>".to_string(),
                "entity 'a' is recursive",
            ),
            (
                "<!DOCTYPE r [<!ENTITY a 'x&a;'>]><r a='&a;'/>".to_string(),
                "entity 'a' is recursive",
            ),
        ] {
            let error = parse_error(&document);
            assert!(error.starts_with("XML DTD rejected: "), "{error}");
            assert!(error.contains(expected), "{error}");
        }
        assert!(started.elapsed() < Duration::from_secs(5));
    }

    #[test]
    fn entity_lookups_and_nesting_are_bounded() {
        let mut dtd = String::from("<!DOCTYPE r [");
        for index in 0..20_000 {
            dtd.push_str(&format!("<!ENTITY e{index} ''>"));
        }
        let last = format!("{dtd}]><r>{}</r>", "&e19999;".repeat(5_000));
        assert!(parse_error(&last).contains("more than 67108864 entity lookups"));

        let mut chain = String::from("<!DOCTYPE r [<!ENTITY c0 'x'>");
        for level in 1..=10 {
            chain.push_str(&format!("<!ENTITY c{level} '&c{};'>", level - 1));
        }
        chain.push_str("]>");
        // c9 is a chain of 10 entities, roxmltree's limit; c10 is one more.
        assert!(XmlTree::parse(&format!("{chain}<r>&c9;</r>")).is_ok());
        let error = parse_error(&format!("{chain}<r>&c10;</r>"));
        assert!(error.contains("nest deeper than 10 levels"), "{error}");
    }

    /// A table of `count` entities whose names have `len` bytes and share
    /// all but their last 6, and `refs` references to the last one.
    fn long_names(len: usize, count: usize, refs: usize) -> String {
        let name = |index: usize| format!("{}{index:06}", "n".repeat(len - 6));
        let mut document = String::from("<!DOCTYPE r [");
        for index in 0..count {
            document.push_str(&format!("<!ENTITY {} ''>", name(index)));
        }
        let reference = format!("&{};", name(count - 1));
        document.push_str(&format!("]><r>{}</r>", reference.repeat(refs)));
        document
    }

    #[test]
    fn entity_lookups_are_bounded_by_the_name_bytes_they_compare() {
        // 4 Mi table entries in all, far below the entry limit, but every
        // entry has a name of the same length, so each one is compared byte
        // by byte: 4 GB. Unchecked, roxmltree spent 2.2 s on a 16 MB file of
        // this kind (release build) and 17 s on a 128 MB one.
        let started = Instant::now();
        let error = parse_error(&long_names(1000, 2000, 2000));
        assert!(started.elapsed() < Duration::from_secs(5));
        assert_eq!(
            error,
            "XML DTD rejected: entity references need entity lookups that compare more than 1073741824 bytes of entity names"
        );
        // The same table and references stay cheap with short names.
        assert!(XmlTree::parse(&long_names(8, 2000, 2000)).is_ok());
        // A search that finds nothing compares every name of the same
        // length, but roxmltree stops at the first one with its own error.
        let mut unknown = long_names(1000, 2000, 0);
        unknown.truncate(unknown.len() - "<r></r>".len());
        let missing = format!("&{}x;", "n".repeat(999));
        unknown.push_str(&format!("<r>{}</r>", missing.repeat(2000)));
        let error = parse_error(&unknown);
        assert!(
            error.starts_with("invalid XML: unknown entity reference"),
            "{error}"
        );
    }

    #[test]
    fn markup_closes_text_nodes_so_only_merged_text_counts() {
        // 400 x 1000 empty elements from one entity: no text node at all.
        // The first estimate charged every '<' as a text merge and rejected
        // this as a quadratic blowup.
        let elements = format!(
            "<!DOCTYPE r [<!ENTITY m '{}'>]><r>{}</r>",
            "<a/>".repeat(1000),
            "<b>&m;</b>".repeat(400)
        );
        let tree = XmlTree::parse(&elements).unwrap();
        assert_eq!(
            tree.doc.descendants().filter(|n| n.is_element()).count(),
            400_401
        );
        // Text around such markup still merges with the text next to the
        // reference: 'x' pieces before the first element of `p` each copy
        // the 1 MiB node they extend, after `&p;` the node is a new one.
        let head = |pieces: usize| {
            format!(
                "<!DOCTYPE r [<!ENTITY e 'x'><!ENTITY p '{}<a/>{}'>]><r>{}&p;{}</r>",
                "&e;".repeat(pieces),
                "y".repeat(1000),
                "z".repeat(1 << 20),
                "&e;".repeat(1000)
            )
        };
        assert!(XmlTree::parse(&head(100)).is_ok());
        let error = parse_error(&head(1100));
        assert!(error.contains("copy more than 1073741824 bytes"), "{error}");
    }

    #[test]
    fn text_node_estimates_compose_like_the_flat_text() {
        use super::TextNodes;
        // Pieces (Some(len)) and closing markup (None) in document order.
        let flat = |events: &[Option<u64>]| {
            let mut nodes = TextNodes::default();
            for event in events {
                match event {
                    Some(len) => nodes.piece(*len),
                    None => nodes.close(),
                }
            }
            nodes
        };
        let inners: [&[Option<u64>]; 4] = [
            &[Some(3), Some(4)],
            &[Some(2), None, Some(5), Some(1)],
            &[None],
            &[Some(7), None, None, Some(2), None],
        ];
        let outers: [&[Option<u64>]; 3] = [&[], &[Some(10)], &[Some(1), None, Some(6)]];
        for inner in inners {
            for before in outers {
                for after in outers {
                    let mut spliced = flat(before);
                    spliced.splice(&flat(inner));
                    let rest = flat(after);
                    spliced.splice(&rest);
                    let all: Vec<_> = [before, inner, after].concat();
                    assert_eq!(spliced, flat(&all), "{before:?} {inner:?} {after:?}");
                }
            }
        }
    }

    #[test]
    fn references_expand_in_comments_only_inside_attribute_values() {
        // Parsed as content, the reference in the comment stays text.
        let comment = "<!DOCTYPE r [<!ENTITY a '<!-- &a; -->x'>]>";
        let in_content = format!("{comment}<r>&a;</r>");
        let tree = XmlTree::parse(&in_content).unwrap();
        let children: Vec<_> = tree.doc.root_element().children().collect();
        assert!(children[0].is_comment());
        assert_eq!(children[0].text(), Some(" &a; "));
        assert_eq!(children[1].text(), Some("x"));
        // In an attribute value roxmltree expands it, so it is recursive.
        let error = parse_error(&format!("{comment}<r v='&a;'/>"));
        assert!(error.contains("entity 'a' is recursive"), "{error}");
        // So an expansion hidden in a comment still counts there.
        let hidden = format!(
            "<!DOCTYPE r [<!ENTITY big '{}'><!ENTITY a '<!--{}-->'>]>",
            "x".repeat(100_000),
            "&big;".repeat(100)
        );
        assert!(XmlTree::parse(&format!("{hidden}<r>&a;</r>")).is_ok());
        let error = parse_error(&format!("{hidden}<r v='&a;'/>"));
        assert!(error.contains("more than 8388608 bytes"), "{error}");
        // Attribute values never parse markup, so balance does not matter.
        let markup = "<!DOCTYPE r [<!ENTITY c '<b/></a>'>]><r v='&c;'/>";
        let tree = XmlTree::parse(markup).unwrap();
        assert_eq!(tree.doc.root_element().attribute("v"), Some("<b/></a>"));
    }

    #[test]
    fn markup_from_entities_counts_toward_the_depth_limit() {
        let nested = |entity_depth: usize, literal_depth: usize| {
            format!(
                "<!DOCTYPE r [<!ENTITY deep '{}x{}'>]><r>{}&deep;{}</r>",
                "<a>".repeat(entity_depth),
                "</a>".repeat(entity_depth),
                "<b>".repeat(literal_depth),
                "</b>".repeat(literal_depth)
            )
        };
        // Unchecked, roxmltree built this 202-level tree although the
        // literal markup is only 101 levels deep.
        let error = parse_error(&nested(100, 100));
        assert!(
            error.starts_with("XML nesting depth exceeds the limit of 128 elements"),
            "{error}"
        );
        assert!(error.contains("entity 'deep' adds 100 levels"), "{error}");
        // r + 27 literal levels + 100 from the entity = 128: allowed.
        let deepest_allowed = nested(100, 27);
        let tree = XmlTree::parse(&deepest_allowed).unwrap();
        let depth = tree
            .doc
            .descendants()
            .filter(|n| n.is_element())
            .map(|n| n.ancestors().count() - 1);
        assert_eq!(depth.max(), Some(MAX_XML_NESTING_DEPTH));
        assert!(XmlTree::parse(&nested(100, 28)).is_err());

        // Entities that nest other entities add up: 10 x 100 levels.
        let mut chain = String::from("<!DOCTYPE r [<!ENTITY d0 'x'>");
        for level in 1..=9 {
            chain.push_str(&format!(
                "<!ENTITY d{level} '{}&d{};{}'>",
                "<a>".repeat(100),
                level - 1,
                "</a>".repeat(100)
            ));
        }
        let error = parse_error(&format!("{chain}]><r>&d9;</r>"));
        assert!(error.contains("adds 900 levels"), "{error}");
    }

    #[test]
    fn entities_with_unbalanced_markup_are_rejected() {
        for value in [
            "<a>",
            "</a>",
            "<b/></a>",
            "x</a><a>y",
            "<a></b>",
            "<!DOCTYPE x>",
            "<a",
        ] {
            let document = format!("<!DOCTYPE r [<!ENTITY bad '{value}'>]><r><a>&bad;</a></r>");
            let error = parse_error(&document);
            assert!(
                error.contains("entity 'bad' contains markup that is not well-formed"),
                "{value}: {error}"
            );
        }
        // Well-formed markup with comments, CDATA and quoted '>' is fine.
        let fine = "<!DOCTYPE r [<!ENTITY ok '<a k=\"1>0\"><!--<x>--><![CDATA[</a>]]><b/></a>'>]><r>&ok;</r>";
        assert!(XmlTree::parse(fine).is_ok());
    }

    #[test]
    fn depth_is_checked_from_the_real_end_of_the_doctype() {
        // The generic declaration skipper loses track of `[` inside a
        // processing instruction and would skip the whole document.
        let deep = format!(
            "<!DOCTYPE r [ <?pi [ ?> ]><r>{}{}</r>",
            "<a>".repeat(200),
            "</a>".repeat(200)
        );
        assert!(super::super::check_xml_depth(&deep, MAX_XML_NESTING_DEPTH).is_ok());
        let error = parse_error(&deep);
        assert_eq!(error, "XML nesting depth exceeds the limit of 128 elements");
    }

    #[test]
    fn malformed_doctypes_get_a_positioned_error() {
        for document in [
            "<!DOCTYPE>",
            "<!DOCTYPE r [ <!ENTITY a> ]><r/>",
            "<!DOCTYPE r [ <!ENTITY % pe 'x'> %pe; ]><r/>",
            "<!DOCTYPE r [ <!ENTITY a 'x' ><r/>",
            "<!DOCTYPE r PUBLIC 'only-one-literal'><r/>",
        ] {
            let error = parse_error(document);
            assert!(
                error.starts_with("XML DTD rejected: malformed or unsupported DOCTYPE at 1:"),
                "{document}: {error}"
            );
        }
    }
}
