//! Bitmap canvas after drawing commands. Shared by tests, PNG save,
//! `expressa-grafico`, and the Aula window.

#[cfg(feature = "graphics")]
use std::io::Cursor;
#[cfg(feature = "graphics")]
use std::path::Path;

use super::folha::cor_rgb;
use super::fonte8x8;
#[cfg(feature = "graphics")]
use super::imagem;
use super::imagem::{Imagem, carimbar_rgba};

#[cfg(feature = "graphics")]
pub const LARGURA_MAX: u32 = 2048;
#[cfg(feature = "graphics")]
pub const ALTURA_MAX: u32 = 2048;
/// Cada pixel lógico vira um quadrado `n`×`n` na janela (1…16).
pub const TAMANHO_PIXEL_MAX: u32 = 16;
/// Largest absolute coordinate or radius the native accepts. Twice the
/// maximum canvas side (`LARGURA_MAX`), so a stroke may start off-screen.
pub const LIMITE_COORD: i32 = 4096;
#[cfg(feature = "graphics")]
const _: () = assert!(LIMITE_COORD == LARGURA_MAX as i32 * 2);

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
    /// O ponto local `eixo` da imagem fica em `(x, y)`. `angulo` é em graus, no sentido
    /// horário, com 0 apontando para cima (a imagem já nasce assim).
    /// Pixels ficam na `figura`; o arquivo não é lido de novo.
    Imagem {
        eixo: [f64; 2],
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
    /// Lado do quadrado na janela para cada pixel do canvas. 1 = 1:1.
    pub tamanho_pixel: u32,
    pub fundo: String,
    pub ops: Vec<OpGraf>,
}

/// Named palette entries and packed RGB colors, shared by drawing primitives.
pub struct Raster {
    pub largura: u32,
    pub altura: u32,
    idx: Vec<u32>,
    cores_rgb: std::collections::HashMap<u32, String>,
}

pub(crate) const PALETA: &[&str] = &[
    "preto",
    "vermelho",
    "verde",
    "amarelo",
    "azul",
    "magenta",
    "ciano",
    "branco",
    "normal",
    "transparente",
];

// Values below this offset retain the identity of named colors, even when
// their RGB channels match a hexadecimal color. u32::MAX means untouched.
const RGB_BASE: u32 = 16;

fn parse_rgb(nome: &str) -> Option<u32> {
    let hex = nome.strip_prefix('#')?;
    if hex.len() != 6 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    u32::from_str_radix(hex, 16).ok()
}

fn idx_cor(nome: &str) -> u32 {
    parse_rgb(nome)
        .map(|rgb| rgb + RGB_BASE)
        .unwrap_or_else(|| PALETA.iter().position(|n| *n == nome).unwrap_or(8) as u32)
}

fn rgba_de_idx(cor: u32) -> [u8; 4] {
    if cor >= RGB_BASE {
        let rgb = cor - RGB_BASE;
        [(rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8, 255]
    } else {
        rgba_de_nome(nome_cor(cor as u8))
    }
}

pub fn nome_cor(idx: u8) -> &'static str {
    PALETA.get(idx as usize).copied().unwrap_or("normal")
}

impl OpGraf {
    fn rasterize(&self, idx: &mut [u32], w: u32, h: u32) -> bool {
        match self {
            OpGraf::Ponto { x, y, cor } => {
                plot_idx(idx, w, h, *x, *y, idx_cor(cor));
            }
            OpGraf::Linha {
                x1,
                y1,
                x2,
                y2,
                cor,
            } => {
                linha_idx(idx, w, h, *x1, *y1, *x2, *y2, idx_cor(cor));
            }
            OpGraf::Circulo { x, y, r, cor } => {
                circulo_idx(idx, w, h, *x, *y, *r, idx_cor(cor));
            }
            OpGraf::Texto {
                x,
                y,
                tamanho,
                cor,
                texto,
            } => {
                texto_idx(idx, w, h, *x, *y, *tamanho, texto, idx_cor(cor));
            }
            OpGraf::Retangulo {
                x1,
                y1,
                x2,
                y2,
                cor,
            } => {
                retangulo_idx(idx, w, h, *x1, *y1, *x2, *y2, idx_cor(cor));
            }
            OpGraf::Imagem { .. } => return false,
        }
        true
    }
}

