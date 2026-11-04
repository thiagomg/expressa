//! Runs and debugs programs in this process. The server is only a file
//! repository: before a run the student's project is copied into a local
//! durable run folder, and files the program created or changed are sent back.

use std::collections::HashMap;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender, SyncSender};
use std::time::Duration;

use expressa::runtime::{
    CartesianoHost, ChannelDebugger, DebugHook, DebugPaused, EventoGraf, FolhaPronta, GrafHost,
    LeiaHost, StopHook, TelaGrafica, run_with_hosts,
};

use crate::store::{Expected, Files};

/// Start of the message for a crash inside the interpreter.
pub const ERRO_INTERNO: &str =
    "erro interno da Expressa (não é um erro do seu programa; avise o professor)";

/// What the console shows for a panic in the interpreter.
fn mensagem_de_panico(panico: &(dyn std::any::Any + Send)) -> String {
    let detalhe = panico
        .downcast_ref::<&str>()
        .map(|s| s.to_string())
        .or_else(|| panico.downcast_ref::<String>().cloned());
    match detalhe {
        Some(d) => format!("{ERRO_INTERNO}: {d}"),
        None => ERRO_INTERNO.to_string(),
    }
}

/// Events the UI may lag behind before the program blocks. Keeps a program
/// that prints forever from piling up memory or starving the UI.
const EVENT_BACKLOG: usize = 256;

pub enum ExecEvent {
    Stdout(String),
    Stderr(String),
    Leia(String),
    Folha(FolhaPronta),
    TelaGraf(TelaGrafica),
    Source(std::sync::Arc<crate::debug_sources::Source>),
    Paused(DebugPaused),
    Finished(Finished),
}

pub struct StackFrame {
    pub name: String,
    pub file: String,
    pub line: u32,
    /// How many more identical frames follow this one (deep recursion).
    pub repeticoes: usize,
}

pub struct Finished {
    pub ok: bool,
    pub error_message: String,
    /// Relative to the project folder.
    pub error_file: String,
    pub error_line: u32,
    /// Innermost first. Empty when the error is only the top-level module.
    pub stack: Vec<StackFrame>,
}

/// Local copy of one student's project.
/// What [`Workspace::upload_changes`] did.
#[derive(Default)]
pub struct Envio {
    /// Files sent, with their new text (`None` for binary files).
    pub enviados: Vec<(String, Option<String>)>,
    /// Files that could not be sent, as messages for the console.
    pub erros: Vec<String>,
    pub avisos: Vec<String>,
    pub conflitos: Vec<UploadConflict>,
    pub pending_dir: Option<PathBuf>,
}

pub struct UploadConflict {
    pub path: String,
    pub bytes: Vec<u8>,
}

pub struct EditorSource {
    pub path: String,
    pub text: String,
    pub expected: Expected,
}

/// Each execution owns a unique durable directory. Journals remain after a
/// crash or failed upload and are never synchronized over by a later run.
pub struct Workspace {
    pub dir: PathBuf,
    revisions: HashMap<String, Expected>,
    journal: crate::pendencias::Journal,
}

impl Workspace {
    #[cfg(test)]
    pub(crate) fn sync_at(root: &Path, files: &Files, project: &str) -> Result<Self, String> {
        Self::sync_em(root, files, project, None)
    }

    pub fn sync(files: &Files, project: &str) -> Result<Self, String> {
        Self::sync_em(&crate::pendencias::root(), files, project, None)
    }

    fn sync_em(
        root: &Path,
        files: &Files,
        project: &str,
        anterior: Option<Self>,
    ) -> Result<Self, String> {
        // Release only our old lease. Its outputs remain durable and are
        // discovered independently; this run never writes into that directory.
        drop(anterior);
        let identity = files.identity(project)?;
        let journal = crate::pendencias::Journal::create(root, identity)?;
        let dir = journal.dir().canonicalize().map_err(|e| e.to_string())?;
        let mut revisions = HashMap::new();
        for entry in files.list_tree(project)? {
            if !crate::pendencias::valid_relative(&entry.path) {
                return Err(format!("caminho inválido no projeto: {}", entry.path));
            }
            let local = dir.join(&entry.path);
            if entry.is_dir {
                std::fs::create_dir_all(&local).map_err(|e| e.to_string())?;
            } else if let Some(expected) = copy_project_file(files, project, &entry.path, &local)? {
                revisions.insert(entry.path.clone(), expected);
            }
        }
        journal.baseline(&revisions)?;
        Ok(Self {
            dir,
            revisions,
            journal,
        })
    }

