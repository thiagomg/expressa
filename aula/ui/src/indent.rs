//! Indentation and `inicio`/`fim` matching for the Aula editor.
//!
//! Body indent is the **leading whitespace of the line that opened the
//! block** plus 4, not `n_tokens * 4`. So `se a == 1 {` opens one level
//! from column 0, not two.

use gtk::prelude::*;
use sourceview5::Buffer as SourceBuffer;

const OFFSET: usize = 4;

fn line_text(buffer: &SourceBuffer, line: i32) -> String {
    let Some(start) = buffer.iter_at_line(line) else {
        return String::new();
    };
    let mut end = start.clone();
    if !end.forward_to_line_end() {
        end = buffer.end_iter();
    }
    buffer.text(&start, &end, false).to_string()
}

fn strip_line_code(s: &str) -> String {
    let mut out = String::new();
    let mut in_str = false;
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if in_str {
            out.push(c);
            if c == '\\' {
                if let Some(n) = chars.next() {
                    out.push(n);
                }
            } else if c == '"' {
                in_str = false;
            }
            continue;
        }
        if c == '"' {
            in_str = true;
            out.push(c);
            continue;
        }
        if c == '/' && chars.peek() == Some(&'/') {
            break;
        }
        out.push(c);
    }
    out
}

fn leading_ws(s: &str) -> usize {
    s.chars().take_while(|c| *c == ' ' || *c == '\t').count()
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Ev {
    Open,
    Close,
}

fn events(code: &str) -> Vec<Ev> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < code.len() {
        if code[i..].starts_with("inicio") && word_at(code, i, "inicio") {
            out.push(Ev::Open);
            i += "inicio".len();
            continue;
        }
        if code[i..].starts_with("início") && word_at(code, i, "início") {
            out.push(Ev::Open);
            i += "início".len();
            continue;
        }
        if code[i..].starts_with("fim") && word_at(code, i, "fim") {
            out.push(Ev::Close);
            i += "fim".len();
            continue;
        }
        match code[i..].chars().next() {
            Some('{') => {
                out.push(Ev::Open);
                i += 1;
            }
            Some('}') => {
                out.push(Ev::Close);
                i += 1;
            }
            Some(c) => i += c.len_utf8(),
            None => break,
        }
    }
    out
}

fn word_at(s: &str, i: usize, word: &str) -> bool {
    let before_ok = i == 0
        || !s[..i]
            .chars()
            .last()
            .is_some_and(|c| c.is_alphanumeric() || c == '_');
    let after = i + word.len();
    let after_ok = after >= s.len()
        || !s[after..]
            .chars()
            .next()
            .is_some_and(|c| c.is_alphanumeric() || c == '_');
    before_ok && after_ok
}

fn starts_with_closer(code: &str) -> bool {
    let t = code.trim_start();
    t.starts_with('}') || (t.starts_with("fim") && word_at(t, 0, "fim"))
}

/// Stack of leading indents of lines that still have an open block.
fn opener_stack(prev_lines: &[&str]) -> Vec<usize> {
    let mut stack = Vec::new();
    for raw in prev_lines {
        let lead = leading_ws(raw);
        let code = strip_line_code(raw);
        let evs = events(&code);
        let mut opened_this_line = false;
        for ev in evs {
            match ev {
                Ev::Close => {
                    stack.pop();
                    opened_this_line = false;
                }
                Ev::Open => {
                    if !opened_this_line {
                        stack.push(lead);
                        opened_this_line = true;
                    }
                }
            }
        }
    }
    stack
}

fn want_indent(prev_lines: &[&str], current: &str) -> usize {
    let stack = opener_stack(prev_lines);
    let code = strip_line_code(current);
    match stack.last().copied() {
        None => 0,
        Some(base) if starts_with_closer(&code) => base,
        Some(base) => base + OFFSET,
    }
}

pub fn line_is_blank(buffer: &SourceBuffer, line: i32) -> bool {
    line_text(buffer, line).trim().is_empty()
}

pub fn line_starts_with_closer(buffer: &SourceBuffer, line: i32) -> bool {
    let t = strip_line_code(&line_text(buffer, line));
    starts_with_closer(&t)
}

/// Reindent `line` only if it is a closer (`fim` / `}`).
pub fn indent_if_closer(buffer: &SourceBuffer, line: i32) {
    if line_starts_with_closer(buffer, line) {
        indent_line(buffer, line);
    }
}

