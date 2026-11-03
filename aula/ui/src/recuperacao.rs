//! Durable recovery copies. New snapshots are published under fresh names;
//! an interrupted write never truncates the previous snapshot. Legacy copies
//! (encoded filename, plain source contents) remain readable.

use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

const HEADER: &str = "EXPRESSA-RECOVERY-1";
pub const UNTITLED_PREFIX: &str = "sem-titulo:";

#[derive(Debug, Clone)]
pub struct Backup {
    pub id: String,
    /// Original project-relative path, or `sem-titulo:<original tab name>`.
    pub name: String,
    pub revision: u64,
    /// Latest snapshot plus any older snapshots whose cleanup failed.
    pub paths: Vec<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct Copy {
    pub backup: Backup,
    pub text: String,
}

#[derive(Default)]
pub struct Scan {
    pub copies: Vec<Copy>,
    pub errors: Vec<String>,
    pub unreadable: Vec<PathBuf>,
}

#[derive(Clone)]
pub struct RecoveryStore(pub PathBuf);

impl RecoveryStore {
    pub fn scan(&self) -> Scan {
        let mut scan = Scan::default();
        let entries = match fs::read_dir(&self.0) {
            Ok(entries) => entries,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return scan,
            Err(e) => {
                scan.errors.push(format!("{}: {e}", self.0.display()));
                return scan;
            }
        };
        let mut copies: BTreeMap<String, Copy> = BTreeMap::new();
        for entry in entries {
            let failed_path = entry.as_ref().ok().map(|entry| entry.path());
            let result = entry.map_err(|e| e.to_string()).and_then(|entry| {
                let path = entry.path();
                let filename = entry.file_name().to_string_lossy().into_owned();
                if filename.starts_with('.') {
                    // Unpublished temporary writes are never recovery candidates.
                    return Ok(None);
                }
                if !entry.file_type().map_err(|e| e.to_string())?.is_file() {
                    return Err(format!(
                        "{}: cópia não é um arquivo regular",
                        path.display()
                    ));
                }
                read_copy(&path, &filename).map(Some)
            });
            match result {
                Ok(Some(mut copy)) => {
                    if let Some(previous) = copies.remove(&copy.backup.id) {
                        // Prefer a complete newer generation after an interrupted
                        // cleanup. Keep all paths so explicit discard removes all.
                        if (previous.backup.revision, &previous.backup.paths[0])
                            > (copy.backup.revision, &copy.backup.paths[0])
                        {
                            let paths = copy.backup.paths;
                            copy = previous;
                            copy.backup.paths.extend(paths);
                        } else {
                            copy.backup.paths.extend(previous.backup.paths);
                        }
                    }
                    copies.insert(copy.backup.id.clone(), copy);
                }
                Ok(None) => {}
                Err(e) => {
                    scan.errors.push(e);
                    if let Some(path) = failed_path {
                        scan.unreadable.push(path);
                    }
                }
            }
        }
        scan.copies = copies.into_values().collect();
        scan
    }

    /// Commit a replacement before retiring any previous copy. Cancelled and
    /// failed recoveries are never swept just because no tab owns them.
    pub fn save(
        &self,
        name: &str,
        text: &str,
        previous: Option<&Backup>,
    ) -> Result<Backup, String> {
        if name.is_empty() || name.contains('\0') {
            return Err("nome inválido para a cópia de recuperação".into());
        }
        fs::create_dir_all(&self.0).map_err(|e| format!("{}: {e}", self.0.display()))?;
        let nonce = unique_name();
        let id = previous.map_or_else(|| nonce.clone(), |b| b.id.clone());
        let revision = previous
            .map_or(Some(1), |b| b.revision.checked_add(1))
            .ok_or("muitas versões da cópia de recuperação")?;
        let temporary = self.0.join(format!(".{nonce}.tmp"));
        let destination = self.0.join(format!("{nonce}.recovery"));
        let result = (|| -> std::io::Result<()> {
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temporary)?;
            write!(
                file,
                "{HEADER}\n{id}\n{revision}\n{}\n{text}",
                nome_de_arquivo(name)
            )?;
            file.sync_all()?;
            drop(file);
            fs::rename(&temporary, &destination)?;
            #[cfg(unix)]
            fs::File::open(&self.0)?.sync_all()?;
            Ok(())
        })();
        if let Err(e) = result {
            let _ = fs::remove_file(&temporary);
            return Err(format!("{}: {e}", destination.display()));
        }
        let mut paths = vec![destination];
        if let Some(previous) = previous {
            for path in &previous.paths {
                if let Err(e) = fs::remove_file(path)
                    && e.kind() != std::io::ErrorKind::NotFound
                {
                    paths.push(path.clone());
                }
            }
        }
        Ok(Backup {
            id,
            name: name.to_string(),
            revision,
            paths,
        })
    }
}

