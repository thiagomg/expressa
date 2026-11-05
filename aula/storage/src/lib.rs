//! Shared conditional, atomic project-file saves. Cooperative writers serialize
//! on a stable directory lock (not the replaced inode). External programs that
//! ignore that lock can still race the final check; no filesystem CAS is claimed.

use sha2::{Digest, Sha256};
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::Path;

const LOCK_NAME: &str = ".expressa-save.lock";
const TEMP_PREFIX: &str = ".expressa-save-";
const OUTPUT_NAME_CHARS: usize = 32;
const OUTPUT_EXTENSION_CHARS: usize = 16;

/// Coordinates local rename/delete with saves in the same editor process.
/// The directory file lock additionally serializes cooperating processes.
pub fn mutation_lock() -> std::sync::MutexGuard<'static, ()> {
    static MUTATIONS: std::sync::Mutex<()> = std::sync::Mutex::new(());
    MUTATIONS.lock().unwrap_or_else(|e| e.into_inner())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Expected {
    Missing,
    Revision(String),
    /// An old server did not supply a revision. Never guess one at save time.
    Unversioned,
}

#[derive(Debug)]
pub enum SaveError {
    Conflict,
    AlreadyExists,
    Unsupported,
    Io(io::Error),
    Other(String),
}

impl std::fmt::Display for SaveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Conflict => f.write_str("o arquivo mudou desde que foi lido"),
            Self::AlreadyExists => f.write_str("o arquivo já existe"),
            Self::Unsupported => f.write_str(
                "o servidor precisa ser atualizado para salvar com proteção contra conflitos",
            ),
            Self::Io(e) => e.fmt(f),
            Self::Other(e) => f.write_str(e),
        }
    }
}

impl std::error::Error for SaveError {}
impl From<io::Error> for SaveError {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}

#[derive(Debug)]
pub struct Snapshot {
    pub bytes: Vec<u8>,
    pub revision: String,
}

#[derive(Debug)]
pub struct Saved {
    pub revision: String,
    /// Publication succeeded. A later durability/cleanup failure must not be
    /// reported as though the original file had remained unchanged.
    pub warning: Option<String>,
}

pub fn revision(bytes: &[u8]) -> String {
    format!(
        "sha256:{}",
        Sha256::digest(bytes)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    )
}

pub fn read(path: &Path) -> io::Result<Snapshot> {
    let bytes = fs::read(path)?;
    let revision = revision(&bytes);
    Ok(Snapshot { bytes, revision })
}

/// Read at most `max_bytes` plus one sentinel byte, including files which grow
/// after the metadata check. Revision always describes the returned bytes.
pub fn read_limited(path: &Path, max_bytes: usize) -> io::Result<Snapshot> {
    use std::io::Read;
    let file = fs::File::open(path)?;
    if file.metadata()?.len() > max_bytes as u64 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "arquivo grande demais",
        ));
    }
    let mut bytes = Vec::new();
    file.take(max_bytes as u64 + 1).read_to_end(&mut bytes)?;
    if bytes.len() > max_bytes {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "arquivo grande demais",
        ));
    }
    let revision = revision(&bytes);
    Ok(Snapshot { bytes, revision })
}

/// Keep binary run output outside a disposable workspace after a conflict.
/// No project file is changed; the returned path can be exported by the student.
pub fn preserve_output(root: &Path, name: &str, bytes: &[u8]) -> io::Result<std::path::PathBuf> {
    fs::create_dir_all(root)?;
    let name = name.replace('\\', "/");
    let name = name.rsplit('/').next().unwrap_or("saida");
    let mut suffix: String = name
        .chars()
        .take(OUTPUT_NAME_CHARS)
        .map(|c| {
            if c.is_alphanumeric() || matches!(c, '.' | '_' | '-') {
                c
            } else {
                '_'
            }
        })
        .collect();
    if let Some((_, extension)) = name.rsplit_once('.') {
        let extension: String = extension
            .chars()
            .take(OUTPUT_EXTENSION_CHARS)
            .filter(|c| c.is_alphanumeric())
            .collect();
        if !extension.is_empty() && !suffix.ends_with(&format!(".{extension}")) {
            suffix.push('.');
            suffix.push_str(&extension);
        }
    }
    let mut file = tempfile::Builder::new()
        .prefix("conflito-")
        .suffix(&format!("-{suffix}"))
        .tempfile_in(root)?;
    file.write_all(bytes)?;
    file.as_file().sync_all()?;
    let (_, path) = file.keep().map_err(|e| e.error)?;
    Ok(path)
}

