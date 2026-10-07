use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::lexer::Span;

use super::env::{Env, FrameKind};
use super::error::EvalError;
use super::eval::Vm;
use super::value::{NumeroLocale, Value, format_numero};

mod arquivo;
mod mat;
mod matriz;
mod nucleo;
mod referencia;
mod tela;

pub use referencia::{BuiltinDoc, builtin_docs, lookup_alvo, lookup_builtin_doc, lookup_ficha};

pub struct Nativa {
    pub names: &'static [&'static str],
    pub module: Option<&'static str>,
}

pub const NATIVAS: &[Nativa] = &[
    Nativa {
        names: &["escreva"],
        module: None,
    },
    Nativa {
        names: &["escreva_erro"],
        module: None,
    },
    Nativa {
        names: &["sair"],
        module: None,
    },
    Nativa {
        names: &["durma"],
        module: None,
    },
    Nativa {
        names: &["leia"],
        module: None,
    },
    Nativa {
        names: &["leia_linhas"],
        module: None,
    },
    Nativa {
        names: &["numero", "número"],
        module: None,
    },
    Nativa {
        names: &["formato"],
        module: None,
    },
    Nativa {
        names: &["formate"],
        module: None,
    },
    Nativa {
        names: &["avaliar"],
        module: None,
    },
    Nativa {
        names: &["catalogo", "catálogo"],
        module: None,
    },
    Nativa {
        names: &["ajuda"],
        module: None,
    },
    Nativa {
        names: &["tamanho"],
        module: None,
    },
    Nativa {
        names: &["primeiro"],
        module: None,
    },
    Nativa {
        names: &["ultimo"],
        module: None,
    },
    Nativa {
        names: &["maiuscula"],
        module: None,
    },
    Nativa {
        names: &["minuscula"],
        module: None,
    },
    Nativa {
        names: &["sem_acento"],
        module: None,
    },
    Nativa {
        names: &["remova"],
        module: None,
    },
    Nativa {
        names: &["substitua"],
        module: None,
    },
    Nativa {
        names: &["procurar"],
        module: None,
    },
    Nativa {
        names: &["separe"],
        module: None,
    },
    Nativa {
        names: &["junte"],
        module: None,
    },
    Nativa {
        names: &["limpe"],
        module: None,
    },
    Nativa {
        names: &["mapa"],
        module: None,
    },
    Nativa {
        names: &["conjunto"],
        module: None,
    },
    Nativa {
        names: &["cls", "limpe_tela"],
        module: Some("tela"),
    },
    Nativa {
        names: &["casa"],
        module: Some("tela"),
    },
    Nativa {
        names: &["eh_terminal", "é_terminal"],
        module: Some("tela"),
    },
    Nativa {
        names: &["pinte"],
        module: Some("tela"),
    },
    Nativa {
        names: &["fundo"],
        module: Some("tela"),
    },
    Nativa {
        names: &["negrito"],
        module: Some("tela"),
    },
    Nativa {
        names: &["colunas"],
        module: Some("tela"),
    },
    Nativa {
        names: &["linhas"],
        module: Some("tela"),
    },
    Nativa {
        names: &["quadro"],
        module: Some("tela"),
    },
    Nativa {
        names: &["bloco"],
        module: Some("tela"),
    },
    Nativa {
        names: &["escreva_em"],
        module: Some("tela"),
    },
    Nativa {
        names: &["raiz"],
        module: Some("mat"),
    },
    Nativa {
        names: &["aleatorio", "aleatório"],
        module: Some("mat"),
    },
    Nativa {
        names: &["semente"],
        module: Some("mat"),
    },
    Nativa {
        names: &["seno", "sen"],
        module: Some("mat"),
    },
    Nativa {
        names: &["cosseno", "cos"],
        module: Some("mat"),
    },
    Nativa {
        names: &["tangente", "tan"],
        module: Some("mat"),
    },
    Nativa {
        names: &["arcoseno"],
        module: Some("mat"),
    },
    Nativa {
        names: &["arcocosseno"],
        module: Some("mat"),
    },
    Nativa {
        names: &["arcotangente"],
        module: Some("mat"),
    },
    Nativa {
        names: &["arcotangente2"],
        module: Some("mat"),
    },
    Nativa {
        names: &["radianos"],
        module: Some("mat"),
    },
    Nativa {
        names: &["graus"],
        module: Some("mat"),
    },
    Nativa {
        names: &["exp"],
        module: Some("mat"),
    },
    Nativa {
        names: &["log"],
        module: Some("mat"),
    },
    Nativa {
        names: &["log10"],
        module: Some("mat"),
    },
    Nativa {
        names: &["potencia"],
        module: Some("mat"),
    },
    Nativa {
        names: &["piso"],
        module: Some("mat"),
    },
    Nativa {
        names: &["teto"],
        module: Some("mat"),
    },
    Nativa {
        names: &["arredonde"],
        module: Some("mat"),
    },
    Nativa {
        names: &["matriz"],
        module: Some("matriz"),
    },
    Nativa {
        names: &["transposta"],
        module: Some("matriz"),
    },
    Nativa {
        names: &["det"],
        module: Some("matriz"),
    },
    Nativa {
        names: &["identidade"],
        module: Some("matriz"),
    },
    Nativa {
        names: &["zeros"],
        module: Some("matriz"),
    },
    Nativa {
        names: &["uns"],
        module: Some("matriz"),
    },
    Nativa {
        names: &["cheia"],
        module: Some("matriz"),
    },
    Nativa {
        names: &["nlinhas"],
        module: Some("matriz"),
    },
    Nativa {
        names: &["ncolunas"],
        module: Some("matriz"),
    },
    Nativa {
        names: &["leia_arquivo"],
        module: Some("arquivo"),
    },
    Nativa {
        names: &["salve_arquivo"],
        module: Some("arquivo"),
    },
    Nativa {
        names: &["adicione_arquivo"],
        module: Some("arquivo"),
    },
    Nativa {
        names: &["leia_csv"],
        module: Some("arquivo"),
    },
    Nativa {
        names: &["salve_csv"],
        module: Some("arquivo"),
    },
];

