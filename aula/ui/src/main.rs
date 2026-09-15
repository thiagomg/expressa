mod rpc;

use std::cell::RefCell;
use std::rc::Rc;

use gtk::gdk::Key;
use gtk::glib;
use gtk::glib::prelude::*;
use gtk::prelude::*;
use gtk::{
    Application, ApplicationWindow, Box as GtkBox, Button, CellRendererText, Entry, Label,
    Orientation, Paned, ScrolledWindow, TextView, TreeStore, TreeView, TreeViewColumn,
};
use sourceview5::prelude::*;
use sourceview5::{
    Buffer as SourceBuffer, LanguageManager, StyleSchemeManager, View as SourceView,
};

use rpc::{ExecEvent, Rpc, ensure_server};

const APP_ID: &str = "dev.expressa.aula";
const DEFAULT_URL: &str = "http://127.0.0.1:50051";

struct Ui {
    rpc: Rpc,
    student: Entry,
    filename: Entry,
    tree: TreeView,
    store: TreeStore,
    editor: SourceView,
    buffer: SourceBuffer,
    output: TextView,
    status: Label,
    window: ApplicationWindow,
    /// Kept alive so GtkSourceView does not drop the Expressa language spec.
    _languages: LanguageManager,
}

fn main() {
    let app = Application::builder().application_id(APP_ID).build();
    app.connect_activate(build_ui);
    app.run();
}

