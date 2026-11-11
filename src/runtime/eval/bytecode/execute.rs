use super::*;

#[derive(Clone, Copy, PartialEq)]
enum FrameKindCode {
    Plain,
    Function,
    Eval,
}
struct Handler {
    target: usize,
    values: usize,
    destinations: usize,
    scopes: usize,
    loops: usize,
}
struct Loop {
    id: usize,
    state: Iteration,
    active: bool,
    values: usize,
    destinations: usize,
    scopes: usize,
    handlers: usize,
}
enum Iteration {
    Repeat(i64),
    Range { next: Option<i64>, end: i64 },
    Items(std::vec::IntoIter<Value>),
    Process(Rc<RefCell<super::super::super::processo::Processo>>),
    While,
}
impl Drop for Loop {
    fn drop(&mut self) {
        if let Iteration::Process(p) = &self.state {
            p.borrow_mut().encerre();
        }
    }
}
struct Frame {
    code: Rc<Chunk>,
    ip: usize,
    envs: Vec<Rc<RefCell<Env>>>,
    handlers: Vec<Handler>,
    loops: Vec<Loop>,
    kind: FrameKindCode,
    values: usize,
    destinations: usize,
    restore: Option<(String, Rc<str>)>,
}
impl Frame {
    fn env(&self) -> &Rc<RefCell<Env>> {
        self.envs.last().expect("compiler scope")
    }
    fn leave_scopes(&mut self, depth: usize) {
        while self.envs.len() > depth {
            let env = self.envs.pop().expect("scope");
            Env::liberar_se_lixo(&env);
        }
    }
    fn handlers_to(&mut self, depth: usize, vm: &mut Vm<'_, '_>) {
        vm.protected -= (self.handlers.len() - depth) as u32;
        self.handlers.truncate(depth);
    }
}
struct Machine {
    frames: Vec<Frame>,
    values: Vec<Value>,
    destinations: Vec<Destination>,
}
enum Step {
    Next,
    Call(Value, Vec<Value>, String, Span),
    CallStack(usize, String, Span),
    Finish(Value),
}

