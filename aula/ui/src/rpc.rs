use expressa_aula_proto::turma_client::TurmaClient;
use expressa_aula_proto::{
    DeleteFileRequest, ListFilesRequest, MkdirRequest, ReadFileRequest, RenameRequest, TreeEntry,
    WriteFileRequest,
};
use std::process::Child;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU32, Ordering};

use tonic::transport::{Channel, Endpoint};

#[cfg(windows)]
const PROCESS_TERMINATE: u32 = 0x0001;
#[cfg(windows)]
const SYNCHRONIZE: u32 = 0x0010_0000;
#[cfg(all(windows, test))]
const WAIT_TIMEOUT: u32 = 0x0000_0102;

#[cfg(windows)]
unsafe extern "system" {
    fn OpenProcess(access: u32, inherit: i32, pid: u32) -> *mut std::ffi::c_void;
    fn TerminateProcess(process: *mut std::ffi::c_void, exit_code: u32) -> i32;
    fn CloseHandle(handle: *mut std::ffi::c_void) -> i32;
    fn WaitForSingleObject(handle: *mut std::ffi::c_void, millis: u32) -> u32;
    fn SetConsoleCtrlHandler(
        handler: Option<unsafe extern "system" fn(u32) -> i32>,
        add: i32,
    ) -> i32;
    fn ExitProcess(code: u32) -> !;
}

static SPAWNED_PID: Mutex<Option<u32>> = Mutex::new(None);
static SPAWNED_PID_ATOMIC: AtomicU32 = AtomicU32::new(0);

fn pidfile() -> std::path::PathBuf {
    std::env::temp_dir().join("expressa-aula-server.pid")
}

#[cfg(any(unix, test))]
fn pid_is_alive(pid: u32) -> bool {
    #[cfg(unix)]
    {
        std::process::Command::new("kill")
            .args(["-0", &pid.to_string()])
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }
    #[cfg(windows)]
    {
        // `kill -0` does not exist here. Wait with timeout 0: still running
        // means the wait timed out.
        unsafe {
            let handle = OpenProcess(SYNCHRONIZE, 0, pid);
            if handle.is_null() {
                return false;
            }
            let alive = WaitForSingleObject(handle, 0) == WAIT_TIMEOUT;
            CloseHandle(handle);
            alive
        }
    }
}

fn kill_pid(pid: u32) {
    #[cfg(unix)]
    {
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
    #[cfg(windows)]
    {
        // No SIGTERM. TerminateProcess is the same end state as `kill -KILL`.
        unsafe {
            let handle = OpenProcess(PROCESS_TERMINATE | SYNCHRONIZE, 0, pid);
            if handle.is_null() {
                return;
            }
            TerminateProcess(handle, 1);
            WaitForSingleObject(handle, 2000);
            CloseHandle(handle);
        }
    }
}

/// Ctrl+C / SIGTERM (and, on Windows, closing the console): kill the child
/// then exit. GTK does not run Drop on those paths.
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
    #[cfg(windows)]
    unsafe {
        SetConsoleCtrlHandler(Some(on_console_ctrl), 1);
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

#[cfg(windows)]
unsafe extern "system" fn on_console_ctrl(ctrl: u32) -> i32 {
    const CTRL_C_EVENT: u32 = 0;
    const CTRL_BREAK_EVENT: u32 = 1;
    const CTRL_CLOSE_EVENT: u32 = 2;
    const CTRL_LOGOFF_EVENT: u32 = 5;
    const CTRL_SHUTDOWN_EVENT: u32 = 6;
    match ctrl {
        CTRL_C_EVENT | CTRL_BREAK_EVENT | CTRL_CLOSE_EVENT | CTRL_LOGOFF_EVENT
        | CTRL_SHUTDOWN_EVENT => {
            let pid = SPAWNED_PID_ATOMIC.swap(0, Ordering::SeqCst);
            if pid != 0 {
                kill_pid(pid);
            }
            // Closing the console ends the process when the handler returns.
            // Ctrl+C would otherwise leave the IDE running.
            if ctrl == CTRL_CLOSE_EVENT {
                1
            } else {
                unsafe { ExitProcess(128 + ctrl) }
            }
        }
        _ => 0,
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
        // The window is built only after this returns. Without a limit, a
        // closed port on Windows can sit in the TCP timeout for several
        // seconds and the click looks like it did nothing.
        Self::connect_within(url, std::time::Duration::from_secs(3))
    }

    fn connect_within(url: &str, limit: std::time::Duration) -> Result<Self, String> {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .map_err(|e| e.to_string())?;
        let channel = rt
            .block_on(async {
                Endpoint::from_shared(url.to_string())
                    .map_err(|e| e.to_string())?
                    .connect_timeout(limit)
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

    /// Kill the server only if this UI process launched it. Connecting to a
    /// remote server must not stop whatever is recorded in the pid file.
    pub fn shutdown_spawned_server(&mut self) {
        let Some(mut child) = self.spawned_server.take() else {
            return;
        };
        let pid = child.id();
        let _ = child.kill();
        let _ = child.wait();
        kill_pid(pid);
        if SPAWNED_PID_ATOMIC.load(Ordering::SeqCst) == pid {
            SPAWNED_PID_ATOMIC.store(0, Ordering::SeqCst);
        }
        if let Ok(mut guard) = SPAWNED_PID.lock() {
            if *guard == Some(pid) {
                *guard = None;
            }
        }
        if std::fs::read_to_string(pidfile())
            .ok()
            .and_then(|s| s.trim().parse::<u32>().ok())
            == Some(pid)
        {
            let _ = std::fs::remove_file(pidfile());
        }
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

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    #[test]
    fn kill_pid_stops_a_live_process() {
        let mut child = std::process::Command::new("ping")
            .args(["-n", "30", "127.0.0.1"])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .expect("ping");
        let pid = child.id();
        assert!(pid_is_alive(pid), "processo recém-criado");
        kill_pid(pid);
        let _ = child.wait();
        assert!(!pid_is_alive(pid), "processo encerrado");
    }
}
