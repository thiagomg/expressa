//! Where project files live.
//!
//! A local project is the directory the user picked: files sit directly in
//! it. A remote project is one project folder of a student on a server,
//! `{root}/{aluno}/{projeto}/`.

use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use expressa_aula_proto::TreeEntry;

use crate::rpc::Rpc;

/// Cheap to clone (the connection is shared), and can be sent to a worker
/// thread, so network and disk work can leave the GTK thread.
#[derive(Clone)]
pub enum Files {
    /// The window is open and no folder has been chosen yet.
    None,
    Local(PathBuf),
    /// `projeto` is the student's project the window has open. The
    /// connection is shared with the project list (`escolher_projeto`) and
    /// with worker threads.
    Remote {
        rpc: Arc<Rpc>,
        projeto: String,
    },
}

impl Files {
    /// Name of the local run copy (see `exec::Workspace`): one per student
    /// and project on a server, one per folder name locally.
    pub fn cache_key(&self, student: &str) -> String {
        match self {
            Files::Remote { projeto, .. } => format!("{student}-{projeto}"),
            _ => student.to_string(),
        }
    }

    pub fn is_remote(&self) -> bool {
        matches!(self, Files::Remote { .. })
    }

    /// Stop a server this window started. A remote connection did not.
    pub fn shutdown(&mut self) {
        if let Files::Remote { rpc, .. } = self
            && let Some(rpc) = Arc::get_mut(rpc)
        {
            rpc.shutdown_spawned_server();
        }
    }

    pub fn list_tree(&self, student: &str) -> Result<Vec<TreeEntry>, String> {
        match self {
            Files::None => Err("escolha uma pasta para o projeto".into()),
            Files::Local(root) => list_local(root),
            Files::Remote { rpc, projeto } => rpc.list_tree(student, projeto),
        }
    }

    pub fn read_file(&self, student: &str, path: &str) -> Result<String, String> {
        match self {
            Files::None => Err("escolha uma pasta para o projeto".into()),
            Files::Local(root) => {
                let path = local_file(root, path)?;
                std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))
            }
            Files::Remote { rpc, projeto } => rpc.read_file(student, projeto, path),
        }
    }

    /// Raw bytes of a local file. Remote projects only carry text.
    pub fn read_bytes(&self, _student: &str, path: &str) -> Result<Vec<u8>, String> {
        match self {
            Files::Local(root) => {
                let path = local_file(root, path)?;
                std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))
            }
            Files::None => Err("escolha uma pasta para o projeto".into()),
            Files::Remote { .. } => Err("arquivo binário no servidor não é suportado".into()),
        }
    }

    pub fn write_bytes(&self, _student: &str, path: &str, bytes: &[u8]) -> Result<(), String> {
        match self {
            Files::Local(root) => {
                let path = local_file(root, path)?;
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
                }
                std::fs::write(&path, bytes).map_err(|e| format!("{}: {e}", path.display()))
            }
            Files::None => Err("escolha uma pasta para o projeto".into()),
            Files::Remote { .. } => Err("arquivo binário no servidor não é suportado".into()),
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
            Files::Remote { rpc, projeto } => rpc.write_file(student, projeto, path, source),
        }
    }

    /// Create `path` with `source`; never replaces an existing file.
    pub fn create_file(&self, student: &str, path: &str, source: &str) -> Result<(), String> {
        match self {
            Files::None => Err("escolha uma pasta para o projeto".into()),
            Files::Local(root) => {
                use std::io::Write;
                let full = local_file(root, path)?;
                if let Some(parent) = full.parent() {
                    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
                }
                let mut file = std::fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(&full)
                    .map_err(|e| match e.kind() {
                        std::io::ErrorKind::AlreadyExists => ja_existe(path),
                        _ => format!("{}: {e}", full.display()),
                    })?;
                file.write_all(source.as_bytes())
                    .map_err(|e| format!("{}: {e}", full.display()))
            }
            Files::Remote { rpc, projeto } => rpc.create_file(student, projeto, path, source),
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
            Files::Remote { rpc, projeto } => rpc.delete_file(student, projeto, path),
        }
    }

    pub fn mkdir(&self, student: &str, path: &str) -> Result<(), String> {
        match self {
            Files::None => Err("escolha uma pasta para o projeto".into()),
            Files::Local(root) => {
                let path = local_file(root, path)?;
                std::fs::create_dir_all(&path).map_err(|e| e.to_string())
            }
            Files::Remote { rpc, projeto } => rpc.mkdir(student, projeto, path),
        }
    }

    pub fn rename(&self, student: &str, from: &str, to: &str) -> Result<(), String> {
        match self {
            Files::None => Err("escolha uma pasta para o projeto".into()),
            Files::Local(root) => {
                let from_path = local_file(root, from)?;
                let to_path = local_file(root, to)?;
                if from_path == to_path {
                    return Ok(());
                }
                // `fs::rename` replaces an existing file on Unix.
                if to_path.symlink_metadata().is_ok() {
                    return Err(ja_existe(to));
                }
                let (from, to) = (from_path, to_path);
                if let Some(parent) = to.parent() {
                    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
                }
                std::fs::rename(&from, &to).map_err(|e| e.to_string())
            }
            Files::Remote { rpc, projeto } => rpc.rename(student, projeto, from, to),
        }
    }
}

/// Same text the server sends, so both kinds of project read alike.
fn ja_existe(rel: &str) -> String {
    format!("já existe `{rel}`")
}

