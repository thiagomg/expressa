#![cfg(feature = "ast")]
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
    let expected = outcome(source, ExecutionBackend::Ast);
    let mut actual = vec![outcome(source, ExecutionBackend::Bytecode)];
    if expressa::parser::parse(source).is_ok() {
        let bytes = expressa::runtime::compile_exb(source, "conformance.lep").unwrap();
        actual.push(expressa::runtime::run_exb_with_options(
            &bytes,
            "conformance.lep",
            "Ana\n",
            ExecutionOptions {
                backend: ExecutionBackend::Bytecode,
                ..ExecutionOptions::default()
            },
        ));
    }
    for actual in actual {
        match (&expected, &actual) {
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
                assert_eq!(frames(ast), frames(bytecode), "{source}");
            }
            _ => panic!("{source}\nAST: {expected:?}\nBytecode: {actual:?}"),
        }
    }
}

#[test]
fn indexed_parameters_share_storage_with_eval_and_closures() {
    for (source, expected) in [
        (
            "f = funcao(a) { a += 1 avalie(\"a = a + 2\") a }\nescreva(f(3))",
            "6\n",
        ),
        ("f = funcao(a) { { a = 8 } a }\nescreva(f(1))", "8\n"),
        (
            "f = funcao(a) { para a de 1 ate 3 { escreva(a) } a }\nescreva(f(9))",
            "1\n2\n3\n9\n",
        ),
        (
            "f = funcao(a) { g = funcao() { a } avalie(\"a = 7\") g }\ng = f(1)\nescreva(g())",
            "7\n",
        ),
        (
            "f = funcao(a) { g = funcao() { a = 7 } g() se_falhar 0 a }\nescreva(f(1))",
            "1\n",
        ),
        (
            "f = funcao(a) { g = funcao(a) { a += 1 a } a + g(2) }\nescreva(f(10))",
            "13\n",
        ),
        (
            "importe \"arquivo\"\nf = funcao(a) { avalie(\"late = 5\") a + late }\nescreva(f(2))",
            "7\n",
        ),
        (
            "escreva(ordene([3, 1, 2], funcao(x) { -x }))",
            "[3, 2, 1]\n",
        ),
        (
            "f = funcao(a) { a[0] += 1 a }\nxs = [1]\nescreva(f(xs))\nescreva(xs)",
            "[2]\n[2]\n",
        ),
    ] {
        for backend in [ExecutionBackend::Ast, ExecutionBackend::Bytecode] {
            assert_eq!(
                outcome(source, backend).unwrap().stdout,
                expected,
                "{backend:?}: {source}"
            );
        }
    }
}

#[test]
fn list_after_assignment_is_a_separate_block_result() {
    let source = r#"
serialize_data = funcao(dt) { dt }
serialize = funcao(tarefa) {
    c = se tarefa:completo { "S" } senao { "N" }
    dt = serialize_data(tarefa:data)
    [c, dt, tarefa:desc]
}
tarefa = mapa([:completo -> verdadeiro, :data -> "2026-11-09", :desc -> "Dormir"])
escreva(serialize(tarefa))
escreva(serialize(tarefa)[
    0
])
"#;
    for backend in [ExecutionBackend::Ast, ExecutionBackend::Bytecode] {
        assert_eq!(
            outcome(source, backend).unwrap().stdout,
            "[\"S\", \"2026-11-09\", \"Dormir\"]\nS\n",
            "{backend:?}"
        );
    }
}

#[test]
fn stack_arguments_preserve_order_results_and_error_recovery() {
    for source in [
        "f = funcao() { 7 }\nescreva(10 + f())",
        "f = funcao(a, b, c) { [a, b, c] }\nescreva(f(1, 2, 3))",
        "f = funcao(a, b) { a * 10 + b }\nescreva(100 + f(f(1, 2), f(3, 4)))",
        "make = funcao(n) { funcao(a, b) { n + a + b } }\nescreva(make(10)(2, 3))",
        "f = funcao(a, b) { [a, b] }\nescreva(f({ escreva(1) 10 }, { escreva(2) 20 }))",
        "f = funcao(a, b) { a + b }\nescreva(f({ escreva(1) 1 / 0 }, { escreva(2) 2 }) se_falhar 9)\nescreva(f(3, 4))",
        "f = funcao(a) { a }\nescreva(f({ escreva(1) 1 }, { escreva(2) 2 }) se_falhar 9)\nescreva(f(3))",
        "f = funcao(a) { a }\nescreva(f() se_falhar 9)\nescreva(f(3))",
        "f = funcao(a) { 1 / 0 }\nescreva(10 + (f([1, 2]) se_falhar 3))",
        "f = funcao(a) { a }\nescreva(f(avalie(\"2 + 3\")))\nescreva(avalie(\"f(7)\"))",
        "escreva(42(1, 2) se_falhar 9)",
        "f = funcao(a, b) { a }\nf(1)",
        "f = funcao(a) { a }\nf(1, 2)",
        "f = funcao(a) { a }\nxs = [1]\nescreva(f(xs) == xs)",
    ] {
        equivalent(source);
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
    let result = outcome(
        "f = funcao(n) { f(n) }\nescreva(f(1) se_falhar \"depth\")",
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
