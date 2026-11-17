use super::*;

const BHASKARA: (&str, &str) = (
    "exemplos/escola/bhaskara.lep",
    include_str!("../../../../exemplos/escola/bhaskara.lep"),
);
const JUROS: (&str, &str) = (
    "exemplos/escola/juros.lep",
    include_str!("../../../../exemplos/escola/juros.lep"),
);

fn backends() -> impl Iterator<Item = ExecutionBackend> {
    [ExecutionBackend::Ast, ExecutionBackend::Bytecode]
        .into_iter()
        .filter(|backend| backend.is_available())
}

fn execute(backend: ExecutionBackend, example: (&str, &str), input: &str) -> String {
    struct CursorHost<'a> {
        lines: std::str::Lines<'a>,
        queries: usize,
    }
    impl LeiaHost for CursorHost<'_> {
        fn cursor_position(&mut self) -> Result<(i64, i64), String> {
            self.queries += 1;
            Ok((4, 0))
        }

        fn ask(&mut self, _prompt: &str) -> Result<String, String> {
            self.lines
                .next()
                .map(str::to_string)
                .ok_or_else(|| "fim da entrada".into())
        }
    }
    let mut host = CursorHost {
        lines: input.lines(),
        queries: 0,
    };
    let cursor_example = example.0 == FRACOES.0;
    let mut stdin = io::Cursor::new(input);
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let exit_code = run_configured_with_io(
        example.1,
        example.0,
        Box::new(NoopHook),
        &mut stdin,
        &mut stdout,
        &mut stderr,
        ExecutionOptions {
            backend,
            time_limit: Some(Duration::from_secs(2)),
            numero_locale: NumeroLocale::PtBr,
            numero_aritmetica: NumeroAritmetica::Decimal,
            ..ExecutionOptions::default()
        },
        if cursor_example {
            Some(&mut host)
        } else {
            None
        },
        None,
        false,
        None,
        &[],
    )
    .unwrap_or_else(|e| panic!("{backend:?} {}: {e}", example.0));
    let output = ExecutionOutput {
        stdout: String::from_utf8(stdout).unwrap(),
        stderr: String::from_utf8(stderr).unwrap(),
        exit_code,
    };
    if cursor_example {
        assert!(host.queries > 0, "o exemplo deve consultar o cursor");
        let errors = output.stdout.matches("Erro:").count();
        assert_eq!(output.stdout.matches("\x1b[5;1H").count(), errors);
        assert_eq!(output.stdout.matches("\r\x1b[2K").count(), errors);
    }
    assert!(
        !output.stdout.contains("Calcular novamente?"),
        "{}",
        output.stdout
    );
    assert_eq!(output.exit_code, 0);
    assert!(output.stderr.is_empty(), "{}", output.stderr);
    output.stdout
}

#[test]
fn bhaskara_validates_coefficients_and_handles_each_root_case() {
    for backend in backends() {
        let out = execute(backend, BHASKARA, "abc\n0\n1\n-5\n6\n1\n-2\n1\n1\n0\n1\n\n");
        assert_eq!(
            out.matches("Digite um número diferente de zero.").count(),
            2,
            "{out}"
        );
        for expected in ["x1 = 3", "x2 = 2"] {
            assert!(out.contains(expected), "{backend:?}: {out}");
        }
        assert_eq!(out.matches("equação: ").count(), 1, "{out}");
        for (input, expected) in [
            ("1\n-2\n1\n", "Uma solução real (raiz dupla): x = 1"),
            ("1\n0\n1\n", "Não há soluções reais para essa equação."),
        ] {
            let out = execute(backend, BHASKARA, input);
            assert!(out.contains(expected), "{out}");
        }
        assert!(
            out.find("a·x² + b·x + c = 0").unwrap() < out.find("a: ").unwrap(),
            "{out}"
        );
    }
}

#[test]
fn juros_validates_inputs_and_compares_compounding() {
    for backend in backends() {
        let out = execute(backend, JUROS, "0\n1000\n-1\n2\n-1\n1,5\n2\n\n");
        for expected in [
            "Digite um capital maior que zero.",
            "Digite uma taxa maior ou igual a zero.",
            "taxa: 2% ao período",
            "montante: R$ 1.040,00\n",
            "montante: R$ 1.040,40\n",
            "diferença (compostos − simples): R$ 0,40",
        ] {
            assert!(out.contains(expected), "{backend:?}: {out}");
        }
        assert_eq!(
            out.matches("Digite uma quantidade inteira").count(),
            2,
            "{out}"
        );
        let out = execute(backend, JUROS, "12\n2\n24\n\n");
        for expected in [
            "montante: R$ 17,76\n",
            "juros:    R$ 5,76\n",
            "montante: R$ 19,30\n",
            "juros:    R$ 7,30\n",
            "diferença (compostos − simples): R$ 1,54\n",
        ] {
            assert!(out.contains(expected), "{backend:?}: {out}");
        }
    }
}

#[test]
fn juros_accepts_zero_rate_and_zero_periods() {
    for backend in backends() {
        for input in ["1000\n0\n5\n", "1000\n2,5\n0\n"] {
            let out = execute(backend, JUROS, input);
            assert_eq!(out.matches("montante: R$ 1.000,00\n").count(), 2, "{out}");
        }
    }
}

