use crate::lexer::Span;

use super::super::error::EvalError;
use super::super::eval::Vm;
use super::super::leia::{CLEAR_SCREEN, CURSOR_HOME};
use super::super::value::{MapKey, Value};

const ERASE_DOWN: &str = "\x1b[J";
const ERASE_LINE: &str = "\x1b[K";
const TELA_COLUNAS_PADRAO: u32 = 80;
const TELA_LINHAS_PADRAO: u32 = 24;

/// `"\r\n"` on Windows, `"\n"` elsewhere. Same value as `nova_linha` in `tela`.
pub(crate) const NOVA_LINHA: &str = if cfg!(windows) { "\r\n" } else { "\n" };

const RESET: &str = "\x1b[0m";

fn sgr_cor(nome: &str, fundo: bool) -> Option<u8> {
    let n = match nome {
        "normal" => return Some(if fundo { 49 } else { 39 }),
        "preto" => 0,
        "vermelho" => 1,
        "verde" => 2,
        "amarelo" => 3,
        "azul" => 4,
        "magenta" => 5,
        "ciano" => 6,
        "branco" => 7,
        _ => return None,
    };
    Some(if fundo { 40 + n } else { 30 + n })
}

fn wrap_sgr(texto: &str, codes: &[u8]) -> String {
    let mut out = String::from("\x1b[");
    for (i, c) in codes.iter().enumerate() {
        if i > 0 {
            out.push(';');
        }
        out.push_str(&c.to_string());
    }
    out.push('m');
    out.push_str(texto);
    out.push_str(RESET);
    out
}

