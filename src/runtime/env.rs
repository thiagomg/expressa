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

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use crate::parser::{AssignTarget, Block, Expr, Stmt};

use super::value::Value;

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

    /// Drops locals that form an `Rc` cycle with this frame, unless an
    /// escaped closure still looks them up by name.
    ///
    /// A nested `g = funcao() { … }` stores `g` in the call frame and the
    /// closure holds that same frame. Without this, the frame (and every
    /// captured value, including a replaced matrix) lives forever. Names
    /// that escaped closures actually read are kept so `faz_somador` and
    /// `retorne g` keep working.
    pub(crate) fn soltar_capturas(env: &Rc<RefCell<Self>>, keep: &Value) {
        let frame = Rc::as_ptr(env);
        let needed = nomes_vivos(keep, frame);
        let soltar: Vec<String> = {
            let e = env.borrow();
            e.vars
                .iter()
                .filter(|(nome, valor)| {
                    !needed.contains(*nome) && fecha_ciclo_com_frame(valor, frame)
                })
                .map(|(nome, _)| nome.clone())
                .collect()
        };
        if soltar.is_empty() {
            return;
        }
        let mut e = env.borrow_mut();
        for nome in soltar {
            e.vars.remove(&nome);
        }
    }
}

fn nomes_vivos(keep: &Value, frame: *const RefCell<Env>) -> HashSet<String> {
    let mut needed = HashSet::new();
    let mut visto = HashSet::new();
    coletar_nomes_vivos(keep, frame, &mut needed, &mut visto);
    needed
}

fn coletar_nomes_vivos(
    v: &Value,
    frame: *const RefCell<Env>,
    needed: &mut HashSet<String>,
    visto: &mut HashSet<usize>,
) {
    match v {
        Value::Funcao(c) => {
            let p = Rc::as_ptr(c) as usize;
            if !visto.insert(p) {
                return;
            }
            if env_alcanca_frame(&c.env, frame) {
                coletar_idents_bloco(&c.body, needed);
            }
        }
        Value::Lista(xs) => {
            let p = Rc::as_ptr(xs) as usize;
            if !visto.insert(p) {
                return;
            }
            for item in xs.borrow().iter() {
                coletar_nomes_vivos(item, frame, needed, visto);
            }
        }
        Value::Mapa(m) => {
            let p = Rc::as_ptr(m) as usize;
            if !visto.insert(p) {
                return;
            }
            for (_, item) in m.borrow().iter() {
                coletar_nomes_vivos(item, frame, needed, visto);
            }
        }
        Value::Par(_, item) => coletar_nomes_vivos(item, frame, needed, visto),
        Value::Nada
        | Value::Numero(_)
        | Value::Texto(_)
        | Value::Bool(_)
        | Value::Conjunto(_)
        | Value::Matriz(_)
        | Value::Imagem(_)
        | Value::Builtin(_)
        | Value::Modulo(_) => {}
    }
}

fn fecha_ciclo_com_frame(v: &Value, frame: *const RefCell<Env>) -> bool {
    let mut visto = HashSet::new();
    valor_ve_frame(v, frame, &mut visto)
}

fn valor_ve_frame(v: &Value, frame: *const RefCell<Env>, visto: &mut HashSet<usize>) -> bool {
    match v {
        Value::Funcao(c) => {
            let p = Rc::as_ptr(c) as usize;
            if !visto.insert(p) {
                return false;
            }
            env_alcanca_frame(&c.env, frame)
        }
        Value::Lista(xs) => {
            let p = Rc::as_ptr(xs) as usize;
            if !visto.insert(p) {
                return false;
            }
            xs.borrow()
                .iter()
                .any(|item| valor_ve_frame(item, frame, visto))
        }
        Value::Mapa(m) => {
            let p = Rc::as_ptr(m) as usize;
            if !visto.insert(p) {
                return false;
            }
            m.borrow()
                .iter()
                .any(|(_, item)| valor_ve_frame(item, frame, visto))
        }
        Value::Par(_, item) => valor_ve_frame(item, frame, visto),
        _ => false,
    }
}

fn env_alcanca_frame(from: &Rc<RefCell<Env>>, frame: *const RefCell<Env>) -> bool {
    let mut atual = Some(Rc::clone(from));
    while let Some(e) = atual {
        if Rc::as_ptr(&e) == frame {
            return true;
        }
        atual = e.borrow().parent.clone();
    }
    false
}

fn coletar_idents_bloco(block: &Block, out: &mut HashSet<String>) {
    for stmt in &block.stmts {
        coletar_idents_stmt(stmt, out);
    }
}

