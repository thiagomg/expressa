//! Open tabs: help, open/save/close, theme, and search in the current file.

use std::cell::RefCell;
use std::rc::{Rc, Weak};

use gtk::gio::prelude::*;
use gtk::glib;
use gtk::prelude::*;
use gtk::{Box as GtkBox, Button, Entry, Orientation};
use sourceview5::prelude::*;

use crate::assist::{Help, Target, ViewHelp};
use crate::docs::Doc;
use crate::documentos::{CloseDecision, Documents, SaveOutcome, UnsavedChoice};
use crate::store::Files;
use crate::*;

// ── Help backed by the open tabs and the server ───────────────────────────

pub(crate) fn make_help(ui: &UiRc) -> Rc<Help> {
    let w1: Weak<RefCell<Ui>> = Rc::downgrade(ui);
    let w2 = w1.clone();
    let w3 = w1.clone();
    Rc::new(Help {
        file_text: Box::new(move |path| {
            let ui = w1.upgrade()?;
            let u = ui.try_borrow().ok()?;
            if let Some((d, _)) = u.docs.iter().find(|(d, _)| d.name() == path) {
                return Some(d.text());
            }
            if let Some(hit) = u.file_cache.borrow().get(path) {
                return hit.clone();
            }
            // On a server, fetch it in the background: this hover goes
            // without, the next one finds it in the cache.
            u.file_cache.borrow_mut().insert(path.to_string(), None);
            let (files, projeto, rel) = (u.files.clone(), u.project.clone(), path.to_string());
            let epoch = u.project_epoch;
            drop(u);
            let p = projeto.clone();
            let r = rel.clone();
            em_segundo_plano(
                &ui,
                move || files.read_file(&p, &r).ok(),
                move |ui, text| {
                    let u = ui.borrow();
                    if u.project_epoch == epoch {
                        u.file_cache.borrow_mut().insert(rel, text);
                    }
                },
            );
            None
        }),
        project_files: Box::new(move || {
            w2.upgrade()
                .and_then(|ui| ui.try_borrow().ok().map(|u| u.project_files.clone()))
                .unwrap_or_default()
        }),
        path_of: Box::new(move |buffer| {
            let ui = w3.upgrade()?;
            let u = ui.try_borrow().ok()?;
            u.docs
                .iter()
                .find(|(d, _)| &d.buffer == buffer)
                .map(|(d, _)| d.name())
        }),
    })
}

// ── Tabs ───────────────────────────────────────────────────────────────────

pub(crate) fn current_doc(ui: &UiRc) -> Option<Rc<Doc>> {
    let u = ui.try_borrow().ok()?;
    let page = u.notebook.nth_page(u.notebook.current_page())?;
    u.docs
        .iter()
        .find(|(d, _)| d.page.upcast_ref::<gtk::Widget>() == &page)
        .map(|(d, _)| Rc::clone(d))
}

pub(crate) fn doc_by_name(ui: &UiRc, name: &str) -> Option<Rc<Doc>> {
    documentos::find_by_name(&all_docs(ui), name)
}

