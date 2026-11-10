//! What survives closing the editor, per project:
//!
//! - the session: open tabs, the line each was at, breakpoints and the
//!   active tab, reopened next time;
//! - copies of unsaved text, every few seconds, so a crash or a power cut
//!   does not lose a lesson's work: next time the editor offers them back.
//!
//! Also watches the open files of a local project, so a change made by
//! another program shows up (or is warned about) in the tab.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::rc::Rc;

use gtk::gio;
use gtk::glib;
use gtk::prelude::*;

use crate::docs::Doc;
use crate::recuperacao::nome_de_volta;
use crate::recuperacao::{
    self, RecoveryStore, UNTITLED_PREFIX as PREFIXO_SEM_TITULO, nome_de_arquivo,
};
use crate::store::Files;
use crate::{
    Ui, UiRc, all_docs, current_doc, open_doc, remember_arquivo, set_status, switch_to,
    with_documents,
};

/// How often unsaved text is copied aside.
const INTERVALO_DAS_COPIAS: u32 = 5;

// GTK returns the position in each dialog's `.buttons` list.
mod resposta_recuperacao {
    pub const CANCELAR: i32 = 0;
    pub const DESCARTAR: i32 = 1;
    pub const RECUPERAR: i32 = 2;
    pub const EXPORTAR: i32 = 3;
    pub const TENTAR_NOVAMENTE: i32 = 4;
}

mod resposta_falhas {
    pub const FECHAR: i32 = 0;
    pub const TENTAR_NOVAMENTE: i32 = 1;
    pub const EXPORTAR: i32 = 2;
}

/// One tab of a saved session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AbaSalva {
    pub caminho: String,
    /// 1-based.
    pub linha: u32,
    pub pontos: Vec<u32>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Sessao {
    pub abas: Vec<AbaSalva>,
    pub ativa: Option<String>,
}

/// One line per tab (`aba`, path, line, breakpoints) and the active one.
pub fn sessao_em_texto(s: &Sessao) -> String {
    let mut t = String::new();
    for a in &s.abas {
        let pontos: Vec<String> = a.pontos.iter().map(u32::to_string).collect();
        t.push_str(&format!(
            "aba\t{}\t{}\t{}\n",
            a.caminho,
            a.linha,
            pontos.join(",")
        ));
    }
    if let Some(ativa) = &s.ativa {
        t.push_str(&format!("ativa\t{ativa}\n"));
    }
    t
}

/// Reads [`sessao_em_texto`]; lines it does not understand are skipped.
pub fn sessao_de_texto(t: &str) -> Sessao {
    let mut s = Sessao::default();
    for linha in t.lines() {
        let partes: Vec<&str> = linha.split('\t').collect();
        match partes.as_slice() {
            ["aba", caminho, linha, pontos] if !caminho.is_empty() => s.abas.push(AbaSalva {
                caminho: caminho.to_string(),
                linha: linha.parse().unwrap_or(1),
                pontos: pontos.split(',').filter_map(|p| p.parse().ok()).collect(),
            }),
            ["ativa", caminho] => s.ativa = Some(caminho.to_string()),
            _ => {}
        }
    }
    s
}

/// What identifies the open project across runs, or `None` without one.
fn chave(u: &Ui) -> Option<String> {
    match &u.files {
        Files::None => None,
        Files::Local(p) => Some(format!("local {}", p.display())),
        Files::Remote { projeto, .. } => Some(format!(
            "servidor {} {} {projeto}",
            u.prefs.servidor, u.project
        )),
    }
}

fn pasta_base() -> PathBuf {
    glib::user_data_dir().join("expressa-aula")
}

fn arquivo_da_sessao(chave: &str) -> PathBuf {
    pasta_base()
        .join("sessoes")
        .join(format!("{}.txt", nome_de_arquivo(chave)))
}

fn pasta_das_copias(chave: &str) -> PathBuf {
    pasta_base()
        .join("recuperacao")
        .join(nome_de_arquivo(chave))
}