impl Machine {
    // Keep dispatch in the run loop so instruction results need not cross a
    // Rust call boundary. Benchmarked independently from other VM changes.
    #[inline(always)]
    fn step(&mut self, vm: &mut Vm<'_, '_>) -> Result<Step, EvalError> {
        let frame = self.frames.last_mut().expect("frame");
        let instruction = frame.code.instructions[frame.ip];
        frame.ip += 1;
        let span = frame.code.spans[instruction.span as usize];
        macro_rules! pop {
            () => {
                self.values.pop().expect("compiler value stack")
            };
        }
        macro_rules! name {
            ($id:expr) => {
                &frame.code.names[$id as usize]
            };
        }
        match instruction.op {
            Op::Nada => self.values.push(Value::Nada),
            Op::Bool(value) => self.values.push(Value::Bool(value)),
            Op::Constant(id) => self.values.push(frame.code.constants[id as usize].clone()),
            Op::BadNumber(id) => {
                return Err(vm.err(format!("número inválido: {}", name!(id)), span));
            }
            Op::Load(id) => {
                self.values
                    .push(Env::get(frame.env(), name!(id)).ok_or_else(|| {
                        vm.err(format!("variável `{}` não definida", name!(id)), span)
                    })?)
            }
            Op::Store(id) => {
                let value = pop!();
                let name = name!(id);
                let destination = Env::assignment_frame(frame.env(), name)
                    .map_err(|error| vm.assignment_error(error, span))?;
                let mut bindings = destination.borrow_mut();
                if let Some(slot) = bindings.vars.get_mut(name) {
                    *slot = value;
                } else {
                    bindings.define(name.clone(), value);
                }
            }
            Op::LoadLocal(index) => self
                .values
                .push(frame.env().borrow().vars.parameter(index).clone()),
            Op::StoreLocal(index) => {
                let value = pop!();
                *frame.env().borrow_mut().vars.parameter_mut(index) = value;
            }
            Op::Drop => {
                pop!();
            }
            Op::SetLast => {
                let value = pop!();
                pop!();
                self.values.push(value);
            }
            Op::List(count) => {
                let values = self.values.split_off(self.values.len() - count as usize);
                self.values.push(Value::lista(values));
            }
            Op::Pair => {
                let value = pop!();
                let key = pop!();
                let key = MapKey::from_value(&key).ok_or_else(|| {
                    vm.err(format!("chave de par inválida ({})", key.type_name()), span)
                })?;
                self.values.push(Value::Par(key, Box::new(value)));
            }
            Op::Unary(op) => {
                let value = pop!();
                self.values.push(match op {
                    UnaryOp::Neg => match &value {
                        Value::Matriz(_) => vm.matriz_scale(&value, &value, -1.0, span)?,
                        _ => Value::Numero(-vm.expect_numero(&value, span)?),
                    },
                    UnaryOp::Not => Value::Bool(!vm.expect_bool(&value, span)?),
                });
            }
            Op::Binary(op) => {
                let right = pop!();
                let left = pop!();
                self.values
                    .push(stacker::maybe_grow(PILHA_RESERVA, PILHA_SEGMENTO, || {
                        vm.apply_binary(&left, op, &right, span, frame.env())
                    })?);
            }
            Op::Field(id) => {
                let object = pop!();
                let field = name!(id);
                self.values.push(match object {
                    Value::Modulo(module) => Env::get(&module, field)
                        .ok_or_else(|| vm.err(format!("módulo não contém `{field}`"), span))?,
                    Value::Mapa(_) => {
                        return Err(vm.err(format!("mapa usa ':' — tente o_mapa:{field}"), span));
                    }
                    other => {
                        return Err(vm.err(
                            format!(
                                "acesso '::' espera um módulo, encontrado {}",
                                other.type_name()
                            ),
                            span,
                        ));
                    }
                });
            }
            Op::MapField(id) => {
                let object = pop!();
                self.values
                    .push(vm.bytecode_map_get(&object, name!(id), span)?);
            }
            Op::Index => {
                let index = pop!();
                let object = pop!();
                self.values.push(vm.index_get(&object, &index, span)?);
            }
            Op::Index2 => {
                let col = pop!();
                let row = pop!();
                let object = pop!();
                self.values.push(vm.index_get2(&object, &row, &col, span)?);
            }
            Op::Slice(has_end) => {
                let end = if has_end { Some(pop!()) } else { None };
                let start = pop!();
                let object = pop!();
                self.values
                    .push(vm.slice_get(&object, &start, end.as_ref(), span)?);
            }
            Op::Closure(id) => {
                let prototype = &frame.code.functions[id as usize];
                check_unique_params(&prototype.params)
                    .map_err(|name| vm.err(format!("parâmetro duplicado `{name}`"), span))?;
                vm.marcar_funcao(frame.env());
                self.values.push(Value::Funcao(Rc::new(Closure {
                    params: prototype.params.clone(),
                    body: FunctionBody::Bytecode(Rc::clone(&prototype.code)),
                    env: Rc::clone(frame.env()),
                    span: prototype.span,
                    file: vm.file.clone(),
                    source: Rc::clone(&vm.source),
                    doc: prototype.doc.clone(),
                })));
            }
            Op::Call { argc, name } => {
                let argc = argc as usize;
                let callee = self.values.len() - argc - 1;
                if matches!(&self.values[callee], Value::Funcao(closure)
                    if matches!(closure.body, FunctionBody::Bytecode(_)))
                {
                    return Ok(Step::CallStack(argc, name!(name).clone(), span));
                }
                let args = self.values.split_off(self.values.len() - argc);
                let function = pop!();
                return Ok(Step::Call(function, args, name!(name).clone(), span));
            }
            Op::Return => {
                return Err(EvalError::Return {
                    value: pop!(),
                    span,
                });
            }
            Op::Finish => return Ok(Step::Finish(pop!())),
            Op::Checkpoint => vm.pause(span, frame.env())?,
            Op::EnterScope => frame.envs.push(Env::child(frame.env(), FrameKind::Block)),
            Op::LeaveScope => frame.leave_scopes(frame.envs.len() - 1),
            Op::ParentScope => {
                vm.check_deadline(span)?;
                frame.loops.last_mut().expect("while").active = false;
                let parent = frame.env().borrow().parent.clone().expect("loop parent");
                frame.envs.push(parent);
            }
            Op::RestoreScope => {
                frame.envs.pop().expect("parent override");
            }
            Op::Jump(target) => frame.ip = target as usize,
            Op::BranchFalse(target) => {
                if !vm.expect_bool(&pop!(), span)? {
                    frame.ip = target as usize;
                }
            }
            Op::ShortCircuit { target, value } => {
                if vm.expect_bool(&pop!(), span)? == value {
                    self.values.push(Value::Bool(value));
                    frame.ip = target as usize;
                }
            }
            Op::CheckInt => {
                vm.expect_int(self.values.last().expect("range start"), span)?;
            }
            Op::CheckBool => {
                let value = vm.expect_bool(&pop!(), span)?;
                self.values.push(Value::Bool(value));
            }
            Op::Try(target) => {
                frame.handlers.push(Handler {
                    target: target as usize,
                    values: self.values.len(),
                    destinations: self.destinations.len(),
                    scopes: frame.envs.len(),
                    loops: frame.loops.len(),
                });
                vm.protected += 1;
            }
            Op::EndTry => {
                frame.handlers.pop().expect("handler");
                vm.protected -= 1;
            }
            Op::Import(id) => stacker::maybe_grow(PILHA_RESERVA, PILHA_SEGMENTO, || {
                vm.eval_import(&frame.code.imports[id as usize], frame.env())
            })?,
            Op::LoopEnter(id) => {
                let spec = &frame.code.loops[id as usize];
                let state = match spec.kind {
                    LoopKind::Repeat => {
                        let n = vm.expect_int(&pop!(), spec.first_span)?;
                        if n < 0 {
                            return Err(vm.err("'repita' espera um número >= 0", spec.first_span));
                        }
                        Iteration::Repeat(n)
                    }
                    LoopKind::Range => {
                        let to = pop!();
                        let from = pop!();
                        let from = vm.expect_int(&from, spec.first_span)?;
                        let to = vm.expect_int(&to, spec.second_span)?;
                        Iteration::Range {
                            next: (from <= to).then_some(from),
                            end: to,
                        }
                    }
                    LoopKind::Items => {
                        let seq = pop!();
                        match seq {
                            Value::Processo(p) => Iteration::Process(p),
                            other => Iteration::Items(
                                vm.iter_items(&other, spec.first_span)?.into_iter(),
                            ),
                        }
                    }
                    LoopKind::While => Iteration::While,
                };
                let scopes = frame.envs.len();
                frame.envs.push(Env::child(frame.env(), FrameKind::Block));
                frame.loops.push(Loop {
                    id: id as usize,
                    state,
                    active: false,
                    values: self.values.len(),
                    destinations: self.destinations.len(),
                    scopes,
                    handlers: frame.handlers.len(),
                });
            }
            Op::LoopNext(id) => {
                let env = Rc::clone(frame.env());
                let looping = frame.loops.last_mut().expect("loop");
                looping.active = false;
                let spec = &frame.code.loops[id as usize];
                let value = match &mut looping.state {
                    Iteration::Repeat(n) => {
                        if *n == 0 {
                            None
                        } else {
                            *n -= 1;
                            Some(Value::Nada)
                        }
                    }
                    Iteration::Range { next, end } => next.take().map(|n| {
                        if n < *end {
                            *next = Some(n + 1);
                        }
                        Value::Numero(n.into())
                    }),
                    Iteration::Items(items) => items.next(),
                    Iteration::Process(p) => vm
                        .processo_proxima(p, spec.first_span, &env)?
                        .map(Value::Texto),
                    Iteration::While => unreachable!(),
                };
                if let Some(value) = value {
                    if let Some(var) = spec.var {
                        let name = &frame.code.names[var as usize];
                        let mut bindings = env.borrow_mut();
                        if let Some(slot) = bindings.vars.get_mut(name) {
                            *slot = value;
                        } else {
                            bindings.define(name.clone(), value);
                        }
                    }
                } else {
                    frame.ip = spec.exit as usize;
                }
            }
            Op::LoopBody => frame.loops.last_mut().expect("loop").active = true,
            Op::LoopEnd => {
                let looping = frame.loops.pop().expect("loop");
                frame.leave_scopes(looping.scopes);
            }
            Op::Break => return Err(EvalError::Break { span }),
            Op::Continue => return Err(EvalError::Continue { span }),
            Op::NameDestination(id) => self.destinations.push(Destination::Name {
                env: Rc::clone(frame.env()),
                name: name!(id).clone(),
                span,
            }),
            Op::ReceiverName(id) => {
                let destination = Destination::Name {
                    env: Rc::clone(frame.env()),
                    name: name!(id).clone(),
                    span,
                };
                self.values.push(vm.read_destination(&destination)?);
                self.destinations.push(destination);
            }
            Op::IndexDestination(compound) => {
                let (object, index) = if compound {
                    let index = pop!();
                    (pop!(), index)
                } else {
                    let object = pop!();
                    (object, pop!())
                };
                let owner = self.destinations.pop().expect("owner");
                self.destinations
                    .push(Vm::index_destination(object, owner, index, span));
            }
            Op::ReceiverIndex => {
                let index = pop!();
                let object = pop!();
                let owner = self.destinations.pop().expect("owner");
                let destination = Vm::index_destination(object, owner, index, span);
                self.values.push(vm.read_destination(&destination)?);
                self.destinations.push(destination);
            }
            Op::ReceiverIndex2 | Op::MatrixDestination => {
                let col = pop!();
                let row = pop!();
                let object = pop!();
                let destination = Destination::Matrix {
                    object,
                    row,
                    col,
                    span,
                };
                if matches!(instruction.op, Op::ReceiverIndex2) {
                    self.values.push(vm.read_destination(&destination)?);
                }
                self.destinations.push(destination);
            }
            Op::ReceiverMap(id) => {
                let object = pop!();
                let value = vm.bytecode_map_get(&object, name!(id), span)?;
                let destination = if matches!(object, Value::Par(_, _)) {
                    Destination::ReadOnly {
                        message: "par não pode ser alterado; :chave e :valor só leem",
                        span,
                    }
                } else {
                    vm.map_destination(object, name!(id), span)?
                };
                self.values.push(value);
                self.destinations.push(destination);
            }
            Op::MapDestination(id) => {
                let object = pop!();
                self.destinations
                    .push(vm.map_destination(object, name!(id), span)?);
            }
            Op::ReceiverReadOnly(literal) => self.destinations.push(Destination::ReadOnly {
                message: if literal {
                    "não é possível alterar um texto literal"
                } else {
                    "não é possível atribuir neste alvo"
                },
                span,
            }),
            Op::ReadDestination => self
                .values
                .push(vm.read_destination(self.destinations.last().expect("destination"))?),
            Op::WriteDestination => {
                let value = pop!();
                vm.write_destination(self.destinations.pop().expect("destination"), value)?;
            }
        }
        Ok(Step::Next)
    }

