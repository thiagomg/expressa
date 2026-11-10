//! Project identity, file tree, local/remote open, and tree context actions.

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;

use crate::prefs::{expand_tilde, path_for_display};
use crate::rpc::Rpc;
use crate::store::{Files, valid_aluno};
use crate::*;
use gtk::gio::prelude::*;
use gtk::glib;
use gtk::prelude::*;
use gtk::{Box as GtkBox, Button, Entry, Label, Orientation, ScrolledWindow, TreeViewColumn};

// ── Project and file tree ──────────────────────────────────────────────────

/// First launch: open the Documents copy if the installer already put it there.
pub(crate) fn open_shipped_exemplos(ui: &UiRc) -> bool {
    let Some(dst) = existing_exemplos_dir() else {
        return false;
    };
    open_local_project(ui, dst.clone());
    let opened = matches!(&ui.borrow().files, Files::Local(_));
    if opened {
        set_status(
            ui,
            &format!("exemplos em {} — pode editar", path_for_display(&dst)),
        );
    }
    opened
}

/// Ask for a local project folder. Does not block the GTK loop.
pub(crate) fn choose_local_folder(ui: &UiRc) {
    if !can_change_project(ui) {
        return;
    }
    pause_raise();
    let start = {
        let u = ui.borrow();
        match &u.files {
            Files::Local(p) => p.display().to_string(),
            _ => u.prefs.pasta.clone(),
        }
    };
    let window = ui.borrow().window.clone();
    let ui = Rc::clone(ui);
    pick_folder(&window, &start, move |chosen| {
        resume_raise();
        if let Some(path) = chosen {
            open_local_project(&ui, path);
        }
    });
}

pub(crate) fn open_local_project(ui: &UiRc, path: PathBuf) {
    if !can_change_project(ui) {
        return;
    }
    ui.borrow_mut().navigation_epoch += 1;
    let navigation = ui.borrow().navigation_epoch;
    async_result(
        ui,
        "Abrindo projeto local",
        true,
        move || {
            let path = expand_tilde(&path.to_string_lossy())
                .canonicalize()
                .map_err(|e| format!("pasta: {e}"))?;
            if !path.is_dir() {
                return Err("escolha uma pasta".into());
            }
            if store::pasta_larga(&path) {
                return Err("escolha a pasta do projeto, não a pasta pessoal".into());
            }
            Ok(path)
        },
        move |ui, result| {
            if ui.borrow().navigation_epoch != navigation {
                return;
            }
            match result {
                Ok(path) => {
                    if matches!(&ui.borrow().files, Files::Local(current) if current == &path) {
                        refresh_tree(ui);
                        return;
                    }
                    let next = ui.clone();
                    after_unsaved(ui, move || apply_local(&next, path));
                }
                Err(e) => set_status(ui, &e),
            }
        },
    );
}

pub(crate) fn apply_local(ui: &UiRc, path: PathBuf) {
    ui.borrow_mut().navigation_epoch += 1;
    sincronizacao::close(ui);
    if !matches!(ui.borrow().files, Files::None) {
        close_project_tabs(ui);
    }
    ui.borrow_mut().sessao_pronta = false;
    let name = path
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| path.display().to_string());
    let shown = path.display().to_string();
    {
        let mut u = ui.borrow_mut();
        u.files.shutdown();
        u.execucao.operations.cancel_all();
        u.project_epoch += 1;
        u.file_operations.clear();
        u.backup_busy.set(false);
        u.backup_review.borrow_mut().take();
        u.recuperacao_aberta = false;
        u.files = Files::Local(path.clone());
        u.project = name;
        u.prefs.pasta = shown.clone();
        u.prefs
            .lembrar_recente(prefs::Recente::Local(shown.clone()));
        u.prefs.save();
        u.file_cache.borrow_mut().clear();
    }
    set_local_project_path(ui, &path);
    finish_open(ui);
}

/// URL and student name, then the student's project (`escolher_projeto`).
/// Files on that server live in `aula-data/aluno/projeto`.
pub(crate) fn ask_server(ui: &UiRc) {
    if !can_change_project(ui) {
        return;
    }
    ui.borrow_mut().navigation_epoch += 1;
    let (url, aluno) = {
        let u = ui.borrow();
        let url = if u.prefs.servidor.is_empty() {
            DEFAULT_URL.to_string()
        } else {
            u.prefs.servidor.clone()
        };
        let aluno = if u.files.is_remote() {
            u.project.clone()
        } else {
            "local".to_string()
        };
        (url, aluno)
    };
    let window = ui.borrow().window.clone();
    pause_raise();
    let answer = perguntar_servidor(&window, &url, &aluno);
    resume_raise();
    let Some((url, aluno)) = answer else {
        return;
    };
    let aluno = aluno.trim().to_string();
    if !valid_aluno(&aluno) {
        set_status(ui, "nome do aluno: letras, números, _ e -");
        return;
    }
    let url = normalize_url(&url);
    let atual = ui.borrow().prefs.projeto.clone();
    conectar_servidor(ui, url, aluno, atual);
}

