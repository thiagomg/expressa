//! Projects of one student: the folders of `{root}/{student}/`.
//!
//! Before projects existed, a student's files sat directly in
//! `{root}/{student}/`. The first time a student is used, [`preparar`] moves
//! those files into `{student}/principal/` and writes [`MARCA`], so new
//! project folders never show up inside the old files' tree.

use std::fs;
use std::io;
use std::path::Path;

use crate::paths::PROJETO_PADRAO;

/// Present once a student folder is laid out as one folder per project.
pub const MARCA: &str = ".projetos";

/// Where files go while being moved. A move cut short is finished by the
/// next call: everything still loose goes into the same folder.
const MUDANDO: &str = ".mudando";

/// Lays out `dir` (one student) as project folders, moving old loose
/// files into [`PROJETO_PADRAO`]. Does nothing once [`MARCA`] exists.
/// Callers serialize calls for the same student.
pub fn preparar(dir: &Path) -> io::Result<()> {
    let marca = dir.join(MARCA);
    if marca.exists() {
        return Ok(());
    }
    fs::create_dir_all(dir)?;
    let soltos: Vec<_> = fs::read_dir(dir)?
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .filter(|e| e.file_name() != MUDANDO)
        .collect();
    let mudando = dir.join(MUDANDO);
    if !soltos.is_empty() || mudando.exists() {
        // Through a temporary folder: an old folder may already be called
        // `principal`.
        fs::create_dir_all(&mudando)?;
        for entry in soltos {
            fs::rename(entry.path(), mudando.join(entry.file_name()))?;
        }
        fs::rename(&mudando, dir.join(PROJETO_PADRAO))?;
    }
    fs::write(&marca, "uma pasta por projeto\n")
}

/// Project names of a prepared student folder, sorted.
pub fn listar(dir: &Path) -> io::Result<Vec<String>> {
    let mut out = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if !name.starts_with('.') && entry.file_type()?.is_dir() {
            out.push(name);
        }
    }
    out.sort();
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn pasta(nome: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("expressa-projetos-{nome}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn aluno_novo_fica_sem_projetos() {
        let dir = pasta("novo");
        preparar(&dir).unwrap();
        assert!(dir.join(MARCA).exists());
        assert_eq!(listar(&dir).unwrap(), Vec::<String>::new());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn arquivos_antigos_vao_para_principal() {
        let dir = pasta("antigos");
        fs::create_dir_all(dir.join("lib")).unwrap();
        fs::write(dir.join("a.lep"), "a").unwrap();
        fs::write(dir.join("lib/m.lep"), "m").unwrap();
        preparar(&dir).unwrap();
        assert_eq!(
            fs::read_to_string(dir.join("principal/a.lep")).unwrap(),
            "a"
        );
        assert_eq!(
            fs::read_to_string(dir.join("principal/lib/m.lep")).unwrap(),
            "m"
        );
        assert_eq!(listar(&dir).unwrap(), ["principal"]);
        // A second call changes nothing.
        fs::create_dir_all(dir.join("aula-2")).unwrap();
        preparar(&dir).unwrap();
        assert_eq!(listar(&dir).unwrap(), ["aula-2", "principal"]);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn pasta_antiga_chamada_principal_nao_se_perde() {
        let dir = pasta("homonimo");
        fs::create_dir_all(dir.join("principal")).unwrap();
        fs::write(dir.join("principal/x.lep"), "x").unwrap();
        fs::write(dir.join("a.lep"), "a").unwrap();
        preparar(&dir).unwrap();
        assert_eq!(
            fs::read_to_string(dir.join("principal/principal/x.lep")).unwrap(),
            "x"
        );
        assert_eq!(
            fs::read_to_string(dir.join("principal/a.lep")).unwrap(),
            "a"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn mudanca_interrompida_termina_na_proxima_vez() {
        let dir = pasta("interrompida");
        // Stopped after moving `a.lep`, before `b.lep`.
        fs::create_dir_all(dir.join(MUDANDO)).unwrap();
        fs::write(dir.join(MUDANDO).join("a.lep"), "a").unwrap();
        fs::write(dir.join("b.lep"), "b").unwrap();
        preparar(&dir).unwrap();
        assert_eq!(
            fs::read_to_string(dir.join("principal/a.lep")).unwrap(),
            "a"
        );
        assert_eq!(
            fs::read_to_string(dir.join("principal/b.lep")).unwrap(),
            "b"
        );
        assert!(!dir.join(MUDANDO).exists());
        let _ = fs::remove_dir_all(&dir);
    }
}
