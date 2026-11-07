//! Ver → REPL: CanalRepl session, same as GTK Aula.

use std::sync::{Arc, Mutex};
use std::sync::mpsc::TryRecvError;

use expressa::runtime::{
    CanalRepl, ConteudoRepl, FimRepl, FluxoRepl, REPL_MAX_CODIGO, TextoRepl,
};

const MAX_LINHAS: usize = 5000;
const MAX_BYTES: usize = 256 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TipoLinha {
    Eco,
    Saida,
    Erro,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinhaRepl {
    pub tipo: TipoLinha,
    pub texto: String,
}

struct Inner {
    canal: Option<CanalRepl>,
    stdout: TextoRepl,
    stderr: TextoRepl,
    pendente: String,
}

#[derive(Clone)]
pub struct Repl {
    inner: Arc<Mutex<Inner>>,
    pub visivel: bool,
    pub linhas: Vec<LinhaRepl>,
    pub prompt: String,
    pub ocupado: bool,
    pub ativo: bool,
}

impl Default for Repl {
    fn default() -> Self {
        Self {
            inner: Arc::new(Mutex::new(Inner {
                canal: None,
                stdout: TextoRepl::default(),
                stderr: TextoRepl::default(),
                pendente: String::new(),
            })),
            visivel: false,
            linhas: Vec::new(),
            prompt: ">".into(),
            ocupado: false,
            ativo: false,
        }
    }
}

impl PartialEq for Repl {
    fn eq(&self, other: &Self) -> bool {
        self.visivel == other.visivel
            && self.linhas == other.linhas
            && self.prompt == other.prompt
            && self.ocupado == other.ocupado
            && self.ativo == other.ativo
    }
}

impl Eq for Repl {}

impl Repl {
    pub fn iniciar(&mut self) {
        let boas = self.linhas.is_empty();
        let resultado = {
            let mut g = self.inner.lock().unwrap();
            if g.canal.is_some() {
                return;
            }
            match CanalRepl::nova() {
                Ok(canal) => {
                    g.canal = Some(canal);
                    g.stdout = TextoRepl::default();
                    g.stderr = TextoRepl::default();
                    g.pendente.clear();
                    Ok(())
                }
                Err(e) => Err(e.to_string()),
            }
        };
        match resultado {
            Ok(()) => {
                self.ativo = true;
                self.prompt = ">".into();
                if boas {
                    self.escrever(
                        "REPL da Expressa: cada linha roda na hora e as variáveis ficam para as próximas.",
                        TipoLinha::Eco,
                    );
                }
            }
            Err(e) => {
                self.ativo = false;
                self.escrever(
                    &format!("não foi possível iniciar o REPL: {e}"),
                    TipoLinha::Erro,
                );
            }
        }
    }

    pub fn enviar(&mut self, linha: &str) -> Result<(), String> {
        self.iniciar();
        let g = self.inner.lock().unwrap();
        if g.canal.is_none() {
            return Err("a sessão parou; aperte Reiniciar".into());
        }
        if g.pendente.len() + linha.len() + 1 > REPL_MAX_CODIGO {
            return Err("código grande demais para o REPL (limite: 64 KiB)".into());
        }
        let primeira = g.pendente.is_empty();
        if primeira && matches!(linha.trim(), "limpe" | "cls" | "limpe_tela") {
            drop(g);
            self.linhas.clear();
            return Ok(());
        }
        if primeira && linha.trim().is_empty() {
            return Ok(());
        }
        let eco = if primeira { "> " } else { "... " };
        drop(g);
        self.escrever(&format!("{eco}{linha}"), TipoLinha::Eco);
        let mut g = self.inner.lock().unwrap();
        g.pendente.push_str(linha);
        g.pendente.push('\n');
        if expressa::parser::needs_more_input(&g.pendente) {
            self.prompt = "...".into();
            return Ok(());
        }
        let codigo = std::mem::take(&mut g.pendente);
        self.prompt = ">".into();
        let r = g.canal.as_mut().unwrap().enviar(codigo);
        self.ocupado = g.canal.as_ref().map(|c| c.pendentes() > 0).unwrap_or(false);
        r.map(|_| ()).map_err(|e| e.to_string())
    }

    pub fn drenar(&mut self) {
        let mut g = self.inner.lock().unwrap();
        let mut novos = Vec::new();
        let mut acabou = false;
        #[allow(clippy::while_let_loop)]
        loop {
            let ev = match g.canal.as_mut() {
                Some(c) => c.receber(),
                None => break,
            };
            match ev {
                Ok(evento) => match evento.conteudo {
                    ConteudoRepl::Saida { fluxo, bytes } => {
                        let (dec, tipo) = match fluxo {
                            FluxoRepl::Stdout => (&mut g.stdout, TipoLinha::Saida),
                            FluxoRepl::Stderr => (&mut g.stderr, TipoLinha::Erro),
                        };
                        let t = dec.decodificar(&bytes);
                        if !t.is_empty() {
                            novos.push((t, tipo));
                        }
                    }
                    ConteudoRepl::Resultado(r) => {
                        let out = g.stdout.terminar();
                        let err = g.stderr.terminar();
                        if !out.is_empty() {
                            novos.push((out, TipoLinha::Saida));
                        }
                        if !err.is_empty() {
                            novos.push((err, TipoLinha::Erro));
                        }
                        match r {
                            Ok(Some(v)) => novos.push((format!("{v}\n"), TipoLinha::Saida)),
                            Ok(None) => {}
                            Err(e) => novos.push((format!("{e}\n"), TipoLinha::Erro)),
                        }
                    }
                    ConteudoRepl::Terminou(fim) => {
                        let out = g.stdout.terminar();
                        let err = g.stderr.terminar();
                        if !out.is_empty() {
                            novos.push((out, TipoLinha::Saida));
                        }
                        if !err.is_empty() {
                            novos.push((err, TipoLinha::Erro));
                        }
                        let (msg, tipo) = match fim {
                            FimRepl::Saida(code) => (
                                format!("sessão encerrada (código {code}); aperte Reiniciar\n"),
                                TipoLinha::Eco,
                            ),
                            FimRepl::Encerrada => (
                                "sessão encerrada; aperte Reiniciar\n".into(),
                                TipoLinha::Eco,
                            ),
                            FimRepl::Falha(e) => (
                                format!("falha na sessão: {e}; aperte Reiniciar\n"),
                                TipoLinha::Erro,
                            ),
                        };
                        novos.push((msg, tipo));
                        acabou = true;
                    }
                },
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    novos.push((
                        "a sessão do REPL foi desconectada; aperte Reiniciar\n".into(),
                        TipoLinha::Erro,
                    ));
                    if let Some(c) = g.canal.as_mut() {
                        c.encerrar();
                    }
                    acabou = true;
                    break;
                }
            }
        }
        self.ocupado = g.canal.as_ref().map(|c| c.pendentes() > 0).unwrap_or(false);
        if acabou {
            g.canal = None;
            self.ativo = false;
            self.ocupado = false;
        }
        drop(g);
        for (t, tipo) in novos {
            self.escrever(&t, tipo);
        }
    }

    pub fn parar(&self) {
        if let Some(c) = self.inner.lock().unwrap().canal.as_ref() {
            c.parar();
        }
    }

    pub fn reiniciar(&mut self) {
        {
            let mut g = self.inner.lock().unwrap();
            g.canal.take();
            g.stdout = TextoRepl::default();
            g.stderr = TextoRepl::default();
            g.pendente.clear();
        }
        self.ativo = false;
        self.ocupado = false;
        self.prompt = ">".into();
        self.escrever(
            "— sessão nova: as variáveis foram esquecidas —",
            TipoLinha::Eco,
        );
        self.iniciar();
    }

    pub fn limpar(&mut self) {
        self.linhas.clear();
    }

    fn escrever(&mut self, texto: &str, tipo: TipoLinha) {
        for linha in texto.split_inclusive('\n') {
            let t = linha.trim_end_matches('\n');
            if t.is_empty() && !linha.ends_with('\n') {
                continue;
            }
            self.linhas.push(LinhaRepl {
                tipo,
                texto: t.to_string(),
            });
        }
        if self.linhas.len() > MAX_LINHAS {
            let extra = self.linhas.len() - MAX_LINHAS;
            self.linhas.drain(..extra);
        }
        let mut bytes = 0usize;
        for l in self.linhas.iter().rev() {
            bytes += l.texto.len();
            if bytes > MAX_BYTES {
                break;
            }
        }
        if bytes > MAX_BYTES {
            while self.linhas.len() > 1 {
                let total: usize = self.linhas.iter().map(|l| l.texto.len()).sum();
                if total <= MAX_BYTES {
                    break;
                }
                self.linhas.remove(0);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn espera(r: &mut Repl, pred: impl Fn(&Repl) -> bool) {
        for _ in 0..400 {
            r.drenar();
            if pred(r) {
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        panic!("timeout REPL: {:?}", r.linhas);
    }

    fn textos(r: &Repl) -> String {
        r.linhas
            .iter()
            .map(|l| l.texto.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn sessao_guarda_variaveis() {
        let mut r = Repl::default();
        r.iniciar();
        r.enviar("2+2").unwrap();
        espera(&mut r, |r| textos(r).contains('4'));
        r.enviar("x = 1").unwrap();
        espera(&mut r, |r| !r.ocupado);
        r.enviar("x").unwrap();
        espera(&mut r, |r| textos(r).lines().any(|l| l.trim() == "1"));
    }

    #[test]
    fn laco_e_parar() {
        let mut r = Repl::default();
        r.iniciar();
        r.enviar("enquanto verdadeiro { escreva(\"vivo\") }").unwrap();
        espera(&mut r, |r| textos(r).contains("vivo"));
        r.parar();
        espera(&mut r, |r| !r.ocupado);
        r.enviar("2+2").unwrap();
        espera(&mut r, |r| textos(r).contains('4'));
    }
}
