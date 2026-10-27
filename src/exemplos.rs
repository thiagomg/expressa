//! Copy shipped examples into the student's Documents folder.
//! Used by `expressa --instalar-exemplos` and by the OS installers.

use std::io;
use std::path::{Path, PathBuf};

/// Templates next to the binary, in `/usr/share/expressa/exemplos`, or the repo.
pub fn bundled_dir() -> Option<PathBuf> {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            for cand in [dir.join("exemplos"), dir.join("share/expressa/exemplos")] {
                if cand.join("linguagem").is_dir() {
                    return Some(cand);
                }
            }
        }
    }
    let share = PathBuf::from("/usr/share/expressa/exemplos");
    if share.join("linguagem").is_dir() {
        return Some(share);
    }
    let dev = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("exemplos");
    dev.join("linguagem").is_dir().then_some(dev)
}

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

/// `Documentos/Expressa Aula/exemplos` (or Documents).
pub fn dest_dir() -> Option<PathBuf> {
    let docs = documents_dir()?;
    Some(docs.join("Expressa Aula").join("exemplos"))
}

fn documents_dir() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("XDG_DOCUMENTS_DIR") {
        let p = PathBuf::from(p);
        if !p.as_os_str().is_empty() {
            return Some(p);
        }
    }
    let home = home_dir()?;
    for name in ["Documents", "Documentos"] {
        let p = home.join(name);
        if p.is_dir() {
            return Some(p);
        }
    }
    Some(home.join("Documents"))
}

/// Copy files that are not already at `dst`. Existing files stay as the user left them.
pub fn copy_missing(src: &Path, dst: &Path) -> io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        if kind.is_symlink() {
            continue;
        }
        let to = dst.join(entry.file_name());
        if kind.is_dir() {
            copy_missing(&entry.path(), &to)?;
        } else if kind.is_file() && !to.exists() {
            std::fs::copy(entry.path(), &to)?;
        }
    }
    Ok(())
}

/// Add new shipped examples next to the ones already in Documents.
pub fn instalar() -> io::Result<PathBuf> {
    let src = bundled_dir().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "exemplos empacotados não encontrados",
        )
    })?;
    let dst = dest_dir().ok_or_else(|| {
        io::Error::new(io::ErrorKind::NotFound, "pasta Documentos não encontrada")
    })?;
    copy_missing(&src, &dst)?;
    Ok(dst)
}

#[cfg(test)]
mod tests {
    use super::copy_missing;

    #[test]
    fn copy_missing_keeps_edits_and_adds_new_files() {
        let root = std::env::temp_dir().join(format!(
            "expressa-exemplos-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let src = root.join("src");
        let dst = root.join("dst");
        std::fs::create_dir_all(src.join("linguagem")).unwrap();
        std::fs::write(src.join("linguagem").join("media.lep"), "novo").unwrap();
        std::fs::create_dir_all(dst.join("linguagem")).unwrap();
        std::fs::write(dst.join("linguagem").join("media.lep"), "editado").unwrap();
        copy_missing(&src, &dst).unwrap();
        assert_eq!(
            std::fs::read_to_string(dst.join("linguagem").join("media.lep")).unwrap(),
            "editado"
        );
        std::fs::write(src.join("linguagem").join("novo.lep"), "x").unwrap();
        std::fs::create_dir_all(src.join("jogos")).unwrap();
        std::fs::write(src.join("jogos").join("termo.lep"), "jogo").unwrap();
        copy_missing(&src, &dst).unwrap();
        assert_eq!(
            std::fs::read_to_string(dst.join("linguagem").join("media.lep")).unwrap(),
            "editado"
        );
        assert_eq!(
            std::fs::read_to_string(dst.join("linguagem").join("novo.lep")).unwrap(),
            "x"
        );
        assert_eq!(
            std::fs::read_to_string(dst.join("jogos").join("termo.lep")).unwrap(),
            "jogo"
        );
        let _ = std::fs::remove_dir_all(root);
    }
}
