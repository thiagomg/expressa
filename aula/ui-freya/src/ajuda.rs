//! F1 ficha, catalog of natives, completion, tutorials.

use std::path::PathBuf;
use std::process::{Command, Stdio};

use expressa::runtime::{NATIVAS, builtin_docs, lookup_alvo, lookup_builtin_doc, lookup_ficha};

use crate::lang::{
    KEYWORDS, Completion, DefKind, call_context, completion_context, import_insert_line,
    linha_no_byte, modules, native_module, parse_definitions, parse_imports, prefixo_do_contexto,
    signature_params, word_at,
};

pub fn palavra_em(texto: &str, byte: usize) -> String {
    let (_, linha, col) = linha_no_byte(texto, byte);
    word_at(linha, col).map(|w| w.name).unwrap_or_default()
}

/// Name under the cursor, with `modulo::` when the student wrote a qualifier.
pub fn nome_no_cursor(texto: &str, byte: usize) -> Option<String> {
    let (_, linha, col) = linha_no_byte(texto, byte);
    let w = word_at(linha, col)?;
    if KEYWORDS.contains(&w.name.as_str()) {
        return None;
    }
    Some(match w.alias {
        Some(a) => format!("{a}::{}", w.name),
        None => w.name,
    })
}

pub fn prefixo_em(texto: &str, byte: usize) -> String {
    let byte = byte.min(texto.len());
    prefixo_do_contexto(&completion_context(&texto[..byte])).to_string()
}

pub fn ficha(nome: &str) -> Option<String> {
    let nome = nome.trim();
    if nome.is_empty() || KEYWORDS.contains(&nome) {
        return None;
    }
    lookup_ficha(nome)
}

pub fn catalogo() -> Vec<(String, String)> {
    entradas_catalogo()
        .into_iter()
        .map(|e| (e.chave, e.rotulo))
        .collect()
}

/// Título da janela irmã de F1 / Funções nativas (igual ao GTK).
pub const TITULO_AJUDA: &str = "Ajuda — funções nativas";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EntradaCatalogo {
    pub chave: String,
    pub rotulo: String,
    pub grupo: String,
}

pub fn entradas_catalogo() -> Vec<EntradaCatalogo> {
    builtin_docs()
        .iter()
        .map(|d| {
            let grupo = match d.module.as_deref() {
                None => "Núcleo".to_string(),
                Some(m) => format!("importe \"{m}\""),
            };
            let chave = match d.module.as_deref() {
                Some(m) => format!("{m}::{}", d.name),
                None => d.name.clone(),
            };
            let rotulo = if d.is_value {
                d.name.clone()
            } else {
                chave.clone()
            };
            EntradaCatalogo {
                chave,
                rotulo,
                grupo,
            }
        })
        .collect()
}

