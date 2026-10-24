//! Native window that paints a `.cart` file and reloads when it changes.

use std::path::Path;
use std::thread;
use std::time::{Duration, SystemTime};

use super::folha::{FolhaPronta, pintar};
use super::graf::png_to_rgb32;

pub fn run_viewer(path: &Path) -> Result<(), String> {
    let png = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("png"))
        .unwrap_or(false);
    if png {
        return run_png_viewer(path);
    }
    let mut folha = FolhaPronta::from_path(path)?;
    let mut mtime = file_mtime(path);
    let mut width = 800usize;
    let mut height = 560usize;
    let mut window = minifb::Window::new(
        "Expressa — gráfico",
        width,
        height,
        minifb::WindowOptions {
            resize: true,
            ..minifb::WindowOptions::default()
        },
    )
    .map_err(|e| format!("não abriu a janela: {e}"))?;
    window.set_target_fps(30);
    let mut buffer = pintar(&folha, width as u32, height as u32);
    while window.is_open() {
        let (w, h) = window.get_size();
        let new_mtime = file_mtime(path);
        let resized = w != width || h != height;
        let changed = new_mtime != mtime;
        if changed {
            if let Ok(f) = FolhaPronta::from_path(path) {
                folha = f;
            }
            mtime = new_mtime;
        }
        if changed || resized || buffer.len() != w * h.max(1) {
            width = w.max(1);
            height = h.max(1);
            buffer = pintar(&folha, width as u32, height as u32);
        }
        if let Err(e) = window.update_with_buffer(&buffer, width, height) {
            return Err(e.to_string());
        }
        thread::sleep(Duration::from_millis(40));
    }
    Ok(())
}

fn file_mtime(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path).and_then(|m| m.modified()).ok()
}

fn run_png_viewer(path: &Path) -> Result<(), String> {
    let bytes = std::fs::read(path).map_err(|e| format!("não leu o PNG: {e}"))?;
    let (mut iw, mut ih, mut pixels) = png_to_rgb32(&bytes)?;
    let mut mtime = file_mtime(path);
    let mut width = iw.max(1) as usize;
    let mut height = ih.max(1) as usize;
    let mut window = minifb::Window::new(
        "Expressa — graf",
        width,
        height,
        minifb::WindowOptions {
            resize: true,
            ..minifb::WindowOptions::default()
        },
    )
    .map_err(|e| format!("não abriu a janela: {e}"))?;
    window.set_target_fps(30);
    let mut buffer = scale_rgb32(&pixels, iw, ih, width as u32, height as u32);
    while window.is_open() {
        let (w, h) = window.get_size();
        let new_mtime = file_mtime(path);
        let resized = w != width || h != height;
        let changed = new_mtime != mtime;
        if changed {
            if let Ok(b) = std::fs::read(path) {
                if let Ok((nw, nh, px)) = png_to_rgb32(&b) {
                    iw = nw;
                    ih = nh;
                    pixels = px;
                }
            }
            mtime = new_mtime;
        }
        if changed || resized || buffer.len() != w * h.max(1) {
            width = w.max(1);
            height = h.max(1);
            buffer = scale_rgb32(&pixels, iw, ih, width as u32, height as u32);
        }
        if let Err(e) = window.update_with_buffer(&buffer, width, height) {
            return Err(e.to_string());
        }
        thread::sleep(Duration::from_millis(40));
    }
    Ok(())
}

fn scale_rgb32(src: &[u32], sw: u32, sh: u32, dw: u32, dh: u32) -> Vec<u32> {
    let dw = dw.max(1);
    let dh = dh.max(1);
    let sw = sw.max(1);
    let sh = sh.max(1);
    let mut out = vec![0u32; (dw * dh) as usize];
    for y in 0..dh {
        let sy = y * sh / dh;
        for x in 0..dw {
            let sx = x * sw / dw;
            out[(y * dw + x) as usize] = src[(sy * sw + sx) as usize];
        }
    }
    out
}
