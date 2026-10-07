//! The output pane: program output with ANSI colors and a small terminal
//! emulation (`cls()` / `casa()`), plus line input for `leia` typed right
//! in the pane, like a terminal.

use std::cell::RefCell;
use std::sync::mpsc::Sender;

use gtk::prelude::*;
use gtk::{TextTag, TextView};

pub struct Console {
    pub view: TextView,
    err_tag: TextTag,
    error_link_tag: TextTag,
    /// Text before the input field: cannot be edited while typing.
    readonly_tag: TextTag,
    input_tag: TextTag,
    screen: RefCell<Screen>,
    /// While a `leia` waits: where the answer starts and where it goes.
    input: RefCell<Option<(gtk::TextMark, Sender<String>)>>,
}

impl Console {
    pub fn new() -> Self {
        let view = TextView::builder()
            .editable(false)
            .monospace(true)
            .cursor_visible(false)
            .wrap_mode(gtk::WrapMode::WordChar)
            .build();
        view.add_css_class("aula-code");
        let table = view.buffer().tag_table();
        let err_tag = TextTag::new(Some("stderr"));
        err_tag.set_foreground(Some("#e53935"));
        table.add(&err_tag);
        add_ansi_tags(table.clone());
        let error_link_tag = TextTag::new(Some("error-link"));
        error_link_tag.set_foreground(Some("#e53935"));
        error_link_tag.set_underline(gtk::pango::Underline::Single);
        table.add(&error_link_tag);
        let readonly_tag = TextTag::new(Some("readonly"));
        readonly_tag.set_editable(false);
        table.add(&readonly_tag);
        let input_tag = TextTag::new(Some("input"));
        input_tag.set_weight(700);
        table.add(&input_tag);
        let console = Self {
            view,
            err_tag,
            error_link_tag,
            readonly_tag,
            input_tag,
            screen: RefCell::new(Screen::default()),
            input: RefCell::new(None),
        };
        console
    }

    fn buffer(&self) -> gtk::TextBuffer {
        self.view.buffer()
    }

    pub fn clear(&self) {
        self.cancel_input();
        *self.screen.borrow_mut() = Screen::default();
        self.buffer().set_text("");
    }

    pub fn stdout(&self, text: &str) {
        self.write(text, false);
    }

    pub fn stderr(&self, text: &str) {
        self.write(text, true);
    }

    fn write(&self, text: &str, stderr: bool) {
        let buf = self.buffer();
        let mut screen = self.screen.borrow_mut();
        if !stderr && (screen.row.is_some() || has_screen_code(text)) {
            write_screen(&buf, &mut screen, text);
            return;
        }
        let extra = if stderr { Some(&self.err_tag) } else { None };
        insert_ansi_text(&buf, text, extra);
        trim_output(&buf);
    }

    /// `erro: … em arquivo:linha`, red and underlined: clicking jumps there.
    pub fn error_link(&self, text: &str) {
        let buf = self.buffer();
        let mut start = buf.end_iter();
        let start_off = start.offset();
        buf.insert(&mut start, text);
        let a = buf.iter_at_offset(start_off);
        let b = buf.end_iter();
        buf.apply_tag(&self.error_link_tag, &a, &b);
    }

    /// `leia`: show the prompt and let the student type the answer in the
    /// pane. Enter sends it to `reply`.
    pub fn begin_input(&self, prompt: &str, reply: Sender<String>) {
        self.cancel_input();
        if !prompt.is_empty() {
            self.stdout(prompt);
            if !prompt.ends_with(' ') && !prompt.ends_with('\n') {
                self.stdout(" ");
            }
        }
        let buf = self.buffer();
        let (start, end) = buf.bounds();
        buf.apply_tag(&self.readonly_tag, &start, &end);
        let mark = buf.create_mark(None, &end, true);
        *self.input.borrow_mut() = Some((mark, reply));
        self.view.set_editable(true);
        self.view.set_cursor_visible(true);
        buf.place_cursor(&buf.end_iter());
        self.view.grab_focus();
        self.view
            .scroll_to_mark(&buf.get_insert(), 0.0, false, 0.0, 0.0);
    }

    pub fn is_reading(&self) -> bool {
        self.input.borrow().is_some()
    }

