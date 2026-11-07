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
        self.destacado = false;
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

pub fn png_de_rgba(largura: u32, altura: u32, rgba: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut out, largura.max(1), altura.max(1));
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        let mut w = enc.write_header().expect("png header");
        w.write_image_data(rgba).expect("png data");
        w.finish().expect("png finish");
    }
    out
}

pub fn png_folha(folha: &FolhaPronta) -> (u32, u32, Vec<u8>) {
    let n = folha.graficos.len().max(1) as u32;
    let h = CART_ALTURA_GRAFICO * n;
    let pix = pintar(folha, CART_LARGURA, h);
    let png = png_de_rgba(CART_LARGURA, h, &rgba_de_rgb32(&pix));
    (CART_LARGURA, h, png)
}

pub fn png_tela(tela: &TelaGrafica) -> (u32, u32, Vec<u8>) {
    let w = tela.largura.max(1);
    let h = tela.altura.max(1);
    let png = png_de_rgba(w, h, &rgba_de_rgb32(&tela.rgb32()));
    (w, h, png)
}

pub fn clique_na_tela(tela: &TelaGrafica, x: f64, y: f64, w: f64, h: f64) -> EventoGraf {
    let sx = if w <= 0.0 { 1.0 } else { tela.largura as f64 / w };
    let sy = if h <= 0.0 { 1.0 } else { tela.altura as f64 / h };
    EventoGraf::clique((x * sx) as i32, (y * sy) as i32, "esquerdo")
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
        let png = png_de_rgba(1, 1, &[255, 0, 0, 255]);
        assert!(png.starts_with(&[137, 80, 78, 71]));
    }
}
