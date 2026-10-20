//! JSONPath subset. Supported features follow RFC 9535 semantics: filters
//! select children of arrays and objects, comparisons treat a missing path
//! as Nothing (RFC 9535 §2.3.5.2.2), number literals use the JSON number
//! grammar and string literals decode the RFC escape sequences.

use serde_json::{Number, Value};
use std::borrow::Cow;
use std::cmp::Ordering;
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Step {
    Child(String),
    Wildcard,
    Descendant(String),
    DescendantWildcard,
    Index(usize),
    Indices(Vec<usize>),
    Filter(FilterExpr),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilterExpr {
    // Alternatives joined by OR; comparisons within each alternative use AND.
    pub groups: Vec<Vec<Comp>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Comp {
    pub left: Operand,
    pub op: Op,
    pub right: Operand,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Operand {
    Path(Vec<String>),
    Str(String),
    /// Parsed exactly like a number in the JSON source (integer, or f64).
    Num(Number),
    Bool(bool),
    Null,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Op {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonPathExpr {
    pub steps: Vec<Step>,
}

pub fn parse(expr: &str) -> Result<JsonPathExpr, String> {
    let mut p = Parser {
        chars: expr.chars().collect(),
        pos: 0,
    };
    p.parse_expr()
}

struct Parser {
    chars: Vec<char>,
    pos: usize,
}

impl Parser {
    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn bump(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.pos += 1;
        Some(c)
    }

    fn skip_ws(&mut self) {
        while matches!(
            self.peek(),
            Some(' ') | Some('\t') | Some('\n') | Some('\r')
        ) {
            self.pos += 1;
        }
    }

    fn parse_expr(&mut self) -> Result<JsonPathExpr, String> {
        self.skip_ws();
        if self.peek() == Some('$') {
            self.bump();
        }
        let mut steps = Vec::new();
        loop {
            self.skip_ws();
            match self.peek() {
                None => break,
                Some('.') => {
                    self.bump();
                    let step = self.parse_dot_step()?;
                    steps.push(step);
                }
                Some('[') => {
                    self.bump();
                    self.skip_ws();
                    let step = self.parse_bracket_step()?;
                    steps.push(step);
                }
                Some(c) => {
                    return Err(format!(
                        "unexpected character '{}' at position {}",
                        c, self.pos
                    ));
                }
            }
        }
        Ok(JsonPathExpr { steps })
    }

    fn parse_dot_step(&mut self) -> Result<Step, String> {
        if self.peek() == Some('.') {
            self.bump();
            if self.peek() == Some('*') {
                self.bump();
                return Ok(Step::DescendantWildcard);
            }
            let name = self.parse_name()?;
            if name.is_empty() {
                return Err("expected property name or '*' after '..'".to_string());
            }
            return Ok(Step::Descendant(name));
        }
        if self.peek() == Some('*') {
            self.bump();
            return Ok(Step::Wildcard);
        }
        let name = self.parse_name()?;
        if name.is_empty() {
            return Err("expected property name after '.'".to_string());
        }
        Ok(Step::Child(name))
    }

    fn parse_bracket_step(&mut self) -> Result<Step, String> {
        if self.peek() == Some('?') {
            self.bump();
            self.expect_char('(')?;
            let f = self.parse_filter()?;
            self.expect_char(')')?;
            self.expect_char(']')?;
            return Ok(Step::Filter(f));
        }
        if self.peek() == Some('*') {
            self.bump();
            self.expect_char(']')?;
            return Ok(Step::Wildcard);
        }
        if self.peek() == Some('\'') || self.peek() == Some('"') {
            let name = self.parse_string_literal()?;
            self.skip_ws();
            self.expect_char(']')?;
            return Ok(Step::Child(name));
        }
        let mut indices = Vec::new();
        loop {
            self.skip_ws();
            let start = self.pos;
            while matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
                self.bump();
            }
            if self.pos == start {
                return Err("expected array index inside '[]'".to_string());
            }
            let digits: String = self.chars[start..self.pos].iter().collect();
            let idx: usize = digits
                .parse()
                .map_err(|_| format!("invalid array index '{}'", digits))?;
            indices.push(idx);
            self.skip_ws();
            match self.peek() {
                Some(',') => {
                    self.bump();
                }
                Some(']') => {
                    self.bump();
                    break;
                }
                other => {
                    return Err(format!(
                        "expected ',' or ']' in index list, got {:?}",
                        other
                    ));
                }
            }
        }
        if indices.len() == 1 {
            Ok(Step::Index(indices[0]))
        } else {
            Ok(Step::Indices(indices))
        }
    }

    fn expect_char(&mut self, c: char) -> Result<(), String> {
        self.skip_ws();
        match self.peek() {
            Some(actual) if actual == c => {
                self.bump();
                Ok(())
            }
            other => Err(format!("expected '{}', got {:?}", c, other)),
        }
    }

    fn parse_name(&mut self) -> Result<String, String> {
        let start = self.pos;
        while let Some(c) = self.peek() {
            if matches!(c, 'a'..='z' | 'A'..='Z' | '0'..='9' | '_' | '-') || c > '\u{7f}' {
                self.bump();
            } else {
                break;
            }
        }
        Ok(self.chars[start..self.pos].iter().collect())
    }

    /// Parses a quoted string at the current position (RFC 9535 §2.3.1.1).
    /// Escapes: `\b \f \n \r \t \/ \\ \uXXXX` (surrogate pairs combined) and
    /// the enclosing quote. Any other escape is rejected instead of being kept
    /// as literal text.
    fn parse_string_literal(&mut self) -> Result<String, String> {
        let start = self.pos;
        let quote = match self.bump() {
            Some(c @ ('\'' | '"')) => c,
            other => return Err(format!("expected a quoted string, got {:?}", other)),
        };
        let unterminated = || format!("unterminated string literal starting at position {start}");
        let mut text = String::new();
        loop {
            let c = self.bump().ok_or_else(unterminated)?;
            if c == quote {
                return Ok(text);
            }
            if c != '\\' {
                text.push(c);
                continue;
            }
            let escape_pos = self.pos - 1;
            let escaped = self.bump().ok_or_else(unterminated)?;
            match escaped {
                'b' => text.push('\u{8}'),
                'f' => text.push('\u{c}'),
                'n' => text.push('\n'),
                'r' => text.push('\r'),
                't' => text.push('\t'),
                '/' => text.push('/'),
                '\\' => text.push('\\'),
                'u' => text.push(self.parse_unicode_escape(escape_pos)?),
                c if c == quote => text.push(c),
                '\'' | '"' => {
                    return Err(format!(
                        "escape '\\{escaped}' at position {escape_pos} is only allowed inside {} strings; write {escaped} without a backslash",
                        if escaped == '\'' {
                            "single-quoted"
                        } else {
                            "double-quoted"
                        }
                    ));
                }
                other => {
                    return Err(format!(
                        "invalid escape '\\{other}' at position {escape_pos} (supported: \\b \\f \\n \\r \\t \\/ \\\\ \\uXXXX and the escaped quote; write \\\\ for a literal backslash)"
                    ));
                }
            }
        }
    }

    /// Parses the `XXXX` of a `\uXXXX` escape (the `\u` is already consumed),
    /// combining a high surrogate with the following `\uXXXX` low surrogate.
    fn parse_unicode_escape(&mut self, escape_pos: usize) -> Result<char, String> {
        let first = self.parse_hex4(escape_pos)?;
        let code = match first {
            0xD800..=0xDBFF => {
                if !self.starts_with("\\u") {
                    return Err(format!(
                        "unpaired surrogate '\\u{first:04X}' at position {escape_pos}: a high surrogate must be followed by a \\uXXXX low surrogate"
                    ));
                }
                let low_pos = self.pos;
                self.pos += 2;
                let low = self.parse_hex4(low_pos)?;
                if !(0xDC00..=0xDFFF).contains(&low) {
                    return Err(format!(
                        "unpaired surrogate '\\u{first:04X}' at position {escape_pos}: '\\u{low:04X}' is not a low surrogate"
                    ));
                }
                0x10000 + ((first - 0xD800) << 10) + (low - 0xDC00)
            }
            0xDC00..=0xDFFF => {
                return Err(format!(
                    "unpaired surrogate '\\u{first:04X}' at position {escape_pos}: a low surrogate must follow a high surrogate"
                ));
            }
            code => code,
        };
        char::from_u32(code)
            .ok_or_else(|| format!("invalid \\u escape at position {escape_pos}: U+{code:X}"))
    }

    fn parse_hex4(&mut self, escape_pos: usize) -> Result<u32, String> {
        let mut code = 0;
        for _ in 0..4 {
            let digit = self.peek().and_then(|c| c.to_digit(16)).ok_or_else(|| {
                format!("invalid \\u escape at position {escape_pos}: expected 4 hex digits")
            })?;
            self.bump();
            code = code * 16 + digit;
        }
        Ok(code)
    }

    /// Parses a number literal: `-? DIGITS (. DIGITS)? ([eE] [+-]? DIGITS)?`
    /// (RFC 9535 §2.3.5.1). Redundant leading zeros (`007`) stay accepted as
    /// before. The value is parsed by serde_json, exactly like a number in the
    /// JSON source, so literal and data share one representation: an integer
    /// literal is exact, a literal with a fraction or exponent is an IEEE-754
    /// double. Literals that this representation would silently change are
    /// rejected: integers outside [-2^63, 2^64) (they would be rounded to a
    /// double), non-zero values that underflow to 0, and overflow.
    fn parse_number_literal(&mut self) -> Result<Number, String> {
        let start = self.pos;
        let invalid = |p: &Self| {
            let s: String = p.chars[start..p.pos].iter().collect();
            format!(
                "invalid number '{s}' at position {start}: expected a digit at position {}",
                p.pos
            )
        };
        if self.peek() == Some('-') {
            self.bump();
        }
        let int_start = self.pos;
        if !self.skip_digits() {
            return Err(invalid(self));
        }
        let int_end = self.pos;
        let mut is_integer = true;
        if self.peek() == Some('.') {
            self.bump();
            is_integer = false;
            if !self.skip_digits() {
                return Err(invalid(self));
            }
        }
        let mantissa_end = self.pos;
        if matches!(self.peek(), Some('e' | 'E')) {
            self.bump();
            is_integer = false;
            if matches!(self.peek(), Some('+' | '-')) {
                self.bump();
            }
            if !self.skip_digits() {
                return Err(invalid(self));
            }
        }
        let literal: String = self.chars[start..self.pos].iter().collect();
        // serde_json follows the strict JSON grammar, which rejects leading zeros.
        let leading_zeros = self.chars[int_start..int_end - 1]
            .iter()
            .take_while(|c| **c == '0')
            .count();
        let json: String = self.chars[start..int_start]
            .iter()
            .chain(&self.chars[int_start + leading_zeros..self.pos])
            .collect();
        let number = json
            .parse::<Number>()
            .map_err(|_| format!("number '{literal}' at position {start} is out of range"))?;
        let nonzero = self.chars[int_start..mantissa_end]
            .iter()
            .any(|c| matches!(c, '1'..='9'));
        // serde_json turns an integer outside i64/u64 into a rounded double
        // (`-0` is the only zero that also becomes a double).
        if is_integer && nonzero && exact_integer(&number).is_none() {
            return Err(format!(
                "integer '{literal}' at position {start} is outside the exact integer range [-2^63, 2^64); \
                 write it with a fraction or exponent (for example '{literal}.0') to compare it as a rounded double"
            ));
        }
        if nonzero && number.as_f64() == Some(0.0) {
            return Err(format!(
                "number '{literal}' at position {start} is out of range: it is too small and would round to 0"
            ));
        }
        Ok(number)
    }

    fn skip_digits(&mut self) -> bool {
        let start = self.pos;
        while matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
            self.bump();
        }
        self.pos > start
    }

    fn parse_filter(&mut self) -> Result<FilterExpr, String> {
        let mut groups = vec![vec![self.parse_comp(true)?]];
        loop {
            self.skip_ws();
            if self.starts_with("&&") {
                self.pos += 2;
                groups.last_mut().unwrap().push(self.parse_comp(true)?);
            } else if self.starts_with("||") {
                self.pos += 2;
                groups.push(vec![self.parse_comp(true)?]);
            } else {
                break;
            }
        }
        Ok(FilterExpr { groups })
    }

    fn starts_with(&self, s: &str) -> bool {
        let t: String = self.chars[self.pos..].iter().take(s.len()).collect();
        t == s
    }

    fn parse_comp(&mut self, expect_close: bool) -> Result<Comp, String> {
        self.skip_ws();
        let left = self.parse_operand()?;
        self.skip_ws();
        if expect_close && matches!(self.peek(), Some(')') | Some(']')) {
            return Err(
                "comparison predicate requires an operator like ==, !=, <, <=, >, >=".to_string(),
            );
        }
        let op = self.parse_op()?;
        self.skip_ws();
        let right = self.parse_operand()?;
        Ok(Comp { left, op, right })
    }

    fn parse_op(&mut self) -> Result<Op, String> {
        let two: String = self.chars[self.pos..].iter().take(2).collect();
        match two.as_str() {
            "==" => {
                self.pos += 2;
                return Ok(Op::Eq);
            }
            "!=" => {
                self.pos += 2;
                return Ok(Op::Ne);
            }
            "<=" => {
                self.pos += 2;
                return Ok(Op::Le);
            }
            ">=" => {
                self.pos += 2;
                return Ok(Op::Ge);
            }
            _ => {}
        }
        match self.peek() {
            Some('<') => {
                self.bump();
                Ok(Op::Lt)
            }
            Some('>') => {
                self.bump();
                Ok(Op::Gt)
            }
            other => Err(format!("expected comparison operator, got {:?}", other)),
        }
    }

    fn parse_operand(&mut self) -> Result<Operand, String> {
        self.skip_ws();
        match self.peek() {
            Some('\'') | Some('"') => Ok(Operand::Str(self.parse_string_literal()?)),
            Some('@') => {
                self.bump();
                let mut segs = Vec::new();
                loop {
                    self.skip_ws();
                    match self.peek() {
                        Some('.') => {
                            self.bump();
                            let n = self.parse_name()?;
                            if n.is_empty() {
                                return Err("expected property name after '@.'".to_string());
                            }
                            segs.push(n);
                        }
                        Some('[') => {
                            self.bump();
                            self.skip_ws();
                            if self.peek() == Some('\'') || self.peek() == Some('"') {
                                let n = self.parse_string_literal()?;
                                self.expect_char(']')?;
                                segs.push(n);
                            } else {
                                return Err("expected quoted key in '@[...]'".to_string());
                            }
                        }
                        _ => break,
                    }
                }
                if segs.is_empty() {
                    return Ok(Operand::Path(Vec::new()));
                }
                Ok(Operand::Path(segs))
            }
            Some('t') if self.starts_with("true") => {
                self.pos += 4;
                Ok(Operand::Bool(true))
            }
            Some('f') if self.starts_with("false") => {
                self.pos += 5;
                Ok(Operand::Bool(false))
            }
            Some('n') if self.starts_with("null") => {
                self.pos += 4;
                Ok(Operand::Null)
            }
            Some(c) if c.is_ascii_digit() || c == '-' => {
                Ok(Operand::Num(self.parse_number_literal()?))
            }
            other => Err(format!("unexpected operand start {:?}", other)),
        }
    }
}

/// Resolves an operand against the current node. `None` is RFC 9535
/// "Nothing": the path does not exist in this node.
fn eval_operand<'a>(op: &'a Operand, node: &'a Value) -> Option<Cow<'a, Value>> {
    match op {
        Operand::Str(s) => Some(Cow::Owned(Value::String(s.clone()))),
        Operand::Num(n) => Some(Cow::Owned(Value::Number(n.clone()))),
        Operand::Bool(b) => Some(Cow::Owned(Value::Bool(*b))),
        Operand::Null => Some(Cow::Owned(Value::Null)),
        Operand::Path(segs) => {
            let mut cur = node;
            for s in segs {
                match cur {
                    Value::Object(m) => {
                        let next = m.get(s);
                        match next {
                            Some(v) => cur = v,
                            None => return None,
                        }
                    }
                    _ => return None,
                }
            }
            Some(Cow::Borrowed(cur))
        }
    }
}

/// RFC 9535 §2.3.5.2.2: `!=` is the negation of `==`, `<=` is `<` or `==`,
/// `>` and `>=` swap the operands. Nothing equals only Nothing and is never
/// less than anything, so `@.missing != 1` holds and `@.missing < 1` does not.
fn eval_comp(comp: &Comp, node: &Value) -> bool {
    let left = eval_operand(&comp.left, node);
    let right = eval_operand(&comp.right, node);
    let (l, r) = (left.as_deref(), right.as_deref());
    match comp.op {
        Op::Eq => operands_equal(l, r),
        Op::Ne => !operands_equal(l, r),
        Op::Lt => operand_less(l, r),
        Op::Le => operand_less(l, r) || operands_equal(l, r),
        Op::Gt => operand_less(r, l),
        Op::Ge => operand_less(r, l) || operands_equal(l, r),
    }
}

fn operands_equal(a: Option<&Value>, b: Option<&Value>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => values_equal(a, b),
        _ => false,
    }
}

/// Only two numbers or two strings are ordered; anything else is not less.
fn operand_less(a: Option<&Value>, b: Option<&Value>) -> bool {
    match (a, b) {
        (Some(Value::Number(x)), Some(Value::Number(y))) => number_cmp(x, y) == Ordering::Less,
        // Byte order of UTF-8 equals Unicode scalar value order.
        (Some(Value::String(x)), Some(Value::String(y))) => x < y,
        _ => false,
    }
}

/// Deep equality (RFC 9535 §2.3.5.2.2): arrays element-wise, objects by the
/// same member names with equal values, numbers by exact numeric value.
fn values_equal(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::String(s1), Value::String(s2)) => s1 == s2,
        (Value::Number(n1), Value::Number(n2)) => numbers_equal(n1, n2),
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::Null, Value::Null) => true,
        (Value::Array(x), Value::Array(y)) => {
            x.len() == y.len() && x.iter().zip(y).all(|(x, y)| values_equal(x, y))
        }
        (Value::Object(x), Value::Object(y)) => {
            x.len() == y.len()
                && x.iter()
                    .all(|(k, v)| y.get(k).is_some_and(|w| values_equal(v, w)))
        }
        _ => false,
    }
}

