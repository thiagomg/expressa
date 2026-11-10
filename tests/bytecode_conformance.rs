//! Compare backends through the same public API, including complete diagnostics.
use expressa::runtime::{
    ExecutionBackend, ExecutionOptions, ExecutionOutput, RuntimeError, run_with_options,
};

fn outcome(source: &str, backend: ExecutionBackend) -> Result<ExecutionOutput, RuntimeError> {
    run_with_options(
        source,
        "conformance.lep",
        "Ana\n",
        ExecutionOptions {
            backend,
            ..ExecutionOptions::default()
        },
    )
}
fn equivalent(source: &str) {
    match (
        outcome(source, ExecutionBackend::Ast),
        outcome(source, ExecutionBackend::Bytecode),
    ) {
        (Ok(ast), Ok(bytecode)) => assert_eq!(ast, bytecode, "{source}"),
        (Err(ast), Err(bytecode)) => {
            assert_eq!(
                (&ast.message, &ast.file, ast.span),
                (&bytecode.message, &bytecode.file, bytecode.span),
                "{source}"
            );
            let frames = |error: &RuntimeError| {
                error
                    .stack
                    .iter()
                    .map(|frame| (frame.name.clone(), frame.file.clone(), frame.span))
                    .collect::<Vec<_>>()
            };
            assert_eq!(frames(&ast), frames(&bytecode), "{source}");
        }
        (ast, bytecode) => panic!("{source}\nAST: {ast:?}\nBytecode: {bytecode:?}"),
    }
}

#[test]
fn expression_and_statement_value_rules_match() {
    for source in [
        "f = funcao() { 10 x = 2 }\nescreva(f())",
        "escreva({ 10 repita 2 vezes { 20 } })",
        "escreva(avalie(\"10\\nx = 2\"))",
        "escreva({ 10 se falso { 20 } })",
        "escreva(nao falso e (verdadeiro ou (1 / 0 == 1)))",
        "escreva([1, 2, 3][1..])\nescreva(\"olá\"[1])",
    ] {
        equivalent(source);
    }
}

#[test]
fn dynamic_eval_updates_existing_and_later_bindings_and_escaping_closures() {
    for source in [
        "f = funcao() { later }\navalie(\"later = 42\")\nescreva(f())",
        "x = 10\nf = funcao() { avalie(\"x = 2\") }\nescreva(f() se_falhar x)",
        "make = funcao(n) { avalie(\"x = n + 1\\nfuncao() { x }\") }\ng = make(4)\nescreva(g())",
        "x = 1\nf = avalie(\"funcao() { x }\")\nx = 9\nescreva(f())",
        "escreva(avalie(\"avalie(\\\"2 + 3\\\")\"))",
    ] {
        equivalent(source);
    }
}

#[test]
fn loop_control_and_error_handlers_unwind_the_same_scopes() {
    for source in [
        "n = 0\npara i de 1 ate 5 { { se i == 3 { continue } n += i } se_falhar 0 }\nescreva(n)",
        "f = funcao() { para i de 1 ate 5 { { retorne i } se_falhar 99 } 0 }\nescreva(f())",
        "n = 0\n{ para i de 1 ate 3 { n += 1 1 / 0 } } se_falhar { n += 10 }\nescreva(n)",
        "n = 0\npara i de 1 ate 3 { enquanto { pare verdadeiro } { n += 100 } n += 1 }\nescreva(n)",
        "f = funcao() { { pare } se_falhar 42 }\nescreva(f() se_falhar 7)",
        "f = funcao() { avalie(\"retorne 1\") }\nescreva(f() se_falhar 7)",
        "n = 0\npara x em \"olá\" { n += 1 }\npara x em conjunto([1, 2]) { n += x }\nescreva(n)",
    ] {
        equivalent(source);
    }
}

#[test]
fn error_messages_spans_and_stacks_match() {
    for source in [
        "x = missing",
        "escreva(1 + verdadeiro)",
        "f = funcao(x) { x }\nf()",
        "f = funcao() { 1 / 0 }\ng = funcao() { f() }\ng()",
        "f = funcao() { pare }\nf()",
        "f = funcao() { continue }\nf()",
        "avalie(\"1 / 0\")",
        "repita -1 vezes { 1 }",
        "para i de 1.5 ate 2 { 1 }",
        "para i de 1 ate 2.5 { 1 }",
        "para i em 42 { 1 }",
        "enquanto 1 { 1 }",
        "f = funcao(a, a) { a }",
        "escreva(999999999999999999999999999999999)",
        "escreva(verdadeiro e 3)",
        "escreva(1 ou falso)",
        "m = mapa()\nescreva(m:x)",
    ] {
        equivalent(source);
    }
}

