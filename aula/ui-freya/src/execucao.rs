//! In-process F5 / Parar / leia, same VM as GTK Aula.

use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::mpsc::{self, Receiver, Sender, SyncSender, TryRecvError};

use expressa::runtime::{ChannelDebugger, DebugHook, LeiaHost, StopHook, run_with_hosts};

use crate::debug::{Quadro, Variavel};

use crate::projeto::skip_name;

const EVENT_BACKLOG: usize = 256;

#[derive(Debug, Clone)]
pub enum EventoExec {
    Saida { texto: String, stderr: bool },
    Leia(String),
    Pausado {
        arquivo: String,
        linha: u32,
        trecho: String,
        vars: Vec<Variavel>,
        pilha: Vec<Quadro>,
        erro: Option<String>,
    },
    Fim { ok: bool, erro: String, arquivo: String, linha: u32, pilha: Vec<String> },
}

struct Inner {
    rx: Option<Receiver<EventoExec>>,
    leia: Option<Sender<String>>,
    parar: Option<Sender<String>>,
}

#[derive(Clone)]
pub struct Execucao {
    inner: Arc<Mutex<Inner>>,
    pub rodando: bool,
    pub args: String,
}

impl Default for Execucao {
    fn default() -> Self {
        Self {
            inner: Arc::new(Mutex::new(Inner {
                rx: None,
                leia: None,
                parar: None,
            })),
            rodando: false,
            args: String::new(),
        }
    }
}

impl PartialEq for Execucao {
    fn eq(&self, other: &Self) -> bool {
        self.rodando == other.rodando && self.args == other.args
    }
}

impl Eq for Execucao {}

impl Execucao {
    pub fn iniciar(
        &mut self,
        fonte: String,
        arquivo: String,
        workspace: PathBuf,
        args: Vec<String>,
        debug: bool,
        pontos: Vec<(String, u32)>,
    ) {
        self.parar();
        let (event_tx, events) = mpsc::sync_channel(EVENT_BACKLOG);
        let (leia_tx, leia_rx) = mpsc::channel::<String>();
        let (parar_ui, parar_mid) = mpsc::channel::<String>();
        let (parar_tx, parar_rx) = mpsc::channel::<String>();
        {
            let leia_tx = leia_tx.clone();
            std::thread::spawn(move || {
                while let Ok(cmd) = parar_mid.recv() {
                    if matches!(cmd.trim(), "terminar" | "q" | "quit") {
                        let _ = leia_tx.send(String::new());
                    }
                    if parar_tx.send(cmd).is_err() {
                        break;
                    }
                }
            });
        }
        {
            let mut g = self.inner.lock().unwrap();
            g.rx = Some(events);
            g.leia = Some(leia_tx);
            g.parar = Some(parar_ui);
        }
        self.rodando = true;
        let pontos = pontos
            .into_iter()
            .map(|(f, l)| (abs_no_workspace(&workspace, &f), l))
            .collect();
        std::thread::Builder::new()
            .name("expressa-aula-exec".into())
            .spawn(move || {
                correr(
                    fonte,
                    arquivo,
                    workspace,
                    args,
                    debug,
                    pontos,
                    event_tx,
                    leia_rx,
                    parar_rx,
                );
            })
            .expect("thread de execução");
    }

    pub fn drenar(&mut self) -> Vec<EventoExec> {
        let mut out = Vec::new();
        let mut g = self.inner.lock().unwrap();
        let Some(rx) = g.rx.as_mut() else {
            return out;
        };
        loop {
            match rx.try_recv() {
                Ok(ev) => {
                    if matches!(ev, EventoExec::Fim { .. }) {
                        self.rodando = false;
                    }
                    out.push(ev);
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    self.rodando = false;
                    g.rx = None;
                    break;
                }
            }
        }
        out
    }

    pub fn responder_leia(&self, linha: String) {
        if let Some(tx) = self.inner.lock().unwrap().leia.as_ref() {
            let _ = tx.send(linha);
        }
    }

    pub fn parar(&mut self) {
        self.comando("terminar");
    }

    pub fn comando(&self, cmd: &str) {
        if let Some(tx) = self.inner.lock().unwrap().parar.as_ref() {
            let _ = tx.send(cmd.to_string());
        }
    }
}