/// Saves the open tabs of this project. Does nothing until the session of
/// the project was restored (it would overwrite it with half the tabs).
pub fn salvar(ui: &UiRc) {
    let Ok(u) = ui.try_borrow() else { return };
    if !u.sessao_pronta {
        return;
    }
    let Some(chave) = chave(&u) else { return };
    let nb = u.notebook.clone();
    drop(u);
    let mut docs = all_docs(ui);
    docs.sort_by_key(|d| nb.page_num(&d.page));
    let sessao = Sessao {
        abas: docs
            .iter()
            .filter(|d| !d.is_untitled())
            .map(|d| AbaSalva {
                caminho: d.name(),
                linha: d.cursor().line() as u32 + 1,
                pontos: d.breakpoint_lines().into_iter().collect(),
            })
            .collect(),
        ativa: current_doc(ui)
            .filter(|d| !d.is_untitled())
            .map(|d| d.name()),
    };
    let arquivo = arquivo_da_sessao(&chave);
    let texto = sessao_em_texto(&sessao);
    if let Err(e) = crate::background::persistence().dispatch(move || {
        if let Some(pasta) = arquivo.parent() {
            std::fs::create_dir_all(pasta).map_err(|e| e.to_string())?;
        }
        std::fs::write(arquivo, texto).map_err(|e| e.to_string())
    }) {
        set_status(ui, &e);
    }
}

/// The project was just listed: reopen its tabs, then offer copies of text
/// left unsaved last time. Without a saved session, reopens the last file.
pub fn restaurar(ui: &UiRc) {
    let project_key = chave(&ui.borrow());
    let Some(chave) = project_key else {
        ui.borrow_mut().sessao_pronta = true;
        oferecer_copias(ui, "sem-projeto");
        return;
    };
    let arquivo = arquivo_da_sessao(&chave);
    crate::async_result(
        ui,
        "Restaurando abas",
        true,
        move || {
            Ok(std::fs::read_to_string(arquivo)
                .map(|t| sessao_de_texto(&t))
                .unwrap_or_default())
        },
        move |ui, result| {
            let Ok(mut sessao) = result else {
                ui.borrow_mut().sessao_pronta = true;
                return;
            };
            let paths = ui.borrow().project_files.clone();
            if sessao.abas.is_empty() {
                let ultimo = ui.borrow().prefs.arquivo.clone();
                if !ultimo.is_empty() && paths.contains(&ultimo) {
                    sessao.abas.push(AbaSalva {
                        caminho: ultimo,
                        linha: 1,
                        pontos: Vec::new(),
                    });
                }
            }
            sessao.abas.retain(|a| paths.contains(&a.caminho));
            restore_next(ui, Rc::new(sessao), 0, chave);
        },
    );
}

fn restore_next(ui: &UiRc, sessao: Rc<Sessao>, index: usize, key: String) {
    if let Some(aba) = sessao.abas.get(index) {
        let path = aba.caminho.clone();
        crate::open_path_then(ui, &path, move |ui, doc| {
            if let Some(doc) = doc {
                let aba = &sessao.abas[index];
                for &p in &aba.pontos {
                    if !doc.breakpoint_lines().contains(&p) {
                        doc.toggle_breakpoint(p);
                    }
                }
                doc.goto_line(aba.linha);
            }
            restore_next(ui, sessao, index + 1, key);
        });
    } else {
        if let Some(doc) = sessao
            .ativa
            .as_deref()
            .and_then(|a| crate::doc_by_name(ui, a))
        {
            switch_to(ui, &doc);
            remember_arquivo(ui, &doc);
        }
        ui.borrow_mut().sessao_pronta = true;
        if !ui.borrow().files.is_remote() {
            oferecer_copias(ui, &key);
        }
    }
}

/// Untitled work has a recovery scope even before a project is chosen.
fn chave_das_copias(u: &Ui) -> String {
    chave(u).unwrap_or_else(|| "sem-projeto".to_string())
}

/// Offer scratch recovery before automatically opening the last project. Keep
/// recovered tabs when a project is subsequently chosen (see `after_unsaved`).
pub fn restaurar_sem_projeto(ui: &UiRc, done: impl FnOnce(&UiRc, bool) + 'static) {
    if !matches!(ui.borrow().files, Files::None) {
        done(ui, false);
        return;
    }
    ui.borrow_mut().sessao_pronta = true;
    oferecer_copias_then(ui, "sem-projeto", done);
}

pub fn synchronization_context(ui: &UiRc) -> Option<(RecoveryStore, Vec<PathBuf>)> {
    let u = ui.borrow();
    if !u.files.is_remote() {
        return None;
    }
    let key = chave(&u)?;
    drop(u);
    Some((RecoveryStore(pasta_das_copias(&key)), copias_abertas(ui)))
}

pub fn review_sync_edits(ui: &UiRc) {
    if ui.borrow().files.is_remote() {
        revisar_copias(ui, true);
    }
}

fn oferecer_copias_atual(ui: &UiRc) {
    revisar_copias(ui, false);
}

