#![cfg(feature = "graphics")]

use expressa::runtime::{
    ExecutionBackend, ExecutionOptions, compile_exb, load_exb, run_compiled_with_options,
    run_with_options,
};
use std::sync::atomic::{AtomicU64, Ordering};

// Exercise the language API and PNG composition in AST, bytecode and loaded EXB.
fn render(draw: &str) -> Vec<u8> {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let path = std::env::temp_dir().join(format!(
        "expressa-image-axis-{}-{}.png",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let source = format!(
        r#"
tg = importe "tela_grafica"
s = tg::nova(3, 2)
s.tg::cor(tg::rgb(255, 0, 0))
s.tg::ponto(0, 0)
s.tg::cor(tg::rgb(0, 255, 0))
s.tg::ponto(1, 0)
s.tg::cor(tg::rgb(0, 0, 255))
s.tg::ponto(2, 0)
s.tg::cor(tg::rgb(255, 255, 0))
s.tg::ponto(0, 1)
s.tg::cor(tg::rgb(255, 0, 255))
s.tg::ponto(1, 1)
s.tg::cor(tg::rgb(0, 255, 255))
s.tg::ponto(2, 1)
figura = s.tg::como_imagem()
g = tg::nova(12, 12)
{draw}
g.tg::salve("{}")
"#,
        path.display()
    );
    let mut expected = None;
    for backend in [ExecutionBackend::Bytecode, ExecutionBackend::Ast] {
        if !backend.is_available() {
            continue;
        }
        run_with_options(
            &source,
            "axis.lep",
            "",
            ExecutionOptions {
                backend,
                ..Default::default()
            },
        )
        .unwrap();
        let bytes = std::fs::read(&path).unwrap();
        if let Some(before) = &expected {
            assert_eq!(&bytes, before);
        }
        expected = Some(bytes);
    }
    let program = load_exb(&compile_exb(&source, "axis.lep").unwrap()).unwrap();
    run_compiled_with_options(&program, "axis.lep", "", ExecutionOptions::default()).unwrap();
    let bytes = std::fs::read(&path).unwrap();
    assert_eq!(Some(bytes.clone()), expected);
    std::fs::remove_file(path).unwrap();
    let mut reader = png::Decoder::new(std::io::Cursor::new(bytes))
        .read_info()
        .unwrap();
    let mut pixels = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut pixels).unwrap();
    assert_eq!(info.color_type, png::ColorType::Rgba);
    pixels.truncate(info.buffer_size());
    pixels
}

fn pixel(pixels: &[u8], x: usize, y: usize) -> &[u8] {
    &pixels[(y * 12 + x) * 4..(y * 12 + x + 1) * 4]
}

#[test]
fn default_origin_places_all_pixels_at_the_top_left_and_clips() {
    let pixels = render("g.tg::imagem(figura, 0, 0)");
    assert_eq!(pixel(&pixels, 0, 0), [255, 0, 0, 255]);
    assert_eq!(pixel(&pixels, 2, 0), [0, 0, 255, 255]);
    assert_eq!(pixel(&pixels, 0, 1), [255, 255, 0, 255]);
    assert_eq!(pixel(&pixels, 2, 1), [0, 255, 255, 255]);
    assert_eq!(pixel(&pixels, 3, 0), [255; 4]);
    let clipped = render("g.tg::imagem(figura, -1, 0, mapa())");
    assert_eq!(pixel(&clipped, 0, 0), [0, 255, 0, 255]);
    assert_eq!(pixel(&clipped, 1, 1), [0, 255, 255, 255]);
}

#[test]
fn quarter_turns_rotate_around_the_chosen_local_point() {
    for (angle, red, blue, yellow) in [
        (0, (4, 4), (6, 4), (4, 5)),
        (90, (4, 4), (4, 6), (3, 4)),
        (180, (4, 4), (2, 4), (4, 3)),
        (270, (4, 4), (4, 2), (5, 4)),
        (450, (4, 4), (4, 6), (3, 4)),
    ] {
        let pixels = render(&format!(
            "g.tg::imagem(figura, 4, 4, mapa([:angulo -> {angle}]))"
        ));
        assert_eq!(pixel(&pixels, red.0, red.1), [255, 0, 0, 255], "{angle}");
        assert_eq!(pixel(&pixels, blue.0, blue.1), [0, 0, 255, 255], "{angle}");
        assert_eq!(
            pixel(&pixels, yellow.0, yellow.1),
            [255, 255, 0, 255],
            "{angle}"
        );
    }
    let pixels = render(
        "g.tg::imagem(figura, 4, 4, mapa([:eixo -> mapa([:x -> 1, :y -> 0]), :angulo -> 90]))",
    );
    assert_eq!(pixel(&pixels, 4, 4), [0, 255, 0, 255]);
    assert_eq!(pixel(&pixels, 4, 3), [255, 0, 0, 255]);
    assert_eq!(pixel(&pixels, 3, 5), [0, 255, 255, 255]);
}

#[test]
fn fractional_external_axes_and_copied_options_work() {
    let pixels = render(
        r#"
eixo = mapa([:x -> 1.25, :y -> 0.25])
opcoes = mapa([:eixo -> eixo])
g.tg::imagem(figura, 4, 4, opcoes)
eixo:x = 10
opcoes:angulo = 90
"#,
    );
    assert_eq!(pixel(&pixels, 3, 4), [255, 0, 0, 255]);
    assert_eq!(pixel(&pixels, 5, 5), [0, 255, 255, 255]);
    let pixels = render(
        "g.tg::imagem(figura, 4, 4, mapa([:eixo -> mapa([:x -> -2, :y -> 0]), :angulo -> 90]))",
    );
    assert_eq!(pixel(&pixels, 4, 6), [255, 0, 0, 255]);
    assert_eq!(pixel(&pixels, 4, 8), [0, 0, 255, 255]);
}

#[test]
fn arbitrary_angles_keep_the_pivot_fixed_and_distant_axes_clip() {
    let pixels = render("g.tg::imagem(figura, 4, 4, mapa([:angulo -> 45]))");
    assert_eq!(pixel(&pixels, 4, 4), [255, 0, 0, 255]);
    assert_eq!(pixel(&pixels, 5, 5), [0, 255, 0, 255]);
    let pixels = render(
        "g.tg::imagem(figura, 4, 4, mapa([:eixo -> mapa([:x -> 1000000000000, :y -> 0]), :angulo -> 45]))",
    );
    assert!(pixels.iter().all(|&channel| channel == 255));
}

#[test]
fn invalid_options_are_rejected_before_recording_a_command() {
    for options in [
        "90",
        "verdadeiro",
        "[]",
        "mapa([:centrada -> verdadeiro])",
        "mapa([:angulo -> :azul])",
        "mapa([:eixo -> [0, 0]])",
        "mapa([:eixo -> mapa([:x -> 0])])",
        "mapa([:eixo -> mapa([:y -> 0])])",
        "mapa([:eixo -> mapa([:x -> 0, :y -> 0, :z -> 0])])",
        "mapa([:eixo -> mapa([:x -> nada, :y -> 0])])",
    ] {
        let source = format!(
            "tg = importe \"tela_grafica\"\ns = tg::nova(1, 1)\ng = tg::nova(2, 2)\nr = g.tg::imagem(s, 0, 0, {options}) se_falhar {{ :erro }}\nescreva(r == :erro, tamanho(g:ops) == 0)"
        );
        for backend in [ExecutionBackend::Bytecode, ExecutionBackend::Ast] {
            if backend.is_available() {
                let output = run_with_options(
                    &source,
                    "axis.lep",
                    "",
                    ExecutionOptions {
                        backend,
                        ..Default::default()
                    },
                )
                .unwrap();
                assert_eq!(output.stdout, "verdadeiro verdadeiro\n", "{options}");
            }
        }
        let program = load_exb(&compile_exb(&source, "axis.lep").unwrap()).unwrap();
        assert_eq!(
            run_compiled_with_options(&program, "axis.lep", "", ExecutionOptions::default())
                .unwrap()
                .stdout,
            "verdadeiro verdadeiro\n"
        );
    }
}
