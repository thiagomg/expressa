//! Bounded editor transport. Dropping the owner cancels the worker without
//! joining it on the UI thread. The command-line REPL keeps its direct I/O.
use std::collections::VecDeque;
use std::io::{self, Write};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, SyncSender, TryRecvError, TrySendError};
use std::thread::JoinHandle;
use std::time::Duration;

use super::ReplSession;
use crate::runtime::debug::{DebugAction, DebugCtx, DebugHook};
use crate::runtime::value::Value;

pub const REPL_MAX_CODIGO: usize = 64 * 1024;
const MAX_PENDENTES: usize = 16;
const EVENTOS_NA_FILA: usize = 64;
const BYTES_POR_EVENTO: usize = 4096;
const MAX_RESULTADO: usize = 16 * 1024;
const INTERVALO_PARADA: Duration = Duration::from_millis(5);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IdSessao(u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IdAvaliacao(u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FluxoRepl {
    Stdout,
    Stderr,
}

#[derive(Debug, PartialEq, Eq)]
pub enum FimRepl {
    Encerrada,
    Saida(i32),
    Falha(String),
}

#[derive(Debug, PartialEq, Eq)]
pub enum ConteudoRepl {
    /// Raw, bounded chunks: decode each stream incrementally, not per event.
    Saida {
        fluxo: FluxoRepl,
        bytes: Vec<u8>,
    },
    Resultado(Result<Option<String>, String>),
    Terminou(FimRepl),
}

#[derive(Debug, PartialEq, Eq)]
pub struct EventoRepl {
    pub sessao: IdSessao,
    pub avaliacao: Option<IdAvaliacao>,
    pub conteudo: ConteudoRepl,
}

struct Pedido {
    id: IdAvaliacao,
    codigo: String,
}

#[derive(Default)]
struct Controle {
    encerrar: AtomicBool,
    atual: AtomicU64,
    concluida: AtomicU64,
    cancelar: AtomicU64,
}

impl Controle {
    fn interrompida(&self, id: IdAvaliacao) -> bool {
        self.encerrar.load(Ordering::SeqCst) || self.cancelar.load(Ordering::SeqCst) == id.0
    }
}

/// Owns exactly one worker, its cancellation state and bounded queues.
pub struct CanalRepl {
    id: IdSessao,
    proxima: u64,
    linhas: Option<SyncSender<Pedido>>,
    eventos: Receiver<EventoRepl>,
    controle: Arc<Controle>,
    pendentes: VecDeque<IdAvaliacao>,
    worker: JoinHandle<()>,
}

impl CanalRepl {
    pub fn nova() -> io::Result<Self> {
        Self::iniciar(executar)
    }

    fn iniciar(
        processar: fn(
            IdSessao,
            Receiver<Pedido>,
            &SyncSender<EventoRepl>,
            &Arc<Controle>,
        ) -> FimRepl,
    ) -> io::Result<Self> {
        static PROXIMA_SESSAO: AtomicU64 = AtomicU64::new(1);
        let id = IdSessao(PROXIMA_SESSAO.fetch_add(1, Ordering::Relaxed));
        let (linhas, rx) = mpsc::sync_channel(MAX_PENDENTES);
        let (tx, eventos) = mpsc::sync_channel(EVENTOS_NA_FILA);
        let controle = Arc::new(Controle::default());
        let c = controle.clone();
        let worker = std::thread::Builder::new()
            .name("repl".into())
            .spawn(move || {
                let fim = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    processar(id, rx, &tx, &c)
                }))
                .unwrap_or_else(|_| FimRepl::Falha("falha interna no interpretador".into()));
                c.atual.store(0, Ordering::SeqCst);
                // On explicit shutdown a full queue must never hold the worker.
                let _ = enviar_evento(
                    &tx,
                    &c,
                    EventoRepl {
                        sessao: id,
                        avaliacao: None,
                        conteudo: ConteudoRepl::Terminou(fim),
                    },
                    false,
                );
            })?;
        Ok(Self {
            id,
            proxima: 1,
            linhas: Some(linhas),
            eventos,
            controle,
            pendentes: VecDeque::new(),
            worker,
        })
    }

    pub fn id(&self) -> IdSessao {
        self.id
    }

    pub fn pendentes(&self) -> usize {
        self.pendentes.len()
    }

    pub fn finalizada(&self) -> bool {
        self.worker.is_finished()
    }

    /// Nonblocking, including when the command queue is full.
    pub fn enviar(&mut self, codigo: String) -> Result<IdAvaliacao, &'static str> {
        if codigo.len() > REPL_MAX_CODIGO {
            return Err("código grande demais para o REPL (limite: 64 KiB)");
        }
        let Some(tx) = self.linhas.as_ref() else {
            return Err("a sessão parou; aperte Reiniciar");
        };
        if self.pendentes.len() >= MAX_PENDENTES {
            return Err("há muitas avaliações na fila; aguarde ou aperte Reiniciar");
        }
        let id = IdAvaliacao(self.proxima);
        match tx.try_send(Pedido { id, codigo }) {
            Ok(()) => {
                self.proxima += 1;
                self.pendentes.push_back(id);
                Ok(id)
            }
            Err(TrySendError::Full(_)) => Err("a fila do REPL está cheia; aguarde"),
            Err(TrySendError::Disconnected(_)) => Err("a sessão parou; aperte Reiniciar"),
        }
    }

    pub fn receber(&mut self) -> Result<EventoRepl, TryRecvError> {
        let evento = self.eventos.try_recv()?;
        match &evento.conteudo {
            ConteudoRepl::Resultado(_) => self.pendentes.retain(|id| Some(*id) != evento.avaliacao),
            ConteudoRepl::Terminou(_) => self.encerrar(),
            ConteudoRepl::Saida { .. } => {}
        }
        Ok(evento)
    }

    /// Target a specific evaluation, so an old Stop cannot cancel later input.
    pub fn parar(&self) {
        let atual = self.controle.atual.load(Ordering::SeqCst);
        let concluida = self.controle.concluida.load(Ordering::SeqCst);
        let alvo = if atual != 0 {
            Some(atual)
        } else {
            self.pendentes
                .iter()
                .find(|id| id.0 > concluida)
                .map(|id| id.0)
        };
        if let Some(alvo) = alvo {
            self.controle.cancelar.store(alvo, Ordering::SeqCst);
        }
    }

    pub fn encerrar(&mut self) {
        self.controle.encerrar.store(true, Ordering::SeqCst);
        self.linhas.take();
        self.pendentes.clear();
    }
}

