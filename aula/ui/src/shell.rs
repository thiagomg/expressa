//! Window chrome: menubar, shortcuts, CSS, status, dialogs, and platform startup.

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use gtk::gio::{self, prelude::*};
use gtk::glib;
use gtk::prelude::*;
use gtk::{
    ApplicationWindow, Box as GtkBox, Button, Entry, Label, Orientation, Paned, ScrolledWindow,
};
use sourceview5::LanguageManager;

use crate::docs::Doc;
use crate::prefs::path_for_display;
use crate::*;

pub(crate) fn editor_panes(
    editor: &impl IsA<gtk::Widget>,
    bottom: &impl IsA<gtk::Widget>,
) -> Paned {
    let pane = Paned::new(Orientation::Vertical);
    pane.add_css_class("aula-paned");
    pane.set_wide_handle(false);
    pane.set_start_child(Some(editor));
    pane.set_end_child(Some(bottom));
    pane.set_resize_start_child(true);
    pane.set_resize_end_child(false);
    pane.set_shrink_start_child(true);
    pane.set_shrink_end_child(false);
    pane
}

pub(crate) fn encerrar(ui: &UiRc) {
    let w = ui.borrow().window.clone();
    w.close();
}

pub(crate) fn fechar_janelas_filhas(ui: &UiRc) {
    let (graficos, telas_graf, ajuda) = {
        let u = ui.borrow();
        (
            Rc::clone(&u.execucao.graficos),
            Rc::clone(&u.execucao.telas_graf),
            u.ajuda_win.borrow_mut().take(),
        )
    };
    graficos.fechar_janela_destacada();
    telas_graf.fechar_todas();
    if let Some(w) = ajuda {
        w.close();
    }
}

pub(crate) fn open_ajuda(ui: &UiRc) {
    if let Some(w) = ui.borrow().ajuda_win.borrow().clone() {
        if w.is_visible() {
            w.present();
            return;
        }
        ui.borrow().ajuda_win.borrow_mut().take();
        w.close();
    }
    let w = ajuda::open(&ui.borrow().window);
    w.set_hide_on_close(false);
    w.connect_close_request({
        let ui = Rc::clone(ui);
        move |_| {
            ui.borrow().ajuda_win.borrow_mut().take();
            glib::Propagation::Proceed
        }
    });
    ui.borrow().ajuda_win.borrow_mut().replace(w);
}

pub(crate) fn connect(button: &impl IsA<Button>, ui: &UiRc, f: impl Fn(&UiRc) + 'static) {
    let ui = Rc::clone(ui);
    button.connect_clicked(move |_| f(&ui));
}

pub(crate) fn add_shortcut(
    controller: &gtk::ShortcutController,
    accel: &str,
    f: impl Fn() + 'static,
) {
    add_shortcut_if(controller, accel, move || {
        f();
        true
    });
}

pub(crate) fn menu_item(label: &str, action: &str, accel: Option<&str>) -> gio::MenuItem {
    let item = gio::MenuItem::new(Some(label), Some(action));
    if let Some(accel) = accel {
        item.set_attribute_value("accel", Some(&accel.to_variant()));
    }
    item
}

pub(crate) fn menu_radio(label: &str, action: &str, target: &str) -> gio::MenuItem {
    let item = gio::MenuItem::new(Some(label), None);
    item.set_action_and_target_value(Some(action), Some(&target.to_variant()));
    item
}

