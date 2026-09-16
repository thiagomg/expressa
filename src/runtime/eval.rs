use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::{Duration, Instant};

use crate::lexer::Span;
use crate::parser::{
    AssignTarget, BinaryOp, Block, Expr, Import, Item, Param, Program, Stmt, UnaryOp, parse,
};

use super::builtins::BUILTINS;
use super::debug::{DebugAction, DebugCtx, DebugHook, NoopHook};
use super::env::{AssignError, Env, FrameKind};
use super::error::{CallFrame, EvalError, RuntimeError};
use super::leia::LeiaHost;
use super::value::{Closure, MapKey, Value, format_numero, parse_numero};

/// Source of lines for `leia()`. Implemented for any [`BufRead`] (tests)
/// and for [`ConsoleInput`] (locks stdin only during the read, so the
/// debugger can still prompt on the same terminal).
pub(crate) trait LineInput {
    fn read_line(&mut self, buf: &mut String) -> io::Result<usize>;
}

impl<T: BufRead> LineInput for T {
    fn read_line(&mut self, buf: &mut String) -> io::Result<usize> {
        BufRead::read_line(self, buf)
    }
}

struct ConsoleInput;

impl LineInput for ConsoleInput {
    fn read_line(&mut self, buf: &mut String) -> io::Result<usize> {
        io::stdin().read_line(buf)
    }
}

pub(crate) struct Vm<'a> {
    pub(crate) out: &'a mut dyn Write,
    pub(crate) input: &'a mut dyn LineInput,
    pub(crate) file: String,
    pub(crate) source: String,
    pub(crate) base_dir: PathBuf,
    hook: Box<dyn DebugHook>,
    stack: Vec<CallFrame>,
    builtins: Rc<RefCell<Env>>,
    loading: HashSet<PathBuf>,
    modules: HashMap<PathBuf, Rc<RefCell<Env>>>,
    /// If set, file I/O and `importe` must stay under this directory.
    workspace_root: Option<PathBuf>,
    deadline: Option<Instant>,
    pub(crate) leia_host: Option<&'a mut dyn LeiaHost>,
}

pub fn run_source(source: &str, file: &str) -> Result<(), RuntimeError> {
    let mut input = ConsoleInput;
    let mut stdout = io::stdout();
    run_with(
        source,
        file,
        Box::new(NoopHook),
        &mut input,
        &mut stdout,
        None,
        None,
        None,
    )
}

/// Run like [`run_source`], but each `leia()` writes [`super::leia::LEIA_MARKER`]
/// plus the prompt on stderr so an IDE can detect it.
pub fn run_source_marcador(source: &str, file: &str) -> Result<(), RuntimeError> {
    let stdin = io::stdin();
    let mut host = super::leia::MarkerLeiaHost {
        stdin: stdin.lock(),
    };
    let mut dummy_in = io::Cursor::new("");
    let mut stdout = io::stdout();
    run_with(
        source,
        file,
        Box::new(NoopHook),
        &mut dummy_in,
        &mut stdout,
        None,
        None,
        Some(&mut host),
    )
}

pub fn debug_source(source: &str, file: &str) -> Result<(), RuntimeError> {
    let mut input = ConsoleInput;
    let mut stdout = io::stdout();
    run_with(
        source,
        file,
        Box::new(super::debug::CliDebugger::new()),
        &mut input,
        &mut stdout,
        None,
        None,
        None,
    )
}

pub fn run_to_string(source: &str, file: &str) -> Result<String, RuntimeError> {
    run_to_string_with(source, file, "", None, None)
}

/// Run a program capturing stdout. `stdin` feeds `leia()`. When
/// `workspace_root` is set, `leia_arquivo` / `importe` cannot leave that folder.
pub fn run_to_string_with(
    source: &str,
    file: &str,
    stdin: &str,
    workspace_root: Option<PathBuf>,
    time_limit: Option<Duration>,
) -> Result<String, RuntimeError> {
    let mut input = io::Cursor::new(stdin.to_string());
    let mut out = Vec::new();
    run_with(
        source,
        file,
        Box::new(NoopHook),
        &mut input,
        &mut out,
        workspace_root,
        time_limit,
        None,
    )?;
    Ok(String::from_utf8_lossy(&out).into_owned())
}

pub fn run_with_leia_host(
    source: &str,
    file: &str,
    workspace_root: Option<PathBuf>,
    time_limit: Option<Duration>,
    out: &mut dyn Write,
    leia_host: &mut dyn LeiaHost,
) -> Result<(), RuntimeError> {
    let mut dummy_in = io::Cursor::new("");
    run_with(
        source,
        file,
        Box::new(NoopHook),
        &mut dummy_in,
        out,
        workspace_root,
        time_limit,
        Some(leia_host),
    )
}

pub(crate) fn run_with<'a>(
    source: &str,
    file: &str,
    hook: Box<dyn DebugHook>,
    input: &'a mut dyn LineInput,
    out: &'a mut dyn Write,
    workspace_root: Option<PathBuf>,
    time_limit: Option<Duration>,
    leia_host: Option<&'a mut dyn LeiaHost>,
) -> Result<(), RuntimeError> {
    let program = parse(source).map_err(|e| RuntimeError {
        message: format!("sintaxe: {}", e.message),
        file: file.to_string(),
        span: e.span,
        stack: vec![],
    })?;
    let mut vm = Vm::new(
        file,
        source,
        hook,
        input,
        out,
        workspace_root,
        time_limit,
        leia_host,
    );
    match vm.run(&program) {
        Ok(()) => Ok(()),
        Err(EvalError::Quit) => Ok(()),
        Err(EvalError::Runtime(e)) => Err(e),
    }
}

