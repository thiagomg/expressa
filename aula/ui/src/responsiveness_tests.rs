//! Interaction regressions use controllable workers, not timing assumptions
//! about how fast a local disk/network happens to respond.
use crate::*;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

fn spin_until(ready: impl Fn() -> bool) {
    let end = Instant::now() + Duration::from_secs(10);
    while !ready() {
        assert!(
            Instant::now() < end,
            "background UI operation did not finish"
        );
        glib::MainContext::default().iteration(false);
        std::thread::sleep(Duration::from_millis(1));
    }
}
fn ui() -> (UiRc, tempfile::TempDir, Application) {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    let id = NEXT.fetch_add(1, Ordering::Relaxed);
    let app = Application::builder()
        .application_id(format!("dev.expressa.aula.tests.responsiveness.t{id}"))
        .flags(gio::ApplicationFlags::NON_UNIQUE)
        .build();
    app.register(None::<&gio::Cancellable>).unwrap();
    let ui = build_ui_state(&app, false);
    let dir = tempfile::tempdir().unwrap();
    ui.borrow_mut().files = Files::Local(dir.path().to_path_buf());
    ui.borrow_mut().project = "local".into();
    (ui, dir, app)
}
fn dispose(ui: &UiRc) {
    let u = ui.borrow();
    u.closing.set(true);
    u.execucao.operations.cancel_all();
    sessao::stop_copies(ui);
    u.repl.encerrar();
    u.window.destroy();
}
fn gate() -> (
    Arc<AtomicBool>,
    std::sync::mpsc::Sender<()>,
    std::sync::mpsc::Receiver<()>,
) {
    let (release, wait) = std::sync::mpsc::channel();
    (Arc::new(AtomicBool::new(false)), release, wait)
}

#[test]
fn delayed_open_allows_typing_and_main_loop_work_then_respects_project_switch() {
    gtk_test::run(|| {
        let (ui, _dir, _app) = ui();
        let editor = open_doc(&ui, None, "a = 1\n");
        let (started, release, wait) = gate();
        let s = started.clone();
        let completed = Rc::new(std::cell::Cell::new(false));
        let done = completed.clone();
        open_path_read(
            &ui,
            "late.lep",
            move || {
                s.store(true, Ordering::Release);
                wait.recv_timeout(Duration::from_secs(10)).unwrap();
                Ok(store::TextSnapshot {
                    text: "old project".into(),
                    revision: Some("r1".into()),
                })
            },
            move |_, _| done.set(true),
        );
        spin_until(|| started.load(Ordering::Acquire));
        editor.buffer.insert_at_cursor("// typed during read\n");
        let tick = Rc::new(std::cell::Cell::new(false));
        let t = tick.clone();
        glib::idle_add_local_once(move || t.set(true));
        spin_until(|| tick.get());
        assert!(editor.text().contains("typed during read"));
        assert!(!completed.get());
        ui.borrow_mut().project_epoch += 1;
        release.send(()).unwrap();
        // A later request supplies a deterministic completion barrier.
        let barrier = Rc::new(std::cell::Cell::new(false));
        let b = barrier.clone();
        async_result(&ui, "", false, || Ok(()), move |_, _| b.set(true));
        spin_until(|| barrier.get());
        assert!(!completed.get(), "old project callback ran");
        assert!(doc_by_name(&ui, "late.lep").is_none());
        assert_eq!(all_docs(&ui).len(), 1);
        dispose(&ui);
    });
}

#[test]
fn edits_during_save_remain_dirty_and_the_next_save_uses_the_new_receipt() {
    gtk_test::run(|| {
        let (ui, dir, _app) = ui();
        std::fs::write(dir.path().join("a.lep"), "base").unwrap();
        let doc = open_doc(&ui, Some("a.lep".into()), "base");
        *doc.revision.borrow_mut() = Some(expressa_aula_storage::revision(b"base"));
        doc.buffer.set_text("first edit");
        let request = with_documents(&ui, |d| d.prepare_save(&doc, None, false, None))
            .unwrap()
            .unwrap();
        let work = request.clone();
        let files = ui.borrow().files.clone();
        let (started, release, wait) = gate();
        let s = started.clone();
        let completed = Rc::new(std::cell::Cell::new(None));
        let done = completed.clone();
        save_prepared(
            &ui,
            &doc,
            request,
            move || {
                s.store(true, Ordering::Release);
                wait.recv_timeout(Duration::from_secs(10)).unwrap();
                work.write(&files, "local")
            },
            move |ok| done.set(Some(ok)),
        );
        spin_until(|| started.load(Ordering::Acquire));
        doc.buffer.set_text("newer edit");
        release.send(()).unwrap();
        spin_until(|| completed.get().is_some());
        assert_eq!(completed.get(), Some(false));
        assert!(doc.is_dirty());
        assert_eq!(doc.text(), "newer edit");
        assert_eq!(
            std::fs::read_to_string(dir.path().join("a.lep")).unwrap(),
            "first edit"
        );
        assert_eq!(
            doc.revision.borrow().as_deref(),
            Some(expressa_aula_storage::revision(b"first edit").as_str())
        );
        let completed = Rc::new(std::cell::Cell::new(None));
        let done = completed.clone();
        save_doc_then(&ui, &doc, move |ok| done.set(Some(ok)));
        spin_until(|| completed.get().is_some());
        assert_eq!(completed.get(), Some(true));
        assert!(!doc.is_dirty());
        assert_eq!(
            std::fs::read_to_string(dir.path().join("a.lep")).unwrap(),
            "newer edit"
        );
        dispose(&ui);
    });
}

