//! Who shows a [`FolhaPronta`]. Tests stay silent; the CLI spawns a viewer.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

use super::folha::FolhaPronta;

pub trait CartesianoHost {
    fn mostre(&mut self, folha: FolhaPronta) -> Result<(), String>;
}

/// `mostre` succeeds and records nothing. Used by `run_to_string` / tests.
pub struct NullCartesiano;

impl CartesianoHost for NullCartesiano {
    fn mostre(&mut self, _folha: FolhaPronta) -> Result<(), String> {
        Ok(())
    }
}

/// Records each `mostre` for tests.
#[derive(Default)]
pub struct CapturaCartesiano {
    pub folhas: Vec<FolhaPronta>,
}

impl CartesianoHost for CapturaCartesiano {
    fn mostre(&mut self, folha: FolhaPronta) -> Result<(), String> {
        self.folhas.push(folha);
        Ok(())
    }
}

/// One child process per documento. Writing the `.cart` file updates it.
#[derive(Default)]
pub struct CliCartesianoHost {
    children: HashMap<u64, Child>,
}

impl CliCartesianoHost {
    pub fn new() -> Self {
        Self::default()
    }

    fn path_for(id: u64) -> PathBuf {
        std::env::temp_dir().join(format!("expressa-cart-{}-{id}.cart", std::process::id()))
    }

    fn spawn_viewer(path: &Path) -> Result<Child, String> {
        let mut cmd = viewer_command(path)?;
        cmd.stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            cmd.process_group(0);
        }
        cmd.spawn()
            .map_err(|e| format!("não abriu a janela do gráfico: {e}"))
    }
}

impl CartesianoHost for CliCartesianoHost {
    fn mostre(&mut self, folha: FolhaPronta) -> Result<(), String> {
        let path = Self::path_for(folha.id);
        fs::write(&path, folha.to_cart()).map_err(|e| format!("não gravou o gráfico: {e}"))?;
        let dead = match self.children.get_mut(&folha.id) {
            Some(child) => child.try_wait().ok().flatten().is_some(),
            None => true,
        };
        if dead {
            let child = Self::spawn_viewer(&path)?;
            self.children.insert(folha.id, child);
        }
        Ok(())
    }
}

fn viewer_command(path: &Path) -> Result<Command, String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    if let Some(dir) = exe.parent() {
        let name = if cfg!(windows) {
            "expressa-grafico.exe"
        } else {
            "expressa-grafico"
        };
        let p = dir.join(name);
        if p.is_file() {
            let mut cmd = Command::new(p);
            cmd.arg(path);
            return Ok(cmd);
        }
    }
    let mut cmd = Command::new(exe);
    cmd.arg("--grafico").arg(path);
    Ok(cmd)
}