impl<'a> Vm<'a> {
    pub(crate) fn new(
        file: &str,
        source: &str,
        hook: Box<dyn DebugHook>,
        input: &'a mut dyn LineInput,
        out: &'a mut dyn Write,
        workspace_root: Option<PathBuf>,
        time_limit: Option<Duration>,
        leia_host: Option<&'a mut dyn LeiaHost>,
    ) -> Self {
        let builtins = Env::new(FrameKind::Builtins, None);
        for name in BUILTINS {
            builtins.borrow_mut().define(*name, Value::Builtin(name));
        }
        let workspace_root = workspace_root.map(|p| lexical_normalize(&p));
        Self {
            out,
            input,
            file: file.to_string(),
            source: source.to_string(),
            base_dir: base_dir_of(file),
            hook,
            stack: Vec::new(),
            builtins,
            loading: HashSet::new(),
            modules: HashMap::new(),
            workspace_root,
            deadline: time_limit.map(|d| Instant::now() + d),
            leia_host,
        }
    }

    fn run(&mut self, program: &Program) -> Result<(), EvalError> {
        let env = Env::child(&self.builtins, FrameKind::Module);
        self.eval_program(program, &env)
    }

    fn eval_program(&mut self, program: &Program, env: &Rc<RefCell<Env>>) -> Result<(), EvalError> {
        self.stack.push(CallFrame {
            name: "<modulo>".into(),
            file: self.file.clone(),
            span: program.span,
        });
        let result = (|| {
            for item in &program.items {
                self.eval_item(item, env)?;
            }
            Ok(())
        })();
        self.stack.pop();
        result
    }

    fn eval_item(&mut self, item: &Item, env: &Rc<RefCell<Env>>) -> Result<(), EvalError> {
        self.eval_item_value(item, env).map(|_| ())
    }

    fn eval_item_value(&mut self, item: &Item, env: &Rc<RefCell<Env>>) -> Result<Value, EvalError> {
        match item {
            Item::Import(import) => {
                self.eval_import(import, env)?;
                Ok(Value::Nada)
            }
            Item::Stmt(stmt) => {
                let v = self.eval_stmt(stmt, env)?;
                if matches!(stmt, Stmt::Expr { .. }) {
                    Ok(v)
                } else {
                    Ok(Value::Nada)
                }
            }
        }
    }

    pub(crate) fn eval_items_value(
        &mut self,
        items: &[Item],
        env: &Rc<RefCell<Env>>,
    ) -> Result<Value, EvalError> {
        let mut last = Value::Nada;
        for item in items {
            last = self.eval_item_value(item, env)?;
        }
        Ok(last)
    }

    pub(crate) fn builtins_env(&self) -> Rc<RefCell<Env>> {
        Rc::clone(&self.builtins)
    }

    pub(crate) fn set_source(&mut self, file: &str, source: &str) {
        self.file = file.to_string();
        self.source = source.to_string();
        self.base_dir = base_dir_of(file);
    }

    pub(crate) fn enter_repl(&mut self) {
        self.stack.push(CallFrame {
            name: "<repl>".into(),
            file: "<repl>".into(),
            span: Span::default(),
        });
    }

    fn eval_import(&mut self, import: &Import, env: &Rc<RefCell<Env>>) -> Result<(), EvalError> {
        let path = self.resolve_import_path(&import.path, import.span)?;
        let key = std::fs::canonicalize(&path).unwrap_or_else(|_| path.clone());

        if self.loading.contains(&key) {
            return Err(self.err(
                format!("importação cíclica de '{}'", path.display()),
                import.span,
            ));
        }

        let module_env = if let Some(existing) = self.modules.get(&key) {
            Rc::clone(existing)
        } else {
            let source = std::fs::read_to_string(&path).map_err(|e| {
                self.err(
                    format!("não foi possível importar '{}': {e}", path.display()),
                    import.span,
                )
            })?;
            let program = parse(&source).map_err(|e| {
                self.err(
                    format!(
                        "sintaxe em '{}': {} ({})",
                        path.display(),
                        e.message,
                        e.span
                    ),
                    import.span,
                )
            })?;

            self.loading.insert(key.clone());

            let saved_file = self.file.clone();
            let saved_source = std::mem::take(&mut self.source);
            let saved_base = self.base_dir.clone();

            self.file = path.display().to_string();
            self.source = source;
            self.base_dir = base_dir_of(&self.file);

            let module_env = Env::child(&self.builtins, FrameKind::Module);
            let result = self.eval_program(&program, &module_env);

            self.file = saved_file;
            self.source = saved_source;
            self.base_dir = saved_base;
            self.loading.remove(&key);
            result?;

            self.modules.insert(key, Rc::clone(&module_env));
            module_env
        };

        if let Some(alias) = &import.alias {
            self.assign_name(env, alias, Value::Modulo(module_env), import.span)?;
        } else {
            let bindings = module_env.borrow().bindings_sorted();
            for (name, value) in bindings {
                self.assign_name(env, &name, value, import.span)?;
            }
        }
        Ok(())
    }

    fn resolve_import_path(&self, spec: &str, span: Span) -> Result<PathBuf, EvalError> {
        let mut p = PathBuf::from(spec);
        if p.extension().is_none() {
            p.set_extension("lep");
        }
        let joined = if p.is_relative() {
            self.base_dir.join(p)
        } else {
            p
        };
        self.confine(joined, span)
    }

    pub(crate) fn confine(&self, path: PathBuf, span: Span) -> Result<PathBuf, EvalError> {
        let Some(root) = &self.workspace_root else {
            return Ok(path);
        };
        let full = lexical_normalize(&path);
        if !full.starts_with(root) {
            return Err(self.err(
                format!("caminho fora da pasta do aluno: {}", full.display()),
                span,
            ));
        }
        Ok(full)
    }

