//! Matrix operations retain f64 cells pending the separate numeric-model decision.
//! All language-produced cells must still be convertible back to Numero.
use super::*;

impl Vm<'_, '_> {
    pub(crate) fn check_matrix_shape(
        &self,
        h: usize,
        w: usize,
        span: Span,
    ) -> Result<(), EvalError> {
        let cells = h.checked_mul(w).filter(|n| *n <= self.max_matrix_cells);
        let bytes = cells.and_then(|n| n.checked_mul(std::mem::size_of::<f64>()));
        let row_bytes = h.checked_mul(std::mem::size_of::<Vec<f64>>());
        if bytes
            .and_then(|n| row_bytes.and_then(|r| n.checked_add(r)))
            .is_none_or(|n| n > isize::MAX as usize)
        {
            return Err(self.err(
                format!(
                    "matriz grande demais (limite: {} células)",
                    self.max_matrix_cells
                ),
                span,
            ));
        }
        Ok(())
    }

    pub(crate) fn allocate_matrix(
        &mut self,
        h: usize,
        w: usize,
        value: f64,
        span: Span,
    ) -> Result<Vec<Vec<f64>>, EvalError> {
        self.check_matrix_shape(h, w, span)?;
        self.matrix_number(value, span)?;
        self.check_espera(span)?;
        let failure = || self.err("memória insuficiente para matriz", span);
        let mut rows = Vec::new();
        rows.try_reserve_exact(h).map_err(|_| failure())?;
        for _ in 0..h {
            self.check_espera(span)?;
            let mut row = Vec::new();
            row.try_reserve_exact(w)
                .map_err(|_| self.err("memória insuficiente para matriz", span))?;
            // The reserve above is fallible; resize cannot allocate again.
            row.resize(w, value);
            rows.push(row);
        }
        Ok(rows)
    }

    fn matrix_number(&self, value: f64, span: Span) -> Result<f64, EvalError> {
        Numero::try_from(value)
            .map(|_| value)
            .map_err(|message| self.err(message, span))
    }

    pub(crate) fn index_get2(
        &self,
        obj: &Value,
        row: &Value,
        col: &Value,
        span: Span,
    ) -> Result<Value, EvalError> {
        let Value::Matriz(m) = obj else {
            return Err(self.err(
                format!(
                    "[i, j] só se aplica a matriz, encontrado {}",
                    obj.type_name()
                ),
                span,
            ));
        };
        let rows = m.borrow();
        let i = self.to_index(row, rows.len(), span)?;
        let j = self.to_index(col, rows[i].len(), span)?;
        self.number_from_f64(rows[i][j], span)
    }

    pub(crate) fn assign_index2(
        &self,
        obj: &Value,
        row: &Value,
        col: &Value,
        value: Value,
        span: Span,
    ) -> Result<(), EvalError> {
        let Value::Matriz(m) = obj else {
            return Err(self.err(
                format!(
                    "[i, j] só se aplica a matriz, encontrado {}",
                    obj.type_name()
                ),
                span,
            ));
        };
        let n = self.matrix_number(self.expect_f64(&value, span)?, span)?;
        let mut rows = m.borrow_mut();
        let i = self.to_index(row, rows.len(), span)?;
        let j = self.to_index(col, rows[i].len(), span)?;
        rows[i][j] = n;
        Ok(())
    }