/// Release the UI borrow before storage calls or buffer notifications.
pub(crate) fn with_documents<T>(ui: &UiRc, f: impl FnOnce(Documents<'_>) -> T) -> T {
    let (files, project) = {
        let u = ui.borrow();
        (u.files.clone(), u.project.clone())
    };
    let docs = all_docs(ui);
    f(Documents::new(&files, &project, &docs))
}

pub(crate) fn view_help(ui: &UiRc, doc: &Rc<Doc>) -> Option<Rc<ViewHelp>> {
    ui.borrow()
        .docs
        .iter()
        .find(|(d, _)| Rc::ptr_eq(d, doc))
        .map(|(_, v)| Rc::clone(v))
}

pub(crate) fn all_docs(ui: &UiRc) -> Vec<Rc<Doc>> {
    ui.borrow().docs.iter().map(|(d, _)| Rc::clone(d)).collect()
}

pub(crate) fn dirty_docs(ui: &UiRc) -> Vec<Rc<Doc>> {
    all_docs(ui).into_iter().filter(|d| d.is_dirty()).collect()
}

pub(crate) fn switch_to(ui: &UiRc, doc: &Rc<Doc>) {
    let nb = ui.borrow().notebook.clone();
    if let Some(n) = nb.page_num(&doc.page) {
        nb.set_current_page(Some(n));
    }
    doc.view.grab_focus();
    refresh_title(ui);
}

/// New tab for `path` (or untitled) with `text`, wired to the editor.
pub(crate) fn open_doc(ui: &UiRc, path: Option<String>, text: &str) -> Rc<Doc> {
    let untitled = {
        let mut u = ui.borrow_mut();
        u.untitled_count += 1;
        if u.untitled_count == 1 {
            "sem-titulo.lep".to_string()
        } else {
            format!("sem-titulo-{}.lep", u.untitled_count)
        }
    };
    let doc = {
        let u = ui.borrow();
        Doc::new(path, untitled, text, &u.languages, &u.search_settings)
    };
    {
        let weak = Rc::downgrade(ui);
        *doc.on_diag.borrow_mut() = Some(Box::new(move |_| {
            if let Some(ui) = weak.upgrade() {
                refresh_diag_label(&ui);
            }
        }));
    }
    {
        let weak = Rc::downgrade(ui);
        doc.buffer.connect_modified_changed(move |_| {
            if let Some(ui) = weak.upgrade() {
                refresh_title(&ui);
            }
        });
    }
    {
        let weak = Rc::downgrade(ui);
        let buf = doc.buffer.clone();
        doc.buffer
            .connect_notify_local(Some("cursor-position"), move |_, _| {
                if let Some(ui) = weak.upgrade() {
                    refresh_cursor(&ui, &buf);
                }
            });
    }
    {
        let source = ui.borrow().execucao.run.as_ref().and_then(|_| {
            ui.borrow()
                .execucao
                .execution_sources
                .get(&debug_sources::normalize_path(&doc.name()))
                .cloned()
        });
        if source.is_some() {
            ui.borrow_mut().execucao.execution_editors.insert(
                debug_sources::normalize_path(&doc.name()),
                Rc::downgrade(&doc),
            );
        }
        doc.set_running_source(source);
        let weak_ui = Rc::downgrade(ui);
        let weak_doc = Rc::downgrade(&doc);
        doc.inspect_source.connect_clicked(move |_| {
            if let (Some(ui), Some(doc)) = (weak_ui.upgrade(), weak_doc.upgrade()) {
                let source = doc.running_source.borrow().clone();
                if let Some(source) = source {
                    let paused = ui
                        .borrow()
                        .execucao
                        .run
                        .as_ref()
                        .and_then(|run| run.paused.clone())
                        .filter(|(path, _, _)| path == &source.path);
                    let (_, line, error) =
                        paused.clone().unwrap_or((source.path.clone(), 1, false));
                    show_running_source(&ui, &source.path, line, error);
                    if paused.is_none()
                        && let Some(viewer) = ui.borrow().execucao.source_viewer.as_ref()
                    {
                        viewer.doc.clear_debug_line();
                    }
                }
            }
        });
    }
    let help = ui.borrow().help.clone().expect("help");
    let vh = {
        let weak = Rc::downgrade(&doc);
        assist::attach(&doc.view, &help, move |_, offset| {
            weak.upgrade()
                .and_then(|d| d.diag_at(offset))
                .map(|d| d.message)
        })
    };
    attach_editor_input(ui, &doc, &vh);
    {
        let ui_c = Rc::clone(ui);
        let weak = Rc::downgrade(&doc);
        doc.close_button.connect_clicked(move |_| {
            if let Some(d) = weak.upgrade() {
                close_doc(&ui_c, &d);
            }
        });
    }
    abas::ligar(ui, &doc);
    sessao::vigiar(ui, &doc);
    {
        let (dark, scheme) = theme_scheme(ui);
        doc.set_theme(scheme.as_ref(), dark);
    }
    ui.borrow_mut().docs.push((Rc::clone(&doc), vh));
    let nb = ui.borrow().notebook.clone();
    let n = nb.append_page(&doc.page, Some(&doc.tab));
    nb.set_tab_reorderable(&doc.page, true);
    nb.set_current_page(Some(n));
    doc.view.grab_focus();
    refresh_title(ui);
    refresh_cursor(ui, &doc.buffer);
    remember_arquivo(ui, &doc);
    doc
}

/// Mouse and keys of one editor view.
pub(crate) fn attach_editor_input(ui: &UiRc, doc: &Rc<Doc>, vh: &Rc<ViewHelp>) {
    let view = doc.view.clone();
    {
        // Click left of the text (line numbers): breakpoint. Ctrl+click on a
        // name: go to its definition.
        let ui_g = Rc::clone(ui);
        let weak = Rc::downgrade(doc);
        // The line-number gutter is a child widget that takes the click:
        // look at it first (Capture), without claiming it.
        let gutter = gtk::GestureClick::new();
        gutter.set_button(1);
        gutter.set_propagation_phase(gtk::PropagationPhase::Capture);
        {
            let ui_g = Rc::clone(&ui_g);
            let weak = weak.clone();
            gutter.connect_pressed(move |_, _, x, y| {
                let Some(doc) = weak.upgrade() else { return };
                let gutter_width =
                    sourceview5::prelude::ViewExt::gutter(&doc.view, gtk::TextWindowType::Left)
                        .width();
                if (x as i32) < gutter_width {
                    let (_, by) = doc.view.window_to_buffer_coords(
                        gtk::TextWindowType::Widget,
                        x as i32,
                        y as i32,
                    );
                    let (it, _) = doc.view.line_at_y(by);
                    toggle_breakpoint(&ui_g, &doc, it.line() as u32 + 1);
                }
            });
        }
        view.add_controller(gutter);
        let click = gtk::GestureClick::new();
        click.set_button(1);
        click.connect_released(move |g, _, x, y| {
            let Some(doc) = weak.upgrade() else { return };
            if g.current_event_state()
                .contains(gtk::gdk::ModifierType::CONTROL_MASK)
            {
                let (tx, ty) = doc.view.window_to_buffer_coords(
                    gtk::TextWindowType::Widget,
                    x as i32,
                    y as i32,
                );
                if let Some(it) = doc.view.iter_at_location(tx, ty) {
                    go_to_definition(&ui_g, &doc, &it);
                }
            }
        });
        view.add_controller(click);
    }
    // Editing keys: Capture so they run before the TextView's own bindings
    // (Tab inserts a tab, Ctrl+/ selects all). Only these keys match.
    let keys = gtk::ShortcutController::new();
    keys.set_propagation_phase(gtk::PropagationPhase::Capture);
    let b = doc.buffer.clone();
    {
        let b = b.clone();
        add_shortcut(&keys, "Tab", move || indent::tab(&b));
    }
    for accel in ["<Shift>Tab", "<Shift>ISO_Left_Tab"] {
        let b = b.clone();
        add_shortcut(&keys, accel, move || indent::backtab(&b));
    }
    let on = |accel: &str, f: fn(&sourceview5::Buffer)| {
        let b = b.clone();
        add_shortcut(&keys, accel, move || f(&b));
    };
    on("<Control><Alt>backslash", indent::indent_region_or_line);
    on("<Primary>slash", editing::toggle_comment);
    on("<Primary>KP_Divide", editing::toggle_comment);
    on("<Primary>d", editing::duplicate_lines);
    on("<Alt>Up", |b| editing::move_lines(b, true));
    on("<Alt>Down", |b| editing::move_lines(b, false));
    {
        let weak = Rc::downgrade(doc);
        let vh = Rc::downgrade(vh);
        add_shortcut(&keys, "<Control>space", move || {
            if let (Some(doc), Some(vh)) = (weak.upgrade(), vh.upgrade()) {
                vh.show_completion(&doc.view);
            }
        });
    }
    {
        let ui_h = Rc::clone(ui);
        let weak = Rc::downgrade(doc);
        let vh = Rc::downgrade(vh);
        add_shortcut(&keys, "F1", move || {
            let (Some(doc), Some(vh)) = (weak.upgrade(), vh.upgrade()) else {
                return;
            };
            let help = ui_h.borrow().help.clone().expect("help");
            if !assist::show_info(&doc.view, &vh, &help) {
                open_ajuda(&ui_h);
            }
        });
    }
    {
        let ui_d = Rc::clone(ui);
        let weak = Rc::downgrade(doc);
        add_shortcut(&keys, "F12", move || {
            if let Some(doc) = weak.upgrade() {
                go_to_definition(&ui_d, &doc, &doc.cursor());
            }
        });
    }
    {
        let vh = Rc::downgrade(vh);
        let weak = Rc::downgrade(doc);
        add_shortcut_if(&keys, "Escape", move || {
            let (Some(vh), Some(doc)) = (vh.upgrade(), weak.upgrade()) else {
                return false;
            };
            vh.dismiss(&doc.view)
        });
    }
    view.add_controller(keys);
}

pub(crate) fn new_untitled_tab(ui: &UiRc) {
    open_doc(ui, None, "escreva(\"Olá\")\n");
    set_status(ui, "arquivo novo (ainda não salvo)");
}

/// Open `path` from the project (or switch to its tab).
pub(crate) fn open_path(ui: &UiRc, path: &str) {
    open_path_then(ui, path, |_, _| {});
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum OpenFailure {
    Cancelled,
    Failed,
}

pub(crate) fn open_path_then(
    ui: &UiRc,
    path: &str,
    done: impl FnOnce(&UiRc, Option<Rc<Doc>>) + 'static,
) {
    open_path_status(ui, path, move |ui, result| done(ui, result.ok()));
}

pub(crate) fn next_file_operation(ui: &UiRc, path: &str) -> u64 {
    let mut u = ui.borrow_mut();
    u.next_file_operation += 1;
    let id = u.next_file_operation;
    u.file_operations.insert(path.replace('\\', "/"), id);
    id
}

pub(crate) fn invalidate_file_operations(ui: &UiRc, path: &str) {
    let path = path.replace('\\', "/");
    let paths: Vec<_> = ui
        .borrow()
        .file_operations
        .keys()
        .filter(|p| **p == path || p.starts_with(&format!("{path}/")))
        .cloned()
        .collect();
    for path in paths {
        next_file_operation(ui, &path);
    }
}

pub(crate) fn open_path_status(
    ui: &UiRc,
    path: &str,
    done: impl FnOnce(&UiRc, Result<Rc<Doc>, OpenFailure>) + 'static,
) {
    if let Some(doc) = doc_by_name(ui, path) {
        next_file_operation(ui, path);
        switch_to(ui, &doc);
        done(ui, Ok(doc));
        return;
    }
    let (files, project) = {
        let u = ui.borrow();
        (u.files.clone(), u.project.clone())
    };
    let path_owned = path.to_string();
    open_path_read(
        ui,
        path,
        move || {
            files
                .current_text(&project, &path_owned)?
                .ok_or_else(|| format!("arquivo `{path_owned}` não existe"))
        },
        done,
    );
}

pub(crate) fn open_path_read(
    ui: &UiRc,
    path: &str,
    read: impl FnOnce() -> Result<store::TextSnapshot, String> + Send + 'static,
    done: impl FnOnce(&UiRc, Result<Rc<Doc>, OpenFailure>) + 'static,
) {
    let path = path.replace('\\', "/");
    let operation = next_file_operation(ui, &path);
    async_result(
        ui,
        &format!("Abrindo {path}"),
        true,
        read,
        move |ui, result| {
            if ui.borrow().file_operations.get(&path) != Some(&operation) {
                done(ui, Err(OpenFailure::Cancelled));
                return;
            }
            match result {
                Ok(snapshot) => {
                    // A second request or the user may have opened/edited the same
                    // file meanwhile. Never replace that buffer with this snapshot.
                    let doc = doc_by_name(ui, &path).unwrap_or_else(|| {
                        let doc = open_doc(ui, Some(path.clone()), &snapshot.text);
                        *doc.revision.borrow_mut() = snapshot.revision;
                        doc
                    });
                    switch_to(ui, &doc);
                    set_status(ui, &format!("aberto {path}"));
                    done(ui, Ok(doc));
                }
                Err(e) => {
                    set_status(ui, &format!("abrir {path}: {e}"));
                    done(
                        ui,
                        Err(if e == "operação cancelada" {
                            OpenFailure::Cancelled
                        } else {
                            OpenFailure::Failed
                        }),
                    );
                }
            }
        },
    );
}

pub(crate) fn goto_file_line(ui: &UiRc, file: &str, line: u32) {
    open_path_then(ui, file, move |_, doc| {
        if let Some(doc) = doc {
            doc.goto_line(line);
        }
    });
}

pub(crate) fn remove_tab(ui: &UiRc, doc: &Rc<Doc>) {
    doc.cancel_pending();
    invalidate_file_operations(ui, &doc.name());
    let (nb, vh) = {
        let mut u = ui.borrow_mut();
        let pos = u.docs.iter().position(|(d, _)| Rc::ptr_eq(d, doc));
        let vh = pos.map(|p| u.docs.remove(p).1);
        (u.notebook.clone(), vh)
    };
    if let Some(vh) = vh {
        vh.detach();
    }
    if let Some(n) = nb.page_num(&doc.page) {
        nb.remove_page(Some(n));
    }
    doc.vigia.replace(None);
    doc.recovery.close();
    forget_recovery_async(ui, doc);
    refresh_title(ui);
    refresh_diag_label(ui);
    sessao::salvar(ui);
}

/// The user closes `doc` (asking about unsaved changes first). Unlike
/// `remove_tab`, it can be reopened with Ctrl+Shift+T.
pub(crate) fn close_doc(ui: &UiRc, doc: &Rc<Doc>) {
    if documentos::close_decision(doc) == CloseDecision::Close {
        abas::lembrar_fechada(ui, doc);
        remove_tab(ui, doc);
        return;
    }
    let ui_c = Rc::clone(ui);
    let d = Rc::clone(doc);
    confirm_unsaved(ui, vec![Rc::clone(doc)], move || {
        abas::lembrar_fechada(&ui_c, &d);
        remove_tab(&ui_c, &d);
    });
}

/// Save one tab; an untitled one asks for a name. False if not saved.
pub(crate) fn save_doc(ui: &UiRc, doc: &Rc<Doc>) {
    save_doc_then(ui, doc, |_| {});
}

pub(crate) fn save_doc_then(ui: &UiRc, doc: &Rc<Doc>, done: impl FnOnce(bool) + 'static) {
    let original_name = doc.name();
    let new_name = if doc.is_untitled() {
        let window = ui.borrow().window.clone();
        perguntar_linha(&window, &format!("Salvar `{}` como:", doc.untitled))
    } else {
        None
    };
    if doc.name() != original_name || !all_docs(ui).iter().any(|d| Rc::ptr_eq(d, doc)) {
        done(false);
        return;
    }
    save_request(ui, doc, new_name.as_deref(), doc.is_untitled(), None, done);
}

pub(crate) fn save_request(
    ui: &UiRc,
    doc: &Rc<Doc>,
    new_name: Option<&str>,
    creating: bool,
    explicit: Option<&store::Expected>,
    done: impl FnOnce(bool) + 'static,
) {
    if doc.saving.get() {
        set_status(ui, "esta aba já está sendo salva");
        done(false);
        return;
    }
    let request = match with_documents(ui, |documents| {
        documents.prepare_save(doc, new_name, creating, explicit)
    }) {
        Ok(Some(request)) => request,
        Ok(None) => {
            done(finish_save(ui, doc, Ok(SaveOutcome::Cancelled)));
            return;
        }
        Err(e) => {
            set_status(ui, &e);
            done(false);
            return;
        }
    };
    let (files, project) = {
        let u = ui.borrow();
        (u.files.clone(), u.project.clone())
    };
    let work = request.clone();
    save_prepared(ui, doc, request, move || work.write(&files, &project), done);
}

pub(crate) fn save_prepared(
    ui: &UiRc,
    doc: &Rc<Doc>,
    request: documentos::SaveRequest,
    write: impl FnOnce() -> Result<store::Saved, store::SaveError> + Send + 'static,
    done: impl FnOnce(bool) + 'static,
) {
    doc.saving.set(true);
    doc.refresh_label();
    let weak = Rc::downgrade(doc);
    async_result(
        ui,
        &format!("Salvando {}", request.name),
        false,
        move || Ok(write()),
        move |ui, result| {
            let Some(doc) = weak.upgrade() else {
                done(false);
                return;
            };
            doc.saving.set(false);
            doc.refresh_label();
            if !all_docs(ui).iter().any(|open| Rc::ptr_eq(open, &doc)) {
                done(false);
                return;
            }
            if request.creating
                && matches!(&result, Ok(Ok(_)))
                && all_docs(ui).iter().any(|other| {
                    !Rc::ptr_eq(other, &doc)
                        && exec::same_project_path(&other.name(), &request.name)
                })
            {
                doc.buffer.set_modified(true);
                set_status(
                    ui,
                    "o arquivo foi salvo, mas o destino foi aberto em outra aba; sua aba foi preservada",
                );
                done(false);
                return;
            }
            let result = result.and_then(|receipt| request.complete(&doc, receipt));
            let succeeded = finish_save(ui, &doc, result);
            if succeeded && !doc.is_dirty() {
                forget_recovery_async(ui, &doc);
            }
            if succeeded && doc.is_dirty() {
                set_status(ui, "arquivo salvo; há novas alterações na aba");
            }
            sessao::check_deferred(ui, &doc);
            done(succeeded && !doc.is_dirty());
        },
    );
}

pub(crate) fn reload_doc(ui: &UiRc, doc: &Rc<Doc>) {
    if doc.saving.get() {
        set_status(ui, "aguarde a gravação desta aba antes de recarregar");
        return;
    }
    let (files, project) = {
        let u = ui.borrow();
        (u.files.clone(), u.project.clone())
    };
    let name = doc.name();
    let path = name.clone();
    let edit = doc.edit_epoch.get();
    let io = doc.io_epoch.get();
    let weak = Rc::downgrade(doc);
    async_result(
        ui,
        &format!("Recarregando {name}"),
        true,
        move || {
            files
                .current_text(&project, &path)?
                .ok_or("o arquivo foi removido; a aba foi preservada".into())
        },
        move |ui, result| {
            let Some(doc) = weak.upgrade() else {
                return;
            };
            if doc.name() != name
                || doc.io_epoch.get() != io
                || doc.edit_epoch.get() != edit
                || !all_docs(ui).iter().any(|open| Rc::ptr_eq(open, &doc))
            {
                set_status(
                    ui,
                    "a aba mudou durante a leitura; seu texto foi preservado",
                );
                return;
            }
            match result {
                Ok(snapshot) => {
                    doc.recarregar_snapshot(&snapshot.text, snapshot.revision);
                    forget_recovery_async(ui, &doc);
                    set_status(ui, &format!("recarregado {name}"));
                }
                Err(e) => set_status(ui, &e),
            }
        },
    );
}

pub(crate) fn forget_recovery_async(ui: &UiRc, doc: &Rc<Doc>) {
    forget_recovery_docs_async(ui, vec![doc.clone()]);
}

pub(crate) fn forget_recovery_docs_async(ui: &UiRc, docs: Vec<Rc<Doc>>) {
    let known: Vec<_> = docs
        .iter()
        .map(|d| d.copia_recuperacao.borrow().clone())
        .collect();
    let work: Vec<_> = docs
        .iter()
        .zip(known.iter())
        .map(|(d, b)| (d.recovery.clone(), b.clone()))
        .collect();
    let weak: Vec<_> = docs.iter().map(Rc::downgrade).collect();
    async_on(
        background::persistence(),
        true,
        ui,
        "",
        false,
        move || {
            Ok(work
                .into_iter()
                .map(|(owner, backup)| owner.retire(backup.as_ref()))
                .collect::<Vec<_>>())
        },
        move |ui, results| match results {
            Ok(results) => {
                for ((doc, known), result) in weak.into_iter().zip(known).zip(results) {
                    match result {
                        Ok(()) => {
                            if let Some(doc) = doc.upgrade() {
                                let same =
                                    match (doc.copia_recuperacao.borrow().as_ref(), known.as_ref())
                                    {
                                        (Some(a), Some(b)) => {
                                            a.id == b.id && a.revision == b.revision
                                        }
                                        _ => false,
                                    };
                                if same {
                                    doc.copia_recuperacao.borrow_mut().take();
                                    doc.resumo_da_copia.set(None);
                                    doc.refresh_label();
                                }
                            }
                        }
                        Err(e) => ui
                            .borrow()
                            .execucao
                            .console
                            .stderr(&format!("aviso: cópia de recuperação preservada: {e}\n")),
                    }
                }
            }
            Err(e) => ui
                .borrow()
                .execucao
                .console
                .stderr(&format!("aviso: {e}\n")),
        },
    );
}

pub(crate) fn save_documents(ui: &UiRc, docs: Vec<Rc<Doc>>, then: impl FnOnce() + 'static) {
    let unrelated = Rc::new(
        dirty_docs(ui)
            .into_iter()
            .filter(|d| !docs.iter().any(|target| Rc::ptr_eq(d, target)))
            .collect::<Vec<_>>(),
    );
    fn next(
        ui: &UiRc,
        docs: Rc<Vec<Rc<Doc>>>,
        unrelated: Rc<Vec<Rc<Doc>>>,
        index: usize,
        then: Box<dyn FnOnce()>,
    ) {
        if index == docs.len() {
            if dirty_docs(ui)
                .iter()
                .all(|d| unrelated.iter().any(|other| Rc::ptr_eq(d, other)))
            {
                then();
            } else {
                set_status(ui, "há novas alterações; confira as abas antes de fechar");
            }
            return;
        }
        let next_ui = Rc::clone(ui);
        let rest = docs.clone();
        save_doc_then(ui, &docs[index], move |saved| {
            if saved {
                next(&next_ui, rest, unrelated, index + 1, then);
            }
        });
    }
    next(ui, Rc::new(docs), unrelated, 0, Box::new(then));
}

pub(crate) fn finish_save(ui: &UiRc, doc: &Rc<Doc>, outcome: Result<SaveOutcome, String>) -> bool {
    match outcome {
        Ok(SaveOutcome::Saved { name, warning }) => {
            saved(ui, doc, &name);
            if let Some(warning) = warning {
                set_status(ui, "arquivo salvo com aviso — veja o console");
                ui.borrow()
                    .execucao
                    .console
                    .stderr(&format!("aviso: {warning}\n"));
            }
            true
        }
        Ok(SaveOutcome::Cancelled) => false,
        Ok(SaveOutcome::Conflict { name }) => {
            set_status(
                ui,
                &format!("conflito ao salvar {name}; a aba foi preservada"),
            );
            conflitos::mostrar(ui, doc);
            false
        }
        Err(e) => {
            set_status(ui, &e);
            false
        }
    }
}

/// UI bookkeeping after the document controller completed a save.
pub(crate) fn saved(ui: &UiRc, doc: &Rc<Doc>, name: &str) {
    // A first save gives the tab a file to watch.
    if doc.vigia.borrow().is_none() {
        sessao::vigiar(ui, doc);
    }
    ui.borrow().file_cache.borrow_mut().remove(name);
    refresh_tree(ui);
    refresh_title(ui);
    set_status(ui, &format!("salvo {name}"));
}

/// Unsaved changes in `docs`: Salvar (all), Descartar or Cancelar. `then`
/// runs unless cancelled or a save failed.
pub(crate) fn confirm_unsaved(ui: &UiRc, docs: Vec<Rc<Doc>>, then: impl FnOnce() + 'static) {
    let window = ui.borrow().window.clone();
    let names: Vec<String> = docs.iter().map(|d| d.name()).collect();
    let (title, detail) = if names.len() == 1 {
        (
            format!("Salvar as alterações em `{}`?", names[0]),
            "Se não salvar, as alterações se perdem.".to_string(),
        )
    } else {
        (
            "Há arquivos com alterações não salvas".to_string(),
            names.join("\n"),
        )
    };
    let then = RefCell::new(Some(then));
    #[allow(deprecated)]
    let dlg = gtk::MessageDialog::builder()
        .transient_for(&window)
        .modal(true)
        .message_type(gtk::MessageType::Warning)
        .buttons(gtk::ButtonsType::None)
        .text(&title)
        .secondary_text(&detail)
        .build();
    #[allow(deprecated)]
    {
        dlg.add_button("Cancelar", gtk::ResponseType::Cancel);
        dlg.add_button("Descartar", gtk::ResponseType::Reject);
        dlg.add_button(
            if names.len() == 1 {
                "Salvar"
            } else {
                "Salvar todos"
            },
            gtk::ResponseType::Accept,
        );
        dlg.set_default_response(gtk::ResponseType::Accept);
    }
    let ui_c = Rc::clone(ui);
    #[allow(deprecated)]
    dlg.connect_response(move |d, resp| {
        d.close();
        let choice = match resp {
            gtk::ResponseType::Reject => UnsavedChoice::Discard,
            gtk::ResponseType::Accept => UnsavedChoice::Save,
            _ => UnsavedChoice::Cancel,
        };
        match choice {
            UnsavedChoice::Save => {
                if let Some(f) = then.borrow_mut().take() {
                    save_documents(&ui_c, docs.clone(), f);
                }
            }
            UnsavedChoice::Discard => {
                if let Some(f) = then.borrow_mut().take() {
                    f();
                }
            }
            UnsavedChoice::Cancel => {}
        }
    });
    dlg.present();
}

/// How the window names the open project: the folder, or `aluno / projeto`.
pub(crate) fn nome_do_projeto(u: &Ui) -> String {
    match &u.files {
        Files::Remote { projeto, .. } => format!("{} / {projeto}", u.project),
        _ => u.project.clone(),
    }
}

pub(crate) fn refresh_title(ui: &UiRc) {
    let doc = current_doc(ui);
    let Ok(u) = ui.try_borrow() else { return };
    let projeto = nome_do_projeto(&u);
    let title = match &doc {
        Some(doc) => {
            let star = if doc.is_dirty() { "● " } else { "" };
            format!("{star}{} — {projeto} — Expressa Aula", doc.name())
        }
        None if projeto.is_empty() => "Expressa Aula".to_string(),
        None => format!("{projeto} — Expressa Aula"),
    };
    u.window.set_title(Some(&title));
    set_action_enabled(&u.window, "sincronizacao", u.files.is_remote());
}

pub(crate) fn refresh_diag_label(ui: &UiRc) {
    let doc = current_doc(ui);
    let Ok(u) = ui.try_borrow() else { return };
    match doc.and_then(|d| d.diag.borrow().clone()) {
        Some(d) => {
            u.diag_label.set_markup(&format!(
                "<span foreground=\"#e53935\">⚠ linha {}: {}</span>",
                d.line,
                glib::markup_escape_text(&d.message)
            ));
        }
        None => u.diag_label.set_text(""),
    }
}

pub(crate) fn go_to_definition(ui: &UiRc, doc: &Rc<Doc>, at: &gtk::TextIter) {
    let help = ui.borrow().help.clone().expect("help");
    let mut ls = *at;
    ls.set_line_offset(0);
    let mut le = *at;
    if !le.ends_line() {
        le.forward_to_line_end();
    }
    let line = ls.text(&le).to_string();
    let col = ls.text(at).len();
    match assist::definition(&help, &doc.name(), &doc.text(), &line, col) {
        Some(Target::Line { file: None, line }) => doc.goto_line(line),
        Some(Target::Line {
            file: Some(f),
            line,
        }) => goto_file_line(ui, &f, line),
        Some(Target::File(f)) => {
            open_path(ui, &f);
        }
        Some(Target::Native(name)) => {
            doc.buffer.place_cursor(at);
            if let Some(vh) = view_help(ui, doc) {
                assist::show_info(&doc.view, &vh, &help);
            }
            set_status(ui, &format!("`{name}` é uma função nativa"));
        }
        None => {}
    }
}

// ── Prefs ──────────────────────────────────────────────────────────────────

pub(crate) fn zoom(ui: &UiRc, step: i32) {
    let mut u = ui.borrow_mut();
    u.prefs.zoom(step);
    u.font_css.set_size(u.prefs.font_size);
    u.prefs.save();
    let size = u.prefs.font_size;
    drop(u);
    set_status(ui, &format!("letra {size}pt (Ctrl+0 volta ao normal)"));
}

pub(crate) fn theme_scheme(ui: &UiRc) -> (bool, Option<sourceview5::StyleScheme>) {
    let u = ui.borrow();
    let dark = u.prefs.dark;
    let ids: &[&str] = if dark {
        &["expressa-aula-dark", "Adwaita-dark", "classic-dark"]
    } else {
        &["expressa-aula", "Adwaita", "classic"]
    };
    (dark, ids.iter().find_map(|id| u.schemes.scheme(id)))
}

pub(crate) fn apply_theme(ui: &UiRc) {
    let (dark, scheme) = theme_scheme(ui);
    let win = {
        let u = ui.borrow();
        u.theme_css.apply(dark);
        u.window.clone()
    };
    set_action_state_str(&win, "tema", if dark { "escuro" } else { "claro" });
    for d in all_docs(ui) {
        d.set_theme(scheme.as_ref(), dark);
    }
    if let Some(viewer) = ui.borrow().execucao.source_viewer.as_ref() {
        viewer.doc.set_theme(scheme.as_ref(), dark);
    }
    ui.borrow().execucao.console.set_dark(dark);
}

pub(crate) fn popdown_popovers(widget: &impl IsA<gtk::Widget>) {
    let widget = widget.as_ref();
    if let Ok(p) = widget.clone().downcast::<gtk::Popover>() {
        p.popdown();
    }
    let mut child = widget.first_child();
    while let Some(c) = child {
        let next = c.next_sibling();
        popdown_popovers(&c);
        child = next;
    }
}

pub(crate) fn set_dark(ui: &UiRc, dark: bool) {
    {
        let mut u = ui.borrow_mut();
        if u.prefs.dark == dark {
            return;
        }
        u.prefs.dark = dark;
        u.prefs.save();
    }
    let win = ui.borrow().window.clone();
    popdown_popovers(&win);
    apply_theme(ui);
}
// ── Search (current tab) ───────────────────────────────────────────────────

pub(crate) fn ir_para_linha(ui: &UiRc) {
    let Some(doc) = current_doc(ui) else { return };
    let parent = ui.borrow().window.clone();
    let win = gtk::Window::builder()
        .title("Ir para linha")
        .transient_for(&parent)
        .modal(true)
        .default_width(280)
        .build();
    let row = GtkBox::new(Orientation::Horizontal, 8);
    row.set_margin_top(12);
    row.set_margin_bottom(12);
    row.set_margin_start(12);
    row.set_margin_end(12);
    let entry = Entry::builder()
        .placeholder_text("número da linha")
        .hexpand(true)
        .build();
    let ok = Button::with_label("Ir");
    ok.add_css_class("suggested-action");
    let go = Rc::new({
        let entry = entry.clone();
        let win = win.clone();
        let doc = Rc::clone(&doc);
        move || {
            let t = entry.text();
            if let Ok(n) = t.trim().parse::<u32>()
                && n >= 1
            {
                doc.goto_line(n);
                doc.view.grab_focus();
            }
            win.close();
        }
    });
    ok.connect_clicked({
        let go = Rc::clone(&go);
        move |_| go()
    });
    entry.connect_activate({
        let go = Rc::clone(&go);
        move |_| go()
    });
    row.append(&entry);
    row.append(&ok);
    win.set_child(Some(&row));
    win.present();
    entry.grab_focus();
}

/// Which tab of the panel below the editor shows.
pub(crate) enum PaginaInferior {
    Saida = 0,
    Busca = 1,
    Repl = 2,
}

pub(crate) fn mostrar_pagina_inferior(ui: &UiRc, p: PaginaInferior) {
    ui.borrow().painel_inferior.set_current_page(Some(p as u32));
}

/// Ctrl+Shift+F: the Busca tab, starting from the selected text.
pub(crate) fn buscar_no_projeto(ui: &UiRc) {
    mostrar_pagina_inferior(ui, PaginaInferior::Busca);
    let selecionado = current_doc(ui).and_then(|d| {
        d.buffer
            .selection_bounds()
            .map(|(a, b)| d.buffer.text(&a, &b, false).to_string())
    });
    if let Some(b) = ui.borrow().busca.clone() {
        b.focar(selecionado);
    }
}

pub(crate) fn mostrar_repl(ui: &UiRc) {
    mostrar_pagina_inferior(ui, PaginaInferior::Repl);
    let repl = Rc::clone(&ui.borrow().repl);
    repl.focar();
}

pub(crate) fn show_search(ui: &UiRc) {
    let u = ui.borrow();
    u.search_revealer.set_reveal_child(true);
    u.search_entry.grab_focus();
}

pub(crate) fn hide_search(ui: &UiRc) {
    ui.borrow().search_revealer.set_reveal_child(false);
    if let Some(d) = current_doc(ui) {
        d.view.grab_focus();
    }
}

pub(crate) fn search_apply(ui: &UiRc) {
    let u = ui.borrow();
    let q = u.search_entry.text();
    if q.is_empty() {
        u.search_settings.set_search_text(None::<&str>);
    } else {
        u.search_settings.set_search_text(Some(q.as_str()));
    }
}

pub(crate) fn search_next(ui: &UiRc) {
    search_apply(ui);
    let Some(d) = current_doc(ui) else { return };
    let from = match d.buffer.selection_bounds() {
        Some((_, end)) => end,
        None => d.cursor(),
    };
    if let Some((a, b, _)) = d.search.forward(&from) {
        d.buffer.select_range(&a, &b);
        d.view.scroll_to_iter(&mut a.clone(), 0.2, false, 0.0, 0.0);
    } else {
        set_status(ui, &nao_encontrado(&d));
    }
}

/// Why a search found nothing: a broken regular expression says so.
pub(crate) fn nao_encontrado(d: &Doc) -> String {
    match d.search.regex_error() {
        Some(e) => format!("expressão regular inválida: {e}"),
        None => "não encontrado".to_string(),
    }
}

pub(crate) fn search_prev(ui: &UiRc) {
    search_apply(ui);
    let Some(d) = current_doc(ui) else { return };
    if let Some((a, b, _)) = d.search.backward(&d.cursor()) {
        d.buffer.select_range(&a, &b);
        d.view.scroll_to_iter(&mut a.clone(), 0.2, false, 0.0, 0.0);
    } else {
        set_status(ui, &nao_encontrado(&d));
    }
}

pub(crate) fn search_replace_one(ui: &UiRc) {
    search_apply(ui);
    let Some(d) = current_doc(ui) else { return };
    let repl = ui.borrow().replace_entry.text().to_string();
    if let Some((mut a, mut b)) = d.buffer.selection_bounds() {
        let _ = d.search.replace(&mut a, &mut b, &repl);
    }
    search_next(ui);
}

pub(crate) fn search_replace_all(ui: &UiRc) {
    search_apply(ui);
    let Some(d) = current_doc(ui) else { return };
    let repl = ui.borrow().replace_entry.text().to_string();
    match d.search.replace_all(&repl) {
        Ok(()) => set_status(ui, "substituições feitas"),
        Err(e) => set_status(ui, &format!("substituir: {e}")),
    }
}