pub(crate) fn add_win_action(
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

pub(crate) fn add_win_toggle(
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

pub(crate) fn set_action_enabled(win: &ApplicationWindow, name: &str, on: bool) {
    if let Some(a) = win.lookup_action(name)
        && let Ok(a) = a.downcast::<gio::SimpleAction>()
    {
        a.set_enabled(on);
    }
}

pub(crate) fn trigger_win_action(win: &ApplicationWindow, name: &str) {
    if let Some(a) = win.lookup_action(name) {
        gio::prelude::ActionExt::activate(&a, None);
    }
}

pub(crate) fn set_action_state_str(win: &ApplicationWindow, name: &str, value: &str) {
    if let Some(a) = win.lookup_action(name)
        && let Ok(a) = a.downcast::<gio::SimpleAction>()
    {
        a.set_state(&value.to_variant());
    }
}

pub(crate) fn set_project_path(ui: &UiRc, shown: &str, tooltip: Option<&str>) {
    let u = ui.borrow();
    u.project_path.set_text(shown);
    if tooltip.unwrap_or("").is_empty() {
        u.project_path.set_tooltip_text(None);
    } else {
        u.project_path.set_tooltip_text(tooltip);
    }
}

pub(crate) fn set_local_project_path(ui: &UiRc, path: &std::path::Path) {
    let full = path.display().to_string();
    set_project_path(ui, &path_for_display(path), Some(&full));
}

pub(crate) fn install_menubar(
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
        close_current_tab(ui);
    });
    add_win_action(win, ui, "pasta", |ui| {
        choose_local_folder(ui);
    });
    add_win_action(win, ui, "servidor", ask_server);
    add_win_action(win, ui, "trocar-projeto", trocar_projeto);
    add_win_action(win, ui, "recentes", navegar::recentes);
    add_win_action(
        win,
        ui,
        "servidores-anteriores",
        navegar::servidores_anteriores,
    );
    add_win_action(win, ui, "novo-projeto-remoto", trocar_projeto);
    add_win_action(win, ui, "sincronizacao", sincronizacao::show)
        .set_enabled(ui.borrow().files.is_remote());
    add_win_action(win, ui, "buscar-projeto", buscar_no_projeto);
    add_win_action(win, ui, "repl", mostrar_repl);
    add_win_action(win, ui, "ver-fonte-executada", inspect_execution_source);
    set_action_enabled(win, "ver-fonte-executada", false);
    add_win_action(win, ui, "abrir-rapido", navegar::abrir_rapido);
    add_win_action(win, ui, "funcoes", navegar::funcoes_do_arquivo);
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
    add_win_action(win, ui, "atalhos", |ui| {
        atalhos::mostrar(&ui.borrow().window)
    });
    add_win_action(win, ui, "tutoriais", |ui| {
        if let Err(e) = tutoriais::spawn_tutorial() {
            set_status(ui, &e);
        }
    });
    add_win_action(win, ui, "buscar", show_search);
    add_win_action(win, ui, "ir-linha", ir_para_linha);
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
    projetos_menu::install(ui, &arquivo);
    let sair = gio::Menu::new();
    sair.append_item(&menu_item("Sa_ir", "win.encerrar", Some("<Primary>q")));
    arquivo.append_section(None, &sair);

    let exec = gio::Menu::new();
    exec.append_item(&menu_item("_Executar", "win.executar", Some("F5")));
    exec.append_item(&menu_item("_Depurar", "win.depurar", Some("F6")));
    exec.append_item(&menu_item("_Parar", "win.parar", None));

    let debug = gio::Menu::new();
    debug.append_item(&menu_item("_Continuar", "win.continuar", Some("F5")));
    debug.append_item(&menu_item("_Próximo", "win.proximo", Some("F10")));
    debug.append_item(&menu_item("E_ntrar", "win.entrar", Some("F11")));
    debug.append_item(&menu_item(
        "Sair da _função",
        "win.sair",
        Some("<Shift>F11"),
    ));
    debug.append_item(&menu_item("Pon_to de parada", "win.ponto", Some("F9")));

    let ver = gio::Menu::new();
    ver.append_item(&menu_item("_Arquivos", "win.mostrar-arquivos", Some("F7")));
    ver.append_item(&menu_item("_Painel", "win.mostrar-painel", Some("F8")));
    ver.append_item(&menu_item("_Buscar", "win.buscar", Some("<Primary>f")));
    ver.append_item(&menu_item(
        "Buscar no pro_jeto",
        "win.buscar-projeto",
        Some("<Primary><Shift>f"),
    ));
    ver.append_item(&menu_item("_REPL", "win.repl", None));
    ver.append_item(&menu_item(
        "Ver código executado",
        "win.ver-fonte-executada",
        None,
    ));
    ver.append_item(&menu_item(
        "Abrir arquivo do _projeto…",
        "win.abrir-rapido",
        Some("<Primary>p"),
    ));
    ver.append_item(&menu_item(
        "_Funções do arquivo…",
        "win.funcoes",
        Some("<Primary><Shift>o"),
    ));
    ver.append_item(&menu_item(
        "_Ir para linha",
        "win.ir-linha",
        Some("<Primary>g"),
    ));
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
    ajuda.append_item(&menu_item(
        "_Atalhos do teclado",
        "win.atalhos",
        Some("<Primary>question"),
    ));

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
    menu_teclado::install(win);
}

