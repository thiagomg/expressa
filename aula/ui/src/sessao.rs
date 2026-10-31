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
    Ui, UiRc, all_docs, current_doc, open_doc, open_path, remember_arquivo, set_status, switch_to,
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
                pontos: d.breakpoints.borrow().iter().copied().collect(),
            })
            .collect(),
        ativa: current_doc(ui)
            .filter(|d| !d.is_untitled())
            .map(|d| d.name()),
    };
    let arquivo = arquivo_da_sessao(&chave);
    if let Some(pasta) = arquivo.parent() {
        let _ = std::fs::create_dir_all(pasta);
    }
    let _ = std::fs::write(arquivo, sessao_em_texto(&sessao));
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
    let sessao = std::fs::read_to_string(arquivo_da_sessao(&chave))
        .map(|t| sessao_de_texto(&t))
        .unwrap_or_default();
    let existe = |c: &str| ui.borrow().project_files.iter().any(|p| p == c);
    if sessao.abas.is_empty() {
        let ultimo = ui.borrow().prefs.arquivo.clone();
        if !ultimo.is_empty() && existe(&ultimo) {
            let _ = open_path(ui, &ultimo);
        }
    }
    for aba in sessao.abas.iter().filter(|a| existe(&a.caminho)) {
        let Some(doc) = open_path(ui, &aba.caminho) else {
            continue;
        };
        for &p in &aba.pontos {
            if !doc.breakpoints.borrow().contains(&p) {
                doc.toggle_breakpoint(p);
            }
        }
        doc.goto_line(aba.linha);
    }
    if let Some(doc) = sessao
        .ativa
        .as_deref()
        .and_then(|a| crate::doc_by_name(ui, a))
    {
        switch_to(ui, &doc);
        remember_arquivo(ui, &doc);
    }
    ui.borrow_mut().sessao_pronta = true;
    oferecer_copias(ui, &chave);
}

/// Untitled work has a recovery scope even before a project is chosen.
fn chave_das_copias(u: &Ui) -> String {
    chave(u).unwrap_or_else(|| "sem-projeto".to_string())
}

/// Offer scratch recovery before automatically opening the last project. Keep
/// recovered tabs when a project is subsequently chosen (see `after_unsaved`).
pub fn restaurar_sem_projeto(ui: &UiRc) -> bool {
    if !matches!(ui.borrow().files, Files::None) {
        return false;
    }
    ui.borrow_mut().sessao_pronta = true;
    oferecer_copias(ui, "sem-projeto")
}

