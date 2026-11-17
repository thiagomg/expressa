//! Exercise the examples through the CLI, including binary stdin and cwd paths.
use expressa::runtime::{
    ExecutionBackend, ExecutionOptions, LeiaHost, run_with_leia_host, run_with_options,
};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "expressa-wc-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }
    fn file(&self, name: &str, bytes: &[u8]) -> PathBuf {
        let path = self.0.join(name);
        std::fs::write(&path, bytes).unwrap();
        path
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn backends() -> impl Iterator<Item = &'static str> {
    [ExecutionBackend::Ast, ExecutionBackend::Bytecode]
        .into_iter()
        .filter(|backend| backend.is_available())
        .map(|backend| {
            if backend == ExecutionBackend::Ast {
                "ast"
            } else {
                "bytecode"
            }
        })
}
fn cli(example: &str, backend: &str, args: &[&str], input: &[u8], cwd: &Path) -> Output {
    cli_locale(example, backend, args, input, cwd, "C.UTF-8", false)
}

fn cli_locale(
    example: &str,
    backend: &str,
    args: &[&str],
    input: &[u8],
    cwd: &Path,
    locale: &str,
    posix: bool,
) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_expressa"));
    command
        .args(["--backend", backend])
        .arg(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("exemplos/unix")
                .join(example),
        )
        .args(args)
        .current_dir(cwd)
        .env("LC_ALL", locale)
        .env_remove("POSIXLY_CORRECT")
        .env("EXPRESSA_NUMEROS", "pt")
        .env("EXPRESSA_ARITMETICA", "decimal")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if posix {
        command.env("POSIXLY_CORRECT", "");
    }
    let mut child = command.spawn().unwrap();
    child.stdin.take().unwrap().write_all(input).unwrap();
    child.wait_with_output().unwrap()
}

