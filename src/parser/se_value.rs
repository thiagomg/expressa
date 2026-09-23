//! `se` used as a value needs `senao` (statement-`se` may omit it).

use crate::lexer::Span;
use crate::parser::block_types::Block;
use crate::parser::core_types::{Item, Program};
use crate::parser::error::ParseError;
use crate::parser::expression_types::Expr;
use crate::parser::statement_types::{AssignTarget, Stmt};

#[derive(Clone, Copy)]
enum Use {
    Value,
    Effect,
}

pub fn check_se_value(program: &Program) -> Result<(), ParseError> {
    for item in &program.items {
        match item {
            Item::Import(_) => {}
            Item::Stmt(stmt) => check_stmt(stmt, Use::Effect)?,
        }
    }
    Ok(())
}

fn falta_senao(span: Span) -> ParseError {
    ParseError {
        message: "se usado como valor precisa de 'senao'".into(),
        span,
    }
}

fn check_stmt(stmt: &Stmt, use_: Use) -> Result<(), ParseError> {
    match stmt {
        Stmt::Expr { expr, .. } => check_expr(expr, use_),
        Stmt::Assign { target, value, .. } => {
            check_target(target)?;
            check_expr(value, Use::Value)
        }
        Stmt::Repita { count, body, .. } => {
            check_expr(count, Use::Value)?;
            check_block(body, Use::Effect)
        }
        Stmt::ParaRange { from, to, body, .. } => {
            check_expr(from, Use::Value)?;
            check_expr(to, Use::Value)?;
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
    }
}

fn check_block(block: &Block, use_: Use) -> Result<(), ParseError> {
    let n = block.stmts.len();
    for (i, stmt) in block.stmts.iter().enumerate() {
        let u = if matches!(use_, Use::Value) && i + 1 == n {
            Use::Value
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

fn check_expr(expr: &Expr, use_: Use) -> Result<(), ParseError> {
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
        // Function body is a statement list (last value is a convenience).
        // A trailing `se` without `senao` is a command, not `x = se …`.
        Expr::Function { body, .. } => check_block(body, Use::Effect),
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
            check_expr(fallback, Use::Value)
        }
        Expr::List { elements, .. } => {
            for e in elements {
                check_expr(e, Use::Value)?;
            }
            Ok(())
        }
        Expr::Map { entries, .. } => {
            for e in entries {
                check_expr(&e.key, Use::Value)?;
                check_expr(&e.value, Use::Value)?;
            }
            Ok(())
        }
        Expr::Matrix { rows, .. } => {
            for r in rows {
                check_expr(r, Use::Value)?;
            }
            Ok(())
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
        Expr::Slice {
            object, start, end, ..
        } => {
            check_expr(object, Use::Value)?;
            check_expr(start, Use::Value)?;
            check_expr(end, Use::Value)
        }
        Expr::Number { .. } | Expr::String { .. } | Expr::Bool { .. } | Expr::Ident { .. } => {
            Ok(())
        }
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
        verdadeiro
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
        let err = parse("x = { se verdadeiro { 1 } }").unwrap_err();
        assert!(err.message.contains("senao"), "{err}");
        parse(r#"x = se verdadeiro { 1 } senao { 2 }"#).unwrap();
        parse("f = funcao() { se verdadeiro { 1 } senao { 2 } }").unwrap();
    }
}