/// Hint for the call that contains the cursor (`escreva(«valor», ...)`).
pub fn assinatura(texto: &str, byte: usize) -> Option<String> {
    let byte = byte.min(texto.len());
    let call = call_context(&texto[..byte])?;
    let ativo = call.active + usize::from(call.method);
    let alvo = match &call.alias {
        Some(a) => format!("{a}::{}", call.name),
        None => call.name.clone(),
    };
    let sigs: Vec<String> = if let Some(doc) = lookup_alvo(&alvo).or_else(|| lookup_alvo(&call.name))
    {
        if doc.signatures.is_empty() {
            doc.sig
                .split("  ou  ")
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        } else {
            doc.signatures.clone()
        }
    } else if let Some(def) = parse_definitions(texto)
        .into_iter()
        .find(|d| d.name == call.name)
    {
        match def.kind {
            DefKind::Function(p) => vec![format!("{}({})", def.name, p.join(", "))],
            _ => return None,
        }
    } else {
        return None;
    };
    let sig = sigs
        .iter()
        .find(|s| signature_params(s).len() > ativo || s.contains("..."))
        .or(sigs.first())?;
    let (a, z) = (sig.find('(')?, sig.find(')')?);
    let params = signature_params(sig);
    let shown: Vec<String> = params
        .iter()
        .enumerate()
        .map(|(i, p)| {
            if i == ativo || (p.contains("...") && i <= ativo) {
                format!("«{p}»")
            } else {
                p.clone()
            }
        })
        .collect();
    Some(format!("{}({}){}", &sig[..a], shown.join(", "), &sig[z + 1..]))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sugestao {
    pub rotulo: String,
    pub inserir: String,
    pub importe: Option<String>,
}

pub fn completar(texto: &str, byte: usize) -> Vec<Sugestao> {
    completar_em(texto, byte, "", &[])
}

pub fn completar_em(texto: &str, byte: usize, caminho: &str, arquivos: &[String]) -> Vec<Sugestao> {
    let byte = byte.min(texto.len());
    let before = &texto[..byte];
    let ctx = completion_context(before);
    let prefix = prefixo_do_contexto(&ctx);
    let mut out = Vec::new();
    let mut visto = std::collections::HashSet::new();
    let mut push = |out: &mut Vec<Sugestao>, s: Sugestao| {
        if visto.insert(s.inserir.clone()) {
            out.push(s);
        }
    };
    match &ctx {
        Completion::Nothing => {}
        Completion::Import { .. } => {
            for m in modules() {
                if !casa_prefixo(m, prefix) {
                    continue;
                }
                push(
                    &mut out,
                    Sugestao {
                        rotulo: format!("{m}  (módulo)"),
                        inserir: m.to_string(),
                        importe: None,
                    },
                );
            }
            let dir = caminho.rsplit_once('/').map_or("", |(d, _)| d);
            for f in arquivos {
                if f == caminho || !f.ends_with(".lep") {
                    continue;
                }
                let Some(rel) = relativo_a(dir, f) else {
                    continue;
                };
                let mut spec = rel.trim_end_matches(".lep").to_string();
                if native_module(&spec).is_some() {
                    spec = format!("./{spec}");
                }
                if !casa_prefixo(&spec, prefix) {
                    continue;
                }
                push(
                    &mut out,
                    Sugestao {
                        rotulo: format!("{spec}  (arquivo)"),
                        inserir: spec,
                        importe: None,
                    },
                );
            }
        }
        Completion::Module { alias, .. } => {
            let modulo = modulo_do_alias(texto, alias);
            match modulo {
                ModuloAlvo::Nativo(m) => {
                    for n in NATIVAS.iter().filter(|n| n.module == Some(m)) {
                        for nome in n.names.iter().copied() {
                            if !casa_prefixo(nome, prefix) {
                                continue;
                            }
                            push(&mut out, sugestao_nativa(nome, Some(m), None));
                        }
                    }
                    for d in builtin_docs()
                        .iter()
                        .filter(|d| d.module.as_deref() == Some(m) && d.is_value)
                    {
                        if !casa_prefixo(&d.name, prefix) {
                            continue;
                        }
                        push(
                            &mut out,
                            Sugestao {
                                rotulo: format!("{}  ({m})", d.name),
                                inserir: d.name.clone(),
                                importe: None,
                            },
                        );
                    }
                }
                ModuloAlvo::Arquivo(path) => {
                    if let Ok(src) = std::fs::read_to_string(&path) {
                        for d in parse_definitions(&src).into_iter().filter(|d| d.top_level) {
                            if !casa_prefixo(&d.name, prefix) {
                                continue;
                            }
                            push(
                                &mut out,
                                Sugestao {
                                    rotulo: format!("{}  ({path})", d.name),
                                    inserir: d.name,
                                    importe: None,
                                },
                            );
                        }
                    }
                }
                ModuloAlvo::Nenhum => {}
            }
        }
        Completion::Method { .. } | Completion::Name { .. } => {
            let method = matches!(ctx, Completion::Method { .. });
            let abertos = modulos_abertos(texto);
            for d in parse_definitions(texto) {
                if method && !matches!(d.kind, DefKind::Function(_)) {
                    continue;
                }
                if !casa_prefixo(&d.name, prefix) {
                    continue;
                }
                let rotulo = d.signature();
                push(
                    &mut out,
                    Sugestao {
                        rotulo,
                        inserir: d.name,
                        importe: None,
                    },
                );
            }
            for n in NATIVAS {
                let params = lookup_builtin_doc(n.names[0])
                    .and_then(|d| d.signatures.first().cloned())
                    .map(|s| signature_params(&s).len())
                    .unwrap_or(1);
                if method && params == 0 {
                    continue;
                }
                let import = match n.module {
                    Some(m) if !abertos.contains(&m) => {
                        if alias_do_modulo(texto, m).is_some() {
                            continue;
                        }
                        Some(m.to_string())
                    }
                    _ => None,
                };
                for nome in n.names.iter().copied() {
                    if !casa_prefixo(nome, prefix) {
                        continue;
                    }
                    push(
                        &mut out,
                        Sugestao {
                            rotulo: match &import {
                                Some(m) => format!("{nome}  ({m})"),
                                None => nome.to_string(),
                            },
                            inserir: nome.to_string(),
                            importe: import.clone(),
                        },
                    );
                }
            }
            if !method {
                for k in KEYWORDS {
                    if !casa_prefixo(k, prefix) {
                        continue;
                    }
                    push(
                        &mut out,
                        Sugestao {
                            rotulo: (*k).to_string(),
                            inserir: (*k).to_string(),
                            importe: None,
                        },
                    );
                }
            }
        }
    }
    out.truncate(40);
    out
}

enum ModuloAlvo {
    Nativo(&'static str),
    Arquivo(String),
    Nenhum,
}

fn modulo_do_alias(texto: &str, alias: &str) -> ModuloAlvo {
    for imp in parse_imports(texto) {
        let nome = imp.alias.as_deref().unwrap_or(imp.spec.as_str());
        if nome != alias && imp.spec != alias {
            continue;
        }
        if let Some(m) = native_module(&imp.spec) {
            return ModuloAlvo::Nativo(m);
        }
        return ModuloAlvo::Arquivo(imp.spec.clone());
    }
    if let Some(m) = native_module(alias) {
        return ModuloAlvo::Nativo(m);
    }
    ModuloAlvo::Nenhum
}

fn modulos_abertos(texto: &str) -> Vec<&'static str> {
    parse_imports(texto)
        .into_iter()
        .filter(|i| i.alias.is_none())
        .filter_map(|i| native_module(&i.spec))
        .collect()
}

fn alias_do_modulo(texto: &str, modulo: &str) -> Option<String> {
    parse_imports(texto).into_iter().find_map(|i| {
        if i.spec == modulo {
            i.alias
        } else {
            None
        }
    })
}

fn sugestao_nativa(nome: &str, modulo: Option<&str>, importe: Option<String>) -> Sugestao {
    Sugestao {
        rotulo: match modulo {
            Some(m) => format!("{nome}  ({m})"),
            None => nome.to_string(),
        },
        inserir: nome.to_string(),
        importe,
    }
}

fn relativo_a(dir: &str, b: &str) -> Option<String> {
    if dir.is_empty() {
        return Some(b.to_string());
    }
    b.strip_prefix(&format!("{dir}/")).map(str::to_string)
}

fn casa_prefixo(rotulo: &str, prefix: &str) -> bool {
    let fold = |s: &str| -> String {
        s.chars()
            .flat_map(char::to_lowercase)
            .map(dobrar_acento)
            .collect()
    };
    fold(rotulo).starts_with(&fold(prefix))
}

fn dobrar_acento(c: char) -> char {
    match c {
        'á' | 'à' | 'â' | 'ã' => 'a',
        'é' | 'ê' => 'e',
        'í' => 'i',
        'ó' | 'ô' | 'õ' => 'o',
        'ú' | 'ü' => 'u',
        'ç' => 'c',
        c => c,
    }
}

pub fn garantir_importe(texto: &str, modulo: &str) -> Option<String> {
    if parse_imports(texto)
        .iter()
        .any(|i| i.spec == modulo && i.alias.is_none())
    {
        return None;
    }
    let marca = format!("importe \"{modulo}\"");
    if texto.contains(&marca) {
        return None;
    }
    let linha = import_insert_line(texto);
    let mut linhas: Vec<&str> = texto.split('\n').collect();
    if texto.is_empty() {
        return Some(format!("{marca}\n"));
    }
    if linha >= linhas.len() {
        if !texto.ends_with('\n') {
            return Some(format!("{texto}\n{marca}\n"));
        }
        return Some(format!("{texto}{marca}\n"));
    }
    linhas.insert(linha, &marca);
    Some(linhas.join("\n"))
}

/// UTF-16 units inserted before `byte` when `garantir_importe` rewrites `antes` into `depois`.
pub fn deslocamento_importe(antes: &str, depois: &str, byte: usize) -> usize {
    if depois.len() <= antes.len() {
        return 0;
    }
    let extra = depois.len() - antes.len();
    let insert_at = depois
        .as_bytes()
        .iter()
        .zip(antes.as_bytes())
        .position(|(a, b)| a != b)
        .unwrap_or(0);
    if byte >= insert_at {
        depois[insert_at..insert_at + extra].encode_utf16().count()
    } else {
        0
    }
}

pub fn spawn_tutorial() -> Result<(), String> {
    let mut cmd = comando_tutorial()?;
    cmd.stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    cmd.spawn()
        .map(|_| ())
        .map_err(|e| format!("não abriu os tutoriais: {e}"))
}

fn comando_tutorial() -> Result<Command, String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    if let Some(dir) = exe.parent() {
        let name = if cfg!(windows) {
            "expressa-tutorial.exe"
        } else {
            "expressa-tutorial"
        };
        let next = dir.join(name);
        if next.is_file() {
            return Ok(Command::new(next));
        }
        let bin = dir.join("bin").join(name);
        if bin.is_file() {
            return Ok(Command::new(bin));
        }
    }
    Ok(Command::new(PathBuf::from("expressa-tutorial")))
}

