//! Recursive-descent / Pratt parser: tokens → AST.

use crate::lexer::scanner::{Token, tokenize};
use crate::lexer::{Span, TokenKind};
use crate::parser::block_types::Block;
use crate::parser::core_types::{Import, Item, Program};
use crate::parser::error::ParseError;
use crate::parser::expression_types::{Expr, IfBranch, MapEntry, Param};
use crate::parser::operator_types::{BinaryOp, UnaryOp};
use crate::parser::statement_types::{AssignTarget, Stmt};

/// Parse Expressa source into a [`Program`] AST.
pub fn parse(source: &str) -> Result<Program, ParseError> {
    let tokens = tokenize(source)?;
    let mut parser = Parser::new(tokens);
    parser.parse_program()
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    // ── token helpers ────────────────────────────────────────────

    fn peek(&self) -> &Token {
        &self.tokens[self.pos]
    }

    fn peek_kind(&self) -> &TokenKind {
        &self.peek().kind
    }

    fn at_eof(&self) -> bool {
        matches!(self.peek_kind(), TokenKind::Eof)
    }

    fn remove(&mut self) -> Token {
        let tok = self.tokens[self.pos].clone();
        if !matches!(tok.kind, TokenKind::Eof) {
            self.pos += 1;
        }
        tok
    }

    fn expect_kind<F>(&mut self, pred: F, msg: &str) -> Result<Token, ParseError>
    where
        F: FnOnce(&TokenKind) -> bool,
    {
        if pred(self.peek_kind()) {
            Ok(self.remove())
        } else {
            Err(self.err(msg))
        }
    }

    fn err(&self, message: impl Into<String>) -> ParseError {
        ParseError {
            message: message.into(),
            span: self.peek().span,
        }
    }

    fn err_at(&self, message: impl Into<String>, span: Span) -> ParseError {
        ParseError {
            message: message.into(),
            span,
        }
    }

    // ── program ──────────────────────────────────────────────────

    fn parse_program(&mut self) -> Result<Program, ParseError> {
        let start = self.peek().span;
        let mut items = Vec::new();

        while !self.at_eof() {
            items.push(self.parse_item()?);
        }

        let span = if let (Some(first), Some(last)) = (items.first(), items.last()) {
            first.span().join(last.span())
        } else {
            start
        };

        Ok(Program { items, span })
    }

    fn parse_item(&mut self) -> Result<Item, ParseError> {
        if matches!(self.peek_kind(), TokenKind::Importe) {
            return Ok(Item::Import(self.parse_import(None)?));
        }

        // `nome = importe "path"`
        if matches!(self.peek_kind(), TokenKind::Ident(_))
            && self.pos + 2 < self.tokens.len()
            && matches!(self.tokens[self.pos + 1].kind, TokenKind::Eq)
            && matches!(self.tokens[self.pos + 2].kind, TokenKind::Importe)
        {
            let name_tok = self.remove();
            let TokenKind::Ident(name) = name_tok.kind else {
                unreachable!();
            };
            self.remove(); // =
            return Ok(Item::Import(self.parse_import(Some(name))?));
        }

        Ok(Item::Stmt(self.parse_stmt()?))
    }

    fn parse_import(&mut self, alias: Option<String>) -> Result<Import, ParseError> {
        let start = self
            .expect_kind(|k| matches!(k, TokenKind::Importe), "esperado 'importe'")?
            .span;
        let path_tok = self.remove();
        match path_tok.kind {
            TokenKind::String(path) => Ok(Import {
                alias,
                path,
                span: start.join(path_tok.span),
            }),
            _ => Err(self.err_at("esperado caminho string após 'importe'", path_tok.span)),
        }
    }

    // ── statements ───────────────────────────────────────────────

    fn parse_stmt(&mut self) -> Result<Stmt, ParseError> {
        match self.peek_kind() {
            TokenKind::Repita => self.parse_repita(),
            TokenKind::Para => self.parse_para(),
            _ => {
                let expr = self.parse_expr()?;
                if matches!(self.peek_kind(), TokenKind::Eq) {
                    let target = expr_to_assign_target(expr)
                        .map_err(|span| self.err_at("lado esquerdo de '=' inválido", span))?;
                    self.remove(); // =
                    let value = self.parse_expr()?;
                    let span = target.span().join(value.span());
                    Ok(Stmt::Assign {
                        target,
                        value,
                        span,
                    })
                } else {
                    let span = expr.span();
                    Ok(Stmt::Expr { expr, span })
                }
            }
        }
    }

    fn parse_repita(&mut self) -> Result<Stmt, ParseError> {
        let start = self.remove().span; // repita
        let count = self.parse_expr()?;
        self.expect_kind(
            |k| matches!(k, TokenKind::Vezes),
            "esperado 'vezes' após contagem do 'repita'",
        )?;
        let body = self.parse_block()?;
        Ok(Stmt::Repita {
            count,
            span: start.join(body.span),
            body,
        })
    }

    fn parse_para(&mut self) -> Result<Stmt, ParseError> {
        let start = self.remove().span; // para
        let var_tok = self.remove();
        let TokenKind::Ident(var) = var_tok.kind else {
            return Err(self.err_at("esperado nome da variável do 'para'", var_tok.span));
        };

        match self.peek_kind() {
            TokenKind::De => {
                self.remove();
                let from = self.parse_expr()?;
                self.expect_kind(|k| matches!(k, TokenKind::Ate), "esperado 'ate' no 'para'")?;
                let to = self.parse_expr()?;
                let body = self.parse_block()?;
                Ok(Stmt::ParaRange {
                    var,
                    from,
                    to,
                    span: start.join(body.span),
                    body,
                })
            }
            TokenKind::Em => {
                self.remove();
                let iter = self.parse_expr()?;
                let body = self.parse_block()?;
                Ok(Stmt::ParaIn {
                    var,
                    iter,
                    span: start.join(body.span),
                    body,
                })
            }
            _ => Err(self.err("esperado 'de' ou 'em' após variável do 'para'")),
        }
    }

    fn parse_block(&mut self) -> Result<Block, ParseError> {
        let open = self.remove();
        let brace = match open.kind {
            TokenKind::Inicio => false,
            TokenKind::LBrace => true,
            _ => {
                return Err(self.err_at("esperado 'inicio' ou '{'", open.span));
            }
        };
        let mut stmts = Vec::new();
        loop {
            match self.peek_kind() {
                TokenKind::Eof => break,
                TokenKind::Fim if !brace => break,
                TokenKind::RBrace if brace => break,
                TokenKind::Fim if brace => {
                    return Err(self.err("esperado '}' (bloco aberto com '{')"));
                }
                TokenKind::RBrace if !brace => {
                    return Err(self.err("esperado 'fim' (bloco aberto com 'inicio')"));
                }
                _ => stmts.push(self.parse_stmt()?),
            }
        }
        let end = if brace {
            self.expect_kind(|k| matches!(k, TokenKind::RBrace), "esperado '}'")?
        } else {
            self.expect_kind(|k| matches!(k, TokenKind::Fim), "esperado 'fim'")?
        };
        if stmts.is_empty() {
            return Err(self.err_at("bloco não pode ser vazio", open.span.join(end.span)));
        }
        Ok(Block {
            stmts,
            span: open.span.join(end.span),
        })
    }

    // ── expressions (Pratt / "binding power" parser) ─────────────
    //
    // Goal: parse `1 + 2 * 3` as `1 + (2 * 3)` without a separate
    // function per precedence level (parse_or, parse_and, parse_add, …).
    //
    // Each infix operator has a *binding power* (bp): higher = tighter.
    //   se_falhar < ou < e < comparisons/contem < + - < * / %
    //
    // `parse_bp(min_bp)` means:
    //   "Build an expression, but stop before any infix whose left-bp
    //    is *weaker* than min_bp."
    //
    // Top-level call uses min_bp = 0 (accept every infix).
    // When we take `+` (left_bp 9), we parse the RHS with min_bp = 10,
    // so `*` (11) is still allowed on the right, but another `+` (9)
    // is not — that second `+` is left for the outer call.
    //
    // Example: 1 + 2 * 3
    //   parse_bp(0): lhs=1, see +, RHS=parse_bp(10)
    //     parse_bp(10): lhs=2, see * (11>=10), RHS=3 → (2*3)
    //   result: 1 + (2*3)

    fn parse_expr(&mut self) -> Result<Expr, ParseError> {
        // 0 = “no restriction”: may consume any infix operator.
        self.parse_bp(0)
    }

    /// Parse an expression that only continues through infix ops whose
    /// **left** binding power is `>= min_bp`.
    ///
    /// See module comments above and [`infix_info`] for the two bp numbers.
    fn parse_bp(&mut self, min_bp: u8) -> Result<Expr, ParseError> {
        // --- left-hand side: atom / unary / already-built subexpr ---
        // Not an infix yet. Examples: 10, x, (a+b), -3, nao ok, se …, funcao …
        let mut lhs = self.parse_unary()?;

        loop {
            // --- infix operator? ---
            // Returns (operator token kind, left_bp, right_bp).
            // If the next token is not an infix (e.g. `)`, `fim`, Eof), stop.
            let (op_kind, l_bp, r_bp) = match infix_info(self.peek_kind()) {
                Some(info) => info,
                None => break,
            };

            // Gate: operator too weak for this nesting level → let the
            // *caller* handle it. Example: inside RHS of `+` we used
            // min_bp=10; a following `+` has l_bp=9 < 10, so we break
            // and the outer parse_bp takes that second `+`.
            if l_bp < min_bp {
                break;
            }

            // Spec: no `a < b < c` (would be confusing with bool vs number).
            if is_comparison_token(&op_kind) {
                if let Expr::Binary { op, span, .. } = &lhs {
                    if is_comparison_op(*op) {
                        return Err(self.err_at(
                            "comparações não podem ser encadeadas; use 'e' (ex.: a < b e b < c)",
                            *span,
                        ));
                    }
                }
            }

            // Commit to this operator, then parse its right-hand side.
            // We pass *right_bp* as the new min_bp so the RHS only
            // absorbs *tighter* operators (see infix_info docs).
            self.remove();
            let rhs = self.parse_bp(r_bp)?;
            let span = lhs.span().join(rhs.span());

            // Fold: lhs becomes (lhs OP rhs), then loop may take another op.
            // e.g. 1 - 2 - 3 → first fold (1-2), then fold ((1-2)-3).
            lhs = if matches!(op_kind, TokenKind::SeFalhar) {
                Expr::SeFalhar {
                    attempt: Box::new(lhs),
                    fallback: Box::new(rhs),
                    span,
                }
            } else {
                let op = token_to_binary(&op_kind).expect("infix op");
                Expr::Binary {
                    left: Box::new(lhs),
                    op,
                    right: Box::new(rhs),
                    span,
                }
            };
        }

        Ok(lhs)
    }

    /// Prefix operators and the primary underneath.
    ///
    /// Call / index / field bind *tighter* than prefix, so
    /// `nao f(x)` is `nao (f(x))` and `-xs[1]` is `-(xs[1])`.
    /// Unary still binds tighter than infix: `nao a e b` → `(nao a) e b`.
    fn parse_unary(&mut self) -> Result<Expr, ParseError> {
        match self.peek_kind() {
            TokenKind::Minus => {
                let start = self.remove().span;
                let expr = self.parse_unary()?;
                let span = start.join(expr.span());
                Ok(Expr::Unary {
                    op: UnaryOp::Neg,
                    expr: Box::new(expr),
                    span,
                })
            }
            TokenKind::Nao => {
                let start = self.remove().span;
                let expr = self.parse_unary()?;
                let span = start.join(expr.span());
                Ok(Expr::Unary {
                    op: UnaryOp::Not,
                    expr: Box::new(expr),
                    span,
                })
            }
            _ => self.parse_postfix(),
        }
    }

    /// `f(a)[i].campo` — postfix attaches to the primary, not to a prefix op.
    fn parse_postfix(&mut self) -> Result<Expr, ParseError> {
        let mut lhs = self.parse_primary()?;
        loop {
            match self.peek_kind() {
                TokenKind::LParen => lhs = self.finish_call(lhs)?,
                TokenKind::LBracket => lhs = self.finish_index_or_slice(lhs)?,
                TokenKind::ColonColon => lhs = self.finish_module_field(lhs)?,
                TokenKind::Colon => lhs = self.finish_map_field(lhs)?,
                TokenKind::Dot => lhs = self.finish_ufcs(lhs)?,
                _ => break,
            }
        }
        Ok(lhs)
    }

    fn parse_primary(&mut self) -> Result<Expr, ParseError> {
        let tok = self.remove();
        match tok.kind {
            TokenKind::Number(raw) => Ok(Expr::Number {
                raw,
                span: tok.span,
            }),
            TokenKind::String(value) => Ok(Expr::String {
                value,
                span: tok.span,
            }),
            TokenKind::Verdadeiro => Ok(Expr::Bool {
                value: true,
                span: tok.span,
            }),
            TokenKind::Falso => Ok(Expr::Bool {
                value: false,
                span: tok.span,
            }),
            TokenKind::Ident(name) => Ok(Expr::Ident {
                name,
                span: tok.span,
            }),
            TokenKind::LParen => {
                let expr = self.parse_expr()?;
                self.expect_kind(|k| matches!(k, TokenKind::RParen), "esperado ')'")?;
                Ok(expr)
            }
            TokenKind::LBracket => self.finish_list(tok.span),
            TokenKind::Inicio | TokenKind::LBrace => {
                self.pos -= 1;
                let block = self.parse_block()?;
                Ok(Expr::Block(block))
            }
            TokenKind::Se => {
                self.pos -= 1;
                self.parse_if()
            }
            TokenKind::Funcao => {
                self.pos -= 1;
                self.parse_function()
            }
            TokenKind::Mapa => {
                self.pos -= 1;
                self.parse_map()
            }
            TokenKind::Matriz => {
                self.pos -= 1;
                self.parse_matrix()
            }
            _ => Err(self.err_at(
                format!("expressão inválida (token inesperado: {:?})", tok.kind),
                tok.span,
            )),
        }
    }

    fn finish_list(&mut self, start: Span) -> Result<Expr, ParseError> {
        let mut elements = Vec::new();
        if !matches!(self.peek_kind(), TokenKind::RBracket) {
            loop {
                elements.push(self.parse_expr()?);
                if matches!(self.peek_kind(), TokenKind::Comma) {
                    self.remove();
                    if matches!(self.peek_kind(), TokenKind::RBracket) {
                        break; // trailing comma ok
                    }
                    continue;
                }
                break;
            }
        }
        let end = self.expect_kind(|k| matches!(k, TokenKind::RBracket), "esperado ']'")?;
        Ok(Expr::List {
            elements,
            span: start.join(end.span),
        })
    }

    fn finish_call(&mut self, callee: Expr) -> Result<Expr, ParseError> {
        let start = callee.span();
        self.remove(); // (
        let mut args = Vec::new();
        if !matches!(self.peek_kind(), TokenKind::RParen) {
            loop {
                args.push(self.parse_expr()?);
                if matches!(self.peek_kind(), TokenKind::Comma) {
                    self.remove();
                    if matches!(self.peek_kind(), TokenKind::RParen) {
                        break;
                    }
                    continue;
                }
                break;
            }
        }
        let end = self.expect_kind(|k| matches!(k, TokenKind::RParen), "esperado ')'")?;
        Ok(Expr::Call {
            callee: Box::new(callee),
            args,
            span: start.join(end.span),
        })
    }

    fn finish_index_or_slice(&mut self, object: Expr) -> Result<Expr, ParseError> {
        let start = object.span();
        self.remove(); // [
        let first = self.parse_expr()?;
        if matches!(self.peek_kind(), TokenKind::DotDot) {
            self.remove();
            let end_expr = self.parse_expr()?;
            let end = self.expect_kind(|k| matches!(k, TokenKind::RBracket), "esperado ']'")?;
            Ok(Expr::Slice {
                object: Box::new(object),
                start: Box::new(first),
                end: Box::new(end_expr),
                span: start.join(end.span),
            })
        } else if matches!(self.peek_kind(), TokenKind::Comma) {
            self.remove();
            let col = self.parse_expr()?;
            let end = self.expect_kind(|k| matches!(k, TokenKind::RBracket), "esperado ']'")?;
            Ok(Expr::Index2 {
                object: Box::new(object),
                row: Box::new(first),
                col: Box::new(col),
                span: start.join(end.span),
            })
        } else {
            let end = self.expect_kind(|k| matches!(k, TokenKind::RBracket), "esperado ']'")?;
            Ok(Expr::Index {
                object: Box::new(object),
                index: Box::new(first),
                span: start.join(end.span),
            })
        }
    }

    fn finish_module_field(&mut self, object: Expr) -> Result<Expr, ParseError> {
        let start = object.span();
        self.remove(); // ::
        let field_tok = self.remove();
        match field_tok.kind {
            TokenKind::Ident(field) => Ok(Expr::Field {
                object: Box::new(object),
                field,
                span: start.join(field_tok.span),
            }),
            _ => Err(self.err_at("esperado nome após '::'", field_tok.span)),
        }
    }

    fn finish_map_field(&mut self, object: Expr) -> Result<Expr, ParseError> {
        let start = object.span();
        self.remove(); // :
        let field_tok = self.remove();
        match field_tok.kind {
            TokenKind::Ident(field) => Ok(Expr::MapField {
                object: Box::new(object),
                field,
                span: start.join(field_tok.span),
            }),
            _ => Err(self.err_at("esperado nome após ':'", field_tok.span)),
        }
    }

    /// `xs.anexe(1)` → `anexe(xs, 1)`. Bare `xs.anexe` is an error.
    fn finish_ufcs(&mut self, object: Expr) -> Result<Expr, ParseError> {
        let start = object.span();
        self.remove(); // .
        let field_tok = self.remove();
        let TokenKind::Ident(name) = field_tok.kind else {
            return Err(self.err_at("esperado nome de função após '.'", field_tok.span));
        };
        if !matches!(self.peek_kind(), TokenKind::LParen) {
            return Err(self.err(
                "esperado '(' após '.' (xs.tamanho() é tamanho(xs); sem parênteses não vale)",
            ));
        }
        let method_span = field_tok.span;
        let call = self.finish_call(Expr::Ident {
            name,
            span: method_span,
        })?;
        match call {
            Expr::Call {
                callee,
                mut args,
                span,
            } => {
                args.insert(0, object);
                Ok(Expr::Call {
                    callee,
                    args,
                    span: start.join(span),
                })
            }
            other => Ok(other),
        }
    }

    fn parse_if(&mut self) -> Result<Expr, ParseError> {
        let start = self
            .expect_kind(|k| matches!(k, TokenKind::Se), "esperado 'se'")?
            .span;

        let mut branches = Vec::new();
        let cond = self.parse_expr()?;
        let body = self.parse_block()?;
        let branch_span = cond.span().join(body.span);
        branches.push(IfBranch {
            cond,
            body,
            span: branch_span,
        });

        // `ou se`
        while matches!(self.peek_kind(), TokenKind::Ou) {
            // Lookahead: `ou se` vs binary `ou` (shouldn't appear here after a block usually)
            if self.pos + 1 < self.tokens.len()
                && matches!(self.tokens[self.pos + 1].kind, TokenKind::Se)
            {
                self.remove(); // ou
                self.remove(); // se
                let cond = self.parse_expr()?;
                let body = self.parse_block()?;
                let branch_span = cond.span().join(body.span);
                branches.push(IfBranch {
                    cond,
                    body,
                    span: branch_span,
                });
            } else {
                break;
            }
        }

        let else_block = if matches!(self.peek_kind(), TokenKind::Senao) {
            self.remove();
            Some(self.parse_block()?)
        } else {
            None
        };

        let end_span = else_block
            .as_ref()
            .map(|b| b.span)
            .or_else(|| branches.last().map(|b| b.span))
            .unwrap_or(start);

        Ok(Expr::If {
            branches,
            else_block,
            span: start.join(end_span),
        })
    }

    fn parse_function(&mut self) -> Result<Expr, ParseError> {
        let start = self
            .expect_kind(|k| matches!(k, TokenKind::Funcao), "esperado 'funcao'")?
            .span;
        self.expect_kind(
            |k| matches!(k, TokenKind::LParen),
            "esperado '(' após 'funcao'",
        )?;

        let mut params = Vec::new();
        if !matches!(self.peek_kind(), TokenKind::RParen) {
            loop {
                let p = self.remove();
                match p.kind {
                    TokenKind::Ident(name) => params.push(Param { name, span: p.span }),
                    _ => {
                        return Err(self.err_at("esperado nome de parâmetro", p.span));
                    }
                }
                if matches!(self.peek_kind(), TokenKind::Comma) {
                    self.remove();
                    continue;
                }
                break;
            }
        }
        self.expect_kind(|k| matches!(k, TokenKind::RParen), "esperado ')'")?;
        let body = self.parse_block()?;
        Ok(Expr::Function {
            params,
            span: start.join(body.span),
            body,
        })
    }

    fn parse_map(&mut self) -> Result<Expr, ParseError> {
        let start = self
            .expect_kind(|k| matches!(k, TokenKind::Mapa), "esperado 'mapa'")?
            .span;

        let brace = match self.peek_kind() {
            TokenKind::LBrace => true,
            TokenKind::Inicio => false,
            _ => {
                return Err(self.err("esperado 'inicio' ou '{' após 'mapa'"));
            }
        };
        let open = self.remove();

        if brace && matches!(self.peek_kind(), TokenKind::RBrace) {
            let end = self.remove();
            return Ok(Expr::Map {
                entries: vec![],
                span: start.join(end.span),
            });
        }

        let mut entries = Vec::new();
        loop {
            match self.peek_kind() {
                TokenKind::Eof => break,
                TokenKind::Fim if !brace => break,
                TokenKind::RBrace if brace => break,
                TokenKind::Fim if brace => {
                    return Err(self.err("esperado '}' (mapa aberto com '{')"));
                }
                TokenKind::RBrace if !brace => {
                    return Err(self.err("esperado 'fim' (mapa aberto com 'inicio')"));
                }
                _ => {
                    let key = self.parse_expr()?;
                    self.expect_kind(
                        |k| matches!(k, TokenKind::Arrow),
                        "esperado '->' entre chave e valor do mapa",
                    )?;
                    let value = self.parse_expr()?;
                    let span = key.span().join(value.span());
                    entries.push(MapEntry { key, value, span });
                }
            }
        }

        let end = if brace {
            self.expect_kind(|k| matches!(k, TokenKind::RBrace), "esperado '}' do mapa")?
        } else {
            self.expect_kind(|k| matches!(k, TokenKind::Fim), "esperado 'fim' do mapa")?
        };
        if entries.is_empty() {
            return Err(self.err_at(
                "literal de mapa com 'inicio'/'fim' precisa de entradas; use 'mapa {}' para vazio",
                open.span.join(end.span),
            ));
        }

        Ok(Expr::Map {
            entries,
            span: start.join(end.span),
        })
    }

    fn parse_matrix(&mut self) -> Result<Expr, ParseError> {
        let start = self
            .expect_kind(|k| matches!(k, TokenKind::Matriz), "esperado 'matriz'")?
            .span;
        let brace = match self.peek_kind() {
            TokenKind::LBrace => true,
            TokenKind::Inicio => false,
            _ => {
                return Err(self.err("esperado 'inicio' ou '{' após 'matriz'"));
            }
        };
        self.remove();
        let mut rows = Vec::new();
        loop {
            match self.peek_kind() {
                TokenKind::Eof => break,
                TokenKind::Fim if !brace => break,
                TokenKind::RBrace if brace => break,
                TokenKind::Fim if brace => {
                    return Err(self.err("esperado '}' (matriz aberta com '{')"));
                }
                TokenKind::RBrace if !brace => {
                    return Err(self.err("esperado 'fim' (matriz aberta com 'inicio')"));
                }
                _ => {
                    rows.push(self.parse_expr()?);
                    if matches!(self.peek_kind(), TokenKind::Comma) {
                        self.remove();
                    }
                }
            }
        }
        let end = if brace {
            self.expect_kind(|k| matches!(k, TokenKind::RBrace), "esperado '}' da matriz")?
        } else {
            self.expect_kind(|k| matches!(k, TokenKind::Fim), "esperado 'fim' da matriz")?
        };
        if rows.is_empty() {
            return Err(self.err_at(
                "matriz precisa de pelo menos uma linha",
                start.join(end.span),
            ));
        }
        Ok(Expr::Matrix {
            rows,
            span: start.join(end.span),
        })
    }
}