struct ChannelOut {
    tx: SyncSender<EventoExec>,
    stderr: bool,
}

impl Write for ChannelOut {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let texto = String::from_utf8_lossy(buf).into_owned();
        let _ = self.tx.send(EventoExec::Saida {
            texto,
            stderr: self.stderr,
        });
        Ok(buf.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

struct ChannelLeia {
    tx: SyncSender<EventoExec>,
    linhas: Receiver<String>,
}

impl LeiaHost for ChannelLeia {
    fn ask(&mut self, prompt: &str) -> Result<String, String> {
        self.tx
            .send(EventoExec::Leia(prompt.to_string()))
            .map_err(|_| "IDE fechada".to_string())?;
        self.linhas.recv().map_err(|_| "IDE fechada".to_string())
    }
}

fn rel_de(root: &Path, file: &str) -> String {
    Path::new(file)
        .strip_prefix(root)
        .map(|p| p.to_string_lossy().replace('\\', "/"))
        .unwrap_or_else(|_| file.replace('\\', "/"))
}

fn abs_no_workspace(ws: &Path, rel: &str) -> String {
    let p = Path::new(rel);
    if p.is_absolute() {
        rel.to_string()
    } else {
        ws.join(p).display().to_string()
    }
}

fn correr(
    fonte: String,
    arquivo: String,
    workspace: PathBuf,
    args: Vec<String>,
    debug: bool,
    pontos: Vec<(String, u32)>,
    tx: SyncSender<EventoExec>,
    leia_rx: Receiver<String>,
    parar_rx: Receiver<String>,
) {
    let mut out = ChannelOut {
        tx: tx.clone(),
        stderr: false,
    };
    let mut err = ChannelOut {
        tx: tx.clone(),
        stderr: true,
    };
    let mut leia = ChannelLeia {
        tx: tx.clone(),
        linhas: leia_rx,
    };
    let hook: Box<dyn DebugHook> = if debug {
        let (pause_tx, pause_rx) = mpsc::channel::<expressa::runtime::DebugPaused>();
        {
            let tx = tx.clone();
            let root = workspace.clone();
            std::thread::spawn(move || {
                while let Ok(p) = pause_rx.recv() {
                    let ev = EventoExec::Pausado {
                        arquivo: rel_de(&root, &p.file),
                        linha: p.line,
                        trecho: p.source_line,
                        vars: p
                            .vars
                            .into_iter()
                            .map(|v| Variavel {
                                escopo: v.scope,
                                nome: v.name,
                                valor: v.value,
                            })
                            .collect(),
                        pilha: p
                            .stack
                            .into_iter()
                            .map(|f| Quadro {
                                nome: f.name,
                                arquivo: rel_de(&root, &f.file),
                                linha: f.line,
                            })
                            .collect(),
                        erro: p.error,
                    };
                    if tx.send(ev).is_err() {
                        break;
                    }
                }
            });
        }
        let mut dbg = ChannelDebugger::new(pause_tx, parar_rx);
        for bp in pontos {
            dbg.session.breakpoints.insert(bp);
        }
        Box::new(dbg)
    } else {
        Box::new(StopHook::new(parar_rx))
    };
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        run_with_hosts(
            &fonte,
            &arquivo,
            hook,
            Some(workspace),
            None,
            &mut out,
            &mut err,
            &mut leia,
            None,
            None,
            &args,
        )
    }));
    let fim = match result {
        Ok(Ok(0)) => EventoExec::Fim {
            ok: true,
            erro: String::new(),
            arquivo: String::new(),
            linha: 0,
            pilha: Vec::new(),
        },
        Ok(Ok(code)) => EventoExec::Fim {
            ok: false,
            erro: format!("encerrou com código {code}"),
            arquivo: String::new(),
            linha: 0,
            pilha: Vec::new(),
        },
        Ok(Err(e)) => {
            let pilha = if e.shows_stack() {
                e.pilha_compacta()
                    .into_iter()
                    .map(|(f, n)| {
                        if n > 0 {
                            format!("{}: {} em {}:{} ×{}", f.name, "", f.file, f.span.line, n + 1)
                        } else {
                            format!("{} em {}:{}", f.name, f.file, f.span.line)
                        }
                    })
                    .collect()
            } else {
                Vec::new()
            };
            EventoExec::Fim {
                ok: false,
                erro: e.message,
                arquivo: e.file,
                linha: e.span.line,
                pilha,
            }
        }
        Err(_) => EventoExec::Fim {
            ok: false,
            erro: "erro interno da Expressa (não é um erro do seu programa; avise o professor)"
                .into(),
            arquivo: String::new(),
            linha: 0,
            pilha: Vec::new(),
        },
    };
    let _ = tx.send(fim);
}

