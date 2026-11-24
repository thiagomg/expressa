//! Launch the `expressa-tutorial` binary (**Ajuda → Tutoriais**).

use std::path::PathBuf;
use std::process::{Command, Stdio};

pub fn spawn_tutorial() -> Result<(), String> {
    let mut cmd = tutorial_command()?;
    cmd.stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0);
    }
    cmd.spawn()
        .map(|_| ())
        .map_err(|e| format!("não abriu os tutoriais: {e}"))
}

fn tutorial_command() -> Result<Command, String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    if let Some(dir) = exe.parent() {
        let name = if cfg!(windows) {
            "expressa-tutorial.exe"
        } else {
            "expressa-tutorial"
        };
        let next = dir.join(name);
        if next.is_file() {
            return Ok(Command::new(next));
        }
        let bin = dir.join("bin").join(name);
        if bin.is_file() {
            return Ok(Command::new(bin));
        }
    }
    let path = PathBuf::from("expressa-tutorial");
    Ok(Command::new(path))
}
