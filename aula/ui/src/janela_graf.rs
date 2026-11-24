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
                    painel.emita(EventoGraf {
                        janela: id,
                        ..EventoGraf::clique(px, py, botao)
                    });
                });
            }
            {
                let painel = Rc::clone(self);
                let area_c = area.clone();
                click.connect_released(move |g, _, x, y| {
                    let telas = painel.telas.borrow();
                    let Some(t) = telas.get(&id) else {
                        return;
                    };
                    let lb = letterbox(
                        area_c.width() as f64,
                        area_c.height() as f64,
                        t.largura as f64,
                        t.altura as f64,
                    );
                    let Some((px, py)) = lb.canvas_pixel(x, y, t.largura as f64, t.altura as f64)
                    else {
                        return;
                    };
                    let botao = match g.current_button() {
                        3 => "direito",
                        2 => "meio",
                        _ => "esquerdo",
                    };
                    painel.emita(EventoGraf {
                        janela: id,
                        tipo: "solte".into(),
                        ..EventoGraf::clique(px, py, botao)
                    });
                });
            }
            area.add_controller(click);

            let keys = EventControllerKey::new();
            {
                let painel = Rc::clone(self);
                keys.connect_key_pressed(move |controller, key, code, _| {
                    let base = controller
                        .widget()
                        .and_then(|w| {
                            w.display()
                                .translate_key(code, gtk::gdk::ModifierType::empty(), 0)
                        })
                        .map(|(k, _, _, _)| k)
                        .unwrap_or(key);
                    let nome = tecla_gdk(base);
                    if !nome.is_empty() {
                        painel.emita(EventoGraf {
                            janela: id,
                            fisica: code.to_string(),
                            ..EventoGraf::tecla(nome)
                        });
                    }
                    glib::Propagation::Proceed
                });
            }
            {
                let painel = Rc::clone(self);
                keys.connect_key_released(move |_, _, code, _| {
                    painel.emita(EventoGraf {
                        janela: id,
                        fisica: code.to_string(),
                        ..EventoGraf::soltura("")
                    });
                });
            }
            {
                let painel = Rc::clone(self);
                win.connect_is_active_notify(move |w| {
                    if !w.is_active()
                        && painel
                            .windows
                            .borrow()
                            .get(&id)
                            .is_some_and(|j| j.win == *w)
                    {
                        painel.emita(EventoGraf {
                            janela: id,
                            tipo: "desfoco".into(),
                            ..EventoGraf::nada()
                        });
                    }
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
                win.connect_close_request(move |w| {
                    // Programmatic closes remove the window first. They must not
                    // enqueue an old :sair event if mostre() later opens it again.
                    if !painel
                        .windows
                        .borrow()
                        .get(&id)
                        .is_some_and(|j| j.win == *w)
                    {
                        return glib::Propagation::Proceed;
                    }
                    painel.emita(EventoGraf {
                        janela: id,
                        ..EventoGraf::sair()
                    });
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

    pub fn fechar(&self, id: u64) {
        let janela = self.windows.borrow_mut().remove(&id);
        self.telas.borrow_mut().remove(&id);
        self.fechadas.borrow_mut().remove(&id);
        if let Some(janela) = janela {
            janela.win.close();
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
    let name = key.name().map(|s| s.to_string()).unwrap_or_default();
    let canonical = match name.as_str() {
        "Return" | "KP_Enter" => "enter",
        "Escape" => "escape",
        "space" => "espaco",
        "Tab" | "ISO_Left_Tab" => "tab",
        "BackSpace" => "backspace",
        "Up" | "uparrow" => "cima",
        "Down" | "downarrow" => "baixo",
        "Left" | "leftarrow" => "esquerda",
        "Right" | "rightarrow" => "direita",
        "Shift_L" | "Shift_R" => "shift",
        "Control_L" | "Control_R" => "control",
        "Alt_L" | "Alt_R" | "ISO_Level3_Shift" => "alt",
        "Super_L" | "Super_R" | "Meta_L" | "Meta_R" => "super",
        "Home" => "inicio",
        "End" => "fim",
        "Page_Up" => "pagina_acima",
        "Page_Down" => "pagina_abaixo",
        "Insert" => "insert",
        "Delete" => "delete",
        "Menu" => "menu",
        "Pause" => "pause",
        "Caps_Lock" => "caps_lock",
        "Num_Lock" => "num_lock",
        "Scroll_Lock" => "scroll_lock",
        "apostrophe" => "apostrofo",
        "grave" => "crase",
        "backslash" => "barra_invertida",
        "comma" => "virgula",
        "equal" => "igual",
        "bracketleft" => "colchete_esquerdo",
        "bracketright" => "colchete_direito",
        "minus" => "menos",
        "period" => "ponto",
        "semicolon" => "ponto_e_virgula",
        "slash" => "barra",
        "KP_0" | "KP_Insert" => "numerico_0",
        "KP_1" | "KP_End" => "numerico_1",
        "KP_2" | "KP_Down" => "numerico_2",
        "KP_3" | "KP_Page_Down" => "numerico_3",
        "KP_4" | "KP_Left" => "numerico_4",
        "KP_5" | "KP_Begin" => "numerico_5",
        "KP_6" | "KP_Right" => "numerico_6",
        "KP_7" | "KP_Home" => "numerico_7",
        "KP_8" | "KP_Up" => "numerico_8",
        "KP_9" | "KP_Page_Up" => "numerico_9",
        "KP_Decimal" | "KP_Delete" | "KP_Separator" => "numerico_ponto",
        "KP_Divide" => "numerico_barra",
        "KP_Multiply" => "numerico_asterisco",
        "KP_Subtract" => "numerico_menos",
        "KP_Add" => "numerico_mais",
        _ => "",
    };
    if !canonical.is_empty() {
        return canonical.into();
    }
    if let Some(c) = key.to_unicode() {
        if c.is_ascii_digit() {
            return format!("num_{c}");
        }
        if c.is_alphabetic() {
            return c.to_lowercase().to_string();
        }
    }
    if name
        .strip_prefix('F')
        .is_some_and(|n| n.parse::<u8>().is_ok_and(|n| (1..=15).contains(&n)))
    {
        return name.to_lowercase();
    }
    String::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_close_targets_one_window_and_permits_reopening() {
        crate::gtk_test::run(|| {
            let painel = Painel::new();
            let (tx, rx) = std::sync::mpsc::channel();
            painel.set_eventos(Some(tx));
            let tela = TelaGrafica {
                id: 101,
                largura: 8,
                altura: 8,
                tamanho_pixel: 1,
                fundo: "preto".into(),
                ops: vec![],
            };
            painel.mostrar(tela.clone());
            painel.mostrar(TelaGrafica {
                id: 102,
                ..tela.clone()
            });
            painel.fechar(101);
            painel.fechar(101);
            assert!(!painel.windows.borrow().contains_key(&101));
            assert!(painel.windows.borrow().contains_key(&102));
            assert!(!painel.fechadas.borrow().contains(&101));
            assert!(!rx.try_iter().any(|e| e.janela == 101 && e.tipo == "sair"));
            painel.mostrar(tela.clone());
            assert!(painel.windows.borrow().contains_key(&101));
            let win = painel.windows.borrow()[&101].win.clone();
            win.close();
            assert!(painel.fechadas.borrow().contains(&101));
            assert!(rx.try_iter().any(|e| e.janela == 101 && e.tipo == "sair"));
            painel.mostrar(tela);
            assert!(!painel.windows.borrow().contains_key(&101));
            painel.fechar_todas();
        });
    }

    #[test]
    fn canonical_keyboard_names_match_cli() {
        for (key, name) in [
            (Key::Right, "direita"),
            (Key::Left, "esquerda"),
            (Key::space, "espaco"),
            (Key::_0, "num_0"),
            (Key::F12, "f12"),
            (Key::Alt_R, "alt"),
            (Key::Control_L, "control"),
            (Key::Super_R, "super"),
            (Key::KP_0, "numerico_0"),
            (Key::A, "a"),
            (Key::ISO_Level3_Shift, "alt"),
        ] {
            assert_eq!(tecla_gdk(key), name);
        }
    }

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
