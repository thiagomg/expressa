use crate::lexer::Span;

use super::super::error::EvalError;
use super::super::eval::Vm;
use super::super::value::Value;

impl Vm<'_> {
    pub(crate) fn expect_matriz(
        &self,
        v: &Value,
        span: Span,
    ) -> Result<std::rc::Rc<std::cell::RefCell<Vec<Vec<f64>>>>, EvalError> {
        match v {
            Value::Matriz(m) => Ok(std::rc::Rc::clone(m)),
            _ => Err(self.err(
                format!("esperado matriz, encontrado {}", v.type_name()),
                span,
            )),
        }
    }
    pub(crate) fn bi_transposta(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        self.expect_arity(args, 1, span)?;
        let m = self.expect_matriz(&args[0], span)?;
        let rows = m.borrow();
        let h = rows.len();
        let w = rows.first().map(|r| r.len()).unwrap_or(0);
        let mut out = vec![vec![0.0; h]; w];
        for i in 0..h {
            for j in 0..w {
                out[j][i] = rows[i][j];
            }
        }
        Ok(Value::matriz(out))
    }
    pub(crate) fn bi_identidade(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        self.expect_arity(args, 1, span)?;
        let n = self.expect_int(&args[0], span)?;
        if n < 1 {
            return Err(self.err("identidade() espera um inteiro >= 1", span));
        }
        let n = n as usize;
        let mut out = vec![vec![0.0; n]; n];
        for i in 0..n {
            out[i][i] = 1.0;
        }
        Ok(Value::matriz(out))
    }
    pub(crate) fn expect_dim(&self, v: &Value, span: Span, nome: &str) -> Result<usize, EvalError> {
        let n = self.expect_int(v, span)?;
        if n < 1 {
            return Err(self.err(
                format!("{nome}() espera um inteiro >= 1"),
                span,
            ));
        }
        Ok(n as usize)
    }
    pub(crate) fn dims_1_or_2(
        &self,
        args: &[Value],
        span: Span,
        nome: &str,
    ) -> Result<(usize, usize), EvalError> {
        match args {
            [n] => {
                let n = self.expect_dim(n, span, nome)?;
                Ok((n, n))
            }
            [h, w] => Ok((
                self.expect_dim(h, span, nome)?,
                self.expect_dim(w, span, nome)?,
            )),
            _ => Err(self.err(
                format!("{nome}() espera 1 ou 2 argumentos (linhas, colunas), recebeu {}", args.len()),
                span,
            )),
        }
    }
    pub(crate) fn bi_zeros(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        let (h, w) = self.dims_1_or_2(args, span, "zeros")?;
        Ok(Value::matriz(vec![vec![0.0; w]; h]))
    }
    pub(crate) fn bi_uns(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        let (h, w) = self.dims_1_or_2(args, span, "uns")?;
        Ok(Value::matriz(vec![vec![1.0; w]; h]))
    }
    pub(crate) fn bi_cheia(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        self.expect_arity(args, 3, span)?;
        let h = self.expect_dim(&args[0], span, "cheia")?;
        let w = self.expect_dim(&args[1], span, "cheia")?;
        let v = self.expect_numero(&args[2], span)?;
        Ok(Value::matriz(vec![vec![v; w]; h]))
    }
    pub(crate) fn bi_nlinhas(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        self.expect_arity(args, 1, span)?;
        let m = self.expect_matriz(&args[0], span)?;
        Ok(Value::Numero(m.borrow().len() as f64))
    }
    pub(crate) fn bi_ncolunas(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        self.expect_arity(args, 1, span)?;
        let m = self.expect_matriz(&args[0], span)?;
        let n = m.borrow().first().map(|r| r.len()).unwrap_or(0);
        Ok(Value::Numero(n as f64))
    }
    pub(crate) fn bi_det(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        self.expect_arity(args, 1, span)?;
        let m = self.expect_matriz(&args[0], span)?;
        let a = m.borrow();
        let n = a.len();
        if n == 0 || a.iter().any(|r| r.len() != n) {
            return Err(self.err("det() exige matriz quadrada", span));
        }
        let d = match n {
            1 => a[0][0],
            2 => a[0][0] * a[1][1] - a[0][1] * a[1][0],
            3 => {
                a[0][0] * (a[1][1] * a[2][2] - a[1][2] * a[2][1])
                    - a[0][1] * (a[1][0] * a[2][2] - a[1][2] * a[2][0])
                    + a[0][2] * (a[1][0] * a[2][1] - a[1][1] * a[2][0])
            }
            _ => {
                return Err(self.err("det() só para matrizes 1×1, 2×2 ou 3×3", span));
            }
        };
        Ok(Value::Numero(d))
    }
    pub(crate) fn bi_matriz(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        self.expect_arity(args, 1, span)?;
        self.lista_para_matriz(&args[0], span)
    }

}
