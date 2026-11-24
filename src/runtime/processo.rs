//! Child processes for `importe "processo"` (`execute`, `linhas_de`).
//!
//! A pipeline is a list of command groups. Commands inside a group are joined
//! by OS pipes; between groups the VM runs Expressa functions on each line
//! (values are `Rc`, so that part stays on the main thread). Reader threads
//! send every stdout/stderr line through one channel; writer threads feed
//! each group's stdin, so the main thread never blocks on a child.

use std::collections::VecDeque;
use std::io::{self, BufRead, BufReader, BufWriter, Read, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, ChildStdout, Command, ExitStatus, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender, TryRecvError};
use std::thread;
use std::time::{Duration, Instant};

use super::value::Value;

/// After the last command exits, how long to wait for stderr still in flight.
const ESPERA_STDERR: Duration = Duration::from_millis(500);

/// One Expressa function stage: `f(linha, extras…)`.
#[derive(Clone)]
pub(crate) struct EtapaFuncao {
    pub f: Value,
    pub extras: Vec<Value>,
    /// Name in errors and in the call stack.
    pub nome: String,
}

#[derive(Default)]
pub(crate) struct Opcoes {
    pub pasta: Option<PathBuf>,
    pub ambiente: Vec<(String, String)>,
    /// Whole stdin of the first command; `None` = no input (null).
    pub entrada: Option<String>,
    pub tempo: Option<Duration>,
    /// Code != 0 is a runtime error.
    pub exija: bool,
    /// stdin, stdout and stderr are the terminal's (vim, ssh, colors).
    pub terminal: bool,
}

/// A running `linhas_de`/`execute`: the processes plus the VM-side stages.
pub struct Processo {
    pub(crate) descricao: String,
    pub(crate) fluxo: Fluxo,
    /// `funcoes[g]`: stages applied to lines leaving group `g`.
    pub(crate) funcoes: Vec<Vec<EtapaFuncao>>,
    /// Lines that passed every stage, waiting for the consumer.
    pub(crate) saida: VecDeque<String>,
    /// stderr lines, when not written as they arrive.
    pub(crate) erros: Vec<String>,
    /// `linhas_de` writes stderr live; `execute` keeps it for `:erro`.
    pub(crate) erro_ao_vivo: bool,
    /// From `:tempo`.
    pub(crate) limite: Option<Instant>,
    /// Set once every process finished.
    pub(crate) codigo: Option<i32>,
    /// From `:exija`.
    pub(crate) exija: bool,
}

impl Processo {
    /// Kill whatever still runs; the iteration ends.
    pub(crate) fn encerre(&mut self) {
        self.fluxo.encerre();
        self.saida.clear();
        if self.codigo.is_none() {
            self.codigo = Some(self.fluxo.codigo());
        }
    }
}

enum Evento {
    Linha(usize, String),
    Erro(String),
    FimSaida(usize),
    FimErro,
    /// The group's stdin closed early (the command stopped reading).
    EntradaFechada(usize),
}

pub(crate) enum Recebido {
    Linha(usize, String),
    Erro(String),
    Nada,
    Acabou,
}

struct Filho {
    child: Child,
    status: Option<ExitStatus>,
    /// Killed by us (stop, or nobody reads its output): status ignored.
    morto_por_nos: bool,
}

impl Filho {
    fn terminou(&mut self) -> bool {
        if self.status.is_none() {
            self.status = self.child.try_wait().ok().flatten();
        }
        self.status.is_some()
    }

    fn mate(&mut self) {
        if !self.terminou() {
            let _ = self.child.kill();
            self.morto_por_nos = true;
        }
    }

    fn colha(&mut self) {
        if self.status.is_none() {
            self.status = self.child.wait().ok();
        }
    }
}

#[derive(Default)]
struct Grupo {
    filhos: Vec<Filho>,
    entrada: Option<Sender<String>>,
}

pub(crate) struct Fluxo {
    grupos: Vec<Grupo>,
    rx: Receiver<Evento>,
    /// Reader threads (stdout of each group, stderr of each child) still open.
    abertos: usize,
    /// The last group's stdout closed.
    saida_fechada: bool,
    encerrando_desde: Option<Instant>,
    acabou: bool,
}

