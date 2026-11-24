//! Explicit retry/export/discard of retained output, including other projects.
use crate::pendencias::{self, Journal, PendingRun};
use crate::{UiRc, em_segundo_plano, set_status};
use gtk::{gio, prelude::*};
use std::rc::Rc;

mod response {
    pub const CLOSE: i32 = 0;
    pub const RETRY: i32 = 1;
    pub const EXPORT: i32 = 2;
    pub const DISCARD: i32 = 3;
    pub const NEXT: i32 = 4;
    pub const RAW: i32 = 5;
    pub const EXPORT_ERRORS: i32 = 1;
    pub const CONFIRM_DISCARD: i32 = 1;
}

/// Local failures are exceptional recovery, never a synchronization menu.
pub fn notify_local_failure(ui: &UiRc) {
    let identity = {
        let u = ui.borrow();
        if u.files.is_remote() {
            return;
        }
        let Ok(id) = u.files.known_identity(&u.project) else {
            return;
        };
        id
    };
    let expected = identity.clone();
    em_segundo_plano(
        ui,
        move || pendencias::scan_project(&pendencias::root(), &identity),
        move |ui, scan| {
            if ui
                .borrow()
                .files
                .known_identity(&ui.borrow().project)
                .ok()
                .as_ref()
                != Some(&expected)
            {
                return;
            }
            if !scan.runs.is_empty() || !scan.errors.is_empty() {
                show(ui);
            }
        },
    );
}

pub fn show(ui: &UiRc) {
    show_at(ui, 0);
}

fn show_at(ui: &UiRc, index: usize) {
    let (identity, remote) = {
        let u = ui.borrow();
        let Ok(id) = u.files.known_identity(&u.project) else {
            return;
        };
        (id, u.files.is_remote())
    };
    let expected = identity.clone();
    em_segundo_plano(
        ui,
        move || pendencias::scan_project(&pendencias::root(), &identity),
        move |ui, scan| {
            if ui
                .borrow()
                .files
                .known_identity(&ui.borrow().project)
                .ok()
                .as_ref()
                != Some(&expected)
            {
                return;
            }
            if scan.runs.is_empty() {
                if scan.errors.is_empty() {
                    set_status(ui, "todos os arquivos gerados pelo programa foram salvos");
                } else {
                    let detail = format!(
                        "Os arquivos foram preservados em {}.\n{}",
                        pendencias::root().display(),
                        scan.errors.join("\n")
                    );
                    let dialog = gtk::AlertDialog::builder()
                        .message("Algumas execuções não puderam ser lidas")
                        .detail(detail)
                        .buttons(["Fechar", "Exportar cópias não lidas…"])
                        .cancel_button(response::CLOSE)
                        .default_button(response::CLOSE)
                        .build();
                    let window = crate::sincronizacao::parent(ui);
                    let ui = Rc::clone(ui);
                    dialog.choose(Some(&window), None::<&gio::Cancellable>, move |result| {
                        if result == Ok(response::EXPORT_ERRORS) {
                            export_unreadable(&ui, scan.unreadable);
                        }
                    });
                }
                return;
            }
            let index = index % scan.runs.len();
            let run = scan.runs[index].clone();
            let files = run
                .files
                .iter()
                .map(|(p, size, reason)| format!("{p} ({size} bytes): {reason}"))
                .collect::<Vec<_>>()
                .join("\n");
            let explanation = if remote {
                "O programa criou ou alterou estes arquivos, mas eles ainda não foram salvos no servidor."
            } else {
                "O programa criou ou alterou estes arquivos, mas não foi possível gravá-los na pasta do projeto."
            };
            let detail = format!(
                "Origem: {}\n\n{explanation}\n\n{files}\n\nTentar salvar novamente não substitui arquivos mais recentes. Exportar salva uma cópia em outra pasta. Descartar apaga apenas as cópias preservadas.\n\n{}{}",
                run.identity.label(),
                run.path.display(),
                if scan.errors.is_empty() {
                    String::new()
                } else {
                    format!("\n\nNão lidos (preservados):\n{}", scan.errors.join("\n"))
                }
            );
            let dialog = gtk::AlertDialog::builder()
                .modal(true)
                .message(format!(
                    "{} — execução {} de {}",
                    if remote {
                        "Arquivos ainda não enviados ao servidor"
                    } else {
                        "Arquivos que não puderam ser salvos"
                    },
                    index + 1,
                    scan.runs.len()
                ))
                .detail(detail)
                .buttons([
                    "Fechar",
                    "Tentar salvar novamente",
                    "Exportar…",
                    "Descartar…",
                    "Próxima",
                    "Exportar não lidas…",
                ])
                .cancel_button(response::CLOSE)
                .default_button(response::CLOSE)
                .build();
            let window = crate::sincronizacao::parent(ui);
            let ui = Rc::clone(ui);
            dialog.choose(
                Some(&window),
                None::<&gio::Cancellable>,
                move |result| match result {
                    Ok(response::RETRY) => retry(&ui, run),
                    Ok(response::EXPORT) => export(&ui, run),
                    Ok(response::DISCARD) => confirm_discard(&ui, run),
                    Ok(response::NEXT) => show_at(&ui, index + 1),
                    Ok(response::RAW) => export_unreadable(&ui, scan.unreadable),
                    _ => {}
                },
            );
        },
    );
}

