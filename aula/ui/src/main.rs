#![cfg_attr(windows, windows_subsystem = "windows")]

mod ajuda;
mod assist;
mod cartesiano;
mod console;
mod docs;
mod editing;
mod exec;
mod indent;
mod janela_graf;
mod lang;
mod prefs;
mod rpc;
mod store;
mod tutoriais;

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::{Rc, Weak};

use gtk::gdk::Key;
use gtk::gio::{self, prelude::*};
use gtk::glib;
use gtk::glib::prelude::*;
use gtk::prelude::*;
use gtk::{
    Application, ApplicationWindow, Box as GtkBox, Button, CellRendererText, Entry, Label,
    Orientation, Paned, PolicyType, Revealer, ScrolledWindow, TreeStore, TreeView, TreeViewColumn,
};
use sourceview5::prelude::*;
use sourceview5::{LanguageManager, SearchSettings, StyleSchemeManager};

use assist::{Help, Target, ViewHelp};
use console::Console;
use docs::Doc;
use exec::{ExecEvent, Workspace};
use prefs::{FontCss, Prefs, ThemeCss, expand_tilde, path_for_display};
use rpc::Rpc;
use store::{Files, valid_aluno};

const APP_ID: &str = "dev.expressa.aula";
const DEFAULT_URL: &str = "http://127.0.0.1:50051";

/// A program running or being debugged.
struct Run {
    debug_cmd: std::sync::mpsc::Sender<String>,
    /// The local copy: breakpoints set while running name files there.
    ws_dir: PathBuf,
}

struct Ui {
    files: Files,
    /// Folder name locally, student name on a remote server.
    project: String,
    /// Full local path or `url — aula-data/aluno`, shown on the status bar.
    project_path: Label,
    tree: TreeView,
    store: TreeStore,
    project_files: Vec<String>,
    notebook: gtk::Notebook,
    docs: Vec<(Rc<Doc>, Rc<ViewHelp>)>,
    untitled_count: u32,
    /// Server files read for help on imports (`None` = missing).
    file_cache: RefCell<HashMap<String, Option<String>>>,
    console: Rc<Console>,
    graficos: Rc<cartesiano::Painel>,
    telas_graf: Rc<janela_graf::Painel>,
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
    /// Kept alive so GtkSourceView does not drop the Expressa language spec.
    languages: LanguageManager,
    schemes: StyleSchemeManager,
    prefs: Prefs,
    font_css: FontCss,
    theme_css: ThemeCss,
    help: Option<Rc<Help>>,
    /// Unsaved changes were handled: the window may close now.
    closing: bool,
    ajuda_win: RefCell<Option<gtk::Window>>,
}

type UiRc = Rc<RefCell<Ui>>;

fn main() {
    prepare_runtime_prefix();
    rpc::install_signal_handlers();
    let app = Application::builder().application_id(APP_ID).build();
    app.connect_activate(build_ui);
    app.run();
}

