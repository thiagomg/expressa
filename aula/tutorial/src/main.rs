#![cfg_attr(windows, windows_subsystem = "windows")]

mod markdown;

use std::path::{Path, PathBuf};
use std::rc::Rc;

use gtk::gdk;
use gtk::pango;
use gtk::prelude::*;
use gtk::{
    Application, ApplicationWindow, Box as GtkBox, CssProvider, Label, ListBox, ListBoxRow,
    Orientation, Paned, PolicyType, ScrolledWindow, SelectionMode, TextBuffer, TextTag, TextView,
};

use markdown::{Block, Inline};

const APP_ID: &str = "dev.expressa.tutorial";
const BASICO: &str = include_str!("../../../tutorial/basico.md");
const AVANCADO: &str = include_str!("../../../tutorial/avancado.md");

fn main() {
    let app = Application::builder()
        .application_id(APP_ID)
        .flags(gtk::gio::ApplicationFlags::NON_UNIQUE)
        .build();
    app.connect_activate(build_ui);
    app.run();
}

fn build_ui(app: &Application) {
    install_css();
    let win = ApplicationWindow::builder()
        .application(app)
        .title("Tutoriais — Expressa")
        .default_width(960)
        .default_height(640)
        .build();

    let books = [
        ("Básico", load_text("basico.md", BASICO)),
        ("Avançado", load_text("avancado.md", AVANCADO)),
    ];

    let outer = GtkBox::new(Orientation::Vertical, 0);
    outer.set_hexpand(true);
    outer.set_vexpand(true);
    let tabs = gtk::Notebook::new();
    tabs.set_hexpand(true);
    tabs.set_vexpand(true);
    for (title, src) in books {
        let page = book_page(&src);
        page.set_hexpand(true);
        page.set_vexpand(true);
        let lab = Label::new(Some(title));
        tabs.append_page(&page, Some(&lab));
    }
    outer.append(&tabs);
    win.set_child(Some(&outer));
    win.present();
}

fn load_text(name: &str, embedded: &str) -> String {
    if let Some(dir) = tutorial_dir() {
        let path = dir.join(name);
        if let Ok(s) = std::fs::read_to_string(&path) {
            return s;
        }
    }
    embedded.to_string()
}

fn tutorial_dir() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("EXPRESSA_TUTORIAL") {
        let p = PathBuf::from(p);
        if p.is_dir() {
            return Some(p);
        }
    }
    if let Ok(exe) = std::env::current_exe()
        && let Some(dir) = exe.parent()
    {
        for cand in [
            dir.join("tutorial"),
            dir.join("share/expressa/tutorial"),
            dir.parent().map(|p| p.join("tutorial")).unwrap_or_default(),
        ] {
            if cand.join("basico.md").is_file() {
                return Some(cand);
            }
        }
    }
    let share = PathBuf::from("/usr/share/expressa/tutorial");
    if share.join("basico.md").is_file() {
        return Some(share);
    }
    let dev = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tutorial");
    if dev.join("basico.md").is_file() {
        return Some(dev);
    }
    None
}

