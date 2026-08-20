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
    /// `mapa {}` or `mapa inicio k -> v ... fim`
    Map {
        /// Empty if `mapa {}`
        entries: Vec<MapEntry>,
        span: Span,
    },
    /// `funcao (a, b) inicio ... fim`
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
    /// `mat.soma` (module field; only valid if `mat` is module — runtime/check).
    Field {
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
            | Expr::Function { span, .. }
            | Expr::Ident { span, .. }
            | Expr::Field { span, .. }
            | Expr::Call { span, .. }
            | Expr::Index { span, .. }
            | Expr::Slice { span, .. }
            | Expr::Unary { span, .. }
            | Expr::Binary { span, .. }
            | Expr::If { span, .. }
            | Expr::SeFalhar { span, .. } => *span,
            Expr::Block(b) => b.span,
        }
    }
}
