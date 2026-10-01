use crate::lexer::Span;

use super::super::error::EvalError;
use super::super::eval::Vm;
use super::super::value::Value;

impl Vm<'_> {
    pub(crate) fn bi_raiz(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        self.expect_arity(args, 1, span)?;
        let n = self.expect_numero(&args[0], span)?;
        if n < 0.0 {
            return Err(self.err("raiz de número negativo", span));
        }
        Ok(Value::Numero(n.sqrt()))
    }
    pub(crate) fn bi_aleatorio(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        self.expect_arity(args, 2, span)?;
        let min = self.expect_int(&args[0], span)?;
        let max = self.expect_int(&args[1], span)?;
        if min > max {
            return Err(self.err("aleatorio() espera min <= max", span));
        }
        Ok(Value::Numero(self.rng.inclusive(min, max) as f64))
    }
    pub(crate) fn bi_semente(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        self.expect_arity(args, 1, span)?;
        let n = self.expect_int(&args[0], span)?;
        self.rng.set_seed(n as u64);
        Ok(Value::Nada)
    }
}
