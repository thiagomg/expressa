use crate::lexer::Span;

use super::super::error::EvalError;
use super::super::eval::Vm;
use super::super::value::Value;

impl Vm<'_, '_> {
    pub(crate) fn bi_xy(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        match args {
            [matriz, x, y] => {
                self.expect_matriz(matriz, span)?;
                self.index_get2(matriz, y, x, span)
            }
            [matriz, x, y, valor] => {
                self.expect_matriz(matriz, span)?;
                self.assign_index2(matriz, y, x, valor.clone(), span)?;
                Ok(matriz.clone())
            }
            _ => Err(self.err(
                format!(
                    "xy espera 3 ou 4 argumentos (matriz, x, y, valor opcional), recebeu {}",
                    args.len()
                ),
                span,
            )),
        }
    }

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
        let mut out = self.allocate_matrix(w, h, 0.0, span)?;
        for i in 0..h {
            self.check_espera(span)?;
            for j in 0..w {
                if j.is_multiple_of(1024) {
                    self.check_espera(span)?;
                }
                out[j][i] = rows[i][j];
            }
        }
        Ok(Value::matriz(out))
    }
    pub(crate) fn bi_identidade(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        self.expect_arity(args, 1, span)?;
        let n = self.expect_dim(&args[0], span, "identidade")?;
        let mut out = self.allocate_matrix(n, n, 0.0, span)?;
        for (i, row) in out.iter_mut().enumerate() {
            row[i] = 1.0;
        }
        Ok(Value::matriz(out))
    }
    pub(crate) fn expect_dim(&self, v: &Value, span: Span, nome: &str) -> Result<usize, EvalError> {
        let n = self.expect_int(v, span)?;
        if n < 1 {
            return Err(self.err(format!("{nome}() espera um inteiro >= 1"), span));
        }
        usize::try_from(n)
            .map_err(|_| self.err(format!("{nome}() dimensão fora do intervalo"), span))
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
                format!(
                    "{nome}() espera 1 ou 2 argumentos (linhas, colunas), recebeu {}",
                    args.len()
                ),
                span,
            )),
        }
    }
    pub(crate) fn bi_zeros(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        let (h, w) = self.dims_1_or_2(args, span, "zeros")?;
        Ok(Value::matriz(self.allocate_matrix(h, w, 0.0, span)?))
    }
    pub(crate) fn bi_uns(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        let (h, w) = self.dims_1_or_2(args, span, "uns")?;
        Ok(Value::matriz(self.allocate_matrix(h, w, 1.0, span)?))
    }
    pub(crate) fn bi_cheia(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        self.expect_arity(args, 3, span)?;
        let h = self.expect_dim(&args[0], span, "cheia")?;
        let w = self.expect_dim(&args[1], span, "cheia")?;
        let v = self.expect_f64(&args[2], span)?;
        Ok(Value::matriz(self.allocate_matrix(h, w, v, span)?))
    }
    pub(crate) fn bi_nlinhas(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        self.expect_arity(args, 1, span)?;
        let m = self.expect_matriz(&args[0], span)?;
        Ok(Value::Numero((m.borrow().len() as i64).into()))
    }
    pub(crate) fn bi_ncolunas(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        self.expect_arity(args, 1, span)?;
        let m = self.expect_matriz(&args[0], span)?;
        let n = m.borrow().first().map(|r| r.len()).unwrap_or(0);
        Ok(Value::Numero((n as i64).into()))
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
        self.number_from_f64(d, span)
    }
    pub(crate) fn bi_matriz(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        self.expect_arity(args, 1, span)?;
        self.lista_para_matriz(&args[0], span)
    }
}
