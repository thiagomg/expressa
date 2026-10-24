//! Where project files live.
//!
//! A local project is the directory the user picked: files sit directly in
//! it. A remote project is one student folder on a server, `{root}/{aluno}/`.

use std::path::{Component, Path, PathBuf};

use expressa_aula_proto::TreeEntry;

use crate::rpc::Rpc;

pub enum Files {
    /// The window is open and no folder has been chosen yet.
    None,
    Local(PathBuf),
    Remote(Rpc),
}

impl Files {
    pub fn is_remote(&self) -> bool {
        matches!(self, Files::Remote(_))
    }

    /// Stop a server this window started. A remote connection did not.
    pub fn shutdown(&mut self) {
        if let Files::Remote(rpc) = self {
            rpc.shutdown_spawned_server();
        }
    }

    pub fn list_tree(&self, student: &str) -> Result<Vec<TreeEntry>, String> {
        match self {
            Files::None => Err("escolha uma pasta para o projeto".into()),
            Files::Local(root) => list_local(root),
            Files::Remote(rpc) => rpc.list_tree(student),
        }
    }

    pub fn read_file(&self, student: &str, path: &str) -> Result<String, String> {
        match self {
            Files::None => Err("escolha uma pasta para o projeto".into()),
            Files::Local(root) => {
                let path = local_file(root, path)?;
                std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))
            }
            Files::Remote(rpc) => rpc.read_file(student, path),
        }
    }

    pub fn write_file(&self, student: &str, path: &str, source: &str) -> Result<(), String> {
        match self {
            Files::None => Err("escolha uma pasta para o projeto".into()),
            Files::Local(root) => {
                let path = local_file(root, path)?;
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
                }
                std::fs::write(&path, source).map_err(|e| format!("{}: {e}", path.display()))
            }
            Files::Remote(rpc) => rpc.write_file(student, path, source),
        }
    }

    pub fn delete_file(&self, student: &str, path: &str) -> Result<(), String> {
        match self {
            Files::None => Err("escolha uma pasta para o projeto".into()),
            Files::Local(root) => {
                let path = local_file(root, path)?;
                if path.is_dir() {
                    std::fs::remove_dir_all(&path).map_err(|e| e.to_string())
                } else {
                    std::fs::remove_file(&path).map_err(|e| e.to_string())
                }
            }
            Files::Remote(rpc) => rpc.delete_file(student, path),
        }
    }

    pub fn mkdir(&self, student: &str, path: &str) -> Result<(), String> {
        match self {
            Files::None => Err("escolha uma pasta para o projeto".into()),
            Files::Local(root) => {
                let path = local_file(root, path)?;
                std::fs::create_dir_all(&path).map_err(|e| e.to_string())
            }
            Files::Remote(rpc) => rpc.mkdir(student, path),
        }
    }

    pub fn rename(&self, student: &str, from: &str, to: &str) -> Result<(), String> {
        match self {
            Files::None => Err("escolha uma pasta para o projeto".into()),
            Files::Local(root) => {
                let from = local_file(root, from)?;
                let to = local_file(root, to)?;
                if let Some(parent) = to.parent() {
                    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
                }
                std::fs::rename(&from, &to).map_err(|e| e.to_string())
            }
            Files::Remote(rpc) => rpc.rename(student, from, to),
        }
    }
}

/// Student folder name on a remote server.
pub fn valid_aluno(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

fn list_local(root: &Path) -> Result<Vec<TreeEntry>, String> {
    let root = root
        .canonicalize()
        .map_err(|e| format!("{}: {e}", root.display()))?;
    let mut out = Vec::new();
    walk(&root, &root, &mut out)?;
    Ok(out)
}

fn walk(dir: &Path, root: &Path, out: &mut Vec<TreeEntry>) -> Result<(), String> {
    let mut entries = std::fs::read_dir(dir)
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    entries.sort_by(|a, b| {
        let a_dir = a.path().is_dir();
        let b_dir = b.path().is_dir();
        match (a_dir, b_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.file_name().cmp(&b.file_name()),
        }
    });
    for entry in entries {
        let path = entry.path();
        if !path.starts_with(root) {
            continue;
        }
        if let Ok(real) = path.canonicalize() {
            if !real.starts_with(root) {
                continue;
            }
        }
        let rel = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        let is_dir = path.is_dir();
        out.push(TreeEntry { path: rel, is_dir });
        if is_dir {
            walk(&path, root, out)?;
        }
    }
    Ok(())
}

fn local_file(root: &Path, rel: &str) -> Result<PathBuf, String> {
    let rel = rel.trim().replace('\\', "/");
    if rel.is_empty() {
        return Err("caminho vazio".into());
    }
    let rel_path = Path::new(&rel);
    // A leading slash is a root (`/etc`, `\Windows`). `C:foo` has a drive
    // prefix and is not absolute, but it still leaves the project folder.
    if rel_path.is_absolute()
        || rel_path
            .components()
            .any(|c| matches!(c, Component::Prefix(_) | Component::RootDir))
    {
        return Err("caminho absoluto não é permitido".into());
    }
    let root = lexical_normalize(root);
    let full = lexical_normalize(&root.join(rel_path));
    if full == root || !full.starts_with(&root) {
        return Err(format!("caminho fora do projeto: {rel}"));
    }
    Ok(full)
}

fn lexical_normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for c in path.components() {
        match c {
            Component::Prefix(p) => out.push(p.as_os_str()),
            Component::RootDir => out.push(Component::RootDir),
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            Component::Normal(s) => out.push(s),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_file_stays_inside_the_project() {
        let root = Path::new(r"C:\projetos\aula");
        let ok = local_file(root, "lib/mat.lep").unwrap();
        assert!(ok.ends_with(Path::new("lib").join("mat.lep")));
        assert!(local_file(root, "../segredo.lep").is_err());
        assert!(local_file(root, "/etc/passwd").is_err());
        assert!(local_file(root, r"C:\Windows\notepad.exe").is_err());
        assert!(local_file(root, "C:segredo.lep").is_err());
        assert!(
            local_file(root, "lib/../mat.lep")
                .unwrap()
                .ends_with("mat.lep")
        );
        assert!(local_file(root, "").is_err());
    }

    #[test]
    fn local_project_roundtrip_does_not_use_aula_data() {
        let root = std::env::temp_dir().join(format!("expressa-pasta-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("lib")).unwrap();
        let files = Files::Local(root.clone());
        files
            .write_file("", "lib/oi.lep", "escreva(\"oi\")\n")
            .unwrap();
        let tree = files.list_tree("").unwrap();
        assert!(tree.iter().any(|e| e.path == "lib/oi.lep"));
        assert_eq!(
            files.read_file("", "lib/oi.lep").unwrap(),
            "escreva(\"oi\")\n"
        );
        assert!(!root.join("aula-data").exists());
        assert!(root.join("lib").join("oi.lep").is_file());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn aluno_names() {
        assert!(valid_aluno("ana"));
        assert!(valid_aluno("turma-1_b"));
        assert!(!valid_aluno(""));
        assert!(!valid_aluno("../x"));
        assert!(!valid_aluno("a/b"));
        assert!(!valid_aluno("Meu Projeto"));
    }
}
