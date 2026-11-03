//! Window listing the keyboard shortcuts (Ctrl+? or Ajuda → Atalhos).
//! Keep the table in step with the shortcuts set up in `main.rs`.

use gtk::prelude::*;
use gtk::{Box as GtkBox, Label, Orientation, ScrolledWindow};

/// Sections of (keys, what they do).
const ATALHOS: &[(&str, &[(&str, &str)])] = &[
    (
        "Arquivos e abas",
        &[
            ("Ctrl+N", "arquivo novo"),
            ("Ctrl+S", "salvar"),
            ("Ctrl+W", "fechar a aba"),
            ("Ctrl+Shift+T", "reabrir a última aba fechada"),
            ("Ctrl+Tab · Ctrl+PgDn", "próxima aba"),
            ("Ctrl+Shift+Tab · Ctrl+PgUp", "aba anterior"),
            ("Alt+1 … Alt+8", "ir para a aba 1 … 8"),
            ("Alt+9", "ir para a última aba"),
            ("clique do meio na aba", "fechar a aba"),
            ("clique direito na aba", "fechar outras, mostrar na árvore…"),
            ("Ctrl+Q", "sair"),
        ],
    ),
    (
        "Editar",
        &[
            ("Ctrl+/", "comentar ou descomentar as linhas"),
            ("Ctrl+D", "duplicar as linhas"),
            ("Alt+↑ · Alt+↓", "mover as linhas"),
            ("Tab · Shift+Tab", "aumentar ou diminuir o recuo"),
            ("Ctrl+Alt+\\", "arrumar o recuo"),
            ("Ctrl+Z · Ctrl+Shift+Z", "desfazer · refazer"),
        ],
    ),
    (
        "Buscar e navegar",
        &[
            ("Ctrl+F · Ctrl+H", "buscar e trocar no arquivo"),
            ("Ctrl+G", "ir para a linha"),
            ("Ctrl+Shift+F", "buscar (e trocar) em todos os arquivos"),
            ("Ctrl+P", "abrir um arquivo do projeto pelo nome"),
            ("Ctrl+Shift+O", "ir para uma função do arquivo"),
            ("F12 · Ctrl+clique", "ir para a definição"),
            ("F1", "ajuda sobre o nome sob o cursor"),
        ],
    ),
    (
        "Executar e depurar",
        &[
            ("F5", "executar"),
            ("F6", "depurar"),
            ("F9", "ponto de parada"),
            ("F10", "próxima linha"),
            ("F11 · Shift+F11", "entrar na função · sair dela"),
        ],
    ),
    (
        "Ver",
        &[
            ("F7", "mostrar ou esconder os arquivos"),
            ("F8", "mostrar ou esconder o painel"),
            ("Ctrl+= · Ctrl+- · Ctrl+0", "letra maior · menor · normal"),
            ("Ctrl+?", "esta lista"),
        ],
    ),
];

pub fn mostrar(parent: &impl IsA<gtk::Window>) {
    let window = gtk::Window::builder()
        .transient_for(parent)
        .title("Atalhos do teclado")
        .default_width(520)
        .default_height(560)
        .build();
    let v = GtkBox::new(Orientation::Vertical, 4);
    v.set_margin_start(16);
    v.set_margin_end(16);
    v.set_margin_top(12);
    v.set_margin_bottom(16);
    for (secao, itens) in ATALHOS {
        let titulo = Label::new(None);
        titulo.set_markup(&format!("<b>{secao}</b>"));
        titulo.set_xalign(0.0);
        titulo.set_margin_top(10);
        v.append(&titulo);
        let grade = gtk::Grid::new();
        grade.set_column_spacing(24);
        grade.set_row_spacing(2);
        for (i, (teclas, o_que)) in itens.iter().enumerate() {
            let t = Label::new(Some(teclas));
            t.set_xalign(0.0);
            t.add_css_class("monospace");
            let o = Label::new(Some(o_que));
            o.set_xalign(0.0);
            grade.attach(&t, 0, i as i32, 1, 1);
            grade.attach(&o, 1, i as i32, 1, 1);
        }
        v.append(&grade);
    }
    let rolagem = ScrolledWindow::builder().child(&v).vexpand(true).build();
    window.set_child(Some(&rolagem));
    let fechar = gtk::ShortcutController::new();
    if let Some(trigger) = gtk::ShortcutTrigger::parse_string("Escape") {
        let w = window.clone();
        fechar.add_shortcut(gtk::Shortcut::new(
            Some(trigger),
            Some(gtk::CallbackAction::new(move |_, _| {
                w.close();
                gtk::glib::Propagation::Stop
            })),
        ));
    }
    window.add_controller(fechar);
    window.present();
}
