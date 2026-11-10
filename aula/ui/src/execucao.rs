//! F5/F6: prepare a workspace, run or debug, stream output, and upload results.

use std::rc::Rc;
use std::sync::Arc;

use gtk::glib;
use gtk::prelude::*;

use crate::console::Console;
use crate::exec::{ExecEvent, Workspace};
use crate::execution_state::{Event as ExecutionEvent, Unavailable};
use crate::*;

// ── Running ────────────────────────────────────────────────────────────────

/// Events handled per 16 ms tick, so a program that prints without end
/// cannot freeze the window. The rest wait in the (bounded) channel.
pub(crate) const EVENTS_PER_TICK: usize = 200;

pub(crate) fn execution_available(ui: &UiRc, running_message: &str) -> bool {
    let availability = ui.borrow().execucao.execution.availability();
    let message = match availability {
        Ok(()) => return true,
        Err(Unavailable::Running)
            if ui
                .borrow()
                .execucao
                .run
                .as_ref()
                .is_some_and(|run| run.stopping) =>
        {
            "aguarde a execução parar".to_string()
        }
        Err(Unavailable::Running) => running_message.to_string(),
        Err(Unavailable::Preparing) => "aguarde: copiando o projeto para executar".to_string(),
        Err(Unavailable::Uploading) => {
            "aguarde: salvando os arquivos que o programa escreveu".to_string()
        }
    };
    set_status(ui, &message);
    false
}

pub(crate) fn can_change_project(ui: &UiRc) -> bool {
    execution_available(ui, "pare o programa antes de trocar de projeto")
}

pub(crate) fn executar(ui: &UiRc, debug: bool) {
    // F5 resumes the paused run, including from the read-only source tab.
    // It must not prepare a new run or pick up edits made since it started.
    if !debug
        && ui
            .borrow()
            .execucao
            .run
            .as_ref()
            .is_some_and(|run| run.paused.is_some() && !run.stopping)
    {
        send_debug_cmd(ui, "continuar");
        return;
    }
    if !execution_available(ui, "já tem um programa executando — aperte Parar primeiro") {
        return;
    }
    let Some(doc) = execution_entry_doc(ui) else {
        let path = ui
            .borrow()
            .execucao
            .source_viewer
            .as_ref()
            .map(|v| v.source.path.clone());
        if let Some(path) = path {
            open_path_then(ui, &path, move |ui, doc| {
                if doc.is_some() {
                    executar(ui, debug);
                }
            });
        } else {
            set_status(ui, "abra um arquivo para executar");
        }
        return;
    };
    let name = doc.name();
    let fonte = doc.text();
    // The text of every open tab goes into the copy, so the program and its
    // `importe`s run what is on screen (saved or not).
    let textos: Vec<exec::EditorSource> = all_docs(ui)
        .iter()
        .map(|doc| exec::EditorSource {
            path: doc.name(),
            text: doc.text(),
            expected: if doc.is_untitled() {
                store::Expected::Missing
            } else {
                doc.revision
                    .borrow()
                    .clone()
                    .map(store::Expected::Revision)
                    .unwrap_or(store::Expected::Unversioned)
            },
        })
        .collect();
    let launch_docs: Vec<_> = all_docs(ui)
        .into_iter()
        .map(|d| {
            let path = debug_sources::normalize_path(&d.name());
            (Rc::downgrade(&d), path)
        })
        .collect();
    let bps: Vec<_> = all_docs(ui)
        .iter()
        .flat_map(|d| {
            let name = debug_sources::normalize_path(&d.name());
            d.breakpoint_lines()
                .into_iter()
                .map(move |line| (name.clone(), line))
        })
        .collect();
    let entry_name = name.clone();
    let (files, project, args) = {
        let mut u = ui.borrow_mut();
        u.execucao
            .execution
            .transition(ExecutionEvent::Prepare)
            .expect("idle before preparing a run");
        let args = parse_run_args(&u.execucao.args_entry.text());
        (u.files.clone(), u.project.clone(), args)
    };
    let chave = ui.borrow().project_epoch;
    set_status(ui, &format!("preparando {name}…"));
    async_result(
        ui,
        "Preparando execução",
        true,
        move || {
            let mut ws = Workspace::sync(&files, &project)?;
            for source in &textos {
                ws.write_editor_source(source)?;
            }
            ws.arm()?;
            let sources = ws.capture_sources()?;
            Ok::<_, String>((ws, sources))
        },
        move |ui, ws| {
            let outro_projeto = {
                let u = ui.borrow();
                u.project_epoch != chave
            };
            if outro_projeto {
                ui.borrow_mut()
                    .execucao
                    .execution
                    .transition(ExecutionEvent::PreparationFailed)
                    .expect("preparation abandoned after a project change");
                return;
            }
            match ws {
                Ok((ws, mut sources)) => {
                    // The entry source has its own immutable identity, even if
                    // its editor tab was renamed while preparation ran.
                    let source = Arc::new(debug_sources::Source::new(&entry_name, fonte.clone()));
                    sources.insert(source.path.clone(), source);
                    close_source_viewer(ui);
                    for d in all_docs(ui) {
                        d.set_running_source(None);
                    }
                    ui.borrow_mut().execucao.execution_editors.clear();
                    for (weak, path) in launch_docs {
                        ui.borrow_mut()
                            .execucao
                            .execution_editors
                            .insert(path.clone(), weak.clone());
                        if let Some(d) = weak.upgrade() {
                            d.set_running_source(sources.get(&path).cloned());
                        }
                    }
                    ui.borrow_mut().execucao.execution_sources = Arc::new(sources);
                    ui.borrow_mut().execucao.execution_breakpoints = bps;
                    iniciar_execucao(ui, ws, entry_name, fonte, debug, args)
                }
                Err(e) => {
                    ui.borrow_mut()
                        .execucao
                        .execution
                        .transition(ExecutionEvent::PreparationFailed)
                        .expect("preparing before a preparation failure");
                    set_status(ui, &format!("não copiou o projeto: {e}"));
                    if !ui.borrow().files.is_remote() {
                        pendencias_ui::notify_local_failure(ui);
                    }
                }
            }
        },
    );
}

