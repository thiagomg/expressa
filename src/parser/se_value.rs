//! Checks on how each expression's value is used:
//! - `se` used directly as a value (`x = se …`, an argument, `retorne se …`)
//!   needs `senao`. A `se` on the last line of a block is a command: without
//!   `senao` and without a branch taken, the block's value is `nada`.
//! - A value computed and thrown away with no effect (`5`, `-3`, `x + 1` on a
//!   line of its own, not the last of a block) is an error.

use crate::lexer::Span;
use crate::parser::block_types::Block;
use crate::parser::core_types::{Item, Program};
use crate::parser::error::{ParseError, ParseErrorKind};
use crate::parser::expression_types::Expr;
use crate::parser::statement_types::{AssignTarget, Stmt};
use crate::runtime::parse_numero;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Use {
    /// Direct value position: `se` needs `senao`.
    Value,
    /// Last line of a block whose value is used: `se` may omit `senao`.
    Tail,
    /// Value thrown away: the expression must have an effect.
    Effect,
}

pub fn check_se_value(program: &Program) -> Result<(), ParseError> {
    let n = program.items.len();
    for (i, item) in program.items.iter().enumerate() {
        match item {
            Item::Import(_) => {}
            Item::Stmt(stmt) => {
                let u = if i + 1 == n { Use::Tail } else { Use::Effect };
                check_stmt(stmt, u)?
            }
        }
    }
    Ok(())
}

fn valor_descartado(expr: &Expr) -> ParseError {
    let message = if matches!(
        expr,
        Expr::Unary {
            op: crate::parser::operator_types::UnaryOp::Neg,
            ..
        }
    ) {
        "este valor é calculado e não é usado; um '-' no começo da linha não continua \
         a linha anterior (use parênteses para quebrar uma conta em várias linhas)"
    } else {
        "este valor é calculado e não é usado; guarde-o em uma variável ou remova a linha"
    };
    ParseError {
        kind: ParseErrorKind::Syntax,
        message: message.into(),
        span: expr.span(),
    }
}

/// No call, no block, no `se`: evaluating it can only produce a value
/// (or an error), so discarding it is a mistake.
fn sem_efeito(expr: &Expr) -> bool {
    match expr {
        Expr::Number { .. }
        | Expr::String { .. }
        | Expr::Nada { .. }
        | Expr::Bool { .. }
        | Expr::Ident { .. }
        | Expr::Function { .. } => true,
        Expr::List { elements, .. } => elements.iter().all(sem_efeito),
        Expr::Par { key, value, .. } => sem_efeito(key) && sem_efeito(value),
        Expr::Field { object, .. } | Expr::MapField { object, .. } => sem_efeito(object),
        Expr::Index { object, index, .. } => sem_efeito(object) && sem_efeito(index),
        Expr::Index2 {
            object, row, col, ..
        } => sem_efeito(object) && sem_efeito(row) && sem_efeito(col),
        Expr::Intervalo { start, end, .. } => {
            sem_efeito(start) && end.as_deref().is_none_or(sem_efeito)
        }
        Expr::Unary { expr, .. } => sem_efeito(expr),
        Expr::Binary { left, right, .. } => sem_efeito(left) && sem_efeito(right),
        Expr::Call { .. } | Expr::Block(_) | Expr::If { .. } | Expr::SeFalhar { .. } => false,
    }
}

fn falta_senao(span: Span) -> ParseError {
    ParseError {
        kind: ParseErrorKind::MissingElse,
        message: "se usado como valor precisa de 'senao'".into(),
        span,
    }
}

fn check_stmt(stmt: &Stmt, use_: Use) -> Result<(), ParseError> {
    match stmt {
        Stmt::Expr { expr, .. } => {
            if use_ == Use::Effect && sem_efeito(expr) {
                return Err(valor_descartado(expr));
            }
            check_expr(expr, use_)
        }
        Stmt::Assign { target, value, .. } => {
            check_target(target)?;
            check_expr(value, Use::Value)
        }
        Stmt::Repita { count, body, .. } => {
            check_expr(count, Use::Value)?;
            check_block(body, Use::Effect)
        }
        Stmt::ParaIn { iter, body, .. } => {
            check_expr(iter, Use::Value)?;
            check_block(body, Use::Effect)
        }
        Stmt::Enquanto { cond, body, .. } => {
            check_expr(cond, Use::Value)?;
            check_block(body, Use::Effect)
        }
        Stmt::Retorne {
            value: Some(expr), ..
        } => check_expr(expr, Use::Value),
        Stmt::Retorne { value: None, .. } => Ok(()),
        Stmt::Pare { .. } | Stmt::Continue { .. } => Ok(()),
    }
}

fn check_block(block: &Block, use_: Use) -> Result<(), ParseError> {
    let n = block.stmts.len();
    for (i, stmt) in block.stmts.iter().enumerate() {
        let u = if use_ != Use::Effect && i + 1 == n {
            Use::Tail
        } else {
            Use::Effect
        };
        check_stmt(stmt, u)?;
    }
    Ok(())
}

fn check_target(target: &AssignTarget) -> Result<(), ParseError> {
    match target {
        AssignTarget::Name { .. } => Ok(()),
        AssignTarget::Index { object, index, .. } => {
            check_expr(object, Use::Value)?;
            check_expr(index, Use::Value)
        }
        AssignTarget::Index2 {
            object, row, col, ..
        } => {
            check_expr(object, Use::Value)?;
            check_expr(row, Use::Value)?;
            check_expr(col, Use::Value)
        }
        AssignTarget::MapField { object, .. } => check_expr(object, Use::Value),
    }
}

