//! Native window that paints a `.cart` file and reloads when it changes.
//! Live mode (`--ao-vivo`) reads framed messages on stdin.

use std::collections::{HashMap, VecDeque};
use std::io::{self, BufReader, Write};
use std::path::Path;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant, SystemTime};

use super::folha::{FolhaPronta, pintar};
use super::graf::png_to_rgb32;
use super::graf_proto::{self, Msg, TIPO_CLIQUE, TIPO_NADA, TIPO_SAIR, TIPO_TECLA, write_msg};
use super::imagem::Imagem;

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
    let mut buffer = scale_rgb32(&pixels, iw, ih, width as u32, height as u32);
    while window.is_open() {
        let (w, h) = window.get_size();
        let new_mtime = file_mtime(path);
        let resized = w != width || h != height;
        let changed = new_mtime != mtime;
        if changed {
            if let Ok(b) = std::fs::read(path) {
                if let Ok((nw, nh, px)) = png_to_rgb32(&b) {
                    iw = nw;
                    ih = nh;
                    pixels = px;
                }
            }
            mtime = new_mtime;
        }
        if changed || resized || buffer.len() != w * h.max(1) {
            width = w.max(1);
            height = h.max(1);
            buffer = scale_rgb32(&pixels, iw, ih, width as u32, height as u32);
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
    let mut fundo = "branco".to_string();
    let mut tela = None;
    let mut window: Option<minifb::Window> = None;
    let mut buffer: Vec<u32> = Vec::new();
    let mut mouse_esq = false;
    let mut mouse_dir = false;
    let mut mouse_meio = false;
    let mut pending: Option<(i32, Instant)> = None;
    let mut stdin_fim = false;
    let mut ja_apareceu = false;
    let mut fila: VecDeque<Msg> = VecDeque::new();

    loop {
        while let Ok(msg) = rx.try_recv() {
            match msg {
                Ok(Msg::Abra {
                    id: nid,
                    largura: w,
                    altura: h,
                    fundo: f,
                }) => {
                    id = nid;
                    largura = w.max(1);
                    altura = h.max(1);
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
                        nid, largura, altura, &fundo, &ops, &figuras,
                    ));
                }
                Ok(Msg::Espere { timeout_ms, .. }) => {
                    let fim = if timeout_ms < 0 {
                        Instant::now() + Duration::from_secs(3600 * 24)
                    } else {
                        Instant::now() + Duration::from_millis(timeout_ms.max(0) as u64)
                    };
                    pending = Some((timeout_ms, fim));
                }
                Ok(Msg::Evento { .. }) => {}
                Err(_) => {
                    stdin_fim = true;
                }
            }
        }

        if window.is_none() {
            if let Some(t) = tela.as_ref() {
                window = Some(abre_janela(t.largura, t.altura)?);
                ja_apareceu = false;
            }
        }

        if let Some(win) = window.as_mut() {
            let (ww, wh) = win.get_size();
            let ww = ww.max(1);
            let wh = wh.max(1);
            if let Some(t) = tela.as_ref() {
                let pixels = t.rgb32();
                buffer = scale_rgb32(&pixels, t.largura, t.altura, ww as u32, wh as u32);
            } else if buffer.len() != ww * wh {
                buffer = vec![0x00FFFFFF; ww * wh];
            }
            if let Err(e) = win.update_with_buffer(&buffer, ww, wh) {
                return Err(e.to_string());
            }
            if win.is_open() {
                ja_apareceu = true;
                enfileira_entrada(
                    win,
                    id,
                    largura,
                    altura,
                    ww,
                    wh,
                    &mut mouse_esq,
                    &mut mouse_dir,
                    &mut mouse_meio,
                    &mut fila,
                );
            } else if ja_apareceu {
                enfileira(&mut fila, evento_sair(id));
                pending_flush(&mut proto, id, &mut pending, &mut fila);
                break;
            }
        }

        pending_flush(&mut proto, id, &mut pending, &mut fila);

        if stdin_fim {
            if pending.take().is_some() {
                let _ = write_msg(&mut proto, &evento_sair(id));
            }
            // Program ended (`casa.lep`): keep the last frame until the student
            // closes the window. No canvas and no window: nothing to show.
            if window.is_none() && tela.is_none() {
                break;
            }
        }
        if window.is_none() && pending.is_none() && !stdin_fim {
            thread::sleep(Duration::from_millis(15));
        }
    }
    Ok(())
}

const FILA_MAX: usize = 32;

fn enfileira(fila: &mut VecDeque<Msg>, msg: Msg) {
    if fila.len() >= FILA_MAX {
        fila.pop_front();
    }
    fila.push_back(msg);
}