/// Arquivo → Remoto → Novo projeto: the project list of the connected student,
/// or the server dialog when the window is not on a server.
pub(crate) fn trocar_projeto(ui: &UiRc) {
    let conectado = {
        let u = ui.borrow();
        match &u.files {
            Files::Remote { rpc, .. } => {
                Some((Arc::clone(rpc), u.prefs.servidor.clone(), u.project.clone()))
            }
            _ => None,
        }
    };
    match conectado {
        Some((rpc, url, aluno)) => abrir_projeto_remoto(ui, rpc, url, aluno),
        None => ask_server(ui),
    }
}

/// Shows `aluno`'s projects on `rpc`; the one chosen opens, after the usual
/// question about unsaved tabs.
pub(crate) fn abrir_projeto_remoto(ui: &UiRc, rpc: Arc<Rpc>, url: String, aluno: String) {
    if !can_change_project(ui) {
        return;
    }
    let atual = {
        let u = ui.borrow();
        match &u.files {
            Files::Remote { projeto, .. } if u.project == aluno => projeto.clone(),
            _ => u.prefs.projeto.clone(),
        }
    };
    listar_e_escolher(ui, rpc, url, aluno, atual, true);
}

pub(crate) fn navigation_task<T: Send + 'static>(
    ui: &UiRc,
    work: impl FnOnce() -> Result<T, String> + Send + 'static,
    done: impl FnOnce(&UiRc, T) + 'static,
) {
    let epoch = ui.borrow().navigation_epoch;
    let reply = background::Reply::spawn(epoch, work);
    let operation =
        ui.borrow()
            .execucao
            .operations
            .begin("Consultando servidor", reply.cancellation(), true);
    let ui = Rc::downgrade(ui);
    let mut done = Some(done);
    glib::timeout_add_local(std::time::Duration::from_millis(30), move || {
        let _keep_progress = &operation;
        let Some(ui) = ui.upgrade() else {
            return glib::ControlFlow::Break;
        };
        if ui.borrow().closing.get() {
            return glib::ControlFlow::Break;
        }
        let epoch = ui.borrow().navigation_epoch;
        match reply.poll(epoch) {
            background::Poll::Ready(Ok(value)) => {
                done.take().unwrap()(&ui, value);
                glib::ControlFlow::Break
            }
            background::Poll::Ready(Err(e)) => {
                set_status(&ui, &e);
                glib::ControlFlow::Break
            }
            background::Poll::Pending => glib::ControlFlow::Continue,
            background::Poll::Obsolete => glib::ControlFlow::Break,
        }
    });
}

pub(crate) fn conectar_servidor(ui: &UiRc, url: String, aluno: String, atual: String) {
    if !can_change_project(ui) {
        return;
    }
    ui.borrow_mut().navigation_epoch += 1;
    let url = normalize_url(&url);
    let address = url.clone();
    set_status(ui, "conectando ao servidor…");
    navigation_task(
        ui,
        move || Rpc::connect(&address).map(Arc::new),
        move |ui, rpc| listar_e_escolher(ui, rpc, url, aluno, atual, false),
    );
}

pub(crate) fn listar_e_escolher(
    ui: &UiRc,
    rpc: Arc<Rpc>,
    url: String,
    aluno: String,
    atual: String,
    novo: bool,
) {
    let client = Arc::clone(&rpc);
    let student = aluno.clone();
    navigation_task(
        ui,
        move || client.list_projects(&student),
        move |ui, projetos| {
            if !can_change_project(ui) {
                return;
            }
            let window = ui.borrow().window.clone();
            let ui = Rc::clone(ui);
            let epoch = ui.borrow().navigation_epoch;
            let titulo = aluno.clone();
            let chooser_ui = Rc::clone(&ui);
            escolher_projeto(
                &chooser_ui,
                &window,
                Arc::clone(&rpc),
                &titulo,
                projetos,
                &atual,
                novo,
                move |projeto| {
                    if ui.borrow().navigation_epoch != epoch {
                        return;
                    }
                    let ui_c = Rc::clone(&ui);
                    after_unsaved(&ui, move || apply_remote(&ui_c, rpc, url, aluno, projeto));
                },
            );
        },
    );
}

pub(crate) fn apply_remote(ui: &UiRc, rpc: Arc<Rpc>, url: String, aluno: String, projeto: String) {
    ui.borrow_mut().navigation_epoch += 1;
    sincronizacao::close(ui);
    if !matches!(ui.borrow().files, Files::None) {
        close_project_tabs(ui);
    }
    ui.borrow_mut().sessao_pronta = false;
    let tip = format!("{url} — aula-data/{aluno}/{projeto}");
    {
        let mut u = ui.borrow_mut();
        u.files.shutdown();
        u.execucao.operations.cancel_all();
        u.project_epoch += 1;
        u.file_operations.clear();
        u.backup_busy.set(false);
        u.backup_review.borrow_mut().take();
        u.recuperacao_aberta = false;
        u.files = Files::Remote {
            rpc,
            projeto: projeto.clone(),
        };
        u.prefs.lembrar_recente(prefs::Recente::Servidor {
            url: url.clone(),
            aluno: aluno.clone(),
            projeto: projeto.clone(),
        });
        u.project = aluno;
        u.prefs.servidor = url;
        u.prefs.projeto = projeto;
        u.prefs.save();
        u.file_cache.borrow_mut().clear();
    }
    set_project_path(ui, &tip, Some(&tip));
    finish_open(ui);
}

