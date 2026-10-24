//! Launch the `expressa-tutorial` binary and the first-run welcome window.

use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::rc::Rc;

use gtk::glib;
use gtk::prelude::*;
use gtk::{Align, ApplicationWindow, Box as GtkBox, Button, Label, Orientation, Window};

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

/// First-launch welcome. `on_done` runs when the student clicks Começar
/// (or closes the window).
pub fn mostrar_boas_vindas(parent: &ApplicationWindow, on_done: impl Fn() + 'static) {
    let win = Window::builder()
        .title("Bem-vindo ao Expressa")
        .default_width(480)
        .default_height(280)
        .modal(true)
        .build();
    if let Some(app) = parent.application() {
        win.set_application(Some(&app));
    }
    win.set_transient_for(Some(parent));

    let col = GtkBox::new(Orientation::Vertical, 16);
    col.set_margin_top(28);
    col.set_margin_bottom(24);
    col.set_margin_start(28);
    col.set_margin_end(28);

    let titulo = Label::new(Some("Bem-vindo ao Expressa"));
    titulo.add_css_class("title-2");
    titulo.set_xalign(0.0);

    let texto = Label::new(Some(
        "Para começar, veja os tutoriais. Eles ensinam a escrever, perguntar, decidir e desenhar, um capítulo de cada vez.",
    ));
    texto.set_wrap(true);
    texto.set_xalign(0.0);
    texto.set_max_width_chars(52);

    let botoes = GtkBox::new(Orientation::Horizontal, 8);
    botoes.set_halign(Align::End);
    botoes.set_hexpand(true);

    let tut = Button::with_label("Abrir tutoriais");
    tut.add_css_class("suggested-action");
    let comecar = Button::with_label("Começar");

    tut.connect_clicked(|_| {
        if let Err(e) = spawn_tutorial() {
            eprintln!("{e}");
        }
    });

    let on_done = Rc::new(on_done);
    let finished = Rc::new(std::cell::Cell::new(false));
    let finish = Rc::new({
        let on_done = Rc::clone(&on_done);
        let finished = Rc::clone(&finished);
        move || {
            if !finished.replace(true) {
                on_done();
            }
        }
    });
    comecar.connect_clicked({
        let win = win.clone();
        move |_| {
            win.close();
        }
    });
    win.connect_close_request({
        let finish = Rc::clone(&finish);
        move |_| {
            finish();
            glib::Propagation::Proceed
        }
    });

    botoes.append(&tut);
    botoes.append(&comecar);
    col.append(&titulo);
    col.append(&texto);
    col.append(&botoes);
    win.set_child(Some(&col));
    win.present();
}
