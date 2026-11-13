//! Sibling windows for `tela_grafica::mostre`. Not the Gráficos dock.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::sync::mpsc::Sender;

use expressa::runtime::{EventoGraf, TelaGrafica, cor_tela_rgb32, letterbox};
use gtk::cairo::{Filter, Format, ImageSurface, SurfacePattern};
use gtk::gdk::Key;
use gtk::glib;
use gtk::prelude::*;
use gtk::{DrawingArea, EventControllerKey, GestureClick, Window};

struct Janela {
    win: Window,
    area: DrawingArea,
    frame: Rc<RefCell<Option<ImageSurface>>>,
}

pub struct Painel {
    telas: Rc<RefCell<HashMap<u64, TelaGrafica>>>,
    windows: Rc<RefCell<HashMap<u64, Janela>>>,
    eventos: Rc<RefCell<Option<Sender<EventoGraf>>>>,
    /// Ids whose window the student closed. `mostre` must not recreate
    /// them until the next run (`limpar`).
    fechadas: Rc<RefCell<HashSet<u64>>>,
}

impl Painel {
    pub fn new() -> Rc<Self> {
        Rc::new(Self {
            telas: Rc::new(RefCell::new(HashMap::new())),
            windows: Rc::new(RefCell::new(HashMap::new())),
            eventos: Rc::new(RefCell::new(None)),
            fechadas: Rc::new(RefCell::new(HashSet::new())),
        })
    }

    pub fn set_eventos(&self, tx: Option<Sender<EventoGraf>>) {
        *self.eventos.borrow_mut() = tx;
    }

    fn emita(&self, e: EventoGraf) {
        if let Some(tx) = self.eventos.borrow().as_ref() {
            let _ = tx.send(e);
        }
    }

    pub fn mostrar(self: &Rc<Self>, tela: TelaGrafica) {
        let id = tela.id;
        if self.fechadas.borrow().contains(&id) {
            return;
        }
        let nova = !self.windows.borrow().contains_key(&id);
        self.telas.borrow_mut().insert(id, tela);
        if nova {
            let win = Window::builder()
                .title("Expressa — tela gráfica")
                .default_width(320)
                .default_height(240)
                .build();
            let area = DrawingArea::new();
            area.set_hexpand(true);
            area.set_vexpand(true);
            area.set_can_focus(true);
            let frame = Rc::new(RefCell::new(None::<ImageSurface>));
            {
                let frame = Rc::clone(&frame);
                let telas = Rc::clone(&self.telas);
                area.set_draw_func(move |_, cr, w, h| {
                    if w < 1 || h < 1 {
                        return;
                    }
                    let paper = telas
                        .borrow()
                        .get(&id)
                        .map(|t| cor_tela_rgb32(&t.fundo))
                        .unwrap_or(0x00FFFFFF);
                    cr.set_source_rgb(
                        ((paper >> 16) & 0xff) as f64 / 255.0,
                        ((paper >> 8) & 0xff) as f64 / 255.0,
                        (paper & 0xff) as f64 / 255.0,
                    );
                    let _ = cr.paint();
                    let frame = frame.borrow();
                    let Some(surf) = frame.as_ref() else { return };
                    let tw = surf.width().max(1) as f64;
                    let th = surf.height().max(1) as f64;
                    let lb = letterbox(w as f64, h as f64, tw, th);
                    cr.save().ok();
                    cr.rectangle(lb.x, lb.y, lb.width, lb.height);
                    cr.clip();
                    cr.translate(lb.x, lb.y);
                    cr.scale(lb.width / tw, lb.height / th);
                    let pattern = SurfacePattern::create(surf);
                    pattern.set_filter(Filter::Nearest);
                    cr.set_source(pattern).ok();
                    let _ = cr.paint();
                    cr.restore().ok();
                });
            }

            let click = GestureClick::new();
            click.set_button(0);
            {
                let painel = Rc::clone(self);
                let area_c = area.clone();
                click.connect_pressed(move |g, _, x, y| {
                    let (tw, th) = {
                        let telas = painel.telas.borrow();
                        match telas.get(&id) {
                            Some(t) => (t.largura.max(1) as f64, t.altura.max(1) as f64),
                            None => return,
                        }
                    };
                    let w = area_c.width() as f64;
                    let h = area_c.height() as f64;
                    if w < 1.0 || h < 1.0 {
                        return;
                    }
                    let lb = letterbox(w, h, tw, th);
                    let Some((px, py)) = lb.canvas_pixel(x, y, tw, th) else {
                        return;
                    };
                    let botao = match g.current_button() {
                        3 => "direito",
                        2 => "meio",
                        _ => "esquerdo",
                    };
                    painel.emita(EventoGraf::clique(px, py, botao));
                });
            }
            area.add_controller(click);

            let keys = EventControllerKey::new();
            {
                let painel = Rc::clone(self);
                keys.connect_key_pressed(move |_, key, _, _| {
                    painel.emita(EventoGraf::tecla(tecla_gdk(key)));
                    glib::Propagation::Proceed
                });
            }
            win.add_controller(keys);
            win.set_can_focus(true);

            win.set_child(Some(&area));
            win.set_default_width(
                self.telas
                    .borrow()
                    .get(&id)
                    .map(|t| t.janela_largura() as i32)
                    .unwrap_or(320),
            );
            win.set_default_height(
                self.telas
                    .borrow()
                    .get(&id)
                    .map(|t| t.janela_altura() as i32)
                    .unwrap_or(200),
            );
            {
                let painel = Rc::clone(self);
                win.connect_close_request(move |_| {
                    painel.emita(EventoGraf::sair());
                    painel.fechadas.borrow_mut().insert(id);
                    painel.telas.borrow_mut().remove(&id);
                    painel.windows.borrow_mut().remove(&id);
                    glib::Propagation::Proceed
                });
            }
            win.present();
            self.windows.borrow_mut().insert(
                id,
                Janela {
                    win,
                    area: area.clone(),
                    frame,
                },
            );
        }
        // Convert pixels once per `mostre`, not on every expose/resize.
        if let Some(janela) = self.windows.borrow().get(&id) {
            *janela.frame.borrow_mut() = self.telas.borrow().get(&id).and_then(surface_from_tela);
            janela.area.queue_draw();
        }
    }

