//! Debugger panel, run-bar sensitivity, breakpoints, and the paused-source viewer.

use std::rc::Rc;
use std::sync::Arc;

use crate::docs::Doc;
use crate::*;
use gtk::gio::prelude::*;
use gtk::prelude::*;
use gtk::{
    Box as GtkBox, Button, CellRendererText, Label, Orientation, ScrolledWindow, TreeStore,
    TreeView, TreeViewColumn,
};

// ── Debugger panel ─────────────────────────────────────────────────────────

pub(crate) fn make_string_tree(store: &TreeStore, titles: &[&str]) -> TreeView {
    let tree = TreeView::with_model(store);
    tree.set_headers_visible(true);
    for (i, title) in titles.iter().enumerate() {
        let col = TreeViewColumn::new();
        col.set_title(title);
        col.set_resizable(true);
        let cell = CellRendererText::new();
        col.pack_start(&cell, true);
        col.add_attribute(&cell, "text", i as i32);
        tree.append_column(&col);
    }
    tree
}

pub(crate) fn make_debug_panel(ui: &UiRc) -> GtkBox {
    let panel = GtkBox::new(Orientation::Vertical, 4);
    panel.set_margin_start(4);
    panel.set_margin_end(4);
    panel.set_hexpand(false);
    panel.set_vexpand(true);

    let watch_row = GtkBox::new(Orientation::Horizontal, 4);
    let lbl = Label::new(Some("Observar:"));
    let btn_add = Button::with_label("+");
    let btn_del = Button::with_label("−");
    watch_row.append(&lbl);
    watch_row.append(&ui.borrow().depuracao.watch_entry);
    watch_row.append(&btn_add);
    watch_row.append(&btn_del);
    panel.append(&watch_row);

    connect(&btn_add, ui, add_watch);
    {
        let ui_a = Rc::clone(ui);
        ui.borrow()
            .depuracao
            .watch_entry
            .connect_activate(move |_| add_watch(&ui_a));
    }
    connect(&btn_del, ui, remove_watch);

    let u = ui.borrow();
    let watch_tree = make_string_tree(&u.depuracao.watch_store, &["observado", "valor"]);
    let vars_tree = make_string_tree(&u.depuracao.vars_store, &["onde", "nome", "valor"]);
    let stack_tree = make_string_tree(&u.depuracao.stack_store, &["função", "linha"]);
    drop(u);
    {
        let ui_s = Rc::clone(ui);
        stack_tree.connect_row_activated(move |tree, path, _col| {
            let Some(model) = tree.model() else { return };
            let Ok(store) = model.downcast::<TreeStore>() else {
                return;
            };
            let Some(iter) = store.iter(path) else { return };
            let loc: String = store.get(&iter, 1);
            if let Some((file, line)) = loc.rsplit_once(':').and_then(|(f, n)| {
                n.parse()
                    .ok()
                    .filter(|l| *l > 0)
                    .map(|l| (f.to_string(), l))
            }) {
                if ui_s
                    .borrow()
                    .execucao
                    .run
                    .as_ref()
                    .is_some_and(|run| run.paused.is_some() && !run.stopping)
                {
                    if !show_running_source(&ui_s, &file, line, false) {
                        set_status(&ui_s, "fonte desta execução não está disponível");
                    }
                } else {
                    navigate_execution_error(&ui_s, &file, line, false);
                }
            }
        });
    }

    panel.append(&Label::new(Some(
        "Observados (ficam visíveis a cada passo)",
    )));
    panel.append(&scroll_tree(&watch_tree, 40));
    panel.append(&Label::new(Some("Variáveis")));
    panel.append(&scroll_tree(&vars_tree, 40));
    panel.append(&Label::new(Some("Pilha")));
    panel.append(&scroll_tree(&stack_tree, 40));
    panel
}

pub(crate) fn scroll_tree(tree: &TreeView, min_h: i32) -> ScrolledWindow {
    ScrolledWindow::builder()
        .min_content_height(min_h)
        .vexpand(true)
        .child(tree)
        .build()
}

pub(crate) fn add_watch(ui: &UiRc) {
    let name = ui.borrow().depuracao.watch_entry.text().trim().to_string();
    if name.is_empty() {
        return;
    }
    {
        let mut u = ui.borrow_mut();
        if !u.depuracao.watches.iter().any(|w| w == &name) {
            u.depuracao.watches.push(name);
        }
        u.depuracao.watch_entry.set_text("");
    }
    refresh_watch_values(ui, &[]);
}

pub(crate) fn remove_watch(ui: &UiRc) {
    ui.borrow_mut().depuracao.watches.pop();
    refresh_watch_values(ui, &[]);
}

