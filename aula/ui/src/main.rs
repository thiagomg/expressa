#![cfg_attr(windows, windows_subsystem = "windows")]
// TreeView, TreeStore and MessageDialog are deprecated since GTK 4.10.
#![allow(deprecated)]

mod abas;
mod ajuda;
mod analysis;
mod app;
mod assist;
mod atalhos;
mod background;
mod busca_projeto;
mod cartesiano;
mod conflitos;
mod console;
mod debug_sources;
#[cfg(test)]
mod debug_ui_tests;
mod depuracao;
mod docs;
mod documentos;
mod editing;
mod editor;
mod escolher;
mod exec;
mod execucao;
mod execution_state;
#[cfg(test)]
pub(crate) mod gtk_test;
mod identidade;
mod indent;
mod janela_graf;
mod lang;
mod menu_teclado;
mod navegar;
mod operations;
mod painel_repl;
mod pares;
mod pendencias;
mod pendencias_ui;
mod prefs;
mod projeto;
mod projetos_menu;
mod recuperacao;
#[cfg(test)]
mod responsiveness_tests;
mod rpc;
mod sessao;
mod shell;
mod sincronizacao;
mod store;
mod tutoriais;

// Domain modules own the handlers; this file wires the window. Re-exports keep
// existing `crate::open_path` / `crate::UiRc` paths used by sessao, abas, etc.
pub(crate) use app::*;
pub(crate) use depuracao::*;
pub(crate) use editor::*;
pub(crate) use execucao::*;
pub(crate) use projeto::*;
pub(crate) use shell::*;

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

use gtk::gdk::Key;
use gtk::gio::{self, prelude::*};
use gtk::glib;
use gtk::prelude::*;
use gtk::{
    Application, ApplicationWindow, Box as GtkBox, Button, CellRendererText, Entry, Label,
    Orientation, Paned, PolicyType, Revealer, ScrolledWindow, TreeStore, TreeView, TreeViewColumn,
};
use sourceview5::prelude::*;
use sourceview5::{SearchSettings, StyleSchemeManager};

use console::Console;
use docs::Doc;
#[cfg(test)]
use exec::Workspace;
use execution_state::{Event as ExecutionEvent, ExecutionState};
use prefs::{FontCss, Prefs, ThemeCss, expand_tilde, path_for_display};
use store::Files;

fn main() {
    prepare_runtime_prefix();
    rpc::install_signal_handlers();
    let app = Application::builder().application_id(APP_ID).build();
    let prefs = Prefs::load();
    app.connect_activate(move |app| {
        build_ui_with_prefs(app, true, prefs.clone());
    });
    app.run();
}

#[cfg(test)]
fn build_ui_state(app: &Application, startup: bool) -> UiRc {
    build_ui_with_prefs(app, startup, Prefs::default())
}

