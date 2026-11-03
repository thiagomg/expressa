//! One open file (a tab): buffer with its own undo history, view,
//! breakpoints, search, block-match highlight and syntax check.

use std::cell::{Cell, RefCell};
use std::collections::BTreeSet;
use std::rc::{Rc, Weak};

use gtk::glib;
use gtk::prelude::*;
use gtk::{Box as GtkBox, Button, Label, Orientation, ScrolledWindow, TextTag};
use sourceview5::prelude::*;
use sourceview5::{
    Buffer as SourceBuffer, LanguageManager, MarkAttributes, SearchContext, SearchSettings,
    StyleScheme, View as SourceView,
};

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
    let line = text[..text
        .char_indices()
        .nth(start as usize)
        .map_or(text.len(), |(i, _)| i)]
        .matches('\n')
        .count() as u32
        + 1;
    Some(Diag {
        line,
        message: err.message,
        start,
        end,
    })
}

/// Named mark `scroll_to_mark` follows. Reused so pauses do not pile up marks.
const SCROLL_MARK: &str = "aula-scroll";

pub struct Doc {
    /// Project-relative path; `None` until the first save.
    pub path: RefCell<Option<String>>,
    /// Revision of the saved bytes this buffer was based on (never refreshed on save failure).
    pub revision: RefCell<Option<String>>,
    /// Monotonic buffer revision, independent of the saved storage revision.
    pub edit_epoch: Cell<u64>,
    pub io_epoch: Cell<u64>,
    pub saving: Cell<bool>,
    pub pending_external_change: Cell<bool>,
    pub recovery: std::sync::Arc<crate::recuperacao::Owner>,
    analysis_epoch: Cell<u64>,
    analysis: RefCell<Option<crate::analysis::Analysis>>,
    analysis_job: RefCell<Option<crate::background::Reply<crate::analysis::Analysis>>>,
    pub untitled: String,
    pub buffer: SourceBuffer,
    pub view: SourceView,
    pub page: GtkBox,
    pub tab: GtkBox,
    label: Label,
    pub close_button: Button,
    breakpoint_anchors: RefCell<Vec<gtk::TextMark>>,
    executable_lines: RefCell<Option<BTreeSet<u32>>>,
    pub running_source: RefCell<Option<std::sync::Arc<crate::debug_sources::Source>>>,
    run_notice: Label,
    pub inspect_source: Button,
    pub search: SearchContext,
    debug_tag: TextTag,
    error_tag: TextTag,
    match_tag: TextTag,
    diag_tag: TextTag,
    pub diag: RefCell<Option<Diag>>,
    diag_timer: RefCell<Option<glib::SourceId>>,
    /// Watches the file of a local project (`sessao::vigiar`).
    pub vigia: RefCell<Option<gtk::gio::FileMonitor>>,
    /// Hash of the text last copied aside while unsaved (`sessao.rs`).
    pub resumo_da_copia: Cell<Option<u64>>,
    /// Exact copies owned by this tab; unrelated pending recoveries stay intact.
    pub copia_recuperacao: RefCell<Option<crate::recuperacao::Backup>>,
    /// Marks the other places of the word under the cursor.
    ocorrencia_tag: TextTag,
    ocorrencia_timer: RefCell<Option<glib::SourceId>>,
    /// Called after each syntax check (status line).
    pub on_diag: RefCell<Option<Box<dyn Fn(&Doc)>>>,
}

const DIAG_DELAY_MS: u64 = 400;
const OCORRENCIAS_DELAY_MS: u64 = 250;