pub fn pasta_exec() -> PathBuf {
    dirs::data_local_dir()
        .or_else(dirs::cache_dir)
        .unwrap_or_else(std::env::temp_dir)
        .join("expressa-aula")
        .join("exec-freya")
}

pub fn preparar_copia(
    raiz: &Path,
    abas: &[(String, String)],
) -> Result<PathBuf, String> {
    let dest = pasta_exec();
    if dest.exists() {
        let _ = std::fs::remove_dir_all(&dest);
    }
    copiar_projeto(raiz, &dest)?;
    for (rel, texto) in abas {
        if rel.is_empty() {
            continue;
        }
        let path = dest.join(rel);
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        std::fs::write(path, texto).map_err(|e| e.to_string())?;
    }
    Ok(dest)
}

fn copiar_projeto(src: &Path, dst: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dst).map_err(|e| e.to_string())?;
    let entries = match std::fs::read_dir(src) {
        Ok(e) => e,
        Err(e) => return Err(e.to_string()),
    };
    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let nome = entry.file_name();
        let nome_s = nome.to_string_lossy();
        if skip_name(&nome_s) {
            continue;
        }
        let kind = entry.file_type().map_err(|e| e.to_string())?;
        if kind.is_symlink() {
            continue;
        }
        let to = dst.join(&nome);
        if kind.is_dir() {
            copiar_projeto(&entry.path(), &to)?;
        } else if kind.is_file() {
            std::fs::copy(entry.path(), &to).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

pub fn caminho_relativo(raiz: &Path, arquivo: Option<&Path>) -> String {
    let Some(arquivo) = arquivo else {
        return "sem-titulo.lep".into();
    };
    arquivo
        .strip_prefix(raiz)
        .map(|p| p.to_string_lossy().replace('\\', "/"))
        .unwrap_or_else(|_| {
            arquivo
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| "sem-titulo.lep".into())
        })
}