// ── helpers ──────────────────────────────────────────────────────

/// Look up an **infix** operator and its two binding powers.
///
/// Returns `None` if `kind` is not a binary/infix operator (e.g. `)`, `fim`).
///
/// # Why two numbers? (`left_bp`, `right_bp`)
///
/// When we see `lhs OP ?`, we need two different questions:
///
/// 1. **`left_bp` — “May this `OP` attach to the current `lhs`?”**
///    Compared with the caller’s `min_bp`:
///    - if `left_bp < min_bp` → do **not** take this op; stop and return `lhs`
///    - else → consume `OP` and parse the right-hand side
///
/// 2. **`right_bp` — “While parsing the RHS, how sticky should we be?”**
///    We call `parse_bp(right_bp)`. The RHS may only continue with operators
///    whose `left_bp >= right_bp`. That decides whether
///    `1 + 2 * 3` becomes `1+(2*3)` (`*` stronger than the RHS gate of `+`).
///
/// # Why are the pairs fixed like `(9, 10)`, `(11, 12)`, …?
///
/// They encode **two language decisions** that do not change at runtime:
///
/// **A) Precedence (which op wins)**  
///    Use *different* bands of numbers for different classes:
///    ```text
///    se_falhar  (1,2)   weakest
///    ou         (3,4)
///    e          (5,6)
///    compare    (7,8)
///    + -        (9,10)
///    * / %      (11,12) strongest infix
///    ```
///    Only the *relative* order matters (`*` > `+` > `e`). Gaps (1,3,5,…)
///    leave room if we add a level later; even numbers on the right are
///    just `left + 1` for left-associativity (below).
///
/// **B) Associativity (how same-level ops nest)**  
///    For left-associative ops (`+`, `-`, `*`, `e`, …) we set:
///    ```text
///    right_bp = left_bp + 1
///    ```
///    Example: `1 - 2 - 3`
///    - Take first `-` (left_bp 9). Parse RHS with `min_bp = 10`.
///    - Second `-` has left_bp 9, and `9 < 10` → RHS stops after `2`.
///    - Outer loop takes the second `-` → `((1-2)-3)`.
///
///    If we used `right_bp == left_bp` (same number both sides), the RHS
///    would also take the second `-` and you’d get right-assoc
///    `1-(2-3)` (or ambiguous behavior). So the “+1” is intentional.
///
/// Right-associative operators (e.g. `**` in some languages) would use
/// `right_bp = left_bp` (or `left_bp - 1`); Expressa has none of those yet.
///
/// # Summary
///
/// | Number    | Role                                      |
/// |-----------|-------------------------------------------|
/// | `left_bp` | “Can I bind to the left operand now?”     |
/// | `right_bp`| “What may the right operand still include?”|
///
/// Both are **constants** from the language precedence table, not computed
/// from the source text.
fn infix_info(kind: &TokenKind) -> Option<(TokenKind, u8, u8)> {
    // (left_bp, right_bp) with right_bp = left_bp + 1 → left-associative.
    // Higher bp = tighter binding.
    let (l_bp, r_bp) = match kind {
        TokenKind::SeFalhar => (1, 2),
        TokenKind::Ou => (3, 4),
        TokenKind::E => (5, 6),
        TokenKind::EqEq
        | TokenKind::BangEq
        | TokenKind::Lt
        | TokenKind::Gt
        | TokenKind::LtEq
        | TokenKind::GtEq
        | TokenKind::Contem => (7, 8),
        TokenKind::Plus | TokenKind::Minus => (9, 10),
        TokenKind::Star | TokenKind::Slash | TokenKind::Percent => (11, 12),
        _ => return None,
    };
    Some((kind.clone(), l_bp, r_bp))
}

