//! Regressions that both the AST evaluator and its future bytecode replacement must pass.
use expressa::runtime::{Numero, run_to_string};

fn run(source: &str) -> String {
    run_to_string(source, "regression.lep").unwrap()
}

#[test]
fn inclusive_ranges_at_integer_limits_terminate() {
    assert_eq!(
        run(
            "n = 0\npara i em 9223372036854775805..=9223372036854775806 { n += 1 continue }\nescreva(n)"
        ),
        "2\n"
    );
    assert_eq!(
        run(
            "n = 0\npara i em -9223372036854775808..=-9223372036854775808 { n += 1 }\npara i em 2..=1 { n += 100 }\nescreva(n)"
        ),
        "1\n"
    );
}

#[test]
fn float_conversion_is_explicit_and_fallible() {
    for n in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 1e100] {
        assert!(Numero::try_from(n).is_err());
    }
    assert_eq!(Numero::try_from(0.5).unwrap().to_plain_string(), "0.5");
    assert_eq!(
        run(
            "importe \"mat\"\nescreva(exp(100) se_falhar \"range\")\nescreva(potencia(10, 100) se_falhar \"range\")"
        ),
        "range\nrange\n"
    );
}

#[test]
fn rounding_preserves_decimal_values_and_halfway_rule() {
    assert_eq!(
        run(r#"
importe "mat"
escreva(piso(9007199254740993) == 9007199254740993)
escreva(teto(9007199254740993) == 9007199254740993)
escreva(arredonde(9007199254740993) == 9007199254740993)
escreva(piso(1.9999999999999999999999999999))
escreva(teto(-1.9999999999999999999999999999))
escreva(arredonde(2.5))
escreva(arredonde(-2.5))
"#),
        "verdadeiro\nverdadeiro\nverdadeiro\n1\n-1\n3\n-3\n"
    );
}

#[test]
fn compound_target_is_evaluated_once_in_read_order() {
    assert_eq!(
        run(r#"
xs = [10, 20]
m = mapa([:n -> 0, :log -> ""])
receiver = funcao() { m:log += "R" xs }
idx = funcao() { m:log += "I" m:n += 1 m:n - 1 }
rhs = funcao() { m:log += "V" 1 }
receiver()[idx()] += rhs()
escreva(xs)
escreva(m:n)
escreva(m:log)
"#),
        "[11, 20]\n1\nRIV\n"
    );
}

#[test]
fn ordinary_assignment_retains_rhs_first_and_index_before_receiver() {
    assert_eq!(
        run(r#"
xs = [10]
m = mapa([:log -> ""])
receiver = funcao() { m:log += "R" xs }
idx = funcao() { m:log += "I" 0 }
rhs = funcao() { m:log += "V" 9 }
receiver()[idx()] = rhs()
escreva(xs)
escreva(m:log)
"#),
        "[9]\nVIR\n"
    );
}

#[test]
fn compound_destination_survives_rhs_rebinding() {
    assert_eq!(
        run("xs = [10]\nold = xs\nxs[0] += { xs = [99] 1 }\nescreva(old)\nescreva(xs)"),
        "[11]\n[99]\n"
    );
}

#[test]
fn nested_text_writeback_keeps_the_original_slot() {
    assert_eq!(
        run(r#"
xs = ["ab", "cd"]
m = mapa([:n -> 0])
idx = funcao() { m:n += 1 m:n - 1 }
xs[idx()][1] = "á"
escreva(xs)
escreva(m:n)
"#),
        "[\"aá\", \"cd\"]\n1\n"
    );
}

#[test]
fn nested_text_map_receiver_is_evaluated_once() {
    assert_eq!(
        run(r#"
m = mapa([:text -> "ab", :n -> 0])
receiver = funcao() { m:n += 1 m }
receiver():text[0] = "X"
escreva(m:text)
escreva(m:n)
"#),
        "Xb\n1\n"
    );
}

#[test]
fn compound_matrix_indices_and_map_receivers_are_evaluated_once() {
    assert_eq!(
        run(r#"
importe "matriz"
A = matriz([[5]])
m = mapa([:n -> 0, :value -> 7])
idx = funcao() { m:n += 1 0 }
receiver = funcao() { m:n += 1 m }
A[idx(), idx()] -= 2
receiver():value += 4
escreva(A[0,0])
escreva(m:value)
escreva(m:n)
"#),
        "3\n11\n3\n"
    );
}

#[test]
fn compound_assignment_preserves_function_write_barriers() {
    assert_eq!(
        run("x = 1\nf = funcao() { x += 1 }\nf() se_falhar 0\nescreva(x)"),
        "1\n"
    );
}

#[test]
fn failed_native_import_publishes_no_bindings() {
    assert_eq!(
        run(r#"
fundo = 1
avalie("importe \"tela\"") se_falhar 0
escreva(tipo(casa) se_falhar "absent")
escreva(fundo)
"#),
        "absent\n1\n"
    );
}

struct ModuleDir(std::path::PathBuf);
impl ModuleDir {
    fn new() -> Self {
        static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "expressa-regression-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn run(&self, source: &str) -> Result<String, expressa::runtime::RuntimeError> {
        run_to_string(source, self.0.join("main.lep").to_str().unwrap())
    }
}
impl Drop for ModuleDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn failed_file_import_with_late_name_collision_is_atomic() {
    let dir = ModuleDir::new();
    std::fs::write(dir.0.join("module.lep"), "aaa = 1\nf = 3\n").unwrap();
    // An ordinary user-name collision after aaa must not leave aaa installed.
    assert_eq!(dir.run("f = 2\navalie(\"importe \\\"./module\\\"\") se_falhar 0\nescreva(tipo(aaa) se_falhar \"absent\")").unwrap(), "absent\n");
}

#[test]
fn invalid_loop_control_keeps_imported_function_source_and_stack() {
    let dir = ModuleDir::new();
    for control in ["pare", "continue"] {
        std::fs::write(
            dir.0.join("module.lep"),
            format!("f = funcao() {{\n {control}\n}}\n"),
        )
        .unwrap();
        let error = dir.run("m = importe \"./module\"\nm::f()").unwrap_err();
        assert!(error.file.ends_with("module.lep"), "{error}");
        assert_eq!(error.span.line, 2);
        assert_eq!(error.stack.last().unwrap().name, "f");
        assert!(error.stack.last().unwrap().file.ends_with("module.lep"));
    }
}

#[test]
fn matrix_overflow_is_catchable_and_does_not_create_invalid_cells() {
    assert_eq!(
        run(r#"
importe "matriz"
A = matriz([[100000000000000000000]])
B = matriz([[10000000000]])
escreva(A * B se_falhar "range")
escreva(A * 10000000000 se_falhar "range")
escreva(det(matriz([[100000000000000000000, 0], [0, 100000000000000000000]])) se_falhar "range")
escreva(A[0,0] == 100000000000000000000)
"#),
        "range\nrange\nrange\nverdadeiro\n"
    );
}

#[test]
fn matrix_dimensions_are_checked_before_allocation() {
    assert_eq!(
        run(r#"
importe "matriz"
escreva(zeros(9223372036854775807, 2) se_falhar "limit")
escreva(identidade(9223372036854775807) se_falhar "limit")
escreva(uns(9223372036854775807) se_falhar "limit")
escreva(cheia(2, 9223372036854775807, 1) se_falhar "limit")
"#),
        "limit\nlimit\nlimit\nlimit\n"
    );
}

#[test]
fn matrix_budget_is_explicit_and_applies_to_results_and_constructors() {
    use expressa::runtime::{ExecutionOptions, run_with_options};
    let options = ExecutionOptions {
        max_matrix_cells: 4,
        ..ExecutionOptions::default()
    };
    let result = run_with_options(
        r#"
importe "matriz"
A = uns(4, 1)
B = uns(1, 4)
escreva(A * B se_falhar "limit")
escreva(zeros(5, 1) se_falhar "limit")
escreva(matriz([[1,2,3,4,5]]) se_falhar "limit")
escreva(tamanho(transposta(A)))
escreva_erro("stderr")
saia(7)
"#,
        "limits.lep",
        "",
        options,
    )
    .unwrap();
    assert_eq!(result.stdout, "limit\nlimit\nlimit\n1\n");
    assert_eq!(result.stderr, "stderr\n");
    assert_eq!(result.exit_code, 7);
}

#[test]
fn typed_incomplete_input_does_not_depend_on_error_wording() {
    use expressa::parser::{ParseErrorKind, needs_more_input, parse};
    for source in [
        "2 +",
        "\"text",
        "/* comment",
        "\"\\u{12",
        "f(",
        "lista = [1,",
    ] {
        let mut error = parse(source).unwrap_err();
        assert!(error.is_incomplete(), "{source}: {error:?}");
        error.message = "arbitrary wording".into();
        assert!(error.is_incomplete());
        assert!(needs_more_input(source));
    }
    for source in ["}", "f(1 + *", "\"\\q", "se verdadeiro { 1 fim"] {
        assert!(!needs_more_input(source), "{source}");
    }
    assert_eq!(
        parse("x = se verdadeiro { 1 }").unwrap_err().kind,
        ParseErrorKind::MissingElse
    );
}

#[test]
fn stable_sort_keeps_equal_keys_in_input_order() {
    assert_eq!(
        run(r#"
xs = [mapa([:k -> 2, :v -> "a"]), mapa([:k -> 1, :v -> "b"]), mapa([:k -> 2, :v -> "c"])]
ys = ordene(xs, funcao(x) { x:k })
escreva(ys[0]:v + ys[1]:v + ys[2]:v)
escreva(xs[0]:v)
escreva(ordene([]))
escreva(ordene([3, 2, 1, 2, 0]))
"#),
        "bac\na\n[]\n[0, 1, 2, 2, 3]\n"
    );
}

#[test]
fn avalie_contract_remains_available_to_the_future_vm() {
    assert_eq!(
        run(r#"
x = 10
avalie("x = x + 5")
avalie("nova = 42")
f = avalie("funcao() { nova }")
escreva(x)
escreva(nova)
escreva(f())
make = funcao(n) { avalie("local = n + 1\nfuncao() { local }") }
g = make(4)
escreva(g())
"#),
        "15\n42\n42\n5\n"
    );
}

#[test]
fn text_writeback_preserves_rhs_updates_to_other_characters() {
    assert_eq!(
        run("s = \"abc\"\ns[0] += { s[1] = \"X\" \"\" }\nescreva(s)"),
        "aXc\n"
    );
    assert_eq!(
        run("xs = [\"abc\"]\nxs[0][0] += { xs[0][1] = \"X\" \"\" }\nescreva(xs[0])"),
        "aXc\n"
    );
}

#[test]
fn cancellable_sort_matches_reference_order_across_merge_boundaries() {
    for len in 0..65 {
        let input: Vec<i64> = (0..len).map(|i| (i * 37 + len * 11) % 13 - 6).collect();
        let mut expected = input.clone();
        expected.sort();
        assert_eq!(
            run(&format!("escreva(ordene({input:?}))")),
            format!("{expected:?}\n")
        );
    }
}
