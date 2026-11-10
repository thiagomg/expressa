use super::*;

struct Compiler {
    code: Chunk,
    names: HashMap<String, u32>,
    spans: HashMap<Span, u32>,
}

impl Compiler {
    fn new() -> Self {
        Self {
            code: Chunk {
                instructions: vec![],
                spans: vec![],
                names: vec![],
                constants: vec![],
                functions: vec![],
                imports: vec![],
                loops: vec![],
            },
            names: HashMap::new(),
            spans: HashMap::new(),
        }
    }
    fn name(&mut self, name: &str) -> u32 {
        if let Some(id) = self.names.get(name) {
            return *id;
        }
        let id = self.code.names.len() as u32;
        self.code.names.push(name.into());
        self.names.insert(name.into(), id);
        id
    }
    fn emit(&mut self, op: Op, span: Span) -> u32 {
        let n = self.code.spans.len() as u32;
        let span = *self.spans.entry(span).or_insert_with(|| {
            self.code.spans.push(span);
            n
        });
        let at = self.here();
        self.code.instructions.push(Instruction { op, span });
        at
    }
    fn here(&self) -> u32 {
        self.code.instructions.len() as u32
    }
    fn jump(&mut self, at: u32, target: u32) {
        match &mut self.code.instructions[at as usize].op {
            Op::Jump(to) | Op::BranchFalse(to) | Op::Try(to) => *to = target,
            Op::ShortCircuit { target: to, .. } => *to = target,
            _ => unreachable!("compiler patch"),
        }
    }
    fn items(&mut self, items: &[Item]) {
        self.emit(Op::Nada, Span::default());
        for item in items {
            match item {
                Item::Stmt(stmt) => {
                    self.stmt(stmt);
                    if matches!(stmt, Stmt::Expr { .. }) {
                        self.emit(Op::SetLast, stmt.span());
                    } else {
                        self.emit(Op::Drop, stmt.span());
                        self.emit(Op::Nada, stmt.span());
                    }
                }
                Item::Import(import) => {
                    let id = self.code.imports.len() as u32;
                    self.code.imports.push(import.clone());
                    self.emit(Op::Import(id), import.span);
                    self.emit(Op::Drop, import.span);
                    self.emit(Op::Nada, import.span);
                }
            }
        }
    }
    fn stmts(&mut self, stmts: &[Stmt]) {
        if let [stmt @ Stmt::Expr { .. }] = stmts {
            self.stmt(stmt);
            return;
        }
        self.emit(Op::Nada, Span::default());
        for stmt in stmts {
            self.stmt(stmt);
            // The AST evaluator retains the last expression across assignments/loops.
            if matches!(stmt, Stmt::Expr { .. }) {
                self.emit(Op::SetLast, stmt.span());
            }
        }
    }
    fn block(&mut self, block: &Block) {
        self.emit(Op::EnterScope, block.span);
        self.stmts(&block.stmts);
        self.emit(Op::LeaveScope, block.span);
    }
    fn stmt(&mut self, stmt: &Stmt) {
        stacker::maybe_grow(PILHA_RESERVA, PILHA_SEGMENTO, || self.stmt_inner(stmt));
    }
    fn stmt_inner(&mut self, stmt: &Stmt) {
        let span = stmt.span();
        self.emit(Op::Checkpoint, span);
        match stmt {
            Stmt::Expr { expr, .. } => self.expr(expr),
            Stmt::Assign {
                target,
                compound,
                value,
                ..
            } => {
                if let AssignTarget::Name {
                    name,
                    span: target_span,
                } = target
                {
                    let id = self.name(name);
                    if let Some(op) = compound {
                        self.emit(Op::Load(id), *target_span);
                        self.expr(value);
                        self.emit(Op::Binary(*op), span);
                    } else {
                        self.expr(value);
                    }
                    self.emit(Op::Store(id), *target_span);
                    return;
                }
                if let Some(op) = compound {
                    self.target(target, true);
                    self.emit(Op::ReadDestination, target.span());
                    self.expr(value);
                    self.emit(Op::Binary(*op), span);
                } else {
                    self.expr(value);
                    self.target(target, false);
                }
                self.emit(Op::WriteDestination, span);
            }
            Stmt::Retorne { value, .. } => {
                if let Some(value) = value {
                    self.expr(value);
                } else {
                    self.emit(Op::Nada, span);
                }
                self.emit(Op::Return, span);
            }
            Stmt::Pare { .. } => {
                self.emit(Op::Break, span);
            }
            Stmt::Continue { .. } => {
                self.emit(Op::Continue, span);
            }
            Stmt::Repita { count, body, .. } => {
                self.expr(count);
                self.loop_body(LoopKind::Repeat, None, count.span(), span, body, None);
            }
            Stmt::ParaRange {
                var,
                from,
                to,
                body,
                ..
            } => {
                self.expr(from);
                self.emit(Op::CheckInt, from.span());
                self.expr(to);
                self.loop_body(
                    LoopKind::Range,
                    Some(var),
                    from.span(),
                    to.span(),
                    body,
                    None,
                );
            }
            Stmt::ParaIn {
                var, iter, body, ..
            } => {
                self.expr(iter);
                self.loop_body(LoopKind::Items, Some(var), iter.span(), span, body, None);
            }
            Stmt::Enquanto { cond, body, .. } => {
                self.loop_body(LoopKind::While, None, cond.span(), span, body, Some(cond));
            }
        }
    }
    fn loop_body(
        &mut self,
        kind: LoopKind,
        var: Option<&str>,
        first_span: Span,
        second_span: Span,
        body: &Block,
        cond: Option<&Expr>,
    ) {
        let var = var.map(|v| self.name(v));
        let id = self.code.loops.len() as u32;
        self.code.loops.push(LoopSpec {
            kind,
            var,
            first_span,
            second_span,
            exit: 0,
            next: 0,
        });
        self.emit(Op::LoopEnter(id), body.span);
        let next = self.here();
        let mut branch = None;
        if let Some(cond) = cond {
            self.emit(Op::ParentScope, second_span);
            self.expr(cond);
            self.emit(Op::RestoreScope, cond.span());
            branch = Some(self.emit(Op::BranchFalse(0), cond.span()));
        } else {
            self.emit(Op::LoopNext(id), first_span);
        }
        self.emit(Op::LoopBody, body.span);
        // Retain expression results until their replacement/end, as the AST does:
        // a processo value has observable cleanup. An assignment-only body has
        // only an unobservable Nada accumulator, which can be omitted safely.
        if body
            .stmts
            .iter()
            .any(|stmt| matches!(stmt, Stmt::Expr { .. }))
        {
            self.stmts(&body.stmts);
            self.emit(Op::Drop, body.span);
        } else {
            for stmt in &body.stmts {
                self.stmt(stmt);
            }
        }
        self.emit(Op::Jump(next), body.span);
        let exit = self.here();
        if let Some(branch) = branch {
            self.jump(branch, exit);
        }
        self.emit(Op::LoopEnd, body.span);
        self.code.loops[id as usize].next = next;
        self.code.loops[id as usize].exit = exit;
    }
    fn expr(&mut self, expr: &Expr) {
        stacker::maybe_grow(PILHA_RESERVA, PILHA_SEGMENTO, || self.expr_inner(expr));
    }
    fn expr_inner(&mut self, expr: &Expr) {
        let span = expr.span();
        match expr {
            Expr::Number { raw, .. } => {
                if let Some(n) = parse_numero(raw) {
                    let id = self.code.constants.len() as u32;
                    self.code.constants.push(Value::Numero(n));
                    self.emit(Op::Constant(id), span);
                } else {
                    let id = self.name(raw);
                    self.emit(Op::BadNumber(id), span);
                }
            }
            Expr::String { value, .. } => {
                let id = self.code.constants.len() as u32;
                self.code.constants.push(Value::Texto(value.clone()));
                self.emit(Op::Constant(id), span);
            }
            Expr::Bool { value, .. } => {
                self.emit(Op::Bool(*value), span);
            }
            Expr::Ident { name, .. } => {
                let id = self.name(name);
                self.emit(Op::Load(id), span);
            }
            Expr::List { elements, .. } => {
                for e in elements {
                    self.expr(e);
                }
                self.emit(Op::List(elements.len() as u32), span);
            }
            Expr::Par { key, value, .. } => {
                self.expr(key);
                self.expr(value);
                self.emit(Op::Pair, span);
            }
            Expr::Function {
                params, body, doc, ..
            } => {
                let mut compiler = Self::new();
                compiler.stmts(&body.stmts);
                compiler.emit(Op::Finish, body.span);
                let id = self.code.functions.len() as u32;
                self.code.functions.push(Prototype {
                    params: params.clone(),
                    code: Rc::new(compiler.code),
                    span,
                    doc: doc.clone(),
                });
                self.emit(Op::Closure(id), span);
            }
            Expr::Field { object, field, .. } => {
                self.expr(object);
                let id = self.name(field);
                self.emit(Op::Field(id), span);
            }
            Expr::MapField { object, field, .. } => {
                self.expr(object);
                let id = self.name(field);
                self.emit(Op::MapField(id), span);
            }
            Expr::Call { callee, args, .. } => {
                self.expr(callee);
                for arg in args {
                    self.expr(arg);
                }
                let name = self.name(&callee_name(callee));
                self.emit(
                    Op::Call {
                        argc: args.len() as u32,
                        name,
                    },
                    span,
                );
            }
            Expr::Index { object, index, .. } => {
                self.expr(object);
                self.expr(index);
                self.emit(Op::Index, span);
            }
            Expr::Index2 {
                object, row, col, ..
            } => {
                self.expr(object);
                self.expr(row);
                self.expr(col);
                self.emit(Op::Index2, span);
            }
            Expr::Slice {
                object, start, end, ..
            } => {
                self.expr(object);
                self.expr(start);
                if let Some(end) = end {
                    self.expr(end);
                }
                self.emit(Op::Slice(end.is_some()), span);
            }
            Expr::Unary { op, expr, .. } => {
                self.expr(expr);
                self.emit(Op::Unary(*op), span);
            }
            Expr::Binary {
                left, op, right, ..
            } => {
                self.expr(left);
                if matches!(op, BinaryOp::And | BinaryOp::Or) {
                    let jump = self.emit(
                        Op::ShortCircuit {
                            target: 0,
                            value: matches!(op, BinaryOp::Or),
                        },
                        span,
                    );
                    self.expr(right);
                    self.emit(Op::CheckBool, span);
                    self.jump(jump, self.here());
                } else {
                    self.expr(right);
                    self.emit(Op::Binary(*op), span);
                }
            }
            Expr::Block(block) => self.block(block),
            Expr::If {
                branches,
                else_block,
                ..
            } => {
                let mut exits = vec![];
                for branch in branches {
                    self.expr(&branch.cond);
                    let next = self.emit(Op::BranchFalse(0), branch.cond.span());
                    self.block(&branch.body);
                    exits.push(self.emit(Op::Jump(0), span));
                    self.jump(next, self.here());
                }
                if let Some(block) = else_block {
                    self.block(block);
                } else {
                    self.emit(Op::Nada, span);
                }
                let end = self.here();
                for exit in exits {
                    self.jump(exit, end);
                }
            }
            Expr::SeFalhar {
                attempt, fallback, ..
            } => {
                let handler = self.emit(Op::Try(0), span);
                self.expr(attempt);
                self.emit(Op::EndTry, span);
                let end = self.emit(Op::Jump(0), span);
                self.jump(handler, self.here());
                self.expr(fallback);
                self.jump(end, self.here());
            }
        }
    }
    fn target(&mut self, target: &AssignTarget, compound: bool) {
        match target {
            AssignTarget::Name { name, span } => {
                let id = self.name(name);
                self.emit(Op::NameDestination(id), *span);
            }
            AssignTarget::Index {
                object,
                index,
                span,
            } => {
                if compound {
                    self.receiver(object);
                    self.expr(index);
                } else {
                    self.expr(index);
                    self.receiver(object);
                }
                self.emit(Op::IndexDestination(compound), *span);
            }
            AssignTarget::Index2 {
                object,
                row,
                col,
                span,
            } => {
                self.expr(object);
                self.expr(row);
                self.expr(col);
                self.emit(Op::MatrixDestination, *span);
            }
            AssignTarget::MapField {
                object,
                field,
                span,
            } => {
                self.expr(object);
                let id = self.name(field);
                self.emit(Op::MapDestination(id), *span);
            }
        }
    }
    fn receiver(&mut self, expr: &Expr) {
        stacker::maybe_grow(PILHA_RESERVA, PILHA_SEGMENTO, || {
            let span = expr.span();
            match expr {
                Expr::Ident { name, .. } => {
                    let id = self.name(name);
                    self.emit(Op::ReceiverName(id), span);
                }
                Expr::Index { object, index, .. } => {
                    self.receiver(object);
                    self.expr(index);
                    self.emit(Op::ReceiverIndex, span);
                }
                Expr::Index2 {
                    object, row, col, ..
                } => {
                    self.expr(object);
                    self.expr(row);
                    self.expr(col);
                    self.emit(Op::ReceiverIndex2, span);
                }
                Expr::MapField { object, field, .. } => {
                    self.expr(object);
                    let id = self.name(field);
                    self.emit(Op::ReceiverMap(id), span);
                }
                _ => {
                    self.expr(expr);
                    self.emit(
                        Op::ReceiverReadOnly(matches!(expr, Expr::String { .. })),
                        span,
                    );
                }
            }
        });
    }
}

pub(super) fn items(items: &[Item]) -> Rc<Chunk> {
    let mut compiler = Compiler::new();
    compiler.items(items);
    compiler.emit(Op::Finish, Span::default());
    Rc::new(compiler.code)
}

/// File/module execution discards each item's value immediately, unlike avalie/REPL.
pub(super) fn module(items: &[Item]) -> Rc<Chunk> {
    let mut compiler = Compiler::new();
    for item in items {
        match item {
            Item::Stmt(stmt) => {
                compiler.stmt(stmt);
                if matches!(stmt, Stmt::Expr { .. }) {
                    compiler.emit(Op::Drop, stmt.span());
                }
            }
            Item::Import(import) => {
                let id = compiler.code.imports.len() as u32;
                compiler.code.imports.push(import.clone());
                compiler.emit(Op::Import(id), import.span);
            }
        }
    }
    compiler.emit(Op::Nada, Span::default());
    compiler.emit(Op::Finish, Span::default());
    Rc::new(compiler.code)
}
