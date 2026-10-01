use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

use crate::lexer::Span;

use super::super::error::EvalError;
use super::super::eval::Vm;
use super::super::value::Value;
use super::{csv_cell, lines_to_text};

impl Vm<'_> {
    pub(crate) fn bi_leia_arquivo(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        self.expect_arity(args, 1, span)?;
        let path = self.resolve_data_path(&self.expect_texto(&args[0], span)?, span)?;
        let contents = fs::read_to_string(&path).map_err(|e| {
            self.err(
                format!("não foi possível ler '{}': {e}", path.display()),
                span,
            )
        })?;
        let lines = contents
            .lines()
            .map(|l| Value::Texto(l.to_string()))
            .collect();
        Ok(Value::lista(lines))
    }
    pub(crate) fn bi_salve_arquivo(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        self.expect_arity(args, 2, span)?;
        let path = self.resolve_data_path(&self.expect_texto(&args[0], span)?, span)?;
        let linhas = self.expect_lista(&args[1], span)?;
        let body = lines_to_text(&linhas.borrow(), span, self)?;
        self.ensure_parent_dir(&path, span)?;
        fs::write(&path, body).map_err(|e| {
            self.err(
                format!("não foi possível salvar '{}': {e}", path.display()),
                span,
            )
        })?;
        Ok(Value::Nada)
    }
    pub(crate) fn bi_adicione_arquivo(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        self.expect_arity(args, 2, span)?;
        let path = self.resolve_data_path(&self.expect_texto(&args[0], span)?, span)?;
        let linhas = self.expect_lista(&args[1], span)?;
        let extra = lines_to_text(&linhas.borrow(), span, self)?;
        self.ensure_parent_dir(&path, span)?;
        let mut f = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|e| {
                self.err(
                    format!("não foi possível abrir '{}': {e}", path.display()),
                    span,
                )
            })?;
        f.write_all(extra.as_bytes())
            .map_err(|e| self.io_err(e, span))?;
        Ok(Value::Nada)
    }
    pub(crate) fn bi_leia_csv(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        self.expect_arity(args, 1, span)?;
        let path = self.resolve_data_path(&self.expect_texto(&args[0], span)?, span)?;
        let contents = fs::read_to_string(&path).map_err(|e| {
            self.err(
                format!("não foi possível ler '{}': {e}", path.display()),
                span,
            )
        })?;
        let rows = contents
            .lines()
            .map(|line| {
                Value::lista(
                    line.split(',')
                        .map(|cell| Value::Texto(cell.to_string()))
                        .collect(),
                )
            })
            .collect();
        Ok(Value::lista(rows))
    }
    pub(crate) fn bi_salve_csv(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        self.expect_arity(args, 2, span)?;
        let path = self.resolve_data_path(&self.expect_texto(&args[0], span)?, span)?;
        let rows = self.expect_lista(&args[1], span)?;
        let mut out = String::new();
        for row in rows.borrow().iter() {
            let cells = self.expect_lista(row, span)?;
            let line: Vec<String> = cells.borrow().iter().map(csv_cell).collect();
            out.push_str(&line.join(","));
            out.push('\n');
        }
        self.ensure_parent_dir(&path, span)?;
        fs::write(&path, out).map_err(|e| {
            self.err(
                format!("não foi possível salvar '{}': {e}", path.display()),
                span,
            )
        })?;
        Ok(Value::Nada)
    }
    pub(crate) fn resolve_data_path(
        &self,
        path: &str,
        span: Span,
    ) -> Result<std::path::PathBuf, EvalError> {
        let p = Path::new(path);
        let joined = if p.is_absolute() {
            p.to_path_buf()
        } else {
            self.base_dir.join(p)
        };
        self.confine(joined, span)
    }
    pub(crate) fn ensure_parent_dir(&self, path: &Path, span: Span) -> Result<(), EvalError> {
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            fs::create_dir_all(parent).map_err(|e| self.io_err(e, span))?;
        }
        Ok(())
    }
}