    /// Put the editor's text of `rel` in the copy (open tabs, saved or not).
    /// Only subsequent program changes are uploaded. Repository revisions stay
    /// separate: unsaved text never grants permission to replace newer files.
    #[cfg(test)]
    pub fn write_editor_text(&mut self, rel: &str, source: &str) -> Result<(), String> {
        if !crate::pendencias::valid_relative(rel) {
            return Err("caminho de aba inválido".into());
        }
        let expected = self
            .revisions
            .get(rel)
            .cloned()
            .unwrap_or(Expected::Missing);
        self.journal.overlay(rel, source.as_bytes(), &expected)?;
        self.revisions.insert(rel.to_string(), expected);
        self.arm()
    }

    pub fn write_editor_source(&mut self, source: &EditorSource) -> Result<(), String> {
        if !crate::pendencias::valid_relative(&source.path) {
            return Err("caminho de aba inválido".into());
        }
        self.journal
            .overlay(&source.path, source.text.as_bytes(), &source.expected)?;
        self.revisions
            .insert(source.path.clone(), source.expected.clone());
        Ok(())
    }

    pub fn arm(&self) -> Result<(), String> {
        self.journal.arm()
    }

    /// Capture before the worker starts: later writes by the program must not
    /// change the source displayed for an existing run.
    pub fn capture_sources(&self) -> Result<crate::debug_sources::Sources, String> {
        let mut sources = crate::debug_sources::Sources::new();
        for path in self.revisions.keys().filter(|p| p.ends_with(".lep")) {
            let bytes = std::fs::read(self.dir.join(path)).map_err(|e| e.to_string())?;
            if let Ok(text) = String::from_utf8(bytes) {
                let source = std::sync::Arc::new(crate::debug_sources::Source::new(path, text));
                sources.insert(source.path.clone(), source);
            }
        }
        Ok(sources)
    }

    /// Absolute path of `rel` in the copy, as the interpreter names files.
    pub fn abs(&self, rel: &str) -> String {
        self.dir.join(rel).to_string_lossy().into_owned()
    }

    /// Send files that are new or differ from the download. One file that
    /// fails does not stop the others: every failure is reported.
    pub fn upload_changes(&self, files: &Files, project: &str) -> Envio {
        let delivery = self.journal.retry(files, project);
        let pending_dir = if delivery.errors.is_empty() && delivery.conflicts.is_empty() {
            None
        } else {
            Some(self.journal.run_dir.clone())
        };
        Envio {
            enviados: delivery.sent,
            erros: delivery.errors,
            avisos: delivery.warnings,
            conflitos: delivery
                .conflicts
                .into_iter()
                .map(|(path, bytes)| UploadConflict { path, bytes })
                .collect(),
            pending_dir,
        }
    }
}

/// Path as the editor names tabs: relative to `root` when it sits inside,
/// always with `/`. The debugger reports Windows paths with `\`; a second
/// form would open another tab of the same file and the breakpoint, which
/// lives on the first tab, would look gone.
pub fn display_path(root: &Path, path: &str) -> String {
    Path::new(path)
        .strip_prefix(root)
        .map(|r| r.to_string_lossy().into_owned())
        .unwrap_or_else(|_| path.to_string())
        .replace('\\', "/")
}

/// Tab names and debugger paths name the same file when only `\` and `/` differ.
pub fn same_project_path(a: &str, b: &str) -> bool {
    a.replace('\\', "/") == b.replace('\\', "/")
}

/// Download text and local binary data, preserving the repository revision.
/// Binary download from a server remains item 04.
fn copy_project_file(
    files: &Files,
    project: &str,
    rel: &str,
    dest: &Path,
) -> Result<Option<Expected>, String> {
    if !files.is_remote() {
        let snapshot = files.read_bytes_snapshot(rel)?;
        write_bytes(dest, &snapshot.bytes)?;
        return Ok(Some(Expected::Revision(snapshot.revision)));
    }
    match files.current_text(project, rel) {
        Ok(Some(snapshot)) => {
            write_file(dest, &snapshot.text)?;
            Ok(Some(
                snapshot
                    .revision
                    .map(Expected::Revision)
                    .unwrap_or(Expected::Unversioned),
            ))
        }
        Ok(None) => Err(format!("arquivo `{rel}` foi removido durante a cópia")),
        Err(e) if e.to_lowercase().contains("utf-8") => Ok(None),
        Err(e) => Err(e),
    }
}

fn write_bytes(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(path, bytes).map_err(|e| format!("{}: {e}", path.display()))
}

fn write_file(path: &Path, source: &str) -> Result<(), String> {
    write_bytes(path, source.as_bytes())
}

