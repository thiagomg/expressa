//! One open file (a tab): buffer with its own undo history, view,
//! breakpoints, search, block-match highlight and syntax check.

use std::cell::RefCell;
use std::collections::BTreeSet;
use std::rc::{Rc, Weak};

use gtk::glib;
use gtk::prelude::*;
use gtk::{Box as GtkBox, Button, Label, Orientation, ScrolledWindow, TextTag};
use sourceview5::prelude::*;
use sourceview5::{Buffer as SourceBuffer, LanguageManager, MarkAttributes, SearchContext, SearchSettings, StyleScheme, View as SourceView};

/// A syntax error found while typing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diag {
    /// 1-based.
    pub line: u32,
    pub message: String,
    /// Char offsets in the buffer.
    pub start: i32,
    pub end: i32,
}

/// Parse `text` like the interpreter does before running it.
pub fn diagnose(text: &str) -> Option<Diag> {
    let err = expressa::parser::parse(text).err()?;
    let to_chars = |b: usize| text[..b.min(text.len())].chars().count() as i32;
    let total = text.chars().count() as i32;
    let mut start = to_chars(err.span.start);
    let mut end = to_chars(err.span.end);
    if end <= start {
        // Zero-width (end of file, missing token): underline one character,
        // the previous one when at the end.
        if start < total && !text[err.span.start.min(text.len())..].starts_with('\n') {
            end = start + 1;
        } else {
            let before = text[..err.span.start.min(text.len())].trim_end();
            end = before.chars().count() as i32;
            start = (end - 1).max(0);
        }
    }
    let line = text[..text.char_indices().nth(start as usize).map_or(text.len(), |(i, _)| i)]
        .matches('\n')
        .count() as u32
        + 1;
    Some(Diag { line, message: err.message, start, end })
}

pub struct Doc {
    /// Project-relative path; `None` until the first save.
    pub path: RefCell<Option<String>>,
    pub untitled: String,
    pub buffer: SourceBuffer,
    pub view: SourceView,
    pub page: ScrolledWindow,
    pub tab: GtkBox,
    label: Label,
    pub close_button: Button,
    pub breakpoints: RefCell<BTreeSet<u32>>,
    pub search: SearchContext,
    debug_tag: TextTag,
    error_tag: TextTag,
    match_tag: TextTag,
    diag_tag: TextTag,
    pub diag: RefCell<Option<Diag>>,
    diag_timer: RefCell<Option<glib::SourceId>>,
    /// Called after each syntax check (status line).
    pub on_diag: RefCell<Option<Box<dyn Fn(&Doc)>>>,
}

const DIAG_DELAY_MS: u64 = 400;

impl Doc {
    pub fn new(
        path: Option<String>,
        untitled: String,
        text: &str,
        languages: &LanguageManager,
        search_settings: &SearchSettings,
    ) -> Rc<Self> {
        let buffer = SourceBuffer::new(None::<&gtk::TextTagTable>);
        if let Some(lang) = languages.language("expressa") {
            buffer.set_language(Some(&lang));
        }
        buffer.set_highlight_syntax(true);
        buffer.set_highlight_matching_brackets(true);
        let table = buffer.tag_table();
        let debug_tag = TextTag::new(Some("debug-current"));
        let error_tag = TextTag::new(Some("debug-error"));
        let match_tag = TextTag::new(Some("block-match"));
        match_tag.set_weight(700);
        let diag_tag = TextTag::new(Some("diag"));
        diag_tag.set_underline(gtk::pango::Underline::Error);
        diag_tag.set_underline_rgba(Some(&gtk::gdk::RGBA::new(0.9, 0.2, 0.2, 1.0)));
        for t in [&debug_tag, &error_tag, &match_tag, &diag_tag] {
            table.add(t);
        }
        let view = SourceView::builder()
            .buffer(&buffer)
            .monospace(true)
            .show_line_numbers(true)
            .show_line_marks(true)
            .highlight_current_line(true)
            .indent_on_tab(false)
            .tab_width(4)
            .indent_width(4)
            .insert_spaces_instead_of_tabs(true)
            .build();
        view.add_css_class("aula-code");
        view.set_wrap_mode(gtk::WrapMode::None);
        view.set_accepts_tab(true);
        view.set_input_hints(gtk::InputHints::NONE);
        view.set_input_purpose(gtk::InputPurpose::FreeForm);
        let bp = MarkAttributes::new();
        bp.set_pixbuf(&dot_pixbuf((0xe5, 0x39, 0x35)));
        view.set_mark_attributes("breakpoint", &bp, 10);
        let err = MarkAttributes::new();
        err.set_icon_name("dialog-error-symbolic");
        view.set_mark_attributes("erro", &err, 20);
        let page = ScrolledWindow::builder().vexpand(true).hexpand(true).child(&view).build();

        let label = Label::new(None);
        let close_button = Button::from_icon_name("window-close-symbolic");
        close_button.set_has_frame(false);
        close_button.set_tooltip_text(Some("Fechar (Ctrl+W)"));
        let tab = GtkBox::new(Orientation::Horizontal, 4);
        tab.append(&label);
        tab.append(&close_button);

        crate::indent::attach_electric(&buffer);
        // Loading the file is not an undo step, and it is the saved state.
        buffer.begin_irreversible_action();
        buffer.set_text(text);
        buffer.end_irreversible_action();
        buffer.set_modified(false);
        buffer.place_cursor(&buffer.start_iter());

        let search = SearchContext::new(&buffer, Some(search_settings));
        search.set_highlight(true);

        let doc = Rc::new(Self {
            path: RefCell::new(path),
            untitled,
            buffer,
            view,
            page,
            tab,
            label,
            close_button,
            breakpoints: RefCell::new(BTreeSet::new()),
            search,
            debug_tag,
            error_tag,
            match_tag,
            diag_tag,
            diag: RefCell::new(None),
            diag_timer: RefCell::new(None),
            on_diag: RefCell::new(None),
        });
        doc.refresh_label();
        doc.attach_signals();
        doc.check_now();
        doc
    }

