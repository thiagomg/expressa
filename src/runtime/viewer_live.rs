//! Live CPU-rendered frames presented through an SDL2 texture with VSync.

use std::cell::OnceCell;
use std::collections::HashMap;
use std::io::{self, BufReader, Write};
use std::thread;
use std::time::Duration;

use sdl2::event::{Event, WindowEvent};
use sdl2::keyboard::{Keycode, Scancode};
use sdl2::mouse::MouseButton;
use sdl2::pixels::{Color, PixelFormatEnum};
use sdl2::rect::Rect;
use sdl2::render::{BlendMode, ScaleMode, TextureCreator, WindowCanvas};
use sdl2::video::WindowContext;

use super::graf::{TelaGrafica, cor_tela_rgb32};
use super::graf_host::{EstadoEntrada, EventoGraf};
use super::graf_proto::{self, Msg, write_msg};
use super::imagem::Imagem;
use super::letterbox::letterbox;
use super::viewer_inbox::CaixaVisor;

pub fn run_live_viewer() -> Result<(), String> {
    // SDL's X11 title conversion uses the C character locale. Rust starts in
    // "C" even when the environment requests UTF-8. Set LC_CTYPE before the
    // reader/SDL threads start, leaving number and date formatting untouched.
    #[cfg(unix)]
    init_character_locale();
    let inbox = CaixaVisor::default();
    let reader = inbox.clone();
    thread::spawn(move || {
        let mut stdin = BufReader::new(io::stdin());
        while let Ok(msg) = graf_proto::read_msg(&mut stdin) {
            if !reader.envie(msg) {
                return;
            }
        }
        reader.feche();
    });
    // Redirect native-library stdout before initializing SDL.
    let mut proto = protocolo_saida()?;
    let result = execute(&inbox, &mut proto);
    inbox.feche(); // Also wake a reader blocked on control backpressure on error.
    result
}

#[cfg(unix)]
fn init_character_locale() {
    // Prefer the user's locale. A C/invalid locale needs a UTF-8 fallback so
    // SDL can publish both WM_NAME and _NET_WM_NAME without corrupting titles.
    for locale in [c"", c"C.UTF-8", c"en_US.UTF-8", c"UTF-8"] {
        // This is the standalone viewer's startup, before it creates threads.
        unsafe {
            if libc::setlocale(libc::LC_CTYPE, locale.as_ptr()).is_null() {
                continue;
            }
            let encoding = std::ffi::CStr::from_ptr(libc::nl_langinfo(libc::CODESET)).to_bytes();
            if encoding.eq_ignore_ascii_case(b"UTF-8") || encoding.eq_ignore_ascii_case(b"UTF8") {
                break;
            }
        }
    }
}