struct ChannelLeia {
    prompts: SyncSender<ExecEvent>,
    lines: Receiver<String>,
}

impl LeiaHost for ChannelLeia {
    fn ask(&mut self, prompt: &str) -> Result<String, String> {
        self.prompts
            .send(ExecEvent::Leia(prompt.to_string()))
            .map_err(|_| "IDE fechada".to_string())?;
        self.lines.recv().map_err(|_| "IDE fechada".to_string())
    }
}

struct ChannelCartesiano {
    tx: SyncSender<ExecEvent>,
}

impl CartesianoHost for ChannelCartesiano {
    fn mostre(&mut self, folha: FolhaPronta) -> Result<(), String> {
        self.tx
            .send(ExecEvent::Folha(folha))
            .map_err(|_| "IDE fechada".to_string())
    }
}

struct ChannelGraf {
    tx: SyncSender<ExecEvent>,
    eventos: Receiver<EventoGraf>,
}

impl GrafHost for ChannelGraf {
    fn mostre(&mut self, tela: TelaGrafica) -> Result<(), String> {
        self.tx
            .send(ExecEvent::TelaGraf(tela))
            .map_err(|_| "IDE fechada".to_string())
    }

    fn evento(
        &mut self,
        _tela: &TelaGrafica,
        timeout: Option<Duration>,
    ) -> Result<EventoGraf, String> {
        match timeout {
            None => self.eventos.recv().map_err(|_| "IDE fechada".to_string()),
            Some(d) if d.is_zero() => Ok(self
                .eventos
                .try_recv()
                .unwrap_or_else(|_| EventoGraf::nada())),
            Some(d) => match self.eventos.recv_timeout(d) {
                Ok(e) => Ok(e),
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => Ok(EventoGraf::nada()),
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => Err("IDE fechada".into()),
            },
        }
    }
}

struct ChannelOut {
    tx: SyncSender<ExecEvent>,
    stderr: bool,
}

