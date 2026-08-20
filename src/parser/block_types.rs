use crate::lexer::Span;
use crate::parser::statement_types::Stmt;

/// `inicio` … `fim` — sequence of statements; value = last *expression* stmt,
/// or last evaluated expr-producing piece (define precisely when implementing).
///
/// Practical rule used by many languages:
/// body is `Vec<Stmt>`; the block value is the value of the last `Stmt::Expr`
/// (or of an assign if you treat assign as non-value — see note below).
#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    pub span: Span,
}
