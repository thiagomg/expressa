//! Durable run copies. The journal lives outside the interpreter's filesystem root.
//! A held OS lock owns each run; discovery/retry never touches a live execution.
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, Mutex};

use crate::identidade::ProjectIdentity;
use crate::store::Files;
use expressa_aula_storage::{Expected, SaveError, Saved, revision};
use serde::{Deserialize, Serialize};

const MANIFEST: &str = "run.json";
const LEASE: &str = "owner.lock";
const FORMAT_VERSION: u32 = 1;
const HASH_BUFFER_BYTES: usize = 64 * 1024;
// Admission limits, never eviction of unsent work. Completed runs are removed.
const MAX_RETAINED_RUNS: usize = 100;
const MAX_RETAINED_BYTES: u64 = 512 * 1024 * 1024;

#[derive(Clone, Debug, Serialize, Deserialize)]
enum Expectation {
    Missing,
    Revision(String),
    Unversioned,
}
impl From<&Expected> for Expectation {
    fn from(e: &Expected) -> Self {
        match e {
            Expected::Missing => Self::Missing,
            Expected::Revision(r) => Self::Revision(r.clone()),
            Expected::Unversioned => Self::Unversioned,
        }
    }
}
impl From<&Expectation> for Expected {
    fn from(e: &Expectation) -> Self {
        match e {
            Expectation::Missing => Self::Missing,
            Expectation::Revision(r) => Self::Revision(r.clone()),
            Expectation::Unversioned => Self::Unversioned,
        }
    }
}
#[derive(Clone, Serialize, Deserialize)]
struct Entry {
    baseline: String,
    expected: Expectation,
    reason: Option<String>,
}
#[derive(Clone, Serialize, Deserialize)]
struct Manifest {
    version: u32,
    identity: ProjectIdentity,
    ready: bool,
    entries: BTreeMap<String, Entry>,
}

pub struct Journal {
    pub run_dir: PathBuf,
    state: Mutex<Manifest>,
    _lease: Arc<File>,
}

pub struct Output {
    pub path: String,
    pub size: u64,
    digest: String,
    pub expected: Expected,
    pub reason: String,
}
#[derive(Clone)]
pub struct PendingRun {
    pub path: PathBuf,
    pub identity: ProjectIdentity,
    pub files: Vec<(String, u64, String)>,
}
#[derive(Default)]
pub struct Scan {
    pub runs: Vec<PendingRun>,
    pub errors: Vec<String>,
    pub unreadable: Vec<PathBuf>,
}
#[derive(Default)]
pub struct Delivery {
    pub sent: Vec<(String, Option<String>)>,
    pub conflicts: Vec<(String, Vec<u8>)>,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

pub fn root() -> PathBuf {
    gtk::glib::user_data_dir()
        .join("expressa-aula")
        .join("execucoes")
}

pub fn valid_relative(path: &str) -> bool {
    !path.is_empty()
        && !path.contains('\\')
        && !path.contains(':')
        && !path.starts_with('/')
        && !path.ends_with('/')
        && path
            .split('/')
            .all(|c| !c.is_empty() && c != "." && c != "..")
        && Path::new(path)
            .components()
            .all(|c| matches!(c, Component::Normal(_)))
}

/// Refuse links rather than copying a target outside the owned run.
fn walk(root: &Path, dir: &Path, out: &mut Vec<(String, PathBuf)>) -> Result<(), String> {
    if !fs::symlink_metadata(dir)
        .map_err(|e| e.to_string())?
        .is_dir()
    {
        return Err(format!(
            "a pasta de saída foi substituída: {}",
            dir.display()
        ));
    }
    let mut errors = Vec::new();
    for entry in fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))? {
        let result = (|| -> Result<(), String> {
            let entry = entry.map_err(|e| e.to_string())?;
            let meta = entry.file_type().map_err(|e| e.to_string())?;
            let path = entry.path();
            if meta.is_symlink() {
                return Err(format!("link em saída de execução: {}", path.display()));
            }
            if meta.is_dir() {
                walk(root, &path, out)?;
            } else if meta.is_file() {
                let rel = path
                    .strip_prefix(root)
                    .map_err(|e| e.to_string())?
                    .to_str()
                    .ok_or("nome de saída não é UTF-8")?
                    .replace('\\', "/");
                if !valid_relative(&rel) {
                    return Err(format!("caminho de saída inválido: {rel}"));
                }
                out.push((rel, path));
            } else {
                return Err(format!("saída não é arquivo regular: {}", path.display()));
            }
            Ok(())
        })();
        if let Err(e) = result {
            errors.push(e);
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("\n"))
    }
}

fn fingerprint(path: &Path) -> Result<(String, u64), String> {
    if !fs::symlink_metadata(path)
        .map_err(|e| e.to_string())?
        .is_file()
    {
        return Err(format!("saída não é arquivo regular: {}", path.display()));
    }
    let mut file = File::open(path).map_err(|e| e.to_string())?;
    let mut hash = Sha256::new();
    let mut bytes = 0;
    let mut buffer = [0; HASH_BUFFER_BYTES];
    loop {
        let read = file.read(&mut buffer).map_err(|e| e.to_string())?;
        if read == 0 {
            break;
        }
        hash.update(&buffer[..read]);
        bytes += read as u64;
    }
    let digest = hash
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    Ok((format!("sha256:{digest}"), bytes))
}

