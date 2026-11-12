//! Transactional graph copy. Explicit work/identity tables preserve aliases and
//! cycles without following the Rust call stack. New shells are cleared on error.
use super::*;
use crate::runtime::eval::bytecode::detached_function;
use crate::runtime::value::{ListaRc, MapKey, MapaRc, desmontar};

const MAX_WORK: usize = 1_000_000;
#[derive(Hash, PartialEq, Eq, Clone, Copy)]
struct Id(u8, usize);
fn id<T>(kind: u8, rc: &Rc<T>) -> Id {
    Id(kind, Rc::as_ptr(rc) as usize)
}

enum Job {
    Visit(Value, usize),
    List(ListaRc, usize),
    Map(MapaRc, usize),
    Pair(MapKey),
}
struct Path {
    parent: usize,
    part: String,
}
struct Copy {
    jobs: Vec<Job>,
    values: Vec<Value>,
    memo: HashMap<Id, Value>,
    lists: Vec<ListaRc>,
    maps: Vec<MapaRc>,
    paths: Vec<Path>,
    remaining: usize,
    committed: bool,
    stopped: Option<EvalError>,
}
impl Copy {
    fn new(value: Value) -> Self {
        Self {
            jobs: vec![Job::Visit(value, 0)],
            values: vec![],
            memo: HashMap::new(),
            lists: vec![],
            maps: vec![],
            paths: vec![Path {
                parent: 0,
                part: "$".into(),
            }],
            remaining: MAX_WORK,
            committed: false,
            stopped: None,
        }
    }
    fn path(&mut self, parent: usize, part: String) -> usize {
        let index = self.paths.len();
        self.paths.push(Path { parent, part });
        index
    }
    fn location(&self, mut path: usize) -> String {
        let mut parts = vec![];
        for _ in 0..24 {
            parts.push(self.paths[path].part.as_str());
            if path == 0 {
                break;
            }
            path = self.paths[path].parent;
        }
        if path != 0 {
            parts.push("$…");
        }
        parts.reverse();
        parts.concat()
    }
    fn stop(&mut self, error: EvalError) -> String {
        self.stopped = Some(error);
        "clone interrompido".into()
    }
    fn charge(&mut self, n: usize) -> Result<(), String> {
        self.remaining = self
            .remaining
            .checked_sub(n)
            .ok_or("clone excede o orçamento de 1.000.000 unidades")?;
        Ok(())
    }
}
impl Drop for Copy {
    fn drop(&mut self) {
        if !self.committed {
            // Only newly allocated objects are cleared. Originals in jobs/memo
            // are never modified; this also releases partially built cycles.
            for list in &self.lists {
                desmontar(std::mem::take(&mut *list.borrow_mut()));
            }
            for map in &self.maps {
                desmontar(
                    std::mem::take(&mut *map.borrow_mut())
                        .into_iter()
                        .map(|(_, v)| v)
                        .collect(),
                );
            }
        }
        let mut discarded = std::mem::take(&mut self.values);
        for job in std::mem::take(&mut self.jobs) {
            if let Job::Visit(value, _) = job {
                discarded.push(value);
            }
        }
        desmontar(discarded);
    }
}

impl Vm<'_, '_> {
    pub(crate) fn bi_clone(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        self.expect_arity(args, 1, span)?;
        let mut copy = Copy::new(args[0].clone());
        let mut steps = 0usize;
        while let Some(job) = copy.jobs.pop() {
            steps += 1;
            if steps.is_multiple_of(128) {
                self.check_espera(span)?;
            }
            match job {
                Job::Pair(key) => {
                    let value = copy.values.pop().expect("clone pair");
                    copy.values.push(Value::Par(key, Box::new(value)));
                }
                Job::List(list, index) => {
                    list.borrow_mut()[index] = copy.values.pop().expect("clone list")
                }
                Job::Map(map, index) => {
                    map.borrow_mut()[index].1 = copy.values.pop().expect("clone map")
                }
                Job::Visit(value, path) => {
                    let result = self.clone_visit(value, path, &mut copy, span);
                    if let Err(message) = result {
                        if let Some(error) = copy.stopped.take() {
                            return Err(error);
                        }
                        return Err(
                            self.err(format!("clone em {}: {message}", copy.location(path)), span)
                        );
                    }
                }
            }
        }
        self.check_espera(span)?;
        let root = copy.values.pop().expect("clone result");
        copy.committed = true;
        Ok(root)
    }

