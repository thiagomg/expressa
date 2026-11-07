//! Local project tree. No GTK, no classroom server.

use std::collections::HashSet;
use std::path::{Component, Path, PathBuf};

use crate::prefs::{Prefs, Recente, expand_tilde};

const MAX_TREE_ENTRIES: usize = 20_000;
const MAX_TREE_DEPTH: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct No {
    pub rel: String,
    pub nome: String,
    pub is_dir: bool,
    pub profundidade: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Projeto {
    pub raiz: Option<PathBuf>,
    pub todos: Vec<No>,
    pub abertos: HashSet<String>,
    pub selecionado: Option<String>,
    pub visivel: bool,
    pub remoto: bool,
}

impl Default for Projeto {
    fn default() -> Self {
        Self {
            raiz: None,
            todos: Vec::new(),
            abertos: HashSet::new(),
            selecionado: None,
            visivel: true,
            remoto: false,
        }
    }
}

impl Projeto {
    pub fn visiveis(&self) -> Vec<No> {
        self.todos
            .iter()
            .filter(|n| pasta_aberta(&self.abertos, &n.rel))
            .cloned()
            .collect()
    }

    pub fn recarregar(&mut self) -> Result<(), String> {
        let Some(raiz) = self.raiz.clone() else {
            self.todos.clear();
            return Ok(());
        };
        self.todos = listar(&raiz)?;
        Ok(())
    }

    /// Expand parent folders and select `rel` (project-relative).
    pub fn revelar(&mut self, rel: &str) {
        if rel.is_empty() {
            return;
        }
        self.selecionado = Some(rel.to_string());
        let mut prefix = String::new();
        let partes: Vec<&str> = rel.split('/').collect();
        for (i, parte) in partes.iter().enumerate() {
            if i + 1 == partes.len() {
                break;
            }
            if !prefix.is_empty() {
                prefix.push('/');
            }
            prefix.push_str(parte);
            self.abertos.insert(prefix.clone());
        }
    }
}

pub fn rel_na_raiz(raiz: &Path, path: &Path) -> Option<String> {
    path.strip_prefix(raiz)
        .ok()
        .map(|p| p.to_string_lossy().replace('\\', "/"))
}

fn pasta_aberta(abertos: &HashSet<String>, rel: &str) -> bool {
    let mut prefix = String::new();
    for part in rel.split('/') {
        if !prefix.is_empty() && !abertos.contains(&prefix) {
            return false;
        }
        if !prefix.is_empty() {
            prefix.push('/');
        }
        prefix.push_str(part);
    }
    true
}

pub fn skip_name(name: &str) -> bool {
    name.starts_with('.')
        || matches!(
            name,
            "target" | "node_modules" | "dist" | "__pycache__" | "build"
        )
}

pub fn pasta_larga(path: &Path) -> bool {
    if path.as_os_str().is_empty() {
        return false;
    }
    if path == Path::new("/") || path == Path::new("\\") {
        return true;
    }
    let Some(home) = dirs::home_dir() else {
        return false;
    };
    path == home
}

pub fn listar(root: &Path) -> Result<Vec<No>, String> {
    let root = root
        .canonicalize()
        .map_err(|e| format!("{}: {e}", root.display()))?;
    if pasta_larga(&root) {
        return Err("escolha uma pasta do projeto, não a pasta pessoal".into());
    }
    let mut out = Vec::new();
    walk(&root, &root, &mut out)?;
    Ok(out)
}

fn walk(dir: &Path, root: &Path, out: &mut Vec<No>) -> Result<(), String> {
    let profundidade = dir.strip_prefix(root).map_or(0, |p| p.components().count());
    if profundidade > MAX_TREE_DEPTH {
        return Err("projeto com pastas profundas demais".into());
    }
    let mut entries = std::fs::read_dir(dir)
        .map_err(|e| e.to_string())?
        .take(MAX_TREE_ENTRIES + 1)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    if entries.len() > MAX_TREE_ENTRIES {
        return Err("pasta com arquivos demais".into());
    }
    entries.sort_by(|a, b| {
        let a_dir = a.file_type().map(|t| t.is_dir()).unwrap_or(false);
        let b_dir = b.file_type().map(|t| t.is_dir()).unwrap_or(false);
        match (a_dir, b_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.file_name().cmp(&b.file_name()),
        }
    });
    for entry in entries {
        let nome = entry.file_name().to_string_lossy().into_owned();
        if skip_name(&nome) {
            continue;
        }
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        if kind.is_symlink() {
            continue;
        }
        let path = entry.path();
        if !path.starts_with(root) {
            continue;
        }
        let rel = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        if out.len() >= MAX_TREE_ENTRIES {
            return Err("pasta grande demais para ser um projeto".into());
        }
        let is_dir = kind.is_dir();
        out.push(No {
            rel: rel.clone(),
            nome,
            is_dir,
            profundidade,
        });
        if is_dir {
            walk(&path, root, out)?;
        }
    }
    Ok(())
}

pub fn caminho_no_projeto(root: &Path, rel: &str) -> Result<PathBuf, String> {
    let rel = rel.trim().replace('\\', "/");
    let rel_path = Path::new(&rel);
    if rel_path.is_absolute()
        || rel_path
            .components()
            .any(|c| matches!(c, Component::Prefix(_) | Component::ParentDir | Component::RootDir))
    {
        return Err("caminho fora do projeto".into());
    }
    if rel.split('/').any(skip_name) {
        return Err("nome reservado".into());
    }
    Ok(root.join(rel_path))
}

pub fn garantir_lep_nome(nome: &str) -> String {
    if nome.contains('.') {
        nome.to_string()
    } else {
        format!("{nome}.lep")
    }
}

pub async fn escolher_pasta() -> Option<PathBuf> {
    rfd::AsyncFileDialog::new()
        .pick_folder()
        .await
        .map(|h| h.path().to_path_buf())
}

pub fn instalar_exemplos() -> Result<PathBuf, String> {
    expressa::exemplos::instalar().map_err(|e| e.to_string())
}

pub fn pasta_exemplos() -> Option<PathBuf> {
    expressa::exemplos::dest_dir().filter(|p| p.is_dir())
}

pub fn restaurar_inicio(prefs: &Prefs) -> Option<PathBuf> {
    if !prefs.pasta.is_empty() {
        let p = expand_tilde(&prefs.pasta);
        if p.is_dir() {
            return Some(p);
        }
    }
    pasta_exemplos()
}

pub fn abrir_raiz(projeto: &mut Projeto, prefs: &mut Prefs, raiz: PathBuf) -> Result<(), String> {
    let raiz = raiz
        .canonicalize()
        .map_err(|e| format!("{}: {e}", raiz.display()))?;
    if pasta_larga(&raiz) {
        return Err("escolha uma pasta do projeto, não a pasta pessoal".into());
    }
    projeto.raiz = Some(raiz.clone());
    projeto.remoto = false;
    projeto.abertos.insert(String::new());
    projeto.recarregar()?;
    prefs.pasta = raiz.display().to_string();
    prefs.lembrar_recente(Recente::Local(prefs.pasta.clone()));
    prefs.save();
    Ok(())
}

pub fn nos_de_entradas(entradas: Vec<(String, bool)>) -> Vec<No> {
    let mut nos: Vec<No> = entradas
        .into_iter()
        .filter(|(rel, _)| {
            !rel.split('/').any(skip_name) && !rel.is_empty()
        })
        .map(|(rel, is_dir)| {
            let profundidade = rel.matches('/').count();
            let nome = Path::new(&rel)
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| rel.clone());
            No {
                rel,
                nome,
                is_dir,
                profundidade,
            }
        })
        .collect();
    nos.sort_by(|a, b| match (a.is_dir, b.is_dir) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.rel.cmp(&b.rel),
    });
    nos
}