fn flush_output(path: &Path, root: &Path) -> Result<(), String> {
    #[cfg(windows)]
    let file = OpenOptions::new().write(true).open(path);
    #[cfg(not(windows))]
    let file = File::open(path);
    file.and_then(|f| f.sync_all()).map_err(|e| e.to_string())?;
    let mut parent = path.parent();
    while let Some(dir) = parent {
        sync_directory(dir)?;
        if dir == root {
            break;
        }
        parent = dir.parent();
    }
    Ok(())
}

fn copy_export(source: &Path, target: &Path) -> Result<(), String> {
    if !fs::symlink_metadata(source)
        .map_err(|e| e.to_string())?
        .is_file()
    {
        return Err("saída não é arquivo regular".into());
    }
    let mut input = File::open(source).map_err(|e| e.to_string())?;
    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(target)
        .map_err(|e| e.to_string())?;
    std::io::copy(&mut input, &mut output)
        .and_then(|_| output.sync_all())
        .map_err(|e| e.to_string())?;
    sync_directory(target.parent().ok_or("saída sem pasta")?)
}

fn persist(dir: &Path, state: &Manifest) -> Result<(), String> {
    let path = dir.join(MANIFEST);
    let expected = match expressa_aula_storage::read(&path) {
        Ok(s) => Expected::Revision(s.revision),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Expected::Missing,
        Err(e) => return Err(e.to_string()),
    };
    let bytes = serde_json::to_vec(state).map_err(|e| e.to_string())?;
    let saved = expressa_aula_storage::save(&path, &bytes, &expected).map_err(|e| e.to_string())?;
    // A journal flush warning is a durability failure, even after publication.
    if let Some(warning) = saved.warning {
        return Err(warning);
    }
    Ok(())
}

fn sync_directory(path: &Path) -> Result<(), String> {
    #[cfg(unix)]
    File::open(path)
        .and_then(|f| f.sync_all())
        .map_err(|e| e.to_string())?;
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
}

// The export is built in a unique TempDir and retained only when complete.
// Exclusive creation also permits exporting raw files with reserved storage names.
fn save_export(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| e.to_string())?;
    file.write_all(bytes)
        .and_then(|()| file.sync_all())
        .map_err(|e| e.to_string())?;
    sync_directory(path.parent().ok_or("arquivo exportado sem pasta")?)
}

fn export_destination(run: &Path, folder: &Path) -> Result<(), String> {
    let folder = folder.canonicalize().map_err(|e| e.to_string())?;
    let run = run.canonicalize().map_err(|e| e.to_string())?;
    if folder.starts_with(run) {
        return Err("exporte para uma pasta fora da execução preservada".into());
    }
    Ok(())
}