fn execute(inbox: &CaixaVisor, proto: &mut impl Write) -> Result<(), String> {
    let mut figuras = HashMap::<u32, Imagem>::new();
    let mut id = 0;
    let mut largura = 320;
    let mut altura = 200;
    let mut tamanho_pixel = 1;
    let mut fundo = "branco".to_string();
    let mut tela: Option<TelaGrafica> = None;
    let mut dirty = false;
    let mut publica = true;
    let mut entrada = EstadoEntrada::default();
    let mut visible = true;
    // Declaration order keeps textures alive only while SDL/video are initialized.
    let mut _sdl = None;
    let mut pump = None;
    let mut canvas: Option<WindowCanvas> = None;
    let creator = OnceCell::<TextureCreator<WindowContext>>::new();
    let mut texture = None;
    let mut texture_size = (0, 0);

    loop {
        let (controls, frames, eof) = inbox.retire();
        for msg in controls.into_iter().chain(frames) {
            match msg {
                Msg::Abra {
                    id: nid,
                    largura: w,
                    altura: h,
                    tamanho_pixel: scale,
                    fundo: paper,
                } => {
                    id = nid;
                    largura = w.max(1);
                    altura = h.max(1);
                    tamanho_pixel = scale.max(1);
                    fundo = paper;
                }
                Msg::CarregueFigura {
                    handle,
                    largura: w,
                    altura: h,
                    rgba,
                } => {
                    if let Ok(img) = Imagem::nova(w, h, rgba) {
                        figuras.insert(handle, img);
                    }
                }
                Msg::Mostre { id: nid, ops } => {
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
                    dirty = true;
                }
                Msg::Evento { .. } => {}
            }
        }

        if canvas.is_none()
            && let Some(t) = &tela
        {
            let sdl = sdl2::init().map_err(|e| format!("não iniciou SDL: {e}"))?;
            let video = sdl
                .video()
                .map_err(|e| format!("não iniciou vídeo SDL: {e}"))?;
            let window = abre_janela(&video, t.janela_largura(), t.janela_altura())?;
            creator
                .set(window.texture_creator())
                .map_err(|_| "texturas já inicializadas".to_string())?;
            pump = Some(
                sdl.event_pump()
                    .map_err(|e| format!("não iniciou entrada SDL: {e}"))?,
            );
            canvas = Some(window);
            _sdl = Some(sdl);
        }

        let mut open = true;
        if let (Some(c), Some(events)) = (canvas.as_ref(), pump.as_mut()) {
            let window_id = c.window().id();
            for event in events.poll_iter() {
                if matches!(event, Event::RenderDeviceReset { .. }) {
                    texture_size = (0, 0);
                    dirty = true;
                }
                if event
                    .get_window_id()
                    .is_some_and(|event_id| event_id != window_id)
                {
                    continue;
                }
                open &= processe_evento(
                    &mut entrada,
                    event,
                    c.window().size(),
                    (largura, altura),
                    &mut visible,
                );
            }
        }
        if !open {
            entrada.registre(EventoGraf::sair());
            publique_entrada(proto, id, &mut publica, &mut entrada)?;
            break;
        }
        // Publish transitions without a request, before blocking on vertical sync.
        publique_entrada(proto, id, &mut publica, &mut entrada)?;

        if let (Some(c), Some(t)) = (canvas.as_mut(), tela.as_ref()) {
            if visible {
                if texture_size != (t.largura, t.altura) {
                    let mut new_texture = creator
                        .get()
                        .unwrap()
                        .create_texture_streaming(PixelFormatEnum::ARGB8888, t.largura, t.altura)
                        .map_err(|e| format!("não criou textura: {e}"))?;
                    new_texture.set_blend_mode(BlendMode::None);
                    new_texture.set_scale_mode(ScaleMode::Nearest);
                    texture = Some(new_texture);
                    texture_size = (t.largura, t.altura);
                    dirty = true;
                }
                let texture = texture.as_mut().unwrap();
                if dirty {
                    let pixels = t.rgb32();
                    texture
                        .with_lock(None, |bytes, pitch| {
                            escreva_pixels(&pixels, bytes, pitch, t.largura, t.altura)
                        })
                        .map_err(|e| format!("não atualizou textura: {e}"))??;
                    dirty = false;
                }
                let (w, h) = c
                    .output_size()
                    .map_err(|e| format!("não leu tamanho do visor: {e}"))?;
                if w > 0 && h > 0 {
                    let paper = cor_tela_rgb32(&t.fundo);
                    c.set_draw_color(Color::RGB(
                        (paper >> 16) as u8,
                        (paper >> 8) as u8,
                        paper as u8,
                    ));
                    c.clear();
                    let (x, y, width, height) =
                        letterbox(w as f64, h as f64, t.largura as f64, t.altura as f64).as_i32();
                    c.copy(texture, None, Rect::new(x, y, width as u32, height as u32))
                        .map_err(|e| format!("não desenhou textura: {e}"))?;
                    c.present(); // Swap the complete backbuffer, synchronized to the display.
                } else {
                    thread::sleep(Duration::from_millis(2));
                }
            } else {
                // Minimized windows must keep servicing input/close/capture requests.
                thread::sleep(Duration::from_millis(2));
            }
        } else {
            if eof {
                break;
            }
            thread::sleep(Duration::from_millis(2));
        }
        // EOF intentionally keeps the last complete scene visible until window close.
    }
    Ok(())
}