pub fn is_internal(name: &std::ffi::OsStr) -> bool {
    name.to_str()
        .is_some_and(|s| s == LOCK_NAME || s.starts_with(TEMP_PREFIX))
}

pub fn save(path: &Path, bytes: &[u8], expected: &Expected) -> Result<Saved, SaveError> {
    let _mutation = mutation_lock();
    save_before_publish(path, bytes, expected, || Ok(()))
}

fn check(path: &Path, expected: &Expected) -> Result<(), SaveError> {
    match (read(path), expected) {
        (Err(e), Expected::Missing) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        (Ok(_), Expected::Missing) => Err(SaveError::AlreadyExists),
        (Ok(current), Expected::Revision(rev)) if current.revision == *rev => Ok(()),
        (Ok(_), Expected::Revision(_)) => Err(SaveError::Conflict),
        (Err(e), Expected::Revision(_)) if e.kind() == io::ErrorKind::NotFound => {
            Err(SaveError::Conflict)
        }
        (_, Expected::Unversioned) => Err(SaveError::Unsupported),
        (Err(e), _) => Err(e.into()),
    }
}

fn save_before_publish(
    path: &Path,
    bytes: &[u8],
    expected: &Expected,
    before_publish: impl FnOnce() -> io::Result<()>,
) -> Result<Saved, SaveError> {
    if matches!(expected, Expected::Unversioned) {
        return Err(SaveError::Unsupported);
    }
    if path.file_name().is_none_or(is_internal) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "nome reservado para salvamento",
        )
        .into());
    }
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    // An ordinary save must not recreate a directory removed by another writer.
    if matches!(expected, Expected::Revision(_)) {
        check(path, expected)?;
    }
    fs::create_dir_all(parent)?;
    let lock = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(parent.join(LOCK_NAME))?;
    lock.lock()?;
    check(path, expected)?;
    let permissions = writable_permissions(path)?;
    let mut temp = tempfile::Builder::new()
        .prefix(TEMP_PREFIX)
        .tempfile_in(parent)?;
    temp.write_all(bytes)?;
    if let Some(permissions) = &permissions {
        temp.as_file().set_permissions(permissions.clone())?;
    }
    temp.as_file().sync_all()?;
    before_publish()?;
    // Catch non-cooperating changes during staging where possible as well.
    check(path, expected)?;
    let latest_permissions = writable_permissions(path)?;
    if matches!(expected, Expected::Revision(_)) && latest_permissions.is_none() {
        return Err(SaveError::Conflict);
    }
    if latest_permissions != permissions
        && let Some(permissions) = latest_permissions
    {
        temp.as_file().set_permissions(permissions)?;
        temp.as_file().sync_all()?;
    }
    let warning = match expected {
        Expected::Missing => {
            temp.persist_noclobber(path).map_err(|e| {
                if e.error.kind() == io::ErrorKind::AlreadyExists {
                    SaveError::AlreadyExists
                } else {
                    SaveError::Io(e.error)
                }
            })?;
            None
        }
        Expected::Revision(_) => replace(temp, path)?,
        Expected::Unversioned => unreachable!(),
    };
    #[cfg(unix)]
    let warning = fs::File::open(parent)
        .and_then(|dir| dir.sync_all())
        .err()
        .map(|e| format!("arquivo salvo, mas a pasta não pôde ser sincronizada: {e}"))
        .or(warning);
    Ok(Saved {
        revision: revision(bytes),
        warning,
    })
}