/// The student's projects: pick one (double-click, Enter or Abrir) or
/// create a new one. Calls `done` with the chosen name; closing or
/// Cancelar calls nothing. Does not block the GTK loop.
pub(crate) fn escolher_projeto(
    ui: &UiRc,
    parent: &impl IsA<gtk::Window>,
    rpc: Arc<Rpc>,
    aluno: &str,
    projetos: Vec<String>,
    atual: &str,
    novo_projeto: bool,
    done: impl FnOnce(String) + 'static,
) {
    let dialog = gtk::Window::builder()
        .transient_for(parent)
        .modal(true)
        .title(format!("Projetos de {aluno}"))
        .default_width(420)
        .default_height(380)
        .build();
    let v = GtkBox::new(Orientation::Vertical, 10);
    v.set_margin_start(16);
    v.set_margin_end(16);
    v.set_margin_top(16);
    v.set_margin_bottom(16);
    let info = Label::new(Some("Escolha um projeto ou crie um novo."));
    info.set_xalign(0.0);
    let lista = gtk::ListBox::new();
    lista.set_selection_mode(gtk::SelectionMode::Single);
    let rolagem = ScrolledWindow::builder()
        .child(&lista)
        .vexpand(true)
        .min_content_height(160)
        .build();
    let novo = Entry::builder()
        .placeholder_text("novo projeto (ex.: aula-03)")
        .hexpand(true)
        .build();
    let criar = Button::with_label("Criar");
    let linha_novo = GtkBox::new(Orientation::Horizontal, 8);
    linha_novo.append(&novo);
    linha_novo.append(&criar);
    let aviso = Label::new(None);
    aviso.set_xalign(0.0);
    aviso.set_wrap(true);
    let buttons = GtkBox::new(Orientation::Horizontal, 8);
    buttons.set_halign(gtk::Align::End);
    let cancelar = Button::with_label("Cancelar");
    let abrir = Button::with_label("Abrir");
    abrir.add_css_class("suggested-action");
    buttons.append(&cancelar);
    buttons.append(&abrir);
    v.append(&info);
    v.append(&rolagem);
    v.append(&linha_novo);
    v.append(&aviso);
    v.append(&buttons);
    dialog.set_child(Some(&v));

    // Row i of `lista` is `nomes[i]`.
    let nomes = Rc::new(RefCell::new(Vec::<String>::new()));
    let preencher = Rc::new({
        let lista = lista.clone();
        let abrir = abrir.clone();
        let nomes = Rc::clone(&nomes);
        move |novos: Vec<String>, escolhido: &str| {
            while let Some(row) = lista.first_child() {
                lista.remove(&row);
            }
            for nome in &novos {
                let label = Label::new(Some(nome));
                label.set_xalign(0.0);
                label.set_margin_top(6);
                label.set_margin_bottom(6);
                label.set_margin_start(8);
                lista.append(&label);
            }
            let i = novos
                .iter()
                .position(|n| n == escolhido)
                .or((!novos.is_empty()).then_some(0));
            if let Some(i) = i {
                lista.select_row(lista.row_at_index(i as i32).as_ref());
            }
            abrir.set_sensitive(i.is_some());
            *nomes.borrow_mut() = novos;
        }
    });
    let vazio = projetos.is_empty();
    preencher(projetos, atual);
    if vazio {
        aviso.set_text("Você ainda não tem projetos: crie o primeiro.");
    }

    let done: RefCell<Option<Box<dyn FnOnce(String)>>> = RefCell::new(Some(Box::new(done)));
    let escolher = Rc::new({
        let lista = lista.clone();
        let nomes = Rc::clone(&nomes);
        let dialog = dialog.clone();
        move || {
            let Some(row) = lista.selected_row() else {
                return;
            };
            let Some(nome) = nomes.borrow().get(row.index() as usize).cloned() else {
                return;
            };
            dialog.close();
            if let Some(done) = done.borrow_mut().take() {
                done(nome);
            }
        }
    });
    {
        let escolher = Rc::clone(&escolher);
        abrir.connect_clicked(move |_| escolher());
    }
    lista.connect_row_activated(move |_, _| escolher());
    {
        let abrir = abrir.clone();
        lista.connect_row_selected(move |_, row| abrir.set_sensitive(row.is_some()));
    }

    let criar_projeto = Rc::new({
        let novo = novo.clone();
        let aviso = aviso.clone();
        let aluno = aluno.to_string();
        let ui = Rc::clone(ui);
        let criar = criar.clone();
        let dialog = dialog.clone();
        move || {
            if !can_change_project(&ui) || !criar.is_sensitive() {
                return;
            }
            let nome = novo.text().trim().to_string();
            if !valid_aluno(&nome) {
                aviso.set_text("nome do projeto: letras, números, _ e - (sem espaços)");
                return;
            }
            criar.set_sensitive(false);
            let rpc = Arc::clone(&rpc);
            let aluno = aluno.clone();
            let criado = nome.clone();
            let preencher = Rc::clone(&preencher);
            let novo = novo.clone();
            let aviso = aviso.clone();
            let criar = criar.clone();
            let dialog = dialog.clone();
            async_result(
                &ui,
                "Criando projeto",
                false,
                move || {
                    rpc.create_project(&aluno, &nome)
                        .and_then(|()| rpc.list_projects(&aluno))
                },
                move |ui, result| {
                    criar.set_sensitive(true);
                    if !dialog.is_visible() {
                        return;
                    }
                    match result {
                        Ok(lista) => {
                            preencher(lista, &criado);
                            novo.set_text("");
                            aviso.set_text(&format!("criado `{criado}`: aperte Abrir"));
                            projetos_menu::refresh(ui);
                        }
                        Err(e) => aviso.set_text(&e),
                    }
                },
            );
        }
    });
    {
        let criar_projeto = Rc::clone(&criar_projeto);
        criar.connect_clicked(move |_| criar_projeto());
    }
    novo.connect_activate(move |_| criar_projeto());
    {
        let dialog = dialog.clone();
        cancelar.connect_clicked(move |_| dialog.close());
    }
    dialog.present();
    if vazio || novo_projeto {
        novo.grab_focus();
    } else {
        lista.grab_focus();
    }
}