fn build_ui(app: &Application) {
    let rpc = match ensure_server(DEFAULT_URL) {
        Ok(rpc) => rpc,
        Err(e) => {
            eprintln!("{e}");
            let win = ApplicationWindow::builder()
                .application(app)
                .title("Expressa Aula")
                .default_width(480)
                .default_height(160)
                .child(&Label::new(Some(&e)))
                .build();
            win.present();
            return;
        }
    };

    let student = Entry::builder()
        .text("local")
        .placeholder_text("aluno")
        .build();
    let filename = Entry::builder()
        .text("sem-titulo.lep")
        .placeholder_text("arquivo.lep")
        .build();
    #[allow(deprecated)]
    let store = TreeStore::new(&[glib::Type::STRING, glib::Type::STRING, glib::Type::BOOL]);
    let tree = TreeView::with_model(&store);
    tree.set_headers_visible(false);
    tree.set_enable_search(true);
    tree.set_search_column(0);
    let column = TreeViewColumn::new();
    let cell = CellRendererText::new();
    column.pack_start(&cell, true);
    column.add_attribute(&cell, "text", 0);
    tree.append_column(&column);

    let languages = language_manager();
    let lang_ok = languages.language("expressa").is_some();
    let buffer = SourceBuffer::new(None::<&gtk::TextTagTable>);
    if let Some(lang) = languages.language("expressa") {
        buffer.set_language(Some(&lang));
    }
    apply_style_scheme(&buffer);
    buffer.set_highlight_syntax(true);

    let editor = SourceView::builder()
        .buffer(&buffer)
        .monospace(true)
        .show_line_numbers(true)
        .highlight_current_line(true)
        .auto_indent(true)
        .tab_width(4)
        .build();
    editor.set_wrap_mode(gtk::WrapMode::None);

    let output = TextView::builder()
        .editable(false)
        .monospace(true)
        .wrap_mode(gtk::WrapMode::WordChar)
        .build();
    let status = Label::new(Some("conectado a 127.0.0.1:50051"));
    status.set_xalign(0.0);
    status.add_css_class("dim-label");

    let win = ApplicationWindow::builder()
        .application(app)
        .title("Expressa Aula")
        .default_width(960)
        .default_height(640)
        .build();

    let ui = Rc::new(RefCell::new(Ui {
        rpc,
        student,
        filename,
        tree,
        store,
        editor,
        buffer,
        output,
        status,
        window: win.clone(),
        _languages: languages,
    }));

    let toolbar = GtkBox::new(Orientation::Horizontal, 8);
    toolbar.set_margin_start(8);
    toolbar.set_margin_end(8);
    toolbar.set_margin_top(8);
    let btn_new = Button::with_label("Novo");
    let btn_save = Button::with_label("Salvar");
    let btn_run = Button::with_label("Rodar");
    let btn_refresh = Button::with_label("Conectar");
    toolbar.append(&btn_new);
    toolbar.append(&btn_save);
    toolbar.append(&btn_run);
    toolbar.append(&Label::new(Some("arquivo:")));
    toolbar.append(&ui.borrow().filename);
    toolbar.append(&Label::new(Some("projeto:")));
    toolbar.append(&ui.borrow().student);
    toolbar.append(&btn_refresh);

    let file_scroll = ScrolledWindow::builder()
        .min_content_width(220)
        .child(&ui.borrow().tree)
        .build();
    let editor_scroll = ScrolledWindow::builder()
        .vexpand(true)
        .hexpand(true)
        .child(&ui.borrow().editor)
        .build();
    let output_scroll = ScrolledWindow::builder()
        .min_content_height(140)
        .child(&ui.borrow().output)
        .build();

    let right = Paned::new(Orientation::Vertical);
    right.set_start_child(Some(&editor_scroll));
    right.set_end_child(Some(&output_scroll));
    right.set_resize_start_child(true);
    right.set_wide_handle(true);

    let body = Paned::new(Orientation::Horizontal);
    body.set_start_child(Some(&file_scroll));
    body.set_end_child(Some(&right));
    body.set_resize_end_child(true);
    body.set_position(200);

    let root = GtkBox::new(Orientation::Vertical, 6);
    root.append(&toolbar);
    root.append(&body);
    root.append(&ui.borrow().status);
    ui.borrow().status.set_margin_start(8);
    ui.borrow().status.set_margin_bottom(6);
    body.set_vexpand(true);

    win.set_child(Some(&root));

    {
        let ui_n = Rc::clone(&ui);
        btn_new.connect_clicked(move |_| novo(&ui_n));
    }
    {
        let ui_s = Rc::clone(&ui);
        btn_save.connect_clicked(move |_| salvar(&ui_s));
    }
    {
        let ui_r = Rc::clone(&ui);
        btn_run.connect_clicked(move |_| rodar(&ui_r));
    }
    {
        let ui_f = Rc::clone(&ui);
        btn_refresh.connect_clicked(move |_| conectar(&ui_f));
    }
    {
        let ui_e = Rc::clone(&ui);
        ui.borrow()
            .student
            .connect_activate(move |_| conectar(&ui_e));
    }
    {
        let ui_o = Rc::clone(&ui);
        ui.borrow()
            .tree
            .connect_row_activated(move |_, path, _col| {
                abrir_no(&ui_o, path);
            });
    }

    let keys = gtk::EventControllerKey::new();
    {
        let ui_k = Rc::clone(&ui);
        keys.connect_key_pressed(move |_, key, _, mods| {
            let ctrl = mods.contains(gtk::gdk::ModifierType::CONTROL_MASK);
            if key == Key::F5 {
                rodar(&ui_k);
                return glib::Propagation::Stop;
            }
            if ctrl && key == Key::s {
                salvar(&ui_k);
                return glib::Propagation::Stop;
            }
            glib::Propagation::Proceed
        });
    }
    win.add_controller(keys);

    conectar(&ui);
    if !lang_ok {
        set_status(
            &ui,
            "aviso: gramática Expressa não carregou; o texto fica sem cores",
        );
    }
    set_source(
        &ui,
        "// F5 roda. Ctrl+S salva.\n// Mude os números e aperte Rodar.\n\nescreva(\"Olá, Expressa!\")\n",
    );
    win.present();
}

fn data_dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("res")
}

fn language_manager() -> LanguageManager {
    // GtkSourceView 5 search paths are the folders that *contain* *.lang
    // files (e.g. .../language-specs), not the parent of that folder.
    let lm = LanguageManager::default();
    let specs = data_dir().join("language-specs");
    if let Some(path) = specs.to_str() {
        lm.prepend_search_path(path);
    }
    lm
}

