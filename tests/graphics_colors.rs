#![cfg(feature = "graphics")]

use expressa::runtime::{
    ExecutionBackend, ExecutionOptions, compile_exb, load_exb, run_compiled_with_options,
    run_with_options,
};

fn check(source: &str, expected: &str) {
    let source = format!("tg = importe \"tela_grafica\"\n{source}");
    for backend in [ExecutionBackend::Bytecode, ExecutionBackend::Ast] {
        if backend.is_available() {
            let output = run_with_options(
                &source,
                "colors.lep",
                "",
                ExecutionOptions {
                    backend,
                    ..Default::default()
                },
            )
            .unwrap();
            assert_eq!(output.stdout, expected, "{backend:?}");
        }
    }
    let program = load_exb(&compile_exb(&source, "colors.lep").unwrap()).unwrap();
    let output =
        run_compiled_with_options(&program, "colors.lep", "", ExecutionOptions::default()).unwrap();
    assert_eq!(output.stdout, expected, "EXB");
}

#[test]
fn palette_is_discoverable_and_each_call_returns_an_independent_list() {
    check(
        r#"
cores = tg::cores()
escreva(cores == [:preto, :vermelho, :verde, :amarelo, :azul, :magenta, :ciano, :branco, :normal, :transparente])
cores[0] = "outra"
escreva(tg::cores()[0])
g = tg::nova(1, 1)
para cor em tg::cores() {
    g.tg::fundo(cor)
    garanta(g.tg::pixel(0, 0) == cor)
}
escreva(verdadeiro)
"#,
        "verdadeiro\npreto\nverdadeiro\n",
    );
}

#[test]
fn rgb_is_canonical_and_works_as_pen_and_background() {
    check(
        r##"
escreva(tg::rgb(0, 0, 0), tg::rgb(255, 255, 255), tg::rgb(255, 128, 0))
g = tg::nova(2, 2)
g.tg::fundo(tg::rgb(1, 2, 3))
g.tg::cor(tg::rgb(255, 128, 0))
g.tg::ponto(0, 0)
escreva(g.tg::pixel(0, 0))
escreva(g.tg::pixel(1, 1))
g.tg::cor("#aBcDeF")
g.tg::linha(0, 1, 1, 1)
escreva(g.tg::pixel(1, 1))
g.tg::cor(:azul)
g.tg::ponto(1, 0)
escreva(g.tg::pixel(1, 0))
"##,
        "#000000 #FFFFFF #FF8000\n#FF8000\n#010203\n#ABCDEF\nazul\n",
    );
}

#[test]
fn invalid_components_formats_and_arities_fail() {
    for expression in [
        "tg::rgb(-1, 0, 0)",
        "tg::rgb(0, 256, 0)",
        "tg::rgb(0, 0, 0.5)",
        "tg::rgb(0.0000000000000000000000000001, 0, 0)",
        "tg::rgb(0, 0, 99999999999999999999999999)",
        "tg::rgb(\"255\", 0, 0)",
        "tg::rgb(verdadeiro, 0, 0)",
        "tg::rgb(0, 0)",
        "tg::rgb(0, 0, 0, 0)",
        "tg::cores(1)",
        "g.tg::cor(\"#FFF\")",
        "g.tg::cor(\"#GG0000\")",
        "g.tg::fundo(\"#FF000080\")",
        "g.tg::fundo([255, 0, 0])",
    ] {
        check(
            &format!(
                "g = tg::nova(1, 1)\nr = {{ {expression} }} se_falhar {{ :erro }}\nescreva(r == :erro)"
            ),
            "verdadeiro\n",
        );
    }
}