#[test]
fn wc_complexo_honors_c_locale_and_posix_word_separators() {
    let tmp = Temp::new();
    for backend in backends() {
        for (locale, posix, expected) in [
            ("C", false, vec!["0", "1", "7", "7"]),
            ("C.UTF-8", false, vec!["0", "3", "5", "7"]),
            ("C.UTF-8", true, vec!["0", "1", "5", "7"]),
        ] {
            let out = cli_locale(
                "wc_complexo.lep",
                backend,
                &["-lwmc"],
                "a\u{a0}b\u{a0}c".as_bytes(),
                &tmp.0,
                locale,
                posix,
            );
            ok(&out);
            assert_eq!(words(&out), expected);
        }
    }
}
fn words(output: &Output) -> Vec<String> {
    String::from_utf8(output.stdout.clone())
        .unwrap()
        .split_whitespace()
        .map(str::to_owned)
        .collect()
}
fn ok(output: &Output) {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stderr.is_empty(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn wc_complexo_counts_raw_stdin_and_combined_options() {
    let tmp = Temp::new();
    let cases: &[(&[&str], &[u8], &[&str])] = &[
        (&[], b"", &["0", "0", "0"]),
        (&[], b"guarda-chuva ...\n", &["1", "2", "17"]),
        (&["-l"], b"abc", &["0"]),
        (&["-l"], b"a\r\nb\r\n", &["2"]),
        (&["-w"], b"a\tb\rc\x0bd\x0ce  f", &["6"]),
        (&["-lwmc"], "Olá\r\n".as_bytes(), &["1", "1", "5", "6"]),
        (
            &["--lines", "--words", "--bytes"],
            b"one\ntwo",
            &["1", "2", "7"],
        ),
        (&["-l", "-w", "-c"], b"one\ntwo", &["1", "2", "7"]),
        (&["-lwmc"], b"a\xff b\0\n", &["1", "2", "5", "6"]),
        (&["-m"], b"\xff\xc3", &["0"]),
        (&["-w"], "a\u{a0}b\u{2003}c".as_bytes(), &["3"]),
        (&["-c"], &[b'x'; 1200], &["1200"]),
        (
            &["-w", "-", "-"],
            b"one two",
            &["2", "-", "0", "-", "2", "total"],
        ),
    ];
    for backend in backends() {
        for (args, input, expected) in cases {
            let output = cli("wc_complexo.lep", backend, args, input, &tmp.0);
            ok(&output);
            assert_eq!(words(&output), *expected, "{backend} {args:?} {input:?}");
        }
    }
}

#[test]
fn wc_complexo_reads_cwd_files_and_prints_totals_only_for_multiple_operands() {
    let tmp = Temp::new();
    tmp.file("a.txt", b"one\ntwo");
    tmp.file("b.txt", "á\r\n".as_bytes());
    tmp.file("-data", b"abc");
    for backend in backends() {
        for (args, input, expected) in [
            (vec!["a.txt"], &b""[..], vec!["1", "2", "7", "a.txt"]),
            (
                vec!["-lwc", "a.txt", "b.txt"],
                &b""[..],
                vec![
                    "1", "2", "7", "a.txt", "1", "1", "4", "b.txt", "2", "3", "11", "total",
                ],
            ),
            (
                vec!["-w", "a.txt", "-"],
                &b"extra word"[..],
                vec!["2", "a.txt", "2", "-", "4", "total"],
            ),
            (vec!["-c", "--", "-data"], &b""[..], vec!["3", "-data"]),
        ] {
            let output = cli("wc_complexo.lep", backend, &args, input, &tmp.0);
            ok(&output);
            assert_eq!(words(&output), expected);
        }
    }
}

#[test]
fn wc_complexo_reports_errors_and_continues_with_later_files() {
    let tmp = Temp::new();
    tmp.file("good.txt", b"a\n");
    for backend in backends() {
        let out = cli(
            "wc_complexo.lep",
            backend,
            &["-l", "missing", "good.txt"],
            b"",
            &tmp.0,
        );
        assert_eq!(out.status.code(), Some(1));
        assert_eq!(words(&out), ["1", "good.txt", "1", "total"]);
        assert!(String::from_utf8_lossy(&out.stderr).contains("missing"));
        for args in [&["-z"][..], &["--unknown"][..]] {
            let out = cli("wc_complexo.lep", backend, args, b"", &tmp.0);
            assert_eq!(out.status.code(), Some(1));
            assert!(String::from_utf8_lossy(&out.stderr).contains("opção desconhecida"));
            assert!(out.stdout.is_empty());
        }
    }
}

#[test]
fn help_functions_do_not_shadow_native_ajuda() {
    let tmp = Temp::new();
    for backend in backends() {
        for (example, arg) in [
            ("cat.lep", "--ajuda"),
            ("wc_complexo.lep", "--ajuda"),
            ("wc_complexo.lep", "--help"),
        ] {
            let out = cli(example, backend, &[arg], b"", &tmp.0);
            ok(&out);
            assert!(!out.stdout.is_empty());
        }
    }
}

#[test]
fn raw_byte_natives_preserve_data_and_validate_decoding() {
    let tmp = Temp::new();
    let file = tmp.file("binary", b"a\r\n\xff\0");
    for backend in [ExecutionBackend::Ast, ExecutionBackend::Bytecode]
        .into_iter()
        .filter(|b| b.is_available())
    {
        let source = format!(
            r#"importe "arquivo"
escreva(leia_bytes("{}"))
escreva(leia_bytes())
escreva(texto_utf8([79, 108, 195, 161]))
escreva(texto_utf8([97, 255, 98], :substitua))
escreva(texto_utf8([97, 255, 98], :ignore))
escreva(texto_utf8([255]) se_falhar {{ "inválido" }})
escreva(texto_utf8([256]) se_falhar {{ "fora do intervalo" }})
escreva(texto_utf8([1.5]) se_falhar {{ "não inteiro" }})
escreva(texto_utf8([65], :inexistente) se_falhar {{ "modo inválido" }})
"#,
            file.to_string_lossy().replace('\\', "\\\\")
        );
        let output = run_with_options(
            &source,
            "bytes.lep",
            "b\r\nc",
            ExecutionOptions {
                backend,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(
            output.stdout,
            "[97, 13, 10, 255, 0]\n[98, 13, 10, 99]\nOlá\na�b\nab\ninválido\nfora do intervalo\nnão inteiro\nmodo inválido\n"
        );
    }
    struct Host;
    impl LeiaHost for Host {
        fn ask(&mut self, _: &str) -> Result<String, String> {
            unreachable!()
        }
        fn read_all_bytes(&mut self) -> Result<Vec<u8>, String> {
            Ok(vec![0, 255, 13, 10])
        }
    }
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    run_with_leia_host(
        "importe \"arquivo\"\nescreva(leia_bytes())",
        "host.lep",
        None,
        None,
        &mut stdout,
        &mut stderr,
        &mut Host,
    )
    .unwrap();
    assert_eq!(String::from_utf8(stdout).unwrap(), "[0, 255, 13, 10]\n");
}

#[test]
fn wc_simples_counts_whitespace_separated_words() {
    let tmp = Temp::new();
    for backend in backends() {
        for (input, expected) in [
            ("", "0"),
            ("guarda-chuva ...", "2"),
            ("a\tb\rc\u{b}d\u{c}e\n f", "6"),
            ("Olá mundo!", "2"),
            ("  um\u{a0}dois\u{2003}três  ", "3"),
            ("abc", "1"),
        ] {
            let out = cli("wc.lep", backend, &[], input.as_bytes(), &tmp.0);
            ok(&out);
            assert_eq!(words(&out), [expected]);
        }
        let out = cli("wc.lep", backend, &["--ajuda"], b"", &tmp.0);
        ok(&out);
        assert!(String::from_utf8_lossy(&out.stdout).contains("espaços em branco"));
    }
}

#[test]
fn wc_simples_keeps_file_input_and_line_option() {
    let tmp = Temp::new();
    tmp.file("a.txt", b"guarda-chuva ...");
    tmp.file("b.txt", b"one\r\ntwo\r\n");
    for backend in backends() {
        let out = cli("wc.lep", backend, &["a.txt", "b.txt"], b"", &tmp.0);
        ok(&out);
        assert_eq!(
            words(&out),
            ["a.txt", "=", "2", "b.txt", "=", "2", "Total", "geral:", "4"]
        );
        for (input, expected) in [("abc", "0"), ("a\r\nb\r\n", "2")] {
            let out = cli("wc.lep", backend, &["-l"], input.as_bytes(), &tmp.0);
            ok(&out);
            assert_eq!(words(&out), [expected]);
        }
    }
}
