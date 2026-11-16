use super::*;
use crate::runtime::{CapturaEvento, CapturaGraf, EventoGraf, GrafHost, TelaGrafica};
use std::cell::Cell;
use std::collections::VecDeque;

fn backends() -> Vec<ExecutionBackend> {
    vec![
        ExecutionBackend::Bytecode,
        #[cfg(feature = "ast")]
        ExecutionBackend::Ast,
    ]
}

fn evaluate(
    source: &str,
    backend: ExecutionBackend,
    host: &mut dyn GrafHost,
    limit: Option<Duration>,
    hook: Box<dyn DebugHook>,
) -> (String, Result<i32, RuntimeError>) {
    let source = format!("tela_grafica = importe \"tela_grafica\"\n{source}");
    let mut out = Vec::new();
    let result = run_configured_with_io(
        &source,
        "laco.lep",
        hook,
        &mut io::Cursor::new(""),
        &mut out,
        &mut Vec::new(),
        ExecutionOptions {
            backend,
            time_limit: limit.or(Some(Duration::from_secs(2))),
            ..Default::default()
        },
        None,
        None,
        false,
        Some(host),
        &[],
    );
    (String::from_utf8(out).unwrap(), result)
}

const SETUP: &str = r#"
g = tela_grafica::nova(8, 8)
l = g.tela_grafica::laço_grafico()
"#;

#[test]
fn callbacks_receive_input_and_dt_and_each_frame_is_cleared() {
    for backend in backends() {
        let source = format!(
            r#"{SETUP}
s = mapa([:quadros -> 0, :log -> ""])
eventos = função(ev, dt) {{
    s:quadros += 1
    s:log += "e"
    escreva(tipo(ev:teclas), dt >= 0)
    se s:quadros == 1 {{
        escreva(dt == 0, ev:teclas contem :direita, ev:apertadas contem :direita)
    }}
    se s:quadros == 2 {{ escreva(ev:teclas contem :direita, tamanho(ev:apertadas)) }}
    se s:quadros == 3 {{ l.tela_grafica::pare() }}
}}
desenhe = função(tela) {{
    s:log += "d"
    tela.tela_grafica::ponto(s:quadros, 0)
}}
config = mapa([:fps -> 1000, :eventos -> eventos, :desenho -> desenhe])
l.tela_grafica::execute(config)
escreva(s:log)
"#
        );
        let mut host = CapturaGraf::default();
        host.eventos.push_back(EventoGraf::tecla("direita"));
        let (out, result) = evaluate(&source, backend, &mut host, None, Box::new(NoopHook));
        result.unwrap();
        assert_eq!(
            out,
            "conjunto verdadeiro\nverdadeiro verdadeiro verdadeiro\nconjunto verdadeiro\nverdadeiro 0\nconjunto verdadeiro\nedede\n",
            "{backend:?}"
        );
        assert_eq!(host.telas.len(), 3); // Initial opening and two presented frames.
        assert!(host.telas[0].ops.is_empty());
        assert_eq!(host.telas[1].ops.len(), 1);
        assert_eq!(host.telas[2].ops.len(), 1);
    }
}

#[test]
fn closing_delivers_a_final_event_without_drawing() {
    for backend in backends() {
        let mut host = CapturaGraf::default();
        host.eventos.push_back(EventoGraf::sair());
        let source = format!(
            r#"{SETUP}
eventos = função(ev, dt) {{ escreva(ev:sair, dt) }}
desenhe = função(tela) {{ escreva("não deve desenhar") }}
l.tela_grafica::execute(mapa([:fps -> 30, :eventos -> eventos, :desenho -> desenhe]))
"#
        );
        let (out, result) = evaluate(&source, backend, &mut host, None, Box::new(NoopHook));
        result.unwrap();
        assert_eq!(out, "verdadeiro 0\n");
        assert_eq!(host.telas.len(), 1);
    }
}

#[test]
fn stop_during_drawing_skips_presentation_and_can_restart() {
    for backend in backends() {
        let source = format!(
            r#"{SETUP}
eventos = função(ev, dt) {{ escreva("eventos") }}
desenhe = função(tela) {{ l.tela_grafica::pare() }}
config = mapa([:fps -> 30, :eventos -> eventos, :desenho -> desenhe])
l.tela_grafica::execute(config)
l.tela_grafica::execute(config)
repita 3 vezes {{ pare }}
"#
        );
        let mut host = CapturaGraf::default();
        let (out, result) = evaluate(&source, backend, &mut host, None, Box::new(NoopHook));
        result.unwrap();
        assert_eq!(out, "eventos\neventos\n");
        assert_eq!(host.telas.len(), 2);
    }
}