pub fn bind_nucleo(env: &Rc<RefCell<Env>>) {
    for n in NATIVAS {
        if n.module.is_none() {
            let canon = n.names[0];
            for name in n.names {
                env.borrow_mut().define(*name, Value::Builtin(canon));
            }
        }
    }
}

pub fn make_native_modules() -> HashMap<String, Rc<RefCell<Env>>> {
    let mut map: HashMap<String, Rc<RefCell<Env>>> = HashMap::new();
    for n in NATIVAS {
        let Some(mod_name) = n.module else { continue };
        let env = map
            .entry(mod_name.to_string())
            .or_insert_with(|| Env::new(FrameKind::Module, None));
        let canon = n.names[0];
        for name in n.names {
            env.borrow_mut().define(*name, Value::Builtin(canon));
        }
    }
    if let Some(mat) = map.get("mat") {
        mat.borrow_mut()
            .define("pi", Value::Numero(std::f64::consts::PI));
    }
    if let Some(tela) = map.get("tela") {
        tela.borrow_mut()
            .define("nova_linha", Value::Texto(tela::NOVA_LINHA.into()));
    }
    map
}

impl Vm<'_> {
    pub(crate) fn call_builtin(
        &mut self,
        name: &str,
        args: &[Value],
        span: Span,
        env: &Rc<RefCell<Env>>,
    ) -> Result<Value, EvalError> {
        match name {
            "escreva" => self.bi_escreva(args, span),
            "escreva_erro" => self.bi_escreva_erro(args, span),
            "sair" => self.bi_sair(args, span),
            "cls" | "limpe_tela" => self.bi_cls(args, span),
            "casa" => self.bi_casa(args, span),
            "durma" => self.bi_durma(args, span),
            "leia" => self.bi_leia(args, span),
            "leia_linhas" => self.bi_leia_linhas(args, span),
            "eh_terminal" | "é_terminal" => self.bi_eh_terminal(args, span),
            "pinte" => self.bi_pinte(args, span),
            "fundo" => self.bi_fundo(args, span),
            "negrito" => self.bi_negrito(args, span),
            "colunas" => self.bi_colunas(args, span),
            "linhas" => self.bi_linhas(args, span),
            "quadro" => self.bi_quadro(args, span),
            "bloco" => self.bi_bloco(args, span),
            "escreva_em" => self.bi_escreva_em(args, span),
            "raiz" => self.bi_raiz(args, span),
            "aleatorio" | "aleatório" => self.bi_aleatorio(args, span),
            "semente" => self.bi_semente(args, span),
            "seno" | "sen" => self.bi_seno(args, span),
            "cosseno" | "cos" => self.bi_cosseno(args, span),
            "tangente" | "tan" => self.bi_tangente(args, span),
            "arcoseno" => self.bi_arcoseno(args, span),
            "arcocosseno" => self.bi_arcocosseno(args, span),
            "arcotangente" => self.bi_arcotangente(args, span),
            "arcotangente2" => self.bi_arcotangente2(args, span),
            "radianos" => self.bi_radianos(args, span),
            "graus" => self.bi_graus(args, span),
            "exp" => self.bi_exp(args, span),
            "log" => self.bi_log(args, span),
            "log10" => self.bi_log10(args, span),
            "potencia" => self.bi_potencia(args, span),
            "piso" => self.bi_piso(args, span),
            "teto" => self.bi_teto(args, span),
            "arredonde" => self.bi_arredonde(args, span),
            "matriz" => self.bi_matriz(args, span),
            "transposta" => self.bi_transposta(args, span),
            "det" => self.bi_det(args, span),
            "identidade" => self.bi_identidade(args, span),
            "zeros" => self.bi_zeros(args, span),
            "uns" => self.bi_uns(args, span),
            "cheia" => self.bi_cheia(args, span),
            "nlinhas" => self.bi_nlinhas(args, span),
            "ncolunas" => self.bi_ncolunas(args, span),
            "numero" | "número" => self.bi_numero(args, span),
            "formato" => self.bi_formato(args, span),
            "formate" => self.bi_formate(args, span),
            "avaliar" => self.bi_avaliar(args, span, env),
            "catalogo" | "catálogo" => self.bi_catalogo(args, span, env),
            "ajuda" => self.bi_ajuda(args, span),
            "tamanho" => self.bi_tamanho(args, span),
            "primeiro" => self.bi_primeiro(args, span),
            "ultimo" => self.bi_ultimo(args, span),
            "maiuscula" => self.bi_maiuscula(args, span),
            "minuscula" => self.bi_minuscula(args, span),
            "sem_acento" => self.bi_sem_acento(args, span),
            "remova" => self.bi_remova(args, span),
            "substitua" => self.bi_substitua(args, span),
            "procurar" => self.bi_procurar(args, span),
            "separe" => self.bi_separe(args, span),
            "junte" => self.bi_junte(args, span),
            "limpe" => self.bi_limpe(args, span),
            "leia_arquivo" => self.bi_leia_arquivo(args, span),
            "salve_arquivo" => self.bi_salve_arquivo(args, span),
            "adicione_arquivo" => self.bi_adicione_arquivo(args, span),
            "leia_csv" => self.bi_leia_csv(args, span),
            "salve_csv" => self.bi_salve_csv(args, span),
            "mapa" => self.bi_mapa(args, span),
            "conjunto" => self.bi_conjunto(args, span),
            other => Err(self.err(format!("função nativa desconhecida: {other}"), span)),
        }
    }
}