#[cfg(test)]
fn faz_parte_de_nome(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Where the word around character `cursor` appears in `texto`, as
/// character ranges, when it is a name worth marking: not a keyword, at
/// least two letters, and in more than one place (one is just itself).
#[cfg(test)]
pub fn ocorrencias_da_palavra(texto: &str, cursor: usize) -> Vec<(usize, usize)> {
    // Big files: not worth scanning on every cursor move.
    if texto.len() > 200_000 {
        return Vec::new();
    }
    let chars: Vec<char> = texto.chars().collect();
    let cursor = cursor.min(chars.len());
    let mut ini = cursor;
    while ini > 0 && faz_parte_de_nome(chars[ini - 1]) {
        ini -= 1;
    }
    let mut fim = cursor;
    while fim < chars.len() && faz_parte_de_nome(chars[fim]) {
        fim += 1;
    }
    let palavra = &chars[ini..fim];
    let nome: String = palavra.iter().collect();
    if palavra.len() < 2
        || palavra[0].is_numeric()
        || crate::lang::KEYWORDS.contains(&nome.as_str())
    {
        return Vec::new();
    }
    let n = palavra.len();
    let mut achadas = Vec::new();
    let mut i = 0;
    while i + n <= chars.len() {
        let antes_ok = i == 0 || !faz_parte_de_nome(chars[i - 1]);
        let depois_ok = i + n == chars.len() || !faz_parte_de_nome(chars[i + n]);
        if antes_ok && depois_ok && chars[i..i + n] == *palavra {
            achadas.push((i, i + n));
            i += n;
        } else {
            i += 1;
        }
    }
    if achadas.len() < 2 {
        achadas.clear();
    }
    achadas
}

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
        // Translucent grey: readable on the light and the dark theme.
        let ocorrencia_tag = TextTag::new(Some("ocorrencia"));
        ocorrencia_tag.set_background_rgba(Some(&gtk::gdk::RGBA::new(0.5, 0.5, 0.5, 0.25)));
        for t in [
            &debug_tag,
            &error_tag,
            &match_tag,
            &diag_tag,
            &ocorrencia_tag,
        ] {
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
        bp.connect_query_tooltip_text(|_, _| "Ponto de parada para a próxima execução".into());
        view.set_mark_attributes("breakpoint", &bp, 10);
        let pending = MarkAttributes::new();
        pending.set_pixbuf(&ring_pixbuf());
        pending.connect_query_tooltip_text(|_, _| {
            "Sem instrução executável nesta linha, ou fonte com erro de sintaxe".into()
        });
        view.set_mark_attributes("breakpoint-pending", &pending, 10);
        let err = MarkAttributes::new();
        err.set_icon_name("dialog-error-symbolic");
        view.set_mark_attributes("erro", &err, 20);
        let scroll = ScrolledWindow::builder()
            .vexpand(true)
            .hexpand(true)
            .child(&view)
            .build();

        let page = GtkBox::new(Orientation::Vertical, 4);
        let run_notice = Label::new(None);
        run_notice.set_xalign(0.0);
        run_notice.set_wrap(true);
        run_notice.set_visible(false);
        let inspect_source = Button::with_label("Ver fonte em execução");
        inspect_source.set_halign(gtk::Align::Start);
        inspect_source.set_visible(false);
        page.append(&run_notice);
        page.append(&inspect_source);
        page.append(&scroll);

        let label = Label::new(None);
        let close_button = Button::from_icon_name("window-close-symbolic");
        close_button.set_has_frame(false);
        close_button.set_tooltip_text(Some("Fechar (Ctrl+W)"));
        let tab = GtkBox::new(Orientation::Horizontal, 4);
        tab.append(&label);
        tab.append(&close_button);

        crate::indent::attach_electric(&buffer);
        crate::pares::attach(&buffer);
        crate::editing::attach_plain_paste(&view);
        // Loading the file is not an undo step, and it is the saved state.
        buffer.begin_irreversible_action();
        buffer.set_text(text);
        buffer.end_irreversible_action();
        buffer.set_modified(false);
        buffer.place_cursor(&buffer.start_iter());

        let search = SearchContext::new(&buffer, Some(search_settings));
        search.set_highlight(true);

        // A storage revision comes from the read receipt, never a guess based on editor text.
        let revision = None;
        let doc = Rc::new(Self {
            path: RefCell::new(path),
            revision: RefCell::new(revision),
            edit_epoch: Cell::new(1),
            io_epoch: Cell::new(0),
            saving: Cell::new(false),
            pending_external_change: Cell::new(false),
            recovery: std::sync::Arc::new(crate::recuperacao::Owner::default()),
            analysis_epoch: Cell::new(0),
            analysis: RefCell::new(None),
            analysis_job: RefCell::new(None),
            untitled,
            buffer,
            view,
            page,
            tab,
            label,
            close_button,
            breakpoint_anchors: RefCell::new(Vec::new()),
            executable_lines: RefCell::new(None),
            running_source: RefCell::new(None),
            run_notice,
            inspect_source,
            search,
            debug_tag,
            error_tag,
            match_tag,
            diag_tag,
            diag: RefCell::new(None),
            diag_timer: RefCell::new(None),
            vigia: RefCell::new(None),
            resumo_da_copia: Cell::new(None),
            copia_recuperacao: RefCell::new(None),
            ocorrencia_tag,
            ocorrencia_timer: RefCell::new(None),
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
                    d.edit_epoch.set(d.edit_epoch.get() + 1);
                    d.analysis_job.borrow_mut().take();
                    let (start, end) = d.buffer.bounds();
                    d.buffer.remove_tag(&d.diag_tag, &start, &end);
                    d.buffer.remove_source_marks(&start, &end, Some("erro"));
                    d.diag.borrow_mut().take();
                    d.executable_lines.borrow_mut().take();
                    d.paint_breakpoints();
                    d.refresh_running_notice();
                    d.clear_debug_line();
                    d.schedule_check();
                }
            }
        });
        self.buffer.connect_cursor_position_notify(move |_| {
            if let Some(d) = weak.upgrade() {
                d.highlight_block();
                d.agendar_ocorrencias();
            }
        });
    }

    /// Project-relative path, or the untitled name (used to run it).
    pub fn name(&self) -> String {
        self.path
            .borrow()
            .clone()
            .unwrap_or_else(|| self.untitled.clone())
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
        let base = name.rsplit(['/', '\\']).next().unwrap_or(&name).to_string();
        self.label.set_text(&if self.saving.get() {
            format!("{base} — salvando…")
        } else if self.is_dirty() {
            format!("● {base}")
        } else {
            base
        });
        let tooltip = match self.copia_recuperacao.borrow().as_ref() {
            Some(backup) if self.is_untitled() => format!(
                "{name}\nRecuperado de {} — confira e salve",
                backup
                    .name
                    .strip_prefix(crate::recuperacao::UNTITLED_PREFIX)
                    .unwrap_or(&backup.name)
            ),
            _ => name,
        };
        self.tab.set_tooltip_text(Some(&tooltip));
    }

    pub fn set_path(&self, path: String) {
        self.io_epoch.set(self.io_epoch.get() + 1);
        *self.path.borrow_mut() = Some(path);
        self.refresh_label();
        self.refresh_running_notice();
    }

    /// Replace the text with `text`, the file's new saved content (the
    /// program changed it). Keeps the cursor on the same line.
    pub fn recarregar(&self, text: &str) {
        self.recarregar_snapshot(text, Some(expressa_aula_storage::revision(text.as_bytes())));
    }

    pub fn recarregar_snapshot(&self, text: &str, revision: Option<String>) {
        self.io_epoch.set(self.io_epoch.get() + 1);
        let linha = self.cursor().line();
        self.buffer.begin_irreversible_action();
        self.buffer.set_text(text);
        self.buffer.end_irreversible_action();
        let iter = self
            .buffer
            .iter_at_line(linha)
            .unwrap_or_else(|| self.buffer.end_iter());
        self.buffer.place_cursor(&iter);
        *self.revision.borrow_mut() = revision;
        self.mark_saved();
    }

    pub fn mark_saved(&self) {
        self.recovery.saved(self.edit_epoch.get());
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
        self.scroll_iter_into_view(&it);
    }

    /// Bring `iter` on screen.
    ///
    /// `scroll_to_iter` reads the line's cached y and returns. On Windows that
    /// cache is filled a page at a time, so one call stops short and the
    /// breakpoint the debugger just hit is left off screen. `scroll_to_mark`
    /// asks again once the heights exist.
    fn scroll_iter_into_view(&self, iter: &gtk::TextIter) {
        if self.buffer.mark(SCROLL_MARK).is_some() {
            self.buffer.move_mark_by_name(SCROLL_MARK, iter);
        } else {
            self.buffer.create_mark(Some(SCROLL_MARK), iter, true);
        }
        if let Some(mark) = self.buffer.mark(SCROLL_MARK) {
            self.view.scroll_to_mark(&mark, 0.0, true, 0.0, 0.25);
        }
    }

    // ── Breakpoints ────────────────────────────────────────────────────────

    /// Anchors use right gravity: inserting lines at a breakpoint moves it
    /// with the original text. Deleting its text collapses it onto the next
    /// surviving position (EOF when necessary). Undo/redo follows that position;
    /// it does not recreate deleted text identities. Coincident anchors dedupe.
    pub fn breakpoint_lines(&self) -> BTreeSet<u32> {
        self.breakpoint_anchors
            .borrow()
            .iter()
            .filter(|m| !m.is_deleted())
            .map(|m| self.buffer.iter_at_mark(m).line() as u32 + 1)
            .collect()
    }

    pub fn toggle_breakpoint(&self, line: u32) -> bool {
        let Some(iter) = line
            .checked_sub(1)
            .and_then(|l| self.buffer.iter_at_line(l as i32))
        else {
            return false;
        };
        let on = !self.breakpoint_lines().contains(&line);
        if on {
            let mark = self.buffer.create_mark(None, &iter, false);
            self.breakpoint_anchors.borrow_mut().push(mark);
        } else {
            self.breakpoint_anchors.borrow_mut().retain(|mark| {
                if self.buffer.iter_at_mark(mark).line() as u32 + 1 == line {
                    self.buffer.delete_mark(mark);
                    false
                } else {
                    true
                }
            });
        }
        self.paint_breakpoints();
        on
    }

    pub fn paint_breakpoints(&self) {
        let (s, e) = self.buffer.bounds();
        for category in ["breakpoint", "breakpoint-pending"] {
            self.buffer.remove_source_marks(&s, &e, Some(category));
        }
        for line in self.breakpoint_lines() {
            if let Some(it) = self.buffer.iter_at_line(line as i32 - 1) {
                let bound = self
                    .executable_lines
                    .borrow()
                    .as_ref()
                    .is_some_and(|lines| lines.contains(&line));
                self.buffer.create_source_mark(
                    None,
                    if bound {
                        "breakpoint"
                    } else {
                        "breakpoint-pending"
                    },
                    &it,
                );
            }
        }
    }

    pub fn set_running_source(&self, source: Option<std::sync::Arc<crate::debug_sources::Source>>) {
        *self.running_source.borrow_mut() = source;
        self.refresh_running_notice();
    }

    fn refresh_running_notice(&self) {
        let running = self.running_source.borrow();
        let changed = running.as_ref().is_some_and(|s| {
            s.differs(&self.text()) || s.path != crate::debug_sources::normalize_path(&self.name())
        });
        self.run_notice.set_text(if changed {
            "Editado após iniciar a execução. As alterações e os pontos de parada valem na próxima execução."
        } else {
            "Esta execução usa uma cópia fixa. Alterações e pontos de parada valem na próxima execução."
        });
        self.run_notice.set_visible(running.is_some());
        self.inspect_source.set_visible(running.is_some());
    }

    // ── Debugger line ──────────────────────────────────────────────────────

    /// Highlight the line where the debugger stopped (`error`: stopped by a
    /// runtime error) and bring it into view.
    pub fn show_debug_line(&self, line: u32, error: bool) {
        self.clear_debug_line();
        let Some(a) = self.buffer.iter_at_line(line as i32 - 1) else {
            return;
        };
        let mut b = a;
        if !b.forward_line() {
            b = self.buffer.end_iter();
        }
        let tag = if error {
            &self.error_tag
        } else {
            &self.debug_tag
        };
        self.buffer.apply_tag(tag, &a, &b);
        self.buffer.place_cursor(&a);
        self.scroll_iter_into_view(&a);
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
        if self.analysis_epoch.get() != self.edit_epoch.get() {
            return;
        }
        let analysis = self.analysis.borrow();
        let cursor = self.cursor();
        let Some((open, close)) = analysis
            .as_ref()
            .and_then(|a| a.block(cursor.line(), cursor.offset()))
        else {
            return;
        };
        for (a, b) in [open, close] {
            self.buffer.apply_tag(
                &self.match_tag,
                &self.buffer.iter_at_offset(a),
                &self.buffer.iter_at_offset(b),
            );
        }
    }

    // ── Syntax errors while typing ─────────────────────────────────────────

    /// Marks the word under the cursor everywhere in the file, shortly
    /// after the cursor stops (not on every keystroke).
    fn agendar_ocorrencias(self: &Rc<Self>) {
        if let Some(id) = self.ocorrencia_timer.borrow_mut().take() {
            id.remove();
        }
        let weak = Rc::downgrade(self);
        let id = glib::timeout_add_local_once(
            std::time::Duration::from_millis(OCORRENCIAS_DELAY_MS),
            move || {
                if let Some(d) = weak.upgrade() {
                    d.ocorrencia_timer.borrow_mut().take();
                    d.marcar_ocorrencias();
                }
            },
        );
        *self.ocorrencia_timer.borrow_mut() = Some(id);
    }

    fn marcar_ocorrencias(&self) {
        let (s, e) = self.buffer.bounds();
        self.buffer.remove_tag(&self.ocorrencia_tag, &s, &e);
        if self.buffer.has_selection() {
            return;
        }
        if self.analysis_epoch.get() != self.edit_epoch.get() {
            return;
        }
        let analysis = self.analysis.borrow();
        if let Some(analysis) = analysis.as_ref() {
            for &(start, end) in analysis.occurrences(self.cursor().offset()) {
                self.buffer.apply_tag(
                    &self.ocorrencia_tag,
                    &self.buffer.iter_at_offset(start),
                    &self.buffer.iter_at_offset(end),
                );
            }
        }
    }

    fn schedule_check(self: &Rc<Self>) {
        if let Some(id) = self.diag_timer.borrow_mut().take() {
            id.remove();
        }
        let weak = Rc::downgrade(self);
        let id = glib::timeout_add_local_once(
            std::time::Duration::from_millis(DIAG_DELAY_MS),
            move || {
                if let Some(d) = weak.upgrade() {
                    d.diag_timer.borrow_mut().take();
                    d.check_now();
                }
            },
        );
        *self.diag_timer.borrow_mut() = Some(id);
        // The block highlight follows edits too (the cursor may not move).
        self.highlight_block();
    }

    pub fn check_now(self: &Rc<Self>) {
        if let Some(source) = self.diag_timer.borrow_mut().take() {
            source.remove();
        }
        self.analysis_job.borrow_mut().take();
        let epoch = self.edit_epoch.get();
        let text = self.text();
        *self.analysis_job.borrow_mut() = Some(
            crate::background::analysis()
                .submit(epoch, move || Ok(crate::analysis::Analysis::new(&text))),
        );
        let weak = Rc::downgrade(self);
        let source = glib::timeout_add_local(std::time::Duration::from_millis(16), move || {
            let Some(doc) = weak.upgrade() else {
                return glib::ControlFlow::Break;
            };
            let result = doc
                .analysis_job
                .borrow()
                .as_ref()
                .map(|job| job.poll(doc.edit_epoch.get()));
            match result {
                Some(crate::background::Poll::Pending) => return glib::ControlFlow::Continue,
                Some(crate::background::Poll::Ready(Ok(analysis))) => {
                    doc.analysis_job.borrow_mut().take();
                    doc.diag_timer.borrow_mut().take();
                    doc.apply_analysis(epoch, analysis);
                }
                Some(crate::background::Poll::Ready(Err(_))) => {
                    doc.analysis_job.borrow_mut().take();
                    doc.diag_timer.borrow_mut().take();
                    doc.schedule_check();
                }
                _ => {
                    doc.analysis_job.borrow_mut().take();
                    doc.diag_timer.borrow_mut().take();
                }
            }
            glib::ControlFlow::Break
        });
        *self.diag_timer.borrow_mut() = Some(source);
    }

    pub fn cancel_pending(&self) {
        self.analysis_job.borrow_mut().take();
        if let Some(monitor) = self.vigia.borrow_mut().take() {
            monitor.cancel();
        }
        if let Some(id) = self.diag_timer.borrow_mut().take() {
            id.remove();
        }
        if let Some(id) = self.ocorrencia_timer.borrow_mut().take() {
            id.remove();
        }
    }

    fn apply_analysis(&self, epoch: u64, analysis: crate::analysis::Analysis) {
        if epoch != self.edit_epoch.get() {
            return;
        }
        self.analysis_epoch.set(epoch);
        let (s, e) = self.buffer.bounds();
        self.buffer.remove_tag(&self.diag_tag, &s, &e);
        self.buffer.remove_source_marks(&s, &e, Some("erro"));
        let diag = analysis.diag.clone();
        *self.executable_lines.borrow_mut() = analysis.lines.clone();
        *self.analysis.borrow_mut() = Some(analysis);
        self.paint_breakpoints();
        if let Some(d) = &diag {
            let a = self.buffer.iter_at_offset(d.start);
            let b = self.buffer.iter_at_offset(d.end);
            self.buffer.apply_tag(&self.diag_tag, &a, &b);
            if let Some(it) = self.buffer.iter_at_line(d.line as i32 - 1) {
                self.buffer.create_source_mark(None, "erro", &it);
            }
        }
        *self.diag.borrow_mut() = diag;
        self.highlight_block();
        self.marcar_ocorrencias();
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
    let pb = gtk::gdk_pixbuf::Pixbuf::new(gtk::gdk_pixbuf::Colorspace::Rgb, true, 8, 12, 12)
        .expect("pixbuf");
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

fn ring_pixbuf() -> gtk::gdk_pixbuf::Pixbuf {
    let pb = dot_pixbuf((0x88, 0x88, 0x88));
    for y in 0..12u32 {
        for x in 0..12u32 {
            let (dx, dy) = (x as i32 - 5, y as i32 - 5);
            if dx * dx + dy * dy < 9 {
                pb.put_pixel(x, y, 0, 0, 0, 0);
            }
        }
    }
    pb
}

impl Drop for Doc {
    fn drop(&mut self) {
        if let Some(id) = self.diag_timer.get_mut().take() {
            id.remove();
        }
        if let Some(id) = self.ocorrencia_timer.get_mut().take() {
            id.remove();
        }
    }
}

#[cfg(test)]
impl Doc {
    pub(crate) fn wait_for_analysis(&self) {
        let end = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while self.analysis_epoch.get() != self.edit_epoch.get() {
            assert!(
                std::time::Instant::now() < end,
                "document analysis did not finish"
            );
            gtk::glib::MainContext::default().iteration(false);
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ocorrencias_so_de_nomes_inteiros() {
        let t = "soma = 0\nsomatorio = soma + 1\nescreva(soma)\n";
        let cursor = t.find("soma +").unwrap() + 2;
        assert_eq!(
            ocorrencias_da_palavra(t, cursor),
            [(0, 4), (21, 25), (38, 42)]
        );
    }

    #[test]
    fn ocorrencias_nada_para_palavra_chave_numero_letra_ou_nome_unico() {
        assert!(ocorrencias_da_palavra("se a {\n} se b {\n}", 0).is_empty());
        assert!(ocorrencias_da_palavra("x = 10 + 10", 5).is_empty());
        assert!(ocorrencias_da_palavra("a = a", 0).is_empty());
        assert!(ocorrencias_da_palavra("contador = 1", 2).is_empty());
    }

    #[test]
    fn ocorrencias_com_acento() {
        let t = "posição = 1\nescreva(posição)";
        assert_eq!(ocorrencias_da_palavra(t, 3).len(), 2);
    }

    #[test]
    fn breakpoint_mark_survives_the_debug_line() {
        crate::gtk_test::run(|| {
            let languages = sourceview5::LanguageManager::new();
            let search = sourceview5::SearchSettings::new();
            let doc = Doc::new(
                Some("jogos/forca.lep".into()),
                "sem-titulo.lep".into(),
                "escreva(1)\nescreva(2)\n",
                &languages,
                &search,
            );
            doc.wait_for_analysis();
            assert!(doc.toggle_breakpoint(2));
            doc.show_debug_line(1, false);
            assert!(doc.breakpoint_lines().contains(&2));
            let marks = doc.buffer.source_marks_at_line(1, Some("breakpoint"));
            assert_eq!(marks.len(), 1);
        });
    }

    fn test_doc(text: &str) -> Rc<Doc> {
        let doc = Doc::new(
            Some("a.lep".into()),
            String::new(),
            text,
            &sourceview5::LanguageManager::new(),
            &SearchSettings::new(),
        );
        doc.wait_for_analysis();
        doc
    }

    #[test]
    fn anchored_breakpoints_follow_insert_delete_and_undo_redo() {
        crate::gtk_test::run(|| {
            let doc = test_doc("a = 1\nescreva(a)\n");
            doc.toggle_breakpoint(2);
            let mut before = doc.buffer.iter_at_line(1).unwrap();
            doc.buffer.begin_user_action();
            doc.buffer.insert(&mut before, "// inserted\n\n");
            doc.buffer.end_user_action();
            assert_eq!(doc.breakpoint_lines(), BTreeSet::from([4]));
            doc.check_now();
            doc.wait_for_analysis();
            assert_eq!(
                doc.buffer.source_marks_at_line(3, Some("breakpoint")).len(),
                1
            );
            doc.buffer.undo();
            assert_eq!(doc.breakpoint_lines(), BTreeSet::from([2]));
            doc.buffer.redo();
            assert_eq!(doc.breakpoint_lines(), BTreeSet::from([4]));
            let mut start = doc.buffer.start_iter();
            let mut end = doc.buffer.iter_at_line(3).unwrap();
            doc.buffer.delete(&mut start, &mut end);
            assert_eq!(doc.breakpoint_lines(), BTreeSet::from([1]));
        });
    }

    #[test]
    fn deleted_and_replaced_breakpoint_lines_follow_next_surviving_text() {
        crate::gtk_test::run(|| {
            let doc = test_doc("a = 1\nb = 2\nc = 3\nescreva(c)\n");
            doc.toggle_breakpoint(2);
            doc.toggle_breakpoint(3);
            doc.toggle_breakpoint(4);
            doc.buffer.begin_user_action();
            let mut start = doc.buffer.iter_at_line(1).unwrap();
            let mut end = doc.buffer.iter_at_line(3).unwrap();
            doc.buffer.delete(&mut start, &mut end);
            doc.buffer.insert(&mut start, "// replacement\n");
            doc.buffer.end_user_action();
            assert_eq!(doc.breakpoint_lines(), BTreeSet::from([3]));
            doc.buffer.undo();
            // All deleted anchors stay attached to the surviving statement.
            assert_eq!(doc.breakpoint_lines(), BTreeSet::from([4]));
            doc.buffer.redo();
            assert_eq!(doc.breakpoint_lines(), BTreeSet::from([3]));
            assert!(!doc.toggle_breakpoint(3));
            assert!(doc.breakpoint_lines().is_empty());
            assert!(!doc.toggle_breakpoint(0));
            assert!(!doc.toggle_breakpoint(999));
        });
    }

    #[test]
    fn non_executable_breakpoints_are_visibly_pending() {
        crate::gtk_test::run(|| {
            let doc = test_doc("// comment\na = 1\n\n");
            doc.toggle_breakpoint(1);
            doc.toggle_breakpoint(2);
            assert_eq!(
                doc.buffer
                    .source_marks_at_line(0, Some("breakpoint-pending"))
                    .len(),
                1
            );
            assert_eq!(
                doc.buffer.source_marks_at_line(1, Some("breakpoint")).len(),
                1
            );
        });
    }

    #[test]
    fn edits_and_rename_show_divergence_without_changing_running_source() {
        crate::gtk_test::run(|| {
            let doc = test_doc("escreva(1)\n");
            let source =
                std::sync::Arc::new(crate::debug_sources::Source::new("a.lep", doc.text()));
            doc.set_running_source(Some(source.clone()));
            assert!(doc.run_notice.is_visible());
            assert!(!doc.run_notice.text().starts_with("Editado"));
            doc.buffer.insert(&mut doc.buffer.start_iter(), "// edit\n");
            assert!(doc.run_notice.text().starts_with("Editado"));
            assert_eq!(source.text, "escreva(1)\n");
            doc.buffer.undo();
            assert!(!doc.run_notice.text().starts_with("Editado"));
            doc.set_path("renamed.lep".into());
            assert!(doc.run_notice.text().starts_with("Editado"));
            assert_eq!(doc.running_source.borrow().as_ref().unwrap().path, "a.lep");
            doc.set_running_source(None);
            assert!(!doc.run_notice.is_visible());
            assert!(!doc.inspect_source.is_visible());
        });
    }

    #[test]
    fn stale_analysis_never_paints_diagnostics_on_a_newer_revision() {
        crate::gtk_test::run(|| {
            let doc = test_doc("a = 1\n");
            doc.buffer.set_text("a = (\n");
            let old = doc.edit_epoch.get();
            let stale = crate::analysis::Analysis::new(&doc.text());
            assert!(stale.diag.is_some());
            doc.buffer.set_text("a = 2\n");
            doc.apply_analysis(old, stale);
            assert!(doc.diag.borrow().is_none());
            doc.wait_for_analysis();
            assert!(doc.diag.borrow().is_none());
            assert_eq!(doc.analysis_epoch.get(), doc.edit_epoch.get());
        });
    }

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
        assert!(
            src[d.start as usize..d.end as usize]
                .iter()
                .collect::<String>()
                .contains('='),
            "{d:?}"
        );
    }
}
