use crate::lexer::Span;
use crate::parser::block_types::Block;
use crate::parser::operator_types::{BinaryOp, UnaryOp};

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    // ── Literals ───────────────────────────────────────────────
    Number {
        /// Raw lexeme, e.g. "1_000", "3.14" (Decimal later).
        raw: String,
        span: Span,
    },
    String {
        value: String, // already unescaped
        span: Span,
    },
    Bool {
        value: bool,
        span: Span,
    },
    List {
        elements: Vec<Expr>,
        span: Span,
    },
    /// `mapa {}` or `mapa inicio k -> v ... fim` or `mapa { k -> v }`
    Map {
        /// Empty if `mapa {}`
        entries: Vec<MapEntry>,
        span: Span,
    },
    /// `matriz { [1, 2], [3, 4] }` — each row is an expression (usually a list).
    Matrix {
        rows: Vec<Expr>,
        span: Span,
    },
    /// `funcao (a, b) inicio ... fim` or `{ ... }`
    Function {
        params: Vec<Param>,
        body: Block,
        span: Span,
    },

    // ── Names & access ─────────────────────────────────────────
    Ident {
        name: String,
        span: Span,
    },
    /// `mat::soma` — name inside a module.
    Field {
        object: Box<Expr>,
        field: String,
        span: Span,
    },
    /// `pessoa:nome` — identifier key of a mapa (`pessoa["nome"]`).
    MapField {
        object: Box<Expr>,
        field: String,
        span: Span,
    },
    /// `f(a, b)`
    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
        span: Span,
    },
    /// `a[i]`
    Index {
        object: Box<Expr>,
        index: Box<Expr>,
        span: Span,
    },
    /// `m[i, j]`
    Index2 {
        object: Box<Expr>,
        row: Box<Expr>,
        col: Box<Expr>,
        span: Span,
    },
    /// `a[i..j]`
    Slice {
        object: Box<Expr>,
        start: Box<Expr>,
        end: Box<Expr>,
        span: Span,
    },

    // ── Operators ──────────────────────────────────────────────
    Unary {
        op: UnaryOp,
        expr: Box<Expr>,
        span: Span,
    },
    Binary {
        left: Box<Expr>,
        op: BinaryOp,
        right: Box<Expr>,
        span: Span,
    },

    // ── Control as expression ──────────────────────────────────
    /// Block used as expression.
    Block(Block),

    /// `se cond inicio ... fim (ou se ...)* (senao inicio ... fim)?`
    If {
        branches: Vec<IfBranch>, // at least one (`se`)
        /// Required when this `If` is in value position (semantic check).
        else_block: Option<Block>,
        span: Span,
    },

    /// `A se_falhar B`
    SeFalhar {
        attempt: Box<Expr>,
        fallback: Box<Expr>,
        span: Span,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct MapEntry {
    pub key: Expr,
    pub value: Expr,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    pub name: String,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IfBranch {
    pub cond: Expr,
    pub body: Block,
    pub span: Span,
}

impl Expr {
    pub fn span(&self) -> Span {
        match self {
            Expr::Number { span, .. }
            | Expr::String { span, .. }
            | Expr::Bool { span, .. }
            | Expr::List { span, .. }
            | Expr::Map { span, .. }
            | Expr::Matrix { span, .. }
            | Expr::Function { span, .. }
            | Expr::Ident { span, .. }
            | Expr::Field { span, .. }
            | Expr::MapField { span, .. }
            | Expr::Call { span, .. }
            | Expr::Index { span, .. }
            | Expr::Index2 { span, .. }
            | Expr::Slice { span, .. }
            | Expr::Unary { span, .. }
            | Expr::Binary { span, .. }
            | Expr::If { span, .. }
            | Expr::SeFalhar { span, .. } => *span,
            Expr::Block(b) => b.span,
        }
    }
}