    /// Enter while reading: the typed text becomes part of the output.
    pub fn commit_input(&self) -> bool {
        let Some((mark, reply)) = self.input.borrow_mut().take() else {
            return false;
        };
        let buf = self.buffer();
        let start = buf.iter_at_mark(&mark);
        let end = buf.end_iter();
        let line = buf.text(&start, &end, false).to_string();
        buf.apply_tag(&self.input_tag, &start, &end);
        let mut end = buf.end_iter();
        buf.insert(&mut end, "\n");
        buf.delete_mark(&mark);
        self.view.set_editable(false);
        self.view.set_cursor_visible(false);
        // After cls/casa the terminal row continues below the answer.
        let mut screen = self.screen.borrow_mut();
        if screen.row.is_some() {
            screen.row = Some(buf.line_count() - 1);
            screen.partial.clear();
        }
        let _ = reply.send(line);
        true
    }

    /// The program ended (or Parar) while waiting for input.
    pub fn cancel_input(&self) {
        if let Some((mark, _)) = self.input.borrow_mut().take() {
            self.buffer().delete_mark(&mark);
        }
        self.view.set_editable(false);
        self.view.set_cursor_visible(false);
    }

    /// Keep typing at the end: a click elsewhere while reading moves back.
    pub fn keep_cursor_in_input(&self) {
        if let Some((mark, _)) = self.input.borrow().as_ref() {
            let buf = self.buffer();
            let at = buf.iter_at_mark(&buf.get_insert());
            if at.offset() < buf.iter_at_mark(mark).offset() {
                buf.place_cursor(&buf.end_iter());
            }
        }
    }

    /// Text of the output line at widget coordinates (for error links).
    pub fn line_at(&self, x: f64, y: f64) -> Option<String> {
        let (bx, by) =
            self.view
                .window_to_buffer_coords(gtk::TextWindowType::Text, x as i32, y as i32);
        let iter = self.view.iter_at_location(bx, by)?;
        let buf = self.buffer();
        let a = buf.iter_at_line(iter.line())?;
        let mut b = a.clone();
        if !b.ends_line() {
            b.forward_to_line_end();
        }
        Some(buf.text(&a, &b, false).to_string())
    }

    pub fn set_dark(&self, dark: bool) {
        let red = if dark { "#ff6e6e" } else { "#e53935" };
        self.err_tag.set_foreground(Some(red));
        self.error_link_tag.set_foreground(Some(red));
    }
}

/// `"erro: … em lib/a.lep:12"` -> `("lib/a.lep", 12)`.
pub fn parse_error_location(line: &str) -> Option<(String, u32)> {
    let em = line.rfind(" em ")?;
    let loc = line[em + 4..].trim();
    let (file, n) = loc.rsplit_once(':')?;
    Some((file.to_string(), n.trim().parse().ok()?))
}

/// Lines kept in the output pane. A program that never ends keeps only the
/// tail instead of growing the buffer without limit.
const MAX_OUTPUT_LINES: i32 = 5000;

/// Drops the oldest lines past [`MAX_OUTPUT_LINES`]; returns how many.
fn trim_output(buf: &gtk::TextBuffer) -> i32 {
    let extra = buf.line_count() - MAX_OUTPUT_LINES;
    if extra <= 0 {
        return 0;
    }
    let mut start = buf.start_iter();
    if let Some(mut end) = buf.iter_at_line(extra) {
        buf.delete(&mut start, &mut end);
    }
    extra
}

/// Cursor of the output pane after `cls()` / `casa()` / `quadro` / `escreva_em`.
/// `row: None` means plain appending (no screen codes seen in this run).
#[derive(Default)]
struct Screen {
    row: Option<i32>,
    /// 0-based visible column where the next character lands.
    col: i32,
    /// Column where `partial` started (0 = replace the whole line on flush).
    start_col: i32,
    /// Text of the current run not yet committed (may hold SGR codes).
    partial: String,
}

