use expressa_aula_proto::turma_client::TurmaClient;
use expressa_aula_proto::{
    DeleteFileRequest, ListFilesRequest, MkdirRequest, ReadFileRequest, RenameRequest, TreeEntry,
    WriteFileRequest,
};
use std::process::Child;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU32, Ordering};

use tonic::transport::{Channel, Endpoint};

static SPAWNED_PID: Mutex<Option<u32>> = Mutex::new(None);
static SPAWNED_PID_ATOMIC: AtomicU32 = AtomicU32::new(0);

fn pidfile() -> std::path::PathBuf {
    std::env::temp_dir().join("expressa-aula-server.pid")
}

fn remember_pid(pid: u32) {
    SPAWNED_PID_ATOMIC.store(pid, Ordering::SeqCst);
    if let Ok(mut g) = SPAWNED_PID.lock() {
        *g = Some(pid);
    }
    let _ = std::fs::write(pidfile(), pid.to_string());
}

fn pid_is_alive(pid: u32) -> bool {
    std::process::Command::new("kill")
        .args(["-0", &pid.to_string()])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn kill_pid(pid: u32) {
    let _ = std::process::Command::new("kill")
        .args(["-TERM", &pid.to_string()])
        .status();
    std::thread::sleep(std::time::Duration::from_millis(80));
    if pid_is_alive(pid) {
        let _ = std::process::Command::new("kill")
            .args(["-KILL", &pid.to_string()])
            .status();
    }
}

/// Stop a server this UI started (or a leftover from a previous UI spawn).
pub fn kill_spawned_server() {
    SPAWNED_PID_ATOMIC.store(0, Ordering::SeqCst);
    let mem = SPAWNED_PID.lock().ok().and_then(|mut g| g.take());
    let file = std::fs::read_to_string(pidfile())
        .ok()
        .and_then(|s| s.trim().parse::<u32>().ok());
    let _ = std::fs::remove_file(pidfile());
    for pid in [mem, file].into_iter().flatten() {
        kill_pid(pid);
    }
}

/// Ctrl+C / SIGTERM: kill the child then exit (GTK does not run Drop).
pub fn install_signal_handlers() {
    #[cfg(unix)]
    unsafe {
        libc::signal(
            libc::SIGINT,
            on_fatal_signal as *const () as libc::sighandler_t,
        );
        libc::signal(
            libc::SIGTERM,
            on_fatal_signal as *const () as libc::sighandler_t,
        );
    }
}

#[cfg(unix)]
extern "C" fn on_fatal_signal(sig: i32) {
    let pid = SPAWNED_PID_ATOMIC.swap(0, Ordering::SeqCst);
    if pid != 0 {
        unsafe {
            libc::kill(pid as i32, libc::SIGKILL);
        }
    }
    unsafe {
        libc::_exit(128 + sig);
    }
}

pub struct Rpc {
    rt: tokio::runtime::Runtime,
    channel: Channel,
    /// Set only when this client started `expressa-aula-server`.
    spawned_server: Option<Child>,
}

impl Rpc {
    pub fn connect(url: &str) -> Result<Self, String> {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .map_err(|e| e.to_string())?;
        let channel = rt
            .block_on(async {
                Endpoint::from_shared(url.to_string())
                    .map_err(|e| e.to_string())?
                    .connect()
                    .await
                    .map_err(|e| e.to_string())
            })
            .map_err(|e| e)?;
        Ok(Self {
            rt,
            channel,
            spawned_server: None,
        })
    }

    /// Kill the server only if this UI process launched it.
    pub fn shutdown_spawned_server(&mut self) {
        if let Some(mut child) = self.spawned_server.take() {
            let pid = child.id();
            let _ = child.kill();
            let _ = child.wait();
            kill_pid(pid);
        }
        kill_spawned_server();
    }
}

impl Drop for Rpc {
    fn drop(&mut self) {
        self.shutdown_spawned_server();
    }
}

impl Rpc {
    pub fn list_tree(&self, student: &str) -> Result<Vec<TreeEntry>, String> {
        let mut c = TurmaClient::new(self.channel.clone());
        let resp = self
            .rt
            .block_on(c.list_files(ListFilesRequest {
                student: student.to_string(),
            }))
            .map_err(|e| e.to_string())?;
        Ok(resp.into_inner().entries)
    }

    pub fn read_file(&self, student: &str, path: &str) -> Result<String, String> {
        let mut c = TurmaClient::new(self.channel.clone());
        let resp = self
            .rt
            .block_on(c.read_file(ReadFileRequest {
                student: student.to_string(),
                path: path.to_string(),
            }))
            .map_err(|e| e.to_string())?;
        Ok(resp.into_inner().source)
    }

    pub fn write_file(&self, student: &str, path: &str, source: &str) -> Result<(), String> {
        let mut c = TurmaClient::new(self.channel.clone());
        self.rt
            .block_on(c.write_file(WriteFileRequest {
                student: student.to_string(),
                path: path.to_string(),
                source: source.to_string(),
            }))
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn delete_file(&self, student: &str, path: &str) -> Result<(), String> {
        let mut c = TurmaClient::new(self.channel.clone());
        self.rt
            .block_on(c.delete_file(DeleteFileRequest {
                student: student.to_string(),
                path: path.to_string(),
            }))
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn mkdir(&self, student: &str, path: &str) -> Result<(), String> {
        let mut c = TurmaClient::new(self.channel.clone());
        self.rt
            .block_on(c.mkdir(MkdirRequest {
                student: student.to_string(),
                path: path.to_string(),
            }))
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn rename(&self, student: &str, from: &str, to: &str) -> Result<(), String> {
        let mut c = TurmaClient::new(self.channel.clone());
        self.rt
            .block_on(c.rename(RenameRequest {
                student: student.to_string(),
                from: from.to_string(),
                to: to.to_string(),
            }))
            .map_err(|e| e.to_string())?;
        Ok(())
    }
}

/// Start `expressa-aula-server` from next to this binary if nothing is listening.
pub fn ensure_server(url: &str) -> Result<Rpc, String> {
    if let Ok(rpc) = Rpc::connect(url) {
        if let Ok(s) = std::fs::read_to_string(pidfile()) {
            if let Ok(pid) = s.trim().parse::<u32>() {
                if pid_is_alive(pid) {
                    remember_pid(pid);
                }
            }
        }
        return Ok(rpc);
    }
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let dir = exe.parent().ok_or("sem pasta do executável")?;
    let server = dir.join("expressa-aula-server");
    if !server.exists() {
        return Err(format!(
            "não conectou em {url} e não achei {}\nRode: cargo run -p expressa-aula-server",
            server.display()
        ));
    }
    let child = std::process::Command::new(&server)
        .args(["--bind", "127.0.0.1:50051", "--root", "./aula-data"])
        .spawn()
        .map_err(|e| format!("não iniciou o servidor: {e}"))?;
    remember_pid(child.id());
    std::thread::sleep(std::time::Duration::from_millis(400));
    let mut rpc = Rpc::connect(url)
        .map_err(|e| format!("servidor iniciado, mas ainda não responde ({e})"))?;
    rpc.spawned_server = Some(child);
    Ok(rpc)
}
