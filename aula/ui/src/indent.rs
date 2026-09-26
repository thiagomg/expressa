//! Indentation and `inicio`/`fim` matching for the Aula editor.

use gtk::prelude::*;
use sourceview5::Buffer as SourceBuffer;

const OFFSET: i32 = 4;

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

fn opens_closes(code: &str) -> (i32, i32) {
    let mut opens = 0;
    let mut closes = 0;
    let bytes = code.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if code[i..].starts_with("inicio") && word_at(code, i, "inicio") {
            opens += 1;
            i += "inicio".len();
            continue;
        }
        if code[i..].starts_with("início") && word_at(code, i, "início") {
            opens += 1;
            i += "início".len();
            continue;
        }
        if code[i..].starts_with("fim") && word_at(code, i, "fim") {
            closes += 1;
            i += "fim".len();
            continue;
        }
        match code[i..].chars().next() {
            Some('{') => {
                opens += 1;
                i += 1;
            }
            Some('}') => {
                closes += 1;
                i += 1;
            }
            Some(c) => i += c.len_utf8(),
            None => break,
        }
    }
    (opens, closes)
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

pub fn line_starts_with_closer(buffer: &SourceBuffer, line: i32) -> bool {
    let t = strip_line_code(&line_text(buffer, line));
    starts_with_closer(&t)
}

fn starts_with_closer(code: &str) -> bool {
    let t = code.trim_start();
    t.starts_with('}') || (t.starts_with("fim") && word_at(t, 0, "fim"))
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
    let mut depth = 0i32;
    for l in 0..line {
        let code = strip_line_code(&line_text(buffer, l));
        let (o, c) = opens_closes(&code);
        depth = (depth + o - c).max(0);
    }
    let cur = strip_line_code(&line_text(buffer, line));
    if starts_with_closer(&cur) {
        depth = (depth - 1).max(0);
    }
    let want = (depth * OFFSET) as usize;
    let raw = line_text(buffer, line);
    let have = raw.chars().take_while(|c| *c == ' ' || *c == '\t').count();
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
    let have = raw.chars().take_while(|c| *c == ' ' || *c == '\t').count();
    let drop = have.min(OFFSET as usize);
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
