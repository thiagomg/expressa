mod assist;
mod console;
mod docs;
mod editing;
mod exec;
mod indent;
mod lang;
mod prefs;
mod rpc;

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::{Rc, Weak};

use gtk::gdk::Key;
use gtk::glib;
use gtk::glib::prelude::*;
use gtk::prelude::*;
use gtk::{
    Application, ApplicationWindow, Box as GtkBox, Button, CellRendererText, Entry, Label,
    Orientation, Paned, Revealer, ScrolledWindow, ToggleButton, TreeStore, TreeView,
    TreeViewColumn,
};
use sourceview5::prelude::*;
use sourceview5::{LanguageManager, SearchSettings, StyleSchemeManager};

use assist::{Help, Target, ViewHelp};
use console::Console;
use docs::Doc;
use exec::{ExecEvent, Workspace};
use prefs::{FontCss, Prefs};
use rpc::{Rpc, ensure_server};

const APP_ID: &str = "dev.expressa.aula";
const DEFAULT_URL: &str = "http://127.0.0.1:50051";
const WELCOME: &str = "// F5 roda. F6 depura. Ctrl+F busca. F1 ajuda. Ctrl+clique vai para a definição.\n\nescreva(\"Olá, Expressa!\")\n";

/// A program running or being debugged.
struct Run {
    debug_cmd: std::sync::mpsc::Sender<String>,
    /// The local copy: breakpoints set while running name files there.
    ws_dir: PathBuf,
}

struct Ui {
    rpc: Rpc,
    student: Entry,
    /// Project whose files the tabs show (the student entry may be edited
    /// before Conectar).
    project: String,
    tree: TreeView,
    store: TreeStore,
    project_files: Vec<String>,
    notebook: gtk::Notebook,
    docs: Vec<(Rc<Doc>, Rc<ViewHelp>)>,
    untitled_count: u32,
    /// Server files read for help on imports (`None` = missing).
    file_cache: RefCell<HashMap<String, Option<String>>>,
    console: Rc<Console>,
    args_entry: Entry,
    search_settings: SearchSettings,
    search_entry: Entry,
    replace_entry: Entry,
    search_revealer: Revealer,
    status: Label,
    diag_label: Label,
    window: ApplicationWindow,
    run: Option<Run>,
    watches: Vec<String>,
    vars_store: TreeStore,
    stack_store: TreeStore,
    watch_store: TreeStore,
    watch_entry: Entry,
    btn_continue: Button,
    btn_next: Button,
    btn_step: Button,
    btn_out: Button,
    btn_stop: Button,
    btn_dark: ToggleButton,
    /// Kept alive so GtkSourceView does not drop the Expressa language spec.
    languages: LanguageManager,
    schemes: StyleSchemeManager,
    prefs: Prefs,
    font_css: FontCss,
    help: Option<Rc<Help>>,
    /// Unsaved changes were handled: the window may close now.
    closing: bool,
}

