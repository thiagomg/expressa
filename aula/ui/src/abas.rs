//! Tabs: moving between them from the keyboard, the right-click menu on a
//! tab, middle-click to close, and reopening tabs closed by mistake.

use std::rc::Rc;

use gtk::prelude::*;

use crate::docs::Doc;
use crate::{
    UiRc, all_docs, close_doc, confirm_unsaved, mostrar_menu, open_path, remove_tab,
    revelar_na_arvore, set_status, switch_to,
};

/// How many closed tabs Ctrl+Shift+T can bring back.
const FECHADAS_MAXIMO: usize = 20;

/// Ctrl+Tab / Ctrl+PgDn (`passo` 1) and Ctrl+Shift+Tab / Ctrl+PgUp (-1), in
/// tab order, wrapping around.
pub fn vizinha(ui: &UiRc, passo: i32) {
    let nb = ui.borrow().notebook.clone();
    let n = nb.n_pages() as i32;
    if n < 2 {
        return;
    }
    let atual = nb.current_page().map_or(0, |p| p as i32);
    let alvo = (atual + passo).rem_euclid(n);
    mostrar_pagina(ui, alvo as u32);
}

/// Alt+1…Alt+8: that tab. Alt+9: the last one, however many there are.
pub fn numero(ui: &UiRc, n: u32) {
    let nb = ui.borrow().notebook.clone();
    let total = nb.n_pages();
    if total == 0 {
        return;
    }
    let alvo = if n == 9 { total - 1 } else { n - 1 };
    if alvo < total {
        mostrar_pagina(ui, alvo);
    }
}

fn mostrar_pagina(ui: &UiRc, pagina: u32) {
    let nb = ui.borrow().notebook.clone();
    let Some(widget) = nb.nth_page(Some(pagina)) else {
        return;
    };
    let doc = all_docs(ui)
        .into_iter()
        .find(|d| d.page.upcast_ref::<gtk::Widget>() == &widget);
    match doc {
        Some(d) => switch_to(ui, &d),
        None => nb.set_current_page(Some(pagina)),
    }
}

/// Mouse on `doc`'s tab: middle button closes it, right button opens the
/// tab menu.
pub fn ligar(ui: &UiRc, doc: &Rc<Doc>) {
    let clique = gtk::GestureClick::new();
    clique.set_button(0);
    let ui = Rc::downgrade(ui);
    let weak = Rc::downgrade(doc);
    clique.connect_pressed(move |g, _, x, y| {
        let (Some(ui), Some(doc)) = (ui.upgrade(), weak.upgrade()) else {
            return;
        };
        match g.current_button() {
            2 => {
                g.set_state(gtk::EventSequenceState::Claimed);
                fechar(&ui, &doc);
            }
            3 => {
                g.set_state(gtk::EventSequenceState::Claimed);
                menu(&ui, &doc, x, y);
            }
            _ => {}
        }
    });
    doc.tab.add_controller(clique);
}

fn menu(ui: &UiRc, doc: &Rc<Doc>, x: f64, y: f64) {
    let mut itens: Vec<(&str, Box<dyn FnOnce()>)> = Vec::new();
    let mais = |f: fn(&UiRc, &Rc<Doc>)| -> Box<dyn FnOnce()> {
        let ui = Rc::clone(ui);
        let doc = Rc::clone(doc);
        Box::new(move || f(&ui, &doc))
    };
    itens.push(("Fechar", mais(fechar)));
    itens.push(("Fechar as outras", mais(fechar_outras)));
    itens.push(("Fechar as da direita", mais(fechar_a_direita)));
    if !doc.is_untitled() {
        itens.push((
            "Mostrar na árvore",
            mais(|ui, d| revelar_na_arvore(ui, &d.name())),
        ));
        itens.push(("Copiar caminho", mais(copiar_caminho)));
    }
    mostrar_menu(ui, doc.tab.upcast_ref(), x, y, itens);
}

