//! Native window that paints a `.cart` file and reloads when it changes.
//! Live mode (`--ao-vivo`) reads framed messages on stdin.

use std::cell::RefCell;
use std::collections::HashMap;
use std::io::{self, BufReader, Write};
use std::path::Path;
use std::rc::Rc;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant, SystemTime};

use super::folha::{FolhaPronta, pintar};
use super::graf::{cor_tela_rgb32, png_to_rgb32};
use super::graf_host::{EstadoEntrada, EventoGraf};
use super::graf_proto::{self, Msg, write_msg};
use super::imagem::Imagem;
use super::letterbox::{blit_letterbox, letterbox};

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
    let mut buffer = blit_letterbox(&pixels, iw, ih, width as u32, height as u32, 0x00FFFFFF);
    while window.is_open() {
        let (w, h) = window.get_size();
        let new_mtime = file_mtime(path);
        let resized = w != width || h != height;
        let changed = new_mtime != mtime;
        if changed {
            if let Ok(b) = std::fs::read(path)
                && let Ok((nw, nh, px)) = png_to_rgb32(&b)
            {
                iw = nw;
                ih = nh;
                pixels = px;
            }
            mtime = new_mtime;
        }
        if changed || resized || buffer.len() != w * h.max(1) {
            width = w.max(1);
            height = h.max(1);
            buffer = blit_letterbox(&pixels, iw, ih, width as u32, height as u32, 0x00FFFFFF);
        }
        if let Err(e) = window.update_with_buffer(&buffer, width, height) {
            return Err(e.to_string());
        }
        thread::sleep(Duration::from_millis(40));
    }
    Ok(())
}

pub fn run_live_viewer() -> Result<(), String> {
    let (tx, rx) = mpsc::channel::<Result<Msg, String>>();
    thread::spawn(move || {
        let mut stdin = BufReader::new(io::stdin());
        loop {
            match graf_proto::read_msg(&mut stdin) {
                Ok(m) => {
                    if tx.send(Ok(m)).is_err() {
                        break;
                    }
                }
                Err(e) => {
                    let _ = tx.send(Err(e.to_string()));
                    break;
                }
            }
        }
    });

    let mut proto: Box<dyn Write> = protocolo_saida()?;
    let mut figuras: HashMap<u32, Imagem> = HashMap::new();
    let mut id = 0u64;
    let mut largura = 320u32;
    let mut altura = 200u32;
    let mut tamanho_pixel = 1u32;
    let mut fundo = "branco".to_string();
    let mut tela = None;
    let mut window: Option<minifb::Window> = None;
    let mut buffer: Vec<u32> = Vec::new();
    let mut mouse_esq = false;
    let mut mouse_dir = false;
    let mut mouse_meio = false;
    let mut pending = false;
    let mut stdin_fim = false;
    let mut ja_apareceu = false;
    let entrada = Rc::new(RefCell::new(EstadoEntrada::default()));
    let mut focada = true;
    let mut pixels_ok = false;
    let mut buf_w = 0usize;
    let mut buf_h = 0usize;
    let mut last_frame: Option<Instant> = None;

    loop {
        // A producer streaming frames must not starve OS input processing.
        for msg in rx.try_iter().take(64) {
            match msg {
                Ok(Msg::Abra {
                    id: nid,
                    largura: w,
                    altura: h,
                    tamanho_pixel: n,
                    fundo: f,
                }) => {
                    id = nid;
                    largura = w.max(1);
                    altura = h.max(1);
                    tamanho_pixel = n.max(1);
                    fundo = f;
                }
                Ok(Msg::CarregueFigura {
                    handle,
                    largura: w,
                    altura: h,
                    rgba,
                }) => {
                    if let Ok(img) = Imagem::nova(w, h, rgba) {
                        figuras.insert(handle, img);
                    }
                }
                Ok(Msg::Mostre { id: nid, ops }) => {
                    id = nid;
                    tela = Some(graf_proto::tela_de_mostre(
                        nid,
                        largura,
                        altura,
                        tamanho_pixel,
                        &fundo,
                        &ops,
                        &figuras,
                    ));
                    pixels_ok = false;
                }
                Ok(Msg::Espere { .. }) => {
                    pending = true;
                }
                Ok(Msg::Evento { .. }) => {}
                Err(_) => {
                    stdin_fim = true;
                }
            }
        }

        if window.is_none()
            && let Some(t) = tela.as_ref()
        {
            let mut win = abre_janela(t.janela_largura(), t.janela_altura())?;
            win.set_input_callback(Box::new(TecladoCallback(Rc::clone(&entrada))));
            window = Some(win);
            ja_apareceu = false;
        }

        if let Some(win) = window.as_mut() {
            let (ww, wh) = win.get_size();
            let ww = ww.max(1);
            let wh = wh.max(1);
            let dirty = !pixels_ok || buf_w != ww || buf_h != wh;
            let redraw =
                last_frame.is_none_or(|t| t.elapsed() >= Duration::from_secs_f64(1.0 / 60.0));
            if redraw {
                if dirty {
                    if let Some(t) = tela.as_ref() {
                        let pixels = t.rgb32();
                        buffer = blit_letterbox(
                            &pixels,
                            t.largura,
                            t.altura,
                            ww as u32,
                            wh as u32,
                            cor_tela_rgb32(&t.fundo),
                        );
                    } else if buffer.len() != ww * wh {
                        buffer = vec![0x00FFFFFF; ww * wh];
                    }
                    pixels_ok = true;
                    buf_w = ww;
                    buf_h = wh;
                }
                // Present the cached buffer too: an exposed X11 window may
                // need repainting even when the program has not drawn again.
                win.update_with_buffer(&buffer, ww, wh)
                    .map_err(|e| e.to_string())?;
                last_frame = Some(Instant::now());
            } else {
                // Poll input between presentations instead of waiting for a frame.
                win.update();
            }
            if focada && !win.is_active() {
                entrada.borrow_mut().registre(EventoGraf {
                    tipo: "desfoco".into(),
                    ..EventoGraf::nada()
                });
            }
            focada = win.is_active();
            if win.is_open() {
                ja_apareceu = true;
                enfileira_entrada(
                    win,
                    largura,
                    altura,
                    ww,
                    wh,
                    &mut mouse_esq,
                    &mut mouse_dir,
                    &mut mouse_meio,
                    &mut entrada.borrow_mut(),
                );
            } else if ja_apareceu {
                entrada.borrow_mut().registre(EventoGraf::sair());
                pending_flush(&mut proto, id, &mut pending, &mut entrada.borrow_mut());
                break;
            }
        }

        pending_flush(&mut proto, id, &mut pending, &mut entrada.borrow_mut());

        if stdin_fim {
            if pending {
                entrada.borrow_mut().registre(EventoGraf::sair());
                pending_flush(&mut proto, id, &mut pending, &mut entrada.borrow_mut());
            }
            // Program ended (`casa.lep`): keep the last frame until the student
            // closes the window. No canvas and no window: nothing to show.
            if window.is_none() && tela.is_none() {
                break;
            }
        }
        // Input is independent of rendering (60 Hz). Keep idle CPU usage low
        // without putting each snapshot behind minifb's frame limiter.
        thread::sleep(Duration::from_millis(2));
    }
    Ok(())
}

