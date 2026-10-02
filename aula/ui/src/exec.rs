//! Runs and debugs programs in this process. The server is only a file
//! repository: before a run the student's project is copied into a local
//! cache folder, and files the program created or changed are sent back.

use std::collections::HashMap;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender, SyncSender};

use expressa::runtime::{
    ChannelDebugger, DebugHook, DebugPaused, LeiaHost, StopHook, run_with_hook,
};

use crate::rpc::Rpc;

/// Events the UI may lag behind before the program blocks. Keeps a program
/// that prints forever from piling up memory or starving the UI.
const EVENT_BACKLOG: usize = 256;

pub enum ExecEvent {
    Stdout(String),
    Stderr(String),
    Leia(String),
    Paused(DebugPaused),
    Finished(Finished),
}

pub struct Finished {
    pub ok: bool,
    pub error_message: String,
    /// Relative to the project folder.
    pub error_file: String,
    pub error_line: u32,
}

/// Local copy of one student's project.
pub struct Workspace {
    pub dir: PathBuf,
    /// Contents as downloaded, to find what the run changed.
    snapshot: HashMap<String, String>,
}

fn cache_root() -> PathBuf {
    std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache")))
        .unwrap_or_else(std::env::temp_dir)
        .join("expressa-aula")
}

/// Same rule as the server: the name becomes a folder name.
fn valid_student(student: &str) -> bool {
    !student.is_empty()
        && student
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

impl Workspace {
    /// Replace the local copy with the server's files.
    pub fn download(rpc: &Rpc, student: &str) -> Result<Self, String> {
        if !valid_student(student) {
            return Err(format!("nome de projeto inválido: {student:?}"));
        }
        let dir = cache_root().join(student);
        if dir.exists() {
            std::fs::remove_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        }
        std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        let dir = dir.canonicalize().map_err(|e| e.to_string())?;
        let mut snapshot = HashMap::new();
        for entry in rpc.list_tree(student)? {
            let local = dir.join(&entry.path);
            if entry.is_dir {
                std::fs::create_dir_all(&local).map_err(|e| e.to_string())?;
                continue;
            }
            let source = rpc.read_file(student, &entry.path)?;
            write_file(&local, &source)?;
            snapshot.insert(entry.path, source);
        }
        Ok(Self { dir, snapshot })
    }

    /// Put the editor's text of `rel` in the copy (open tabs, saved or not).
    /// It counts as already on the server: only what the program itself
    /// writes is uploaded afterwards.
    pub fn write_editor_text(&mut self, rel: &str, source: &str) -> Result<(), String> {
        write_file(&self.dir.join(rel), source)?;
        self.snapshot.insert(rel.to_string(), source.to_string());
        Ok(())
    }

    /// Absolute path of `rel` in the copy, as the interpreter names files.
    pub fn abs(&self, rel: &str) -> String {
        self.dir.join(rel).to_string_lossy().into_owned()
    }

    /// Send files that are new or differ from the download. Returns how many.
    pub fn upload_changes(&self, rpc: &Rpc, student: &str) -> Result<usize, String> {
        let mut files = Vec::new();
        walk(&self.dir, &mut files).map_err(|e| e.to_string())?;
        let mut sent = 0;
        for path in files {
            let rel = self.relative(&path);
            // The repository holds text; skip anything that is not UTF-8.
            let Ok(source) = std::fs::read_to_string(&path) else {
                continue;
            };
            if self.snapshot.get(&rel) != Some(&source) {
                rpc.write_file(student, &rel, &source)?;
                sent += 1;
            }
        }
        Ok(sent)
    }

    /// `/cache/ana/lib/m.lep` -> `lib/m.lep`; other paths unchanged.
    pub fn relative(&self, path: &Path) -> String {
        path.strip_prefix(&self.dir)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/")
    }
}

fn write_file(path: &Path, source: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(path, source).map_err(|e| format!("{}: {e}", path.display()))
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) -> io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            walk(&path, out)?;
        } else {
            out.push(path);
        }
    }
    Ok(())
}

struct ChannelLeia {
    prompts: SyncSender<ExecEvent>,
    lines: Receiver<String>,
}

impl LeiaHost for ChannelLeia {
    fn ask(&mut self, prompt: &str) -> Result<String, String> {
        self.prompts
            .send(ExecEvent::Leia(prompt.to_string()))
            .map_err(|_| "IDE fechada".to_string())?;
        self.lines.recv().map_err(|_| "IDE fechada".to_string())
    }
}

struct ChannelOut {
    tx: SyncSender<ExecEvent>,
    stderr: bool,
}

