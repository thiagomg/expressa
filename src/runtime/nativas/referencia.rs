//! Built-in help parsed from documentation-only `.lep` files (`///` + signatures).
//! Those files are not executed.

use std::sync::OnceLock;

use crate::parser::{AssignTarget, Expr, Item, Stmt, parse};

use super::NATIVAS;

pub struct BuiltinDoc {
    pub name: String,
    pub module: Option<String>,
    pub sig: String,
    pub signatures: Vec<String>,
    pub summary: String,
    pub example: String,
    pub doc: String,
    pub is_value: bool,
}

impl BuiltinDoc {
    pub fn args_text(&self) -> String {
        self.signatures
            .iter()
            .map(|s| match (s.find('('), s.find(')')) {
                (Some(a), Some(z)) if z > a => s[a + 1..z].to_string(),
                _ => String::new(),
            })
            .max_by_key(|a| a.len())
            .unwrap_or_default()
    }

    pub fn ficha(&self) -> String {
        let mut out = String::new();
        if self.signatures.is_empty() {
            out.push_str(&self.name);
        } else {
            out.push_str(&self.signatures.join("\n"));
        }
        match &self.module {
            Some(m) => {
                out.push_str(&format!("\nimporte \"{m}\""));
            }
            None => {}
        }
        if !self.doc.is_empty() {
            out.push('\n');
            out.push('\n');
            out.push_str(&self.doc);
        }
        out
    }
}

const NUCLEO: &str = include_str!("../../../docs/nativas/nucleo.lep");
const TELA: &str = include_str!("../../../docs/nativas/tela.lep");
const MAT: &str = include_str!("../../../docs/nativas/mat.lep");
const MATRIZ: &str = include_str!("../../../docs/nativas/matriz.lep");
const ARQUIVO: &str = include_str!("../../../docs/nativas/arquivo.lep");

pub fn builtin_docs() -> &'static [BuiltinDoc] {
    static DOCS: OnceLock<Vec<BuiltinDoc>> = OnceLock::new();
    DOCS.get_or_init(load_all)
}

pub fn lookup_builtin_doc(name: &str) -> Option<&'static BuiltinDoc> {
    let canon = NATIVAS
        .iter()
        .find(|n| n.names.iter().any(|nm| *nm == name))
        .map(|n| n.names[0])
        .unwrap_or(name);
    builtin_docs()
        .iter()
        .find(|d| d.name == canon || d.name == name)
}

/// `escreva`, `matriz::zeros`, aliases.
pub fn lookup_ficha(alvo: &str) -> Option<String> {
    lookup_alvo(alvo).map(|d| d.ficha())
}

pub fn lookup_alvo(alvo: &str) -> Option<&'static BuiltinDoc> {
    let alvo = alvo.trim();
    if let Some((modulo, nome)) = alvo.split_once("::") {
        return builtin_docs().iter().find(|d| {
            d.module.as_deref() == Some(modulo)
                && (d.name == nome || nativa_aliases(d.name.as_str()).any(|a| a == nome))
        });
    }
    lookup_builtin_doc(alvo)
}

fn nativa_aliases(canon: &str) -> impl Iterator<Item = &'static str> {
    NATIVAS
        .iter()
        .find(|n| n.names[0] == canon)
        .into_iter()
        .flat_map(|n| n.names.iter().copied())
}

fn load_all() -> Vec<BuiltinDoc> {
    let mut out = Vec::new();
    for (src, module) in [
        (NUCLEO, None),
        (TELA, Some("tela")),
        (MAT, Some("mat")),
        (MATRIZ, Some("matriz")),
        (ARQUIVO, Some("arquivo")),
    ] {
        out.extend(extract(src, module).unwrap_or_else(|e| {
            panic!("docs/nativas ({module:?}): {e}");
        }));
    }
    out
}

struct Partial {
    name: String,
    signatures: Vec<String>,
    doc: String,
    is_value: bool,
}

