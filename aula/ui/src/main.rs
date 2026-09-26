mod rpc;

use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;

use gtk::gdk::Key;
use gtk::glib;
use gtk::glib::prelude::*;
use gtk::prelude::*;
use gtk::{
    Application, ApplicationWindow, Box as GtkBox, Button, CellRendererText, Entry, Label,
    Orientation, Paned, ScrolledWindow, TextTag, TextView, ToggleButton, TreeStore, TreeView,
    TreeViewColumn,
};
use sourceview5::prelude::*;
use sourceview5::{
    Buffer as SourceBuffer, LanguageManager, MarkAttributes, StyleSchemeManager, View as SourceView,
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
    debug_cmd: Option<std::sync::mpsc::Sender<String>>,
    breakpoints: HashSet<u32>,
    watches: Vec<String>,
    debug_tag: TextTag,
    vars_store: TreeStore,
    stack_store: TreeStore,
    watch_store: TreeStore,
    watch_entry: Entry,
    btn_continue: Button,
    btn_next: Button,
    btn_step: Button,
    btn_out: Button,
    btn_stop: Button,
    /// Kept alive so GtkSourceView does not drop the Expressa language spec.
    _languages: LanguageManager,
}

fn main() {
    let app = Application::builder().application_id(APP_ID).build();
    app.connect_activate(build_ui);
    app.connect_shutdown(|_| {
        rpc::kill_spawned_server();
    });
    app.run();
    rpc::kill_spawned_server();
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
    let debug_tag = TextTag::new(Some("debug-current"));
    debug_tag.set_background(Some("#fff3bf"));
    buffer.tag_table().add(&debug_tag);

    #[allow(deprecated)]
    let vars_store = TreeStore::new(&[glib::Type::STRING, glib::Type::STRING, glib::Type::STRING]);
    #[allow(deprecated)]
    let stack_store = TreeStore::new(&[glib::Type::STRING, glib::Type::STRING]);
    #[allow(deprecated)]
    let watch_store = TreeStore::new(&[glib::Type::STRING, glib::Type::STRING]);
    let watch_entry = Entry::builder()
        .placeholder_text("nome da variável")
        .build();
    let btn_continue = Button::with_label("Continuar");
    let btn_next = Button::with_label("Próximo");
    let btn_step = Button::with_label("Entrar");
    let btn_out = Button::with_label("Sair");
    let btn_stop = Button::with_label("Parar");
    for b in [&btn_continue, &btn_next, &btn_step, &btn_out, &btn_stop] {
        b.set_sensitive(false);
    }

    let editor = SourceView::builder()
        .buffer(&buffer)
        .monospace(true)
        .show_line_numbers(true)
        .show_line_marks(true)
        .highlight_current_line(true)
        .auto_indent(true)
        .tab_width(4)
        .build();
    editor.set_wrap_mode(gtk::WrapMode::None);
    let bp_attrs = MarkAttributes::new();
    bp_attrs.set_pixbuf(&red_breakpoint_pixbuf());
    editor.set_mark_attributes("breakpoint", &bp_attrs, 10);

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
        .default_width(1400)
        .default_height(900)
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
        debug_cmd: None,
        breakpoints: HashSet::new(),
        watches: Vec::new(),
        debug_tag,
        vars_store,
        stack_store,
        watch_store,
        watch_entry,
        btn_continue,
        btn_next,
        btn_step,
        btn_out,
        btn_stop,
        _languages: languages,
    }));

    let toolbar = GtkBox::new(Orientation::Horizontal, 8);
    toolbar.set_margin_start(8);
    toolbar.set_margin_end(8);
    toolbar.set_margin_top(8);
    let btn_new = Button::with_label("Novo");
    let btn_save = Button::with_label("Salvar");
    let btn_run = Button::with_label("Rodar");
    let btn_debug = Button::with_label("Depurar");
    let btn_bp = Button::with_label("Ponto");
    let btn_refresh = Button::with_label("Conectar");
    toolbar.append(&btn_new);
    toolbar.append(&btn_save);
    toolbar.append(&btn_run);
    toolbar.append(&btn_debug);
    toolbar.append(&btn_bp);
    toolbar.append(&ui.borrow().btn_continue);
    toolbar.append(&ui.borrow().btn_next);
    toolbar.append(&ui.borrow().btn_step);
    toolbar.append(&ui.borrow().btn_out);
    toolbar.append(&ui.borrow().btn_stop);
    toolbar.append(&Label::new(Some("arquivo:")));
    toolbar.append(&ui.borrow().filename);
    toolbar.append(&Label::new(Some("projeto:")));
    toolbar.append(&ui.borrow().student);
    toolbar.append(&btn_refresh);
    let btn_files = ToggleButton::with_label("Arquivos");
    btn_files.set_active(true);
    btn_files.set_tooltip_text(Some("Mostrar ou ocultar a lista de arquivos (F7)"));
    let btn_panel = ToggleButton::with_label("Painel");
    btn_panel.set_active(true);
    btn_panel.set_tooltip_text(Some("Mostrar ou ocultar o depurador (F8)"));
    toolbar.append(&btn_files);
    toolbar.append(&btn_panel);

    let file_scroll = ScrolledWindow::builder()
        .min_content_width(220)
        .child(&ui.borrow().tree)
        .build();
    let editor_scroll = ScrolledWindow::builder()
        .vexpand(true)
        .hexpand(true)
        .child(&ui.borrow().editor)
        .build();
    const OUTPUT_HEIGHT: i32 = 160;
    let output_scroll = ScrolledWindow::builder()
        .min_content_height(OUTPUT_HEIGHT)
        .vexpand(false)
        .hexpand(true)
        .child(&ui.borrow().output)
        .build();
    output_scroll.set_height_request(OUTPUT_HEIGHT);
    output_scroll.set_valign(gtk::Align::Fill);

    let debug_panel = make_debug_panel(&ui);
    const DEBUG_PANEL_WIDTH: i32 = 320;
    debug_panel.set_width_request(DEBUG_PANEL_WIDTH);
    debug_panel.set_hexpand(false);

    // Box, not Paned: GtkPaned's default position is 0 and keeps collapsing
    // the output. The editor takes leftover height; the console stays 160px.
    let editor_col = GtkBox::new(Orientation::Vertical, 0);
    editor_scroll.set_vexpand(true);
    editor_col.append(&editor_scroll);
    editor_col.append(&gtk::Separator::new(Orientation::Horizontal));
    editor_col.append(&output_scroll);

    let main_split = Paned::new(Orientation::Horizontal);
    main_split.set_start_child(Some(&editor_col));
    main_split.set_end_child(Some(&debug_panel));
    main_split.set_resize_start_child(true);
    main_split.set_shrink_end_child(false);
    main_split.set_wide_handle(true);

    let body = Paned::new(Orientation::Horizontal);
    body.set_start_child(Some(&file_scroll));
    body.set_end_child(Some(&main_split));
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
        btn_run.connect_clicked(move |_| rodar(&ui_r, false));
    }
    {
        let ui_d = Rc::clone(&ui);
        btn_debug.connect_clicked(move |_| rodar(&ui_d, true));
    }
    {
        let ui_b = Rc::clone(&ui);
        btn_bp.connect_clicked(move |_| toggle_breakpoint_here(&ui_b));
    }
    {
        let ui_c = Rc::clone(&ui);
        ui.borrow()
            .btn_continue
            .connect_clicked(move |_| send_debug_cmd(&ui_c, "continuar"));
    }
    {
        let ui_n = Rc::clone(&ui);
        ui.borrow()
            .btn_next
            .connect_clicked(move |_| send_debug_cmd(&ui_n, "proximo"));
    }
    {
        let ui_s = Rc::clone(&ui);
        ui.borrow()
            .btn_step
            .connect_clicked(move |_| send_debug_cmd(&ui_s, "entrar"));
    }
    {
        let ui_o = Rc::clone(&ui);
        ui.borrow()
            .btn_out
            .connect_clicked(move |_| send_debug_cmd(&ui_o, "sair"));
    }
    {
        let ui_t = Rc::clone(&ui);
        ui.borrow()
            .btn_stop
            .connect_clicked(move |_| send_debug_cmd(&ui_t, "terminar"));
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
    {
        let pane = file_scroll.clone();
        btn_files.connect_toggled(move |b| pane.set_visible(b.is_active()));
    }
    {
        let pane = debug_panel.clone();
        btn_panel.connect_toggled(move |b| pane.set_visible(b.is_active()));
    }
    {
        let ui_g = Rc::clone(&ui);
        let click = gtk::GestureClick::new();
        click.set_button(1);
        click.connect_pressed(move |_, _, x, y| {
            if x > 56.0 {
                return;
            }
            let line = {
                let u = ui_g.borrow();
                let (_, by) = u.editor.window_to_buffer_coords(
                    gtk::TextWindowType::Widget,
                    x as i32,
                    y as i32,
                );
                u.editor
                    .iter_at_location(0, by)
                    .map(|it| (it.line() + 1) as u32)
            };
            if let Some(line) = line {
                toggle_breakpoint_line(&ui_g, line);
            }
        });
        ui.borrow().editor.add_controller(click);
    }

    let keys = gtk::EventControllerKey::new();
    {
        let ui_k = Rc::clone(&ui);
        let btn_files_k = btn_files.clone();
        let btn_panel_k = btn_panel.clone();
        keys.connect_key_pressed(move |_, key, _, mods| {
            let ctrl = mods.contains(gtk::gdk::ModifierType::CONTROL_MASK);
            if key == Key::F5 {
                rodar(&ui_k, false);
                return glib::Propagation::Stop;
            }
            if key == Key::F6 {
                rodar(&ui_k, true);
                return glib::Propagation::Stop;
            }
            if key == Key::F7 {
                btn_files_k.set_active(!btn_files_k.is_active());
                return glib::Propagation::Stop;
            }
            if key == Key::F8 {
                btn_panel_k.set_active(!btn_panel_k.is_active());
                return glib::Propagation::Stop;
            }
            if key == Key::F9 {
                toggle_breakpoint_here(&ui_k);
                return glib::Propagation::Stop;
            }
            if key == Key::F10 {
                send_debug_cmd(&ui_k, "proximo");
                return glib::Propagation::Stop;
            }
            if key == Key::F11 {
                send_debug_cmd(&ui_k, "entrar");
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
        "// F5 roda. F6 depura. F7 arquivos. F8 painel. F9 ponto. F10 próximo.\n\nescreva(\"Olá, Expressa!\")\n",
    );
    {
        let ui_close = Rc::clone(&ui);
        win.connect_close_request(move |_| {
            ui_close.borrow_mut().rpc.shutdown_spawned_server();
            glib::Propagation::Proceed
        });
    }
    win.maximize();
    win.present();
    {
        let split = main_split.clone();
        glib::timeout_add_local(std::time::Duration::from_millis(80), move || {
            let w = split.allocated_width();
            if w > DEBUG_PANEL_WIDTH + 400 {
                split.set_position(w - DEBUG_PANEL_WIDTH);
            }
            glib::ControlFlow::Break
        });
    }
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
    ui.borrow_mut().breakpoints.clear();
    set_source(ui, "escreva(\"Olá\")\n");
    set_output(ui, "");
    paint_breakpoints(ui);
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

#[cfg(test)]
fn leia_call_count(src: &str) -> usize {
    leia_prompts(src).len()
}

/// Prompt string of each `leia(...)` call, in order. `None` if there is no
/// string literal (`leia()` or `leia(nome)`).
#[cfg(test)]
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

#[cfg(test)]
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

fn make_string_tree(store: &TreeStore, titles: &[&str]) -> TreeView {
    let tree = TreeView::with_model(store);
    tree.set_headers_visible(true);
    for (i, title) in titles.iter().enumerate() {
        let col = TreeViewColumn::new();
        col.set_title(title);
        col.set_resizable(true);
        let cell = CellRendererText::new();
        col.pack_start(&cell, true);
        col.add_attribute(&cell, "text", i as i32);
        tree.append_column(&col);
    }
    tree
}

fn make_debug_panel(ui: &Rc<RefCell<Ui>>) -> GtkBox {
    let panel = GtkBox::new(Orientation::Vertical, 4);
    panel.set_margin_start(4);
    panel.set_margin_end(4);
    panel.set_hexpand(false);
    panel.set_vexpand(true);

    let watch_row = GtkBox::new(Orientation::Horizontal, 4);
    let lbl = Label::new(Some("Observar:"));
    let btn_add = Button::with_label("+");
    let btn_del = Button::with_label("−");
    watch_row.append(&lbl);
    {
        let u = ui.borrow();
        watch_row.append(&u.watch_entry);
    }
    watch_row.append(&btn_add);
    watch_row.append(&btn_del);
    panel.append(&watch_row);

    {
        let ui_a = Rc::clone(ui);
        btn_add.connect_clicked(move |_| add_watch(&ui_a));
    }
    {
        let ui_a = Rc::clone(ui);
        ui.borrow()
            .watch_entry
            .connect_activate(move |_| add_watch(&ui_a));
    }
    {
        let ui_d = Rc::clone(ui);
        btn_del.connect_clicked(move |_| remove_watch(&ui_d));
    }

    let u = ui.borrow();
    let watch_tree = make_string_tree(&u.watch_store, &["observado", "valor"]);
    let vars_tree = make_string_tree(&u.vars_store, &["onde", "nome", "valor"]);
    let stack_tree = make_string_tree(&u.stack_store, &["função", "linha"]);
    drop(u);

    panel.append(&Label::new(Some(
        "Observados (ficam visíveis a cada passo)",
    )));
    panel.append(&scroll_tree(&watch_tree, 70));
    panel.append(&Label::new(Some("Variáveis")));
    panel.append(&scroll_tree(&vars_tree, 120));
    panel.append(&Label::new(Some("Pilha")));
    panel.append(&scroll_tree(&stack_tree, 70));
    panel
}

fn scroll_tree(tree: &TreeView, min_h: i32) -> ScrolledWindow {
    ScrolledWindow::builder()
        .min_content_height(min_h)
        .vexpand(true)
        .child(tree)
        .build()
}

fn add_watch(ui: &Rc<RefCell<Ui>>) {
    let name = {
        let u = ui.borrow();
        u.watch_entry.text().trim().to_string()
    };
    if name.is_empty() {
        return;
    }
    {
        let mut u = ui.borrow_mut();
        if !u.watches.iter().any(|w| w == &name) {
            u.watches.push(name);
        }
        u.watch_entry.set_text("");
    }
    refresh_watch_values(ui, &[]);
}

fn remove_watch(ui: &Rc<RefCell<Ui>>) {
    let mut u = ui.borrow_mut();
    u.watches.pop();
    drop(u);
    refresh_watch_values(ui, &[]);
}

fn refresh_watch_values(ui: &Rc<RefCell<Ui>>, vars: &[expressa_aula_proto::DebugVar]) {
    let u = ui.borrow();
    u.watch_store.clear();
    for name in &u.watches {
        let val = vars
            .iter()
            .find(|v| v.name == *name)
            .map(|v| v.value.as_str())
            .unwrap_or("—");
        let iter = u.watch_store.append(None);
        u.watch_store.set(&iter, &[(0, name), (1, &val)]);
    }
}

fn send_debug_cmd(ui: &Rc<RefCell<Ui>>, cmd: &str) {
    let u = ui.borrow();
    if let Some(tx) = &u.debug_cmd {
        let _ = tx.send(cmd.to_string());
    }
}

fn set_debug_buttons(ui: &Rc<RefCell<Ui>>, paused: bool, running: bool) {
    let u = ui.borrow();
    u.btn_continue.set_sensitive(paused);
    u.btn_next.set_sensitive(paused);
    u.btn_step.set_sensitive(paused);
    u.btn_out.set_sensitive(paused);
    u.btn_stop.set_sensitive(running);
}

fn red_breakpoint_pixbuf() -> gdk_pixbuf::Pixbuf {
    let pb = gdk_pixbuf::Pixbuf::new(gdk_pixbuf::Colorspace::Rgb, true, 8, 12, 12)
        .expect("pixbuf");
    pb.fill(0x0000_0000);
    for y in 0..12 {
        for x in 0..12 {
            let dx = x as i32 - 5;
            let dy = y as i32 - 5;
            if dx * dx + dy * dy <= 16 {
                pb.put_pixel(x as u32, y as u32, 0xe5, 0x39, 0x35, 0xff);
            }
        }
    }
    pb
}

fn toggle_breakpoint_here(ui: &Rc<RefCell<Ui>>) {
    let line = {
        let u = ui.borrow();
        let insert = u.buffer.iter_at_mark(&u.buffer.get_insert());
        (insert.line() + 1) as u32
    };
    toggle_breakpoint_line(ui, line);
}

fn toggle_breakpoint_line(ui: &Rc<RefCell<Ui>>, line: u32) {
    if line == 0 {
        return;
    }
    {
        let mut u = ui.borrow_mut();
        if !u.breakpoints.remove(&line) {
            u.breakpoints.insert(line);
        }
    }
    paint_breakpoints(ui);
    let u = ui.borrow();
    if u.debug_cmd.is_some() {
        if u.breakpoints.contains(&line) {
            drop(u);
            send_debug_cmd(ui, &format!("ponto {line}"));
        } else {
            drop(u);
            send_debug_cmd(ui, &format!("remover {line}"));
        }
    }
}

fn paint_breakpoints(ui: &Rc<RefCell<Ui>>) {
    let u = ui.borrow();
    let (start, end) = u.buffer.bounds();
    u.buffer
        .remove_source_marks(&start, &end, Some("breakpoint"));
    let lines: Vec<u32> = u.breakpoints.iter().copied().collect();
    let buffer = u.buffer.clone();
    drop(u);
    for line in lines {
        if line == 0 {
            continue;
        }
        let Some(a) = buffer.iter_at_line((line - 1) as i32) else {
            continue;
        };
        buffer.create_source_mark(Some(&format!("bp-{line}")), "breakpoint", &a);
    }
}

fn mark_debug_line(ui: &Rc<RefCell<Ui>>, line: u32) {
    let u = ui.borrow();
    let (start, end) = u.buffer.bounds();
    u.buffer.remove_tag(&u.debug_tag, &start, &end);
    if line == 0 {
        return;
    }
    let Some(a) = u.buffer.iter_at_line((line - 1) as i32) else {
        return;
    };
    let mut b = a.clone();
    if !b.forward_line() {
        b = u.buffer.end_iter();
    }
    u.buffer.apply_tag(&u.debug_tag, &a, &b);
    u.buffer.place_cursor(&a);
    u.editor
        .scroll_to_iter(&mut a.clone(), 0.2, false, 0.0, 0.0);
}

fn fill_debug_views(ui: &Rc<RefCell<Ui>>, paused: &expressa_aula_proto::DebugPaused) {
    {
        let u = ui.borrow();
        u.vars_store.clear();
        u.stack_store.clear();
        for v in &paused.vars {
            let iter = u.vars_store.append(None);
            u.vars_store
                .set(&iter, &[(0, &v.scope), (1, &v.name), (2, &v.value)]);
        }
        for f in &paused.stack {
            let loc = format!("{}:{}", f.file, f.line);
            let iter = u.stack_store.append(None);
            u.stack_store.set(&iter, &[(0, &f.name), (1, &loc)]);
        }
    }
    refresh_watch_values(ui, &paused.vars);
    mark_debug_line(ui, paused.line);
}

fn clear_debug_views(ui: &Rc<RefCell<Ui>>) {
    let u = ui.borrow();
    u.vars_store.clear();
    u.stack_store.clear();
    let (start, end) = u.buffer.bounds();
    u.buffer.remove_tag(&u.debug_tag, &start, &end);
}

fn append_output(ui: &Rc<RefCell<Ui>>, text: &str) {
    let u = ui.borrow();
    let buf = u.output.buffer();
    let mut end = buf.end_iter();
    buf.insert(&mut end, text);
}

fn rodar(ui: &Rc<RefCell<Ui>>, debug: bool) {
    set_output(ui, "");
    clear_debug_views(ui);
    let (student, name, src, bps) = {
        let u = ui.borrow();
        (
            student(&u),
            filename(&u),
            source_text(&u),
            u.breakpoints.iter().copied().collect::<Vec<_>>(),
        )
    };
    set_status(
        ui,
        &format!("{} {name}…", if debug { "depurando" } else { "rodando" }),
    );

    let (event_tx, event_rx) = std::sync::mpsc::channel::<ExecEvent>();
    let (line_tx, line_rx) = std::sync::mpsc::channel::<String>();
    let (dcmd_tx, dcmd_rx) = std::sync::mpsc::channel::<String>();
    ui.borrow_mut().debug_cmd = Some(dcmd_tx);
    set_debug_buttons(ui, false, true);

    {
        let u = ui.borrow();
        u.rpc.spawn_exec(
            student,
            name.clone(),
            src,
            debug,
            bps,
            event_tx,
            line_rx,
            dcmd_rx,
        );
    }

    let ui_ev = Rc::clone(ui);
    let event_rx = Rc::new(RefCell::new(event_rx));
    glib::timeout_add_local(std::time::Duration::from_millis(16), move || {
        loop {
            match event_rx.borrow().try_recv() {
                Ok(ExecEvent::Stdout(s)) => append_output(&ui_ev, &s),
                Ok(ExecEvent::Paused(p)) => {
                    fill_debug_views(&ui_ev, &p);
                    set_debug_buttons(&ui_ev, true, true);
                    set_status(
                        &ui_ev,
                        &format!("pausado {}:{}  {}", p.file, p.line, p.source_line.trim()),
                    );
                }
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
                    ui_ev.borrow_mut().debug_cmd = None;
                    set_debug_buttons(&ui_ev, false, false);
                    clear_debug_views(&ui_ev);
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
            ui.borrow_mut().breakpoints.clear();
            set_source(ui, &src);
            paint_breakpoints(ui);
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
