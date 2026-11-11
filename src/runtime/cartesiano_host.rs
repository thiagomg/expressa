//! Who shows a [`FolhaPronta`]. Tests stay silent; the CLI spawns a viewer.

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

#[cfg(feature = "graphics")]
mod desktop;
#[cfg(feature = "graphics")]
pub use desktop::CliCartesianoHost;