    fn push(
        &mut self,
        vm: &mut Vm<'_, '_>,
        code: Rc<Chunk>,
        env: Rc<RefCell<Env>>,
        kind: FrameKindCode,
        file: String,
        source: Rc<str>,
        name: String,
        span: Span,
    ) {
        vm.stack.push(CallFrame {
            name,
            file: vm.file.clone(),
            span,
        });
        let restore = Some((
            std::mem::replace(&mut vm.file, file),
            std::mem::replace(&mut vm.source, source),
        ));
        self.frames.push(Frame {
            code,
            ip: 0,
            envs: vec![env],
            handlers: vec![],
            loops: vec![],
            kind,
            values: self.values.len(),
            destinations: self.destinations.len(),
            restore,
        });
    }
    fn finish(&mut self, vm: &mut Vm<'_, '_>) {
        let mut frame = self.frames.pop().expect("frame");
        self.values.truncate(frame.values);
        self.destinations.truncate(frame.destinations);
        frame.handlers_to(0, vm);
        frame.loops.clear();
        frame.leave_scopes(0);
        if let Some((file, source)) = frame.restore {
            vm.file = file;
            vm.source = source;
            vm.stack.pop();
        }
    }
    fn call_stack(
        &mut self,
        vm: &mut Vm<'_, '_>,
        argc: usize,
        name: String,
        span: Span,
    ) -> Result<(), EvalError> {
        let args_start = self.values.len() - argc;
        let stack_base = args_start - 1;
        // Keep a placeholder until arguments have been drained, avoiding a move
        // of every argument merely to remove the callee below them.
        let Value::Funcao(closure) = std::mem::replace(&mut self.values[stack_base], Value::Nada)
        else {
            unreachable!("bytecode call dispatch")
        };
        if argc != closure.params.len() {
            let error = vm.err(
                format!(
                    "`{name}` espera {} argumento(s), recebeu {}",
                    closure.params.len(),
                    argc
                ),
                span,
            );
            self.values.truncate(stack_base);
            return Err(error);
        }
        if let Err(error) = vm.check_profundidade(span) {
            self.values.truncate(stack_base);
            return Err(error);
        }
        let env = Env::child(&closure.env, FrameKind::Function);
        let FunctionBody::Bytecode(code) = &closure.body else {
            unreachable!("bytecode call dispatch")
        };
        code.bind_parameters(&mut env.borrow_mut(), self.values.drain(args_start..));
        self.values.truncate(stack_base);
        self.push(
            vm,
            Rc::clone(code),
            env,
            FrameKindCode::Function,
            closure.file.clone(),
            Rc::clone(&closure.source),
            name,
            span,
        );
        Ok(())
    }
    fn call(
        &mut self,
        vm: &mut Vm<'_, '_>,
        function: Value,
        args: Vec<Value>,
        name: String,
        span: Span,
    ) -> Result<(), EvalError> {
        match &function {
            Value::Builtin("avalie") => {
                vm.expect_arity(&args, 1, span)?;
                let source = vm.expect_texto(&args[0], span)?;
                let program =
                    parse(&source).map_err(|e| vm.err(format!("avalie: {}", e.message), span))?;
                if program.items.is_empty() {
                    return Err(vm.err("avalie espera uma expressão", span));
                }
                vm.check_profundidade(span)?;
                let code = compiler::items(&program.items);
                let env = Rc::clone(self.frames.last().expect("frame").env());
                self.push(
                    vm,
                    code,
                    env,
                    FrameKindCode::Eval,
                    "<avalie>".into(),
                    Rc::from(source),
                    "avalie".into(),
                    span,
                );
            }
            _ => {
                let env = self.frames.last().expect("frame").env();
                let value = stacker::maybe_grow(PILHA_RESERVA, PILHA_SEGMENTO, || {
                    vm.call_value(&function, &args, span, &name, env)
                })?;
                self.values.push(value);
            }
        }
        Ok(())
    }
    fn signal(&mut self, vm: &mut Vm<'_, '_>, mut error: EvalError) -> Result<(), EvalError> {
        loop {
            let frame = self.frames.last_mut().expect("frame");
            if matches!(error, EvalError::Break { .. } | EvalError::Continue { .. })
                && let Some(index) = frame.loops.iter().rposition(|l| l.active)
            {
                frame.loops.truncate(index + 1);
                let looping = &frame.loops[index];
                let (scopes, handlers, values, destinations, id) = (
                    looping.scopes + 1,
                    looping.handlers,
                    looping.values,
                    looping.destinations,
                    looping.id,
                );
                let spec = &frame.code.loops[id];
                frame.ip = if matches!(error, EvalError::Break { .. }) {
                    spec.exit
                } else {
                    spec.next
                } as usize;
                frame.loops[index].active = false;
                self.values.truncate(values);
                self.destinations.truncate(destinations);
                frame.handlers_to(handlers, vm);
                frame.leave_scopes(scopes);
                return Ok(());
            }
            if matches!(error, EvalError::Return { .. }) && frame.kind == FrameKindCode::Function {
                let EvalError::Return { value, .. } = error else {
                    unreachable!()
                };
                self.finish(vm);
                self.values.push(value);
                return Ok(());
            }
            // Control signals cross handlers unchanged, and become errors only
            // at the function/eval boundary, never catchable by that same frame.
            let control_boundary = !matches!(error, EvalError::Runtime(_) | EvalError::Quit(_))
                && frame.kind != FrameKindCode::Plain;
            if control_boundary {
                frame.handlers_to(0, vm);
                error = match error {
                    EvalError::Return { span, .. } => {
                        vm.err("retorne só pode ser usado numa função", span)
                    }
                    EvalError::Break { span } => vm.err("pare só pode ser usado num laço", span),
                    EvalError::Continue { span } => {
                        vm.err("continue só pode ser usado num laço", span)
                    }
                    _ => unreachable!(),
                };
            }
            vm.report_eval_error(&error, frame.env());
            if !control_boundary
                && matches!(error, EvalError::Runtime(_))
                && let Some(handler) = frame.handlers.pop()
            {
                vm.protected -= 1;
                self.values.truncate(handler.values);
                self.destinations.truncate(handler.destinations);
                frame.loops.truncate(handler.loops);
                frame.leave_scopes(handler.scopes);
                frame.ip = handler.target;
                return Ok(());
            }
            self.finish(vm);
            if self.frames.is_empty() {
                return Err(error);
            }
        }
    }
    fn run(&mut self, vm: &mut Vm<'_, '_>) -> Result<Value, EvalError> {
        loop {
            let step = self.step(vm);
            match step {
                Ok(Step::Next) => {}
                Ok(Step::CallStack(argc, name, span)) => {
                    if let Err(error) = self.call_stack(vm, argc, name, span) {
                        self.signal(vm, error)?;
                    }
                }
                Ok(Step::Call(function, args, name, span)) => {
                    if let Err(error) = self.call(vm, function, args, name, span) {
                        self.signal(vm, error)?;
                    }
                }
                Ok(Step::Finish(value)) => {
                    self.finish(vm);
                    if self.frames.is_empty() {
                        return Ok(value);
                    }
                    self.values.push(value);
                }
                Err(error) => {
                    // Return from the root function is a successful completion.
                    if self.frames.len() == 1
                        && self.frames[0].kind == FrameKindCode::Function
                        && matches!(error, EvalError::Return { .. })
                    {
                        let EvalError::Return { value, .. } = error else {
                            unreachable!()
                        };
                        self.finish(vm);
                        return Ok(value);
                    }
                    self.signal(vm, error)?;
                }
            }
        }
    }
}