fn token_to_binary(kind: &TokenKind) -> Option<BinaryOp> {
    Some(match kind {
        TokenKind::Plus => BinaryOp::Add,
        TokenKind::Minus => BinaryOp::Sub,
        TokenKind::Star => BinaryOp::Mul,
        TokenKind::Slash => BinaryOp::Div,
        TokenKind::Percent => BinaryOp::Rem,
        TokenKind::EqEq => BinaryOp::Eq,
        TokenKind::BangEq => BinaryOp::Ne,
        TokenKind::Lt => BinaryOp::Lt,
        TokenKind::Gt => BinaryOp::Gt,
        TokenKind::LtEq => BinaryOp::Le,
        TokenKind::GtEq => BinaryOp::Ge,
        TokenKind::E => BinaryOp::And,
        TokenKind::Ou => BinaryOp::Or,
        TokenKind::Contem => BinaryOp::Contem,
        _ => return None,
    })
}

fn is_comparison_token(kind: &TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::EqEq
            | TokenKind::BangEq
            | TokenKind::Lt
            | TokenKind::Gt
            | TokenKind::LtEq
            | TokenKind::GtEq
    )
}

fn is_comparison_op(op: BinaryOp) -> bool {
    matches!(
        op,
        BinaryOp::Eq | BinaryOp::Ne | BinaryOp::Lt | BinaryOp::Le | BinaryOp::Gt | BinaryOp::Ge
    )
}