/// Portuguese diacritics → base letter. Combining marks (NFD) are dropped.
pub(super) fn sem_acento(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if is_combining_mark(c) {
            continue;
        }
        out.push(strip_diacritic(c));
    }
    out
}

fn is_combining_mark(c: char) -> bool {
    matches!(c, '\u{0300}'..='\u{036F}')
}

fn strip_diacritic(c: char) -> char {
    match c {
        'á' | 'à' | 'â' | 'ã' | 'ä' => 'a',
        'é' | 'è' | 'ê' | 'ë' => 'e',
        'í' | 'ì' | 'î' | 'ï' => 'i',
        'ó' | 'ò' | 'ô' | 'õ' | 'ö' => 'o',
        'ú' | 'ù' | 'û' | 'ü' => 'u',
        'ý' | 'ÿ' => 'y',
        'ç' => 'c',
        'ñ' => 'n',
        'Á' | 'À' | 'Â' | 'Ã' | 'Ä' => 'A',
        'É' | 'È' | 'Ê' | 'Ë' => 'E',
        'Í' | 'Ì' | 'Î' | 'Ï' => 'I',
        'Ó' | 'Ò' | 'Ô' | 'Õ' | 'Ö' => 'O',
        'Ú' | 'Ù' | 'Û' | 'Ü' => 'U',
        'Ý' => 'Y',
        'Ç' => 'C',
        'Ñ' => 'N',
        other => other,
    }
}

pub(crate) fn builtin_modulo(name: &str) -> &'static str {
    NATIVAS
        .iter()
        .find(|n| n.names.iter().any(|nm| *nm == name))
        .and_then(|n| n.module)
        .unwrap_or("")
}

pub(crate) fn builtin_args(name: &str) -> String {
    lookup_builtin_doc(name)
        .map(|d| d.args_text())
        .unwrap_or_default()
}

pub(crate) fn builtin_doc_summary(name: &str) -> String {
    lookup_builtin_doc(name)
        .map(|d| d.summary.clone())
        .unwrap_or_default()
}

pub(super) fn value_as_texto(v: &Value, loc: NumeroLocale) -> String {
    match v {
        Value::Texto(s) => s.clone(),
        Value::Numero(n) => loc.format(*n),
        other => other.to_string(),
    }
}

pub(super) fn csv_cell(v: &Value) -> String {
    match v {
        Value::Texto(s) => s.clone(),
        Value::Numero(n) => format_numero(*n),
        other => other.to_string(),
    }
}

