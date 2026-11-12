//! Conservative trial deletion for collection-only cycles, supplementing Env's
//! closure collector. References from environments, closures, hosts, or active
//! evaluator temporaries count as external roots and are never cleared here.
use super::*;
use std::collections::{HashMap, VecDeque};
use std::rc::Weak;

const PERIOD: usize = 1024;
const MAX_VISITED: usize = 100_000;

struct Registry {
    objects: Vec<Tracked>,
    allocated: usize,
    next_collection: usize,
}

impl Default for Registry {
    fn default() -> Self {
        Self {
            objects: Vec::new(),
            allocated: 0,
            next_collection: PERIOD,
        }
    }
}

enum Tracked {
    List(Weak<ItensLista>),
    Map(Weak<ItensMapa>),
}
enum Object {
    List(ListaRc),
    Map(MapaRc),
}

thread_local! {
    static REGISTRY: RefCell<Registry> = RefCell::new(Registry::default());
}

pub(super) fn track_list(list: &ListaRc) {
    REGISTRY.with(|registry| {
        let mut registry = registry.borrow_mut();
        registry.objects.push(Tracked::List(Rc::downgrade(list)));
        registry.allocated = registry.allocated.saturating_add(1);
    });
}

pub(super) fn track_map(map: &MapaRc) {
    REGISTRY.with(|registry| {
        let mut registry = registry.borrow_mut();
        registry.objects.push(Tracked::Map(Rc::downgrade(map)));
        registry.allocated = registry.allocated.saturating_add(1);
    });
}

impl Tracked {
    fn upgrade(&self) -> Option<Object> {
        match self {
            Self::List(w) => w.upgrade().map(Object::List),
            Self::Map(w) => w.upgrade().map(Object::Map),
        }
    }
    fn alive(&self) -> bool {
        match self {
            Self::List(w) => w.strong_count() > 0,
            Self::Map(w) => w.strong_count() > 0,
        }
    }
}

impl Object {
    fn id(&self) -> usize {
        match self {
            Self::List(v) => id(v),
            Self::Map(v) => id(v),
        }
    }
    fn strong_count(&self) -> usize {
        match self {
            Self::List(v) => Rc::strong_count(v),
            Self::Map(v) => Rc::strong_count(v),
        }
    }
    fn visit(&self, mut f: impl FnMut(&Value)) -> Option<usize> {
        match self {
            Self::List(v) => {
                let values = v.try_borrow().ok()?;
                for value in values.iter() {
                    f(value);
                }
                Some(values.len())
            }
            Self::Map(v) => {
                let values = v.try_borrow().ok()?;
                for (_, value) in values.iter() {
                    f(value);
                }
                Some(values.len())
            }
        }
    }
    fn clear(&self, garbage: &mut Vec<Value>) {
        match self {
            Self::List(v) => garbage.extend(std::mem::take(&mut *v.borrow_mut())),
            Self::Map(v) => garbage.extend(
                std::mem::take(&mut *v.borrow_mut())
                    .into_iter()
                    .map(|(_, v)| v),
            ),
        }
    }
}

fn collection_id(mut value: &Value) -> Option<usize> {
    // Pairs can wrap a collection; they are owned values, not extra Rc nodes.
    loop {
        match value {
            Value::Lista(v) => return Some(id(v)),
            Value::Mapa(v) => return Some(id(v)),
            Value::Par(_, child) => value = child,
            _ => return None,
        }
    }
}

pub(crate) fn collect_cycles_if_due() {
    if REGISTRY.with(|r| {
        let registry = r.borrow();
        registry.allocated >= registry.next_collection
    }) {
        collect_cycles();
    }
}

