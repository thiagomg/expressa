//! Who shows a [`TelaGrafica`]. Tests stay silent; the CLI talks to a child.

use std::collections::{HashMap, HashSet, VecDeque};
use std::io::BufReader;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::Duration;

use super::graf::TelaGrafica;
use super::graf_proto::{
    self, write_msg, FiguraIntern, Msg, TIPO_CLIQUE, TIPO_SAIR, TIPO_SOLTE, TIPO_TECLA,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventoGraf {
    pub tipo: String,
    pub x: i32,
    pub y: i32,
    pub botao: String,
    pub tecla: String,
}

impl EventoGraf {
    pub fn nada() -> Self {
        Self {
            tipo: "nada".into(),
            x: 0,
            y: 0,
            botao: "nenhum".into(),
            tecla: String::new(),
        }
    }

    pub fn sair() -> Self {
        Self {
            tipo: "sair".into(),
            x: 0,
            y: 0,
            botao: "nenhum".into(),
            tecla: String::new(),
        }
    }

    pub fn clique(x: i32, y: i32, botao: &str) -> Self {
        Self {
            tipo: "clique".into(),
            x,
            y,
            botao: botao.into(),
            tecla: String::new(),
        }
    }

    pub fn tecla(nome: impl Into<String>) -> Self {
        Self {
            tipo: "tecla".into(),
            x: 0,
            y: 0,
            botao: "nenhum".into(),
            tecla: nome.into(),
        }
    }

    pub fn from_msg(tipo: u8, x: i32, y: i32, botao: u8, tecla: String) -> Self {
        Self {
            tipo: match tipo {
                TIPO_CLIQUE => "clique",
                TIPO_SOLTE => "solte",
                TIPO_TECLA => "tecla",
                TIPO_SAIR => "sair",
                _ => "nada",
            }
            .into(),
            x,
            y,
            botao: match botao {
                graf_proto::BOTAO_ESQUERDO => "esquerdo",
                graf_proto::BOTAO_DIREITO => "direito",
                graf_proto::BOTAO_MEIO => "meio",
                _ => "nenhum",
            }
            .into(),
            tecla,
        }
    }
}

pub trait GrafHost {
    fn mostre(&mut self, tela: TelaGrafica) -> Result<(), String>;

    fn evento(
        &mut self,
        _tela: &TelaGrafica,
        _timeout: Option<Duration>,
    ) -> Result<EventoGraf, String> {
        Err("evento() precisa de uma janela gráfica".into())
    }
}

pub struct NullGraf;

impl GrafHost for NullGraf {
    fn mostre(&mut self, _tela: TelaGrafica) -> Result<(), String> {
        Ok(())
    }
}

#[derive(Default)]
pub struct CapturaGraf {
    pub telas: Vec<TelaGrafica>,
    pub eventos: VecDeque<EventoGraf>,
}

impl GrafHost for CapturaGraf {
    fn mostre(&mut self, tela: TelaGrafica) -> Result<(), String> {
        self.telas.push(tela);
        Ok(())
    }

    fn evento(
        &mut self,
        _tela: &TelaGrafica,
        _timeout: Option<Duration>,
    ) -> Result<EventoGraf, String> {
        Ok(self.eventos.pop_front().unwrap_or_else(EventoGraf::nada))
    }
}

struct Filho {
    stdin: Option<ChildStdin>,
    child: Option<Child>,
    rx: Receiver<Result<Msg, String>>,
    enviadas: HashSet<u32>,
}

impl Drop for Filho {
    fn drop(&mut self) {
        // EOF to the viewer so it keeps the last frame; do not wait, or
        // `expressa casa.lep` would sit until the student closes the window.
        self.stdin.take();
        if let Some(mut child) = self.child.take() {
            match child.try_wait() {
                Ok(None) => std::mem::forget(child),
                _ => {}
            }
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
        match filho.child.as_mut().map(|c| c.try_wait()) {
            Some(Ok(None)) => true,
            _ => false,
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

    fn evento(
        &mut self,
        tela: &TelaGrafica,
        timeout: Option<Duration>,
    ) -> Result<EventoGraf, String> {
        let morto = match self.filhos.get_mut(&tela.id) {
            Some(f) => !Self::vivo(f),
            None => {
                self.ensure(tela)?;
                false
            }
        };
        if morto {
            self.filhos.remove(&tela.id);
            return Ok(EventoGraf::sair());
        }
        let timeout_ms = match timeout {
            None => -1,
            Some(d) => i32::try_from(d.as_millis().min(i32::MAX as u128)).unwrap_or(i32::MAX),
        };
        let filho = self.filhos.get_mut(&tela.id).unwrap();
        let stdin = filho
            .stdin
            .as_mut()
            .ok_or_else(|| "visor sem stdin".to_string())?;
        if write_msg(
            stdin,
            &Msg::Espere {
                id: tela.id,
                timeout_ms,
            },
        )
        .is_err()
        {
            self.filhos.remove(&tela.id);
            return Ok(EventoGraf::sair());
        }
        let wait = match timeout {
            None => Duration::from_secs(3600),
            Some(d) => d.saturating_add(Duration::from_millis(400)),
        };
        match filho.rx.recv_timeout(wait) {
            Ok(Ok(Msg::Evento {
                tipo,
                x,
                y,
                botao,
                tecla,
                ..
            })) => Ok(EventoGraf::from_msg(tipo, x, y, botao, tecla)),
            Ok(Ok(_)) => Ok(EventoGraf::nada()),
            Ok(Err(_)) => Ok(EventoGraf::sair()),
            Err(mpsc::RecvTimeoutError::Timeout) => Ok(EventoGraf::nada()),
            Err(mpsc::RecvTimeoutError::Disconnected) => Ok(EventoGraf::sair()),
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