fn abre_janela(
    video: &sdl2::VideoSubsystem,
    largura: u32,
    altura: u32,
) -> Result<WindowCanvas, String> {
    sdl2::hint::set_with_priority("SDL_RENDER_VSYNC", "1", &sdl2::hint::Hint::Override);
    let window = video
        .window("Expressa — tela gráfica", largura.max(1), altura.max(1))
        .position_centered()
        .resizable()
        .allow_highdpi()
        .build()
        .map_err(|e| format!("não abriu a janela: {e}"))?;
    let canvas = window
        .into_canvas()
        .accelerated()
        .present_vsync()
        .build()
        .map_err(|e| format!("não abriu o renderizador com VSync: {e}"))?;
    let info = canvas.info();
    let accelerated = sdl2::sys::SDL_RendererFlags::SDL_RENDERER_ACCELERATED as u32;
    if info.flags & accelerated == 0 {
        return Err("o visor exige um renderizador acelerado com VSync".into());
    }
    // SDL may report PRESENTVSYNC even when it only emulates it with a timer.
    // GL exposes the actual native swap interval, so verify that separately.
    if info.name.starts_with("opengl") {
        video
            .gl_set_swap_interval(sdl2::video::SwapInterval::VSync)
            .map_err(|e| format!("não ativou VSync nativo ({}): {e}", info.name))?;
        if video.gl_get_swap_interval() != sdl2::video::SwapInterval::VSync {
            return Err("o driver não confirmou o intervalo de VSync".into());
        }
    } else if !matches!(
        info.name,
        "direct3d" | "direct3d11" | "direct3d12" | "metal"
    ) {
        return Err(format!("VSync nativo não verificado para {}", info.name));
    }
    // SDL2 0.38 has no safe wrapper for this API. The canvas owns a valid
    // SDL_Renderer and this function runs on the SDL/main thread. On GL the
    // successful native request above also prevents SDL's timer fallback.
    if unsafe { sdl2::sys::SDL_RenderSetVSync(canvas.raw(), 1) } != 0 {
        return Err(format!("não ativou VSync: {}", sdl2::get_error()));
    }
    let flag = sdl2::sys::SDL_RendererFlags::SDL_RENDERER_PRESENTVSYNC as u32;
    if canvas.info().flags & flag == 0 {
        return Err("o renderizador não confirmou VSync".into());
    }
    Ok(canvas)
}

fn escreva_pixels(
    pixels: &[u32],
    bytes: &mut [u8],
    pitch: usize,
    w: u32,
    h: u32,
) -> Result<(), String> {
    let row = w as usize * 4;
    if pixels.len() != w as usize * h as usize || pitch < row || bytes.len() < pitch * h as usize {
        return Err("buffer de textura com tamanho inválido".into());
    }
    for y in 0..h as usize {
        let dst = &mut bytes[y * pitch..y * pitch + row];
        for (dest, &rgb) in dst
            .chunks_exact_mut(4)
            .zip(&pixels[y * w as usize..(y + 1) * w as usize])
        {
            dest.copy_from_slice(&(0xff000000 | rgb).to_ne_bytes());
        }
    }
    Ok(())
}

fn identidade(scancode: Option<Scancode>, keycode: Option<Keycode>) -> Option<String> {
    scancode
        .map(|s| format!("scan:{}", s as i32))
        .or_else(|| keycode.map(|k| format!("key:{}", i32::from(k))))
}