impl Fluxo {
    /// Spawns every command. `grupos[g]` is a non-empty list of argv.
    /// With `opcoes.terminal` there is one group and no reader: the first
    /// stdin, the last stdout and every stderr are the terminal.
    pub fn inicie(grupos: Vec<Vec<Vec<String>>>, opcoes: &Opcoes) -> Result<Fluxo, String> {
        let (tx, rx) = mpsc::channel();
        let mut fluxo = Fluxo {
            grupos: Vec::new(),
            rx,
            abertos: 0,
            saida_fechada: false,
            encerrando_desde: None,
            acabou: false,
        };
        for (g, cmds) in grupos.into_iter().enumerate() {
            fluxo.grupos.push(Grupo::default());
            let com_entrada = g > 0 || opcoes.entrada.is_some();
            let ultimo = cmds.len() - 1;
            let mut anterior: Option<ChildStdout> = None;
            for (i, argv) in cmds.into_iter().enumerate() {
                let mut cmd = Command::new(&argv[0]);
                cmd.args(&argv[1..]);
                if let Some(p) = &opcoes.pasta {
                    cmd.current_dir(p);
                }
                for (k, v) in &opcoes.ambiente {
                    cmd.env(k, v);
                }
                let stdin = match anterior.take() {
                    Some(out) => Stdio::from(out),
                    None if opcoes.terminal => Stdio::inherit(),
                    None if com_entrada => Stdio::piped(),
                    None => Stdio::null(),
                };
                let (stdout, stderr) = match opcoes.terminal {
                    true if i == ultimo => (Stdio::inherit(), Stdio::inherit()),
                    true => (Stdio::piped(), Stdio::inherit()),
                    false => (Stdio::piped(), Stdio::piped()),
                };
                cmd.stdin(stdin).stdout(stdout).stderr(stderr);
                // On error `fluxo` drops and kills what already started.
                let mut child = cmd.spawn().map_err(|e| erro_ao_iniciar(&argv[0], &e))?;
                drop(cmd);

                if let Some(err) = child.stderr.take() {
                    fluxo.abertos += 1;
                    let tx = tx.clone();
                    thread::spawn(move || {
                        leia_linhas(err, |l| tx.send(Evento::Erro(l)).is_ok());
                        let _ = tx.send(Evento::FimErro);
                    });
                }
                if i == 0
                    && com_entrada
                    && let Some(stdin) = child.stdin.take()
                {
                    let (etx, erx) = mpsc::channel();
                    let tx = tx.clone();
                    thread::spawn(move || escreva_entrada(stdin, erx, g, tx));
                    fluxo.grupos[g].entrada = Some(etx);
                }
                let out = child.stdout.take();
                if i < ultimo {
                    anterior = out;
                } else if let Some(out) = out {
                    fluxo.abertos += 1;
                    let tx = tx.clone();
                    thread::spawn(move || {
                        leia_linhas(out, |l| tx.send(Evento::Linha(g, l)).is_ok());
                        let _ = tx.send(Evento::FimSaida(g));
                    });
                }
                fluxo.grupos[g].filhos.push(Filho {
                    child,
                    status: None,
                    morto_por_nos: false,
                });
            }
        }
        if let Some(texto) = &opcoes.entrada
            && let Some(e) = fluxo.grupos[0].entrada.take()
        {
            let _ = e.send(texto.clone());
        }
        if opcoes.terminal {
            // Nothing to read: it ends when the last command exits.
            fluxo.saida_fechada = true;
        }
        Ok(fluxo)
    }

    pub fn grupos(&self) -> usize {
        self.grupos.len()
    }

    /// Line for the stdin of group `g` (after the functions ran).
    pub fn envie(&mut self, g: usize, mut linha: String) {
        if let Some(e) = &self.grupos[g].entrada {
            linha.push('\n');
            let _ = e.send(linha);
        }
    }

    /// Next event, waiting at most `espera`. Internal events give `Nada`.
    pub fn receba(&mut self, espera: Duration) -> Recebido {
        if self.acabou {
            return Recebido::Acabou;
        }
        if self.saida_fechada && self.ultimo().is_none_or(|f| f.terminou()) {
            // The final command is gone: nothing more will reach the consumer,
            // so what still runs upstream is stopped (as SIGPIPE would).
            let desde = match self.encerrando_desde {
                Some(t) => t,
                None => {
                    self.mate_todos();
                    *self.encerrando_desde.insert(Instant::now())
                }
            };
            if self.abertos == 0 || desde.elapsed() >= ESPERA_STDERR {
                self.encerre();
                return Recebido::Acabou;
            }
        }
        match self.rx.recv_timeout(espera) {
            Ok(Evento::Linha(g, l)) => Recebido::Linha(g, l),
            Ok(Evento::Erro(l)) => Recebido::Erro(l),
            Ok(Evento::FimErro) => {
                self.abertos -= 1;
                Recebido::Nada
            }
            Ok(Evento::FimSaida(g)) => {
                self.abertos -= 1;
                if g + 1 < self.grupos.len() {
                    self.grupos[g + 1].entrada = None;
                } else {
                    self.saida_fechada = true;
                }
                Recebido::Nada
            }
            Ok(Evento::EntradaFechada(g)) => {
                // Like SIGPIPE: nobody reads what the earlier groups write.
                for grupo in &mut self.grupos[..g] {
                    grupo.entrada = None;
                    for f in &mut grupo.filhos {
                        f.mate();
                    }
                }
                Recebido::Nada
            }
            Err(RecvTimeoutError::Timeout) => Recebido::Nada,
            Err(RecvTimeoutError::Disconnected) => {
                // Every reader is done; only the exit is missing.
                self.abertos = 0;
                self.saida_fechada = true;
                thread::sleep(espera.min(Duration::from_millis(10)));
                Recebido::Nada
            }
        }
    }