    fn attach_signals(self: &Rc<Self>) {
        let weak: Weak<Self> = Rc::downgrade(self);
        self.buffer.connect_modified_changed({
            let weak = weak.clone();
            move |_| {
                if let Some(d) = weak.upgrade() {
                    d.refresh_label();
                }
            }
        });
        self.buffer.connect_changed({
            let weak = weak.clone();
            move |_| {
                if let Some(d) = weak.upgrade() {
                    d.schedule_check();
                }
            }
        });
        self.buffer.connect_cursor_position_notify(move |_| {
            if let Some(d) = weak.upgrade() {
                d.highlight_block();
            }
        });
    }

    /// Project-relative path, or the untitled name (used to run it).
    pub fn name(&self) -> String {
        self.path.borrow().clone().unwrap_or_else(|| self.untitled.clone())
    }

    pub fn is_untitled(&self) -> bool {
        self.path.borrow().is_none()
    }

    pub fn is_dirty(&self) -> bool {
        self.buffer.is_modified()
    }

    pub fn text(&self) -> String {
        let (s, e) = self.buffer.bounds();
        self.buffer.text(&s, &e, true).to_string()
    }

    pub fn refresh_label(&self) {
        let name = self.name();
        let base = name.rsplit('/').next().unwrap_or(&name).to_string();
        self.label.set_text(&if self.is_dirty() { format!("● {base}") } else { base });
        self.tab.set_tooltip_text(Some(&name));
    }

    pub fn set_path(&self, path: String) {
        *self.path.borrow_mut() = Some(path);
        self.refresh_label();
    }

    pub fn mark_saved(&self) {
        self.buffer.set_modified(false);
        self.refresh_label();
    }

    pub fn cursor(&self) -> gtk::TextIter {
        self.buffer.iter_at_mark(&self.buffer.get_insert())
    }

    pub fn goto_line(&self, line: u32) {
        let it = self
            .buffer
            .iter_at_line(line.saturating_sub(1) as i32)
            .unwrap_or_else(|| self.buffer.start_iter());
        self.buffer.place_cursor(&it);
        self.view.scroll_to_iter(&mut it.clone(), 0.2, false, 0.0, 0.0);
    }

    // ── Breakpoints ────────────────────────────────────────────────────────

    pub fn toggle_breakpoint(&self, line: u32) -> bool {
        let on = {
            let mut bps = self.breakpoints.borrow_mut();
            if bps.remove(&line) {
                false
            } else {
                bps.insert(line);
                true
            }
        };
        self.paint_breakpoints();
        on
    }

    pub fn paint_breakpoints(&self) {
        let (s, e) = self.buffer.bounds();
        self.buffer.remove_source_marks(&s, &e, Some("breakpoint"));
        for &line in self.breakpoints.borrow().iter() {
            if let Some(it) = self.buffer.iter_at_line(line as i32 - 1) {
                self.buffer.create_source_mark(None, "breakpoint", &it);
            }
        }
    }

    // ── Debugger line ──────────────────────────────────────────────────────

    /// Highlight the line where the debugger stopped (`error`: stopped by a
    /// runtime error) and bring it into view.
    pub fn show_debug_line(&self, line: u32, error: bool) {
        self.clear_debug_line();
        let Some(a) = self.buffer.iter_at_line(line as i32 - 1) else {
            return;
        };
        let mut b = a.clone();
        if !b.forward_line() {
            b = self.buffer.end_iter();
        }
        let tag = if error { &self.error_tag } else { &self.debug_tag };
        self.buffer.apply_tag(tag, &a, &b);
        self.buffer.place_cursor(&a);
        self.view.scroll_to_iter(&mut a.clone(), 0.2, false, 0.0, 0.0);
    }

