mod indent;
mod rpc;

use std::cell::{Cell, RefCell};
use std::collections::HashSet;
use std::rc::Rc;

use gtk::gdk::Key;
use gtk::glib;
use gtk::glib::prelude::*;
use gtk::prelude::*;
use gtk::{
    Application, ApplicationWindow, Box as GtkBox, Button, CellRendererText, Entry, Label,
    Orientation, Paned, Revealer, ScrolledWindow, TextTag, TextView, ToggleButton, TreeStore,
    TreeView, TreeViewColumn,
};
use sourceview5::prelude::*;
use sourceview5::{
    Buffer as SourceBuffer, LanguageManager, MarkAttributes, SearchContext, SearchSettings,
    StyleSchemeManager, View as SourceView,
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
    args_entry: Entry,
    dirty: bool,
    suppress_dirty: bool,
    err_tag: TextTag,
    error_link_tag: TextTag,
    match_tag: TextTag,
    search_settings: SearchSettings,
    search_ctx: SearchContext,
    search_entry: Entry,
    replace_entry: Entry,
    search_revealer: Revealer,
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
    rpc::install_signal_handlers();
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
    let match_tag = TextTag::new(Some("block-match"));
    match_tag.set_background(Some("#c5e1a5"));
    buffer.tag_table().add(&match_tag);
    let search_settings = SearchSettings::new();
    search_settings.set_wrap_around(true);
    let search_ctx = SearchContext::new(&buffer, Some(&search_settings));
    search_ctx.set_highlight(true);
    let search_entry = Entry::builder()
        .placeholder_text("buscar")
        .hexpand(true)
        .build();
    let replace_entry = Entry::builder()
        .placeholder_text("substituir por")
        .hexpand(true)
        .build();
    let search_revealer = Revealer::new();
    search_revealer.set_reveal_child(false);

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
        .auto_indent(false)
        .indent_on_tab(false)
        .tab_width(4)
        .indent_width(4)
        .build();
    editor.set_wrap_mode(gtk::WrapMode::None);
    editor.set_accepts_tab(true);
    let bp_attrs = MarkAttributes::new();
    bp_attrs.set_pixbuf(&red_breakpoint_pixbuf());
    editor.set_mark_attributes("breakpoint", &bp_attrs, 10);

    let output = TextView::builder()
        .editable(false)
        .monospace(true)
        .cursor_visible(false)
        .wrap_mode(gtk::WrapMode::WordChar)
        .build();
    let err_tag = TextTag::new(Some("stderr"));
    err_tag.set_foreground(Some("#e53935"));
    output.buffer().tag_table().add(&err_tag);
    let error_link_tag = TextTag::new(Some("error-link"));
    error_link_tag.set_foreground(Some("#e53935"));
    error_link_tag.set_underline(gtk::pango::Underline::Single);
    output.buffer().tag_table().add(&error_link_tag);
    let args_entry = Entry::builder()
        .placeholder_text("argumentos (ex.: Thiago --ajuda)")
        .hexpand(true)
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
        args_entry,
        dirty: false,
        suppress_dirty: false,
        err_tag,
        error_link_tag,
        match_tag,
        search_settings,
        search_ctx,
        search_entry,
        replace_entry,
        search_revealer,
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

    let args_row = GtkBox::new(Orientation::Horizontal, 8);
    args_row.set_margin_start(8);
    args_row.set_margin_end(8);
    args_row.append(&Label::new(Some("argumentos:")));
    args_row.append(&ui.borrow().args_entry);

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
        .min_content_height(80)
        .hexpand(true)
        .child(&ui.borrow().output)
        .build();

    let debug_panel = make_debug_panel(&ui);
    const DEBUG_PANEL_WIDTH: i32 = 320;
    debug_panel.set_width_request(DEBUG_PANEL_WIDTH);
    debug_panel.set_hexpand(false);

    install_paned_css();

    let editor_col = Paned::new(Orientation::Vertical);
    editor_col.add_css_class("aula-paned");
    editor_col.set_wide_handle(false);
    editor_col.set_start_child(Some(&editor_scroll));
    editor_col.set_end_child(Some(&output_scroll));
    editor_col.set_resize_start_child(true);
    editor_col.set_resize_end_child(false);
    editor_col.set_shrink_start_child(false);
    editor_col.set_shrink_end_child(false);

    let main_split = Paned::new(Orientation::Horizontal);
    main_split.add_css_class("aula-paned");
    main_split.set_start_child(Some(&editor_col));
    main_split.set_end_child(Some(&debug_panel));
    main_split.set_resize_start_child(true);
    main_split.set_shrink_end_child(false);
    main_split.set_wide_handle(false);

    let body = Paned::new(Orientation::Horizontal);
    body.add_css_class("aula-paned");
    body.set_start_child(Some(&file_scroll));
    body.set_end_child(Some(&main_split));
    body.set_resize_end_child(true);
    body.set_position(200);

    let root = GtkBox::new(Orientation::Vertical, 6);
    let search_bar = GtkBox::new(Orientation::Horizontal, 6);
    search_bar.set_margin_start(8);
    search_bar.set_margin_end(8);
    search_bar.append(&Label::new(Some("buscar:")));
    search_bar.append(&ui.borrow().search_entry);
    let btn_find_next = Button::with_label("↓");
    let btn_find_prev = Button::with_label("↑");
    search_bar.append(&btn_find_next);
    search_bar.append(&btn_find_prev);
    search_bar.append(&Label::new(Some("trocar:")));
    search_bar.append(&ui.borrow().replace_entry);
    let btn_repl = Button::with_label("Trocar");
    let btn_repl_all = Button::with_label("Todas");
    let btn_find_close = Button::with_label("Fechar");
    search_bar.append(&btn_repl);
    search_bar.append(&btn_repl_all);
    search_bar.append(&btn_find_close);
    ui.borrow().search_revealer.set_child(Some(&search_bar));

    root.append(&toolbar);
    root.append(&args_row);
    root.append(&ui.borrow().search_revealer);
    root.append(&body);
    root.append(&ui.borrow().status);
    ui.borrow().status.set_margin_start(8);
    ui.borrow().status.set_margin_bottom(6);
    body.set_vexpand(true);

    win.set_child(Some(&root));

    {
        let ui_n = Rc::clone(&ui);
        btn_new.connect_clicked(move |_| {
            let ui = Rc::clone(&ui_n);
            confirm_discard(&ui_n, move || novo(&ui));
        });
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
                let ui_cb = Rc::clone(&ui_o);
                let p = path.clone();
                confirm_discard(&ui_o, move || abrir_no(&ui_cb, &p));
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
    {
        let ui_ch = Rc::clone(&ui);
        let buffer = ui.borrow().buffer.clone();
        buffer.connect_changed(move |_| {
            let Ok(mut u) = ui_ch.try_borrow_mut() else {
                return;
            };
            if u.suppress_dirty {
                return;
            }
            if !u.dirty {
                u.dirty = true;
                drop(u);
                refresh_title(&ui_ch);
            }
        });
    }
    {
        let ui_o = Rc::clone(&ui);
        let click = gtk::GestureClick::new();
        click.set_button(1);
        click.connect_pressed(move |_, _, x, y| {
            jump_from_output_click(&ui_o, x, y);
        });
        ui.borrow().output.add_controller(click);
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
            if ctrl && key == Key::f {
                show_search(&ui_k);
                return glib::Propagation::Stop;
            }
            if ctrl && key == Key::h {
                show_search(&ui_k);
                return glib::Propagation::Stop;
            }
            glib::Propagation::Proceed
        });
    }
    win.add_controller(keys);
    {
        let ui_s = Rc::clone(&ui);
        btn_find_next.connect_clicked(move |_| search_next(&ui_s));
    }
    {
        let ui_s = Rc::clone(&ui);
        btn_find_prev.connect_clicked(move |_| search_prev(&ui_s));
    }
    {
        let ui_s = Rc::clone(&ui);
        btn_repl.connect_clicked(move |_| search_replace_one(&ui_s));
    }
    {
        let ui_s = Rc::clone(&ui);
        btn_repl_all.connect_clicked(move |_| search_replace_all(&ui_s));
    }
    {
        let ui_s = Rc::clone(&ui);
        btn_find_close.connect_clicked(move |_| hide_search(&ui_s));
    }
    {
        let ui_s = Rc::clone(&ui);
        ui.borrow()
            .search_entry
            .connect_activate(move |_| search_next(&ui_s));
    }
    {
        let ui_s = Rc::clone(&ui);
        let search_keys = gtk::EventControllerKey::new();
        search_keys.connect_key_pressed(move |_, key, _, _| {
            if key == Key::Escape {
                hide_search(&ui_s);
                return glib::Propagation::Stop;
            }
            glib::Propagation::Proceed
        });
        ui.borrow().search_entry.add_controller(search_keys);
    }
    {
        let ui_tab = Rc::clone(&ui);
        let ui_untab = Rc::clone(&ui);
        let shortcuts = gtk::ShortcutController::new();
        shortcuts.set_propagation_phase(gtk::PropagationPhase::Capture);
        if let Some(trigger) = gtk::ShortcutTrigger::parse_string("Tab") {
            shortcuts.add_shortcut(gtk::Shortcut::new(
                Some(trigger),
                Some(gtk::CallbackAction::new(move |_, _| {
                    indent::indent_current_line(&ui_tab.borrow().buffer);
                    glib::Propagation::Stop
                })),
            ));
        }
        if let Some(trigger) = gtk::ShortcutTrigger::parse_string("<Shift>Tab") {
            shortcuts.add_shortcut(gtk::Shortcut::new(
                Some(trigger),
                Some(gtk::CallbackAction::new(move |_, _| {
                    indent::unindent_current_line(&ui_untab.borrow().buffer);
                    glib::Propagation::Stop
                })),
            ));
        }
        ui.borrow().editor.add_controller(shortcuts);
    }
    {
        #[derive(Clone, Copy)]
        enum IndentPending {
            None,
            Newline,
            Closer,
        }
        let pending = Rc::new(Cell::new(IndentPending::None));
        let buffer = ui.borrow().buffer.clone();
        {
            let pending = Rc::clone(&pending);
            buffer.connect_insert_text(move |_, _, text| {
                if text == "\n" {
                    pending.set(IndentPending::Newline);
                } else if text == "}" || text == "m" {
                    pending.set(IndentPending::Closer);
                }
            });
        }
        {
            let pending = Rc::clone(&pending);
            let buffer_ch = buffer.clone();
            buffer.connect_changed(move |_| {
                match pending.replace(IndentPending::None) {
                    IndentPending::None => {}
                    IndentPending::Newline => indent::indent_current_line(&buffer_ch),
                    IndentPending::Closer => {
                        let insert = buffer_ch.iter_at_mark(&buffer_ch.get_insert());
                        indent::indent_if_closer(&buffer_ch, insert.line());
                    }
                }
            });
        }
    }
    {
        let ui_m = Rc::clone(&ui);
        ui.borrow().buffer.connect_mark_set(move |_, _, mark| {
            if mark.name().as_deref() != Some("insert") {
                return;
            }
            highlight_matching(&ui_m);
        });
    }
    {
        let ui_c = Rc::clone(&ui);
        let click = gtk::GestureClick::new();
        click.set_button(1);
        click.connect_pressed(move |g, _, x, y| {
            if !g
                .current_event_state()
                .contains(gtk::gdk::ModifierType::CONTROL_MASK)
            {
                return;
            }
            open_import_at(&ui_c, x, y);
        });
        ui.borrow().editor.add_controller(click);
    }
    {
        let ui_t = Rc::clone(&ui);
        let right = gtk::GestureClick::new();
        right.set_button(3);
        right.connect_pressed(move |_, _, x, y| {
            tree_popup(&ui_t, x, y);
        });
        ui.borrow().tree.add_controller(right);
    }

    conectar(&ui);
    if !lang_ok {
        set_status(
            &ui,
            "aviso: gramática Expressa não carregou; o texto fica sem cores",
        );
    }
    set_source(
        &ui,
        "// F5 roda. F6 depura. Ctrl+F busca. Ctrl+clique em importe \"mod\".\n\nescreva(\"Olá, Expressa!\")\n",
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
        let editor_split = editor_col.clone();
        glib::timeout_add_local(std::time::Duration::from_millis(120), move || {
            let w = split.allocated_width();
            if w > DEBUG_PANEL_WIDTH + 400 {
                split.set_position(w - DEBUG_PANEL_WIDTH);
            }
            let h = editor_split.allocated_height();
            if h > OUTPUT_HEIGHT + 200 {
                editor_split.set_position(h - OUTPUT_HEIGHT);
            }
            glib::ControlFlow::Break
        });
    }
}

