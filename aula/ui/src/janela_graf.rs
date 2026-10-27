//! Sibling windows for `tela_grafica::mostre`. Not the Gráficos dock.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::mpsc::Sender;

use expressa::runtime::{EventoGraf, TelaGrafica};
use gtk::cairo::{Format, ImageSurface};
use gtk::gdk::Key;
use gtk::glib;
use gtk::prelude::*;
use gtk::{DrawingArea, EventControllerKey, GestureClick, Window};

pub struct Painel {
    telas: Rc<RefCell<HashMap<u64, TelaGrafica>>>,
    windows: Rc<RefCell<HashMap<u64, Window>>>,
    eventos: Rc<RefCell<Option<Sender<EventoGraf>>>>,
}

impl Painel {
    pub fn new() -> Rc<Self> {
        Rc::new(Self {
            telas: Rc::new(RefCell::new(HashMap::new())),
            windows: Rc::new(RefCell::new(HashMap::new())),
            eventos: Rc::new(RefCell::new(None)),
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
            let telas = Rc::clone(&self.telas);
            area.set_draw_func(move |_, cr, w, h| {
                let telas = telas.borrow();
                let Some(tela) = telas.get(&id) else { return };
                if w < 1 || h < 1 {
                    return;
                }
                let pixels = tela.rgb32();
                let tw = tela.largura.max(1);
                let th = tela.altura.max(1);
                let Ok(mut surf) = ImageSurface::create(Format::Rgb24, tw as i32, th as i32) else {
                    return;
                };
                let stride = surf.stride() as usize;
                if let Ok(mut data) = surf.data() {
                    for y in 0..th as usize {
                        for x in 0..tw as usize {
                            let px = pixels[y * tw as usize + x];
                            let i = y * stride + x * 4;
                            if i + 2 < data.len() {
                                data[i] = (px & 0xff) as u8;
                                data[i + 1] = ((px >> 8) & 0xff) as u8;
                                data[i + 2] = ((px >> 16) & 0xff) as u8;
                            }
                        }
                    }
                }
                cr.save().ok();
                cr.scale(w as f64 / tw as f64, h as f64 / th as f64);
                cr.set_source_surface(&surf, 0.0, 0.0).ok();
                let _ = cr.paint();
                cr.restore().ok();
            });

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
                    let px = (x / w * tw).floor() as i32;
                    let py = (y / h * th).floor() as i32;
                    let botao = match g.current_button() {
                        3 => "direito",
                        2 => "meio",
                        _ => "esquerdo",
                    };
                    painel.emita(EventoGraf::clique(
                        px.clamp(0, tw as i32 - 1),
                        py.clamp(0, th as i32 - 1),
                        botao,
                    ));
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
                    .map(|t| t.largura as i32)
                    .unwrap_or(320),
            );
            win.set_default_height(
                self.telas
                    .borrow()
                    .get(&id)
                    .map(|t| t.altura as i32)
                    .unwrap_or(200),
            );
            {
                let painel = Rc::clone(self);
                win.connect_close_request(move |_| {
                    painel.emita(EventoGraf::sair());
                    painel.telas.borrow_mut().remove(&id);
                    painel.windows.borrow_mut().remove(&id);
                    glib::Propagation::Proceed
                });
            }
            win.present();
            self.windows.borrow_mut().insert(id, win);
        }
        if let Some(win) = self.windows.borrow().get(&id) {
            if let Some(child) = win.child() {
                child.queue_draw();
            }
            win.present();
        }
    }

    pub fn limpar(&self) {
        self.fechar_todas();
    }

    pub fn fechar_todas(&self) {
        *self.eventos.borrow_mut() = None;
        let windows: Vec<Window> = self.windows.borrow_mut().drain().map(|(_, w)| w).collect();
        self.telas.borrow_mut().clear();
        for w in windows {
            w.close();
        }
    }
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