/// Recursive like the tree it walks, which can be
/// `parser::PROFUNDIDADE_MAXIMA` deep: grow the stack instead of overflowing.
fn check_expr(expr: &Expr, use_: Use) -> Result<(), ParseError> {
    stacker::maybe_grow(128 * 1024, 2 * 1024 * 1024, || check_expr_inner(expr, use_))
}

fn check_expr_inner(expr: &Expr, use_: Use) -> Result<(), ParseError> {
    match expr {
        Expr::If {
            branches,
            else_block,
            span,
        } => {
            if matches!(use_, Use::Value) && else_block.is_none() {
                return Err(falta_senao(*span));
            }
            for b in branches {
                check_expr(&b.cond, Use::Value)?;
                check_block(&b.body, use_)?;
            }
            if let Some(body) = else_block {
                check_block(body, use_)?;
            }
            Ok(())
        }
        // The last line of the body is the result; a `se` there may omit `senao`.
        Expr::Function { body, .. } => check_block(body, Use::Tail),
        Expr::Block(block) => check_block(block, use_),
        Expr::Call { callee, args, .. } => {
            check_expr(callee, Use::Value)?;
            for a in args {
                check_expr(a, Use::Value)?;
            }
            Ok(())
        }
        Expr::Unary { expr, .. } => check_expr(expr, Use::Value),
        Expr::Binary { left, right, .. } => {
            check_expr(left, Use::Value)?;
            check_expr(right, Use::Value)
        }
        Expr::SeFalhar {
            attempt, fallback, ..
        } => {
            check_expr(attempt, Use::Value)?;
            check_block(fallback, Use::Value)
        }
        Expr::List { elements, .. } => {
            for e in elements {
                check_expr(e, Use::Value)?;
            }
            Ok(())
        }
        Expr::Par { key, value, .. } => {
            check_expr(key, Use::Value)?;
            check_expr(value, Use::Value)
        }
        Expr::Field { object, .. } | Expr::MapField { object, .. } => {
            check_expr(object, Use::Value)
        }
        Expr::Index { object, index, .. } => {
            check_expr(object, Use::Value)?;
            check_expr(index, Use::Value)
        }
        Expr::Index2 {
            object, row, col, ..
        } => {
            check_expr(object, Use::Value)?;
            check_expr(row, Use::Value)?;
            check_expr(col, Use::Value)
        }
        Expr::Intervalo { start, end, .. } => {
            check_expr(start, Use::Value)?;
            if let Some(end) = end {
                check_expr(end, Use::Value)?;
            }
            Ok(())
        }
        Expr::Number { raw, span } => {
            if parse_numero(raw).is_none() {
                return Err(ParseError {
                    kind: ParseErrorKind::Syntax,
                    message: format!("número inválido: {raw}"),
                    span: *span,
                });
            }
            Ok(())
        }
        Expr::Nada { .. } | Expr::String { .. } | Expr::Bool { .. } | Expr::Ident { .. } => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use crate::parser::parse;

    #[test]
    fn statement_se_may_omit_senao() {
        parse("se verdadeiro { escreva(1) }").unwrap();
        parse(
            r#"
para x em [1]
inicio
    se x == 1
    inicio
        escreva(verdadeiro)
    fim
fim
"#,
        )
        .unwrap();
    }

    #[test]
    fn value_se_needs_senao() {
        let err = parse(r#"x = se verdadeiro { 1 }"#).unwrap_err();
        assert!(err.message.contains("senao"), "{err}");
        let err = parse(r#"escreva(se verdadeiro { 1 })"#).unwrap_err();
        assert!(err.message.contains("senao"), "{err}");
        parse("f = funcao() { se verdadeiro { 1 } }").unwrap();
        // Last line of any block: a command, value `nada` without a branch.
        parse("x = { se verdadeiro { 1 } }").unwrap();
        parse("x = 1 se_falhar { se verdadeiro { 2 } }").unwrap();
        let err = parse("retorne_x = funcao() { retorne se verdadeiro { 1 } }").unwrap_err();
        assert!(err.message.contains("senao"), "{err}");
        parse(r#"x = se verdadeiro { 1 } senao { 2 }"#).unwrap();
        parse("f = funcao() { se verdadeiro { 1 } senao { 2 } }").unwrap();
    }
}

#[cfg(test)]
mod descarte_tests {
    use crate::parser::parse;

    fn descartado(src: &str) {
        let err = parse(src).unwrap_err();
        assert!(err.message.contains("não é usado"), "{src}: {err}");
    }

    #[test]
    fn valor_sem_efeito_descartado_e_erro() {
        descartado("total = 10\n- 3\nescreva(total)");
        descartado("5\nescreva(1)");
        descartado("x = 1\nx + 1\nescreva(x)");
        descartado(":a\nescreva(1)");
        descartado("para i em 0..3 { i * 2 }");
        descartado("f = funcao() {\n    1\n    2\n}");
        descartado("se verdadeiro { 1 }\nescreva(2)");
    }

    #[test]
    fn valor_usado_ou_com_efeito_e_aceito() {
        parse("x = 1\nx + 1").unwrap(); // última linha do programa
        parse("f = funcao(x) { -x }").unwrap();
        parse("escreva(1)\nescreva(2)").unwrap();
        parse("[escreva(1)]\nescreva(2)").unwrap();
        parse("f = funcao(n) { se n > 0 { \"pos\" } }").unwrap();
        parse("r = { escreva(1)\n se falso { 2 } }").unwrap();
    }
}