/// Terminal-like writing: cursor home goes back to the first line and each
/// new line overwrites the old one. `escreva_em` uses CUP (`\x1b[n;mH`) to
/// overlay from a column. `\x1b[J` erases from the cursor down (`quadro`).
fn write_screen(buf: &gtk::TextBuffer, screen: &mut Screen, text: &str) {
    let mut row = screen.row.unwrap_or_else(|| (buf.line_count() - 1).max(0));
    let mut col = screen.col;
    let mut start_col = screen.start_col;
    let mut rest = text;
    while let Some(c) = rest.chars().next() {
        if let Some((len, final_byte, params)) = parse_csi(rest) {
            let csi = &rest[..len];
            rest = &rest[len..];
            match final_byte {
                b'J' => {
                    flush_partial(buf, &mut row, &mut col, &mut start_col, screen);
                    if params == "2" {
                        buf.set_text("");
                        row = 0;
                        col = 0;
                        start_col = 0;
                    } else {
                        erase_down(buf, row, col);
                    }
                }
                b'K' => {
                    flush_partial(buf, &mut row, &mut col, &mut start_col, screen);
                    erase_line_from(buf, row, col);
                }
                b'H' | b'f' => {
                    flush_partial(buf, &mut row, &mut col, &mut start_col, screen);
                    let (r, c) = parse_cup(params);
                    row = r - 1;
                    col = c - 1;
                    start_col = col;
                    ensure_line(buf, row);
                }
                _ => {
                    // SGR stays in `partial` so insert_ansi can apply it.
                    screen.partial.push_str(csi);
                }
            }
            continue;
        }
        if c == '\r' {
            rest = &rest[1..];
        } else if c == '\n' {
            flush_partial(buf, &mut row, &mut col, &mut start_col, screen);
            row += 1;
            col = 0;
            start_col = 0;
            ensure_line(buf, row);
            row -= trim_output(buf);
            rest = &rest[1..];
        } else {
            screen.partial.push(c);
            rest = &rest[c.len_utf8()..];
        }
    }
    flush_partial(buf, &mut row, &mut col, &mut start_col, screen);
    screen.row = Some(row);
    screen.col = col;
    screen.start_col = start_col;
}

fn flush_partial(
    buf: &gtk::TextBuffer,
    row: &mut i32,
    col: &mut i32,
    start_col: &mut i32,
    screen: &mut Screen,
) {
    if screen.partial.is_empty() {
        return;
    }
    let content = std::mem::take(&mut screen.partial);
    let vis = visible_len(&content) as i32;
    if *start_col == 0 {
        replace_line(buf, *row, &content);
    } else {
        overlay_line(buf, *row, *start_col, &content);
    }
    *col = *start_col + vis;
    *start_col = *col;
}

fn visible_len(text: &str) -> usize {
    ansi_spans(text)
        .iter()
        .map(|(s, _)| s.chars().count())
        .sum()
}

fn parse_cup(params: &str) -> (i32, i32) {
    let mut row = 1i32;
    let mut col = 1i32;
    let mut parts = params.split(';');
    if let Some(p) = parts.next() {
        if !p.is_empty() {
            row = p.parse().unwrap_or(1);
        }
    }
    if let Some(p) = parts.next() {
        if !p.is_empty() {
            col = p.parse().unwrap_or(1);
        }
    }
    (row.max(1), col.max(1))
}

fn ensure_line(buf: &gtk::TextBuffer, row: i32) {
    while row >= buf.line_count() {
        let mut end = buf.end_iter();
        buf.insert(&mut end, "\n");
    }
}

fn line_range(buf: &gtk::TextBuffer, row: i32) -> Option<(i32, i32)> {
    let start = buf.iter_at_line(row)?;
    let mut end = start.clone();
    if !end.ends_line() {
        end.forward_to_line_end();
    }
    Some((start.offset(), end.offset()))
}

fn replace_line(buf: &gtk::TextBuffer, row: i32, content: &str) {
    ensure_line(buf, row);
    let Some((off, end_off)) = line_range(buf, row) else {
        return;
    };
    let mut start = buf.iter_at_offset(off);
    let mut end = buf.iter_at_offset(end_off);
    buf.delete(&mut start, &mut end);
    insert_ansi_at(buf, off, content, None);
}

fn overlay_line(buf: &gtk::TextBuffer, row: i32, col: i32, content: &str) {
    ensure_line(buf, row);
    let Some((line_start, line_end)) = line_range(buf, row) else {
        return;
    };
    let line_len = line_end - line_start;
    if col > line_len {
        let mut at = buf.iter_at_offset(line_start + line_len);
        buf.insert(&mut at, &" ".repeat((col - line_len) as usize));
    }
    let Some((line_start, line_end)) = line_range(buf, row) else {
        return;
    };
    let vis = visible_len(content) as i32;
    let at = line_start + col;
    let del = vis.min((line_end - at).max(0));
    if del > 0 {
        let mut a = buf.iter_at_offset(at);
        let mut b = buf.iter_at_offset(at + del);
        buf.delete(&mut a, &mut b);
    }
    insert_ansi_at(buf, at, content, None);
}

