//! A single place to review remote files and locally preserved editor text.
//! Local projects keep automatic recovery without a synchronization interface.
use crate::pendencias;
use crate::{UiRc, all_docs, em_segundo_plano, set_status};
use gtk::prelude::*;
use std::rc::Rc;

pub fn close(ui: &UiRc) {
    let window = ui.borrow_mut().sync_window.take();
    if let Some(window) = window {
        window.close();
    }
}

pub fn parent(ui: &UiRc) -> gtk::Window {
    let u = ui.borrow();
    u.sync_window
        .clone()
        .unwrap_or_else(|| u.window.clone().upcast())
}

pub fn notify(ui: &UiRc) {
    let remote = ui.borrow().files.is_remote();
    if !remote {
        crate::pendencias_ui::notify_local_failure(ui);
        return;
    }
    let (identity, context) = {
        let u = ui.borrow();
        let Ok(identity) = u.files.known_identity(&u.project) else {
            return;
        };
        (identity, crate::sessao::synchronization_context(ui))
    };
    let expected = identity.clone();
    em_segundo_plano(
        ui,
        move || {
            let files = pendencias::scan_project(&pendencias::root(), &identity);
            let edits = context
                .map(|(store, owned)| {
                    let scan = store.scan();
                    scan.copies
                        .iter()
                        .any(|c| !c.backup.paths.iter().any(|p| owned.contains(p)))
                        || !scan.errors.is_empty()
                })
                .unwrap_or(false);
            !files.runs.is_empty() || !files.errors.is_empty() || edits
        },
        move |ui, pending| {
            let u = ui.borrow();
            if u.files.known_identity(&u.project).ok().as_ref() != Some(&expected) {
                return;
            }
            if pending {
                let message = "Há arquivos ou edições para revisar em Arquivo → Sincronização…";
                u.console.stderr(&format!("{message}\n"));
                drop(u);
                set_status(ui, message);
            }
        },
    );
}

pub fn show(ui: &UiRc) {
    if !ui.borrow().files.is_remote() {
        return;
    }
    if let Some(window) = ui.borrow().sync_window.as_ref() {
        window.present();
        return;
    }
    let window = gtk::Window::builder()
        .title("Sincronização")
        .transient_for(&ui.borrow().window)
        .destroy_with_parent(true)
        .default_width(600)
        .resizable(true)
        .build();
    let body = gtk::Box::new(gtk::Orientation::Vertical, 14);
    body.set_margin_start(20);
    body.set_margin_end(20);
    body.set_margin_top(20);
    body.set_margin_bottom(20);
    let origin = {
        let u = ui.borrow();
        u.files
            .known_identity(&u.project)
            .map(|i| i.label())
            .unwrap_or_default()
    };
    let source = label(&format!("Projeto no servidor\n{origin}"));
    source.set_selectable(true);
    body.append(&source);
    body.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
    let files_title = label("Arquivos ainda não enviados ao servidor");
    files_title.add_css_class("heading");
    body.append(&files_title);
    body.append(&label("Arquivos que seu programa criou ou alterou e que não puderam ser salvos no servidor. As cópias continuam guardadas neste computador."));
    let files = label("Verificando arquivos…");
    body.append(&files);
    let review_files = gtk::Button::with_label("Revisar arquivos…");
    review_files.set_halign(gtk::Align::Start);
    body.append(&review_files);
    body.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
    let edits_title = label("Edições não salvas");
    edits_title.add_css_class("heading");
    body.append(&edits_title);
    body.append(&label("Texto das abas guardado neste computador, inclusive quando a IDE foi interrompida. Revisar uma cópia abre o texto para você conferir e salvar; não envia nada automaticamente."));
    let edits = label("Verificando edições…");
    body.append(&edits);
    let review_edits = gtk::Button::with_label("Revisar cópias das edições…");
    review_edits.set_halign(gtk::Align::Start);
    body.append(&review_edits);
    let status = label("");
    body.append(&status);
    let buttons = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    buttons.set_halign(gtk::Align::End);
    let refresh = gtk::Button::with_label("Atualizar");
    let dismiss = gtk::Button::with_label("Fechar");
    buttons.append(&refresh);
    buttons.append(&dismiss);
    body.append(&buttons);
    let scroll = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .child(&body)
        .build();
    window.set_child(Some(&scroll));
    let weak = Rc::downgrade(ui);
    review_files.connect_clicked(move |_| {
        if let Some(ui) = weak.upgrade() {
            crate::pendencias_ui::show(&ui);
        }
    });
    let weak = Rc::downgrade(ui);
    review_edits.connect_clicked(move |_| {
        if let Some(ui) = weak.upgrade() {
            crate::sessao::review_sync_edits(&ui);
        }
    });
    let weak = window.downgrade();
    dismiss.connect_clicked(move |_| {
        if let Some(window) = weak.upgrade() {
            window.close();
        }
    });
    let weak = Rc::downgrade(ui);
    window.connect_close_request(move |_| {
        if let Some(ui) = weak.upgrade() {
            ui.borrow_mut().sync_window = None;
        }
        gtk::glib::Propagation::Proceed
    });
    let weak = Rc::downgrade(ui);
    let view = (files, edits, status);
    let labels = view.clone();
    refresh.connect_clicked(move |_| {
        if let Some(ui) = weak.upgrade() {
            refresh_summary(&ui, labels.clone());
        }
    });
    ui.borrow_mut().sync_window = Some(window.clone());
    window.present();
    refresh_summary(ui, view);
}

fn label(text: &str) -> gtk::Label {
    gtk::Label::builder()
        .label(text)
        .wrap(true)
        .xalign(0.0)
        .build()
}

fn refresh_summary(ui: &UiRc, labels: (gtk::Label, gtk::Label, gtk::Label)) {
    let (identity, context) = {
        let u = ui.borrow();
        if !u.files.is_remote() {
            return;
        }
        let Ok(identity) = u.files.known_identity(&u.project) else {
            return;
        };
        (identity, crate::sessao::synchronization_context(ui))
    };
    let expected = identity.clone();
    let dirty = all_docs(ui).iter().filter(|d| d.is_dirty()).count();
    labels
        .2
        .set_text("Verificando as cópias guardadas neste computador…");
    em_segundo_plano(
        ui,
        move || {
            let files = pendencias::scan_project(&pendencias::root(), &identity);
            let file_count: usize = files.runs.iter().map(|r| r.files.len()).sum();
            let (edits, errors) = context
                .map(|(store, owned)| {
                    let scan = store.scan();
                    (
                        scan.copies
                            .iter()
                            .filter(|c| !c.backup.paths.iter().any(|p| owned.contains(p)))
                            .count(),
                        scan.errors.len(),
                    )
                })
                .unwrap_or_default();
            (file_count, edits, files.errors.len() + errors)
        },
        move |ui, (files, edits, errors)| {
            let u = ui.borrow();
            if !u.files.is_remote()
                || u.files.known_identity(&u.project).ok().as_ref() != Some(&expected)
            {
                return;
            }
            labels.0.set_text(if files == 0 {
                "Nenhum arquivo aguardando envio."
            } else {
                ""
            });
            if files > 0 {
                labels
                    .0
                    .set_text(&format!("{files} arquivo(s) aguardando envio."));
            }
            labels.1.set_text(&format!("{dirty} aba(s) com edições abertas; {edits} cópia(s) de sessões anteriores para revisar."));
            labels.2.set_text(if errors == 0 { "Revisar não substitui arquivos mais recentes no servidor." } else { "Algumas cópias não puderam ser lidas. Elas foram preservadas; use Revisar para conferir ou exportar." });
        },
    );
}
