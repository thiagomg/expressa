//! Graphs pane: stacked `FolhaPronta` in a notebook, detachable window.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use gtk::cairo::{Format, ImageSurface};
use gtk::glib;
use gtk::prelude::*;
use gtk::{Align, Box as GtkBox, Button, DrawingArea, Label, Notebook, Orientation, Window};

use expressa::runtime::{FolhaPronta, pintar};

/// Minimum plot height while docked, so the pane does not collapse to the
/// header. The drawing area still grows to whatever it is allocated — the
/// same way `expressa-grafico` paints the whole window.
const ALTURA_MIN: i32 = 280;

pub struct Painel {
    /// Stays in the editor column so we can dock again.
    pub dock: GtkBox,
    root: GtkBox,
    notebook: Notebook,
    folhas: Rc<RefCell<HashMap<u64, FolhaPronta>>>,
    areas: RefCell<HashMap<u64, DrawingArea>>,
    detached: RefCell<Option<Window>>,
    btn_destacar: Button,
    app: RefCell<Option<gtk::Application>>,
}

impl Painel {
    pub fn new() -> Rc<Self> {
        let dock = GtkBox::new(Orientation::Vertical, 0);
        dock.set_visible(false);
        dock.set_hexpand(true);
        dock.set_vexpand(true);

        let header = GtkBox::new(Orientation::Horizontal, 8);
        header.set_margin_start(8);
        header.set_margin_end(8);
        header.set_margin_top(4);
        let title = Label::new(Some("Gráficos"));
        title.set_halign(Align::Start);
        title.set_hexpand(true);
        let btn_destacar = Button::with_label("Destacar");
        btn_destacar.set_tooltip_text(Some("Abrir numa janela à parte"));
        header.append(&title);
        header.append(&btn_destacar);

        let notebook = Notebook::new();
        notebook.set_hexpand(true);
        notebook.set_vexpand(true);
        notebook.set_scrollable(true);

        let root = GtkBox::new(Orientation::Vertical, 0);
        root.set_hexpand(true);
        root.set_vexpand(true);
        root.append(&header);
        root.append(&notebook);
        dock.append(&root);

        let painel = Rc::new(Self {
            dock,
            root,
            notebook,
            folhas: Rc::new(RefCell::new(HashMap::new())),
            areas: RefCell::new(HashMap::new()),
            detached: RefCell::new(None),
            btn_destacar,
            app: RefCell::new(None),
        });
        {
            let p = Rc::clone(&painel);
            painel.btn_destacar.connect_clicked(move |_| {
                if p.detached.borrow().is_some() {
                    p.acoplar();
                } else {
                    p.destacar_rc();
                }
            });
        }
        painel
    }

    pub fn mostrar(&self, folha: FolhaPronta) {
        let id = folha.id;
        let nova = !self.areas.borrow().contains_key(&id);
        self.folhas.borrow_mut().insert(id, folha);
        if nova {
            let area = DrawingArea::new();
            area.set_hexpand(true);
            area.set_vexpand(true);
            area.set_halign(Align::Fill);
            area.set_valign(Align::Fill);
            // Floor for the docked pane. A taller allocation (the detached
            // window, or a dragged split) is what `pintar` receives.
            area.set_content_height(ALTURA_MIN);
            let folhas = Rc::clone(&self.folhas);
            area.set_draw_func(move |_, cr, w, h| {
                let folhas = folhas.borrow();
                let Some(folha) = folhas.get(&id) else { return };
                if w < 8 || h < 8 {
                    return;
                }
                let pixels = pintar(folha, w as u32, h as u32);
                let Ok(mut surf) = ImageSurface::create(Format::Rgb24, w, h) else {
                    return;
                };
                let stride = surf.stride() as usize;
                if let Ok(mut data) = surf.data() {
                    let ww = w as usize;
                    let hh = h as usize;
                    for y in 0..hh {
                        for x in 0..ww {
                            let px = pixels[y * ww + x];
                            let i = y * stride + x * 4;
                            if i + 2 < data.len() {
                                data[i] = (px & 0xff) as u8;
                                data[i + 1] = ((px >> 8) & 0xff) as u8;
                                data[i + 2] = ((px >> 16) & 0xff) as u8;
                            }
                        }
                    }
                }
                cr.set_source_surface(&surf, 0.0, 0.0).ok();
                let _ = cr.paint();
            });
            let n = self.areas.borrow().len() + 1;
            let lab = Label::new(Some(&format!("{n}")));
            self.notebook.append_page(&area, Some(&lab));
            self.areas.borrow_mut().insert(id, area);
        }
        if let Some(area) = self.areas.borrow().get(&id) {
            area.queue_draw();
            if let Some(num) = self.notebook.page_num(area) {
                self.notebook.set_current_page(Some(num));
            }
        }
        self.dock.set_visible(self.detached.borrow().is_none());
        if let Some(win) = self.detached.borrow().as_ref() {
            win.present();
        }
    }

    pub fn limpar(&self) {
        while self.notebook.n_pages() > 0 {
            self.notebook.remove_page(Some(0));
        }
        self.areas.borrow_mut().clear();
        self.folhas.borrow_mut().clear();
        if self.detached.borrow().is_none() {
            self.dock.set_visible(false);
        }
    }

    fn destacar_rc(self: &Rc<Self>) {
        if self.detached.borrow().is_some() {
            return;
        }
        self.dock.remove(&self.root);
        self.dock.set_visible(false);
        let win = Window::builder()
            .title("Gráficos — Expressa")
            .default_width(800)
            .default_height(560)
            .build();
        win.set_child(Some(&self.root));
        if let Some(app) = self.app.borrow().as_ref() {
            win.set_application(Some(app));
        }
        self.btn_destacar.set_label("Acoplar");
        let p = Rc::clone(self);
        win.connect_close_request(move |_| {
            p.voltar_ao_dock();
            glib::Propagation::Proceed
        });
        win.present();
        *self.detached.borrow_mut() = Some(win);
    }

    fn voltar_ao_dock(&self) {
        let Some(win) = self.detached.borrow_mut().take() else {
            return;
        };
        win.set_child(None::<&gtk::Widget>);
        if self.root.parent().is_none() {
            self.dock.append(&self.root);
        }
        self.btn_destacar.set_label("Destacar");
        self.dock.set_visible(!self.areas.borrow().is_empty());
    }

    fn acoplar(&self) {
        let win = self.detached.borrow().clone();
        self.voltar_ao_dock();
        if let Some(win) = win {
            win.close();
        }
    }

    /// Close the floating graphs window without docking it back. Used when
    /// the editor itself is closing, so the sibling window does not keep
    /// the GtkApplication alive.
    pub fn fechar_janela_destacada(&self) {
        let Some(win) = self.detached.borrow_mut().take() else {
            return;
        };
        win.set_child(None::<&gtk::Widget>);
        win.close();
    }

    pub fn set_application(&self, app: &gtk::Application) {
        *self.app.borrow_mut() = Some(app.clone());
        if let Some(win) = self.detached.borrow().as_ref() {
            win.set_application(Some(app));
        }
    }
}