#[test]
fn callback_errors_propagate_and_release_execution_guard() {
    for backend in backends() {
        let source = format!(
            r#"{SETUP}
desenhe = função(tela) {{ tela.tela_grafica::ponto(0, 0) }}
eventos = função(ev, dt) {{ 1 / 0 }}
config = mapa([:fps -> 30, :eventos -> eventos, :desenho -> desenhe])
l.tela_grafica::execute(config) se_falhar {{ escreva("erro capturado") }}
config:eventos = função(ev, dt) {{ l.tela_grafica::pare() }}
l.tela_grafica::execute(config)
escreva("reiniciou")
"#
        );
        let mut host = CapturaGraf::default();
        let (out, result) = evaluate(&source, backend, &mut host, None, Box::new(NoopHook));
        result.unwrap();
        assert_eq!(out, "erro capturado\nreiniciou\n");
        assert_eq!(host.telas.len(), 2);
    }
}

#[test]
fn rejects_invalid_configuration_before_opening_a_window() {
    for backend in backends() {
        for (config, message) in [
            ("0", "mapa"),
            ("mapa()", "falta :fps"),
            ("mapa([:fps -> 0])", "fps deve"),
            ("mapa([:fps -> -1])", "fps deve"),
            ("mapa([:fps -> 0.00000000000000000001])", "fps deve"),
            ("mapa([:fps -> 10000000000000000000000000000])", "fps deve"),
            ("mapa([:fps -> 30])", "falta :eventos"),
            (
                "mapa([:fps -> 30, :eventos -> função(ev, dt) { 0 }])",
                "falta :desenho",
            ),
            (
                "mapa([:fps -> 30, :eventos -> 0, :desenho -> função(g) { 0 }])",
                ":eventos deve ser",
            ),
            (
                "mapa([:fps -> 30, :eventos -> função(ev) { 0 }, :desenho -> função(g) { 0 }])",
                ":eventos deve receber",
            ),
            (
                "mapa([:fps -> 30, :eventos -> função(ev, dt) { 0 }, :desenho -> função() { 0 }])",
                ":desenho deve receber",
            ),
            (
                "mapa([:fps -> 30, :eventos -> função(ev, dt) { 0 }, :desenho -> 0])",
                ":desenho deve ser",
            ),
            ("mapa([:fps -> 30, :fpps -> 30])", "opção desconhecida"),
        ] {
            let source = format!("{SETUP}\nl.tela_grafica::execute({config})");
            let mut host = CapturaGraf::default();
            let (_, result) = evaluate(&source, backend, &mut host, None, Box::new(NoopHook));
            let error = result.unwrap_err();
            assert!(
                error.message.contains(message),
                "{backend:?}: {config}: {}",
                error.message
            );
            assert!(host.telas.is_empty());
        }
    }
}

#[test]
fn rejects_a_forged_handle_and_cloning_a_loop() {
    for backend in backends() {
        for (source, message) in [
            ("mapa().tela_grafica::pare()", "esperado laço"),
            ("clone(g.tela_grafica::laco_grafico())", "clon"),
        ] {
            let mut host = CapturaGraf::default();
            let (_, result) = evaluate(
                &format!("{SETUP}\n{source}"),
                backend,
                &mut host,
                None,
                Box::new(NoopHook),
            );
            assert!(result.unwrap_err().message.contains(message));
            assert!(host.telas.is_empty());
        }
    }
}

#[test]
fn refuses_nested_execution_on_the_same_window() {
    for backend in backends() {
        let source = format!(
            r#"{SETUP}
outro = g.tela_grafica::laço_grafico()
config = mapa([:fps -> 30, :eventos -> função(ev, dt) {{ 0 }}, :desenho -> função(tela) {{ 0 }}])
eventos = função(ev, dt) {{ outro.tela_grafica::execute(config) }}
l.tela_grafica::execute(mapa([:fps -> 30, :eventos -> eventos, :desenho -> config:desenho]))
"#
        );
        let mut host = CapturaGraf::default();
        let (_, result) = evaluate(&source, backend, &mut host, None, Box::new(NoopHook));
        assert!(result.unwrap_err().message.contains("já existe um laço"));
        assert_eq!(host.telas.len(), 1);
    }
}

#[test]
fn deadline_interrupts_frame_pacing() {
    for backend in backends() {
        let source = format!(
            r#"{SETUP}
l.tela_grafica::execute(mapa([:fps -> 1, :eventos -> função(ev, dt) {{ 0 }}, :desenho -> função(tela) {{ 0 }}]))
"#
        );
        let mut host = CapturaGraf::default();
        let start = Instant::now();
        let (_, result) = evaluate(
            &source,
            backend,
            &mut host,
            Some(Duration::from_millis(30)),
            Box::new(NoopHook),
        );
        assert!(result.unwrap_err().message.contains("tempo esgotado"));
        assert!(start.elapsed() < Duration::from_secs(1));
        assert_eq!(host.telas.len(), 2);
    }
}