impl Write for ChannelOut {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        // Blocks while the UI is behind. Parar still works: the hook checks
        // for it before every statement, and the UI keeps draining.
        let text = String::from_utf8_lossy(buf).into_owned();
        let _ = self.tx.send(if self.stderr {
            ExecEvent::Stderr(text)
        } else {
            ExecEvent::Stdout(text)
        });
        Ok(buf.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Channels the UI keeps for one run.
pub struct RunHandle {
    /// Output, prompts, pauses and the end, in program order.
    pub events: Receiver<ExecEvent>,
    /// Answers to `leia`.
    pub lines: Sender<String>,
    /// continuar | proximo | entrar | sair | terminar
    pub debug_cmd: Sender<String>,
}

/// Run `rel_path` inside `ws` on a new thread. No time limit: Parar
/// (`terminar`) is how a program that never ends is stopped.
pub fn spawn(
    ws: &Workspace,
    rel_path: &str,
    source: String,
    debug: bool,
    breakpoints: Vec<(String, u32)>,
    args: Vec<String>,
) -> RunHandle {
    let (event_tx, events) = std::sync::mpsc::sync_channel(EVENT_BACKLOG);
    let (lines, line_rx) = std::sync::mpsc::channel::<String>();
    let (debug_cmd, ui_cmd_rx) = std::sync::mpsc::channel::<String>();
    let (dcmd_tx, dcmd_rx) = std::sync::mpsc::channel::<String>();
    let (pause_tx, pause_rx) = std::sync::mpsc::channel::<DebugPaused>();

    // Parar during `leia`: the program waits on a line, not on the hook.
    {
        let lines = lines.clone();
        std::thread::spawn(move || {
            while let Ok(cmd) = ui_cmd_rx.recv() {
                if matches!(cmd.trim(), "terminar" | "q" | "quit") {
                    let _ = lines.send(String::new());
                }
                if dcmd_tx.send(cmd).is_err() {
                    break;
                }
            }
        });
    }

    let file = ws.abs(rel_path);
    let breakpoints: Vec<(String, u32)> = breakpoints.into_iter().map(|(f, l)| (ws.abs(&f), l)).collect();
    let root = ws.dir.clone();
    let rel = move |p: &str| {
        Path::new(p)
            .strip_prefix(&root)
            .map(|r| r.to_string_lossy().into_owned())
            .unwrap_or_else(|_| p.to_string())
    };
    let ws_dir = ws.dir.clone();

    // Paused snapshots carry cache paths; show them relative to the project.
    {
        let event_tx = event_tx.clone();
        let rel = rel.clone();
        std::thread::spawn(move || {
            while let Ok(mut p) = pause_rx.recv() {
                p.file = rel(&p.file);
                for f in &mut p.stack {
                    f.file = rel(&f.file);
                }
                if event_tx.send(ExecEvent::Paused(p)).is_err() {
                    break;
                }
            }
        });
    }

    std::thread::spawn(move || {
        let mut host = ChannelLeia {
            prompts: event_tx.clone(),
            lines: line_rx,
        };
        let mut out = ChannelOut {
            tx: event_tx.clone(),
            stderr: false,
        };
        let mut err = ChannelOut {
            tx: event_tx.clone(),
            stderr: true,
        };
        let hook: Box<dyn DebugHook> = if debug {
            let mut dbg = ChannelDebugger::new(pause_tx, dcmd_rx);
            for bp in breakpoints {
                dbg.session.breakpoints.insert(bp);
            }
            Box::new(dbg)
        } else {
            drop(pause_tx);
            Box::new(StopHook::new(dcmd_rx))
        };
        let result = run_with_hook(
            &source,
            &file,
            hook,
            Some(ws_dir),
            None,
            &mut out,
            &mut err,
            &mut host,
            &args,
        );
        let finished = match result {
            Ok(0) => Finished {
                ok: true,
                error_message: String::new(),
                error_file: String::new(),
                error_line: 0,
            },
            Ok(code) => Finished {
                ok: false,
                error_message: format!("encerrou com código {code}"),
                error_file: String::new(),
                error_line: 0,
            },
            Err(e) => Finished {
                ok: false,
                error_file: rel(&e.file),
                error_message: e.message,
                error_line: e.span.line,
            },
        };
        let _ = event_tx.send(ExecEvent::Finished(finished));
    });

    RunHandle {
        events,
        lines,
        debug_cmd,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn student_names_are_folder_safe() {
        assert!(valid_student("ana"));
        assert!(valid_student("turma-1_b"));
        assert!(!valid_student(""));
        assert!(!valid_student("../x"));
        assert!(!valid_student("a/b"));
    }
}