fn coletar_idents_stmt(stmt: &Stmt, out: &mut HashSet<String>) {
    match stmt {
        Stmt::Expr { expr, .. } => coletar_idents_expr(expr, out),
        Stmt::Assign { target, value, .. } => {
            coletar_idents_alvo(target, out);
            coletar_idents_expr(value, out);
        }
        Stmt::Repita { count, body, .. } => {
            coletar_idents_expr(count, out);
            coletar_idents_bloco(body, out);
        }
        Stmt::ParaRange { from, to, body, .. } => {
            coletar_idents_expr(from, out);
            coletar_idents_expr(to, out);
            coletar_idents_bloco(body, out);
        }
        Stmt::ParaIn { iter, body, .. } => {
            coletar_idents_expr(iter, out);
            coletar_idents_bloco(body, out);
        }
        Stmt::Enquanto { cond, body, .. } => {
            coletar_idents_expr(cond, out);
            coletar_idents_bloco(body, out);
        }
        Stmt::Retorne { value, .. } => {
            if let Some(expr) = value {
                coletar_idents_expr(expr, out);
            }
        }
        Stmt::Pare { .. } | Stmt::Continue { .. } => {}
    }
}

fn coletar_idents_alvo(target: &AssignTarget, out: &mut HashSet<String>) {
    match target {
        AssignTarget::Name { .. } => {}
        AssignTarget::Index { object, index, .. } => {
            coletar_idents_expr(object, out);
            coletar_idents_expr(index, out);
        }
        AssignTarget::Index2 {
            object, row, col, ..
        } => {
            coletar_idents_expr(object, out);
            coletar_idents_expr(row, out);
            coletar_idents_expr(col, out);
        }
        AssignTarget::MapField { object, .. } => coletar_idents_expr(object, out),
    }
}

fn coletar_idents_expr(expr: &Expr, out: &mut HashSet<String>) {
    match expr {
        Expr::Ident { name, .. } => {
            out.insert(name.clone());
        }
        Expr::Number { .. } | Expr::String { .. } | Expr::Bool { .. } => {}
        Expr::List { elements, .. } => {
            for el in elements {
                coletar_idents_expr(el, out);
            }
        }
        Expr::Par { key, value, .. } => {
            coletar_idents_expr(key, out);
            coletar_idents_expr(value, out);
        }
        Expr::Function { body, .. } => coletar_idents_bloco(body, out),
        Expr::Field { object, .. } | Expr::MapField { object, .. } => {
            coletar_idents_expr(object, out);
        }
        Expr::Call { callee, args, .. } => {
            coletar_idents_expr(callee, out);
            for arg in args {
                coletar_idents_expr(arg, out);
            }
        }
        Expr::Index { object, index, .. } => {
            coletar_idents_expr(object, out);
            coletar_idents_expr(index, out);
        }
        Expr::Index2 {
            object, row, col, ..
        } => {
            coletar_idents_expr(object, out);
            coletar_idents_expr(row, out);
            coletar_idents_expr(col, out);
        }
        Expr::Slice {
            object, start, end, ..
        } => {
            coletar_idents_expr(object, out);
            coletar_idents_expr(start, out);
            if let Some(end) = end {
                coletar_idents_expr(end, out);
            }
        }
        Expr::Unary { expr, .. } => coletar_idents_expr(expr, out),
        Expr::Binary { left, right, .. } => {
            coletar_idents_expr(left, out);
            coletar_idents_expr(right, out);
        }
        Expr::Block(block) => coletar_idents_bloco(block, out),
        Expr::If {
            branches,
            else_block,
            ..
        } => {
            for br in branches {
                coletar_idents_expr(&br.cond, out);
                coletar_idents_bloco(&br.body, out);
            }
            if let Some(body) = else_block {
                coletar_idents_bloco(body, out);
            }
        }
        Expr::SeFalhar {
            attempt, fallback, ..
        } => {
            coletar_idents_expr(attempt, out);
            coletar_idents_expr(fallback, out);
        }
    }
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

    #[test]
    fn soltar_capturas_quebra_ciclo_de_funcao_local() {
        let module = Env::new(FrameKind::Module, None);
        let frame = Env::child(&module, FrameKind::Function);
        frame.borrow_mut().define("m", num(1.0));
        let g = closao(Rc::clone(&frame), "m");
        frame.borrow_mut().define("g", g);
        assert!(Rc::strong_count(&frame) >= 2);
        Env::soltar_capturas(&frame, &Value::Nada);
        assert_eq!(Rc::strong_count(&frame), 1);
    }

    #[test]
    fn soltar_capturas_preserva_local_lido_pela_funcao_devolvida() {
        let module = Env::new(FrameKind::Module, None);
        let frame = Env::child(&module, FrameKind::Function);
        frame.borrow_mut().define("n", num(5.0));
        let soma = closao(Rc::clone(&frame), "n");
        frame.borrow_mut().define("soma", soma.clone());
        Env::soltar_capturas(&frame, &soma);
        assert!(frame.borrow().vars.contains_key("n"));
        assert!(!frame.borrow().vars.contains_key("soma"));
    }
}