    fn eval_stmt(&mut self, stmt: &Stmt, env: &Rc<RefCell<Env>>) -> Result<Value, EvalError> {
        self.pause(stmt.span(), env)?;
        match stmt {
            Stmt::Expr { expr, .. } => self.eval_expr(expr, env),
            Stmt::Assign { target, value, .. } => {
                let value = self.eval_expr(value, env)?;
                match target {
                    AssignTarget::Name { name, span } => {
                        self.assign_name(env, name, value, *span)?;
                    }
                    AssignTarget::Index {
                        object,
                        index,
                        span,
                    } => {
                        let obj = self.eval_expr(object, env)?;
                        let idx = self.eval_expr(index, env)?;
                        self.assign_index(&obj, &idx, value, *span)?;
                    }
                }
                Ok(Value::Nada)
            }
            Stmt::Repita { count, body, .. } => {
                let count_v = self.eval_expr(count, env)?;
                let n = self.expect_int(&count_v, count.span())?;
                if n < 0 {
                    return Err(self.err("'repita' espera um número >= 0", count.span()));
                }
                let loop_env = Env::child(env, FrameKind::Block);
                for _ in 0..n {
                    self.eval_stmts(&body.stmts, &loop_env)?;
                }
                Ok(Value::Nada)
            }
            Stmt::ParaRange {
                var,
                from,
                to,
                body,
                ..
            } => {
                let from_v = self.eval_expr(from, env)?;
                let start = self.expect_int(&from_v, from.span())?;
                let to_v = self.eval_expr(to, env)?;
                let end = self.expect_int(&to_v, to.span())?;
                let loop_env = Env::child(env, FrameKind::Block);
                let mut i = start;
                while i <= end {
                    loop_env
                        .borrow_mut()
                        .define(var.clone(), Value::Numero(i as f64));
                    self.eval_stmts(&body.stmts, &loop_env)?;
                    i += 1;
                }
                Ok(Value::Nada)
            }
            Stmt::ParaIn {
                var, iter, body, ..
            } => {
                let seq = self.eval_expr(iter, env)?;
                let items = self.expect_lista(&seq, iter.span())?.borrow().clone();
                let loop_env = Env::child(env, FrameKind::Block);
                for item in items {
                    loop_env.borrow_mut().define(var.clone(), item);
                    self.eval_stmts(&body.stmts, &loop_env)?;
                }
                Ok(Value::Nada)
            }
        }
    }

    fn eval_stmts(&mut self, stmts: &[Stmt], env: &Rc<RefCell<Env>>) -> Result<Value, EvalError> {
        let mut last = Value::Nada;
        for stmt in stmts {
            let v = self.eval_stmt(stmt, env)?;
            if matches!(stmt, Stmt::Expr { .. }) {
                last = v;
            }
        }
        Ok(last)
    }

    fn eval_block(&mut self, block: &Block, env: &Rc<RefCell<Env>>) -> Result<Value, EvalError> {
        let child = Env::child(env, FrameKind::Block);
        self.eval_stmts(&block.stmts, &child)
    }

    fn eval_expr(&mut self, expr: &Expr, env: &Rc<RefCell<Env>>) -> Result<Value, EvalError> {
        match expr {
            Expr::Number { raw, span } => {
                let n = parse_numero(raw)
                    .ok_or_else(|| self.err(format!("número inválido: {raw}"), *span))?;
                Ok(Value::Numero(n))
            }
            Expr::String { value, .. } => Ok(Value::Texto(value.clone())),
            Expr::Bool { value, .. } => Ok(Value::Bool(*value)),
            Expr::List { elements, .. } => {
                let mut xs = Vec::with_capacity(elements.len());
                for el in elements {
                    xs.push(self.eval_expr(el, env)?);
                }
                Ok(Value::lista(xs))
            }
            Expr::Map { entries, span } => {
                let mut map = Vec::with_capacity(entries.len());
                for entry in entries {
                    let k = self.eval_expr(&entry.key, env)?;
                    let v = self.eval_expr(&entry.value, env)?;
                    let key = MapKey::from_value(&k).ok_or_else(|| {
                        self.err(format!("chave de mapa inválida ({})", k.type_name()), *span)
                    })?;
                    if let Some(slot) = map.iter_mut().find(|(mk, _)| *mk == key) {
                        slot.1 = v;
                    } else {
                        map.push((key, v));
                    }
                }
                Ok(Value::mapa(map))
            }
            Expr::Function { params, body, span } => {
                check_unique_params(params)
                    .map_err(|name| self.err(format!("parâmetro duplicado `{name}`"), *span))?;
                Ok(Value::Funcao(Rc::new(Closure {
                    params: params.clone(),
                    body: body.clone(),
                    env: Rc::clone(env),
                    span: *span,
                })))
            }
            Expr::Ident { name, span } => Env::get(env, name)
                .ok_or_else(|| self.err(format!("variável `{name}` não definida"), *span)),
            Expr::Field {
                object,
                field,
                span,
            } => {
                let obj = self.eval_expr(object, env)?;
                match obj {
                    Value::Modulo(module) => Env::get(&module, field)
                        .ok_or_else(|| self.err(format!("módulo não contém `{field}`"), *span)),
                    other => Err(self.err(
                        format!(
                            "acesso '.' espera um módulo, encontrado {}",
                            other.type_name()
                        ),
                        *span,
                    )),
                }
            }
            Expr::Call { callee, args, span } => {
                let name = callee_name(callee);
                let func = self.eval_expr(callee, env)?;
                let mut vals = Vec::with_capacity(args.len());
                for a in args {
                    vals.push(self.eval_expr(a, env)?);
                }
                self.call_value(&func, &vals, *span, &name)
            }
            Expr::Index {
                object,
                index,
                span,
            } => {
                let obj = self.eval_expr(object, env)?;
                let idx = self.eval_expr(index, env)?;
                self.index_get(&obj, &idx, *span)
            }
            Expr::Slice {
                object,
                start,
                end,
                span,
            } => {
                let obj = self.eval_expr(object, env)?;
                let s = self.eval_expr(start, env)?;
                let e = self.eval_expr(end, env)?;
                self.slice_get(&obj, &s, &e, *span)
            }
            Expr::Unary { op, expr, span } => {
                let v = self.eval_expr(expr, env)?;
                match op {
                    UnaryOp::Neg => Ok(Value::Numero(-self.expect_numero(&v, *span)?)),
                    UnaryOp::Not => Ok(Value::Bool(!self.expect_bool(&v, *span)?)),
                }
            }
            Expr::Binary {
                left,
                op,
                right,
                span,
            } => self.eval_binary(left, *op, right, *span, env),
            Expr::Block(block) => self.eval_block(block, env),
            Expr::If {
                branches,
                else_block,
                ..
            } => {
                for branch in branches {
                    let cond = self.eval_expr(&branch.cond, env)?;
                    if self.expect_bool(&cond, branch.cond.span())? {
                        return self.eval_block(&branch.body, env);
                    }
                }
                if let Some(body) = else_block {
                    self.eval_block(body, env)
                } else {
                    Ok(Value::Nada)
                }
            }
            Expr::SeFalhar {
                attempt, fallback, ..
            } => match self.eval_expr(attempt, env) {
                Ok(v) => Ok(v),
                Err(EvalError::Quit) => Err(EvalError::Quit),
                Err(EvalError::Runtime(_)) => self.eval_expr(fallback, env),
            },
        }
    }

