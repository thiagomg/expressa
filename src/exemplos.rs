//! Copy shipped examples into the student's Documents folder.
//! Used by `expressa --instalar-exemplos` and by the OS installers.

use std::io;
use std::path::{Path, PathBuf};

/// Templates next to the binary, in `/usr/share/expressa/exemplos`, or the repo.
pub fn bundled_dir() -> Option<PathBuf> {
    if let Ok(exe) = std::env::current_exe()
        && let Some(dir) = exe.parent()
    {
        for cand in [dir.join("exemplos"), dir.join("share/expressa/exemplos")] {
            if cand.join("linguagem").is_dir() {
                return Some(cand);
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

/// `Documentos/Expressa/exemplos` (or the configured Documents folder).
pub fn dest_dir() -> Option<PathBuf> {
    let docs = documents_dir()?;
    Some(docs.join("Expressa").join("exemplos"))
}

fn documents_dir() -> Option<PathBuf> {
    let home = home_dir()?;
    let config_home = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .unwrap_or_else(|| home.join(".config"));
    let config = std::fs::read_to_string(config_home.join("user-dirs.dirs")).unwrap_or_default();
    Some(documents_dir_for(
        &home,
        std::env::var("XDG_DOCUMENTS_DIR").ok().as_deref(),
        &config,
    ))
}

fn documents_dir_for(home: &Path, override_dir: Option<&str>, config: &str) -> PathBuf {
    fn expand(home: &Path, value: &str) -> Option<PathBuf> {
        let value = value.trim();
        let value = value
            .strip_prefix('"')
            .and_then(|s| s.strip_suffix('"'))
            .unwrap_or(value);
        let path = if value == "$HOME" || value == "${HOME}" {
            home.to_path_buf()
        } else if let Some(tail) = value
            .strip_prefix("$HOME/")
            .or_else(|| value.strip_prefix("${HOME}/"))
        {
            home.join(tail)
        } else {
            PathBuf::from(value)
        };
        path.is_absolute().then_some(path)
    }

    if let Some(path) = override_dir.and_then(|value| expand(home, value)) {
        return path;
    }
    for line in config.lines() {
        if let Some((key, value)) = line.trim().split_once('=')
            && key.trim() == "XDG_DOCUMENTS_DIR"
            && let Some(path) = expand(home, value)
        {
            return path;
        }
    }
    for name in ["Documentos", "Documents"] {
        let path = home.join(name);
        if path.is_dir() {
            return path;
        }
    }
    home.join("Documentos")
}

/// Copy files that are not already at `dst`. Existing files stay as the user left them.
pub fn copy_missing(src: &Path, dst: &Path) -> io::Result<()> {
    // Do not follow a destination symlink or replace a file with a directory.
    match std::fs::symlink_metadata(dst) {
        Ok(metadata) if !metadata.is_dir() => return Ok(()),
        Ok(_) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => std::fs::create_dir_all(dst)?,
        Err(error) => return Err(error),
    }
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        if kind.is_symlink() {
            continue;
        }
        let to = dst.join(entry.file_name());
        if kind.is_dir() {
            copy_missing(&entry.path(), &to)?;
        } else if kind.is_file() {
            if std::fs::symlink_metadata(&to).is_ok() {
                continue;
            }
            let mut source = std::fs::File::open(entry.path())?;
            // Atomic creation also protects edits made while installation runs.
            match std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&to)
            {
                Ok(mut target) => {
                    io::copy(&mut source, &mut target)?;
                }
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
                Err(error) => return Err(error),
            }
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
    use super::{copy_missing, documents_dir_for};

    #[test]
    fn documents_respects_xdg_configuration_without_shell_expansion() {
        let home = std::env::temp_dir().join("expressa-documents-home");
        assert_eq!(
            documents_dir_for(
                &home,
                None,
                "# configuração\nXDG_DOCUMENTS_DIR=\"$HOME/Meus Documentos\"\n"
            ),
            home.join("Meus Documentos")
        );
        assert_eq!(
            documents_dir_for(
                &home,
                Some("${HOME}/Documentos"),
                "XDG_DOCUMENTS_DIR=\"/outro\""
            ),
            home.join("Documentos")
        );
        assert_eq!(
            documents_dir_for(&home, Some(""), "XDG_DOCUMENTS_DIR=relativo"),
            home.join("Documentos")
        );
        assert_eq!(
            documents_dir_for(&home, None, "XDG_DOCUMENTS_DIR=\"$HOME\""),
            home
        );
    }

    #[cfg(unix)]
    #[test]
    fn copy_missing_preserves_destination_symlinks_and_type_conflicts() {
        let root =
            std::env::temp_dir().join(format!("expressa-exemplos-links-{}", std::process::id()));
        let src = root.join("src");
        let dst = root.join("dst");
        std::fs::create_dir_all(src.join("pasta")).unwrap();
        std::fs::create_dir_all(&dst).unwrap();
        std::fs::write(src.join("pasta/novo.lep"), "novo").unwrap();
        std::fs::write(src.join("existente.lep"), "modelo").unwrap();
        std::fs::write(src.join("novo.lep"), "novo").unwrap();
        std::os::unix::fs::symlink(root.join("ausente"), dst.join("existente.lep")).unwrap();
        std::os::unix::fs::symlink(root.join("fora"), dst.join("pasta")).unwrap();
        copy_missing(&src, &dst).unwrap();
        assert!(dst.join("existente.lep").is_symlink());
        assert!(!root.join("ausente").exists());
        assert!(!root.join("fora").exists());
        assert_eq!(
            std::fs::read_to_string(dst.join("novo.lep")).unwrap(),
            "novo"
        );
        std::fs::remove_file(dst.join("pasta")).unwrap();
        std::fs::write(dst.join("pasta"), "arquivo do aluno").unwrap();
        copy_missing(&src, &dst).unwrap();
        assert_eq!(
            std::fs::read_to_string(dst.join("pasta")).unwrap(),
            "arquivo do aluno"
        );
        std::fs::remove_dir_all(root).unwrap();
    }

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
        copy_missing(&src, &dst).unwrap();
        assert_eq!(
            std::fs::read_to_string(dst.join("linguagem/media.lep")).unwrap(),
            "novo"
        );
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
