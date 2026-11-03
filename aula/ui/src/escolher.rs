//! A list to pick from by typing part of a name: quick open (Ctrl+P), the
//! functions of the file (Ctrl+Shift+O), recent projects. The letters typed
//! must appear in order (`bsk` finds `bhaskara.lep`); accents and case do
//! not matter.

use std::cell::RefCell;
use std::rc::Rc;

use gtk::prelude::*;
use gtk::{Box as GtkBox, Entry, Label, ListBox, Orientation, ScrolledWindow};

/// Most rows shown at once; typing narrows the list.
const MAXIMO_DE_LINHAS: usize = 200;

pub struct Item {
    pub rotulo: String,
    /// Shown dimmed after the label (folder, line, server…).
    pub detalhe: String,
}

fn sem_acento(c: char) -> char {
    match c {
        'á' | 'à' | 'â' | 'ã' | 'ä' => 'a',
        'é' | 'è' | 'ê' | 'ë' => 'e',
        'í' | 'ì' | 'î' | 'ï' => 'i',
        'ó' | 'ò' | 'ô' | 'õ' | 'ö' => 'o',
        'ú' | 'ù' | 'û' | 'ü' => 'u',
        'ç' => 'c',
        c => c,
    }
}

fn normal(s: &str) -> Vec<char> {
    s.chars()
        .flat_map(char::to_lowercase)
        .map(sem_acento)
        .collect()
}

/// How well `alvo` matches `consulta`, higher is better; `None` when the
/// letters of `consulta` do not all appear in `alvo` in order. Letters
/// next to each other, at the start of a word, or in the last part of a
/// path (the file name) count more.
pub fn pontuar(consulta: &str, alvo: &str) -> Option<i64> {
    let q = normal(consulta);
    let a = normal(alvo);
    if q.is_empty() {
        return Some(0);
    }
    let nome_desde = a.iter().rposition(|&c| c == '/').map_or(0, |i| i + 1);
    let mut pontos = 0i64;
    let mut i = 0;
    let mut anterior: Option<usize> = None;
    for &c in &q {
        while i < a.len() && a[i] != c {
            i += 1;
        }
        if i == a.len() {
            return None;
        }
        pontos += 1;
        if anterior == Some(i.wrapping_sub(1)) {
            pontos += 5;
        }
        if i == 0 || !a[i - 1].is_alphanumeric() {
            pontos += 3;
        }
        if i >= nome_desde {
            pontos += 2;
        }
        anterior = Some(i);
        i += 1;
    }
    // Shorter names win a tie: `soma.lep` before `somatorio.lep`.
    Some(pontos * 1000 - a.len() as i64)
}

/// Indexes of the `itens` that match, best first (ties keep their order).
pub fn filtrar(consulta: &str, itens: &[Item]) -> Vec<usize> {
    let mut com_pontos: Vec<(i64, usize)> = itens
        .iter()
        .enumerate()
        .filter_map(|(i, item)| {
            let alvo = if item.detalhe.is_empty() {
                item.rotulo.clone()
            } else {
                format!("{}/{}", item.detalhe, item.rotulo)
            };
            pontuar(consulta, &alvo).map(|p| (p, i))
        })
        .collect();
    com_pontos.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    com_pontos.into_iter().map(|(_, i)| i).collect()
}