/// Sibling `{nome}.lep` or `lib/{nome}.lep` next to the current file or project root.
pub fn achar_importe(spec: &str, atual: Option<&std::path::Path>, raiz: Option<&std::path::Path>) -> Option<PathBuf> {
    let mut nome = spec.trim_start_matches("./").to_string();
    if !nome.ends_with(".lep") {
        nome.push_str(".lep");
    }
    let mut candidatos = Vec::new();
    if let Some(dir) = atual.and_then(|p| p.parent()) {
        candidatos.push(dir.join(&nome));
        candidatos.push(dir.join("lib").join(&nome));
    }
    if let Some(raiz) = raiz {
        candidatos.push(raiz.join(&nome));
        candidatos.push(raiz.join("lib").join(&nome));
    }
    candidatos.into_iter().find(|p| p.is_file())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn palavra_com_acento() {
        let t = "escreva(função)";
        let i = t.find("função").unwrap();
        assert_eq!(palavra_em(t, i + 3), "função");
    }

    #[test]
    fn ficha_de_escreva() {
        let f = ficha("escreva").expect("escreva");
        assert!(f.contains("escreva"), "{f}");
        let q = ficha("matriz::zeros").expect("matriz::zeros");
        assert!(q.contains("zeros"), "{q}");
    }

    #[test]
    fn completar_matriz_zeros() {
        let t = "importe \"matriz\"\nmatriz::zer";
        let s = completar(t, t.len());
        assert!(
            s.iter()
                .any(|x| x.inserir.contains("zeros") || x.rotulo.contains("zeros")),
            "{s:?}"
        );
        assert!(
            s.iter().all(|x| x.rotulo.contains("matriz") || x.inserir.contains("zero")),
            "só membros de matriz, não as primeiras nativas: {s:?}"
        );
    }

    #[test]
    fn completar_m_dois_pontos_lista_o_modulo() {
        let t = "m = importe \"mat\"\nm::";
        let s = completar(t, t.len());
        assert!(s.iter().any(|x| x.inserir == "raiz" || x.inserir == "seno"), "{s:?}");
        assert!(!s.iter().any(|x| x.inserir == "escreva"), "{s:?}");
    }

    #[test]
    fn completar_importe_lista_modulos() {
        let t = "importe \"ma";
        let s = completar(t, t.len());
        assert!(s.iter().any(|x| x.inserir == "matriz" || x.inserir == "mat"), "{s:?}");
    }

    #[test]
    fn f1_fora_de_nome_nao_tem_ficha() {
        assert!(nome_no_cursor("escreva(1)", 0).is_some());
        assert_eq!(nome_no_cursor("   ", 1), None);
        assert_eq!(nome_no_cursor("se x {", 1), None);
        assert_eq!(
            nome_no_cursor("matriz::zeros", "matriz::z".len()),
            Some("matriz::zeros".into())
        );
    }

    #[test]
    fn assinatura_marca_o_parametro() {
        let t = "escreva(1, ";
        let a = assinatura(t, t.len()).expect("sig");
        assert!(a.contains('«') && a.contains("escreva"), "{a}");
    }

    #[test]
    fn catalogo_tem_nucleo() {
        let c = catalogo();
        assert!(c.iter().any(|(n, _)| n == "escreva"));
    }

    #[test]
    fn importe_so_quando_faltar() {
        let t = "importe \"matriz\"\nmatriz::zeros()\n";
        assert!(garantir_importe(t, "matriz").is_none());
        let novo = garantir_importe("escreva(1)\n", "tela").unwrap();
        assert!(novo.starts_with("importe \"tela\"\n"), "{novo}");
    }
}
