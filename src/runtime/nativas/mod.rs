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
mod tela;

pub struct Nativa {
    pub names: &'static [&'static str],
    pub module: Option<&'static str>,
}

pub const NATIVAS: &[Nativa] = &[
    Nativa { names: &["escreva"], module: None },
    Nativa { names: &["escreva_erro"], module: None },
    Nativa { names: &["sair"], module: None },
    Nativa { names: &["durma"], module: None },
    Nativa { names: &["leia"], module: None },
    Nativa { names: &["leia_linhas"], module: None },
    Nativa { names: &["numero"], module: None },
    Nativa { names: &["formato"], module: None },
    Nativa { names: &["formate"], module: None },
    Nativa { names: &["avaliar"], module: None },
    Nativa { names: &["tamanho"], module: None },
    Nativa { names: &["primeiro"], module: None },
    Nativa { names: &["ultimo"], module: None },
    Nativa { names: &["maiuscula"], module: None },
    Nativa { names: &["minuscula"], module: None },
    Nativa { names: &["sem_acento"], module: None },
    Nativa { names: &["remova"], module: None },
    Nativa { names: &["substitua"], module: None },
    Nativa { names: &["separe"], module: None },
    Nativa { names: &["junte"], module: None },
    Nativa { names: &["limpe"], module: None },
    Nativa { names: &["mapa"], module: None },
    Nativa { names: &["conjunto"], module: None },
    Nativa { names: &["cls", "limpe_tela"], module: Some("tela") },
    Nativa { names: &["casa"], module: Some("tela") },
    Nativa { names: &["eh_terminal", "é_terminal"], module: Some("tela") },
    Nativa { names: &["pinte"], module: Some("tela") },
    Nativa { names: &["fundo"], module: Some("tela") },
    Nativa { names: &["negrito"], module: Some("tela") },
    Nativa { names: &["raiz"], module: Some("mat") },
    Nativa { names: &["aleatorio", "aleatório"], module: Some("mat") },
    Nativa { names: &["semente"], module: Some("mat") },
    Nativa { names: &["matriz"], module: Some("matriz") },
    Nativa { names: &["transposta"], module: Some("matriz") },
    Nativa { names: &["det"], module: Some("matriz") },
    Nativa { names: &["identidade"], module: Some("matriz") },
    Nativa { names: &["zeros"], module: Some("matriz") },
    Nativa { names: &["uns"], module: Some("matriz") },
    Nativa { names: &["cheia"], module: Some("matriz") },
    Nativa { names: &["nlinhas"], module: Some("matriz") },
    Nativa { names: &["ncolunas"], module: Some("matriz") },
    Nativa { names: &["leia_arquivo"], module: Some("arquivo") },
    Nativa { names: &["salve_arquivo"], module: Some("arquivo") },
    Nativa { names: &["adicione_arquivo"], module: Some("arquivo") },
    Nativa { names: &["leia_csv"], module: Some("arquivo") },
    Nativa { names: &["salve_csv"], module: Some("arquivo") },
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
            "raiz" => self.bi_raiz(args, span),
            "aleatorio" | "aleatório" => self.bi_aleatorio(args, span),
            "semente" => self.bi_semente(args, span),
            "matriz" => self.bi_matriz(args, span),
            "transposta" => self.bi_transposta(args, span),
            "det" => self.bi_det(args, span),
            "identidade" => self.bi_identidade(args, span),
            "zeros" => self.bi_zeros(args, span),
            "uns" => self.bi_uns(args, span),
            "cheia" => self.bi_cheia(args, span),
            "nlinhas" => self.bi_nlinhas(args, span),
            "ncolunas" => self.bi_ncolunas(args, span),
            "numero" => self.bi_numero(args, span),
            "formato" => self.bi_formato(args, span),
            "formate" => self.bi_formate(args, span),
            "avaliar" => self.bi_avaliar(args, span, env),
            "tamanho" => self.bi_tamanho(args, span),
            "primeiro" => self.bi_primeiro(args, span),
            "ultimo" => self.bi_ultimo(args, span),
            "maiuscula" => self.bi_maiuscula(args, span),
            "minuscula" => self.bi_minuscula(args, span),
            "sem_acento" => self.bi_sem_acento(args, span),
            "remova" => self.bi_remova(args, span),
            "substitua" => self.bi_substitua(args, span),
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

pub(crate) struct BuiltinDoc {
    pub name: &'static str,
    pub sig: &'static str,
    pub summary: &'static str,
    pub example: &'static str,
}

pub(crate) const BUILTIN_DOCS: &[BuiltinDoc] = &[
    BuiltinDoc {
        name: "escreva",
        sig: "escreva(valor, ...)",
        summary: "Imprime os argumentos separados por espaço e quebra a linha.",
        example: r#"escreva("Olá", 10)"#,
    },
    BuiltinDoc {
        name: "escreva_erro",
        sig: "escreva_erro(valor, ...)",
        summary: "Como escreva, mas na saída de erro (stderr). Não vai para arquivos com >.",
        example: r#"escreva_erro("nome vazio")"#,
    },
    BuiltinDoc {
        name: "sair",
        sig: "sair()  ou  sair(codigo)",
        summary: "Encerra o programa. Sem argumento, código 0; sair(1) indica falha.",
        example: r#"se nome == "" { sair(1) }"#,
    },
    BuiltinDoc {
        name: "cls",
        sig: "cls()",
        summary: "Limpa a tela do terminal. Também limpe_tela().",
        example: r#"cls()"#,
    },
    BuiltinDoc {
        name: "casa",
        sig: "casa()",
        summary: "Cursor no canto, sem apagar. Use no lugar de cls() em animações.",
        example: r#"casa()"#,
    },
    BuiltinDoc {
        name: "durma",
        sig: "durma(segundos)",
        summary: "Espera o número de segundos (aceita fração: durma(0.5)).",
        example: r#"durma(1)"#,
    },
    BuiltinDoc {
        name: "leia",
        sig: "leia()  ou  leia(prompt)",
        summary: "Lê uma linha do teclado (sem o Enter). Prompt opcional.",
        example: r#"nome = leia("Seu nome:")"#,
    },
    BuiltinDoc {
        name: "leia_linhas",
        sig: "leia_linhas() -> lista",
        summary: "Lê todas as linhas da entrada padrão (pipe/arquivo) até o fim.",
        example: r#"para linha em leia_linhas() { escreva(linha) }"#,
    },
    BuiltinDoc {
        name: "eh_terminal",
        sig: "eh_terminal() -> bool",
        summary: "Verdadeiro se a entrada é o teclado (não um pipe). Também é_terminal().",
        example: r#"se eh_terminal() { nome = leia("Nome: ") }"#,
    },
    BuiltinDoc {
        name: "pinte",
        sig: "pinte(texto, frente)  ou  pinte(texto, frente, fundo) -> texto",
        summary: "Devolve o texto com cor ANSI (letra e fundo opcional) e reset no fim. Cores: normal, preto, vermelho, verde, amarelo, azul, magenta, ciano, branco.",
        example: r#""HP".pinte(:verde)"#,
    },
    BuiltinDoc {
        name: "fundo",
        sig: "fundo(texto, cor) -> texto",
        summary: "Devolve o texto com cor de fundo ANSI e reset no fim.",
        example: r#""    ".fundo(:vermelho)"#,
    },
    BuiltinDoc {
        name: "negrito",
        sig: "negrito(texto) -> texto",
        summary: "Devolve o texto em negrito (ANSI) e reset no fim.",
        example: r#""GAME OVER".negrito()"#,
    },
    BuiltinDoc {
        name: "numero",
        sig: "numero(texto|numero) -> numero",
        summary: "Transforma texto em número (aceita 3.14 ou 3,14). Erro se não for número.",
        example: r#"idade = numero(leia("Idade: ")) se_falhar 0"#,
    },
    BuiltinDoc {
        name: "formato",
        sig: r#"formato("pt")  ou  formato("en")"#,
        summary: "Escolhe o padrão de texto de números: pt-BR (1.000,5) ou en-US (1,000.5).",
        example: r#"formato("en")"#,
    },
    BuiltinDoc {
        name: "formate",
        sig: r#"formate("modelo", valores…) -> texto"#,
        summary: "Monta um texto: {} valor, {:<n} esquerda, {:>n} direita, {:^n} centro.",
        example: r#"formate("{:<8} {:>5}", "Ana", 10)"#,
    },
    BuiltinDoc {
        name: "avaliar",
        sig: "avaliar(texto) -> valor",
        summary: "Executa o texto como código Expressa no escopo atual e devolve o último valor.",
        example: r#"avaliar("2 + 3 * 4")    // 14"#,
    },
    BuiltinDoc {
        name: "transposta",
        sig: "transposta(matriz) -> matriz",
        summary: "Troca linhas por colunas.",
        example: "transposta(A)",
    },
    BuiltinDoc {
        name: "det",
        sig: "det(matriz) -> numero",
        summary: "Determinante (1×1, 2×2 ou 3×3).",
        example: "det(A)",
    },
    BuiltinDoc {
        name: "identidade",
        sig: "identidade(n) -> matriz",
        summary: "Matriz identidade n×n.",
        example: "identidade(3)",
    },
    BuiltinDoc {
        name: "zeros",
        sig: "zeros(n)  ou  zeros(linhas, colunas) -> matriz",
        summary: "Matriz só de zeros. Um argumento: n×n.",
        example: "zeros(2, 3)",
    },
    BuiltinDoc {
        name: "uns",
        sig: "uns(n)  ou  uns(linhas, colunas) -> matriz",
        summary: "Matriz só de uns. Um argumento: n×n.",
        example: "uns(2, 3)",
    },
    BuiltinDoc {
        name: "cheia",
        sig: "cheia(linhas, colunas, valor) -> matriz",
        summary: "Matriz retangular preenchida com o mesmo número.",
        example: "cheia(2, 3, 7)",
    },
    BuiltinDoc {
        name: "nlinhas",
        sig: "nlinhas(matriz) -> numero",
        summary: "Quantidade de linhas da matriz.",
        example: "nlinhas(A)    // 2",
    },
    BuiltinDoc {
        name: "ncolunas",
        sig: "ncolunas(matriz) -> numero",
        summary: "Quantidade de colunas da matriz.",
        example: "ncolunas(A)    // 3",
    },
    BuiltinDoc {
        name: "raiz",
        sig: "raiz(numero) -> numero",
        summary: "Raiz quadrada. Erro se o número for negativo (use se_falhar).",
        example: "raiz(9)    // 3",
    },
    BuiltinDoc {
        name: "aleatorio",
        sig: "aleatorio(min, max) -> numero",
        summary: "Inteiro ao acaso entre min e max (inclusive). Também aleatório().",
        example: "aleatorio(1, 6)    // dado",
    },
    BuiltinDoc {
        name: "semente",
        sig: "semente(n)",
        summary: "Fixa a sequência de aleatorio() (útil para repetir um teste).",
        example: "semente(1)",
    },
    BuiltinDoc {
        name: "tamanho",
        sig: "tamanho(texto|lista|mapa|conjunto|matriz) -> numero",
        summary: "Quantidade de caracteres, itens, pares ou linhas.",
        example: r#"tamanho("olá")    // 3"#,
    },
    BuiltinDoc {
        name: "primeiro",
        sig: "primeiro(lista) -> valor",
        summary: "Primeiro elemento da lista (índice 1). Erro se vazia.",
        example: "primeiro([10, 20])    // 10",
    },
    BuiltinDoc {
        name: "ultimo",
        sig: "ultimo(lista) -> valor",
        summary: "Último elemento da lista. Erro se vazia.",
        example: "ultimo([10, 20])    // 20",
    },
    BuiltinDoc {
        name: "maiuscula",
        sig: "maiuscula(texto) -> texto",
        summary: "Copia o texto em letras maiúsculas.",
        example: r#"maiuscula("olá")    // "OLÁ""#,
    },
    BuiltinDoc {
        name: "minuscula",
        sig: "minuscula(texto) -> texto",
        summary: "Copia o texto em letras minúsculas.",
        example: r#"minuscula("Olá")    // "olá""#,
    },
    BuiltinDoc {
        name: "sem_acento",
        sig: "sem_acento(texto) -> texto",
        summary: "Tira acentos e cedilha: ã/á→a, é→e, ç→c. Não muda maiúscula.",
        example: r#"sem_acento("São Paulo")    // "Sao Paulo""#,
    },
    BuiltinDoc {
        name: "remova",
        sig: "remova(lista|texto, i)  ou  remova(..., inicio, fim)  ou  remova(mapa, chave)",
        summary: "Devolve uma cópia sem o índice/faixa (1…n) ou sem a chave do mapa.",
        example: r#"xs = xs.remova(2)    // sem o 2º item"#,
    },
    BuiltinDoc {
        name: "substitua",
        sig: "substitua(texto, antigo, novo) -> texto",
        summary: "Troca todas as ocorrências de antigo por novo.",
        example: r#"substitua("aa", "a", "b")    // "bb""#,
    },
    BuiltinDoc {
        name: "separe",
        sig: "separe(texto, separador) -> lista",
        summary: "Parte o texto. Separador \"\" gera um item por caractere.",
        example: r#"separe("a,b", ",")    // ["a", "b"]"#,
    },
    BuiltinDoc {
        name: "junte",
        sig: "junte(lista, separador) -> texto",
        summary: "Junta os itens da lista com o separador no meio.",
        example: r#"junte(["a", "b"], "-")    // "a-b""#,
    },
    BuiltinDoc {
        name: "limpe",
        sig: "limpe(texto) -> texto",
        summary: "Remove espaços do começo e do fim.",
        example: r#"limpe("  x  ")    // "x""#,
    },
    BuiltinDoc {
        name: "leia_arquivo",
        sig: "leia_arquivo(caminho) -> lista",
        summary: "Lê o arquivo; cada linha vira um texto (sem \\n).",
        example: r#"leia_arquivo("dados.txt") se_falhar []"#,
    },
    BuiltinDoc {
        name: "salve_arquivo",
        sig: "salve_arquivo(caminho, linhas)",
        summary: "Grava a lista de textos, substituindo o arquivo.",
        example: r#"salve_arquivo("saida.txt", ["a", "b"])"#,
    },
    BuiltinDoc {
        name: "adicione_arquivo",
        sig: "adicione_arquivo(caminho, linhas)",
        summary: "Acrescenta linhas no fim do arquivo (cria se não existir).",
        example: r#"adicione_arquivo("saida.txt", ["c"])"#,
    },
    BuiltinDoc {
        name: "leia_csv",
        sig: "leia_csv(caminho) -> lista",
        summary: "Lê CSV simples (vírgula, sem aspas). Lista de listas de textos.",
        example: r#"leia_csv("notas.csv")"#,
    },
    BuiltinDoc {
        name: "salve_csv",
        sig: "salve_csv(caminho, dados)",
        summary: "Grava uma lista de listas como CSV.",
        example: r#"salve_csv("saida.csv", [["Ana", 25]])"#,
    },
    BuiltinDoc {
        name: "mapa",
        sig: "mapa()  ou  mapa(pares) -> mapa",
        summary: "Monta um mapa a partir de uma lista de pares `chave -> valor`.",
        example: r#"mapa([:nome -> "Ana", :idade -> 25])"#,
    },
    BuiltinDoc {
        name: "conjunto",
        sig: "conjunto()  ou  conjunto(xs) -> conjunto",
        summary: "Monta um conjunto a partir de uma lista (valores únicos).",
        example: r#"conjunto([:ana, :bia])"#,
    },
    BuiltinDoc {
        name: "matriz",
        sig: "matriz(linhas) -> matriz",
        summary: "Monta uma matriz a partir de uma lista de listas de números.",
        example: r#"matriz([[1, 2], [3, 4]])"#,
    },
];

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

pub(crate) fn lookup_builtin_doc(name: &str) -> Option<&'static BuiltinDoc> {
    let name = match name {
        "é_terminal" => "eh_terminal",
        "limpe_tela" => "cls",
        "aleatório" => "aleatorio",
        other => other,
    };
    BUILTIN_DOCS.iter().find(|d| d.name == name)
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

    use super::{BUILTIN_DOCS, NATIVAS, lookup_builtin_doc};
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
        let names: Vec<_> = NATIVAS.iter().flat_map(|n| n.names.iter().copied()).collect();
        assert!(names.contains(&"escreva"));
        assert!(names.contains(&"leia"));
        assert!(names.contains(&"leia_linhas"));
        assert!(names.contains(&"raiz"));
        assert!(names.contains(&"numero"));
        assert!(names.contains(&"eh_terminal"));
        assert!(names.contains(&"é_terminal"));
        assert!(names.contains(&"sem_acento"));
        assert!(names.contains(&"remova"));
        assert!(names.contains(&"mapa"));
        assert!(names.contains(&"conjunto"));
        assert!(names.contains(&"matriz"));
        assert_eq!(
            lookup_builtin_doc("é_terminal").map(|d| d.name),
            Some("eh_terminal")
        );
        assert_eq!(
            lookup_builtin_doc("limpe_tela").map(|d| d.name),
            Some("cls")
        );
        assert_eq!(
            lookup_builtin_doc("aleatório").map(|d| d.name),
            Some("aleatorio")
        );
        for name in &names {
            assert!(
                lookup_builtin_doc(name).is_some(),
                "falta ajuda para {name}"
            );
        }
        for n in NATIVAS {
            if n.module.is_none() {
                assert!(n.names.iter().any(|nm| *nm == "escreva" || lookup_builtin_doc(nm).is_some()));
            }
        }
        let _ = BUILTIN_DOCS.len();
    }

    #[test]
    fn avaliar_runs_source_in_current_scope() {
        assert_eq!(run(r#"escreva(avaliar("2 + 3 * 4"))"#), "14\n");
        assert_eq!(run(r#"escreva("10 - 3".avaliar())"#), "7\n");
        assert_eq!(
            run(
                r#"
x = 10
escreva(avaliar("x + 1"))
avaliar("x = 20")
escreva(x)
"#
            ),
            "11\n20\n"
        );
        assert_eq!(
            run(
                r#"
f = funcao(n) { avaliar("n * 2") }
escreva(f(3))
"#
            ),
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
            run(
                r#"
semente(1)
a = aleatorio(1, 6)
semente(1)
b = aleatorio(1, 6)
escreva(a == b)
escreva(aleatorio(5, 5))
"#
            ),
            "verdadeiro\n5\n"
        );
        assert_eq!(
            run(
                r#"
semente(3)
para i de 1 ate 40
inicio
    n = aleatorio(1, 6)
    se n < 1 ou n > 6 { escreva("fora") }
fim
"#
            ),
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
        assert_eq!(
            run(r#"escreva("{1} {1}".formate("oi"))"#),
            "oi oi\n"
        );
        assert!(run_err(r#"formate("{}")"#).contains("espera pelo menos"));
        assert!(run_err(r#"formate(10, "x")"#).contains("texto"));
    }

    #[test]
    fn cls_writes_ansi_clear() {
        use crate::runtime::CLEAR_SCREEN;
        assert_eq!(run("cls()"), CLEAR_SCREEN);
        assert_eq!(run("limpe_tela()"), CLEAR_SCREEN);
        assert_eq!(
            run(
                r#"
escreva("a")
cls()
escreva("b")
"#
            ),
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
            run(
                r#"
escreva("a")
casa()
escreva("b")
"#
            ),
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
        assert_eq!(
            run(r#"escreva("GO".negrito())"#),
            "\x1b[1mGO\x1b[0m\n"
        );
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
            run(
                r#"
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
"#
            ),
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
            run(
                r#"
A = matriz([
    [1, 2, 3],
    [4, 5, 6],
])
escreva(nlinhas(A))
escreva(ncolunas(A))
escreva(A.nlinhas())
escreva(A.ncolunas())
"#
            ),
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