fn processe_evento(
    entrada: &mut EstadoEntrada,
    event: Event,
    window_size: (u32, u32),
    logical_size: (u32, u32),
    visible: &mut bool,
) -> bool {
    match event {
        Event::Quit { .. }
        | Event::Window {
            win_event: WindowEvent::Close,
            ..
        } => return false,
        Event::Window {
            win_event: WindowEvent::FocusLost,
            ..
        } => entrada.registre(EventoGraf {
            tipo: "desfoco".into(),
            ..EventoGraf::nada()
        }),
        Event::Window {
            win_event: WindowEvent::Minimized | WindowEvent::Hidden,
            ..
        } => *visible = false,
        Event::Window {
            win_event: WindowEvent::Restored | WindowEvent::Shown | WindowEvent::Exposed,
            ..
        } => *visible = true,
        Event::KeyDown {
            keycode: Some(key),
            scancode,
            repeat: false,
            ..
        } => {
            let nome = tecla_sdl(key);
            if !nome.is_empty() {
                entrada.registre(EventoGraf {
                    fisica: identidade(scancode, Some(key)).unwrap(),
                    ..EventoGraf::tecla(nome)
                });
            }
        }
        Event::KeyUp {
            keycode, scancode, ..
        } => {
            if let Some(fisica) = identidade(scancode, keycode) {
                entrada.registre(EventoGraf {
                    fisica,
                    ..EventoGraf::soltura("")
                });
            }
        }
        Event::MouseButtonDown {
            x, y, mouse_btn, ..
        }
        | Event::MouseButtonUp {
            x, y, mouse_btn, ..
        } => {
            let down = matches!(event, Event::MouseButtonDown { .. });
            let button = match mouse_btn {
                MouseButton::Left => "esquerdo",
                MouseButton::Right => "direito",
                MouseButton::Middle => "meio",
                _ => return true,
            };
            let lb = letterbox(
                window_size.0 as f64,
                window_size.1 as f64,
                logical_size.0 as f64,
                logical_size.1 as f64,
            );
            if let Some((x, y)) = lb.canvas_pixel(
                x as f64,
                y as f64,
                logical_size.0 as f64,
                logical_size.1 as f64,
            ) {
                let mut e = EventoGraf::clique(x, y, button);
                if !down {
                    e.tipo = "solte".into();
                }
                entrada.registre(e);
            }
        }
        _ => {}
    }
    true
}

fn publique_entrada(
    proto: &mut impl Write,
    id: u64,
    publica: &mut bool,
    entrada: &mut EstadoEntrada,
) -> Result<(), String> {
    let mut captura = entrada.capture();
    if !*publica || captura.eventos.is_empty() {
        return Ok(());
    }
    for e in &mut captura.eventos {
        e.janela = id;
    }
    if let Err(e) = write_msg(proto, &Msg::Evento { id, captura }) {
        if e.kind() == io::ErrorKind::BrokenPipe {
            // The program may finish before the window is closed. Keep its
            // last scene visible, but stop publishing to the departed VM.
            *publica = false;
        } else {
            return Err(format!("não publicou entrada: {e}"));
        }
    }
    Ok(())
}

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

