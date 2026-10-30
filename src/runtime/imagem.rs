//! RGBA image values: load, save, rotate, scale, stamp onto a canvas.

use std::io::Cursor;
use std::path::Path;
use std::sync::Arc;

pub const LARGURA_MAX: u32 = 2048;
pub const ALTURA_MAX: u32 = 2048;

#[derive(Clone, Debug)]
pub struct Imagem {
    pub largura: u32,
    pub altura: u32,
    pub px: Arc<[u8]>,
}

impl PartialEq for Imagem {
    fn eq(&self, other: &Self) -> bool {
        self.largura == other.largura
            && self.altura == other.altura
            && self.px.as_ref() == other.px.as_ref()
    }
}

impl Imagem {
    pub fn nova(largura: u32, altura: u32, px: Vec<u8>) -> Result<Self, String> {
        check_dim(largura, altura)?;
        let n = (largura as usize) * (altura as usize) * 4;
        if px.len() != n {
            return Err("pixels inconsistentes com o tamanho".into());
        }
        Ok(Self {
            largura,
            altura,
            px: Arc::from(px),
        })
    }

    pub fn pixel(&self, x: u32, y: u32) -> Option<[u8; 4]> {
        if x >= self.largura || y >= self.altura {
            return None;
        }
        let i = ((y * self.largura + x) as usize) * 4;
        Some([self.px[i], self.px[i + 1], self.px[i + 2], self.px[i + 3]])
    }
}

fn check_dim(largura: u32, altura: u32) -> Result<(), String> {
    if largura == 0 || altura == 0 || largura > LARGURA_MAX || altura > ALTURA_MAX {
        return Err(format!(
            "imagem deve ter entre 1×1 e {LARGURA_MAX}×{ALTURA_MAX} (tem {largura}×{altura})"
        ));
    }
    Ok(())
}

pub fn leia_png(path: &Path) -> Result<Imagem, String> {
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    decode_png(&bytes)
}

pub fn decode_png(bytes: &[u8]) -> Result<Imagem, String> {
    let mut dec = png::Decoder::new(Cursor::new(bytes));
    dec.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = dec.read_info().map_err(|e| e.to_string())?;
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).map_err(|e| e.to_string())?;
    check_dim(info.width, info.height)?;
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
    Imagem::nova(info.width, info.height, px)
}

pub fn encode_png_rgba(img: &Imagem) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut out, img.largura, img.altura);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        let mut writer = enc.write_header().map_err(|e| e.to_string())?;
        writer
            .write_image_data(img.px.as_ref())
            .map_err(|e| e.to_string())?;
    }
    Ok(out)
}

pub fn salve_png(img: &Imagem, path: &Path) -> Result<(), String> {
    std::fs::write(path, encode_png_rgba(img)?).map_err(|e| e.to_string())
}

/// Clockwise degrees. 0 keeps the top of the image pointing up.
pub fn rotacione(img: &Imagem, graus: f64) -> Result<Imagem, String> {
    if !graus.is_finite() {
        return Err("ângulo da imagem deve ser um número finito".into());
    }
    let mut a = graus % 360.0;
    if a < 0.0 {
        a += 360.0;
    }
    if (a - 0.0).abs() < 1e-9 || (a - 360.0).abs() < 1e-9 {
        return Ok(img.clone());
    }
    if (a - 90.0).abs() < 1e-9 {
        return Ok(rotacione_90(img));
    }
    if (a - 180.0).abs() < 1e-9 {
        return Ok(rotacione_180(img));
    }
    if (a - 270.0).abs() < 1e-9 {
        return Ok(rotacione_270(img));
    }
    rotacione_geral(img, a)
}

fn rotacione_90(src: &Imagem) -> Imagem {
    let w = src.largura;
    let h = src.altura;
    let mut px = vec![0u8; (w * h * 4) as usize];
    for y in 0..h {
        for x in 0..w {
            let dx = h - 1 - y;
            let dy = x;
            copy_px(src, x, y, &mut px, dx, dy, h);
        }
    }
    Imagem {
        largura: h,
        altura: w,
        px: Arc::from(px),
    }
}

fn rotacione_180(src: &Imagem) -> Imagem {
    let w = src.largura;
    let h = src.altura;
    let mut px = vec![0u8; (w * h * 4) as usize];
    for y in 0..h {
        for x in 0..w {
            copy_px(src, x, y, &mut px, w - 1 - x, h - 1 - y, w);
        }
    }
    Imagem {
        largura: w,
        altura: h,
        px: Arc::from(px),
    }
}