pub(crate) fn collect_cycles() {
    REGISTRY.with(|registry| {
        let mut registry = registry.borrow_mut();
        registry.allocated = 0;
        registry.objects.retain(Tracked::alive);
        // Amortize scans over large live graphs rather than rescanning every
        // fixed number of allocations (quadratic for deeply nested values).
        registry.next_collection = registry.objects.len().max(PERIOD);
        if registry.objects.len() > MAX_VISITED {
            return;
        }
        // One temporary strong handle per node. Nothing evaluates language code
        // during this pass, and no user-visible finalizers run on these objects.
        let nodes: Vec<_> = registry
            .objects
            .iter()
            .filter_map(Tracked::upgrade)
            .collect();
        let count = nodes.len();
        if count > MAX_VISITED {
            return;
        }
        let ids: HashMap<_, _> = nodes
            .iter()
            .enumerate()
            .map(|(i, node)| (node.id(), i))
            .collect();
        let mut edges = vec![Vec::new(); count];
        let mut incoming = vec![0usize; count];
        let mut visited = count;
        for (i, node) in nodes.iter().enumerate() {
            let Some(size) = node.visit(|value| {
                if let Some(j) = collection_id(value).and_then(|id| ids.get(&id).copied()) {
                    edges[i].push(j);
                    incoming[j] += 1;
                }
            }) else {
                return;
            }; // Borrowed by a host: conservatively defer.
            visited = visited.saturating_add(size);
            if visited > MAX_VISITED {
                return;
            }
        }
        let mut live = vec![false; count];
        let mut pending = VecDeque::new();
        for (i, node) in nodes.iter().enumerate() {
            if node.strong_count() > incoming[i] + 1 {
                live[i] = true;
                pending.push_back(i);
            }
        }
        while let Some(i) = pending.pop_front() {
            for &j in &edges[i] {
                if !live[j] {
                    live[j] = true;
                    pending.push_back(j);
                }
            }
        }
        let mut garbage = Vec::new();
        for (i, node) in nodes.iter().enumerate() {
            if !live[i] {
                node.clear(&mut garbage);
            }
        }
        // Drop after all edges have been removed, preserving stack-safe Drop.
        drop(garbage);
        drop(nodes);
        registry.objects.retain(Tracked::alive);
        registry.next_collection = registry.objects.len().max(PERIOD);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collection_cycles_are_reclaimed_but_external_roots_survive() {
        let a = Value::lista(vec![Value::Nada]);
        let b = Value::mapa(vec![]);
        let Value::Lista(list) = &a else {
            unreachable!()
        };
        let Value::Mapa(map) = &b else { unreachable!() };
        list.borrow_mut()[0] = b.clone();
        map.borrow_mut().push((
            MapKey::Texto("back".into()),
            Value::Par(MapKey::Bool(true), Box::new(a.clone())),
        ));
        let weak_list = Rc::downgrade(list);
        let weak_map = Rc::downgrade(map);
        collect_cycles();
        assert!(weak_list.upgrade().is_some());
        assert!(!map.borrow().is_empty());
        drop(a);
        drop(b);
        collect_cycles();
        assert!(weak_list.upgrade().is_none());
        assert!(weak_map.upgrade().is_none());
    }

    #[test]
    fn reachable_children_and_borrowed_cycles_are_preserved() {
        let root = Value::lista(vec![Value::Nada]);
        let Value::Lista(list) = &root else {
            unreachable!()
        };
        list.borrow_mut()[0] = root.clone();
        let weak = Rc::downgrade(list);
        let borrow = list.borrow_mut();
        collect_cycles();
        assert_eq!(borrow.len(), 1);
        drop(borrow);
        collect_cycles();
        assert_eq!(list.borrow().len(), 1);
        drop(root);
        collect_cycles();
        assert!(weak.upgrade().is_none());
    }

    #[test]
    fn discarded_cycles_do_not_accumulate_during_a_long_run() {
        use crate::runtime::eval::run_with;
        use crate::runtime::{DebugAction, DebugCtx, DebugHook};
        use std::cell::Cell;
        struct Count(Rc<Cell<usize>>);
        impl DebugHook for Count {
            fn before_stmt(&mut self, _: &DebugCtx<'_>) -> DebugAction {
                let alive =
                    REGISTRY.with(|r| r.borrow().objects.iter().filter(|v| v.alive()).count());
                self.0.set(self.0.get().max(alive));
                DebugAction::Continue
            }
        }
        collect_cycles();
        let baseline = REGISTRY.with(|r| r.borrow().objects.len());
        let max = Rc::new(Cell::new(0));
        let mut input = std::io::Cursor::new("");
        let (mut out, mut err) = (Vec::new(), Vec::new());
        run_with(
            "repita 5000 vezes { x = [0] x[0] = x }",
            "cycles.lep",
            Box::new(Count(max.clone())),
            &mut input,
            &mut out,
            &mut err,
            None,
            None,
            None,
            &[],
        )
        .unwrap();
        assert!(
            max.get() < baseline + PERIOD * 2,
            "peak live collections: {}",
            max.get()
        );
        collect_cycles();
        assert_eq!(REGISTRY.with(|r| r.borrow().objects.len()), baseline);
    }

    #[test]
    fn dropping_a_vm_releases_cycles_owned_by_imported_modules() {
        collect_cycles();
        let baseline = REGISTRY.with(|r| r.borrow().objects.len());
        let dir = std::env::temp_dir().join(format!(
            "expressa-cycle-module-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("module.lep"), "xs = [0]\nxs[0] = xs").unwrap();
        let result = crate::runtime::run_to_string(
            "m = importe \"./module\"",
            dir.join("main.lep").to_str().unwrap(),
        );
        std::fs::remove_dir_all(&dir).unwrap();
        result.unwrap();
        assert_eq!(REGISTRY.with(|r| r.borrow().objects.len()), baseline);
    }

    #[test]
    #[cfg(feature = "terminal")]
    fn values_returned_to_a_host_survive_repl_collection_and_shutdown() {
        let mut input = std::io::Cursor::new("");
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let mut session = crate::runtime::repl::ReplSession::new(&mut input, &mut out, &mut err);
        let value = session.eval("xs = [0]\nxs[0] = xs\nxs").unwrap();
        let Value::Lista(list) = &value else {
            unreachable!()
        };
        let weak = Rc::downgrade(list);
        session.eval("xs = 0").unwrap();
        assert_eq!(list.borrow().len(), 1);
        drop(session);
        assert_eq!(list.borrow().len(), 1);
        drop(value);
        collect_cycles();
        assert!(weak.upgrade().is_none());
    }
}

#[cfg(test)]
mod clone_failure_tests {
    use super::*;
    use crate::runtime::{DebugAction, DebugCtx, DebugHook, LeiaHost, run_with_hook};
    use std::cell::Cell;
    #[test]
    fn failed_graph_copies_do_not_leave_partial_cycles_between_checkpoints() {
        struct Hook {
            baseline: usize,
            checked: Rc<Cell<bool>>,
        }
        impl DebugHook for Hook {
            fn before_stmt(&mut self, ctx: &DebugCtx<'_>) -> DebugAction {
                let alive =
                    || REGISTRY.with(|r| r.borrow().objects.iter().filter(|o| o.alive()).count());
                if ctx.span.line == 1 {
                    self.baseline = alive();
                }
                if ctx.span.line == 7 {
                    assert_eq!(alive(), self.baseline + 3);
                    self.checked.set(true);
                }
                DebugAction::Continue
            }
        }
        struct Input;
        impl LeiaHost for Input {
            fn ask(&mut self, _: &str) -> Result<String, String> {
                unreachable!()
            }
        }
        let checked = Rc::new(Cell::new(false));
        run_with_hook("a = [0]\na[0] = a\nb = mapa()\nb:_clonavel = falso\nxs = [a,b]\nrepita 100 vezes { clone(xs) se_falhar 0 }\nobserve = 0","partial.lep",Box::new(Hook {baseline:0,checked:Rc::clone(&checked)}),None,None,&mut Vec::new(),&mut Vec::new(),&mut Input,&[]).unwrap();
        assert!(checked.get());
    }
}
