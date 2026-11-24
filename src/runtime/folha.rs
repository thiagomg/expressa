//! Geometry of a cartesian document after sampling. Shared by tests, the
//! `expressa-grafico` window, and the Aula panel.

use std::fmt::Write as _;
use std::path::Path;

/// One sampled curve. `y = None` breaks the polyline (NaN / infinite).
#[derive(Clone, Debug, PartialEq)]
pub struct CurvaPronta {
    pub cor: String,
    pub pontos: Vec<(f64, Option<f64>)>,
}

/// One xy plane, already sampled.
#[derive(Clone, Debug, PartialEq)]
pub struct GraficoPronto {
    pub xmin: f64,
    pub xmax: f64,
    pub ymin: f64,
    pub ymax: f64,
    pub curvas: Vec<CurvaPronta>,
}

/// Window contents: one or more graphs stacked top to bottom.
#[derive(Clone, Debug, PartialEq)]
pub struct FolhaPronta {
    /// Identity of the Expressa `documento()` map (pointer).
    pub id: u64,
    pub graficos: Vec<GraficoPronto>,
}

const CART_VER: &str = "EXPRESSA-CART 1";

impl FolhaPronta {
    pub fn to_cart(&self) -> String {
        let mut s = String::new();
        let _ = writeln!(s, "{CART_VER}");
        let _ = writeln!(s, "id {}", self.id);
        for g in &self.graficos {
            let _ = writeln!(
                s,
                "grafico {} {} {} {}",
                fmt_f(g.xmin),
                fmt_f(g.xmax),
                fmt_f(g.ymin),
                fmt_f(g.ymax)
            );
            for c in &g.curvas {
                let _ = writeln!(s, "curva {}", c.cor);
                for (x, y) in &c.pontos {
                    match y {
                        Some(y) if y.is_finite() && x.is_finite() => {
                            let _ = writeln!(s, "p {} {}", fmt_f(*x), fmt_f(*y));
                        }
                        _ => {
                            let _ = writeln!(s, "h");
                        }
                    }
                }
            }
        }
        s
    }

    pub fn from_cart(text: &str) -> Result<Self, String> {
        let mut lines = text.lines();
        let header = lines.next().unwrap_or("").trim();
        if header != CART_VER {
            return Err(format!("arquivo de gráfico desconhecido ({header})"));
        }
        let id_line = lines.next().unwrap_or("").trim();
        let id = id_line
            .strip_prefix("id ")
            .and_then(|t| t.trim().parse().ok())
            .ok_or_else(|| format!("id inválido ({id_line})"))?;
        let mut folha = FolhaPronta {
            id,
            graficos: Vec::new(),
        };
        for line in lines {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            if let Some(rest) = line.strip_prefix("grafico ") {
                let nums: Vec<f64> = rest
                    .split_whitespace()
                    .filter_map(|t| t.parse().ok())
                    .collect();
                if nums.len() != 4 {
                    return Err(format!("eixos inválidos ({line})"));
                }
                folha.graficos.push(GraficoPronto {
                    xmin: nums[0],
                    xmax: nums[1],
                    ymin: nums[2],
                    ymax: nums[3],
                    curvas: Vec::new(),
                });
            } else if let Some(cor) = line.strip_prefix("curva ") {
                let g = folha.graficos.last_mut().ok_or("curva sem gráfico")?;
                g.curvas.push(CurvaPronta {
                    cor: cor.trim().to_string(),
                    pontos: Vec::new(),
                });
            } else if line == "h" {
                let c = curva_atual(&mut folha)?;
                c.pontos.push((0.0, None));
            } else if let Some(rest) = line.strip_prefix("p ") {
                let mut it = rest.split_whitespace();
                let x: f64 = it
                    .next()
                    .and_then(|t| t.parse().ok())
                    .ok_or_else(|| format!("ponto inválido ({line})"))?;
                let y: f64 = it
                    .next()
                    .and_then(|t| t.parse().ok())
                    .ok_or_else(|| format!("ponto inválido ({line})"))?;
                let c = curva_atual(&mut folha)?;
                c.pontos.push((x, Some(y)));
            } else {
                return Err(format!("linha desconhecida ({line})"));
            }
        }
        Ok(folha)
    }