type UiRc = Rc<RefCell<Ui>>;

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

    // ibus + GTK4 leaves the dead key (´ ~ `) stuck as preedit after composing
    // á/ã. GTK's built-in input method composes dead keys itself.
    // GTK_IM_MODULE in the environment still takes precedence over this.
    if let Some(settings) = gtk::Settings::default() {
        settings.set_gtk_im_module(Some("gtk-im-context-simple"));
    }

    let prefs = Prefs::load();
    prefs::apply_widget_theme(prefs.dark);
    let font_css = FontCss::install();
    font_css.set_size(prefs.font_size);

    let student = Entry::builder()
        .text("local")
        .placeholder_text("aluno")
        .width_chars(10)
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
    let schemes = StyleSchemeManager::default();
    if let Some(path) = data_dir().join("styles").to_str() {
        schemes.prepend_search_path(path);
    }
    let search_settings = SearchSettings::new();
    search_settings.set_wrap_around(true);
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
    let btn_dark = ToggleButton::with_label("Escuro");
    btn_dark.set_active(prefs.dark);
    btn_dark.set_tooltip_text(Some("Tema escuro"));

    let notebook = gtk::Notebook::new();
    notebook.set_scrollable(true);
    notebook.set_vexpand(true);
    notebook.set_hexpand(true);

    let console = Rc::new(Console::new());
    let args_entry = Entry::builder()
        .placeholder_text("argumentos (ex.: Thiago --ajuda)")
        .hexpand(true)
        .build();
    let status = Label::new(Some("conectado a 127.0.0.1:50051"));
    status.set_xalign(0.0);
    status.set_hexpand(true);
    status.add_css_class("dim-label");
    let diag_label = Label::new(None);
    diag_label.set_xalign(1.0);
    diag_label.set_ellipsize(gtk::pango::EllipsizeMode::End);
    diag_label.set_max_width_chars(80);

    let win = ApplicationWindow::builder()
        .application(app)
        .title("Expressa Aula")
        .default_width(1400)
        .default_height(900)
        .build();

    let ui: UiRc = Rc::new(RefCell::new(Ui {
        rpc,
        project: student.text().to_string(),
        student,
        tree,
        store,
        project_files: Vec::new(),
        notebook,
        docs: Vec::new(),
        untitled_count: 0,
        file_cache: RefCell::new(HashMap::new()),
        console,
        args_entry,
        search_settings,
        search_entry,
        replace_entry,
        search_revealer,
        status,
        diag_label,
        window: win.clone(),
        run: None,
        watches: Vec::new(),
        vars_store,
        stack_store,
        watch_store,
        watch_entry,
        btn_continue,
        btn_next,
        btn_step,
        btn_out,
        btn_stop,
        btn_dark,
        languages,
        schemes,
        prefs,
        font_css,
        help: None,
        closing: false,
    }));
    let help = make_help(&ui);
    ui.borrow_mut().help = Some(help);

    let toolbar = GtkBox::new(Orientation::Horizontal, 8);
    toolbar.set_margin_start(8);
    toolbar.set_margin_end(8);
    toolbar.set_margin_top(8);
    let btn_new = Button::with_label("Novo");
    btn_new.set_tooltip_text(Some("Arquivo novo em outra aba (Ctrl+N)"));
    let btn_save = Button::with_label("Salvar");
    btn_save.set_tooltip_text(Some("Salvar a aba atual (Ctrl+S)"));
    let btn_run = Button::with_label("Rodar");
    btn_run.set_tooltip_text(Some("Rodar a aba atual (F5)"));
    let btn_debug = Button::with_label("Depurar");
    btn_debug.set_tooltip_text(Some("Depurar a aba atual (F6)"));
    let btn_bp = Button::with_label("Ponto");
    btn_bp.set_tooltip_text(Some("Ponto de parada na linha (F9)"));
    let btn_refresh = Button::with_label("Conectar");
    let btn_smaller = Button::with_label("A−");
    btn_smaller.set_tooltip_text(Some("Letra menor (Ctrl+-)"));
    let btn_bigger = Button::with_label("A+");
    btn_bigger.set_tooltip_text(Some("Letra maior (Ctrl+=)"));
    for b in [&btn_new, &btn_save, &btn_run, &btn_debug, &btn_bp] {
        toolbar.append(b);
    }
    {
        let u = ui.borrow();
        for b in [
            &u.btn_continue,
            &u.btn_next,
            &u.btn_step,
            &u.btn_out,
            &u.btn_stop,
        ] {
            toolbar.append(b);
        }
        toolbar.append(&Label::new(Some("projeto:")));
        toolbar.append(&u.student);
    }
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
    args_row.append(&btn_smaller);
    args_row.append(&btn_bigger);
    args_row.append(&ui.borrow().btn_dark);

    const FILES_WIDTH: i32 = 240;
    let file_scroll = ScrolledWindow::builder()
        .min_content_width(FILES_WIDTH)
        .child(&ui.borrow().tree)
        .build();
    const OUTPUT_HEIGHT: i32 = 160;
    let output_scroll = ScrolledWindow::builder()
        .min_content_height(80)
        .hexpand(true)
        .child(&ui.borrow().console.view)
        .build();

    let debug_panel = make_debug_panel(&ui);
    const DEBUG_PANEL_WIDTH: i32 = 320;
    debug_panel.set_width_request(DEBUG_PANEL_WIDTH);
    debug_panel.set_hexpand(false);

    install_css();

    let editor_col = Paned::new(Orientation::Vertical);
    editor_col.add_css_class("aula-paned");
    editor_col.set_wide_handle(false);
    editor_col.set_start_child(Some(&ui.borrow().notebook));
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
    main_split.set_resize_end_child(false);
    main_split.set_shrink_end_child(false);
    main_split.set_wide_handle(false);

    let body = Paned::new(Orientation::Horizontal);
    body.add_css_class("aula-paned");
    body.set_start_child(Some(&file_scroll));
    body.set_end_child(Some(&main_split));
    // The file list keeps its width; the editor takes what the window gains.
    body.set_resize_start_child(false);
    body.set_shrink_start_child(false);
    body.set_resize_end_child(true);
    body.set_position(FILES_WIDTH);

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

    let status_row = GtkBox::new(Orientation::Horizontal, 12);
    status_row.set_margin_start(8);
    status_row.set_margin_end(8);
    status_row.set_margin_bottom(6);
    status_row.append(&ui.borrow().status);
    status_row.append(&ui.borrow().diag_label);

    root.append(&toolbar);
    root.append(&args_row);
    root.append(&ui.borrow().search_revealer);
    root.append(&body);
    root.append(&status_row);
    body.set_vexpand(true);
    win.set_child(Some(&root));

    connect(&btn_new, &ui, new_untitled_tab);
    connect(&btn_save, &ui, |ui| {
        if let Some(d) = current_doc(ui) {
            save_doc(ui, &d);
        }
    });
    connect(&btn_run, &ui, |ui| rodar(ui, false));
    connect(&btn_debug, &ui, |ui| rodar(ui, true));
    connect(&btn_bp, &ui, toggle_breakpoint_here);
    connect(&btn_refresh, &ui, conectar);
    connect(&btn_smaller, &ui, |ui| zoom(ui, -1));
    connect(&btn_bigger, &ui, |ui| zoom(ui, 1));
    {
        let u = ui.borrow();
        connect(&u.btn_continue, &ui, |ui| send_debug_cmd(ui, "continuar"));
        connect(&u.btn_next, &ui, |ui| send_debug_cmd(ui, "proximo"));
        connect(&u.btn_step, &ui, |ui| send_debug_cmd(ui, "entrar"));
        connect(&u.btn_out, &ui, |ui| send_debug_cmd(ui, "sair"));
        connect(&u.btn_stop, &ui, |ui| send_debug_cmd(ui, "terminar"));
    }
    {
        let ui_d = Rc::clone(&ui);
        ui.borrow()
            .btn_dark
            .connect_toggled(move |b| set_dark(&ui_d, b.is_active()));
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
            .connect_row_activated(move |_, path, _col| abrir_no(&ui_o, path));
    }
    {
        let ui_t = Rc::clone(&ui);
        ui.borrow().notebook.connect_switch_page(move |_, _, _| {
            let ui_t = Rc::clone(&ui_t);
            // After the switch completes (current_page is still the old one).
            glib::idle_add_local_once(move || {
                refresh_title(&ui_t);
                refresh_diag_label(&ui_t);
            });
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
        // Click on `erro: … em arquivo:linha`; clicks while reading keep the
        // cursor in the answer.
        let ui_o = Rc::clone(&ui);
        let click = gtk::GestureClick::new();
        click.set_button(1);
        click.connect_released(move |_, _, x, y| {
            let console = Rc::clone(&ui_o.borrow().console);
            if console.is_reading() {
                console.keep_cursor_in_input();
                return;
            }
            if let Some((file, line)) = console
                .line_at(x, y)
                .and_then(|l| console::parse_error_location(&l))
            {
                goto_file_line(&ui_o, &file, line);
            }
        });
        ui.borrow().console.view.add_controller(click);
    }
    {
        // Enter in the output pane answers `leia`.
        let console = Rc::clone(&ui.borrow().console);
        let keys = gtk::ShortcutController::new();
        keys.set_propagation_phase(gtk::PropagationPhase::Capture);
        for accel in ["Return", "KP_Enter"] {
            let c = Rc::clone(&console);
            add_shortcut_if(&keys, accel, move || c.commit_input());
        }
        console.view.add_controller(keys);
    }

    {
        // Do not use EventControllerKey on the window (defaults to Capture
        // and eats dead keys). Function keys go in their own Capture
        // controller: GtkPaned binds F6/F8 (cycle focus) and would swallow
        // them in Bubble. A ShortcutController only matches its own keys, so
        // dead keys still reach the editor.
        let shortcuts = gtk::ShortcutController::new();
        shortcuts.set_propagation_phase(gtk::PropagationPhase::Bubble);
        let fkeys = gtk::ShortcutController::new();
        fkeys.set_propagation_phase(gtk::PropagationPhase::Capture);
        let k = |c: &gtk::ShortcutController, accel: &str, f: fn(&UiRc)| {
            let ui = Rc::clone(&ui);
            add_shortcut(c, accel, move || f(&ui));
        };
        k(&fkeys, "F5", |ui| rodar(ui, false));
        k(&fkeys, "F6", |ui| rodar(ui, true));
        k(&fkeys, "F9", toggle_breakpoint_here);
        k(&fkeys, "F10", |ui| send_debug_cmd(ui, "proximo"));
        k(&fkeys, "F11", |ui| send_debug_cmd(ui, "entrar"));
        k(&fkeys, "<Shift>F11", |ui| send_debug_cmd(ui, "sair"));
        add_shortcut(&fkeys, "F7", {
            let b = btn_files.clone();
            move || b.set_active(!b.is_active())
        });
        add_shortcut(&fkeys, "F8", {
            let b = btn_panel.clone();
            move || b.set_active(!b.is_active())
        });
        k(&shortcuts, "<Primary>s", |ui| {
            if let Some(d) = current_doc(ui) {
                save_doc(ui, &d);
            }
        });
        k(&shortcuts, "<Primary>n", new_untitled_tab);
        k(&shortcuts, "<Primary>w", |ui| {
            if let Some(d) = current_doc(ui) {
                close_doc(ui, &d);
            }
        });
        k(&shortcuts, "<Primary>f", show_search);
        k(&shortcuts, "<Primary>h", show_search);
        for accel in ["<Primary>equal", "<Primary>plus", "<Primary>KP_Add"] {
            k(&fkeys, accel, |ui| zoom(ui, 1));
        }
        for accel in ["<Primary>minus", "<Primary>KP_Subtract"] {
            k(&fkeys, accel, |ui| zoom(ui, -1));
        }
        k(&fkeys, "<Primary>0", |ui| zoom(ui, 0));
        win.add_controller(fkeys);
        win.add_controller(shortcuts);
    }
    connect(&btn_find_next, &ui, search_next);
    connect(&btn_find_prev, &ui, search_prev);
    connect(&btn_repl, &ui, search_replace_one);
    connect(&btn_repl_all, &ui, search_replace_all);
    connect(&btn_find_close, &ui, hide_search);
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
        let ui_t = Rc::clone(&ui);
        let right = gtk::GestureClick::new();
        right.set_button(3);
        right.connect_pressed(move |_, _, x, y| tree_popup(&ui_t, x, y));
        ui.borrow().tree.add_controller(right);
    }
    {
        let ui_close = Rc::clone(&ui);
        win.connect_close_request(move |w| {
            if ui_close.borrow().closing {
                ui_close.borrow_mut().rpc.shutdown_spawned_server();
                return glib::Propagation::Proceed;
            }
            let dirty: Vec<Rc<Doc>> = dirty_docs(&ui_close);
            if dirty.is_empty() {
                ui_close.borrow_mut().rpc.shutdown_spawned_server();
                return glib::Propagation::Proceed;
            }
            let ui_c = Rc::clone(&ui_close);
            let w = w.clone();
            confirm_unsaved(&ui_close, dirty, move || {
                ui_c.borrow_mut().closing = true;
                w.close();
            });
            glib::Propagation::Stop
        });
    }

    conectar(&ui);
    let doc = open_doc(&ui, None, WELCOME);
    doc.mark_saved();
    apply_theme(&ui);
    if !lang_ok {
        set_status(
            &ui,
            "aviso: gramática Expressa não carregou; o texto fica sem cores",
        );
    }
    win.maximize();
    win.present();
    {
        // Place the splits once the window has its final size (the window
        // manager may maximize it a moment after it appears): re-apply
        // while the size changes, for up to 2 s.
        let body = body.clone();
        let split = main_split.clone();
        let editor_split = editor_col.clone();
        let last = std::cell::Cell::new((0, 0));
        let ticks = std::cell::Cell::new(0);
        glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
            let size = (split.allocated_width(), editor_split.allocated_height());
            if size != last.get() {
                last.set(size);
                body.set_position(FILES_WIDTH);
                if size.0 > DEBUG_PANEL_WIDTH + 400 {
                    split.set_position(size.0 - DEBUG_PANEL_WIDTH);
                }
                if size.1 > OUTPUT_HEIGHT + 200 {
                    editor_split.set_position(size.1 - OUTPUT_HEIGHT);
                }
            }
            ticks.set(ticks.get() + 1);
            if ticks.get() >= 20 {
                glib::ControlFlow::Break
            } else {
                glib::ControlFlow::Continue
            }
        });
    }
}