    fn clone_visit(
        &mut self,
        value: Value,
        path: usize,
        copy: &mut Copy,
        span: Span,
    ) -> Result<(), String> {
        copy.charge(1)?;
        let identity = match &value {
            Value::Lista(v) => Some(id(0, v)),
            Value::Mapa(v) => Some(id(1, v)),
            Value::Conjunto(v) => Some(id(2, v)),
            Value::Matriz(v) => Some(id(3, v)),
            Value::Imagem(v) => Some(id(4, v)),
            Value::Funcao(v) => Some(id(5, v)),
            _ => None,
        };
        if let Some(value) = identity.and_then(|id| copy.memo.get(&id)).cloned() {
            copy.values.push(value);
            return Ok(());
        }
        let cloned = match value {
            Value::Lista(original) => {
                let original = original
                    .try_borrow()
                    .map_err(|_| "lista emprestada pelo host")?;
                if original.len() > copy.remaining {
                    return Err("lista excede o orçamento de clone".into());
                }
                copy.charge(original.len())?;
                let mut values = Vec::new();
                values
                    .try_reserve_exact(original.len())
                    .map_err(|_| "memória insuficiente para clone")?;
                values.resize(original.len(), Value::Nada);
                let v = Value::lista(values);
                let Value::Lista(list) = &v else {
                    unreachable!()
                };
                copy.lists.push(Rc::clone(list));
                copy.memo.insert(identity.unwrap(), v.clone());
                for (i, value) in original.iter().enumerate().rev() {
                    let child = copy.path(path, format!("[{i}]"));
                    copy.jobs.push(Job::List(Rc::clone(list), i));
                    copy.jobs.push(Job::Visit(value.clone(), child));
                }
                v
            }
            Value::Mapa(original) => {
                if original.recurso_nao_clonavel() {
                    return Err("recurso nativo não clonável".into());
                }
                let original = original
                    .try_borrow()
                    .map_err(|_| "mapa emprestado pelo host")?;
                match original
                    .iter()
                    .find(|(k, _)| *k == MapKey::Texto("_clonavel".into()))
                    .map(|(_, v)| v)
                {
                    Some(Value::Bool(false)) => {
                        return Err("mapa marcado com :_clonavel = falso".into());
                    }
                    Some(Value::Bool(true)) | None => {}
                    _ => return Err(":_clonavel deve ser bool".into()),
                }
                if original.len() > copy.remaining {
                    return Err("mapa excede o orçamento de clone".into());
                }
                copy.charge(original.len())?;
                let mut values = Vec::new();
                values
                    .try_reserve_exact(original.len())
                    .map_err(|_| "memória insuficiente para clone")?;
                values.extend(original.iter().map(|(k, _)| (k.clone(), Value::Nada)));
                let v = Value::mapa(values);
                let Value::Mapa(map) = &v else { unreachable!() };
                copy.maps.push(Rc::clone(map));
                copy.memo.insert(identity.unwrap(), v.clone());
                for (i, (key, value)) in original.iter().enumerate().rev() {
                    let child = copy.path(
                        path,
                        match key {
                            MapKey::Texto(k) => format!(":{k}"),
                            other => format!("[{other:?}]"),
                        },
                    );
                    copy.jobs.push(Job::Map(Rc::clone(map), i));
                    copy.jobs.push(Job::Visit(value.clone(), child));
                }
                v
            }
            Value::Conjunto(items) => {
                let items = items
                    .try_borrow()
                    .map_err(|_| "conjunto emprestado pelo host")?;
                copy.charge(items.len())?;
                Value::conjunto(items.clone())
            }
            Value::Matriz(rows) => {
                let rows = rows
                    .try_borrow()
                    .map_err(|_| "matriz emprestada pelo host")?;
                let mut result = Vec::new();
                result
                    .try_reserve_exact(rows.len())
                    .map_err(|_| "memória insuficiente para clone")?;
                for row in rows.iter() {
                    copy.charge(row.len() + 1)?;
                    let mut cells = Vec::new();
                    cells
                        .try_reserve_exact(row.len())
                        .map_err(|_| "memória insuficiente para clone")?;
                    cells.extend_from_slice(row);
                    result.push(cells);
                    self.check_espera(span).map_err(|error| copy.stop(error))?;
                }
                Value::matriz(result)
            }
            Value::Imagem(image) => Value::imagem(
                image
                    .try_borrow()
                    .map_err(|_| "imagem emprestada pelo host")?
                    .clone(),
            ),
            Value::Funcao(function) => {
                let mut poll = || self.check_espera(span).map_err(|error| copy.stop(error));
                Value::Funcao(detached_function(&function, &mut poll)?)
            }
            Value::Par(key, value) => {
                let child = copy.path(path, ":valor".into());
                copy.jobs.push(Job::Pair(key));
                copy.jobs.push(Job::Visit(*value, child));
                return Ok(());
            }
            Value::Modulo(_) => return Err("módulo não suporta clone".into()),
            #[cfg(feature = "process")]
            Value::Processo(_) => return Err("processo não suporta clone".into()),
            scalar => scalar,
        };
        if let Some(id) = identity {
            copy.memo.insert(id, cloned.clone());
        }
        copy.values.push(cloned);
        Ok(())
    }
}
