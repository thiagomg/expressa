//! Bounded control-flow verification before untrusted file instructions reach
//! VM paths that assume compiler-owned stack/scope invariants.
use super::*;
use std::collections::VecDeque;

const MAX_WORK: usize = 4_000_000;
#[derive(Clone, PartialEq, Eq)]
enum Scope {
    Root,
    Block,
    Loop(u32),
    Parent(u32),
}
#[derive(Clone, PartialEq, Eq)]
struct Loop {
    id: u32,
    values: usize,
    dest: usize,
    scopes: usize,
    handlers: usize,
    active: bool,
}
#[derive(Clone, PartialEq, Eq)]
struct Handler {
    target: usize,
    values: usize,
    dest: usize,
    scopes: usize,
    loops: usize,
}
#[derive(Clone, PartialEq, Eq)]
struct State {
    values: usize,
    dest: usize,
    scopes: Vec<Scope>,
    loops: Vec<Loop>,
    handlers: Vec<Handler>,
}
impl State {
    fn effect(&mut self, pop: usize, push: usize, dp: usize, dpush: usize) -> Result<()> {
        let values = self
            .values
            .checked_sub(pop)
            .ok_or_else(|| error("pilha de valores EXB insuficiente"))?;
        let dest = self
            .dest
            .checked_sub(dp)
            .ok_or_else(|| error("pilha de destinos EXB insuficiente"))?;
        if self
            .handlers
            .iter()
            .any(|h| values < h.values || dest < h.dest)
            || self
                .loops
                .iter()
                .any(|l| values < l.values || dest < l.dest)
        {
            return Err(error(
                "operação EXB atravessa a base de uma pilha protegida",
            ));
        }
        self.values = values
            .checked_add(push)
            .ok_or_else(|| error("pilha EXB grande demais"))?;
        self.dest = dest
            .checked_add(dpush)
            .ok_or_else(|| error("pilha EXB grande demais"))?;
        Ok(())
    }
    fn scope_floor(&self, depth: usize) -> Result<()> {
        if self.handlers.iter().any(|h| h.scopes > depth)
            || self.loops.iter().any(|l| l.scopes + 1 > depth)
        {
            return Err(error("escopo EXB atravessa estrutura protegida"));
        }
        Ok(())
    }
    fn weight(&self) -> usize {
        1 + self.scopes.len() + self.loops.len() * 6 + self.handlers.len() * 5
    }
}

