//! Graphs pane: stacked `FolhaPronta` in a notebook, detachable window.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use gtk::cairo::{Format, ImageSurface};
use gtk::glib;
use gtk::prelude::*;
use gtk::{
    Align, Box as GtkBox, Button, DrawingArea, Label, Notebook, Orientation, PolicyType,
    ScrolledWindow, Window,
};

use expressa::runtime::{FolhaPronta, pintar};

/// Viewport height for one graph at the viewer ratio, capped so the editor
/// keeps room. Extra graphs (and a taller plot) scroll inside this.
const VIEWPORT_MAX: i32 = 360;

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
        dock.set_vexpand(false);

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
        notebook.set_vexpand(false);
        notebook.set_scrollable(true);

        let root = GtkBox::new(Orientation::Vertical, 0);
        root.set_hexpand(true);
        root.set_vexpand(false);
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
            area.set_vexpand(false);
            area.set_halign(Align::Fill);
            area.set_valign(Align::Start);
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
            let inner = GtkBox::new(Orientation::Vertical, 0);
            inner.set_halign(Align::Fill);
            inner.set_valign(Align::Start);
            inner.set_hexpand(true);
            inner.set_vexpand(false);
            inner.append(&area);
            let scroll = ScrolledWindow::builder()
                .hscrollbar_policy(PolicyType::Never)
                .vscrollbar_policy(PolicyType::Automatic)
                .propagate_natural_width(false)
                .propagate_natural_height(true)
                .overlay_scrolling(true)
                .hexpand(true)
                .vexpand(false)
                .max_content_height(VIEWPORT_MAX)
                .child(&inner)
                .build();
            {
                let folhas = Rc::clone(&self.folhas);
                let pending = Rc::new(Cell::new(false));
                area.connect_resize(move |area, w, _h| {
                    if w < 8 || pending.get() {
                        return;
                    }
                    pending.set(true);
                    let area = area.clone();
                    let folhas = Rc::clone(&folhas);
                    let pending = Rc::clone(&pending);
                    glib::idle_add_local_once(move || {
                        pending.set(false);
                        let n = folhas
                            .borrow()
                            .get(&id)
                            .map(|f| f.graficos.len())
                            .unwrap_or(1);
                        let w = area.allocated_width();
                        if w >= 8 {
                            ajustar_conteudo(&area, w, n);
                        }
                    });
                });
            }
            let n = self.areas.borrow().len() + 1;
            let lab = Label::new(Some(&format!("{n}")));
            self.notebook.append_page(&scroll, Some(&lab));
            self.areas.borrow_mut().insert(id, area);
        }
        if let Some(area) = self.areas.borrow().get(&id) {
            let n = self
                .folhas
                .borrow()
                .get(&id)
                .map(|f| f.graficos.len())
                .unwrap_or(1);
            let w = largura_util(area);
            if w >= 8 {
                ajustar_conteudo(area, w, n);
            }
            area.queue_draw();
            if let Some(page) = area.ancestor(ScrolledWindow::static_type()) {
                self.notebook.set_current_page(self.notebook.page_num(&page));
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
            .default_width(720)
            .default_height(480)
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

/// Viewer default 800×560 → 7:10. Each stacked graph keeps that ratio from
/// the allocated width; extra height scrolls instead of shrinking the plot.
fn altura_folha(largura: i32, n_graficos: usize) -> i32 {
    let n = n_graficos.max(1) as i64;
    let por = (largura as i64 * 7 / 10).max(64);
    let gap = 8 * (n - 1);
    (por * n + gap).clamp(64, 4096) as i32
}

fn ajustar_conteudo(area: &DrawingArea, largura: i32, n_graficos: usize) {
    let w = largura.max(1);
    let h = altura_folha(w, n_graficos);
    if area.content_width() != w {
        area.set_content_width(w);
    }
    if area.content_height() != h {
        area.set_content_height(h);
    }
    if area.height_request() != h {
        area.set_height_request(h);
    }
    let cap = altura_folha(w, 1).clamp(96, VIEWPORT_MAX);
    if let Some(scroll) = area
        .ancestor(ScrolledWindow::static_type())
        .and_then(|wid| wid.downcast::<ScrolledWindow>().ok())
    {
        if scroll.max_content_height() != cap {
            scroll.set_max_content_height(cap);
        }
    }
}

fn largura_util(area: &DrawingArea) -> i32 {
    let da_janela = area
        .ancestor(ScrolledWindow::static_type())
        .and_then(|w| w.downcast::<ScrolledWindow>().ok())
        .map(|s| s.allocated_width())
        .unwrap_or(0);
    if da_janela >= 8 {
        da_janela
    } else {
        area.allocated_width()
    }
}

#[cfg(test)]
mod tests {
    use super::altura_folha;

    #[test]
    fn altura_segue_a_largura_e_o_numero_de_graficos() {
        assert_eq!(altura_folha(800, 1), 560);
        assert_eq!(altura_folha(800, 2), 560 * 2 + 8);
        assert_eq!(altura_folha(400, 1), 280);
    }
}