fn revisar_copias(ui: &UiRc, somente_atual: bool) {
    copy_async(
        ui,
        Some(Box::new(move |ui, success| {
            if success {
                revisar_copias_prontas(ui, somente_atual);
            } else {
                let key = chave_das_copias(&ui.borrow());
                mostrar_falhas(ui, RecoveryStore(pasta_das_copias(&key)), vec!["As edições ainda não foram todas guardadas. Tente novamente ou exporte o texto das abas.".into()]);
            }
        })),
    );
}

fn revisar_copias_prontas(ui: &UiRc, somente_atual: bool) {
    let key = chave_das_copias(&ui.borrow());
    if somente_atual {
        oferecer_copias(ui, &key);
        return;
    }
    let owned = copias_abertas(ui);
    let root = pasta_base().join("recuperacao");
    crate::async_result(
        ui,
        "Procurando edições guardadas",
        true,
        move || projetos_com_copias(&root, &owned),
        move |ui, result| {
            let keys = match result {
                Ok(keys) => keys,
                Err(e) => {
                    mostrar_falhas(ui, RecoveryStore(pasta_das_copias(&key)), vec![e]);
                    return;
                }
            };
            if keys.is_empty() {
                set_status(ui, "não há alterações pendentes para recuperar");
            } else if keys.len() == 1 && keys[0] == key {
                oferecer_copias(ui, &key);
            } else {
                // Recovery is local even for server projects: no connection is needed.
                let items = keys
                    .iter()
                    .map(|key| crate::escolher::Item {
                        rotulo: key.clone(),
                        detalhe: "alterações guardadas neste computador".into(),
                    })
                    .collect();
                let window = crate::sincronizacao::parent(ui);
                let ui = Rc::clone(ui);
                crate::escolher::mostrar(
                    &window,
                    "Revisar edições guardadas",
                    "projeto de origem",
                    items,
                    move |i| {
                        oferecer_copias(&ui, &keys[i]);
                    },
                );
            }
        },
    );
}

fn copias_abertas(ui: &UiRc) -> Vec<PathBuf> {
    all_docs(ui)
        .iter()
        .flat_map(|doc| {
            doc.copia_recuperacao
                .borrow()
                .as_ref()
                .map(|backup| backup.paths.clone())
                .unwrap_or_default()
        })
        .collect()
}

fn projetos_com_copias(root: &Path, owned: &[PathBuf]) -> Result<Vec<String>, String> {
    let entries = match std::fs::read_dir(root) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(format!("{}: {e}", root.display())),
    };
    let mut keys = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        if !entry.file_type().map_err(|e| e.to_string())?.is_dir() {
            continue;
        }
        let Some(key) = nome_de_volta(&entry.file_name().to_string_lossy()) else {
            continue;
        };
        let scan = RecoveryStore(entry.path()).scan();
        if scan
            .copies
            .iter()
            .any(|copy| !copy.backup.paths.iter().any(|path| owned.contains(path)))
            || !scan.errors.is_empty()
        {
            keys.push(key);
        }
    }
    keys.sort();
    Ok(keys)
}

/// Return whether pending copies/errors need attention. Cancellation and failed
/// reads leave every affected copy on disk, including across a normal shutdown.
fn oferecer_copias(ui: &UiRc, key: &str) {
    oferecer_copias_then(ui, key, |_, _| {});
}

fn oferecer_copias_then(ui: &UiRc, key: &str, done: impl FnOnce(&UiRc, bool) + 'static) {
    if ui.borrow().recuperacao_aberta {
        done(ui, true);
        return;
    }
    ui.borrow_mut().recuperacao_aberta = true;
    let key = key.to_string();
    let store = RecoveryStore(pasta_das_copias(&key));
    let work_store = store.clone();
    let owned = copias_abertas(ui);
    crate::async_result(
        ui,
        "Lendo edições guardadas",
        true,
        move || {
            let mut scan = work_store.scan();
            scan.copies
                .retain(|c| !c.backup.paths.iter().any(|p| owned.contains(p)));
            Ok(scan)
        },
        move |ui, result| {
            ui.borrow_mut().recuperacao_aberta = false;
            match result {
                Ok(scan) => {
                    let offered = mostrar_copias(ui, &key, store, scan);
                    done(ui, offered);
                }
                Err(e) => {
                    set_status(ui, &e);
                    done(ui, false);
                }
            }
        },
    );
}