pub fn argumentos(texto: &str) -> Vec<String> {
    texto.split_whitespace().map(str::to_string).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn argumentos_sem_aspas() {
        assert_eq!(argumentos(" a  b\tc "), ["a", "b", "c"]);
        assert!(argumentos("  ").is_empty());
    }

    #[test]
    fn relativo_dentro_da_raiz() {
        let raiz = Path::new("/home/ana/aula");
        assert_eq!(
            caminho_relativo(raiz, Some(Path::new("/home/ana/aula/lib/a.lep"))),
            "lib/a.lep"
        );
        assert_eq!(caminho_relativo(raiz, None), "sem-titulo.lep");
    }

    #[test]
    fn roda_escreva_ola() {
        let dir = std::env::temp_dir().join(format!(
            "aula-run-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let src = r#"escreva("olá")"#;
        std::fs::write(dir.join("t.lep"), src).unwrap();
        let mut e = Execucao::default();
        e.iniciar(
            src.into(),
            dir.join("t.lep").display().to_string(),
            dir.clone(),
            vec![],
            false,
            vec![],
        );
        let mut saida = String::new();
        let mut fim = false;
        for _ in 0..200 {
            std::thread::sleep(std::time::Duration::from_millis(10));
            for ev in e.drenar() {
                match ev {
                    EventoExec::Saida { texto, stderr: false } => saida.push_str(&texto),
                    EventoExec::Fim { ok, .. } => {
                        assert!(ok, "{saida}");
                        fim = true;
                    }
                    _ => {}
                }
            }
            if fim {
                break;
            }
        }
        assert!(fim, "não terminou: {saida}");
        assert!(saida.contains("olá"), "{saida}");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn leia_responde() {
        let dir = std::env::temp_dir().join(format!(
            "aula-leia-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let src = r#"n = leia("nome:")
escreva(n)"#;
        std::fs::write(dir.join("t.lep"), src).unwrap();
        let mut e = Execucao::default();
        e.iniciar(
            src.into(),
            dir.join("t.lep").display().to_string(),
            dir.clone(),
            vec![],
            false,
            vec![],
        );
        let mut saida = String::new();
        let mut pediu = false;
        let mut fim = false;
        for _ in 0..200 {
            std::thread::sleep(std::time::Duration::from_millis(10));
            for ev in e.drenar() {
                match ev {
                    EventoExec::Leia(_) => {
                        e.responder_leia("Ana".into());
                        pediu = true;
                    }
                    EventoExec::Saida { texto, stderr: false } => saida.push_str(&texto),
                    EventoExec::Fim { ok, .. } => {
                        assert!(ok, "{saida}");
                        fim = true;
                    }
                    _ => {}
                }
            }
            if fim {
                break;
            }
        }
        assert!(pediu && fim, "leia={pediu} fim={fim} {saida}");
        assert!(saida.contains("Ana"), "{saida}");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn copia_sobrepoe_aba() {
        let root = std::env::temp_dir().join(format!(
            "aula-exec-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(root.join("lib")).unwrap();
        std::fs::write(root.join("a.lep"), "disco").unwrap();
        let dest = preparar_copia(&root, &[("a.lep".into(), "aba".into())]).unwrap();
        assert_eq!(std::fs::read_to_string(dest.join("a.lep")).unwrap(), "aba");
        let _ = std::fs::remove_dir_all(root);
        let _ = std::fs::remove_dir_all(dest);
    }

    fn espera_evento(e: &mut Execucao, n: usize) -> Vec<EventoExec> {
        let mut todos = Vec::new();
        for _ in 0..n {
            std::thread::sleep(std::time::Duration::from_millis(10));
            let novos = e.drenar();
            let parou = novos.iter().any(|ev| {
                matches!(ev, EventoExec::Fim { .. } | EventoExec::Pausado { .. })
            });
            todos.extend(novos);
            if parou {
                break;
            }
        }
        todos
    }

    #[test]
    fn depura_pausa_na_primeira_linha_e_continua() {
        let dir = std::env::temp_dir().join(format!(
            "aula-dbg-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let src = "x = 1\nescreva(x)\n";
        std::fs::write(dir.join("t.lep"), src).unwrap();
        let mut e = Execucao::default();
        e.iniciar(
            src.into(),
            dir.join("t.lep").display().to_string(),
            dir.clone(),
            vec![],
            true,
            vec![],
        );
        let evs = espera_evento(&mut e, 200);
        let pausa = evs.iter().find_map(|ev| match ev {
            EventoExec::Pausado { linha, .. } => Some(*linha),
            _ => None,
        });
        assert_eq!(pausa, Some(1), "{evs:?}");
        e.comando("continuar");
        e.comando("continuar");
        e.comando("continuar");
        let evs = espera_evento(&mut e, 200);
        let ok = evs.iter().any(|ev| matches!(ev, EventoExec::Fim { ok: true, .. }));
        assert!(ok, "{evs:?}");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn depura_para_no_ponto() {
        let dir = std::env::temp_dir().join(format!(
            "aula-bp-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let src = "x = 1\ny = 2\nescreva(y)\n";
        let arquivo = dir.join("t.lep");
        std::fs::write(&arquivo, src).unwrap();
        let mut e = Execucao::default();
        e.iniciar(
            src.into(),
            arquivo.display().to_string(),
            dir.clone(),
            vec![],
            true,
            vec![("t.lep".into(), 2)],
        );
        let evs = espera_evento(&mut e, 200);
        assert!(
            evs.iter()
                .any(|ev| matches!(ev, EventoExec::Pausado { linha: 1, .. })),
            "{evs:?}"
        );
        e.comando("continuar");
        let evs = espera_evento(&mut e, 200);
        assert!(
            evs.iter()
                .any(|ev| matches!(ev, EventoExec::Pausado { linha: 2, .. })),
            "{evs:?}"
        );
        e.comando("continuar");
        let evs = espera_evento(&mut e, 200);
        assert!(
            evs.iter()
                .any(|ev| matches!(ev, EventoExec::Fim { ok: true, .. })),
            "{evs:?}"
        );
        let _ = std::fs::remove_dir_all(dir);
    }
}
