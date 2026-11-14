use super::super::graf_proto::{self, FiguraIntern, Msg, write_msg};
use super::*;
use std::collections::{HashMap, HashSet};
use std::io::BufReader;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
struct Filho {
    stdin: Option<ChildStdin>,
    child: Option<Child>,
    entrada: Arc<Mutex<EntradaRecebida>>,
    enviadas: HashSet<u32>,
}

/// Only transitions accumulate: held keys are replaced by the latest snapshot.
/// The reader never holds this lock while waiting for the viewer or the pipe.
#[derive(Default)]
struct EntradaRecebida {
    captura: CapturaEvento,
    encerrado: bool,
}

impl EntradaRecebida {
    fn receba(&mut self, captura: CapturaEvento) {
        self.captura.teclas = captura.teclas;
        self.captura.apertadas.extend(captura.apertadas);
        self.captura.soltas.extend(captura.soltas);
        self.captura.eventos.extend(captura.eventos);
        self.captura.sair |= captura.sair;
    }

    fn encerre(&mut self, id: u64) {
        self.encerrado = true;
        self.captura.soltas.append(&mut self.captura.teclas);
        if !self.captura.sair {
            self.captura.eventos.push(EventoGraf {
                janela: id,
                ..EventoGraf::sair()
            });
        }
        self.captura.sair = true;
    }

    fn capture(&mut self) -> CapturaEvento {
        let teclas = self.captura.teclas.clone();
        std::mem::replace(
            &mut self.captura,
            CapturaEvento {
                teclas,
                ..Default::default()
            },
        )
    }
}

impl Drop for Filho {
    fn drop(&mut self) {
        // EOF to the viewer so it keeps the last frame; do not wait, or
        // `expressa casa.lep` would sit until the student closes the window.
        self.stdin.take();
        if let Some(mut child) = self.child.take()
            && let Ok(None) = child.try_wait()
        {
            std::mem::forget(child)
        }
    }
}

#[derive(Default)]
pub struct CliGrafHost {
    filhos: HashMap<u64, Filho>,
    intern: FiguraIntern,
}

impl CliGrafHost {
    fn vivo(filho: &mut Filho) -> bool {
        matches!(filho.child.as_mut().map(|c| c.try_wait()), Some(Ok(None)))
    }

    fn ensure(&mut self, tela: &TelaGrafica) -> Result<&mut Filho, String> {
        let morto = match self.filhos.get_mut(&tela.id) {
            Some(f) => !Self::vivo(f),
            None => true,
        };
        if morto {
            self.filhos.remove(&tela.id);
            let filho = spawn_live(tela.id)?;
            self.filhos.insert(tela.id, filho);
            let intern = &mut self.intern;
            let filho = self.filhos.get_mut(&tela.id).unwrap();
            for msg in graf_proto::mensagens_de(tela, intern, &mut filho.enviadas, true) {
                let stdin = filho
                    .stdin
                    .as_mut()
                    .ok_or_else(|| "visor sem stdin".to_string())?;
                write_msg(stdin, &msg).map_err(|e| format!("não falou com o visor: {e}"))?;
            }
        }
        Ok(self.filhos.get_mut(&tela.id).unwrap())
    }
}

impl GrafHost for CliGrafHost {
    fn mostre(&mut self, tela: TelaGrafica) -> Result<(), String> {
        let id = tela.id;
        if let Some(filho) = self.filhos.get_mut(&id) {
            // Preserve a close that happened during the drawing callback.
            // evento() must deliver :sair before any subsequent frame reopens it.
            if !Self::vivo(filho) {
                return Ok(());
            }
            let intern = &mut self.intern;
            let filho = self.filhos.get_mut(&id).unwrap();
            for msg in graf_proto::mensagens_de(&tela, intern, &mut filho.enviadas, false) {
                let stdin = filho
                    .stdin
                    .as_mut()
                    .ok_or_else(|| "visor sem stdin".to_string())?;
                if write_msg(stdin, &msg).is_err() {
                    // The viewer may close between try_wait and this write.
                    // Let the next capture report its exit normally.
                    return Ok(());
                }
            }
            return Ok(());
        }
        self.ensure(&tela)?;
        Ok(())
    }

    fn evento(&mut self, tela: &TelaGrafica) -> Result<CapturaEvento, String> {
        if !self.filhos.contains_key(&tela.id) {
            self.ensure(tela)?;
        }
        // No request, pipe read or refresh wait on the VM thread. The reader
        // has already merged all received batches into this local accumulator.
        let (captura, encerrado) = {
            let mut entrada = self.filhos[&tela.id].entrada.lock().unwrap();
            let encerrado = entrada.encerrado;
            (entrada.capture(), encerrado)
        };
        if encerrado || captura.sair {
            self.filhos.remove(&tela.id);
        }
        Ok(captura)
    }
}

fn spawn_live(id: u64) -> Result<Filho, String> {
    let mut cmd = live_command()?;
    cmd.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit());
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0);
    }
    let mut child = cmd
        .spawn()
        .map_err(|e| format!("não abriu a janela do graf: {e}"))?;
    let stdin = child
        .stdin
        .take()
        .ok_or_else(|| "visor sem stdin".to_string())?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "visor sem stdout".to_string())?;
    let entrada = Arc::new(Mutex::new(EntradaRecebida::default()));
    let destino = Arc::downgrade(&entrada);
    thread::spawn(move || {
        let mut r = BufReader::new(stdout);
        while let Ok(Msg::Evento {
            id: janela,
            captura,
        }) = graf_proto::read_msg(&mut r)
        {
            if janela != id {
                break;
            }
            let Some(destino) = destino.upgrade() else {
                break;
            };
            destino.lock().unwrap().receba(captura);
        }
        // Drain the pipe before reporting EOF, preserving the final tap/close
        // even if the native process has already exited.
        if let Some(destino) = destino.upgrade() {
            destino.lock().unwrap().encerre(id);
        }
    });
    Ok(Filho {
        stdin: Some(stdin),
        child: Some(child),
        entrada,
        enviadas: HashSet::new(),
    })
}

