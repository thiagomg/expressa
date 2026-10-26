//! Sibling windows for `tela_grafica::mostre`. Not the Gráficos dock.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use expressa::runtime::TelaGrafica;
use gtk::cairo::{Format, ImageSurface};
use gtk::glib;
use gtk::prelude::*;
use gtk::{DrawingArea, Window};

pub struct Painel {
    telas: Rc<RefCell<HashMap<u64, TelaGrafica>>>,
    windows: Rc<RefCell<HashMap<u64, Window>>>,
}

impl Painel {
    pub fn new() -> Rc<Self> {
        Rc::new(Self {
            telas: Rc::new(RefCell::new(HashMap::new())),
            windows: Rc::new(RefCell::new(HashMap::new())),
        })
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
                let telas = Rc::clone(&self.telas);
                let windows = Rc::clone(&self.windows);
                win.connect_close_request(move |_| {
                    telas.borrow_mut().remove(&id);
                    windows.borrow_mut().remove(&id);
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
        let windows: Vec<Window> = self.windows.borrow_mut().drain().map(|(_, w)| w).collect();
        self.telas.borrow_mut().clear();
        for w in windows {
            w.close();
        }
    }
}
