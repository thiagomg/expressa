//! Nested scopes (frames) for Expressa.
//!
//! An [`Env`] is one frame: a map of names plus an optional parent. Lookup
//! always walks toward the root. Assignment is the interesting part: it
//! *updates* a name if it already exists in some ancestor, otherwise it
//! *declares* it in the current frame.
//!
//! Typical chain while a program runs:
//!
//! ```text
//! Builtins  (escreva, leia, argumentos, …)  read-only
//!    └── Module  (top-level of a .lep file)
//!           └── Block     (`inicio` … `fim`, `se`, `para`, `enquanto`, `repita`)
//!                  └── Function   (call frame: params + locals)
//! ```
//!
//! # Reading — `get`
//!
//! Walks this frame, then parent, then grandparent, … until the name is
//! found. Missing name → `None` (the evaluator turns that into an error).
//!
//! ```text
//! x = 10
//! inicio
//!     escreva(x)    // 10  — not in the block, found in the module
//! fim
//! ```
//!
//! # Writing — `assign`
//!
//! 1. Search for `nome` from the current frame toward the root.
//! 2. If found **and** we did not walk out of a function to get there,
//!    update that binding in place.
//! 3. If found only after leaving a [`FrameKind::Function`], error:
//!    functions may *read* outer names, not change them.
//! 4. If found in [`FrameKind::Builtins`], error: `escreva = 1` is illegal.
//! 5. If never found, declare `nome` in the **current** frame.
//!
//! ## Blocks *can* change outer variables
//!
//! ```text
//! soma = 0
//! para nota em notas
//! inicio
//!     soma = soma + nota   // updates the module's `soma`
//! fim
//! ```
//!
//! A new name inside a block stays in that block:
//!
//! ```text
//! inicio
//!     y = 20
//! fim
//! // y does not exist here
//! ```
//!
//! ## Functions *cannot* change outer variables
//!
//! ```text
//! x = 1
//! f = funcao()
//! inicio
//!     escreva(x)   // ok, reads 1
//!     x = 2        // error: função não pode alterar `x`
//! fim
//! ```
//!
//! A new name inside the function is a local (even if the same spelling
//! exists outside, assignment to an *existing* outer name is the error;
//! a truly new name is declared in the function frame):
//!
//! ```text
//! f = funcao()
//! inicio
//!     n = 1
//!     n = 2        // ok, `n` lives in the function
//! fim
//! ```
//!
//! # `define` vs `assign`
//!
//! [`Env::define`] always writes **this** frame (params, loop variables).
//! [`Env::assign`] is what `nome = valor` in source code does.
//!
//! # Memory
//!
//! Frames are `Rc`, so a function defined inside a function makes a cycle:
//!
//! ```text
//! vizinhos = funcao(m, l, c) {
//!     pos_lin = funcao(l) { … }   // frame → pos_lin → frame
//!     …
//! }
//! ```
//!
//! Three steps free these cycles without ever changing what a program does:
//!
//! 1. Creating a function marks its frame (and the blocks up to the
//!    function frame) with [`Env::marcar_funcao`].
//! 2. When a marked frame ends, [`Env::liberar_se_lixo`] empties it if
//!    nothing outside can still reach it (trial deletion over the objects
//!    that lead back to the frame). If a function escaped, it does nothing.
//! 3. The VM keeps weak references to marked frames. Now and then it runs
//!    the same check on them again (an escaped function may have been
//!    dropped since), and at the end of the run it empties all of them.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use super::value::{Closure, MapKey, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameKind {
    Builtins,
    Module,
    Block,
    Function,
}

pub struct Env {
    pub vars: HashMap<String, Value>,
    pub parent: Option<Rc<RefCell<Env>>>,
    pub kind: FrameKind,
    /// A function was defined here or in a block inside (see
    /// [`Env::marcar_funcao`]).
    pub(crate) criou_funcao: bool,
}

#[derive(Debug)]
pub enum AssignError {
    FunctionBoundary { name: String },
    Builtin { name: String },
}

