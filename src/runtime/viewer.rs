//! Native window that paints a `.cart` file and reloads when it changes.

use std::path::Path;
use std::thread;
use std::time::{Duration, SystemTime};

use super::folha::{FolhaPronta, pintar};

pub fn run_viewer(path: &Path) -> Result<(), String> {
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
