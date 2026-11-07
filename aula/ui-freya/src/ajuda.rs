//! F1 ficha, catalog of natives, completion, tutorials.

use std::path::PathBuf;
use std::process::{Command, Stdio};

use expressa::runtime::{builtin_docs, lookup_ficha};

use crate::lang::KEYWORDS;

pub fn is_ident_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

pub fn palavra_em(texto: &str, byte: usize) -> String {
    let byte = byte.min(texto.len());
    let start = texto[..byte]
        .char_indices()
        .rev()
        .take_while(|(_, c)| is_ident_char(*c))
        .last()
        .map(|(i, _)| i)
        .unwrap_or(byte);
    let end = texto[byte..]
        .char_indices()
        .take_while(|(_, c)| is_ident_char(*c))
        .last()
        .map(|(i, c)| byte + i + c.len_utf8())
        .unwrap_or(byte);
    texto[start..end].to_string()
}

pub fn prefixo_em(texto: &str, byte: usize) -> String {
    let byte = byte.min(texto.len());
    let start = texto[..byte]
        .char_indices()
        .rev()
        .take_while(|(_, c)| is_ident_char(*c))
        .last()
        .map(|(i, _)| i)
        .unwrap_or(byte);
    texto[start..byte].to_string()
}

pub fn ficha(nome: &str) -> Option<String> {
    let nome = nome.trim();
    if nome.is_empty() || KEYWORDS.contains(&nome) {
        return None;
    }
    lookup_ficha(nome)
}

pub fn catalogo() -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = builtin_docs()
        .iter()
        .map(|d| {
            let nome = match &d.module {
                Some(m) => format!("{m}::{}", d.name),
                None => d.name.clone(),
            };
            (nome, d.ficha())
        })
        .collect();
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sugestao {
    pub rotulo: String,
    pub inserir: String,
    pub importe: Option<String>,
}

pub fn completar(texto: &str, byte: usize) -> Vec<Sugestao> {
    let prefix = prefixo_em(texto, byte);
    let prefix_l = prefix.to_lowercase();
    let mut out = Vec::new();
    for d in builtin_docs() {
        let nomes = std::iter::once(d.name.as_str());
        for nome in nomes {
            if !prefix_l.is_empty() && !nome.to_lowercase().starts_with(&prefix_l) {
                continue;
            }
            out.push(Sugestao {
                rotulo: match &d.module {
                    Some(m) => format!("{nome}  ({m})"),
                    None => nome.to_string(),
                },
                inserir: nome.to_string(),
                importe: d.module.clone(),
            });
        }
    }
    for linha in texto.lines() {
        let t = linha.trim_start();
        let nome_fim = t.find(|c: char| !is_ident_char(c)).unwrap_or(t.len());
        let nome = &t[..nome_fim];
        if nome.is_empty() || KEYWORDS.contains(&nome) {
            continue;
        }
        if !prefix_l.is_empty() && !nome.to_lowercase().starts_with(&prefix_l) {
            continue;
        }
        if !out.iter().any(|s| s.inserir == nome) {
            out.push(Sugestao {
                rotulo: nome.to_string(),
                inserir: nome.to_string(),
                importe: None,
            });
        }
    }
    out.truncate(40);
    out
}

pub fn garantir_importe(texto: &str, modulo: &str) -> Option<String> {
    let marca = format!("importe \"{modulo}\"");
    if texto.contains(&marca) {
        return None;
    }
    Some(format!("{marca}\n{texto}"))
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
    }

    #[test]
    fn completar_matriz_zeros() {
        let t = "importe \"matriz\"\nmatriz::zer";
        let s = completar(t, t.len());
        assert!(
            s.iter().any(|x| x.inserir.contains("zeros") || x.rotulo.contains("zeros")),
            "{s:?}"
        );
    }

    #[test]
    fn catalogo_tem_nucleo() {
        let c = catalogo();
        assert!(c.iter().any(|(n, _)| n == "escreva"));
    }
}