fn extract(source: &str, module: Option<&str>) -> Result<Vec<BuiltinDoc>, String> {
    let program = parse(source).map_err(|e| e.to_string())?;
    let mut parts: Vec<Partial> = Vec::new();
    for item in program.items {
        let Item::Stmt(Stmt::Assign {
            target: AssignTarget::Name { name, .. },
            value,
            doc,
            ..
        }) = item
        else {
            continue;
        };
        let (is_value, sig) = match &value {
            Expr::Function { params, .. } => {
                let args = params
                    .iter()
                    .map(|p| p.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");
                (false, Some(format!("{name}({args})")))
            }
            Expr::List { elements, .. } if elements.is_empty() => (true, None),
            Expr::Number { .. } => (true, None),
            Expr::String { .. } => (true, None),
            _ => continue,
        };
        if let Some(existing) = parts.iter_mut().find(|p| p.name == name) {
            if let Some(s) = sig {
                if !existing.signatures.contains(&s) {
                    existing.signatures.push(s);
                }
            }
            if existing.doc.is_empty() {
                if let Some(d) = doc {
                    existing.doc = d;
                }
            }
            existing.is_value |= is_value;
        } else {
            parts.push(Partial {
                name,
                signatures: sig.into_iter().collect(),
                doc: doc.unwrap_or_default(),
                is_value,
            });
        }
    }
    Ok(parts
        .into_iter()
        .map(|p| {
            let (from_doc, body) = signatures_in_doc(&p.name, &p.doc);
            let signatures = if from_doc.is_empty() {
                p.signatures
            } else {
                from_doc
            };
            let (summary, example) = summary_and_example(&body);
            BuiltinDoc {
                name: p.name.clone(),
                module: module.map(|m| m.to_string()),
                sig: signatures.join("  ou  "),
                signatures,
                summary,
                example,
                doc: body,
                is_value: p.is_value,
            }
        })
        .collect())
}

/// Optional human signatures as the first `nome(...)` lines of the `///` block.
fn signatures_in_doc(name: &str, doc: &str) -> (Vec<String>, String) {
    let prefix = format!("{name}(");
    let mut sigs = Vec::new();
    let mut consumed = 0usize;
    for line in doc.lines() {
        let t = line.trim();
        if t.is_empty() {
            if sigs.is_empty() {
                consumed += line.len() + 1;
                continue;
            }
            consumed += line.len() + 1;
            break;
        }
        if t.starts_with(&prefix) {
            sigs.push(t.to_string());
            consumed += line.len() + 1;
        } else {
            break;
        }
    }
    if sigs.is_empty() {
        return (Vec::new(), doc.to_string());
    }
    let rest = doc.get(consumed..).unwrap_or("").trim_start().to_string();
    (sigs, rest)
}

fn summary_and_example(doc: &str) -> (String, String) {
    let mut paras = doc.split("\n\n");
    let summary = paras
        .next()
        .unwrap_or("")
        .lines()
        .map(str::trim)
        .collect::<Vec<_>>()
        .join(" ");
    let example = doc
        .lines()
        .find_map(|l| {
            l.trim()
                .strip_prefix("Exemplo:")
                .map(|rest| rest.trim().to_string())
        })
        .unwrap_or_default();
    (summary, example)
}

#[cfg(test)]
mod tests {
    use super::{builtin_docs, lookup_alvo, lookup_ficha};
    use crate::runtime::nativas::NATIVAS;

    #[test]
    fn every_nativa_has_doc() {
        for n in NATIVAS {
            assert!(
                lookup_alvo(n.names[0]).is_some(),
                "falta /// para {}",
                n.names[0]
            );
        }
        assert!(lookup_alvo("argumentos").is_some());
        let pi = lookup_ficha("mat::pi").expect("pi");
        assert!(pi.contains("importe \"mat\""), "{pi}");
        let nl = lookup_ficha("tela::nova_linha").expect("nova_linha");
        assert!(nl.contains("importe \"tela\""), "{nl}");
        assert!(lookup_alvo("sen").is_some());
    }

    #[test]
    fn ficha_modulo_nome() {
        let t = lookup_ficha("matriz::zeros").expect("zeros");
        assert!(t.contains("zeros(n)"));
        assert!(t.contains("importe \"matriz\""));
        assert!(t.contains("Matriz só de zeros"));
    }

    #[test]
    fn docs_parse_overloads() {
        let d = lookup_alvo("escreva").unwrap();
        assert!(d.signatures.len() >= 2);
        assert!(!d.summary.is_empty());
        assert!(builtin_docs().len() > NATIVAS.len());
    }
}