    pub fn clear_debug_line(&self) {
        let (s, e) = self.buffer.bounds();
        self.buffer.remove_tag(&self.debug_tag, &s, &e);
        self.buffer.remove_tag(&self.error_tag, &s, &e);
    }

    // ── inicio/fim and {/} pairs ───────────────────────────────────────────

    fn highlight_block(&self) {
        let (s, e) = self.buffer.bounds();
        self.buffer.remove_tag(&self.match_tag, &s, &e);
        let text = self.buffer.text(&s, &e, true).to_string();
        let cursor = self.cursor().offset() as usize;
        let byte = text.char_indices().nth(cursor).map_or(text.len(), |(i, _)| i);
        let Some((open, close)) = crate::indent::block_pair(&text, byte) else {
            return;
        };
        let at = |b: usize| self.buffer.iter_at_offset(text[..b].chars().count() as i32);
        for (a, z) in [open, close] {
            self.buffer.apply_tag(&self.match_tag, &at(a), &at(z));
        }
    }

    // ── Syntax errors while typing ─────────────────────────────────────────

    fn schedule_check(self: &Rc<Self>) {
        if let Some(id) = self.diag_timer.borrow_mut().take() {
            id.remove();
        }
        let weak = Rc::downgrade(self);
        let id = glib::timeout_add_local_once(std::time::Duration::from_millis(DIAG_DELAY_MS), move || {
            if let Some(d) = weak.upgrade() {
                d.diag_timer.borrow_mut().take();
                d.check_now();
            }
        });
        *self.diag_timer.borrow_mut() = Some(id);
        // The block highlight follows edits too (the cursor may not move).
        self.highlight_block();
    }

    pub fn check_now(&self) {
        let (s, e) = self.buffer.bounds();
        self.buffer.remove_tag(&self.diag_tag, &s, &e);
        self.buffer.remove_source_marks(&s, &e, Some("erro"));
        let diag = diagnose(&self.buffer.text(&s, &e, true));
        if let Some(d) = &diag {
            let a = self.buffer.iter_at_offset(d.start);
            let b = self.buffer.iter_at_offset(d.end);
            self.buffer.apply_tag(&self.diag_tag, &a, &b);
            if let Some(it) = self.buffer.iter_at_line(d.line as i32 - 1) {
                self.buffer.create_source_mark(None, "erro", &it);
            }
        }
        *self.diag.borrow_mut() = diag;
        if let Some(f) = self.on_diag.borrow().as_ref() {
            f(self);
        }
    }

    /// The syntax error at char `offset`, if any.
    pub fn diag_at(&self, offset: i32) -> Option<Diag> {
        self.diag
            .borrow()
            .clone()
            .filter(|d| offset >= d.start && offset <= d.end)
    }

    // ── Theme ──────────────────────────────────────────────────────────────

    pub fn set_theme(&self, scheme: Option<&StyleScheme>, dark: bool) {
        if let Some(s) = scheme {
            self.buffer.set_style_scheme(Some(s));
        }
        let (debug_bg, error_bg, match_bg) = if dark {
            ("#4d4520", "#5c2626", "#31523a")
        } else {
            ("#fff3bf", "#ffcdd2", "#c5e1a5")
        };
        self.debug_tag.set_background(Some(debug_bg));
        self.error_tag.set_background(Some(error_bg));
        self.match_tag.set_background(Some(match_bg));
    }
}

/// Round dot for the breakpoint gutter.
pub fn dot_pixbuf((r, g, b): (u8, u8, u8)) -> gtk::gdk_pixbuf::Pixbuf {
    let pb = gtk::gdk_pixbuf::Pixbuf::new(gtk::gdk_pixbuf::Colorspace::Rgb, true, 8, 12, 12).expect("pixbuf");
    pb.fill(0);
    for y in 0..12u32 {
        for x in 0..12u32 {
            let (dx, dy) = (x as i32 - 5, y as i32 - 5);
            if dx * dx + dy * dy <= 16 {
                pb.put_pixel(x, y, r, g, b, 0xff);
            }
        }
    }
    pb
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn syntax_errors_while_typing() {
        assert_eq!(diagnose("escreva(1)\n"), None);
        let d = diagnose("x = (1 +\n").unwrap();
        assert!(d.end > d.start, "{d:?}");
        let d = diagnose("se a {\n    x = 1\n").unwrap();
        assert!(d.line >= 1, "{d:?}");
        // Offsets are characters, not bytes.
        let d = diagnose("ação = \"olá\"\nx = = 2\n").unwrap();
        assert_eq!(d.line, 2, "{d:?}");
        let src: Vec<char> = "ação = \"olá\"\nx = = 2\n".chars().collect();
        assert!(src[d.start as usize..d.end as usize].iter().collect::<String>().contains('='), "{d:?}");
    }
}