fn book_page(src: &str) -> gtk::Widget {
    let blocks = Rc::new(markdown::parse(src));
    let chapters = markdown::chapters(&blocks);

    let list = ListBox::new();
    list.set_selection_mode(SelectionMode::Single);
    list.set_activate_on_single_click(true);
    list.add_css_class("navigation-sidebar");
    list.set_vexpand(true);
    for ch in chapters.iter() {
        let lab = Label::new(Some(&ch.title));
        lab.set_xalign(0.0);
        lab.set_wrap(true);
        lab.set_wrap_mode(pango::WrapMode::WordChar);
        let row = ListBoxRow::new();
        row.set_child(Some(&lab));
        list.append(&row);
    }

    let list_scroll = ScrolledWindow::builder()
        .hscrollbar_policy(PolicyType::Never)
        .vscrollbar_policy(PolicyType::Automatic)
        .min_content_width(220)
        .vexpand(true)
        .child(&list)
        .build();

    let buffer = TextBuffer::new(None);
    install_tags(&buffer);

    let view = TextView::with_buffer(&buffer);
    view.set_editable(false);
    view.set_cursor_visible(false);
    view.set_wrap_mode(gtk::WrapMode::WordChar);
    view.set_left_margin(18);
    view.set_right_margin(18);
    view.set_top_margin(14);
    view.set_bottom_margin(24);
    view.set_hexpand(true);
    view.set_vexpand(true);
    view.add_css_class("tutorial-body");
    let body = ScrolledWindow::builder()
        .hscrollbar_policy(PolicyType::Never)
        .vscrollbar_policy(PolicyType::Automatic)
        .vexpand(true)
        .hexpand(true)
        .child(&view)
        .build();

    let chapters = Rc::new(chapters);
    let show = {
        let buffer = buffer.clone();
        let blocks = Rc::clone(&blocks);
        let chapters = Rc::clone(&chapters);
        move |i: usize| {
            let span = markdown::chapter_span(&chapters, i, blocks.len());
            let slice = if span.is_empty() {
                blocks.as_slice()
            } else {
                &blocks[span]
            };
            buffer.set_text("");
            fill_buffer(&buffer, slice);
        }
    };
    show(0);
    list.connect_row_selected({
        let show = show.clone();
        move |_, row| {
            let Some(row) = row else { return };
            let i = row.index();
            if i >= 0 {
                show(i as usize);
            }
        }
    });
    list.connect_row_activated({
        let show = show.clone();
        move |_, row| {
            let i = row.index();
            if i >= 0 {
                show(i as usize);
            }
        }
    });
    if let Some(row) = list.row_at_index(0) {
        list.select_row(Some(&row));
    }

    let paned = Paned::new(Orientation::Horizontal);
    paned.set_hexpand(true);
    paned.set_vexpand(true);
    paned.set_start_child(Some(&list_scroll));
    paned.set_end_child(Some(&body));
    paned.set_resize_start_child(false);
    paned.set_shrink_start_child(true);
    paned.set_resize_end_child(true);
    paned.set_position(260);
    paned.upcast()
}

fn install_tags(buffer: &TextBuffer) {
    let tag = |name: &str, apply: fn(&TextTag)| {
        let t = TextTag::new(Some(name));
        apply(&t);
        buffer.tag_table().add(&t);
    };
    tag("h1", |t| {
        t.set_weight(700);
        t.set_scale(1.6);
        t.set_pixels_above_lines(16);
        t.set_pixels_below_lines(8);
    });
    tag("h2", |t| {
        t.set_weight(700);
        t.set_scale(1.25);
        t.set_pixels_above_lines(14);
        t.set_pixels_below_lines(6);
    });
    tag("h3", |t| {
        t.set_weight(600);
        t.set_scale(1.1);
        t.set_pixels_above_lines(10);
        t.set_pixels_below_lines(4);
    });
    tag("code", |t| {
        t.set_family(Some("monospace"));
        t.set_scale(0.95);
        t.set_background(Some("#ececec"));
    });
    tag("codeblock", |t| {
        t.set_family(Some("monospace"));
        t.set_scale(0.95);
        t.set_background(Some("#f4f4f4"));
        t.set_pixels_above_lines(8);
        t.set_pixels_below_lines(8);
        t.set_left_margin(12);
        t.set_right_margin(12);
    });
    tag("bold", |t| t.set_weight(700));
    tag("quote", |t| {
        t.set_style(pango::Style::Italic);
        t.set_left_margin(18);
        t.set_pixels_above_lines(6);
        t.set_pixels_below_lines(6);
    });
    tag("link", |t| {
        t.set_underline(pango::Underline::Single);
        t.set_foreground(Some("#1a5fb4"));
    });
    tag("hr", |t| {
        t.set_pixels_above_lines(8);
        t.set_pixels_below_lines(8);
    });
}