pub(crate) fn refresh_watch_values(ui: &UiRc, vars: &[expressa::runtime::DebugBinding]) {
    let u = ui.borrow();
    u.depuracao.watch_store.clear();
    for name in &u.depuracao.watches {
        let val = vars
            .iter()
            .find(|v| v.name == *name)
            .map(|v| v.value.as_str())
            .unwrap_or("—");
        let iter = u.depuracao.watch_store.append(None);
        u.depuracao.watch_store.set(&iter, &[(0, name), (1, &val)]);
    }
}

pub(crate) fn send_debug_cmd(ui: &UiRc, cmd: &str) {
    if cmd == "terminar" {
        if let Some(run) = ui.borrow_mut().execucao.run.as_mut() {
            run.stopping = true;
            run.paused = None;
        }
        ui.borrow().execucao.console.cancel_input();
        clear_debug_views(ui);
        finish_execution_view(ui, true);
        set_debug_buttons(ui, false, false);
        set_status(ui, "parando…");
    } else if ui
        .borrow()
        .execucao
        .run
        .as_ref()
        .is_some_and(|run| run.stopping)
    {
        return;
    }
    let sent = match &ui.borrow().execucao.run {
        Some(run) => run.debug_cmd.send(cmd.to_string()).is_ok(),
        None => false,
    };
    // Resuming: the program runs until the next pause, so the step buttons
    // are off and the stopped line is no longer current.
    if sent && matches!(cmd, "continuar" | "proximo" | "entrar" | "sair") {
        if let Some(run) = ui.borrow_mut().execucao.run.as_mut() {
            run.paused = None;
        }
        set_debug_buttons(ui, false, true);
        clear_debug_views(ui);
        set_status(ui, "executando… (Parar encerra)");
    }
}

pub(crate) fn set_debug_buttons(ui: &UiRc, paused: bool, running: bool) {
    let win = ui.borrow().window.clone();
    set_action_enabled(&win, "continuar", paused);
    set_action_enabled(&win, "proximo", paused);
    set_action_enabled(&win, "entrar", paused);
    set_action_enabled(&win, "sair", paused);
    set_action_enabled(&win, "parar", running);
    let barra = &ui.borrow().depuracao.barra_exec;
    barra.caixa.set_visible(running);
    barra.estado.set_text(if !running {
        ""
    } else if paused {
        "pausado"
    } else {
        "executando"
    });
    // Actions disable the buttons, but the theme paints `button` with a
    // solid color and the inactive ones would look the same as Parar.
    let apagado = 0.42;
    let aceso = 1.0;
    barra
        .continuar
        .set_opacity(if paused { aceso } else { apagado });
    barra
        .proximo
        .set_opacity(if paused { aceso } else { apagado });
    barra
        .entrar
        .set_opacity(if paused { aceso } else { apagado });
    barra.sair.set_opacity(if paused { aceso } else { apagado });
    barra
        .parar
        .set_opacity(if running { aceso } else { apagado });
}

pub(crate) fn toggle_breakpoint_here(ui: &UiRc) {
    if let Some(doc) = current_doc(ui) {
        let line = doc.cursor().line() as u32 + 1;
        toggle_breakpoint(ui, &doc, line);
    }
}

pub(crate) fn toggle_breakpoint(ui: &UiRc, doc: &Rc<Doc>, line: u32) {
    doc.toggle_breakpoint(line);
    sessao::salvar(ui);
    if !matches!(ui.borrow().execucao.execution.availability(), Ok(())) {
        set_status(
            ui,
            "ponto de parada alterado para a próxima execução; a execução atual usa os pontos iniciais",
        );
    }
}

pub(crate) fn fill_debug_views(ui: &UiRc, paused: &expressa::runtime::DebugPaused) {
    {
        let u = ui.borrow();
        u.depuracao.vars_store.clear();
        u.depuracao.stack_store.clear();
        for v in &paused.vars {
            let iter = u.depuracao.vars_store.append(None);
            u.depuracao
                .vars_store
                .set(&iter, &[(0, &v.scope), (1, &v.name), (2, &v.value)]);
        }
        for f in &paused.stack {
            let loc = format!("{}:{}", f.file, f.line);
            let iter = u.depuracao.stack_store.append(None);
            u.depuracao
                .stack_store
                .set(&iter, &[(0, &f.name), (1, &loc)]);
        }
    }
    refresh_watch_values(ui, &paused.vars);
}