fn rotacione_270(src: &Imagem) -> Imagem {
    let w = src.largura;
    let h = src.altura;
    let mut px = vec![0u8; (w * h * 4) as usize];
    for y in 0..h {
        for x in 0..w {
            copy_px(src, x, y, &mut px, y, w - 1 - x, h);
        }
    }
    Imagem {
        largura: h,
        altura: w,
        px: Arc::from(px),
    }
}

fn copy_px(src: &Imagem, sx: u32, sy: u32, dest: &mut [u8], dx: u32, dy: u32, dest_w: u32) {
    let si = ((sy * src.largura + sx) as usize) * 4;
    let di = ((dy * dest_w + dx) as usize) * 4;
    dest[di..di + 4].copy_from_slice(&src.px[si..si + 4]);
}

fn rotacione_geral(src: &Imagem, graus: f64) -> Result<Imagem, String> {
    let rad = graus.to_radians();
    let (cos, sin) = (rad.cos(), rad.sin());
    let w = src.largura as f64;
    let h = src.altura as f64;
    let nw = (w * cos.abs() + h * sin.abs()).ceil().max(1.0) as u32;
    let nh = (w * sin.abs() + h * cos.abs()).ceil().max(1.0) as u32;
    check_dim(nw, nh)?;
    let ox = (src.largura as f64 - 1.0) / 2.0;
    let oy = (src.altura as f64 - 1.0) / 2.0;
    let ndx = (nw as f64 - 1.0) / 2.0;
    let ndy = (nh as f64 - 1.0) / 2.0;
    let sw = src.largura as i32;
    let sh = src.altura as i32;
    let mut px = vec![0u8; (nw * nh * 4) as usize];
    for y in 0..nh {
        for x in 0..nw {
            let sx = x as f64 - ndx;
            let sy = y as f64 - ndy;
            let ix = (sx * cos + sy * sin + ox).round() as i32;
            let iy = (-sx * sin + sy * cos + oy).round() as i32;
            if ix < 0 || iy < 0 || ix >= sw || iy >= sh {
                continue;
            }
            let si = ((iy as u32 * src.largura + ix as u32) as usize) * 4;
            let di = ((y * nw + x) as usize) * 4;
            px[di..di + 4].copy_from_slice(&src.px[si..si + 4]);
        }
    }
    Imagem::nova(nw, nh, px)
}

pub fn escala(img: &Imagem, fx: f64, fy: f64) -> Result<Imagem, String> {
    if !fx.is_finite() || !fy.is_finite() || fx <= 0.0 || fy <= 0.0 {
        return Err("escala deve ser um número maior que 0".into());
    }
    if (fx - 1.0).abs() < 1e-12 && (fy - 1.0).abs() < 1e-12 {
        return Ok(img.clone());
    }
    let nw = (img.largura as f64 * fx).round();
    let nh = (img.altura as f64 * fy).round();
    if !nw.is_finite() || !nh.is_finite() || nw < 1.0 || nh < 1.0 {
        return Err("escala resulta em imagem vazia".into());
    }
    let nw = nw as u32;
    let nh = nh as u32;
    check_dim(nw, nh)?;
    let mut px = vec![0u8; (nw * nh * 4) as usize];
    for y in 0..nh {
        let sy = ((y as f64 + 0.5) * img.altura as f64 / nh as f64).floor() as u32;
        let sy = sy.min(img.altura - 1);
        for x in 0..nw {
            let sx = ((x as f64 + 0.5) * img.largura as f64 / nw as f64).floor() as u32;
            let sx = sx.min(img.largura - 1);
            let si = ((sy * img.largura + sx) as usize) * 4;
            let di = ((y * nw + x) as usize) * 4;
            px[di..di + 4].copy_from_slice(&img.px[si..si + 4]);
        }
    }
    Imagem::nova(nw, nh, px)
}

fn angulo_zero(angulo: f64) -> bool {
    if !angulo.is_finite() {
        return false;
    }
    let a = angulo.rem_euclid(360.0);
    a < 1e-9 || (a - 360.0).abs() < 1e-9
}