pub(crate) fn after_unsaved(ui: &UiRc, then: impl FnOnce() + 'static) {
    let epoch = ui.borrow().navigation_epoch;
    let next = Rc::clone(ui);
    let then = move || {
        if next.borrow().navigation_epoch == epoch && can_change_project(&next) {
            then();
        }
    };
    if !can_change_project(ui) {
        return;
    }
    // Choosing the first project adopts scratch/recovered tabs instead of
    // forcing a save to a repository that has not been selected yet.
    if matches!(ui.borrow().files, Files::None) {
        then();
        return;
    }
    let dirty = dirty_docs(ui);
    if dirty.is_empty() {
        then();
    } else {
        confirm_unsaved(ui, dirty, then);
    }
}

pub(crate) fn close_project_tabs(ui: &UiRc) {
    close_source_viewer(ui);
    ui.borrow_mut().execucao.last_execution_location = None;
    set_action_enabled(&ui.borrow().window, "ver-fonte-executada", false);
    ui.borrow_mut().execucao.execution_sources = Arc::new(debug_sources::Sources::new());
    ui.borrow_mut().execucao.execution_breakpoints.clear();
    ui.borrow_mut().execucao.execution_editors.clear();
    // Keep this project's tabs for next time. Its unsaved tabs were saved
    // or discarded already, so their copies go.
    sessao::salvar(ui);
    sessao::limpar_copias(ui);
    {
        let mut u = ui.borrow_mut();
        u.sessao_pronta = false;
        // Paths of the project being left mean nothing in the next one.
        u.abas_fechadas.clear();
    }
    for d in all_docs(ui) {
        d.mark_saved();
        remove_tab(ui, &d);
    }
}

/// Show the project tree. Reopens the last file of this project, if it is still there.
pub(crate) fn finish_open(ui: &UiRc) {
    projetos_menu::rebuild(ui);
    projetos_menu::refresh(ui);
    refresh_title(ui);
    refresh_diag_label(ui);
    // Reopen the tabs of last time once the listing says what is there.
    refresh_tree_e(ui, |ui| {
        sessao::restaurar(ui);
        sincronizacao::notify(ui);
    });
}

pub(crate) fn normalize_url(url: &str) -> String {
    let url = url.trim();
    let url = if url.is_empty() {
        DEFAULT_URL.to_string()
    } else if url.contains("://") {
        url.to_string()
    } else {
        format!("http://{url}")
    };
    identidade::normalize_endpoint(&url).unwrap_or(url)
}

/// Folder picker. `FileChooserNative` never maps a window on this desktop, and
/// with `modal` the editor then ignores the mouse and the keyboard. `GtkFileDialog`
/// asks the portal without a nested main loop and does show a window.
pub(crate) fn pick_folder(
    parent: &impl IsA<gtk::Window>,
    start: &str,
    done: impl FnOnce(Option<PathBuf>) + 'static,
) {
    let dialog = gtk::FileDialog::new();
    dialog.set_title("Pasta do projeto");
    dialog.set_accept_label(Some("Abrir"));
    // Modal would freeze the editor again if the portal window failed to map.
    dialog.set_modal(false);
    let start_path = expand_tilde(start);
    if start_path.is_absolute() {
        dialog.set_initial_folder(Some(&gtk::gio::File::for_path(&start_path)));
    }
    let dialog_keep = dialog.clone();
    dialog.select_folder(
        Some(parent),
        None::<&gtk::gio::Cancellable>,
        move |result| {
            let _keep = dialog_keep;
            let path = result.ok().and_then(|file| file.path());
            done(path);
        },
    );
}