fn numbers_equal(a: &Number, b: &Number) -> bool {
    number_cmp(a, b) == Ordering::Equal
}

/// Exact numeric order of two JSON numbers. Integers are never rounded to
/// f64 and floats are never rounded or saturated into the integer domain.
fn number_cmp(a: &Number, b: &Number) -> Ordering {
    match (exact_integer(a), exact_integer(b)) {
        (Some(x), Some(y)) => x.cmp(&y),
        (Some(x), None) => integer_float_cmp(x, float_of(b)),
        (None, Some(y)) => integer_float_cmp(y, float_of(a)).reverse(),
        // serde_json numbers are always finite, so partial_cmp cannot fail.
        (None, None) => float_of(a)
            .partial_cmp(&float_of(b))
            .unwrap_or(Ordering::Equal),
    }
}

fn exact_integer(n: &Number) -> Option<i128> {
    n.as_i64()
        .map(i128::from)
        .or_else(|| n.as_u64().map(i128::from))
}

fn float_of(n: &Number) -> f64 {
    n.as_f64().unwrap_or(0.0)
}

/// Orders `integer` (within the JSON integer domain [-2^63, 2^64)) against a
/// finite `float` without losing precision on either side.
fn integer_float_cmp(integer: i128, float: f64) -> Ordering {
    const TWO_POW_64: f64 = 18_446_744_073_709_551_616.0;
    if float >= TWO_POW_64 {
        return Ordering::Less;
    }
    if float < -TWO_POW_64 {
        return Ordering::Greater;
    }
    // |whole| <= 2^64 and has no fractional part, so the cast is exact.
    let whole = float.trunc();
    match integer.cmp(&(whole as i128)) {
        Ordering::Equal if float > whole => Ordering::Less,
        Ordering::Equal if float < whole => Ordering::Greater,
        ordering => ordering,
    }
}