fn erase_line_from(buf: &gtk::TextBuffer, row: i32, col: i32) {
    let Some((line_start, line_end)) = line_range(buf, row) else {
        return;
    };
    let at = line_start + col.min(line_end - line_start);
    if at < line_end {
        let mut a = buf.iter_at_offset(at);
        let mut b = buf.iter_at_offset(line_end);
        buf.delete(&mut a, &mut b);
    }
}

fn erase_down(buf: &gtk::TextBuffer, row: i32, col: i32) {
    // After a trailing `\n`, the cursor is at column 0 of the next line.
    // Erasing from there must drop that empty line too, or a shorter
    // `quadro` leaves a blank row where the old frame used to continue.
    let from = if col <= 0 { row } else { row + 1 };
    if col > 0 {
        erase_line_from(buf, row, col);
    }
    if let Some(mut start) = buf.iter_at_line(from) {
        let mut end = buf.end_iter();
        buf.delete(&mut start, &mut end);
    }
}

const ANSI_FG: &[(&str, &str)] = &[
    ("ansi-fg-preto", "#212121"),
    ("ansi-fg-vermelho", "#c62828"),
    ("ansi-fg-verde", "#2e7d32"),
    ("ansi-fg-amarelo", "#f9a825"),
    ("ansi-fg-azul", "#1565c0"),
    ("ansi-fg-magenta", "#6a1b9a"),
    ("ansi-fg-ciano", "#00838f"),
    ("ansi-fg-branco", "#424242"),
];
const ANSI_BG: &[(&str, &str)] = &[
    ("ansi-bg-preto", "#212121"),
    ("ansi-bg-vermelho", "#ef9a9a"),
    ("ansi-bg-verde", "#a5d6a7"),
    ("ansi-bg-amarelo", "#fff59d"),
    ("ansi-bg-azul", "#90caf9"),
    ("ansi-bg-magenta", "#ce93d8"),
    ("ansi-bg-ciano", "#80deea"),
    ("ansi-bg-branco", "#eeeeee"),
];

fn add_ansi_tags(table: gtk::TextTagTable) {
    for (name, color) in ANSI_FG {
        let tag = TextTag::new(Some(name));
        tag.set_foreground(Some(color));
        table.add(&tag);
    }
    for (name, color) in ANSI_BG {
        let tag = TextTag::new(Some(name));
        tag.set_background(Some(color));
        table.add(&tag);
    }
    let bold = TextTag::new(Some("ansi-negrito"));
    bold.set_weight(700);
    table.add(&bold);
}

fn has_screen_code(text: &str) -> bool {
    let mut pos = 0;
    while pos < text.len() {
        let rest = &text[pos..];
        if let Some((len, final_byte, _)) = parse_csi(rest) {
            if matches!(final_byte, b'H' | b'f' | b'J' | b'K') {
                return true;
            }
            pos += len;
            continue;
        }
        pos += rest.chars().next().unwrap().len_utf8();
    }
    false
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct AnsiStyle {
    fg: Option<u8>,
    bg: Option<u8>,
    bold: bool,
}

fn ansi_fg_tag(code: u8) -> Option<&'static str> {
    Some(match code {
        30 => "ansi-fg-preto",
        31 => "ansi-fg-vermelho",
        32 => "ansi-fg-verde",
        33 => "ansi-fg-amarelo",
        34 => "ansi-fg-azul",
        35 => "ansi-fg-magenta",
        36 => "ansi-fg-ciano",
        37 => "ansi-fg-branco",
        _ => return None,
    })
}

fn ansi_bg_tag(code: u8) -> Option<&'static str> {
    Some(match code {
        40 => "ansi-bg-preto",
        41 => "ansi-bg-vermelho",
        42 => "ansi-bg-verde",
        43 => "ansi-bg-amarelo",
        44 => "ansi-bg-azul",
        45 => "ansi-bg-magenta",
        46 => "ansi-bg-ciano",
        47 => "ansi-bg-branco",
        _ => return None,
    })
}