#[test]
fn natives_callbacks_custom_formatting_and_io_match() {
    for source in [
        "escreva(leia(\"nome? \"))\nescreva_erro(\"err\")\nsaia(7)",
        "escreva(ordene([3, 1, 2], funcao(x) { -x }))",
        "m = mapa([:_como_texto -> funcao(x) { \"custom\" }])\nescreva(\"m=\" + m)",
        "importe \"matriz\"\nA = uns(2)\nescreva(A * A)\nescreva(A.xy(1, 0, 7))",
        "s = \"abc\"\ns[0] += { s[1] = \"X\" \"\" }\nescreva(s)",
    ] {
        equivalent(source);
    }
}

#[test]
fn recursion_uses_vm_frames_and_still_obeys_the_language_limit() {
    let source = "f = funcao(n) { se n == 0 { 0 } senao { f(n - 1) + 1 } }\nescreva(f(3000))";
    let worker = std::thread::Builder::new()
        .stack_size(256 * 1024)
        .spawn(move || outcome(source, ExecutionBackend::Bytecode))
        .unwrap();
    assert_eq!(worker.join().unwrap().unwrap().stdout, "3.000\n");
    let result = outcome(
        "f = funcao() { f() }\nescreva(f() se_falhar \"depth\")",
        ExecutionBackend::Bytecode,
    )
    .unwrap();
    assert_eq!(result.stdout, "depth\n");
}

#[test]
fn range_start_is_validated_before_evaluating_the_end() {
    equivalent(
        "m = mapa([:n -> 0])\n{ para i de 1.5 ate { m:n = 9 2 } { 0 } } se_falhar 0\nescreva(m:n)",
    );
}

#[test]
fn top_level_control_signals_have_identical_diagnostics() {
    for source in [
        "pare",
        "continue",
        "retorne 1",
        "{ pare } se_falhar 0",
        "avalie(\"pare\")",
        "f = funcao() { { pare } se_falhar 0 }\nf()",
    ] {
        equivalent(source);
    }
}

#[cfg(unix)]
#[test]
fn discarded_process_values_keep_the_ast_cleanup_timing() {
    use std::path::PathBuf;
    struct Temp(PathBuf);
    impl Drop for Temp {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    let temp = Temp(
        std::env::temp_dir().join(format!("expressa-bytecode-cleanup-{}", std::process::id())),
    );
    std::fs::create_dir_all(&temp.0).unwrap();
    for backend in [ExecutionBackend::Ast, ExecutionBackend::Bytecode] {
        for in_loop in [false, true] {
            let marker = temp.0.join(format!("{backend:?}-{in_loop}"));
            let command = format!(
                "p::linhas_de([\"sh\", \"-c\", \"sleep 0.05; touch \\\"$1\\\"\", \"sh\", {:?}])",
                marker.to_str().unwrap()
            );
            let body = format!("{command}\ndurma(0.2)");
            let body = if in_loop {
                format!("repita 1 vezes {{ {body} }}")
            } else {
                body
            };
            let source = format!(
                "p = importe \"processo\"\na = importe \"arquivo\"\n{body}\nescreva(a::existe({:?}))",
                marker.to_str().unwrap()
            );
            let output = outcome(&source, backend).unwrap();
            assert_eq!(
                output.stdout,
                if in_loop { "verdadeiro\n" } else { "falso\n" },
                "{backend:?}: {source}"
            );
        }
    }
}

#[test]
fn backend_selection_does_not_consume_script_arguments() {
    let file =
        std::env::temp_dir().join(format!("expressa-backend-args-{}.lep", std::process::id()));
    std::fs::write(&file, "escreva(argumentos)").unwrap();
    for prefix in [
        vec![],
        vec!["--backend", "ast"],
        vec!["--numeros", "en", "--backend", "bytecode"],
    ] {
        let output = std::process::Command::new(env!("CARGO_BIN_EXE_expressa"))
            .args(prefix)
            .arg(&file)
            .args(["--backend", "literal"])
            .env("EXPRESSA_BACKEND", "bytecode")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(output.stdout, b"[\"--backend\", \"literal\"]\n");
    }
    std::fs::remove_file(file).unwrap();
}
