//! Window with the ficha of every builtin (núcleo + native modules).
//! Same text as `ajuda("matriz::zeros")` / `lookup_ficha`.

use std::rc::Rc;

use gtk::prelude::*;
use gtk::{
    Box as GtkBox, Entry, Label, ListBox, ListBoxRow, Orientation, Paned, PolicyType,
    ScrolledWindow, SelectionMode, Window, pango,
};

use expressa::runtime::{docs_agrupados, lookup_ficha, tema_de, titulo_modulo};

struct EntryRef {
    key: String,
    label: String,
    modulo: String,
    tema: String,
}

fn sem_marca(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'á' | 'à' | 'â' | 'ã' | 'Á' | 'À' | 'Â' | 'Ã' => 'a',
            'é' | 'ê' | 'É' | 'Ê' => 'e',
            'í' | 'Í' => 'i',
            'ó' | 'ô' | 'õ' | 'Ó' | 'Ô' | 'Õ' => 'o',
            'ú' | 'Ú' => 'u',
            'ç' | 'Ç' => 'c',
            c => c.to_ascii_lowercase(),
        })
        .collect()
}

fn item_bate(e: &EntryRef, q: &str) -> bool {
    if q.is_empty() {
        return true;
    }
    let q = sem_marca(q);
    sem_marca(&e.key).contains(&q)
        || sem_marca(&e.label).contains(&q)
        || sem_marca(&e.tema).contains(&q)
        || sem_marca(&e.modulo).contains(&q)
}

fn entries() -> Vec<EntryRef> {
    let mut out = Vec::new();
    for d in docs_agrupados() {
        let modulo = titulo_modulo(d.module.as_deref());
        let tema = tema_de(d.module.as_deref(), &d.name).to_string();
        let key = match d.module.as_deref() {
            Some(m) => format!("{}::{}", m, d.name),
            None => d.name.clone(),
        };
        let label = if d.summary.is_empty() {
            d.name.clone()
        } else {
            format!("{}    {}", d.name, d.summary)
        };
        out.push(EntryRef {
            key,
            label,
            modulo,
            tema,
        });
    }
    out
}