fn mostrar_copias(ui: &UiRc, key: &str, store: RecoveryStore, scan: recuperacao::Scan) -> bool {
    use resposta_recuperacao as resposta;
    if scan.copies.is_empty() && scan.errors.is_empty() {
        return false;
    }
    let names: Vec<&str> = scan
        .copies
        .iter()
        .map(|copy| sem_prefixo(&copy.backup.name))
        .collect();
    let detail = format!(
        "Origem: {key}\n{}\n\nAbrir o texto em abas permite revisar suas edições antes de salvá-las. Depois mantém as cópias guardadas.{}",
        names.join(", "),
        if scan.errors.is_empty() {
            String::new()
        } else {
            format!(
                "\n\nAlgumas cópias não puderam ser lidas e serão preservadas:\n{}",
                scan.errors.join("\n")
            )
        }
    );
    let dialog = gtk::AlertDialog::builder()
        .modal(true)
        .message("Há texto das abas guardado neste computador")
        .detail(detail)
        .buttons([
            "Depois",
            "Descartar cópias",
            "Abrir texto em abas",
            "Salvar cópias em outra pasta…",
            "Verificar novamente",
        ])
        .cancel_button(resposta::CANCELAR)
        .default_button(if scan.copies.is_empty() {
            resposta::TENTAR_NOVAMENTE
        } else {
            resposta::RECUPERAR
        })
        .build();
    let window = crate::sincronizacao::parent(ui);
    ui.borrow_mut().recuperacao_aberta = true;
    let ui = Rc::clone(ui);
    let key = key.to_string();
    let active_key = chave_das_copias(&ui.borrow());
    let detached = key != active_key;
    dialog.choose(Some(&window), None::<&gio::Cancellable>, move |result| {
        ui.borrow_mut().recuperacao_aberta = false;
        // A late dialog cannot restore into, or discard copies for, a new project.
        if chave_das_copias(&ui.borrow()) != active_key {
            return;
        }
        let mut errors = scan.errors;
        match result {
            Ok(resposta::DESCARTAR) => {
                crate::async_on(crate::background::persistence(), true, &ui, "Descartando cópias", false,
                    move || { for copy in scan.copies { if let Err(e) = copy.backup.remove() { errors.push(e); } } Ok(errors) },
                    move |ui, result| match result {
                        Ok(errors) if errors.is_empty() => set_status(ui, "as cópias selecionadas foram descartadas"),
                        Ok(errors) => mostrar_falhas(ui, store, errors),
                        Err(e) => mostrar_falhas(ui, store, vec![e]),
                    });
            }
            Ok(resposta::RECUPERAR) => {
                let (files, student) = { let u = ui.borrow(); (u.files.clone(), u.project.clone()) };
                let existing: Vec<_> = all_docs(&ui).iter().map(|d| d.name()).collect();
                crate::async_result(&ui, "Abrindo edições guardadas", true,
                    move || Ok(scan.copies.into_iter().map(|copy| {
                        let hash = resumo(&copy.text);
                        let original = if detached { Err(format!("o projeto de origem não está aberto: {key}")) }
                            else if copy.backup.name.starts_with(PREFIXO_SEM_TITULO) || existing.contains(&copy.backup.name) { Ok(None) }
                            else { files.current_text(&student, &copy.backup.name) };
                        (copy, hash, original)
                    }).collect::<Vec<_>>()),
                    move |ui, result| {
                        let mut restored = 0;
                        match result {
                            Ok(copies) => for (copy, hash, original) in copies {
                                let result = if detached {
                                    crate::documentos::Documents::new(&Files::None, "", &[]).recover_loaded(&copy, original, |_, text| Ok(open_doc(ui, None, text)))
                                } else {
                                    with_documents(ui, |documents| documents.recover_loaded(&copy, original, |path, text| Ok(open_doc(ui, path, text))))
                                };
                                match result {
                                    Ok(recovered) => {
                                        recovered.doc.resumo_da_copia.set(Some(hash));
                                        switch_to(ui, &recovered.doc);
                                        if let Some(warning) = recovered.warning { ui.borrow().execucao.console.stderr(&format!("aviso: `{}` recuperado em uma nova aba: {warning}\n", sem_prefixo(&copy.backup.name))); }
                                        restored += 1;
                                    }
                                    Err(e) => errors.push(e),
                                }
                            },
                            Err(e) => errors.push(e),
                        }
                        set_status(ui, &format!("{restored} arquivo(s) recuperado(s): confira e salve"));
                        if !errors.is_empty() { mostrar_falhas(ui, store, errors); }
                    });
            }
            Ok(resposta::EXPORTAR) => {
                exportar_copias(&ui, store);
            }
            Ok(resposta::TENTAR_NOVAMENTE) => {
                oferecer_copias(&ui, &key);
            }
            _ => {
                set_status(
                    &ui,
                    "edições preservadas neste computador; você poderá revisá-las ao abrir o projeto ou em Sincronização, para projetos no servidor",
                );
            }
        }
    });
    true
}

