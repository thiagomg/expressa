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
//! Builtins  (escreva, leia, tamanho, …)     read-only
//!    └── Module  (top-level of a .lep file)
//!           └── Block     (`inicio` … `fim`, `se`, `para`, `repita`)
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
use std::collections::HashMap;
use std::rc::Rc;

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

impl Env {
    pub fn new(kind: FrameKind, parent: Option<Rc<RefCell<Env>>>) -> Rc<RefCell<Self>> {
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
}

#[cfg(test)]
mod tests {
    use super::*;

    fn num(n: f64) -> Value {
        Value::Numero(n)
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
        assert!(block.borrow().vars.get("x").is_none());
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
}
