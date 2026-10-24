//! Who shows a [`TelaGrafica`]. Tests stay silent; the CLI spawns a viewer.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

use super::graf::TelaGrafica;

pub trait GrafHost {
    fn mostre(&mut self, tela: TelaGrafica) -> Result<(), String>;
}

pub struct NullGraf;

impl GrafHost for NullGraf {
    fn mostre(&mut self, _tela: TelaGrafica) -> Result<(), String> {
        Ok(())
    }
}

#[derive(Default)]
pub struct CapturaGraf {
    pub telas: Vec<TelaGrafica>,
}

impl GrafHost for CapturaGraf {
    fn mostre(&mut self, tela: TelaGrafica) -> Result<(), String> {
        self.telas.push(tela);
        Ok(())
    }
}

/// One child process per `nova()`. Rewriting the PNG updates the window.
#[derive(Default)]
pub struct CliGrafHost {
    children: HashMap<u64, Child>,
}

impl CliGrafHost {
    fn path_for(id: u64) -> PathBuf {
        std::env::temp_dir().join(format!("expressa-graf-{}-{id}.png", std::process::id()))
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
            .map_err(|e| format!("não abriu a janela do graf: {e}"))
    }
}

impl GrafHost for CliGrafHost {
    fn mostre(&mut self, tela: TelaGrafica) -> Result<(), String> {
        let path = Self::path_for(tela.id);
        tela.raster()
            .write_png(&path)
            .map_err(|e| format!("não gravou o graf: {e}"))?;
        let dead = match self.children.get_mut(&tela.id) {
            Some(child) => child.try_wait().ok().flatten().is_some(),
            None => true,
        };
        if dead {
            let child = Self::spawn_viewer(&path)?;
            self.children.insert(tela.id, child);
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