    pub(crate) fn lista_para_matriz(
        &mut self,
        value: &Value,
        span: Span,
    ) -> Result<Value, EvalError> {
        let xs = self.expect_lista(value, span)?;
        let borrowed = xs.borrow();
        if borrowed.is_empty() {
            return Err(self.err("matriz precisa de pelo menos uma linha", span));
        }
        let width = self.expect_lista(&borrowed[0], span)?.borrow().len();
        if width == 0 {
            return Err(self.err("linha de matriz não pode ser vazia", span));
        }
        // Preflight dimensions before allocating output rows.
        for row in borrowed.iter() {
            self.check_espera(span)?;
            let cells = self.expect_lista(row, span)?;
            let len = cells.borrow().len();
            if len != width {
                return Err(self.err(
                    format!("linhas da matriz têm tamanhos diferentes ({width} e {len})"),
                    span,
                ));
            }
        }
        let mut grid = self.allocate_matrix(borrowed.len(), width, 0.0, span)?;
        for (i, row) in borrowed.iter().enumerate() {
            self.check_espera(span)?;
            let cells = self.expect_lista(row, span)?;
            for (j, item) in cells.borrow().iter().enumerate() {
                if j.is_multiple_of(1024) {
                    self.check_espera(span)?;
                }
                grid[i][j] = self.matrix_number(self.expect_f64(item, span)?, span)?;
            }
        }
        Ok(Value::matriz(grid))
    }

    pub(super) fn matriz_add(
        &mut self,
        l: &Value,
        r: &Value,
        span: Span,
        sign: f64,
    ) -> Result<Value, EvalError> {
        let (Value::Matriz(a), Value::Matriz(b)) = (l, r) else {
            unreachable!()
        };
        let a = a.borrow();
        let b = b.borrow();
        if a.len() != b.len() || a.first().map(|r| r.len()) != b.first().map(|r| r.len()) {
            return Err(self.err("matrizes de tamanhos diferentes", span));
        }
        let width = a.first().map_or(0, Vec::len);
        let mut out = self.allocate_matrix(a.len(), width, 0.0, span)?;
        for (i, (ra, rb)) in a.iter().zip(b.iter()).enumerate() {
            self.check_espera(span)?;
            for (j, (x, y)) in ra.iter().zip(rb.iter()).enumerate() {
                if j.is_multiple_of(1024) {
                    self.check_espera(span)?;
                }
                out[i][j] = self.matrix_number(x + sign * y, span)?;
            }
        }
        Ok(Value::matriz(out))
    }

    pub(super) fn matriz_scale(
        &mut self,
        l: &Value,
        r: &Value,
        k: f64,
        span: Span,
    ) -> Result<Value, EvalError> {
        let m = match (l, r) {
            (Value::Matriz(m), _) | (_, Value::Matriz(m)) => m,
            _ => unreachable!(),
        };
        let m = m.borrow();
        let mut out = self.allocate_matrix(m.len(), m.first().map_or(0, Vec::len), 0.0, span)?;
        for (i, row) in m.iter().enumerate() {
            self.check_espera(span)?;
            for (j, value) in row.iter().enumerate() {
                if j.is_multiple_of(1024) {
                    self.check_espera(span)?;
                }
                out[i][j] = self.matrix_number(value * k, span)?;
            }
        }
        Ok(Value::matriz(out))
    }

    pub(super) fn matriz_mul(
        &mut self,
        l: &Value,
        r: &Value,
        span: Span,
    ) -> Result<Value, EvalError> {
        let (Value::Matriz(a), Value::Matriz(b)) = (l, r) else {
            unreachable!()
        };
        let a = a.borrow();
        let b = b.borrow();
        let n = a.len();
        let p = a.first().map_or(0, Vec::len);
        let q = b.first().map_or(0, Vec::len);
        if p != b.len() {
            return Err(self.err(
                format!(
                    "produto de matrizes exige colunas da esquerda = linhas da direita ({p} ≠ {})",
                    b.len()
                ),
                span,
            ));
        }
        let mut out = self.allocate_matrix(n, q, 0.0, span)?;
        let mut work = 0usize;
        for i in 0..n {
            self.check_espera(span)?;
            for j in 0..q {
                let mut sum = 0.0;
                for k in 0..p {
                    if work.is_multiple_of(1024) {
                        self.check_espera(span)?;
                    }
                    work = work.wrapping_add(1);
                    sum += a[i][k] * b[k][j];
                }
                out[i][j] = self.matrix_number(sum, span)?;
            }
        }
        Ok(Value::matriz(out))
    }
}