impl Drop for CanalRepl {
    fn drop(&mut self) {
        self.encerrar();
    }
}

fn enviar_evento(
    tx: &SyncSender<EventoRepl>,
    controle: &Controle,
    mut evento: EventoRepl,
    saida: bool,
) -> io::Result<()> {
    loop {
        if controle.encerrar.load(Ordering::SeqCst) {
            return Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "sessão encerrada",
            ));
        }
        if saida && evento.avaliacao.is_some_and(|id| controle.interrompida(id)) {
            // Write::write_all retries ErrorKind::Interrupted indefinitely.
            return Err(io::Error::other("interrompido"));
        }
        match tx.try_send(evento) {
            Ok(()) => return Ok(()),
            Err(TrySendError::Full(e)) => {
                evento = e;
                std::thread::sleep(INTERVALO_PARADA);
            }
            Err(TrySendError::Disconnected(_)) => {
                controle.encerrar.store(true, Ordering::SeqCst);
                return Err(io::Error::new(
                    io::ErrorKind::BrokenPipe,
                    "editor desconectado",
                ));
            }
        }
    }
}

struct Saida<'a> {
    sessao: IdSessao,
    avaliacao: &'a std::cell::Cell<IdAvaliacao>,
    fluxo: FluxoRepl,
    tx: &'a SyncSender<EventoRepl>,
    controle: &'a Controle,
}

impl Write for Saida<'_> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let len = bytes.len().min(BYTES_POR_EVENTO);
        if len == 0 {
            return Ok(0);
        }
        enviar_evento(
            self.tx,
            self.controle,
            EventoRepl {
                sessao: self.sessao,
                avaliacao: Some(self.avaliacao.get()),
                conteudo: ConteudoRepl::Saida {
                    fluxo: self.fluxo,
                    bytes: bytes[..len].to_vec(),
                },
            },
            true,
        )?;
        Ok(len)
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

struct Parada {
    controle: Arc<Controle>,
    avaliacao: std::rc::Rc<std::cell::Cell<IdAvaliacao>>,
}

impl DebugHook for Parada {
    fn before_stmt(&mut self, _: &DebugCtx<'_>) -> DebugAction {
        if self.controle.encerrar.load(Ordering::SeqCst) {
            DebugAction::Quit
        } else if self.controle.interrompida(self.avaliacao.get()) {
            DebugAction::Interrompido
        } else {
            DebugAction::Continue
        }
    }
    fn interrupted(&mut self) -> bool {
        self.controle.interrompida(self.avaliacao.get())
    }
}