fn fill_buffer(buffer: &TextBuffer, blocks: &[Block]) {
    for block in blocks {
        match block {
            Block::Heading { level, text } => {
                let tag = match level {
                    1 => "h1",
                    2 => "h2",
                    _ => "h3",
                };
                insert_tagged(buffer, text, tag);
                buffer.insert(&mut buffer.end_iter(), "\n");
            }
            Block::Paragraph(xs) => {
                insert_inlines(buffer, xs);
                buffer.insert(&mut buffer.end_iter(), "\n\n");
            }
            Block::Quote(xs) => {
                let start = buffer.end_iter().offset();
                insert_inlines(buffer, xs);
                let a = buffer.iter_at_offset(start);
                let b = buffer.end_iter();
                buffer.apply_tag_by_name("quote", &a, &b);
                buffer.insert(&mut buffer.end_iter(), "\n\n");
            }
            Block::Code { text, .. } => {
                let start = buffer.end_iter().offset();
                buffer.insert(&mut buffer.end_iter(), text);
                buffer.insert(&mut buffer.end_iter(), "\n");
                let a = buffer.iter_at_offset(start);
                let b = buffer.end_iter();
                buffer.apply_tag_by_name("codeblock", &a, &b);
                buffer.insert(&mut buffer.end_iter(), "\n");
            }
            Block::List(items) => {
                for it in items {
                    buffer.insert(&mut buffer.end_iter(), "• ");
                    insert_inlines(buffer, it);
                    buffer.insert(&mut buffer.end_iter(), "\n");
                }
                buffer.insert(&mut buffer.end_iter(), "\n");
            }
            Block::Table { headers, rows } => {
                let line = |cols: &[String]| -> String {
                    cols.iter()
                        .map(|c| c.trim().to_string())
                        .filter(|c| !c.is_empty())
                        .collect::<Vec<_>>()
                        .join("  ·  ")
                };
                let h = line(headers);
                if !h.is_empty() {
                    insert_tagged(buffer, &h, "bold");
                    buffer.insert(&mut buffer.end_iter(), "\n");
                }
                for r in rows {
                    buffer.insert(&mut buffer.end_iter(), &line(r));
                    buffer.insert(&mut buffer.end_iter(), "\n");
                }
                buffer.insert(&mut buffer.end_iter(), "\n");
            }
            Block::Hr => {
                insert_tagged(buffer, "────────", "hr");
                buffer.insert(&mut buffer.end_iter(), "\n\n");
            }
        }
    }
}

fn insert_tagged(buffer: &TextBuffer, text: &str, tag: &str) {
    let start = buffer.end_iter().offset();
    buffer.insert(&mut buffer.end_iter(), text);
    let a = buffer.iter_at_offset(start);
    let b = buffer.end_iter();
    buffer.apply_tag_by_name(tag, &a, &b);
}

fn insert_inlines(buffer: &TextBuffer, xs: &[Inline]) {
    for x in xs {
        match x {
            Inline::Text(s) => buffer.insert(&mut buffer.end_iter(), s),
            Inline::Bold(s) => insert_tagged(buffer, s, "bold"),
            Inline::Code(s) => insert_tagged(buffer, s, "code"),
            Inline::Link { text, .. } => insert_tagged(buffer, text, "link"),
        }
    }
}

fn install_css() {
    let css = CssProvider::new();
    css.load_from_data(
        r#"
window {
    background: #fafafa;
    color: #222;
}
textview.tutorial-body,
textview.tutorial-body text {
    font-size: 12pt;
    background: #fafafa;
    color: #222;
}
        "#,
    );
    if let Some(display) = gdk::Display::default() {
        gtk::style_context_add_provider_for_display(
            &display,
            &css,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}