fn connect(button: &impl IsA<Button>, ui: &UiRc, f: impl Fn(&UiRc) + 'static) {
    let ui = Rc::clone(ui);
    button.connect_clicked(move |_| f(&ui));
}

fn add_shortcut(controller: &gtk::ShortcutController, accel: &str, f: impl Fn() + 'static) {
    add_shortcut_if(controller, accel, move || {
        f();
        true
    });
}

/// Like `add_shortcut`, but `f` returning false lets the key through.
fn add_shortcut_if(
    controller: &gtk::ShortcutController,
    accel: &str,
    f: impl Fn() -> bool + 'static,
) {
    let Some(trigger) = gtk::ShortcutTrigger::parse_string(accel) else {
        return;
    };
    controller.add_shortcut(gtk::Shortcut::new(
        Some(trigger),
        Some(gtk::CallbackAction::new(move |_, _| {
            if f() {
                glib::Propagation::Stop
            } else {
                glib::Propagation::Proceed
            }
        })),
    ));
}

fn install_css() {
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
        popover.aula-signature > contents {
            padding: 4px 8px;
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
    if let Some(path) = data_dir().join("language-specs").to_str() {
        lm.prepend_search_path(path);
    }
    lm
}

fn set_status(ui: &UiRc, text: &str) {
    if let Ok(u) = ui.try_borrow() {
        u.status.set_text(text);
    }
}

fn parse_run_args(s: &str) -> Vec<String> {
    s.split_whitespace().map(|w| w.to_string()).collect()
}

// ── Help backed by the open tabs and the server ───────────────────────────

fn make_help(ui: &UiRc) -> Rc<Help> {
    let w1: Weak<RefCell<Ui>> = Rc::downgrade(ui);
    let w2 = w1.clone();
    let w3 = w1.clone();
    Rc::new(Help {
        file_text: Box::new(move |path| {
            let ui = w1.upgrade()?;
            let u = ui.try_borrow().ok()?;
            if let Some((d, _)) = u.docs.iter().find(|(d, _)| d.name() == path) {
                return Some(d.text());
            }
            if let Some(hit) = u.file_cache.borrow().get(path) {
                return hit.clone();
            }
            let text = u.rpc.read_file(&u.project, path).ok();
            u.file_cache
                .borrow_mut()
                .insert(path.to_string(), text.clone());
            text
        }),
        project_files: Box::new(move || {
            w2.upgrade()
                .and_then(|ui| ui.try_borrow().ok().map(|u| u.project_files.clone()))
                .unwrap_or_default()
        }),
        path_of: Box::new(move |buffer| {
            let ui = w3.upgrade()?;
            let u = ui.try_borrow().ok()?;
            u.docs
                .iter()
                .find(|(d, _)| &d.buffer == buffer)
                .map(|(d, _)| d.name())
        }),
    })
}

// ── Tabs ───────────────────────────────────────────────────────────────────

fn current_doc(ui: &UiRc) -> Option<Rc<Doc>> {
    let u = ui.try_borrow().ok()?;
    let page = u.notebook.nth_page(u.notebook.current_page())?;
    u.docs
        .iter()
        .find(|(d, _)| d.page.upcast_ref::<gtk::Widget>() == &page)
        .map(|(d, _)| Rc::clone(d))
}

fn doc_by_name(ui: &UiRc, name: &str) -> Option<Rc<Doc>> {
    ui.borrow()
        .docs
        .iter()
        .find(|(d, _)| d.name() == name)
        .map(|(d, _)| Rc::clone(d))
}

fn view_help(ui: &UiRc, doc: &Rc<Doc>) -> Option<Rc<ViewHelp>> {
    ui.borrow()
        .docs
        .iter()
        .find(|(d, _)| Rc::ptr_eq(d, doc))
        .map(|(_, v)| Rc::clone(v))
}

fn all_docs(ui: &UiRc) -> Vec<Rc<Doc>> {
    ui.borrow().docs.iter().map(|(d, _)| Rc::clone(d)).collect()
}

fn dirty_docs(ui: &UiRc) -> Vec<Rc<Doc>> {
    all_docs(ui).into_iter().filter(|d| d.is_dirty()).collect()
}

fn switch_to(ui: &UiRc, doc: &Rc<Doc>) {
    let nb = ui.borrow().notebook.clone();
    if let Some(n) = nb.page_num(&doc.page) {
        nb.set_current_page(Some(n));
    }
    doc.view.grab_focus();
    refresh_title(ui);
}

/// New tab for `path` (or untitled) with `text`, wired to the editor.
fn open_doc(ui: &UiRc, path: Option<String>, text: &str) -> Rc<Doc> {
    let untitled = {
        let mut u = ui.borrow_mut();
        u.untitled_count += 1;
        if u.untitled_count == 1 {
            "sem-titulo.lep".to_string()
        } else {
            format!("sem-titulo-{}.lep", u.untitled_count)
        }
    };
    let doc = {
        let u = ui.borrow();
        Doc::new(path, untitled, text, &u.languages, &u.search_settings)
    };
    {
        let weak = Rc::downgrade(ui);
        *doc.on_diag.borrow_mut() = Some(Box::new(move |_| {
            if let Some(ui) = weak.upgrade() {
                refresh_diag_label(&ui);
            }
        }));
    }
    {
        let weak = Rc::downgrade(ui);
        doc.buffer.connect_modified_changed(move |_| {
            if let Some(ui) = weak.upgrade() {
                refresh_title(&ui);
            }
        });
    }
    let help = ui.borrow().help.clone().expect("help");
    let vh = {
        let weak = Rc::downgrade(&doc);
        assist::attach(&doc.view, &help, move |_, offset| {
            weak.upgrade()
                .and_then(|d| d.diag_at(offset))
                .map(|d| d.message)
        })
    };
    attach_editor_input(ui, &doc, &vh);
    {
        let ui_c = Rc::clone(ui);
        let weak = Rc::downgrade(&doc);
        doc.close_button.connect_clicked(move |_| {
            if let Some(d) = weak.upgrade() {
                close_doc(&ui_c, &d);
            }
        });
    }
    {
        let (dark, scheme) = theme_scheme(ui);
        doc.set_theme(scheme.as_ref(), dark);
    }
    ui.borrow_mut().docs.push((Rc::clone(&doc), vh));
    let nb = ui.borrow().notebook.clone();
    let n = nb.append_page(&doc.page, Some(&doc.tab));
    nb.set_tab_reorderable(&doc.page, true);
    nb.set_current_page(Some(n));
    doc.view.grab_focus();
    refresh_title(ui);
    doc
}

/// Mouse and keys of one editor view.
fn attach_editor_input(ui: &UiRc, doc: &Rc<Doc>, vh: &Rc<ViewHelp>) {
    let view = doc.view.clone();
    {
        // Click left of the text (line numbers): breakpoint. Ctrl+click on a
        // name: go to its definition.
        let ui_g = Rc::clone(ui);
        let weak = Rc::downgrade(doc);
        // The line-number gutter is a child widget that takes the click:
        // look at it first (Capture), without claiming it.
        let gutter = gtk::GestureClick::new();
        gutter.set_button(1);
        gutter.set_propagation_phase(gtk::PropagationPhase::Capture);
        {
            let ui_g = Rc::clone(&ui_g);
            let weak = weak.clone();
            gutter.connect_pressed(move |_, _, x, y| {
                let Some(doc) = weak.upgrade() else { return };
                let gutter_width =
                    sourceview5::prelude::ViewExt::gutter(&doc.view, gtk::TextWindowType::Left)
                        .width();
                if (x as i32) < gutter_width {
                    let (_, by) = doc.view.window_to_buffer_coords(
                        gtk::TextWindowType::Widget,
                        x as i32,
                        y as i32,
                    );
                    let (it, _) = doc.view.line_at_y(by);
                    toggle_breakpoint(&ui_g, &doc, it.line() as u32 + 1);
                }
            });
        }
        view.add_controller(gutter);
        let click = gtk::GestureClick::new();
        click.set_button(1);
        click.connect_released(move |g, _, x, y| {
            let Some(doc) = weak.upgrade() else { return };
            if g.current_event_state()
                .contains(gtk::gdk::ModifierType::CONTROL_MASK)
            {
                let (tx, ty) = doc.view.window_to_buffer_coords(
                    gtk::TextWindowType::Widget,
                    x as i32,
                    y as i32,
                );
                if let Some(it) = doc.view.iter_at_location(tx, ty) {
                    go_to_definition(&ui_g, &doc, &it);
                }
            }
        });
        view.add_controller(click);
    }
    // Editing keys: Capture so they run before the TextView's own bindings
    // (Tab inserts a tab, Ctrl+/ selects all). Only these keys match.
    let keys = gtk::ShortcutController::new();
    keys.set_propagation_phase(gtk::PropagationPhase::Capture);
    let b = doc.buffer.clone();
    {
        let b = b.clone();
        add_shortcut(&keys, "Tab", move || indent::tab(&b));
    }
    for accel in ["<Shift>Tab", "<Shift>ISO_Left_Tab"] {
        let b = b.clone();
        add_shortcut(&keys, accel, move || indent::backtab(&b));
    }
    let on = |accel: &str, f: fn(&sourceview5::Buffer)| {
        let b = b.clone();
        add_shortcut(&keys, accel, move || f(&b));
    };
    on("<Control><Alt>backslash", indent::indent_region_or_line);
    on("<Primary>slash", editing::toggle_comment);
    on("<Primary>KP_Divide", editing::toggle_comment);
    on("<Primary>d", editing::duplicate_lines);
    on("<Alt>Up", |b| editing::move_lines(b, true));
    on("<Alt>Down", |b| editing::move_lines(b, false));
    {
        let ui_h = Rc::clone(ui);
        let weak = Rc::downgrade(doc);
        let vh = Rc::downgrade(vh);
        add_shortcut(&keys, "F1", move || {
            let (Some(doc), Some(vh)) = (weak.upgrade(), vh.upgrade()) else {
                return;
            };
            let help = ui_h.borrow().help.clone().expect("help");
            if !assist::show_info(&doc.view, &vh, &help) {
                set_status(&ui_h, "F1: coloque o cursor no nome de uma função");
            }
        });
    }
    {
        let ui_d = Rc::clone(ui);
        let weak = Rc::downgrade(doc);
        add_shortcut(&keys, "F12", move || {
            if let Some(doc) = weak.upgrade() {
                go_to_definition(&ui_d, &doc, &doc.cursor());
            }
        });
    }
    {
        let vh = Rc::downgrade(vh);
        add_shortcut_if(&keys, "Escape", move || {
            if let Some(vh) = vh.upgrade() {
                vh.hide_signature();
            }
            false
        });
    }
    view.add_controller(keys);
}

fn new_untitled_tab(ui: &UiRc) {
    open_doc(ui, None, "escreva(\"Olá\")\n");
    set_status(ui, "arquivo novo (ainda não salvo)");
}

/// Open `path` from the project (or switch to its tab).
fn open_path(ui: &UiRc, path: &str) -> Option<Rc<Doc>> {
    if let Some(d) = doc_by_name(ui, path) {
        switch_to(ui, &d);
        return Some(d);
    }
    let result = {
        let u = ui.borrow();
        u.rpc.read_file(&u.project, path)
    };
    match result {
        Ok(src) => {
            let doc = open_doc(ui, Some(path.to_string()), &src);
            set_status(ui, &format!("aberto {path}"));
            Some(doc)
        }
        Err(e) => {
            set_status(ui, &format!("abrir {path}: {e}"));
            None
        }
    }
}

fn goto_file_line(ui: &UiRc, file: &str, line: u32) {
    let doc = doc_by_name(ui, file).or_else(|| open_path(ui, file));
    if let Some(d) = doc {
        switch_to(ui, &d);
        d.goto_line(line);
    }
}

fn remove_tab(ui: &UiRc, doc: &Rc<Doc>) {
    let (nb, vh) = {
        let mut u = ui.borrow_mut();
        let pos = u.docs.iter().position(|(d, _)| Rc::ptr_eq(d, doc));
        let vh = pos.map(|p| u.docs.remove(p).1);
        (u.notebook.clone(), vh)
    };
    if let Some(vh) = vh {
        vh.detach();
    }
    if let Some(n) = nb.page_num(&doc.page) {
        nb.remove_page(Some(n));
    }
    if ui.borrow().docs.is_empty() {
        let d = open_doc(ui, None, "");
        d.mark_saved();
    }
    refresh_title(ui);
}

fn close_doc(ui: &UiRc, doc: &Rc<Doc>) {
    if !doc.is_dirty() {
        remove_tab(ui, doc);
        return;
    }
    let ui_c = Rc::clone(ui);
    let d = Rc::clone(doc);
    confirm_unsaved(ui, vec![Rc::clone(doc)], move || remove_tab(&ui_c, &d));
}

/// Save one tab; an untitled one asks for a name. False if not saved.
fn save_doc(ui: &UiRc, doc: &Rc<Doc>) -> bool {
    if doc.is_untitled() {
        let window = ui.borrow().window.clone();
        let Some(name) = perguntar_linha(&window, &format!("Salvar `{}` como:", doc.untitled))
        else {
            return false;
        };
        let name = name.trim().trim_start_matches('/').to_string();
        if name.is_empty() {
            return false;
        }
        let name = if name.contains('.') {
            name
        } else {
            format!("{name}.lep")
        };
        if doc_by_name(ui, &name).is_some() {
            set_status(ui, &format!("`{name}` já está aberto em outra aba"));
            return false;
        }
        doc.set_path(name);
    }
    let name = doc.name();
    let result = {
        let u = ui.borrow();
        u.rpc.write_file(&u.project, &name, &doc.text())
    };
    match result {
        Ok(()) => {
            doc.mark_saved();
            ui.borrow().file_cache.borrow_mut().remove(&name);
            refresh_tree(ui);
            refresh_title(ui);
            set_status(ui, &format!("salvo {name}"));
            true
        }
        Err(e) => {
            set_status(ui, &format!("erro ao salvar {name}: {e}"));
            false
        }
    }
}

/// Unsaved changes in `docs`: Salvar (all), Descartar or Cancelar. `then`
/// runs unless cancelled or a save failed.
fn confirm_unsaved(ui: &UiRc, docs: Vec<Rc<Doc>>, then: impl FnOnce() + 'static) {
    let window = ui.borrow().window.clone();
    let names: Vec<String> = docs.iter().map(|d| d.name()).collect();
    let (title, detail) = if names.len() == 1 {
        (
            format!("Salvar as alterações em `{}`?", names[0]),
            "Se não salvar, as alterações se perdem.".to_string(),
        )
    } else {
        (
            "Há arquivos com alterações não salvas".to_string(),
            names.join("\n"),
        )
    };
    let then = RefCell::new(Some(then));
    #[allow(deprecated)]
    let dlg = gtk::MessageDialog::builder()
        .transient_for(&window)
        .modal(true)
        .message_type(gtk::MessageType::Warning)
        .buttons(gtk::ButtonsType::None)
        .text(&title)
        .secondary_text(&detail)
        .build();
    #[allow(deprecated)]
    {
        dlg.add_button("Cancelar", gtk::ResponseType::Cancel);
        dlg.add_button("Descartar", gtk::ResponseType::Reject);
        dlg.add_button(
            if names.len() == 1 {
                "Salvar"
            } else {
                "Salvar todos"
            },
            gtk::ResponseType::Accept,
        );
        dlg.set_default_response(gtk::ResponseType::Accept);
    }
    let ui_c = Rc::clone(ui);
    #[allow(deprecated)]
    dlg.connect_response(move |d, resp| {
        d.close();
        let go = match resp {
            gtk::ResponseType::Reject => true,
            gtk::ResponseType::Accept => docs.iter().all(|doc| save_doc(&ui_c, doc)),
            _ => false,
        };
        if go {
            if let Some(f) = then.borrow_mut().take() {
                f();
            }
        }
    });
    dlg.present();
}

fn refresh_title(ui: &UiRc) {
    let Some(doc) = current_doc(ui) else { return };
    let Ok(u) = ui.try_borrow() else { return };
    let star = if doc.is_dirty() { "● " } else { "" };
    u.window.set_title(Some(&format!(
        "{star}{} — {} — Expressa Aula",
        doc.name(),
        u.project
    )));
}

fn refresh_diag_label(ui: &UiRc) {
    let doc = current_doc(ui);
    let Ok(u) = ui.try_borrow() else { return };
    match doc.and_then(|d| d.diag.borrow().clone()) {
        Some(d) => {
            u.diag_label.set_markup(&format!(
                "<span foreground=\"#e53935\">⚠ linha {}: {}</span>",
                d.line,
                glib::markup_escape_text(&d.message)
            ));
        }
        None => u.diag_label.set_text(""),
    }
}

fn go_to_definition(ui: &UiRc, doc: &Rc<Doc>, at: &gtk::TextIter) {
    let help = ui.borrow().help.clone().expect("help");
    let mut ls = at.clone();
    ls.set_line_offset(0);
    let mut le = at.clone();
    if !le.ends_line() {
        le.forward_to_line_end();
    }
    let line = ls.text(&le).to_string();
    let col = ls.text(at).len();
    match assist::definition(&help, &doc.name(), &doc.text(), &line, col) {
        Some(Target::Line { file: None, line }) => doc.goto_line(line),
        Some(Target::Line {
            file: Some(f),
            line,
        }) => goto_file_line(ui, &f, line),
        Some(Target::File(f)) => {
            open_path(ui, &f);
        }
        Some(Target::Native(name)) => {
            doc.buffer.place_cursor(at);
            if let Some(vh) = view_help(ui, doc) {
                assist::show_info(&doc.view, &vh, &help);
            }
            set_status(ui, &format!("`{name}` é uma função nativa"));
        }
        None => {}
    }
}

// ── Prefs ──────────────────────────────────────────────────────────────────

fn zoom(ui: &UiRc, step: i32) {
    let mut u = ui.borrow_mut();
    u.prefs.zoom(step);
    u.font_css.set_size(u.prefs.font_size);
    u.prefs.save();
    let size = u.prefs.font_size;
    drop(u);
    set_status(ui, &format!("letra {size}pt (Ctrl+0 volta ao normal)"));
}

fn theme_scheme(ui: &UiRc) -> (bool, Option<sourceview5::StyleScheme>) {
    let u = ui.borrow();
    let dark = u.prefs.dark;
    let ids: &[&str] = if dark {
        &["expressa-aula-dark", "Adwaita-dark", "classic-dark"]
    } else {
        &["expressa-aula", "Adwaita", "classic"]
    };
    (dark, ids.iter().find_map(|id| u.schemes.scheme(id)))
}

fn apply_theme(ui: &UiRc) {
    let (dark, scheme) = theme_scheme(ui);
    prefs::apply_widget_theme(dark);
    for d in all_docs(ui) {
        d.set_theme(scheme.as_ref(), dark);
    }
    ui.borrow().console.set_dark(dark);
}

fn set_dark(ui: &UiRc, dark: bool) {
    {
        let mut u = ui.borrow_mut();
        if u.prefs.dark == dark {
            return;
        }
        u.prefs.dark = dark;
        u.prefs.save();
    }
    apply_theme(ui);
}

// ── Dialogs ────────────────────────────────────────────────────────────────

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

// ── Debugger panel ─────────────────────────────────────────────────────────

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

fn make_debug_panel(ui: &UiRc) -> GtkBox {
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
    watch_row.append(&ui.borrow().watch_entry);
    watch_row.append(&btn_add);
    watch_row.append(&btn_del);
    panel.append(&watch_row);

    connect(&btn_add, ui, add_watch);
    {
        let ui_a = Rc::clone(ui);
        ui.borrow()
            .watch_entry
            .connect_activate(move |_| add_watch(&ui_a));
    }
    connect(&btn_del, ui, remove_watch);

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

fn add_watch(ui: &UiRc) {
    let name = ui.borrow().watch_entry.text().trim().to_string();
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

fn remove_watch(ui: &UiRc) {
    ui.borrow_mut().watches.pop();
    refresh_watch_values(ui, &[]);
}

fn refresh_watch_values(ui: &UiRc, vars: &[expressa::runtime::DebugBinding]) {
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

fn send_debug_cmd(ui: &UiRc, cmd: &str) {
    let sent = match &ui.borrow().run {
        Some(run) => run.debug_cmd.send(cmd.to_string()).is_ok(),
        None => false,
    };
    // Resuming: the program runs until the next pause, so the step buttons
    // are off and the stopped line is no longer current.
    if sent && matches!(cmd, "continuar" | "proximo" | "entrar" | "sair") {
        set_debug_buttons(ui, false, true);
        clear_debug_views(ui);
        set_status(ui, "rodando… (Parar encerra)");
    }
}

fn set_debug_buttons(ui: &UiRc, paused: bool, running: bool) {
    let u = ui.borrow();
    u.btn_continue.set_sensitive(paused);
    u.btn_next.set_sensitive(paused);
    u.btn_step.set_sensitive(paused);
    u.btn_out.set_sensitive(paused);
    u.btn_stop.set_sensitive(running);
}

fn toggle_breakpoint_here(ui: &UiRc) {
    if let Some(doc) = current_doc(ui) {
        let line = doc.cursor().line() as u32 + 1;
        toggle_breakpoint(ui, &doc, line);
    }
}

fn toggle_breakpoint(ui: &UiRc, doc: &Rc<Doc>, line: u32) {
    let on = doc.toggle_breakpoint(line);
    // While running, tell the debugger (it names files by their copy).
    let cmd = ui.borrow().run.as_ref().map(|run| {
        let file = run.ws_dir.join(doc.name()).to_string_lossy().into_owned();
        format!("{} {file}:{line}", if on { "ponto" } else { "remover" })
    });
    if let Some(cmd) = cmd {
        send_debug_cmd(ui, &cmd);
    }
}

fn fill_debug_views(ui: &UiRc, paused: &expressa::runtime::DebugPaused) {
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
}

fn clear_debug_views(ui: &UiRc) {
    {
        let u = ui.borrow();
        u.vars_store.clear();
        u.stack_store.clear();
    }
    for d in all_docs(ui) {
        d.clear_debug_line();
    }
}

/// Show where the debugger stopped, opening that file if needed.
fn show_pause(ui: &UiRc, p: &expressa::runtime::DebugPaused) {
    for d in all_docs(ui) {
        d.clear_debug_line();
    }
    match doc_by_name(ui, &p.file).or_else(|| open_path(ui, &p.file)) {
        Some(doc) => {
            switch_to(ui, &doc);
            doc.show_debug_line(p.line, p.error.is_some());
        }
        None => set_status(
            ui,
            &format!("pausado em {}:{} (arquivo fora do projeto)", p.file, p.line),
        ),
    }
}

// ── Running ────────────────────────────────────────────────────────────────

/// Events handled per 16 ms tick, so a program that prints without end
/// cannot freeze the window. The rest wait in the (bounded) channel.
const EVENTS_PER_TICK: usize = 200;

fn rodar(ui: &UiRc, debug: bool) {
    if ui.borrow().run.is_some() {
        set_status(ui, "já tem um programa rodando — aperte Parar primeiro");
        return;
    }
    let Some(doc) = current_doc(ui) else { return };
    let name = doc.name();
    let docs = all_docs(ui);
    let (project, args) = {
        let u = ui.borrow();
        (u.project.clone(), parse_run_args(&u.args_entry.text()))
    };
    // Fresh copy of the project with the text of every open tab, so the
    // program and its `importe`s run what is on screen (saved or not).
    let ws = {
        let u = ui.borrow();
        Workspace::download(&u.rpc, &project).and_then(|mut ws| {
            for d in &docs {
                ws.write_editor_text(&d.name(), &d.text())?;
            }
            Ok(ws)
        })
    };
    let ws = match ws {
        Ok(ws) => ws,
        Err(e) => {
            set_status(ui, &format!("não copiou o projeto: {e}"));
            return;
        }
    };
    let bps: Vec<(String, u32)> = docs
        .iter()
        .flat_map(|d| {
            let n = d.name();
            d.breakpoints
                .borrow()
                .iter()
                .map(move |&l| (n.clone(), l))
                .collect::<Vec<_>>()
        })
        .collect();
    let console = Rc::clone(&ui.borrow().console);
    console.clear();
    clear_debug_views(ui);
    if let Some(d) = doc.diag.borrow().as_ref() {
        set_status(
            ui,
            &format!(
                "atenção: erro de sintaxe na linha {} — rodando mesmo assim",
                d.line
            ),
        );
    } else {
        set_status(
            ui,
            &format!("{} {name}…", if debug { "depurando" } else { "rodando" }),
        );
    }

    let run = exec::spawn(&ws, &name, doc.text(), debug, bps, args);
    ui.borrow_mut().run = Some(Run {
        debug_cmd: run.debug_cmd.clone(),
        ws_dir: ws.dir.clone(),
    });
    set_debug_buttons(ui, false, true);

    let ui_ev = Rc::clone(ui);
    glib::timeout_add_local(std::time::Duration::from_millis(16), move || {
        for _ in 0..EVENTS_PER_TICK {
            match run.events.try_recv() {
                Ok(ExecEvent::Stdout(s)) => console.stdout(&s),
                Ok(ExecEvent::Stderr(s)) => console.stderr(&s),
                Ok(ExecEvent::Paused(p)) => {
                    fill_debug_views(&ui_ev, &p);
                    set_debug_buttons(&ui_ev, true, true);
                    show_pause(&ui_ev, &p);
                    match &p.error {
                        Some(e) => set_status(
                            &ui_ev,
                            &format!(
                                "erro em {}:{}: {e} — veja as variáveis; Continuar encerra",
                                p.file, p.line
                            ),
                        ),
                        None => set_status(
                            &ui_ev,
                            &format!("pausado {}:{}  {}", p.file, p.line, p.source_line.trim()),
                        ),
                    }
                }
                Ok(ExecEvent::Leia(prompt)) => {
                    console.begin_input(&prompt, run.lines.clone());
                    set_status(
                        &ui_ev,
                        "o programa espera uma resposta: digite na saída e aperte Enter",
                    );
                }
                Ok(ExecEvent::Finished(f)) => {
                    console.cancel_input();
                    ui_ev.borrow_mut().run = None;
                    set_debug_buttons(&ui_ev, false, false);
                    clear_debug_views(&ui_ev);
                    // Files the program wrote (salve_arquivo, …) go back to
                    // the server.
                    let sent = {
                        let u = ui_ev.borrow();
                        ws.upload_changes(&u.rpc, &project)
                    };
                    if matches!(sent, Ok(n) if n > 0) {
                        refresh_tree(&ui_ev);
                    }
                    if f.ok {
                        set_status(&ui_ev, &format!("ok — {name}"));
                    } else {
                        console.error_link(&format!(
                            "erro: {} em {}:{}\n",
                            f.error_message, f.error_file, f.error_line
                        ));
                        set_status(&ui_ev, "erro ao rodar (clique no erro para ir à linha)");
                        if f.error_line > 0 {
                            let file = if f.error_file.is_empty() {
                                name.clone()
                            } else {
                                f.error_file.clone()
                            };
                            goto_file_line(&ui_ev, &file, f.error_line);
                        }
                    }
                    if let Err(e) = sent {
                        set_status(&ui_ev, &format!("não enviou os arquivos: {e}"));
                    }
                    return glib::ControlFlow::Break;
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => break,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    console.cancel_input();
                    ui_ev.borrow_mut().run = None;
                    set_debug_buttons(&ui_ev, false, false);
                    return glib::ControlFlow::Break;
                }
            }
        }
        glib::ControlFlow::Continue
    });
}

// ── Project and file tree ──────────────────────────────────────────────────

/// Load the project named in the entry. Another project replaces the tabs
/// (asking about unsaved changes first).
fn conectar(ui: &UiRc) {
    let wanted = {
        let s = ui.borrow().student.text().trim().to_string();
        if s.is_empty() { "local".to_string() } else { s }
    };
    let same = ui.borrow().project == wanted;
    if same || ui.borrow().docs.is_empty() {
        ui.borrow_mut().project = wanted;
        refresh_tree(ui);
        return;
    }
    let ui_c = Rc::clone(ui);
    let switch = move || {
        for d in all_docs(&ui_c) {
            // Already saved or discarded: close without asking again.
            d.mark_saved();
            remove_tab(&ui_c, &d);
        }
        ui_c.borrow_mut().project = wanted.clone();
        ui_c.borrow().file_cache.borrow_mut().clear();
        refresh_tree(&ui_c);
        refresh_title(&ui_c);
    };
    let dirty = dirty_docs(ui);
    if dirty.is_empty() {
        switch();
    } else {
        confirm_unsaved(ui, dirty, switch);
    }
}

fn refresh_tree(ui: &UiRc) {
    let (entries, projeto, err) = {
        let u = ui.borrow();
        let projeto = u.project.clone();
        match u.rpc.list_tree(&projeto) {
            Ok(p) => (p, projeto, None),
            Err(e) => (Vec::new(), projeto, Some(e)),
        }
    };
    {
        let mut u = ui.borrow_mut();
        u.project_files = entries
            .iter()
            .filter(|e| !e.is_dir)
            .map(|e| e.path.clone())
            .collect();
        u.file_cache.borrow_mut().clear();
    }
    preencher_arvore(ui, &projeto, &entries);
    ui.borrow().tree.expand_all();
    // Long names must not leave the list scrolled sideways.
    {
        let tree = ui.borrow().tree.clone();
        glib::idle_add_local_once(move || {
            if let Some(h) = tree.hadjustment() {
                h.set_value(0.0);
            }
        });
    }
    match err {
        Some(e) => set_status(ui, &format!("conectar: {e}")),
        None => set_status(
            ui,
            &format!("projeto `{projeto}` — {} itens", entries.len()),
        ),
    }
}

#[allow(deprecated)]
fn preencher_arvore(ui: &UiRc, projeto: &str, entries: &[expressa_aula_proto::TreeEntry]) {
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

#[allow(deprecated)]
fn abrir_no(ui: &UiRc, path: &gtk::TreePath) {
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
    open_path(ui, &rel);
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

#[allow(deprecated)]
fn selected_tree_entry(ui: &UiRc) -> (String, bool) {
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

#[allow(deprecated)]
fn tree_popup(ui: &UiRc, x: f64, y: f64) {
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
    {
        let pop_c = pop.clone();
        pop.connect_closed(move |_| {
            let p = pop_c.clone();
            glib::idle_add_local_once(move || p.unparent());
        });
    }
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
    let dir = parent_dir(&rel, is_dir);
    add_item(
        &box_,
        "Novo arquivo",
        Box::new({
            let ui = Rc::clone(ui);
            let dir = dir.clone();
            let window = window.clone();
            move || {
                if let Some(name) = perguntar_linha(&window, "Nome do arquivo:") {
                    let name = name.trim().to_string();
                    if name.is_empty() {
                        return;
                    }
                    let name = if name.contains('.') {
                        name
                    } else {
                        format!("{name}.lep")
                    };
                    let path = join_path(&dir, &name);
                    let result = {
                        let u = ui.borrow();
                        u.rpc.write_file(&u.project, &path, "escreva(\"Olá\")\n")
                    };
                    match result {
                        Ok(()) => {
                            refresh_tree(&ui);
                            open_path(&ui, &path);
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
            let ui = Rc::clone(ui);
            let dir = dir.clone();
            let window = window.clone();
            move || {
                if let Some(name) = perguntar_linha(&window, "Nome da pasta:") {
                    if name.trim().is_empty() {
                        return;
                    }
                    let path = join_path(&dir, name.trim());
                    let result = {
                        let u = ui.borrow();
                        u.rpc.mkdir(&u.project, &path)
                    };
                    match result {
                        Ok(()) => {
                            refresh_tree(&ui);
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
                let ui = Rc::clone(ui);
                let rel = rel.clone();
                let window = window.clone();
                move || {
                    let base = rel.rsplit('/').next().unwrap_or(&rel).to_string();
                    if let Some(name) = perguntar_linha(&window, &format!("Renomear `{base}`:")) {
                        if name.trim().is_empty() {
                            return;
                        }
                        let to = join_path(&parent_dir(&rel, false), name.trim());
                        let result = {
                            let u = ui.borrow();
                            u.rpc.rename(&u.project, &rel, &to)
                        };
                        match result {
                            Ok(()) => {
                                // Open tabs follow the rename (files and folders).
                                for d in all_docs(&ui) {
                                    let n = d.name();
                                    if n == rel {
                                        d.set_path(to.clone());
                                    } else if let Some(rest) = n.strip_prefix(&format!("{rel}/")) {
                                        d.set_path(format!("{to}/{rest}"));
                                    }
                                }
                                refresh_tree(&ui);
                                refresh_title(&ui);
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
                let ui = Rc::clone(ui);
                let rel = rel.clone();
                let window = window.clone();
                move || {
                    let then = {
                        let ui = Rc::clone(&ui);
                        let rel = rel.clone();
                        move || {
                            let result = {
                                let u = ui.borrow();
                                u.rpc.delete_file(&u.project, &rel)
                            };
                            match result {
                                Ok(()) => {
                                    for d in all_docs(&ui) {
                                        let n = d.name();
                                        if n == rel || n.starts_with(&format!("{rel}/")) {
                                            d.mark_saved();
                                            remove_tab(&ui, &d);
                                        }
                                    }
                                    refresh_tree(&ui);
                                    set_status(&ui, &format!("apagado {rel}"));
                                }
                                Err(e) => set_status(&ui, &format!("apagar: {e}")),
                            }
                        }
                    };
                    let then = RefCell::new(Some(then));
                    let dlg = gtk::MessageDialog::builder()
                        .transient_for(&window)
                        .modal(true)
                        .message_type(gtk::MessageType::Warning)
                        .buttons(gtk::ButtonsType::None)
                        .text(format!("Apagar `{rel}`?"))
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

// ── Search (current tab) ───────────────────────────────────────────────────

fn show_search(ui: &UiRc) {
    let u = ui.borrow();
    u.search_revealer.set_reveal_child(true);
    u.search_entry.grab_focus();
}

fn hide_search(ui: &UiRc) {
    ui.borrow().search_revealer.set_reveal_child(false);
    if let Some(d) = current_doc(ui) {
        d.view.grab_focus();
    }
}

fn search_apply(ui: &UiRc) {
    let u = ui.borrow();
    let q = u.search_entry.text();
    if q.is_empty() {
        u.search_settings.set_search_text(None::<&str>);
    } else {
        u.search_settings.set_search_text(Some(q.as_str()));
    }
}

fn search_next(ui: &UiRc) {
    search_apply(ui);
    let Some(d) = current_doc(ui) else { return };
    let from = match d.buffer.selection_bounds() {
        Some((_, end)) => end,
        None => d.cursor(),
    };
    if let Some((a, b, _)) = d.search.forward(&from) {
        d.buffer.select_range(&a, &b);
        d.view.scroll_to_iter(&mut a.clone(), 0.2, false, 0.0, 0.0);
    } else {
        set_status(ui, "não encontrado");
    }
}

fn search_prev(ui: &UiRc) {
    search_apply(ui);
    let Some(d) = current_doc(ui) else { return };
    if let Some((a, b, _)) = d.search.backward(&d.cursor()) {
        d.buffer.select_range(&a, &b);
        d.view.scroll_to_iter(&mut a.clone(), 0.2, false, 0.0, 0.0);
    } else {
        set_status(ui, "não encontrado");
    }
}

fn search_replace_one(ui: &UiRc) {
    search_apply(ui);
    let Some(d) = current_doc(ui) else { return };
    let repl = ui.borrow().replace_entry.text().to_string();
    if let Some((mut a, mut b)) = d.buffer.selection_bounds() {
        let _ = d.search.replace(&mut a, &mut b, &repl);
    }
    search_next(ui);
}

fn search_replace_all(ui: &UiRc) {
    search_apply(ui);
    let Some(d) = current_doc(ui) else { return };
    let repl = ui.borrow().replace_entry.text().to_string();
    match d.search.replace_all(&repl) {
        Ok(()) => set_status(ui, "substituições feitas"),
        Err(e) => set_status(ui, &format!("substituir: {e}")),
    }
}

/// GTK may only be used from the thread that initialized it, but tests run
/// on many threads: every GTK test runs its body on one shared thread.
#[cfg(test)]
mod gtk_test {
    use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};
    use std::sync::mpsc::{Sender, channel};
    use std::sync::{Mutex, OnceLock};

    type Job = (Box<dyn FnOnce() + Send>, Sender<std::thread::Result<()>>);

    /// Runs `f` on the GTK thread; skips (returns) when there is no display.
    pub fn run(f: impl FnOnce() + Send + 'static) {
        static JOBS: OnceLock<Option<Mutex<Sender<Job>>>> = OnceLock::new();
        let jobs = JOBS.get_or_init(|| {
            let (tx, rx) = channel::<Job>();
            let (ready_tx, ready_rx) = channel();
            std::thread::spawn(move || {
                let ok = gtk::init().is_ok();
                let _ = ready_tx.send(ok);
                if ok {
                    for (job, done) in rx {
                        let _ = done.send(catch_unwind(AssertUnwindSafe(job)));
                    }
                }
            });
            ready_rx.recv().unwrap_or(false).then(|| Mutex::new(tx))
        });
        let Some(jobs) = jobs else { return };
        let (done_tx, done_rx) = channel();
        jobs.lock().unwrap().send((Box::new(f), done_tx)).unwrap();
        if let Err(panic) = done_rx.recv().unwrap() {
            resume_unwind(panic);
        }
    }
}