fn apply_style_scheme(buffer: &SourceBuffer) {
    let sm = StyleSchemeManager::default();
    let styles = data_dir().join("styles");
    if let Some(path) = styles.to_str() {
        sm.prepend_search_path(path);
    }
    for id in ["expressa-aula", "Yaru", "Yaru-dark", "Adwaita", "classic"] {
        if let Some(scheme) = sm.scheme(id) {
            buffer.set_style_scheme(Some(&scheme));
            return;
        }
    }
}

fn student(ui: &Ui) -> String {
    let s = ui.student.text();
    if s.is_empty() {
        "local".into()
    } else {
        s.to_string()
    }
}

fn filename(ui: &Ui) -> String {
    let s = ui.filename.text();
    if s.is_empty() {
        "sem-titulo.lep".into()
    } else if s.ends_with(".lep") {
        s.to_string()
    } else {
        format!("{s}.lep")
    }
}

fn source_text(ui: &Ui) -> String {
    let (start, end) = ui.buffer.bounds();
    ui.buffer.text(&start, &end, true).to_string()
}

fn set_source(ui: &Rc<RefCell<Ui>>, text: &str) {
    let u = ui.borrow();
    u.buffer.set_text(text);
}

fn set_output(ui: &Rc<RefCell<Ui>>, text: &str) {
    let u = ui.borrow();
    u.output.buffer().set_text(text);
}

fn set_status(ui: &Rc<RefCell<Ui>>, text: &str) {
    ui.borrow().status.set_text(text);
}

fn novo(ui: &Rc<RefCell<Ui>>) {
    ui.borrow().filename.set_text("sem-titulo.lep");
    set_source(ui, "escreva(\"Olá\")\n");
    set_output(ui, "");
    set_status(ui, "arquivo novo (ainda não salvo)");
}

fn salvar(ui: &Rc<RefCell<Ui>>) {
    let (rpc_result, name) = {
        let u = ui.borrow();
        let name = filename(&u);
        let stu = student(&u);
        let src = source_text(&u);
        (u.rpc.write_file(&stu, &name, &src), name)
    };
    match rpc_result {
        Ok(()) => {
            set_status(ui, &format!("salvo {name}"));
            conectar(ui);
        }
        Err(e) => set_status(ui, &format!("erro ao salvar: {e}")),
    }
}

fn leia_call_count(src: &str) -> usize {
    leia_prompts(src).len()
}

/// Prompt string of each `leia(...)` call, in order. `None` if there is no
/// string literal (`leia()` or `leia(nome)`).
fn leia_prompts(src: &str) -> Vec<Option<String>> {
    let mut out = Vec::new();
    let mut rest = src;
    while let Some(i) = rest.find("leia") {
        let before = if i == 0 {
            None
        } else {
            rest[..i].chars().last()
        };
        let ident_cont = before.is_some_and(|c| c.is_alphanumeric() || c == '_');
        let after = rest[i + 4..].trim_start();
        if !ident_cont && after.starts_with('(') {
            let inside = after[1..].trim_start();
            out.push(parse_string_literal(inside));
        }
        rest = &rest[i + 4..];
    }
    out
}

fn parse_string_literal(s: &str) -> Option<String> {
    let s = s.trim_start();
    if !s.starts_with('"') {
        return None;
    }
    let mut out = String::new();
    let mut chars = s[1..].chars();
    while let Some(c) = chars.next() {
        match c {
            '"' => return Some(out),
            '\\' => match chars.next()? {
                'n' => out.push('\n'),
                't' => out.push('\t'),
                '"' => out.push('"'),
                '\\' => out.push('\\'),
                other => out.push(other),
            },
            _ => out.push(c),
        }
    }
    None
}

