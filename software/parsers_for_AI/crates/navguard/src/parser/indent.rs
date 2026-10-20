//! Indentation-based symbol ranges: Python, and `...:` blocks in the
//! generic mode.

use super::lexer::LexLine;
use super::{py_class_re, py_def_re, RawSym};

/// Indentation width; a tab advances to the next multiple of 8 (as Python does).
pub(super) fn indent_width(line: &str) -> usize {
    let mut width = 0;
    for c in line.chars() {
        match c {
            ' ' => width += 1,
            '\t' => width = (width / 8 + 1) * 8,
            _ => break,
        }
    }
    width
}

/// Python: `def`, `async def` and `class` at any depth. A block ends before
/// the next logical line indented at or below its header; its range ends at
/// the last code line (or deeper-indented comment) of the body. Lines inside
/// brackets, multi-line strings or after a backslash continuation are not
/// logical line starts, so multi-line signatures and docstrings at column 0
/// do not end a block.
pub(super) fn scan_python(lines: &[&str], lexed: &[LexLine]) -> Vec<RawSym> {
    let n = lines.len();
    let mut logical = vec![false; n];
    let mut depth: i32 = 0;
    let mut continued = false;
    for i in 0..n {
        logical[i] = lexed[i].code_start && depth == 0 && !continued;
        for &c in &lexed[i].masked {
            match c {
                '(' | '[' | '{' => depth += 1,
                ')' | ']' | '}' => depth = (depth - 1).max(0),
                _ => {}
            }
        }
        let masked: String = lexed[i].masked.iter().collect();
        continued = masked.trim_end().ends_with('\\');
    }
    let blank: Vec<bool> = lines.iter().map(|l| l.trim().is_empty()).collect();
    let comment_only: Vec<bool> = (0..n)
        .map(|i| {
            !blank[i]
                && lexed[i].code_start
                && lexed[i].masked.iter().all(|c| c.is_whitespace())
                && lines[i].trim_start().starts_with('#')
        })
        .collect();
    let indents: Vec<usize> = lines.iter().map(|l| indent_width(l)).collect();

    let close = |start: usize, base: usize, before: usize| -> usize {
        (start + 1..before)
            .rev()
            .find(|&k| !blank[k] && (!comment_only[k] || indents[k] > base))
            .unwrap_or(start)
    };

    let mut syms: Vec<RawSym> = Vec::new();
    let mut stack: Vec<(usize, usize)> = Vec::new(); // (symbol index, header indent)
    for i in 0..n {
        if !logical[i] || blank[i] || comment_only[i] {
            continue;
        }
        let ind = indents[i];
        while let Some(&(s, base)) = stack.last() {
            if ind > base {
                break;
            }
            syms[s].end = close(syms[s].start, base, i);
            stack.pop();
        }
        let t = lines[i].trim_start();
        let found = if let Some(c) = py_def_re().captures(t) {
            Some((c[1].to_string(), "fn"))
        } else {
            py_class_re()
                .captures(t)
                .map(|c| (c[1].to_string(), "class"))
        };
        if let Some((name, kind)) = found {
            syms.push(RawSym {
                label: lines[i].trim().to_string(),
                name,
                kind,
                start: i,
                end: i,
            });
            stack.push((syms.len() - 1, ind));
        }
    }
    while let Some((s, base)) = stack.pop() {
        syms[s].end = close(syms[s].start, base, n);
    }
    syms
}

/// Generic `header:` blocks: the range ends at the last non-blank line
/// indented deeper than the header, before the first non-blank line that is
/// not.
pub(super) fn indent_extent(lines: &[&str], start: usize) -> usize {
    let base = indent_width(lines[start]);
    let mut end = start;
    for (j, line) in lines.iter().enumerate().skip(start + 1) {
        if line.trim().is_empty() {
            continue;
        }
        if indent_width(line) <= base {
            break;
        }
        end = j;
    }
    end
}