impl TelaGrafica {
    pub fn tamanho_pixel_efetivo(&self) -> u32 {
        self.tamanho_pixel.clamp(1, TAMANHO_PIXEL_MAX)
    }

    pub fn janela_largura(&self) -> u32 {
        self.largura.saturating_mul(self.tamanho_pixel_efetivo())
    }

    pub fn janela_altura(&self) -> u32 {
        self.altura.saturating_mul(self.tamanho_pixel_efetivo())
    }

    pub fn raster(&self) -> Raster {
        let w = self.largura.max(1);
        let h = self.altura.max(1);
        let fundo = idx_cor(&self.fundo);
        let mut idx = vec![fundo; (w * h) as usize];
        for op in &self.ops {
            op.rasterize(&mut idx, w, h);
        }
        let cores_rgb = idx
            .iter()
            .copied()
            .filter(|&cor| cor >= RGB_BASE)
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .map(|cor| (cor, format!("#{:06X}", cor - RGB_BASE)))
            .collect();
        Raster {
            cores_rgb,
            largura: w,
            altura: h,
            idx,
        }
    }

    /// Composite in command order, flattening transparency onto the window paper.
    pub fn rgb32(&self) -> Vec<u32> {
        let papel = rgba_de_nome(if self.fundo == "transparente" {
            "branco"
        } else {
            &self.fundo
        });
        self.como_imagem()
            .px
            .chunks_exact(4)
            .map(|p| {
                let a = p[3] as u32;
                let channel = |i: usize| (p[i] as u32 * a + papel[i] as u32 * (255 - a)) / 255;
                (channel(0) << 16) | (channel(1) << 8) | channel(2)
            })
            .collect()
    }

    /// Palette primitives and image stamps share the same ordered RGBA composition.
    pub fn como_imagem(&self) -> Imagem {
        let w = self.largura.max(1);
        let h = self.altura.max(1);
        let fundo = rgba_de_nome(&self.fundo);
        let mut px = fundo.repeat((w * h) as usize);
        // u32::MAX means untouched, unlike the actual :transparente drawing color.
        let mut camada = vec![u32::MAX; (w * h) as usize];
        let mut suja = false;
        for op in &self.ops {
            if let OpGraf::Imagem {
                eixo,
                x,
                y,
                angulo,
                figura,
            } = op
            {
                if suja {
                    aplique_camada(&mut px, &camada);
                    camada.fill(u32::MAX);
                    suja = false;
                }
                carimbar_rgba(&mut px, w, h, *x, *y, *angulo, *eixo, figura);
            } else {
                suja |= op.rasterize(&mut camada, w, h);
            }
        }
        if suja {
            aplique_camada(&mut px, &camada);
        }
        Imagem {
            largura: w,
            altura: h,
            px: std::sync::Arc::from(px),
        }
    }

    #[cfg(feature = "graphics")]
    pub fn png_bytes(&self) -> Result<Vec<u8>, String> {
        imagem::encode_png_rgba(&self.como_imagem())
    }

    #[cfg(feature = "graphics")]
    pub fn write_png(&self, path: &Path) -> Result<(), String> {
        std::fs::write(path, self.png_bytes()?).map_err(|e| e.to_string())
    }
}

fn aplique_camada(px: &mut [u8], camada: &[u32]) {
    for (pixel, &cor) in px.chunks_exact_mut(4).zip(camada) {
        if cor != u32::MAX {
            pixel.copy_from_slice(&rgba_de_idx(cor));
        }
    }
}