    fn eval_binary(
        &mut self,
        left: &Expr,
        op: BinaryOp,
        right: &Expr,
        span: Span,
        env: &Rc<RefCell<Env>>,
    ) -> Result<Value, EvalError> {
        match op {
            BinaryOp::And => {
                let l = self.eval_expr(left, env)?;
                if !self.expect_bool(&l, span)? {
                    return Ok(Value::Bool(false));
                }
                let r = self.eval_expr(right, env)?;
                Ok(Value::Bool(self.expect_bool(&r, span)?))
            }
            BinaryOp::Or => {
                let l = self.eval_expr(left, env)?;
                if self.expect_bool(&l, span)? {
                    return Ok(Value::Bool(true));
                }
                let r = self.eval_expr(right, env)?;
                Ok(Value::Bool(self.expect_bool(&r, span)?))
            }
            _ => {
                let l = self.eval_expr(left, env)?;
                let r = self.eval_expr(right, env)?;
                self.apply_binary(&l, op, &r, span)
            }
        }
    }

    fn apply_binary(
        &self,
        l: &Value,
        op: BinaryOp,
        r: &Value,
        span: Span,
    ) -> Result<Value, EvalError> {
        match op {
            BinaryOp::Add => match (l, r) {
                (Value::Numero(a), Value::Numero(b)) => Ok(Value::Numero(a + b)),
                (Value::Lista(a), Value::Lista(b)) => {
                    let mut out = a.borrow().clone();
                    out.extend(b.borrow().clone());
                    Ok(Value::lista(out))
                }
                (Value::Texto(_), _) | (_, Value::Texto(_)) => Ok(Value::Texto(format!(
                    "{}{}",
                    stringify_value(l),
                    stringify_value(r)
                ))),
                _ => Err(self.err(
                    format!(
                        "operador '+' não se aplica a {} e {}",
                        l.type_name(),
                        r.type_name()
                    ),
                    span,
                )),
            },
            BinaryOp::Sub => Ok(Value::Numero(
                self.expect_numero(l, span)? - self.expect_numero(r, span)?,
            )),
            BinaryOp::Mul => Ok(Value::Numero(
                self.expect_numero(l, span)? * self.expect_numero(r, span)?,
            )),
            BinaryOp::Div => {
                let a = self.expect_numero(l, span)?;
                let b = self.expect_numero(r, span)?;
                if b == 0.0 {
                    return Err(self.err("divisão por zero", span));
                }
                Ok(Value::Numero(a / b))
            }
            BinaryOp::Rem => {
                let a = self.expect_numero(l, span)?;
                let b = self.expect_numero(r, span)?;
                if b == 0.0 {
                    return Err(self.err("resto de divisão por zero", span));
                }
                Ok(Value::Numero(a % b))
            }
            BinaryOp::Eq => Ok(Value::Bool(l == r)),
            BinaryOp::Ne => Ok(Value::Bool(l != r)),
            BinaryOp::Lt | BinaryOp::Le | BinaryOp::Gt | BinaryOp::Ge => {
                self.compare(l, op, r, span)
            }
            BinaryOp::Contem => self.contains(l, r, span),
            BinaryOp::And | BinaryOp::Or => unreachable!("short-circuit ops handled above"),
        }
    }

    fn compare(&self, l: &Value, op: BinaryOp, r: &Value, span: Span) -> Result<Value, EvalError> {
        let ord = match (l, r) {
            (Value::Numero(a), Value::Numero(b)) => a.partial_cmp(b),
            (Value::Texto(a), Value::Texto(b)) => Some(a.cmp(b)),
            _ => {
                return Err(self.err(
                    format!(
                        "comparação inválida entre {} e {}",
                        l.type_name(),
                        r.type_name()
                    ),
                    span,
                ));
            }
        };
        let Some(ord) = ord else {
            return Err(self.err("comparação inválida (número não-ordenado)", span));
        };
        Ok(Value::Bool(match op {
            BinaryOp::Lt => ord.is_lt(),
            BinaryOp::Le => ord.is_le(),
            BinaryOp::Gt => ord.is_gt(),
            BinaryOp::Ge => ord.is_ge(),
            _ => unreachable!(),
        }))
    }

    fn contains(&self, l: &Value, r: &Value, span: Span) -> Result<Value, EvalError> {
        match l {
            Value::Lista(xs) => Ok(Value::Bool(xs.borrow().iter().any(|x| x == r))),
            Value::Texto(s) => {
                let needle = self.expect_texto(r, span)?;
                Ok(Value::Bool(s.contains(&needle)))
            }
            Value::Mapa(xs) => {
                let key = MapKey::from_value(r).ok_or_else(|| {
                    self.err(format!("chave de mapa inválida ({})", r.type_name()), span)
                })?;
                Ok(Value::Bool(xs.borrow().iter().any(|(k, _)| *k == key)))
            }
            other => Err(self.err(
                format!("'contem' não se aplica a {}", other.type_name()),
                span,
            )),
        }
    }

