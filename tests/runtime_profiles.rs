//! The same language core and wire format run in full and reduced desktop builds.
use expressa::runtime::{
    ExecutionBackend, ExecutionOptions, NATIVAS, builtin_docs, compile_exb, load_exb,
    run_compiled_with_options, run_with_options,
};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

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
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "expressa-profile-{}-{}",
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

#[test]
fn all_common_workloads_run_from_source_and_exb_in_every_profile() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("benchmarks/programs");
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|x| x.to_str()) != Some("lep") {
            continue;
        }
        let name = path.to_str().unwrap();
        let source = std::fs::read_to_string(&path).unwrap();
        let expected = run_with_options(&source, name, "", options()).unwrap();
        assert_eq!(expected.stdout, "verdadeiro\n", "{name}");
        let program = load_exb(&compile_exb(&source, name).unwrap()).unwrap();
        assert_eq!(
            run_compiled_with_options(&program, name, "", options()).unwrap(),
            expected,
            "{name}"
        );
    }
}

#[test]
fn native_registration_help_and_imports_agree_with_enabled_features() {
    for (module, enabled) in [
        ("tela", cfg!(feature = "terminal")),
        ("cartesiano", cfg!(feature = "graphics")),
        ("tela_grafica", cfg!(feature = "graphics")),
        ("imagem", cfg!(feature = "graphics")),
        ("processo", cfg!(feature = "process")),
    ] {
        assert_eq!(
            NATIVAS.iter().any(|n| n.module == Some(module)),
            enabled,
            "{module}"
        );
        assert_eq!(
            builtin_docs()
                .iter()
                .any(|d| d.module.as_deref() == Some(module)),
            enabled,
            "{module}"
        );
        let source = format!("m = importe \"{module}\"\nescreva(tipo(m))");
        let result = run_with_options(&source, "profile.lep", "", options());
        if enabled {
            assert_eq!(result.unwrap().stdout, "modulo\n");
        } else {
            let err = result.unwrap_err();
            assert!(err.message.contains("indisponível neste build"), "{err}");
            assert_eq!(err.span.line, 1);
        }
    }
}

#[test]
fn ast_availability_is_explicit_and_syntax_tree_printing_keeps_the_frontend() {
    assert_eq!(ExecutionBackend::Ast.is_available(), cfg!(feature = "ast"));
    let result = run_with_options(
        "escreva(2 + 3)",
        "ast.lep",
        "",
        ExecutionOptions {
            backend: ExecutionBackend::Ast,
            ..options()
        },
    );
    if cfg!(feature = "ast") {
        assert_eq!(result.unwrap().stdout, "5\n");
    } else {
        assert!(
            result
                .unwrap_err()
                .message
                .contains("backend AST indisponível")
        );
    }
    let t = Temp::new();
    let source = t.0.join("sample.lep");
    std::fs::write(&source, "escreva(1)").unwrap();
    assert!(
        cli()
            .arg("--ast")
            .arg(&source)
            .output()
            .unwrap()
            .status
            .success()
    );
    if !cfg!(feature = "ast") {
        let no_file = cli().env("EXPRESSA_BACKEND", "ast").output().unwrap();
        assert!(!no_file.status.success());
        assert!(String::from_utf8_lossy(&no_file.stderr).contains("backend AST indisponível"));
        for c in [
            cli()
                .args(["--backend", "ast"])
                .arg(&source)
                .output()
                .unwrap(),
            cli()
                .env("EXPRESSA_BACKEND", "ast")
                .arg(&source)
                .output()
                .unwrap(),
        ] {
            assert!(!c.status.success());
            assert!(String::from_utf8_lossy(&c.stderr).contains("backend AST indisponível"));
        }
    }
}

#[test]
fn cli_help_and_unavailable_services_match_the_build() {
    let out = cli().arg("--help").output().unwrap();
    assert!(out.status.success());
    let text = String::from_utf8(out.stderr).unwrap();
    assert_eq!(text.contains("REPL"), cfg!(feature = "terminal"));
    assert_eq!(text.contains("expressa debug"), cfg!(feature = "terminal"));
    assert_eq!(text.contains("--grafico"), cfg!(feature = "graphics"));
    assert_eq!(
        text.contains("--backend ast|bytecode"),
        cfg!(feature = "ast")
    );
    if !cfg!(feature = "terminal") {
        let out = cli().output().unwrap();
        assert!(!out.status.success());
        assert!(String::from_utf8_lossy(&out.stderr).contains("REPL indisponível"));
        let out = cli().args(["debug", "missing.exb"]).output().unwrap();
        assert!(!out.status.success());
        assert!(
            String::from_utf8_lossy(&out.stderr).contains("depurador de terminal indisponível")
        );
    }
    if !cfg!(feature = "graphics") {
        for args in [vec!["--grafico", "sample.cart"], vec!["--ao-vivo"]] {
            let out = cli().args(args).output().unwrap();
            assert!(!out.status.success());
            assert!(String::from_utf8_lossy(&out.stderr).contains("gráficos indisponíveis"));
        }
    }
}

#[test]
fn explicit_file_imports_work_even_when_a_native_with_that_name_is_omitted() {
    let t = Temp::new();
    let name = if !cfg!(feature = "process") {
        "processo"
    } else {
        "custom"
    };
    std::fs::write(t.0.join(format!("{name}.lep")), "n = 42").unwrap();
    let source = format!("m = importe \"./{name}\"\nescreva(m::n)");
    let file = t.0.join("main.lep");
    assert_eq!(
        run_with_options(&source, file.to_str().unwrap(), "", options())
            .unwrap()
            .stdout,
        "42\n"
    );
    if !cfg!(feature = "process") {
        let err = run_with_options(
            "importe \"processo\"",
            file.to_str().unwrap(),
            "",
            options(),
        )
        .unwrap_err();
        assert!(err.message.contains("indisponível neste build"));
    }
}