fn mostrar_falhas(ui: &UiRc, store: RecoveryStore, errors: Vec<String>) {
    use resposta_falhas as resposta;

    let dialog = gtk::AlertDialog::builder()
        .modal(true)
        .message("Algumas cópias foram preservadas")
        .detail(errors.join("\n"))
        .buttons(["Fechar", "Tentar novamente", "Exportar…"])
        .cancel_button(resposta::FECHAR)
        .default_button(resposta::TENTAR_NOVAMENTE)
        .build();
    let window = crate::sincronizacao::parent(ui);
    let ui = Rc::clone(ui);
    dialog.choose(
        Some(&window),
        None::<&gio::Cancellable>,
        move |result| match result {
            Ok(resposta::TENTAR_NOVAMENTE) => oferecer_copias_atual(&ui),
            Ok(resposta::EXPORTAR) => exportar_copias(&ui, store),
            _ => {}
        },
    );
}

fn exportar_copias(ui: &UiRc, store: RecoveryStore) {
    let dialog = gtk::FileDialog::builder()
        .title("Exportar alterações recuperáveis")
        .accept_label("Exportar")
        .modal(false)
        .build();
    let window = crate::sincronizacao::parent(ui);
    let ui = Rc::clone(ui);
    dialog.select_folder(Some(&window), None::<&gio::Cancellable>, move |result| {
        let Some(folder) = result.ok().and_then(|file| file.path()) else {
            return;
        };
        let mut current = Vec::new();
        if pasta_das_copias(&chave_das_copias(&ui.borrow())) == store.0 {
            for (index, doc) in all_docs(&ui)
                .iter()
                .filter(|doc| doc.is_dirty())
                .enumerate()
            {
                let backup =
                    doc.copia_recuperacao
                        .borrow()
                        .clone()
                        .unwrap_or_else(|| recuperacao::Backup {
                            id: format!("export:{index}"),
                            name: nome_da_copia(doc),
                            revision: 0,
                            paths: Vec::new(),
                        });
                current.push(recuperacao::Copy {
                    backup,
                    text: doc.text(),
                });
            }
        }

        let work_store = store.clone();
        crate::async_result(
            &ui,
            "Exportando edições",
            false,
            move || {
                let mut scan = work_store.scan();
                for copy in current {
                    scan.copies.retain(|old| old.backup.id != copy.backup.id);
                    scan.copies.push(copy);
                }
                let mut errors = if scan.unreadable.is_empty() {
                    scan.errors
                } else {
                    Vec::new()
                };
                errors.extend(recuperacao::export(&scan.copies, &folder));
                errors.extend(recuperacao::export_unreadable(&scan.unreadable, &folder));
                Ok(errors)
            },
            move |ui, result| match result {
                Ok(errors) if errors.is_empty() => set_status(
                    ui,
                    "cópias exportadas; as cópias de recuperação foram preservadas",
                ),
                Ok(errors) => mostrar_falhas(ui, store, errors),
                Err(e) => mostrar_falhas(ui, store, vec![e]),
            },
        );
    });
}

fn sem_prefixo(nome: &str) -> &str {
    nome.strip_prefix(PREFIXO_SEM_TITULO).unwrap_or(nome)
}

fn nome_da_copia(d: &Doc) -> String {
    if d.is_untitled() {
        format!("{PREFIXO_SEM_TITULO}{}", d.untitled)
    } else {
        d.name()
    }
}

fn resumo(texto: &str) -> u64 {
    let mut h = DefaultHasher::new();
    texto.hash(&mut h);
    h.finish()
}

fn nome_de_backup(doc: &Doc) -> String {
    if doc.is_untitled() {
        doc.copia_recuperacao
            .borrow()
            .as_ref()
            .map_or_else(|| nome_da_copia(doc), |backup| backup.name.clone())
    } else {
        doc.name()
    }
}

#[cfg(test)]
fn copia_atual(store: &RecoveryStore, doc: &Doc, hash: u64, name: &str) -> bool {
    doc.resumo_da_copia.get() == Some(hash)
        && doc
            .copia_recuperacao
            .borrow()
            .as_ref()
            .is_some_and(|backup| {
                backup.name == name
                    && backup.paths.first().is_some_and(|path| {
                        path.is_file() && path.parent() == Some(store.0.as_path())
                    })
            })
}

