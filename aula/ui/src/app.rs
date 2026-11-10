//! Shared application state (`Ui`) and the run handle. Other modules borrow this; they do not own a second copy.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

use gtk::gio;
use gtk::glib;
use gtk::prelude::*;
use gtk::{ApplicationWindow, Button, Entry, FlowBox, Label, Revealer, TreeStore, TreeView};
use sourceview5::{LanguageManager, SearchSettings, StyleSchemeManager};

use crate::assist::{Help, ViewHelp};
use crate::busca_projeto;
use crate::cartesiano;
use crate::console::Console;
use crate::debug_sources;
use crate::docs::Doc;
use crate::execution_state::ExecutionState;
use crate::janela_graf;
use crate::operations;
use crate::painel_repl;
use crate::prefs::{FontCss, Prefs, ThemeCss};
use crate::store::Files;

pub(crate) const APP_ID: &str = "dev.expressa.aula";
pub(crate) const DEFAULT_URL: &str = "http://127.0.0.1:50051";

/// A program running or being debugged.
pub(crate) struct Run {
    pub(crate) stopping: bool,
    pub(crate) debug_cmd: std::sync::mpsc::Sender<String>,
    pub(crate) paused: Option<(String, u32, bool)>,
}

/// IDE bar shown while a program is running (Rodar or Depurar).
pub(crate) struct BarraExec {
    pub(crate) caixa: FlowBox,
    pub(crate) estado: Label,
    pub(crate) continuar: Button,
    pub(crate) proximo: Button,
    pub(crate) entrar: Button,
    pub(crate) sair: Button,
    pub(crate) parar: Button,
}

pub(crate) fn make_run_bar() -> BarraExec {
    let caixa = FlowBox::new();
    caixa.set_selection_mode(gtk::SelectionMode::None);
    caixa.set_homogeneous(false);
    caixa.set_min_children_per_line(1);
    caixa.set_max_children_per_line(24);
    caixa.set_column_spacing(8);
    caixa.set_row_spacing(4);
    caixa.set_hexpand(true);
    caixa.add_css_class("aula-run-bar");
    caixa.set_margin_start(8);
    caixa.set_margin_end(8);
    caixa.set_visible(false);

    let estado = Label::new(None);
    estado.add_css_class("aula-run-estado");
    estado.set_xalign(0.0);
    estado.set_width_chars(11);
    caixa.append(&estado);

    let botao = |rotulo: &str, acao: &str, dica: &str| {
        let b = Button::with_label(rotulo);
        b.set_tooltip_text(Some(dica));
        b.set_action_name(Some(acao));
        b
    };
    let continuar = botao(
        "Continuar",
        "win.continuar",
        "Continuar até o próximo ponto (F5)",
    );
    let proximo = botao("Próximo", "win.proximo", "Próxima linha (F10)");
    let entrar = botao("Entrar", "win.entrar", "Entrar na função (F11)");
    let sair = botao(
        "Sair da função",
        "win.sair",
        "Sair da função atual (Shift+F11)",
    );
    let parar = botao("Parar", "win.parar", "Parar o programa");
    caixa.append(&continuar);
    caixa.append(&proximo);
    caixa.append(&entrar);
    caixa.append(&sair);
    caixa.append(&parar);
    BarraExec {
        caixa,
        estado,
        continuar,
        proximo,
        entrar,
        sair,
        parar,
    }
}

/// Debugger panel, run bar, and the paused-source viewer.
pub(crate) struct Depuracao {
    pub(crate) watches: Vec<String>,
    pub(crate) vars_store: TreeStore,
    pub(crate) stack_store: TreeStore,
    pub(crate) watch_store: TreeStore,
    pub(crate) watch_entry: Entry,
    pub(crate) barra_exec: BarraExec,
}