fn perguntar_linha(parent: &impl IsA<gtk::Window>, pergunta: &str) -> Option<String> {
    let dialog = gtk::Window::builder()
        .transient_for(parent)
        .modal(true)
        .title(pergunta)
        .default_width(400)
        .build();
    let v = GtkBox::new(Orientation::Vertical, 10);
    v.set_margin_start(16);
    v.set_margin_end(16);
    v.set_margin_top(16);
    v.set_margin_bottom(16);
    let lbl = Label::new(Some(pergunta));
    lbl.set_wrap(true);
    lbl.set_xalign(0.0);
    let entry = Entry::new();
    let buttons = GtkBox::new(Orientation::Horizontal, 8);
    buttons.set_halign(gtk::Align::End);
    let cancel = Button::with_label("Cancelar");
    let ok = Button::with_label("OK");
    ok.add_css_class("suggested-action");
    buttons.append(&cancel);
    buttons.append(&ok);
    v.append(&lbl);
    v.append(&entry);
    v.append(&buttons);
    dialog.set_child(Some(&v));

    let loop_ = glib::MainLoop::new(None, false);
    let out = Rc::new(RefCell::new(None::<String>));
    let accept = Rc::new({
        let loop_q = loop_.clone();
        let out_q = Rc::clone(&out);
        let entry_q = entry.clone();
        let dialog_q = dialog.clone();
        move || {
            *out_q.borrow_mut() = Some(entry_q.text().to_string());
            dialog_q.close();
            loop_q.quit();
        }
    });
    {
        let accept = Rc::clone(&accept);
        ok.connect_clicked(move |_| accept());
    }
    {
        let accept = Rc::clone(&accept);
        entry.connect_activate(move |_| accept());
    }
    {
        let loop_c = loop_.clone();
        let dialog_c = dialog.clone();
        cancel.connect_clicked(move |_| {
            dialog_c.close();
            loop_c.quit();
        });
    }
    {
        let loop_c = loop_.clone();
        dialog.connect_close_request(move |_| {
            loop_c.quit();
            glib::Propagation::Proceed
        });
    }
    dialog.present();
    entry.grab_focus();
    loop_.run();
    out.borrow_mut().take()
}

fn append_output(ui: &Rc<RefCell<Ui>>, text: &str) {
    let u = ui.borrow();
    let buf = u.output.buffer();
    let mut end = buf.end_iter();
    buf.insert(&mut end, text);
}

fn rodar(ui: &Rc<RefCell<Ui>>) {
    set_output(ui, "");
    let (student, name, src) = {
        let u = ui.borrow();
        (student(&u), filename(&u), source_text(&u))
    };
    set_status(ui, &format!("rodando {name}…"));

    let (event_tx, event_rx) = std::sync::mpsc::channel::<ExecEvent>();
    let (line_tx, line_rx) = std::sync::mpsc::channel::<String>();

    {
        let u = ui.borrow();
        u.rpc
            .spawn_exec(student, name.clone(), src, event_tx, line_rx);
    }

    let ui_ev = Rc::clone(ui);
    let event_rx = Rc::new(RefCell::new(event_rx));
    glib::timeout_add_local(std::time::Duration::from_millis(16), move || {
        loop {
            match event_rx.borrow().try_recv() {
                Ok(ExecEvent::Stdout(s)) => append_output(&ui_ev, &s),
                Ok(ExecEvent::Leia(prompt)) => {
                    let pergunta = if prompt.trim().is_empty() {
                        "Digite um texto:".to_string()
                    } else {
                        prompt
                    };
                    let window = ui_ev.borrow().window.clone();
                    let line = perguntar_linha(&window, &pergunta).unwrap_or_default();
                    let _ = line_tx.send(line);
                }
                Ok(ExecEvent::Finished(f)) => {
                    if f.ok {
                        set_status(&ui_ev, &format!("ok — {name}"));
                    } else {
                        append_output(
                            &ui_ev,
                            &format!(
                                "erro: {} em {}:{}\n",
                                f.error_message, f.error_file, f.error_line
                            ),
                        );
                        set_status(&ui_ev, "erro ao rodar");
                        if f.error_line > 0 {
                            ir_para_linha(&ui_ev, f.error_line);
                        }
                    }
                    return glib::ControlFlow::Break;
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => break,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    return glib::ControlFlow::Break;
                }
            }
        }
        glib::ControlFlow::Continue
    });
}

fn ir_para_linha(ui: &Rc<RefCell<Ui>>, line: u32) {
    let u = ui.borrow();
    let line = line.saturating_sub(1) as i32;
    let iter = u.buffer.iter_at_line(line).unwrap_or_else(|| {
        let (s, _) = u.buffer.bounds();
        s
    });
    u.buffer.place_cursor(&iter);
    u.editor
        .scroll_to_iter(&mut iter.clone(), 0.2, false, 0.0, 0.0);
}