impl Backup {
    /// Only call after a successful save or an explicit discard of this copy.
    pub fn remove(&self) -> Result<(), String> {
        let mut errors = Vec::new();
        for path in &self.paths {
            if let Err(e) = fs::remove_file(path)
                && e.kind() != std::io::ErrorKind::NotFound
            {
                errors.push(format!("{}: {e}", path.display()));
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors.join("\n"))
        }
    }
}

fn read_copy(path: &Path, filename: &str) -> Result<Copy, String> {
    let contents = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let read = || -> Option<Copy> {
        let text = contents;
        let (id, name, revision, text) = if filename.ends_with(".recovery") {
            let mut fields = text.splitn(5, '\n');
            if fields.next()? != HEADER {
                return None;
            }
            let id = fields.next()?.to_string();
            if id.is_empty()
                || !id
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"-_%:".contains(&b))
            {
                return None;
            }
            let revision = fields.next()?.parse().ok()?;
            let name = nome_de_volta(fields.next()?)?;
            (id, name, revision, fields.next()?.to_string())
        } else {
            (
                format!("legacy:{}", nome_de_arquivo(filename)),
                nome_de_volta(filename)?,
                0,
                text,
            )
        };
        if name.is_empty() || name.contains('\0') {
            return None;
        }
        Some(Copy {
            backup: Backup {
                id,
                name,
                revision,
                paths: vec![path.to_path_buf()],
            },
            text,
        })
    };
    read().ok_or_else(|| {
        format!(
            "{}: não foi possível ler a cópia de recuperação",
            path.display()
        )
    })
}

