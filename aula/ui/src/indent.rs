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
    // On an empty line forward_to_line_end() would jump to the next line's end.
    if !end.ends_line() && !end.forward_to_line_end() {
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

/// Replace the leading whitespace of `line` with `want` spaces. Like Emacs
/// `indent-line-to`: a cursor inside the indentation lands on the first
/// non-blank char, a cursor after it keeps its place in the text.
fn set_line_indent(buffer: &SourceBuffer, line: i32, want: usize) {
    let have = leading_ws(&line_text(buffer, line));
    let Some(start) = buffer.iter_at_line(line) else {
        return;
    };
    let cursor = buffer.iter_at_mark(&buffer.get_insert());
    let cursor_col = (cursor.line() == line).then(|| cursor.line_offset() as usize);
    let has_selection = buffer.has_selection();
    if have != want {
        let mut ws_end = start.clone();
        ws_end.forward_chars(have as i32);
        buffer.delete(&mut start.clone(), &mut ws_end);
        let mut ins = buffer.iter_at_line(line).unwrap_or(start);
        buffer.insert(&mut ins, &" ".repeat(want));
    }
    if let (Some(col), false) = (cursor_col, has_selection) {
        let col = if col <= have { want } else { col - have + want };
        if let Some(it) = buffer.iter_at_line_offset(line, col as i32) {
            buffer.place_cursor(&it);
        }
    }
}

pub fn indent_line(buffer: &SourceBuffer, line: i32) {
    if line < 0 || line >= buffer.line_count() {
        return;
    }
    let prev: Vec<String> = (0..line).map(|l| line_text(buffer, l)).collect();
    let prev_refs: Vec<&str> = prev.iter().map(String::as_str).collect();
    let want = want_indent(&prev_refs, &line_text(buffer, line));
    set_line_indent(buffer, line, want);
}

pub fn indent_current_line(buffer: &SourceBuffer) {
    let insert = buffer.iter_at_mark(&buffer.get_insert());
    buffer.begin_user_action();
    indent_line(buffer, insert.line());
    buffer.end_user_action();
}

/// First and last line touched by the selection. A selection ending at
/// column 0 does not include that last line (as when selecting whole lines).
fn selection_lines(buffer: &SourceBuffer) -> Option<(i32, i32)> {
    let (a, b) = buffer.selection_bounds()?;
    let last = if b.starts_line() && b.line() > a.line() {
        b.line() - 1
    } else {
        b.line()
    };
    Some((a.line(), last))
}

/// Lines to shift with Tab / Shift+Tab: the selection, if it spans more than
/// one line or covers a whole line. `None` for no selection or a selection
/// inside a single line.
pub fn selected_full_lines(buffer: &SourceBuffer) -> Option<(i32, i32)> {
    let (a, b) = buffer.selection_bounds()?;
    let (first, last) = selection_lines(buffer)?;
    let whole_line = a.starts_line() && (b.ends_line() || b.line() > a.line());
    (last > first || whole_line).then_some((first, last))
}

/// Emacs `indent-region` (C-M-\): reindent every non-blank line of the
/// selection, or the current line without a selection.
pub fn indent_region_or_line(buffer: &SourceBuffer) {
    let Some((first, last)) = selection_lines(buffer) else {
        indent_current_line(buffer);
        return;
    };
    buffer.begin_user_action();
    for line in first..=last {
        if !line_is_blank(buffer, line) {
            indent_line(buffer, line);
        }
    }
    select_lines(buffer, first, last);
    buffer.end_user_action();
}

/// Emacs `indent-rigidly`: move lines `first..=last` right (`delta > 0`) or
/// left by `delta` columns. Blank lines are left alone.
pub fn shift_lines(buffer: &SourceBuffer, first: i32, last: i32, delta: i32) {
    buffer.begin_user_action();
    for line in first..=last {
        if line_is_blank(buffer, line) {
            continue;
        }
        let have = leading_ws(&line_text(buffer, line)) as i32;
        set_line_indent(buffer, line, (have + delta).max(0) as usize);
    }
    select_lines(buffer, first, last);
    buffer.end_user_action();
}

fn select_lines(buffer: &SourceBuffer, first: i32, last: i32) {
    let Some(start) = buffer.iter_at_line(first) else {
        return;
    };
    let end = buffer
        .iter_at_line(last + 1)
        .unwrap_or_else(|| buffer.end_iter());
    buffer.select_range(&end, &start);
}

/// Tab: shift the selected lines, else auto-indent the current line.
pub fn tab(buffer: &SourceBuffer) {
    match selected_full_lines(buffer) {
        Some((first, last)) => shift_lines(buffer, first, last, OFFSET as i32),
        None => indent_current_line(buffer),
    }
}

/// Shift+Tab: shift the selected lines (or the current line) left.
pub fn backtab(buffer: &SourceBuffer) {
    if let Some((first, last)) = selected_full_lines(buffer) {
        shift_lines(buffer, first, last, -(OFFSET as i32));
        return;
    }
    let line = buffer.iter_at_mark(&buffer.get_insert()).line();
    let have = leading_ws(&line_text(buffer, line));
    buffer.begin_user_action();
    set_line_indent(buffer, line, have.saturating_sub(OFFSET));
    buffer.end_user_action();
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

#[cfg(test)]
mod buffer_tests {
    use super::*;

    fn buf(text: &str) -> Option<SourceBuffer> {
        if !gtk::is_initialized() && gtk::init().is_err() {
            return None; // no display
        }
        let b = SourceBuffer::new(None::<&gtk::TextTagTable>);
        b.set_text(text);
        Some(b)
    }

    fn text(b: &SourceBuffer) -> String {
        let (s, e) = b.bounds();
        b.text(&s, &e, false).to_string()
    }

    fn cursor(b: &SourceBuffer, line: i32, col: i32) {
        b.place_cursor(&b.iter_at_line_offset(line, col).unwrap());
    }

    fn select(b: &SourceBuffer, l1: i32, c1: i32, l2: i32, c2: i32) {
        let a = b.iter_at_line_offset(l1, c1).unwrap();
        let z = b.iter_at_line_offset(l2, c2).unwrap();
        b.select_range(&z, &a);
    }

    // One test: GTK objects must stay on the thread that initialized GTK.
    #[test]
    fn tab_backtab_and_region() {
        let Some(b) = buf("se a {\nx = 1\n}\n") else {
            return;
        };

        // Tab without selection auto-indents; cursor in the indentation
        // moves to the first char.
        cursor(&b, 1, 0);
        tab(&b);
        assert_eq!(text(&b), "se a {\n    x = 1\n}\n");
        assert_eq!(b.iter_at_mark(&b.get_insert()).line_offset(), 4);
        // Again: no change (Emacs Tab is idempotent), cursor after the text
        // keeps its place.
        cursor(&b, 1, 9);
        tab(&b);
        assert_eq!(text(&b), "se a {\n    x = 1\n}\n");
        assert_eq!(b.iter_at_mark(&b.get_insert()).line_offset(), 9);

        // Shift+Tab without selection unindents the current line.
        backtab(&b);
        assert_eq!(text(&b), "se a {\nx = 1\n}\n");
        assert_eq!(b.iter_at_mark(&b.get_insert()).line_offset(), 5);

        // Selection inside one line: Tab auto-indents instead of shifting.
        select(&b, 1, 1, 1, 3);
        tab(&b);
        assert_eq!(text(&b), "se a {\n    x = 1\n}\n");

        // Full lines selected (ending at column 0 of the next line): shift.
        select(&b, 0, 0, 2, 0);
        tab(&b);
        assert_eq!(text(&b), "    se a {\n        x = 1\n}\n");
        tab(&b);
        assert_eq!(text(&b), "        se a {\n            x = 1\n}\n");
        backtab(&b);
        backtab(&b);
        backtab(&b);
        assert_eq!(text(&b), "se a {\nx = 1\n}\n");
        let (s, e) = b.selection_bounds().unwrap();
        assert_eq!((s.line(), s.line_offset(), e.line(), e.line_offset()), (0, 0, 2, 0));

        // Ctrl+Alt+\ reindents the region, skipping blank lines.
        b.set_text("se a {\n      x\n\nse b {\ny\n  }\n}\n");
        select(&b, 0, 0, 7, 0);
        indent_region_or_line(&b);
        assert_eq!(text(&b), "se a {\n    x\n\n    se b {\n        y\n    }\n}\n");
    }
}
