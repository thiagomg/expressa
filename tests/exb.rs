use expressa::runtime::{
    ExecutionBackend, ExecutionOptions, compile_exb, load_exb, run_compiled_with_options,
    run_exb_with_options, run_with_options,
};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let p = std::env::temp_dir().join(format!(
            "expressa-exb-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&p).unwrap();
        Self(p)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn options() -> ExecutionOptions {
    ExecutionOptions {
        backend: ExecutionBackend::Bytecode,
        ..Default::default()
    }
}
fn cli() -> std::process::Command {
    let mut c = std::process::Command::new(env!("CARGO_BIN_EXE_expressa"));
    c.env("EXPRESSA_BACKEND", "bytecode")
        .env("EXPRESSA_NUMEROS", "en")
        .env("EXPRESSA_ARITMETICA", "decimal");
    c
}
#[test]
fn compiled_execution_matches_source_values_input_exit_codes_and_reuses_fresh_state() {
    for source in [
        "escreva(leia(\"nome? \"))\nescreva_erro(\"erro\")\nsaia(7)",
        "f = funcao(a) { avalie(\"a = a + 1\") a }\nescreva(f(2))",
        "make = funcao(a) { funcao() { a } }\nescreva(make(9)())",
        "escreva(0.1234567890123456789012345678 + 0.0000000000000000000000000001)",
        "xs = [0]\nxs[0] += 1\nescreva(xs)",
        "f = funcao(n) { se n == 0 { 0 } senao { f(n - 1) + 1 } }\nescreva(f(2000))",
    ] {
        let expected = run_with_options(source, "sample.lep", "Ana\n", options()).unwrap();
        let bytes = compile_exb(source, "sample.lep").unwrap();
        let loaded = load_exb(&bytes).unwrap();
        for _ in 0..2 {
            assert_eq!(
                run_compiled_with_options(&loaded, "sample.exb", "Ana\n", options()).unwrap(),
                expected
            );
        }
    }
}
#[test]
fn cli_compiles_without_executing_and_runs_without_the_lep() {
    let t = Temp::new();
    let source = t.0.join("main.lep");
    let output = t.0.join("main.exb");
    std::fs::write(&source, "escreva(argumentos)\nsaia(7)").unwrap();
    let result = cli().arg("compilar").arg(&source).output().unwrap();
    assert!(result.status.success(), "{:?}", result);
    assert!(result.stdout.is_empty());
    assert!(output.exists());
    std::fs::remove_file(&source).unwrap();
    let result = cli()
        .arg(&output)
        .args(["--backend", "literal"])
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(7));
    assert_eq!(result.stdout, b"[\"--backend\", \"literal\"]\n");
    assert!(result.stderr.is_empty());
    let rejected = cli()
        .args(["--backend", "ast"])
        .arg(&output)
        .output()
        .unwrap();
    assert!(!rejected.status.success());
    assert!(String::from_utf8_lossy(&rejected.stderr).contains("backend bytecode"));
}
#[test]
fn cli_compilation_is_atomic_and_does_not_replace_source() {
    let t = Temp::new();
    let source = t.0.join("bad.lep");
    let output = t.0.join("keep.exb");
    std::fs::write(&source, "[").unwrap();
    std::fs::write(&output, b"keep").unwrap();
    let result = cli()
        .arg("compilar")
        .arg(&source)
        .arg("-o")
        .arg(&output)
        .output()
        .unwrap();
    assert!(!result.status.success());
    assert_eq!(std::fs::read(&output).unwrap(), b"keep");
    std::fs::write(&source, "1").unwrap();
    let result = cli()
        .arg("compilar")
        .arg(&source)
        .arg("-o")
        .arg(&source)
        .output()
        .unwrap();
    assert!(!result.status.success());
    assert_eq!(std::fs::read_to_string(&source).unwrap(), "1");
}
#[test]
fn compiled_modules_import_and_cache_relative_to_artifact_location() {
    let t = Temp::new();
    let module = t.0.join("helper.exb");
    std::fs::write(
        &module,
        compile_exb("escreva(\"loaded\")\nf = funcao(a) { a + 1 }", "helper.lep").unwrap(),
    )
    .unwrap();
    let source = "m = importe \"./helper.exb\"\nn = importe \"./helper.exb\"\nescreva(m::f(4))\nescreva(n::f(5))";
    let bytes = compile_exb(source, "old-location/main.lep").unwrap();
    let file = t.0.join("main.exb").to_str().unwrap().to_owned();
    assert_eq!(
        run_exb_with_options(&bytes, &file, "", options())
            .unwrap()
            .stdout,
        "loaded\n5\n6\n"
    );
    let error = run_exb_with_options(
        &bytes,
        &file,
        "",
        ExecutionOptions {
            workspace_root: Some(t.0.join("restricted")),
            ..options()
        },
    )
    .unwrap_err();
    assert!(error.message.contains("fora da pasta"));
}
#[test]
fn imported_compiled_errors_preserve_original_source_and_stack() {
    let t = Temp::new();
    let module = t.0.join("m.exb");
    std::fs::write(
        &module,
        compile_exb("f = funcao(a) { a / 0 }", "original-m.lep").unwrap(),
    )
    .unwrap();
    let src = "m = importe \"./m.exb\"\nm::f(2)";
    let file = t.0.join("main.lep").to_str().unwrap().to_owned();
    let expected = run_with_options(src, &file, "", options()).unwrap_err();
    let bytes = compile_exb(src, &file).unwrap();
    let actual = run_exb_with_options(&bytes, &file, "", options()).unwrap_err();
    assert_eq!(
        (&expected.message, &expected.file, expected.span),
        (&actual.message, &actual.file, actual.span)
    );
    assert_eq!(actual.file, "original-m.lep");
    let frames = |e: &expressa::runtime::RuntimeError| {
        e.stack
            .iter()
            .map(|f| (f.name.clone(), f.file.clone(), f.span))
            .collect::<Vec<_>>()
    };
    assert_eq!(frames(&actual), frames(&expected));
}
#[test]
fn compiled_cycles_and_limits_are_errors() {
    let t = Temp::new();
    let module = t.0.join("cycle.exb");
    std::fs::write(
        &module,
        compile_exb("importe \"./cycle.exb\"", "cycle.lep").unwrap(),
    )
    .unwrap();
    let bytes = compile_exb("importe \"./cycle.exb\"", "main.lep").unwrap();
    let file = t.0.join("main.exb").to_str().unwrap().to_owned();
    assert!(
        run_exb_with_options(&bytes, &file, "", options())
            .unwrap_err()
            .message
            .contains("cíclica")
    );
    let bytes = compile_exb("importe \"matriz\"\nescreva(uns(10))", "main.lep").unwrap();
    assert!(
        run_exb_with_options(
            &bytes,
            &file,
            "",
            ExecutionOptions {
                max_matrix_cells: 4,
                ..options()
            }
        )
        .unwrap_err()
        .message
        .contains("matriz")
    );
}
#[test]
fn cli_marker_and_debug_use_embedded_source() {
    use std::io::Write;
    let t = Temp::new();
    let file = t.0.join("main.exb");
    std::fs::write(
        &file,
        compile_exb("escreva(leia(\"nome? \"))", "missing.lep").unwrap(),
    )
    .unwrap();
    let mut child = cli()
        .arg("--marcador-leia")
        .arg(&file)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(b"Ana\n").unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    assert_eq!(output.stdout, b"Ana\n");
    assert!(String::from_utf8_lossy(&output.stderr).contains("<<<EXPRESSA-LEIA>>>"));
    std::fs::write(&file, compile_exb("escreva(42)", "missing.lep").unwrap()).unwrap();
    let mut child = cli()
        .arg("debug")
        .arg(&file)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(b"c\n").unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success(), "{:?}", output);
    assert!(String::from_utf8_lossy(&output.stdout).contains("42"));
}
