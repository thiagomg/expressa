//! Launch the `expressa-tutorial` binary and the first-run welcome window.

use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::rc::Rc;

use gtk::gdk;
use gtk::glib;
use gtk::prelude::*;
use gtk::{
    Align, ApplicationWindow, Box as GtkBox, Button, CenterBox, CssProvider, Label, Orientation,
    Overlay,
};

pub fn spawn_tutorial() -> Result<(), String> {
    let mut cmd = tutorial_command()?;
    cmd.stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0);
    }
    cmd.spawn()
        .map(|_| ())
        .map_err(|e| format!("não abriu os tutoriais: {e}"))
}

fn tutorial_command() -> Result<Command, String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    if let Some(dir) = exe.parent() {
        let name = if cfg!(windows) {
            "expressa-tutorial.exe"
        } else {
            "expressa-tutorial"
        };
        let next = dir.join(name);
        if next.is_file() {
            return Ok(Command::new(next));
        }
        let bin = dir.join("bin").join(name);
        if bin.is_file() {
            return Ok(Command::new(bin));
        }
    }
    let path = PathBuf::from("expressa-tutorial");
    Ok(Command::new(path))
}

/// First-launch welcome as an overlay on the Aula window (not a second
/// toplevel). Extra windows were grabbing focus and freezing the IDE.
pub fn mostrar_boas_vindas(parent: &ApplicationWindow, on_done: impl Fn() + 'static) {
    install_css();
    let overlay = Overlay::new();
    overlay.set_hexpand(true);
    overlay.set_vexpand(true);
    if let Some(child) = parent.child() {
        parent.set_child(None::<&gtk::Widget>);
        overlay.set_child(Some(&child));
    }
    parent.set_child(Some(&overlay));

    let cartao = GtkBox::new(Orientation::Vertical, 16);
    cartao.add_css_class("boas-vindas-cartao");
    cartao.set_halign(Align::Center);
    cartao.set_valign(Align::Center);

    let titulo = Label::new(Some("Bem-vindo ao Expressa"));
    titulo.add_css_class("title-2");
    titulo.set_xalign(0.0);

    let texto = Label::new(Some(
        "Para começar, veja os tutoriais. Eles ensinam a escrever, perguntar, decidir e desenhar, um capítulo de cada vez.",
    ));
    texto.set_wrap(true);
    texto.set_xalign(0.0);
    texto.set_max_width_chars(52);

    let erro = Label::new(None);
    erro.add_css_class("error");
    erro.set_wrap(true);
    erro.set_xalign(0.0);
    erro.set_visible(false);

    let botoes = GtkBox::new(Orientation::Horizontal, 8);
    botoes.set_halign(Align::End);
    botoes.set_hexpand(true);
    let tut = Button::with_label("Abrir tutoriais");
    tut.add_css_class("suggested-action");
    let comecar = Button::with_label("Começar");
    botoes.append(&tut);
    botoes.append(&comecar);
    cartao.append(&titulo);
    cartao.append(&texto);
    cartao.append(&erro);
    cartao.append(&botoes);

    let fundo = CenterBox::new();
    fundo.set_orientation(Orientation::Vertical);
    fundo.set_halign(Align::Fill);
    fundo.set_valign(Align::Fill);
    fundo.set_hexpand(true);
    fundo.set_vexpand(true);
    fundo.add_css_class("boas-vindas-fundo");
    fundo.set_center_widget(Some(&cartao));
    let eat = gtk::GestureClick::new();
    eat.connect_pressed(|g, _, _, _| {
        g.set_state(gtk::EventSequenceState::Claimed);
    });
    fundo.add_controller(eat);
    overlay.add_overlay(&fundo);

    let on_done = Rc::new(on_done);
    let finished = Rc::new(std::cell::Cell::new(false));
    let dismiss = Rc::new({
        let fundo = fundo.clone();
        let overlay = overlay.clone();
        let on_done = Rc::clone(&on_done);
        let finished = Rc::clone(&finished);
        move || {
            if finished.replace(true) {
                return;
            }
            overlay.remove_overlay(&fundo);
            let on_done = Rc::clone(&on_done);
            glib::idle_add_local_once(move || on_done());
        }
    });
    tut.connect_clicked({
        let dismiss = Rc::clone(&dismiss);
        let erro = erro.clone();
        move |_| {
            if let Err(e) = spawn_tutorial() {
                erro.set_text(&e);
                erro.set_visible(true);
                return;
            }
            dismiss();
        }
    });
    comecar.connect_clicked({
        let dismiss = Rc::clone(&dismiss);
        move |_| dismiss()
    });
    let esc = gtk::EventControllerKey::new();
    esc.connect_key_pressed({
        let dismiss = Rc::clone(&dismiss);
        move |_, key, _, _| {
            if key == gtk::gdk::Key::Escape {
                dismiss();
                glib::Propagation::Stop
            } else {
                glib::Propagation::Proceed
            }
        }
    });
    fundo.set_focusable(true);
    fundo.add_controller(esc);
    fundo.grab_focus();
}

fn install_css() {
    let css = CssProvider::new();
    css.load_from_data(
        r#"
.boas-vindas-fundo {
    background-color: rgba(0, 0, 0, 0.45);
}
.boas-vindas-cartao {
    background-color: @theme_bg_color;
    color: @theme_fg_color;
    padding: 28px;
    border-radius: 12px;
    min-width: 440px;
}
        "#,
    );
    if let Some(display) = gdk::Display::default() {
        gtk::style_context_add_provider_for_display(
            &display,
            &css,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}