    fn call_value(
        &mut self,
        func: &Value,
        args: &[Value],
        span: Span,
        name: &str,
    ) -> Result<Value, EvalError> {
        match func {
            Value::Builtin(b) => self.call_builtin(b, args, span),
            Value::Funcao(closure) => {
                if args.len() != closure.params.len() {
                    return Err(self.err(
                        format!(
                            "`{name}` espera {} argumento(s), recebeu {}",
                            closure.params.len(),
                            args.len()
                        ),
                        span,
                    ));
                }
                let call_env = Env::child(&closure.env, FrameKind::Function);
                for (param, arg) in closure.params.iter().zip(args) {
                    call_env
                        .borrow_mut()
                        .define(param.name.clone(), arg.clone());
                }
                self.stack.push(CallFrame {
                    name: name.to_string(),
                    file: self.file.clone(),
                    span,
                });
                let result = self.eval_stmts(&closure.body.stmts, &call_env);
                self.stack.pop();
                result
            }
            other => Err(self.err(
                format!(
                    "tentativa de chamar {}, que não é função",
                    other.type_name()
                ),
                span,
            )),
        }
    }

    fn index_get(&self, obj: &Value, index: &Value, span: Span) -> Result<Value, EvalError> {
        match obj {
            Value::Lista(xs) => {
                let i = self.to_index(index, xs.borrow().len(), span)?;
                Ok(xs.borrow()[i].clone())
            }
            Value::Texto(s) => {
                let chars: Vec<char> = s.chars().collect();
                let i = self.to_index(index, chars.len(), span)?;
                Ok(Value::Texto(chars[i].to_string()))
            }
            Value::Mapa(xs) => {
                let key = MapKey::from_value(index).ok_or_else(|| {
                    self.err(
                        format!("chave de mapa inválida ({})", index.type_name()),
                        span,
                    )
                })?;
                xs.borrow()
                    .iter()
                    .find(|(k, _)| *k == key)
                    .map(|(_, v)| v.clone())
                    .ok_or_else(|| self.err(format!("chave {key} não existe no mapa"), span))
            }
            other => Err(self.err(
                format!("não é possível indexar {}", other.type_name()),
                span,
            )),
        }
    }

    fn slice_get(
        &self,
        obj: &Value,
        start: &Value,
        end: &Value,
        span: Span,
    ) -> Result<Value, EvalError> {
        match obj {
            Value::Lista(xs) => {
                let xs = xs.borrow();
                let (a, b) = self.to_slice_bounds(start, end, xs.len(), span)?;
                Ok(Value::lista(xs[a..b].to_vec()))
            }
            Value::Texto(s) => {
                let chars: Vec<char> = s.chars().collect();
                let (a, b) = self.to_slice_bounds(start, end, chars.len(), span)?;
                Ok(Value::Texto(chars[a..b].iter().collect()))
            }
            other => Err(self.err(format!("não é possível fatiar {}", other.type_name()), span)),
        }
    }

    fn assign_index(
        &self,
        obj: &Value,
        index: &Value,
        value: Value,
        span: Span,
    ) -> Result<(), EvalError> {
        match obj {
            Value::Lista(xs) => {
                let i = self.to_index(index, xs.borrow().len(), span)?;
                xs.borrow_mut()[i] = value;
                Ok(())
            }
            Value::Mapa(xs) => {
                let key = MapKey::from_value(index).ok_or_else(|| {
                    self.err(
                        format!("chave de mapa inválida ({})", index.type_name()),
                        span,
                    )
                })?;
                let mut map = xs.borrow_mut();
                if let Some(slot) = map.iter_mut().find(|(k, _)| *k == key) {
                    slot.1 = value;
                } else {
                    map.push((key, value));
                }
                Ok(())
            }
            other => Err(self.err(
                format!("não é possível atribuir índice em {}", other.type_name()),
                span,
            )),
        }
    }

    fn to_index(&self, index: &Value, len: usize, span: Span) -> Result<usize, EvalError> {
        let n = self.expect_int(index, span)?;
        if n < 1 {
            return Err(self.err(format!("índice deve ser >= 1, encontrado {n}"), span));
        }
        let i = (n as usize) - 1;
        if i >= len {
            return Err(self.err(
                format!("índice {n} fora do intervalo (tamanho {len})"),
                span,
            ));
        }
        Ok(i)
    }

    fn to_slice_bounds(
        &self,
        start: &Value,
        end: &Value,
        len: usize,
        span: Span,
    ) -> Result<(usize, usize), EvalError> {
        let s = self.expect_int(start, span)?;
        let e = self.expect_int(end, span)?;
        if s < 1 || e < 1 {
            return Err(self.err("índices de fatia devem ser >= 1", span));
        }
        if s as usize > len || e as usize > len {
            return Err(self.err(
                format!("fatia [{s}..{e}] fora do intervalo (tamanho {len})"),
                span,
            ));
        }
        if s > e {
            return Ok((0, 0));
        }
        Ok((s as usize - 1, e as usize))
    }

    fn assign_name(
        &self,
        env: &Rc<RefCell<Env>>,
        name: &str,
        value: Value,
        span: Span,
    ) -> Result<(), EvalError> {
        Env::assign(env, name, value).map_err(|e| match e {
            AssignError::FunctionBoundary { name } => {
                self.err(format!("função não pode alterar `{name}`"), span)
            }
            AssignError::Builtin { name } => self.err(
                format!("não é possível alterar a função nativa `{name}`"),
                span,
            ),
        })
    }

    fn pause(&mut self, span: Span, env: &Rc<RefCell<Env>>) -> Result<(), EvalError> {
        if let Some(deadline) = self.deadline {
            if Instant::now() >= deadline {
                return Err(self.err("tempo esgotado", span));
            }
        }
        let file = self.file.clone();
        let source = self.source.clone();
        let stack = self.stack.clone();
        match self.hook.before_stmt(&DebugCtx {
            file: &file,
            source: &source,
            span,
            env,
            stack: &stack,
        }) {
            DebugAction::Continue => Ok(()),
            DebugAction::Quit => Err(EvalError::Quit),
            DebugAction::Timeout => Err(self.err("tempo esgotado", span)),
        }
    }

    pub(crate) fn err(&self, message: impl Into<String>, span: Span) -> EvalError {
        EvalError::Runtime(RuntimeError {
            message: message.into(),
            file: self.file.clone(),
            span,
            stack: self.stack.clone(),
        })
    }