pub fn pasta_do_item(projeto: &Projeto) -> String {
    match projeto.selecionado.as_deref() {
        Some(rel) => {
            if projeto
                .todos
                .iter()
                .any(|n| n.rel == rel && n.is_dir)
            {
                rel.to_string()
            } else {
                Path::new(rel)
                    .parent()
                    .map(|p| p.to_string_lossy().replace('\\', "/"))
                    .unwrap_or_default()
            }
        }
        None => String::new(),
    }
}

pub fn criar_arquivo(raiz: &Path, pasta: &str, nome: &str) -> Result<PathBuf, String> {
    let nome = garantir_lep_nome(nome.trim());
    if nome.is_empty() || nome == ".lep" {
        return Err("nome vazio".into());
    }
    let rel = if pasta.is_empty() {
        nome
    } else {
        format!("{pasta}/{nome}")
    };
    let path = caminho_no_projeto(raiz, &rel)?;
    if path.exists() {
        return Err("já existe".into());
    }
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    std::fs::write(&path, "escreva(\"olá\")\n").map_err(|e| e.to_string())?;
    Ok(path)
}

pub fn criar_pasta(raiz: &Path, pasta: &str, nome: &str) -> Result<(), String> {
    let nome = nome.trim();
    if nome.is_empty() {
        return Err("nome vazio".into());
    }
    let rel = if pasta.is_empty() {
        nome.to_string()
    } else {
        format!("{pasta}/{nome}")
    };
    let path = caminho_no_projeto(raiz, &rel)?;
    std::fs::create_dir_all(path).map_err(|e| e.to_string())
}