fn acquire(path: &Path) -> Result<Option<File>, String> {
    if !fs::symlink_metadata(path)
        .map_err(|e| e.to_string())?
        .is_dir()
        || !fs::symlink_metadata(path.join(LEASE))
            .map_err(|e| e.to_string())?
            .is_file()
    {
        return Err("registro ou trava não é arquivo regular".into());
    }
    let lease = OpenOptions::new()
        .write(true)
        .open(path.join(LEASE))
        .map_err(|e| e.to_string())?;
    match lease.try_lock() {
        Ok(()) => Ok(Some(lease)),
        Err(std::fs::TryLockError::WouldBlock) => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}

/// Preserve raw records too when a corrupt/unknown manifest cannot be decoded.
/// The lease remains required: corruption never permits exporting a live run.
pub fn export_raw(path: &Path, folder: &Path) -> Result<PathBuf, String> {
    let _lease = acquire(path)?.ok_or("execução em uso")?;
    export_destination(path, folder)?;
    let mut paths = Vec::new();
    walk(path, path, &mut paths)?;
    let dest = tempfile::Builder::new()
        .prefix("execucao-nao-lida-")
        .tempdir_in(folder)
        .map_err(|e| e.to_string())?;
    for (rel, source) in paths {
        let target = dest.path().join(rel);
        fs::create_dir_all(target.parent().unwrap()).map_err(|e| e.to_string())?;
        copy_export(&source, &target)?;
    }
    sync_directory(folder)?;
    Ok(dest.keep())
}

impl Journal {
    pub fn create(root: &Path, identity: ProjectIdentity) -> Result<Self, String> {
        fs::create_dir_all(root).map_err(|e| e.to_string())?;
        let _ = scan(root);
        check_budget(root)?;
        let parent = root.join(identity.key());
        fs::create_dir_all(&parent).map_err(|e| e.to_string())?;
        let run_dir = tempfile::Builder::new()
            .prefix("run-")
            .tempdir_in(&parent)
            .map_err(|e| e.to_string())?
            .keep();
        let lease = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(run_dir.join(LEASE))
            .map_err(|e| e.to_string())?;
        lease.lock().map_err(|e| e.to_string())?;
        fs::create_dir(run_dir.join("files")).map_err(|e| e.to_string())?;
        let state = Manifest {
            version: FORMAT_VERSION,
            identity,
            ready: false,
            entries: BTreeMap::new(),
        };
        persist(&run_dir, &state)?;
        // Persist the links to the new project/run directories as well.
        for parent in run_dir.ancestors() {
            sync_directory(parent)?;
        }
        Ok(Self {
            run_dir,
            state: Mutex::new(state),
            _lease: Arc::new(lease),
        })
    }

    pub fn open(path: &Path) -> Result<Option<Self>, String> {
        let Some(lease) = acquire(path)? else {
            return Ok(None);
        };
        if !fs::symlink_metadata(path.join(MANIFEST))
            .map_err(|e| e.to_string())?
            .is_file()
        {
            return Err("registro de execução não é arquivo regular".into());
        }
        let bytes = fs::read(path.join(MANIFEST)).map_err(|e| e.to_string())?;
        let state: Manifest = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
        if state.version != FORMAT_VERSION {
            return Err("versão desconhecida do registro de execução".into());
        }
        if path
            .parent()
            .and_then(Path::file_name)
            .and_then(|s| s.to_str())
            != Some(state.identity.key().as_str())
        {
            return Err("identidade do registro não corresponde à pasta".into());
        }
        if state.entries.keys().any(|p| !valid_relative(p)) {
            return Err("registro com caminho inválido".into());
        }
        Ok(Some(Self {
            run_dir: path.to_path_buf(),
            state: Mutex::new(state),
            _lease: Arc::new(lease),
        }))
    }

    pub fn dir(&self) -> PathBuf {
        self.run_dir.join("files")
    }
    pub fn lease(&self) -> Arc<File> {
        Arc::clone(&self._lease)
    }

    pub fn identity(&self) -> ProjectIdentity {
        self.state.lock().unwrap().identity.clone()
    }

    pub fn baseline(&self, revisions: &HashMap<String, Expected>) -> Result<(), String> {
        let mut paths = Vec::new();
        walk(&self.dir(), &self.dir(), &mut paths)?;
        let mut state = self.state.lock().unwrap();
        for (rel, path) in paths {
            let (digest, _) = fingerprint(&path)?;
            let expected = revisions.get(&rel).cloned().unwrap_or(Expected::Missing);
            state.entries.insert(
                rel,
                Entry {
                    baseline: digest,
                    expected: (&expected).into(),
                    reason: None,
                },
            );
        }
        state.ready = false;
        persist(&self.run_dir, &state)
    }

    pub fn overlay(&self, path: &str, bytes: &[u8], expected: &Expected) -> Result<(), String> {
        if !valid_relative(path) {
            return Err("caminho de aba inválido".into());
        }
        let mut state = self.state.lock().unwrap();
        // Mark preparation before writing an overlay. A crash here must never
        // turn a half-written input into output eligible for repository upload.
        state.ready = false;
        persist(&self.run_dir, &state)?;
        let target = self.dir().join(path);
        fs::create_dir_all(target.parent().ok_or("aba sem pasta")?).map_err(|e| e.to_string())?;
        fs::write(&target, bytes).map_err(|e| e.to_string())?;
        state.entries.insert(
            path.into(),
            Entry {
                baseline: revision(bytes),
                expected: expected.into(),
                reason: None,
            },
        );
        persist(&self.run_dir, &state)
    }

    /// Seal all prepared inputs before the interpreter may start. Interrupted
    /// preparation is disposable; an armed run must be reconciled after a crash.
    pub fn arm(&self) -> Result<(), String> {
        let mut state = self.state.lock().unwrap();
        if state.ready {
            return Ok(());
        }
        let mut paths = Vec::new();
        walk(&self.dir(), &self.dir(), &mut paths)?;
        for (_, path) in paths {
            flush_output(&path, &self.dir())?;
        }
        sync_directory(&self.dir())?;
        state.ready = true;
        persist(&self.run_dir, &state)
    }

    pub fn outputs(&self) -> Result<Vec<Output>, String> {
        let (outputs, errors) = self.collect_outputs();
        if errors.is_empty() {
            Ok(outputs)
        } else {
            Err(errors.join("\n"))
        }
    }

    fn collect_outputs(&self) -> (Vec<Output>, Vec<String>) {
        let state = self.state.lock().unwrap();
        if !state.ready {
            return (Vec::new(), Vec::new());
        }
        let mut paths = Vec::new();
        let mut errors = Vec::new();
        if let Err(e) = walk(&self.dir(), &self.dir(), &mut paths) {
            errors.push(e);
        }
        let mut outputs = Vec::new();
        for (rel, path) in paths {
            let (digest, size) = match fingerprint(&path) {
                Ok(value) => value,
                Err(e) => {
                    errors.push(format!("{rel}: {e}"));
                    continue;
                }
            };
            let entry = state.entries.get(&rel);
            if entry.is_some_and(|e| e.baseline == digest) {
                continue;
            }
            outputs.push(Output {
                path: rel,
                size,
                digest,
                expected: entry
                    .map(|e| (&e.expected).into())
                    .unwrap_or(Expected::Missing),
                reason: entry
                    .and_then(|e| e.reason.clone())
                    .unwrap_or_else(|| "execução interrompida antes de concluir o envio".into()),
            });
        }
        (outputs, errors)
    }

    /// The closure allows deterministic network/disk fault injection in tests.
    pub fn send_with(
        &self,
        identity: &ProjectIdentity,
        mut send: impl FnMut(&str, &[u8], &Expected) -> Result<Saved, SaveError>,
    ) -> Delivery {
        let mut delivery = Delivery::default();
        if &self.identity() != identity {
            delivery
                .errors
                .push("o projeto de destino não corresponde à origem da execução".into());
            return delivery;
        }
        let (outputs, errors) = self.collect_outputs();
        delivery.errors.extend(errors);
        for output in outputs {
            let bytes = match fs::read(self.dir().join(&output.path)) {
                Ok(bytes) if revision(&bytes) == output.digest => bytes,
                Ok(_) => {
                    delivery.errors.push(format!(
                        "{}: a saída mudou durante o envio; tente novamente",
                        output.path
                    ));
                    continue;
                }
                Err(e) => {
                    delivery.errors.push(format!("{}: {e}", output.path));
                    continue;
                }
            };
            // Flush the generated bytes before acknowledging their journal entry.
            let result = match flush_output(&self.dir().join(&output.path), &self.dir()) {
                Ok(()) => send(&output.path, &bytes, &output.expected),
                Err(e) => Err(SaveError::Other(e)),
            };
            let mut state = self.state.lock().unwrap();
            match result {
                Ok(saved) => {
                    state.entries.insert(
                        output.path.clone(),
                        Entry {
                            baseline: revision(&bytes),
                            expected: Expectation::Revision(saved.revision),
                            reason: None,
                        },
                    );
                    delivery
                        .sent
                        .push((output.path.clone(), String::from_utf8(bytes).ok()));
                    if let Some(warning) = saved.warning {
                        delivery
                            .warnings
                            .push(format!("{}: {warning}", output.path));
                    }
                }
                Err(e) => {
                    let reason = e.to_string();
                    state
                        .entries
                        .entry(output.path.clone())
                        .or_insert_with(|| Entry {
                            baseline: String::new(),
                            expected: (&output.expected).into(),
                            reason: None,
                        })
                        .reason = Some(reason.clone());
                    if matches!(
                        e,
                        SaveError::Conflict | SaveError::AlreadyExists | SaveError::Unsupported
                    ) {
                        delivery.conflicts.push((output.path.clone(), bytes));
                    } else {
                        delivery.errors.push(format!("{}: {reason}", output.path));
                    }
                }
            }
            if let Err(e) = persist(&self.run_dir, &state) {
                delivery.errors.push(format!(
                    "registro preservado em {}: {e}",
                    self.run_dir.display()
                ));
                // Do not remove the run on a journal durability failure.
                return delivery;
            }
        }
        if delivery.errors.is_empty()
            && delivery.conflicts.is_empty()
            && let Err(e) = fs::remove_dir_all(&self.run_dir)
        {
            delivery
                .warnings
                .push(format!("limpeza de {}: {e}", self.run_dir.display()));
        }
        delivery
    }

    pub fn retry(&self, files: &Files, student: &str) -> Delivery {
        let identity = match files.identity(student) {
            Ok(i) => i,
            Err(e) => {
                return Delivery {
                    errors: vec![e],
                    ..Default::default()
                };
            }
        };
        self.send_with(&identity, |rel, bytes, expected| {
            // A previous response/acknowledgement may have been lost. Matching
            // destination bytes account for output without performing a write.
            let current = files
                .current_bytes(student, rel)
                .ok()
                .flatten()
                .map(|s| s.bytes);
            if current.as_deref() == Some(bytes) {
                return Ok(Saved {
                    revision: revision(bytes),
                    warning: None,
                });
            }
            if let Ok(text) = std::str::from_utf8(bytes) {
                files.write_file(student, rel, text, expected)
            } else {
                files.write_bytes(student, rel, bytes, expected)
            }
        })
    }

    /// Export is exclusive and never retires the retained run.
    pub fn export(&self, folder: &Path) -> Result<PathBuf, String> {
        export_destination(&self.run_dir, folder)?;
        let outputs = self.outputs()?;
        let dest = tempfile::Builder::new()
            .prefix("saida-execucao-")
            .tempdir_in(folder)
            .map_err(|e| e.to_string())?;
        for output in outputs {
            let path = dest.path().join(&output.path);
            fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
            copy_export(&self.dir().join(&output.path), &path)?;
            if fingerprint(&path)?.0 != output.digest {
                return Err("a saída mudou durante a exportação; tente novamente".into());
            }
        }
        let manifest =
            serde_json::to_vec_pretty(&*self.state.lock().unwrap()).map_err(|e| e.to_string())?;
        // Metadata is separate from the exported project paths.
        let metadata = dest.path().with_extension("json");
        save_export(&metadata, &manifest)?;
        Ok(dest.keep())
    }

    /// Called only after explicit discard confirmation, or for an empty run.
    pub fn discard(self) -> Result<(), String> {
        fs::remove_dir_all(&self.run_dir).map_err(|e| e.to_string())
    }
}

pub fn scan(root: &Path) -> Scan {
    scan_filtered(root, None)
}

pub fn scan_project(root: &Path, identity: &ProjectIdentity) -> Scan {
    scan_filtered(root, Some(&identity.key()))
}

fn scan_filtered(root: &Path, project_key: Option<&str>) -> Scan {
    let mut scan = Scan::default();
    let projects = match fs::read_dir(root) {
        Ok(p) => p,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return scan,
        Err(e) => {
            scan.errors.push(e.to_string());
            return scan;
        }
    };
    for project in projects {
        let result = (|| -> Result<(), String> {
            let project = project.map_err(|e| e.to_string())?.path();
            if project_key
                .is_some_and(|key| project.file_name().and_then(|n| n.to_str()) != Some(key))
            {
                return Ok(());
            }
            if !fs::symlink_metadata(&project)
                .map_err(|e| e.to_string())?
                .is_dir()
            {
                return Ok(());
            }
            for run in fs::read_dir(&project).map_err(|e| e.to_string())? {
                let path = run.map_err(|e| e.to_string())?.path();
                if !path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.starts_with("run-"))
                {
                    continue;
                }
                let result = (|| -> Result<(), String> {
                    if !fs::symlink_metadata(&path)
                        .map_err(|e| e.to_string())?
                        .is_dir()
                    {
                        return Ok(());
                    }
                    let Some(journal) = Journal::open(&path)? else {
                        return Ok(());
                    };
                    let outputs = journal.outputs()?;
                    if outputs.is_empty() {
                        journal.discard()?;
                    } else {
                        scan.runs.push(PendingRun {
                            path: path.clone(),
                            identity: journal.identity(),
                            files: outputs
                                .into_iter()
                                .map(|o| (o.path, o.size, o.reason))
                                .collect(),
                        });
                    }
                    Ok(())
                })();
                if let Err(e) = result {
                    scan.unreadable.push(path.clone());
                    scan.errors.push(format!("{}: {e}", path.display()));
                }
            }
            Ok(())
        })();
        if let Err(e) = result {
            scan.errors.push(e);
        }
    }
    scan.runs.sort_by(|a, b| a.path.cmp(&b.path));
    scan
}