impl Raster {
    pub fn pixel(&self, x: i32, y: i32) -> Option<&str> {
        if x < 0 || y < 0 {
            return None;
        }
        let x = x as u32;
        let y = y as u32;
        if x >= self.largura || y >= self.altura {
            return None;
        }
        let i = (y * self.largura + x) as usize;
        if self.idx[i] >= RGB_BASE {
            self.cores_rgb.get(&self.idx[i]).map(String::as_str)
        } else {
            Some(nome_cor(self.idx[i] as u8))
        }
    }

    pub fn rgb32(&self) -> Vec<u32> {
        self.idx
            .iter()
            .map(|&i| {
                if i >= RGB_BASE {
                    i - RGB_BASE
                } else {
                    rgb32_de_nome(nome_cor(i as u8))
                }
            })
            .collect()
    }

    #[cfg(feature = "graphics")]
    pub fn png_bytes(&self) -> Result<Vec<u8>, String> {
        encode_png(self.largura, self.altura, &self.rgb32())
    }

    #[cfg(feature = "graphics")]
    pub fn write_png(&self, path: &Path) -> Result<(), String> {
        std::fs::write(path, self.png_bytes()?).map_err(|e| e.to_string())
    }
}

#[cfg(feature = "graphics")]
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

#[cfg(feature = "graphics")]
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

#[cfg(feature = "graphics")]
pub fn cor_graf_conhecida(nome: &str) -> bool {
    parse_rgb(nome).is_some()
        || matches!(
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
                | "transparente"
        )
}

pub fn cor_tela_rgb32(nome: &str) -> u32 {
    rgb32_de_nome(nome)
}

fn rgb32_de_nome(nome: &str) -> u32 {
    if let Some(rgb) = parse_rgb(nome) {
        return rgb;
    }
    match nome {
        "branco" | "transparente" => 0x00FFFFFF,
        "preto" => 0x00000000,
        _ => cor_rgb(nome),
    }
}

fn rgba_de_nome(nome: &str) -> [u8; 4] {
    if nome == "transparente" {
        return [0, 0, 0, 0];
    }
    let c = rgb32_de_nome(nome);
    [
        ((c >> 16) & 0xff) as u8,
        ((c >> 8) & 0xff) as u8,
        (c & 0xff) as u8,
        255,
    ]
}

