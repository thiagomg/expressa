use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::lexer::Span;

use super::env::{Env, FrameKind};
use super::error::EvalError;
use super::eval::Vm;
use super::value::{Value, format_numero};

mod arquivo;
mod cartesiano;
mod graf;
mod imagem;
mod mat;
mod matriz;
mod nucleo;
mod processo;
mod referencia;
mod sistema;
mod tela;

pub use referencia::{
    BuiltinDoc, builtin_docs, docs_agrupados, lookup_alvo, lookup_builtin_doc, lookup_ficha,
    tema_de, titulo_modulo,
};

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
        names: &["saia"],
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
        names: &["avalie"],
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
        names: &["tipo"],
        module: None,
    },
    Nativa {
        names: &["tipos"],
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
        names: &["procure"],
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
        names: &["ambiente"],
        module: None,
    },
    Nativa {
        names: &["ordene"],
        module: None,
    },
    Nativa {
        names: &["comeca_com", "começa_com"],
        module: None,
    },
    Nativa {
        names: &["termina_com"],
        module: None,
    },
    Nativa {
        names: &["agora"],
        module: None,
    },
    Nativa {
        names: &["formate_data"],
        module: None,
    },
    Nativa {
        names: &["cls", "limpe_tela"],
        module: Some("tela"),
    },
    Nativa {
        names: &["posicao_cursor", "posição_cursor"],
        module: Some("tela"),
    },
    Nativa {
        names: &["apague_linha"],
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
        names: &["xy"],
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
    Nativa {
        names: &["existe"],
        module: Some("arquivo"),
    },
    Nativa {
        names: &["eh_pasta", "é_pasta"],
        module: Some("arquivo"),
    },
    Nativa {
        names: &["liste"],
        module: Some("arquivo"),
    },
    Nativa {
        names: &["crie_pasta"],
        module: Some("arquivo"),
    },
    Nativa {
        names: &["apague_arquivo"],
        module: Some("arquivo"),
    },
    Nativa {
        names: &["apague_pasta"],
        module: Some("arquivo"),
    },
    Nativa {
        names: &["mova"],
        module: Some("arquivo"),
    },
    Nativa {
        names: &["copie"],
        module: Some("arquivo"),
    },
    Nativa {
        names: &["junte_caminho"],
        module: Some("arquivo"),
    },
    Nativa {
        names: &["nome_do_arquivo"],
        module: Some("arquivo"),
    },
    Nativa {
        names: &["pasta_de"],
        module: Some("arquivo"),
    },
    Nativa {
        names: &["extensao", "extensão"],
        module: Some("arquivo"),
    },
    Nativa {
        names: &["pasta_atual"],
        module: Some("arquivo"),
    },
    Nativa {
        names: &["execute"],
        module: Some("processo"),
    },
    Nativa {
        names: &["linhas_de"],
        module: Some("processo"),
    },
    Nativa {
        names: &["documento"],
        module: Some("cartesiano"),
    },
    Nativa {
        names: &["grafico"],
        module: Some("cartesiano"),
    },
    Nativa {
        names: &["eixos"],
        module: Some("cartesiano"),
    },
    Nativa {
        names: &["plote"],
        module: Some("cartesiano"),
    },
    Nativa {
        names: &["adicione"],
        module: Some("cartesiano"),
    },
    Nativa {
        names: &["mostre"],
        module: Some("cartesiano"),
    },
    Nativa {
        names: &["nova"],
        module: Some("tela_grafica"),
    },
    Nativa {
        names: &["cor"],
        module: Some("tela_grafica"),
    },
    Nativa {
        names: &["fundo"],
        module: Some("tela_grafica"),
    },
    Nativa {
        names: &["linha"],
        module: Some("tela_grafica"),
    },
    Nativa {
        names: &["circulo", "círculo"],
        module: Some("tela_grafica"),
    },
    Nativa {
        names: &["ponto"],
        module: Some("tela_grafica"),
    },
    Nativa {
        names: &["retangulo", "retângulo"],
        module: Some("tela_grafica"),
    },
    Nativa {
        names: &["texto"],
        module: Some("tela_grafica"),
    },
    Nativa {
        names: &["imagem"],
        module: Some("tela_grafica"),
    },
    Nativa {
        names: &["apague"],
        module: Some("tela_grafica"),
    },
    Nativa {
        names: &["pixel"],
        module: Some("tela_grafica"),
    },
    Nativa {
        names: &["como_imagem"],
        module: Some("tela_grafica"),
    },
    Nativa {
        names: &["salve"],
        module: Some("tela_grafica"),
    },
    Nativa {
        names: &["mostre"],
        module: Some("tela_grafica"),
    },
    Nativa {
        names: &["evento"],
        module: Some("tela_grafica"),
    },
    Nativa {
        names: &["carregue"],
        module: Some("imagem"),
    },
    Nativa {
        names: &["salve"],
        module: Some("imagem"),
    },
    Nativa {
        names: &["rotacione"],
        module: Some("imagem"),
    },
    Nativa {
        names: &["redimensione"],
        module: Some("imagem"),
    },
    Nativa {
        names: &["largura"],
        module: Some("imagem"),
    },
    Nativa {
        names: &["altura"],
        module: Some("imagem"),
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
            .define("pi", Value::Numero(std::f64::consts::PI.into()));
    }
    if let Some(tela) = map.get("tela") {
        tela.borrow_mut()
            .define("nova_linha", Value::Texto(tela::NOVA_LINHA.into()));
    }
    map
}