impl Vm<'_> {
    fn cor_codigo(&self, v: &Value, fundo: bool, span: Span) -> Result<u8, EvalError> {
        let nome = self.expect_texto(v, span)?;
        sgr_cor(&nome, fundo).ok_or_else(|| {
            self.err(
                format!(
                    "cor desconhecida (\"{nome}\"); use normal, preto, vermelho, verde, amarelo, azul, magenta, ciano ou branco"
                ),
                span,
            )
        })
    }

    pub(crate) fn bi_cls(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        if !args.is_empty() {
            return Err(self.err(
                format!("cls() não espera argumentos, recebeu {}", args.len()),
                span,
            ));
        }
        write!(self.out, "{CLEAR_SCREEN}").map_err(|e| self.io_err(e, span))?;
        self.out.flush().map_err(|e| self.io_err(e, span))?;
        Ok(Value::Nada)
    }
    pub(crate) fn bi_casa(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        if !args.is_empty() {
            return Err(self.err(
                format!("casa() não espera argumentos, recebeu {}", args.len()),
                span,
            ));
        }
        // No flush: the next escreva() should paint in the same burst, so the
        // terminal never shows a blank frame.
        write!(self.out, "{CURSOR_HOME}").map_err(|e| self.io_err(e, span))?;
        Ok(Value::Nada)
    }
    pub(crate) fn bi_eh_terminal(
        &mut self,
        args: &[Value],
        span: Span,
    ) -> Result<Value, EvalError> {
        if !args.is_empty() {
            return Err(self.err(
                format!(
                    "eh_terminal() não espera argumentos, recebeu {}",
                    args.len()
                ),
                span,
            ));
        }
        Ok(Value::Bool(self.input.is_terminal()))
    }

    pub(crate) fn bi_pinte(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        if args.len() < 2 || args.len() > 3 {
            return Err(self.err(
                format!("pinte espera 2 ou 3 argumentos, recebeu {}", args.len()),
                span,
            ));
        }
        let texto = self.expect_texto(&args[0], span)?;
        let frente = self.cor_codigo(&args[1], false, span)?;
        let mut codes = vec![frente];
        if args.len() == 3 {
            codes.push(self.cor_codigo(&args[2], true, span)?);
        }
        Ok(Value::Texto(wrap_sgr(&texto, &codes)))
    }

    pub(crate) fn bi_fundo(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        self.expect_arity(args, 2, span)?;
        let texto = self.expect_texto(&args[0], span)?;
        let fundo = self.cor_codigo(&args[1], true, span)?;
        Ok(Value::Texto(wrap_sgr(&texto, &[fundo])))
    }

    pub(crate) fn bi_negrito(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        self.expect_arity(args, 1, span)?;
        let texto = self.expect_texto(&args[0], span)?;
        Ok(Value::Texto(wrap_sgr(&texto, &[1])))
    }

    pub(crate) fn bi_colunas(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        if !args.is_empty() {
            return Err(self.err(
                format!("colunas() não espera argumentos, recebeu {}", args.len()),
                span,
            ));
        }
        Ok(Value::Numero((tamanho_tela().0 as i64).into()))
    }

    pub(crate) fn bi_linhas(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        if !args.is_empty() {
            return Err(self.err(
                format!("linhas() não espera argumentos, recebeu {}", args.len()),
                span,
            ));
        }
        Ok(Value::Numero((tamanho_tela().1 as i64).into()))
    }

    pub(crate) fn bi_quadro(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        self.expect_arity(args, 1, span)?;
        let texto = self.expect_texto(&args[0], span)?;
        write!(self.out, "{CURSOR_HOME}").map_err(|e| self.io_err(e, span))?;
        let mut rest = texto.as_str();
        loop {
            match rest.split_once('\n') {
                Some((line, after)) => {
                    let line = line.strip_suffix('\r').unwrap_or(line);
                    write!(self.out, "{line}{ERASE_LINE}{NOVA_LINHA}")
                        .map_err(|e| self.io_err(e, span))?;
                    rest = after;
                }
                None => {
                    if !rest.is_empty() {
                        write!(self.out, "{rest}{ERASE_LINE}").map_err(|e| self.io_err(e, span))?;
                    }
                    break;
                }
            }
        }
        write!(self.out, "{ERASE_DOWN}").map_err(|e| self.io_err(e, span))?;
        self.out.flush().map_err(|e| self.io_err(e, span))?;
        Ok(Value::Nada)
    }

    pub(crate) fn bi_bloco(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        self.expect_arity(args, 2, span)?;
        let linha = self.expect_int(&args[0], span)?;
        let coluna = self.expect_int(&args[1], span)?;
        if linha < 1 || coluna < 1 {
            return Err(self.err("bloco espera linha e coluna >= 1", span));
        }
        Ok(Value::mapa(vec![
            (MapKey::Texto("linha".into()), Value::Numero(linha.into())),
            (MapKey::Texto("coluna".into()), Value::Numero(coluna.into())),
        ]))
    }

    pub(crate) fn bi_escreva_em(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        self.expect_arity(args, 2, span)?;
        let texto = self.expect_texto(&args[1], span)?;
        if texto.contains('\n') || texto.contains('\r') {
            return Err(self.err(
                "escreva_em escreve uma linha; chame de novo para a linha seguinte",
                span,
            ));
        }
        let Value::Mapa(xs) = &args[0] else {
            return Err(self.err(
                format!(
                    "escreva_em espera um mapa de bloco, encontrado {}",
                    args[0].type_name()
                ),
                span,
            ));
        };
        let linha = map_int(xs, "linha", span, self)?;
        let coluna = map_int(xs, "coluna", span, self)?;
        if linha < 1 || coluna < 1 {
            return Err(self.err("bloco espera linha e coluna >= 1", span));
        }
        write!(self.out, "\x1b[{linha};{coluna}H{texto}{ERASE_LINE}")
            .map_err(|e| self.io_err(e, span))?;
        self.out.flush().map_err(|e| self.io_err(e, span))?;
        {
            let mut entries = xs.borrow_mut();
            set_map_numero(&mut entries, "linha", linha + 1);
        }
        Ok(args[0].clone())
    }
}

fn map_int(
    xs: &std::rc::Rc<std::cell::RefCell<Vec<(MapKey, Value)>>>,
    chave: &str,
    span: Span,
    vm: &Vm<'_>,
) -> Result<i64, EvalError> {
    let key = MapKey::Texto(chave.into());
    let v = xs
        .borrow()
        .iter()
        .find(|(k, _)| *k == key)
        .map(|(_, v)| v.clone())
        .ok_or_else(|| vm.err(format!("bloco precisa de :{chave}"), span))?;
    vm.expect_int(&v, span)
}

fn set_map_numero(entries: &mut Vec<(MapKey, Value)>, chave: &str, n: i64) {
    let key = MapKey::Texto(chave.into());
    if let Some((_, v)) = entries.iter_mut().find(|(k, _)| *k == key) {
        *v = Value::Numero(n.into());
    }
}

fn tamanho_tela() -> (u32, u32) {
    stdout_winsize().unwrap_or((TELA_COLUNAS_PADRAO, TELA_LINHAS_PADRAO))
}

fn stdout_winsize() -> Option<(u32, u32)> {
    #[cfg(unix)]
    {
        let fd = 1; // stdout
        let mut ws = libc::winsize {
            ws_row: 0,
            ws_col: 0,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };
        let r = unsafe { libc::ioctl(fd, libc::TIOCGWINSZ, &mut ws) };
        if r == 0 && ws.ws_col > 0 && ws.ws_row > 0 {
            return Some((ws.ws_col as u32, ws.ws_row as u32));
        }
    }
    None
}