fn apply_sgr(style: &mut AnsiStyle, params: &str) {
    if params.is_empty() {
        *style = AnsiStyle::default();
        return;
    }
    for p in params.split(';') {
        match p.parse::<u8>().unwrap_or(0) {
            0 => *style = AnsiStyle::default(),
            1 => style.bold = true,
            22 => style.bold = false,
            39 => style.fg = None,
            49 => style.bg = None,
            n @ 30..=37 => style.fg = Some(n),
            n @ 40..=47 => style.bg = Some(n),
            _ => {}
        }
    }
}

/// Visible spans after stripping CSI. SGR updates style; other CSI is skipped.
fn ansi_spans(text: &str) -> Vec<(String, AnsiStyle)> {
    let mut out = Vec::new();
    let mut style = AnsiStyle::default();
    let mut chunk = String::new();
    let mut pos = 0;
    let flush = |chunk: &mut String, style: AnsiStyle, out: &mut Vec<(String, AnsiStyle)>| {
        if !chunk.is_empty() {
            out.push((std::mem::take(chunk), style));
        }
    };
    while pos < text.len() {
        let rest = &text[pos..];
        if rest.as_bytes().first() == Some(&0x1b) {
            if let Some((len, final_byte, params)) = parse_csi(rest) {
                flush(&mut chunk, style, &mut out);
                if final_byte == b'm' {
                    apply_sgr(&mut style, params);
                }
                pos += len;
                continue;
            }
        }
        let ch = rest.chars().next().unwrap();
        chunk.push(ch);
        pos += ch.len_utf8();
    }
    flush(&mut chunk, style, &mut out);
    out
}

/// `\x1b[` + params + final byte (0x40..=0x7E). `params` is the inner string.
fn parse_csi(text: &str) -> Option<(usize, u8, &str)> {
    let bytes = text.as_bytes();
    if bytes.len() < 2 || bytes[0] != 0x1b || bytes[1] != b'[' {
        return None;
    }
    let mut i = 2;
    while i < bytes.len() && (0x30..=0x3f).contains(&bytes[i]) {
        i += 1;
    }
    while i < bytes.len() && (0x20..=0x2f).contains(&bytes[i]) {
        i += 1;
    }
    if i >= bytes.len() || !(0x40..=0x7e).contains(&bytes[i]) {
        return None;
    }
    let params = std::str::from_utf8(&bytes[2..i]).ok()?;
    Some((i + 1, bytes[i], params))
}

fn insert_ansi_text(buf: &gtk::TextBuffer, text: &str, extra: Option<&TextTag>) {
    insert_ansi_at(buf, buf.end_iter().offset(), text, extra);
}