/// Persist a dirty document's next snapshot before retiring its previous copy.
/// The store is explicit so filesystem failures and restart behavior are testable.
#[cfg(test)]
fn copiar_documento(store: &RecoveryStore, doc: &Doc) -> Result<(), String> {
    if !doc.is_dirty() {
        return Ok(());
    }
    let text = doc.text();
    let hash = resumo(&text);
    let previous = doc.copia_recuperacao.borrow().clone();
    let name = nome_de_backup(doc);
    if copia_atual(store, doc, hash, &name) {
        return Ok(());
    }
    let backup = store.save(&name, &text, previous.as_ref())?;
    *doc.copia_recuperacao.borrow_mut() = Some(backup);
    doc.resumo_da_copia.set(Some(hash));
    Ok(())
}

pub fn copiar_nao_salvos(ui: &UiRc) {
    if ui.borrow().closing.get() || ui.borrow().recuperacao_aberta {
        return;
    }
    copy_async(ui, None);
}

fn copy_async(ui: &UiRc, done: Option<Box<dyn FnOnce(&UiRc, bool)>>) {
    if let Some(done) = done {
        *ui.borrow().backup_review.borrow_mut() = Some(done);
    }
    if ui.borrow().backup_busy.replace(true) {
        return;
    }
    let key = chave_das_copias(&ui.borrow());
    let store = RecoveryStore(pasta_das_copias(&key));
    let docs: Vec<_> = all_docs(ui).into_iter().filter(|d| d.is_dirty()).collect();
    let snapshots: Vec<_> = docs
        .iter()
        .map(|d| {
            (
                nome_de_backup(d),
                d.text(),
                d.edit_epoch.get(),
                d.copia_recuperacao.borrow().clone(),
                d.resumo_da_copia.get(),
                d.recovery.clone(),
            )
        })
        .collect();
    let docs: Vec<_> = docs.iter().map(Rc::downgrade).collect();
    crate::async_on(
        crate::background::persistence(),
        true,
        ui,
        "",
        false,
        move || {
            Ok(snapshots
                .into_iter()
                .map(|(name, text, epoch, previous, old_hash, owner)| {
                    let hash = resumo(&text);
                    owner
                        .save(
                            &store,
                            &name,
                            &text,
                            epoch,
                            previous,
                            old_hash == Some(hash),
                        )
                        .map(|backup| (hash, backup))
                })
                .collect::<Vec<Result<_, String>>>())
        },
        move |ui, result| {
            ui.borrow().backup_busy.set(false);
            let mut success = true;
            match result {
                Ok(results) => {
                    for (doc, result) in docs.into_iter().zip(results) {
                        match result {
                            Ok((hash, Some(backup))) => {
                                if let Some(doc) = doc.upgrade()
                                    && all_docs(ui).iter().any(|open| Rc::ptr_eq(open, &doc))
                                {
                                    *doc.copia_recuperacao.borrow_mut() = Some(backup);
                                    doc.resumo_da_copia.set(Some(hash));
                                }
                            }
                            Ok((_, None)) => {}
                            Err(e) => {
                                success = false;
                                set_status(
                                    ui,
                                    "não foi possível guardar uma cópia de segurança; salve ou exporte a aba",
                                );
                                ui.borrow()
                                    .execucao
                                    .console
                                    .stderr(&format!("erro na cópia de recuperação: {e}\n"));
                            }
                        }
                    }
                }
                Err(e) => {
                    success = false;
                    set_status(ui, &e);
                }
            }
            let done = ui.borrow().backup_review.borrow_mut().take();
            if let Some(done) = done {
                done(ui, success);
            }
        },
    );
}

/// Explicit acknowledgement, ordered after pending recovery writes.
pub fn limpar_copias(ui: &UiRc) {
    let docs = all_docs(ui);
    for doc in &docs {
        doc.recovery.close();
    }
    crate::forget_recovery_docs_async(ui, docs);
}

pub fn stop_copies(ui: &UiRc) {
    if let Some(source) = ui.borrow().recovery_timer.borrow_mut().take() {
        source.remove();
    }
}

pub fn iniciar_copias(ui: &UiRc) {
    stop_copies(ui);
    let weak = Rc::downgrade(ui);
    let source = glib::timeout_add_seconds_local(INTERVALO_DAS_COPIAS, move || {
        let Some(ui) = weak.upgrade() else {
            return glib::ControlFlow::Break;
        };
        if ui.borrow().closing.get() {
            ui.borrow().recovery_timer.borrow_mut().take();
            return glib::ControlFlow::Break;
        }
        copiar_nao_salvos(&ui);
        glib::ControlFlow::Continue
    });
    *ui.borrow().recovery_timer.borrow_mut() = Some(source);
}