pub(super) fn program(program: &CompiledProgram) -> Result<()> {
    let mut spans = vec![program.span];
    let mut work = MAX_WORK;
    chunk(&program.code, 0, 0, &mut work, &mut spans)?;
    // Validate UTF-8 offsets first, then line/character columns in one source
    // traversal rather than rescanning long lines for every span.
    spans.retain(|s| *s != Span::default());
    for s in &spans {
        if s.start > s.end
            || s.end > program.source.len()
            || !program.source.is_char_boundary(s.start)
            || !program.source.is_char_boundary(s.end)
        {
            return Err(error("posição EXB fora do fonte"));
        }
    }
    spans.sort_unstable_by_key(|s| s.start);
    let mut spans = spans.into_iter().peekable();
    let (mut line, mut col) = (1, 1);
    for (offset, ch) in program
        .source
        .char_indices()
        .chain(std::iter::once((program.source.len(), '\0')))
    {
        while spans.peek().is_some_and(|s| s.start == offset) {
            let s = spans.next().unwrap();
            if s.line != line || s.col != col {
                return Err(error("linha/coluna EXB inconsistente com o fonte"));
            }
        }
        if ch == '\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
    }
    if spans.next().is_some() {
        return Err(error("posição EXB inválida"));
    }
    Ok(())
}
fn index(id: u32, len: usize) -> Result<()> {
    if id as usize >= len {
        Err(error("índice EXB fora da tabela"))
    } else {
        Ok(())
    }
}
fn chunk(
    code: &Chunk,
    params: usize,
    depth: usize,
    work: &mut usize,
    spans: &mut Vec<Span>,
) -> Result<()> {
    if depth > MAX_DEPTH || code.instructions.is_empty() {
        return Err(error("bloco EXB vazio ou profundo demais"));
    }
    spans.extend_from_slice(&code.spans);
    for f in &code.functions {
        spans.push(f.span);
        spans.extend(f.params.iter().map(|p| p.span));
        chunk(&f.code, f.params.len(), depth + 1, work, spans)?;
    }
    spans.extend(code.imports.iter().map(|i| i.span));
    let n = code.instructions.len();
    for (id, l) in code.loops.iter().enumerate() {
        spans.extend([l.first_span, l.second_span]);
        index(l.exit, n)?;
        index(l.next, n)?;
        if l.exit <= l.next || !matches!(code.instructions[l.exit as usize].op, Op::LoopEnd) {
            return Err(error("destinos de laço EXB inválidos"));
        }
        match l.kind {
            LoopKind::While
                if !matches!(code.instructions[l.next as usize].op, Op::ParentScope) =>
            {
                return Err(error("condição de laço EXB inválida"));
            }
            LoopKind::Repeat | LoopKind::Range | LoopKind::Items if !matches!(code.instructions[l.next as usize].op, Op::LoopNext(i) if i as usize == id) =>
            {
                return Err(error("avanço de laço EXB inválido"));
            }
            _ => {}
        }
        if let Some(v) = l.var {
            index(v, code.names.len())?;
        }
    }
    for (ip, i) in code.instructions.iter().enumerate() {
        index(i.span, code.spans.len())?;
        if code.spans[i.span as usize] == Span::default() && !matches!(i.op, Op::Nada | Op::Finish)
        {
            return Err(error("instrução EXB sem posição de fonte"));
        }
        match i.op {
            Op::Constant(id) => index(id, code.constants.len())?,
            Op::BadNumber(id)
            | Op::Load(id)
            | Op::Store(id)
            | Op::Field(id)
            | Op::MapField(id)
            | Op::NameDestination(id)
            | Op::ReceiverName(id)
            | Op::ReceiverMap(id)
            | Op::MapDestination(id) => index(id, code.names.len())?,
            Op::Call { name, .. } => index(name, code.names.len())?,
            Op::LoadLocal(id) | Op::StoreLocal(id) => index(id, params)?,
            Op::Closure(id) => index(id, code.functions.len())?,
            Op::Import(id) => index(id, code.imports.len())?,
            Op::Jump(id)
            | Op::BranchFalse(id)
            | Op::ShortCircuit { target: id, .. }
            | Op::Try(id) => index(id, n)?,
            Op::LoopEnter(id) => {
                index(id, code.loops.len())?;
                if code.loops[id as usize].next as usize != ip + 1 {
                    return Err(error("entrada de laço EXB inválida"));
                }
            }
            Op::LoopNext(id) => {
                index(id, code.loops.len())?;
                if code.loops[id as usize].next as usize != ip
                    || matches!(code.loops[id as usize].kind, LoopKind::While)
                {
                    return Err(error("avanço de laço EXB inválido"));
                }
            }
            _ => {}
        }
    }
    let initial = State {
        values: 0,
        dest: 0,
        scopes: vec![Scope::Root],
        loops: vec![],
        handlers: vec![],
    };
    let mut queue = VecDeque::from([(0usize, initial, None::<usize>)]);
    let mut edges: Vec<Vec<usize>> = vec![Vec::new(); n];
    let mut seen: HashMap<usize, State> = HashMap::new();
    while let Some((ip, mut s, from)) = queue.pop_front() {
        *work = work
            .checked_sub(s.weight())
            .ok_or_else(|| error("verificação EXB complexa demais"))?;
        if ip >= n {
            return Err(error("fluxo EXB sai do bloco sem terminar"));
        }
        if let Some(from) = from {
            edges[from].push(ip);
        }
        let op = code.instructions[ip].op;
        if matches!(op, Op::LoopNext(_) | Op::ParentScope | Op::LoopBody)
            && let Some(l) = s.loops.last_mut()
        {
            l.active = false;
        }
        if let Some(old) = seen.get(&ip) {
            if old != &s {
                return Err(error(format!(
                    "pilhas/escopos EXB incompatíveis na instrução {ip}"
                )));
            }
            continue;
        }
        *work = work
            .checked_sub(s.weight())
            .ok_or_else(|| error("verificação EXB complexa demais"))?;
        seen.insert(ip, s.clone());
        // Every runtime failure may unwind to the nearest protected expression.
        if let Some(h) = s.handlers.last().cloned() {
            let mut catch = s.clone();
            catch.handlers.pop();
            catch.values = h.values;
            catch.dest = h.dest;
            catch.scopes.truncate(h.scopes);
            catch.loops.truncate(h.loops);
            queue.push_back((h.target, catch, Some(ip)));
        }
        let mut fallthrough = true;
        match op {
            Op::Nada | Op::Bool(_) | Op::Constant(_) | Op::Load(_) | Op::Closure(_) => {
                s.effect(0, 1, 0, 0)?
            }
            Op::LoadLocal(_) | Op::StoreLocal(_) => {
                if s.scopes != [Scope::Root] {
                    return Err(error("slot EXB fora do escopo da função"));
                }
                s.effect(
                    usize::from(matches!(op, Op::StoreLocal(_))),
                    usize::from(matches!(op, Op::LoadLocal(_))),
                    0,
                    0,
                )?;
            }
            Op::Store(_) | Op::Drop => s.effect(1, 0, 0, 0)?,
            Op::SetLast | Op::Pair | Op::Binary(_) | Op::Index => s.effect(2, 1, 0, 0)?,
            Op::List(count) => s.effect(count as usize, 1, 0, 0)?,
            Op::Unary(_) | Op::Field(_) | Op::MapField(_) | Op::CheckBool => {
                s.effect(1, 1, 0, 0)?
            }
            Op::Index2 => s.effect(3, 1, 0, 0)?,
            Op::Slice(end) => s.effect(if end { 3 } else { 2 }, 1, 0, 0)?,
            Op::Call { argc, .. } => s.effect(
                (argc as usize)
                    .checked_add(1)
                    .ok_or_else(|| error("aridade EXB fora do intervalo"))?,
                1,
                0,
                0,
            )?,
            Op::Return => {
                s.effect(1, 0, 0, 0)?;
                fallthrough = false;
            }
            Op::Finish => {
                if s.values != 1
                    || s.dest != 0
                    || s.scopes != [Scope::Root]
                    || !s.loops.is_empty()
                    || !s.handlers.is_empty()
                {
                    return Err(error("término EXB com pilhas abertas"));
                }
                fallthrough = false;
            }
            Op::BadNumber(_) => fallthrough = false,
            Op::Checkpoint | Op::Import(_) => {}
            Op::CheckInt => {
                if s.values == 0 {
                    return Err(error("pilha EXB vazia na validação de inteiro"));
                }
            }
            Op::EnterScope => s.scopes.push(Scope::Block),
            Op::LeaveScope => {
                if s.scopes.pop() != Some(Scope::Block) {
                    return Err(error("saída de escopo EXB inválida"));
                }
                s.scope_floor(s.scopes.len())?;
            }
            Op::ParentScope => {
                let l = s
                    .loops
                    .last()
                    .ok_or_else(|| error("condição EXB sem laço"))?;
                if !matches!(code.loops[l.id as usize].kind, LoopKind::While)
                    || s.scopes.last() != Some(&Scope::Loop(l.id))
                    || s.scopes.len() != l.scopes + 1
                {
                    return Err(error("escopo de condição EXB inválido"));
                }
                s.scopes.push(Scope::Parent(l.id));
            }
            Op::RestoreScope => {
                let l = s
                    .loops
                    .last()
                    .ok_or_else(|| error("restauração EXB sem laço"))?;
                if s.scopes.pop() != Some(Scope::Parent(l.id)) {
                    return Err(error("restauração de escopo EXB inválida"));
                }
                s.scope_floor(s.scopes.len())?;
            }
            Op::Jump(target) => {
                queue.push_back((target as usize, s.clone(), Some(ip)));
                fallthrough = false;
            }
            Op::BranchFalse(target) => {
                s.effect(1, 0, 0, 0)?;
                queue.push_back((target as usize, s.clone(), Some(ip)));
            }
            Op::ShortCircuit { target, .. } => {
                s.effect(1, 0, 0, 0)?;
                let mut jump = s.clone();
                jump.effect(0, 1, 0, 0)?;
                queue.push_back((target as usize, jump, Some(ip)));
            }
            Op::Try(target) => s.handlers.push(Handler {
                target: target as usize,
                values: s.values,
                dest: s.dest,
                scopes: s.scopes.len(),
                loops: s.loops.len(),
            }),
            Op::EndTry => {
                if s.handlers.pop().is_none()
                    || s.loops.iter().any(|l| l.handlers > s.handlers.len())
                {
                    return Err(error("fim de proteção EXB inválido"));
                }
            }
            Op::LoopEnter(id) => {
                let spec = &code.loops[id as usize];
                let count = match spec.kind {
                    LoopKind::While => 0,
                    LoopKind::Range => 2,
                    _ => 1,
                };
                s.effect(count, 0, 0, 0)?;
                s.loops.push(Loop {
                    id,
                    values: s.values,
                    dest: s.dest,
                    scopes: s.scopes.len(),
                    handlers: s.handlers.len(),
                    active: false,
                });
                s.scopes.push(Scope::Loop(id));
            }
            Op::LoopNext(id) => {
                let l = s.loops.last().ok_or_else(|| error("avanço EXB sem laço"))?;
                if l.id != id
                    || s.values != l.values
                    || s.dest != l.dest
                    || s.scopes.len() != l.scopes + 1
                    || s.handlers.len() != l.handlers
                {
                    return Err(error("pilhas EXB desequilibradas no laço"));
                }
                queue.push_back((code.loops[id as usize].exit as usize, s.clone(), Some(ip)));
            }
            Op::LoopBody => {
                s.loops
                    .last_mut()
                    .ok_or_else(|| error("corpo EXB sem laço"))?
                    .active = true
            }
            Op::LoopEnd => {
                let l = s.loops.pop().ok_or_else(|| error("fim EXB sem laço"))?;
                if code.loops[l.id as usize].exit as usize != ip
                    || s.values != l.values
                    || s.dest != l.dest
                    || s.scopes.len() != l.scopes + 1
                    || s.handlers.len() != l.handlers
                {
                    return Err(error("fim de laço EXB desequilibrado"));
                }
                s.scopes.truncate(l.scopes);
            }
            Op::Break | Op::Continue => {
                fallthrough = false;
                if let Some(i) = s.loops.iter().rposition(|l| l.active) {
                    s.loops.truncate(i + 1);
                    let l = s.loops[i].clone();
                    s.loops[i].active = false;
                    if l.handlers > s.handlers.len() || l.scopes + 1 > s.scopes.len() {
                        return Err(error("desvio de laço EXB inválido"));
                    }
                    s.handlers.truncate(l.handlers);
                    s.scopes.truncate(l.scopes + 1);
                    s.values = l.values;
                    s.dest = l.dest;
                    let spec = &code.loops[l.id as usize];
                    queue.push_back((
                        if matches!(op, Op::Break) {
                            spec.exit
                        } else {
                            spec.next
                        } as usize,
                        s.clone(),
                        Some(ip),
                    ));
                }
            }
            Op::NameDestination(_) | Op::ReceiverReadOnly(_) => s.effect(0, 0, 0, 1)?,
            Op::ReceiverName(_) => s.effect(0, 1, 0, 1)?,
            Op::IndexDestination(_) => s.effect(2, 0, 1, 1)?,
            Op::ReceiverIndex => s.effect(2, 1, 1, 1)?,
            Op::ReceiverIndex2 => s.effect(3, 1, 0, 1)?,
            Op::MatrixDestination => s.effect(3, 0, 0, 1)?,
            Op::ReceiverMap(_) => s.effect(1, 1, 0, 1)?,
            Op::MapDestination(_) => s.effect(1, 0, 0, 1)?,
            Op::ReadDestination => {
                if s.dest == 0 {
                    return Err(error("leitura EXB sem destino"));
                }
                s.effect(0, 1, 0, 0)?;
            }
            Op::WriteDestination => s.effect(1, 0, 1, 0)?,
        }
        if fallthrough {
            queue.push_back((ip + 1, s, Some(ip)));
        }
    }
    // Every reachable cycle must pass an execution checkpoint. Otherwise a
    // hand-written Jump cycle could bypass the VM's cooperative cancellation.
    let live = |ip: usize| !matches!(code.instructions[ip].op, Op::Checkpoint);
    let mut indegree = vec![0usize; n];
    for (from, targets) in edges.iter().enumerate() {
        if live(from) {
            for &to in targets {
                if live(to) {
                    indegree[to] += 1;
                }
            }
        }
    }
    let mut ready: VecDeque<_> = (0..n).filter(|&ip| live(ip) && indegree[ip] == 0).collect();
    let mut removed = 0;
    while let Some(from) = ready.pop_front() {
        removed += 1;
        for &to in &edges[from] {
            if live(to) {
                indegree[to] -= 1;
                if indegree[to] == 0 {
                    ready.push_back(to);
                }
            }
        }
    }
    if removed != (0..n).filter(|&ip| live(ip)).count() {
        return Err(error("ciclo EXB sem checkpoint de execução"));
    }
    Ok(())
}
