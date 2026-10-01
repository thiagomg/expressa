use crate::lexer::Span;

use super::super::error::EvalError;
use super::super::eval::Vm;
use super::super::leia::{CLEAR_SCREEN, CURSOR_HOME};
use super::super::value::Value;

impl Vm<'_> {
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
}