fn retry(ui: &UiRc, run: PendingRun) {
    // Run/project transitions already own their lifecycle; don't refresh tabs in
    // the middle of a run or start a request against a project being switched.
    if !crate::execution_available(ui, "pare o programa antes de tentar salvar esses arquivos") {
        return;
    }
    let (files, student) = {
        let u = ui.borrow();
        (u.files.clone(), u.project.clone())
    };
    if files.known_identity(&student).ok().as_ref() != Some(&run.identity) {
        set_status(
            ui,
            "abra o projeto de origem antes de reenviar; também é possível exportar a saída",
        );
        return;
    }
    ui.borrow_mut()
        .execucao
        .execution
        .transition(crate::ExecutionEvent::Prepare)
        .expect("idle before retry");
    crate::async_result(
        ui,
        "Reenviando arquivos",
        true,
        move || {
            let journal =
                Journal::open(&run.path)?.ok_or("a execução está sendo usada por outra janela")?;
            Ok::<_, String>(journal.retry(&files, &student))
        },
        |ui, result| {
            ui.borrow_mut()
                .execucao
                .execution
                .transition(crate::ExecutionEvent::PreparationFailed)
                .expect("retry finished");
            match result {
                Ok(delivery) => {
                    let failures = delivery.errors.len() + delivery.conflicts.len();
                    let console = ui.borrow().execucao.console.clone();
                    crate::mostrar_envio(
                        ui,
                        &console,
                        crate::exec::Envio {
                            enviados: delivery.sent,
                            erros: delivery.errors,
                            avisos: delivery.warnings,
                            conflitos: delivery
                                .conflicts
                                .into_iter()
                                .map(|(path, bytes)| crate::exec::UploadConflict { path, bytes })
                                .collect(),
                            pending_dir: None,
                        },
                    );
                    set_status(
                        ui,
                        if failures == 0 {
                            "arquivos salvos"
                        } else {
                            "arquivos continuam preservados; veja o console"
                        },
                    );
                }
                Err(e) => set_status(ui, &format!("não foi possível reenviar: {e}")),
            }
        },
    );
}

fn export(ui: &UiRc, run: PendingRun) {
    let dialog = gtk::FileDialog::builder()
        .title("Salvar cópia dos arquivos")
        .accept_label("Exportar")
        .build();
    let window = crate::sincronizacao::parent(ui);
    let ui = Rc::clone(ui);
    dialog.select_folder(Some(&window), None::<&gio::Cancellable>, move |result| {
        let Some(folder) = result.ok().and_then(|f| f.path()) else {
            return;
        };
        em_segundo_plano(
            &ui,
            move || {
                let journal = Journal::open(&run.path)?.ok_or("execução em uso")?;
                journal.export(&folder)
            },
            |ui, result| match result {
                Ok(path) => set_status(
                    ui,
                    &format!(
                        "exportado em {}; a saída pendente foi preservada",
                        path.display()
                    ),
                ),
                Err(e) => set_status(ui, &format!("não foi possível exportar: {e}")),
            },
        );
    });
}

fn confirm_discard(ui: &UiRc, run: PendingRun) {
    let dialog = gtk::AlertDialog::builder()
        .modal(true)
        .message("Descartar as cópias destes arquivos?")
        .detail(format!(
            "{}\n{} arquivo(s). Esta ação remove as cópias preservadas, sem alterar o projeto.",
            run.identity.label(),
            run.files.len()
        ))
        .buttons(["Cancelar", "Descartar"])
        .cancel_button(response::CLOSE)
        .default_button(response::CLOSE)
        .build();
    let window = crate::sincronizacao::parent(ui);
    let ui = Rc::clone(ui);
    dialog.choose(Some(&window), None::<&gio::Cancellable>, move |result| {
        if result != Ok(response::CONFIRM_DISCARD) {
            return;
        }
        em_segundo_plano(
            &ui,
            move || {
                Journal::open(&run.path)?
                    .ok_or("execução em uso")?
                    .discard()
            },
            |ui, result| match result {
                Ok(()) => set_status(ui, "cópias dos arquivos descartadas"),
                Err(e) => set_status(ui, &format!("não foi possível descartar: {e}")),
            },
        );
    });
}

fn export_unreadable(ui: &UiRc, paths: Vec<std::path::PathBuf>) {
    if paths.is_empty() {
        set_status(ui, "não há registros não lidos para exportar");
        return;
    }
    let dialog = gtk::FileDialog::builder()
        .title("Exportar cópias de execução não lidas")
        .accept_label("Exportar")
        .build();
    let window = crate::sincronizacao::parent(ui);
    let ui = Rc::clone(ui);
    dialog.select_folder(Some(&window), None::<&gio::Cancellable>, move |result| {
        let Some(folder) = result.ok().and_then(|f| f.path()) else {
            return;
        };
        em_segundo_plano(
            &ui,
            move || {
                paths
                    .into_iter()
                    .map(|path| {
                        pendencias::export_raw(&path, &folder)
                            .map_err(|e| format!("{}: {e}", path.display()))
                    })
                    .collect::<Vec<_>>()
            },
            |ui, results| {
                let mut ok = 0;
                for result in results {
                    match result {
                        Ok(path) => {
                            ok += 1;
                            ui.borrow()
                                .execucao
                                .console
                                .stderr(&format!("cópia exportada em {}\n", path.display()));
                        }
                        Err(e) => ui
                            .borrow()
                            .execucao
                            .console
                            .stderr(&format!("erro na exportação; original preservado: {e}\n")),
                    }
                }
                set_status(
                    ui,
                    &format!(
                        "{ok} cópia(s) exportada(s); os originais foram preservados; veja o console"
                    ),
                );
            },
        );
    });
}
