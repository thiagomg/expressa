//! Window with the ficha of every builtin (núcleo + native modules).
//! Same text as `ajuda("matriz::zeros")` / `lookup_ficha`.

use std::rc::Rc;

use gtk::prelude::*;
use gtk::{
    Box as GtkBox, Entry, Label, ListBox, ListBoxRow, Orientation, Paned, PolicyType,
    ScrolledWindow, SelectionMode, Window, pango,
};

use expressa::runtime::{builtin_docs, lookup_ficha};

struct EntryRef {
    key: String,
    label: String,
    group: String,
}

fn entries() -> Vec<EntryRef> {
    let mut out = Vec::new();
    for d in builtin_docs() {
        let group = match d.module.as_deref() {
            None => "Núcleo".to_string(),
            Some(m) => format!("importe \"{m}\""),
        };
        let key = match d.module.as_deref() {
            Some(m) => format!("{}::{}", m, d.name),
            None => d.name.clone(),
        };
        let label = if d.is_value {
            d.name.clone()
        } else {
            key.clone()
        };
        out.push(EntryRef { key, label, group });
    }
    out
}

/// New window listing builtins; selecting a name shows its ficha.
pub fn open(parent: &impl IsA<gtk::Window>) -> Window {
    let win = Window::builder()
        .transient_for(parent)
        .title("Ajuda — funções nativas")
        .default_width(860)
        .default_height(540)
        .build();

    let search = Entry::builder()
        .placeholder_text("filtrar (escreva, matriz::zeros, …)")
        .hexpand(true)
        .build();

    let list = ListBox::new();
    list.set_selection_mode(SelectionMode::Single);
    list.set_activate_on_single_click(true);

    let all = Rc::new(entries());
    let mut last_group = String::new();
    for e in all.iter() {
        if e.group != last_group {
            last_group = e.group.clone();
            let h = Label::new(Some(&e.group));
            h.set_xalign(0.0);
            h.add_css_class("aula-ajuda-grupo");
            let row = ListBoxRow::new();
            row.set_selectable(false);
            row.set_activatable(false);
            row.set_child(Some(&h));
            row.set_widget_name("grupo");
            list.append(&row);
        }
        let lab = Label::new(Some(&e.label));
        lab.set_xalign(0.0);
        lab.set_widget_name(&e.key);
        let row = ListBoxRow::new();
        row.set_child(Some(&lab));
        row.set_widget_name(&e.key);
        list.append(&row);
    }

    let filter = Rc::clone(&all);
    list.set_filter_func({
        let search = search.clone();
        move |row| {
            if row.widget_name() == "grupo" {
                return true;
            }
            let q = search.text().to_lowercase();
            if q.is_empty() {
                return true;
            }
            row.widget_name().to_lowercase().contains(&q)
                || filter.iter().any(|e| {
                    e.key == row.widget_name().as_str() && e.label.to_lowercase().contains(&q)
                })
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
            if key == "grupo" {
                return;
            }
            if let Some(text) = lookup_ficha(&key) {
                detail.set_text(&text);
            }
        }
    });

    const LEFT_WIDTH: i32 = 320;
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