/// Local projects: watch `doc`'s file. Changed by another program, the tab
/// reloads (no unsaved changes) or warns (it has some).
pub fn vigiar(ui: &UiRc, doc: &Rc<Doc>) {
    let raiz = match &ui.borrow().files {
        Files::Local(raiz) if !doc.is_untitled() => raiz.clone(),
        _ => {
            doc.vigia.replace(None);
            return;
        }
    };
    let nome = doc.name();
    let caminho = raiz.join(&nome);
    let Ok(monitor) = gio::File::for_path(&caminho)
        .monitor_file(gio::FileMonitorFlags::WATCH_MOVES, gio::Cancellable::NONE)
    else {
        return;
    };
    let ui_w = Rc::downgrade(ui);
    let doc_w = Rc::downgrade(doc);
    monitor.connect_changed(move |_, _, _, evento| {
        if !matches!(
            evento,
            gio::FileMonitorEvent::ChangesDoneHint
                | gio::FileMonitorEvent::Created
                | gio::FileMonitorEvent::Renamed
                | gio::FileMonitorEvent::MovedIn
        ) {
            return;
        }
        if let (Some(ui), Some(doc)) = (ui_w.upgrade(), doc_w.upgrade())
            && doc.name() == nome
        {
            mudou_fora(&ui, &doc);
        }
    });
    doc.vigia.replace(Some(monitor));
}

pub(crate) fn check_deferred(ui: &UiRc, doc: &Rc<Doc>) {
    if doc.pending_external_change.replace(false) {
        mudou_fora(ui, doc);
    }
}