    /// Kill what still runs and reap every child.
    pub fn encerre(&mut self) {
        if self.acabou {
            return;
        }
        self.mate_todos();
        for g in &mut self.grupos {
            for f in &mut g.filhos {
                f.colha();
            }
        }
        self.acabou = true;
    }

    /// First failure among the commands, in order (like bash `pipefail`).
    /// Processes we killed, and SIGPIPE before the last command, count as ok.
    pub fn codigo(&self) -> i32 {
        let filhos: Vec<&Filho> = self.grupos.iter().flat_map(|g| &g.filhos).collect();
        let ultimo = filhos.len().saturating_sub(1);
        for (i, f) in filhos.iter().enumerate() {
            let Some(st) = f.status else { continue };
            if f.morto_por_nos || st.success() {
                continue;
            }
            if i < ultimo && morreu_de_sigpipe(&st) {
                continue;
            }
            return codigo_de(&st);
        }
        0
    }

    fn ultimo(&mut self) -> Option<&mut Filho> {
        self.grupos.last_mut().and_then(|g| g.filhos.last_mut())
    }

    fn mate_todos(&mut self) {
        for g in &mut self.grupos {
            g.entrada = None;
            for f in &mut g.filhos {
                f.mate();
            }
        }
    }
}

impl Drop for Fluxo {
    fn drop(&mut self) {
        self.encerre();
    }
}

fn erro_ao_iniciar(nome: &str, e: &io::Error) -> String {
    match e.kind() {
        io::ErrorKind::NotFound => format!("comando não encontrado: '{nome}'"),
        io::ErrorKind::PermissionDenied => format!("sem permissão para executar '{nome}'"),
        _ => format!("não foi possível executar '{nome}': {e}"),
    }
}

/// Lines without `\n` / `\r\n`; invalid UTF-8 becomes U+FFFD.
fn leia_linhas(r: impl Read, mut envie: impl FnMut(String) -> bool) {
    let mut r = BufReader::new(r);
    let mut buf = Vec::new();
    loop {
        buf.clear();
        match r.read_until(b'\n', &mut buf) {
            Ok(0) | Err(_) => return,
            Ok(_) => {
                if buf.last() == Some(&b'\n') {
                    buf.pop();
                    if buf.last() == Some(&b'\r') {
                        buf.pop();
                    }
                }
                if !envie(String::from_utf8_lossy(&buf).into_owned()) {
                    return;
                }
            }
        }
    }
}

/// Writes what arrives on `rx`; flushes whenever the queue is empty, so lines
/// reach the child while they stream. Ends (closing stdin) when `rx` closes.
fn escreva_entrada(stdin: ChildStdin, rx: Receiver<String>, g: usize, tx: Sender<Evento>) {
    let mut w = BufWriter::new(stdin);
    let fechou = |e: io::Error| {
        if e.kind() == io::ErrorKind::BrokenPipe {
            let _ = tx.send(Evento::EntradaFechada(g));
        }
    };
    while let Ok(texto) = rx.recv() {
        let mut texto = texto;
        loop {
            if let Err(e) = w.write_all(texto.as_bytes()) {
                return fechou(e);
            }
            match rx.try_recv() {
                Ok(mais) => texto = mais,
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    if let Err(e) = w.flush() {
                        fechou(e);
                    }
                    return;
                }
            }
        }
        if let Err(e) = w.flush() {
            return fechou(e);
        }
    }
    if let Err(e) = w.flush() {
        fechou(e);
    }
}

#[cfg(unix)]
fn morreu_de_sigpipe(st: &ExitStatus) -> bool {
    use std::os::unix::process::ExitStatusExt;
    st.signal() == Some(13)
}

#[cfg(not(unix))]
fn morreu_de_sigpipe(_st: &ExitStatus) -> bool {
    false
}

#[cfg(unix)]
fn codigo_de(st: &ExitStatus) -> i32 {
    use std::os::unix::process::ExitStatusExt;
    st.code()
        .or_else(|| st.signal().map(|s| 128 + s))
        .unwrap_or(1)
}