/// Close one tab (asks about unsaved changes); Ctrl+Shift+T can reopen it.
pub fn fechar(ui: &UiRc, doc: &Rc<Doc>) {
    close_doc(ui, doc);
}

fn fechar_outras(ui: &UiRc, doc: &Rc<Doc>) {
    let outras = all_docs(ui)
        .into_iter()
        .filter(|d| !Rc::ptr_eq(d, doc))
        .collect();
    fechar_varias(ui, outras);
}

fn fechar_a_direita(ui: &UiRc, doc: &Rc<Doc>) {
    let nb = ui.borrow().notebook.clone();
    let Some(aqui) = nb.page_num(&doc.page) else {
        return;
    };
    let direita = all_docs(ui)
        .into_iter()
        .filter(|d| nb.page_num(&d.page).is_some_and(|p| p > aqui))
        .collect();
    fechar_varias(ui, direita);
}

/// Close `docs`, with one question for all the unsaved ones among them.
fn fechar_varias(ui: &UiRc, docs: Vec<Rc<Doc>>) {
    let sujas: Vec<Rc<Doc>> = docs.iter().filter(|d| d.is_dirty()).cloned().collect();
    let fechar_todas = {
        let ui = Rc::clone(ui);
        move || {
            for d in &docs {
                lembrar_fechada(&ui, d);
                remove_tab(&ui, d);
            }
        }
    };
    if sujas.is_empty() {
        fechar_todas();
    } else {
        confirm_unsaved(ui, sujas, fechar_todas);
    }
}

/// `doc` is being closed by the user: keep where it was, for Ctrl+Shift+T.
/// Tabs never saved have no file to reopen.
pub fn lembrar_fechada(ui: &UiRc, doc: &Rc<Doc>) {
    if doc.is_untitled() {
        return;
    }
    let linha = doc.cursor().line() as u32 + 1;
    let mut u = ui.borrow_mut();
    u.abas_fechadas.retain(|(n, _)| *n != doc.name());
    u.abas_fechadas.push((doc.name(), linha));
    if u.abas_fechadas.len() > FECHADAS_MAXIMO {
        u.abas_fechadas.remove(0);
    }
}

/// Ctrl+Shift+T: reopen the last closed tab, on the line it was at.
pub fn reabrir(ui: &UiRc) {
    loop {
        let Some((nome, linha)) = ui.borrow_mut().abas_fechadas.pop() else {
            set_status(ui, "nenhuma aba fechada para reabrir");
            return;
        };
        // Deleted or renamed since: try the one before.
        if let Some(doc) = open_path(ui, &nome) {
            doc.goto_line(linha);
            return;
        }
    }
}

fn copiar_caminho(ui: &UiRc, doc: &Rc<Doc>) {
    let caminho = {
        let u = ui.borrow();
        match &u.files {
            crate::store::Files::Local(raiz) => raiz.join(doc.name()).display().to_string(),
            _ => doc.name(),
        }
    };
    let window = ui.borrow().window.clone();
    window.clipboard().set_text(&caminho);
    set_status(ui, &format!("copiado: {caminho}"));
}

#[cfg(test)]
mod tests {
    /// `add_shortcut` drops a string GTK cannot parse without a word: a
    /// typo here would leave the shortcut dead.
    #[test]
    fn atalhos_das_abas_sao_validos() {
        crate::gtk_test::run(|| {
            let mut teclas = vec![
                "<Primary>Tab".to_string(),
                "<Primary>Page_Down".into(),
                "<Primary><Shift>Tab".into(),
                "<Primary><Shift>ISO_Left_Tab".into(),
                "<Primary>Page_Up".into(),
                "<Primary><Shift>t".into(),
                "<Primary>question".into(),
            ];
            teclas.extend((1..=9).map(|n| format!("<Alt>{n}")));
            for t in teclas {
                assert!(gtk::ShortcutTrigger::parse_string(&t).is_some(), "{t}");
            }
        });
    }
}
