//! Explicit absence has the same value in source execution and compiled files.
use expressa::runtime::{
    ExecutionBackend, ExecutionOptions, compile_exb, load_exb, run_compiled_with_options,
    run_with_options,
};

fn options(backend: ExecutionBackend) -> ExecutionOptions {
    ExecutionOptions {
        backend,
        ..Default::default()
    }
}
fn backends() -> impl Iterator<Item = ExecutionBackend> {
    [ExecutionBackend::Ast, ExecutionBackend::Bytecode]
        .into_iter()
        .filter(|b| b.is_available())
}
fn check(source: &str, expected: &str) {
    for backend in backends() {
        let out = run_with_options(source, "nada.lep", "", options(backend))
            .unwrap_or_else(|e| panic!("{backend:?}: {e}"));
        assert_eq!(out.stdout, expected, "{backend:?}");
        assert!(out.stderr.is_empty());
    }
    let program = load_exb(&compile_exb(source, "nada.lep").unwrap()).unwrap();
    let out = run_compiled_with_options(
        &program,
        "nada.lep",
        "",
        options(ExecutionBackend::Bytecode),
    )
    .unwrap();
    assert_eq!(out.stdout, expected, "EXB");
}

#[test]
fn nada_can_be_assigned_compared_and_returned_explicitly_or_implicitly() {
    check(
        r#"
x = nada
garanta(tipo(x) == "nada")
garanta(x == nada)
garanta(nada != falso e nada != 0 e nada != "")
f = funcao() { retorne nada
    falhe("não deveria executar")
}
g = funcao() { retorne }
h = funcao() { local_unico = 1 }
garanta(f() == nada e g() == nada e h() == nada)
identidade = funcao(valor) { valor }
garanta(identidade(nada) == nada)
escreva(nada, tipo(nada), f() == g())
"#,
        "nada nada verdadeiro\n",
    );
}

#[test]
fn nada_works_in_collections_conditions_fallbacks_and_cloned_functions() {
    check(
        r#"
xs = [nada, 1]
m = mapa([:ausente -> nada])
garanta(xs[0] == nada e m:ausente == nada)
garanta(clone(xs) == xs e clone(nada) == nada)
f = clone(funcao() { nada })
garanta(f() == nada)
x = se verdadeiro { nada } senao { 2 }
y = se falso { 1 } senao { nada }
z = falhe("erro") se_falhar { nada }
garanta(x == nada e y == nada e z == nada)
// Keyword prefixes remain ordinary identifiers, and quoted text remains text.
nada2 = 2
nada_mais = 3
garanta(nada2 + nada_mais == 5 e tipo("nada") == "texto")
escreva(xs, m:ausente)
"#,
        "[nada, 1] nada\n",
    );
}

#[test]
fn nada_selects_the_current_scope_for_function_discovery() {
    check(
        r#"
f = funcao() { nada }
a = funcoes(nada, :dados)
garanta(a == funcoes("", :dados))
garanta(a == funções(nada, :dados))
found = falso
para f em a { se f:nome == "f" { found = verdadeiro } }
garanta(found)
"#,
        "",
    );
}

#[test]
fn nada_is_reserved_and_does_not_silently_become_a_boolean_or_number() {
    for backend in backends() {
        for source in [
            "nada = 1",
            "nada += 1",
            "f = funcao(nada) { 1 }",
            "para nada em [1] { escreva(1) }",
            "se nada { escreva(1) }",
            "nada + 1",
            "nada()",
        ] {
            assert!(
                run_with_options(source, "nada.lep", "", options(backend)).is_err(),
                "{backend:?}: {source}"
            );
        }
    }
}