/// Dest pixels of a stamp: `(dx, dy, byte index in sprite.px)`.
fn cada_pixel_do_carimbo(
    cx: i32,
    cy: i32,
    angulo: f64,
    sprite: &Imagem,
    mut f: impl FnMut(i32, i32, usize),
) {
    let sw = sprite.largura as i32;
    let sh = sprite.altura as i32;
    if angulo_zero(angulo) {
        // Integer blit: even sizes keep both edges (round of (w-1)/2
        // would drop one column and look like one-sided corners).
        let x0 = cx - sw / 2;
        let y0 = cy - sh / 2;
        for iy in 0..sh {
            for ix in 0..sw {
                let i = ((iy as u32 * sprite.largura) + ix as u32) as usize * 4;
                f(x0 + ix, y0 + iy, i);
            }
        }
        return;
    }
    let rad = angulo.to_radians();
    let (cos, sin) = (rad.cos(), rad.sin());
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
            let i = ((iy as u32 * sprite.largura) + ix as u32) as usize * 4;
            f(cx + sx, cy + sy, i);
        }
    }
}

/// Stamp `sprite` centered on `(cx, cy)`. `angulo` is degrees clockwise;
/// 0 keeps the top of the image pointing up.
pub fn carimbar(buf: &mut [u32], w: u32, h: u32, cx: i32, cy: i32, angulo: f64, sprite: &Imagem) {
    cada_pixel_do_carimbo(cx, cy, angulo, sprite, |dx, dy, i| {
        if dx < 0 || dy < 0 || dx as u32 >= w || dy as u32 >= h {
            return;
        }
        let a = sprite.px[i + 3] as u32;
        if a < 16 {
            return;
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
    });
}

/// Same stamp as [`carimbar`], onto packed RGBA (`4` bytes per pixel).
pub fn carimbar_rgba(
    buf: &mut [u8],
    w: u32,
    h: u32,
    cx: i32,
    cy: i32,
    angulo: f64,
    sprite: &Imagem,
) {
    cada_pixel_do_carimbo(cx, cy, angulo, sprite, |dx, dy, i| {
        if dx < 0 || dy < 0 || dx as u32 >= w || dy as u32 >= h {
            return;
        }
        let a = sprite.px[i + 3] as u32;
        if a < 16 {
            return;
        }
        let di = (dy as u32 * w + dx as u32) as usize * 4;
        let da = buf[di + 3] as u32;
        let out_a = a + da * (255 - a) / 255;
        if out_a == 0 {
            buf[di..di + 4].copy_from_slice(&[0, 0, 0, 0]);
            return;
        }
        for c in 0..3 {
            let s = sprite.px[i + c] as u32 * a;
            let d = buf[di + c] as u32 * da * (255 - a) / 255;
            buf[di + c] = ((s + d) / out_a) as u8;
        }
        buf[di + 3] = out_a as u8;
    });
}

#[cfg(test)]
#[allow(clippy::identity_op)] // `1 * 4`: pixel 1, 4 bytes each
mod tests {
    use super::*;

    fn sprite_topo_vermelho() -> Imagem {
        let mut raw = vec![0u8; 3 * 3 * 4];
        raw[1 * 4] = 255;
        raw[1 * 4 + 3] = 255;
        Imagem::nova(3, 3, raw).unwrap()
    }

    #[test]
    fn noventa_graus_leva_o_topo_para_a_direita() {
        let src = sprite_topo_vermelho();
        assert_eq!(src.pixel(1, 0), Some([255, 0, 0, 255]));
        let r = rotacione(&src, 90.0).unwrap();
        assert_eq!((r.largura, r.altura), (3, 3));
        assert_eq!(r.pixel(2, 1), Some([255, 0, 0, 255]));
        assert_eq!(r.pixel(1, 0), Some([0, 0, 0, 0]));
    }

    #[test]
    fn escala_dobra_o_pixel() {
        let src = sprite_topo_vermelho();
        let d = escala(&src, 2.0, 2.0).unwrap();
        assert_eq!((d.largura, d.altura), (6, 6));
        assert_eq!(d.pixel(2, 0), Some([255, 0, 0, 255]));
        assert_eq!(d.pixel(3, 1), Some([255, 0, 0, 255]));
    }

    #[test]
    fn carimbo_par_mantem_as_duas_bordas() {
        let mut raw = vec![0u8; 4 * 4 * 4];
        for y in 0..4 {
            for x in 0..4 {
                let i = (y * 4 + x) * 4;
                if x == 0 {
                    continue;
                }
                raw[i] = 255;
                raw[i + 3] = 255;
            }
        }
        let sprite = Imagem::nova(4, 4, raw).unwrap();
        let mut buf = vec![0x00FFFFFFu32; 8 * 8];
        carimbar(&mut buf, 8, 8, 2, 2, 0.0, &sprite);
        assert_eq!(buf[0], 0x00FFFFFF, "coluna 0 do sprite é furo");
        assert_eq!(buf[1], 0x00FF0000, "coluna 1 aparece");
        assert_eq!(buf[3], 0x00FF0000, "borda direita não some");
    }
}