fn limitar(mut texto: String) -> String {
    if texto.len() > MAX_RESULTADO {
        let mut fim = MAX_RESULTADO;
        while !texto.is_char_boundary(fim) {
            fim -= 1;
        }
        texto.truncate(fim);
        texto.push_str("… [resultado abreviado]");
        texto.shrink_to_fit();
    }
    texto
}

fn executar(
    id: IdSessao,
    linhas: Receiver<Pedido>,
    tx: &SyncSender<EventoRepl>,
    controle: &Arc<Controle>,
) -> FimRepl {
    let avaliacao = std::rc::Rc::new(std::cell::Cell::new(IdAvaliacao(0)));
    let mut out = Saida {
        sessao: id,
        avaliacao: &avaliacao,
        fluxo: FluxoRepl::Stdout,
        tx,
        controle,
    };
    let mut err = Saida {
        fluxo: FluxoRepl::Stderr,
        ..out
    };
    let mut input = io::Cursor::new(String::new());
    let mut session = ReplSession::with_hook(
        &mut input,
        &mut out,
        &mut err,
        Box::new(Parada {
            controle: controle.clone(),
            avaliacao: avaliacao.clone(),
        }),
    );
    while let Ok(pedido) = linhas.recv() {
        if controle.encerrar.load(Ordering::SeqCst) {
            break;
        }
        avaliacao.set(pedido.id);
        controle.atual.store(pedido.id.0, Ordering::SeqCst);
        let valor = session.eval(&pedido.codigo);
        let resultado = if controle.interrompida(pedido.id) {
            // Interruptible native waits use EvalError::Quit internally too.
            // Stop cancels this evaluation; only an explicit saia() ends the session.
            session.exit_code = None;
            Err("interrompido".into())
        } else {
            match valor {
                Ok(Value::Nada) => Ok(None),
                Ok(v) => Ok(Some(limitar(v.repl_format(session.vm.numero_locale)))),
                Err(e) => Err(limitar(e.to_string())),
            }
        };
        controle.concluida.store(pedido.id.0, Ordering::SeqCst);
        controle.atual.store(0, Ordering::SeqCst);
        if enviar_evento(
            tx,
            controle,
            EventoRepl {
                sessao: id,
                avaliacao: Some(pedido.id),
                conteudo: ConteudoRepl::Resultado(resultado),
            },
            false,
        )
        .is_err()
        {
            break;
        }
        if let Some(code) = session.exit_code {
            return FimRepl::Saida(code);
        }
    }
    FimRepl::Encerrada
}

/// Keeps at most the three trailing bytes of an incomplete UTF-8 character.
/// Invalid input is replaced, but valid characters split across writes survive.
#[derive(Default)]
pub struct TextoRepl {
    pendente: Vec<u8>,
}

impl TextoRepl {
    pub fn decodificar(&mut self, bytes: &[u8]) -> String {
        self.pendente.extend_from_slice(bytes);
        let mut texto = String::new();
        let mut inicio = 0;
        while inicio < self.pendente.len() {
            match std::str::from_utf8(&self.pendente[inicio..]) {
                Ok(s) => {
                    texto.push_str(s);
                    inicio = self.pendente.len();
                }
                Err(e) => {
                    let fim = inicio + e.valid_up_to();
                    texto.push_str(std::str::from_utf8(&self.pendente[inicio..fim]).unwrap());
                    inicio = fim;
                    if let Some(len) = e.error_len() {
                        texto.push('\u{fffd}');
                        inicio += len;
                    } else {
                        break;
                    }
                }
            }
        }
        self.pendente.drain(..inicio);
        texto
    }

