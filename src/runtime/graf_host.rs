//! Graphics hosts and per-window keyboard snapshots.

use super::graf::TelaGrafica;
use std::collections::{BTreeSet, HashMap, VecDeque};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventoGraf {
    pub janela: u64,
    pub tipo: String,
    pub x: i32,
    pub y: i32,
    pub botao: String,
    pub tecla: String,
    /// Backend identity; distinguishes both physical Shift keys, for example.
    pub fisica: String,
    pub modificadores: BTreeSet<String>,
    pub repeticao: bool,
}

impl EventoGraf {
    pub fn nada() -> Self {
        Self {
            janela: 0,
            tipo: "nada".into(),
            x: 0,
            y: 0,
            botao: "nenhum".into(),
            tecla: String::new(),
            fisica: String::new(),
            modificadores: BTreeSet::new(),
            repeticao: false,
        }
    }
    pub fn sair() -> Self {
        Self {
            tipo: "sair".into(),
            ..Self::nada()
        }
    }
    pub fn clique(x: i32, y: i32, botao: &str) -> Self {
        Self {
            tipo: "clique".into(),
            x,
            y,
            botao: botao.into(),
            ..Self::nada()
        }
    }
    pub fn tecla(nome: impl Into<String>) -> Self {
        let tecla = nome.into();
        Self {
            tipo: "tecla".into(),
            fisica: tecla.clone(),
            tecla,
            ..Self::nada()
        }
    }
    pub fn soltura(nome: impl Into<String>) -> Self {
        Self {
            tipo: "tecla_solta".into(),
            ..Self::tecla(nome)
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CapturaEvento {
    pub teclas: BTreeSet<String>,
    pub apertadas: BTreeSet<String>,
    pub soltas: BTreeSet<String>,
    pub eventos: Vec<EventoGraf>,
    pub sair: bool,
}

/// Physical state and unconsumed transitions of exactly one window.
#[derive(Default)]
pub struct EstadoEntrada {
    fisicas: HashMap<String, String>,
    captura: CapturaEvento,
}

impl EstadoEntrada {
    pub fn registre(&mut self, mut e: EventoGraf) {
        match e.tipo.as_str() {
            "nada" => return,
            "desfoco" | "sair" => {
                let fisicas = std::mem::take(&mut self.fisicas);
                for tecla in fisicas.into_values().collect::<BTreeSet<_>>() {
                    self.captura.soltas.insert(tecla);
                }
                self.captura.teclas.clear();
                self.captura.sair |= e.tipo == "sair";
            }
            "tecla" => {
                if self.fisicas.contains_key(&e.fisica) {
                    return;
                } // OS autorepeat
                let nova = !self.fisicas.values().any(|k| k == &e.tecla);
                self.fisicas.insert(e.fisica.clone(), e.tecla.clone());
                self.captura.teclas.insert(e.tecla.clone());
                if nova {
                    self.captura.apertadas.insert(e.tecla.clone());
                }
            }
            "tecla_solta" => {
                let Some(tecla) = self.fisicas.remove(&e.fisica) else {
                    return;
                };
                e.tecla = tecla;
                if !self.fisicas.values().any(|k| k == &e.tecla) {
                    self.captura.teclas.remove(&e.tecla);
                    self.captura.soltas.insert(e.tecla.clone());
                }
            }
            _ => {}
        }
        e.modificadores = self
            .captura
            .teclas
            .iter()
            .filter(|k| matches!(k.as_str(), "shift" | "control" | "alt" | "super"))
            .cloned()
            .collect();
        self.captura.eventos.push(e);
    }
    pub fn capture(&mut self) -> CapturaEvento {
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

pub trait GrafHost {
    fn mostre(&mut self, tela: TelaGrafica) -> Result<(), String>;
    fn evento(&mut self, _tela: &TelaGrafica) -> Result<CapturaEvento, String> {
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
    estados: HashMap<u64, EstadoEntrada>,
}
impl GrafHost for CapturaGraf {
    fn mostre(&mut self, tela: TelaGrafica) -> Result<(), String> {
        self.telas.push(tela);
        Ok(())
    }
    fn evento(&mut self, tela: &TelaGrafica) -> Result<CapturaEvento, String> {
        while let Some(e) = self.eventos.pop_front() {
            let id = if e.janela == 0 { tela.id } else { e.janela };
            self.estados.entry(id).or_default().registre(e);
        }
        Ok(self.estados.entry(tela.id).or_default().capture())
    }
}
#[cfg(feature = "graphics")]
mod desktop;
#[cfg(feature = "graphics")]
pub use desktop::CliGrafHost;

#[cfg(test)]
mod tests {
    use super::*;

    fn set(names: &[&str]) -> BTreeSet<String> {
        names.iter().map(|s| (*s).into()).collect()
    }

    #[test]
    fn simultaneous_keys_persist_and_release_independently() {
        let mut estado = EstadoEntrada::default();
        estado.registre(EventoGraf::tecla("direita"));
        estado.registre(EventoGraf::tecla("espaco"));
        let first = estado.capture();
        assert_eq!(first.teclas, set(&["direita", "espaco"]));
        assert_eq!(first.apertadas, first.teclas);
        let held = estado.capture();
        assert_eq!(held.teclas, first.teclas);
        assert!(held.apertadas.is_empty());
        assert!(held.eventos.is_empty());
        estado.registre(EventoGraf::soltura("direita"));
        let released = estado.capture();
        assert_eq!(released.teclas, set(&["espaco"]));
        assert_eq!(released.soltas, set(&["direita"]));
        assert_eq!(released.eventos[0].tipo, "tecla_solta");
        assert!(estado.capture().soltas.is_empty());
    }

    #[test]
    fn quick_taps_and_event_order_survive_one_capture() {
        let mut estado = EstadoEntrada::default();
        estado.registre(EventoGraf::tecla("control"));
        estado.registre(EventoGraf::tecla("s"));
        estado.registre(EventoGraf::soltura("s"));
        estado.registre(EventoGraf::soltura("control"));
        let ev = estado.capture();
        assert!(ev.teclas.is_empty());
        assert_eq!(ev.apertadas, set(&["control", "s"]));
        assert_eq!(ev.soltas, ev.apertadas);
        assert_eq!(
            ev.eventos
                .iter()
                .map(|e| e.tipo.as_str())
                .collect::<Vec<_>>(),
            ["tecla", "tecla", "tecla_solta", "tecla_solta"]
        );
        assert_eq!(ev.eventos[1].modificadores, set(&["control"]));
        assert!(ev.eventos[3].modificadores.is_empty());
    }

    #[test]
    fn repeats_do_not_generate_new_presses() {
        let mut estado = EstadoEntrada::default();
        estado.registre(EventoGraf::tecla("espaco"));
        estado.capture();
        estado.registre(EventoGraf::tecla("espaco"));
        let ev = estado.capture();
        assert!(ev.apertadas.is_empty());
        assert!(ev.eventos.is_empty());
        assert_eq!(ev.teclas, set(&["espaco"]));
    }

    #[test]
    fn both_modifier_sides_and_focus_loss() {
        let mut estado = EstadoEntrada::default();
        let left = EventoGraf {
            fisica: "left".into(),
            ..EventoGraf::tecla("shift")
        };
        let right = EventoGraf {
            fisica: "right".into(),
            ..EventoGraf::tecla("shift")
        };
        estado.registre(left.clone());
        estado.registre(right);
        assert_eq!(estado.capture().apertadas, set(&["shift"]));
        estado.registre(EventoGraf {
            tipo: "tecla_solta".into(),
            ..left
        });
        let ev = estado.capture();
        assert_eq!(ev.teclas, set(&["shift"]));
        assert!(ev.soltas.is_empty());
        estado.registre(EventoGraf::tecla("direita"));
        estado.registre(EventoGraf {
            tipo: "desfoco".into(),
            ..EventoGraf::nada()
        });
        let ev = estado.capture();
        assert!(ev.teclas.is_empty());
        assert_eq!(ev.soltas, set(&["shift", "direita"]));
        assert!(!ev.sair);
        estado.registre(EventoGraf::tecla("shift"));
        estado.registre(EventoGraf::sair());
        let ev = estado.capture();
        assert!(ev.sair);
        assert_eq!(ev.soltas, set(&["shift"]));
        assert!(ev.teclas.is_empty());
        assert!(!estado.capture().sair);
    }

    #[test]
    fn each_window_has_its_own_pending_events_and_state() {
        let a = TelaGrafica {
            id: 1,
            largura: 8,
            altura: 8,
            tamanho_pixel: 1,
            fundo: "branco".into(),
            ops: vec![],
        };
        let b = TelaGrafica { id: 2, ..a.clone() };
        let mut host = CapturaGraf::default();
        host.eventos.push_back(EventoGraf {
            janela: a.id,
            ..EventoGraf::tecla("a")
        });
        host.eventos.push_back(EventoGraf {
            janela: b.id,
            ..EventoGraf::tecla("b")
        });
        assert_eq!(host.evento(&a).unwrap().apertadas, set(&["a"]));
        assert_eq!(host.evento(&b).unwrap().apertadas, set(&["b"]));
        assert_eq!(host.evento(&a).unwrap().teclas, set(&["a"]));
        assert!(host.evento(&b).unwrap().apertadas.is_empty());
    }
}
