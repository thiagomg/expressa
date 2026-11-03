//! Conflict UI. Storage/controller operations remain conditional even when the
//! student explicitly chooses overwrite, since the destination can change again.

use crate::docs::Doc;
use crate::store::{Expected, TextSnapshot};
use crate::{UiRc, all_docs, async_result, perguntar_linha, set_status};
use gtk::{gio, prelude::*};
use std::rc::Rc;

mod resposta {
    pub const CANCELAR: i32 = 0;
    pub const COMPARAR: i32 = 1;
    pub const RECARREGAR: i32 = 2;
    pub const SALVAR_COMO: i32 = 3;
    pub const SUBSTITUIR: i32 = 4;
}

pub fn mostrar(ui: &UiRc, doc: &Rc<Doc>) {
    let name = doc.name();
    if !documento_atual(ui, doc, &name) {
        return;
    }
    let (files, project) = {
        let u = ui.borrow();
        (u.files.clone(), u.project.clone())
    };
    let path = name.clone();
    let weak = Rc::downgrade(doc);
    async_result(
        ui,
        &format!("Comparando {name}"),
        true,
        move || files.current_text(&project, &path),
        move |ui, result| {
            let Some(doc) = weak.upgrade() else {
                return;
            };
            if !documento_atual(ui, &doc, &name) {
                return;
            }
            match result {
                Ok(snapshot) => mostrar_com_snapshot(ui, &doc, snapshot),
                Err(e) => set_status(ui, &format!("não foi possível comparar {name}: {e}")),
            }
        },
    );
}

fn mostrar_com_snapshot(ui: &UiRc, doc: &Rc<Doc>, snapshot: Option<TextSnapshot>) {
    let name = doc.name();
    if !documento_atual(ui, doc, &name) {
        return;
    }
    let detail = if snapshot.is_some() {
        "O arquivo salvo mudou desde que esta aba foi aberta. Suas alterações continuam na aba. Substituir grava o texto da aba sobre a versão deste aviso; uma nova mudança será detectada."
    } else {
        "O arquivo foi removido. Suas alterações continuam na aba. Substituir recria o arquivo somente se o nome ainda estiver livre."
    };
    let dialog = gtk::AlertDialog::builder()
        .modal(true)
        .message(format!("Conflito ao salvar {name}"))
        .detail(detail)
        .buttons([
            "Cancelar",
            "Comparar",
            "Recarregar",
            "Salvar como…",
            "Substituir",
        ])
        .cancel_button(resposta::CANCELAR)
        .default_button(resposta::COMPARAR)
        .build();
    let window = ui.borrow().window.clone();
    let ui = Rc::clone(ui);
    let doc = Rc::clone(doc);
    dialog.choose(Some(&window), None::<&gio::Cancellable>, move |result| {
        // A delayed response must not act on a closed, renamed or old-project tab.
        if !documento_atual(&ui, &doc, &name) {
            return;
        }
        match result {
            Ok(resposta::COMPARAR) => comparar(&ui, &doc, snapshot),
            Ok(resposta::RECARREGAR) => crate::reload_doc(&ui, &doc),
            Ok(resposta::SALVAR_COMO) => {
                let window = ui.borrow().window.clone();
                let Some(new_name) =
                    perguntar_linha(&window, &format!("Salvar cópia de `{name}` como:"))
                else {
                    return;
                };
                if !documento_atual(&ui, &doc, &name) {
                    return;
                }
                crate::save_request(&ui, &doc, Some(&new_name), true, None, |_| {});
            }
            Ok(resposta::SUBSTITUIR) => {
                let expected = snapshot.map_or(Expected::Missing, |s| {
                    s.revision
                        .map(Expected::Revision)
                        .unwrap_or(Expected::Unversioned)
                });
                crate::save_request(&ui, &doc, None, false, Some(&expected), |_| {});
            }
            _ => {}
        }
    });
}

fn documento_atual(ui: &UiRc, doc: &Rc<Doc>, name: &str) -> bool {
    doc.name() == name && all_docs(ui).iter().any(|d| Rc::ptr_eq(d, doc))
}

fn comparar(ui: &UiRc, doc: &Rc<Doc>, snapshot: Option<TextSnapshot>) {
    let compared_name = doc.name();
    let parent = ui.borrow().window.clone();
    let window = gtk::Window::builder()
        .title(format!("Comparar — {}", doc.name()))
        .transient_for(&parent)
        .destroy_with_parent(true)
        .modal(true)
        .default_width(960)
        .default_height(600)
        .build();
    let columns = gtk::Paned::new(gtk::Orientation::Horizontal);
    let column = |title: &str, text: &str| {
        let column = gtk::Box::new(gtk::Orientation::Vertical, 6);
        column.append(&gtk::Label::new(Some(title)));
        let view = gtk::TextView::builder()
            .monospace(true)
            .editable(false)
            .cursor_visible(false)
            .build();
        view.buffer().set_text(text);
        column.append(
            &gtk::ScrolledWindow::builder()
                .vexpand(true)
                .hexpand(true)
                .child(&view)
                .build(),
        );
        column
    };
    columns.set_start_child(Some(&column("Sua aba (preservada)", &doc.text())));
    columns.set_end_child(Some(&column(
        "Arquivo salvo",
        snapshot
            .as_ref()
            .map_or("O arquivo foi removido.", |s| s.text.as_str()),
    )));
    columns.set_position(window.default_width() / 2);
    let body = gtk::Box::new(gtk::Orientation::Vertical, 6);
    body.append(&columns);
    let back = gtk::Button::with_label("Voltar às opções de conflito");
    let (ui, doc, close) = (Rc::clone(ui), Rc::clone(doc), window.clone());
    back.connect_clicked(move |_| {
        close.close();
        // Keep the revision actually displayed. Re-reading here would silently
        // approve an external change made while the comparison was open.
        if documento_atual(&ui, &doc, &compared_name) {
            mostrar_com_snapshot(&ui, &doc, snapshot.clone());
        }
    });
    body.append(&back);
    window.set_child(Some(&body));
    window.present();
}
