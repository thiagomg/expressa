use expressa::runtime::{
    ExecutionBackend, ExecutionOptions, NumeroAritmetica, NumeroLocale, compile_exb,
    run_exb_with_options, run_with_options,
};

fn options(backend: ExecutionBackend) -> ExecutionOptions {
    ExecutionOptions {
        backend,
        numero_locale: NumeroLocale::PtBr,
        numero_aritmetica: NumeroAritmetica::Decimal,
        ..Default::default()
    }
}

#[test]
fn numeric_options_are_independent_in_ast_bytecode_and_exb() {
    let source = r#"
define_opcao(:tipo_numero, :float)
define_opcao(:formato_numero, :en)
escreva(0.1 + 0.2 == 0.3)
escreva(numero("1,500.5"))
define_opcao(:tipo_numero, :decimal)
escreva(0.1 + 0.2)
define_opção(:formato_numero, :pt)
escreva(0.1 + 0.2)
escreva(tipo(define_opcao(:fechar_janelas, verdadeiro)))
"#;
    for backend in [ExecutionBackend::Ast, ExecutionBackend::Bytecode]
        .into_iter()
        .filter(|b| b.is_available())
    {
        let out = run_with_options(source, "opcoes.lep", "", options(backend)).unwrap();
        assert_eq!(out.stdout, "falso\n1,500.5\n0.3\n0,3\nnada\n");
    }
    let bytes = compile_exb(source, "opcoes.lep").unwrap();
    let out = run_exb_with_options(
        &bytes,
        "opcoes.lepc",
        "",
        options(ExecutionBackend::Bytecode),
    )
    .unwrap();
    assert_eq!(out.stdout, "falso\n1,500.5\n0.3\n0,3\nnada\n");
}

#[test]
fn invalid_options_do_not_modify_the_previous_configuration() {
    let source = r#"
define_opcao(:tipo_numero, :decimal)
define_opcao(:tipo_numero, :en) se_falhar { nada }
escreva(0.1 + 0.2 == 0.3)
define_opcao(:formato_numero, :pt)
define_opcao(:formato_numero, :float) se_falhar { nada }
escreva(numero("1,5"))
"#;
    for backend in [ExecutionBackend::Ast, ExecutionBackend::Bytecode]
        .into_iter()
        .filter(|b| b.is_available())
    {
        let out = run_with_options(source, "opcoes.lep", "", options(backend)).unwrap();
        assert_eq!(out.stdout, "verdadeiro\n1,5\n");
        for (bad, message) in [
            ("define_opcao()", "2 argumento"),
            (
                "define_opcao(:desconhecida, verdadeiro)",
                "opção desconhecida",
            ),
            (
                "define_opcao(:tipo_numero, :flutuante)",
                ":decimal ou :float",
            ),
            ("define_opcao(:formato_numero, :br)", ":pt ou :en"),
            ("define_opcao(:fechar_janelas, :sim)", "verdadeiro ou falso"),
        ] {
            let error = run_with_options(bad, "opcoes.lep", "", options(backend)).unwrap_err();
            assert!(error.message.contains(message), "{backend:?}: {error}");
        }
        assert!(run_with_options("formato(\"pt\")", "opcoes.lep", "", options(backend)).is_err());
    }
}