impl Vm<'_, '_> {
    pub(crate) fn call_builtin(
        &mut self,
        name: &str,
        args: &[Value],
        span: Span,
        env: &Rc<RefCell<Env>>,
    ) -> Result<Value, EvalError> {
        match name {
            "escreva" => self.bi_escreva(args, span, env),
            "escreva_erro" => self.bi_escreva_erro(args, span, env),
            "saia" => self.bi_saia(args, span),
            "cls" | "limpe_tela" => self.bi_cls(args, span),
            "casa" => self.bi_casa(args, span),
            "posicao_cursor" | "posição_cursor" => self.bi_posicao_cursor(args, span),
            "apague_linha" => self.bi_apague_linha(args, span),
            "durma" => self.bi_durma(args, span),
            "leia" => self.bi_leia(args, span),
            "leia_linhas" => self.bi_leia_linhas(args, span),
            "eh_terminal" | "é_terminal" => self.bi_eh_terminal(args, span),
            "pinte" => self.bi_pinte(args, span),
            "fundo" => {
                if !args.is_empty() && graf::is_graf(&args[0]) {
                    self.bi_graf_fundo(args, span)
                } else {
                    self.bi_fundo(args, span)
                }
            }
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
            "xy" => self.bi_xy(args, span),
            "numero" | "número" => self.bi_numero(args, span),
            "formato" => self.bi_formato(args, span),
            "formate" => self.bi_formate(args, span, env),
            "avalie" => self.bi_avalie(args, span, env),
            "catalogo" | "catálogo" => self.bi_catalogo(args, span, env),
            "ajuda" => self.bi_ajuda(args, span),
            "tipo" => self.bi_tipo(args, span),
            "tipos" => self.bi_tipos(args, span),
            "tamanho" => self.bi_tamanho(args, span),
            "primeiro" => self.bi_primeiro(args, span),
            "ultimo" => self.bi_ultimo(args, span),
            "maiuscula" => self.bi_maiuscula(args, span),
            "minuscula" => self.bi_minuscula(args, span),
            "sem_acento" => self.bi_sem_acento(args, span),
            "remova" => self.bi_remova(args, span),
            "substitua" => self.bi_substitua(args, span),
            "procure" => self.bi_procure(args, span),
            "separe" => self.bi_separe(args, span),
            "junte" => self.bi_junte(args, span, env),
            "limpe" => self.bi_limpe(args, span),
            "apague" => self.bi_graf_limpe(args, span),
            "evento" => self.bi_graf_evento(args, span),
            "leia_arquivo" => self.bi_leia_arquivo(args, span),
            "salve_arquivo" => self.bi_salve_arquivo(args, span),
            "adicione_arquivo" => self.bi_adicione_arquivo(args, span),
            "leia_csv" => self.bi_leia_csv(args, span),
            "salve_csv" => self.bi_salve_csv(args, span),
            "ambiente" => self.bi_ambiente(args, span),
            "ordene" => self.bi_ordene(args, span, env),
            "comeca_com" | "começa_com" => self.bi_comeca_com(args, span),
            "termina_com" => self.bi_termina_com(args, span),
            "agora" => self.bi_agora(args, span),
            "formate_data" => self.bi_formate_data(args, span),
            "existe" => self.bi_existe(args, span),
            "eh_pasta" | "é_pasta" => self.bi_eh_pasta(args, span),
            "liste" => self.bi_liste(args, span),
            "crie_pasta" => self.bi_crie_pasta(args, span),
            "apague_arquivo" => self.bi_apague_arquivo(args, span),
            "apague_pasta" => self.bi_apague_pasta(args, span),
            "mova" => self.bi_mova(args, span),
            "copie" => self.bi_copie(args, span),
            "junte_caminho" => self.bi_junte_caminho(args, span),
            "nome_do_arquivo" => self.bi_nome_do_arquivo(args, span),
            "pasta_de" => self.bi_pasta_de(args, span),
            "extensao" | "extensão" => self.bi_extensao(args, span),
            "pasta_atual" => self.bi_pasta_atual(args, span),
            "execute" => self.bi_execute(args, span, env),
            "linhas_de" => self.bi_linhas_de(args, span),
            "documento" => self.bi_documento(args, span),
            "grafico" => self.bi_grafico(args, span),
            "eixos" => self.bi_eixos(args, span),
            "plote" => self.bi_plote(args, span, env),
            "adicione" => self.bi_adicione(args, span),
            "nova" => self.bi_graf_nova(args, span),
            "cor" => self.bi_graf_cor(args, span),
            "linha" => self.bi_graf_linha(args, span),
            "circulo" | "círculo" => self.bi_graf_circulo(args, span),
            "ponto" => self.bi_graf_ponto(args, span),
            "retangulo" | "retângulo" => self.bi_graf_retangulo(args, span),
            "texto" => self.bi_graf_texto(args, span),
            "imagem" => self.bi_graf_imagem(args, span),
            "pixel" => self.bi_graf_pixel(args, span),
            "como_imagem" => self.bi_graf_como_imagem(args, span),
            "carregue" => self.bi_img_carregue(args, span),
            "rotacione" => self.bi_img_rotacione(args, span),
            "redimensione" => self.bi_img_redimensione(args, span),
            "largura" => self.bi_img_largura(args, span),
            "altura" => self.bi_img_altura(args, span),
            "salve" => {
                if !args.is_empty() && matches!(&args[0], Value::Imagem(_)) {
                    self.bi_img_salve(args, span)
                } else {
                    self.bi_graf_salve(args, span)
                }
            }
            "mostre" => {
                if !args.is_empty() && graf::is_graf(&args[0]) {
                    self.bi_graf_mostre(args, span)
                } else {
                    self.bi_mostre(args, span)
                }
            }
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
        .find(|n| n.names.contains(&name))
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

pub(super) fn csv_cell(v: &Value) -> String {
    match v {
        Value::Texto(s) => s.clone(),
        Value::Numero(n) => format_numero(*n),
        other => other.to_string(),
    }
}

pub(super) fn lines_to_text(
    lines: &[Value],
    span: Span,
    vm: &Vm<'_, '_>,
) -> Result<String, EvalError> {
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
        assert!(names.contains(&"procure"));
        assert!(names.contains(&"tipo"));
        assert!(names.contains(&"tipos"));
        assert!(names.contains(&"catalogo"));
        assert!(names.contains(&"catálogo"));
        assert!(names.contains(&"mapa"));
        assert!(names.contains(&"conjunto"));
        assert!(names.contains(&"matriz"));
        for (old, new) in [
            ("procurar", "procure"),
            ("avaliar", "avalie"),
            ("plotar", "plote"),
            ("sair", "saia"),
            ("escala", "redimensione"),
        ] {
            assert!(names.contains(&new), "falta registro de {new}");
            assert!(!names.contains(&old), "registro antigo de {old}");
        }

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
    fn tipo_names_match_errors() {
        assert_eq!(run(r#"escreva(tipo(10))"#), "numero\n");
        assert_eq!(run(r#"escreva(tipo("x"))"#), "texto\n");
        assert_eq!(run(r#"escreva(tipo(verdadeiro))"#), "bool\n");
        assert_eq!(run(r#"escreva(tipo([1]))"#), "lista\n");
        assert_eq!(run(r#"escreva(tipo(:n -> 1))"#), "par\n");
        assert_eq!(run(r#"escreva(tipo(mapa()))"#), "mapa\n");
        assert_eq!(run(r#"escreva(tipo(conjunto()))"#), "conjunto\n");
        assert_eq!(run(r#"escreva(tipo(escreva))"#), "funcao\n");
        assert_eq!(
            run(r#"m = importe "mat"
escreva(tipo(m))"#),
            "modulo\n"
        );
        assert_eq!(run(r#"escreva(tipo(10) == "numero")"#), "verdadeiro\n");
        assert_eq!(
            run(r#"f = função(x) { x * 2 }
escreva(tipo(f))"#),
            "funcao\n"
        );
        assert!(run_err("tipo()").contains("argumento"));
        assert_eq!(run(r#"escreva(10.tipo())"#), "numero\n");
        assert_eq!(
            run(r#"escreva(tipos())"#),
            "[\"numero\", \"texto\", \"bool\", \"lista\", \"par\", \"mapa\", \"conjunto\", \"matriz\", \"imagem\", \"funcao\", \"modulo\", \"processo\", \"nada\"]\n"
        );
        assert_eq!(run(r#"escreva(tipos() contem "numero")"#), "verdadeiro\n");
        assert!(run_err("tipos(1)").contains("argumento"));
    }

    #[test]
    fn avaliar_runs_source_in_current_scope() {
        assert_eq!(run(r#"escreva(avalie("2 + 3 * 4"))"#), "14\n");
        assert_eq!(run(r#"escreva("10 - 3".avalie())"#), "7\n");
        assert_eq!(
            run(r#"
x = 10
escreva(avalie("x + 1"))
avalie("x = 20")
escreva(x)
"#),
            "11\n20\n"
        );
        assert_eq!(
            run(r#"
f = funcao(n) { avalie("n * 2") }
escreva(f(3))
"#),
            "6\n"
        );
        assert_eq!(run(r#"escreva(avalie("1 / 0") se_falhar 0)"#), "0\n");
        assert_eq!(run(r#"escreva(avalie("2 +") se_falhar "ops")"#), "ops\n");
        assert!(run_err("avalie(1)").contains("texto"));
        assert!(run_err(r#"avalie("")"#).contains("expressão"));
        let err = run_to_string(r#"avalie("1 / 0")"#, "teste.lep").expect_err("divisão");
        assert!(err.message.contains("divisão por zero"), "{err}");
        assert!(
            err.stack.iter().any(|f| f.name == "avalie"),
            "pilha: {:?}",
            err.stack
        );
    }

    #[test]
    fn aritmetica_decimal_padrao_e_float_opcional() {
        assert_eq!(run("escreva(35.87+12.63+11.05+11.47+10.35)"), "81,37\n");
        assert_eq!(run("escreva(0.1 + 0.2)"), "0,3\n");
        assert_eq!(run("escreva(0.1 + 0.2 == 0.3)"), "verdadeiro\n");
        assert_eq!(
            run(r#"formato("float")
escreva(0.1 + 0.2 == 0.3)"#),
            "falso\n"
        );
        assert_ne!(
            run(r#"formato("float")
escreva(35.87+12.63+11.05+11.47+10.35)"#),
            "81,37\n"
        );
        assert_eq!(
            run(r#"formato("float")
formato("decimal")
escreva(0.1 + 0.2 == 0.3)"#),
            "verdadeiro\n"
        );
        assert_eq!(
            run(r#"formato("float")
formato("en")
escreva(0.1 + 0.2 == 0.3)"#),
            "falso\n"
        );
        assert_eq!(
            run(r#"formato("flutuante")
formato("exato")
escreva(0.1 + 0.2)"#),
            "0,3\n"
        );
        assert!(run_err(r#"formato("xyz")"#).contains("pt"));
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
        assert!(run_err(r#"saia("x")"#).contains("esperado numero"));
        assert!(run_err("saia(1, 2)").contains("espera 0 ou 1"));
        assert!(run_err("saia(1.5)").contains("inteiro"));
    }

    #[test]
    fn remova_list_text_map() {
        assert_eq!(
            run(r#"
xs = [10, 20, 30, 40]
escreva(xs.remova(1))
escreva(xs)
escreva(xs.remova(1, 2))
"#),
            "[10, 30, 40]\n[10, 20, 30, 40]\n[10, 40]\n"
        );
        assert_eq!(run(r#"escreva("abcd".remova(1, 2))"#), "ad\n");
        assert_eq!(run(r#"escreva("olá".remova(1))"#), "oá\n");
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
        assert_eq!(run(r#"escreva("{0} {0}".formate("oi"))"#), "oi oi\n");
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
    fn cursor_movement_and_line_erasure() {
        assert_eq!(
            run("posicao_cursor(bloco(2, 4))\napague_linha()"),
            "\x1b[3;5H\r\x1b[2K"
        );
        assert_eq!(run("posição_cursor(bloco(0, 0))"), "\x1b[1;1H");
        assert!(run_err("posicao_cursor()").contains("exige um terminal"));
        assert!(run_err("posicao_cursor(1)").contains("mapa"));
        assert!(run_err("posicao_cursor(mapa([:linha -> 1]))").contains(":coluna"));
        assert!(run_err("posicao_cursor(bloco(1, 2), 3)").contains("0 ou 1"));
        assert!(run_err("posicao_cursor(mapa([:linha -> -1, :coluna -> 0]))").contains(">= 0"));
        assert!(run_err("apague_linha(1)").contains("0 argumento"));
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
b = bloco(0, 7)
escreva(b:linha)
escreva(b:coluna)
b.escreva_em("X")
escreva(b:linha)
"#),
            "0\n7\n\x1b[1;8HX\x1b[K1\n"
        );
        assert_eq!(
            run(r#"
b = bloco(1, 2)
b.escreva_em("A").escreva_em("B")
escreva(b:linha)
"#),
            "\x1b[2;3HA\x1b[K\x1b[3;3HB\x1b[K3\n"
        );
        assert!(run_err("bloco(-1, 0)").contains(">= 0"));
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
escreva(Z[0, 0])
escreva(Z[1, 2])
Q = zeros(2)
escreva(ncolunas(Q))
U = uns(1, 4)
escreva(U[0, 3])
C = cheia(2, 3, 7)
escreva(C[1, 0])
C[0, 1] = 9
escreva(C[0, 1])
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
    fn matriz_xy_reads_and_writes_in_graphics_order() {
        assert_eq!(
            run(r#"
A = matriz([[1, 2, 3], [4, 5, 6]])
B = A
escreva(A.xy(2, 0), xy(A, 0, 1))
A.xy(2, 0, 9).xy(0, 1, 8)
escreva(A[0, 2], B[1, 0], A.xy(1, 1))
"#),
            "3 4\n9 8 5\n"
        );
        let source = r#"
mat = importe "matriz"
A = mat::zeros(2, 3)
mat::xy(A, 2, 1, 7)
escreva(A.mat::xy(2, 1), mat::xy(A, 2, 1))
"#;
        assert_eq!(run_to_string(source, "teste.lep").unwrap(), "7 7\n");
    }

    #[test]
    fn matriz_xy_rejects_invalid_coordinates_without_mutating() {
        for (x, y) in [
            ("-1", "0"),
            ("0", "-1"),
            ("3", "0"),
            ("0", "2"),
            ("0.5", "0"),
            ("0", "0.5"),
            ("\"x\"", "0"),
            ("0", "\"y\""),
        ] {
            let source = format!(
                r#"
A = cheia(2, 3, 7)
escreva(A.xy({x}, {y}) se_falhar :erro)
escreva(A.xy({x}, {y}, 9) se_falhar :erro)
escreva(A == cheia(2, 3, 7))
"#
            );
            assert_eq!(run(&source), "erro\nerro\nverdadeiro\n", "{x}, {y}");
        }
        assert!(run_err("xy([], 0, 0)").contains("esperado matriz"));
        assert!(run_err("xy([], 0, 0, 1)").contains("esperado matriz"));
        assert!(run_err("xy(zeros(2), 0)").contains("3 ou 4 argumentos"));
        assert!(run_err("xy(zeros(2), 0, 0, 1, 2)").contains("3 ou 4 argumentos"));
        assert_eq!(
            run(r#"
A = cheia(2, 3, 7)
escreva(A.xy(0, 0, "a") se_falhar :erro)
escreva(A == cheia(2, 3, 7))
"#),
            "erro\nverdadeiro\n"
        );
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
    fn procurar_first_index_or_minus_one() {
        assert_eq!(
            run(r#"escreva(procure([10, 20], 10), procure([], 10))"#),
            "0 -1\n"
        );
        assert_eq!(
            run(r#"escreva(procure("á😀z", "😀"), procure("", "z"))"#),
            "1 -1\n"
        );
        assert_eq!(run(r#"escreva(procure("onde", "n"))"#), "1\n");
        assert_eq!(run(r#"escreva("onde".procure("n"))"#), "1\n");
        assert_eq!(run(r#"escreva(procure("onde", "a"))"#), "-1\n");
        assert_eq!(run(r#"escreva(procure("banana", "na"))"#), "2\n");
        assert_eq!(run(r#"escreva(procure("ação", "ç"))"#), "1\n");
        assert_eq!(run(r#"escreva(procure("abc", "abc"))"#), "0\n");
        assert_eq!(run(r#"escreva(procure([10, 20, 30], 20))"#), "1\n");
        assert_eq!(run(r#"escreva(procure([10, 20], 99))"#), "-1\n");
        assert!(run_err(r#"procure("onde", "")"#).contains("não vazio"));
        assert!(run_err("procure(10, 1)").contains("texto ou lista"));
        assert!(run_err(r#"procure("onde", 1)"#).contains("esperado texto"));
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
escreva(dados[0][0])
escreva(dados[1][1])
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