pub(crate) fn perguntar_servidor(
    parent: &impl IsA<gtk::Window>,
    url: &str,
    aluno: &str,
) -> Option<(String, String)> {
    let dialog = gtk::Window::builder()
        .transient_for(parent)
        .modal(true)
        .title("Servidor remoto")
        .default_width(460)
        .build();
    let v = GtkBox::new(Orientation::Vertical, 10);
    v.set_margin_start(16);
    v.set_margin_end(16);
    v.set_margin_top(16);
    v.set_margin_bottom(16);
    let info = Label::new(Some(
        "Depois de conectar, escolha um dos seus projetos ou crie um novo.",
    ));
    info.set_wrap(true);
    info.set_xalign(0.0);
    let url_entry = Entry::builder().text(url).hexpand(true).build();
    let aluno_entry = Entry::builder().text(aluno).hexpand(true).build();
    let buttons = GtkBox::new(Orientation::Horizontal, 8);
    buttons.set_halign(gtk::Align::End);
    let cancel = Button::with_label("Cancelar");
    let ok = Button::with_label("Conectar");
    ok.add_css_class("suggested-action");
    buttons.append(&cancel);
    buttons.append(&ok);
    v.append(&info);
    v.append(&Label::new(Some("endereço:")));
    v.append(&url_entry);
    v.append(&Label::new(Some("aluno:")));
    v.append(&aluno_entry);
    v.append(&buttons);
    dialog.set_child(Some(&v));

    let loop_ = glib::MainLoop::new(None, false);
    let out = Rc::new(RefCell::new(None::<(String, String)>));
    let accept = Rc::new({
        let loop_q = loop_.clone();
        let out_q = Rc::clone(&out);
        let url_q = url_entry.clone();
        let aluno_q = aluno_entry.clone();
        let dialog_q = dialog.clone();
        move || {
            *out_q.borrow_mut() = Some((url_q.text().to_string(), aluno_q.text().to_string()));
            dialog_q.close();
            loop_q.quit();
        }
    });
    {
        let accept = Rc::clone(&accept);
        ok.connect_clicked(move |_| accept());
    }
    {
        let accept = Rc::clone(&accept);
        aluno_entry.connect_activate(move |_| accept());
    }
    {
        let loop_c = loop_.clone();
        let dialog_c = dialog.clone();
        cancel.connect_clicked(move |_| {
            dialog_c.close();
            loop_c.quit();
        });
    }
    {
        let loop_c = loop_.clone();
        dialog.connect_close_request(move |_| {
            loop_c.quit();
            glib::Propagation::Proceed
        });
    }
    dialog.present();
    url_entry.grab_focus();
    loop_.run();
    out.borrow_mut().take()
}

/// Runs `trabalho` (network, disk) on a worker thread and `depois` back on
/// the GTK thread with its result, so the window keeps responding while a
/// server is slow. Nothing runs if the window is gone by then.
pub(crate) fn em_segundo_plano<T: Send + 'static>(
    ui: &UiRc,
    trabalho: impl FnOnce() -> T + Send + 'static,
    depois: impl FnOnce(&UiRc, T) + 'static,
) {
    async_result(
        ui,
        "",
        false,
        move || Ok(trabalho()),
        move |ui, result| match result {
            Ok(value) => depois(ui, value),
            Err(e) => set_status(ui, &e),
        },
    );
}

pub(crate) fn async_result<T: Send + 'static>(
    ui: &UiRc,
    label: &str,
    cancellable: bool,
    work: impl FnOnce() -> Result<T, String> + Send + 'static,
    done: impl FnOnce(&UiRc, Result<T, String>) + 'static,
) {
    async_on(background::io(), false, ui, label, cancellable, work, done);
}

pub(crate) fn async_on<T: Send + 'static>(
    pool: &background::Pool,
    durable: bool,
    ui: &UiRc,
    label: &str,
    cancellable: bool,
    work: impl FnOnce() -> Result<T, String> + Send + 'static,
    done: impl FnOnce(&UiRc, Result<T, String>) + 'static,
) {
    let epoch = ui.borrow().project_epoch;
    let mut reply = pool.submit(epoch, work);
    if durable {
        reply.keep_running();
    }
    let operation = if label.is_empty() {
        None
    } else {
        Some(
            ui.borrow()
                .execucao
                .operations
                .begin(label, reply.cancellation(), cancellable),
        )
    };
    let weak = Rc::downgrade(ui);
    let mut done = Some(done);
    let mut operation = operation;
    glib::timeout_add_local(std::time::Duration::from_millis(16), move || {
        let Some(ui) = weak.upgrade() else {
            return glib::ControlFlow::Break;
        };
        if ui.borrow().closing.get() || ui.borrow().project_epoch != epoch {
            return glib::ControlFlow::Break;
        }
        let result = match reply.poll(epoch) {
            background::Poll::Pending => return glib::ControlFlow::Continue,
            background::Poll::Ready(result) => result,
            background::Poll::Obsolete => Err("operação cancelada".into()),
        };
        operation.take();
        done.take().unwrap()(&ui, result);
        glib::ControlFlow::Break
    });
}