    pub fn limpar(&self) {
        self.fechar_todas();
    }

    pub fn fechar_todas(&self) {
        *self.eventos.borrow_mut() = None;
        let windows: Vec<Window> = self
            .windows
            .borrow_mut()
            .drain()
            .map(|(_, j)| j.win)
            .collect();
        self.telas.borrow_mut().clear();
        for w in windows {
            w.close();
        }
        // `close()` runs `close-request` and would fill `fechadas`; a new
        // run (F5) must be allowed to open windows again.
        self.fechadas.borrow_mut().clear();
    }
}

/// `rgb32` is 0x00RRGGBB. On little-endian that is already Cairo RGB24
/// (B, G, R, unused) so the frame can be copied as bytes.
fn fill_rgb24(pixels: &[u32], width: usize, height: usize, dest: &mut [u8], stride: usize) {
    if width == 0 || height == 0 {
        return;
    }
    if cfg!(target_endian = "little") {
        // SAFETY: `u32` is four bytes; little-endian 0x00RRGGBB is B,G,R,pad,
        // which is Cairo RGB24. The slice length is `pixels.len() * 4`.
        let src = unsafe {
            std::slice::from_raw_parts(pixels.as_ptr().cast::<u8>(), pixels.len().saturating_mul(4))
        };
        if stride == width.saturating_mul(4) {
            let n = src.len().min(dest.len());
            dest[..n].copy_from_slice(&src[..n]);
            return;
        }
        for y in 0..height {
            let src_off = y.saturating_mul(width).saturating_mul(4);
            let dst_off = y.saturating_mul(stride);
            let n = (width * 4)
                .min(dest.len().saturating_sub(dst_off))
                .min(src.len().saturating_sub(src_off));
            dest[dst_off..dst_off + n].copy_from_slice(&src[src_off..src_off + n]);
        }
        return;
    }
    for y in 0..height {
        for x in 0..width {
            let Some(&px) = pixels.get(y * width + x) else {
                continue;
            };
            let i = y * stride + x * 4;
            if i + 2 < dest.len() {
                dest[i] = (px & 0xff) as u8;
                dest[i + 1] = ((px >> 8) & 0xff) as u8;
                dest[i + 2] = ((px >> 16) & 0xff) as u8;
            }
        }
    }
}

fn surface_from_tela(tela: &TelaGrafica) -> Option<ImageSurface> {
    let pixels = tela.rgb32();
    let tw = tela.largura.max(1) as usize;
    let th = tela.altura.max(1) as usize;
    let mut surf = ImageSurface::create(Format::Rgb24, tw as i32, th as i32).ok()?;
    let stride = surf.stride() as usize;
    if let Ok(mut data) = surf.data() {
        fill_rgb24(&pixels, tw, th, &mut data, stride);
    }
    surf.mark_dirty();
    Some(surf)
}

fn tecla_gdk(key: Key) -> String {
    if key == Key::Return || key == Key::KP_Enter {
        return "Enter".into();
    }
    if key == Key::Escape {
        return "Esc".into();
    }
    if key == Key::space {
        return " ".into();
    }
    if key == Key::Tab {
        return "Tab".into();
    }
    if key == Key::BackSpace {
        return "Backspace".into();
    }
    if key == Key::Up || key == Key::uparrow {
        return "Cima".into();
    }
    if key == Key::Down || key == Key::downarrow {
        return "Baixo".into();
    }
    if key == Key::Left || key == Key::leftarrow {
        return "Esquerda".into();
    }
    if key == Key::Right || key == Key::rightarrow {
        return "Direita".into();
    }
    match key.to_unicode() {
        Some(c) if !c.is_control() => c.to_lowercase().to_string(),
        _ => key
            .name()
            .map(|n| n.as_str().to_string())
            .unwrap_or_default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rgb24_copies_little_endian_pixels_as_bytes() {
        let pixels = [0x00AABBCC, 0x00112233];
        let mut dest = vec![0u8; 8];
        fill_rgb24(&pixels, 2, 1, &mut dest, 8);
        assert_eq!(dest, [0xCC, 0xBB, 0xAA, 0x00, 0x33, 0x22, 0x11, 0x00]);
    }

    #[test]
    fn rgb24_respects_cairo_stride() {
        let pixels = [0x00000001, 0x00000002];
        let mut dest = vec![0xFFu8; 16];
        fill_rgb24(&pixels, 2, 1, &mut dest, 16);
        assert_eq!(&dest[..8], &[1, 0, 0, 0, 2, 0, 0, 0]);
        assert_eq!(&dest[8..], &[0xFF; 8]);
    }
}
