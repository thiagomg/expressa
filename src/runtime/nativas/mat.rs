use crate::lexer::Span;

use super::super::error::EvalError;
use super::super::eval::Vm;
use super::super::value::Value;

impl Vm<'_> {
    pub(crate) fn bi_raiz(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        self.expect_arity(args, 1, span)?;
        let n = self.expect_f64(&args[0], span)?;
        if n < 0.0 {
            return Err(self.err("raiz de número negativo", span));
        }
        Ok(Value::Numero(n.sqrt().into()))
    }
    pub(crate) fn bi_aleatorio(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        self.expect_arity(args, 2, span)?;
        let min = self.expect_int(&args[0], span)?;
        let max = self.expect_int(&args[1], span)?;
        if min > max {
            return Err(self.err("aleatorio() espera min <= max", span));
        }
        Ok(Value::Numero(self.rng.inclusive(min, max).into()))
    }
    pub(crate) fn bi_semente(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        self.expect_arity(args, 1, span)?;
        let n = self.expect_int(&args[0], span)?;
        self.rng.set_seed(n as u64);
        Ok(Value::Nada)
    }

    pub(crate) fn bi_seno(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        self.mat_unario(args, span, f64::sin, "seno indefinido")
    }
    pub(crate) fn bi_cosseno(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        self.mat_unario(args, span, f64::cos, "cosseno indefinido")
    }
    pub(crate) fn bi_tangente(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        self.mat_unario(args, span, f64::tan, "tangente indefinida")
    }
    pub(crate) fn bi_arcoseno(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        self.mat_unario_dom(
            args,
            span,
            |x| (-1.0..=1.0).contains(&x),
            f64::asin,
            "arcoseno espera número entre -1 e 1",
        )
    }
    pub(crate) fn bi_arcocosseno(
        &mut self,
        args: &[Value],
        span: Span,
    ) -> Result<Value, EvalError> {
        self.mat_unario_dom(
            args,
            span,
            |x| (-1.0..=1.0).contains(&x),
            f64::acos,
            "arcocosseno espera número entre -1 e 1",
        )
    }
    pub(crate) fn bi_arcotangente(
        &mut self,
        args: &[Value],
        span: Span,
    ) -> Result<Value, EvalError> {
        self.mat_unario(args, span, f64::atan, "arcotangente indefinida")
    }
    pub(crate) fn bi_arcotangente2(
        &mut self,
        args: &[Value],
        span: Span,
    ) -> Result<Value, EvalError> {
        self.expect_arity(args, 2, span)?;
        let y = self.expect_f64(&args[0], span)?;
        let x = self.expect_f64(&args[1], span)?;
        self.mat_finite(y.atan2(x), span, "arcotangente2 indefinida")
    }
    pub(crate) fn bi_radianos(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        self.mat_unario(
            args,
            span,
            |g| g * std::f64::consts::PI / 180.0,
            "radianos indefinido",
        )
    }
    pub(crate) fn bi_graus(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        self.mat_unario(
            args,
            span,
            |r| r * 180.0 / std::f64::consts::PI,
            "graus indefinido",
        )
    }
    pub(crate) fn bi_exp(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        self.mat_unario(args, span, f64::exp, "exp indefinido")
    }
    pub(crate) fn bi_log(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        self.mat_unario_dom(
            args,
            span,
            |x| x > 0.0,
            f64::ln,
            "log espera número positivo",
        )
    }
    pub(crate) fn bi_log10(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        self.mat_unario_dom(
            args,
            span,
            |x| x > 0.0,
            f64::log10,
            "log10 espera número positivo",
        )
    }
    pub(crate) fn bi_potencia(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        self.expect_arity(args, 2, span)?;
        let base = self.expect_f64(&args[0], span)?;
        let exp = self.expect_f64(&args[1], span)?;
        self.mat_finite(base.powf(exp), span, "potencia indefinida")
    }
    pub(crate) fn bi_piso(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        self.mat_unario(args, span, f64::floor, "piso indefinido")
    }
    pub(crate) fn bi_teto(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        self.mat_unario(args, span, f64::ceil, "teto indefinido")
    }
    pub(crate) fn bi_arredonde(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        self.mat_unario(args, span, f64::round, "arredonde indefinido")
    }

    fn mat_unario(
        &mut self,
        args: &[Value],
        span: Span,
        f: impl FnOnce(f64) -> f64,
        err: &str,
    ) -> Result<Value, EvalError> {
        self.expect_arity(args, 1, span)?;
        let n = self.expect_f64(&args[0], span)?;
        self.mat_finite(f(n), span, err)
    }

    fn mat_unario_dom(
        &mut self,
        args: &[Value],
        span: Span,
        ok: impl FnOnce(f64) -> bool,
        f: impl FnOnce(f64) -> f64,
        err: &str,
    ) -> Result<Value, EvalError> {
        self.expect_arity(args, 1, span)?;
        let n = self.expect_f64(&args[0], span)?;
        if !n.is_finite() || !ok(n) {
            return Err(self.err(err, span));
        }
        self.mat_finite(f(n), span, err)
    }

    fn mat_finite(&self, n: f64, span: Span, err: &str) -> Result<Value, EvalError> {
        if n.is_finite() {
            Ok(Value::Numero(n.into()))
        } else {
            Err(self.err(err, span))
        }
    }
}