pub(super) fn lines_to_text(lines: &[Value], span: Span, vm: &Vm<'_>) -> Result<String, EvalError> {
    let mut body = String::new();
    for line in lines {
        match line {
            Value::Texto(s) => {
                body.push_str(s);
                body.push('\n');
            }
            other => {
                return Err(vm.err(
                    format!("esperado lista de textos, encontrado {}", other.type_name()),
                    span,
                ));
            }
        }
    }
    Ok(body)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::{NATIVAS, builtin_docs, lookup_builtin_doc};
    use crate::runtime::run_to_string;

    const PRELUDE: &str =
        "importe \"matriz\"\nimporte \"arquivo\"\nimporte \"tela\"\nimporte \"mat\"\n";

    fn with_mods(src: &str) -> String {
        format!("{PRELUDE}{src}")
    }

    fn run(src: &str) -> String {
        let src = with_mods(src);
        run_to_string(&src, "teste.lep").unwrap_or_else(|e| panic!("run failed: {e}\n{src}"))
    }

    fn run_err(src: &str) -> String {
        let src = with_mods(src);
        match run_to_string(&src, "teste.lep") {
            Err(e) => e.message,
            Ok(out) => panic!("expected error, got {out:?}\n{src}"),
        }
    }

    fn temp_dir() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("expressa-bi-{nanos}"));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn run_in(dir: &Path, src: &str) -> String {
        let file = dir.join("teste.lep");
        let src = with_mods(src);
        run_to_string(&src, file.to_str().unwrap())
            .unwrap_or_else(|e| panic!("run failed: {e}\n{src}"))
    }

    #[test]
    fn builtins_are_registered() {
        let names: Vec<_> = NATIVAS
            .iter()
            .flat_map(|n| n.names.iter().copied())
            .collect();
        assert!(names.contains(&"escreva"));
        assert!(names.contains(&"leia"));
        assert!(names.contains(&"leia_linhas"));
        assert!(names.contains(&"raiz"));
        assert!(names.contains(&"numero"));
        assert!(names.contains(&"número"));
        assert!(names.contains(&"eh_terminal"));
        assert!(names.contains(&"é_terminal"));
        assert!(names.contains(&"sem_acento"));
        assert!(names.contains(&"remova"));
        assert!(names.contains(&"procurar"));
        assert!(names.contains(&"catalogo"));
        assert!(names.contains(&"catálogo"));
        assert!(names.contains(&"mapa"));
        assert!(names.contains(&"conjunto"));
        assert!(names.contains(&"matriz"));
        assert_eq!(
            lookup_builtin_doc("é_terminal").map(|d| d.name.as_str()),
            Some("eh_terminal")
        );
        assert_eq!(
            lookup_builtin_doc("limpe_tela").map(|d| d.name.as_str()),
            Some("cls")
        );
        assert_eq!(
            lookup_builtin_doc("aleatório").map(|d| d.name.as_str()),
            Some("aleatorio")
        );
        assert_eq!(
            lookup_builtin_doc("número").map(|d| d.name.as_str()),
            Some("numero")
        );
        assert_eq!(
            lookup_builtin_doc("catálogo").map(|d| d.name.as_str()),
            Some("catalogo")
        );
        for name in &names {
            assert!(
                lookup_builtin_doc(name).is_some(),
                "falta ajuda para {name}"
            );
        }
        for n in NATIVAS {
            if n.module.is_none() {
                assert!(
                    n.names
                        .iter()
                        .any(|nm| *nm == "escreva" || lookup_builtin_doc(nm).is_some())
                );
            }
        }
        let _ = builtin_docs().len();
    }

    #[test]
    fn avaliar_runs_source_in_current_scope() {
        assert_eq!(run(r#"escreva(avaliar("2 + 3 * 4"))"#), "14\n");
        assert_eq!(run(r#"escreva("10 - 3".avaliar())"#), "7\n");
        assert_eq!(
            run(r#"
x = 10
escreva(avaliar("x + 1"))
avaliar("x = 20")
escreva(x)
"#),
            "11\n20\n"
        );
        assert_eq!(
            run(r#"
f = funcao(n) { avaliar("n * 2") }
escreva(f(3))
"#),
            "6\n"
        );
        assert_eq!(run(r#"escreva(avaliar("1 / 0") se_falhar 0)"#), "0\n");
        assert_eq!(run(r#"escreva(avaliar("2 +") se_falhar "ops")"#), "ops\n");
        assert!(run_err("avaliar(1)").contains("texto"));
        assert!(run_err(r#"avaliar("")"#).contains("expressão"));
        let err = run_to_string(r#"avaliar("1 / 0")"#, "teste.lep").expect_err("divisão");
        assert!(err.message.contains("divisão por zero"), "{err}");
        assert!(
            err.stack.iter().any(|f| f.name == "avaliar"),
            "pilha: {:?}",
            err.stack
        );
    }

    #[test]
    fn numero_from_text() {
        assert_eq!(run(r#"escreva(numero("10"))"#), "10\n");
        assert_eq!(run(r#"escreva(número("10"))"#), "10\n");
        assert_eq!(run(r#"escreva(numero("  3,14  "))"#), "3,14\n");
        assert_eq!(run(r#"escreva(numero("1_000"))"#), "1.000\n");
        assert_eq!(run(r#"escreva(numero(-8))"#), "-8\n");
        assert_eq!(run(r#"escreva(numero("abc") se_falhar 0)"#), "0\n");
        assert!(run_err(r#"numero("xyz")"#).contains("transformar"));
        assert_eq!(run(r#"escreva(numero("1.000"))"#), "1.000\n");
        assert_eq!(
            run(r#"formato("en")
escreva(numero("1,000.5"))"#),
            "1,000.5\n"
        );
    }

    #[test]
    fn aleatorio_inclusive_and_seed() {
        assert_eq!(
            run(r#"
semente(1)
a = aleatorio(1, 6)
semente(1)
b = aleatorio(1, 6)
escreva(a == b)
escreva(aleatorio(5, 5))
"#),
            "verdadeiro\n5\n"
        );
        assert_eq!(
            run(r#"
semente(3)
para i de 1 ate 40
inicio
    n = aleatorio(1, 6)
    se n < 1 ou n > 6 { escreva("fora") }
fim
"#),
            ""
        );
        assert_eq!(run("escreva(aleatório(-2, -2))"), "-2\n");
        assert!(run_err("aleatorio(6, 1)").contains("min <= max"));
        assert!(run_err("aleatorio(1)").contains("esperado 2"));
        assert!(run_err(r#"aleatorio(1, "x")"#).contains("numero"));
        assert!(run_err("semente()").contains("esperado 1"));
    }

    #[test]
    fn raiz_quadrada() {
        assert_eq!(run(r#"escreva(raiz(0))"#), "0\n");
        assert_eq!(run(r#"escreva(raiz(9))"#), "3\n");
        assert_eq!(run(r#"escreva(raiz(2.25))"#), "1,5\n");
        assert_eq!(run(r#"escreva(raiz(-1) se_falhar 0)"#), "0\n");
        assert!(run_err("raiz(-4)").contains("raiz de número negativo"));
        assert!(run_err(r#"raiz("9")"#).contains("esperado numero"));
    }

    #[test]
    fn mat_trig_log_e_constantes() {
        assert_eq!(run("escreva(seno(0))"), "0\n");
        assert_eq!(run("escreva(sen(0))"), "0\n");
        assert_eq!(run("escreva(cosseno(0))"), "1\n");
        assert_eq!(run("escreva(cos(0))"), "1\n");
        assert_eq!(run("escreva(tangente(0))"), "0\n");
        assert_eq!(run("escreva(tan(0))"), "0\n");
        assert_eq!(run("escreva(arcoseno(0))"), "0\n");
        assert_eq!(run("escreva(arcocosseno(1))"), "0\n");
        assert_eq!(run("escreva(arcotangente(0))"), "0\n");
        assert_eq!(run("escreva(arcotangente2(0, 1))"), "0\n");
        assert_eq!(run("escreva(exp(0))"), "1\n");
        assert_eq!(run("escreva(log(1))"), "0\n");
        assert_eq!(run("escreva(log10(1000))"), "3\n");
        assert_eq!(run("escreva(potencia(2, 3))"), "8\n");
        assert_eq!(run("escreva(potencia(9, 0.5))"), "3\n");
        assert_eq!(run("escreva(piso(3.7))"), "3\n");
        assert_eq!(run("escreva(piso(-1.2))"), "-2\n");
        assert_eq!(run("escreva(teto(3.1))"), "4\n");
        assert_eq!(run("escreva(teto(-1.2))"), "-1\n");
        assert_eq!(run("escreva(arredonde(1.4))"), "1\n");
        assert_eq!(run("escreva(arredonde(1.5))"), "2\n");
        assert_eq!(run("escreva(arredonde(-1.5))"), "-2\n");
        assert_eq!(run("escreva(arredonde(graus(pi)))"), "180\n");
        assert_eq!(run("escreva(pi > 3 e pi < 4)"), "verdadeiro\n");
        assert_eq!(
            run("escreva(seno(radianos(90)) > 0.999 e seno(radianos(90)) < 1.001)"),
            "verdadeiro\n"
        );
        assert_eq!(run(r#"escreva(0.seno())"#), "0\n");
        assert_eq!(
            run(r#"escreva(90.radianos().seno() > 0.999)"#),
            "verdadeiro\n"
        );
        assert_eq!(
            run(r#"m = importe "mat"
escreva(m::cosseno(0))"#),
            "1\n"
        );
        assert_eq!(run("escreva(arcoseno(2) se_falhar 0)"), "0\n");
        assert!(run_err("arcoseno(2)").contains("entre -1 e 1"));
        assert!(run_err("log(0)").contains("positivo"));
        assert!(run_err("log10(-1)").contains("positivo"));
        assert!(run_err("potencia(-1, 0.5)").contains("indefinida"));
        assert!(run_err("pi()").contains("chamar"));
    }

    #[test]
    fn escreva_joins_args_with_space() {
        assert_eq!(run(r#"escreva("a", 1, verdadeiro)"#), "a 1 verdadeiro\n");
        assert_eq!(run(r#"escreva()"#), "\n");
    }

    #[test]
    fn sair_rejects_bad_args() {
        assert!(run_err(r#"sair("x")"#).contains("esperado numero"));
        assert!(run_err("sair(1, 2)").contains("espera 0 ou 1"));
        assert!(run_err("sair(1.5)").contains("inteiro"));
    }

    #[test]
    fn remova_list_text_map() {
        assert_eq!(
            run(r#"
xs = [10, 20, 30, 40]
escreva(xs.remova(2))
escreva(xs)
escreva(xs.remova(2, 3))
"#),
            "[10, 30, 40]\n[10, 20, 30, 40]\n[10, 40]\n"
        );
        assert_eq!(run(r#"escreva("abcd".remova(2, 3))"#), "ad\n");
        assert_eq!(run(r#"escreva("olá".remova(2))"#), "oá\n");
        assert_eq!(
            run(r#"
p = mapa([
    "nome" -> "Ana",
    "idade" -> 25,
])
p = p.remova("idade")
escreva(p contem "idade")
escreva(p:nome)
"#),
            "falso\nAna\n"
        );
        assert_eq!(run(r#"escreva([10].remova(2))"#), "[10]\n");
        assert!(run_err(r#"mapa(["a" -> 1]).remova("b")"#).contains("não existe"));
        assert!(run_err(r#"[1, 2].remova(2, 1)"#).contains("maior que o fim"));
        assert!(run_err(r#"mapa(["a" -> 1]).remova("a", "b")"#).contains("chave"));
    }

    #[test]
    pub(super) fn sem_acento_strips_portuguese() {
        assert_eq!(run(r#"escreva(sem_acento("São Paulo"))"#), "Sao Paulo\n");
        assert_eq!(run(r#"escreva(sem_acento("olá"))"#), "ola\n");
        assert_eq!(run(r#"escreva(sem_acento("AÇÃO"))"#), "ACAO\n");
        assert_eq!(run(r#"escreva(sem_acento("açaí"))"#), "acai\n");
        assert_eq!(run(r#"escreva(sem_acento("é"))"#), "e\n");
        assert_eq!(run(r#"escreva(sem_acento("abc"))"#), "abc\n");
        assert_eq!(run(r#"escreva(sem_acento(""))"#), "\n");
        // e + combining acute (NFD)
        assert_eq!(run("escreva(sem_acento(\"e\u{0301}\"))"), "e\n");
        assert!(run_err("sem_acento(1)").contains("texto"));
    }

    #[test]
    fn formate_alignment_and_tables() {
        assert_eq!(
            run(r#"escreva(formate("Hello {:<5}!", "x"))"#),
            "Hello x    !\n"
        );
        assert_eq!(
            run(r#"escreva(formate("Hello {:-<5}!", "x"))"#),
            "Hello x----!\n"
        );
        assert_eq!(
            run(r#"escreva(formate("Hello {:^5}!", "x"))"#),
            "Hello   x  !\n"
        );
        assert_eq!(
            run(r#"escreva(formate("Hello {:>5}!", "x"))"#),
            "Hello     x!\n"
        );
        assert_eq!(
            run(r#"escreva(formate("{:<8} {:>6}", "Ana", 7.5))"#),
            "Ana         7,5\n"
        );
        assert_eq!(run(r#"escreva("{1} {1}".formate("oi"))"#), "oi oi\n");
        assert!(run_err(r#"formate("{}")"#).contains("espera pelo menos"));
        assert!(run_err(r#"formate(10, "x")"#).contains("texto"));
    }

    #[test]
    fn cls_writes_ansi_clear() {
        use crate::runtime::CLEAR_SCREEN;
        assert_eq!(run("cls()"), CLEAR_SCREEN);
        assert_eq!(run("limpe_tela()"), CLEAR_SCREEN);
        assert_eq!(
            run(r#"
escreva("a")
cls()
escreva("b")
"#),
            format!("a\n{CLEAR_SCREEN}b\n")
        );
        assert!(run_err("cls(1)").contains("não espera argumentos"));
        assert!(run_err("limpe_tela(\"x\")").contains("não espera argumentos"));
    }

    #[test]
    fn casa_writes_cursor_home() {
        use crate::runtime::CURSOR_HOME;
        assert_eq!(run("casa()"), CURSOR_HOME);
        assert_eq!(
            run(r#"
escreva("a")
casa()
escreva("b")
"#),
            format!("a\n{CURSOR_HOME}b\n")
        );
        assert!(run_err("casa(1)").contains("não espera argumentos"));
    }

    #[test]
    fn pinte_wraps_ansi_and_reset() {
        assert_eq!(
            run(r#"escreva(pinte("HP", :verde))"#),
            "\x1b[32mHP\x1b[0m\n"
        );
        assert_eq!(
            run(r#"escreva("HP".pinte(:branco, :vermelho))"#),
            "\x1b[37;41mHP\x1b[0m\n"
        );
        assert_eq!(
            run(r#"escreva("HP".pinte(:verde, :normal))"#),
            "\x1b[32;49mHP\x1b[0m\n"
        );
        assert_eq!(
            run(r#"escreva("HP".pinte(:normal, :azul))"#),
            "\x1b[39;44mHP\x1b[0m\n"
        );
        assert_eq!(
            run(r#"escreva("    ".fundo(:verde))"#),
            "\x1b[42m    \x1b[0m\n"
        );
        assert_eq!(run(r#"escreva("GO".negrito())"#), "\x1b[1mGO\x1b[0m\n");
        assert!(run_err(r#"pinte("HP", :roxo)"#).contains("cor desconhecida"));
        assert!(run_err(r#"pinte("HP")"#).contains("2 ou 3"));
        assert!(run_err(r#"fundo("HP")"#).contains("esperado 2"));
        assert!(run_err("negrito(1)").contains("texto"));
        assert_eq!(
            run(r#"escreva(pinte("x", :rosa) se_falhar "ops")"#),
            "ops\n"
        );
    }

    #[test]
    fn tela_quadro_bloco_escreva_em() {
        assert_eq!(run("escreva(colunas() >= 1)"), "verdadeiro\n");
        assert_eq!(run("escreva(linhas() >= 1)"), "verdadeiro\n");
        let nl = super::tela::NOVA_LINHA;
        assert_eq!(run(r#"quadro("oi")"#), "\x1b[Hoi\x1b[K\x1b[J");
        assert_eq!(
            run(r#"quadro("a\nb")"#),
            format!("\x1b[Ha\x1b[K{nl}b\x1b[K\x1b[J")
        );
        assert_eq!(
            run(r#"quadro("a\r\nb")"#),
            format!("\x1b[Ha\x1b[K{nl}b\x1b[K\x1b[J")
        );
        assert_eq!(run(r#"escreva(nova_linha contem "\n")"#), "verdadeiro\n");
        assert!(run_err("nova_linha()").contains("chamar"));
        assert_eq!(
            run(r#"
b = bloco(1, 8)
escreva(b:linha)
escreva(b:coluna)
b.escreva_em("X")
escreva(b:linha)
"#),
            "1\n8\n\x1b[1;8HX\x1b[K2\n"
        );
        assert_eq!(
            run(r#"
b = bloco(2, 3)
b.escreva_em("A").escreva_em("B")
escreva(b:linha)
"#),
            "\x1b[2;3HA\x1b[K\x1b[3;3HB\x1b[K4\n"
        );
        assert!(run_err("bloco(0, 1)").contains(">= 1"));
        assert!(run_err(r#"escreva_em(1, "a")"#).contains("mapa"));
        assert!(run_err(r#"bloco(1, 1).escreva_em("a\nb")"#).contains("uma linha"));
    }

    #[test]
    fn durma_zero_and_errors() {
        assert_eq!(run("durma(0)"), "");
        assert!(run_err("durma(-1)").contains(">= 0"));
        assert!(run_err("durma()").contains("esperado 1"));
        assert!(run_err(r#"durma("x")"#).contains("numero"));
    }

    #[test]
    fn durma_hits_time_limit() {
        use std::time::Duration;

        use crate::runtime::run_to_string_with;
        let err = run_to_string_with(
            "durma(1)",
            "t.lep",
            "",
            None,
            Some(Duration::from_millis(30)),
        )
        .expect_err("should time out");
        assert!(err.message.contains("tempo esgotado"), "{err}");
    }

    #[test]
    fn eh_terminal_is_false_on_captured_stdin() {
        assert_eq!(run("escreva(eh_terminal())"), "falso\n");
        assert_eq!(run("escreva(é_terminal())"), "falso\n");
        assert!(run_err("eh_terminal(1)").contains("não espera argumentos"));
    }

    #[test]
    fn leia_linhas_rejects_args() {
        assert!(run_err(r#"leia_linhas(1)"#).contains("não espera argumentos"));
    }

    #[test]
    fn escreva_erro_goes_to_stderr() {
        use crate::runtime::run_to_strings_with;
        let (out, err) = run_to_strings_with(
            r#"escreva("ok")
escreva_erro("falhou")"#,
            "t.lep",
            "",
        )
        .unwrap();
        assert_eq!(out, "ok\n");
        assert_eq!(err, "falhou\n");
    }

    #[test]
    fn zeros_uns_cheia() {
        assert_eq!(
            run(r#"
Z = zeros(2, 3)
escreva(nlinhas(Z))
escreva(ncolunas(Z))
escreva(Z[1, 1])
escreva(Z[2, 3])
Q = zeros(2)
escreva(ncolunas(Q))
U = uns(1, 4)
escreva(U[1, 4])
C = cheia(2, 3, 7)
escreva(C[2, 1])
C[1, 2] = 9
escreva(C[1, 2])
"#),
            "2\n3\n0\n0\n2\n1\n7\n9\n"
        );
        assert!(run_err("zeros(0)").contains(">= 1"));
        assert!(run_err("uns(-1, 2)").contains(">= 1"));
        assert!(run_err("cheia(2, 3)").contains("esperado 3"));
        assert!(run_err(r#"cheia(2, 3, "x")"#).contains("numero"));
    }

    #[test]
    fn nlinhas_e_ncolunas() {
        assert_eq!(
            run(r#"
A = matriz([
    [1, 2, 3],
    [4, 5, 6],
])
escreva(nlinhas(A))
escreva(ncolunas(A))
escreva(A.nlinhas())
escreva(A.ncolunas())
"#),
            "2\n3\n2\n3\n"
        );
        assert_eq!(run("escreva(nlinhas(identidade(1)))"), "1\n");
        assert_eq!(run("escreva(ncolunas(identidade(1)))"), "1\n");
        assert_eq!(
            run("escreva(ncolunas(transposta(matriz([[1, 2, 3]]))))"),
            "1\n"
        );
        assert!(run_err("nlinhas([1, 2])").contains("esperado matriz"));
        assert!(run_err("ncolunas(10)").contains("esperado matriz"));
        assert!(run_err("nlinhas()").contains("esperado 1 argumento"));
        assert!(run_err("ncolunas(identidade(2), 1)").contains("esperado 1 argumento"));
    }

    #[test]
    fn tamanho_on_text_list_and_map() {
        assert_eq!(run(r#"escreva(tamanho("olá"))"#), "3\n");
        assert_eq!(run(r#"escreva(tamanho([1, 2, 3]))"#), "3\n");
        assert_eq!(
            run(r#"escreva(tamanho(mapa(["a" -> 1, "b" -> 2])))"#),
            "2\n"
        );
        assert!(run_err("tamanho(10)").contains("espera texto, lista, mapa, conjunto ou matriz"));
    }

    #[test]
    fn primeiro_e_ultimo() {
        assert_eq!(run(r#"escreva(primeiro([10, 20, 30]))"#), "10\n");
        assert_eq!(run(r#"escreva(ultimo([10, 20, 30]))"#), "30\n");
        assert!(run_err("primeiro([])").contains("lista vazia"));
        assert!(run_err("ultimo([])").contains("lista vazia"));
    }

    #[test]
    fn text_transforms() {
        assert_eq!(run(r#"escreva(maiuscula("olá"))"#), "OLÁ\n");
        assert_eq!(run(r#"escreva(minuscula("Olá"))"#), "olá\n");
        assert_eq!(
            run(r#"escreva(substitua("Maria Silva", "Maria", "Ana"))"#),
            "Ana Silva\n"
        );
        assert_eq!(run(r#"escreva(substitua("aaa", "a", "b"))"#), "bbb\n");
        assert_eq!(run(r#"escreva(limpe("  x  "))"#), "x\n");
    }

    #[test]
    fn procurar_first_index_or_zero() {
        assert_eq!(run(r#"escreva(procurar("onde", "n"))"#), "2\n");
        assert_eq!(run(r#"escreva("onde".procurar("n"))"#), "2\n");
        assert_eq!(run(r#"escreva(procurar("onde", "a"))"#), "0\n");
        assert_eq!(run(r#"escreva(procurar("banana", "na"))"#), "3\n");
        assert_eq!(run(r#"escreva(procurar("ação", "ç"))"#), "2\n");
        assert_eq!(run(r#"escreva(procurar("abc", "abc"))"#), "1\n");
        assert_eq!(run(r#"escreva(procurar([10, 20, 30], 20))"#), "2\n");
        assert_eq!(run(r#"escreva(procurar([10, 20], 99))"#), "0\n");
        assert!(run_err(r#"procurar("onde", "")"#).contains("não vazio"));
        assert!(run_err("procurar(10, 1)").contains("texto ou lista"));
        assert!(run_err(r#"procurar("onde", 1)"#).contains("esperado texto"));
    }

    #[test]
    fn catalogo_lists_natives_user_functions_and_modules() {
        assert_eq!(
            run(r#"
achou = 0
para f em catalogo() {
    se f:nome == "escreva" e f:tipo == "nativa" {
        achou = 1
    }
}
escreva(achou)
"#),
            "1\n"
        );
        assert_eq!(
            run(r#"
soma = funcao(a, b) { a + b }
para f em catalogo() {
    se f:nome == "soma" {
        escreva(f:tipo)
        escreva(f:args)
    }
}
"#),
            "funcao\na, b\n"
        );
        assert_eq!(
            run(r#"
para f em catalogo() {
    se f:nome == "raiz" {
        escreva(f:modulo)
        escreva(f:args)
    }
}
"#),
            "mat\nnumero\n"
        );
        assert_eq!(
            run(r#"
tela = importe "tela"
para f em catalogo() {
    se f:nome == "tela" {
        escreva(f:tipo)
    }
}
"#),
            "modulo\n"
        );
        assert_eq!(
            run(r#"escreva(tamanho(catálogo()))"#),
            run(r#"escreva(tamanho(catalogo()))"#)
        );
        assert_eq!(
            run(r#"
c = mapa([
    :nome -> "Thiago",
    :mais -> funcao(x) { x + 1 }
])
para f em catalogo(c) {
    se f:nome == "mais" {
        escreva(f:tipo)
        escreva(f:args)
    }
}
escreva(c)
"#),
            "funcao\nx\nmapa{\"nome\": \"Thiago\", \"mais\": <funcao(x)>}\n"
        );
        assert_eq!(
            run(r#"
para f em catalogo("matriz") {
    se f:nome == "zeros" {
        escreva(f:modulo)
        escreva(f:tipo)
    }
}
"#),
            "matriz\nnativa\n"
        );
        assert_eq!(
            run(r#"
para f em catalogo("mat") {
    se f:nome == "pi" {
        escreva(f:tipo)
    }
}
"#),
            "numero\n"
        );
        assert_eq!(
            run(r#"
para f em catalogo("tela") {
    se f:nome == "nova_linha" {
        escreva(f:tipo)
    }
}
"#),
            "texto\n"
        );
        assert_eq!(
            run(r#"
inc = funcao(x) { x + 1 }
para f em catalogo(inc) {
    escreva(f:args)
    escreva(f:tipo)
}
"#),
            "x\nfuncao\n"
        );
        assert!(run_err("catalogo(1)").contains("módulo, mapa, função"));
        assert!(run_err("catalogo(1, 2)").contains("0 ou 1"));
    }

    #[test]
    fn ajuda_returns_ficha() {
        let out = run(r#"escreva(ajuda("escreva"))"#);
        assert!(out.contains("escreva("), "{out}");
        assert!(out.contains("Imprime"), "{out}");
        let out = run(r#"escreva(ajuda("matriz::zeros"))"#);
        assert!(out.contains("zeros"), "{out}");
        assert!(out.contains("importe \"matriz\""), "{out}");
        let out = run(r#"
/// dobra o valor
dobro = funcao(x) { x * 2 }
escreva(ajuda(dobro))
"#);
        assert!(out.contains("funcao(x)"), "{out}");
        assert!(out.contains("dobra o valor"), "{out}");
        assert!(run_err(r#"ajuda("nao_existe")"#).contains("ficha"));
    }

    #[test]
    fn separe_and_junte() {
        assert_eq!(
            run(r#"escreva(separe("a,b,c", ","))"#),
            "[\"a\", \"b\", \"c\"]\n"
        );
        assert_eq!(
            run(r#"escreva(separe("a,,b", ","))"#),
            "[\"a\", \"\", \"b\"]\n"
        );
        assert_eq!(
            run(r#"escreva(separe("ola", ""))"#),
            "[\"o\", \"l\", \"a\"]\n"
        );
        assert_eq!(run(r#"escreva(junte(["a", "b"], " - "))"#), "a - b\n");
        assert_eq!(run(r#"escreva(junte([1, 2], ","))"#), "1,2\n");
        assert_eq!(run(r#"escreva(junte([], ","))"#), "\n");
    }

    #[test]
    fn file_roundtrip_relative_to_script_dir() {
        let dir = temp_dir();
        let out = run_in(
            &dir,
            r#"
salve_arquivo("notas.txt", ["7.5", "8"])
adicione_arquivo("notas.txt", ["9"])
linhas = leia_arquivo("notas.txt")
escreva(junte(linhas, "|"))
"#,
        );
        assert_eq!(out, "7.5|8|9\n");
        assert_eq!(
            fs::read_to_string(dir.join("notas.txt")).unwrap(),
            "7.5\n8\n9\n"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn missing_file_is_a_runtime_error() {
        let msg = run_err(r#"leia_arquivo("nao-existe-expressa.txt")"#);
        assert!(msg.contains("não foi possível ler"), "{msg}");
    }

    #[test]
    fn csv_roundtrip() {
        let dir = temp_dir();
        let out = run_in(
            &dir,
            r#"
salve_csv("pessoas.csv", [["Ana", 25], ["Bruno", 30]])
dados = leia_csv("pessoas.csv")
escreva(dados[1][1])
escreva(dados[2][2])
"#,
        );
        assert_eq!(out, "Ana\n30\n");
        assert_eq!(
            fs::read_to_string(dir.join("pessoas.csv")).unwrap(),
            "Ana,25\nBruno,30\n"
        );
        let _ = fs::remove_dir_all(&dir);
    }
}
