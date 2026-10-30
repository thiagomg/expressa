//! Bitmap canvas after drawing commands. Shared by tests, PNG save,
//! `expressa-grafico`, and the Aula window.

use std::io::Cursor;
use std::path::Path;

use super::folha::cor_rgb;
use super::fonte8x8;
use super::imagem::{carimbar, Imagem};

pub const LARGURA_MAX: u32 = 2048;
pub const ALTURA_MAX: u32 = 2048;

#[derive(Clone, Debug, PartialEq)]
pub enum OpGraf {
    Ponto {
        x: i32,
        y: i32,
        cor: String,
    },
    Linha {
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        cor: String,
    },
    Circulo {
        x: i32,
        y: i32,
        r: i32,
        cor: String,
    },
    /// PNG carimbado no centro `(x, y)`. `angulo` é em graus, no sentido
    /// horário, com 0 apontando para cima (a imagem já nasce assim).
    /// Pixels ficam na `figura`; o arquivo não é lido de novo.
    Imagem {
        x: i32,
        y: i32,
        angulo: f64,
        figura: Imagem,
    },
    /// Canto superior esquerdo `(x, y)`, y para baixo. `tamanho` é a
    /// altura da célula em pixels (padrão 8). Fundo do glifo transparente.
    Texto {
        x: i32,
        y: i32,
        tamanho: i32,
        cor: String,
        texto: String,
    },
    /// Inclusive corners; filled with the pen. Order of the two
    /// corners does not matter. Clips like `linha`.
    Retangulo {
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        cor: String,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct TelaGrafica {
    pub id: u64,
    pub largura: u32,
    pub altura: u32,
    pub fundo: String,
    pub ops: Vec<OpGraf>,
}

/// Palette index per pixel, then RGB for PNG / minifb.
pub struct Raster {
    pub largura: u32,
    pub altura: u32,
    idx: Vec<u8>,
}

const PALETA: &[&str] = &[
    "preto", "vermelho", "verde", "amarelo", "azul", "magenta", "ciano", "branco", "normal",
];

fn idx_cor(nome: &str) -> u8 {
    PALETA.iter().position(|n| *n == nome).unwrap_or(8) as u8
}

pub fn nome_cor(idx: u8) -> &'static str {
    PALETA.get(idx as usize).copied().unwrap_or("normal")
}

impl TelaGrafica {
    pub fn raster(&self) -> Raster {
        let w = self.largura.max(1);
        let h = self.altura.max(1);
        let fundo = idx_cor(&self.fundo);
        let mut idx = vec![fundo; (w * h) as usize];
        for op in &self.ops {
            match op {
                OpGraf::Ponto { x, y, cor } => {
                    plot_idx(&mut idx, w, h, *x, *y, idx_cor(cor));
                }
                OpGraf::Linha {
                    x1,
                    y1,
                    x2,
                    y2,
                    cor,
                } => {
                    linha_idx(&mut idx, w, h, *x1, *y1, *x2, *y2, idx_cor(cor));
                }
                OpGraf::Circulo { x, y, r, cor } => {
                    circulo_idx(&mut idx, w, h, *x, *y, *r, idx_cor(cor));
                }
                OpGraf::Texto {
                    x,
                    y,
                    tamanho,
                    cor,
                    texto,
                } => {
                    texto_idx(&mut idx, w, h, *x, *y, *tamanho, texto, idx_cor(cor));
                }
                OpGraf::Retangulo {
                    x1,
                    y1,
                    x2,
                    y2,
                    cor,
                } => {
                    retangulo_idx(&mut idx, w, h, *x1, *y1, *x2, *y2, idx_cor(cor));
                }
                OpGraf::Imagem { .. } => {}
            }
        }
        Raster {
            largura: w,
            altura: h,
            idx,
        }
    }

    /// Palette drawing, then any `imagem` stamps in true color.
    pub fn rgb32(&self) -> Vec<u32> {
        let mut buf = self.raster().rgb32();
        for op in &self.ops {
            let OpGraf::Imagem {
                x,
                y,
                angulo,
                figura,
            } = op
            else {
                continue;
            };
            carimbar(&mut buf, self.largura, self.altura, *x, *y, *angulo, figura);
        }
        buf
    }

    pub fn png_bytes(&self) -> Result<Vec<u8>, String> {
        encode_png(self.largura, self.altura, &self.rgb32())
    }

    pub fn write_png(&self, path: &Path) -> Result<(), String> {
        std::fs::write(path, self.png_bytes()?).map_err(|e| e.to_string())
    }
}

impl Raster {
    pub fn pixel(&self, x: i32, y: i32) -> Option<&'static str> {
        if x < 0 || y < 0 {
            return None;
        }
        let x = x as u32;
        let y = y as u32;
        if x >= self.largura || y >= self.altura {
            return None;
        }
        let i = (y * self.largura + x) as usize;
        Some(nome_cor(self.idx[i]))
    }