    pub fn terminar(&mut self) -> String {
        let texto = String::from_utf8_lossy(&self.pendente).into_owned();
        self.pendente.clear();
        texto
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    fn esperar(pronto: impl Fn() -> bool) {
        let fim = Instant::now() + Duration::from_secs(5);
        while !pronto() {
            assert!(Instant::now() < fim, "REPL worker did not respond");
            std::thread::sleep(Duration::from_millis(1));
        }
    }

    fn proximo(c: &mut CanalRepl) -> EventoRepl {
        let fim = Instant::now() + Duration::from_secs(5);
        loop {
            match c.receber() {
                Ok(e) => {
                    assert_eq!(e.sessao, c.id());
                    return e;
                }
                Err(TryRecvError::Empty) if Instant::now() < fim => {
                    std::thread::sleep(Duration::from_millis(1))
                }
                Err(e) => panic!("missing REPL event: {e}"),
            }
        }
    }

    fn resultado(
        c: &mut CanalRepl,
        id: IdAvaliacao,
    ) -> (String, String, Result<Option<String>, String>) {
        let mut out = TextoRepl::default();
        let mut err = TextoRepl::default();
        let (mut stdout, mut stderr) = (String::new(), String::new());
        loop {
            let e = proximo(c);
            assert_eq!(e.avaliacao, Some(id));
            match e.conteudo {
                ConteudoRepl::Saida { fluxo, bytes } => {
                    assert!(bytes.len() <= BYTES_POR_EVENTO);
                    match fluxo {
                        FluxoRepl::Stdout => stdout.push_str(&out.decodificar(&bytes)),
                        FluxoRepl::Stderr => stderr.push_str(&err.decodificar(&bytes)),
                    }
                }
                ConteudoRepl::Resultado(r) => {
                    stdout.push_str(&out.terminar());
                    stderr.push_str(&err.terminar());
                    return (stdout, stderr, r);
                }
                outro => panic!("unexpected event: {outro:?}"),
            }
        }
    }

    #[test]
    fn bindings_streams_results_and_errors_keep_evaluation_identity() {
        let mut c = CanalRepl::nova().unwrap();
        let a = c.enviar("x = 20".into()).unwrap();
        let b = c
            .enviar("escreva(\"olá\")\nescreva_erro(\"atenção\")\nx + 1".into())
            .unwrap();
        let d = c.enviar("1 / 0".into()).unwrap();
        assert_eq!(
            resultado(&mut c, a),
            (String::new(), String::new(), Ok(None))
        );
        assert_eq!(
            resultado(&mut c, b),
            ("olá\n".into(), "atenção\n".into(), Ok(Some("21".into())))
        );
        assert!(
            resultado(&mut c, d)
                .2
                .unwrap_err()
                .contains("divisão por zero")
        );
        assert_eq!(c.pendentes(), 0);
    }

    #[test]
    fn printing_loop_streams_before_completion_and_stop_leaves_session_usable() {
        let mut c = CanalRepl::nova().unwrap();
        let a = c
            .enviar("enquanto verdadeiro { escreva(\"vivo\") }".into())
            .unwrap();
        let primeiro = proximo(&mut c);
        assert_eq!(primeiro.avaliacao, Some(a));
        assert!(matches!(primeiro.conteudo, ConteudoRepl::Saida { .. }));
        assert_eq!(c.controle.concluida.load(Ordering::SeqCst), 0);
        // No more draining until the evaluation has actually been interrupted.
        c.parar();
        esperar(|| c.controle.concluida.load(Ordering::SeqCst) == a.0);
        assert!(resultado(&mut c, a).2.unwrap_err().contains("interrompido"));
        c.parar(); // idle Stop must not affect future input
        let b = c.enviar("2 + 2".into()).unwrap();
        assert_eq!(resultado(&mut c, b).2, Ok(Some("4".into())));
    }

    #[test]
    fn stop_immediately_after_submit_and_behind_unread_results_targets_new_input() {
        let mut c = CanalRepl::nova().unwrap();
        let a = c.enviar("1".into()).unwrap();
        esperar(|| c.controle.concluida.load(Ordering::SeqCst) == a.0);
        let b = c.enviar("enquanto verdadeiro { x = 1 }".into()).unwrap();
        c.parar();
        assert_eq!(resultado(&mut c, a).2, Ok(Some("1".into())));
        assert!(resultado(&mut c, b).2.unwrap_err().contains("interrompido"));
        let d = c.enviar("3".into()).unwrap();
        assert_eq!(resultado(&mut c, d).2, Ok(Some("3".into())));
    }

    #[test]
    fn full_output_queue_is_cancellable_without_a_consumer() {
        for shutdown in [false, true] {
            let (tx, rx) = mpsc::sync_channel(1);
            let controle = Arc::new(Controle::default());
            let evento = || EventoRepl {
                sessao: IdSessao(1),
                avaliacao: Some(IdAvaliacao(1)),
                conteudo: ConteudoRepl::Saida {
                    fluxo: FluxoRepl::Stdout,
                    bytes: vec![b'a'],
                },
            };
            tx.send(evento()).unwrap();
            assert!(matches!(tx.try_send(evento()), Err(TrySendError::Full(_))));
            let c = controle.clone();
            let e = evento();
            let worker = std::thread::spawn(move || enviar_evento(&tx, &c, e, true));
            if shutdown {
                controle.encerrar.store(true, Ordering::SeqCst);
            } else {
                controle.cancelar.store(1, Ordering::SeqCst);
            }
            esperar(|| worker.is_finished());
            assert!(worker.join().unwrap().is_err());
            // Nothing drained or dropped from the original full queue.
            assert_eq!(rx.try_recv().unwrap(), evento());
        }
    }

    #[test]
    fn dropping_active_idle_and_backpressured_sessions_releases_workers() {
        for codigo in [
            None,
            Some("enquanto verdadeiro { x = 1 }"),
            Some("enquanto verdadeiro { escreva(\"x\") }"),
        ] {
            let mut c = CanalRepl::nova().unwrap();
            let controle = Arc::downgrade(&c.controle);
            if let Some(codigo) = codigo {
                let id = c.enviar(codigo.into()).unwrap();
                esperar(|| c.controle.atual.load(Ordering::SeqCst) == id.0);
            }
            drop(c);
            esperar(|| controle.upgrade().is_none());
        }
    }

    #[test]
    fn explicit_shutdown_discards_queued_work_and_quit_reports_exit_code() {
        let mut c = CanalRepl::nova().unwrap();
        let a = c.enviar("saia(7)".into()).unwrap();
        c.enviar("enquanto verdadeiro { x = 1 }".into()).unwrap();
        assert_eq!(resultado(&mut c, a).2, Ok(None));
        assert_eq!(
            proximo(&mut c).conteudo,
            ConteudoRepl::Terminou(FimRepl::Saida(7))
        );
        esperar(|| c.finalizada());
        assert_eq!(c.pendentes(), 0);
        assert!(c.enviar("1".into()).is_err());

        let mut c = CanalRepl::nova().unwrap();
        c.enviar("enquanto verdadeiro { x = 1 }".into()).unwrap();
        c.enviar("enquanto verdadeiro { escreva(\"should never run\") }".into())
            .unwrap();
        c.encerrar();
        esperar(|| c.finalizada());
        assert_eq!(c.pendentes(), 0);
    }

    #[test]
    fn worker_panic_is_reported_and_clears_pending_evaluations() {
        let mut c = CanalRepl::iniciar(|_, linhas, _, _| {
            let _ = linhas.recv().unwrap();
            panic!("injected worker failure");
        })
        .unwrap();
        c.enviar("1".into()).unwrap();
        assert!(matches!(
            proximo(&mut c).conteudo,
            ConteudoRepl::Terminou(FimRepl::Falha(_))
        ));
        assert_eq!(c.pendentes(), 0);
        assert!(c.enviar("2".into()).is_err());
        esperar(|| c.finalizada());
    }

    #[test]
    fn stopping_a_native_wait_keeps_the_session_available() {
        let mut c = CanalRepl::nova().unwrap();
        let id = c.enviar("durma(60)".into()).unwrap();
        esperar(|| c.controle.atual.load(Ordering::SeqCst) == id.0);
        // Allow entry into the native wait, whose interruption uses Quit internally.
        std::thread::sleep(Duration::from_millis(20));
        c.parar();
        assert!(
            resultado(&mut c, id)
                .2
                .unwrap_err()
                .contains("interrompido")
        );
        let proxima = c.enviar("42".into()).unwrap();
        assert_eq!(resultado(&mut c, proxima).2, Ok(Some("42".into())));
    }

    #[test]
    fn command_queue_and_payloads_are_bounded_and_rejected_input_is_not_counted() {
        let mut c = CanalRepl::nova().unwrap();
        assert!(c.enviar("x".repeat(REPL_MAX_CODIGO + 1)).is_err());
        assert_eq!(c.pendentes(), 0);
        for _ in 0..MAX_PENDENTES {
            c.enviar("1".into()).unwrap();
        }
        assert!(c.enviar("1".into()).is_err());
        assert_eq!(c.pendentes(), MAX_PENDENTES);
        let result = limitar("á".repeat(MAX_RESULTADO));
        assert!(result.ends_with("[resultado abreviado]"));
        assert!(result.len() < MAX_RESULTADO + 100);
    }

    #[test]
    fn utf8_survives_every_split_and_incomplete_characters_end_at_evaluation_boundary() {
        let texto = "ação 日本 🐈\n";
        for corte in 0..=texto.len() {
            let mut d = TextoRepl::default();
            let a = d.decodificar(&texto.as_bytes()[..corte]);
            assert!(d.pendente.len() <= 3);
            let b = d.decodificar(&texto.as_bytes()[corte..]);
            assert_eq!(format!("{a}{b}{}", d.terminar()), texto);
        }
        let mut d = TextoRepl::default();
        assert_eq!(d.decodificar(&[0xc3]), "");
        assert_eq!(d.terminar(), "�");
        assert_eq!(d.decodificar(&[0xa1, b'!']), "�!");
        assert_eq!(d.terminar(), "");
    }
}