fn retangulo_idx(idx: &mut [u32], w: u32, h: u32, x1: i32, y1: i32, x2: i32, y2: i32, cor: u32) {
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

fn plot_idx(idx: &mut [u32], w: u32, h: u32, x: i32, y: i32, cor: u32) {
    plot_i64(idx, w, h, i64::from(x), i64::from(y), cor);
}

fn plot_i64(idx: &mut [u32], w: u32, h: u32, x: i64, y: i64, cor: u32) {
    if x < 0 || y < 0 || x >= i64::from(w) || y >= i64::from(h) {
        return;
    }
    idx[(y as u32 * w + x as u32) as usize] = cor;
}

/// Pixels of this Bresenham segment that land inside the canvas.
/// Each column (or row, when the line is steeper) is one step of
/// `y = (2*k*dy + dx) / (2*dx)`. A truncating edge intersection drops a
/// shallow stroke or paints a whole border the segment never touches.
fn linha_idx(idx: &mut [u32], w: u32, h: u32, x0: i32, y0: i32, x1: i32, y1: i32, cor: u32) {
    if w == 0 || h == 0 {
        return;
    }
    let x0 = i128::from(x0);
    let y0 = i128::from(y0);
    let x1 = i128::from(x1);
    let y1 = i128::from(y1);
    let dx = (x1 - x0).abs();
    let dy = (y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let largura = i128::from(w);
    let altura = i128::from(h);
    if dx == 0 && dy == 0 {
        plot_i64(idx, w, h, x0 as i64, y0 as i64, cor);
        return;
    }
    if dx >= dy {
        let (de, ate) = if sx > 0 {
            (0i128.max(-x0), dx.min(largura - 1 - x0))
        } else {
            (0i128.max(x0 - (largura - 1)), dx.min(x0))
        };
        if de > ate {
            return;
        }
        for k in de..=ate {
            let y = y0 + sy * ((2 * k * dy + dx) / (2 * dx));
            plot_i64(idx, w, h, (x0 + sx * k) as i64, y as i64, cor);
        }
    } else {
        let (de, ate) = if sy > 0 {
            (0i128.max(-y0), dy.min(altura - 1 - y0))
        } else {
            (0i128.max(y0 - (altura - 1)), dy.min(y0))
        };
        if de > ate {
            return;
        }
        for k in de..=ate {
            let x = x0 + sx * ((2 * k * dx + dy) / (2 * dy));
            plot_i64(idx, w, h, x as i64, (y0 + sy * k) as i64, cor);
        }
    }
}

fn circulo_idx(idx: &mut [u32], w: u32, h: u32, cx: i32, cy: i32, r: i32, cor: u32) {
    if !(0..=LIMITE_COORD).contains(&r) {
        return;
    }
    let cx = i64::from(cx);
    let cy = i64::from(cy);
    if r == 0 {
        plot_i64(idx, w, h, cx, cy, cor);
        return;
    }
    let mut x = i64::from(r);
    let mut y = 0i64;
    let mut err = 1 - x;
    while x >= y {
        plot_i64(idx, w, h, cx + x, cy + y, cor);
        plot_i64(idx, w, h, cx + y, cy + x, cor);
        plot_i64(idx, w, h, cx - y, cy + x, cor);
        plot_i64(idx, w, h, cx - x, cy + y, cor);
        plot_i64(idx, w, h, cx - x, cy - y, cor);
        plot_i64(idx, w, h, cx - y, cy - x, cor);
        plot_i64(idx, w, h, cx + y, cy - x, cor);
        plot_i64(idx, w, h, cx + x, cy - y, cor);
        y += 1;
        if err < 0 {
            err += 2 * y + 1;
        } else {
            x -= 1;
            err += 2 * (y - x) + 1;
        }
    }
}

fn texto_idx(idx: &mut [u32], w: u32, h: u32, x: i32, y: i32, tamanho: i32, texto: &str, cor: u32) {
    let t = tamanho.max(1);
    let mut cx = i64::from(x);
    let mut cy = i64::from(y);
    for ch in texto.chars() {
        if ch == '\r' {
            continue;
        }
        if ch == '\n' {
            cx = i64::from(x);
            cy = cy.saturating_add(i64::from(t));
            continue;
        }
        if ch == '\t' {
            cx = cx.saturating_add(i64::from(t).saturating_mul(2));
            continue;
        }
        let g = fonte8x8::glifo(ch);
        for py in 0..t {
            let fy = ((py * fonte8x8::LADO) / t) as usize;
            let row = g[fy];
            for px in 0..t {
                let fx = ((px * fonte8x8::LADO) / t) as usize;
                if row & (1 << fx) != 0 {
                    plot_i64(idx, w, h, cx + i64::from(px), cy + i64::from(py), cor);
                }
            }
        }
        cx = cx.saturating_add(i64::from(t));
    }
}

#[cfg(all(test, feature = "graphics"))]
#[allow(clippy::identity_op)] // `1 * 4`: pixel 1, 4 bytes each
mod tests {
    use super::*;

    #[test]
    fn rgb_supports_more_than_256_distinct_colors() {
        let t = TelaGrafica {
            id: 1,
            largura: 300,
            altura: 1,
            tamanho_pixel: 1,
            fundo: "branco".into(),
            ops: (0..300)
                .map(|x| OpGraf::Ponto {
                    x,
                    y: 0,
                    cor: format!("#{x:06X}"),
                })
                .collect(),
        };
        let raster = t.raster();
        let img = t.como_imagem();
        for x in 0..300 {
            assert_eq!(raster.pixel(x, 0), Some(format!("#{x:06X}").as_str()));
            assert_eq!(
                img.pixel(x as u32, 0),
                Some([0, (x >> 8) as u8, x as u8, 255])
            );
        }
    }

    #[test]
    fn rgb_primitives_images_png_and_transport_preserve_colors() {
        let t = TelaGrafica {
            id: 1,
            largura: 16,
            altura: 16,
            tamanho_pixel: 1,
            fundo: "#010203".into(),
            ops: vec![
                OpGraf::Retangulo {
                    x1: 1,
                    y1: 1,
                    x2: 3,
                    y2: 3,
                    cor: "#ff8000".into(),
                },
                OpGraf::Linha {
                    x1: 4,
                    y1: 0,
                    x2: 4,
                    y2: 3,
                    cor: "#102030".into(),
                },
                OpGraf::Circulo {
                    x: 6,
                    y: 1,
                    r: 1,
                    cor: "#405060".into(),
                },
                OpGraf::Texto {
                    x: 8,
                    y: 0,
                    tamanho: 8,
                    texto: "A".into(),
                    cor: "#708090".into(),
                },
                OpGraf::Imagem {
                    eixo: [0.0, 0.0],
                    x: 2,
                    y: 2,
                    angulo: 0.0,
                    figura: Imagem::nova(1, 1, vec![0, 0, 255, 255]).unwrap(),
                },
                OpGraf::Ponto {
                    x: 2,
                    y: 2,
                    cor: "#ABCDEF".into(),
                },
                OpGraf::Ponto {
                    x: 0,
                    y: 0,
                    cor: "transparente".into(),
                },
            ],
        };
        let raster = t.raster();
        assert_eq!(raster.pixel(1, 1), Some("#FF8000"));
        assert_eq!(raster.rgb32()[0], 0xFFFFFF);
        assert_eq!(raster.pixel(2, 2), Some("#ABCDEF"));
        assert_eq!(raster.pixel(15, 15), Some("#010203"));
        let img = t.como_imagem();
        assert_eq!(img.pixel(0, 0), Some([0, 0, 0, 0]));
        assert_eq!(img.pixel(1, 1), Some([255, 128, 0, 255]));
        assert_eq!(img.pixel(2, 2), Some([171, 205, 239, 255]));
        assert_eq!(img.pixel(4, 1), Some([16, 32, 48, 255]));
        assert_eq!(img.pixel(6, 0), Some([64, 80, 96, 255]));
        assert!(img.px.chunks_exact(4).any(|p| p == [112, 128, 144, 255]));
        assert_eq!(t.rgb32()[0], 0x010203);
        assert_eq!(cor_tela_rgb32("#ff8000"), 0xFF8000);
        let decoded = imagem::decode_png(&t.png_bytes().unwrap()).unwrap();
        assert_eq!(decoded.px, img.px);

        use crate::runtime::graf_proto::{self, Msg};
        let mut figuras = std::collections::HashMap::new();
        for msg in
            graf_proto::mensagens_de(&t, &mut Default::default(), &mut Default::default(), true)
        {
            let mut bytes = Vec::new();
            graf_proto::write_msg(&mut bytes, &msg).unwrap();
            match graf_proto::read_msg(&mut bytes.as_slice()).unwrap() {
                Msg::CarregueFigura {
                    handle,
                    largura,
                    altura,
                    rgba,
                } => {
                    figuras.insert(handle, Imagem::nova(largura, altura, rgba).unwrap());
                }
                Msg::Mostre { id, ops } => {
                    // Primitive colors survive the same reconstruction used by the viewer.
                    let rebuilt =
                        graf_proto::tela_de_mostre(id, 16, 16, 1, &t.fundo, &ops, &figuras);
                    assert_eq!(rebuilt.raster().rgb32(), raster.rgb32());
                    assert_eq!(rebuilt.como_imagem().px, img.px);
                }
                _ => {}
            }
        }
    }

    fn tela(ops: Vec<OpGraf>) -> TelaGrafica {
        TelaGrafica {
            id: 1,
            largura: 10,
            altura: 10,
            tamanho_pixel: 1,
            fundo: "branco".into(),
            ops,
        }
    }

    #[test]
    fn linha_extrema_recorte_sem_panico() {
        let r = tela(vec![OpGraf::Linha {
            x1: i32::MIN,
            y1: 0,
            x2: i32::MAX,
            y2: 5,
            cor: "azul".into(),
        }])
        .raster();
        let mut azuis = 0;
        for y in 0..10 {
            for x in 0..10 {
                if r.pixel(x, y) == Some("azul") {
                    azuis += 1;
                }
            }
        }
        assert!((1..=10).contains(&azuis), "{azuis}");
    }

    #[test]
    fn linha_de_fora_pinta_a_faixa_visivel() {
        let r = tela(vec![OpGraf::Linha {
            x1: -20,
            y1: 1,
            x2: 30,
            y2: 1,
            cor: "azul".into(),
        }])
        .raster();
        assert_eq!(r.pixel(0, 1), Some("azul"));
        assert_eq!(r.pixel(9, 1), Some("azul"));
        assert_eq!(r.pixel(0, 0), Some("branco"));
    }

    fn tela_wh(largura: u32, altura: u32, linha: (i32, i32, i32, i32)) -> Raster {
        TelaGrafica {
            id: 1,
            largura,
            altura,
            tamanho_pixel: 1,
            fundo: "branco".into(),
            ops: vec![OpGraf::Linha {
                x1: linha.0,
                y1: linha.1,
                x2: linha.2,
                y2: linha.3,
                cor: "azul".into(),
            }],
        }
        .raster()
    }

    fn azuis(r: &Raster) -> Vec<(u32, u32)> {
        let mut out = Vec::new();
        for y in 0..r.altura {
            for x in 0..r.largura {
                if r.pixel(x as i32, y as i32) == Some("azul") {
                    out.push((x, y));
                }
            }
        }
        out
    }

    #[test]
    fn linha_rasa_junto_da_borda_segue_o_bresenham() {
        let r = tela_wh(10, 10, (24, 10, -22, 7));
        assert_eq!(
            azuis(&r),
            vec![
                (0, 8),
                (1, 8),
                (2, 9),
                (3, 9),
                (4, 9),
                (5, 9),
                (6, 9),
                (7, 9),
                (8, 9),
                (9, 9),
            ]
        );

        let r = tela_wh(10, 10, (-30, 0, 10, -1));
        assert!(azuis(&r).is_empty());

        let r = tela_wh(320, 200, (-50, 0, 320, -1));
        let pts = azuis(&r);
        assert_eq!(pts.len(), 135);
        assert_eq!(pts.first(), Some(&(0, 0)));
        assert_eq!(pts.last(), Some(&(134, 0)));

        let r = tela_wh(320, 200, (800, 200, -4096, 190));
        let pts = azuis(&r);
        assert_eq!(pts.len(), 320);
        assert!(pts.iter().take(66).all(|p| p.1 == 198));
        assert!(pts.iter().skip(66).all(|p| p.1 == 199));
        assert_eq!(pts.first(), Some(&(0, 198)));
        assert_eq!(pts.last(), Some(&(319, 199)));
    }

    #[test]
    fn circulo_raio_enorme_nao_trava_e_o_pequeno_pinta() {
        let vazio = tela(vec![OpGraf::Circulo {
            x: 0,
            y: 0,
            r: i32::MAX,
            cor: "azul".into(),
        }])
        .raster();
        assert_eq!(vazio.pixel(0, 0), Some("branco"));
        let r = tela(vec![OpGraf::Circulo {
            x: 5,
            y: 5,
            r: 3,
            cor: "azul".into(),
        }])
        .raster();
        assert_eq!(r.pixel(8, 5), Some("azul"));
        assert_eq!(r.pixel(5, 5), Some("branco"));
    }

    #[test]
    fn diagonal_paints_and_png_roundtrip() {
        let t = TelaGrafica {
            id: 1,
            largura: 16,
            altura: 16,
            tamanho_pixel: 1,
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
            tamanho_pixel: 1,
            fundo: "branco".into(),
            ops: vec![OpGraf::Imagem {
                eixo: [1.0, 1.0],
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
            tamanho_pixel: 1,
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

    #[test]
    fn transparente_vira_alpha_em_como_imagem() {
        let t = TelaGrafica {
            id: 1,
            largura: 4,
            altura: 4,
            tamanho_pixel: 1,
            fundo: "transparente".into(),
            ops: vec![OpGraf::Ponto {
                x: 1,
                y: 2,
                cor: "azul".into(),
            }],
        };
        assert_eq!(t.raster().pixel(0, 0), Some("transparente"));
        assert_eq!(t.raster().pixel(1, 2), Some("azul"));
        let img = t.como_imagem();
        assert_eq!(img.pixel(0, 0), Some([0, 0, 0, 0]));
        assert_eq!(img.pixel(1, 2), Some([0x33, 0x66, 0xCC, 255]));
        let paper = t.rgb32();
        assert_eq!(paper[0], 0x00FFFFFF);
        assert_eq!(paper[(2 * 4 + 1) as usize], 0x003366CC);
        let bytes = t.png_bytes().unwrap();
        let volta = imagem::decode_png(&bytes).unwrap();
        assert_eq!(volta.pixel(0, 0), Some([0, 0, 0, 0]));
        assert_eq!(volta.pixel(1, 2).unwrap()[3], 255);
    }

    #[test]
    fn tamanho_pixel_so_muda_a_janela() {
        let t = TelaGrafica {
            id: 1,
            largura: 160,
            altura: 144,
            tamanho_pixel: 4,
            fundo: "branco".into(),
            ops: vec![],
        };
        assert_eq!(t.janela_largura(), 640);
        assert_eq!(t.janela_altura(), 576);
        assert_eq!(t.raster().largura, 160);
        assert_eq!(t.raster().altura, 144);
    }
    #[test]
    fn images_shapes_text_and_transparent_erasures_follow_command_order() {
        let background = Imagem::nova(16, 16, [255, 0, 0, 255].repeat(16 * 16)).unwrap();
        let blue = Imagem::nova(1, 1, vec![0, 0, 255, 255]).unwrap();
        let tela = TelaGrafica {
            id: 1,
            largura: 16,
            altura: 16,
            tamanho_pixel: 1,
            fundo: "branco".into(),
            ops: vec![
                OpGraf::Imagem {
                    eixo: [8.0, 8.0],
                    x: 8,
                    y: 8,
                    angulo: 0.0,
                    figura: background,
                },
                OpGraf::Retangulo {
                    x1: 0,
                    y1: 0,
                    x2: 15,
                    y2: 7,
                    cor: "preto".into(),
                },
                OpGraf::Texto {
                    x: 0,
                    y: 0,
                    tamanho: 8,
                    texto: "A".into(),
                    cor: "branco".into(),
                },
                OpGraf::Imagem {
                    eixo: [0.0, 0.0],
                    x: 15,
                    y: 15,
                    angulo: 0.0,
                    figura: blue,
                },
                OpGraf::Ponto {
                    x: 15,
                    y: 15,
                    cor: "verde".into(),
                },
                OpGraf::Ponto {
                    x: 12,
                    y: 12,
                    cor: "transparente".into(),
                },
            ],
        };
        let image = tela.como_imagem();
        assert_eq!(image.pixel(0, 0), Some([0, 0, 0, 255]));
        assert_eq!(image.pixel(2, 0), Some([255, 255, 255, 255]));
        assert_eq!(image.pixel(12, 10), Some([255, 0, 0, 255]));
        assert_eq!(image.pixel(12, 12), Some([0, 0, 0, 0]));
        let pixels = tela.rgb32();
        assert_eq!(pixels[12 * 16 + 12], 0x00FFFFFF);
        assert_eq!(pixels[15 * 16 + 15], cor_tela_rgb32("verde"));
        let png = imagem::decode_png(&tela.png_bytes().unwrap()).unwrap();
        assert_eq!(png.px, image.px);
    }
}