impl Write for ChannelOut {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        // Blocks while the UI is behind. Parar still works: the hook checks
        // for it before every statement, and the UI keeps draining.
        let text = String::from_utf8_lossy(buf).into_owned();
        let _ = self.tx.send(if self.stderr {
            ExecEvent::Stderr(text)
        } else {
            ExecEvent::Stdout(text)
        });
        Ok(buf.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Channels the UI keeps for one run.
pub struct RunHandle {
    /// Output, prompts, pauses and the end, in program order.
    pub events: Receiver<ExecEvent>,
    /// Answers to `leia`.
    pub lines: Sender<String>,
    /// Clique / tecla / fechar da janela gráfica.
    pub graf_eventos: Sender<EventoGraf>,
    /// continuar | proximo | entrar | sair | terminar
    pub debug_cmd: Sender<String>,
}

/// Report the source actually loaded by the interpreter before forwarding a
/// pause. This also covers modules generated by a program before importing
/// them; their contents cannot be inferred by rereading a mutable run folder.
struct SourceDebugger {
    inner: ChannelDebugger,
    root: PathBuf,
    events: SyncSender<ExecEvent>,
    seen: HashMap<String, std::sync::Arc<crate::debug_sources::Source>>,
}
impl SourceDebugger {
    fn report(&mut self, ctx: &expressa::runtime::DebugCtx<'_>) -> bool {
        if self
            .seen
            .get(ctx.file)
            .is_some_and(|source| source.text == ctx.source)
        {
            return true;
        }
        let source = std::sync::Arc::new(crate::debug_sources::Source::new(
            &display_path(&self.root, ctx.file),
            ctx.source.to_string(),
        ));
        self.seen.insert(ctx.file.to_string(), source.clone());
        self.events.send(ExecEvent::Source(source)).is_ok()
    }
}
impl DebugHook for SourceDebugger {
    fn before_stmt(
        &mut self,
        ctx: &expressa::runtime::DebugCtx<'_>,
    ) -> expressa::runtime::DebugAction {
        if !self.report(ctx) {
            return expressa::runtime::DebugAction::Quit;
        }
        self.inner.before_stmt(ctx)
    }
    fn on_error(&mut self, ctx: &expressa::runtime::DebugCtx<'_>, message: &str) {
        if self.report(ctx) {
            self.inner.on_error(ctx, message);
        }
    }
    fn interrupted(&mut self) -> bool {
        self.inner.interrupted()
    }
}

/// Run `rel_path` inside `ws` on a new thread. No time limit: Parar
/// (`terminar`) is how a program that never ends is stopped.
pub fn spawn(
    ws: &Workspace,
    rel_path: &str,
    source: String,
    debug: bool,
    breakpoints: Vec<(String, u32)>,
    args: Vec<String>,
) -> RunHandle {
    let (event_tx, events) = std::sync::mpsc::sync_channel(EVENT_BACKLOG);
    let (lines, line_rx) = std::sync::mpsc::channel::<String>();
    let (graf_eventos, graf_rx) = std::sync::mpsc::channel::<EventoGraf>();
    let (debug_cmd, ui_cmd_rx) = std::sync::mpsc::channel::<String>();
    let (dcmd_tx, dcmd_rx) = std::sync::mpsc::channel::<String>();
    let (pause_tx, pause_rx) = std::sync::mpsc::channel::<DebugPaused>();

    // Parar during `leia`: the program waits on a line, not on the hook.
    {
        let lines = lines.clone();
        let graf_eventos = graf_eventos.clone();
        std::thread::spawn(move || {
            while let Ok(cmd) = ui_cmd_rx.recv() {
                if matches!(cmd.trim(), "terminar" | "q" | "quit") {
                    let _ = lines.send(String::new());
                    let _ = graf_eventos.send(EventoGraf::sair());
                }
                if dcmd_tx.send(cmd).is_err() {
                    break;
                }
            }
        });
    }

    let file = ws.abs(rel_path);
    let breakpoints: Vec<(String, u32)> = breakpoints
        .into_iter()
        .map(|(f, l)| (ws.abs(&f), l))
        .collect();
    let root = ws.dir.clone();
    let rel = move |p: &str| display_path(&root, p);
    let ws_dir = ws.dir.clone();

    // Paused snapshots carry cache paths; show them relative to the project.
    {
        let event_tx = event_tx.clone();
        let rel = rel.clone();
        std::thread::spawn(move || {
            while let Ok(mut p) = pause_rx.recv() {
                p.file = rel(&p.file);
                for f in &mut p.stack {
                    f.file = rel(&f.file);
                }
                if event_tx.send(ExecEvent::Paused(p)).is_err() {
                    break;
                }
            }
        });
    }

    let lease = ws.journal.lease();
    std::thread::spawn(move || {
        let _lease = lease; // Keep the run owned even if its window closes.
        let mut host = ChannelLeia {
            prompts: event_tx.clone(),
            lines: line_rx,
        };
        let mut cart = ChannelCartesiano {
            tx: event_tx.clone(),
        };
        let mut graf = ChannelGraf {
            tx: event_tx.clone(),
            eventos: graf_rx,
        };
        let mut out = ChannelOut {
            tx: event_tx.clone(),
            stderr: false,
        };
        let mut err = ChannelOut {
            tx: event_tx.clone(),
            stderr: true,
        };
        let hook: Box<dyn DebugHook> = if debug {
            let mut dbg = ChannelDebugger::new(pause_tx, dcmd_rx);
            for bp in breakpoints {
                dbg.session.breakpoints.insert(bp);
            }
            Box::new(SourceDebugger {
                inner: dbg,
                root: ws_dir.clone(),
                events: event_tx.clone(),
                seen: HashMap::new(),
            })
        } else {
            drop(pause_tx);
            Box::new(StopHook::new(dcmd_rx))
        };
        // A panic is a bug in the interpreter, not in the student's program:
        // say so, and still end the run normally (upload files, clean up).
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            run_with_hosts(
                &source,
                &file,
                hook,
                Some(ws_dir),
                None,
                &mut out,
                &mut err,
                &mut host,
                Some(&mut cart),
                Some(&mut graf),
                &args,
            )
        }));
        let result = match result {
            Ok(result) => result,
            Err(panico) => {
                let _ = event_tx.send(ExecEvent::Finished(Finished {
                    ok: false,
                    error_message: mensagem_de_panico(panico.as_ref()),
                    error_file: String::new(),
                    error_line: 0,
                    stack: Vec::new(),
                }));
                return;
            }
        };
        let finished = match result {
            Ok(0) => Finished {
                ok: true,
                error_message: String::new(),
                error_file: String::new(),
                error_line: 0,
                stack: Vec::new(),
            },
            Ok(code) => Finished {
                ok: false,
                error_message: format!("encerrou com código {code}"),
                error_file: String::new(),
                error_line: 0,
                stack: Vec::new(),
            },
            Err(e) => {
                let stack = if e.shows_stack() {
                    e.pilha_compacta()
                        .into_iter()
                        .map(|(f, repeticoes)| StackFrame {
                            name: f.name.clone(),
                            file: rel(&f.file),
                            line: f.span.line,
                            repeticoes,
                        })
                        .collect()
                } else {
                    Vec::new()
                };
                Finished {
                    ok: false,
                    error_file: rel(&e.file),
                    error_message: e.message,
                    error_line: e.span.line,
                    stack,
                }
            }
        };
        let _ = event_tx.send(ExecEvent::Finished(finished));
    });
    RunHandle {
        events,
        lines,
        graf_eventos,
        debug_cmd,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::execution_state::{Event, ExecutionState, Unavailable};

    struct RunProject {
        root: PathBuf,
        files: Files,
        project: String,
        cache: PathBuf,
    }

    impl RunProject {
        fn new(name: &str) -> Self {
            let (root, files, project, cache) = projeto_para_sync(name);
            Self {
                root,
                files,
                project,
                cache,
            }
        }

        fn workspace(&self) -> Workspace {
            let ws = Workspace::sync_em(&self.cache, &self.files, &self.project, None).unwrap();
            ws.arm().unwrap();
            ws
        }
    }

    impl Drop for RunProject {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.cache);
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn lifecycle_with_worker_completion_error_and_stop() {
        let project = RunProject::new("lifecycle");
        let mut state = ExecutionState::default();
        state.transition(Event::Prepare).unwrap();
        assert!(Workspace::sync_em(&project.cache, &Files::None, "", None).is_err());
        state.transition(Event::PreparationFailed).unwrap();
        assert_eq!(state.availability(), Ok(()));

        for (source, stop, succeeds) in [
            ("escreva(1)\n", false, true),
            ("escreva(nome_inexistente)\n", false, false),
            ("leia(\"nome: \")\n", true, true),
        ] {
            state.transition(Event::Prepare).unwrap();
            let mut ws = project.workspace();
            ws.write_editor_text("a.lep", source).unwrap();
            let run = spawn(&ws, "a.lep", source.into(), false, vec![], vec![]);
            state.transition(Event::Started).unwrap();
            let mut stopped = false;
            loop {
                match run
                    .events
                    .recv_timeout(Duration::from_secs(5))
                    .expect("run should finish or request input")
                {
                    ExecEvent::Leia(_) => {
                        assert!(stop);
                        assert_eq!(state.availability(), Err(Unavailable::Running));
                        run.debug_cmd.send("terminar".into()).unwrap();
                        stopped = true;
                    }
                    ExecEvent::Finished(finished) => {
                        if !stop {
                            assert_eq!(finished.ok, succeeds, "{}", finished.error_message);
                        }
                        assert_eq!(stopped, stop);
                        state.transition(Event::Finished).unwrap();
                        break;
                    }
                    _ => {}
                }
            }
            assert_eq!(state.availability(), Err(Unavailable::Uploading));
            // Failed uploads finish the lifecycle while the journal retains output.
            write_file(&ws.dir.join("generated.txt"), "output from the run").unwrap();
            let failed = ws.upload_changes(&Files::None, "");
            assert!(!failed.erros.is_empty());
            state.transition(Event::Uploaded).unwrap();
            assert_eq!(state.availability(), Ok(()));
        }
    }

    #[test]
    fn debug_uses_unsaved_import_and_project_relative_breakpoint() {
        let project = RunProject::new("debug-import");
        let mut ws = project.workspace();
        let source = "importe \"lib/b\"\nescreva(dobro(3))\n";
        ws.write_editor_text("a.lep", source).unwrap();
        ws.write_editor_text("lib/b.lep", "dobro = funcao(x) {\n    retorne x * 2\n}\n")
            .unwrap();
        let run = spawn(
            &ws,
            "a.lep",
            source.into(),
            true,
            vec![("lib/b.lep".into(), 2)],
            vec![],
        );
        let mut paused_in_import = false;
        let mut output = String::new();
        loop {
            match run
                .events
                .recv_timeout(Duration::from_secs(5))
                .expect("debugger should pause or finish")
            {
                ExecEvent::Paused(pause) => {
                    assert!(pause.error.is_none(), "{:?}", pause.error);
                    // Import evaluation can also pause at the module's first
                    // statement. The breakpoint must still pause inside the call.
                    if pause.file == "lib/b.lep" && pause.line == 2 {
                        assert_eq!(pause.source_line.trim(), "retorne x * 2");
                        paused_in_import = true;
                    }
                    run.debug_cmd.send("continuar".into()).unwrap();
                }
                ExecEvent::Stdout(text) => output.push_str(&text),
                ExecEvent::Finished(finished) => {
                    assert!(finished.ok, "{}", finished.error_message);
                    break;
                }
                _ => {}
            }
        }
        assert!(paused_in_import);
        assert_eq!(output, "6\n");
        assert_eq!(
            std::fs::read_to_string(project.root.join("lib/b.lep")).unwrap(),
            "b"
        );
    }

    #[test]
    fn moved_breakpoint_pauses_on_snapshot_statement_and_next_run_applies_edits() {
        crate::gtk_test::run(|| {
            use gtk::prelude::*;
            let project = RunProject::new("anchored-debug-source");
            let language = sourceview5::LanguageManager::new();
            let search = sourceview5::SearchSettings::new();
            let imported = crate::docs::Doc::new(
                Some("lib/b.lep".into()),
                String::new(),
                "dobro = funcao(x) {\n    retorne x * 2\n}\n",
                &language,
                &search,
            );
            imported.toggle_breakpoint(2);
            imported.buffer.insert(
                &mut imported.buffer.start_iter(),
                "// inserted before breakpoint\n",
            );
            assert_eq!(
                imported.breakpoint_lines(),
                std::collections::BTreeSet::from([3])
            );
            let initial = imported.text();
            for (iteration, expected) in [(0, "6\n"), (1, "297\n")] {
                let mut ws = project.workspace();
                let path = imported.name();
                let source = format!(
                    "importe \"{}\"\nescreva(dobro(3))\n",
                    path.trim_end_matches(".lep")
                );
                ws.write_editor_text("a.lep", &source).unwrap();
                ws.write_editor_text(&path, &imported.text()).unwrap();
                let sources = ws.capture_sources().unwrap();
                let snapshot = sources[&path].clone();
                imported.set_running_source(Some(snapshot.clone()));
                let points = imported
                    .breakpoint_lines()
                    .into_iter()
                    .map(|line| (path.clone(), line))
                    .collect();
                let run = spawn(&ws, "a.lep", source, true, points, vec![]);
                let mut hit = false;
                let mut output = String::new();
                let mut observed = HashMap::new();
                loop {
                    match run
                        .events
                        .recv_timeout(Duration::from_secs(5))
                        .expect("debugger event")
                    {
                        ExecEvent::Source(source) => {
                            observed.insert(source.path.clone(), source);
                        }
                        ExecEvent::Paused(pause) => {
                            assert!(pause.error.is_none(), "{:?}", pause.error);
                            assert!(
                                observed.contains_key(&pause.file),
                                "source must precede its pause"
                            );
                            if iteration == 0 && !hit {
                                // Change and rename the editor while its original source is running.
                                imported.buffer.set_text(
                                    "// next run\ndobro = funcao(x) {\n    retorne x * 99\n}\n",
                                );
                                imported.set_path("lib/renamed.lep".into());
                            }
                            if pause.file == path && pause.line == 3 {
                                hit = true;
                                assert_eq!(
                                    pause.source_line.trim(),
                                    if iteration == 0 {
                                        "retorne x * 2"
                                    } else {
                                        "retorne x * 99"
                                    }
                                );
                                assert!(pause.vars.iter().any(|v| v.name == "x" && v.value == "3"));
                                assert!(!pause.stack.is_empty());
                                assert_eq!(observed[&path].revision, snapshot.revision);
                                let viewer = crate::debug_sources::Viewer::new(
                                    snapshot.clone(),
                                    &language,
                                    &search,
                                );
                                assert_eq!(viewer.doc.text(), snapshot.text);
                                assert!(!viewer.doc.view.is_editable());
                                if iteration == 0 {
                                    assert_eq!(snapshot.text, initial);
                                }
                            }
                            run.debug_cmd.send("continuar".into()).unwrap();
                        }
                        ExecEvent::Stdout(text) => output.push_str(&text),
                        ExecEvent::Finished(finished) => {
                            assert!(finished.ok, "{}", finished.error_message);
                            break;
                        }
                        _ => (),
                    }
                }
                assert!(
                    hit,
                    "breakpoint must pause on the imported function statement"
                );
                assert_eq!(output, expected);
                imported.set_running_source(None);
                if iteration == 0 {
                    // Replacing all text moves old anchors to EOF by policy;
                    // the user sets the desired breakpoint for the next run.
                    for line in imported.breakpoint_lines() {
                        imported.toggle_breakpoint(line);
                    }
                    imported.toggle_breakpoint(3);
                }
            }
        });
    }

    #[test]
    fn source_capture_survives_workspace_file_replacement() {
        let project = RunProject::new("immutable-source");
        let mut ws = project.workspace();
        ws.write_editor_text("a.lep", "escreva(1)\n").unwrap();
        let sources = ws.capture_sources().unwrap();
        std::fs::write(ws.dir.join("a.lep"), "escreva(999)\n").unwrap();
        assert_eq!(sources["a.lep"].text, "escreva(1)\n");
    }

    #[test]
    fn uploads_detect_changes_to_text_binary_and_new_file_destinations() {
        let project = RunProject::new("upload-conflicts");
        std::fs::write(project.root.join("asset.bin"), [0xff, 0x01]).unwrap();
        let ws = project.workspace();
        std::fs::write(ws.dir.join("a.lep"), "program output").unwrap();
        std::fs::write(ws.dir.join("asset.bin"), [0xff, 0x02]).unwrap();
        std::fs::write(ws.dir.join("new.lep"), "new program output").unwrap();
        // A user/editor changes all three destination files during execution.
        std::fs::write(project.root.join("a.lep"), "user text").unwrap();
        std::fs::write(project.root.join("asset.bin"), [0xff, 0x03]).unwrap();
        std::fs::write(project.root.join("new.lep"), "user creation").unwrap();
        let upload = ws.upload_changes(&project.files, &project.project);
        assert!(upload.enviados.is_empty());
        assert_eq!(upload.conflitos.len(), 3);
        assert_eq!(
            std::fs::read_to_string(project.root.join("a.lep")).unwrap(),
            "user text"
        );
        assert_eq!(
            std::fs::read(project.root.join("asset.bin")).unwrap(),
            [0xff, 0x03]
        );
        assert_eq!(
            std::fs::read_to_string(project.root.join("new.lep")).unwrap(),
            "user creation"
        );
        assert!(
            upload
                .conflitos
                .iter()
                .any(|c| c.bytes == b"program output")
        );
    }

    #[test]
    fn editor_overlay_keeps_its_original_revision_even_if_preparation_reads_newer_disk_text() {
        let project = RunProject::new("stale-editor");
        let old_revision = expressa_aula_storage::revision(b"a");
        std::fs::write(project.root.join("a.lep"), "external update before F5").unwrap();
        let mut ws = project.workspace();
        ws.write_editor_source(&EditorSource {
            path: "a.lep".into(),
            text: "unsaved editor text".into(),
            expected: Expected::Revision(old_revision),
        })
        .unwrap();
        ws.arm().unwrap();
        std::fs::write(ws.dir.join("a.lep"), "program changed stale editor text").unwrap();
        let upload = ws.upload_changes(&project.files, &project.project);
        assert_eq!(upload.conflitos.len(), 1);
        assert_eq!(
            std::fs::read_to_string(project.root.join("a.lep")).unwrap(),
            "external update before F5"
        );
    }

    #[test]
    fn debug_file_uses_the_same_slashes_as_the_tab() {
        let root = std::env::temp_dir().join("expressa-aula-path");
        let abs = root.join("jogos").join("forca.lep");
        assert_eq!(
            display_path(&root, &abs.to_string_lossy()),
            "jogos/forca.lep"
        );
        assert!(same_project_path(
            "jogos/forca.lep",
            &display_path(&root, &abs.to_string_lossy())
        ));
        assert!(same_project_path(r"jogos\forca.lep", "jogos/forca.lep"));
        assert!(!same_project_path("jogos/forca.lep", "jogos/termo.lep"));
    }

    #[test]
    fn download_copies_a_png_without_failing() {
        let root = std::env::temp_dir().join(format!("expressa-png-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("tela_grafica")).unwrap();
        std::fs::create_dir_all(root.join("jogos")).unwrap();
        std::fs::write(root.join("jogos/estrela.lep"), "escreva(1)\n").unwrap();
        let png = [0xff, 0xd8, 0x00, 0xff];
        std::fs::write(root.join("tela_grafica/casa.png"), png).unwrap();
        let files = Files::Local(root.clone());
        let name = format!("png{}", std::process::id());
        let cache = root.with_extension("cache");
        let ws = Workspace::sync_em(&cache, &files, &name, None).unwrap();
        assert_eq!(
            std::fs::read_to_string(ws.dir.join("jogos/estrela.lep")).unwrap(),
            "escreva(1)\n"
        );
        assert_eq!(
            std::fs::read(ws.dir.join("tela_grafica/casa.png")).unwrap(),
            png
        );
        let _ = std::fs::remove_dir_all(&cache);
        let _ = std::fs::remove_dir_all(&root);
    }

    /// A local project, under a fresh folder, with `a.lep` and `lib/b.lep`,
    /// and a folder for its run copies.
    fn projeto_para_sync(nome: &str) -> (PathBuf, Files, String, PathBuf) {
        let root =
            std::env::temp_dir().join(format!("expressa-sync-{nome}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("lib")).unwrap();
        std::fs::write(root.join("a.lep"), "a").unwrap();
        std::fs::write(root.join("lib/b.lep"), "b").unwrap();
        let projeto = format!("sync-{nome}-{}", std::process::id());
        let cache = root.with_extension("copias");
        (root.clone(), Files::Local(root), projeto, cache)
    }

    #[test]
    fn sync_always_uses_a_fresh_directory() {
        let project = RunProject::new("fresh");
        let first = project.workspace();
        let first_dir = first.dir.clone();
        let second = project.workspace();
        assert_ne!(first.dir, second.dir);
        assert!(
            first_dir.join("a.lep").exists(),
            "another run deleted the active copy"
        );
        assert_eq!(first.journal.identity(), second.journal.identity());
    }

    #[test]
    fn sync_refreshes_sources_without_touching_prior_output() {
        let project = RunProject::new("refresh");
        let first = project.workspace();
        let first_dir = first.dir.clone();
        std::fs::write(first.dir.join("new.txt"), "not uploaded").unwrap();
        std::fs::write(project.root.join("a.lep"), "a2").unwrap();
        let second = Workspace::sync_em(
            &project.cache,
            &project.files,
            &project.project,
            Some(first),
        )
        .unwrap();
        assert_eq!(
            std::fs::read_to_string(second.dir.join("a.lep")).unwrap(),
            "a2"
        );
        assert!(!second.dir.join("new.txt").exists());
        assert_eq!(
            std::fs::read_to_string(first_dir.join("new.txt")).unwrap(),
            "not uploaded"
        );
        let scan = crate::pendencias::scan(&project.cache);
        assert!(scan.errors.is_empty(), "{:?}", scan.errors);
        assert_eq!(scan.runs.len(), 1);
        assert_eq!(scan.runs[0].files[0].0, "new.txt");
    }

    #[test]
    fn identical_folder_names_in_distinct_locations_have_distinct_identities() {
        let temp = tempfile::tempdir().unwrap();
        let a = temp.path().join("one/project");
        let b = temp.path().join("two/project");
        std::fs::create_dir_all(&a).unwrap();
        std::fs::create_dir_all(&b).unwrap();
        let first = Files::Local(a.clone());
        let second = Files::Local(b);
        assert_ne!(first.cache_key("project"), second.cache_key("project"));
        assert_eq!(
            first.cache_key("display name"),
            Files::Local(a.join(".")).cache_key("other display name")
        );
        #[cfg(unix)]
        {
            let alias = temp.path().join("alias");
            std::os::unix::fs::symlink(&a, &alias).unwrap();
            assert_eq!(first.cache_key("a"), Files::Local(alias).cache_key("b"));
        }
        let cache = temp.path().join("runs");
        let wa = Workspace::sync_em(&cache, &first, "project", None).unwrap();
        let wb = Workspace::sync_em(&cache, &second, "project", None).unwrap();
        assert_ne!(
            wa.dir.parent().unwrap().parent(),
            wb.dir.parent().unwrap().parent()
        );
    }

    #[test]
    fn panico_vira_erro_interno_com_o_detalhe() {
        let texto = std::panic::catch_unwind(|| panic!("índice fora")).unwrap_err();
        assert_eq!(
            mensagem_de_panico(texto.as_ref()),
            format!("{ERRO_INTERNO}: índice fora")
        );
        let formatado = std::panic::catch_unwind(|| panic!("valor {}", 3)).unwrap_err();
        assert_eq!(
            mensagem_de_panico(formatado.as_ref()),
            format!("{ERRO_INTERNO}: valor 3")
        );
        let outro = std::panic::catch_unwind(|| std::panic::panic_any(7)).unwrap_err();
        assert_eq!(mensagem_de_panico(outro.as_ref()), ERRO_INTERNO);
    }

    #[test]
    fn upload_changes_continua_depois_de_uma_falha() {
        let project = RunProject::new("upload-partial");
        std::fs::write(project.root.join("sub"), "blocker").unwrap();
        let ws = project.workspace();
        write_file(&ws.dir.join("b.lep"), "novo").unwrap();
        std::fs::remove_file(ws.dir.join("sub")).unwrap();
        write_file(&ws.dir.join("sub/d.lep"), "x").unwrap();
        let envio = ws.upload_changes(&project.files, &project.project);
        assert_eq!(envio.enviados, [("b.lep".into(), Some("novo".into()))]);
        assert_eq!(envio.erros.len(), 1, "{:?}", envio.erros);
        assert!(envio.erros[0].starts_with("sub/d.lep"));
        assert_eq!(
            std::fs::read_to_string(project.root.join("a.lep")).unwrap(),
            "a"
        );
        assert!(envio.pending_dir.is_some());
        assert_eq!(ws.journal.outputs().unwrap().len(), 1);
    }
}