#[test]
fn examples_cancel_on_blank_input_or_eof_without_calculating() {
    for backend in backends() {
        for (example, input, result) in [
            (BHASKARA, "1\n", "equação: "),
            (BHASKARA, "1\n0\n\n", "equação: "),
            (JUROS, "1000\n", "montante:"),
            (JUROS, "1000\n2\n\n", "montante:"),
        ] {
            let out = execute(backend, example, input);
            assert!(!out.contains(result), "{}: {out}", example.0);
        }
    }
}

macro_rules! exemplo {
    ($path:literal) => {
        (
            concat!("exemplos/", $path),
            include_str!(concat!("../../../../exemplos/", $path)),
        )
    };
}

const ESTATISTICA: (&str, &str) = exemplo!("escola/estatistica.lep");
const FRACOES: (&str, &str) = exemplo!("escola/compara_fracoes.lep");
const CINEMATICA: (&str, &str) = exemplo!("escola/cinematica.lep");
const MDC: (&str, &str) = exemplo!("escola/mdc_mmc.lep");

#[test]
fn interactive_examples_calculate_from_supplied_inputs() {
    let cases = [
        (
            ESTATISTICA,
            "4\n1\n1\n3\n3\n",
            vec![
                "média (soma dividida pela quantidade): 2",
                "mediana (valor central ou média dos dois centrais): 2",
                "valores que mais se repetem (moda): [1, 3] (2 vezes cada)",
                "1 | ##",
                "3 | ##",
            ],
        ),
        (
            ESTATISTICA,
            "3\n-1\n0\n4\n",
            vec![
                "mediana (valor central ou média dos dois centrais): 0",
                "nenhum valor se repete; não há moda",
                "diferença entre o maior e o menor valor (amplitude): 5",
            ],
        ),
        (
            ESTATISTICA,
            "1\n2,5\n",
            vec![
                "média (soma dividida pela quantidade): 2,5",
                "mediana (valor central ou média dos dois centrais): 2,5",
                "diferença entre o maior e o menor valor (amplitude): 0",
            ],
        ),
        (FRACOES, "3/4\n2/3\n", vec!["3/4 > 2/3"]),
        (FRACOES, "1/4\n2/8\n", vec!["1/4 = 2/8"]),
        (FRACOES, "1/-2\n0\n", vec!["-1/2 < 0"]),
        (
            CINEMATICA,
            "1\n10\n-2\n5\n",
            vec![
                "velocidade final: -2 m/s",
                "posição final: 0 m",
                "deslocamento (posição final menos posição inicial): -10 m",
            ],
        ),
        (
            CINEMATICA,
            "2\n0\n0\n2\n8\n",
            vec!["velocidade final: 16 m/s", "posição final: 64 m"],
        ),
        (
            CINEMATICA,
            "2\n10\n2\n0\n0\n",
            vec![
                "posição final: 10 m",
                "deslocamento (posição final menos posição inicial): 0 m",
            ],
        ),
        (
            MDC,
            "48\n18\n",
            vec![
                "resto de 48 dividido por 18 = 12",
                "mdc(48, 18) = 6",
                "mmc(48, 18) = 144",
            ],
        ),
    ];
    for backend in backends() {
        for (example, input, expected) in &cases {
            let out = execute(backend, *example, input);
            for text in expected {
                assert!(
                    out.contains(text),
                    "{backend:?} {}: faltou {text:?} em {out}",
                    example.0
                );
            }
        }
    }
}

#[test]
fn interactive_examples_retry_invalid_values() {
    let cases = [
        (
            ESTATISTICA,
            "1,5\n1\nabc\n2\n",
            vec![
                "Digite uma quantidade inteira",
                "Digite um número válido.",
                "média (soma dividida pela quantidade): 2",
            ],
        ),
        (
            FRACOES,
            "1/2/3\n1/0\n1,5/2\nabc\n1/2\n1\n",
            vec![
                "Use um inteiro",
                "denominador) não pode ser zero",
                "Use inteiros",
                "1/2 < 1",
            ],
        ),
        (
            CINEMATICA,
            "3\n1\nabc\n0\n1\n-1\n2\n",
            vec![
                "Escolha 1 ou 2.",
                "Digite um número válido.",
                "Digite um tempo",
                "posição final: 2 m",
            ],
        ),
        (
            MDC,
            "-1\n1,5\n3\n0\n2\n",
            vec!["Digite um inteiro maior que zero.", "mdc(3, 2) = 1"],
        ),
    ];
    for backend in backends() {
        for (example, input, expected) in &cases {
            let out = execute(backend, *example, input);
            for text in expected {
                assert!(
                    out.contains(text),
                    "{backend:?} {}: faltou {text:?} em {out}",
                    example.0
                );
            }
        }
    }
}

#[test]
fn interactive_examples_cancel_without_a_result() {
    for backend in backends() {
        for (example, partial, result) in [
            (
                ESTATISTICA,
                "2\n1\n",
                "média (soma dividida pela quantidade):",
            ),
            (FRACOES, "1/2\n", "1/2 ="),
            (CINEMATICA, "2\n0\n0\n", "posição final:"),
            (MDC, "48\n", "Passos do algoritmo"),
        ] {
            for input in [
                String::new(),
                "\n".to_string(),
                partial.to_string(),
                format!("{partial}\n"),
            ] {
                let out = execute(backend, example, &input);
                assert!(!out.contains(result), "{}: {out}", example.0);
            }
        }
    }
}
