//! File dialogs and Turma RPC off the UI thread.

use std::path::PathBuf;
use std::sync::{mpsc, Arc, Mutex};

use crate::turma::Turma;

pub enum ResultadoFundo {
    Abrir(Option<PathBuf>),
    Pasta(Option<PathBuf>),
    Salvar {
        path: Option<PathBuf>,
        forcar_nome: bool,
    },
    Conectou(Result<Turma, String>),
}

#[derive(Clone)]
pub struct Fundo {
    tx: mpsc::Sender<ResultadoFundo>,
    rx: Arc<Mutex<mpsc::Receiver<ResultadoFundo>>>,
}

impl Default for Fundo {
    fn default() -> Self {
        let (tx, rx) = mpsc::channel();
        Self {
            tx,
            rx: Arc::new(Mutex::new(rx)),
        }
    }
}

impl PartialEq for Fundo {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

impl Fundo {
    pub fn drenar(&self) -> Vec<ResultadoFundo> {
        let rx = self.rx.lock().unwrap();
        let mut out = Vec::new();
        while let Ok(r) = rx.try_recv() {
            out.push(r);
        }
        out
    }

    pub fn enviar(&self, r: ResultadoFundo) {
        let _ = self.tx.send(r);
    }
}

pub fn spawn_tokio(fut: impl std::future::Future<Output = ()> + Send + 'static) {
    match tokio::runtime::Handle::try_current() {
        Ok(h) => {
            h.spawn(fut);
        }
        Err(_) => {
            std::thread::spawn(move || {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build();
                if let Ok(rt) = rt {
                    rt.block_on(fut);
                }
            });
        }
    }
}

pub fn spawn_bloqueante(f: impl FnOnce() + Send + 'static) {
    match tokio::runtime::Handle::try_current() {
        Ok(h) => {
            h.spawn_blocking(f);
        }
        Err(_) => {
            std::thread::spawn(f);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drenar_esvazia_a_fila() {
        let f = Fundo::default();
        f.enviar(ResultadoFundo::Abrir(None));
        f.enviar(ResultadoFundo::Pasta(None));
        assert_eq!(f.drenar().len(), 2);
        assert!(f.drenar().is_empty());
    }
}