struct TecladoCallback(Rc<RefCell<EstadoEntrada>>);
impl minifb::InputCallback for TecladoCallback {
    fn add_char(&mut self, _c: u32) {}
    fn set_key_state(&mut self, key: minifb::Key, down: bool) {
        let tecla = tecla_minifb(key);
        if tecla.is_empty() {
            return;
        }
        let mut e = if down {
            EventoGraf::tecla(tecla)
        } else {
            EventoGraf::soltura(tecla)
        };
        e.fisica = format!("{key:?}");
        self.0.borrow_mut().registre(e);
    }
}

fn enfileira_entrada(
    win: &minifb::Window,
    largura: u32,
    altura: u32,
    ww: usize,
    wh: usize,
    mouse_esq: &mut bool,
    mouse_dir: &mut bool,
    mouse_meio: &mut bool,
    entrada: &mut EstadoEntrada,
) {
    let anteriores = [*mouse_esq, *mouse_dir, *mouse_meio];
    let pixel = mouse_px(win, largura, altura, ww, wh);
    let atuais = [
        win.get_mouse_down(minifb::MouseButton::Left),
        win.get_mouse_down(minifb::MouseButton::Right),
        win.get_mouse_down(minifb::MouseButton::Middle),
    ];
    if let Some((x, y)) = pixel {
        for ((antes, agora), botao) in anteriores
            .into_iter()
            .zip(atuais)
            .zip(["esquerdo", "direito", "meio"])
        {
            if antes != agora {
                let mut e = EventoGraf::clique(x, y, botao);
                if !agora {
                    e.tipo = "solte".into();
                }
                entrada.registre(e);
            }
        }
    }
    [*mouse_esq, *mouse_dir, *mouse_meio] = atuais;
}