fn writable_permissions(path: &Path) -> io::Result<Option<fs::Permissions>> {
    match fs::symlink_metadata(path) {
        Ok(meta) => {
            if !meta.file_type().is_file() || meta.permissions().readonly() {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "o destino não é um arquivo regular gravável",
                ));
            }
            // Directory write permission alone must not bypass a file's ACL.
            drop(OpenOptions::new().write(true).open(path)?);
            Ok(Some(meta.permissions()))
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e),
    }
}

#[cfg(not(windows))]
fn replace(temp: tempfile::NamedTempFile, path: &Path) -> io::Result<Option<String>> {
    temp.persist(path).map_err(|e| e.error)?;
    Ok(None)
}

#[cfg(windows)]
fn replace(temp: tempfile::NamedTempFile, path: &Path) -> io::Result<Option<String>> {
    use std::os::windows::ffi::OsStrExt;
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn SetFileAttributesW(path: *const u16, attributes: u32) -> i32;
        fn ReplaceFileW(
            replaced: *const u16,
            replacement: *const u16,
            backup: *const u16,
            flags: u32,
            exclude: *mut std::ffi::c_void,
            reserved: *mut std::ffi::c_void,
        ) -> i32;
    }
    const FILE_ATTRIBUTE_NORMAL: u32 = 0x0000_0080;
    const PRESERVE_METADATA_FLAGS: u32 = 0;
    // ReplaceFile preserves the target's DACL, attributes and named streams.
    // Its documented exceptional failures may move the original to the backup;
    // keep that backup and restore it when possible instead of deleting data.
    let backup = tempfile::Builder::new()
        .prefix(TEMP_PREFIX)
        .tempfile_in(path.parent().unwrap())?;
    let backup_path = backup.path().to_path_buf();
    backup.close()?;
    let wide = |p: &Path| {
        p.as_os_str()
            .encode_wide()
            .chain(Some(0))
            .collect::<Vec<_>>()
    };
    let target = wide(path);
    // ReplaceFile opens its replacement without sharing. Close the staging
    // handle first, while keeping its TempPath alive for cleanup on failure.
    let (file, temp_path) = temp.into_parts();
    drop(file);
    let source = wide(&temp_path);
    let backup_name = wide(&backup_path);
    // SAFETY: `source` remains NUL-terminated and live for the native call.
    if unsafe { SetFileAttributesW(source.as_ptr(), FILE_ATTRIBUTE_NORMAL) } == 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: all paths are NUL-terminated UTF-16; pointers remain live through
    // the call. Reserved pointers are null; default flags preserve metadata.
    let ok = unsafe {
        ReplaceFileW(
            target.as_ptr(),
            source.as_ptr(),
            backup_name.as_ptr(),
            PRESERVE_METADATA_FLAGS,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };
    if ok == 0 {
        let error = io::Error::last_os_error();
        if backup_path.exists()
            && !path.exists()
            && let Err(restore) = fs::rename(&backup_path, path)
        {
            return Err(io::Error::other(format!(
                "{error}; original preservado em {}: {restore}",
                backup_path.display()
            )));
        }
        return Err(if backup_path.exists() {
            io::Error::new(
                error.kind(),
                format!("{error}; cópia original: {}", backup_path.display()),
            )
        } else {
            error
        });
    }
    Ok(fs::remove_file(&backup_path).err().map(|e| {
        format!(
            "arquivo salvo; cópia anterior preservada em {}: {e}",
            backup_path.display()
        )
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn limited_reads_accept_exact_bounds_and_reject_larger_files() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.bin");
        fs::write(&path, b"abc").unwrap();
        let snapshot = read_limited(&path, 3).unwrap();
        assert_eq!(snapshot.bytes, b"abc");
        assert_eq!(snapshot.revision, revision(b"abc"));
        assert_eq!(
            read_limited(&path, 2).unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
        fs::write(&path, []).unwrap();
        assert!(read_limited(&path, 0).unwrap().bytes.is_empty());
    }

    #[test]
    fn failure_before_publication_preserves_original_and_cleans_temporary_files() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("a.lep");
        fs::write(&path, "original").unwrap();
        let expected = Expected::Revision(read(&path).unwrap().revision);
        assert!(
            save_before_publish(&path, b"partial replacement", &expected, || Err(
                io::Error::other("injected failure")
            ))
            .is_err()
        );
        assert_eq!(fs::read(&path).unwrap(), b"original");
        assert!(!fs::read_dir(root.path()).unwrap().any(|e| {
            e.unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with(TEMP_PREFIX)
        }));
    }

    #[test]
    fn stale_revision_and_exclusive_creation_never_replace_newer_content() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("ação.lep");
        let first = save(&path, b"first", &Expected::Missing).unwrap();
        assert!(matches!(
            save(&path, b"duplicate", &Expected::Missing),
            Err(SaveError::AlreadyExists)
        ));
        save(&path, b"newer", &Expected::Revision(first.revision.clone())).unwrap();
        assert!(matches!(
            save(&path, b"stale", &Expected::Revision(first.revision)),
            Err(SaveError::Conflict)
        ));
        assert_eq!(fs::read(&path).unwrap(), b"newer");
        let expected = Expected::Revision(read(&path).unwrap().revision);
        fs::remove_dir_all(root.path()).unwrap();
        assert!(matches!(
            save(&path, b"recreated", &expected),
            Err(SaveError::Conflict)
        ));
        assert!(
            !root.path().exists(),
            "ordinary save recreated a removed parent directory"
        );
    }

    #[test]
    fn concurrent_writers_share_a_lock_across_atomic_replacements() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("a.lep");
        let revision = save(&path, b"base", &Expected::Missing).unwrap().revision;
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
        let results = std::thread::scope(|scope| {
            let jobs: Vec<_> = [b"one", b"two"]
                .into_iter()
                .map(|bytes| {
                    let (path, revision, barrier) =
                        (path.clone(), revision.clone(), barrier.clone());
                    scope.spawn(move || {
                        barrier.wait();
                        save(&path, bytes, &Expected::Revision(revision))
                    })
                })
                .collect();
            jobs.into_iter()
                .map(|job| job.join().unwrap())
                .collect::<Vec<_>>()
        });
        assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 1);
        assert_eq!(
            results
                .iter()
                .filter(|r| matches!(r, Err(SaveError::Conflict)))
                .count(),
            1
        );
    }

    #[test]
    fn read_only_and_external_staging_changes_are_preserved() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("a.lep");
        fs::write(&path, "base").unwrap();
        let expected = Expected::Revision(read(&path).unwrap().revision);
        assert!(matches!(
            save_before_publish(&path, b"ours", &expected, || fs::write(&path, "external")),
            Err(SaveError::Conflict)
        ));
        let mut permissions = fs::metadata(&path).unwrap().permissions();
        #[cfg(windows)]
        let original_permissions = permissions.clone();
        permissions.set_readonly(true);
        let expected = Expected::Revision(read(&path).unwrap().revision);
        assert!(
            save_before_publish(&path, b"ours", &expected, || fs::set_permissions(
                &path,
                permissions
            ))
            .is_err()
        );
        assert!(
            save(
                &path,
                b"ours",
                &Expected::Revision(read(&path).unwrap().revision)
            )
            .is_err()
        );
        assert_eq!(fs::read(&path).unwrap(), b"external");
        #[cfg(windows)]
        {
            fs::set_permissions(&path, original_permissions).unwrap();
        }
    }

    #[cfg(unix)]
    #[test]
    fn preserves_unix_mode_bits() {
        use std::os::unix::fs::PermissionsExt;
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("a.lep");
        fs::write(&path, "base").unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o640)).unwrap();
        save(
            &path,
            b"new",
            &Expected::Revision(read(&path).unwrap().revision),
        )
        .unwrap();
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o640
        );
    }

    const CONFLICT_EXIT: i32 = 23;
    const CHILD_PATH: &str = "EXPRESSA_STORAGE_TEST_PATH";
    const CHILD_REVISION: &str = "EXPRESSA_STORAGE_TEST_REVISION";
    const CHILD_TEXT: &str = "EXPRESSA_STORAGE_TEST_TEXT";

    #[test]
    #[ignore = "subprocess helper, invoked by the cross-process test"]
    fn writer_process() {
        let path = std::path::PathBuf::from(std::env::var_os(CHILD_PATH).unwrap());
        let expected = Expected::Revision(std::env::var(CHILD_REVISION).unwrap());
        let text = std::env::var(CHILD_TEXT).unwrap();
        match save(&path, text.as_bytes(), &expected) {
            Ok(_) => {}
            Err(SaveError::Conflict) => std::process::exit(CONFLICT_EXIT),
            Err(e) => panic!("{e}"),
        }
    }

    #[test]
    fn competing_processes_cannot_both_commit_the_same_baseline() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("a.lep");
        let revision = save(&path, b"base", &Expected::Missing).unwrap().revision;
        let mut children: Vec<_> = ["first", "second"]
            .into_iter()
            .map(|text| {
                std::process::Command::new(std::env::current_exe().unwrap())
                    .args(["--ignored", "--exact", "tests::writer_process"])
                    .env(CHILD_PATH, &path)
                    .env(CHILD_REVISION, &revision)
                    .env(CHILD_TEXT, text)
                    .spawn()
                    .unwrap()
            })
            .collect();
        let codes: Vec<_> = children
            .iter_mut()
            .map(|child| child.wait().unwrap().code())
            .collect();
        assert_eq!(codes.iter().filter(|code| **code == Some(0)).count(), 1);
        assert_eq!(
            codes
                .iter()
                .filter(|code| **code == Some(CONFLICT_EXIT))
                .count(),
            1
        );
    }

    #[test]
    fn binary_conflict_outputs_keep_bytes_and_extension_without_clobbering() {
        let root = tempfile::tempdir().unwrap();
        let name = format!("{}.png", "ação".repeat(80));
        let first = preserve_output(root.path(), &name, &[0xff, 0x00]).unwrap();
        let second = preserve_output(root.path(), &name, &[0xff, 0x01]).unwrap();
        assert_ne!(first, second);
        assert_eq!(first.extension().unwrap(), "png");
        assert_eq!(fs::read(first).unwrap(), [0xff, 0x00]);
        assert_eq!(fs::read(second).unwrap(), [0xff, 0x01]);
    }

    #[cfg(windows)]
    #[test]
    fn windows_replacement_preserves_explicit_acl_and_named_streams() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("ação.lep");
        fs::write(&path, "base").unwrap();
        let grant = std::process::Command::new("icacls.exe")
            .arg(&path)
            .args(["/grant", "*S-1-1-0:(R)"])
            .output()
            .unwrap();
        assert!(grant.status.success(), "{:?}", grant);
        let before = std::process::Command::new("icacls.exe")
            .arg(&path)
            .output()
            .unwrap();
        assert!(before.status.success());
        let mut stream = path.as_os_str().to_os_string();
        stream.push(":student-note");
        fs::write(&stream, "keep this stream").unwrap();
        save(
            &path,
            b"new",
            &Expected::Revision(read(&path).unwrap().revision),
        )
        .unwrap();
        let after = std::process::Command::new("icacls.exe")
            .arg(&path)
            .output()
            .unwrap();
        assert!(after.status.success());
        assert_eq!(before.stdout, after.stdout);
        assert_eq!(fs::read_to_string(&stream).unwrap(), "keep this stream");
        assert_eq!(fs::read(&path).unwrap(), b"new");
    }
}