pub fn renomear(raiz: &Path, de: &str, nome: &str) -> Result<PathBuf, String> {
    let nome = nome.trim();
    if nome.is_empty() {
        return Err("nome vazio".into());
    }
    let dest_rel = match Path::new(de).parent() {
        Some(p) if p.as_os_str().is_empty() => nome.to_string(),
        Some(p) => format!("{}/{nome}", p.to_string_lossy().replace('\\', "/")),
        None => nome.to_string(),
    };
    let from = caminho_no_projeto(raiz, de)?;
    let to = caminho_no_projeto(raiz, &dest_rel)?;
    if to.exists() {
        return Err("já existe".into());
    }
    std::fs::rename(&from, &to).map_err(|e| e.to_string())?;
    Ok(to)
}

pub fn apagar(raiz: &Path, rel: &str) -> Result<(), String> {
    let path = caminho_no_projeto(raiz, rel)?;
    if path.is_dir() {
        std::fs::remove_dir_all(path).map_err(|e| e.to_string())
    } else {
        std::fs::remove_file(path).map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp() -> PathBuf {
        let p = std::env::temp_dir().join(format!(
            "aula-freya-proj-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn esconde_ponto_e_target() {
        assert!(skip_name(".git"));
        assert!(skip_name("target"));
        assert!(skip_name("node_modules"));
        assert!(!skip_name("linguagem"));
    }

    #[test]
    fn lista_e_ignora_lixo() {
        let root = temp();
        std::fs::create_dir_all(root.join("target")).unwrap();
        std::fs::write(root.join("target/a.o"), "x").unwrap();
        std::fs::write(root.join("a.lep"), "1").unwrap();
        std::fs::create_dir_all(root.join("lib")).unwrap();
        std::fs::write(root.join("lib/b.lep"), "2").unwrap();
        let nos = listar(&root).unwrap();
        let rels: Vec<_> = nos.iter().map(|n| n.rel.as_str()).collect();
        assert!(rels.contains(&"a.lep"));
        assert!(rels.contains(&"lib"));
        assert!(rels.contains(&"lib/b.lep"));
        assert!(!rels.iter().any(|p| p.contains("target")));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn recusa_caminho_fora() {
        let root = Path::new("/tmp/proj");
        assert!(caminho_no_projeto(root, "../x").is_err());
        assert!(caminho_no_projeto(root, "/etc/passwd").is_err());
        assert!(caminho_no_projeto(root, "ok/a.lep").is_ok());
    }

    #[test]
    fn revelar_abre_os_pais() {
        let mut p = Projeto::default();
        p.revelar("lib/tartaruga.lep");
        assert!(p.abertos.contains("lib"));
        assert_eq!(p.selecionado.as_deref(), Some("lib/tartaruga.lep"));
        p.revelar("a.lep");
        assert_eq!(p.selecionado.as_deref(), Some("a.lep"));
    }

    #[test]
    fn entradas_viram_nos() {
        let nos = nos_de_entradas(vec![
            ("lib/b.lep".into(), false),
            ("lib".into(), true),
            ("a.lep".into(), false),
            (".git".into(), true),
        ]);
        let rels: Vec<_> = nos.iter().map(|n| n.rel.as_str()).collect();
        assert_eq!(rels, vec!["lib", "a.lep", "lib/b.lep"]);
        assert_eq!(nos[0].profundidade, 0);
        assert_eq!(nos[2].profundidade, 1);
    }

    #[test]
    fn nome_sem_ponto_vira_lep() {
        assert_eq!(garantir_lep_nome("poema"), "poema.lep");
        assert_eq!(garantir_lep_nome("a.lep"), "a.lep");
    }

    #[test]
    fn cria_arquivo_na_pasta() {
        let root = temp();
        std::fs::create_dir_all(root.join("lib")).unwrap();
        let p = criar_arquivo(&root, "lib", "novo").unwrap();
        assert_eq!(p.file_name().unwrap(), "novo.lep");
        assert!(p.exists());
        let _ = std::fs::remove_dir_all(root);
    }
}
