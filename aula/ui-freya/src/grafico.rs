//! Cartesiano and tela_grafica panes for a Freya run.

use expressa::runtime::{EventoGraf, FolhaPronta, TelaGrafica, pintar};

const CART_LARGURA: u32 = 400;
const CART_ALTURA_GRAFICO: u32 = 220;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Graficos {
    pub folhas: Vec<FolhaPronta>,
    pub telas: Vec<TelaGrafica>,
    pub fechadas: Vec<u64>,
    pub destacado: bool,
}

impl Graficos {
    pub fn visivel(&self) -> bool {
        !self.folhas.is_empty() || !self.telas.is_empty()
    }

    pub fn limpar(&mut self) {
        self.folhas.clear();
        self.telas.clear();
        self.fechadas.clear();
    }

    pub fn aplicar_folha(&mut self, folha: FolhaPronta) {
        if self.fechadas.contains(&folha.id) {
            return;
        }
        if let Some(atual) = self.folhas.iter_mut().find(|f| f.id == folha.id) {
            *atual = folha;
        } else {
            self.folhas.push(folha);
        }
    }

    pub fn aplicar_tela(&mut self, tela: TelaGrafica) {
        if self.fechadas.contains(&tela.id) {
            return;
        }
        if let Some(atual) = self.telas.iter_mut().find(|t| t.id == tela.id) {
            *atual = tela;
        } else {
            self.telas.push(tela);
        }
    }

    pub fn fechar_tela(&mut self, id: u64) -> EventoGraf {
        self.telas.retain(|t| t.id != id);
        if !self.fechadas.contains(&id) {
            self.fechadas.push(id);
        }
        EventoGraf::sair()
    }
}

pub fn rgba_de_rgb32(pix: &[u32]) -> Vec<u8> {
    let mut out = Vec::with_capacity(pix.len() * 4);
    for p in pix {
        out.push(((*p >> 16) & 0xff) as u8);
        out.push(((*p >> 8) & 0xff) as u8);
        out.push((*p & 0xff) as u8);
        out.push(255);
    }
    out
}

pub fn rgba_folha(folha: &FolhaPronta) -> (u32, u32, Vec<u8>) {
    let n = folha.graficos.len().max(1) as u32;
    let h = CART_ALTURA_GRAFICO * n;
    let pix = pintar(folha, CART_LARGURA, h);
    (CART_LARGURA, h, rgba_de_rgb32(&pix))
}

pub fn rgba_tela(tela: &TelaGrafica) -> (u32, u32, Vec<u8>) {
    let w = tela.largura.max(1);
    let h = tela.altura.max(1);
    (w, h, rgba_de_rgb32(&tela.rgb32()))
}

pub fn clique_na_tela(tela: &TelaGrafica, x: f64, y: f64, w: f64, h: f64) -> EventoGraf {
    let sx = if w <= 0.0 { 1.0 } else { tela.largura as f64 / w };
    let sy = if h <= 0.0 { 1.0 } else { tela.altura as f64 / h };
    EventoGraf::clique((x * sx) as i32, (y * sy) as i32, "esquerdo")
}

/// Nomes iguais aos do GTK (`janela_graf::tecla_gdk`) e do visor minifb.
pub fn tecla_de_freya(key: &freya::prelude::Key) -> String {
    use freya::prelude::{Key, NamedKey};
    match key {
        Key::Named(NamedKey::Enter) => "Enter".into(),
        Key::Named(NamedKey::Escape) => "Esc".into(),
        Key::Named(NamedKey::Tab) => "Tab".into(),
        Key::Named(NamedKey::Backspace) => "Backspace".into(),
        Key::Named(NamedKey::ArrowUp) => "Cima".into(),
        Key::Named(NamedKey::ArrowDown) => "Baixo".into(),
        Key::Named(NamedKey::ArrowLeft) => "Esquerda".into(),
        Key::Named(NamedKey::ArrowRight) => "Direita".into(),
        Key::Character(c) if !c.chars().any(char::is_control) => c.to_lowercase(),
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use expressa::runtime::{CurvaPronta, GraficoPronto};

    #[test]
    fn mostre_substitui_mesma_id() {
        let mut g = Graficos::default();
        g.aplicar_folha(FolhaPronta {
            id: 1,
            graficos: vec![],
        });
        g.aplicar_folha(FolhaPronta {
            id: 1,
            graficos: vec![GraficoPronto {
                xmin: -1.0,
                xmax: 1.0,
                ymin: -1.0,
                ymax: 1.0,
                curvas: vec![CurvaPronta {
                    cor: "azul".into(),
                    pontos: vec![(0.0, Some(0.0))],
                }],
            }],
        });
        assert_eq!(g.folhas.len(), 1);
        assert_eq!(g.folhas[0].graficos.len(), 1);
    }

    #[test]
    fn fecha_ignora_proximo_mostre() {
        let mut g = Graficos::default();
        g.aplicar_tela(TelaGrafica {
            id: 7,
            largura: 8,
            altura: 8,
            fundo: "branco".into(),
            ops: vec![],
        });
        g.fechar_tela(7);
        g.aplicar_tela(TelaGrafica {
            id: 7,
            largura: 8,
            altura: 8,
            fundo: "azul".into(),
            ops: vec![],
        });
        assert!(g.telas.is_empty());
        g.limpar();
        g.aplicar_tela(TelaGrafica {
            id: 7,
            largura: 8,
            altura: 8,
            fundo: "azul".into(),
            ops: vec![],
        });
        assert_eq!(g.telas.len(), 1);
    }

    #[test]
    fn rgb32_vira_rgba() {
        assert_eq!(rgba_de_rgb32(&[0x00FF0000]), [255, 0, 0, 255]);
        let tela = TelaGrafica {
            id: 1,
            largura: 1,
            altura: 1,
            fundo: "vermelho".into(),
            ops: vec![],
        };
        let (w, h, rgba) = rgba_tela(&tela);
        assert_eq!((w, h), (1, 1));
        assert_eq!(rgba.len(), 4);
    }

    #[test]
    fn limpar_mantem_destacado() {
        let mut g = Graficos {
            destacado: true,
            ..Graficos::default()
        };
        g.aplicar_tela(TelaGrafica {
            id: 1,
            largura: 8,
            altura: 8,
            fundo: "branco".into(),
            ops: vec![],
        });
        g.limpar();
        assert!(g.telas.is_empty());
        assert!(g.fechadas.is_empty());
        assert!(g.destacado);
    }

    #[test]
    fn tecla_de_freya_bate_com_gtk() {
        use freya::prelude::{Key, NamedKey};
        assert_eq!(tecla_de_freya(&Key::Named(NamedKey::ArrowUp)), "Cima");
        assert_eq!(tecla_de_freya(&Key::Named(NamedKey::Escape)), "Esc");
        assert_eq!(tecla_de_freya(&Key::Named(NamedKey::Enter)), "Enter");
        assert_eq!(tecla_de_freya(&Key::Character(" ".into())), " ");
        assert_eq!(tecla_de_freya(&Key::Character("Q".into())), "q");
        assert_eq!(tecla_de_freya(&Key::Named(NamedKey::F5)), "");
    }
}