// Live frames on this thread. Tests use it to see frames that are never
// freed (an `Rc` cycle between a frame and a function defined in it).
#[cfg(test)]
thread_local! {
    pub(crate) static ENVS_VIVOS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
impl Drop for Env {
    fn drop(&mut self) {
        ENVS_VIVOS.with(|n| n.set(n.get() - 1));
    }
}

impl Env {
    pub fn new(kind: FrameKind, parent: Option<Rc<RefCell<Env>>>) -> Rc<RefCell<Self>> {
        #[cfg(test)]
        ENVS_VIVOS.with(|n| n.set(n.get() + 1));
        Rc::new(RefCell::new(Self {
            vars: HashMap::new(),
            parent,
            kind,
            criou_funcao: false,
        }))
    }

    pub fn child(parent: &Rc<RefCell<Env>>, kind: FrameKind) -> Rc<RefCell<Self>> {
        Self::new(kind, Some(Rc::clone(parent)))
    }

    pub fn define(&mut self, name: impl Into<String>, value: Value) {
        self.vars.insert(name.into(), value);
    }

    pub fn get(env: &Rc<RefCell<Self>>, name: &str) -> Option<Value> {
        let e = env.borrow();
        if let Some(v) = e.vars.get(name) {
            return Some(v.clone());
        }
        let parent = e.parent.clone();
        drop(e);
        parent.and_then(|p| Self::get(&p, name))
    }

    /// Assign to an existing binding, or declare in `env` if the name is new.
    ///
    /// Walking past a [`FrameKind::Function`] parent to mutate an outer name
    /// is an error (functions may read outer names, not write them).
    pub fn assign(env: &Rc<RefCell<Self>>, name: &str, value: Value) -> Result<(), AssignError> {
        let mut current = Some(Rc::clone(env));
        let mut crossed_function = false;

        while let Some(cell) = current {
            let e = cell.borrow();
            if e.vars.contains_key(name) {
                if crossed_function {
                    return Err(AssignError::FunctionBoundary {
                        name: name.to_string(),
                    });
                }
                if e.kind == FrameKind::Builtins {
                    return Err(AssignError::Builtin {
                        name: name.to_string(),
                    });
                }
                drop(e);
                cell.borrow_mut().vars.insert(name.to_string(), value);
                return Ok(());
            }
            let is_fn = e.kind == FrameKind::Function;
            let parent = e.parent.clone();
            drop(e);
            if is_fn {
                crossed_function = true;
            }
            current = parent;
        }

        env.borrow_mut().vars.insert(name.to_string(), value);
        Ok(())
    }

    pub fn bindings_sorted(&self) -> Vec<(String, Value)> {
        let mut items: Vec<_> = self
            .vars
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        items.sort_by(|a, b| a.0.cmp(&b.0));
        items
    }

    /// Names visible from `env`, inner frame first (shadows outer).
    pub fn visible_bindings(env: &Rc<RefCell<Self>>) -> Vec<(String, Value)> {
        let mut seen = HashSet::new();
        let mut out = Vec::new();
        let mut current = Some(Rc::clone(env));
        while let Some(cell) = current {
            let e = cell.borrow();
            let parent = e.parent.clone();
            for (k, v) in e.bindings_sorted() {
                if seen.insert(k.clone()) {
                    out.push((k, v));
                }
            }
            drop(e);
            current = parent;
        }
        out.sort_by(|a, b| a.0.cmp(&b.0));
        out
    }

    /// Marks `env` and the blocks around it, up to the enclosing function
    /// or module frame, as holding a function defined there. Only marked
    /// frames can be in an `Rc` cycle with a closure, so only they are
    /// checked by [`Env::liberar_se_lixo`]. `novo` gets each frame marked now.
    pub(crate) fn marcar_funcao(env: &Rc<RefCell<Self>>, mut novo: impl FnMut(&Rc<RefCell<Self>>)) {
        let mut atual = Some(Rc::clone(env));
        while let Some(e) = atual {
            let mut frame = e.borrow_mut();
            if frame.criou_funcao {
                // Marked before, so its ancestors are marked too.
                return;
            }
            frame.criou_funcao = true;
            let parar = matches!(
                frame.kind,
                FrameKind::Function | FrameKind::Module | FrameKind::Builtins
            );
            let parent = frame.parent.clone();
            drop(frame);
            novo(&e);
            if parar {
                return;
            }
            atual = parent;
        }
    }

    /// Called when a frame ends, with `frame` the evaluator's own handle to
    /// it. If nothing outside can still reach the frame, empties it: that
    /// breaks the `Rc` cycles between it and the functions defined in it
    /// (`g = funcao() { … }` inside a function), which would otherwise keep
    /// the frame and everything it holds alive forever.
    ///
    /// Conservative: when in doubt (the frame or a function in it escaped,
    /// the graph is too big, something is borrowed), it does nothing and
    /// the frame is freed at the end of the run instead.
    pub(crate) fn liberar_se_lixo(frame: &Rc<RefCell<Self>>) {
        if !frame.borrow().criou_funcao || !ciclo_morto(frame) {
            return;
        }
        // Drop the values after the borrow ends: dropping a closure can
        // drop a block frame whose parent is this one.
        let vars = std::mem::take(&mut frame.borrow_mut().vars);
        drop(vars);
    }
}

/// Values visited by one [`ciclo_morto`] check before it gives up.
const LIMITE_DA_BUSCA: usize = 100_000;

/// An object that holds `Rc`s of other values.
enum Objeto {
    Env(Rc<RefCell<Env>>),
    Lista(Rc<RefCell<Vec<Value>>>),
    Mapa(Rc<RefCell<Vec<(MapKey, Value)>>>),
    Funcao(Rc<Closure>),
}

impl Objeto {
    fn de_valor(v: &Value) -> Option<Self> {
        match v {
            Value::Lista(xs) => Some(Self::Lista(Rc::clone(xs))),
            Value::Mapa(m) => Some(Self::Mapa(Rc::clone(m))),
            Value::Funcao(c) => Some(Self::Funcao(Rc::clone(c))),
            Value::Par(_, v) => Self::de_valor(v),
            // A module's frame is not inside any function frame, so it is
            // never part of the cycle being checked (see `ciclo_morto`).
            Value::Modulo(_)
            | Value::Nada
            | Value::Numero(_)
            | Value::Texto(_)
            | Value::Bool(_)
            | Value::Conjunto(_)
            | Value::Matriz(_)
            | Value::Imagem(_)
            | Value::Processo(_)
            | Value::Builtin(_) => None,
        }
    }

    fn id(&self) -> usize {
        match self {
            Self::Env(e) => Rc::as_ptr(e) as *const () as usize,
            Self::Lista(xs) => Rc::as_ptr(xs) as *const () as usize,
            Self::Mapa(m) => Rc::as_ptr(m) as *const () as usize,
            Self::Funcao(c) => Rc::as_ptr(c) as *const () as usize,
        }
    }

    fn fortes(&self) -> usize {
        match self {
            Self::Env(e) => Rc::strong_count(e),
            Self::Lista(xs) => Rc::strong_count(xs),
            Self::Mapa(m) => Rc::strong_count(m),
            Self::Funcao(c) => Rc::strong_count(c),
        }
    }

    /// Objects this one holds, one entry per `Rc` it owns. `None` when
    /// something is borrowed (the check then gives up).
    fn filhos(&self, raiz: usize, visitados: &mut usize) -> Option<Vec<Objeto>> {
        let mut out = Vec::new();
        match self {
            Self::Env(e) => {
                let e = e.try_borrow().ok()?;
                *visitados += e.vars.len();
                out.extend(e.vars.values().filter_map(Objeto::de_valor));
                // Frames inside `raiz` point at their parent; `raiz`'s own
                // parent is outside the search.
                if self.id() != raiz
                    && let Some(p) = &e.parent
                {
                    out.push(Self::Env(Rc::clone(p)));
                }
            }
            Self::Lista(xs) => {
                let xs = xs.try_borrow().ok()?;
                *visitados += xs.len();
                out.extend(xs.iter().filter_map(Objeto::de_valor));
            }
            Self::Mapa(m) => {
                let m = m.try_borrow().ok()?;
                *visitados += m.len();
                out.extend(m.iter().filter_map(|(_, v)| Objeto::de_valor(v)));
            }
            Self::Funcao(c) => out.push(Self::Env(Rc::clone(&c.env))),
        }
        // Frames outside `raiz` are not followed (see `ciclo_morto`).
        out.retain(|o| match o {
            Self::Env(e) => dentro_de(e, raiz),
            _ => true,
        });
        Some(out)
    }
}

/// Trial deletion (as in CPython's cycle collector) over the objects that
/// can reach `frame`: true when every one of them is referenced only from
/// inside that set, plus the evaluator's handle to `frame`. Then nothing
/// else can reach the frame, and emptying it frees the whole set.
///
/// The search does not go above `frame` (its parent chain is alive anyway)
/// and only enters frames nested inside it (blocks of the function), so an
/// outside frame that reaches in shows up as an extra strong count.
fn ciclo_morto(frame: &Rc<RefCell<Env>>) -> bool {
    let raiz = Rc::as_ptr(frame) as *const () as usize;
    // One clone of each object found; `arestas[i]` lists what `nos[i]`
    // holds, one entry per `Rc`.
    let mut nos: Vec<Objeto> = vec![Objeto::Env(Rc::clone(frame))];
    let mut arestas: Vec<Vec<usize>> = vec![Vec::new()];
    let mut indice: HashMap<usize, usize> = HashMap::from([(raiz, 0)]);
    let mut pendentes = vec![0];
    let mut visitados = 0;

    while let Some(i) = pendentes.pop() {
        let Some(filhos) = nos[i].filhos(raiz, &mut visitados) else {
            return false;
        };
        if visitados > LIMITE_DA_BUSCA {
            return false;
        }
        for filho in filhos {
            let j = *indice.entry(filho.id()).or_insert_with(|| {
                nos.push(filho);
                arestas.push(Vec::new());
                pendentes.push(nos.len() - 1);
                nos.len() - 1
            });
            arestas[i].push(j);
        }
    }

    // Only objects on a path back to `frame` matter: an outside reference
    // to anything else does not keep the frame alive.
    let n = nos.len();
    let mut entram: Vec<Vec<usize>> = vec![Vec::new(); n];
    for (de, para) in arestas.iter().enumerate() {
        for &p in para {
            entram[p].push(de);
        }
    }
    let mut alcanca = vec![false; n];
    alcanca[0] = true;
    let mut fila = vec![0];
    while let Some(i) = fila.pop() {
        for &de in &entram[i] {
            if !alcanca[de] {
                alcanca[de] = true;
                fila.push(de);
            }
        }
    }

    let mut internas = vec![0usize; n];
    for (de, para) in arestas.iter().enumerate() {
        if alcanca[de] {
            for &p in para {
                internas[p] += 1;
            }
        }
    }
    // Counts are read now that the only clones left are the ones in `nos`
    // (one each); `frame` also has the evaluator's handle.
    (0..n).all(|i| {
        let esperado = internas[i] + 1 + usize::from(i == 0);
        !alcanca[i] || nos[i].fortes() == esperado
    })
}

/// True when `env` is the frame `raiz` or nested inside it.
fn dentro_de(env: &Rc<RefCell<Env>>, raiz: usize) -> bool {
    let mut atual = Some(Rc::clone(env));
    while let Some(e) = atual {
        if Rc::as_ptr(&e) as *const () as usize == raiz {
            return true;
        }
        atual = match e.try_borrow() {
            Ok(b) => b.parent.clone(),
            Err(_) => return false,
        };
    }
    false
}

#[cfg(test)]
mod tests {
    use super::super::value::Closure;
    use super::*;
    use crate::lexer::Span;
    use crate::parser::{Block, Expr, Stmt};

    fn num(n: f64) -> Value {
        Value::Numero(n.into())
    }

    fn closao(env: Rc<RefCell<Env>>, ident: &str) -> Value {
        Value::Funcao(Rc::new(Closure {
            params: vec![],
            body: Block {
                stmts: vec![Stmt::Expr {
                    expr: Expr::Ident {
                        name: ident.into(),
                        span: Span::default(),
                    },
                    span: Span::default(),
                }],
                span: Span::default(),
            },
            env,
            span: Span::default(),
            file: String::new(),
            source: "".into(),
            doc: None,
        }))
    }

    #[test]
    fn get_walks_parents() {
        let module = Env::new(FrameKind::Module, None);
        module.borrow_mut().define("x", num(1.0));
        let block = Env::child(&module, FrameKind::Block);
        assert_eq!(Env::get(&block, "x"), Some(num(1.0)));
        assert_eq!(Env::get(&block, "y"), None);
    }

    #[test]
    fn assign_updates_existing_outer_from_a_block() {
        // `inicio x = 2 fim` may mutate the module's `x`.
        let module = Env::new(FrameKind::Module, None);
        module.borrow_mut().define("x", num(1.0));
        let block = Env::child(&module, FrameKind::Block);
        Env::assign(&block, "x", num(2.0)).unwrap();
        assert_eq!(Env::get(&module, "x"), Some(num(2.0)));
        assert!(!block.borrow().vars.contains_key("x"));
    }

    #[test]
    fn assign_declares_in_current_frame_when_name_is_new() {
        let module = Env::new(FrameKind::Module, None);
        let block = Env::child(&module, FrameKind::Block);
        Env::assign(&block, "y", num(3.0)).unwrap();
        assert_eq!(Env::get(&block, "y"), Some(num(3.0)));
        assert_eq!(Env::get(&module, "y"), None);
    }

    #[test]
    fn function_may_read_but_not_write_outer_names() {
        let module = Env::new(FrameKind::Module, None);
        module.borrow_mut().define("x", num(1.0));
        let func = Env::child(&module, FrameKind::Function);
        assert_eq!(Env::get(&func, "x"), Some(num(1.0)));
        match Env::assign(&func, "x", num(2.0)) {
            Err(AssignError::FunctionBoundary { name }) => assert_eq!(name, "x"),
            other => panic!("expected FunctionBoundary, got {other:?}"),
        }
        assert_eq!(Env::get(&module, "x"), Some(num(1.0)));
    }

    #[test]
    fn function_locals_do_not_cross_the_write_barrier() {
        let module = Env::new(FrameKind::Module, None);
        let func = Env::child(&module, FrameKind::Function);
        Env::assign(&func, "n", num(1.0)).unwrap();
        Env::assign(&func, "n", num(2.0)).unwrap();
        assert_eq!(Env::get(&func, "n"), Some(num(2.0)));
        assert_eq!(Env::get(&module, "n"), None);
    }

    #[test]
    fn builtins_frame_is_read_only() {
        let builtins = Env::new(FrameKind::Builtins, None);
        builtins
            .borrow_mut()
            .define("escreva", Value::Builtin("escreva"));
        let module = Env::child(&builtins, FrameKind::Module);
        match Env::assign(&module, "escreva", num(1.0)) {
            Err(AssignError::Builtin { name }) => assert_eq!(name, "escreva"),
            other => panic!("expected Builtin, got {other:?}"),
        }
    }

    #[test]
    fn bindings_sorted_is_alphabetical() {
        let env = Env::new(FrameKind::Module, None);
        env.borrow_mut().define("b", num(2.0));
        env.borrow_mut().define("a", num(1.0));
        let names: Vec<_> = env
            .borrow()
            .bindings_sorted()
            .into_iter()
            .map(|(k, _)| k)
            .collect();
        assert_eq!(names, ["a", "b"]);
    }

    #[test]
    fn visible_bindings_inner_shadows_outer() {
        let outer = Env::new(FrameKind::Module, None);
        outer.borrow_mut().define("x", num(1.0));
        outer.borrow_mut().define("y", num(2.0));
        let inner = Env::child(&outer, FrameKind::Block);
        inner.borrow_mut().define("x", num(9.0));
        let got: Vec<_> = Env::visible_bindings(&inner)
            .into_iter()
            .map(|(k, v)| match v {
                Value::Numero(n) => (k, n),
                _ => panic!("expected number"),
            })
            .collect();
        assert_eq!(got, [("x".into(), 9.0.into()), ("y".into(), 2.0.into())]);
    }

    /// A function frame inside `module` holding `g`, a function that
    /// closes over that frame (the `vizinhos` / `pos_lin` shape).
    fn frame_com_funcao_local() -> (Rc<RefCell<Env>>, Rc<RefCell<Env>>, Value) {
        let module = Env::new(FrameKind::Module, None);
        let frame = Env::child(&module, FrameKind::Function);
        frame.borrow_mut().define("m", num(1.0));
        let g = closao(Rc::clone(&frame), "m");
        Env::marcar_funcao(&frame, |_| {});
        frame.borrow_mut().define("g", g.clone());
        (module, frame, g)
    }

    #[test]
    fn liberar_se_lixo_quebra_ciclo_de_funcao_local() {
        let (_module, frame, g) = frame_com_funcao_local();
        drop(g);
        assert_eq!(Rc::strong_count(&frame), 2);
        Env::liberar_se_lixo(&frame);
        assert_eq!(Rc::strong_count(&frame), 1);
        assert!(frame.borrow().vars.is_empty());
    }

    #[test]
    fn liberar_se_lixo_preserva_funcao_que_escapou() {
        // `g` is still held outside (returned, or kept by the caller).
        let (_module, frame, g) = frame_com_funcao_local();
        Env::liberar_se_lixo(&frame);
        assert!(frame.borrow().vars.contains_key("g"));
        assert!(frame.borrow().vars.contains_key("m"));
        drop(g);
    }

    #[test]
    fn liberar_se_lixo_preserva_funcao_guardada_numa_lista_de_fora() {
        // `xs[1] = g` with `xs` a list that the caller also holds.
        let (_module, frame, g) = frame_com_funcao_local();
        let de_fora = Value::lista(vec![g]);
        frame.borrow_mut().define("xs", de_fora.clone());
        Env::liberar_se_lixo(&frame);
        assert!(frame.borrow().vars.contains_key("g"));
        drop(de_fora);
    }

    #[test]
    fn liberar_se_lixo_libera_lista_local_com_a_funcao() {
        // A list only this frame holds is part of the dead cycle.
        let (_module, frame, g) = frame_com_funcao_local();
        frame
            .borrow_mut()
            .define("fs", Value::lista(vec![g.clone(), g]));
        Env::liberar_se_lixo(&frame);
        assert!(frame.borrow().vars.is_empty());
    }

    #[test]
    fn liberar_se_lixo_funcao_definida_num_bloco_interno() {
        // `se … { h = funcao() { … } }` with `h` assigned to a name of the
        // function frame: the cycle goes through the block's frame.
        let module = Env::new(FrameKind::Module, None);
        let frame = Env::child(&module, FrameKind::Function);
        frame.borrow_mut().define("h", Value::Nada);
        let bloco = Env::child(&frame, FrameKind::Block);
        Env::marcar_funcao(&bloco, |_| {});
        assert!(frame.borrow().criou_funcao, "marca sobe até a função");
        let h = closao(Rc::clone(&bloco), "h");
        Env::assign(&bloco, "h", h).unwrap();
        // The block ends first: the cycle is not through it alone.
        Env::liberar_se_lixo(&bloco);
        drop(bloco);
        Env::liberar_se_lixo(&frame);
        assert!(frame.borrow().vars.is_empty());
        assert_eq!(Rc::strong_count(&frame), 1);
    }

    #[test]
    fn liberar_se_lixo_ignora_frame_sem_funcao() {
        let module = Env::new(FrameKind::Module, None);
        let frame = Env::child(&module, FrameKind::Function);
        frame.borrow_mut().define("n", num(1.0));
        Env::liberar_se_lixo(&frame);
        assert!(frame.borrow().vars.contains_key("n"));
    }
}