impl Vm<'_, '_> {
    fn execute_code(
        &mut self,
        code: Rc<Chunk>,
        env: &Rc<RefCell<Env>>,
        kind: FrameKindCode,
    ) -> Result<Value, EvalError> {
        Machine {
            frames: vec![Frame {
                code,
                ip: 0,
                envs: vec![Rc::clone(env)],
                handlers: vec![],
                loops: vec![],
                kind,
                values: 0,
                destinations: 0,
                restore: None,
            }],
            values: vec![],
            destinations: vec![],
        }
        .run(self)
    }
    pub(in crate::runtime::eval) fn execute_module(
        &mut self,
        program: Program,
        env: &Rc<RefCell<Env>>,
    ) -> Result<Value, EvalError> {
        let code = compiler::module(&program.items);
        drop(program);
        self.execute_loaded_module(code, env)
    }
    pub(in crate::runtime::eval) fn execute_loaded_module(
        &mut self,
        code: Rc<Chunk>,
        env: &Rc<RefCell<Env>>,
    ) -> Result<Value, EvalError> {
        self.execute_code(code, env, FrameKindCode::Plain)
    }
    pub(in crate::runtime) fn execute_program_value(
        &mut self,
        program: Program,
        env: &Rc<RefCell<Env>>,
    ) -> Result<Value, EvalError> {
        if self.backend == ExecutionBackend::Ast {
            return self.eval_items_value(&program.items, env);
        }
        let code = compiler::items(&program.items);
        drop(program);
        let result = self.execute_code(code, env, FrameKindCode::Plain);
        self.retorne_so_em_funcao(result)
    }
    pub(in crate::runtime::eval) fn execute_items(
        &mut self,
        items: &[Item],
        env: &Rc<RefCell<Env>>,
    ) -> Result<Value, EvalError> {
        let code = compiler::items(items);
        let result = self.execute_code(code, env, FrameKindCode::Plain);
        self.retorne_so_em_funcao(result)
    }
    pub(in crate::runtime::eval) fn execute_function(
        &mut self,
        code: Rc<Chunk>,
        env: &Rc<RefCell<Env>>,
    ) -> Result<Value, EvalError> {
        self.execute_code(code, env, FrameKindCode::Function)
    }
    fn bytecode_map_get(
        &self,
        object: &Value,
        field: &str,
        span: Span,
    ) -> Result<Value, EvalError> {
        match object {
            Value::Mapa(_) => self.index_get(object, &Value::Texto(field.into()), span),
            Value::Par(key, value) => match field {
                "chave" => Ok(key.to_value()),
                "valor" => Ok(value.as_ref().clone()),
                _ => Err(self.err(format!("par só tem :chave e :valor, não `{field}`"), span)),
            },
            Value::Modulo(_) => {
                Err(self.err(format!("módulo usa '::' — tente o_modulo::{field}"), span))
            }
            other => Err(self.err(
                format!(
                    "acesso ':' espera um mapa, encontrado {}",
                    other.type_name()
                ),
                span,
            )),
        }
    }
}