pub(crate) fn refresh_tree(ui: &UiRc) {
    refresh_tree_e(ui, |_| {});
}

/// Lists the project off the GTK thread, then fills the tree and runs
/// `depois`. A listing that comes back after a newer one started (another
/// refresh, another project) is dropped.
pub(crate) fn refresh_tree_e(ui: &UiRc, depois: impl FnOnce(&UiRc) + 'static) {
    let (files, projeto, onde, geracao) = {
        let mut u = ui.borrow_mut();
        u.geracao_arvore += 1;
        let onde = match &u.files {
            Files::None => "sem pasta".to_string(),
            Files::Local(p) => p.display().to_string(),
            Files::Remote { .. } => {
                format!("servidor {} \\ {}", u.prefs.servidor, nome_do_projeto(&u))
            }
        };
        (u.files.clone(), u.project.clone(), onde, u.geracao_arvore)
    };
    let p = projeto.clone();
    async_result(
        ui,
        "Atualizando arquivos",
        true,
        move || files.list_tree(&p),
        move |ui, listagem| {
            if ui.borrow().geracao_arvore != geracao {
                return;
            }
            let (entries, err) = match listagem {
                Ok(p) => (p, None),
                Err(e) => (Vec::new(), Some(e)),
            };
            mostrar_arvore(ui, &projeto, &entries, err, &onde);
            depois(ui);
        },
    );
}

pub(crate) fn mostrar_arvore(
    ui: &UiRc,
    projeto: &str,
    entries: &[expressa_aula_proto::TreeEntry],
    err: Option<String>,
    onde: &str,
) {
    {
        let mut u = ui.borrow_mut();
        u.project_files = entries
            .iter()
            .filter(|e| !e.is_dir)
            .map(|e| e.path.clone())
            .collect();
        u.file_cache.borrow_mut().clear();
    }
    preencher_arvore(ui, projeto, entries);
    ui.borrow().tree.expand_all();
    // Long names must not leave the list scrolled sideways.
    {
        let tree = ui.borrow().tree.clone();
        glib::idle_add_local_once(move || {
            if let Some(h) = tree.hadjustment() {
                h.set_value(0.0);
            }
        });
    }
    match err {
        Some(e) => set_status(ui, &e),
        None => set_status(ui, &format!("{onde} — {} itens", entries.len())),
    }
}

#[allow(deprecated)]
pub(crate) fn preencher_arvore(
    ui: &UiRc,
    projeto: &str,
    entries: &[expressa_aula_proto::TreeEntry],
) {
    let store = ui.borrow().store.clone();
    store.clear();
    let root = store.append(None);
    let root_label = format!("{projeto}/");
    store.set(&root, &[(0, &root_label), (1, &""), (2, &true)]);

    let mut dirs: HashMap<String, gtk::TreeIter> = HashMap::new();
    dirs.insert(String::new(), root);

    for entry in entries {
        let parent_key = match entry.path.rfind('/') {
            Some(i) => entry.path[..i].to_string(),
            None => String::new(),
        };
        let Some(parent) = dirs.get(&parent_key) else {
            continue;
        };
        let name = entry
            .path
            .rsplit('/')
            .next()
            .unwrap_or(&entry.path)
            .to_string();
        let label = if entry.is_dir {
            format!("{name}/")
        } else {
            name
        };
        let iter = store.append(Some(parent));
        store.set(&iter, &[(0, &label), (1, &entry.path), (2, &entry.is_dir)]);
        if entry.is_dir {
            dirs.insert(entry.path.clone(), iter);
        }
    }
}

#[allow(deprecated)]
pub(crate) fn abrir_no(ui: &UiRc, path: &gtk::TreePath) {
    let (rel, is_dir) = {
        let u = ui.borrow();
        let model = u.store.upcast_ref::<gtk::TreeModel>();
        let Some(iter) = model.iter(path) else {
            return;
        };
        let rel: String = model.get(&iter, 1);
        let is_dir: bool = model.get(&iter, 2);
        (rel, is_dir)
    };
    if is_dir || rel.is_empty() {
        let u = ui.borrow();
        if u.tree.row_expanded(path) {
            u.tree.collapse_row(path);
        } else {
            u.tree.expand_row(path, false);
        }
        return;
    }
    open_path(ui, &rel);
}

pub(crate) fn join_path(dir: &str, name: &str) -> String {
    if dir.is_empty() {
        name.to_string()
    } else {
        format!("{dir}/{name}")
    }
}

pub(crate) fn parent_dir(path: &str, is_dir: bool) -> String {
    if is_dir {
        return path.to_string();
    }
    match path.rfind('/') {
        Some(i) => path[..i].to_string(),
        None => String::new(),
    }
}