    pub fn rgb32(&self) -> Vec<u32> {
        self.idx
            .iter()
            .map(|&i| {
                let n = nome_cor(i);
                if n == "branco" {
                    0x00FFFFFF
                } else if n == "preto" {
                    0x00000000
                } else {
                    cor_rgb(n)
                }
            })
            .collect()
    }

    pub fn png_bytes(&self) -> Result<Vec<u8>, String> {
        encode_png(self.largura, self.altura, &self.rgb32())
    }

    pub fn write_png(&self, path: &Path) -> Result<(), String> {
        std::fs::write(path, self.png_bytes()?).map_err(|e| e.to_string())
    }
}

fn encode_png(w: u32, h: u32, rgb32: &[u32]) -> Result<Vec<u8>, String> {
    let mut raw = Vec::with_capacity((w * h * 3) as usize);
    for px in rgb32 {
        raw.push(((px >> 16) & 0xff) as u8);
        raw.push(((px >> 8) & 0xff) as u8);
        raw.push((px & 0xff) as u8);
    }
    let mut out = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut out, w, h);
        enc.set_color(png::ColorType::Rgb);
        enc.set_depth(png::BitDepth::Eight);
        let mut writer = enc.write_header().map_err(|e| e.to_string())?;
        writer.write_image_data(&raw).map_err(|e| e.to_string())?;
    }
    Ok(out)
}

pub fn png_to_rgb32(bytes: &[u8]) -> Result<(u32, u32, Vec<u32>), String> {
    let mut dec = png::Decoder::new(Cursor::new(bytes));
    dec.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = dec.read_info().map_err(|e| e.to_string())?;
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).map_err(|e| e.to_string())?;
    let w = info.width;
    let h = info.height;
    let mut out = Vec::with_capacity((w * h) as usize);
    match info.color_type {
        png::ColorType::Rgb => {
            for c in buf[..info.buffer_size()].chunks(3) {
                out.push(((c[0] as u32) << 16) | ((c[1] as u32) << 8) | c[2] as u32);
            }
        }
        png::ColorType::Rgba => {
            for c in buf[..info.buffer_size()].chunks(4) {
                out.push(((c[0] as u32) << 16) | ((c[1] as u32) << 8) | c[2] as u32);
            }
        }
        png::ColorType::Grayscale => {
            for &c in &buf[..info.buffer_size()] {
                let v = c as u32;
                out.push((v << 16) | (v << 8) | v);
            }
        }
        other => return Err(format!("PNG não suportado ({other:?})")),
    }
    Ok((w, h, out))
}

pub fn cor_graf_conhecida(nome: &str) -> bool {
    matches!(
        nome,
        "normal"
            | "preto"
            | "vermelho"
            | "verde"
            | "amarelo"
            | "azul"
            | "magenta"
            | "ciano"
            | "branco"
    )
}

fn retangulo_idx(idx: &mut [u8], w: u32, h: u32, x1: i32, y1: i32, x2: i32, y2: i32, cor: u8) {
    if w == 0 || h == 0 {
        return;
    }
    let (x1, x2) = if x1 <= x2 { (x1, x2) } else { (x2, x1) };
    let (y1, y2) = if y1 <= y2 { (y1, y2) } else { (y2, y1) };
    let x1 = x1.max(0);
    let y1 = y1.max(0);
    let x2 = x2.min(w as i32 - 1);
    let y2 = y2.min(h as i32 - 1);
    if x2 < x1 || y2 < y1 {
        return;
    }
    for y in y1..=y2 {
        let row = (y as u32 * w) as usize;
        for x in x1..=x2 {
            idx[row + x as usize] = cor;
        }
    }
}

fn plot_idx(idx: &mut [u8], w: u32, h: u32, x: i32, y: i32, cor: u8) {
    if x < 0 || y < 0 {
        return;
    }
    let x = x as u32;
    let y = y as u32;
    if x >= w || y >= h {
        return;
    }
    idx[(y * w + x) as usize] = cor;
}

fn linha_idx(idx: &mut [u8], w: u32, h: u32, x0: i32, y0: i32, x1: i32, y1: i32, cor: u8) {
    let mut x = x0;
    let mut y = y0;
    let dx = (x1 - x0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let dy = -(y1 - y0).abs();
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;
    loop {
        plot_idx(idx, w, h, x, y, cor);
        if x == x1 && y == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x += sx;
        }
        if e2 <= dx {
            err += dx;
            y += sy;
        }
    }
}