fn unique_name() -> String {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!(
        "{time:039}-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    )
}

/// Export source text without replacing any existing file in the chosen folder.
pub fn export(copies: &[Copy], folder: &Path) -> Vec<String> {
    let mut errors = Vec::new();
    for copy in copies {
        let name = copy
            .backup
            .name
            .strip_prefix(UNTITLED_PREFIX)
            .unwrap_or(&copy.backup.name);
        let normalized = name.replace('\\', "/");
        let basename = normalized.rsplit('/').next().unwrap_or("recuperado.lep");
        let basename: String = basename
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || matches!(c, '.' | '_' | '-') {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        let result = export_file(folder, &basename, copy.text.as_bytes());
        if let Err(e) = result {
            errors.push(format!("{}: {e}", copy.backup.name));
        }
    }
    errors
}

/// A malformed/non-UTF-8 recovery file can still be exported byte for byte for
/// manual inspection. Never follow symlinks or remove the source on failure.
pub fn export_unreadable(paths: &[PathBuf], folder: &Path) -> Vec<String> {
    let mut errors = Vec::new();
    for path in paths {
        let result = (|| -> std::io::Result<()> {
            if !fs::symlink_metadata(path)?.file_type().is_file() {
                return Err(std::io::Error::other("cópia não é um arquivo regular"));
            }
            let name = path.file_name().unwrap_or_default().to_string_lossy();
            export_file(folder, &name, &fs::read(path)?)
        })();
        if let Err(e) = result {
            errors.push(format!("{}: {e}", path.display()));
        }
    }
    errors
}

fn export_file(folder: &Path, basename: &str, bytes: &[u8]) -> std::io::Result<()> {
    let basename = if matches!(basename, "" | "." | "..") {
        "recuperado.lep"
    } else {
        basename
    };
    for suffix in 0..1000 {
        let name = if suffix == 0 {
            basename.to_string()
        } else {
            format!("{suffix}-{basename}")
        };
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(folder.join(name))
        {
            Ok(mut file) => {
                file.write_all(bytes)?;
                return file.sync_all();
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e),
        }
    }
    Err(std::io::Error::other("muitos arquivos com o mesmo nome"))
}

/// Reversible encoding used by legacy backups and project/session keys.
pub fn nome_de_arquivo(nome: &str) -> String {
    let mut out = String::new();
    for b in nome.bytes() {
        if b.is_ascii_alphanumeric() || b == b'-' || b == b'_' {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

pub fn nome_de_volta(arquivo: &str) -> Option<String> {
    let mut bytes = Vec::new();
    let mut it = arquivo.bytes();
    while let Some(b) = it.next() {
        if b == b'%' {
            let hex = [it.next()?, it.next()?];
            bytes.push(u8::from_str_radix(std::str::from_utf8(&hex).ok()?, 16).ok()?);
        } else {
            bytes.push(b);
        }
    }
    String::from_utf8(bytes).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fixture(PathBuf);

    impl Fixture {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!("expressa-recovery-{}", unique_name()));
            fs::create_dir(&path).unwrap();
            Self(path)
        }

        fn store(&self) -> RecoveryStore {
            RecoveryStore(self.0.join("copies"))
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn legacy_and_unreadable_copies_survive_a_scan() {
        let fixture = Fixture::new();
        let store = fixture.store();
        fs::create_dir(&store.0).unwrap();
        let legacy = store.0.join(nome_de_arquivo("ação/a.lep"));
        fs::write(&legacy, "escreva(\"olá\")\n").unwrap();
        let unreadable = store.0.join("invalid.recovery");
        fs::write(&unreadable, [0xff, 0x00]).unwrap();
        let malformed = store.0.join("bad.recovery");
        fs::write(&malformed, "incomplete header").unwrap();
        let scan = store.scan();
        assert_eq!(scan.copies.len(), 1);
        assert_eq!(scan.errors.len(), 2);
        assert_eq!(scan.unreadable.len(), 2);
        assert_eq!(scan.copies[0].backup.name, "ação/a.lep");
        assert_eq!(scan.copies[0].text, "escreva(\"olá\")\n");
        assert!(legacy.exists() && unreadable.exists() && malformed.exists());
        assert!(export_unreadable(&scan.unreadable, &fixture.0).is_empty());
        assert_eq!(
            fs::read(fixture.0.join("invalid.recovery")).unwrap(),
            [0xff, 0x00]
        );
        // Explicit discard of the readable copy does not touch failed reads.
        scan.copies[0].backup.remove().unwrap();
        assert!(store.scan().copies.is_empty());
        assert!(unreadable.exists() && malformed.exists());
    }

    #[test]
    fn failed_and_interrupted_updates_keep_the_previous_snapshot() {
        let fixture = Fixture::new();
        let store = fixture.store();
        let original = store.save("a.lep", "before", None).unwrap();
        let blocked = fixture.0.join("blocked");
        fs::write(&blocked, "not a directory").unwrap();
        assert!(
            RecoveryStore(blocked)
                .save("a.lep", "after", Some(&original))
                .is_err()
        );
        fs::write(store.0.join(".unfinished.tmp"), "partial snapshot").unwrap();
        let scan = store.scan();
        assert!(scan.errors.is_empty());
        assert_eq!(scan.copies.len(), 1);
        assert_eq!(scan.copies[0].text, "before");
        assert!(original.paths[0].exists());
    }

    #[test]
    fn restart_selects_latest_complete_generation_and_discards_all_its_versions() {
        let fixture = Fixture::new();
        let store = fixture.store();
        fs::create_dir(&store.0).unwrap();
        let legacy = store.0.join(nome_de_arquivo("a.lep"));
        fs::write(&legacy, "before").unwrap();
        let original = store.scan().copies.pop().unwrap();
        let next = store
            .save("a.lep", "after\n", Some(&original.backup))
            .unwrap();
        // Simulate a crash/failure during cleanup after publishing the new copy.
        fs::write(&legacy, "before").unwrap();
        let scan = RecoveryStore(store.0.clone()).scan();
        assert_eq!(scan.copies.len(), 1);
        let copy = &scan.copies[0];
        assert_eq!(copy.text, "after\n");
        assert_eq!(copy.backup.id, next.id);
        assert_eq!(copy.backup.paths.len(), 2);
        copy.backup.remove().unwrap();
        assert!(!legacy.exists());
        assert!(store.scan().copies.is_empty());
    }

    #[test]
    fn autosave_and_individual_discard_do_not_sweep_cancelled_copies() {
        let fixture = Fixture::new();
        let store = fixture.store();
        let cancelled = store
            .save("sem-titulo:novo.lep", "cancelled work", None)
            .unwrap();
        let active = store
            .save("sem-titulo:novo.lep", "active work", None)
            .unwrap();
        assert_ne!(cancelled.id, active.id);
        let active = store
            .save("sem-titulo:novo.lep", "new active work", Some(&active))
            .unwrap();
        assert_eq!(store.scan().copies.len(), 2);
        active.remove().unwrap();
        let scan = store.scan();
        assert_eq!(scan.copies.len(), 1);
        assert_eq!(scan.copies[0].text, "cancelled work");
        assert!(cancelled.paths[0].exists());
    }

    #[test]
    fn export_never_replaces_files_or_retires_recovery_copies() {
        let fixture = Fixture::new();
        let store = fixture.store();
        let backup = store.save("lib/a.lep", "recovered source", None).unwrap();
        fs::write(fixture.0.join("a.lep"), "keep me").unwrap();
        let scan = store.scan();
        assert!(export(&scan.copies, &fixture.0).is_empty());
        assert_eq!(
            fs::read_to_string(fixture.0.join("a.lep")).unwrap(),
            "keep me"
        );
        assert_eq!(
            fs::read_to_string(fixture.0.join("1-a.lep")).unwrap(),
            "recovered source"
        );
        assert!(backup.paths[0].exists());
        assert_eq!(export(&scan.copies, &fixture.0.join("a.lep")).len(), 1);
        assert!(backup.paths[0].exists());
    }
}