fn pending_flush(proto: &mut impl Write, id: u64, pending: &mut bool, entrada: &mut EstadoEntrada) {
    if !*pending {
        return;
    }
    *pending = false;
    let mut captura = entrada.capture();
    for e in &mut captura.eventos {
        e.janela = id;
    }
    let _ = write_msg(proto, &Msg::Evento { id, captura });
}

/// Keep the protocol on the original stdout pipe; send minifb/OS noise elsewhere.
fn protocolo_saida() -> Result<Box<dyn Write>, String> {
    #[cfg(unix)]
    {
        use std::os::fd::{FromRawFd, IntoRawFd};
        let proto = unsafe { libc::dup(1) };
        if proto < 0 {
            return Err("não duplicou stdout do visor".into());
        }
        let null = std::fs::File::options()
            .write(true)
            .open("/dev/null")
            .map_err(|e| e.to_string())?;
        let nfd = null.into_raw_fd();
        if unsafe { libc::dup2(nfd, 1) } < 0 {
            return Err("não redirecionou stdout do visor".into());
        }
        unsafe { libc::close(nfd) };
        Ok(Box::new(unsafe { std::fs::File::from_raw_fd(proto) }))
    }
    #[cfg(not(unix))]
    {
        Ok(Box::new(io::stdout()))
    }
}

fn abre_janela(largura: u32, altura: u32) -> Result<minifb::Window, String> {
    // minifb's Wayland backend leaves proxies attached on drop (the
    // "queue destroyed while proxies still attached" warning). Prefer X11.
    let wayland = std::env::var("WAYLAND_DISPLAY").ok();
    if wayland.is_some() {
        unsafe { std::env::remove_var("WAYLAND_DISPLAY") };
    }
    let mut win = match minifb::Window::new(
        "Expressa — tela gráfica",
        largura.max(1) as usize,
        altura.max(1) as usize,
        minifb::WindowOptions {
            resize: true,
            ..minifb::WindowOptions::default()
        },
    ) {
        Ok(w) => w,
        Err(_) if wayland.is_some() => {
            unsafe { std::env::set_var("WAYLAND_DISPLAY", wayland.as_deref().unwrap()) };
            minifb::Window::new(
                "Expressa — tela gráfica",
                largura.max(1) as usize,
                altura.max(1) as usize,
                minifb::WindowOptions {
                    resize: true,
                    ..minifb::WindowOptions::default()
                },
            )
            .map_err(|e| format!("não abriu a janela: {e}"))?
        }
        Err(e) => return Err(format!("não abriu a janela: {e}")),
    };
    // The live loop paces rendering separately so keyboard polling stays fast.
    win.set_target_fps(0);
    Ok(win)
}

fn mouse_px(
    win: &minifb::Window,
    largura: u32,
    altura: u32,
    ww: usize,
    wh: usize,
) -> Option<(i32, i32)> {
    let (mx, my) = win.get_mouse_pos(minifb::MouseMode::Clamp)?;
    let lb = letterbox(
        ww.max(1) as f64,
        wh.max(1) as f64,
        largura.max(1) as f64,
        altura.max(1) as f64,
    );
    lb.canvas_pixel(mx as f64, my as f64, largura as f64, altura as f64)
}