    pub fn from_path(path: &Path) -> Result<Self, String> {
        let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        Self::from_cart(&text)
    }
}

fn curva_atual(folha: &mut FolhaPronta) -> Result<&mut CurvaPronta, String> {
    folha
        .graficos
        .last_mut()
        .and_then(|g| g.curvas.last_mut())
        .ok_or_else(|| "ponto sem curva".into())
}

fn fmt_f(n: f64) -> String {
    let s = format!("{n:.12}");
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

/// 0x00RRGGBB pixels, row-major, for minifb and the Aula panel.
pub fn pintar(folha: &FolhaPronta, width: u32, height: u32) -> Vec<u32> {
    let w = width.max(1) as usize;
    let h = height.max(1) as usize;
    let mut buf = vec![COR_FUNDO; w * h];
    if folha.graficos.is_empty() || w < 40 || h < 40 {
        return buf;
    }
    let n = folha.graficos.len();
    let gap = 8usize;
    let usable_h = h.saturating_sub(gap * n.saturating_sub(1).min(h));
    let gh = (usable_h / n).max(32);
    for (i, g) in folha.graficos.iter().enumerate() {
        let top = i * (gh + gap);
        if top >= h {
            break;
        }
        let bottom = (top + gh).min(h);
        pintar_grafico(&mut buf, w, h, top, bottom, g);
    }
    buf
}

const COR_FUNDO: u32 = 0x00FAFAFA;
const COR_EIXO: u32 = 0x00333333;
const COR_GRADE: u32 = 0x00E4E4E4;
const COR_TEXTO: u32 = 0x00444444;

fn pintar_grafico(
    buf: &mut [u32],
    w: usize,
    h: usize,
    top: usize,
    bottom: usize,
    g: &GraficoPronto,
) {
    let left = 48usize;
    let right = w.saturating_sub(8);
    let inner_top = top + 8;
    let inner_bot = bottom.saturating_sub(20);
    if right <= left + 4 || inner_bot <= inner_top + 4 {
        return;
    }
    let plot_w = (right - left) as f64;
    let plot_h = (inner_bot - inner_top) as f64;
    let dx = (g.xmax - g.xmin).abs().max(1e-12);
    let dy = (g.ymax - g.ymin).abs().max(1e-12);

    let to_s = |x: f64, y: f64| -> (i32, i32) {
        let sx = left as f64 + (x - g.xmin) / dx * plot_w;
        let sy = inner_top as f64 + (g.ymax - y) / dy * plot_h;
        (sx.round() as i32, sy.round() as i32)
    };

    let xticks = nice_ticks(g.xmin, g.xmax, 6);
    let yticks = nice_ticks(g.ymin, g.ymax, 5);
    for x in &xticks {
        let (sx, y0) = to_s(*x, g.ymin);
        let (_, y1) = to_s(*x, g.ymax);
        linha(buf, w, h, sx, y0, sx, y1, COR_GRADE);
    }
    for y in &yticks {
        let (x0, sy) = to_s(g.xmin, *y);
        let (x1, _) = to_s(g.xmax, *y);
        linha(buf, w, h, x0, sy, x1, sy, COR_GRADE);
    }

    let (x0, y0) = to_s(g.xmin, 0.0);
    let (x1, y1) = to_s(g.xmax, 0.0);
    if g.ymin <= 0.0 && g.ymax >= 0.0 {
        linha(buf, w, h, x0, y0, x1, y1, COR_EIXO);
    }
    let (zx0, zy0) = to_s(0.0, g.ymin);
    let (zx1, zy1) = to_s(0.0, g.ymax);
    if g.xmin <= 0.0 && g.xmax >= 0.0 {
        linha(buf, w, h, zx0, zy0, zx1, zy1, COR_EIXO);
    }

    rect(
        buf,
        w,
        h,
        left as i32,
        inner_top as i32,
        right as i32,
        inner_bot as i32,
        COR_EIXO,
    );

    for x in &xticks {
        let (sx, _) = to_s(*x, g.ymin);
        linha(
            buf,
            w,
            h,
            sx,
            inner_bot as i32,
            sx,
            inner_bot as i32 + 4,
            COR_EIXO,
        );
        let label = tick_label(*x);
        texto(
            buf,
            w,
            h,
            sx - (label.len() as i32 * 3),
            inner_bot as i32 + 6,
            &label,
            COR_TEXTO,
        );
    }
    for y in &yticks {
        let (_, sy) = to_s(g.xmin, *y);
        linha(buf, w, h, left as i32 - 4, sy, left as i32, sy, COR_EIXO);
        let label = tick_label(*y);
        texto(
            buf,
            w,
            h,
            left as i32 - 6 - (label.len() as i32 * 6),
            sy - 3,
            &label,
            COR_TEXTO,
        );
    }

    for curva in &g.curvas {
        let rgb = cor_rgb(&curva.cor);
        let mut prev: Option<(i32, i32)> = None;
        for (x, y) in &curva.pontos {
            let Some(y) = y else {
                prev = None;
                continue;
            };
            if !x.is_finite() || !y.is_finite() {
                prev = None;
                continue;
            }
            let p = to_s(*x, *y);
            if let Some(q) = prev {
                linha_clip(
                    buf,
                    w,
                    h,
                    q.0,
                    q.1,
                    p.0,
                    p.1,
                    rgb,
                    left as i32,
                    inner_top as i32,
                    right as i32,
                    inner_bot as i32,
                );
            }
            prev = Some(p);
        }
    }
}

fn nice_ticks(min: f64, max: f64, max_n: usize) -> Vec<f64> {
    let span = (max - min).abs().max(1e-12);
    let raw = span / max_n.max(2) as f64;
    let mag = 10f64.powf(raw.log10().floor());
    let mut step = mag;
    let r = raw / mag;
    if r > 5.0 {
        step = 10.0 * mag;
    } else if r > 2.0 {
        step = 5.0 * mag;
    } else if r > 1.0 {
        step = 2.0 * mag;
    }
    let start = (min / step).ceil() * step;
    let mut out = Vec::new();
    let mut x = start;
    let mut guard = 0;
    while x <= max + step * 0.01 && guard < 40 {
        if x >= min - step * 0.01 {
            out.push(if x.abs() < step * 1e-9 { 0.0 } else { x });
        }
        x += step;
        guard += 1;
    }
    if out.is_empty() {
        out.push(min);
        out.push(max);
    }
    out
}

fn tick_label(n: f64) -> String {
    if n == 0.0 {
        return "0".into();
    }
    let a = n.abs();
    if a >= 1000.0 || a < 0.01 {
        format!("{n:.2e}")
    } else if (n - n.round()).abs() < 1e-9 {
        format!("{:.0}", n.round())
    } else {
        let s = format!("{n:.2}");
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    }
}

pub fn cor_rgb(nome: &str) -> u32 {
    match nome {
        "preto" => 0x00222222,
        "vermelho" => 0x00CC3333,
        "verde" => 0x0022AA44,
        "amarelo" => 0x00C9A227,
        "azul" => 0x003366CC,
        "magenta" => 0x00AA44AA,
        "ciano" => 0x0022AAAA,
        "branco" => 0x00888888,
        "normal" => 0x00333333,
        _ => 0x00333333,
    }
}

fn linha(buf: &mut [u32], w: usize, h: usize, x0: i32, y0: i32, x1: i32, y1: i32, cor: u32) {
    let mut x = x0;
    let mut y = y0;
    let dx = (x1 - x0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let dy = -(y1 - y0).abs();
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;
    loop {
        plot(buf, w, h, x, y, cor);
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

fn linha_clip(
    buf: &mut [u32],
    w: usize,
    h: usize,
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
    cor: u32,
    l: i32,
    t: i32,
    r: i32,
    b: i32,
) {
    let mut x = x0;
    let mut y = y0;
    let dx = (x1 - x0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let dy = -(y1 - y0).abs();
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;
    loop {
        if x >= l && x < r && y >= t && y < b {
            plot(buf, w, h, x, y, cor);
        }
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

fn rect(buf: &mut [u32], w: usize, h: usize, l: i32, t: i32, r: i32, b: i32, cor: u32) {
    linha(buf, w, h, l, t, r, t, cor);
    linha(buf, w, h, l, b, r, b, cor);
    linha(buf, w, h, l, t, l, b, cor);
    linha(buf, w, h, r, t, r, b, cor);
}

fn plot(buf: &mut [u32], w: usize, h: usize, x: i32, y: i32, cor: u32) {
    if x < 0 || y < 0 {
        return;
    }
    let x = x as usize;
    let y = y as usize;
    if x < w && y < h {
        buf[y * w + x] = cor;
    }
}

fn texto(buf: &mut [u32], w: usize, h: usize, x: i32, y: i32, s: &str, cor: u32) {
    let mut cx = x;
    for ch in s.chars() {
        glyph(buf, w, h, cx, y, ch, cor);
        cx += 6;
    }
}

/// 5×7 glyphs for tick labels.
fn glyph(buf: &mut [u32], w: usize, h: usize, x: i32, y: i32, ch: char, cor: u32) {
    let bits: u64 = match ch {
        '0' => 0b01110_10001_10011_10101_11001_10001_01110,
        '1' => 0b00100_01100_00100_00100_00100_00100_01110,
        '2' => 0b01110_10001_00001_00010_00100_01000_11111,
        '3' => 0b01110_10001_00001_00110_00001_10001_01110,
        '4' => 0b00010_00110_01010_10010_11111_00010_00010,
        '5' => 0b11111_10000_11110_00001_00001_10001_01110,
        '6' => 0b01110_10000_10000_11110_10001_10001_01110,
        '7' => 0b11111_00001_00010_00100_01000_01000_01000,
        '8' => 0b01110_10001_10001_01110_10001_10001_01110,
        '9' => 0b01110_10001_10001_01111_00001_00001_01110,
        '-' => 0b00000_00000_00000_11111_00000_00000_00000,
        '.' => 0b00000_00000_00000_00000_00000_00100_00100,
        'e' => 0b00000_00000_01110_10001_11111_10000_01110,
        '+' => 0b00000_00100_00100_11111_00100_00100_00000,
        _ => 0,
    };
    for row in 0..7 {
        for col in 0..5 {
            let on = (bits >> (34 - (row * 5 + col))) & 1;
            if on == 1 {
                plot(buf, w, h, x + col, y + row, cor);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cart_roundtrip() {
        let f = FolhaPronta {
            id: 42,
            graficos: vec![GraficoPronto {
                xmin: -1.0,
                xmax: 1.0,
                ymin: -2.0,
                ymax: 2.0,
                curvas: vec![CurvaPronta {
                    cor: "azul".into(),
                    pontos: vec![(-1.0, Some(0.0)), (0.0, None), (1.0, Some(1.0))],
                }],
            }],
        };
        let t = f.to_cart();
        let g = FolhaPronta::from_cart(&t).unwrap();
        assert_eq!(g.id, 42);
        assert_eq!(g.graficos.len(), 1);
        assert_eq!(g.graficos[0].curvas[0].cor, "azul");
        assert_eq!(g.graficos[0].curvas[0].pontos.len(), 3);
        assert!(g.graficos[0].curvas[0].pontos[1].1.is_none());
    }

    #[test]
    fn pintar_fills_buffer() {
        let f = FolhaPronta {
            id: 1,
            graficos: vec![GraficoPronto {
                xmin: -1.0,
                xmax: 1.0,
                ymin: -1.0,
                ymax: 1.0,
                curvas: vec![CurvaPronta {
                    cor: "azul".into(),
                    pontos: vec![(-1.0, Some(-1.0)), (1.0, Some(1.0))],
                }],
            }],
        };
        let buf = pintar(&f, 200, 150);
        assert_eq!(buf.len(), 200 * 150);
        assert!(buf.iter().any(|&p| p == cor_rgb("azul")));
        assert!(buf.contains(&COR_EIXO));
    }
}