/// Run handle, output, graphics windows, and the source snapshot of this run.
pub(crate) struct Execucao {
    pub(crate) run: Option<Run>,
    pub(crate) execution: ExecutionState,
    pub(crate) operations: Rc<operations::Operations>,
    pub(crate) console: Rc<Console>,
    pub(crate) graficos: Rc<cartesiano::Painel>,
    pub(crate) telas_graf: Rc<janela_graf::Painel>,
    pub(crate) args_entry: Entry,
    pub(crate) execution_sources: Arc<debug_sources::Sources>,
    pub(crate) last_execution_location: Option<(String, u32, bool)>,
    pub(crate) execution_breakpoints: Vec<(String, u32)>,
    pub(crate) execution_editors: HashMap<String, std::rc::Weak<Doc>>,
    pub(crate) source_viewer: Option<Rc<debug_sources::Viewer>>,
}

pub(crate) struct Ui {
    pub(crate) depuracao: Depuracao,
    pub(crate) execucao: Execucao,
    pub(crate) project_epoch: u64,
    pub(crate) file_operations: HashMap<String, u64>,
    pub(crate) next_file_operation: u64,
    pub(crate) navigation_epoch: u64,
    pub(crate) listing_epoch: u64,
    pub(crate) remote_menu: gio::Menu,
    pub(crate) projects_menu: gio::Menu,
    pub(crate) files: Files,
    /// Folder name locally, student name on a remote server.
    pub(crate) project: String,
    /// Full local path or `url — aula-data/aluno`, shown on the status bar.
    pub(crate) project_path: Label,
    pub(crate) tree: TreeView,
    pub(crate) store: TreeStore,
    pub(crate) project_files: Vec<String>,
    pub(crate) notebook: gtk::Notebook,
    pub(crate) docs: Vec<(Rc<Doc>, Rc<ViewHelp>)>,
    pub(crate) untitled_count: u32,
    /// Server files read for help on imports (`None` = missing).
    pub(crate) file_cache: RefCell<HashMap<String, Option<String>>>,
    pub(crate) search_settings: SearchSettings,
    pub(crate) search_entry: Entry,
    pub(crate) replace_entry: Entry,
    pub(crate) search_revealer: Revealer,
    pub(crate) status: Label,
    pub(crate) cursor_pos: Label,
    pub(crate) diag_label: Label,
    pub(crate) window: ApplicationWindow,
    /// Kept alive so GtkSourceView does not drop the Expressa language spec.
    pub(crate) languages: LanguageManager,
    pub(crate) schemes: StyleSchemeManager,
    pub(crate) prefs: Prefs,
    pub(crate) font_css: FontCss,
    pub(crate) theme_css: ThemeCss,
    pub(crate) help: Option<Rc<Help>>,
    /// Unsaved changes were handled: the window may close now.
    pub(crate) closing: std::cell::Cell<bool>,
    /// Bumped by each tree listing; an older listing that comes back late
    /// is dropped.
    pub(crate) geracao_arvore: u64,
    /// Tabs the user closed (file, line), last one at the end: Ctrl+Shift+T.
    pub(crate) abas_fechadas: Vec<(String, u32)>,
    /// The open project's session was restored (and its unsaved copies
    /// dealt with): from now on it can be saved over (`sessao.rs`).
    pub(crate) sessao_pronta: bool,
    pub(crate) recuperacao_aberta: bool,
    pub(crate) recovery_timer: RefCell<Option<glib::SourceId>>,
    pub(crate) backup_busy: std::cell::Cell<bool>,
    pub(crate) backup_review: RefCell<Option<Box<dyn FnOnce(&UiRc, bool)>>>,
    pub(crate) sync_window: Option<gtk::Window>,
    /// Below the editor: Saída (program output), Busca, REPL.
    pub(crate) painel_inferior: gtk::Notebook,
    pub(crate) busca: Option<Rc<busca_projeto::PainelBusca>>,
    pub(crate) repl: Rc<painel_repl::PainelRepl>,
    pub(crate) ajuda_win: RefCell<Option<gtk::Window>>,
}

pub(crate) type UiRc = Rc<RefCell<Ui>>;