/// Student folder name on a remote server.
pub fn valid_aluno(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

/// Home or the filesystem root. Listing either walks hundreds of thousands
/// of files on the GTK thread and the window stops taking input.
pub fn pasta_larga(path: &Path) -> bool {
    if path.as_os_str().is_empty() {
        return false;
    }
    if path == Path::new("/") || path == Path::new("\\") {
        return true;
    }
    let Some(home) = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")) else {
        return false;
    };
    let home = Path::new(&home);
    if path == home {
        return true;
    }
    match (path.canonicalize(), home.canonicalize()) {
        (Ok(path), Ok(home)) => path == home || path == Path::new("/"),
        _ => false,
    }
}

const MAX_TREE_ENTRIES: usize = 20_000;

/// Modification time in nanoseconds since 1970, or 0 if the system has none
/// (same as the server sends).
pub fn modificado_ns(meta: &std::fs::Metadata) -> i64 {
    meta.modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map_or(0, |d| d.as_nanos() as i64)
}

fn list_local(root: &Path) -> Result<Vec<TreeEntry>, String> {
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

fn skip_name(name: &std::ffi::OsStr) -> bool {
    let Some(s) = name.to_str() else {
        return true;
    };
    s.starts_with('.')
        || matches!(
            s,
            "target" | "node_modules" | "dist" | "__pycache__" | "build"
        )
}

fn walk(dir: &Path, root: &Path, out: &mut Vec<TreeEntry>) -> Result<(), String> {
    let mut entries = std::fs::read_dir(dir)
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
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
        if skip_name(&entry.file_name()) {
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
        let is_dir = kind.is_dir();
        if out.len() >= MAX_TREE_ENTRIES {
            return Err("pasta grande demais para ser um projeto".into());
        }
        let (size, modified_ns) = if is_dir {
            (0, 0)
        } else {
            entry
                .metadata()
                .map(|m| (m.len(), modificado_ns(&m)))
                .unwrap_or((0, 0))
        };
        out.push(TreeEntry {
            path: rel,
            is_dir,
            size,
            modified_ns,
        });
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
        // Drive prefixes only exist on Windows; elsewhere `C:` is an
        // ordinary folder name inside the project.
        if cfg!(windows) {
            assert!(local_file(root, r"C:\Windows\notepad.exe").is_err());
            assert!(local_file(root, "C:segredo.lep").is_err());
        }
        assert!(
            local_file(root, "lib/../mat.lep")
                .unwrap()
                .ends_with("mat.lep")
        );
        assert!(local_file(root, "").is_err());
    }

    #[test]
    fn pasta_larga_is_home_or_root_only() {
        assert!(pasta_larga(Path::new("/")));
        let home = home_for_test();
        assert!(pasta_larga(&home));
        assert!(!pasta_larga(&home.join("src")));
        assert!(!pasta_larga(Path::new("")));
    }

    fn home_for_test() -> PathBuf {
        std::env::var_os("HOME")
            .or_else(|| std::env::var_os("USERPROFILE"))
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("/home/thiago"))
    }

    #[test]
    fn list_local_skips_hidden_and_build_dirs() {
        let root = std::env::temp_dir().join(format!("expressa-tree-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::create_dir_all(root.join(".git")).unwrap();
        std::fs::create_dir_all(root.join("target")).unwrap();
        std::fs::write(root.join("src/ola.lep"), "escreva(1)\n").unwrap();
        std::fs::write(root.join(".git/config"), "x\n").unwrap();
        std::fs::write(root.join("target/a.o"), "x\n").unwrap();
        let files = Files::Local(root.clone());
        let tree = files.list_tree("").unwrap();
        let paths: Vec<_> = tree.iter().map(|e| e.path.as_str()).collect();
        assert!(paths.contains(&"src"));
        assert!(paths.iter().any(|p| p.ends_with("ola.lep")));
        assert!(
            !paths
                .iter()
                .any(|p| p.contains(".git") || p.contains("target"))
        );
        let _ = std::fs::remove_dir_all(&root);
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

    /// A local project with `a.lep` = "original".
    fn projeto_local(nome: &str) -> (PathBuf, Files) {
        let root =
            std::env::temp_dir().join(format!("expressa-store-{nome}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("a.lep"), "original").unwrap();
        (root.clone(), Files::Local(root))
    }

    #[test]
    fn create_file_nao_substitui_arquivo() {
        let (root, files) = projeto_local("create");
        let err = files.create_file("", "a.lep", "novo").unwrap_err();
        assert_eq!(err, "já existe `a.lep`");
        assert_eq!(
            std::fs::read_to_string(root.join("a.lep")).unwrap(),
            "original"
        );
        files.create_file("", "lib/b.lep", "novo").unwrap();
        assert_eq!(
            std::fs::read_to_string(root.join("lib/b.lep")).unwrap(),
            "novo"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn rename_nao_substitui_destino() {
        let (root, files) = projeto_local("rename");
        std::fs::write(root.join("b.lep"), "outro").unwrap();
        let err = files.rename("", "b.lep", "a.lep").unwrap_err();
        assert_eq!(err, "já existe `a.lep`");
        assert_eq!(
            std::fs::read_to_string(root.join("a.lep")).unwrap(),
            "original"
        );
        files.rename("", "b.lep", "c.lep").unwrap();
        assert_eq!(
            std::fs::read_to_string(root.join("c.lep")).unwrap(),
            "outro"
        );
        files.rename("", "a.lep", "a.lep").unwrap();
        assert_eq!(
            std::fs::read_to_string(root.join("a.lep")).unwrap(),
            "original"
        );
        let _ = std::fs::remove_dir_all(&root);
    }
}
