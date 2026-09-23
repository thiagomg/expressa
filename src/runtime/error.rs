use std::fmt;

use crate::lexer::Span;

use super::value::Value;

#[derive(Debug, Clone)]
pub struct CallFrame {
    pub name: String,
    pub file: String,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct RuntimeError {
    pub message: String,
    pub file: String,
    pub span: Span,
    pub stack: Vec<CallFrame>,
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "erro: {} em {}",
            self.message,
            self.span.location_in(&self.file)
        )?;
        if !self.stack.is_empty() {
            writeln!(f)?;
            write!(f, "pilha de chamadas:")?;
            for (i, frame) in self.stack.iter().rev().enumerate() {
                write!(
                    f,
                    "\n  {i}: {} ({})",
                    frame.name,
                    frame.span.location_in(&frame.file)
                )?;
            }
        }
        Ok(())
    }
}

impl std::error::Error for RuntimeError {}

#[derive(Debug)]
pub(crate) enum EvalError {
    Runtime(RuntimeError),
    /// `sair(codigo)` or the debugger's `terminar`. Not caught by `se_falhar`.
    Quit(i32),
    /// `retorne` inside a `funcao`. Not caught by `se_falhar`.
    Return {
        value: Value,
        span: Span,
    },
}

impl From<RuntimeError> for EvalError {
    fn from(e: RuntimeError) -> Self {
        EvalError::Runtime(e)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn span(line: u32, col: u32) -> Span {
        Span::new(line, col, 0, 1)
    }

    #[test]
    fn display_includes_file_location() {
        let err = RuntimeError {
            message: "divisão por zero".into(),
            file: "media.lep".into(),
            span: span(12, 11),
            stack: vec![],
        };
        assert_eq!(err.to_string(), "erro: divisão por zero em media.lep:12:11");
    }

    #[test]
    fn display_prints_call_stack_innermost_first() {
        let err = RuntimeError {
            message: "índice 99 fora do intervalo".into(),
            file: "app.lep".into(),
            span: span(4, 1),
            stack: vec![
                CallFrame {
                    name: "<modulo>".into(),
                    file: "app.lep".into(),
                    span: span(1, 1),
                },
                CallFrame {
                    name: "pega".into(),
                    file: "app.lep".into(),
                    span: span(4, 1),
                },
            ],
        };
        let text = err.to_string();
        assert!(text.starts_with("erro: índice 99 fora do intervalo em app.lep:4:1"));
        assert!(text.contains("0: pega (app.lep:4:1)"));
        assert!(text.contains("1: <modulo> (app.lep:1:1)"));
        let pega = text.find("pega").unwrap();
        let modulo = text.find("<modulo>").unwrap();
        assert!(pega < modulo, "innermost frame should be listed first");
    }

    #[test]
    fn runtime_error_converts_into_eval_error() {
        let err = RuntimeError {
            message: "x".into(),
            file: "t.lep".into(),
            span: span(1, 1),
            stack: vec![],
        };
        assert!(matches!(EvalError::from(err), EvalError::Runtime(_)));
    }
}