/// Insert `text` at char `offset`, turning SGR codes into tags.
fn insert_ansi_at(buf: &gtk::TextBuffer, mut offset: i32, text: &str, extra: Option<&TextTag>) {
    let table = buf.tag_table();
    for (chunk, style) in ansi_spans(text) {
        let mut at = buf.iter_at_offset(offset);
        buf.insert(&mut at, &chunk);
        let a = buf.iter_at_offset(offset);
        offset += chunk.chars().count() as i32;
        let b = buf.iter_at_offset(offset);
        if let Some(tag) = extra {
            buf.apply_tag(tag, &a, &b);
        }
        if let Some(name) = style.fg.and_then(ansi_fg_tag) {
            if let Some(tag) = table.lookup(name) {
                buf.apply_tag(&tag, &a, &b);
            }
        }
        if let Some(name) = style.bg.and_then(ansi_bg_tag) {
            if let Some(tag) = table.lookup(name) {
                buf.apply_tag(&tag, &a, &b);
            }
        }
        if style.bold {
            if let Some(tag) = table.lookup("ansi-negrito") {
                buf.apply_tag(&tag, &a, &b);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_location() {
        assert_eq!(
            parse_error_location("erro: x em lib/a.lep:12"),
            Some(("lib/a.lep".into(), 12))
        );
        assert_eq!(
            parse_error_location("erro: divisão em main.lep:3\n"),
            Some(("main.lep".into(), 3))
        );
        assert_eq!(parse_error_location("nada aqui"), None);
    }

    fn screen_text(chunks: &[&str]) -> Option<String> {
        if !gtk::is_initialized() && gtk::init().is_err() {
            return None; // no display
        }
        let buf = gtk::TextBuffer::new(None);
        let mut screen = Screen::default();
        for c in chunks {
            write_screen(&buf, &mut screen, c);
        }
        let (s, e) = buf.bounds();
        Some(buf.text(&s, &e, false).to_string())
    }

    // One test: GTK objects must stay on the thread that initialized GTK.
    #[test]
    fn write_screen_like_a_terminal() {
        crate::gtk_test::run(|| {
            let Some(t) = screen_text(&["velho\n", "\x1b[2J\x1b[Hnovo\n"]) else {
                return;
            };
            assert_eq!(t, "novo\n", "cls clears then appends");
            // casa(): old frame stays until each line is overwritten.
            let t = screen_text(&["q1 a\nq1 b\n", "\x1b[H", "q2 a\n"]).unwrap();
            assert_eq!(t, "q2 a\nq1 b\n");
            let t = screen_text(&["q1 a\nq1 b\n", "\x1b[H", "q2 a\n", "q2 b\n"]).unwrap();
            assert_eq!(t, "q2 a\nq2 b\n");
            // Writes split across chunks, and SGR codes become tags, not text.
            let t = screen_text(&["\x1b[H", "\x1b[32m*", "\x1b[0m!\n"]).unwrap();
            assert_eq!(t, "*!\n");
            // After cls, plain lines keep appending.
            let t = screen_text(&["\x1b[2J\x1b[H", "a\n", "b\n"]).unwrap();
            assert_eq!(t, "a\nb\n");
            // quadro: home + shorter frame + erase down drops leftover lines.
            let t =
                screen_text(&["q1 a\nq1 b\nq1 c\n", "\x1b[H", "q2 a\nq2 b\n", "\x1b[J"]).unwrap();
            assert_eq!(t, "q2 a\nq2 b\n");
            // EL at end of a short line drops leftover to the right (old cargo output).
            let t = screen_text(&["cargo leftover here\n", "\x1b[Hok\x1b[K\n\x1b[J"]).unwrap();
            assert_eq!(t, "ok\n");
            let t = screen_text(&["\x1b[Ha\x1b[K\r\nb\x1b[K\r\n\x1b[J"]).unwrap();
            assert_eq!(t, "a\nb\n");
            // escreva_em: CUP + overlay from a column, EL clears the tail.
            let t = screen_text(&["AAAAAA\n", "\x1b[1;4HXY\x1b[K"]).unwrap();
            assert_eq!(t, "AAAXY\n");
        });
    }

    #[test]
    fn ansi_spans_splits_color_and_reset() {
        let spans = ansi_spans("\x1b[32mHP\x1b[0m!");
        assert_eq!(spans.len(), 2);
        assert_eq!(spans[0].0, "HP");
        assert_eq!(
            spans[0].1,
            AnsiStyle {
                fg: Some(32),
                bg: None,
                bold: false
            }
        );
        assert_eq!(spans[1].0, "!");
        assert_eq!(spans[1].1, AnsiStyle::default());
    }

    #[test]
    fn ansi_spans_frente_e_fundo() {
        let spans = ansi_spans("\x1b[37;41mGO\x1b[0m");
        assert_eq!(spans[0].0, "GO");
        assert_eq!(
            spans[0].1,
            AnsiStyle {
                fg: Some(37),
                bg: Some(41),
                bold: false
            }
        );
    }

    #[test]
    fn input_in_the_pane() {
        crate::gtk_test::run(|| {
            if !gtk::is_initialized() && gtk::init().is_err() {
                return;
            }
            let c = Console::new();
            c.stdout("olá\n");
            let (tx, rx) = std::sync::mpsc::channel();
            c.begin_input("Seu nome:", tx);
            assert!(c.is_reading());
            let buf = c.view.buffer();
            // The old text is protected; typing goes at the end.
            let mut inside = buf.iter_at_offset(2);
            assert!(!buf.insert_interactive(&mut inside, "X", true));
            buf.insert_interactive_at_cursor("Ana", true);
            assert!(c.commit_input());
            assert_eq!(rx.recv().unwrap(), "Ana");
            let (s, e) = buf.bounds();
            assert_eq!(buf.text(&s, &e, false), "olá\nSeu nome: Ana\n");
            assert!(!c.is_reading());
            assert!(!c.commit_input());
        });
    }
}
