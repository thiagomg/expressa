use crate::lexer::Span;

use super::super::error::EvalError;
use super::super::eval::Vm;
use super::super::leia::{CLEAR_SCREEN, CURSOR_HOME};
use super::super::value::Value;

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
    pub(crate) fn bi_eh_terminal(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
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
}