fn tecla_minifb(k: minifb::Key) -> String {
    use minifb::Key::*;
    match k {
        A => "a".into(),
        B => "b".into(),
        C => "c".into(),
        D => "d".into(),
        E => "e".into(),
        F => "f".into(),
        G => "g".into(),
        H => "h".into(),
        I => "i".into(),
        J => "j".into(),
        K => "k".into(),
        L => "l".into(),
        M => "m".into(),
        N => "n".into(),
        O => "o".into(),
        P => "p".into(),
        Q => "q".into(),
        R => "r".into(),
        S => "s".into(),
        T => "t".into(),
        U => "u".into(),
        V => "v".into(),
        W => "w".into(),
        X => "x".into(),
        Y => "y".into(),
        Z => "z".into(),
        Key0 => "num_0".into(),
        Key1 => "num_1".into(),
        Key2 => "num_2".into(),
        Key3 => "num_3".into(),
        Key4 => "num_4".into(),
        Key5 => "num_5".into(),
        Key6 => "num_6".into(),
        Key7 => "num_7".into(),
        Key8 => "num_8".into(),
        Key9 => "num_9".into(),
        Enter => "enter".into(),
        Escape => "escape".into(),
        Space => "espaco".into(),
        Tab => "tab".into(),
        Backspace => "backspace".into(),
        Up => "cima".into(),
        Down => "baixo".into(),
        Left => "esquerda".into(),
        Right => "direita".into(),
        LeftShift | RightShift => "shift".into(),
        LeftCtrl | RightCtrl => "control".into(),
        LeftAlt | RightAlt => "alt".into(),
        LeftSuper | RightSuper => "super".into(),
        Home => "inicio".into(),
        End => "fim".into(),
        PageUp => "pagina_acima".into(),
        PageDown => "pagina_abaixo".into(),
        Insert => "insert".into(),
        Delete => "delete".into(),
        Apostrophe => "apostrofo".into(),
        Backquote => "crase".into(),
        Backslash => "barra_invertida".into(),
        Comma => "virgula".into(),
        Equal => "igual".into(),
        LeftBracket => "colchete_esquerdo".into(),
        RightBracket => "colchete_direito".into(),
        Minus => "menos".into(),
        Period => "ponto".into(),
        Semicolon => "ponto_e_virgula".into(),
        Slash => "barra".into(),
        CapsLock => "caps_lock".into(),
        NumLock => "num_lock".into(),
        ScrollLock => "scroll_lock".into(),
        Menu => "menu".into(),
        Pause => "pause".into(),
        NumPad0 => "numerico_0".into(),
        NumPad1 => "numerico_1".into(),
        NumPad2 => "numerico_2".into(),
        NumPad3 => "numerico_3".into(),
        NumPad4 => "numerico_4".into(),
        NumPad5 => "numerico_5".into(),
        NumPad6 => "numerico_6".into(),
        NumPad7 => "numerico_7".into(),
        NumPad8 => "numerico_8".into(),
        NumPad9 => "numerico_9".into(),
        NumPadDot => "numerico_ponto".into(),
        NumPadSlash => "numerico_barra".into(),
        NumPadAsterisk => "numerico_asterisco".into(),
        NumPadMinus => "numerico_menos".into(),
        NumPadPlus => "numerico_mais".into(),
        NumPadEnter => "enter".into(),
        Unknown | Count => String::new(),
        other => format!("{other:?}").to_lowercase(),
    }
}

#[cfg(test)]
mod input_tests {
    use super::*;
    use minifb::{InputCallback, Key};

    #[test]
    fn callback_preserves_tap_and_modifier_order_in_a_single_pump() {
        let state = Rc::new(RefCell::new(EstadoEntrada::default()));
        let mut callback = TecladoCallback(Rc::clone(&state));
        callback.set_key_state(Key::LeftCtrl, true);
        callback.set_key_state(Key::S, true);
        callback.set_key_state(Key::S, false);
        callback.set_key_state(Key::LeftCtrl, false);
        let mut bytes = Vec::new();
        let mut pending = true;
        pending_flush(&mut bytes, 42, &mut pending, &mut state.borrow_mut());
        assert!(!pending);
        let Msg::Evento { id, captura } = graf_proto::read_msg(&mut bytes.as_slice()).unwrap()
        else {
            panic!()
        };
        assert_eq!(id, 42);
        assert!(captura.teclas.is_empty());
        assert!(captura.apertadas.contains("s"));
        assert!(captura.soltas.contains("s"));
        assert_eq!(captura.eventos.len(), 4);
        assert!(captura.eventos[1].modificadores.contains("control"));
        assert_eq!(captura.eventos[1].janela, 42);
    }

    #[test]
    fn canonical_keyboard_names() {
        for (key, name) in [
            (Key::Right, "direita"),
            (Key::Left, "esquerda"),
            (Key::Space, "espaco"),
            (Key::Key0, "num_0"),
            (Key::F12, "f12"),
            (Key::RightAlt, "alt"),
            (Key::LeftCtrl, "control"),
            (Key::RightSuper, "super"),
            (Key::NumPad0, "numerico_0"),
        ] {
            assert_eq!(tecla_minifb(key), name);
        }
    }

    #[test]
    fn release_is_not_queued_behind_auto_repeat() {
        let state = Rc::new(RefCell::new(EstadoEntrada::default()));
        let mut callback = TecladoCallback(Rc::clone(&state));
        callback.set_key_state(Key::Right, true);
        assert!(state.borrow_mut().capture().teclas.contains("direita"));
        for _ in 0..10_000 {
            callback.set_key_state(Key::Right, true);
        }
        callback.set_key_state(Key::Right, false);
        let capture = state.borrow_mut().capture();
        assert!(capture.teclas.is_empty());
        assert!(capture.apertadas.is_empty());
        assert!(capture.soltas.contains("direita"));
        assert_eq!(capture.eventos.len(), 1);
        assert_eq!(capture.eventos[0].tipo, "tecla_solta");
        assert!(state.borrow_mut().capture().teclas.is_empty());
    }
}