fn tecla_sdl(k: Keycode) -> String {
    use sdl2::keyboard::Keycode as K;
    match k {
        K::A => "a".into(),
        K::B => "b".into(),
        K::C => "c".into(),
        K::D => "d".into(),
        K::E => "e".into(),
        K::F => "f".into(),
        K::G => "g".into(),
        K::H => "h".into(),
        K::I => "i".into(),
        K::J => "j".into(),
        K::K => "k".into(),
        K::L => "l".into(),
        K::M => "m".into(),
        K::N => "n".into(),
        K::O => "o".into(),
        K::P => "p".into(),
        K::Q => "q".into(),
        K::R => "r".into(),
        K::S => "s".into(),
        K::T => "t".into(),
        K::U => "u".into(),
        K::V => "v".into(),
        K::W => "w".into(),
        K::X => "x".into(),
        K::Y => "y".into(),
        K::Z => "z".into(),
        K::Num0 => "num_0".into(),
        K::Num1 => "num_1".into(),
        K::Num2 => "num_2".into(),
        K::Num3 => "num_3".into(),
        K::Num4 => "num_4".into(),
        K::Num5 => "num_5".into(),
        K::Num6 => "num_6".into(),
        K::Num7 => "num_7".into(),
        K::Num8 => "num_8".into(),
        K::Num9 => "num_9".into(),
        K::Return => "enter".into(),
        K::Escape => "escape".into(),
        K::Space => "espaco".into(),
        K::Tab => "tab".into(),
        K::Backspace => "backspace".into(),
        K::Up => "cima".into(),
        K::Down => "baixo".into(),
        K::Left => "esquerda".into(),
        K::Right => "direita".into(),
        K::LShift | K::RShift => "shift".into(),
        K::LCtrl | K::RCtrl => "control".into(),
        K::LAlt | K::RAlt => "alt".into(),
        K::LGui | K::RGui => "super".into(),
        K::Home => "inicio".into(),
        K::End => "fim".into(),
        K::PageUp => "pagina_acima".into(),
        K::PageDown => "pagina_abaixo".into(),
        K::Insert => "insert".into(),
        K::Delete => "delete".into(),
        K::Quote => "apostrofo".into(),
        K::Backquote => "crase".into(),
        K::Backslash => "barra_invertida".into(),
        K::Comma => "virgula".into(),
        K::Equals => "igual".into(),
        K::LeftBracket => "colchete_esquerdo".into(),
        K::RightBracket => "colchete_direito".into(),
        K::Minus => "menos".into(),
        K::Period => "ponto".into(),
        K::Semicolon => "ponto_e_virgula".into(),
        K::Slash => "barra".into(),
        K::CapsLock => "caps_lock".into(),
        K::NumLockClear => "num_lock".into(),
        K::ScrollLock => "scroll_lock".into(),
        K::Menu => "menu".into(),
        K::Pause => "pause".into(),
        K::Kp0 => "numerico_0".into(),
        K::Kp1 => "numerico_1".into(),
        K::Kp2 => "numerico_2".into(),
        K::Kp3 => "numerico_3".into(),
        K::Kp4 => "numerico_4".into(),
        K::Kp5 => "numerico_5".into(),
        K::Kp6 => "numerico_6".into(),
        K::Kp7 => "numerico_7".into(),
        K::Kp8 => "numerico_8".into(),
        K::Kp9 => "numerico_9".into(),
        K::KpPeriod => "numerico_ponto".into(),
        K::KpDivide => "numerico_barra".into(),
        K::KpMultiply => "numerico_asterisco".into(),
        K::KpMinus => "numerico_menos".into(),
        K::KpPlus => "numerico_mais".into(),
        K::KpEnter => "enter".into(),
        K::F1 => "f1".into(),
        K::F2 => "f2".into(),
        K::F3 => "f3".into(),
        K::F4 => "f4".into(),
        K::F5 => "f5".into(),
        K::F6 => "f6".into(),
        K::F7 => "f7".into(),
        K::F8 => "f8".into(),
        K::F9 => "f9".into(),
        K::F10 => "f10".into(),
        K::F11 => "f11".into(),
        K::F12 => "f12".into(),
        K::F13 => "f13".into(),
        K::F14 => "f14".into(),
        K::F15 => "f15".into(),
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sdl2::keyboard::Mod;

    fn key(keycode: Option<Keycode>, scancode: Scancode, down: bool, repeat: bool) -> Event {
        if down {
            Event::KeyDown {
                timestamp: 0,
                window_id: 1,
                keycode,
                scancode: Some(scancode),
                keymod: Mod::NOMOD,
                repeat,
            }
        } else {
            Event::KeyUp {
                timestamp: 0,
                window_id: 1,
                keycode,
                scancode: Some(scancode),
                keymod: Mod::NOMOD,
                repeat,
            }
        }
    }
    fn input(state: &mut EstadoEntrada, event: Event) {
        assert!(processe_evento(
            state,
            event,
            (800, 400),
            (160, 144),
            &mut true
        ));
    }

    #[test]
    fn publishes_only_changes_without_a_capture_request() {
        let mut state = EstadoEntrada::default();
        let mut bytes = Vec::new();
        let mut publica = true;
        publique_entrada(&mut bytes, 9, &mut publica, &mut state).unwrap();
        assert!(bytes.is_empty());
        state.registre(EventoGraf::tecla("direita"));
        publique_entrada(&mut bytes, 9, &mut publica, &mut state).unwrap();
        let len = bytes.len();
        publique_entrada(&mut bytes, 9, &mut publica, &mut state).unwrap();
        assert_eq!(bytes.len(), len);
        state.registre(EventoGraf::soltura("direita"));
        publique_entrada(&mut bytes, 9, &mut publica, &mut state).unwrap();
        let mut stream = bytes.as_slice();
        let Msg::Evento {
            captura: pressed, ..
        } = graf_proto::read_msg(&mut stream).unwrap()
        else {
            panic!()
        };
        let Msg::Evento {
            captura: released, ..
        } = graf_proto::read_msg(&mut stream).unwrap()
        else {
            panic!()
        };
        assert!(pressed.teclas.contains("direita"));
        assert!(released.teclas.is_empty());
        assert!(released.soltas.contains("direita"));
        assert!(stream.is_empty());
    }

    #[test]
    fn a_departed_vm_does_not_close_the_last_scene() {
        struct Closed;
        impl Write for Closed {
            fn write(&mut self, _: &[u8]) -> io::Result<usize> {
                Err(io::ErrorKind::BrokenPipe.into())
            }
            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }
        let mut state = EstadoEntrada::default();
        state.registre(EventoGraf::tecla("a"));
        let mut publica = true;
        publique_entrada(&mut Closed, 9, &mut publica, &mut state).unwrap();
        assert!(!publica);
    }

    #[test]
    fn ordered_taps_keep_modifiers_and_physical_release_independent_of_layout() {
        let mut state = EstadoEntrada::default();
        input(
            &mut state,
            key(Some(Keycode::LCtrl), Scancode::LCtrl, true, false),
        );
        input(&mut state, key(Some(Keycode::S), Scancode::S, true, false));
        input(&mut state, key(None, Scancode::S, false, false));
        input(&mut state, key(None, Scancode::LCtrl, false, false));
        let mut bytes = Vec::new();
        publique_entrada(&mut bytes, 42, &mut true, &mut state).unwrap();
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
    fn simultaneous_keys_and_ten_thousand_repeats_do_not_delay_release() {
        let mut state = EstadoEntrada::default();
        input(
            &mut state,
            key(Some(Keycode::Right), Scancode::Right, true, false),
        );
        input(
            &mut state,
            key(Some(Keycode::Space), Scancode::Space, true, false),
        );
        assert_eq!(state.capture().teclas.len(), 2);
        for _ in 0..10_000 {
            input(
                &mut state,
                key(Some(Keycode::Right), Scancode::Right, true, true),
            );
        }
        input(&mut state, key(None, Scancode::Right, false, false));
        let capture = state.capture();
        assert!(!capture.teclas.contains("direita"));
        assert!(capture.teclas.contains("espaco"));
        assert!(capture.soltas.contains("direita"));
        assert!(capture.apertadas.is_empty());
        assert_eq!(capture.eventos.len(), 1);
    }

    #[test]
    fn both_modifier_sides_and_focus_loss_are_preserved() {
        let mut state = EstadoEntrada::default();
        input(
            &mut state,
            key(Some(Keycode::LShift), Scancode::LShift, true, false),
        );
        input(
            &mut state,
            key(Some(Keycode::RShift), Scancode::RShift, true, false),
        );
        state.capture();
        input(&mut state, key(None, Scancode::LShift, false, false));
        let capture = state.capture();
        assert!(capture.teclas.contains("shift"));
        assert!(capture.soltas.is_empty());
        input(
            &mut state,
            Event::Window {
                timestamp: 0,
                window_id: 1,
                win_event: WindowEvent::FocusLost,
            },
        );
        let capture = state.capture();
        assert!(capture.teclas.is_empty());
        assert!(capture.soltas.contains("shift"));
        input(
            &mut state,
            key(Some(Keycode::Right), Scancode::Right, true, true),
        );
        assert!(state.capture().teclas.is_empty());
    }

    #[test]
    fn mouse_uses_logical_pixels_after_letterboxing_and_resize() {
        let mut state = EstadoEntrada::default();
        let mouse = |down, x, y| {
            if down {
                Event::MouseButtonDown {
                    timestamp: 0,
                    window_id: 1,
                    which: 0,
                    mouse_btn: MouseButton::Left,
                    clicks: 1,
                    x,
                    y,
                }
            } else {
                Event::MouseButtonUp {
                    timestamp: 0,
                    window_id: 1,
                    which: 0,
                    mouse_btn: MouseButton::Left,
                    clicks: 1,
                    x,
                    y,
                }
            }
        };
        input(&mut state, mouse(true, 0, 200)); // Outside canvas, inside the letterbox bar.
        input(&mut state, mouse(true, 178, 0));
        input(&mut state, mouse(false, 400, 200));
        let capture = state.capture();
        assert_eq!(capture.eventos.len(), 2);
        assert_eq!((capture.eventos[0].x, capture.eventos[0].y), (0, 0));
        assert_eq!((capture.eventos[1].x, capture.eventos[1].y), (80, 72));
        assert_eq!(capture.eventos[1].tipo, "solte");
    }

    #[test]
    fn visibility_and_close_work_without_presenting_a_frame() {
        let mut state = EstadoEntrada::default();
        let mut visible = true;
        let event = |win_event| Event::Window {
            timestamp: 0,
            window_id: 1,
            win_event,
        };
        assert!(processe_evento(
            &mut state,
            event(WindowEvent::Minimized),
            (800, 400),
            (160, 144),
            &mut visible
        ));
        assert!(!visible);
        assert!(processe_evento(
            &mut state,
            event(WindowEvent::Restored),
            (800, 400),
            (160, 144),
            &mut visible
        ));
        assert!(visible);
        assert!(!processe_evento(
            &mut state,
            event(WindowEvent::Close),
            (800, 400),
            (160, 144),
            &mut visible
        ));
    }

    #[test]
    fn texture_upload_has_correct_colors_opaque_alpha_and_row_pitch() {
        let mut bytes = vec![0xee; 24];
        escreva_pixels(
            &[0xff0000, 0x00ff00, 0x0000ff, 0xffffff],
            &mut bytes,
            12,
            2,
            2,
        )
        .unwrap();
        assert_eq!(&bytes[..4], &0xffff0000u32.to_ne_bytes());
        assert_eq!(&bytes[4..8], &0xff00ff00u32.to_ne_bytes());
        assert_eq!(&bytes[8..12], &[0xee; 4]);
        assert_eq!(&bytes[12..16], &0xff0000ffu32.to_ne_bytes());
        assert_eq!(&bytes[16..20], &0xffffffffu32.to_ne_bytes());
        assert!(escreva_pixels(&[0], &mut bytes, 12, 2, 2).is_err());
    }

    #[test]
    fn keyboard_names_remain_the_language_names() {
        for (key, name) in [
            (Keycode::Right, "direita"),
            (Keycode::Space, "espaco"),
            (Keycode::Num0, "num_0"),
            (Keycode::Kp0, "numerico_0"),
            (Keycode::F15, "f15"),
            (Keycode::RAlt, "alt"),
            (Keycode::LCtrl, "control"),
            (Keycode::RGui, "super"),
        ] {
            assert_eq!(tecla_sdl(key), name);
        }
    }
}
