use super::super::graf_proto::{self, FiguraIntern, Msg, write_msg};
use super::*;
use std::collections::{HashMap, HashSet};
use std::io::BufReader;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::thread;
struct Filho {
    stdin: Option<ChildStdin>,
    child: Option<Child>,
    rx: Receiver<Result<Msg, String>>,
    enviadas: HashSet<u32>,
    teclas: BTreeSet<String>,
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

    fn fim(&mut self, id: u64) -> CapturaEvento {
        let soltas = self
            .filhos
            .remove(&id)
            .map(|f| f.teclas.clone())
            .unwrap_or_default();
        CapturaEvento {
            sair: true,
            soltas,
            eventos: vec![EventoGraf {
                janela: id,
                ..EventoGraf::sair()
            }],
            ..Default::default()
        }
    }

    fn ensure(&mut self, tela: &TelaGrafica) -> Result<&mut Filho, String> {
        let morto = match self.filhos.get_mut(&tela.id) {
            Some(f) => !Self::vivo(f),
            None => true,
        };
        if morto {
            self.filhos.remove(&tela.id);
            let filho = spawn_live()?;
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
        if self.filhos.get_mut(&id).map(Self::vivo).unwrap_or(false) {
            let intern = &mut self.intern;
            let filho = self.filhos.get_mut(&id).unwrap();
            for msg in graf_proto::mensagens_de(&tela, intern, &mut filho.enviadas, false) {
                let stdin = filho
                    .stdin
                    .as_mut()
                    .ok_or_else(|| "visor sem stdin".to_string())?;
                write_msg(stdin, &msg).map_err(|e| format!("não falou com o visor: {e}"))?;
            }
            return Ok(());
        }
        self.ensure(&tela)?;
        Ok(())
    }

    fn evento(&mut self, tela: &TelaGrafica) -> Result<CapturaEvento, String> {
        let morto = match self.filhos.get_mut(&tela.id) {
            Some(f) => !Self::vivo(f),
            None => {
                self.ensure(tela)?;
                false
            }
        };
        if morto {
            return Ok(self.fim(tela.id));
        }
        let filho = self.filhos.get_mut(&tela.id).unwrap();
        let stdin = filho
            .stdin
            .as_mut()
            .ok_or_else(|| "visor sem stdin".to_string())?;
        if write_msg(stdin, &Msg::Espere { id: tela.id }).is_err() {
            return Ok(self.fim(tela.id));
        }
        // Wait only for transport, never for user input. The viewer answers each capture.
        match filho.rx.recv_timeout(std::time::Duration::from_millis(400)) {
            Ok(Ok(Msg::Evento { id, captura })) if id == tela.id => {
                filho.teclas = captura.teclas.clone();
                Ok(captura)
            }
            Ok(Ok(_)) => Err("resposta gráfica inesperada".into()),
            Ok(Err(_)) | Err(mpsc::RecvTimeoutError::Disconnected) => Ok(self.fim(tela.id)),
            Err(mpsc::RecvTimeoutError::Timeout) => {
                Err("visor não respondeu à captura de eventos".into())
            }
        }
    }
}

fn spawn_live() -> Result<Filho, String> {
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
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let mut r = BufReader::new(stdout);
        loop {
            match graf_proto::read_msg(&mut r) {
                Ok(m) => {
                    if tx.send(Ok(m)).is_err() {
                        break;
                    }
                }
                Err(e) => {
                    let _ = tx.send(Err(e.to_string()));
                    break;
                }
            }
        }
    });
    Ok(Filho {
        stdin: Some(stdin),
        child: Some(child),
        rx,
        enviadas: HashSet::new(),
        teclas: BTreeSet::new(),
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