fn check_budget(root: &Path) -> Result<(), String> {
    let mut runs = 0;
    let mut bytes = 0;
    for project in fs::read_dir(root).map_err(|e| e.to_string())? {
        let project = project.map_err(|e| e.to_string())?.path();
        if !fs::symlink_metadata(&project)
            .map_err(|e| e.to_string())?
            .is_dir()
        {
            continue;
        }
        for run in fs::read_dir(project).map_err(|e| e.to_string())? {
            let run = run.map_err(|e| e.to_string())?.path();
            if !run
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("run-"))
                || !fs::symlink_metadata(&run)
                    .map_err(|e| e.to_string())?
                    .is_dir()
            {
                continue;
            }
            runs += 1;
            // Live programs can add/remove files concurrently; never traverse
            // their mutable tree for retention or cleanup.
            let Ok(Some(_lease)) = acquire(&run) else {
                continue;
            };
            let mut paths = Vec::new();
            walk(&run.join("files"), &run.join("files"), &mut paths)?;
            for (_, path) in paths {
                bytes += fs::metadata(path).map_err(|e| e.to_string())?.len();
            }
        }
    }
    if runs >= MAX_RETAINED_RUNS || bytes >= MAX_RETAINED_BYTES {
        return Err("há muitas saídas de execução preservadas; revise os arquivos preservados; em projetos no servidor, use Arquivo → Sincronização…".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Barrier};
    use std::thread;

    struct Project {
        _temp: tempfile::TempDir,
        root: PathBuf,
        runs: PathBuf,
        files: Files,
    }
    impl Project {
        fn new() -> Self {
            let temp = tempfile::tempdir().unwrap();
            let root = temp.path().join("project");
            let runs = temp.path().join("runs");
            fs::create_dir(&root).unwrap();
            fs::write(root.join("a.txt"), "original").unwrap();
            Self {
                files: Files::Local(root.clone()),
                _temp: temp,
                root,
                runs,
            }
        }
        fn journal(&self) -> Journal {
            let j = Journal::create(&self.runs, self.files.identity("").unwrap()).unwrap();
            fs::write(j.dir().join("a.txt"), "original").unwrap();
            j.baseline(&HashMap::from([(
                "a.txt".into(),
                Expected::Revision(revision(b"original")),
            )]))
            .unwrap();
            j.arm().unwrap();
            j
        }
    }

    #[test]
    fn failed_output_survives_next_run_restart_retry_and_export() {
        let project = Project::new();
        let j = project.journal();
        fs::write(j.dir().join("a.txt"), "generated").unwrap();
        fs::write(j.dir().join("new.bin"), [0xff, 0, 42]).unwrap();
        let path = j.run_dir.clone();
        let sent = j.send_with(&j.identity(), |_, _, _| {
            Err(SaveError::Other("network unavailable".into()))
        });
        assert_eq!(sent.errors.len(), 2);
        assert!(
            scan(&project.runs).runs.is_empty(),
            "live execution was offered for cleanup"
        );
        drop(j);
        let next = project.journal();
        assert_ne!(next.run_dir, path);
        let recovered = scan(&project.runs);
        assert!(recovered.errors.is_empty(), "{:?}", recovered.errors);
        assert_eq!(recovered.runs.len(), 1);
        assert!(
            recovered.runs[0]
                .files
                .iter()
                .all(|f| f.2 == "network unavailable")
        );
        let j = Journal::open(&path).unwrap().unwrap();
        let export_root = project._temp.path().join("exports");
        fs::create_dir(&export_root).unwrap();
        let exported = j.export(&export_root).unwrap();
        assert_eq!(fs::read(exported.join("new.bin")).unwrap(), [0xff, 0, 42]);
        assert_eq!(
            fs::read_to_string(exported.join("a.txt")).unwrap(),
            "generated"
        );
        assert!(path.exists(), "export retired the only retry copy");
        let exported_again = j.export(&export_root).unwrap();
        assert_ne!(exported, exported_again);
        let sent = j.retry(&project.files, "");
        assert!(sent.errors.is_empty() && sent.conflicts.is_empty());
        assert_eq!(sent.sent.len(), 2);
        assert_eq!(
            fs::read_to_string(project.root.join("a.txt")).unwrap(),
            "generated"
        );
        assert_eq!(
            fs::read(project.root.join("new.bin")).unwrap(),
            [0xff, 0, 42]
        );
        assert!(!path.exists(), "fully accounted run was retained");
    }

    #[test]
    fn crash_before_upload_and_lost_acknowledgement_are_recoverable() {
        let project = Project::new();
        let j = project.journal();
        fs::write(j.dir().join("a.txt"), "generated").unwrap();
        let path = j.run_dir.clone();
        // Simulate a server committing and the client crashing before its ack.
        fs::write(project.root.join("a.txt"), "generated").unwrap();
        drop(j);
        assert_eq!(scan(&project.runs).runs.len(), 1);
        let j = Journal::open(&path).unwrap().unwrap();
        let sent = j.retry(&project.files, "");
        assert_eq!(sent.sent.len(), 1);
        assert!(sent.conflicts.is_empty());
        assert!(!path.exists());
    }

    #[test]
    fn delayed_retry_never_rebases_over_a_changed_or_new_destination() {
        let project = Project::new();
        let j = project.journal();
        fs::write(j.dir().join("a.txt"), "run output").unwrap();
        fs::write(j.dir().join("new.txt"), "new output").unwrap();
        let path = j.run_dir.clone();
        drop(j);
        fs::write(project.root.join("a.txt"), "later edit").unwrap();
        fs::write(project.root.join("new.txt"), "later creation").unwrap();
        let j = Journal::open(&path).unwrap().unwrap();
        let sent = j.retry(&project.files, "");
        assert_eq!(sent.conflicts.len(), 2);
        assert_eq!(
            fs::read_to_string(project.root.join("a.txt")).unwrap(),
            "later edit"
        );
        assert_eq!(
            fs::read_to_string(project.root.join("new.txt")).unwrap(),
            "later creation"
        );
        assert_eq!(j.outputs().unwrap().len(), 2);
        drop(j);
        let j = Journal::open(&path).unwrap().unwrap();
        assert_eq!(
            j.retry(&project.files, "").conflicts.len(),
            2,
            "retry acquired a newer baseline"
        );
        j.discard().unwrap();
        assert!(!path.exists());
        assert_eq!(
            fs::read_to_string(project.root.join("a.txt")).unwrap(),
            "later edit"
        );
    }

    #[test]
    fn partial_success_is_not_repeated_and_wrong_identity_never_sends() {
        let project = Project::new();
        let j = project.journal();
        fs::write(j.dir().join("a.txt"), "run output").unwrap();
        fs::write(j.dir().join("new.txt"), "new output").unwrap();
        let wrong = ProjectIdentity::Remote {
            endpoint: "http://other:80".into(),
            student: "s".into(),
            project: "p".into(),
        };
        assert_eq!(
            j.send_with(&wrong, |_, _, _| panic!("sent to wrong origin"))
                .errors
                .len(),
            1
        );
        let sent = j.send_with(&j.identity(), |rel, bytes, expected| {
            if rel == "a.txt" {
                expressa_aula_storage::save(&project.root.join(rel), bytes, expected)
            } else {
                Err(SaveError::Other("injected failure".into()))
            }
        });
        assert_eq!(sent.sent.len(), 1);
        let path = j.run_dir.clone();
        drop(j);
        let j = Journal::open(&path).unwrap().unwrap();
        let pending = j.outputs().unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].path, "new.txt");
        assert!(matches!(pending[0].expected, Expected::Missing));
    }

    #[test]
    fn simultaneous_preparations_and_cleanup_preserve_live_leases() {
        let project = Project::new();
        let identity = project.files.identity("").unwrap();
        let barrier = Arc::new(Barrier::new(3));
        let handles: Vec<_> = (0..2)
            .map(|_| {
                let root = project.runs.clone();
                let identity = identity.clone();
                let barrier = barrier.clone();
                thread::spawn(move || {
                    let j = Journal::create(&root, identity).unwrap();
                    barrier.wait();
                    barrier.wait();
                    j
                })
            })
            .collect();
        barrier.wait();
        let found = scan(&project.runs);
        assert!(found.runs.is_empty() && found.errors.is_empty());
        barrier.wait();
        let mut journals = handles.into_iter().map(|h| h.join().unwrap());
        let first = journals.next().unwrap();
        let second = journals.next().unwrap();
        assert_ne!(first.run_dir, second.run_dir);
        assert!(first.run_dir.exists() && second.run_dir.exists());
        let paths = [first.run_dir.clone(), second.run_dir.clone()];
        drop((first, second));
        // Other parallel tests may fork a child while these file descriptors
        // are open. Its inherited CLOEXEC descriptors retain the lease briefly
        // until exec. Discovery must conservatively skip them, then clean up.
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        loop {
            let found = scan(&project.runs);
            assert!(found.runs.is_empty() && found.errors.is_empty());
            if paths.iter().all(|p| !p.exists()) {
                break;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "released run leases remained locked"
            );
            thread::sleep(std::time::Duration::from_millis(10));
        }
    }

    #[test]
    fn malformed_journals_and_paths_are_preserved_not_uploaded() {
        let project = Project::new();
        let j = project.journal();
        let path = j.run_dir.clone();
        drop(j);
        fs::write(path.join(MANIFEST), b"interrupted or unknown record").unwrap();
        let found = scan(&project.runs);
        assert_eq!(found.errors.len(), 1);
        assert!(path.join("files/a.txt").exists());
        for path in ["", "../x", "a/../x", "a//x", "/x", "C:/x", "a\\b", "a/"] {
            assert!(!valid_relative(path), "{path}");
        }
    }

    #[cfg(unix)]
    #[test]
    fn output_symlinks_cannot_export_or_upload_external_data() {
        let project = Project::new();
        let j = project.journal();
        std::os::unix::fs::symlink(project.root.join("a.txt"), j.dir().join("link")).unwrap();
        assert!(j.outputs().is_err());
        fs::write(j.dir().join("new.txt"), "regular output").unwrap();
        let result = j.retry(&project.files, "");
        assert_eq!(result.errors.len(), 1);
        assert_eq!(result.sent.len(), 1);
        assert_eq!(
            fs::read(project.root.join("new.txt")).unwrap(),
            b"regular output"
        );
        assert_eq!(fs::read(project.root.join("a.txt")).unwrap(), b"original");
        assert!(j.run_dir.exists());
    }
    #[test]
    fn interpreter_lease_outlives_window_and_corrupt_records_can_be_exported() {
        let project = Project::new();
        let j = project.journal();
        fs::write(j.dir().join("a.txt"), "output").unwrap();
        let path = j.run_dir.clone();
        let worker_lease = j.lease();
        drop(j);
        assert!(scan(&project.runs).runs.is_empty());
        assert!(path.exists());
        drop(worker_lease);
        assert_eq!(scan(&project.runs).runs.len(), 1);
        assert!(
            Journal::open(&path)
                .unwrap()
                .unwrap()
                .export(&path.join("files"))
                .is_err()
        );
        fs::write(
            path.join("files/.expressa-save-user-data"),
            b"reserved raw output",
        )
        .unwrap();
        fs::write(path.join(MANIFEST), b"broken manifest").unwrap();
        let scan = scan(&project.runs);
        assert_eq!(scan.unreadable.as_slice(), std::slice::from_ref(&path));
        let exports = project._temp.path().join("exports");
        fs::create_dir(&exports).unwrap();
        let exported = export_raw(&path, &exports).unwrap();
        assert_eq!(
            fs::read(exported.join(MANIFEST)).unwrap(),
            b"broken manifest"
        );
        assert_eq!(fs::read(exported.join("files/a.txt")).unwrap(), b"output");
        assert_eq!(
            fs::read(exported.join("files/.expressa-save-user-data")).unwrap(),
            b"reserved raw output"
        );
        assert!(path.exists());
    }

    #[test]
    fn interrupted_preparation_never_uploads_a_partial_input() {
        let project = Project::new();
        let j = project.journal();
        let expected = Expected::Revision(revision(b"original"));
        j.overlay("a.txt", b"unsaved editor source", &expected)
            .unwrap();
        // Simulate an interruption while staging a later input, before arming.
        fs::write(j.dir().join("a.txt"), "partial input").unwrap();
        let path = j.run_dir.clone();
        drop(j);
        let found = scan(&project.runs);
        assert!(found.runs.is_empty() && found.errors.is_empty());
        assert!(!path.exists());
        assert_eq!(fs::read(project.root.join("a.txt")).unwrap(), b"original");
    }

    #[test]
    fn project_scan_does_not_show_or_clean_another_projects_records() {
        let project = Project::new();
        let local = project.journal();
        fs::write(local.dir().join("a.txt"), "local output").unwrap();
        let local_path = local.run_dir.clone();
        drop(local);
        let remote_id = ProjectIdentity::Remote {
            endpoint: "http://server:3000".into(),
            student: "ana".into(),
            project: "lesson".into(),
        };
        let remote = Journal::create(&project.runs, remote_id.clone()).unwrap();
        remote.baseline(&HashMap::new()).unwrap();
        remote.arm().unwrap();
        fs::write(remote.dir().join("server.txt"), "server output").unwrap();
        let remote_path = remote.run_dir.clone();
        drop(remote);
        let another = ProjectIdentity::Remote {
            endpoint: "http://other:3000".into(),
            student: "ana".into(),
            project: "lesson".into(),
        };
        assert!(scan_project(&project.runs, &another).runs.is_empty());
        let selected = scan_project(&project.runs, &remote_id);
        assert!(selected.errors.is_empty());
        assert_eq!(selected.runs.len(), 1);
        assert_eq!(selected.runs[0].path, remote_path);
        assert!(local_path.exists());
        let selected = scan_project(&project.runs, &project.files.identity("").unwrap());
        assert_eq!(selected.runs.len(), 1);
        assert_eq!(selected.runs[0].path, local_path);
        assert!(remote_path.exists());
    }

    #[test]
    fn journal_failure_after_publication_keeps_retry_copy() {
        let project = Project::new();
        let j = project.journal();
        fs::write(j.dir().join("a.txt"), "output").unwrap();
        let original_manifest = fs::read(j.run_dir.join(MANIFEST)).unwrap();
        fs::remove_file(j.run_dir.join(MANIFEST)).unwrap();
        fs::create_dir(j.run_dir.join(MANIFEST)).unwrap();
        let delivery = j.retry(&project.files, "");
        assert_eq!(delivery.sent.len(), 1);
        assert!(!delivery.errors.is_empty());
        let path = j.run_dir.clone();
        assert!(path.join("files/a.txt").exists());
        drop(j);
        fs::remove_dir(path.join(MANIFEST)).unwrap();
        fs::write(path.join(MANIFEST), original_manifest).unwrap();
        let recovered = Journal::open(&path).unwrap().unwrap();
        assert_eq!(recovered.retry(&project.files, "").sent.len(), 1);
        assert_eq!(fs::read(project.root.join("a.txt")).unwrap(), b"output");
    }

    #[test]
    fn retention_limits_refuse_new_work_without_evicting_pending_files() {
        let temp = tempfile::tempdir().unwrap();
        let parent = temp.path().join("identity");
        fs::create_dir(&parent).unwrap();
        for n in 0..MAX_RETAINED_RUNS {
            let run = parent.join(format!("run-{n}"));
            fs::create_dir_all(run.join("files")).unwrap();
            fs::write(run.join(MANIFEST), b"retained").unwrap();
            fs::write(run.join(LEASE), []).unwrap();
        }
        assert!(check_budget(temp.path()).is_err());
        assert_eq!(fs::read_dir(&parent).unwrap().count(), MAX_RETAINED_RUNS);
        // Sparse length lets the byte admission limit be tested without writing
        // or reading hundreds of megabytes into a real cache.
        let temp = tempfile::tempdir().unwrap();
        let run = temp.path().join("identity/run-limit");
        fs::create_dir_all(run.join("files")).unwrap();
        fs::write(run.join(MANIFEST), b"retained").unwrap();
        fs::write(run.join(LEASE), []).unwrap();
        let output = File::create(run.join("files/large.bin")).unwrap();
        output.set_len(MAX_RETAINED_BYTES).unwrap();
        assert!(check_budget(temp.path()).is_err());
        assert_eq!(output.metadata().unwrap().len(), MAX_RETAINED_BYTES);
    }

    #[test]
    fn lease_is_respected_by_another_process() {
        let project = Project::new();
        let j = project.journal();
        let path = j.run_dir.clone();
        let check = |busy: bool| {
            let status = std::process::Command::new(std::env::current_exe().unwrap())
                .args(["--exact", "pendencias::tests::lease_child", "--ignored"])
                .env("EXPRESSA_LEASE_CHILD_PATH", &path)
                .env("EXPRESSA_LEASE_CHILD_BUSY", if busy { "yes" } else { "no" })
                .status()
                .unwrap();
            assert!(status.success());
        };
        check(true);
        drop(j);
        check(false);
    }

    #[test]
    #[ignore = "subprocess helper, invoked by lease_is_respected_by_another_process"]
    fn lease_child() {
        let path = std::env::var_os("EXPRESSA_LEASE_CHILD_PATH").expect("child path");
        let busy = std::env::var("EXPRESSA_LEASE_CHILD_BUSY").unwrap() == "yes";
        let acquired = Journal::open(Path::new(&path)).unwrap();
        assert_eq!(acquired.is_none(), busy);
    }
}