    pub(crate) fn io_err(&self, e: io::Error, span: Span) -> EvalError {
        self.err(format!("erro de entrada/saída: {e}"), span)
    }

    pub(crate) fn expect_arity(
        &self,
        args: &[Value],
        n: usize,
        span: Span,
    ) -> Result<(), EvalError> {
        if args.len() != n {
            Err(self.err(
                format!("esperado {n} argumento(s), recebeu {}", args.len()),
                span,
            ))
        } else {
            Ok(())
        }
    }

    pub(crate) fn expect_numero(&self, v: &Value, span: Span) -> Result<f64, EvalError> {
        match v {
            Value::Numero(n) => Ok(*n),
            _ => Err(self.err(
                format!("esperado numero, encontrado {}", v.type_name()),
                span,
            )),
        }
    }

    pub(crate) fn expect_int(&self, v: &Value, span: Span) -> Result<i64, EvalError> {
        let n = self.expect_numero(v, span)?;
        if !n.is_finite() || n.fract() != 0.0 {
            return Err(self.err(
                format!("esperado número inteiro, encontrado {}", format_numero(n)),
                span,
            ));
        }
        if n > i64::MAX as f64 || n < i64::MIN as f64 {
            return Err(self.err("número inteiro fora do intervalo", span));
        }
        Ok(n as i64)
    }

    pub(crate) fn expect_bool(&self, v: &Value, span: Span) -> Result<bool, EvalError> {
        match v {
            Value::Bool(b) => Ok(*b),
            _ => Err(self.err(format!("esperado bool, encontrado {}", v.type_name()), span)),
        }
    }

    pub(crate) fn expect_texto(&self, v: &Value, span: Span) -> Result<String, EvalError> {
        match v {
            Value::Texto(s) => Ok(s.clone()),
            _ => Err(self.err(
                format!("esperado texto, encontrado {}", v.type_name()),
                span,
            )),
        }
    }

    pub(crate) fn expect_lista(
        &self,
        v: &Value,
        span: Span,
    ) -> Result<Rc<RefCell<Vec<Value>>>, EvalError> {
        match v {
            Value::Lista(xs) => Ok(Rc::clone(xs)),
            _ => Err(self.err(
                format!("esperado lista, encontrado {}", v.type_name()),
                span,
            )),
        }
    }
}

fn lexical_normalize(path: &Path) -> PathBuf {
    use std::path::Component;
    let mut out = PathBuf::new();
    for c in path.components() {
        match c {
            Component::Prefix(p) => out.push(p.as_os_str()),
            Component::RootDir => out.push(Component::RootDir),
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            Component::Normal(s) => out.push(s),
        }
    }
    out
}

fn base_dir_of(file: &str) -> PathBuf {
    let path = Path::new(file);
    match path.parent() {
        Some(p) if !p.as_os_str().is_empty() => p.to_path_buf(),
        _ => PathBuf::from("."),
    }
}

fn callee_name(expr: &Expr) -> String {
    match expr {
        Expr::Ident { name, .. } => name.clone(),
        Expr::Field { field, .. } => field.clone(),
        _ => "<funcao>".into(),
    }
}

fn check_unique_params(params: &[Param]) -> Result<(), String> {
    let mut seen = HashSet::new();
    for p in params {
        if !seen.insert(&p.name) {
            return Err(p.name.clone());
        }
    }
    Ok(())
}