fn circulo_idx(idx: &mut [u8], w: u32, h: u32, cx: i32, cy: i32, r: i32, cor: u8) {
    if r < 0 {
        return;
    }
    if r == 0 {
        plot_idx(idx, w, h, cx, cy, cor);
        return;
    }
    let mut x = r;
    let mut y = 0;
    let mut err = 1 - r;
    while x >= y {
        plot_idx(idx, w, h, cx + x, cy + y, cor);
        plot_idx(idx, w, h, cx + y, cy + x, cor);
        plot_idx(idx, w, h, cx - y, cy + x, cor);
        plot_idx(idx, w, h, cx - x, cy + y, cor);
        plot_idx(idx, w, h, cx - x, cy - y, cor);
        plot_idx(idx, w, h, cx - y, cy - x, cor);
        plot_idx(idx, w, h, cx + y, cy - x, cor);
        plot_idx(idx, w, h, cx + x, cy - y, cor);
        y += 1;
        if err < 0 {
            err += 2 * y + 1;
        } else {
            x -= 1;
            err += 2 * (y - x) + 1;
        }
    }
}

fn texto_idx(idx: &mut [u8], w: u32, h: u32, x: i32, y: i32, tamanho: i32, texto: &str, cor: u8) {
    let t = tamanho.max(1);
    let mut cx = x;
    let mut cy = y;
    for ch in texto.chars() {
        if ch == '\r' {
            continue;
        }
        if ch == '\n' {
            cx = x;
            cy += t;
            continue;
        }
        if ch == '\t' {
            cx += 2 * t;
            continue;
        }
        let g = fonte8x8::glifo(ch);
        for py in 0..t {
            let fy = ((py * fonte8x8::LADO) / t) as usize;
            let row = g[fy];
            for px in 0..t {
                let fx = ((px * fonte8x8::LADO) / t) as usize;
                if row & (1 << fx) != 0 {
                    plot_idx(idx, w, h, cx + px, cy + py, cor);
                }
            }
        }
        cx += t;
    }
}

#[cfg(test)]
#[allow(clippy::identity_op)] // `1 * 4`: pixel 1, 4 bytes each
mod tests {
    use super::*;

    #[test]
    fn diagonal_paints_and_png_roundtrip() {
        let t = TelaGrafica {
            id: 1,
            largura: 16,
            altura: 16,
            fundo: "branco".into(),
            ops: vec![OpGraf::Linha {
                x1: 0,
                y1: 0,
                x2: 15,
                y2: 15,
                cor: "azul".into(),
            }],
        };
        let r = t.raster();
        assert_eq!(r.pixel(0, 0), Some("azul"));
        assert_eq!(r.pixel(15, 15), Some("azul"));
        assert_eq!(r.pixel(15, 0), Some("branco"));
        let bytes = r.png_bytes().unwrap();
        let (w, h, px) = png_to_rgb32(&bytes).unwrap();
        assert_eq!((w, h), (16, 16));
        assert_eq!(px.len(), 256);
        assert_eq!(px[0], 0x003366CC);
    }

    #[test]
    fn imagem_aponta_para_cima_e_vira_para_a_direita() {
        // 3×3, only the top-center pixel is red. The rest is transparent.
        let mut raw = vec![0u8; 3 * 3 * 4];
        raw[1 * 4] = 255;
        raw[1 * 4 + 3] = 255;
        let figura = Imagem::nova(3, 3, raw).unwrap();
        let tela = |angulo: f64| TelaGrafica {
            id: 1,
            largura: 16,
            altura: 16,
            fundo: "branco".into(),
            ops: vec![OpGraf::Imagem {
                x: 8,
                y: 8,
                angulo,
                figura: figura.clone(),
            }],
        };
        let cima = tela(0.0).rgb32();
        let direita = tela(90.0).rgb32();
        let at = |buf: &[u32], x: u32, y: u32| buf[(y * 16 + x) as usize];
        assert_eq!(at(&cima, 8, 7), 0x00FF0000, "topo fica acima do centro");
        assert_eq!(at(&cima, 8, 8), 0x00FFFFFF);
        assert_eq!(
            at(&direita, 9, 8),
            0x00FF0000,
            "90° leva o topo para a direita"
        );
        assert_eq!(at(&direita, 8, 7), 0x00FFFFFF);
    }

    #[test]
    fn texto_pinta_e_acento_nao_e_caixa() {
        let t = TelaGrafica {
            id: 1,
            largura: 64,
            altura: 16,
            fundo: "branco".into(),
            ops: vec![OpGraf::Texto {
                x: 0,
                y: 0,
                tamanho: 8,
                cor: "azul".into(),
                texto: "Aã".into(),
            }],
        };
        let r = t.raster();
        assert_eq!(r.pixel(0, 0), Some("branco"), "A não preenche o canto");
        assert_eq!(r.pixel(2, 0), Some("azul"));
        // segundo glifo (ã) começa em x=8; o til está no topo
        assert_eq!(r.pixel(8, 0), Some("branco"));
        let mut algum_azul = false;
        for x in 8..16 {
            if r.pixel(x, 0) == Some("azul") {
                algum_azul = true;
            }
        }
        assert!(algum_azul, "ã tem pixels no topo (til)");
    }
}