fn mudou_fora(ui: &UiRc, doc: &Rc<Doc>) {
    if doc.saving.get() {
        doc.pending_external_change.set(true);
        return;
    }
    doc.io_epoch.set(doc.io_epoch.get() + 1);
    let (files, project) = {
        let u = ui.borrow();
        (u.files.clone(), u.project.clone())
    };
    let name = doc.name();
    let path = name.clone();
    let epoch = doc.edit_epoch.get();
    let io = doc.io_epoch.get();
    let weak = Rc::downgrade(doc);
    crate::async_result(
        ui,
        "",
        false,
        move || files.current_text(&project, &path),
        move |ui, result| {
            let (Some(doc), Ok(Some(snapshot))) = (weak.upgrade(), result) else {
                return;
            };
            if doc.saving.get() {
                doc.pending_external_change.set(true);
                return;
            }
            if doc.name() != name
                || doc.io_epoch.get() != io
                || !all_docs(ui).iter().any(|open| Rc::ptr_eq(open, &doc))
            {
                return;
            }
            if *doc.revision.borrow() == snapshot.revision || snapshot.text == doc.text() {
                return;
            }
            if doc.is_dirty() || doc.edit_epoch.get() != epoch {
                set_status(
                    ui,
                    &format!("`{name}` mudou fora do editor; salvar exigirá resolver o conflito"),
                );
            } else {
                doc.recarregar_snapshot(&snapshot.text, snapshot.revision);
                set_status(
                    ui,
                    &format!("`{name}` mudou fora do editor e foi recarregado"),
                );
            }
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_round_trip_uses_current_anchored_breakpoint_positions() {
        crate::gtk_test::run(|| {
            let language = sourceview5::LanguageManager::new();
            let search = sourceview5::SearchSettings::new();
            let doc = crate::docs::Doc::new(
                Some("lib/a.lep".into()),
                String::new(),
                "a = 1\nescreva(a)\n",
                &language,
                &search,
            );
            doc.toggle_breakpoint(2);
            doc.buffer.insert(&mut doc.buffer.start_iter(), "// new\n");
            let session = Sessao {
                abas: vec![AbaSalva {
                    caminho: doc.name(),
                    linha: 1,
                    pontos: doc.breakpoint_lines().into_iter().collect(),
                }],
                ativa: Some(doc.name()),
            };
            let restored = sessao_de_texto(&sessao_em_texto(&session));
            assert_eq!(restored.abas[0].pontos, [3]);
            let reopened = crate::docs::Doc::new(
                Some(doc.name()),
                String::new(),
                &doc.text(),
                &language,
                &search,
            );
            for line in &restored.abas[0].pontos {
                reopened.toggle_breakpoint(*line);
            }
            assert_eq!(reopened.breakpoint_lines(), doc.breakpoint_lines());
        });
    }

    #[test]
    fn sessao_vai_e_volta() {
        let s = Sessao {
            abas: vec![
                AbaSalva {
                    caminho: "lib/mat.lep".into(),
                    linha: 12,
                    pontos: vec![3, 9],
                },
                AbaSalva {
                    caminho: "a b.lep".into(),
                    linha: 1,
                    pontos: vec![],
                },
            ],
            ativa: Some("lib/mat.lep".into()),
        };
        assert_eq!(sessao_de_texto(&sessao_em_texto(&s)), s);
        // Junk is skipped, not fatal.
        assert_eq!(sessao_de_texto("lixo\naba\t\t1\t\n"), Sessao::default());
    }

    #[test]
    fn nome_de_arquivo_vai_e_volta() {
        for nome in [
            "lib/mat.lep",
            "sem-titulo:sem título 1",
            "ação/é.lep",
            "a%b",
        ] {
            let f = nome_de_arquivo(nome);
            assert!(
                f.chars()
                    .all(|c| c.is_ascii_alphanumeric() || "-_%".contains(c)),
                "{f}"
            );
            assert_eq!(nome_de_volta(&f).as_deref(), Some(nome));
        }
        assert_ne!(nome_de_arquivo("a/b"), nome_de_arquivo("a_b"));
    }

    #[test]
    fn pending_scopes_are_discoverable_without_opening_the_original_server() {
        let root =
            std::env::temp_dir().join(format!("expressa-recovery-scopes-{}", std::process::id()));
        let remote_key = "servidor http://offline.invalid:50051 ana projeto";
        let remote = RecoveryStore(root.join(nome_de_arquivo(remote_key)));
        let scratch = RecoveryStore(root.join(nome_de_arquivo("sem-projeto")));
        let pending = remote.save("a.lep", "escreva(1)\n", None).unwrap();
        let owned = scratch
            .save("sem-titulo:novo.lep", "escreva(2)\n", None)
            .unwrap();
        assert_eq!(
            projetos_com_copias(&root, &owned.paths).unwrap(),
            [remote_key]
        );
        // Unreadable records remain reachable even when readable copies are owned.
        std::fs::write(remote.0.join("bad.recovery"), "incomplete").unwrap();
        assert_eq!(
            projetos_com_copias(
                &root,
                &[owned.paths.clone(), pending.paths.clone()].concat()
            )
            .unwrap(),
            [remote_key]
        );
        assert!(remote.0.join("bad.recovery").exists());
        // A newer snapshot left in a different project after interrupted
        // migration must remain visible even if an old snapshot is open.
        let old_bytes = std::fs::read(&owned.paths[0]).unwrap();
        remote
            .save(&owned.name, "newer migrated work", Some(&owned))
            .unwrap();
        std::fs::write(&owned.paths[0], old_bytes).unwrap();
        pending.remove().unwrap();
        std::fs::remove_file(remote.0.join("bad.recovery")).unwrap();
        assert_eq!(
            projetos_com_copias(&root, &owned.paths).unwrap(),
            [remote_key]
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn scratch_autosave_migrates_safely_and_failed_writes_can_retry() {
        crate::gtk_test::run(|| {
            let root =
                std::env::temp_dir().join(format!("expressa-scratch-{}", std::process::id()));
            std::fs::create_dir_all(&root).unwrap();
            let scratch = RecoveryStore(root.join("sem-projeto"));
            let project = RecoveryStore(root.join("projeto"));
            let doc = Doc::new(
                None,
                "novo.lep".into(),
                "escreva(1)\n",
                &sourceview5::LanguageManager::new(),
                &sourceview5::SearchSettings::new(),
            );
            doc.buffer.set_modified(true);
            copiar_documento(&scratch, &doc).unwrap();
            assert_eq!(scratch.scan().copies.len(), 1);
            let original_id = doc.copia_recuperacao.borrow().as_ref().unwrap().id.clone();
            // Selecting a project also migrates unchanged scratch text, after
            // publishing the new snapshot; the original is safe until then.
            copiar_documento(&project, &doc).unwrap();
            assert!(scratch.scan().copies.is_empty());
            assert_eq!(project.scan().copies[0].backup.id, original_id);
            let blocked = root.join("blocked");
            std::fs::write(&blocked, "not a directory").unwrap();
            doc.buffer.set_text("escreva(2)\n");
            assert!(copiar_documento(&RecoveryStore(blocked), &doc).is_err());
            assert_eq!(project.scan().copies[0].text, "escreva(1)\n");
            copiar_documento(&project, &doc).unwrap();
            assert_eq!(project.scan().copies[0].text, "escreva(2)\n");
            // Normal close/discard only retires this tab's owned copy.
            project.save("pending.lep", "pending work", None).unwrap();
            crate::documentos::forget_recovery(&doc).unwrap();
            assert_eq!(project.scan().copies.len(), 1);
            assert_eq!(project.scan().copies[0].text, "pending work");
            let _ = std::fs::remove_dir_all(root);
        });
    }
}
