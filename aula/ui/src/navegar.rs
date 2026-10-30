//! Jumping by name: a project file (Ctrl+P), a function of the open file
//! (Ctrl+Shift+O), a project opened before (Arquivo → Recentes).

use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;

use gtk::prelude::*;

use crate::escolher::{self, Item};
use crate::prefs::Recente;
use crate::rpc::Rpc;
use crate::{
    UiRc, after_unsaved, apply_remote, current_doc, open_local_project, open_path, set_status,
};

/// Splits `lib/mat.lep` into (`mat.lep`, `lib`).
fn nome_e_pasta(caminho: &str) -> (String, String) {
    match caminho.rsplit_once('/') {
        Some((pasta, nome)) => (nome.to_string(), pasta.to_string()),
        None => (caminho.to_string(), String::new()),
    }
}

/// Ctrl+P: open any file of the project by typing part of its name.
pub fn abrir_rapido(ui: &UiRc) {
    let (window, arquivos) = {
        let u = ui.borrow();
        (u.window.clone(), u.project_files.clone())
    };
    if arquivos.is_empty() {
        set_status(ui, "o projeto não tem arquivos (ou ainda está carregando)");
        return;
    }
    let itens = arquivos
        .iter()
        .map(|c| {
            let (rotulo, detalhe) = nome_e_pasta(c);
            Item { rotulo, detalhe }
        })
        .collect();
    let ui = Rc::clone(ui);
    escolher::mostrar(
        &window,
        "Abrir arquivo",
        "parte do nome do arquivo",
        itens,
        move |i| {
            open_path(&ui, &arquivos[i]);
        },
    );
}

/// Ctrl+Shift+O: go to a function (or name) defined in the open file.
pub fn funcoes_do_arquivo(ui: &UiRc) {
    let Some(doc) = current_doc(ui) else {
        set_status(ui, "abra um arquivo primeiro");
        return;
    };
    let defs = crate::lang::parse_definitions(&doc.text());
    if defs.is_empty() {
        set_status(ui, "este arquivo não define nomes");
        return;
    }
    // Functions first: they are what one looks for in a file.
    let mut defs = defs;
    defs.sort_by_key(|d| !matches!(d.kind, crate::lang::DefKind::Function(_)));
    let itens = defs
        .iter()
        .map(|d| Item {
            rotulo: d.signature(),
            detalhe: format!("linha {}", d.line + 1),
        })
        .collect();
    let window = ui.borrow().window.clone();
    escolher::mostrar(
        &window,
        "Ir para a definição",
        "parte do nome",
        itens,
        move |i| {
            doc.goto_line(defs[i].line as u32 + 1);
            doc.view.grab_focus();
        },
    );
}

/// Arquivo → Recentes: reopen a folder or a server project used before.
pub fn recentes(ui: &UiRc) {
    let (window, recentes) = {
        let u = ui.borrow();
        (u.window.clone(), u.prefs.recentes.clone())
    };
    if recentes.is_empty() {
        set_status(ui, "nenhum projeto aberto antes");
        return;
    }
    let itens = recentes
        .iter()
        .map(|r| match r {
            Recente::Local(p) => {
                let (nome, pasta) = nome_e_pasta(&p.replace('\\', "/"));
                Item {
                    rotulo: nome,
                    detalhe: pasta,
                }
            }
            Recente::Servidor {
                url,
                aluno,
                projeto,
            } => Item {
                rotulo: format!("{aluno} / {projeto}"),
                detalhe: url.clone(),
            },
        })
        .collect();
    let ui = Rc::clone(ui);
    escolher::mostrar(
        &window,
        "Projetos recentes",
        "parte do nome",
        itens,
        move |i| abrir_recente(&ui, recentes[i].clone()),
    );
}

fn abrir_recente(ui: &UiRc, r: Recente) {
    match r {
        Recente::Local(p) => open_local_project(ui, PathBuf::from(p)),
        Recente::Servidor {
            url,
            aluno,
            projeto,
        } => match Rpc::connect(&url) {
            Ok(rpc) => {
                let ui_c = Rc::clone(ui);
                let rpc = Arc::new(rpc);
                after_unsaved(ui, move || apply_remote(&ui_c, rpc, url, aluno, projeto));
            }
            Err(e) => set_status(ui, &format!("servidor {url}: {e}")),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::nome_e_pasta;

    #[test]
    fn separa_nome_e_pasta() {
        assert_eq!(
            nome_e_pasta("lib/mat.lep"),
            ("mat.lep".into(), "lib".into())
        );
        assert_eq!(nome_e_pasta("a.lep"), ("a.lep".into(), String::new()));
        assert_eq!(
            nome_e_pasta("escola/x/y.lep"),
            ("y.lep".into(), "escola/x".into())
        );
    }
}