fn eval_filter(f: &FilterExpr, node: &Value) -> bool {
    f.groups
        .iter()
        .any(|group| group.iter().all(|comp| eval_comp(comp, node)))
}

fn collect_descendants<'a>(node: &'a Value, key: Option<&str>, out: &mut Vec<&'a Value>) {
    match node {
        Value::Object(m) => {
            for (k, v) in m {
                let want_key = key.is_none_or(|k2| k == k2);
                if want_key {
                    out.push(v);
                }
                collect_descendants(v, key, out);
            }
        }
        Value::Array(items) => {
            for item in items {
                if key.is_none() {
                    out.push(item);
                }
                collect_descendants(item, key, out);
            }
        }
        _ => {}
    }
}

pub fn evaluate<'a>(expr: &JsonPathExpr, doc: &'a Value) -> Vec<&'a Value> {
    let mut nodes: Vec<&Value> = vec![doc];
    let mut seen: HashSet<*const Value> = HashSet::new();
    for step in &expr.steps {
        let mut next: Vec<&Value> = Vec::new();
        for node in &nodes {
            match step {
                Step::Child(name) => {
                    if let Value::Object(m) = node {
                        if let Some(v) = m.get(name) {
                            next.push(v);
                        }
                    }
                }
                Step::Wildcard => match node {
                    Value::Object(m) => next.extend(m.values()),
                    Value::Array(items) => next.extend(items.iter()),
                    _ => {}
                },
                Step::Index(i) => {
                    if let Value::Array(items) = node {
                        if let Some(v) = items.get(*i) {
                            next.push(v);
                        }
                    }
                }
                Step::Indices(idxs) => {
                    if let Value::Array(items) = node {
                        for i in idxs {
                            if let Some(v) = items.get(*i) {
                                next.push(v);
                            }
                        }
                    }
                }
                Step::Descendant(name) => collect_descendants(node, Some(name), &mut next),
                Step::DescendantWildcard => collect_descendants(node, None, &mut next),
                // RFC 9535 §2.3.5: a filter selects matching children of
                // arrays (elements) and of objects (member values).
                Step::Filter(f) => match node {
                    Value::Array(items) => {
                        next.extend(items.iter().filter(|item| eval_filter(f, item)))
                    }
                    Value::Object(m) => next.extend(m.values().filter(|v| eval_filter(f, v))),
                    _ => {}
                },
            }
        }
        nodes = next;
        if nodes.is_empty() {
            break;
        }
    }
    let mut dedup: Vec<&Value> = Vec::new();
    for n in nodes {
        if seen.insert(n as *const Value) {
            dedup.push(n);
        }
    }
    dedup
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn ev(expr: &str, doc: &Value) -> Vec<Value> {
        let p = parse(expr).unwrap();
        evaluate(&p, doc).into_iter().cloned().collect()
    }

    #[test]
    fn basic_paths() {
        let doc = json!({"messages": [{"role": "user", "content": "hi"}, {"role": "assistant", "content": "hello"}]});
        assert_eq!(ev("$.messages[0].role", &doc), vec![json!("user")]);
        assert_eq!(
            ev("$.messages[*].content", &doc),
            vec![json!("hi"), json!("hello")]
        );
        assert_eq!(ev("$.messages[1].role", &doc), vec![json!("assistant")]);
        assert_eq!(
            ev("$.messages[0,1].role", &doc),
            vec![json!("user"), json!("assistant")]
        );
    }

    #[test]
    fn wildcard_and_recursive() {
        let doc = json!({"a": {"b": {"c": 1}}, "b": 2});
        assert_eq!(ev("$..b", &doc), vec![json!({"c": 1}), json!(2)]);
        assert_eq!(ev("$.*", &doc), vec![json!({"b": {"c": 1}}), json!(2)]);
    }

    #[test]
    fn filters() {
        let doc = json!({"messages": [
            {"role": "user", "content": "hi"},
            {"role": "assistant", "content": "hello"},
            {"role": "user", "content": "bye"}
        ]});
        assert_eq!(
            ev("$.messages[?(@.role == 'user')].content", &doc),
            vec![json!("hi"), json!("bye")]
        );
        assert_eq!(
            ev("$.messages[?(@.role == 'assistant')].content", &doc),
            vec![json!("hello")]
        );
        // RFC 9535: a missing field is Nothing, and Nothing != 1 holds.
        assert_eq!(
            ev("$.messages[?(@.missing != 1)].content", &doc),
            vec![json!("hi"), json!("hello"), json!("bye")]
        );
        assert!(ev("$.messages[?(@.missing == 1)].content", &doc).is_empty());
        let num_doc = json!({"vals": [1, 5, 9, 12]});
        assert_eq!(ev("$.vals[?(@ > 5)]", &num_doc), vec![json!(9), json!(12)]);
    }

    #[test]
    fn filters_with_or() {
        let doc = json!({"messages": [
            {"id": "u", "role": "user"},
            {"id": "a", "role": "assistant"},
            {"id": "s", "role": "system"},
            {"id": "missing"}
        ]});
        assert_eq!(
            ev(
                "$.messages[?(@.role == 'user' || @.role == 'assistant')].id",
                &doc
            ),
            vec![json!("u"), json!("a")]
        );
        assert_eq!(
            ev(
                "$.messages[?(@.missing == true || @.role == 'system')].id",
                &doc
            ),
            vec![json!("s")]
        );
    }

    #[test]
    fn filters_and_bind_before_or() {
        let doc = json!({"messages": [
            {"id": "u0", "role": "user", "active": false, "tier": 1},
            {"id": "u1", "role": "user", "active": true, "tier": 1},
            {"id": "a0", "role": "assistant", "active": false, "tier": 1},
            {"id": "a1", "role": "assistant", "active": true, "tier": 1},
            {"id": "s0", "role": "system", "active": true, "tier": 1},
            {"id": "s1", "role": "system", "active": false, "tier": 2}
        ]});
        assert_eq!(
            ev(
                "$.messages[?(@.role == 'user' || @.role == 'assistant' && @.active == true)].id",
                &doc
            ),
            vec![json!("u0"), json!("u1"), json!("a1")]
        );
        assert_eq!(
            ev(
                "$.messages[?(@.role == 'user' && @.active == true || @.role == 'assistant')].id",
                &doc
            ),
            vec![json!("u1"), json!("a0"), json!("a1")]
        );
        assert_eq!(
            ev(
                "$.messages[?(@.role == 'user' && @.active == true || @.role == 'assistant' && @.active == true || @.role == 'system' && @.tier > 1)].id",
                &doc
            ),
            vec![json!("u1"), json!("a1"), json!("s1")]
        );
    }

    #[test]
    fn subtree_extraction() {
        let doc = json!({"a": {"x": [1, 2], "y": "z"}});
        assert_eq!(ev("$.a", &doc), vec![json!({"x": [1, 2], "y": "z"})]);
    }

    #[test]
    fn parse_errors() {
        assert!(parse("$.[]").is_err());
        assert!(parse("$.a[?()]").is_err());
        assert!(parse("$.a[abc]").is_err());
        assert!(parse("$..").is_err());
        assert!(parse("$...name").is_err());
    }

    #[test]
    fn recursive_wildcard_includes_every_descendant_in_source_order() {
        let doc = json!({"a": {"b": 1}, "array": [2, {"c": 3}, null], "last": false});
        assert_eq!(
            ev("$..*", &doc),
            vec![
                json!({"b": 1}),
                json!(1),
                json!([2, {"c": 3}, null]),
                json!(2),
                json!({"c": 3}),
                json!(3),
                Value::Null,
                json!(false),
            ]
        );
        assert_eq!(
            ev("$..*", &json!([1, [2, 3], {"last": 4}])),
            vec![
                json!(1),
                json!([2, 3]),
                json!(2),
                json!(3),
                json!({"last": 4}),
                json!(4)
            ]
        );
        assert!(ev("$..*", &json!(5)).is_empty());
    }

    #[test]
    fn numeric_filter_equality_matches_integer_and_float_representations() {
        let doc = json!({"values": [1, 1.0, 1.5, -1, -1.0, 0, -0.0]});
        assert_eq!(ev("$.values[?(@ == 1)]", &doc), vec![json!(1), json!(1.0)]);
        assert_eq!(
            ev("$.values[?(@ == -1)]", &doc),
            vec![json!(-1), json!(-1.0)]
        );
        assert_eq!(ev("$.values[?(@ == 0)]", &doc), vec![json!(0), json!(-0.0)]);
    }

    #[test]
    fn numeric_equality_does_not_round_large_integers_or_saturate_float_casts() {
        assert!(values_equal(&json!(u64::MAX), &json!(u64::MAX)));
        assert!(!values_equal(&json!(u64::MAX), &json!(u64::MAX - 1)));
        assert!(!values_equal(
            &json!(9_007_199_254_740_993u64),
            &json!(9_007_199_254_740_992.0)
        ));
        assert!(!values_equal(
            &json!(u64::MAX),
            &json!(18_446_744_073_709_551_616.0)
        ));
        assert!(!values_equal(
            &json!(i64::MAX),
            &json!(9_223_372_036_854_775_808.0)
        ));
        assert!(values_equal(
            &json!(i64::MIN),
            &json!(-9_223_372_036_854_775_808.0)
        ));
        assert!(values_equal(
            &json!(9_223_372_036_854_775_808u64),
            &json!(9_223_372_036_854_775_808.0)
        ));
        assert!(!values_equal(&json!(-1), &json!(u64::MAX)));
    }

    fn ids(expr: &str, doc: &Value) -> Vec<i64> {
        ev(expr, doc).iter().map(|v| v.as_i64().unwrap()).collect()
    }

    #[test]
    fn comparisons_with_a_missing_path_follow_rfc9535_nothing_semantics() {
        let doc = json!({"r": [{"id": 1, "v": 1}, {"id": 2}, {"id": 3, "v": null}]});
        let cases: &[(&str, &[i64])] = &[
            ("@.v == 1", &[1]),
            ("@.v != 1", &[2, 3]),
            ("1 != @.v", &[2, 3]),
            ("@.v < 5", &[1]),
            ("@.v <= 5", &[1]),
            ("@.v > 0", &[1]),
            ("@.v >= 0", &[1]),
            ("5 > @.v", &[1]),
            // Nothing is not null.
            ("@.v == null", &[3]),
            ("@.v != null", &[1, 2]),
            // Both sides Nothing: equal, never less.
            ("@.x == @.y", &[1, 2, 3]),
            ("@.x != @.y", &[]),
            ("@.x <= @.y", &[1, 2, 3]),
            ("@.x >= @.y", &[1, 2, 3]),
            ("@.x < @.y", &[]),
            ("@.x > @.y", &[]),
            ("@.v == @.x", &[2]),
            ("@.v != @.x", &[1, 3]),
            ("@.id.deeper != 1", &[1, 2, 3]),
        ];
        for (filter, expected) in cases {
            assert_eq!(
                ids(&format!("$.r[?({filter})].id"), &doc),
                expected.to_vec(),
                "{filter}"
            );
        }
    }

    #[test]
    fn filters_select_matching_object_member_values_in_source_order() {
        let doc = json!({"obj": {"z": {"v": 3}, "x": {"v": 1}, "y": {"v": 2}}, "s": 5});
        assert_eq!(
            ev("$.obj[?(@.v > 1)]", &doc),
            vec![json!({"v": 3}), json!({"v": 2})]
        );
        assert_eq!(ev("$.obj[?(@.v > 1)].v", &doc), vec![json!(3), json!(2)]);
        assert_eq!(ev("$.obj.x[?(@ == 1)]", &doc), vec![json!(1)]);
        assert_eq!(
            ev("$.obj[?(@.v != 1)]", &doc),
            vec![json!({"v": 3}), json!({"v": 2})]
        );
        // Scalars have no children to filter.
        assert!(ev("$.s[?(@ == 5)]", &doc).is_empty());
        // The root object is filtered like any other object.
        assert_eq!(ev("$[?(@ == 5)]", &doc), vec![json!(5)]);
    }

    #[test]
    fn decimal_and_exponent_literals_compare_exactly() {
        let doc =
            json!({"n": [1, 1.5, 2, 2.25, -2.25, 1000, 0.025, 9_007_199_254_740_993u64, -0.0]});
        let cases: &[(&str, Value)] = &[
            ("@ > 1.5", json!([2, 2.25, 1000, 9_007_199_254_740_993u64])),
            (
                "@ >= 1.5",
                json!([1.5, 2, 2.25, 1000, 9_007_199_254_740_993u64]),
            ),
            ("@ == 1.5", json!([1.5])),
            ("@ < -2", json!([-2.25])),
            ("@ == -2.25", json!([-2.25])),
            ("@ == 1e3", json!([1000])),
            ("@ == 1E3", json!([1000])),
            ("@ == 1e+3", json!([1000])),
            ("@ == 1.0e3", json!([1000])),
            ("@ == 2.5E-2", json!([0.025])),
            ("@ == 2.5e-2", json!([0.025])),
            ("@ == 2.0", json!([2])),
            ("@ == -0", json!([-0.0])),
            ("@ == 0.0", json!([-0.0])),
            ("@ == 007", json!([])),
            ("@ == 0010", json!([])),
            ("@ == 01.5", json!([1.5])),
            // An integer is never rounded to f64: 2^53 + 1 > 2^53.
            ("@ > 9007199254740992.0", json!([9_007_199_254_740_993u64])),
            ("@ == 9007199254740992.0", json!([])),
            ("@ == 9007199254740993", json!([9_007_199_254_740_993u64])),
            ("@ < 99999999999999999999.0", doc["n"].clone()),
            ("@ < 1e20", doc["n"].clone()),
        ];
        for (filter, expected) in cases {
            assert_eq!(
                Value::Array(ev(&format!("$.n[?({filter})]"), &doc)),
                *expected,
                "{filter}"
            );
        }
        assert_eq!(ev("$.n[?(@ == 1000)]", &doc), ev("$.n[?(@ == 1e3)]", &doc));

        for bad in [
            "1.", ".5", "1e", "1e+", "+1", "-", "--1", "1.5.5", "1e400", "-1e400",
        ] {
            assert!(
                parse(&format!("$.n[?(@ == {bad})]")).is_err(),
                "'{bad}' must be rejected"
            );
        }
        assert!(
            parse("$.n[?(@ == 1e400)]")
                .unwrap_err()
                .contains("out of range")
        );
    }

    #[test]
    fn exact_number_order_between_integers_and_floats() {
        let n = |v: Value| match v {
            Value::Number(n) => n,
            _ => unreachable!(),
        };
        let cmp = |a: Value, b: Value| number_cmp(&n(a), &n(b));
        assert_eq!(
            cmp(
                json!(9_007_199_254_740_993u64),
                json!(9_007_199_254_740_992.0)
            ),
            Ordering::Greater
        );
        assert_eq!(
            cmp(
                json!(9_007_199_254_740_992.0),
                json!(9_007_199_254_740_993u64)
            ),
            Ordering::Less
        );
        assert_eq!(
            cmp(json!(u64::MAX), json!(18_446_744_073_709_551_616.0)),
            Ordering::Less
        );
        assert_eq!(
            cmp(json!(i64::MIN), json!(-9_223_372_036_854_775_808.0)),
            Ordering::Equal
        );
        assert_eq!(cmp(json!(i64::MIN), json!(-1e300)), Ordering::Greater);
        assert_eq!(cmp(json!(-2), json!(-2.5)), Ordering::Greater);
        assert_eq!(cmp(json!(-3), json!(-2.5)), Ordering::Less);
        assert_eq!(cmp(json!(2), json!(2.5)), Ordering::Less);
        assert_eq!(cmp(json!(0), json!(-0.0)), Ordering::Equal);
        assert_eq!(cmp(json!(-1), json!(u64::MAX)), Ordering::Less);
        assert_eq!(cmp(json!(1.5), json!(2.25)), Ordering::Less);
    }

    #[test]
    fn number_literals_that_would_be_silently_rounded_are_rejected() {
        // Integers outside [-2^63, 2^64) would become rounded doubles, and a
        // non-zero literal that underflows would become 0.
        for (bad, message) in [
            ("-9223372036854775809", "outside the exact integer range"),
            ("18446744073709551616", "outside the exact integer range"),
            ("99999999999999999999", "outside the exact integer range"),
            ("-099999999999999999999", "outside the exact integer range"),
            ("1e-400", "would round to 0"),
            ("-1e-400", "would round to 0"),
            ("0.0001e-325", "would round to 0"),
            ("1e-99999999999999999999", "would round to 0"),
        ] {
            let error = parse(&format!("$.n[?(@ == {bad})]")).unwrap_err();
            assert!(error.contains(message), "{bad}: {error}");
            assert!(error.contains(&format!("'{bad}'")), "{bad}: {error}");
        }

        let doc = json!({"n": [0, i64::MIN, u64::MAX, -1, 5e-324, 1e20]});
        let cases: &[(&str, Value)] = &[
            // The exact integer domain ends at i64::MIN and u64::MAX.
            ("@ == -9223372036854775808", json!([i64::MIN])),
            ("@ == 18446744073709551615", json!([u64::MAX])),
            ("@ == 0018446744073709551615", json!([u64::MAX])),
            // Zero literals stay zero whatever their exponent.
            ("@ == -0", json!([0])),
            ("@ == 0e-400", json!([0])),
            ("@ == -0.000e-99999", json!([0])),
            // The smallest subnormal double does not underflow.
            ("@ > 0 && @ < 1e-323", json!([5e-324])),
            // A rounded double is written explicitly with a fraction or exponent.
            ("@ == 99999999999999999999.0", json!([1e20])),
            ("@ == 1e20", json!([1e20])),
            // ...and is rounded like a JSON double: this one becomes -2^63.
            ("@ < -9223372036854775809.0", json!([])),
            ("@ == -9223372036854775809.0", json!([i64::MIN])),
        ];
        for (filter, expected) in cases {
            assert_eq!(
                Value::Array(ev(&format!("$.n[?({filter})]"), &doc)),
                *expected,
                "{filter}"
            );
        }
    }

    #[test]
    fn string_literal_escapes_follow_rfc9535() {
        let doc = json!({
            "it's": 1, "a\\b": 2, "A": 3, "say \"hi\"": 4, "😀": 5,
            "C:\\new": 6, "tab\tkey": 7, "s/": 8,
            "list": [{"n": "O'Brien"}, {"n": "a\\b"}, {"n": "A"}, {"n": "say \"hi\""}]
        });
        let cases: &[(&str, Value)] = &[
            (r"$['it\'s']", json!([1])),
            (r#"$["it's"]"#, json!([1])),
            (r"$['a\\b']", json!([2])),
            (r"$['\u0041']", json!([3])),
            (r"$['\u0041' ]", json!([3])),
            (r#"$["say \"hi\""]"#, json!([4])),
            (r#"$['say "hi"']"#, json!([4])),
            (r"$['\uD83D\uDE00']", json!([5])),
            (r"$['\ud83d\ude00']", json!([5])),
            (r"$['C:\\new']", json!([6])),
            // `\n` is a newline per RFC 9535, not a backslash and an `n`.
            (r"$['C:\new']", json!([])),
            (r"$['tab\tkey']", json!([7])),
            (r"$['s\/']", json!([8])),
            (r"$.list[?(@.n == 'O\'Brien')].n", json!(["O'Brien"])),
            (r"$.list[?(@.n == 'a\\b')].n", json!(["a\\b"])),
            (r"$.list[?(@.n == '\u0041')].n", json!(["A"])),
            (r#"$.list[?(@.n == "say \"hi\"")].n"#, json!(["say \"hi\""])),
            (r"$.list[?(@['\u006E'] == 'A')].n", json!(["A"])),
        ];
        for (expr, expected) in cases {
            assert_eq!(Value::Array(ev(expr, &doc)), *expected, "{expr}");
        }

        for bad in [
            r"$['\x']",
            r"$['\U0041']",
            r"$['\u004']",
            r"$['\u00G1']",
            r"$['\uD83D']",
            r"$['\uD83Dx']",
            r"$['\uD83D\u0041']",
            r"$['\uDE00']",
            r#"$['\"']"#,
            r#"$["\'"]"#,
            r"$['unterminated",
            r"$['a\",
            r"$.list[?(@.n == 'a\d')]",
            r"$.list[?(@['\q'] == 1)]",
        ] {
            assert!(parse(bad).is_err(), "{bad} must be rejected");
        }
        assert!(parse(r"$['\x']").unwrap_err().contains("invalid escape"));
        assert!(
            parse(r#"$["\'"]"#)
                .unwrap_err()
                .contains("only allowed inside single-quoted")
        );
        assert!(parse(r"$['\uD83D']").unwrap_err().contains("surrogate"));
    }

    #[test]
    fn equality_compares_arrays_and_objects_deeply() {
        let doc = json!({"items": [
            {"id": 1, "a": [1, 2], "b": [1, 2]},
            {"id": 2, "a": [1, 2], "b": [2, 1]},
            {"id": 3, "a": {"k": 1, "j": [2]}, "b": {"j": [2.0], "k": 1}},
            {"id": 4, "a": {"k": 1}, "b": {"k": 1, "x": null}},
            {"id": 5, "a": [], "b": {}},
            {"id": 6, "a": true, "b": true}
        ]});
        assert_eq!(ids("$.items[?(@.a == @.b)].id", &doc), vec![1, 3, 6]);
        assert_eq!(ids("$.items[?(@.a != @.b)].id", &doc), vec![2, 4, 5]);
        // `<=` is `<` or `==`; arrays, objects and booleans are not ordered.
        assert_eq!(ids("$.items[?(@.a <= @.b)].id", &doc), vec![1, 3, 6]);
        assert!(ids("$.items[?(@.a < @.b)].id", &doc).is_empty());
    }
}