/// Second half of [`executar`], once the copy is ready.
pub(crate) fn iniciar_execucao(
    ui: &UiRc,
    ws: Workspace,
    name: String,
    fonte: String,
    debug: bool,
    args: Vec<String>,
) {
    mostrar_pagina_inferior(ui, PaginaInferior::Saida);
    remember_execution_location(ui, &name, 1, false);
    let (bps, unbound) = {
        let u = ui.borrow();
        u.execucao
            .execution_breakpoints
            .iter()
            .cloned()
            .partition::<Vec<_>, _>(|(path, line)| {
                u.execucao
                    .execution_sources
                    .get(path)
                    .and_then(|s| s.lines.as_ref())
                    .is_some_and(|lines| lines.contains(line))
            })
    };
    let console = Rc::clone(&ui.borrow().execucao.console);
    let graficos = Rc::clone(&ui.borrow().execucao.graficos);
    let telas_graf = Rc::clone(&ui.borrow().execucao.telas_graf);
    console.clear();
    if debug {
        for (path, line) in unbound {
            console.stderr(&format!("ponto de parada inativo em {path}:{line}: sem instrução executável, ou fonte com erro de sintaxe\n"));
        }
    }
    graficos.limpar();
    ui.borrow().execucao.telas_graf.limpar();
    clear_debug_views(ui);
    if let Some(d) = docs::diagnose(&fonte) {
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

    let run = exec::spawn(&ws, &name, fonte, debug, bps, args);
    telas_graf.set_eventos(Some(run.graf_eventos.clone()));
    {
        let mut u = ui.borrow_mut();
        u.execucao
            .execution
            .transition(ExecutionEvent::Started)
            .expect("prepared before starting a run");
        u.execucao.run = Some(Run {
            stopping: false,
            debug_cmd: run.debug_cmd.clone(),
            paused: None,
        });
    }
    set_debug_buttons(ui, false, true);

    let ui_ev = Rc::clone(ui);
    // Taken when the run ends, to send back what the program wrote.
    let mut ws = Some(ws);
    glib::timeout_add_local(std::time::Duration::from_millis(16), move || {
        for _ in 0..EVENTS_PER_TICK {
            match run.events.try_recv() {
                Ok(ExecEvent::Stdout(s)) => console.stdout(&s),
                Ok(ExecEvent::Stderr(s)) => console.stderr(&s),
                Ok(ExecEvent::Source(source)) => {
                    {
                        let mut u = ui_ev.borrow_mut();
                        Arc::make_mut(&mut u.execucao.execution_sources)
                            .insert(source.path.clone(), Arc::clone(&source));
                    }
                    if ui_ev
                        .borrow()
                        .execucao
                        .run
                        .as_ref()
                        .is_some_and(|run| run.stopping)
                    {
                        continue;
                    }
                    for doc in all_docs(&ui_ev) {
                        let matches = doc
                            .running_source
                            .borrow()
                            .as_ref()
                            .map(|s| s.path == source.path)
                            .unwrap_or_else(|| {
                                debug_sources::normalize_path(&doc.name()) == source.path
                            });
                        if matches {
                            doc.set_running_source(Some(Arc::clone(&source)));
                        }
                    }
                }
                Ok(ExecEvent::Paused(p)) => {
                    handle_debug_pause(&ui_ev, &p);
                }
                Ok(ExecEvent::CursorPosition(reply)) => {
                    let _ = reply.send(console.cursor_position());
                }
                Ok(ExecEvent::Leia(prompt)) => {
                    if ui_ev
                        .borrow()
                        .execucao
                        .run
                        .as_ref()
                        .is_some_and(|run| run.stopping)
                    {
                        continue;
                    }
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
                    telas_graf.set_eventos(None);
                    set_debug_buttons(&ui_ev, false, false);
                    clear_debug_views(&ui_ev);
                    // Files the program wrote (salve_arquivo, …) go back to
                    // the project.
                    let stopped = ui_ev
                        .borrow()
                        .execucao
                        .run
                        .as_ref()
                        .is_some_and(|run| run.stopping);
                    enviar_depois_da_execucao(
                        &ui_ev,
                        &console,
                        ws.take().expect("workspace belongs to the finishing run"),
                    );
                    if f.ok {
                        if stopped {
                            set_status(&ui_ev, "execução interrompida");
                        } else {
                            set_status(&ui_ev, &format!("ok — {name}"));
                        }
                    } else {
                        console.error_link(&format!(
                            "erro: {} em {}:{}\n",
                            f.error_message, f.error_file, f.error_line
                        ));
                        if !f.stack.is_empty() {
                            console.stderr("pilha de chamadas:\n");
                            let mut i = 0;
                            for fr in &f.stack {
                                console.error_link(&format!(
                                    "  {i}: {} em {}:{}\n",
                                    fr.name, fr.file, fr.line
                                ));
                                if fr.repeticoes > 0 {
                                    console.stderr(&format!(
                                        "  … a mesma chamada, mais {} vezes\n",
                                        fr.repeticoes
                                    ));
                                }
                                i += 1 + fr.repeticoes;
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
                            remember_execution_location(&ui_ev, &file, f.error_line, true);
                            if !stopped {
                                navigate_execution_error(&ui_ev, &file, f.error_line, true);
                            }
                        }
                    }
                    return glib::ControlFlow::Break;
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => break,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    // The run thread ended without `Finished`: clean up the
                    // same way, and say it instead of stopping silently.
                    console.cancel_input();
                    telas_graf.set_eventos(None);
                    set_debug_buttons(&ui_ev, false, false);
                    clear_debug_views(&ui_ev);
                    enviar_depois_da_execucao(
                        &ui_ev,
                        &console,
                        ws.take().expect("workspace belongs to the finishing run"),
                    );
                    console.stderr(&format!(
                        "{}: a execução parou sem terminar.\n",
                        exec::ERRO_INTERNO
                    ));
                    set_status(&ui_ev, "a execução parou com um erro interno");
                    return glib::ControlFlow::Break;
                }
            }
        }
        glib::ControlFlow::Continue
    });
}

/// After a run: send what the program wrote back to the project off the
/// GTK thread; failures remain in the durable journal for explicit retry.
pub(crate) fn enviar_depois_da_execucao(ui: &UiRc, console: &Rc<Console>, ws: Workspace) {
    finish_execution_view(ui, false);

    let (files, project) = {
        let mut u = ui.borrow_mut();
        u.execucao
            .execution
            .transition(ExecutionEvent::Finished)
            .expect("running before uploading its files");
        u.execucao.run = None;
        (u.files.clone(), u.project.clone())
    };
    let console = Rc::clone(console);
    async_result(
        ui,
        "Salvando arquivos da execução",
        false,
        move || Ok(ws.upload_changes(&files, &project)),
        move |ui, result| {
            let envio = result.unwrap_or_else(|e| exec::Envio {
                erros: vec![e],
                ..Default::default()
            });
            {
                let mut u = ui.borrow_mut();
                u.execucao
                    .execution
                    .transition(ExecutionEvent::Uploaded)
                    .expect("uploading before completing uploads");
            }
            let nao_enviados = mostrar_envio(ui, &console, envio);
            if nao_enviados > 0 {
                if !ui.borrow().files.is_remote() {
                    pendencias_ui::show(ui);
                }
                set_status(
                    ui,
                    &format!("não enviou {nao_enviados} arquivo(s): veja o console"),
                );
            }
        },
    );
}

/// After a run: refresh what the program changed, and say what failed.
/// Returns how many files could not be sent.
pub(crate) fn mostrar_envio(ui: &UiRc, console: &Console, envio: exec::Envio) -> usize {
    if let Some(path) = &envio.pending_dir {
        let help = if ui.borrow().files.is_remote() {
            "Revise os arquivos em Arquivo → Sincronização…"
        } else {
            "As cópias foram preservadas para tentar salvar novamente ou exportar."
        };
        console.stderr(&format!(
            "arquivos que não puderam ser salvos foram preservados em {}. {help}\n",
            path.display()
        ));
    }
    for (rel, texto) in &envio.enviados {
        ui.borrow().file_cache.borrow_mut().remove(rel);
        let (Some(doc), Some(texto)) = (doc_by_name(ui, rel), texto) else {
            continue;
        };
        if doc.is_dirty() {
            // Saving the tab now would replace what the program wrote.
            console.stderr(&format!(
                "aviso: o programa alterou `{rel}`, que tem alterações não salvas \
                 na aba. Salvar exigirá resolver o conflito.\n"
            ));
        } else if doc.text() != *texto {
            doc.recarregar(texto);
        }
    }
    for warning in &envio.avisos {
        console.stderr(&format!("aviso: {warning}\n"));
    }
    for conflict in &envio.conflitos {
        console.stderr(&format!(
            "conflito: `{}` mudou no projeto; a saída do programa não substituiu o arquivo.\n",
            conflict.path
        ));
        if let Ok(source) = std::str::from_utf8(&conflict.bytes) {
            let recovered = open_doc(ui, None, source);
            recovered.buffer.set_modified(true);
            recovered.tab.set_tooltip_text(Some(&format!(
                "Saída do programa para {} — salva como uma nova cópia",
                conflict.path
            )));
            sessao::copiar_nao_salvos(ui);
        } else {
            console
                .stderr("arquivo binário preservado; você pode salvar uma cópia em outra pasta.\n");
        }
    }
    for erro in &envio.erros {
        console.stderr(&format!(
            "erro: o arquivo que o programa escreveu não foi salvo no projeto: {erro}\n"
        ));
    }
    if !envio.enviados.is_empty() {
        refresh_tree(ui);
    }
    envio.erros.len() + envio.conflitos.len()
}