fn expr_to_assign_target(expr: Expr) -> Result<AssignTarget, Span> {
    match expr {
        Expr::Ident { name, span } => Ok(AssignTarget::Name { name, span }),
        Expr::Index {
            object,
            index,
            span,
        } => Ok(AssignTarget::Index {
            object: *object,
            index: *index,
            span,
        }),
        Expr::Index2 {
            object,
            row,
            col,
            span,
        } => Ok(AssignTarget::Index2 {
            object: *object,
            row: *row,
            col: *col,
            span,
        }),
        Expr::MapField {
            object,
            field,
            span,
        } => Ok(AssignTarget::MapField {
            object: *object,
            field,
            span,
        }),
        other => Err(other.span()),
    }
}

impl Item {
    pub fn span(&self) -> Span {
        match self {
            Item::Import(i) => i.span,
            Item::Stmt(s) => s.span(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::operator_types::BinaryOp;

    fn parse_ok(src: &str) -> Program {
        parse(src).unwrap_or_else(|e| panic!("parse failed for {src:?}: {e}"))
    }

    fn first_stmt(p: &Program) -> &Stmt {
        match &p.items[0] {
            Item::Stmt(s) => s,
            other => panic!("expected stmt, got {other:?}"),
        }
    }

    fn expr_stmt(p: &Program) -> &Expr {
        match first_stmt(p) {
            Stmt::Expr { expr, .. } => expr,
            other => panic!("expected expr stmt, got {other:?}"),
        }
    }

    #[test]
    fn number_literal() {
        let p = parse_ok("42");
        let e = expr_stmt(&p);
        assert!(matches!(e, Expr::Number { raw, .. } if raw == "42"));
    }

    #[test]
    fn string_and_bool() {
        let p = parse_ok(r#""oi""#);
        assert!(matches!(
            expr_stmt(&p),
            Expr::String { value, .. } if value == "oi"
        ));
        let p = parse_ok("verdadeiro");
        assert!(matches!(expr_stmt(&p), Expr::Bool { value: true, .. }));
    }

    #[test]
    fn precedence_mul_over_add() {
        // 1 + 2 * 3  →  1 + (2 * 3)
        let p = parse_ok("1 + 2 * 3");
        let e = expr_stmt(&p);
        match e {
            Expr::Binary {
                op: BinaryOp::Add,
                left,
                right,
                ..
            } => {
                assert!(matches!(left.as_ref(), Expr::Number { raw, .. } if raw == "1"));
                assert!(matches!(
                    right.as_ref(),
                    Expr::Binary {
                        op: BinaryOp::Mul,
                        ..
                    }
                ));
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn assign_name() {
        match first_stmt(&parse_ok("x = 10")) {
            Stmt::Assign {
                target: AssignTarget::Name { name, .. },
                value: Expr::Number { raw, .. },
                ..
            } => {
                assert_eq!(name, "x");
                assert_eq!(raw, "10");
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn block_expression() {
        let p = parse_ok("inicio\n  1 + 2\nfim");
        let e = expr_stmt(&p);
        match e {
            Expr::Block(b) => {
                assert_eq!(b.stmts.len(), 1);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn empty_block_errors() {
        let err = parse("inicio fim").unwrap_err();
        assert!(err.message.contains("vazio"));
        let err = parse("{ }").unwrap_err();
        assert!(err.message.contains("vazio"));
    }

    #[test]
    fn brace_block_same_as_inicio_fim() {
        let a = parse_ok("inicio\n  1 + 2\nfim");
        let b = parse_ok("{ 1 + 2 }");
        match (expr_stmt(&a), expr_stmt(&b)) {
            (Expr::Block(x), Expr::Block(y)) => assert_eq!(x.stmts.len(), y.stmts.len()),
            other => panic!("{other:?}"),
        }
        let p = parse_ok("se verdadeiro { 1 } senao { 2 }");
        match expr_stmt(&p) {
            Expr::If {
                else_block: Some(_),
                ..
            } => {}
            other => panic!("{other:?}"),
        }
        let p = parse_ok("f = funcao(n) { n * 2 }");
        assert!(matches!(&p.items[0], Item::Stmt(Stmt::Assign { .. })));
        let p = parse_ok(r#"mapa { "a" -> 1 }"#);
        match expr_stmt(&p) {
            Expr::Map { entries, .. } => assert_eq!(entries.len(), 1),
            other => panic!("{other:?}"),
        }
        let err = parse("inicio 1 }").unwrap_err();
        assert!(err.message.contains("fim"), "{err}");
        let err = parse("{ 1 fim").unwrap_err();
        assert!(err.message.contains("'}'"), "{err}");
    }

    #[test]
    fn matriz_literal_and_index2() {
        let p = parse_ok(
            r#"
A = matriz {
    [1, 2, 3],
    [4, 5, 6]
}
"#,
        );
        match &p.items[0] {
            Item::Stmt(Stmt::Assign { value, .. }) => {
                assert!(matches!(value, Expr::Matrix { rows, .. } if rows.len() == 2));
            }
            other => panic!("{other:?}"),
        }
        let p = parse_ok("A[1, 2]");
        assert!(matches!(expr_stmt(&p), Expr::Index2 { .. }));
    }

    #[test]
    fn if_with_else() {
        let src = r#"
se x > 0
inicio
    1
fim
senao
inicio
    2
fim
"#;
        let p = parse_ok(src);
        match expr_stmt(&p) {
            Expr::If {
                branches,
                else_block: Some(_),
                ..
            } => {
                assert_eq!(branches.len(), 1);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn access_colon_ufcs_module() {
        match expr_stmt(&parse_ok("pessoa:nome")) {
            Expr::MapField { field, .. } => assert_eq!(field, "nome"),
            other => panic!("{other:?}"),
        }
        match expr_stmt(&parse_ok("mat::soma")) {
            Expr::Field { field, .. } => assert_eq!(field, "soma"),
            other => panic!("{other:?}"),
        }
        match expr_stmt(&parse_ok("xs.tamanho()")) {
            Expr::Call { callee, args, .. } => {
                assert!(matches!(callee.as_ref(), Expr::Ident { name, .. } if name == "tamanho"));
                assert_eq!(args.len(), 1);
            }
            other => panic!("{other:?}"),
        }
        let err = parse("xs.tamanho").unwrap_err();
        assert!(err.message.contains("esper"), "{err}");
    }

    #[test]
    fn call_and_list() {
        let p = parse_ok(r#"escreva([1, 2])"#);
        match expr_stmt(&p) {
            Expr::Call { callee, args, .. } => {
                assert!(matches!(callee.as_ref(), Expr::Ident { name, .. } if name == "escreva"));
                assert_eq!(args.len(), 1);
                assert!(matches!(args[0], Expr::List { .. }));
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn prefix_does_not_steal_postfix() {
        match expr_stmt(&parse_ok("nao eh_par(n)")) {
            Expr::Unary {
                op: UnaryOp::Not,
                expr,
                ..
            } => {
                assert!(matches!(expr.as_ref(), Expr::Call { .. }));
            }
            other => panic!("expected nao (call), got {other:?}"),
        }
        match expr_stmt(&parse_ok("-xs[1]")) {
            Expr::Unary {
                op: UnaryOp::Neg,
                expr,
                ..
            } => {
                assert!(matches!(expr.as_ref(), Expr::Index { .. }));
            }
            other => panic!("expected -(index), got {other:?}"),
        }
    }

    #[test]
    fn index_and_slice() {
        let p = parse_ok("xs[1]");
        assert!(matches!(expr_stmt(&p), Expr::Index { .. }));
        let p = parse_ok("xs[1..3]");
        assert!(matches!(expr_stmt(&p), Expr::Slice { .. }));
    }

    #[test]
    fn se_falhar() {
        let p = parse_ok("a se_falhar b");
        match expr_stmt(&p) {
            Expr::SeFalhar { .. } => {}
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn function_literal() {
        let src = r#"
funcao(x, y)
inicio
    x + y
fim
"#;
        let p = parse_ok(src);
        match expr_stmt(&p) {
            Expr::Function { params, body, .. } => {
                assert_eq!(params.len(), 2);
                assert_eq!(params[0].name, "x");
                assert_eq!(body.stmts.len(), 1);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn mapa_vazio_and_entries() {
        let p = parse_ok("mapa {}");
        assert!(matches!(
            expr_stmt(&p),
            Expr::Map { entries, .. } if entries.is_empty()
        ));

        let src = r#"
mapa
inicio
    "a" -> 1
fim
"#;
        let p = parse_ok(src);
        match expr_stmt(&p) {
            Expr::Map { entries, .. } => assert_eq!(entries.len(), 1),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn repita_and_para() {
        let src = r#"
repita 3 vezes
inicio
    escreva("x")
fim
"#;
        assert!(matches!(first_stmt(&parse_ok(src)), Stmt::Repita { .. }));

        let src = r#"
para i de 1 ate 5
inicio
    escreva(i)
fim
"#;
        assert!(matches!(first_stmt(&parse_ok(src)), Stmt::ParaRange { .. }));

        let src = r#"
para x em xs
inicio
    escreva(x)
fim
"#;
        assert!(matches!(first_stmt(&parse_ok(src)), Stmt::ParaIn { .. }));
    }

    #[test]
    fn importe() {
        match &parse_ok(r#"importe "math""#).items[0] {
            Item::Import(Import {
                alias: None, path, ..
            }) => assert_eq!(path, "math"),
            other => panic!("{other:?}"),
        }
        match &parse_ok(r#"m = importe "math""#).items[0] {
            Item::Import(Import {
                alias: Some(a),
                path,
                ..
            }) => {
                assert_eq!(a, "m");
                assert_eq!(path, "math");
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn comparison_chain_errors() {
        assert!(parse("1 < 2 < 3").is_err());
    }

    #[test]
    fn assign_index() {
        match first_stmt(&parse_ok("xs[1] = 9")) {
            Stmt::Assign {
                target: AssignTarget::Index { .. },
                ..
            } => {}
            other => panic!("{other:?}"),
        }
    }
}
