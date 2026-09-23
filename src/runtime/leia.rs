use std::io::{self, BufRead, Write};

/// Line written to stderr (CLI `--marcador-leia`) or sent to the IDE
/// before `leia()` waits for a reply. The rest of the line is the prompt.
pub const LEIA_MARKER: &str = "<<<EXPRESSA-LEIA>>>";

pub trait LeiaHost {
    fn ask(&mut self, prompt: &str) -> Result<String, String>;

    /// Drain remaining stdin as lines (no prompt). Default: not available
    /// (the Aula dialog has no EOF).
    fn read_all_lines(&mut self) -> Result<Vec<String>, String> {
        Err(
            "leia_linhas() lê a entrada padrão (pipe/arquivo); na Aula use leia() ou leia_arquivo"
                .into(),
        )
    }
}

/// Prints [`LEIA_MARKER`] plus the prompt on stderr, then reads stdin.
pub struct MarkerLeiaHost<R: BufRead> {
    pub stdin: R,
}

impl<R: BufRead> LeiaHost for MarkerLeiaHost<R> {
    fn ask(&mut self, prompt: &str) -> Result<String, String> {
        let mut err = io::stderr();
        writeln!(err, "{LEIA_MARKER}{prompt}").map_err(|e| e.to_string())?;
        err.flush().map_err(|e| e.to_string())?;
        let mut line = String::new();
        let n = self.stdin.read_line(&mut line).map_err(|e| e.to_string())?;
        if n == 0 {
            return Err("fim da entrada".into());
        }
        strip_newline(&mut line);
        Ok(line)
    }

    fn read_all_lines(&mut self) -> Result<Vec<String>, String> {
        let mut lines = Vec::new();
        loop {
            let mut line = String::new();
            let n = self.stdin.read_line(&mut line).map_err(|e| e.to_string())?;
            if n == 0 {
                break;
            }
            strip_newline(&mut line);
            lines.push(line);
        }
        Ok(lines)
    }
}

pub(crate) fn strip_newline(line: &mut String) {
    if line.ends_with('\n') {
        line.pop();
        if line.ends_with('\r') {
            line.pop();
        }
    }
}
