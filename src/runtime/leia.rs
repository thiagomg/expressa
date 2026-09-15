use std::io::{self, BufRead, Write};

/// Line written to stderr (CLI `--marcador-leia`) or sent to the IDE
/// before `leia()` waits for a reply. The rest of the line is the prompt.
pub const LEIA_MARKER: &str = "<<<EXPRESSA-LEIA>>>";

pub trait LeiaHost {
    fn ask(&mut self, prompt: &str) -> Result<String, String>;
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
        if line.ends_with('\n') {
            line.pop();
            if line.ends_with('\r') {
                line.pop();
            }
        }
        Ok(line)
    }
}