pub fn indent_line(buffer: &SourceBuffer, line: i32) {
    let n = buffer.line_count();
    if line < 0 || line >= n {
        return;
    }
    let prev: Vec<String> = (0..line).map(|l| line_text(buffer, l)).collect();
    let prev_refs: Vec<&str> = prev.iter().map(String::as_str).collect();
    let current = line_text(buffer, line);
    let want = want_indent(&prev_refs, &current);
    let have = leading_ws(&current);
    if have == want {
        return;
    }
    let Some(start) = buffer.iter_at_line(line) else {
        return;
    };
    let mut ws_end = start.clone();
    for _ in 0..have {
        if !ws_end.forward_char() {
            break;
        }
    }
    buffer.delete(&mut start.clone(), &mut ws_end);
    let mut ins = buffer.iter_at_line(line).unwrap_or(start);
    buffer.insert(&mut ins, &" ".repeat(want));
}

pub fn indent_current_line(buffer: &SourceBuffer) {
    let insert = buffer.iter_at_mark(&buffer.get_insert());
    indent_line(buffer, insert.line());
}

pub fn unindent_current_line(buffer: &SourceBuffer) {
    let insert = buffer.iter_at_mark(&buffer.get_insert());
    let line = insert.line();
    let raw = line_text(buffer, line);
    let have = leading_ws(&raw);
    let drop = have.min(OFFSET);
    if drop == 0 {
        return;
    }
    let Some(start) = buffer.iter_at_line(line) else {
        return;
    };
    let mut end = start.clone();
    for _ in 0..drop {
        end.forward_char();
    }
    buffer.delete(&mut start.clone(), &mut end);
}

fn opens_closes(code: &str) -> (i32, i32) {
    let mut o = 0;
    let mut c = 0;
    for ev in events(code) {
        match ev {
            Ev::Open => o += 1,
            Ev::Close => c += 1,
        }
    }
    (o, c)
}

/// Returns (open_line, close_line) 0-based if the cursor sits on inicio/fim/{/}.
pub fn matching_block(buffer: &SourceBuffer) -> Option<(i32, i32)> {
    let insert = buffer.iter_at_mark(&buffer.get_insert());
    let line = insert.line();
    let text = strip_line_code(&line_text(buffer, line));
    let trimmed = text.trim_start();
    let n = buffer.line_count();
    if trimmed.starts_with("inicio") || trimmed.starts_with("início") || trimmed.starts_with('{') {
        let mut depth = 0;
        for l in line..n {
            let code = strip_line_code(&line_text(buffer, l));
            let (o, c) = opens_closes(&code);
            depth += o - c;
            if l > line && depth <= 0 {
                return Some((line, l));
            }
            if l == line && starts_with_closer(&code) {
                continue;
            }
        }
        None
    } else if starts_with_closer(trimmed) {
        let mut depth = 0;
        for l in (0..=line).rev() {
            let code = strip_line_code(&line_text(buffer, l));
            let (o, c) = opens_closes(&code);
            depth += c - o;
            if l < line && depth <= 0 {
                return Some((l, line));
            }
        }
        None
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn brace_on_same_line_as_se_is_one_level() {
        assert_eq!(want_indent(&["se a == 1 {"], "x"), 4);
        assert_eq!(want_indent(&["se a == 1 {"], "}"), 0);
        assert_eq!(want_indent(&["x = funcao(a) {"], "retorne a"), 4);
    }

    #[test]
    fn top_level_statement_does_not_indent() {
        assert_eq!(want_indent(&["escreva(1)"], "escreva(2)"), 0);
        assert_eq!(want_indent(&[], "escreva(1)"), 0);
        assert_eq!(want_indent(&["x = 1"], ""), 0);
    }

    #[test]
    fn after_closer_next_line_is_not_indented() {
        assert_eq!(want_indent(&["se a {", "    x", "}"], ""), 0);
        assert_eq!(want_indent(&["se a {", "}"], "escreva(1)"), 0);
        assert_eq!(want_indent(&["se a {", "    }"], ""), 0);
    }

    #[test]
    fn nested_uses_opener_line_indent() {
        assert_eq!(want_indent(&["se a {", "    se b {"], "x"), 8);
        assert_eq!(want_indent(&["se a {", "    se b {", "        x"], "}"), 4);
        assert_eq!(
            want_indent(&["    se a == 1 {"], "x"),
            8,
            "opener already indented by 4"
        );
    }

    #[test]
    fn close_then_open_on_same_line() {
        assert_eq!(
            want_indent(&["se a {", "    x", "} se b {"], "y"),
            4
        );
    }
}
