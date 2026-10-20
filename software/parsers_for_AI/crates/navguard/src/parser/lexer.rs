//! Masks comments and string literals so the symbol indexer can count
//! braces, brackets and semicolons without being fooled by `"{"` or `// }`.
//!
//! This is a best-effort, line-oriented lexer. It understands the comment
//! and string syntax of the supported language families (including
//! multi-line strings and nested block comments) but it does not parse.

/// Comment/string syntax family of a source file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Syntax {
    Rust,
    Python,
    Js,
    Go,
    /// C and C++ (preprocessor lines, raw strings, digit separators).
    CLike,
    CSharp,
    Java,
    Kotlin,
    /// Unknown file types: nothing is masked.
    Plain,
}

/// One physical line after masking.
pub(crate) struct LexLine {
    /// The line with every comment and string-literal character replaced
    /// by a space. Same number of chars as the raw line.
    pub masked: Vec<char>,
    /// True when the line starts in plain code, i.e. not inside a block
    /// comment, a multi-line string or a continued preprocessor directive.
    pub code_start: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum State {
    Code,
    Block(u32),
    Str(Quote),
    Preproc,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Quote {
    Double,
    Single,
    /// JavaScript template literal.
    Template,
    /// Go raw string (backticks, no escapes).
    GoRaw,
    /// Rust raw string with the given number of `#`.
    RustRaw(usize),
    /// Triple-quoted string (Python, Kotlin, Java text blocks, C# raw).
    Triple(char),
    /// C# verbatim string `@"..."` (`""` escapes a quote).
    Verbatim,
    /// C++ raw string `R"delim( ... )delim"`.
    CppRaw(String),
}

fn is_ident(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

fn starts(chars: &[char], i: usize, pat: &str) -> bool {
    pat.chars()
        .enumerate()
        .all(|(k, p)| chars.get(i + k) == Some(&p))
}

fn blank(masked: &mut [char], from: usize, to: usize) {
    let to = to.min(masked.len());
    for c in masked.iter_mut().take(to).skip(from) {
        *c = ' ';
    }
}

/// Lexes `lines` (without line terminators) and returns one [`LexLine`] per line.
pub(crate) fn lex(lines: &[&str], syntax: Syntax) -> Vec<LexLine> {
    let mut out = Vec::with_capacity(lines.len());
    let mut state = State::Code;
    // Last significant code char, for the JavaScript regex-literal heuristic.
    let mut prev_sig: Option<char> = None;
    let nested_comments = matches!(syntax, Syntax::Rust | Syntax::Kotlin);

    for raw in lines {
        let chars: Vec<char> = raw.chars().collect();
        if syntax == Syntax::Plain {
            out.push(LexLine {
                masked: chars,
                code_start: true,
            });
            continue;
        }
        let code_start = state == State::Code;
        let mut masked = chars.clone();
        let n = chars.len();
        if state == State::Code
            && matches!(syntax, Syntax::CLike | Syntax::CSharp)
            && raw.trim_start().starts_with('#')
        {
            state = State::Preproc;
        }

        let mut i = 0;
        while i < n {
            match state.clone() {
                State::Preproc => {
                    blank(&mut masked, i, n);
                    i = n;
                }
                State::Block(depth) => {
                    if starts(&chars, i, "*/") {
                        blank(&mut masked, i, i + 2);
                        i += 2;
                        state = if depth > 1 {
                            State::Block(depth - 1)
                        } else {
                            State::Code
                        };
                    } else if nested_comments && starts(&chars, i, "/*") {
                        blank(&mut masked, i, i + 2);
                        i += 2;
                        state = State::Block(depth + 1);
                    } else {
                        masked[i] = ' ';
                        i += 1;
                    }
                }
                State::Str(quote) => match close_string(&chars, i, &quote) {
                    Some(end) => {
                        blank(&mut masked, i, end);
                        i = end;
                        state = State::Code;
                        prev_sig = Some('"');
                    }
                    None => {
                        blank(&mut masked, i, n);
                        i = n;
                    }
                },
                State::Code => {
                    let c = chars[i];
                    if syntax == Syntax::Python {
                        if c == '#' {
                            blank(&mut masked, i, n);
                            break;
                        }
                    } else if starts(&chars, i, "//") {
                        blank(&mut masked, i, n);
                        break;
                    } else if starts(&chars, i, "/*") {
                        blank(&mut masked, i, i + 2);
                        i += 2;
                        state = State::Block(1);
                        continue;
                    }
                    if let Some((quote, open_len)) = open_string(&chars, i, syntax) {
                        blank(&mut masked, i, i + open_len);
                        i += open_len;
                        state = State::Str(quote);
                        continue;
                    }
                    if c == '\'' {
                        if let Some(len) = char_literal(&chars, i, syntax) {
                            blank(&mut masked, i, i + len);
                            i += len;
                            prev_sig = Some('\'');
                            continue;
                        }
                    }
                    if syntax == Syntax::Js
                        && c == '/'
                        && (regex_allowed(prev_sig) || after_keyword(&chars, i))
                    {
                        if let Some(end) = js_regex_end(&chars, i) {
                            blank(&mut masked, i, end);
                            i = end;
                            prev_sig = Some('/');
                            continue;
                        }
                    }
                    if !c.is_whitespace() {
                        prev_sig = Some(c);
                    }
                    i += 1;
                }
            }
        }

        // End of line: single-line constructs end here.
        let continued = raw.trim_end().ends_with('\\');
        match &state {
            State::Preproc if !continued => state = State::Code,
            State::Str(Quote::Double | Quote::Single) if syntax != Syntax::Rust && !continued => {
                state = State::Code
            }
            _ => {}
        }
        out.push(LexLine { masked, code_start });
    }
    out
}

/// Detects a string opening at `i`; returns the quote kind and the length
/// of the opening delimiter.
fn open_string(chars: &[char], i: usize, syntax: Syntax) -> Option<(Quote, usize)> {
    let c = chars[i];
    let prev_ident = i > 0 && is_ident(chars[i - 1]);
    match syntax {
        Syntax::Python => {
            if starts(chars, i, "\"\"\"") {
                Some((Quote::Triple('"'), 3))
            } else if starts(chars, i, "'''") {
                Some((Quote::Triple('\''), 3))
            } else if c == '"' {
                Some((Quote::Double, 1))
            } else if c == '\'' {
                Some((Quote::Single, 1))
            } else {
                None
            }
        }
        Syntax::Rust => {
            if (c == 'r' || (c == 'b' && chars.get(i + 1) == Some(&'r'))) && !prev_ident {
                let mut j = if c == 'r' { i + 1 } else { i + 2 };
                let mut hashes = 0;
                while chars.get(j) == Some(&'#') {
                    hashes += 1;
                    j += 1;
                }
                if chars.get(j) == Some(&'"') {
                    return Some((Quote::RustRaw(hashes), j + 1 - i));
                }
                None
            } else if c == '"' {
                Some((Quote::Double, 1))
            } else {
                None
            }
        }
        Syntax::Js => match c {
            '"' => Some((Quote::Double, 1)),
            '\'' => Some((Quote::Single, 1)),
            '`' => Some((Quote::Template, 1)),
            _ => None,
        },
        Syntax::Go => match c {
            '"' => Some((Quote::Double, 1)),
            '`' => Some((Quote::GoRaw, 1)),
            _ => None,
        },
        Syntax::CLike => {
            if c == 'R' && chars.get(i + 1) == Some(&'"') && raw_prefix_ok(chars, i) {
                let mut j = i + 2;
                let mut delim = String::new();
                while let Some(&d) = chars.get(j) {
                    if d == '(' {
                        return Some((Quote::CppRaw(delim), j + 1 - i));
                    }
                    if d.is_whitespace() || d == ')' || d == '\\' || delim.len() > 16 {
                        break;
                    }
                    delim.push(d);
                    j += 1;
                }
                Some((Quote::Double, 2))
            } else if c == '"' {
                Some((Quote::Double, 1))
            } else {
                None
            }
        }
        Syntax::CSharp => {
            if starts(chars, i, "@\"") {
                Some((Quote::Verbatim, 2))
            } else if starts(chars, i, "$@\"") || starts(chars, i, "@$\"") {
                Some((Quote::Verbatim, 3))
            } else if starts(chars, i, "\"\"\"") {
                Some((Quote::Triple('"'), 3))
            } else if c == '"' {
                Some((Quote::Double, 1))
            } else {
                None
            }
        }
        Syntax::Java | Syntax::Kotlin => {
            if starts(chars, i, "\"\"\"") {
                Some((Quote::Triple('"'), 3))
            } else if c == '"' {
                Some((Quote::Double, 1))
            } else {
                None
            }
        }
        Syntax::Plain => None,
    }
}

/// `R"` starts a raw string when `R` is not part of a longer identifier,
/// allowing the encoding prefixes `u8R`, `uR`, `UR` and `LR`.
fn raw_prefix_ok(chars: &[char], i: usize) -> bool {
    ["u8", "u", "U", "L", ""].iter().any(|prefix| {
        let len = prefix.chars().count();
        i >= len
            && chars[i - len..i].iter().copied().eq(prefix.chars())
            && (i == len || !is_ident(chars[i - len - 1]))
    })
}

/// Scans a string body from `i`; returns the index just past the closing
/// delimiter, or `None` when the string continues past the end of the line.
fn close_string(chars: &[char], mut i: usize, quote: &Quote) -> Option<usize> {
    let n = chars.len();
    while i < n {
        let c = chars[i];
        match quote {
            Quote::Double | Quote::Single | Quote::Template => {
                let close = match quote {
                    Quote::Double => '"',
                    Quote::Single => '\'',
                    _ => '`',
                };
                if c == '\\' {
                    i += 2;
                    continue;
                }
                if c == close {
                    return Some(i + 1);
                }
            }
            Quote::GoRaw => {
                if c == '`' {
                    return Some(i + 1);
                }
            }
            Quote::RustRaw(hashes) => {
                if c == '"' && (1..=*hashes).all(|k| chars.get(i + k) == Some(&'#')) {
                    return Some(i + 1 + hashes);
                }
            }
            Quote::Triple(q) => {
                if c == '\\' {
                    i += 2;
                    continue;
                }
                if c == *q && chars.get(i + 1) == Some(q) && chars.get(i + 2) == Some(q) {
                    return Some(i + 3);
                }
            }
            Quote::Verbatim => {
                if c == '"' {
                    if chars.get(i + 1) == Some(&'"') {
                        i += 2;
                        continue;
                    }
                    return Some(i + 1);
                }
            }
            Quote::CppRaw(delim) => {
                if c == ')' {
                    let mut j = i + 1;
                    let mut ok = true;
                    for d in delim.chars() {
                        if chars.get(j) != Some(&d) {
                            ok = false;
                            break;
                        }
                        j += 1;
                    }
                    if ok && chars.get(j) == Some(&'"') {
                        return Some(j + 1);
                    }
                }
            }
        }
        i += 1;
    }
    None
}

/// Length of a character literal starting at `i` (`'x'`, `'\n'`, `'\u{1F600}'`),
/// or `None` for Rust lifetimes, C++ digit separators and stray quotes.
fn char_literal(chars: &[char], i: usize, syntax: Syntax) -> Option<usize> {
    match syntax {
        Syntax::Rust
        | Syntax::Go
        | Syntax::CLike
        | Syntax::CSharp
        | Syntax::Java
        | Syntax::Kotlin => {}
        _ => return None,
    }
    if syntax == Syntax::CLike
        && i > 0
        && chars[i - 1].is_ascii_digit()
        && chars.get(i + 1).is_some_and(|c| c.is_ascii_hexdigit())
    {
        return None;
    }
    match chars.get(i + 1) {
        Some('\\') => {
            let mut j = i + 3;
            while j < chars.len() && j < i + 14 {
                if chars[j] == '\'' {
                    return Some(j + 1 - i);
                }
                j += 1;
            }
            None
        }
        Some(_) if chars.get(i + 2) == Some(&'\'') => Some(3),
        _ => None,
    }
}

fn regex_allowed(prev: Option<char>) -> bool {
    match prev {
        None => true,
        Some(c) => "(,=:[!&|?{};+-*%~^<>".contains(c),
    }
}

/// `return /x/` and friends: a regex literal may follow these keywords.
fn after_keyword(chars: &[char], i: usize) -> bool {
    let before: String = chars[..i].iter().collect();
    let before = before.trim_end();
    let word_start = before
        .char_indices()
        .rev()
        .take_while(|(_, c)| is_ident(*c))
        .last()
        .map_or(before.len(), |(p, _)| p);
    matches!(
        &before[word_start..],
        "return"
            | "typeof"
            | "case"
            | "in"
            | "of"
            | "void"
            | "yield"
            | "await"
            | "delete"
            | "throw"
            | "new"
            | "else"
            | "do"
    ) && (word_start == 0 || !before[..word_start].ends_with(['.', '$']))
}

fn js_regex_end(chars: &[char], i: usize) -> Option<usize> {
    if matches!(chars.get(i + 1), Some('/') | Some('*') | None) {
        return None;
    }
    let mut j = i + 1;
    let mut in_class = false;
    while j < chars.len() {
        match chars[j] {
            '\\' => {
                j += 2;
                continue;
            }
            '[' => in_class = true,
            ']' => in_class = false,
            '/' if !in_class => return Some(j + 1),
            _ => {}
        }
        j += 1;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn masked(src: &str, syntax: Syntax) -> Vec<String> {
        let lines: Vec<&str> = src.lines().collect();
        lex(&lines, syntax)
            .into_iter()
            .map(|l| {
                l.masked
                    .into_iter()
                    .collect::<String>()
                    .trim_end()
                    .to_string()
            })
            .collect()
    }

    #[test]
    fn masks_strings_comments_and_chars_in_rust() {
        let out = masked(
            "let s = \"{\"; // }\nlet c = '{'; let l: &'a str = r#\"}\"#;\n/* { /* } */ { */ x",
            Syntax::Rust,
        );
        assert_eq!(out[0], "let s =    ;");
        assert_eq!(out[1], "let c =    ; let l: &'a str =       ;");
        assert_eq!(out[2].trim(), "x");
    }

    #[test]
    fn tracks_multiline_strings_and_comments() {
        let lines = ["x = \"\"\"", "def fake():", "\"\"\"", "y = 1"];
        let lexed = lex(&lines, Syntax::Python);
        assert!(lexed[0].code_start);
        assert!(!lexed[1].code_start);
        assert!(!lexed[2].code_start);
        assert!(lexed[3].code_start);

        let lines = ["/*", "fn fake() {", "*/", "fn real() {}"];
        let lexed = lex(&lines, Syntax::Rust);
        assert!(!lexed[1].code_start);
        assert!(lexed[3].code_start);
    }

    #[test]
    fn handles_js_templates_regexes_and_cpp_specials() {
        let out = masked("const r = /[{]/g; const t = `${a}{`;", Syntax::Js);
        assert!(!out[0].contains('{'));
        let out = masked(
            "#define OPEN {\nauto s = R\"x({)x\"; int n = 1'000; char c = '}';",
            Syntax::CLike,
        );
        assert_eq!(out[0].trim(), "");
        assert!(!out[1].contains('{') && !out[1].contains('}'));
        assert!(out[1].contains("1'000"));
        let out = masked("var s = @\"a\"\"{\"; var t = \"}\";", Syntax::CSharp);
        assert!(!out[0].contains('{') && !out[0].contains('}'));
    }
}
