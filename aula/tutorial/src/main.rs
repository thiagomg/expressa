#![cfg_attr(windows, windows_subsystem = "windows")]

mod markdown;

use std::cell::{Cell, RefCell};
use std::path::{Path, PathBuf};
use std::rc::Rc;

use gtk::gio::prelude::FileExt;

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
    let tutorial = tutorial_dir();
    let abrir: Rc<dyn Fn(&str, &gtk::Widget)> = {
        let tabs = tabs.clone();
        Rc::new(move |href, widget| abrir_link(href, &tabs, tutorial.as_deref(), widget))
    };
    for (title, src) in books {
        let page = book_page(&src, Rc::clone(&abrir));
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
        for cand in candidatos_do_executavel(dir) {
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

fn candidatos_do_executavel(dir: &Path) -> [PathBuf; 3] {
    [
        dir.join("tutorial"),
        dir.parent()
            .map(|pai| pai.join("share/expressa/tutorial"))
            .unwrap_or_default(),
        dir.parent()
            .map(|pai| pai.join("tutorial"))
            .unwrap_or_default(),
    ]
}

fn destino_do_link(href: &str) -> Destino<'_> {
    let caminho = href.split(['#', '?']).next().unwrap_or(href);
    if caminho.starts_with("http://") || caminho.starts_with("https://") {
        return Destino::Endereco(caminho);
    }
    let nome = Path::new(caminho)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("");
    if matches!(nome, "basico.md" | "avancado.md")
        && !caminho.contains('/')
        && !caminho.contains('\\')
    {
        return Destino::Tutorial(nome);
    }
    Destino::Arquivo(caminho)
}

enum Destino<'a> {
    Tutorial(&'a str),
    Endereco(&'a str),
    Arquivo(&'a str),
}

fn abrir_link(href: &str, tabs: &gtk::Notebook, tutorial: Option<&Path>, widget: &gtk::Widget) {
    match destino_do_link(href) {
        Destino::Tutorial("avancado.md") => {
            tabs.set_current_page(Some(1));
        }
        Destino::Tutorial(_) => {
            tabs.set_current_page(Some(0));
        }
        Destino::Endereco(uri) => {
            let pai = widget
                .root()
                .and_then(|raiz| raiz.downcast::<gtk::Window>().ok());
            gtk::show_uri(pai.as_ref(), uri, 0);
        }
        Destino::Arquivo(rel) => {
            let Some(base) = tutorial else {
                avisar(
                    widget,
                    "não achei a pasta dos tutoriais para abrir o arquivo",
                );
                return;
            };
            let alvo = base.join(rel);
            if !alvo.is_file() {
                avisar(widget, &format!("não achei o arquivo `{rel}`"));
                return;
            }
            let uri = gtk::gio::File::for_path(&alvo).uri();
            let pai = widget
                .root()
                .and_then(|raiz| raiz.downcast::<gtk::Window>().ok());
            gtk::show_uri(pai.as_ref(), uri.as_str(), 0);
        }
    }
}

fn avisar(widget: &gtk::Widget, texto: &str) {
    let pai = widget
        .root()
        .and_then(|raiz| raiz.downcast::<gtk::Window>().ok());
    let dialog = gtk::MessageDialog::new(
        pai.as_ref(),
        gtk::DialogFlags::MODAL,
        gtk::MessageType::Info,
        gtk::ButtonsType::Close,
        texto,
    );
    dialog.connect_response(|dialog, _| dialog.close());
    dialog.present();
}

fn book_page(src: &str, abrir: Rc<dyn Fn(&str, &gtk::Widget)>) -> gtk::Widget {
    let blocks = Rc::new(markdown::parse(src));
    let chapters = markdown::chapters(&blocks);

    let list = ListBox::new();
    list.set_selection_mode(SelectionMode::Single);
    list.set_activate_on_single_click(false);
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
    let links: Rc<RefCell<Vec<(i32, i32, String)>>> = Rc::new(RefCell::new(Vec::new()));
    let mostrado = Rc::new(Cell::new(None));
    let show = {
        let buffer = buffer.clone();
        let blocks = Rc::clone(&blocks);
        let chapters = Rc::clone(&chapters);
        let links = Rc::clone(&links);
        let mostrado = Rc::clone(&mostrado);
        move |i: usize| {
            if mostrado.get() == Some(i) {
                return;
            }
            mostrado.set(Some(i));
            let span = markdown::chapter_span(&chapters, i, blocks.len());
            let slice = if span.is_empty() {
                blocks.as_slice()
            } else {
                &blocks[span]
            };
            buffer.set_text("");
            let mut achados = Vec::new();
            fill_buffer(&buffer, slice, &mut achados);
            *links.borrow_mut() = achados;
        }
    };
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
    show(0);
    if let Some(row) = list.row_at_index(0) {
        list.select_row(Some(&row));
    }

    let clique = gtk::GestureClick::new();
    clique.set_button(1);
    clique.connect_pressed({
        let links = Rc::clone(&links);
        let abrir = Rc::clone(&abrir);
        move |gesto, _, x, y| {
            let Some(widget) = gesto.widget() else {
                return;
            };
            let Some(view) = widget.downcast_ref::<TextView>().cloned() else {
                return;
            };
            let (x, y) =
                view.window_to_buffer_coords(gtk::TextWindowType::Widget, x as i32, y as i32);
            let Some(iter) = view.iter_at_location(x, y) else {
                return;
            };
            let off = iter.offset();
            let href = links
                .borrow()
                .iter()
                .find(|(inicio, fim, _)| off >= *inicio && off < *fim)
                .map(|(_, _, href)| href.clone());
            if let Some(href) = href {
                gesto.set_state(gtk::EventSequenceState::Claimed);
                abrir(&href, view.upcast_ref());
            }
        }
    });
    view.add_controller(clique);

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

fn fill_buffer(buffer: &TextBuffer, blocks: &[Block], links: &mut Vec<(i32, i32, String)>) {
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
                insert_inlines(buffer, xs, links);
                buffer.insert(&mut buffer.end_iter(), "\n\n");
            }
            Block::Quote(xs) => {
                let start = buffer.end_iter().offset();
                insert_inlines(buffer, xs, links);
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
                    insert_inlines(buffer, it, links);
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

fn insert_inlines(buffer: &TextBuffer, xs: &[Inline], links: &mut Vec<(i32, i32, String)>) {
    for x in xs {
        match x {
            Inline::Text(s) => buffer.insert(&mut buffer.end_iter(), s),
            Inline::Bold(s) => insert_tagged(buffer, s, "bold"),
            Inline::Code(s) => insert_tagged(buffer, s, "code"),
            Inline::Link { text, href } => {
                let inicio = buffer.end_iter().offset();
                insert_tagged(buffer, text, "link");
                let fim = buffer.end_iter().offset();
                if fim > inicio {
                    links.push((inicio, fim, href.clone()));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn candidato_em_bin_sobe_para_share() {
        let c = candidatos_do_executavel(Path::new("/usr/bin"));
        assert_eq!(c[0], PathBuf::from("/usr/bin/tutorial"));
        assert_eq!(c[1], PathBuf::from("/usr/share/expressa/tutorial"));
        assert_eq!(c[2], PathBuf::from("/usr/tutorial"));
    }

    #[test]
    fn destino_separa_tutorial_endereco_e_arquivo() {
        assert!(matches!(
            destino_do_link("basico.md"),
            Destino::Tutorial("basico.md")
        ));
        assert!(matches!(
            destino_do_link("avancado.md#fim"),
            Destino::Tutorial("avancado.md")
        ));
        assert!(matches!(
            destino_do_link("https://exemplo.test/a?q=1"),
            Destino::Endereco("https://exemplo.test/a")
        ));
        assert!(matches!(
            destino_do_link("../docs/manual.md"),
            Destino::Arquivo("../docs/manual.md")
        ));
        assert!(matches!(
            destino_do_link("pasta/basico.md"),
            Destino::Arquivo("pasta/basico.md")
        ));
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