fn install_paned_css() {
    let css = gtk::CssProvider::new();
    css.load_from_data(
        "
        paned.aula-paned > separator {
            min-width: 4px;
            min-height: 4px;
            margin: 0;
            background-color: alpha(currentColor, 0.12);
        }
        paned.aula-paned > separator:hover {
            background-color: alpha(currentColor, 0.28);
        }
        ",
    );
    if let Some(display) = gtk::gdk::Display::default() {
        gtk::style_context_add_provider_for_display(
            &display,
            &css,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
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
    ui.borrow_mut().suppress_dirty = true;
    let buffer = ui.borrow().buffer.clone();
    buffer.set_text(text);
    {
        let mut u = ui.borrow_mut();
        u.suppress_dirty = false;
        u.dirty = false;
    }
    refresh_title(ui);
}

fn refresh_title(ui: &Rc<RefCell<Ui>>) {
    let u = ui.borrow();
    let name = filename(&u);
    let star = if u.dirty { "*" } else { "" };
    u.window
        .set_title(Some(&format!("{star}{name} — Expressa Aula")));
}

fn parse_run_args(s: &str) -> Vec<String> {
    s.split_whitespace().map(|w| w.to_string()).collect()
}

fn confirm_discard(ui: &Rc<RefCell<Ui>>, then: impl FnOnce() + 'static) {
    if !ui.borrow().dirty {
        then();
        return;
    }
    let window = ui.borrow().window.clone();
    let then = RefCell::new(Some(then));
    #[allow(deprecated)]
    let dlg = gtk::MessageDialog::builder()
        .transient_for(&window)
        .modal(true)
        .message_type(gtk::MessageType::Warning)
        .buttons(gtk::ButtonsType::None)
        .text("Há alterações não salvas")
        .secondary_text("Descartar e continuar?")
        .build();
    dlg.add_button("Cancelar", gtk::ResponseType::Cancel);
    dlg.add_button("Descartar", gtk::ResponseType::Accept);
    dlg.connect_response(move |d, resp| {
        d.close();
        if resp == gtk::ResponseType::Accept {
            if let Some(f) = then.borrow_mut().take() {
                f();
            }
        }
    });
    dlg.present();
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
            ui.borrow_mut().dirty = false;
            refresh_title(ui);
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
    let pb = gdk_pixbuf::Pixbuf::new(gdk_pixbuf::Colorspace::Rgb, true, 8, 12, 12).expect("pixbuf");
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
    append_output_tagged(ui, text, false);
}

fn append_stderr(ui: &Rc<RefCell<Ui>>, text: &str) {
    append_output_tagged(ui, text, true);
}

fn append_output_tagged(ui: &Rc<RefCell<Ui>>, text: &str, stderr: bool) {
    let u = ui.borrow();
    let buf = u.output.buffer();
    let mut start = buf.end_iter();
    let start_off = start.offset();
    buf.insert(&mut start, text);
    if stderr {
        let a = buf.iter_at_offset(start_off);
        let b = buf.end_iter();
        buf.apply_tag(&u.err_tag, &a, &b);
    }
}

fn append_error_link(ui: &Rc<RefCell<Ui>>, text: &str) {
    let u = ui.borrow();
    let buf = u.output.buffer();
    let mut start = buf.end_iter();
    let start_off = start.offset();
    buf.insert(&mut start, text);
    let a = buf.iter_at_offset(start_off);
    let b = buf.end_iter();
    buf.apply_tag(&u.error_link_tag, &a, &b);
}

fn jump_from_output_click(ui: &Rc<RefCell<Ui>>, x: f64, y: f64) {
    let u = ui.borrow();
    let (bx, by) = u
        .output
        .window_to_buffer_coords(gtk::TextWindowType::Text, x as i32, y as i32);
    let Some(iter) = u.output.iter_at_location(bx, by) else {
        return;
    };
    let line_idx = iter.line();
    let a = u.output.buffer().iter_at_line(line_idx).unwrap_or(iter);
    let mut b = a.clone();
    if !b.forward_to_line_end() {
        b = u.output.buffer().end_iter();
    }
    let line = u.output.buffer().text(&a, &b, false);
    drop(u);
    if let Some(n) = parse_error_line(&line) {
        ir_para_linha(ui, n);
    }
}

fn parse_error_line(line: &str) -> Option<u32> {
    // "erro: … em arquivo.lep:12" or "… em /path/x.lep:8"
    let em = line.rfind(" em ")?;
    let loc = line[em + 4..].trim();
    let colon = loc.rfind(':')?;
    loc[colon + 1..].trim().parse().ok()
}

fn rodar(ui: &Rc<RefCell<Ui>>, debug: bool) {
    set_output(ui, "");
    clear_debug_views(ui);
    let (student, name, src, bps, args) = {
        let u = ui.borrow();
        (
            student(&u),
            filename(&u),
            source_text(&u),
            u.breakpoints.iter().copied().collect::<Vec<_>>(),
            parse_run_args(&u.args_entry.text()),
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
            args,
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
                Ok(ExecEvent::Stderr(s)) => append_stderr(&ui_ev, &s),
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
                        append_error_link(
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

fn show_search(ui: &Rc<RefCell<Ui>>) {
    let u = ui.borrow();
    u.search_revealer.set_reveal_child(true);
    u.search_entry.grab_focus();
}

fn hide_search(ui: &Rc<RefCell<Ui>>) {
    ui.borrow().search_revealer.set_reveal_child(false);
    ui.borrow().editor.grab_focus();
}

fn search_apply(ui: &Rc<RefCell<Ui>>) {
    let u = ui.borrow();
    let q = u.search_entry.text();
    if q.is_empty() {
        u.search_settings.set_search_text(None::<&str>);
    } else {
        u.search_settings.set_search_text(Some(q.as_str()));
    }
}

fn search_next(ui: &Rc<RefCell<Ui>>) {
    search_apply(ui);
    let u = ui.borrow();
    let insert = u.buffer.iter_at_mark(&u.buffer.get_insert());
    let from = if let Some((_, end)) = u.buffer.selection_bounds() {
        end
    } else {
        insert
    };
    if let Some((a, b, _)) = u.search_ctx.forward(&from) {
        u.buffer.select_range(&a, &b);
        u.editor
            .scroll_to_iter(&mut a.clone(), 0.2, false, 0.0, 0.0);
    } else {
        drop(u);
        set_status(ui, "não encontrado");
    }
}

fn search_prev(ui: &Rc<RefCell<Ui>>) {
    search_apply(ui);
    let u = ui.borrow();
    let insert = u.buffer.iter_at_mark(&u.buffer.get_insert());
    if let Some((a, b, _)) = u.search_ctx.backward(&insert) {
        u.buffer.select_range(&a, &b);
        u.editor
            .scroll_to_iter(&mut a.clone(), 0.2, false, 0.0, 0.0);
    } else {
        drop(u);
        set_status(ui, "não encontrado");
    }
}

fn search_replace_one(ui: &Rc<RefCell<Ui>>) {
    search_apply(ui);
    let u = ui.borrow();
    let repl = u.replace_entry.text().to_string();
    if let Some((mut a, mut b)) = u.buffer.selection_bounds() {
        if u.search_ctx.replace(&mut a, &mut b, &repl).is_ok() {
            drop(u);
            search_next(ui);
            return;
        }
    }
    drop(u);
    search_next(ui);
}

fn search_replace_all(ui: &Rc<RefCell<Ui>>) {
    search_apply(ui);
    let u = ui.borrow();
    let repl = u.replace_entry.text().to_string();
    match u.search_ctx.replace_all(&repl) {
        Ok(()) => set_status(ui, "substituições feitas"),
        Err(e) => set_status(ui, &format!("substituir: {e}")),
    }
}

fn highlight_matching(ui: &Rc<RefCell<Ui>>) {
    let Ok(u) = ui.try_borrow() else {
        return;
    };
    let (s, e) = u.buffer.bounds();
    u.buffer.remove_tag(&u.match_tag, &s, &e);
    let Some((a, b)) = indent::matching_block(&u.buffer) else {
        return;
    };
    let tag = u.match_tag.clone();
    let buf = u.buffer.clone();
    drop(u);
    for line in [a, b] {
        if let Some(start) = buf.iter_at_line(line) {
            let mut end = start.clone();
            if !end.forward_to_line_end() {
                end = buf.end_iter();
            }
            buf.apply_tag(&tag, &start, &end);
        }
    }
}

fn join_path(dir: &str, name: &str) -> String {
    if dir.is_empty() {
        name.to_string()
    } else {
        format!("{dir}/{name}")
    }
}

fn parent_dir(path: &str, is_dir: bool) -> String {
    if is_dir {
        return path.to_string();
    }
    match path.rfind('/') {
        Some(i) => path[..i].to_string(),
        None => String::new(),
    }
}

fn selected_tree_entry(ui: &Rc<RefCell<Ui>>) -> (String, bool) {
    let u = ui.borrow();
    let sel = u.tree.selection();
    if let Some((_, iter)) = sel.selected() {
        let model = u.store.upcast_ref::<gtk::TreeModel>();
        let path: String = model.get(&iter, 1);
        let is_dir: bool = model.get(&iter, 2);
        return (path, is_dir);
    }
    (String::new(), true)
}

fn tree_popup(ui: &Rc<RefCell<Ui>>, x: f64, y: f64) {
    let tree = ui.borrow().tree.clone();
    if let Some((Some(path), _, _, _)) = tree.path_at_pos(x as i32, y as i32) {
        tree.selection().select_path(&path);
    }
    let (rel, is_dir) = selected_tree_entry(ui);
    let window = ui.borrow().window.clone();
    let pop = gtk::Popover::new();
    pop.set_parent(&tree);
    pop.set_has_arrow(false);
    pop.set_pointing_to(Some(&gtk::gdk::Rectangle::new(x as i32, y as i32, 1, 1)));
    let box_ = GtkBox::new(Orientation::Vertical, 0);
    let add_item = |box_: &GtkBox, label: &str, action: Box<dyn FnOnce() + 'static>| {
        let btn = Button::with_label(label);
        btn.set_has_frame(false);
        let pop_c = pop.clone();
        let action = RefCell::new(Some(action));
        btn.connect_clicked(move |_| {
            pop_c.popdown();
            if let Some(f) = action.borrow_mut().take() {
                f();
            }
        });
        box_.append(&btn);
    };
    let ui_a = Rc::clone(ui);
    let dir = parent_dir(&rel, is_dir);
    add_item(
        &box_,
        "Novo arquivo",
        Box::new({
            let ui = Rc::clone(&ui_a);
            let dir = dir.clone();
            let window = window.clone();
            move || {
                if let Some(name) = perguntar_linha(&window, "Nome do arquivo:") {
                    let name = if name.ends_with(".lep") {
                        name
                    } else if name.contains('.') {
                        name
                    } else {
                        format!("{name}.lep")
                    };
                    let path = join_path(&dir, &name);
                    let stu = student(&ui.borrow());
                    match ui
                        .borrow()
                        .rpc
                        .write_file(&stu, &path, "escreva(\"Olá\")\n")
                    {
                        Ok(()) => {
                            conectar(&ui);
                            set_status(&ui, &format!("criado {path}"));
                        }
                        Err(e) => set_status(&ui, &format!("criar: {e}")),
                    }
                }
            }
        }),
    );
    add_item(
        &box_,
        "Nova pasta",
        Box::new({
            let ui = Rc::clone(&ui_a);
            let dir = dir.clone();
            let window = window.clone();
            move || {
                if let Some(name) = perguntar_linha(&window, "Nome da pasta:") {
                    if name.is_empty() {
                        return;
                    }
                    let path = join_path(&dir, &name);
                    let stu = student(&ui.borrow());
                    match ui.borrow().rpc.mkdir(&stu, &path) {
                        Ok(()) => {
                            conectar(&ui);
                            set_status(&ui, &format!("pasta {path}"));
                        }
                        Err(e) => set_status(&ui, &format!("pasta: {e}")),
                    }
                }
            }
        }),
    );
    if !rel.is_empty() {
        add_item(
            &box_,
            "Renomear",
            Box::new({
                let ui = Rc::clone(&ui_a);
                let rel = rel.clone();
                let window = window.clone();
                move || {
                    let base = rel.rsplit('/').next().unwrap_or(&rel).to_string();
                    if let Some(name) = perguntar_linha(&window, &format!("Renomear `{base}`:")) {
                        if name.is_empty() {
                            return;
                        }
                        let to = join_path(&parent_dir(&rel, false), &name);
                        let stu = student(&ui.borrow());
                        match ui.borrow().rpc.rename(&stu, &rel, &to) {
                            Ok(()) => {
                                if ui.borrow().filename.text() == rel.as_str() {
                                    ui.borrow().filename.set_text(&to);
                                }
                                conectar(&ui);
                                set_status(&ui, &format!("{rel} → {to}"));
                            }
                            Err(e) => set_status(&ui, &format!("renomear: {e}")),
                        }
                    }
                }
            }),
        );
        add_item(
            &box_,
            "Apagar",
            Box::new({
                let ui = Rc::clone(&ui_a);
                let rel = rel.clone();
                let window = window.clone();
                move || {
                    let then = {
                        let ui = Rc::clone(&ui);
                        let rel = rel.clone();
                        move || {
                            let stu = student(&ui.borrow());
                            match ui.borrow().rpc.delete_file(&stu, &rel) {
                                Ok(()) => {
                                    conectar(&ui);
                                    set_status(&ui, &format!("apagado {rel}"));
                                }
                                Err(e) => set_status(&ui, &format!("apagar: {e}")),
                            }
                        }
                    };
                    let then = RefCell::new(Some(then));
                    #[allow(deprecated)]
                    let dlg = gtk::MessageDialog::builder()
                        .transient_for(&window)
                        .modal(true)
                        .message_type(gtk::MessageType::Warning)
                        .buttons(gtk::ButtonsType::None)
                        .text(&format!("Apagar `{rel}`?"))
                        .build();
                    dlg.add_button("Cancelar", gtk::ResponseType::Cancel);
                    dlg.add_button("Apagar", gtk::ResponseType::Accept);
                    dlg.connect_response(move |d, resp| {
                        d.close();
                        if resp == gtk::ResponseType::Accept {
                            if let Some(f) = then.borrow_mut().take() {
                                f();
                            }
                        }
                    });
                    dlg.present();
                }
            }),
        );
    }
    pop.set_child(Some(&box_));
    pop.popup();
}

fn open_import_at(ui: &Rc<RefCell<Ui>>, x: f64, y: f64) {
    let line_txt = {
        let u = ui.borrow();
        let (_, by) =
            u.editor
                .window_to_buffer_coords(gtk::TextWindowType::Text, x as i32, y as i32);
        let Some(iter) = u.editor.iter_at_location(0, by) else {
            return;
        };
        let line = iter.line();
        let Some(a) = u.buffer.iter_at_line(line) else {
            return;
        };
        let mut b = a.clone();
        if !b.forward_to_line_end() {
            b = u.buffer.end_iter();
        }
        u.buffer.text(&a, &b, false).to_string()
    };
    let Some(mod_name) = parse_importe(&line_txt) else {
        return;
    };
    let current = filename(&ui.borrow());
    let mut candidates = Vec::new();
    let with_lep = if mod_name.ends_with(".lep") {
        mod_name.clone()
    } else {
        format!("{mod_name}.lep")
    };
    if let Some(slash) = current.rfind('/') {
        candidates.push(format!("{}/{}", &current[..slash], with_lep));
    }
    candidates.push(format!("lib/{with_lep}"));
    candidates.push(with_lep);
    let stu = student(&ui.borrow());
    for rel in candidates {
        if let Ok(src) = ui.borrow().rpc.read_file(&stu, &rel) {
            confirm_discard(ui, {
                let ui = Rc::clone(ui);
                let rel = rel.clone();
                let src = src.clone();
                move || {
                    ui.borrow().filename.set_text(&rel);
                    ui.borrow_mut().breakpoints.clear();
                    set_source(&ui, &src);
                    paint_breakpoints(&ui);
                    set_status(&ui, &format!("aberto {rel}"));
                }
            });
            return;
        }
    }
    set_status(ui, &format!("importe: não achei `{mod_name}`"));
}

fn parse_importe(line: &str) -> Option<String> {
    let i = line.find("importe")?;
    let rest = line[i + "importe".len()..].trim_start();
    let rest = rest.strip_prefix('"')?;
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
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