pub(crate) fn fill_error_stack(ui: &UiRc, frames: &[exec::StackFrame]) {
    let u = ui.borrow();
    u.depuracao.stack_store.clear();
    for f in frames {
        let loc = format!("{}:{}", f.file, f.line);
        let name = if f.repeticoes > 0 {
            format!("{} (×{})", f.name, f.repeticoes + 1)
        } else {
            f.name.clone()
        };
        let iter = u.depuracao.stack_store.append(None);
        u.depuracao.stack_store.set(&iter, &[(0, &name), (1, &loc)]);
    }
}

pub(crate) fn clear_debug_views(ui: &UiRc) {
    if let Some(viewer) = ui.borrow().execucao.source_viewer.as_ref() {
        viewer.doc.clear_debug_line();
    }
    {
        let u = ui.borrow();
        u.depuracao.vars_store.clear();
        u.depuracao.stack_store.clear();
    }
    for d in all_docs(ui) {
        d.clear_debug_line();
    }
}

pub(crate) fn handle_debug_pause(ui: &UiRc, p: &expressa::runtime::DebugPaused) -> bool {
    if !ui
        .borrow()
        .execucao
        .run
        .as_ref()
        .is_some_and(|run| !run.stopping)
    {
        return false;
    }
    if let Some(run) = ui.borrow_mut().execucao.run.as_mut() {
        run.paused = Some((p.file.clone(), p.line, p.error.is_some()));
    }
    fill_debug_views(ui, p);
    set_debug_buttons(ui, true, true);
    show_pause(ui, p);
    match &p.error {
        Some(e) => set_status(
            ui,
            &format!(
                "erro em {}:{}: {e} — veja as variáveis; Continuar encerra",
                p.file, p.line
            ),
        ),
        None => set_status(
            ui,
            &format!("pausado {}:{}  {}", p.file, p.line, p.source_line.trim()),
        ),
    }
    true
}

/// Show where the debugger stopped, opening that file if needed.
pub(crate) fn show_pause(ui: &UiRc, p: &expressa::runtime::DebugPaused) {
    remember_execution_location(ui, &p.file, p.line, p.error.is_some());
    for d in all_docs(ui) {
        d.clear_debug_line();
    }
    if !show_running_source(ui, &p.file, p.line, p.error.is_some()) {
        set_status(
            ui,
            &format!(
                "pausado em {}:{} (fonte da execução indisponível)",
                p.file, p.line
            ),
        );
    }
}

pub(crate) fn close_current_tab(ui: &UiRc) {
    if let Some(doc) = current_doc(ui) {
        close_doc(ui, &doc);
    } else {
        return_to_editor(ui, true);
    }
}

pub(crate) fn execution_editor(ui: &UiRc, path: &str) -> Option<Rc<Doc>> {
    let original = ui
        .borrow()
        .execucao
        .execution_editors
        .get(path)
        .and_then(std::rc::Weak::upgrade);
    original
        .filter(|d| all_docs(ui).iter().any(|open| Rc::ptr_eq(d, open)))
        .or_else(|| doc_by_name(ui, path))
}

pub(crate) fn execution_entry_doc(ui: &UiRc) -> Option<Rc<Doc>> {
    current_doc(ui).or_else(|| {
        let path = {
            let u = ui.borrow();
            let viewer = u.execucao.source_viewer.as_ref()?;
            (u.notebook.current_page() == u.notebook.page_num(&viewer.page))
                .then(|| viewer.source.path.clone())?
        };
        execution_editor(ui, &path)
    })
}

/// Keep execution sources outside the editable document/recovery lifecycle.
pub(crate) fn close_source_viewer(ui: &UiRc) {
    let viewer = ui.borrow_mut().execucao.source_viewer.take();
    if let Some(viewer) = viewer {
        viewer.doc.cancel_pending();
        let notebook = ui.borrow().notebook.clone();
        if let Some(n) = notebook.page_num(&viewer.page) {
            notebook.remove_page(Some(n));
        }
    }
}

/// Close the temporary page without taking focus from another editor the user
/// already selected. Stop explicitly returns keyboard focus to editable code.
pub(crate) fn return_to_editor(ui: &UiRc, force_focus: bool) {
    let (path, was_active) = {
        let u = ui.borrow();
        match u.execucao.source_viewer.as_ref() {
            Some(viewer) => (
                Some(viewer.source.path.clone()),
                u.notebook.current_page() == u.notebook.page_num(&viewer.page),
            ),
            None => (None, false),
        }
    };
    let target = current_doc(ui).or_else(|| path.as_deref().and_then(|p| execution_editor(ui, p)));
    close_source_viewer(ui);
    if (force_focus || was_active)
        && let Some(doc) = target
            .or_else(|| current_doc(ui))
            .or_else(|| all_docs(ui).into_iter().next())
    {
        switch_to(ui, &doc);
    }
}

