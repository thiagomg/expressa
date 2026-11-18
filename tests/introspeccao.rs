use expressa::runtime::{
    ExecutionBackend, ExecutionOptions, compile_exb, load_exb, run_compiled_with_options,
    run_with_options,
};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "expressa-discovery-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn backends() -> impl Iterator<Item = ExecutionBackend> {
    [ExecutionBackend::Ast, ExecutionBackend::Bytecode]
        .into_iter()
        .filter(|b| b.is_available())
}
fn options(backend: ExecutionBackend) -> ExecutionOptions {
    ExecutionOptions {
        backend,
        ..Default::default()
    }
}
fn check(source: &str, file: &str, expected: &str) {
    for backend in backends() {
        let out = run_with_options(source, file, "", options(backend))
            .unwrap_or_else(|e| panic!("{backend:?}: {e}"));
        assert_eq!(out.stdout, expected, "{backend:?}");
        assert!(out.stderr.is_empty());
    }
    let compiled = load_exb(&compile_exb(source, file).unwrap()).unwrap();
    let out = run_compiled_with_options(&compiled, file, "", options(ExecutionBackend::Bytecode))
        .unwrap();
    assert_eq!(out.stdout, expected, "EXB");
}

#[test]
fn scope_data_has_full_docs_argument_lists_and_only_public_functions() {
    check(
        r#"
/// Soma dois valores.
///
/// Exemplo: soma(1, 2)
soma = funcao(a, b) { a + b }
_sem_doc = funcao() { 1 }
sem_doc = funcao() { 2 }
numero_guardado = 3
fs = funções("", :dados)
garanta(fs == funcoes("", :dados))
names = []
para f em fs {
    names += [f:nome]
    garanta(tipo(f:nome) == "texto")
    garanta(tipo(f:args) == "lista")
    garanta(tipo(f:doc) == "texto")
    garanta(tamanho(f) == 3)
    se f:nome == "soma" {
        garanta(f:args == [:a, :b])
        garanta(f:doc == "Soma dois valores.\n\nExemplo: soma(1, 2)")
    }
    se f:nome == "sem_doc" { garanta(f:args == [] e f:doc == "") }
}
garanta(names == ordene(names))
garanta(names contem :soma e names contem :sem_doc)
garanta(nao (names contem :_sem_doc))
garanta(nao (names contem :numero_guardado))
garanta(nao (names contem :catalogo))
garanta(nao (names contem :catálogo))
// Local parameters shadow functions of the same name.
local = funcao(soma) {
    para f em funcoes("", :dados) { garanta(f:nome != "soma") }
}
local(1)
"#,
        "scope.lep",
        "",
    );
}

#[test]
fn file_modules_are_discovered_once_with_functions_and_can_be_queried_by_name_or_alias() {
    let tmp = Temp::new();
    std::fs::write(tmp.0.join("helper.lep"), "/// Soma dois valores.\n///\n/// Exemplo: soma(1, 2)\nsoma = funcao(a, b) { a + b }\n_oculta = funcao() { 1 }\nvalor = 42\n").unwrap();
    let file = tmp.0.join("main.lep");
    check(
        r#"
a = importe "./helper.lep"
b = importe "./helper.lep"
importe "./helper.lep"
garanta(funcoes(a, :dados) == funcoes(b, :dados))
garanta(funcoes(a, :dados) == funcoes("a", :dados))
garanta(funcoes(a, :dados) == funcoes("./helper.lep", :dados))
garanta(tamanho(funcoes(a, :dados)) == 1)
ms = módulos(:dados)
garanta(ms == modulos(:dados))
count = 0
names = []
para m em ms {
    names += [m:nome]
    garanta(m:funcoes == funcoes(m:nome, :dados))
    se m:nome == "./helper.lep" {
        count += 1
        garanta(m:funcoes == funcoes(a, :dados))
    }
}
garanta(names == ordene(names))
garanta(count == 1)
garanta(names contem :nucleo e names contem :arquivo e names contem :sistema)
// Returned data is a snapshot and contains no callable values.
fs = funcoes(a, :dados)
fs[0]:doc = "alterada"
garanta(funcoes(a, :dados)[0]:doc != "alterada")
garanta(tipo(funcoes(a)) == "nada")
"#,
        file.to_str().unwrap(),
        "soma(a, b)\n    Soma dois valores.\n    \n    Exemplo: soma(1, 2)\n",
    );
}

#[test]
fn native_metadata_uses_the_target_module_and_excludes_constants() {
    check(
        r#"
garanta(funcoes("nucleo", :dados) == funcoes("núcleo", :dados))
f = funcoes("arquivo", :dados)[0]
garanta(f:nome == "adicione_arquivo")
garanta(f:args == [:caminho, :linhas])
garanta(f:doc contem "Exemplo:")
para f em funcoes("mat", :dados) { garanta(f:nome != "pi") }
"#,
        "native.lep",
        "",
    );
    #[cfg(feature = "graphics")]
    check(
        r#"
c = importe "cartesiano"
garanta(funcoes(c, :dados) == funcoes("cartesiano", :dados))
para f em funcoes(c, :dados) {
    se f:nome == "adicione" { garanta(f:args == [:documento, :grafico]) }
}
para f em funcoes("crono", :dados) {
    se f:nome == "adicione" { garanta(f:args == [:data, :n, :unidade]) }
}
para f em funcoes("tela_grafica", :dados) {
    se f:nome == "execute" { garanta(f:args == [:l, :config]) }
}
"#,
        "native.lep",
        "",
    );
}

#[test]
fn pretty_prints_return_nada_and_include_docs_while_data_calls_are_silent() {
    for backend in backends() {
        let out = run_with_options("modulos()\n", "pretty.lep", "", options(backend)).unwrap();
        assert!(out.stdout.lines().any(|s| s == "nucleo"));
        assert!(out.stdout.lines().any(|s| s == "sistema"));
        let out = run_with_options(
            "garanta(tipo(funcoes()) == \"nada\")",
            "pretty.lep",
            "",
            options(backend),
        )
        .unwrap();
        assert!(out.stdout.contains("escreva(valor, ...)"));
        assert!(out.stdout.contains("Imprime os argumentos"));
        assert!(out.stdout.contains("Exemplo:"));
    }
}

#[test]
fn invalid_targets_modes_and_removed_catalog_functions_are_errors() {
    for backend in backends() {
        for source in [
            "modulos(1)",
            "modulos(:outro)",
            "modulos(:dados, :dados)",
            "funcoes(:dados)",
            "funcoes(1)",
            "funcoes(mapa())",
            "funcoes(\"inexistente\")",
            "funcoes(\"\", 1)",
            "funcoes(\"\", :outro)",
            "funcoes(\"\", :dados, 1)",
            "catalogo()",
            "catálogo()",
        ] {
            let error = run_with_options(source, "errors.lep", "", options(backend)).unwrap_err();
            assert!(!error.message.is_empty(), "{source}");
        }
        let out = run_with_options("catalogo = 1\ncatálogo = 2\nls = 3\ngaranta(catalogo + catálogo + ls == 6)\ngaranta(ajuda(\"funções\") contem \"funcoes\")", "errors.lep", "", options(backend)).unwrap();
        assert!(out.stdout.is_empty());
    }
}