pub fn oferecer_copias_atual(ui: &UiRc) {
    // Retry failed autosaves too, not only copies left by a previous session.
    copiar_nao_salvos(ui);
    let key = chave_das_copias(&ui.borrow());
    let store = RecoveryStore(pasta_das_copias(&key));
    let unprotected = all_docs(ui).iter().any(|doc| {
        doc.is_dirty() && !copia_atual(&store, doc, resumo(&doc.text()), &nome_de_backup(doc))
    });
    if unprotected {
        mostrar_falhas(ui, store, vec![
            "Não foi possível guardar todas as alterações abertas. Tente novamente ou exporte o texto para outra pasta.".to_string(),
        ]);
        return;
    }
    let owned = copias_abertas(ui);
    let keys = match projetos_com_copias(&pasta_base().join("recuperacao"), &owned) {
        Ok(keys) => keys,
        Err(e) => {
            mostrar_falhas(ui, store, vec![e]);
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
        let window = ui.borrow().window.clone();
        let ui = Rc::clone(ui);
        crate::escolher::mostrar(
            &window,
            "Recuperar alterações",
            "projeto de origem",
            items,
            move |i| {
                oferecer_copias(&ui, &keys[i]);
            },
        );
    }
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
fn oferecer_copias(ui: &UiRc, key: &str) -> bool {
    use resposta_recuperacao as resposta;

    if ui.borrow().recuperacao_aberta {
        return true;
    }
    let store = RecoveryStore(pasta_das_copias(key));
    let mut scan = store.scan();
    let owned = copias_abertas(ui);
    scan.copies
        .retain(|copy| !copy.backup.paths.iter().any(|path| owned.contains(path)));
    if scan.copies.is_empty() && scan.errors.is_empty() {
        return false;
    }
    let names: Vec<&str> = scan
        .copies
        .iter()
        .map(|copy| sem_prefixo(&copy.backup.name))
        .collect();
    let detail = format!(
        "Origem: {key}\n{}\n\nRecuperar abre as alterações sem salvá-las. Cancelar mantém as cópias para depois.{}",
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
        .message("Há alterações que não foram salvas")
        .detail(detail)
        .buttons([
            "Cancelar",
            "Descartar",
            "Recuperar",
            "Exportar…",
            "Tentar novamente",
        ])
        .cancel_button(resposta::CANCELAR)
        .default_button(if scan.copies.is_empty() {
            resposta::TENTAR_NOVAMENTE
        } else {
            resposta::RECUPERAR
        })
        .build();
    let window = ui.borrow().window.clone();
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
                for copy in &scan.copies {
                    if let Err(e) = copy.backup.remove() {
                        errors.push(e);
                    }
                }
                set_status(
                    &ui,
                    "as cópias selecionadas foram descartadas; falhas foram preservadas",
                );
            }
            Ok(resposta::RECUPERAR) => {
                let mut restored = 0;
                for copy in &scan.copies {
                    let result = if detached {
                        // Never read/write a similarly named file in the current
                        // project when recovering a different (possibly offline) one.
                        let files = Files::None;
                        crate::documentos::Documents::new(&files, "", &[])
                            .recover(copy, |path, text| Ok(open_doc(&ui, path, text)))
                            .map(|mut recovered| {
                                recovered.warning =
                                    Some(format!("o projeto de origem não está aberto: {key}"));
                                recovered
                            })
                    } else {
                        with_documents(&ui, |documents| {
                            documents.recover(copy, |path, text| Ok(open_doc(&ui, path, text)))
                        })
                    };
                    match result {
                        Ok(recovered) => {
                            recovered.doc.resumo_da_copia.set(Some(resumo(&copy.text)));
                            switch_to(&ui, &recovered.doc);
                            if let Some(warning) = recovered.warning {
                                ui.borrow().console.stderr(&format!(
                                    "aviso: `{}` recuperado em uma nova aba: {warning}\n",
                                    sem_prefixo(&copy.backup.name)
                                ));
                            }
                            restored += 1;
                        }
                        Err(e) => errors.push(format!("{}: {e}", copy.backup.name)),
                    }
                }
                set_status(
                    &ui,
                    &format!("{restored} arquivo(s) recuperado(s): confira e salve"),
                );
            }
            Ok(resposta::EXPORTAR) => {
                exportar_copias(&ui, store);
                return;
            }
            Ok(resposta::TENTAR_NOVAMENTE) => {
                oferecer_copias(&ui, &key);
                return;
            }
            _ => {
                set_status(
                    &ui,
                    "cópias preservadas; use Arquivo → Recuperar alterações… para tentar novamente",
                );
                return;
            }
        }
        if !errors.is_empty() {
            mostrar_falhas(&ui, store, errors);
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
    let window = ui.borrow().window.clone();
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
    let window = ui.borrow().window.clone();
    let ui = Rc::clone(ui);
    dialog.select_folder(Some(&window), None::<&gio::Cancellable>, move |result| {
        let Some(folder) = result.ok().and_then(|file| file.path()) else {
            return;
        };
        let mut scan = store.scan();
        // Export remains available when autosave itself failed: include the
        // current buffers instead of relying solely on readable disk copies.
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
                scan.copies.retain(|copy| copy.backup.id != backup.id);
                scan.copies.push(recuperacao::Copy {
                    backup,
                    text: doc.text(),
                });
            }
        }
        let mut errors = if scan.unreadable.is_empty() {
            scan.errors
        } else {
            Vec::new()
        };
        errors.extend(recuperacao::export(&scan.copies, &folder));
        errors.extend(recuperacao::export_unreadable(&scan.unreadable, &folder));
        if errors.is_empty() {
            set_status(
                &ui,
                "cópias exportadas; as cópias de recuperação foram preservadas",
            );
        } else {
            mostrar_falhas(&ui, store, errors);
        }
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
    let key = {
        let Ok(u) = ui.try_borrow() else { return };
        chave_das_copias(&u)
    };
    let store = RecoveryStore(pasta_das_copias(&key));
    for doc in all_docs(ui) {
        if let Err(e) = copiar_documento(&store, &doc) {
            set_status(
                ui,
                &format!(
                    "não foi possível guardar as alterações de `{}`; tente Arquivo → Recuperar alterações…",
                    doc.name()
                ),
            );
            ui.borrow()
                .console
                .stderr(&format!("erro na cópia de recuperação: {e}\n"));
        }
    }
    // Never sweep unowned files: cancelled, unreadable and failed recoveries
    // must survive autosave, tab changes and a clean application shutdown.
}

/// Normal close only acknowledges the documents the student actually closed.
/// Copies that were cancelled or could not be recovered remain on disk.
pub fn limpar_copias(ui: &UiRc) {
    for doc in all_docs(ui) {
        if let Err(e) = crate::documentos::forget_recovery(&doc) {
            ui.borrow()
                .console
                .stderr(&format!("aviso: cópia de recuperação preservada: {e}\n"));
        }
    }
}

pub fn iniciar_copias(ui: &UiRc) {
    let ui = Rc::downgrade(ui);
    glib::timeout_add_seconds_local(INTERVALO_DAS_COPIAS, move || match ui.upgrade() {
        Some(ui) => {
            copiar_nao_salvos(&ui);
            glib::ControlFlow::Continue
        }
        None => glib::ControlFlow::Break,
    });
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
    let caminho = raiz.join(doc.name());
    let Ok(monitor) = gio::File::for_path(&caminho)
        .monitor_file(gio::FileMonitorFlags::NONE, gio::Cancellable::NONE)
    else {
        return;
    };
    let ui_w = Rc::downgrade(ui);
    let doc_w = Rc::downgrade(doc);
    monitor.connect_changed(move |_, _, _, evento| {
        if !matches!(
            evento,
            gio::FileMonitorEvent::ChangesDoneHint | gio::FileMonitorEvent::Created
        ) {
            return;
        }
        if let (Some(ui), Some(doc)) = (ui_w.upgrade(), doc_w.upgrade()) {
            mudou_fora(&ui, &doc, &caminho);
        }
    });
    doc.vigia.replace(Some(monitor));
}

fn mudou_fora(ui: &UiRc, doc: &Rc<Doc>, caminho: &Path) {
    let Ok(texto) = std::fs::read_to_string(caminho) else {
        return;
    };
    // Our own save, or a run's output already reloaded.
    if texto == doc.text() {
        return;
    }
    let nome = doc.name();
    if doc.is_dirty() {
        set_status(
            ui,
            &format!("`{nome}` mudou fora do editor; salvar a aba substitui essa mudança"),
        );
    } else {
        doc.recarregar(&texto);
        set_status(
            ui,
            &format!("`{nome}` mudou fora do editor e foi recarregado"),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