fn stringify_value(v: &Value) -> String {
    match v {
        Value::Texto(s) => s.clone(),
        Value::Numero(n) => format_numero(*n),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::debug::DebugAction;

    fn run(src: &str) -> String {
        run_to_string(src, "teste.lep").unwrap_or_else(|e| panic!("run failed: {e}"))
    }

    fn run_in(src: &str, input: &str) -> String {
        let mut input = io::Cursor::new(input.to_string());
        let mut out = Vec::new();
        run_with(
            src,
            "teste.lep",
            Box::new(NoopHook),
            &mut input,
            &mut out,
            None,
            None,
            None,
        )
        .unwrap_or_else(|e| panic!("run failed: {e}"));
        String::from_utf8_lossy(&out).into_owned()
    }

    fn run_err(src: &str) -> String {
        match run_to_string(src, "teste.lep") {
            Err(e) => e.message,
            Ok(out) => panic!("expected error, got output {out:?}"),
        }
    }

    #[test]
    fn media_example() {
        let src = include_str!("../../examples/media.lep");
        let out = run_to_string(src, "media.lep").unwrap();
        assert_eq!(out, "Média: 7.3\nResultado: Aprovado\n");
    }

    #[test]
    fn example_programs_run() {
        let examples = [
            (
                "examples/tabuada.lep",
                include_str!("../../examples/tabuada.lep"),
            ),
            (
                "examples/fatorial.lep",
                include_str!("../../examples/fatorial.lep"),
            ),
            (
                "examples/filtra.lep",
                include_str!("../../examples/filtra.lep"),
            ),
            (
                "examples/receita.lep",
                include_str!("../../examples/receita.lep"),
            ),
            (
                "examples/poema.lep",
                include_str!("../../examples/poema.lep"),
            ),
            (
                "examples/turma.lep",
                include_str!("../../examples/turma.lep"),
            ),
            (
                "examples/palindromo.lep",
                include_str!("../../examples/palindromo.lep"),
            ),
            (
                "examples/ascii.lep",
                include_str!("../../examples/ascii.lep"),
            ),
            (
                "examples/caixa.lep",
                include_str!("../../examples/caixa.lep"),
            ),
            (
                "examples/diario.lep",
                include_str!("../../examples/diario.lep"),
            ),
            (
                "examples/contatos.lep",
                include_str!("../../examples/contatos.lep"),
            ),
            (
                "examples/usa_matematica.lep",
                include_str!("../../examples/usa_matematica.lep"),
            ),
            (
                "examples/closures.lep",
                include_str!("../../examples/closures.lep"),
            ),
            (
                "examples/escola/bhaskara.lep",
                include_str!("../../examples/escola/bhaskara.lep"),
            ),
            (
                "examples/escola/juros.lep",
                include_str!("../../examples/escola/juros.lep"),
            ),
            (
                "examples/escola/regra_de_tres.lep",
                include_str!("../../examples/escola/regra_de_tres.lep"),
            ),
            (
                "examples/escola/progressoes.lep",
                include_str!("../../examples/escola/progressoes.lep"),
            ),
            (
                "examples/escola/estatistica.lep",
                include_str!("../../examples/escola/estatistica.lep"),
            ),
            (
                "examples/escola/pitagoras.lep",
                include_str!("../../examples/escola/pitagoras.lep"),
            ),
            (
                "examples/escola/geometria.lep",
                include_str!("../../examples/escola/geometria.lep"),
            ),
            (
                "examples/escola/mdc_mmc.lep",
                include_str!("../../examples/escola/mdc_mmc.lep"),
            ),
            (
                "examples/escola/media_ponderada.lep",
                include_str!("../../examples/escola/media_ponderada.lep"),
            ),
            (
                "examples/escola/cinematica.lep",
                include_str!("../../examples/escola/cinematica.lep"),
            ),
        ];
        for (file, src) in examples {
            run_to_string(src, file).unwrap_or_else(|e| panic!("{file} failed: {e}"));
        }
    }

    #[test]
    fn erros_example_does_not_crash() {
        let src = include_str!("../../examples/erros.lep");
        let out = run_to_string(src, "erros.lep").unwrap();
        assert_eq!(out, "");
    }

    #[test]
    fn arithmetic_and_escreva() {
        assert_eq!(run(r#"escreva(10 + 5 * 2)"#), "20\n");
        assert_eq!(run(r#"escreva(10 / 2)"#), "5\n");
        assert_eq!(run(r#"escreva(10 % 3)"#), "1\n");
    }

    #[test]
    fn leia_linha_e_prompt() {
        assert_eq!(run_in(r#"escreva(leia())"#, "Ana\n"), "Ana\n");
        assert_eq!(
            run_in(
                r#"
nome = leia("Nome: ")
escreva("Oi " + nome)
"#,
                "Ana\n"
            ),
            "Nome: Oi Ana\n"
        );
        assert_eq!(
            run_in(r#"escreva(leia() se_falhar "vazio")"#, ""),
            "vazio\n"
        );
        assert_eq!(run_in(r#"escreva(leia())"#, "  x  \n"), "  x  \n");
    }

    #[test]
    fn se_falhar_div_zero() {
        assert_eq!(run(r#"escreva(10 / 0 se_falhar 0)"#), "0\n");
    }

    #[test]
    fn uncaught_div_zero() {
        let msg = run_err("10 / 0");
        assert!(msg.contains("divisão por zero"), "{msg}");
    }

    #[test]
    fn function_cannot_assign_outer() {
        let src = r#"
x = 1
f = funcao()
inicio
    x = 2
fim
f()
"#;
        let msg = run_err(src);
        assert!(msg.contains("função não pode alterar"), "{msg}");
    }

    #[test]
    fn block_can_assign_outer() {
        let src = r#"
x = 1
inicio
    x = 2
fim
escreva(x)
"#;
        assert_eq!(run(src), "2\n");
    }

    #[test]
    fn brace_blocks_run() {
        assert_eq!(run("f = funcao(n) { n * 2 }\nescreva(f(21))"), "42\n");
        assert_eq!(
            run(r#"escreva(se 2 > 1 { "sim" } senao { "nao" })"#),
            "sim\n"
        );
        assert_eq!(run("m = mapa { \"a\" -> 1 }\nescreva(m[\"a\"])"), "1\n");
    }

    #[test]
    fn function_call_and_closure_read() {
        let src = r#"
x = 10
soma = funcao(a, b)
inicio
    a + b + x
fim
escreva(soma(1, 2))
"#;
        assert_eq!(run(src), "13\n");
    }

    #[test]
    fn para_range_and_para_in() {
        let src = r#"
s = 0
para i de 1 ate 3
inicio
    s = s + i
fim
escreva(s)
t = 0
para n em [10, 20]
inicio
    t = t + n
fim
escreva(t)
"#;
        assert_eq!(run(src), "6\n30\n");
    }

    #[test]
    fn list_index_is_one_based() {
        let src = r#"
a = [10, 20, 30, 40]
escreva(a[1])
escreva(a[2..3])
a[1] = 99
escreva(a[1])
"#;
        assert_eq!(run(src), "10\n[20, 30]\n99\n");
    }

    #[test]
    fn maps_and_contem() {
        let src = r#"
p = mapa inicio
    "nome" -> "Ana"
    "idade" -> 25
fim
escreva(p["nome"])
p["cidade"] = "Fortaleza"
escreva(tamanho(p))
escreva(p contem "idade")
"#;
        assert_eq!(run(src), "Ana\n3\nverdadeiro\n");
    }

    #[test]
    fn string_builtins() {
        let src = r#"
escreva(maiuscula("oi"))
escreva(junte(["a", "b"], "-"))
escreva(limpe("  x  "))
"#;
        assert_eq!(run(src), "OI\na-b\nx\n");
    }

    #[test]
    fn plus_concatenates_when_either_side_is_text() {
        assert_eq!(run(r#"escreva("Média: " + 7.5)"#), "Média: 7.5\n");
        assert_eq!(run(r#"escreva(10 + " itens")"#), "10 itens\n");
        assert_eq!(run(r#"escreva("a" + "b")"#), "ab\n");
        assert_eq!(run(r#"escreva([1] + [2, 3])"#), "[1, 2, 3]\n");
    }

    #[test]
    fn unary_and_logic_short_circuit() {
        assert_eq!(run(r#"escreva(-8)"#), "-8\n");
        assert_eq!(run(r#"escreva(nao falso)"#), "verdadeiro\n");
        assert_eq!(run(r#"escreva(falso e (1 / 0))"#), "falso\n");
        assert_eq!(run(r#"escreva(verdadeiro ou (1 / 0))"#), "verdadeiro\n");
    }

    #[test]
    fn if_is_an_expression() {
        let src = r#"
r = se 1 > 2
inicio
    "a"
fim
ou se 3 > 2
inicio
    "b"
fim
senao
inicio
    "c"
fim
escreva(r)
"#;
        assert_eq!(run(src), "b\n");
    }

    #[test]
    fn block_value_is_the_last_expression() {
        let src = r#"
r = inicio
    x = 1
    x + 4
fim
escreva(r)
"#;
        assert_eq!(run(src), "5\n");
    }

    #[test]
    fn block_locals_do_not_leak() {
        let src = r#"
inicio
    y = 1
fim
escreva(y)
"#;
        let msg = run_err(src);
        assert!(msg.contains("não definida"), "{msg}");
    }

    #[test]
    fn repita_runs_the_body_n_times() {
        assert_eq!(
            run(r#"
repita 3 vezes
inicio
    escreva("x")
fim
"#),
            "x\nx\nx\n"
        );
    }

    #[test]
    fn text_index_and_slice_are_one_based() {
        assert_eq!(run(r#"escreva("abc"[1])"#), "a\n");
        assert_eq!(run(r#"escreva("abc"[2..3])"#), "bc\n");
    }

    #[test]
    fn se_falhar_catches_undefined_and_bad_index() {
        assert_eq!(run(r#"escreva(lista[1] se_falhar "não")"#), "não\n");
        assert_eq!(run(r#"escreva([1][99] se_falhar 0)"#), "0\n");
    }

    #[test]
    fn function_arity_and_duplicate_params() {
        let arity = run_err(
            r#"
f = funcao(a, b)
inicio
    a
fim
f(1)
"#,
        );
        assert!(arity.contains("espera 2 argumento"), "{arity}");

        let dup = run_err(
            r#"
f = funcao(a, a)
inicio
    a
fim
"#,
        );
        assert!(dup.contains("parâmetro duplicado"), "{dup}");
    }

    #[test]
    fn importe_with_and_without_alias() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("expressa-imp-{nanos}"));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("mat.lep"),
            "soma = funcao(a, b)\ninicio\n    a + b\nfim\n",
        )
        .unwrap();
        let main = dir.join("main.lep");
        let src = r#"
m = importe "mat"
escreva(m.soma(2, 3))
importe "mat"
escreva(soma(4, 5))
"#;
        let out = run_to_string(src, main.to_str().unwrap()).unwrap();
        assert_eq!(out, "5\n9\n");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn call_stack_names_the_callee() {
        let err = match run_to_string(
            r#"
boom = funcao()
inicio
    1 / 0
fim
boom()
"#,
            "app.lep",
        ) {
            Err(e) => e,
            Ok(_) => panic!("expected error"),
        };
        let text = err.to_string();
        assert!(text.contains("boom"), "{text}");
        assert!(text.contains("app.lep"), "{text}");
    }

    #[test]
    fn helpers_used_by_eval() {
        assert_eq!(base_dir_of("examples/media.lep").as_os_str(), "examples");
        assert_eq!(base_dir_of("media.lep").as_os_str(), ".");
        assert_eq!(stringify_value(&Value::Numero(3.0)), "3");
        assert_eq!(stringify_value(&Value::Texto("a".into())), "a");
        assert!(
            check_unique_params(&[Param {
                name: "a".into(),
                span: Span::default(),
            }])
            .is_ok()
        );
        assert_eq!(
            check_unique_params(&[
                Param {
                    name: "a".into(),
                    span: Span::default(),
                },
                Param {
                    name: "a".into(),
                    span: Span::default(),
                },
            ])
            .unwrap_err(),
            "a"
        );
    }

    #[test]
    fn time_limit_stops_a_long_loop() {
        let src = r#"
repita 1000000 vezes
inicio
    x = 1
fim
"#;
        let err = run_to_string_with(src, "t.lep", "", None, Some(Duration::from_millis(30)))
            .expect_err("should time out");
        assert!(err.message.contains("tempo esgotado"), "{err}");
    }

    #[test]
    fn workspace_root_blocks_path_escape() {
        let dir = std::env::temp_dir().join("expressa-ws-test");
        std::fs::create_dir_all(&dir).unwrap();
        let src = r#"leia_arquivo("../secret.txt")"#;
        let file = dir.join("main.lep");
        let err = run_to_string_with(src, file.to_str().unwrap(), "", Some(dir.clone()), None)
            .expect_err("should block escape");
        assert!(err.message.contains("fora da pasta"), "{err}");
    }

    #[test]
    fn leia_host_receives_prompt() {
        struct Capture {
            prompt: String,
            reply: String,
        }
        impl LeiaHost for Capture {
            fn ask(&mut self, prompt: &str) -> Result<String, String> {
                self.prompt = prompt.to_string();
                Ok(self.reply.clone())
            }
        }
        let mut host = Capture {
            prompt: String::new(),
            reply: "Ana".into(),
        };
        let mut input = io::Cursor::new("");
        let mut out = Vec::new();
        run_with(
            r#"escreva(leia("Seu nome:"))"#,
            "t.lep",
            Box::new(NoopHook),
            &mut input,
            &mut out,
            None,
            None,
            Some(&mut host),
        )
        .unwrap();
        assert_eq!(host.prompt, "Seu nome:");
        assert_eq!(String::from_utf8(out).unwrap(), "Ana\n");
    }

    #[test]
    fn debugger_hook_sees_statements() {
        use std::cell::Cell;

        struct Shared(Rc<Cell<usize>>);
        impl DebugHook for Shared {
            fn before_stmt(&mut self, _ctx: &DebugCtx<'_>) -> DebugAction {
                self.0.set(self.0.get() + 1);
                DebugAction::Continue
            }
        }

        let src = "x = 1\ny = 2\nescreva(x + y)\n";
        let n = Rc::new(Cell::new(0));
        let mut buf = Vec::new();
        let mut input = io::Cursor::new("");
        run_with(
            src,
            "t.lep",
            Box::new(Shared(Rc::clone(&n))),
            &mut input,
            &mut buf,
            None,
            None,
            None,
        )
        .unwrap();
        assert_eq!(n.get(), 3);
        assert_eq!(String::from_utf8(buf).unwrap(), "3\n");
    }
}