fn build_ui(app: &Application) {
    // ibus + GTK4 leaves the dead key (´ ~ `) stuck as preedit after composing
    // á/ã. GTK's built-in input method composes dead keys itself.
    // GTK_IM_MODULE in the environment still takes precedence over this.
    if let Some(settings) = gtk::Settings::default() {
        settings.set_gtk_im_module(Some("gtk-im-context-simple"));
    }

    let prefs = Prefs::load();
    let theme_css = ThemeCss::install();
    theme_css.apply(prefs.dark);
    let font_css = FontCss::install();
    font_css.set_size(prefs.font_size);

    let pasta_abs = expand_tilde(&prefs.pasta);
    let pasta_shown = if prefs.pasta.is_empty() {
        String::new()
    } else {
        path_for_display(&pasta_abs)
    };
    let project_path = Label::new(if pasta_shown.is_empty() {
        None
    } else {
        Some(pasta_shown.as_str())
    });
    project_path.set_xalign(1.0);
    project_path.set_ellipsize(gtk::pango::EllipsizeMode::Middle);
    project_path.set_max_width_chars(42);
    project_path.set_selectable(true);
    project_path.add_css_class("dim-label");
    if !prefs.pasta.is_empty() {
        project_path.set_tooltip_text(Some(&pasta_abs.display().to_string()));
    }
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
    // The row, not just the letters, is the click and highlight target.
    column.set_expand(true);
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

    let notebook = gtk::Notebook::new();
    notebook.set_scrollable(true);
    notebook.set_vexpand(true);
    notebook.set_hexpand(true);

    let console = Rc::new(Console::new());
    let graficos = cartesiano::Painel::new();
    graficos.set_application(app);
    let telas_graf = janela_graf::Painel::new();
    let args_entry = Entry::builder()
        .placeholder_text("argumentos (ex.: Thiago --ajuda)")
        .hexpand(true)
        .build();
    let status = Label::new(Some("escolha uma pasta para o projeto"));
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
    win.add_css_class("aula-window");

    let ui: UiRc = Rc::new(RefCell::new(Ui {
        files: Files::None,
        project: String::new(),
        project_path,
        tree,
        store,
        project_files: Vec::new(),
        notebook,
        docs: Vec::new(),
        untitled_count: 0,
        file_cache: RefCell::new(HashMap::new()),
        console,
        graficos,
        telas_graf,
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
        languages,
        schemes,
        prefs,
        font_css,
        theme_css,
        help: None,
        closing: false,
        ajuda_win: RefCell::new(None),
    }));
    let help = make_help(&ui);
    ui.borrow_mut().help = Some(help);

    let btn_smaller = Button::with_label("A−");
    btn_smaller.set_tooltip_text(Some("Letra menor (Ctrl+-)"));
    let btn_bigger = Button::with_label("A+");
    btn_bigger.set_tooltip_text(Some("Letra maior (Ctrl+=)"));

    let args_row = GtkBox::new(Orientation::Horizontal, 8);
    args_row.set_margin_start(8);
    args_row.set_margin_end(8);
    args_row.append(&Label::new(Some("argumentos:")));
    args_row.append(&ui.borrow().args_entry);
    args_row.append(&btn_smaller);
    args_row.append(&btn_bigger);

    const FILES_WIDTH: i32 = 240;
    let file_scroll = ScrolledWindow::builder()
        .hscrollbar_policy(PolicyType::Automatic)
        .vscrollbar_policy(PolicyType::Automatic)
        .child(&ui.borrow().tree)
        .build();
    const OUTPUT_HEIGHT: i32 = 160;
    let output_scroll = ScrolledWindow::builder()
        .hscrollbar_policy(PolicyType::Automatic)
        .vscrollbar_policy(PolicyType::Automatic)
        .hexpand(true)
        .vexpand(true)
        .min_content_height(64)
        .child(&ui.borrow().console.view)
        .build();

    let debug_panel = make_debug_panel(&ui);
    const DEBUG_PANEL_WIDTH: i32 = 320;
    debug_panel.set_hexpand(false);

    install_css();

    let bottom = GtkBox::new(Orientation::Vertical, 0);
    bottom.set_hexpand(true);
    {
        let u = ui.borrow();
        bottom.append(&u.graficos.dock);
        bottom.append(&output_scroll);
    }
    let editor_col = Paned::new(Orientation::Vertical);
    editor_col.add_css_class("aula-paned");
    editor_col.set_wide_handle(false);
    editor_col.set_start_child(Some(&ui.borrow().notebook));
    editor_col.set_end_child(Some(&bottom));
    editor_col.set_resize_start_child(true);
    editor_col.set_resize_end_child(false);
    editor_col.set_shrink_start_child(true);
    editor_col.set_shrink_end_child(true);

    let main_split = Paned::new(Orientation::Horizontal);
    main_split.add_css_class("aula-paned");
    main_split.set_start_child(Some(&editor_col));
    main_split.set_end_child(Some(&debug_panel));
    main_split.set_resize_start_child(true);
    main_split.set_resize_end_child(false);
    main_split.set_shrink_end_child(true);
    main_split.set_wide_handle(false);

    let body = Paned::new(Orientation::Horizontal);
    body.add_css_class("aula-paned");
    body.set_start_child(Some(&file_scroll));
    body.set_end_child(Some(&main_split));
    // The file list keeps its width; the editor takes what the window gains.
    body.set_resize_start_child(false);
    body.set_shrink_start_child(true);
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
    status_row.append(&ui.borrow().project_path);

    install_menubar(&win, &ui, &file_scroll, &debug_panel);
    root.append(&args_row);
    root.append(&ui.borrow().search_revealer);
    root.append(&body);
    root.append(&status_row);
    body.set_vexpand(true);
    win.set_child(Some(&root));

    connect(&btn_smaller, &ui, |ui| zoom(ui, -1));
    connect(&btn_bigger, &ui, |ui| zoom(ui, 1));
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
        k(&fkeys, "F5", |ui| executar(ui, false));
        k(&fkeys, "F6", |ui| executar(ui, true));
        k(&fkeys, "F9", toggle_breakpoint_here);
        k(&fkeys, "F10", |ui| send_debug_cmd(ui, "proximo"));
        k(&fkeys, "F11", |ui| send_debug_cmd(ui, "entrar"));
        k(&fkeys, "<Shift>F11", |ui| send_debug_cmd(ui, "sair"));
        add_shortcut(&fkeys, "F7", {
            let win = win.clone();
            move || trigger_win_action(&win, "mostrar-arquivos")
        });
        add_shortcut(&fkeys, "F8", {
            let win = win.clone();
            move || trigger_win_action(&win, "mostrar-painel")
        });
        k(&shortcuts, "<Primary>q", encerrar);
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
        // Capture + released: a Popover shown on pressed is dismissed by the
        // same click. Parent later is the window so the ScrolledWindow does
        // not clip the menu. Claim so TreeView does not eat the event.
        let ui_t = Rc::clone(&ui);
        let tree = ui.borrow().tree.clone();
        let scroll = file_scroll.clone();
        let right = gtk::GestureClick::new();
        right.set_button(gtk::gdk::BUTTON_SECONDARY);
        right.set_propagation_phase(gtk::PropagationPhase::Capture);
        right.connect_pressed(|g, _, _, _| {
            g.set_state(gtk::EventSequenceState::Claimed);
        });
        right.connect_released(move |_, _, x, y| {
            let (tx, ty) = scroll.translate_coordinates(&tree, x, y).unwrap_or((x, y));
            tree_popup(&ui_t, tx, ty);
        });
        file_scroll.add_controller(right);
        let ui_del = Rc::clone(&ui);
        let del = gtk::EventControllerKey::new();
        del.connect_key_pressed(move |_, key, _, _| {
            if key == Key::Delete || key == Key::KP_Delete {
                let (rel, _) = selected_tree_entry(&ui_del);
                if !rel.is_empty() {
                    confirmar_apagar(&ui_del, &rel);
                }
                return glib::Propagation::Stop;
            }
            glib::Propagation::Proceed
        });
        ui.borrow().tree.add_controller(del);
    }
    {
        let ui_close = Rc::clone(&ui);
        win.connect_close_request(move |w| {
            if ui_close.borrow().closing {
                fechar_janelas_filhas(&ui_close);
                ui_close.borrow_mut().files.shutdown();
                return glib::Propagation::Proceed;
            }
            let dirty: Vec<Rc<Doc>> = dirty_docs(&ui_close);
            if dirty.is_empty() {
                fechar_janelas_filhas(&ui_close);
                ui_close.borrow_mut().files.shutdown();
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

    apply_theme(&ui);
    win.maximize();
    win.present();
    schedule_raise();
    {
        let ui_p = Rc::clone(&ui);
        glib::timeout_add_local_once(std::time::Duration::from_millis(250), move || {
            let start = Rc::new({
                let ui = Rc::clone(&ui_p);
                move || {
                    let first = ui.borrow().prefs.pasta.trim().is_empty();
                    let opened = if first && open_shipped_exemplos(&ui) {
                        true
                    } else {
                        choose_local_folder(&ui)
                    };
                    schedule_raise();
                    if !opened {
                        set_status(&ui, "escolha uma pasta para o projeto");
                    }
                    if !lang_ok {
                        set_status(
                            &ui,
                            "aviso: gramática Expressa não carregou; o texto fica sem cores",
                        );
                    }
                }
            });
            let viu = ui_p.borrow().prefs.viu_boas_vindas;
            if viu {
                start();
            } else {
                let window = ui_p.borrow().window.clone();
                let ui_w = Rc::clone(&ui_p);
                let start = Rc::clone(&start);
                tutoriais::mostrar_boas_vindas(&window, move || {
                    {
                        let mut u = ui_w.borrow_mut();
                        u.prefs.viu_boas_vindas = true;
                        u.prefs.save();
                    }
                    start();
                });
            }
        });
    }
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

fn encerrar(ui: &UiRc) {
    let w = ui.borrow().window.clone();
    w.close();
}

fn fechar_janelas_filhas(ui: &UiRc) {
    let (graficos, telas_graf, ajuda) = {
        let u = ui.borrow();
        (
            Rc::clone(&u.graficos),
            Rc::clone(&u.telas_graf),
            u.ajuda_win.borrow_mut().take(),
        )
    };
    graficos.fechar_janela_destacada();
    telas_graf.fechar_todas();
    if let Some(w) = ajuda {
        w.close();
    }
}

fn open_ajuda(ui: &UiRc) {
    let u = ui.borrow();
    if let Some(w) = u.ajuda_win.borrow().as_ref() {
        w.present();
        return;
    }
    let w = ajuda::open(&u.window);
    w.connect_destroy({
        let ui = Rc::clone(ui);
        move |_| {
            *ui.borrow().ajuda_win.borrow_mut() = None;
        }
    });
    *u.ajuda_win.borrow_mut() = Some(w);
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

fn menu_item(label: &str, action: &str, accel: Option<&str>) -> gio::MenuItem {
    let item = gio::MenuItem::new(Some(label), Some(action));
    if let Some(accel) = accel {
        item.set_attribute_value("accel", Some(&accel.to_variant()));
    }
    item
}

fn menu_radio(label: &str, action: &str, target: &str) -> gio::MenuItem {
    let item = gio::MenuItem::new(Some(label), None);
    item.set_action_and_target_value(Some(action), Some(&target.to_variant()));
    item
}

fn add_win_action(
    win: &ApplicationWindow,
    ui: &UiRc,
    name: &str,
    f: impl Fn(&UiRc) + 'static,
) -> gio::SimpleAction {
    let action = gio::SimpleAction::new(name, None);
    let ui = Rc::clone(ui);
    action.connect_activate(move |_, _| f(&ui));
    win.add_action(&action);
    action
}

fn add_win_toggle(
    win: &ApplicationWindow,
    name: &str,
    initial: bool,
    f: impl Fn(bool) + 'static,
) -> gio::SimpleAction {
    let action = gio::SimpleAction::new_stateful(name, None, &initial.to_variant());
    // Stateful actions: GTK toggles via change-state (menu checkbox, activate).
    // Connecting activate and inverting again would flip twice.
    action.connect_change_state(move |a, value| {
        let Some(value) = value else { return };
        let on = value.get::<bool>().unwrap_or(initial);
        if a.state().and_then(|v| v.get::<bool>()) == Some(on) {
            return;
        }
        a.set_state(value);
        f(on);
    });
    win.add_action(&action);
    action
}

fn set_action_enabled(win: &ApplicationWindow, name: &str, on: bool) {
    if let Some(a) = win.lookup_action(name) {
        if let Ok(a) = a.downcast::<gio::SimpleAction>() {
            a.set_enabled(on);
        }
    }
}

fn trigger_win_action(win: &ApplicationWindow, name: &str) {
    if let Some(a) = win.lookup_action(name) {
        gio::prelude::ActionExt::activate(&a, None);
    }
}

fn set_action_state_str(win: &ApplicationWindow, name: &str, value: &str) {
    if let Some(a) = win.lookup_action(name) {
        if let Ok(a) = a.downcast::<gio::SimpleAction>() {
            a.set_state(&value.to_variant());
        }
    }
}

fn set_project_path(ui: &UiRc, shown: &str, tooltip: Option<&str>) {
    let u = ui.borrow();
    u.project_path.set_text(shown);
    if tooltip.unwrap_or("").is_empty() {
        u.project_path.set_tooltip_text(None);
    } else {
        u.project_path.set_tooltip_text(tooltip);
    }
}

fn set_local_project_path(ui: &UiRc, path: &std::path::Path) {
    let full = path.display().to_string();
    set_project_path(ui, &path_for_display(path), Some(&full));
}

fn install_menubar(
    win: &ApplicationWindow,
    ui: &UiRc,
    file_scroll: &ScrolledWindow,
    debug_panel: &GtkBox,
) {
    add_win_action(win, ui, "novo", new_untitled_tab);
    add_win_action(win, ui, "salvar", |ui| {
        if let Some(d) = current_doc(ui) {
            save_doc(ui, &d);
        }
    });
    add_win_action(win, ui, "fechar-aba", |ui| {
        if let Some(d) = current_doc(ui) {
            close_doc(ui, &d);
        }
    });
    add_win_action(win, ui, "pasta", |ui| {
        choose_local_folder(ui);
    });
    add_win_action(win, ui, "servidor", ask_server);
    add_win_action(win, ui, "executar", |ui| executar(ui, false));
    add_win_action(win, ui, "depurar", |ui| executar(ui, true));
    add_win_action(win, ui, "parar", |ui| send_debug_cmd(ui, "terminar"));
    add_win_action(win, ui, "continuar", |ui| send_debug_cmd(ui, "continuar"));
    add_win_action(win, ui, "proximo", |ui| send_debug_cmd(ui, "proximo"));
    add_win_action(win, ui, "entrar", |ui| send_debug_cmd(ui, "entrar"));
    add_win_action(win, ui, "sair", |ui| send_debug_cmd(ui, "sair"));
    add_win_action(win, ui, "ponto", toggle_breakpoint_here);
    add_win_action(win, ui, "letra-maior", |ui| zoom(ui, 1));
    add_win_action(win, ui, "letra-menor", |ui| zoom(ui, -1));
    add_win_action(win, ui, "letra-normal", |ui| zoom(ui, 0));
    add_win_action(win, ui, "ajuda", open_ajuda);
    add_win_action(win, ui, "tutoriais", |ui| {
        if let Err(e) = tutoriais::spawn_tutorial() {
            set_status(ui, &e);
        }
    });
    add_win_action(win, ui, "buscar", show_search);
    add_win_action(win, ui, "encerrar", encerrar);
    for name in ["continuar", "proximo", "entrar", "sair", "parar"] {
        set_action_enabled(win, name, false);
    }

    {
        let pane = file_scroll.clone();
        add_win_toggle(win, "mostrar-arquivos", true, move |on| {
            pane.set_visible(on)
        });
    }
    {
        let pane = debug_panel.clone();
        add_win_toggle(win, "mostrar-painel", true, move |on| pane.set_visible(on));
    }
    {
        let dark = ui.borrow().prefs.dark;
        let initial = if dark { "escuro" } else { "claro" };
        let action = gio::SimpleAction::new_stateful(
            "tema",
            Some(gtk::glib::VariantTy::STRING),
            &initial.to_variant(),
        );
        let ui = Rc::clone(ui);
        action.connect_change_state(move |a, value| {
            let Some(value) = value else { return };
            if a.state().as_ref() == Some(value) {
                return;
            }
            a.set_state(value);
            set_dark(&ui, value.str() == Some("escuro"));
        });
        win.add_action(&action);
    }

    let arquivo = gio::Menu::new();
    arquivo.append_item(&menu_item("_Novo", "win.novo", Some("<Primary>n")));
    arquivo.append_item(&menu_item("_Salvar", "win.salvar", Some("<Primary>s")));
    arquivo.append_item(&menu_item(
        "_Fechar aba",
        "win.fechar-aba",
        Some("<Primary>w"),
    ));
    let projeto = gio::Menu::new();
    projeto.append_item(&menu_item("_Pasta…", "win.pasta", None));
    projeto.append_item(&menu_item("Se_rvidor…", "win.servidor", None));
    arquivo.append_section(None, &projeto);
    let sair = gio::Menu::new();
    sair.append_item(&menu_item("Sa_ir", "win.encerrar", Some("<Primary>q")));
    arquivo.append_section(None, &sair);

    let exec = gio::Menu::new();
    exec.append_item(&menu_item("_Executar", "win.executar", Some("F5")));
    exec.append_item(&menu_item("_Depurar", "win.depurar", Some("F6")));
    exec.append_item(&menu_item("_Parar", "win.parar", None));

    let debug = gio::Menu::new();
    debug.append_item(&menu_item("_Continuar", "win.continuar", None));
    debug.append_item(&menu_item("_Próximo", "win.proximo", Some("F10")));
    debug.append_item(&menu_item("E_ntrar", "win.entrar", Some("F11")));
    debug.append_item(&menu_item("_Sair", "win.sair", Some("<Shift>F11")));
    debug.append_item(&menu_item("Pon_to de parada", "win.ponto", Some("F9")));

    let ver = gio::Menu::new();
    ver.append_item(&menu_item("_Arquivos", "win.mostrar-arquivos", Some("F7")));
    ver.append_item(&menu_item("_Painel", "win.mostrar-painel", Some("F8")));
    ver.append_item(&menu_item("_Buscar", "win.buscar", Some("<Primary>f")));
    let aparencia = gio::Menu::new();
    aparencia.append_item(&menu_item(
        "Letra _maior",
        "win.letra-maior",
        Some("<Primary>equal"),
    ));
    aparencia.append_item(&menu_item(
        "Letra m_enor",
        "win.letra-menor",
        Some("<Primary>minus"),
    ));
    aparencia.append_item(&menu_item(
        "_Tamanho normal",
        "win.letra-normal",
        Some("<Primary>0"),
    ));
    let tema = gio::Menu::new();
    tema.append_item(&menu_radio("_Claro", "win.tema", "claro"));
    tema.append_item(&menu_radio("E_scuro", "win.tema", "escuro"));
    aparencia.append_section(None, &tema);
    ver.append_section(None, &aparencia);

    let ajuda = gio::Menu::new();
    ajuda.append_item(&menu_item("_Tutoriais", "win.tutoriais", None));
    ajuda.append_item(&menu_item("_Funções nativas", "win.ajuda", Some("F1")));

    let model = gio::Menu::new();
    model.append_submenu(Some("_Arquivo"), &arquivo);
    model.append_submenu(Some("_Executar"), &exec);
    model.append_submenu(Some("_Depurar"), &debug);
    model.append_submenu(Some("_Ver"), &ver);
    model.append_submenu(Some("A_juda"), &ajuda);

    if let Some(app) = win.application() {
        app.set_menubar(Some(&model));
    }
    win.set_show_menubar(true);
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
        menubar { padding: 2px 8px 0 8px; }
        label.aula-catalogo { font-family: monospace; }
        label.aula-ajuda-grupo { font-weight: bold; opacity: 0.7; padding-top: 6px; }
        paned.aula-paned > separator:hover {
            background-color: alpha(currentColor, 0.28);
        }
        popover.aula-signature > contents {
            padding: 4px 8px;
        }
        /* GtkSourceView default is 150px; that clips name, params and nativa. */
        popover.completion > contents,
        assistant.completion > contents {
            min-width: 32em;
        }
        popover.menu > contents {
            min-width: 0;
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

/// Documents copy created by the installer, if it already exists.
fn existing_exemplos_dir() -> Option<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(d) = expressa::exemplos::dest_dir() {
        candidates.push(d);
    }
    if let Some(docs) = glib::user_special_dir(glib::UserDirectory::Documents) {
        if !docs.as_os_str().is_empty() {
            candidates.push(docs.join("Expressa Aula").join("exemplos"));
        }
    }
    candidates.push(glib::user_data_dir().join("expressa-aula").join("exemplos"));
    for dir in candidates {
        if dir.join("linguagem").is_dir() {
            return Some(dir);
        }
    }
    None
}

fn data_dir() -> PathBuf {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let bundled = dir.join("res");
            if bundled.join("language-specs").is_dir() {
                return bundled;
            }
        }
    }
    let share = PathBuf::from("/usr/share/expressa-aula");
    if share.join("language-specs").is_dir() {
        return share;
    }
    Path::new(env!("CARGO_MANIFEST_DIR")).join("res")
}

/// A packaged build keeps GTK data beside the exe. Point GLib, GDK and
/// fontconfig at that tree before GTK starts. A dev build has no `share/`
/// next to the binary and keeps using `deps/gtk` from the environment.
fn prepare_runtime_prefix() {
    let Ok(exe) = std::env::current_exe() else {
        return;
    };
    let Some(dir) = exe.parent() else {
        return;
    };
    let schemas = dir.join("share").join("glib-2.0").join("schemas");
    if !schemas.join("gschemas.compiled").is_file() {
        return;
    }
    let fonts = dir.join("etc").join("fonts").join("fonts.conf");
    let loaders = dir
        .join("lib")
        .join("gdk-pixbuf-2.0")
        .join("2.10.0")
        .join("loaders.cache");
    let share = dir.join("share");
    unsafe {
        std::env::set_var("GSETTINGS_SCHEMA_DIR", &schemas);
        if fonts.is_file() {
            std::env::set_var("FONTCONFIG_FILE", &fonts);
        }
        if loaders.is_file() {
            std::env::set_var("GDK_PIXBUF_MODULE_FILE", &loaders);
        }
        if share.is_dir() {
            std::env::set_var("XDG_DATA_DIRS", &share);
        }
    }
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
            let text = u.files.read_file(&u.project, path).ok();
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
        .find(|(d, _)| exec::same_project_path(&d.name(), name))
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
                open_ajuda(&ui_h);
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
        let weak = Rc::downgrade(doc);
        add_shortcut_if(&keys, "Escape", move || {
            let (Some(vh), Some(doc)) = (vh.upgrade(), weak.upgrade()) else {
                return false;
            };
            vh.dismiss(&doc.view)
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
        u.files.read_file(&u.project, path)
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
    refresh_title(ui);
    refresh_diag_label(ui);
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
        u.files.write_file(&u.project, &name, &doc.text())
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
    let doc = current_doc(ui);
    let Ok(u) = ui.try_borrow() else { return };
    let title = match &doc {
        Some(doc) => {
            let star = if doc.is_dirty() { "● " } else { "" };
            format!("{star}{} — {} — Expressa Aula", doc.name(), u.project)
        }
        None if u.project.is_empty() => "Expressa Aula".to_string(),
        None => format!("{} — Expressa Aula", u.project),
    };
    u.window.set_title(Some(&title));
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
    let win = {
        let u = ui.borrow();
        u.theme_css.apply(dark);
        u.window.clone()
    };
    set_action_state_str(&win, "tema", if dark { "escuro" } else { "claro" });
    for d in all_docs(ui) {
        d.set_theme(scheme.as_ref(), dark);
    }
    ui.borrow().console.set_dark(dark);
}

fn popdown_popovers(widget: &impl IsA<gtk::Widget>) {
    let widget = widget.as_ref();
    if let Ok(p) = widget.clone().downcast::<gtk::Popover>() {
        p.popdown();
    }
    let mut child = widget.first_child();
    while let Some(c) = child {
        let next = c.next_sibling();
        popdown_popovers(&c);
        child = next;
    }
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
    let win = ui.borrow().window.clone();
    popdown_popovers(&win);
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
    {
        let ui_s = Rc::clone(ui);
        stack_tree.connect_row_activated(move |tree, path, _col| {
            let Some(model) = tree.model() else { return };
            let Ok(store) = model.downcast::<TreeStore>() else {
                return;
            };
            let Some(iter) = store.iter(path) else { return };
            let loc: String = store.get(&iter, 1);
            if let Some((file, line)) = loc.rsplit_once(':').and_then(|(f, n)| {
                n.parse()
                    .ok()
                    .filter(|l| *l > 0)
                    .map(|l| (f.to_string(), l))
            }) {
                goto_file_line(&ui_s, &file, line);
            }
        });
    }

    panel.append(&Label::new(Some(
        "Observados (ficam visíveis a cada passo)",
    )));
    panel.append(&scroll_tree(&watch_tree, 40));
    panel.append(&Label::new(Some("Variáveis")));
    panel.append(&scroll_tree(&vars_tree, 40));
    panel.append(&Label::new(Some("Pilha")));
    panel.append(&scroll_tree(&stack_tree, 40));
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
        set_status(ui, "executando… (Parar encerra)");
    }
}

fn set_debug_buttons(ui: &UiRc, paused: bool, running: bool) {
    let win = ui.borrow().window.clone();
    set_action_enabled(&win, "continuar", paused);
    set_action_enabled(&win, "proximo", paused);
    set_action_enabled(&win, "entrar", paused);
    set_action_enabled(&win, "sair", paused);
    set_action_enabled(&win, "parar", running);
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

fn fill_error_stack(ui: &UiRc, frames: &[exec::StackFrame]) {
    let u = ui.borrow();
    u.stack_store.clear();
    for f in frames {
        let loc = format!("{}:{}", f.file, f.line);
        let iter = u.stack_store.append(None);
        u.stack_store.set(&iter, &[(0, &f.name), (1, &loc)]);
    }
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

fn executar(ui: &UiRc, debug: bool) {
    if ui.borrow().run.is_some() {
        set_status(ui, "já tem um programa executando — aperte Parar primeiro");
        return;
    }
    let Some(doc) = current_doc(ui) else {
        set_status(ui, "abra um arquivo para executar");
        return;
    };
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
        Workspace::download(&u.files, &project).and_then(|mut ws| {
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
    let graficos = Rc::clone(&ui.borrow().graficos);
    let telas_graf = Rc::clone(&ui.borrow().telas_graf);
    console.clear();
    graficos.limpar();
    ui.borrow().telas_graf.limpar();
    clear_debug_views(ui);
    if let Some(d) = doc.diag.borrow().as_ref() {
        set_status(
            ui,
            &format!(
                "atenção: erro de sintaxe na linha {} — executando mesmo assim",
                d.line
            ),
        );
    } else {
        set_status(
            ui,
            &format!("{} {name}…", if debug { "depurando" } else { "executando" }),
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
                Ok(ExecEvent::Folha(folha)) => {
                    graficos.mostrar(folha);
                }
                Ok(ExecEvent::TelaGraf(tela)) => {
                    telas_graf.mostrar(tela);
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
                        ws.upload_changes(&u.files, &project)
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
                        if !f.stack.is_empty() {
                            console.stderr("pilha de chamadas:\n");
                            for (i, fr) in f.stack.iter().enumerate() {
                                console.error_link(&format!(
                                    "  {i}: {} em {}:{}\n",
                                    fr.name, fr.file, fr.line
                                ));
                            }
                            fill_error_stack(&ui_ev, &f.stack);
                        }
                        set_status(&ui_ev, "erro ao executar (clique no erro para ir à linha)");
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

/// First launch: open the Documents copy if the installer already put it there.
fn open_shipped_exemplos(ui: &UiRc) -> bool {
    let Some(dst) = existing_exemplos_dir() else {
        return false;
    };
    open_local_project(ui, dst.clone());
    let opened = matches!(&ui.borrow().files, Files::Local(_));
    if opened {
        set_status(
            ui,
            &format!("exemplos em {} — pode editar", path_for_display(&dst)),
        );
    }
    opened
}

/// Ask for a local project folder. True when the user picked one.
fn choose_local_folder(ui: &UiRc) -> bool {
    if ui.borrow().run.is_some() {
        set_status(ui, "pare o programa antes de trocar de projeto");
        return false;
    }
    pause_raise();
    let start = {
        let u = ui.borrow();
        match &u.files {
            Files::Local(p) => p.display().to_string(),
            _ => u.prefs.pasta.clone(),
        }
    };
    let window = ui.borrow().window.clone();
    let chosen = pick_folder(&window, &start);
    resume_raise();
    if let Some(path) = chosen {
        open_local_project(ui, path);
        true
    } else {
        false
    }
}

fn open_local_project(ui: &UiRc, path: PathBuf) {
    let path = expand_tilde(&path.to_string_lossy());
    let path = match path.canonicalize() {
        Ok(p) if p.is_dir() => p,
        Ok(_) => {
            set_status(ui, "escolha uma pasta");
            return;
        }
        Err(e) => {
            set_status(ui, &format!("pasta: {e}"));
            return;
        }
    };
    let same = { matches!(&ui.borrow().files, Files::Local(cur) if cur == &path) };
    if same {
        refresh_tree(ui);
        return;
    }
    let ui_c = Rc::clone(ui);
    after_unsaved(ui, move || apply_local(&ui_c, path));
}

fn apply_local(ui: &UiRc, path: PathBuf) {
    close_project_tabs(ui);
    let name = path
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| path.display().to_string());
    let shown = path.display().to_string();
    {
        let mut u = ui.borrow_mut();
        u.files.shutdown();
        u.files = Files::Local(path.clone());
        u.project = name;
        u.prefs.pasta = shown.clone();
        u.prefs.save();
        u.file_cache.borrow_mut().clear();
    }
    set_local_project_path(ui, &path);
    finish_open(ui);
}

/// URL and student name. Files on that server live in `aula-data/aluno`.
fn ask_server(ui: &UiRc) {
    if ui.borrow().run.is_some() {
        set_status(ui, "pare o programa antes de trocar de projeto");
        return;
    }
    let (url, aluno) = {
        let u = ui.borrow();
        let url = if u.prefs.servidor.is_empty() {
            DEFAULT_URL.to_string()
        } else {
            u.prefs.servidor.clone()
        };
        let aluno = if u.files.is_remote() {
            u.project.clone()
        } else {
            "local".to_string()
        };
        (url, aluno)
    };
    let window = ui.borrow().window.clone();
    pause_raise();
    let answer = perguntar_servidor(&window, &url, &aluno);
    resume_raise();
    let Some((url, aluno)) = answer else {
        return;
    };
    let aluno = aluno.trim().to_string();
    if !valid_aluno(&aluno) {
        set_status(ui, "nome do aluno: letras, números, _ e -");
        return;
    }
    let url = normalize_url(&url);
    match Rpc::connect(&url) {
        Ok(rpc) => {
            let ui_c = Rc::clone(ui);
            after_unsaved(ui, move || apply_remote(&ui_c, rpc, url, aluno));
        }
        Err(e) => set_status(ui, &format!("servidor: {e}")),
    }
}

fn apply_remote(ui: &UiRc, rpc: Rpc, url: String, aluno: String) {
    close_project_tabs(ui);
    let tip = format!("{url} — aula-data/{aluno}");
    {
        let mut u = ui.borrow_mut();
        u.files.shutdown();
        u.files = Files::Remote(rpc);
        u.project = aluno.clone();
        u.prefs.servidor = url;
        u.prefs.save();
        u.file_cache.borrow_mut().clear();
    }
    set_project_path(ui, &tip, Some(&tip));
    finish_open(ui);
}

fn after_unsaved(ui: &UiRc, then: impl FnOnce() + 'static) {
    if ui.borrow().run.is_some() {
        set_status(ui, "pare o programa antes de trocar de projeto");
        return;
    }
    let dirty = dirty_docs(ui);
    if dirty.is_empty() {
        then();
    } else {
        confirm_unsaved(ui, dirty, then);
    }
}

fn close_project_tabs(ui: &UiRc) {
    for d in all_docs(ui) {
        d.mark_saved();
        remove_tab(ui, &d);
    }
}

/// Show the project tree. No file is opened; the user picks one.
fn finish_open(ui: &UiRc) {
    refresh_tree(ui);
    refresh_title(ui);
    refresh_diag_label(ui);
}

fn normalize_url(url: &str) -> String {
    let url = url.trim();
    if url.is_empty() {
        DEFAULT_URL.to_string()
    } else if url.starts_with("http://") || url.starts_with("https://") {
        url.to_string()
    } else {
        format!("http://{url}")
    }
}

/// Folder picker. `start` is preselected when it is an existing directory.
fn pick_folder(parent: &impl IsA<gtk::Window>, start: &str) -> Option<PathBuf> {
    let dialog = gtk::FileChooserNative::builder()
        .title("Pasta do projeto")
        .transient_for(parent)
        .modal(true)
        .action(gtk::FileChooserAction::SelectFolder)
        .accept_label("Abrir")
        .cancel_label("Cancelar")
        .build();
    let start_path = expand_tilde(start);
    if start_path.is_dir() {
        let _ = dialog.set_current_folder(Some(&gtk::gio::File::for_path(&start_path)));
    }
    let loop_ = glib::MainLoop::new(None, false);
    let chosen = Rc::new(RefCell::new(None::<PathBuf>));
    {
        let loop_ = loop_.clone();
        let chosen = Rc::clone(&chosen);
        dialog.connect_response(move |dialog, resp| {
            if resp == gtk::ResponseType::Accept {
                if let Some(file) = dialog.file() {
                    *chosen.borrow_mut() = file.path();
                }
            }
            dialog.destroy();
            loop_.quit();
        });
    }
    dialog.show();
    loop_.run();
    chosen.borrow().clone()
}

fn perguntar_servidor(
    parent: &impl IsA<gtk::Window>,
    url: &str,
    aluno: &str,
) -> Option<(String, String)> {
    let dialog = gtk::Window::builder()
        .transient_for(parent)
        .modal(true)
        .title("Servidor remoto")
        .default_width(460)
        .build();
    let v = GtkBox::new(Orientation::Vertical, 10);
    v.set_margin_start(16);
    v.set_margin_end(16);
    v.set_margin_top(16);
    v.set_margin_bottom(16);
    let info = Label::new(Some("Os arquivos ficam em aula-data/aluno nesse servidor."));
    info.set_wrap(true);
    info.set_xalign(0.0);
    let url_entry = Entry::builder().text(url).hexpand(true).build();
    let aluno_entry = Entry::builder().text(aluno).hexpand(true).build();
    let buttons = GtkBox::new(Orientation::Horizontal, 8);
    buttons.set_halign(gtk::Align::End);
    let cancel = Button::with_label("Cancelar");
    let ok = Button::with_label("Conectar");
    ok.add_css_class("suggested-action");
    buttons.append(&cancel);
    buttons.append(&ok);
    v.append(&info);
    v.append(&Label::new(Some("endereço:")));
    v.append(&url_entry);
    v.append(&Label::new(Some("aluno:")));
    v.append(&aluno_entry);
    v.append(&buttons);
    dialog.set_child(Some(&v));

    let loop_ = glib::MainLoop::new(None, false);
    let out = Rc::new(RefCell::new(None::<(String, String)>));
    let accept = Rc::new({
        let loop_q = loop_.clone();
        let out_q = Rc::clone(&out);
        let url_q = url_entry.clone();
        let aluno_q = aluno_entry.clone();
        let dialog_q = dialog.clone();
        move || {
            *out_q.borrow_mut() = Some((url_q.text().to_string(), aluno_q.text().to_string()));
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
        aluno_entry.connect_activate(move |_| accept());
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
    url_entry.grab_focus();
    loop_.run();
    out.borrow_mut().take()
}

fn refresh_tree(ui: &UiRc) {
    let (entries, projeto, err, onde) = {
        let u = ui.borrow();
        let projeto = u.project.clone();
        let onde = match &u.files {
            Files::None => "sem pasta".to_string(),
            Files::Local(p) => p.display().to_string(),
            Files::Remote(_) => format!("servidor {} \\ {}", u.prefs.servidor, projeto),
        };
        match u.files.list_tree(&projeto) {
            Ok(p) => (p, projeto, None, onde),
            Err(e) => (Vec::new(), projeto, Some(e), onde),
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
        Some(e) => set_status(ui, &e),
        None => set_status(ui, &format!("{onde} — {} itens", entries.len())),
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
    let (px, py) = tree.translate_coordinates(&window, x, y).unwrap_or((x, y));
    let pop = gtk::Popover::new();
    pop.add_css_class("menu");
    pop.set_parent(&window);
    pop.set_has_arrow(false);
    pop.set_autohide(true);
    pop.set_pointing_to(Some(&gtk::gdk::Rectangle::new(
        px.round() as i32,
        py.round() as i32,
        1,
        1,
    )));
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
        btn.set_halign(gtk::Align::Start);
        btn.set_hexpand(true);
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
    let novo_arq = if is_dir && !rel.is_empty() {
        "Novo arquivo nesta pasta…"
    } else {
        "Novo arquivo…"
    };
    let nova_pasta = if is_dir && !rel.is_empty() {
        "Nova pasta nesta pasta…"
    } else {
        "Nova pasta…"
    };
    add_item(
        &box_,
        novo_arq,
        Box::new({
            let ui = Rc::clone(ui);
            let dir = dir.clone();
            move || criar_arquivo(&ui, &dir)
        }),
    );
    add_item(
        &box_,
        nova_pasta,
        Box::new({
            let ui = Rc::clone(ui);
            let dir = dir.clone();
            move || criar_pasta(&ui, &dir)
        }),
    );
    if !rel.is_empty() {
        add_item(
            &box_,
            "Renomear…",
            Box::new({
                let ui = Rc::clone(ui);
                let rel = rel.clone();
                move || renomear_entrada(&ui, &rel)
            }),
        );
        add_item(
            &box_,
            "Apagar",
            Box::new({
                let ui = Rc::clone(ui);
                let rel = rel.clone();
                move || confirmar_apagar(&ui, &rel)
            }),
        );
    }
    pop.set_child(Some(&box_));
    glib::idle_add_local_once(move || {
        pop.popup();
    });
}

fn criar_arquivo(ui: &UiRc, dir: &str) {
    let window = ui.borrow().window.clone();
    let Some(name) = perguntar_linha(&window, "Nome do arquivo:") else {
        return;
    };
    let name = name.trim().to_string();
    if name.is_empty() {
        return;
    }
    let name = if name.contains('.') {
        name
    } else {
        format!("{name}.lep")
    };
    let path = join_path(dir, &name);
    let result = {
        let u = ui.borrow();
        u.files.write_file(&u.project, &path, "escreva(\"Olá\")\n")
    };
    match result {
        Ok(()) => {
            refresh_tree(ui);
            open_path(ui, &path);
            set_status(ui, &format!("criado {path}"));
        }
        Err(e) => set_status(ui, &format!("criar: {e}")),
    }
}

fn criar_pasta(ui: &UiRc, dir: &str) {
    let window = ui.borrow().window.clone();
    let Some(name) = perguntar_linha(&window, "Nome da pasta:") else {
        return;
    };
    if name.trim().is_empty() {
        return;
    }
    let path = join_path(dir, name.trim());
    let result = {
        let u = ui.borrow();
        u.files.mkdir(&u.project, &path)
    };
    match result {
        Ok(()) => {
            refresh_tree(ui);
            set_status(ui, &format!("pasta {path}"));
        }
        Err(e) => set_status(ui, &format!("pasta: {e}")),
    }
}

fn renomear_entrada(ui: &UiRc, rel: &str) {
    let window = ui.borrow().window.clone();
    let base = rel.rsplit('/').next().unwrap_or(rel).to_string();
    let Some(name) = perguntar_linha(&window, &format!("Renomear `{base}`:")) else {
        return;
    };
    if name.trim().is_empty() {
        return;
    }
    let to = join_path(&parent_dir(rel, false), name.trim());
    let result = {
        let u = ui.borrow();
        u.files.rename(&u.project, rel, &to)
    };
    match result {
        Ok(()) => {
            for d in all_docs(ui) {
                let n = d.name();
                if n == rel {
                    d.set_path(to.clone());
                } else if let Some(rest) = n.strip_prefix(&format!("{rel}/")) {
                    d.set_path(format!("{to}/{rest}"));
                }
            }
            refresh_tree(ui);
            refresh_title(ui);
            set_status(ui, &format!("{rel} → {to}"));
        }
        Err(e) => set_status(ui, &format!("renomear: {e}")),
    }
}

fn confirmar_apagar(ui: &UiRc, rel: &str) {
    if rel.is_empty() {
        return;
    }
    let window = ui.borrow().window.clone();
    let then = {
        let ui = Rc::clone(ui);
        let rel = rel.to_string();
        move || {
            let result = {
                let u = ui.borrow();
                u.files.delete_file(&u.project, &rel)
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

/// Ask Windows to put our top-level window in front. A newly started process
/// is often not allowed to cover the window that already has focus, so the
/// IDE was opening maximized and invisible behind it.
fn schedule_raise() {
    #[cfg(windows)]
    win32_foreground::schedule();
}

/// Stop stealing focus while a folder or server dialog is open.
fn pause_raise() {
    #[cfg(windows)]
    win32_foreground::pause();
}

fn resume_raise() {
    #[cfg(windows)]
    win32_foreground::resume();
}

#[cfg(windows)]
mod win32_foreground {
    use std::ffi::c_void;
    use std::sync::atomic::{AtomicBool, AtomicI64, AtomicUsize, Ordering};

    static HWND: AtomicUsize = AtomicUsize::new(0);
    static AREA: AtomicI64 = AtomicI64::new(0);
    static PAUSED: AtomicBool = AtomicBool::new(false);

    pub fn pause() {
        PAUSED.store(true, Ordering::SeqCst);
    }

    pub fn resume() {
        PAUSED.store(false, Ordering::SeqCst);
    }

    const SW_SHOWMAXIMIZED: i32 = 3;
    const KEYEVENTF_KEYUP: u32 = 2;
    const VK_MENU: u8 = 0x12;

    #[repr(C)]
    struct Rect {
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
    }

    unsafe extern "system" {
        fn EnumWindows(
            cb: Option<unsafe extern "system" fn(*mut c_void, isize) -> i32>,
            lparam: isize,
        ) -> i32;
        fn GetWindowThreadProcessId(hwnd: *mut c_void, pid: *mut u32) -> u32;
        fn GetCurrentProcessId() -> u32;
        fn GetCurrentThreadId() -> u32;
        fn IsWindowVisible(hwnd: *mut c_void) -> i32;
        fn GetWindowTextW(hwnd: *mut c_void, buf: *mut u16, max: i32) -> i32;
        fn GetWindowRect(hwnd: *mut c_void, rect: *mut Rect) -> i32;
        fn GetForegroundWindow() -> *mut c_void;
        fn SetForegroundWindow(hwnd: *mut c_void) -> i32;
        fn BringWindowToTop(hwnd: *mut c_void) -> i32;
        fn ShowWindow(hwnd: *mut c_void, cmd: i32) -> i32;
        fn AttachThreadInput(from: u32, to: u32, attach: i32) -> i32;
        fn keybd_event(vk: u8, scan: u8, flags: u32, extra: usize);
        fn GetModuleHandleW(name: *const u16) -> *mut c_void;
        fn LoadImageW(
            inst: *mut c_void,
            name: *const u16,
            typ: u32,
            cx: i32,
            cy: i32,
            flags: u32,
        ) -> *mut c_void;
        fn SendMessageW(hwnd: *mut c_void, msg: u32, wparam: usize, lparam: isize) -> isize;
        fn SetClassLongPtrW(hwnd: *mut c_void, index: i32, new_long: isize) -> isize;
    }

    const IMAGE_ICON: u32 = 1;
    const LR_SHARED: u32 = 0x8000;
    const WM_SETICON: u32 = 0x0080;
    const ICON_SMALL: usize = 0;
    const ICON_BIG: usize = 1;
    const GCLP_HICON: i32 = -14;
    const GCLP_HICONSM: i32 = -34;

    static ICON_SET: AtomicBool = AtomicBool::new(false);

    /// The file icon is resource 1. Point the window class at it so the
    /// taskbar and the title bar use the same picture as the exe.
    fn apply_icon(hwnd: *mut c_void) {
        if ICON_SET.swap(true, Ordering::SeqCst) {
            return;
        }
        unsafe {
            let module = GetModuleHandleW(std::ptr::null());
            let id = 1u16 as usize as *const u16;
            let big = LoadImageW(module, id, IMAGE_ICON, 32, 32, LR_SHARED);
            let small = LoadImageW(module, id, IMAGE_ICON, 16, 16, LR_SHARED);
            if !big.is_null() {
                SendMessageW(hwnd, WM_SETICON, ICON_BIG, big as isize);
                SetClassLongPtrW(hwnd, GCLP_HICON, big as isize);
            }
            if !small.is_null() {
                SendMessageW(hwnd, WM_SETICON, ICON_SMALL, small as isize);
                SetClassLongPtrW(hwnd, GCLP_HICONSM, small as isize);
            }
        }
    }

    enum Step {
        Wait,
        Done,
        Failed,
    }

    pub fn schedule() {
        let ticks = std::cell::Cell::new(0i32);
        let fails = std::cell::Cell::new(0i32);
        gtk::glib::timeout_add_local(std::time::Duration::from_millis(50), move || {
            ticks.set(ticks.get() + 1);
            match poke() {
                Step::Done => gtk::glib::ControlFlow::Break,
                Step::Failed => {
                    fails.set(fails.get() + 1);
                    if fails.get() >= 3 {
                        gtk::glib::ControlFlow::Break
                    } else {
                        gtk::glib::ControlFlow::Continue
                    }
                }
                Step::Wait => {
                    if ticks.get() >= 40 {
                        gtk::glib::ControlFlow::Break
                    } else {
                        gtk::glib::ControlFlow::Continue
                    }
                }
            }
        });
    }

    fn poke() -> Step {
        let hwnd = our_main_hwnd();
        if !hwnd.is_null() {
            apply_icon(hwnd);
        }
        if PAUSED.load(Ordering::SeqCst) {
            return Step::Done;
        }
        if hwnd.is_null() {
            return Step::Wait;
        }
        unsafe {
            if GetForegroundWindow() == hwnd {
                return Step::Done;
            }
            let fg = GetForegroundWindow();
            let mut fg_pid = 0u32;
            let fg_thread = GetWindowThreadProcessId(fg, &mut fg_pid);
            let our_thread = GetCurrentThreadId();
            let attached = fg_thread != 0
                && fg_thread != our_thread
                && AttachThreadInput(fg_thread, our_thread, 1) != 0;
            ShowWindow(hwnd, SW_SHOWMAXIMIZED);
            BringWindowToTop(hwnd);
            // A bare Alt press is what lets SetForegroundWindow succeed when
            // this process was not started by the foreground window. Release
            // it before taking focus so the menu bar does not open.
            keybd_event(VK_MENU, 0x38, 0, 0);
            keybd_event(VK_MENU, 0x38, KEYEVENTF_KEYUP, 0);
            SetForegroundWindow(hwnd);
            if attached {
                AttachThreadInput(fg_thread, our_thread, 0);
            }
            if GetForegroundWindow() == hwnd {
                Step::Done
            } else {
                Step::Failed
            }
        }
    }

    fn our_main_hwnd() -> *mut c_void {
        HWND.store(0, Ordering::Relaxed);
        AREA.store(0, Ordering::Relaxed);
        unsafe {
            EnumWindows(Some(enum_our_window), 0);
        }
        HWND.load(Ordering::Relaxed) as *mut c_void
    }

    unsafe extern "system" fn enum_our_window(hwnd: *mut c_void, _lparam: isize) -> i32 {
        unsafe {
            let mut pid = 0u32;
            GetWindowThreadProcessId(hwnd, &mut pid);
            if pid != GetCurrentProcessId() || IsWindowVisible(hwnd) == 0 {
                return 1;
            }
            let mut title = [0u16; 4];
            if GetWindowTextW(hwnd, title.as_mut_ptr(), title.len() as i32) <= 0 {
                return 1;
            }
            let mut rect = Rect {
                left: 0,
                top: 0,
                right: 0,
                bottom: 0,
            };
            if GetWindowRect(hwnd, &mut rect) == 0 {
                return 1;
            }
            let area = (rect.right - rect.left) as i64 * (rect.bottom - rect.top) as i64;
            if area > AREA.load(Ordering::Relaxed) {
                AREA.store(area, Ordering::Relaxed);
                HWND.store(hwnd as usize, Ordering::Relaxed);
            }
            1
        }
    }
}
