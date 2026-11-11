//! Who shows a [`TelaGrafica`]. Tests stay silent; the CLI talks to a child.

use std::collections::VecDeque;
use std::time::Duration;

use super::graf::TelaGrafica;
#[cfg(feature = "graphics")]
use super::graf_proto::{self, TIPO_CLIQUE, TIPO_SAIR, TIPO_SOLTE, TIPO_TECLA};

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

    #[cfg(feature = "graphics")]
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

#[cfg(feature = "graphics")]
mod desktop;
#[cfg(feature = "graphics")]
pub use desktop::CliGrafHost;