fn enfileira_entrada(
    win: &minifb::Window,
    id: u64,
    largura: u32,
    altura: u32,
    ww: usize,
    wh: usize,
    mouse_esq: &mut bool,
    mouse_dir: &mut bool,
    mouse_meio: &mut bool,
    fila: &mut VecDeque<Msg>,
) {
    let (x, y, botao) = amostra_mouse(
        win, largura, altura, ww, wh, mouse_esq, mouse_dir, mouse_meio,
    );
    if botao != 0 {
        enfileira(
            fila,
            Msg::Evento {
                id,
                tipo: TIPO_CLIQUE,
                x,
                y,
                botao,
                tecla: String::new(),
            },
        );
    }
    for k in win.get_keys_pressed(minifb::KeyRepeat::No) {
        enfileira(
            fila,
            Msg::Evento {
                id,
                tipo: TIPO_TECLA,
                x,
                y,
                botao: 0,
                tecla: tecla_minifb(k),
            },
        );
    }
}

fn pending_flush(
    proto: &mut impl Write,
    id: u64,
    pending: &mut Option<(i32, Instant)>,
    fila: &mut VecDeque<Msg>,
) {
    let Some((timeout_ms, fim)) = *pending else {
        return;
    };
    if let Some(msg) = fila.pop_front() {
        *pending = None;
        let _ = write_msg(proto, &msg);
        return;
    }
    if timeout_ms == 0 || Instant::now() >= fim {
        *pending = None;
        let _ = write_msg(
            proto,
            &Msg::Evento {
                id,
                tipo: TIPO_NADA,
                x: 0,
                y: 0,
                botao: 0,
                tecla: String::new(),
            },
        );
    }
}

fn evento_sair(id: u64) -> Msg {
    Msg::Evento {
        id,
        tipo: TIPO_SAIR,
        x: 0,
        y: 0,
        botao: 0,
        tecla: String::new(),
    }
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
    win.set_target_fps(30);
    Ok(win)
}

fn amostra_mouse(
    win: &minifb::Window,
    largura: u32,
    altura: u32,
    ww: usize,
    wh: usize,
    mouse_esq: &mut bool,
    mouse_dir: &mut bool,
    mouse_meio: &mut bool,
) -> (i32, i32, u8) {
    let (x, y) = mouse_px(win, largura, altura, ww, wh);
    let esq = win.get_mouse_down(minifb::MouseButton::Left);
    let dir = win.get_mouse_down(minifb::MouseButton::Right);
    let meio = win.get_mouse_down(minifb::MouseButton::Middle);
    let botao = if esq && !*mouse_esq {
        1u8
    } else if dir && !*mouse_dir {
        2
    } else if meio && !*mouse_meio {
        3
    } else {
        0
    };
    *mouse_esq = esq;
    *mouse_dir = dir;
    *mouse_meio = meio;
    (x, y, botao)
}

fn mouse_px(win: &minifb::Window, largura: u32, altura: u32, ww: usize, wh: usize) -> (i32, i32) {
    match win.get_mouse_pos(minifb::MouseMode::Clamp) {
        Some((mx, my)) => {
            let x = (mx / ww.max(1) as f32 * largura as f32).floor() as i32;
            let y = (my / wh.max(1) as f32 * altura as f32).floor() as i32;
            (
                x.clamp(0, largura.saturating_sub(1) as i32),
                y.clamp(0, altura.saturating_sub(1) as i32),
            )
        }
        None => (0, 0),
    }
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
        Key0 => "0".into(),
        Key1 => "1".into(),
        Key2 => "2".into(),
        Key3 => "3".into(),
        Key4 => "4".into(),
        Key5 => "5".into(),
        Key6 => "6".into(),
        Key7 => "7".into(),
        Key8 => "8".into(),
        Key9 => "9".into(),
        Enter => "Enter".into(),
        Escape => "Esc".into(),
        Space => " ".into(),
        Tab => "Tab".into(),
        Backspace => "Backspace".into(),
        Up => "Cima".into(),
        Down => "Baixo".into(),
        Left => "Esquerda".into(),
        Right => "Direita".into(),
        other => format!("{other:?}"),
    }
}

fn scale_rgb32(src: &[u32], sw: u32, sh: u32, dw: u32, dh: u32) -> Vec<u32> {
    let dw = dw.max(1);
    let dh = dh.max(1);
    let sw = sw.max(1);
    let sh = sh.max(1);
    let mut out = vec![0u32; (dw * dh) as usize];
    for y in 0..dh {
        let sy = y * sh / dh;
        for x in 0..dw {
            let sx = x * sw / dw;
            out[(y * dw + x) as usize] = src[(sy * sw + sx) as usize];
        }
    }
    out
}