/// New window listing builtins; selecting a name shows its ficha.
/// Sibling of the editor (same GtkApplication), not a transient dialog:
/// the user can keep it open and type in the IDE.
pub fn open(parent: &impl IsA<gtk::Window>) -> Window {
    let win = Window::builder()
        .title("Ajuda — funções nativas")
        .default_width(860)
        .default_height(540)
        .build();
    if let Some(app) = parent.application() {
        win.set_application(Some(&app));
    }

    let search = Entry::builder()
        .placeholder_text("filtrar (texto, data, escreva, …)")
        .hexpand(true)
        .build();

    let list = ListBox::new();
    list.set_selection_mode(SelectionMode::Single);
    list.set_activate_on_single_click(true);

    let all = Rc::new(entries());
    let mut last_modulo = String::new();
    let mut last_tema = String::new();
    for e in all.iter() {
        if e.modulo != last_modulo {
            last_modulo = e.modulo.clone();
            last_tema.clear();
            let h = Label::new(Some(&e.modulo));
            h.set_xalign(0.0);
            h.add_css_class("aula-ajuda-grupo");
            let row = ListBoxRow::new();
            row.set_selectable(false);
            row.set_activatable(false);
            row.set_child(Some(&h));
            row.set_widget_name(&format!("m:{}", e.modulo));
            list.append(&row);
        }
        if !e.tema.is_empty() && e.tema != last_tema {
            last_tema = e.tema.clone();
            let h = Label::new(Some(&e.tema));
            h.set_xalign(0.0);
            h.add_css_class("aula-ajuda-tema");
            let row = ListBoxRow::new();
            row.set_selectable(false);
            row.set_activatable(false);
            row.set_child(Some(&h));
            row.set_widget_name(&format!("t:{}\t{}", e.modulo, e.tema));
            list.append(&row);
        }
        let lab = Label::new(Some(&e.label));
        lab.set_xalign(0.0);
        lab.set_ellipsize(pango::EllipsizeMode::End);
        lab.set_tooltip_text(Some(&e.label));
        let row = ListBoxRow::new();
        row.set_child(Some(&lab));
        row.set_widget_name(&e.key);
        list.append(&row);
    }

    let filter = Rc::clone(&all);
    list.set_filter_func({
        let search = search.clone();
        move |row| {
            let q = search.text().to_string();
            let name = row.widget_name();
            if let Some(modulo) = name.strip_prefix("m:") {
                return filter
                    .iter()
                    .any(|e| e.modulo == modulo && item_bate(e, &q));
            }
            if let Some(rest) = name.strip_prefix("t:")
                && let Some((modulo, tema)) = rest.split_once('\t')
            {
                return filter
                    .iter()
                    .any(|e| e.modulo == modulo && e.tema == tema && item_bate(e, &q));
            }
            filter
                .iter()
                .find(|e| e.key == name.as_str())
                .is_some_and(|e| item_bate(e, &q))
        }
    });
    search.connect_changed({
        let list = list.clone();
        move |_| {
            list.invalidate_filter();
        }
    });

    let detail = Label::new(Some("Escolha uma função à esquerda."));
    detail.set_xalign(0.0);
    detail.set_yalign(0.0);
    detail.set_selectable(true);
    detail.set_wrap(true);
    detail.set_wrap_mode(pango::WrapMode::WordChar);
    detail.add_css_class("aula-catalogo");

    list.connect_row_selected({
        let detail = detail.clone();
        move |_, row| {
            let Some(row) = row else { return };
            let key = row.widget_name();
            if key.starts_with("m:") || key.starts_with("t:") {
                return;
            }
            if let Some(text) = lookup_ficha(&key) {
                detail.set_text(&text);
            }
        }
    });

    const LEFT_WIDTH: i32 = 380;
    let left_scroll = ScrolledWindow::builder()
        .hscrollbar_policy(PolicyType::Automatic)
        .vscrollbar_policy(PolicyType::Automatic)
        .min_content_width(LEFT_WIDTH)
        .vexpand(true)
        .child(&list)
        .build();
    let right_scroll = ScrolledWindow::builder()
        .hscrollbar_policy(PolicyType::Automatic)
        .vscrollbar_policy(PolicyType::Automatic)
        .hexpand(true)
        .child(&detail)
        .build();
    right_scroll.set_margin_start(8);
    right_scroll.set_margin_end(8);
    right_scroll.set_margin_top(8);
    right_scroll.set_margin_bottom(8);

    let left = GtkBox::new(Orientation::Vertical, 6);
    left.set_margin_start(8);
    left.set_margin_top(8);
    left.set_margin_bottom(8);
    left.set_hexpand(false);
    left.set_width_request(LEFT_WIDTH);
    left.append(&search);
    left.append(&left_scroll);

    let paned = Paned::new(Orientation::Horizontal);
    paned.set_start_child(Some(&left));
    paned.set_end_child(Some(&right_scroll));
    paned.set_resize_start_child(false);
    paned.set_resize_end_child(true);
    paned.set_shrink_start_child(false);
    paned.set_position(LEFT_WIDTH);
    paned.connect_realize(move |p| {
        p.set_position(LEFT_WIDTH);
    });

    win.set_child(Some(&paned));
    win.present();
    win
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filtro_acha_agora_por_data() {
        let all = entries();
        let vis: Vec<&str> = all
            .iter()
            .filter(|e| item_bate(e, "data"))
            .map(|e| e.key.as_str())
            .collect();
        assert!(vis.contains(&"agora"), "{vis:?}");
        assert!(vis.contains(&"formate_data"), "{vis:?}");
    }

    #[test]
    fn filtro_texto_junta_funcoes() {
        let all = entries();
        let vis: Vec<&str> = all
            .iter()
            .filter(|e| item_bate(e, "texto"))
            .map(|e| e.key.as_str())
            .collect();
        assert!(vis.contains(&"maiuscula"), "{vis:?}");
        assert!(vis.contains(&"limpe"), "{vis:?}");
        assert!(vis.contains(&"formate"), "{vis:?}");
        assert!(!vis.contains(&"escreva"), "{vis:?}");
    }

    #[test]
    fn filtro_ignora_acento() {
        let all = entries();
        assert!(
            all.iter()
                .any(|e| item_bate(e, "saida") && e.tema == "Entrada e saída")
        );
        assert!(
            all.iter()
                .any(|e| item_bate(e, "nucleo") && e.modulo == "Núcleo")
        );
    }
}