/// Opens the chooser over `parent`. `escolhido` gets the index (in `itens`)
/// of the row picked with Enter or a click; Esc or closing picks nothing.
pub fn mostrar(
    parent: &impl IsA<gtk::Window>,
    titulo: &str,
    dica: &str,
    itens: Vec<Item>,
    escolhido: impl FnOnce(usize) + 'static,
) {
    let window = gtk::Window::builder()
        .transient_for(parent)
        .modal(true)
        .title(titulo)
        .default_width(560)
        .default_height(420)
        .build();
    let v = GtkBox::new(Orientation::Vertical, 6);
    v.set_margin_start(10);
    v.set_margin_end(10);
    v.set_margin_top(10);
    v.set_margin_bottom(10);
    let entrada = Entry::builder()
        .placeholder_text(dica)
        .hexpand(true)
        .build();
    let lista = ListBox::new();
    lista.set_selection_mode(gtk::SelectionMode::Browse);
    let rolagem = ScrolledWindow::builder()
        .child(&lista)
        .vexpand(true)
        .build();
    v.append(&entrada);
    v.append(&rolagem);
    window.set_child(Some(&v));

    let itens = Rc::new(itens);
    // Row i of `lista` shows `itens[mostrados[i]]`.
    let mostrados: Rc<RefCell<Vec<usize>>> = Rc::new(RefCell::new(Vec::new()));
    let preencher = {
        let lista = lista.clone();
        let itens = Rc::clone(&itens);
        let mostrados = Rc::clone(&mostrados);
        move |consulta: &str| {
            while let Some(row) = lista.first_child() {
                lista.remove(&row);
            }
            let mut achados = filtrar(consulta, &itens);
            achados.truncate(MAXIMO_DE_LINHAS);
            for &i in &achados {
                let linha = GtkBox::new(Orientation::Horizontal, 10);
                let rotulo = Label::new(Some(&itens[i].rotulo));
                rotulo.set_xalign(0.0);
                linha.append(&rotulo);
                if !itens[i].detalhe.is_empty() {
                    let detalhe = Label::new(Some(&itens[i].detalhe));
                    detalhe.add_css_class("dim-label");
                    detalhe.set_xalign(0.0);
                    detalhe.set_ellipsize(gtk::pango::EllipsizeMode::Start);
                    linha.append(&detalhe);
                }
                linha.set_margin_top(3);
                linha.set_margin_bottom(3);
                lista.append(&linha);
            }
            lista.select_row(lista.row_at_index(0).as_ref());
            *mostrados.borrow_mut() = achados;
        }
    };
    preencher("");

    let escolhido: RefCell<Option<Box<dyn FnOnce(usize)>>> =
        RefCell::new(Some(Box::new(escolhido)));
    let escolher = Rc::new({
        let lista = lista.clone();
        let window = window.clone();
        move || {
            let Some(row) = lista.selected_row() else {
                return;
            };
            let Some(&i) = mostrados.borrow().get(row.index() as usize) else {
                return;
            };
            window.close();
            if let Some(f) = escolhido.borrow_mut().take() {
                f(i);
            }
        }
    });
    entrada.connect_changed(move |e| preencher(&e.text()));
    {
        let escolher = Rc::clone(&escolher);
        entrada.connect_activate(move |_| escolher());
    }
    lista.connect_row_activated(move |_, _| escolher());

    // Arrows move in the list while typing stays in the entry; Esc closes.
    let teclas = gtk::ShortcutController::new();
    teclas.set_propagation_phase(gtk::PropagationPhase::Capture);
    let mover = |lista: &ListBox, passo: i32| {
        let atual = lista.selected_row().map_or(-1, |r| r.index());
        if let Some(row) = lista.row_at_index(atual + passo) {
            lista.select_row(Some(&row));
            row.grab_focus();
        }
    };
    for (tecla, passo) in [("Down", 1), ("Up", -1), ("Page_Down", 10), ("Page_Up", -10)] {
        let lista = lista.clone();
        let entrada = entrada.clone();
        atalho(&teclas, tecla, move || {
            mover(&lista, passo);
            entrada.grab_focus();
            entrada.set_position(-1);
        });
    }
    {
        let window = window.clone();
        atalho(&teclas, "Escape", move || window.close());
    }
    window.add_controller(teclas);
    window.present();
    entrada.grab_focus();
}

fn atalho(c: &gtk::ShortcutController, tecla: &str, f: impl Fn() + 'static) {
    if let Some(trigger) = gtk::ShortcutTrigger::parse_string(tecla) {
        c.add_shortcut(gtk::Shortcut::new(
            Some(trigger),
            Some(gtk::CallbackAction::new(move |_, _| {
                f();
                gtk::glib::Propagation::Stop
            })),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn itens(nomes: &[&str]) -> Vec<Item> {
        nomes
            .iter()
            .map(|n| Item {
                rotulo: n.to_string(),
                detalhe: String::new(),
            })
            .collect()
    }

    fn nomes<'a>(lista: &'a [Item], idx: &[usize]) -> Vec<&'a str> {
        idx.iter().map(|&i| lista[i].rotulo.as_str()).collect()
    }

    #[test]
    fn letras_em_ordem_sem_acento_e_maiusculas() {
        assert!(pontuar("bsk", "escola/bhaskara.lep").is_some());
        assert!(pontuar("BHASK", "escola/bhaskara.lep").is_some());
        assert!(pontuar("funcao", "função.lep").is_some());
        assert!(pontuar("ksb", "escola/bhaskara.lep").is_none());
        assert_eq!(pontuar("", "qualquer"), Some(0));
    }

    #[test]
    fn melhor_primeiro() {
        let l = itens(&["lib/somatorio.lep", "soma.lep", "jogos/forca.lep"]);
        assert_eq!(
            nomes(&l, &filtrar("soma", &l)),
            ["soma.lep", "lib/somatorio.lep"]
        );
        // The file name counts more than the folder.
        let l = itens(&["forca/a.lep", "jogos/forca.lep"]);
        assert_eq!(nomes(&l, &filtrar("forca", &l))[0], "jogos/forca.lep");
        // Empty query: everything, in the given order.
        assert_eq!(filtrar("", &l), [0, 1]);
    }
}