fn conectar(ui: &Rc<RefCell<Ui>>) {
    let (entries, projeto, err) = {
        let u = ui.borrow();
        let projeto = student(&u);
        match u.rpc.list_tree(&projeto) {
            Ok(p) => (p, projeto, None),
            Err(e) => (Vec::new(), projeto, Some(e)),
        }
    };
    preencher_arvore(ui, &projeto, &entries);
    ui.borrow().tree.expand_all();
    match err {
        Some(e) => set_status(ui, &format!("conectar: {e}")),
        None => set_status(
            ui,
            &format!("projeto `{projeto}` — {} itens", entries.len()),
        ),
    }
}

#[allow(deprecated)]
fn preencher_arvore(
    ui: &Rc<RefCell<Ui>>,
    projeto: &str,
    entries: &[expressa_aula_proto::TreeEntry],
) {
    use std::collections::HashMap;

    let store = ui.borrow().store.clone();
    store.clear();
    let root = store.append(None);
    let root_label = format!("{projeto}/");
    store.set(&root, &[(0, &root_label), (1, &""), (2, &true)]);

    let mut dirs: HashMap<String, gtk::TreeIter> = HashMap::new();
    dirs.insert(String::new(), root);

    for entry in entries {
        let parent_key = match entry.path.rfind('/') {
            Some(i) => entry.path[..i].to_string(),
            None => String::new(),
        };
        let Some(parent) = dirs.get(&parent_key) else {
            continue;
        };
        let name = entry
            .path
            .rsplit('/')
            .next()
            .unwrap_or(&entry.path)
            .to_string();
        let label = if entry.is_dir {
            format!("{name}/")
        } else {
            name
        };
        let iter = store.append(Some(parent));
        store.set(&iter, &[(0, &label), (1, &entry.path), (2, &entry.is_dir)]);
        if entry.is_dir {
            dirs.insert(entry.path.clone(), iter);
        }
    }
}

fn abrir_no(ui: &Rc<RefCell<Ui>>, path: &gtk::TreePath) {
    let (rel, is_dir) = {
        let u = ui.borrow();
        let model = u.store.upcast_ref::<gtk::TreeModel>();
        let Some(iter) = model.iter(path) else {
            return;
        };
        let rel: String = model.get(&iter, 1);
        let is_dir: bool = model.get(&iter, 2);
        (rel, is_dir)
    };
    if is_dir || rel.is_empty() {
        let u = ui.borrow();
        if u.tree.row_expanded(path) {
            u.tree.collapse_row(path);
        } else {
            u.tree.expand_row(path, false);
        }
        return;
    }
    let result = {
        let u = ui.borrow();
        u.rpc.read_file(&student(&u), &rel)
    };
    match result {
        Ok(src) => {
            ui.borrow().filename.set_text(&rel);
            set_source(ui, &src);
            set_status(ui, &format!("aberto {rel}"));
        }
        Err(e) => set_status(ui, &format!("abrir: {e}")),
    }
}

#[cfg(test)]
mod tests {
    use super::{leia_call_count, leia_prompts};

    #[test]
    fn leia_count_ignores_leia_arquivo() {
        assert_eq!(
            leia_call_count("x = leia()\nlinhas = leia_arquivo(\"a\")"),
            1
        );
        assert_eq!(leia_call_count("leia(\"n\")\nleia()"), 2);
        assert_eq!(leia_call_count("escreva(1)"), 0);
        assert_eq!(leia_call_count("aleia()"), 0);
    }

    #[test]
    fn leia_prompt_from_string_literal() {
        assert_eq!(
            leia_prompts(r#"nome = leia("Seu nome:")"#),
            vec![Some("Seu nome:".into())]
        );
        assert_eq!(leia_prompts("x = leia()"), vec![None]);
        assert_eq!(
            leia_prompts("leia(\"a\")\nleia()"),
            vec![Some("a".into()), None]
        );
    }
}
