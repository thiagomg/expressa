use crate::lexer::Span;
use crate::parser::block_types::Block;
use crate::parser::expression_types::Expr;

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    /// Expression used only for effect (e.g. escreva("Thiago") or any other call); value discarded.
    Expr {
        expr: Expr,
        span: Span,
    },

    /// `alvo = valor`
    Assign {
        target: AssignTarget,
        value: Expr,
        span: Span,
    },

    /// `repita n vezes inicio ... fim`
    Repita {
        count: Expr,
        body: Block,
        span: Span,
    },

    /// `para i de a ate b inicio ... fim`
    ParaRange {
        var: String,
        from: Expr,
        to: Expr,
        body: Block,
        span: Span,
    },

    /// `para x em lista inicio ... fim`
    ParaIn {
        var: String,
        iter: Expr,
        body: Block,
        span: Span,
    },
}

/// Left-hand side of `=`.
#[derive(Debug, Clone, PartialEq)]
pub enum AssignTarget {
    Name {
        name: String,
        span: Span,
    },
    /// `lista[i]` or `mapa[k]` (not a slice).
    Index {
        object: Expr,
        index: Expr,
        span: Span,
    },
}

impl Stmt {
    pub fn span(&self) -> Span {
        match self {
            Stmt::Expr { span, .. }
            | Stmt::Assign { span, .. }
            | Stmt::Repita { span, .. }
            | Stmt::ParaRange { span, .. }
            | Stmt::ParaIn { span, .. } => *span,
        }
    }
}

impl AssignTarget {
    pub fn span(&self) -> Span {
        match self {
            AssignTarget::Name { span, .. } | AssignTarget::Index { span, .. } => *span,
        }
    }
}