#[cfg(not(unix))]
fn codigo_de(st: &ExitStatus) -> i32 {
    st.code().unwrap_or(1)
}

/// Splits command text into a pipeline: words separated by spaces, `"…"` and
/// `'…'` keep spaces, `|` starts the next command. No shell: globs,
/// redirections and variables are refused instead of silently passed on.
pub(crate) fn separe_comando(texto: &str) -> Result<Vec<Vec<String>>, String> {
    let mut cmds: Vec<Vec<String>> = Vec::new();
    let mut atual: Vec<String> = Vec::new();
    let mut palavra = String::new();
    let mut tem_palavra = false;
    let mut chars = texto.chars();
    while let Some(c) = chars.next() {
        match c {
            '"' | '\'' => {
                tem_palavra = true;
                loop {
                    match chars.next() {
                        Some(q) if q == c => break,
                        Some(q) => palavra.push(q),
                        None => return Err(format!("aspas {c} sem fechar no comando: {texto}")),
                    }
                }
            }
            '|' => {
                if tem_palavra {
                    atual.push(std::mem::take(&mut palavra));
                    tem_palavra = false;
                }
                if atual.is_empty() {
                    return Err(format!("comando vazio ao lado de '|': {texto}"));
                }
                cmds.push(std::mem::take(&mut atual));
            }
            '>' | '<' | '&' | ';' | '$' | '*' | '?' | '`' => {
                return Err(format!(
                    "'{c}' não funciona no texto do comando (não há shell): {texto}; \
                     use aspas para ter o caractere, ou uma lista como [\"sh\", \"-c\", \"…\"]"
                ));
            }
            c if c.is_whitespace() => {
                if tem_palavra {
                    atual.push(std::mem::take(&mut palavra));
                    tem_palavra = false;
                }
            }
            c => {
                palavra.push(c);
                tem_palavra = true;
            }
        }
    }
    if tem_palavra {
        atual.push(palavra);
    }
    if atual.is_empty() {
        return Err(if cmds.is_empty() {
            "comando vazio".to_string()
        } else {
            format!("comando vazio ao lado de '|': {texto}")
        });
    }
    cmds.push(atual);
    Ok(cmds)
}

#[cfg(test)]
mod tests {
    use super::separe_comando;

    fn s(xs: &[&[&str]]) -> Vec<Vec<String>> {
        xs.iter()
            .map(|c| c.iter().map(|w| w.to_string()).collect())
            .collect()
    }

    #[test]
    fn palavras_e_aspas() {
        assert_eq!(separe_comando("ls -l").unwrap(), s(&[&["ls", "-l"]]));
        assert_eq!(
            separe_comando("grep 'a b' \"c d\"").unwrap(),
            s(&[&["grep", "a b", "c d"]])
        );
        assert_eq!(
            separe_comando("x a\"b c\"d").unwrap(),
            s(&[&["x", "ab cd"]])
        );
        assert_eq!(separe_comando("x ''").unwrap(), s(&[&["x", ""]]));
        assert_eq!(separe_comando("  x  ").unwrap(), s(&[&["x"]]));
        assert_eq!(
            separe_comando("dir C:\\pasta").unwrap(),
            s(&[&["dir", "C:\\pasta"]])
        );
    }

    #[test]
    fn barra_vertical_separa_comandos() {
        assert_eq!(
            separe_comando("cat a | sort|uniq -c").unwrap(),
            s(&[&["cat", "a"], &["sort"], &["uniq", "-c"]])
        );
        assert_eq!(
            separe_comando("echo 'a|b'").unwrap(),
            s(&[&["echo", "a|b"]])
        );
    }

    #[test]
    fn recusa_o_que_so_um_shell_faria() {
        for t in [
            "ls *.txt",
            "ls > a",
            "a && b",
            "echo $HOME",
            "a; b",
            "x `y`",
        ] {
            let e = separe_comando(t).unwrap_err();
            assert!(e.contains("não há shell"), "{t}: {e}");
        }
        assert_eq!(
            separe_comando("echo '*' \"$x\"").unwrap(),
            s(&[&["echo", "*", "$x"]])
        );
    }

    #[test]
    fn comandos_vazios() {
        assert_eq!(separe_comando("").unwrap_err(), "comando vazio");
        assert_eq!(separe_comando("   ").unwrap_err(), "comando vazio");
        assert!(separe_comando("ls |").is_err());
        assert!(separe_comando("| ls").is_err());
        assert!(separe_comando("a || b").is_err());
        assert!(
            separe_comando("echo 'x")
                .unwrap_err()
                .contains("sem fechar")
        );
    }
}