#[test]
fn cancellation_of_a_read_releases_progress_without_opening_a_tab() {
    gtk_test::run(|| {
        let (ui, _dir, _app) = ui();
        let (started, release, wait) = gate();
        let s = started.clone();
        let completed = Rc::new(std::cell::Cell::new(false));
        let done = completed.clone();
        open_path_read(
            &ui,
            "cancelled.lep",
            move || {
                s.store(true, Ordering::Release);
                wait.recv_timeout(Duration::from_secs(10)).unwrap();
                Ok(store::TextSnapshot {
                    text: "late".into(),
                    revision: None,
                })
            },
            move |_, doc| {
                assert_eq!(doc.err(), Some(OpenFailure::Cancelled));
                done.set(true);
            },
        );
        spin_until(|| started.load(Ordering::Acquire));
        ui.borrow().execucao.operations.cancel_all();
        spin_until(|| completed.get());
        assert!(all_docs(&ui).is_empty());
        assert!(!ui.borrow().execucao.operations.button.is_visible());
        release.send(()).unwrap();
        dispose(&ui);
    });
}

#[test]
fn opening_twice_never_replaces_an_already_edited_tab() {
    gtk_test::run(|| {
        let (ui, _dir, _app) = ui();
        let (started, release, wait) = gate();
        let s = started.clone();
        let completed = Rc::new(std::cell::Cell::new(false));
        let done = completed.clone();
        open_path_read(
            &ui,
            "a.lep",
            move || {
                s.store(true, Ordering::Release);
                wait.recv_timeout(Duration::from_secs(10)).unwrap();
                Ok(store::TextSnapshot {
                    text: "old disk bytes".into(),
                    revision: Some("old".into()),
                })
            },
            move |_, _| done.set(true),
        );
        spin_until(|| started.load(Ordering::Acquire));
        let doc = open_doc(&ui, Some("a.lep".into()), "new text");
        doc.buffer.set_modified(true);
        release.send(()).unwrap();
        spin_until(|| completed.get());
        assert_eq!(all_docs(&ui).len(), 1);
        assert_eq!(doc.text(), "new text");
        assert!(doc.is_dirty());
        dispose(&ui);
    });
}

#[test]
fn a_newer_open_request_supersedes_a_delayed_result_for_the_same_path() {
    gtk_test::run(|| {
        let (ui, _dir, _app) = ui();
        let (started, release, wait) = gate();
        let s = started.clone();
        let old_done = Rc::new(std::cell::Cell::new(false));
        let done = old_done.clone();
        open_path_read(
            &ui,
            "a.lep",
            move || {
                s.store(true, Ordering::Release);
                wait.recv_timeout(Duration::from_secs(10)).unwrap();
                Ok(store::TextSnapshot {
                    text: "old".into(),
                    revision: Some("old".into()),
                })
            },
            move |_, result| {
                assert_eq!(result.err(), Some(OpenFailure::Cancelled));
                done.set(true);
            },
        );
        spin_until(|| started.load(Ordering::Acquire));
        let new_done = Rc::new(std::cell::Cell::new(false));
        let done = new_done.clone();
        open_path_read(
            &ui,
            "a.lep",
            || {
                Ok(store::TextSnapshot {
                    text: "new".into(),
                    revision: Some("new".into()),
                })
            },
            move |_, result| {
                assert!(result.is_ok());
                done.set(true);
            },
        );
        spin_until(|| new_done.get());
        release.send(()).unwrap();
        spin_until(|| old_done.get());
        assert_eq!(doc_by_name(&ui, "a.lep").unwrap().text(), "new");
        assert_eq!(all_docs(&ui).len(), 1);
        dispose(&ui);
    });
}

#[test]
fn save_as_completion_does_not_create_two_tabs_with_the_same_file_identity() {
    gtk_test::run(|| {
        let (ui, _dir, _app) = ui();
        let doc = open_doc(&ui, None, "draft");
        doc.buffer.set_modified(true);
        let request = with_documents(&ui, |d| d.prepare_save(&doc, Some("a.lep"), true, None))
            .unwrap()
            .unwrap();
        let work = request.clone();
        let files = ui.borrow().files.clone();
        let (started, release, wait) = gate();
        let s = started.clone();
        let completed = Rc::new(std::cell::Cell::new(None));
        let done = completed.clone();
        save_prepared(
            &ui,
            &doc,
            request,
            move || {
                let receipt = work.write(&files, "local");
                s.store(true, Ordering::Release);
                wait.recv_timeout(Duration::from_secs(10)).unwrap();
                receipt
            },
            move |ok| done.set(Some(ok)),
        );
        spin_until(|| started.load(Ordering::Acquire));
        let other = open_doc(&ui, Some("a.lep".into()), "draft");
        release.send(()).unwrap();
        spin_until(|| completed.get().is_some());
        assert_eq!(completed.get(), Some(false));
        assert!(doc.is_untitled());
        assert!(doc.is_dirty());
        assert!(Rc::ptr_eq(&doc_by_name(&ui, "a.lep").unwrap(), &other));
        dispose(&ui);
    });
}

#[test]
fn closing_stops_the_recovery_timer_and_document_analysis() {
    gtk_test::run(|| {
        let (ui, _dir, _app) = ui();
        let doc = open_doc(&ui, None, "a = 1\n");
        sessao::iniciar_copias(&ui);
        let source = glib::MainContext::default()
            .find_source_by_id(ui.borrow().recovery_timer.borrow().as_ref().unwrap())
            .unwrap();
        doc.cancel_pending();
        dispose(&ui);
        assert!(source.is_destroyed());
        assert!(ui.borrow().recovery_timer.borrow().is_none());
    });
}