/// Select `rel` in the file tree, opening its folders and scrolling to it.
#[allow(deprecated)]
pub(crate) fn revelar_na_arvore(ui: &UiRc, rel: &str) {
    let (tree, store) = {
        let u = ui.borrow();
        (u.tree.clone(), u.store.clone())
    };
    let model = store.upcast_ref::<gtk::TreeModel>();
    let mut achado = None;
    model.foreach(|m, path, iter| {
        let caminho: String = m.get(iter, 1);
        if exec::same_project_path(&caminho, rel) {
            achado = Some(path.clone());
            return true;
        }
        false
    });
    match achado {
        Some(path) => {
            tree.expand_to_path(&path);
            tree.selection().select_path(&path);
            tree.scroll_to_cell(Some(&path), None::<&TreeViewColumn>, true, 0.5, 0.0);
        }
        None => set_status(ui, &format!("`{rel}` não está na árvore")),
    }
}

#[allow(deprecated)]
pub(crate) fn selected_tree_entry(ui: &UiRc) -> (String, bool) {
    let u = ui.borrow();
    let sel = u.tree.selection();
    if let Some((_, iter)) = sel.selected() {
        let model = u.store.upcast_ref::<gtk::TreeModel>();
        let path: String = model.get(&iter, 1);
        let is_dir: bool = model.get(&iter, 2);
        return (path, is_dir);
    }
    (String::new(), true)
}

#[allow(deprecated)]
pub(crate) fn tree_popup(ui: &UiRc, x: f64, y: f64) {
    let tree = ui.borrow().tree.clone();
    if let Some((Some(path), _, _, _)) = tree.path_at_pos(x as i32, y as i32) {
        tree.selection().select_path(&path);
    }
    let (rel, is_dir) = selected_tree_entry(ui);
    let mut itens: Vec<(&str, Box<dyn FnOnce()>)> = Vec::new();
    let dir = parent_dir(&rel, is_dir);
    let novo_arq = if is_dir && !rel.is_empty() {
        "Novo arquivo nesta pasta…"
    } else {
        "Novo arquivo…"
    };
    let nova_pasta = if is_dir && !rel.is_empty() {
        "Nova pasta nesta pasta…"
    } else {
        "Nova pasta…"
    };
    itens.push((
        novo_arq,
        Box::new({
            let ui = Rc::clone(ui);
            let dir = dir.clone();
            move || criar_arquivo(&ui, &dir)
        }),
    ));
    itens.push((
        nova_pasta,
        Box::new({
            let ui = Rc::clone(ui);
            let dir = dir.clone();
            move || criar_pasta(&ui, &dir)
        }),
    ));
    if !rel.is_empty() {
        itens.push((
            "Renomear…",
            Box::new({
                let ui = Rc::clone(ui);
                let rel = rel.clone();
                move || renomear_entrada(&ui, &rel)
            }),
        ));
        itens.push((
            "Apagar",
            Box::new({
                let ui = Rc::clone(ui);
                let rel = rel.clone();
                move || confirmar_apagar(&ui, &rel)
            }),
        ));
    }
    mostrar_menu(ui, tree.upcast_ref(), x, y, itens);
}

/// A right-click menu at `(x, y)` of `widget`: one button per item, and the
/// chosen item runs after the menu closes.
pub(crate) fn mostrar_menu(
    ui: &UiRc,
    widget: &gtk::Widget,
    x: f64,
    y: f64,
    itens: Vec<(&str, Box<dyn FnOnce()>)>,
) {
    let window = ui.borrow().window.clone();
    let (px, py) = widget
        .translate_coordinates(&window, x, y)
        .unwrap_or((x, y));
    let pop = gtk::Popover::new();
    pop.add_css_class("menu");
    pop.set_parent(&window);
    pop.set_has_arrow(false);
    pop.set_autohide(true);
    pop.set_pointing_to(Some(&gtk::gdk::Rectangle::new(
        px.round() as i32,
        py.round() as i32,
        1,
        1,
    )));
    {
        let pop_c = pop.clone();
        pop.connect_closed(move |_| {
            let p = pop_c.clone();
            glib::idle_add_local_once(move || p.unparent());
        });
    }
    let box_ = GtkBox::new(Orientation::Vertical, 0);
    for (label, action) in itens {
        let btn = Button::with_label(label);
        btn.set_has_frame(false);
        btn.set_halign(gtk::Align::Start);
        btn.set_hexpand(true);
        let pop_c = pop.clone();
        let action = RefCell::new(Some(action));
        btn.connect_clicked(move |_| {
            pop_c.popdown();
            if let Some(f) = action.borrow_mut().take() {
                f();
            }
        });
        box_.append(&btn);
    }
    pop.set_child(Some(&box_));
    glib::idle_add_local_once(move || {
        pop.popup();
    });
}

pub(crate) fn criar_arquivo(ui: &UiRc, dir: &str) {
    let window = ui.borrow().window.clone();
    let Some(name) = perguntar_linha(&window, "Nome do arquivo:") else {
        return;
    };
    let name = name.trim().to_string();
    if name.is_empty() {
        return;
    }
    let name = if name.contains('.') {
        name
    } else {
        format!("{name}.lep")
    };
    let path = join_path(dir, &name);
    let (files, project) = {
        let u = ui.borrow();
        (u.files.clone(), u.project.clone())
    };
    let target = path.clone();
    async_result(
        ui,
        &format!("Criando {path}"),
        false,
        move || files.create_file(&project, &target, "escreva(\"Olá\")\n"),
        move |ui, result| match result {
            Ok(()) => {
                refresh_tree(ui);
                open_path(ui, &path);
                set_status(ui, &format!("criado {path}"));
            }
            Err(e) => set_status(ui, &format!("criar: {e}")),
        },
    );
}