struct Scripted {
    input: VecDeque<CapturaEvento>,
    frames: usize,
    capture_delay: Duration,
    stop: Option<Rc<Cell<bool>>>,
}
impl GrafHost for Scripted {
    fn mostre(&mut self, _tela: TelaGrafica) -> Result<(), String> {
        self.frames += 1;
        if self.frames == 2
            && let Some(stop) = &self.stop
        {
            stop.set(true);
        }
        Ok(())
    }
    fn evento(&mut self, _tela: &TelaGrafica) -> Result<CapturaEvento, String> {
        std::thread::sleep(self.capture_delay);
        Ok(self.input.pop_front().unwrap_or_default())
    }
}

#[test]
fn dt_measures_elapsed_time_and_does_not_accumulate_a_catch_up_queue() {
    for backend in backends() {
        let source = format!(
            r#"{SETUP}
s = mapa([:n -> 0])
eventos = função(ev, dt) {{
    s:n += 1
    se s:n == 2 {{ escreva(dt >= 0.01) }}
    se ev:sair {{ escreva(s:n) }}
}}
l.tela_grafica::execute(mapa([:fps -> 1000, :eventos -> eventos, :desenho -> função(tela) {{ 0 }}]))
"#
        );
        let mut host = Scripted {
            input: VecDeque::from([
                CapturaEvento::default(),
                CapturaEvento {
                    sair: true,
                    ..Default::default()
                },
            ]),
            frames: 0,
            capture_delay: Duration::from_millis(12),
            stop: None,
        };
        let (out, result) = evaluate(&source, backend, &mut host, None, Box::new(NoopHook));
        result.unwrap();
        assert_eq!(out, "verdadeiro\n2\n");
        assert_eq!(host.frames, 2);
    }
}

struct Interrupt(Rc<Cell<bool>>);
impl DebugHook for Interrupt {
    fn before_stmt(&mut self, _ctx: &crate::runtime::debug::DebugCtx<'_>) -> DebugAction {
        DebugAction::Continue
    }
    fn interrupted(&mut self) -> bool {
        self.0.get()
    }
}

#[test]
fn stop_hook_interrupts_the_wait_between_frames() {
    for backend in backends() {
        let stop = Rc::new(Cell::new(false));
        let mut host = Scripted {
            input: VecDeque::new(),
            frames: 0,
            capture_delay: Duration::ZERO,
            stop: Some(Rc::clone(&stop)),
        };
        let source = format!(
            r#"{SETUP}
l.tela_grafica::execute(mapa([:fps -> 1, :eventos -> função(ev, dt) {{ 0 }}, :desenho -> função(tela) {{ 0 }}]))
"#
        );
        let start = Instant::now();
        let (_, result) = evaluate(&source, backend, &mut host, None, Box::new(Interrupt(stop)));
        assert_eq!(result.unwrap(), 0);
        assert!(start.elapsed() < Duration::from_secs(1));
        assert_eq!(host.frames, 2);
    }
}

#[test]
fn native_help_and_catalog_keep_process_and_graphical_execute_distinct() {
    use crate::runtime::nativas::{builtin_modulo, lookup_alvo};
    assert_eq!(builtin_modulo("tela_grafica::execute"), "tela_grafica");
    assert_eq!(
        lookup_alvo("tela_grafica::execute").unwrap().args_text(),
        "l, config"
    );
    assert_eq!(lookup_alvo("tela_grafica::pare").unwrap().name, "pare_laco");
    #[cfg(feature = "process")]
    {
        assert_eq!(builtin_modulo("execute"), "processo");
        assert_eq!(
            lookup_alvo("execute").unwrap().module.as_deref(),
            Some("processo")
        );
    }
}

#[test]
fn movement_example_runs_with_the_graphical_loop() {
    for backend in backends() {
        let source = include_str!("../../../../exemplos/tela_grafica/movimento.lep");
        let mut host = Scripted {
            input: VecDeque::from([
                CapturaEvento::default(),
                CapturaEvento {
                    sair: true,
                    ..Default::default()
                },
            ]),
            frames: 0,
            capture_delay: Duration::ZERO,
            stop: None,
        };
        let (out, result) = evaluate(source, backend, &mut host, None, Box::new(NoopHook));
        result.unwrap();
        assert!(out.is_empty());
        assert_eq!(host.frames, 2);
    }
}
