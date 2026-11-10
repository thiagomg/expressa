//! GTK tests for debugger/run-bar behaviour. A display is required.
use super::*;

struct DebugFixture {
    ui: UiRc,
    doc: Rc<Doc>,
    commands: std::sync::mpsc::Receiver<String>,
    _app: Application,
}
impl DebugFixture {
    fn new(text: &str) -> Self {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        let id = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let app = Application::builder()
            .application_id(format!("dev.expressa.aula.tests.debugview.t{id}"))
            .flags(gio::ApplicationFlags::NON_UNIQUE)
            .build();
        app.register(None::<&gio::Cancellable>).unwrap();
        let ui = build_ui_state(&app, false);
        let doc = open_doc(&ui, Some("main.lep".into()), text);
        let source = Arc::new(debug_sources::Source::new("main.lep", text.into()));
        ui.borrow_mut().execucao.execution_sources =
            Arc::new(HashMap::from([(source.path.clone(), source.clone())]));
        ui.borrow_mut()
            .execucao
            .execution_editors
            .insert(source.path.clone(), Rc::downgrade(&doc));
        doc.set_running_source(Some(source));
        let (tx, commands) = std::sync::mpsc::channel();
        ui.borrow_mut().execucao.run = Some(Run {
            stopping: false,
            debug_cmd: tx,
            paused: None,
        });
        remember_execution_location(&ui, "main.lep", 1, false);
        set_debug_buttons(&ui, true, true);
        Self {
            ui,
            doc,
            commands,
            _app: app,
        }
    }
    fn pause(&self) -> expressa::runtime::DebugPaused {
        expressa::runtime::DebugPaused {
            file: "main.lep".into(),
            line: 1,
            col: 1,
            source_line: self.doc.text(),
            vars: Vec::new(),
            stack: Vec::new(),
            error: None,
        }
    }
}
impl Drop for DebugFixture {
    fn drop(&mut self) {
        self.ui.borrow().closing.set(true);
        self.ui.borrow().repl.encerrar();
        for doc in all_docs(&self.ui) {
            doc.cancel_pending();
        }
        close_source_viewer(&self.ui);
        self.ui.borrow_mut().execucao.run = None;
        self.ui.borrow().execucao.telas_graf.fechar_todas();
        self.ui.borrow().window.destroy();
    }
}
fn spin_until(ready: impl Fn() -> bool) {
    let end = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while !ready() {
        assert!(
            std::time::Instant::now() < end,
            "execution UI did not complete"
        );
        glib::MainContext::default().iteration(false);
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
}

#[test]
fn ide_run_bar_sends_debug_commands() {
    gtk_test::run(|| {
        let fixture = DebugFixture::new("escreva(1)\n");
        let barra = {
            let u = fixture.ui.borrow();
            (
                u.depuracao.barra_exec.caixa.clone(),
                u.depuracao.barra_exec.estado.clone(),
                u.depuracao.barra_exec.continuar.clone(),
                u.depuracao.barra_exec.proximo.clone(),
                u.depuracao.barra_exec.entrar.clone(),
                u.depuracao.barra_exec.sair.clone(),
                u.depuracao.barra_exec.parar.clone(),
            )
        };
        let (caixa, estado, continuar, proximo, entrar, sair, parar) = barra;
        assert!(
            caixa.get_visible(),
            "barra visível enquanto o programa roda"
        );
        assert_eq!(estado.text().as_str(), "pausado");
        assert_eq!(sair.label().as_deref(), Some("Sair da função"));
        assert!(continuar.get_sensitive());
        assert!(parar.get_sensitive());

        continuar.emit_clicked();
        assert_eq!(fixture.commands.try_recv().unwrap(), "continuar");
        assert_eq!(estado.text().as_str(), "executando");
        assert!(!continuar.get_sensitive());
        assert!(parar.get_sensitive());
        assert!(
            continuar.opacity() < parar.opacity(),
            "botão desabilitado deve aparecer apagado (cinza) em relação ao Parar"
        );

        set_debug_buttons(&fixture.ui, true, true);
        proximo.emit_clicked();
        assert_eq!(fixture.commands.try_recv().unwrap(), "proximo");
        set_debug_buttons(&fixture.ui, true, true);
        entrar.emit_clicked();
        assert_eq!(fixture.commands.try_recv().unwrap(), "entrar");
        set_debug_buttons(&fixture.ui, true, true);
        sair.emit_clicked();
        assert_eq!(fixture.commands.try_recv().unwrap(), "sair");
        set_debug_buttons(&fixture.ui, false, true);
        parar.emit_clicked();
        assert_eq!(fixture.commands.try_recv().unwrap(), "terminar");

        set_debug_buttons(&fixture.ui, false, false);
        assert!(!caixa.get_visible());
    });
}

#[test]
fn f5_resumes_the_paused_run_from_the_read_only_source() {
    gtk_test::run(|| {
        let fixture = DebugFixture::new("escreva(1)\n");
        let ui = &fixture.ui;
        assert!(handle_debug_pause(ui, &fixture.pause()));
        let source = ui.borrow().execucao.execution_sources.clone();
        fixture.doc.buffer.set_text("escreva(2)\n");
        let window = ui.borrow().window.clone();
        let controllers = window.observe_controllers();
        let shortcut = (0..controllers.n_items())
            .filter_map(|i| {
                controllers
                    .item(i)?
                    .downcast::<gtk::ShortcutController>()
                    .ok()
            })
            .filter(|c| c.propagation_phase() == gtk::PropagationPhase::Capture)
            .flat_map(|c| {
                (0..c.n_items())
                    .filter_map(|i| c.item(i)?.downcast::<gtk::Shortcut>().ok())
                    .collect::<Vec<_>>()
            })
            .find(|s| s.trigger().is_some_and(|t| t.to_str() == "F5"))
            .expect("F5 must be registered in capture phase");
        assert!(shortcut.action().unwrap().activate(
            gtk::ShortcutActionFlags::empty(),
            &window,
            None,
        ));
        assert_eq!(fixture.commands.try_recv().unwrap(), "continuar");
        assert!(ui.borrow().execucao.run.as_ref().unwrap().paused.is_none());
        assert!(Arc::ptr_eq(
            &source,
            &ui.borrow().execucao.execution_sources
        ));
        assert!(ui.borrow().execucao.source_viewer.is_some());
        assert!(!window.lookup_action("continuar").unwrap().is_enabled());
        // Repeated F5 while running must not send another resume or launch.
        executar(ui, false);
        assert!(fixture.commands.try_recv().is_err());
        assert!(Arc::ptr_eq(
            &source,
            &ui.borrow().execucao.execution_sources
        ));
    });
}

#[test]
fn stop_closes_the_snapshot_before_worker_ack_and_rejects_late_pauses() {
    gtk_test::run(|| {
        let fixture = DebugFixture::new("escreva(1)\n");
        let ui = &fixture.ui;
        assert!(handle_debug_pause(ui, &fixture.pause()));
        let viewer = ui.borrow().execucao.source_viewer.clone().unwrap();
        assert!(!viewer.doc.view.is_editable());
        fixture.doc.set_path("renamed.lep".into());
        let (input, _answer) = std::sync::mpsc::channel();
        ui.borrow().execucao.console.begin_input("entrada:", input);
        let window = ui.borrow().window.clone();
        trigger_win_action(&window, "parar");
        assert_eq!(fixture.commands.try_recv().unwrap(), "terminar");
        assert!(ui.borrow().execucao.run.as_ref().unwrap().stopping);
        assert!(
            ui.borrow().execucao.source_viewer.is_none(),
            "Stop waited for Finished"
        );
        assert!(ui.borrow().notebook.page_num(&viewer.page).is_none());
        assert!(Rc::ptr_eq(&current_doc(ui).unwrap(), &fixture.doc));
        assert!(fixture.doc.view.is_editable());
        assert!(fixture.doc.running_source.borrow().is_none());
        assert!(!ui.borrow().execucao.console.is_reading());
        assert!(
            !handle_debug_pause(ui, &fixture.pause()),
            "queued pause was accepted after Stop"
        );
        assert!(ui.borrow().execucao.source_viewer.is_none());
        send_debug_cmd(ui, "proximo");
        assert!(fixture.commands.try_recv().is_err());
        assert!(
            !ui.borrow()
                .window
                .lookup_action("continuar")
                .unwrap()
                .is_enabled()
        );
        finish_execution_view(ui, false);
        assert!(ui.borrow().execucao.source_viewer.is_none());
    });
}

#[test]
fn natural_end_returns_to_editor_and_preserves_another_selected_tab() {
    gtk_test::run(|| {
        let fixture = DebugFixture::new("escreva(1)\n");
        let ui = &fixture.ui;
        assert!(handle_debug_pause(ui, &fixture.pause()));
        finish_execution_view(ui, false);
        assert!(ui.borrow().execucao.source_viewer.is_none());
        assert!(Rc::ptr_eq(&current_doc(ui).unwrap(), &fixture.doc));
        assert!(handle_debug_pause(ui, &fixture.pause()));
        let other = open_doc(ui, None, "another edit");
        other.buffer.place_cursor(&other.buffer.iter_at_offset(4));
        finish_execution_view(ui, false);
        assert!(ui.borrow().execucao.source_viewer.is_none());
        assert!(Rc::ptr_eq(&current_doc(ui).unwrap(), &other));
        assert_eq!(other.cursor().offset(), 4);
        assert_eq!(other.text(), "another edit");
    });
}

#[test]
fn changed_source_is_not_given_old_error_positions_and_snapshot_is_explicit() {
    gtk_test::run(|| {
        let fixture = DebugFixture::new("a = 1\n1 / 0\n");
        let ui = &fixture.ui;
        fixture
            .doc
            .buffer
            .insert(&mut fixture.doc.buffer.start_iter(), "// new line\n");
        fixture
            .doc
            .buffer
            .place_cursor(&fixture.doc.buffer.end_iter());
        let cursor = fixture.doc.cursor().offset();
        ui.borrow_mut().execucao.run = None;
        finish_execution_view(ui, false);
        remember_execution_location(ui, "main.lep", 2, true);
        navigate_execution_error(ui, "main.lep", 2, true);
        assert!(ui.borrow().execucao.source_viewer.is_none());
        assert_eq!(fixture.doc.cursor().offset(), cursor);
        assert!(
            ui.borrow()
                .status
                .text()
                .contains("alterado desde esta execução")
        );
        let window = ui.borrow().window.clone();
        trigger_win_action(&window, "ver-fonte-executada");
        let viewer = ui.borrow().execucao.source_viewer.clone().unwrap();
        assert_eq!(viewer.doc.text(), "a = 1\n1 / 0\n");
        assert!(!viewer.doc.view.is_editable());
        viewer.close.emit_clicked();
        assert!(ui.borrow().execucao.source_viewer.is_none());
        assert!(Rc::ptr_eq(&current_doc(ui).unwrap(), &fixture.doc));
    });
}

#[test]
fn repeated_f5_errors_finish_in_editable_code_without_snapshot_tabs() {
    gtk_test::run(|| {
        let text = "a = 1\n1 / 0\n";
        let fixture = DebugFixture::new(text);
        let ui = &fixture.ui;
        let project = tempfile::tempdir().unwrap();
        let runs = tempfile::tempdir().unwrap();
        std::fs::write(project.path().join("main.lep"), text).unwrap();
        let files = Files::Local(project.path().to_path_buf());
        ui.borrow_mut().files = files.clone();
        ui.borrow_mut().project = "local".into();
        ui.borrow_mut().execucao.run = None;
        for _ in 0..3 {
            let ws = Workspace::sync_at(runs.path(), &files, "local").unwrap();
            ws.arm().unwrap();
            ui.borrow_mut()
                .execucao
                .execution
                .transition(ExecutionEvent::Prepare)
                .unwrap();
            iniciar_execucao(ui, ws, "main.lep".into(), text.into(), false, Vec::new());
            spin_until(|| {
                ui.borrow().execucao.run.is_none()
                    && ui.borrow().execucao.execution.availability().is_ok()
            });
            assert!(ui.borrow().execucao.source_viewer.is_none());
            assert_eq!(all_docs(ui).len(), 1);
            assert!(Rc::ptr_eq(&current_doc(ui).unwrap(), &fixture.doc));
            assert!(fixture.doc.view.is_editable());
            assert_eq!(fixture.doc.cursor().line(), 1);
            let buffer = ui.borrow().execucao.console.view.buffer();
            assert!(
                buffer
                    .text(&buffer.start_iter(), &buffer.end_iter(), false)
                    .contains("divisão por zero")
            );
        }
    });
}

#[test]
fn snapshot_navigation_keeps_editor_identity_and_frozen_breakpoints() {
    gtk_test::run(|| {
        let app = Application::builder()
            .application_id("dev.expressa.aula.tests.sources")
            .flags(gio::ApplicationFlags::NON_UNIQUE)
            .build();
        app.register(None::<&gio::Cancellable>).unwrap();
        let ui = build_ui_state(&app, false);
        let doc = open_doc(&ui, Some("lib/a.lep".into()), "escreva(1)\n");
        let source = Arc::new(debug_sources::Source::new(&doc.name(), doc.text()));
        ui.borrow_mut().execucao.execution_sources =
            Arc::new(HashMap::from([(source.path.clone(), source.clone())]));
        ui.borrow_mut()
            .execucao
            .execution_editors
            .insert(source.path.clone(), Rc::downgrade(&doc));
        ui.borrow_mut()
            .execucao
            .execution_breakpoints
            .push((source.path.clone(), 1));
        let (sender, receiver) = std::sync::mpsc::channel();
        ui.borrow_mut().execucao.run = Some(Run {
            stopping: false,
            debug_cmd: sender,
            paused: Some((source.path.clone(), 1, false)),
        });
        doc.set_running_source(Some(source.clone()));
        doc.buffer
            .insert(&mut doc.buffer.start_iter(), "// edited\n");
        doc.set_path("lib/renamed.lep".into());
        toggle_breakpoint(&ui, &doc, 2);
        assert!(
            receiver.try_recv().is_err(),
            "editor breakpoints must not change this run"
        );
        assert!(show_running_source(&ui, "lib\\a.lep", 1, false));
        assert!(
            current_doc(&ui).is_none(),
            "snapshot must not become a save/recovery document"
        );
        assert_eq!(all_docs(&ui).len(), 1);
        let viewer = ui.borrow().execucao.source_viewer.clone().unwrap();
        assert_eq!(viewer.doc.text(), "escreva(1)\n");
        assert_eq!(
            viewer.doc.breakpoint_lines(),
            std::collections::BTreeSet::from([1])
        );
        assert!(
            Rc::ptr_eq(&execution_entry_doc(&ui).unwrap(), &doc),
            "restarting from the snapshot uses the renamed editor"
        );
        viewer.edit.emit_clicked();
        assert!(Rc::ptr_eq(&current_doc(&ui).unwrap(), &doc));
        doc.inspect_source.emit_clicked();
        assert!(current_doc(&ui).is_none());
        send_debug_cmd(&ui, "proximo");
        assert_eq!(receiver.try_recv().unwrap(), "proximo");
        assert!(ui.borrow().execucao.run.as_ref().unwrap().paused.is_none());
        close_current_tab(&ui);
        assert!(ui.borrow().execucao.source_viewer.is_none());
        assert!(Rc::ptr_eq(&current_doc(&ui).unwrap(), &doc));
        ui.borrow_mut().execucao.run = None;
        doc.mark_saved();
        ui.borrow().window.destroy();
    });
}
