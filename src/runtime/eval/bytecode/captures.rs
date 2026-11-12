//! Capture analysis on executable instructions, including loaded EXB. Debug source
//! is deliberately not parsed: it is not authoritative for serialized code.
use super::*;
use std::collections::VecDeque;

type Names = HashSet<String>;
#[derive(Clone, PartialEq, Eq)]
struct LoopState {
    id: u32,
    scopes: usize,
    handlers: usize,
    active: bool,
    values: usize,
}
#[derive(Clone, PartialEq, Eq)]
struct Handler {
    target: usize,
    scopes: Vec<Names>,
    loops: usize,
    values: Vec<Option<&'static str>>,
}
#[derive(Clone, PartialEq, Eq)]
struct State {
    values: Vec<Option<&'static str>>,
    scopes: Vec<Names>,
    loops: Vec<LoopState>,
    handlers: Vec<Handler>,
}

fn intersect(a: &mut [Names], b: &[Names]) -> Result<bool, String> {
    if a.len() != b.len() {
        return Err("fluxo não classificável para clone".into());
    }
    let mut changed = false;
    for (a, b) in a.iter_mut().zip(b) {
        let n = a.len();
        a.retain(|name| b.contains(name));
        changed |= a.len() != n;
    }
    Ok(changed)
}
fn merge_values(
    a: &mut [Option<&'static str>],
    b: &[Option<&'static str>],
) -> Result<bool, String> {
    if a.len() != b.len() {
        return Err("pilha não classificável para clone".into());
    }
    let mut changed = false;
    for (a, b) in a.iter_mut().zip(b) {
        if a != b && a.is_some() {
            *a = None;
            changed = true;
        }
    }
    Ok(changed)
}
fn effect(s: &mut State, pop: usize, push: usize) -> Result<(), String> {
    let n = s
        .values
        .len()
        .checked_sub(pop)
        .ok_or("pilha inválida para clone")?;
    s.values.truncate(n);
    s.values.resize(n + push, None);
    Ok(())
}

fn merge(a: &mut State, b: &State) -> Result<bool, String> {
    if a.loops != b.loops || a.handlers.len() != b.handlers.len() {
        return Err("fluxo não classificável para clone".into());
    }
    let mut changed = merge_values(&mut a.values, &b.values)?;
    changed |= intersect(&mut a.scopes, &b.scopes)?;
    for (a, b) in a.handlers.iter_mut().zip(&b.handlers) {
        if a.target != b.target || a.loops != b.loops {
            return Err("handlers não classificáveis para clone".into());
        }
        changed |= intersect(&mut a.scopes, &b.scopes)?;
        changed |= merge_values(&mut a.values, &b.values)?;
    }
    Ok(changed)
}

/// Clone a function only after proving that its executable body and any nested
/// prototypes depend on locals/parameters or stable núcleo natives. The new
/// closure does not retain any frame of the original program.
pub(crate) fn detached_function(
    closure: &Closure,
    poll: &mut dyn FnMut() -> Result<(), String>,
) -> Result<Rc<Closure>, String> {
    let root = match &closure.body {
        FunctionBody::Bytecode(c) => Rc::clone(c),
        #[cfg(feature = "ast")]
        FunctionBody::Ast(body) => compiler::capture_body(&closure.params, body),
    };
    let mut todo = vec![(
        root,
        closure
            .params
            .iter()
            .map(|p| p.name.clone())
            .collect::<Names>(),
        Names::new(),
        0usize,
    )];
    let mut natives = HashMap::new();
    let mut budget = 200_000usize;
    let mut steps = 0usize;
    while let Some((code, params, inherited, depth)) = todo.pop() {
        if depth > 64 {
            return Err("clone: funções aninhadas excedem o limite de análise (64)".into());
        }
        let mut queue = VecDeque::from([(
            0usize,
            State {
                values: vec![],
                scopes: vec![params],
                loops: vec![],
                handlers: vec![],
            },
        )]);
        let mut seen: HashMap<usize, State> = HashMap::new();
        while let Some((ip, mut s)) = queue.pop_front() {
            budget = budget
                .checked_sub(
                    1 + s.values.len()
                        + s.scopes.len()
                        + s.scopes
                            .iter()
                            .flat_map(|n| n.iter())
                            .map(|n| 1 + n.len())
                            .sum::<usize>()
                        + s.loops.len() * 8
                        + s.handlers
                            .iter()
                            .map(|h| {
                                1 + h.values.len()
                                    + h.scopes
                                        .iter()
                                        .flat_map(|n| n.iter())
                                        .map(|n| 1 + n.len())
                                        .sum::<usize>()
                            })
                            .sum::<usize>(),
                )
                .ok_or("clone: análise de capturas excede o orçamento")?;
            steps += 1;
            if steps.is_multiple_of(64) {
                poll()?;
            }
            let op = code
                .instructions
                .get(ip)
                .ok_or("fluxo inválido na análise de clone")?
                .op;
            if matches!(
                op,
                Op::LoopNext(_) | Op::ParentScope | Op::LoopBody | Op::LoopEnd
            ) && let Some(l) = s.loops.last_mut()
            {
                l.active = false;
            }
            if let Some(old) = seen.get_mut(&ip) {
                if !merge(old, &s)? {
                    continue;
                }
                s = old.clone();
            } else {
                seen.insert(ip, s.clone());
            }
            // An attempt can fail before publishing a local. Its entry bindings
            // are a conservative lower bound on names available in the handler.
            if let Some(h) = s.handlers.last().cloned() {
                let mut caught = s.clone();
                caught.scopes = h.scopes;
                caught.values = h.values;
                caught.loops.truncate(h.loops);
                caught.handlers.pop();
                queue.push_back((h.target, caught));
            }
            let known = |name: &str, scopes: &[Names]| {
                inherited.contains(name) || scopes.iter().any(|n| n.contains(name))
            };
            // Track callable provenance from operands, not the diagnostic Call
            // name. An EXB producer can choose arbitrary diagnostic names.
            match op {
                Op::Nada
                | Op::Bool(_)
                | Op::Constant(_)
                | Op::Load(_)
                | Op::LoadLocal(_)
                | Op::Closure(_)
                | Op::ReceiverName(_)
                | Op::ReadDestination => effect(&mut s, 0, 1)?,
                Op::Store(_)
                | Op::StoreLocal(_)
                | Op::Drop
                | Op::WriteDestination
                | Op::MapDestination(_) => effect(&mut s, 1, 0)?,
                Op::Pair | Op::Binary(_) | Op::Index | Op::ReceiverIndex => effect(&mut s, 2, 1)?,
                Op::SetLast => {
                    let last = s
                        .values
                        .last()
                        .copied()
                        .ok_or("pilha inválida para clone")?;
                    effect(&mut s, 2, 1)?;
                    *s.values.last_mut().unwrap() = last;
                }
                Op::List(n) => effect(&mut s, n as usize, 1)?,
                Op::Unary(_)
                | Op::Field(_)
                | Op::MapField(_)
                | Op::CheckBool
                | Op::ReceiverMap(_) => effect(&mut s, 1, 1)?,
                Op::Index2 | Op::ReceiverIndex2 => effect(&mut s, 3, 1)?,
                Op::Slice(end) => effect(&mut s, if end { 3 } else { 2 }, 1)?,
                Op::IndexDestination(_) => effect(&mut s, 2, 0)?,
                Op::MatrixDestination => effect(&mut s, 3, 0)?,
                Op::Call { argc, .. } => {
                    let n = (argc as usize)
                        .checked_add(1)
                        .ok_or("aridade inválida para clone")?;
                    let base = s
                        .values
                        .len()
                        .checked_sub(n)
                        .ok_or("pilha inválida para clone")?;
                    let native=s.values[base].ok_or("função tem chamada indireta; não é possível provar ausência de acesso ao ambiente")?;
                    if matches!(native, "avalie" | "catalogo" | "catálogo" | "ordene") {
                        return Err(format!("função pode consultar o ambiente por `{native}`"));
                    }
                    effect(&mut s, n, 1)?;
                }
                Op::LoopEnter(id) => {
                    effect(
                        &mut s,
                        match code.loops[id as usize].kind {
                            LoopKind::While => 0,
                            LoopKind::Range => 2,
                            _ => 1,
                        },
                        0,
                    )?;
                }
                Op::BranchFalse(_) | Op::ShortCircuit { .. } => effect(&mut s, 1, 0)?,
                Op::BadNumber(_) => continue,
                _ => {}
            }
            match op {
                Op::Load(id) | Op::ReceiverName(id) => {
                    let name = &code.names[id as usize];
                    if !known(name, &s.scopes) {
                        let native = resolve_native(closure, name, &mut natives)?;
                        *s.values.last_mut().unwrap() = Some(native);
                    }
                }
                Op::Store(id) | Op::NameDestination(id) => {
                    let name = &code.names[id as usize];
                    if !known(name, &s.scopes) {
                        // Assignment to an existing outer binding is itself an
                        // environmental dependency (and normally an error).
                        if has_binding(&closure.env, name) {
                            resolve_native(closure, name, &mut natives)?;
                        } else if matches!(op, Op::Store(_)) {
                            s.scopes.last_mut().unwrap().insert(name.clone());
                        }
                    }
                }
                Op::Closure(id) => {
                    let proto = &code.functions[id as usize];
                    let mut visible = inherited.clone();
                    for scope in &s.scopes {
                        visible.extend(scope.iter().cloned());
                    }
                    todo.push((
                        Rc::clone(&proto.code),
                        proto.params.iter().map(|p| p.name.clone()).collect(),
                        visible,
                        depth + 1,
                    ));
                }
                Op::Import(_) => {
                    return Err(
                        "função com importação não pode ser classificada sem capturas".into(),
                    );
                }
                Op::EnterScope => s.scopes.push(Names::new()),
                Op::LeaveScope => {
                    s.scopes.pop().ok_or("escopo inválido para clone")?;
                }
                Op::ParentScope => {
                    let parent = s
                        .scopes
                        .get(
                            s.scopes
                                .len()
                                .checked_sub(2)
                                .ok_or("escopo inválido para clone")?,
                        )
                        .ok_or("escopo inválido para clone")?
                        .clone();
                    s.scopes.push(parent);
                }
                Op::RestoreScope => {
                    s.scopes.pop().ok_or("escopo inválido para clone")?;
                }
                Op::LoopEnter(id) => {
                    s.loops.push(LoopState {
                        id,
                        scopes: s.scopes.len(),
                        handlers: s.handlers.len(),
                        values: s.values.len(),
                        active: false,
                    });
                    s.scopes.push(Names::new());
                }
                Op::LoopNext(id) => {
                    let spec = &code.loops[id as usize];
                    queue.push_back((spec.exit as usize, s.clone()));
                    if let Some(var) = spec.var {
                        s.scopes
                            .last_mut()
                            .unwrap()
                            .insert(code.names[var as usize].clone());
                    }
                }
                Op::LoopBody => s.loops.last_mut().ok_or("laço inválido para clone")?.active = true,
                Op::LoopEnd => {
                    let l = s.loops.pop().ok_or("laço inválido para clone")?;
                    s.scopes.truncate(l.scopes);
                }
                Op::Try(target) => s.handlers.push(Handler {
                    target: target as usize,
                    scopes: s.scopes.clone(),
                    loops: s.loops.len(),
                    values: s.values.clone(),
                }),
                Op::EndTry => {
                    s.handlers.pop().ok_or("handler inválido para clone")?;
                }
                Op::Break | Op::Continue => {
                    if let Some(pos) = s.loops.iter().rposition(|l| l.active) {
                        let l = s.loops[pos].clone();
                        let spec = &code.loops[l.id as usize];
                        s.scopes.truncate(l.scopes + 1);
                        s.handlers.truncate(l.handlers);
                        s.loops.truncate(pos + 1);
                        s.values.truncate(l.values);
                        queue.push_back((
                            if matches!(op, Op::Break) {
                                spec.exit
                            } else {
                                spec.next
                            } as usize,
                            s,
                        ));
                    }
                    continue;
                }
                Op::Return | Op::Finish => continue,
                Op::Jump(target) => {
                    queue.push_back((target as usize, s));
                    continue;
                }
                Op::BranchFalse(target) | Op::ShortCircuit { target, .. } => {
                    let mut jump = s.clone();
                    if matches!(op, Op::ShortCircuit { .. }) {
                        jump.values.push(None);
                    }
                    queue.push_back((target as usize, jump));
                }
                _ => {}
            }
            queue.push_back((ip + 1, s));
        }
    }
    poll()?;
    let env = Env::new(FrameKind::Builtins, None);
    for (name, value) in natives {
        env.borrow_mut().define(name, value);
    }
    Ok(Rc::new(Closure {
        params: closure.params.clone(),
        body: closure.body.clone(),
        env,
        span: closure.span,
        file: closure.file.clone(),
        source: Rc::clone(&closure.source),
        doc: closure.doc.clone(),
    }))
}

fn has_binding(env: &Rc<RefCell<Env>>, name: &str) -> bool {
    let mut current = Some(Rc::clone(env));
    while let Some(cell) = current {
        let frame = cell.borrow();
        if frame.vars.contains_key(name) {
            return true;
        }
        current = frame.parent.clone();
    }
    false
}

fn resolve_native(
    closure: &Closure,
    name: &str,
    natives: &mut HashMap<String, Value>,
) -> Result<&'static str, String> {
    let mut env = Some(Rc::clone(&closure.env));
    while let Some(frame) = env {
        let frame = frame.borrow();
        if let Some(value) = frame.vars.get(name) {
            if frame.kind == FrameKind::Builtins
                && let Value::Builtin(canonical) = value
            {
                if matches!(*canonical, "avalie" | "catalogo" | "catálogo") {
                    return Err(format!(
                        "função consulta o ambiente dinamicamente por `{name}`"
                    ));
                }
                natives.insert(name.into(), value.clone());
                return Ok(canonical);
            }
            break;
        }
        env = frame.parent.clone();
    }
    Err(format!(
        "função captura ou depende do nome externo `{name}`"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn function(source: &str) -> Closure {
        let code = compiler::items(&parse(source).unwrap().items);
        let proto = &code.functions[0];
        Closure {
            params: proto.params.clone(),
            body: FunctionBody::Bytecode(Rc::clone(&proto.code)),
            env: Env::new(FrameKind::Module, None),
            span: proto.span,
            file: "test.lep".into(),
            source: "not valid Expressa source".into(),
            doc: None,
        }
    }
    #[test]
    fn analysis_uses_instructions_and_detaches_original_environment() {
        let f = function("f = funcao(x) { x + 1 }");
        let weak = Rc::downgrade(&f.env);
        let cloned = detached_function(&f, &mut || Ok(())).unwrap();
        assert!(!Rc::ptr_eq(&f.env, &cloned.env));
        drop(f);
        assert!(weak.upgrade().is_none());
    }
    #[test]
    fn forged_call_display_name_does_not_authorize_indirect_execution() {
        let mut f = function("f = funcao(callback) { callback(1) }");
        #[cfg(feature = "ast")]
        let FunctionBody::Bytecode(code) = &mut f.body else {
            unreachable!()
        };
        #[cfg(not(feature = "ast"))]
        let FunctionBody::Bytecode(code) = &mut f.body;
        let code = Rc::get_mut(code).unwrap();
        code.names.push("tipo".into());
        let name = (code.names.len() - 1) as u32;
        for instruction in &mut code.instructions {
            if let Op::Call { name: n, .. } = &mut instruction.op {
                *n = name;
            }
        }
        f.env = Env::new(FrameKind::Builtins, None);
        f.env.borrow_mut().define("tipo", Value::Builtin("tipo"));
        let Err(error) = detached_function(&f, &mut || Ok(())) else {
            panic!("forged call accepted")
        };
        assert!(error.contains("chamada indireta"), "{error}");
    }
}