/// Preferences are read before entering GTK's event loop; tests inject defaults.
fn build_ui_with_prefs(app: &Application, startup: bool, prefs: Prefs) -> UiRc {
    // ibus + GTK4 leaves the dead key (´ ~ `) stuck as preedit after composing
    // á/ã. GTK's built-in input method composes dead keys itself.
    // GTK_IM_MODULE in the environment still takes precedence over this.
    if let Some(settings) = gtk::Settings::default() {
        settings.set_gtk_im_module(Some("gtk-im-context-simple"));
    }

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
    let barra_exec = make_run_bar();
    let status = Label::new(Some("escolha uma pasta para o projeto"));
    status.set_xalign(0.0);
    status.set_hexpand(true);
    status.add_css_class("dim-label");
    let cursor_pos = Label::new(Some("1:1"));
    cursor_pos.set_xalign(1.0);
    cursor_pos.add_css_class("dim-label");
    cursor_pos.set_width_chars(8);
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
        depuracao: Depuracao {
            watches: Vec::new(),
            vars_store,
            stack_store,
            watch_store,
            watch_entry,
            barra_exec,
        },
        execucao: Execucao {
            run: None,
            execution: ExecutionState::default(),
            operations: operations::Operations::new(),
            console,
            graficos,
            telas_graf,
            args_entry,
            execution_sources: Arc::new(debug_sources::Sources::new()),
            last_execution_location: None,
            execution_breakpoints: Vec::new(),
            execution_editors: HashMap::new(),
            source_viewer: None,
        },
        project_epoch: 0,
        file_operations: HashMap::new(),
        next_file_operation: 0,
        navigation_epoch: 0,
        listing_epoch: 0,
        remote_menu: gio::Menu::new(),
        projects_menu: gio::Menu::new(),
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
        search_settings,
        search_entry,
        replace_entry,
        search_revealer,
        status,
        cursor_pos,
        diag_label,
        window: win.clone(),
        languages,
        schemes,
        prefs,
        font_css,
        theme_css,
        help: None,
        closing: std::cell::Cell::new(false),
        geracao_arvore: 0,
        abas_fechadas: Vec::new(),
        sessao_pronta: false,
        recuperacao_aberta: false,
        recovery_timer: RefCell::new(None),
        backup_busy: std::cell::Cell::new(false),
        backup_review: RefCell::new(None),
        sync_window: None,
        painel_inferior: gtk::Notebook::new(),
        busca: None,
        repl: painel_repl::PainelRepl::new(),
        ajuda_win: RefCell::new(None),
    }));
    let help = make_help(&ui);
    ui.borrow_mut().help = Some(help);
    let busca = busca_projeto::PainelBusca::new(&ui);
    ui.borrow_mut().busca = Some(busca);

    let btn_smaller = Button::with_label("A−");
    btn_smaller.set_tooltip_text(Some("Letra menor (Ctrl+-)"));
    let btn_bigger = Button::with_label("A+");
    btn_bigger.set_tooltip_text(Some("Letra maior (Ctrl+=)"));

    let args_row = GtkBox::new(Orientation::Horizontal, 8);
    args_row.set_margin_start(8);
    args_row.set_margin_end(8);
    args_row.append(&Label::new(Some("argumentos:")));
    args_row.append(&ui.borrow().execucao.args_entry);
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
        .child(&ui.borrow().execucao.console.view)
        .build();

    let debug_panel = make_debug_panel(&ui);
    const DEBUG_PANEL_WIDTH: i32 = 320;
    debug_panel.set_hexpand(false);

    install_css();

    let bottom = GtkBox::new(Orientation::Vertical, 0);
    bottom.set_hexpand(true);
    {
        let u = ui.borrow();
        bottom.append(&u.execucao.graficos.dock);
        let painel = &u.painel_inferior;
        painel.append_page(&output_scroll, Some(&Label::new(Some("Saída"))));
        if let Some(b) = &u.busca {
            painel.append_page(&b.widget, Some(&Label::new(Some("Busca"))));
        }
        painel.append_page(&u.repl.widget, Some(&Label::new(Some("REPL"))));
        painel.set_vexpand(true);
        bottom.append(painel);
    }
    let editor_col = editor_panes(&ui.borrow().notebook, &bottom);

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
    {
        // Search options, on the settings every tab's search shares.
        let settings = ui.borrow().search_settings.clone();
        let opcao = |rotulo: &str, dica: &str, f: fn(&SearchSettings, bool)| {
            let b = gtk::ToggleButton::with_label(rotulo);
            b.set_tooltip_text(Some(dica));
            let settings = settings.clone();
            b.connect_toggled(move |b| f(&settings, b.is_active()));
            search_bar.append(&b);
        };
        opcao(
            "Aa",
            "Diferenciar maiúsculas de minúsculas",
            SearchSettings::set_case_sensitive,
        );
        opcao(
            "Palavra",
            "Só a palavra inteira (`soma` não acha `somatorio`)",
            SearchSettings::set_at_word_boundaries,
        );
        opcao(
            ".*",
            "Expressão regular (ex.: `nota[0-9]`)",
            SearchSettings::set_regex_enabled,
        );
    }
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
    status_row.append(&ui.borrow().execucao.operations.button);
    status_row.append(&ui.borrow().cursor_pos);
    status_row.append(&ui.borrow().diag_label);
    status_row.append(&ui.borrow().project_path);

    install_menubar(&win, &ui, &file_scroll, &debug_panel);
    root.append(&args_row);
    root.append(&ui.borrow().depuracao.barra_exec.caixa);
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
                if let Some(d) = current_doc(&ui_t) {
                    refresh_cursor(&ui_t, &d.buffer);
                    remember_arquivo(&ui_t, &d);
                }
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
            let console = Rc::clone(&ui_o.borrow().execucao.console);
            if console.is_reading() {
                console.keep_cursor_in_input();
                return;
            }
            if let Some((file, line)) = console
                .line_at(x, y)
                .and_then(|l| console::parse_error_location(&l))
            {
                navigate_execution_error(&ui_o, &file, line, false);
            }
        });
        ui.borrow().execucao.console.view.add_controller(click);
    }
    {
        // Enter in the output pane answers `leia`.
        let console = Rc::clone(&ui.borrow().execucao.console);
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
            close_current_tab(ui);
        });
        k(&shortcuts, "<Primary>f", show_search);
        k(&shortcuts, "<Primary>h", show_search);
        k(&shortcuts, "<Primary>g", ir_para_linha);
        for accel in ["<Primary>equal", "<Primary>plus", "<Primary>KP_Add"] {
            k(&fkeys, accel, |ui| zoom(ui, 1));
        }
        for accel in ["<Primary>minus", "<Primary>KP_Subtract"] {
            k(&fkeys, accel, |ui| zoom(ui, -1));
        }
        k(&fkeys, "<Primary>0", |ui| zoom(ui, 0));
        // Capture: the editor would take Ctrl+Tab to move focus out.
        k(&fkeys, "<Primary>Tab", |ui| abas::vizinha(ui, 1));
        k(&fkeys, "<Primary>Page_Down", |ui| abas::vizinha(ui, 1));
        // Shift+Tab arrives as ISO_Left_Tab.
        for accel in ["<Primary><Shift>Tab", "<Primary><Shift>ISO_Left_Tab"] {
            k(&fkeys, accel, |ui| abas::vizinha(ui, -1));
        }
        k(&fkeys, "<Primary>Page_Up", |ui| abas::vizinha(ui, -1));
        for n in 1..=9u32 {
            let ui = Rc::clone(&ui);
            add_shortcut(&fkeys, &format!("<Alt>{n}"), move || abas::numero(&ui, n));
        }
        k(&shortcuts, "<Primary><Shift>t", abas::reabrir);
        k(&shortcuts, "<Primary>p", navegar::abrir_rapido);
        k(&shortcuts, "<Primary><Shift>f", buscar_no_projeto);
        k(&shortcuts, "<Primary><Shift>o", navegar::funcoes_do_arquivo);
        k(&shortcuts, "<Primary>question", |ui| {
            atalhos::mostrar(&ui.borrow().window)
        });
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
        let repl = Rc::downgrade(&ui.borrow().repl);
        let weak = Rc::downgrade(&ui);
        win.connect_unrealize(move |_| {
            if let Some(ui) = weak.upgrade() {
                ui.borrow().closing.set(true);
                ui.borrow().execucao.operations.cancel_all();
                sessao::stop_copies(&ui);
                for doc in all_docs(&ui) {
                    doc.cancel_pending();
                }
            }
            if let Some(repl) = repl.upgrade() {
                repl.encerrar();
            }
        });
        let ui_close = Rc::clone(&ui);
        win.connect_close_request(move |w| {
            if ui_close.borrow().closing.get() {
                sessao::salvar(&ui_close);
                sessao::limpar_copias(&ui_close);
                fechar_janelas_filhas(&ui_close);
                ui_close.borrow().closing.set(true);
                ui_close.borrow().execucao.operations.cancel_all();
                sessao::stop_copies(&ui_close);
                for doc in all_docs(&ui_close) {
                    doc.cancel_pending();
                }
                ui_close.borrow().repl.encerrar();
                ui_close.borrow_mut().files.shutdown();
                return glib::Propagation::Proceed;
            }
            let dirty: Vec<Rc<Doc>> = dirty_docs(&ui_close);
            if dirty.is_empty() {
                sessao::salvar(&ui_close);
                sessao::limpar_copias(&ui_close);
                fechar_janelas_filhas(&ui_close);
                ui_close.borrow().closing.set(true);
                ui_close.borrow().execucao.operations.cancel_all();
                sessao::stop_copies(&ui_close);
                for doc in all_docs(&ui_close) {
                    doc.cancel_pending();
                }
                ui_close.borrow().repl.encerrar();
                ui_close.borrow_mut().files.shutdown();
                return glib::Propagation::Proceed;
            }
            let ui_c = Rc::clone(&ui_close);
            let w = w.clone();
            confirm_unsaved(&ui_close, dirty, move || {
                ui_c.borrow().closing.set(true);
                w.close();
            });
            glib::Propagation::Stop
        });
    }

    apply_theme(&ui);
    if !startup {
        return ui;
    }
    sessao::iniciar_copias(&ui);
    win.maximize();
    win.present();
    schedule_raise();
    {
        let ui_p = Rc::clone(&ui);
        glib::timeout_add_local_once(std::time::Duration::from_millis(250), move || {
            sessao::restaurar_sem_projeto(&ui_p, move |ui_p, offered| {
                if offered {
                    set_project_path(ui_p, "", None);
                    return;
                }
                let pasta = ui_p.borrow().prefs.pasta.clone();
                async_result(
                    ui_p,
                    "Abrindo projeto inicial",
                    true,
                    move || {
                        let path = expand_tilde(&pasta);
                        let wide = store::pasta_larga(&path);
                        let valid = !pasta.is_empty() && !wide && path.is_dir();
                        Ok((path, valid, wide))
                    },
                    move |ui_p, result| {
                        let (pasta_path, valid, wide) = match result {
                            Ok(value) => value,
                            Err(e) => {
                                set_status(ui_p, &e);
                                return;
                            }
                        };
                        let opened = if valid {
                            open_local_project(ui_p, pasta_path);
                            true
                        } else {
                            if wide {
                                let mut u = ui_p.borrow_mut();
                                u.prefs.pasta.clear();
                                u.prefs.save();
                                drop(u);
                                set_project_path(ui_p, "", None);
                            }
                            if open_shipped_exemplos(ui_p) {
                                true
                            } else {
                                choose_local_folder(ui_p);
                                false
                            }
                        };
                        schedule_raise();
                        if !opened {
                            set_status(ui_p, "escolha a pasta do projeto");
                        }
                        if !lang_ok {
                            set_status(
                                ui_p,
                                "aviso: gramática Expressa não carregou; o texto fica sem cores",
                            );
                        }
                    },
                );
            });
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
    ui
}
