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
use crate::store::Files;
use crate::{
    Ui, UiRc, all_docs, current_doc, open_doc, open_path, remember_arquivo, set_status, switch_to,
};

/// How often unsaved text is copied aside.
const INTERVALO_DAS_COPIAS: u32 = 5;

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

/// A file name for any tab name: letters and digits stay, the rest becomes
/// `%XX`. Reversible, so a copy says which tab it belongs to.
pub fn nome_de_arquivo(nome: &str) -> String {
    let mut out = String::new();
    for b in nome.bytes() {
        if b.is_ascii_alphanumeric() || b == b'-' || b == b'_' {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

pub fn nome_de_volta(arquivo: &str) -> Option<String> {
    let mut bytes = Vec::new();
    let mut it = arquivo.bytes();
    while let Some(b) = it.next() {
        if b == b'%' {
            let hex = [it.next()?, it.next()?];
            bytes.push(u8::from_str_radix(std::str::from_utf8(&hex).ok()?, 16).ok()?);
        } else {
            bytes.push(b);
        }
    }
    String::from_utf8(bytes).ok()
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
    let Some(chave) = chave(&ui.borrow()) else {
        ui.borrow_mut().sessao_pronta = true;
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
    oferecer_copias(ui, &chave);
}

/// Copies left by a session that did not close normally: ask, then put
/// them back in their tabs (unsaved) or drop them.
fn oferecer_copias(ui: &UiRc, chave: &str) {
    let pasta = pasta_das_copias(chave);
    let copias: Vec<(String, String)> = std::fs::read_dir(&pasta)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let nome = nome_de_volta(&e.file_name().to_string_lossy())?;
            let texto = std::fs::read_to_string(e.path()).ok()?;
            Some((nome, texto))
        })
        .collect();
    if copias.is_empty() {
        ui.borrow_mut().sessao_pronta = true;
        return;
    }
    let nomes: Vec<&str> = copias.iter().map(|(n, _)| sem_prefixo(n)).collect();
    let dialogo = gtk::AlertDialog::builder()
        .modal(true)
        .message("Há alterações que não foram salvas")
        .detail(format!(
            "O editor fechou sem salvar: {}.\nRecuperar essas alterações?",
            nomes.join(", ")
        ))
        .buttons(["Descartar", "Recuperar"])
        .cancel_button(0)
        .default_button(1)
        .build();
    let window = ui.borrow().window.clone();
    let ui = Rc::clone(ui);
    dialogo.choose(Some(&window), None::<&gio::Cancellable>, move |r| {
        if r == Ok(1) {
            for (nome, texto) in &copias {
                recuperar(&ui, nome, texto);
            }
        }
        let _ = std::fs::remove_dir_all(&pasta);
        ui.borrow_mut().sessao_pronta = true;
        // Copy what is unsaved again right away (the folder was emptied).
        copiar_nao_salvos(&ui);
    });
}

/// Copies of tabs never saved are named `sem-titulo:<tab name>`.
const PREFIXO_SEM_TITULO: &str = "sem-titulo:";

fn sem_prefixo(nome: &str) -> &str {
    nome.strip_prefix(PREFIXO_SEM_TITULO).unwrap_or(nome)
}

fn recuperar(ui: &UiRc, nome: &str, texto: &str) {
    let doc = if nome.starts_with(PREFIXO_SEM_TITULO) {
        Some(open_doc(ui, None, texto))
    } else {
        open_path(ui, nome)
    };
    let Some(doc) = doc else { return };
    if doc.text() != texto {
        let linha = doc.cursor().line();
        doc.buffer.begin_user_action();
        doc.buffer.set_text(texto);
        doc.buffer.end_user_action();
        if let Some(it) = doc.buffer.iter_at_line(linha) {
            doc.buffer.place_cursor(&it);
        }
    }
    set_status(ui, "alterações recuperadas: confira e salve");
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

/// Every few seconds: copy the text of unsaved tabs aside, and drop copies
/// of tabs that are saved or closed now.
pub fn copiar_nao_salvos(ui: &UiRc) {
    let chave = {
        let Ok(u) = ui.try_borrow() else { return };
        if !u.sessao_pronta {
            return;
        }
        match chave(&u) {
            Some(c) => c,
            None => return,
        }
    };
    let pasta = pasta_das_copias(&chave);
    let mut vivas = std::collections::HashSet::new();
    for d in all_docs(ui).iter().filter(|d| d.is_dirty()) {
        let nome = nome_de_arquivo(&nome_da_copia(d));
        let texto = d.text();
        let h = resumo(&texto);
        if d.resumo_da_copia.get() != Some(h) {
            let _ = std::fs::create_dir_all(&pasta);
            if std::fs::write(pasta.join(&nome), &texto).is_ok() {
                d.resumo_da_copia.set(Some(h));
            }
        }
        vivas.insert(nome);
    }
    for d in all_docs(ui).iter().filter(|d| !d.is_dirty()) {
        d.resumo_da_copia.set(None);
    }
    for e in std::fs::read_dir(&pasta).into_iter().flatten().flatten() {
        if !vivas.contains(&*e.file_name().to_string_lossy()) {
            let _ = std::fs::remove_file(e.path());
        }
    }
}

/// The editor closes normally (unsaved tabs were saved or discarded): no
/// copy is needed any more.
pub fn limpar_copias(ui: &UiRc) {
    let Ok(u) = ui.try_borrow() else { return };
    if let Some(chave) = chave(&u) {
        let _ = std::fs::remove_dir_all(pasta_das_copias(&chave));
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
}
