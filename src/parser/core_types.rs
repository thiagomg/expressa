use crate::lexer::Span;
use crate::parser::statement_types::Stmt;

/// Whole file.
#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub items: Vec<Item>,
    pub span: Span,
}

/// Top-level only (and what a block body mixes with stmts).
#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    /// `mat = importe "x"` or `importe "x"`
    Import(Import),
    Stmt(Stmt),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Import {
    /// `None` = bring names into current scope; `Some(name)` = namespace binding.
    pub alias: Option<String>,
    pub path: String, // string literal contents, e.g. "lib/math"
    pub span: Span,
}
