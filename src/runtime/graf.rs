//! Bitmap canvas after drawing commands. Shared by tests, PNG save,
//! `expressa-grafico`, and the Aula window.

use std::io::Cursor;
use std::path::Path;

use super::folha::cor_rgb;

pub const LARGURA_MAX: u32 = 2048;
pub const ALTURA_MAX: u32 = 2048;

#[derive(Clone, Debug, PartialEq)]
pub enum OpGraf {
    Ponto { x: i32, y: i32, cor: String },
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
    Imagem {
        x: i32,
        y: i32,
        angulo: f64,
        caminho: String,
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
    "preto",
    "vermelho",
    "verde",
    "amarelo",
    "azul",
    "magenta",
    "ciano",
    "branco",
    "normal",
];

fn idx_cor(nome: &str) -> u8 {
    PALETA
        .iter()
        .position(|n| *n == nome)
        .unwrap_or(8) as u8
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
                caminho,
            } = op
            else {
                continue;
            };
            let Ok(sprite) = png_rgba(Path::new(caminho)) else {
                continue;
            };
            carimbar(&mut buf, self.largura, self.altura, *x, *y, *angulo, &sprite);
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

pub struct PngRgba {
    pub largura: u32,
    pub altura: u32,
    pub px: Vec<u8>,
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

/// RGBA PNG. Alpha 0 is transparent. Larger than 256×256 is refused.
pub fn png_rgba(path: &Path) -> Result<PngRgba, String> {
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    let mut dec = png::Decoder::new(Cursor::new(bytes));
    dec.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = dec.read_info().map_err(|e| e.to_string())?;
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).map_err(|e| e.to_string())?;
    if info.width > 256 || info.height > 256 || info.width == 0 || info.height == 0 {
        return Err(format!(
            "imagem deve ter entre 1×1 e 256×256 (tem {}×{})",
            info.width, info.height
        ));
    }
    let src = &buf[..info.buffer_size()];
    let mut px = Vec::with_capacity((info.width * info.height * 4) as usize);
    match info.color_type {
        png::ColorType::Rgba => px.extend_from_slice(src),
        png::ColorType::Rgb => {
            for c in src.chunks(3) {
                px.extend_from_slice(&[c[0], c[1], c[2], 255]);
            }
        }
        png::ColorType::Grayscale => {
            for &c in src {
                px.extend_from_slice(&[c, c, c, 255]);
            }
        }
        png::ColorType::GrayscaleAlpha => {
            for c in src.chunks(2) {
                px.extend_from_slice(&[c[0], c[0], c[0], c[1]]);
            }
        }
        other => return Err(format!("PNG não suportado ({other:?})")),
    }
    Ok(PngRgba {
        largura: info.width,
        altura: info.height,
        px,
    })
}

/// Stamp `sprite` centered on `(cx, cy)`. `angulo` is degrees clockwise;
/// 0 keeps the top of the image pointing up.
fn carimbar(
    buf: &mut [u32],
    w: u32,
    h: u32,
    cx: i32,
    cy: i32,
    angulo: f64,
    sprite: &PngRgba,
) {
    let rad = angulo.to_radians();
    let (cos, sin) = (rad.cos(), rad.sin());
    let sw = sprite.largura as i32;
    let sh = sprite.altura as i32;
    let ox = (sprite.largura as f64 - 1.0) / 2.0;
    let oy = (sprite.altura as f64 - 1.0) / 2.0;
    let reach = ((sw * sw + sh * sh) as f64).sqrt() / 2.0 + 1.0;
    let r = reach.ceil() as i32;
    for sy in -r..=r {
        for sx in -r..=r {
            let ix = (sx as f64 * cos + sy as f64 * sin + ox).round() as i32;
            let iy = (-sx as f64 * sin + sy as f64 * cos + oy).round() as i32;
            if ix < 0 || iy < 0 || ix >= sw || iy >= sh {
                continue;
            }
            let dx = cx + sx;
            let dy = cy + sy;
            if dx < 0 || dy < 0 || dx as u32 >= w || dy as u32 >= h {
                continue;
            }
            let i = ((iy as u32 * sprite.largura) + ix as u32) as usize * 4;
            let a = sprite.px[i + 3] as u32;
            if a < 16 {
                continue;
            }
            let dest_i = (dy as u32 * w + dx as u32) as usize;
            let dest = buf[dest_i];
            let sr = sprite.px[i] as u32;
            let sg = sprite.px[i + 1] as u32;
            let sb = sprite.px[i + 2] as u32;
            let dr = (dest >> 16) & 255;
            let dg = (dest >> 8) & 255;
            let db = dest & 255;
            let rch = (sr * a + dr * (255 - a)) / 255;
            let gch = (sg * a + dg * (255 - a)) / 255;
            let bch = (sb * a + db * (255 - a)) / 255;
            buf[dest_i] = (rch << 16) | (gch << 8) | bch;
        }
    }
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

#[cfg(test)]
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
        let path = std::env::temp_dir().join(format!("expressa-sprite-{}.png", std::process::id()));
        // 3×3, only the top-center pixel is red. The rest is transparent.
        let mut raw = vec![0u8; 3 * 3 * 4];
        raw[1 * 4] = 255;
        raw[1 * 4 + 3] = 255;
        let mut bytes = Vec::new();
        {
            let mut enc = png::Encoder::new(&mut bytes, 3, 3);
            enc.set_color(png::ColorType::Rgba);
            enc.set_depth(png::BitDepth::Eight);
            enc.write_header().unwrap().write_image_data(&raw).unwrap();
        }
        std::fs::write(&path, bytes).unwrap();
        let tela = |angulo: f64| TelaGrafica {
            id: 1,
            largura: 16,
            altura: 16,
            fundo: "branco".into(),
            ops: vec![OpGraf::Imagem {
                x: 8,
                y: 8,
                angulo,
                caminho: path.display().to_string(),
            }],
        };
        let cima = tela(0.0).rgb32();
        let direita = tela(90.0).rgb32();
        let at = |buf: &[u32], x: u32, y: u32| buf[(y * 16 + x) as usize];
        assert_eq!(at(&cima, 8, 7), 0x00FF0000, "topo fica acima do centro");
        assert_eq!(at(&cima, 8, 8), 0x00FFFFFF);
        assert_eq!(at(&direita, 9, 8), 0x00FF0000, "90° leva o topo para a direita");
        assert_eq!(at(&direita, 8, 7), 0x00FFFFFF);
        let _ = std::fs::remove_file(&path);
    }
}
