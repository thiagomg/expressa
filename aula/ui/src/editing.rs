//! Line commands: comment/uncomment (Ctrl+/), duplicate (Ctrl+D) and move
//! (Alt+↑/↓). Each one is a single undo step.

use gtk::prelude::*;
use sourceview5::Buffer as SourceBuffer;

fn line_text(buffer: &SourceBuffer, line: i32) -> String {
    let Some(start) = buffer.iter_at_line(line) else {
        return String::new();
    };
    let mut end = start.clone();
    if !end.ends_line() {
        end.forward_to_line_end();
    }
    buffer.text(&start, &end, false).to_string()
}

/// Lines covered by the selection, or the cursor line. A selection that ends
/// at column 0 does not include that line.
fn target_lines(buffer: &SourceBuffer) -> (i32, i32) {
    match buffer.selection_bounds() {
        Some((a, b)) => {
            let last = if b.starts_line() && b.line() > a.line() {
                b.line() - 1
            } else {
                b.line()
            };
            (a.line(), last)
        }
        None => {
            let l = buffer.iter_at_mark(&buffer.get_insert()).line();
            (l, l)
        }
    }
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

fn leading_ws(s: &str) -> usize {
    s.chars().take_while(|c| *c == ' ' || *c == '\t').count()
}

/// Ctrl+/: comment the lines with `// ` at their common indentation, or
/// uncomment them when every non-blank line already starts with `//`.
pub fn toggle_comment(buffer: &SourceBuffer) {
    let (first, last) = target_lines(buffer);
    let had_selection = buffer.has_selection();
    let lines: Vec<(i32, String)> = (first..=last).map(|l| (l, line_text(buffer, l))).collect();
    let code: Vec<&(i32, String)> = lines.iter().filter(|(_, t)| !t.trim().is_empty()).collect();
    let targets: Vec<&(i32, String)> = if code.is_empty() { lines.iter().collect() } else { code };
    let uncomment = !targets.is_empty() && targets.iter().all(|(_, t)| t.trim_start().starts_with("//"));
    buffer.begin_user_action();
    if uncomment {
        for (l, t) in &targets {
            let col = leading_ws(t) as i32;
            let drop = if t.trim_start().starts_with("// ") { 3 } else { 2 };
            if let (Some(mut a), Some(mut b)) = (
                buffer.iter_at_line_offset(*l, col),
                buffer.iter_at_line_offset(*l, col + drop),
            ) {
                buffer.delete(&mut a, &mut b);
            }
        }
    } else {
        let col = targets.iter().map(|(_, t)| leading_ws(t)).min().unwrap_or(0) as i32;
        for (l, _) in &targets {
            if let Some(mut at) = buffer.iter_at_line_offset(*l, col) {
                buffer.insert(&mut at, "// ");
            }
        }
    }
    if had_selection {
        select_lines(buffer, first, last);
    }
    buffer.end_user_action();
}

/// Ctrl+D: copy the lines below themselves; the cursor (or selection) moves
/// to the copy.
pub fn duplicate_lines(buffer: &SourceBuffer) {
    let (first, last) = target_lines(buffer);
    let had_selection = buffer.has_selection();
    let cursor = buffer.iter_at_mark(&buffer.get_insert());
    let (cur_line, cur_col) = (cursor.line(), cursor.line_offset());
    let block: Vec<String> = (first..=last).map(|l| line_text(buffer, l)).collect();
    let count = block.len() as i32;
    buffer.begin_user_action();
    let mut end = buffer.iter_at_line(last).unwrap();
    if !end.ends_line() {
        end.forward_to_line_end();
    }
    buffer.insert(&mut end, &format!("\n{}", block.join("\n")));
    if had_selection {
        select_lines(buffer, first + count, last + count);
    } else if let Some(it) = buffer.iter_at_line_offset(cur_line + count, cur_col) {
        buffer.place_cursor(&it);
    }
    buffer.end_user_action();
}

/// Alt+↑ / Alt+↓: swap the lines with the one above or below.
pub fn move_lines(buffer: &SourceBuffer, up: bool) {
    let (first, last) = target_lines(buffer);
    let n = buffer.line_count();
    // The empty line after a final newline is not a line to move past.
    let real_last = if line_text(buffer, n - 1).is_empty() && n > 1 { n - 2 } else { n - 1 };
    if (up && first == 0) || (!up && last >= real_last) {
        return;
    }
    let had_selection = buffer.has_selection();
    let cursor = buffer.iter_at_mark(&buffer.get_insert());
    let (cur_line, cur_col) = (cursor.line(), cursor.line_offset());
    let (lo, hi) = if up { (first - 1, last) } else { (first, last + 1) };
    let mut lines: Vec<String> = (lo..=hi).map(|l| line_text(buffer, l)).collect();
    if up {
        lines.rotate_left(1);
    } else {
        lines.rotate_right(1);
    }
    let delta = if up { -1 } else { 1 };
    buffer.begin_user_action();
    let mut a = buffer.iter_at_line(lo).unwrap();
    let mut b = buffer.iter_at_line(hi).unwrap();
    if !b.ends_line() {
        b.forward_to_line_end();
    }
    buffer.delete(&mut a, &mut b);
    let mut at = buffer.iter_at_line(lo).unwrap();
    buffer.insert(&mut at, &lines.join("\n"));
    if had_selection {
        select_lines(buffer, first + delta, last + delta);
    } else if let Some(it) = buffer.iter_at_line_offset(cur_line + delta, cur_col) {
        buffer.place_cursor(&it);
    }
    buffer.end_user_action();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn buf(text: &str) -> Option<SourceBuffer> {
        if !gtk::is_initialized() && gtk::init().is_err() {
            return None; // no display
        }
        let b = SourceBuffer::new(None::<&gtk::TextTagTable>);
        b.begin_irreversible_action();
        b.set_text(text);
        b.end_irreversible_action();
        Some(b)
    }

    fn text(b: &SourceBuffer) -> String {
        let (s, e) = b.bounds();
        b.text(&s, &e, false).to_string()
    }

    fn cursor(b: &SourceBuffer) -> (i32, i32) {
        let it = b.iter_at_mark(&b.get_insert());
        (it.line(), it.line_offset())
    }

    fn put(b: &SourceBuffer, line: i32, col: i32) {
        b.place_cursor(&b.iter_at_line_offset(line, col).unwrap());
    }

    fn select(b: &SourceBuffer, l1: i32, c1: i32, l2: i32, c2: i32) {
        b.select_range(
            &b.iter_at_line_offset(l2, c2).unwrap(),
            &b.iter_at_line_offset(l1, c1).unwrap(),
        );
    }

    // One test: GTK objects must stay on the thread that initialized GTK.
    #[test]
    fn line_commands() {
        crate::gtk_test::run(|| {
            let Some(b) = buf("se a {\n    x = 1\n\n    y = 2\n}\n") else {
                return;
            };
    
            // Comment a block at its common indentation, skipping the blank line.
            select(&b, 1, 0, 4, 0);
            toggle_comment(&b);
            assert_eq!(text(&b), "se a {\n    // x = 1\n\n    // y = 2\n}\n");
            // Again: uncomment.
            toggle_comment(&b);
            assert_eq!(text(&b), "se a {\n    x = 1\n\n    y = 2\n}\n");
            // One undo step each.
            b.undo();
            assert_eq!(text(&b), "se a {\n    // x = 1\n\n    // y = 2\n}\n");
            b.undo();
            assert_eq!(text(&b), "se a {\n    x = 1\n\n    y = 2\n}\n");
    
            // Cursor line only; `//x` (no space) also uncomments.
            b.set_text("a\n//b\n");
            put(&b, 0, 1);
            toggle_comment(&b);
            assert_eq!(text(&b), "// a\n//b\n");
            put(&b, 1, 0);
            toggle_comment(&b);
            assert_eq!(text(&b), "// a\nb\n");
    
            // Duplicate the cursor line; the cursor goes to the copy.
            b.set_text("um\ndois\n");
            put(&b, 0, 1);
            duplicate_lines(&b);
            assert_eq!(text(&b), "um\num\ndois\n");
            assert_eq!(cursor(&b), (1, 1));
            b.undo();
            assert_eq!(text(&b), "um\ndois\n");
            // Duplicate a selected block, last line without a newline.
            b.set_text("a\nb");
            select(&b, 0, 0, 1, 1);
            duplicate_lines(&b);
            assert_eq!(text(&b), "a\nb\na\nb");
    
            // Move down and up, keeping the column.
            b.set_text("1\n22\n3\n");
            put(&b, 1, 1);
            move_lines(&b, false);
            assert_eq!(text(&b), "1\n3\n22\n");
            assert_eq!(cursor(&b), (2, 1));
            // Already the last real line: nothing happens.
            move_lines(&b, false);
            assert_eq!(text(&b), "1\n3\n22\n");
            move_lines(&b, true);
            move_lines(&b, true);
            assert_eq!(text(&b), "22\n1\n3\n");
            move_lines(&b, true);
            assert_eq!(text(&b), "22\n1\n3\n");
            b.undo();
            assert_eq!(text(&b), "1\n22\n3\n");
            // A selected block moves as one.
            select(&b, 0, 0, 2, 0);
            move_lines(&b, false);
            assert_eq!(text(&b), "3\n1\n22\n");
            let (s, e) = b.selection_bounds().unwrap();
            assert_eq!((s.line(), e.line()), (1, 3));
        });
    }
}
