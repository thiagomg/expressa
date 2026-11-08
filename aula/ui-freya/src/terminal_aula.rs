//! Custom Freya terminal backend: Expressa VM stdout/stderr and inline `leia`.
//!
//! The VM runs in-process (not a PTY). Program bytes go to [`TerminalOutput`];
//! keyboard bytes arrive in [`BackendExpressa::write`] while `leia` is open.

use std::sync::{Arc, Mutex};
use std::sync::mpsc::Sender;

use freya::terminal::{TerminalBackend, TerminalError, TerminalOutput};

#[derive(Default)]
pub struct EstadoLeia {
    pub aberta: bool,
    pub buffer: String,
    pub responder: Option<Sender<String>>,
}

#[derive(Clone, Default)]
pub struct PonteSaida {
    output: Arc<Mutex<Option<TerminalOutput>>>,
    leia: Arc<Mutex<EstadoLeia>>,
}

impl PartialEq for PonteSaida {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.output, &other.output)
    }
}

impl Eq for PonteSaida {}

impl std::fmt::Debug for PonteSaida {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("PonteSaida")
    }
}

impl PonteSaida {
    pub fn escrever(&self, texto: &str, stderr: bool) {
        let Some(out) = self.output.lock().unwrap().clone() else {
            return;
        };
        let bytes = if stderr {
            let mut v = b"\x1b[31m".to_vec();
            v.extend_from_slice(para_vt(texto).as_bytes());
            v.extend_from_slice(b"\x1b[0m");
            v
        } else {
            para_vt(texto).into_bytes()
        };
        let _ = out.write(bytes);
    }

    pub fn limpar(&self) {
        self.cancelar_leia();
        if let Some(out) = self.output.lock().unwrap().clone() {
            let _ = out.write(b"\x1b[2J\x1b[H".to_vec());
        }
    }

    pub fn pedir_leia(&self, prompt: &str, responder: Sender<String>) {
        {
            let mut leia = self.leia.lock().unwrap();
            leia.aberta = true;
            leia.buffer.clear();
            leia.responder = Some(responder);
        }
        if prompt.is_empty() {
            return;
        }
        let mut texto = prompt.to_string();
        if !texto.ends_with(' ') && !texto.ends_with('\n') {
            texto.push(' ');
        }
        self.escrever(&texto, false);
    }

    pub fn cancelar_leia(&self) {
        let mut leia = self.leia.lock().unwrap();
        leia.aberta = false;
        leia.buffer.clear();
        leia.responder = None;
    }

    pub fn leia_aberta(&self) -> bool {
        self.leia.lock().unwrap().aberta
    }
}

pub struct BackendExpressa {
    output: Option<TerminalOutput>,
    slot: Arc<Mutex<Option<TerminalOutput>>>,
    leia: Arc<Mutex<EstadoLeia>>,
}

pub fn criar() -> (PonteSaida, BackendExpressa) {
    let slot = Arc::new(Mutex::new(None));
    let leia = Arc::new(Mutex::new(EstadoLeia::default()));
    let ponte = PonteSaida {
        output: slot.clone(),
        leia: leia.clone(),
    };
    let backend = BackendExpressa {
        output: None,
        slot,
        leia,
    };
    (ponte, backend)
}

impl BackendExpressa {
    fn print(&self, data: Vec<u8>) {
        if let Some(output) = &self.output {
            let _ = output.write(data);
        }
    }
}

impl TerminalBackend for BackendExpressa {
    fn start(&mut self, output: TerminalOutput) -> Result<(), TerminalError> {
        *self.slot.lock().unwrap() = Some(output.clone());
        self.output = Some(output);
        Ok(())
    }

    fn write(&mut self, data: &[u8]) -> Result<(), TerminalError> {
        let echo = {
            let mut leia = self.leia.lock().unwrap();
            tratar_entrada(&mut leia, data)
        };
        if !echo.is_empty() {
            self.print(echo);
        }
        Ok(())
    }

    fn resize(&mut self, _rows: u16, _cols: u16) {}
}

/// VT line feed is Index; Expressa `escreva` uses `\n` as a new line.
fn para_vt(texto: &str) -> String {
    let mut out = String::with_capacity(texto.len());
    let mut chars = texto.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\n' {
            out.push_str("\r\n");
        } else if c == '\r' {
            out.push('\r');
            if chars.peek() == Some(&'\n') {
                out.push(chars.next().unwrap());
            }
        } else {
            out.push(c);
        }
    }
    out
}

fn tratar_entrada(estado: &mut EstadoLeia, data: &[u8]) -> Vec<u8> {
    if !estado.aberta {
        return Vec::new();
    }
    if data.first() == Some(&0x1b) {
        return Vec::new();
    }
    let mut echo = Vec::new();
    let mut just_cr = false;
    for character in String::from_utf8_lossy(data).chars() {
        match character {
            '\n' if just_cr => {
                just_cr = false;
            }
            '\r' | '\n' => {
                just_cr = character == '\r';
                echo.extend_from_slice(b"\r\n");
                let linha = std::mem::take(&mut estado.buffer);
                estado.aberta = false;
                if let Some(tx) = estado.responder.take() {
                    let _ = tx.send(linha);
                }
            }
            '\u{7f}' | '\u{8}' => {
                just_cr = false;
                if estado.buffer.pop().is_some() {
                    echo.extend_from_slice(b"\x08 \x08");
                }
            }
            character if !character.is_control() => {
                just_cr = false;
                estado.buffer.push(character);
                echo.extend(character.to_string().into_bytes());
            }
            _ => {
                just_cr = false;
            }
        }
    }
    echo
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lf_vira_crlf() {
        assert_eq!(para_vt("olá\n"), "olá\r\n");
        assert_eq!(para_vt("a\r\nb"), "a\r\nb");
    }

    #[test]
    fn leia_enter_envia_e_eco() {
        let (tx, rx) = std::sync::mpsc::channel();
        let mut e = EstadoLeia {
            aberta: true,
            buffer: String::new(),
            responder: Some(tx),
        };
        let echo = tratar_entrada(&mut e, "Ana".as_bytes());
        assert_eq!(echo, b"Ana");
        let echo = tratar_entrada(&mut e, b"\r");
        assert_eq!(echo, b"\r\n");
        assert!(!e.aberta);
        assert_eq!(rx.recv().unwrap(), "Ana");
    }

    #[test]
    fn leia_backspace_apaga_caractere() {
        let mut e = EstadoLeia {
            aberta: true,
            buffer: String::new(),
            responder: None,
        };
        tratar_entrada(&mut e, "ão".as_bytes());
        tratar_entrada(&mut e, b"\x7f");
        assert_eq!(e.buffer, "ã");
    }

    #[test]
    fn sem_leia_ignora_teclado() {
        let mut e = EstadoLeia::default();
        assert!(tratar_entrada(&mut e, b"x").is_empty());
    }
}