/// Like `add_shortcut`, but `f` returning false lets the key through.
pub(crate) fn add_shortcut_if(
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

pub(crate) fn install_css() {
    let css = gtk::CssProvider::new();
    css.load_from_data(
        "
        paned.aula-paned > separator {
            min-width: 4px;
            min-height: 4px;
            margin: 0;
            background-color: alpha(currentColor, 0.12);
        }
        flowbox.aula-run-bar { padding: 0; }
        flowbox.aula-run-bar > flowboxchild { padding: 0; }
        flowbox.aula-run-bar button:disabled,
        flowbox.aula-run-bar button:disabled:hover {
            background-image: none;
            background-color: alpha(currentColor, 0.08);
            color: alpha(currentColor, 0.40);
            border-color: alpha(currentColor, 0.12);
        }
        flowbox.aula-run-bar button:disabled label { color: inherit; }
        menubar { padding: 2px 8px 0 8px; }
        label.aula-catalogo { font-family: monospace; }
        label.aula-ajuda-grupo { font-weight: bold; opacity: 0.7; padding-top: 8px; }
        label.aula-ajuda-tema { font-weight: bold; opacity: 0.55; padding-top: 4px; padding-left: 8px; }
        paned.aula-paned > separator:hover {
            background-color: alpha(currentColor, 0.28);
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
pub(crate) fn existing_exemplos_dir() -> Option<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(d) = expressa::exemplos::dest_dir() {
        candidates.push(d);
    }
    if let Some(docs) = glib::user_special_dir(glib::UserDirectory::Documents)
        && !docs.as_os_str().is_empty()
    {
        candidates.push(docs.join("Expressa Aula").join("exemplos"));
    }
    candidates.push(glib::user_data_dir().join("expressa-aula").join("exemplos"));
    candidates
        .into_iter()
        .find(|dir| dir.join("linguagem").is_dir())
}

pub(crate) fn data_dir() -> PathBuf {
    if let Ok(exe) = std::env::current_exe()
        && let Some(dir) = exe.parent()
    {
        let bundled = dir.join("res");
        if bundled.join("language-specs").is_dir() {
            return bundled;
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
pub(crate) fn prepare_runtime_prefix() {
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

pub(crate) fn language_manager() -> LanguageManager {
    // GtkSourceView 5 search paths are the folders that *contain* *.lang
    // files (e.g. .../language-specs), not the parent of that folder.
    let lm = LanguageManager::default();
    if let Some(path) = data_dir().join("language-specs").to_str() {
        lm.prepend_search_path(path);
    }
    lm
}

pub(crate) fn set_status(ui: &UiRc, text: &str) {
    if let Ok(u) = ui.try_borrow() {
        u.status.set_text(text);
    }
}

pub(crate) fn refresh_cursor(ui: &UiRc, buffer: &sourceview5::Buffer) {
    let it = buffer.iter_at_mark(&buffer.get_insert());
    let text = format!("{}:{}", it.line() + 1, it.line_offset() + 1);
    if let Ok(u) = ui.try_borrow() {
        u.cursor_pos.set_text(&text);
    }
}

pub(crate) fn remember_arquivo(ui: &UiRc, doc: &Doc) {
    let Some(path) = doc.path.borrow().clone() else {
        return;
    };
    if let Ok(mut u) = ui.try_borrow_mut()
        && u.prefs.arquivo != path
    {
        u.prefs.arquivo = path;
        u.prefs.save();
    }
    sessao::salvar(ui);
}

pub(crate) fn parse_run_args(s: &str) -> Vec<String> {
    s.split_whitespace().map(|w| w.to_string()).collect()
}
// ── Dialogs ────────────────────────────────────────────────────────────────

pub(crate) fn perguntar_linha(parent: &impl IsA<gtk::Window>, pergunta: &str) -> Option<String> {
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
/// Ask Windows to put our top-level window in front. A newly started process
/// is often not allowed to cover the window that already has focus, so the
/// IDE was opening maximized and invisible behind it.
pub(crate) fn schedule_raise() {
    #[cfg(windows)]
    win32_foreground::schedule();
}

/// Stop stealing focus while a folder or server dialog is open.
pub(crate) fn pause_raise() {
    #[cfg(windows)]
    win32_foreground::pause();
}

pub(crate) fn resume_raise() {
    #[cfg(windows)]
    win32_foreground::resume();
}
#[cfg(windows)]
pub(crate) mod win32_foreground {
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