pub(crate) fn criar_pasta(ui: &UiRc, dir: &str) {
    let window = ui.borrow().window.clone();
    let Some(name) = perguntar_linha(&window, "Nome da pasta:") else {
        return;
    };
    if name.trim().is_empty() {
        return;
    }
    let path = join_path(dir, name.trim());
    let (files, project) = {
        let u = ui.borrow();
        (u.files.clone(), u.project.clone())
    };
    let target = path.clone();
    async_result(
        ui,
        &format!("Criando pasta {path}"),
        false,
        move || files.mkdir(&project, &target),
        move |ui, result| match result {
            Ok(()) => {
                refresh_tree(ui);
                set_status(ui, &format!("pasta {path}"));
            }
            Err(e) => set_status(ui, &format!("pasta: {e}")),
        },
    );
}

pub(crate) fn renomear_entrada(ui: &UiRc, rel: &str) {
    let window = ui.borrow().window.clone();
    let base = rel.rsplit('/').next().unwrap_or(rel).to_string();
    let Some(name) = perguntar_linha(&window, &format!("Renomear `{base}`:")) else {
        return;
    };
    if name.trim().is_empty() {
        return;
    }
    let to = join_path(&parent_dir(rel, false), name.trim());
    let (files, project) = {
        let u = ui.borrow();
        (u.files.clone(), u.project.clone())
    };
    invalidate_file_operations(ui, rel);
    let from = rel.to_string();
    let target = to.clone();
    let rel = rel.to_string();
    async_result(
        ui,
        &format!("Renomeando {rel}"),
        false,
        move || files.rename(&project, &from, &target),
        move |ui, result| match result {
            Ok(()) => {
                invalidate_file_operations(ui, &rel);
                invalidate_file_operations(ui, &to);
                for d in all_docs(ui) {
                    let n = d.name();
                    if n == rel {
                        d.set_path(to.clone());
                        sessao::vigiar(ui, &d);
                    } else if let Some(rest) = n.strip_prefix(&format!("{rel}/")) {
                        d.set_path(format!("{to}/{rest}"));
                        sessao::vigiar(ui, &d);
                    }
                }
                ui.borrow().file_cache.borrow_mut().clear();
                refresh_tree(ui);
                refresh_title(ui);
                set_status(ui, &format!("{rel} → {to}"));
            }
            Err(e) => set_status(ui, &format!("renomear: {e}")),
        },
    );
}

pub(crate) fn confirmar_apagar(ui: &UiRc, rel: &str) {
    if rel.is_empty() {
        return;
    }
    let window = ui.borrow().window.clone();
    let then = {
        let ui = Rc::clone(ui);
        let rel = rel.to_string();
        move || {
            let (files, project) = {
                let u = ui.borrow();
                (u.files.clone(), u.project.clone())
            };
            let snapshots: Vec<_> = all_docs(&ui)
                .into_iter()
                .map(|d| (Rc::downgrade(&d), d.name(), d.edit_epoch.get()))
                .collect();
            invalidate_file_operations(&ui, &rel);
            let path = rel.clone();
            async_result(
                &ui,
                &format!("Apagando {rel}"),
                false,
                move || files.delete_file(&project, &path),
                move |ui, result| match result {
                    Ok(()) => {
                        invalidate_file_operations(ui, &rel);
                        let mut preserved = false;
                        for (weak, name, epoch) in snapshots {
                            let Some(doc) = weak.upgrade() else {
                                continue;
                            };
                            if doc.name() != name
                                || !(name == rel || name.starts_with(&format!("{rel}/")))
                            {
                                continue;
                            }
                            if doc.edit_epoch.get() != epoch {
                                preserved = true;
                                continue;
                            }
                            doc.mark_saved();
                            remove_tab(ui, &doc);
                        }
                        refresh_tree(ui);
                        set_status(
                            ui,
                            &if preserved {
                                format!("apagado {rel}; novas edições foram preservadas nas abas")
                            } else {
                                format!("apagado {rel}")
                            },
                        );
                    }
                    Err(e) => set_status(ui, &format!("apagar: {e}")),
                },
            );
        }
    };
    let then = RefCell::new(Some(then));
    let dlg = gtk::MessageDialog::builder()
        .transient_for(&window)
        .modal(true)
        .message_type(gtk::MessageType::Warning)
        .buttons(gtk::ButtonsType::None)
        .text(format!("Apagar `{rel}`?"))
        .build();
    dlg.add_button("Cancelar", gtk::ResponseType::Cancel);
    dlg.add_button("Apagar", gtk::ResponseType::Accept);
    dlg.connect_response(move |d, resp| {
        d.close();
        if resp == gtk::ResponseType::Accept
            && let Some(f) = then.borrow_mut().take()
        {
            f();
        }
    });
    dlg.present();
}