pub(crate) fn finish_execution_view(ui: &UiRc, force_focus: bool) {
    for doc in all_docs(ui) {
        doc.set_running_source(None);
    }
    return_to_editor(ui, force_focus);
}

pub(crate) fn remember_execution_location(ui: &UiRc, file: &str, line: u32, error: bool) {
    let path = debug_sources::normalize_path(file);
    let available = ui.borrow().execucao.execution_sources.contains_key(&path);
    ui.borrow_mut().execucao.last_execution_location = Some((path, line, error));
    set_action_enabled(&ui.borrow().window, "ver-fonte-executada", available);
}

pub(crate) fn inspect_execution_source(ui: &UiRc) {
    let location = ui.borrow().execucao.last_execution_location.clone();
    if let Some((path, line, error)) = location
        && show_running_source(ui, &path, line, error)
    {
        return;
    }
    set_status(ui, "fonte da última execução indisponível");
}

/// Execution errors belong in editable code. A diverged document must not be
/// given a line marker from an earlier source revision or an automatic viewer.
pub(crate) fn navigate_execution_error(ui: &UiRc, file: &str, line: u32, automatic: bool) {
    let path = debug_sources::normalize_path(file);
    let source = ui.borrow().execucao.execution_sources.get(&path).cloned();
    let Some(source) = source else {
        if !automatic {
            open_path(ui, file);
        }
        return;
    };
    if let Some(doc) = execution_editor(ui, &path) {
        highlight_execution_error(ui, &doc, &source, line);
    } else if !automatic {
        open_path_then(ui, file, move |ui, doc| {
            if let Some(doc) = doc
                && ui
                    .borrow()
                    .execucao
                    .execution_sources
                    .get(&source.path)
                    .is_some_and(|current| Arc::ptr_eq(current, &source))
            {
                highlight_execution_error(ui, &doc, &source, line);
            }
        });
    }
}

pub(crate) fn highlight_execution_error(
    ui: &UiRc,
    doc: &Rc<Doc>,
    source: &debug_sources::Source,
    line: u32,
) {
    if doc.text() == source.text {
        switch_to(ui, doc);
        doc.show_debug_line(line, true);
    } else {
        let message =
            "o código foi alterado desde esta execução; consulte Ver → Ver código executado";
        set_status(ui, message);
    }
}

pub(crate) fn show_running_source(ui: &UiRc, path: &str, line: u32, error: bool) -> bool {
    let source = ui
        .borrow()
        .execucao
        .execution_sources
        .get(&debug_sources::normalize_path(path))
        .cloned();
    let Some(source) = source else { return false };
    let existing = ui.borrow().execucao.source_viewer.clone();
    let viewer = if let Some(viewer) =
        existing.filter(|v| v.source.revision == source.revision && v.source.path == source.path)
    {
        viewer
    } else {
        close_source_viewer(ui);
        let viewer = {
            let u = ui.borrow();
            Rc::new(debug_sources::Viewer::new(
                Arc::clone(&source),
                &u.languages,
                &u.search_settings,
            ))
        };
        let (dark, scheme) = theme_scheme(ui);
        viewer.doc.set_theme(scheme.as_ref(), dark);
        for (file, line) in &ui.borrow().execucao.execution_breakpoints {
            if file == &source.path {
                viewer.doc.toggle_breakpoint(*line);
            }
        }
        let weak = Rc::downgrade(ui);
        let path = source.path.clone();
        viewer.edit.connect_clicked(move |_| {
            let Some(ui) = weak.upgrade() else { return };
            // Prefer the original tab even after Save As/rename.
            if let Some(doc) = execution_editor(&ui, &path) {
                switch_to(&ui, &doc);
            } else {
                open_path(&ui, &path);
            }
        });
        let weak = Rc::downgrade(ui);
        viewer.close.connect_clicked(move |_| {
            if let Some(ui) = weak.upgrade() {
                return_to_editor(&ui, true);
            }
        });
        let notebook = ui.borrow().notebook.clone();
        notebook.append_page(
            &viewer.page,
            Some(&Label::new(Some(&format!("Execução: {}", source.path)))),
        );
        ui.borrow_mut().execucao.source_viewer = Some(Rc::clone(&viewer));
        viewer
    };
    if ui.borrow().execucao.run.is_none() {
        viewer.finish();
    }
    let notebook = ui.borrow().notebook.clone();
    if let Some(n) = notebook.page_num(&viewer.page) {
        notebook.set_current_page(Some(n));
    }
    viewer.doc.show_debug_line(line, error);
    viewer.doc.view.grab_focus();
    true
}