fn live_command() -> Result<Command, String> {
    // Same binary as the parent, so `--ao-vivo` matches this build (a
    // leftover `expressa-grafico` next to it may still be the file viewer).
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let mut cmd = Command::new(exe);
    cmd.arg("--ao-vivo");
    Ok(cmd)
}

#[cfg(test)]
mod loop_tests {
    use super::*;

    fn tela(id: u64) -> TelaGrafica {
        TelaGrafica {
            id,
            largura: 8,
            altura: 8,
            tamanho_pixel: 1,
            fundo: "branco".into(),
            ops: vec![],
        }
    }

    #[test]
    fn capture_never_needs_a_pipe_reply_and_keeps_held_keys() {
        let entrada = Arc::new(Mutex::new(EntradaRecebida::default()));
        let mut host = CliGrafHost::default();
        // No stdin or process: a capture must use only the local accumulator.
        host.filhos.insert(
            1,
            Filho {
                stdin: None,
                child: None,
                entrada: entrada.clone(),
                enviadas: HashSet::new(),
            },
        );
        assert_eq!(host.evento(&tela(1)).unwrap(), CapturaEvento::default());
        let mut origem = EstadoEntrada::default();
        origem.registre(EventoGraf::tecla("direita"));
        entrada.lock().unwrap().receba(origem.capture());
        assert!(host.evento(&tela(1)).unwrap().apertadas.contains("direita"));
        for _ in 0..1000 {
            let captura = host.evento(&tela(1)).unwrap();
            assert!(captura.teclas.contains("direita"));
            assert!(captura.apertadas.is_empty());
            assert!(captura.eventos.is_empty());
        }
        origem.registre(EventoGraf::soltura("direita"));
        entrada.lock().unwrap().receba(origem.capture());
        let captura = host.evento(&tela(1)).unwrap();
        assert!(captura.teclas.is_empty());
        assert!(captura.soltas.contains("direita"));
    }

    #[test]
    fn batches_preserve_quick_taps_order_modifiers_and_the_latest_state() {
        let mut origem = EstadoEntrada::default();
        let mut destino = EntradaRecebida::default();
        origem.registre(EventoGraf::tecla("control"));
        destino.receba(origem.capture());
        for _ in 0..100 {
            origem.registre(EventoGraf::tecla("s"));
            destino.receba(origem.capture());
            origem.registre(EventoGraf::soltura("s"));
            destino.receba(origem.capture());
        }
        let captura = destino.capture();
        assert_eq!(captura.teclas, BTreeSet::from(["control".into()]));
        assert_eq!(
            captura.apertadas,
            BTreeSet::from(["control".into(), "s".into()])
        );
        assert_eq!(captura.soltas, BTreeSet::from(["s".into()]));
        assert_eq!(captura.eventos.len(), 201);
        for par in captura.eventos[1..].chunks_exact(2) {
            assert_eq!(par[0].tipo, "tecla");
            assert_eq!(par[1].tipo, "tecla_solta");
            assert!(par[0].modificadores.contains("control"));
        }
        assert!(destino.capture().eventos.is_empty());
        origem.registre(EventoGraf {
            tipo: "desfoco".into(),
            ..EventoGraf::nada()
        });
        destino.receba(origem.capture());
        let captura = destino.capture();
        assert!(captura.teclas.is_empty());
        assert!(captura.soltas.contains("control"));
    }

    #[test]
    fn eof_preserves_unconsumed_transitions_and_releases_held_keys() {
        let mut origem = EstadoEntrada::default();
        let mut destino = EntradaRecebida::default();
        origem.registre(EventoGraf::tecla("direita"));
        destino.receba(origem.capture());
        destino.encerre(7);
        let captura = destino.capture();
        assert!(captura.sair);
        assert!(captura.teclas.is_empty());
        assert!(captura.apertadas.contains("direita"));
        assert!(captura.soltas.contains("direita"));
        assert_eq!(captura.eventos.len(), 2);
        assert_eq!(captura.eventos[1].janela, 7);
        // Explicit native close followed by EOF must not duplicate :sair.
        let mut destino = EntradaRecebida::default();
        origem.registre(EventoGraf::sair());
        destino.receba(origem.capture());
        destino.encerre(7);
        assert_eq!(
            destino
                .capture()
                .eventos
                .iter()
                .filter(|e| e.tipo == "sair")
                .count(),
            1
        );
    }

    #[test]
    fn presenting_after_close_keeps_exit_pending_instead_of_reopening() {
        let mut entrada = EntradaRecebida::default();
        entrada.captura.teclas.insert("direita".into());
        entrada.encerre(1);
        let mut host = CliGrafHost::default();
        host.filhos.insert(
            1,
            Filho {
                stdin: None,
                child: None,
                entrada: Arc::new(Mutex::new(entrada)),
                enviadas: HashSet::new(),
            },
        );
        let tela = TelaGrafica {
            id: 1,
            largura: 8,
            altura: 8,
            tamanho_pixel: 1,
            fundo: "branco".into(),
            ops: vec![],
        };
        host.mostre(tela.clone()).unwrap();
        assert!(host.filhos.contains_key(&1));
        let ev = host.evento(&tela).unwrap();
        assert!(ev.sair);
        assert!(ev.teclas.is_empty());
        assert!(ev.soltas.contains("direita"));
        assert!(!host.filhos.contains_key(&1));
    }
}
