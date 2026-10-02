use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::io::{self, BufRead, IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::thread;
use std::time::{Duration, Instant};

use crate::lexer::Span;
use crate::parser::{
    AssignTarget, BinaryOp, Block, Expr, Import, Item, Param, Program, Stmt, UnaryOp, parse,
};

use super::debug::{DebugAction, DebugCtx, DebugHook, NoopHook};
use super::env::{AssignError, Env, FrameKind};
use super::error::{CallFrame, EvalError, RuntimeError};
use super::leia::LeiaHost;
use super::nativas;
use super::rng::Rng;
use super::value::{
    Closure, MapKey, NumeroLocale, Value, default_numero_locale, format_numero, parse_numero,
};

/// Source of lines for `leia()`. Implemented for any [`BufRead`] (tests)
/// and for [`ConsoleInput`] (locks stdin only during the read, so the
/// debugger can still prompt on the same terminal).
pub(crate) trait LineInput {
    fn read_line(&mut self, buf: &mut String) -> io::Result<usize>;
    fn is_terminal(&self) -> bool {
        false
    }
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

    fn is_terminal(&self) -> bool {
        io::stdin().is_terminal()
    }
}

pub(crate) struct Vm<'a> {
    pub(crate) out: &'a mut dyn Write,
    pub(crate) err: &'a mut dyn Write,
    pub(crate) input: &'a mut dyn LineInput,
    pub(crate) file: String,
    /// Shared: debugger snapshots and closures keep it without copying.
    pub(crate) source: Rc<str>,
    pub(crate) base_dir: PathBuf,
    hook: Box<dyn DebugHook>,
    pub(crate) stack: Vec<CallFrame>,
    builtins: Rc<RefCell<Env>>,
    loading: HashSet<PathBuf>,
    modules: HashMap<PathBuf, Rc<RefCell<Env>>>,
    native_modules: HashMap<String, Rc<RefCell<Env>>>,
    /// If set, file I/O and `importe` must stay under this directory.
    workspace_root: Option<PathBuf>,
    deadline: Option<Instant>,
    pub(crate) leia_host: Option<&'a mut dyn LeiaHost>,
    pub(crate) numero_locale: NumeroLocale,
    pub(crate) rng: Rng,
    /// Depth of `se_falhar` attempts being evaluated: errors there are
    /// handled, so the debugger must not stop on them.
    protected: u32,
    /// The debugger already saw the runtime error now unwinding.
    error_reported: bool,
}

fn script_args_value(args: &[String]) -> Value {
    Value::lista(args.iter().cloned().map(Value::Texto).collect())
}

pub fn run_source(source: &str, file: &str) -> Result<i32, RuntimeError> {
    run_source_args(source, file, &[])
}

pub fn run_source_args(source: &str, file: &str, args: &[String]) -> Result<i32, RuntimeError> {
    let mut input = ConsoleInput;
    let mut stdout = io::stdout();
    let mut stderr = io::stderr();
    run_with(
        source,
        file,
        Box::new(NoopHook),
        &mut input,
        &mut stdout,
        &mut stderr,
        None,
        None,
        None,
        args,
    )
}

/// Run like [`run_source`], but each `leia()` writes [`super::leia::LEIA_MARKER`]
/// plus the prompt on stderr so an IDE can detect it.
pub fn run_source_marcador(source: &str, file: &str) -> Result<i32, RuntimeError> {
    run_source_marcador_args(source, file, &[])
}

pub fn run_source_marcador_args(
    source: &str,
    file: &str,
    args: &[String],
) -> Result<i32, RuntimeError> {
    let stdin = io::stdin();
    let mut host = super::leia::MarkerLeiaHost {
        stdin: stdin.lock(),
    };
    let mut dummy_in = io::Cursor::new("");
    let mut stdout = io::stdout();
    let mut stderr = io::stderr();
    run_with(
        source,
        file,
        Box::new(NoopHook),
        &mut dummy_in,
        &mut stdout,
        &mut stderr,
        None,
        None,
        Some(&mut host),
        args,
    )
}

pub fn debug_source(source: &str, file: &str) -> Result<i32, RuntimeError> {
    debug_source_args(source, file, &[])
}

pub fn debug_source_args(source: &str, file: &str, args: &[String]) -> Result<i32, RuntimeError> {
    let mut input = ConsoleInput;
    let mut stdout = io::stdout();
    let mut stderr = io::stderr();
    run_with(
        source,
        file,
        Box::new(super::debug::CliDebugger::new()),
        &mut input,
        &mut stdout,
        &mut stderr,
        None,
        None,
        None,
        args,
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
    let mut err = Vec::new();
    run_with(
        source,
        file,
        Box::new(NoopHook),
        &mut input,
        &mut out,
        &mut err,
        workspace_root,
        time_limit,
        None,
        &[],
    )?;
    Ok(String::from_utf8_lossy(&out).into_owned())
}

/// Like [`run_to_string_with`], also capturing stderr (`escreva_erro`).
pub fn run_to_strings_with(
    source: &str,
    file: &str,
    stdin: &str,
) -> Result<(String, String), RuntimeError> {
    let mut input = io::Cursor::new(stdin.to_string());
    let mut out = Vec::new();
    let mut err = Vec::new();
    run_with(
        source,
        file,
        Box::new(NoopHook),
        &mut input,
        &mut out,
        &mut err,
        None,
        None,
        None,
        &[],
    )?;
    Ok((
        String::from_utf8_lossy(&out).into_owned(),
        String::from_utf8_lossy(&err).into_owned(),
    ))
}

pub fn run_with_leia_host(
    source: &str,
    file: &str,
    workspace_root: Option<PathBuf>,
    time_limit: Option<Duration>,
    out: &mut dyn Write,
    err: &mut dyn Write,
    leia_host: &mut dyn LeiaHost,
) -> Result<i32, RuntimeError> {
    run_with_hook(
        source,
        file,
        Box::new(NoopHook),
        workspace_root,
        time_limit,
        out,
        err,
        leia_host,
        &[],
    )
}

pub fn run_with_hook(
    source: &str,
    file: &str,
    hook: Box<dyn DebugHook>,
    workspace_root: Option<PathBuf>,
    time_limit: Option<Duration>,
    out: &mut dyn Write,
    err: &mut dyn Write,
    leia_host: &mut dyn LeiaHost,
    script_args: &[String],
) -> Result<i32, RuntimeError> {
    let mut dummy_in = io::Cursor::new("");
    run_with(
        source,
        file,
        hook,
        &mut dummy_in,
        out,
        err,
        workspace_root,
        time_limit,
        Some(leia_host),
        script_args,
    )
}

pub(crate) fn run_with<'a>(
    source: &str,
    file: &str,
    hook: Box<dyn DebugHook>,
    input: &'a mut dyn LineInput,
    out: &'a mut dyn Write,
    err: &'a mut dyn Write,
    workspace_root: Option<PathBuf>,
    time_limit: Option<Duration>,
    leia_host: Option<&'a mut dyn LeiaHost>,
    script_args: &[String],
) -> Result<i32, RuntimeError> {
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
        err,
        workspace_root,
        time_limit,
        leia_host,
        script_args,
    );
    match vm.run(&program) {
        Ok(()) => Ok(0),
        Err(EvalError::Quit(code)) => Ok(code),
        Err(EvalError::Return { span, .. }) => Err(RuntimeError {
            message: "retorne só pode ser usado numa função".into(),
            file: file.to_string(),
            span,
            stack: vec![],
        }),
        Err(EvalError::Break { span }) => Err(RuntimeError {
            message: "pare só pode ser usado num laço".into(),
            file: file.to_string(),
            span,
            stack: vec![],
        }),
        Err(EvalError::Continue { span }) => Err(RuntimeError {
            message: "continue só pode ser usado num laço".into(),
            file: file.to_string(),
            span,
            stack: vec![],
        }),
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
        err: &'a mut dyn Write,
        workspace_root: Option<PathBuf>,
        time_limit: Option<Duration>,
        leia_host: Option<&'a mut dyn LeiaHost>,
        script_args: &[String],
    ) -> Self {
        let builtins = Env::new(FrameKind::Builtins, None);
        nativas::bind_nucleo(&builtins);
        let native_modules = nativas::make_native_modules();
        builtins
            .borrow_mut()
            .define("argumentos", script_args_value(script_args));
        let workspace_root = workspace_root.map(|p| lexical_normalize(&p));
        Self {
            out,
            err,
            input,
            file: file.to_string(),
            source: Rc::from(source),
            base_dir: base_dir_of(file),
            hook,
            stack: Vec::new(),
            builtins,
            loading: HashSet::new(),
            modules: HashMap::new(),
            native_modules,
            workspace_root,
            deadline: time_limit.map(|d| Instant::now() + d),
            leia_host,
            numero_locale: default_numero_locale(),
            rng: Rng::new(),
            protected: 0,
            error_reported: false,
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
        self.retorne_so_em_funcao(result)
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
            last = match self.eval_item_value(item, env) {
                Ok(v) => v,
                Err(e) => return self.retorne_so_em_funcao(Err(e)),
            };
        }
        Ok(last)
    }

    pub(crate) fn builtins_env(&self) -> Rc<RefCell<Env>> {
        Rc::clone(&self.builtins)
    }

    pub(crate) fn set_source(&mut self, file: &str, source: &str) {
        self.file = file.to_string();
        self.source = Rc::from(source);
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
        if is_bare_module_spec(&import.path) {
            if let Some(native) = self.native_modules.get(&import.path).cloned() {
                if self.native_conflicts_with_file(&import.path, import.span)? {
                    return Err(self.err(
                        format!(
                            "'{}' é um módulo nativo e existe {}.lep nesta pasta. Use importe \"./{}\" para o arquivo, ou mude o nome/pasta do arquivo para usar o nativo.",
                            import.path, import.path, import.path
                        ),
                        import.span,
                    ));
                }
                return self.bind_imported(import, env, native);
            }
        }
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
            let saved_source = std::mem::replace(&mut self.source, Rc::from(""));
            let saved_base = self.base_dir.clone();

            self.file = path.display().to_string();
            self.source = Rc::from(source);
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

        self.bind_imported(import, env, module_env)
    }

    fn bind_imported(
        &mut self,
        import: &Import,
        env: &Rc<RefCell<Env>>,
        module_env: Rc<RefCell<Env>>,
    ) -> Result<(), EvalError> {
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

    fn native_conflicts_with_file(&self, spec: &str, span: Span) -> Result<bool, EvalError> {
        let path = self.resolve_import_path(spec, span)?;
        Ok(path.exists())
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
        let result = self.eval_stmt_inner(stmt, env);
        if let Err(EvalError::Runtime(e)) = &result {
            // The innermost statement sees the error first, with its
            // variables still in scope: let the debugger show them.
            if self.protected == 0 && !self.error_reported {
                self.error_reported = true;
                let file = self.file.clone();
                let source = Rc::clone(&self.source);
                let stack = self.stack.clone();
                self.hook.on_error(
                    &DebugCtx {
                        file: &file,
                        source: &source,
                        span: e.span,
                        env,
                        stack: &stack,
                    },
                    &e.message,
                );
            }
        }
        result
    }

    fn eval_stmt_inner(&mut self, stmt: &Stmt, env: &Rc<RefCell<Env>>) -> Result<Value, EvalError> {
        match stmt {
            Stmt::Expr { expr, .. } => self.eval_expr(expr, env),
            Stmt::Assign { target, value, .. } => {
                let value = self.eval_expr(value, env)?;
                self.assign_target(target, value, env)?;
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
                    if self.eval_loop_body(&body.stmts, &loop_env)? {
                        break;
                    }
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
                    if self.eval_loop_body(&body.stmts, &loop_env)? {
                        break;
                    }
                    i += 1;
                }
                Ok(Value::Nada)
            }
            Stmt::ParaIn {
                var, iter, body, ..
            } => {
                let seq = self.eval_expr(iter, env)?;
                let items = self.iter_items(&seq, iter.span())?;
                let loop_env = Env::child(env, FrameKind::Block);
                for item in items {
                    loop_env.borrow_mut().define(var.clone(), item);
                    if self.eval_loop_body(&body.stmts, &loop_env)? {
                        break;
                    }
                }
                Ok(Value::Nada)
            }
            Stmt::Enquanto { cond, body, span } => {
                let loop_env = Env::child(env, FrameKind::Block);
                loop {
                    self.check_deadline(*span)?;
                    let cond_v = self.eval_expr(cond, env)?;
                    if !self.expect_bool(&cond_v, cond.span())? {
                        break;
                    }
                    if self.eval_loop_body(&body.stmts, &loop_env)? {
                        break;
                    }
                }
                Ok(Value::Nada)
            }
            Stmt::Pare { span } => Err(EvalError::Break { span: *span }),
            Stmt::Continue { span } => Err(EvalError::Continue { span: *span }),
            Stmt::Retorne { value, span } => {
                let v = match value {
                    Some(expr) => self.eval_expr(expr, env)?,
                    None => Value::Nada,
                };
                Err(EvalError::Return {
                    value: v,
                    span: *span,
                })
            }
        }
    }

    /// `true` = `pare` left the loop.
    fn eval_loop_body(
        &mut self,
        stmts: &[Stmt],
        env: &Rc<RefCell<Env>>,
    ) -> Result<bool, EvalError> {
        match self.eval_stmts(stmts, env) {
            Ok(_) | Err(EvalError::Continue { .. }) => Ok(false),
            Err(EvalError::Break { .. }) => Ok(true),
            Err(e) => Err(e),
        }
    }

    fn iter_items(&self, seq: &Value, span: Span) -> Result<Vec<Value>, EvalError> {
        match seq {
            Value::Lista(xs) => Ok(xs.borrow().clone()),
            Value::Texto(s) => Ok(s.chars().map(|c| Value::Texto(c.to_string())).collect()),
            Value::Conjunto(xs) => Ok(xs.borrow().iter().map(|k| k.to_value()).collect()),
            other => Err(self.err(
                format!(
                    "'para … em' espera lista, texto ou conjunto, encontrado {}",
                    other.type_name()
                ),
                span,
            )),
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
            Expr::Par { key, value, span } => {
                let k = self.eval_expr(key, env)?;
                let v = self.eval_expr(value, env)?;
                let key = MapKey::from_value(&k).ok_or_else(|| {
                    self.err(format!("chave de par inválida ({})", k.type_name()), *span)
                })?;
                Ok(Value::Par(key, Box::new(v)))
            }
            Expr::Function { params, body, span } => {
                check_unique_params(params)
                    .map_err(|name| self.err(format!("parâmetro duplicado `{name}`"), *span))?;
                Ok(Value::Funcao(Rc::new(Closure {
                    params: params.clone(),
                    body: body.clone(),
                    env: Rc::clone(env),
                    span: *span,
                    file: self.file.clone(),
                    source: Rc::clone(&self.source),
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
                    Value::Mapa(_) => {
                        Err(self.err(format!("mapa usa ':' — tente o_mapa:{field}"), *span))
                    }
                    other => Err(self.err(
                        format!(
                            "acesso '::' espera um módulo, encontrado {}",
                            other.type_name()
                        ),
                        *span,
                    )),
                }
            }
            Expr::MapField {
                object,
                field,
                span,
            } => {
                let obj = self.eval_expr(object, env)?;
                match &obj {
                    Value::Mapa(_) => self.index_get(&obj, &Value::Texto(field.clone()), *span),
                    Value::Par(k, v) => {
                        match field.as_str() {
                            "chave" => Ok(k.to_value()),
                            "valor" => Ok(v.as_ref().clone()),
                            _ => Err(self
                                .err(format!("par só tem :chave e :valor, não `{field}`"), *span)),
                        }
                    }
                    Value::Modulo(_) => {
                        Err(self.err(format!("módulo usa '::' — tente o_modulo::{field}"), *span))
                    }
                    other => Err(self.err(
                        format!(
                            "acesso ':' espera um mapa, encontrado {}",
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
                self.call_value(&func, &vals, *span, &name, env)
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
            Expr::Index2 {
                object,
                row,
                col,
                span,
            } => {
                let obj = self.eval_expr(object, env)?;
                let i = self.eval_expr(row, env)?;
                let j = self.eval_expr(col, env)?;
                self.index_get2(&obj, &i, &j, *span)
            }
            Expr::Slice {
                object,
                start,
                end,
                span,
            } => {
                let obj = self.eval_expr(object, env)?;
                let s = self.eval_expr(start, env)?;
                let e = match end {
                    Some(e) => Some(self.eval_expr(e, env)?),
                    None => None,
                };
                self.slice_get(&obj, &s, e.as_ref(), *span)
            }
            Expr::Unary { op, expr, span } => {
                let v = self.eval_expr(expr, env)?;
                match op {
                    UnaryOp::Neg => match &v {
                        Value::Matriz(m) => {
                            let rows: Vec<Vec<f64>> = m
                                .borrow()
                                .iter()
                                .map(|r| r.iter().map(|x| -x).collect())
                                .collect();
                            Ok(Value::matriz(rows))
                        }
                        _ => Ok(Value::Numero(-self.expect_numero(&v, *span)?)),
                    },
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
            } => match {
                self.protected += 1;
                let r = self.eval_expr(attempt, env);
                self.protected -= 1;
                r
            } {
                Ok(v) => Ok(v),
                Err(EvalError::Quit(code)) => Err(EvalError::Quit(code)),
                Err(EvalError::Return { value, span }) => Err(EvalError::Return { value, span }),
                Err(EvalError::Break { span }) => Err(EvalError::Break { span }),
                Err(EvalError::Continue { span }) => Err(EvalError::Continue { span }),
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
                (Value::Conjunto(a), Value::Conjunto(b)) => {
                    let mut out = a.borrow().clone();
                    for k in b.borrow().iter() {
                        super::value::conjunto_insert(&mut out, k.clone());
                    }
                    Ok(Value::conjunto(out))
                }
                (Value::Conjunto(a), other) => {
                    let k = MapKey::from_value(other).ok_or_else(|| {
                        self.err(
                            format!(
                                "operador '+' não se aplica a conjunto e {}",
                                other.type_name()
                            ),
                            span,
                        )
                    })?;
                    let mut out = a.borrow().clone();
                    super::value::conjunto_insert(&mut out, k);
                    Ok(Value::conjunto(out))
                }
                (Value::Matriz(_), Value::Matriz(_)) => self.matriz_add(l, r, span, 1.0),
                (Value::Texto(_), _) | (_, Value::Texto(_)) => Ok(Value::Texto(format!(
                    "{}{}",
                    self.format_value(l),
                    self.format_value(r)
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
            BinaryOp::Sub => match (l, r) {
                (Value::Matriz(_), Value::Matriz(_)) => self.matriz_add(l, r, span, -1.0),
                _ => Ok(Value::Numero(
                    self.expect_numero(l, span)? - self.expect_numero(r, span)?,
                )),
            },
            BinaryOp::Mul => match (l, r) {
                (Value::Matriz(_), Value::Matriz(_)) => self.matriz_mul(l, r, span),
                (Value::Matriz(_), Value::Numero(k)) | (Value::Numero(k), Value::Matriz(_)) => {
                    self.matriz_scale(l, r, *k, span)
                }
                _ => Ok(Value::Numero(
                    self.expect_numero(l, span)? * self.expect_numero(r, span)?,
                )),
            },
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
            Value::Conjunto(xs) => {
                let key = MapKey::from_value(r).ok_or_else(|| {
                    self.err(
                        format!("elemento de conjunto inválido ({})", r.type_name()),
                        span,
                    )
                })?;
                Ok(Value::Bool(xs.borrow().contains(&key)))
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
        env: &Rc<RefCell<Env>>,
    ) -> Result<Value, EvalError> {
        match func {
            Value::Builtin(b) => self.call_builtin(b, args, span, env),
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
                // Errors and the debugger name the file that defined the
                // function (an imported module). Relative paths still
                // resolve from the running program's folder.
                let saved_file = std::mem::replace(&mut self.file, closure.file.clone());
                let saved_source = std::mem::replace(&mut self.source, Rc::clone(&closure.source));
                let result = self.eval_stmts(&closure.body.stmts, &call_env);
                self.file = saved_file;
                self.source = saved_source;
                self.stack.pop();
                match result {
                    Ok(v) => Ok(v),
                    Err(EvalError::Return { value, .. }) => Ok(value),
                    Err(EvalError::Break { span }) => {
                        Err(self.err("pare só pode ser usado num laço", span))
                    }
                    Err(EvalError::Continue { span }) => {
                        Err(self.err("continue só pode ser usado num laço", span))
                    }
                    Err(e) => Err(e),
                }
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
            Value::Matriz(m) => {
                let rows = m.borrow();
                let i = self.to_index(index, rows.len(), span)?;
                let row: Vec<Value> = rows[i].iter().copied().map(Value::Numero).collect();
                Ok(Value::lista(row))
            }
            other => Err(self.err(
                format!("não é possível indexar {}", other.type_name()),
                span,
            )),
        }
    }

    fn index_get2(
        &self,
        obj: &Value,
        row: &Value,
        col: &Value,
        span: Span,
    ) -> Result<Value, EvalError> {
        let Value::Matriz(m) = obj else {
            return Err(self.err(
                format!(
                    "[i, j] só se aplica a matriz, encontrado {}",
                    obj.type_name()
                ),
                span,
            ));
        };
        let rows = m.borrow();
        let i = self.to_index(row, rows.len(), span)?;
        let j = self.to_index(col, rows[i].len(), span)?;
        Ok(Value::Numero(rows[i][j]))
    }

    fn assign_index2(
        &self,
        obj: &Value,
        row: &Value,
        col: &Value,
        value: Value,
        span: Span,
    ) -> Result<(), EvalError> {
        let Value::Matriz(m) = obj else {
            return Err(self.err(
                format!(
                    "[i, j] só se aplica a matriz, encontrado {}",
                    obj.type_name()
                ),
                span,
            ));
        };
        let n = self.expect_numero(&value, span)?;
        let mut rows = m.borrow_mut();
        let i = self.to_index(row, rows.len(), span)?;
        let j = self.to_index(col, rows[i].len(), span)?;
        rows[i][j] = n;
        Ok(())
    }

    pub(crate) fn lista_para_matriz(&self, rows: &Value, span: Span) -> Result<Value, EvalError> {
        let xs = self.expect_lista(rows, span)?;
        let borrowed = xs.borrow();
        if borrowed.is_empty() {
            return Err(self.err("matriz precisa de pelo menos uma linha", span));
        }
        let mut grid: Vec<Vec<f64>> = Vec::new();
        let mut width = None;
        for row in borrowed.iter() {
            let cells = self.expect_lista(row, span)?;
            let mut nums = Vec::new();
            for item in cells.borrow().iter() {
                nums.push(self.expect_numero(item, span)?);
            }
            if nums.is_empty() {
                return Err(self.err("linha de matriz não pode ser vazia", span));
            }
            match width {
                None => width = Some(nums.len()),
                Some(w) if w != nums.len() => {
                    return Err(self.err(
                        format!(
                            "linhas da matriz têm tamanhos diferentes ({w} e {})",
                            nums.len()
                        ),
                        span,
                    ));
                }
                _ => {}
            }
            grid.push(nums);
        }
        Ok(Value::matriz(grid))
    }

    fn matriz_add(&self, l: &Value, r: &Value, span: Span, sign: f64) -> Result<Value, EvalError> {
        let (Value::Matriz(a), Value::Matriz(b)) = (l, r) else {
            unreachable!();
        };
        let a = a.borrow();
        let b = b.borrow();
        if a.len() != b.len() || a.first().map(|r| r.len()) != b.first().map(|r| r.len()) {
            return Err(self.err("matrizes de tamanhos diferentes", span));
        }
        let out: Vec<Vec<f64>> = a
            .iter()
            .zip(b.iter())
            .map(|(ra, rb)| {
                ra.iter()
                    .zip(rb.iter())
                    .map(|(x, y)| x + sign * y)
                    .collect()
            })
            .collect();
        Ok(Value::matriz(out))
    }

    fn matriz_scale(&self, l: &Value, r: &Value, k: f64, _span: Span) -> Result<Value, EvalError> {
        let m = match (l, r) {
            (Value::Matriz(m), _) | (_, Value::Matriz(m)) => m,
            _ => unreachable!(),
        };
        let out: Vec<Vec<f64>> = m
            .borrow()
            .iter()
            .map(|row| row.iter().map(|x| x * k).collect())
            .collect();
        Ok(Value::matriz(out))
    }

    fn matriz_mul(&self, l: &Value, r: &Value, span: Span) -> Result<Value, EvalError> {
        let (Value::Matriz(a), Value::Matriz(b)) = (l, r) else {
            unreachable!();
        };
        let a = a.borrow();
        let b = b.borrow();
        let n = a.len();
        let p = a.first().map(|r| r.len()).unwrap_or(0);
        let q = b.first().map(|r| r.len()).unwrap_or(0);
        if p != b.len() {
            return Err(self.err(
                format!(
                    "produto de matrizes exige colunas da esquerda = linhas da direita ({p} ≠ {})",
                    b.len()
                ),
                span,
            ));
        }
        let mut out = vec![vec![0.0; q]; n];
        for i in 0..n {
            for j in 0..q {
                let mut s = 0.0;
                for k in 0..p {
                    s += a[i][k] * b[k][j];
                }
                out[i][j] = s;
            }
        }
        Ok(Value::matriz(out))
    }

    fn slice_get(
        &self,
        obj: &Value,
        start: &Value,
        end: Option<&Value>,
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

    fn assign_target(
        &mut self,
        target: &AssignTarget,
        value: Value,
        env: &Rc<RefCell<Env>>,
    ) -> Result<(), EvalError> {
        match target {
            AssignTarget::Name { name, span } => self.assign_name(env, name, value, *span),
            AssignTarget::Index {
                object,
                index,
                span,
            } => {
                let idx = self.eval_expr(index, env)?;
                self.assign_indexed(object, &idx, value, *span, env)
            }
            AssignTarget::Index2 {
                object,
                row,
                col,
                span,
            } => {
                let obj = self.eval_expr(object, env)?;
                let i = self.eval_expr(row, env)?;
                let j = self.eval_expr(col, env)?;
                self.assign_index2(&obj, &i, &j, value, *span)
            }
            AssignTarget::MapField {
                object,
                field,
                span,
            } => self.assign_map_field(object, field, value, *span, env),
        }
    }

    /// Write `value` into `lista[i]` / `mapa[k]` in place, or replace one
    /// character of a texto and rebind the location (`s[2] = "b"`).
    fn assign_indexed(
        &mut self,
        object: &Expr,
        index: &Value,
        value: Value,
        span: Span,
        env: &Rc<RefCell<Env>>,
    ) -> Result<(), EvalError> {
        let obj = self.eval_expr(object, env)?;
        match &obj {
            Value::Lista(_) | Value::Mapa(_) => self.assign_index(&obj, index, value, span),
            Value::Texto(s) => {
                let novo = self.replace_texto_char(s, index, &value, span)?;
                self.assign_location(object, Value::Texto(novo), env, span)
            }
            other => Err(self.err(
                format!("não é possível atribuir índice em {}", other.type_name()),
                span,
            )),
        }
    }

    fn assign_location(
        &mut self,
        loc: &Expr,
        value: Value,
        env: &Rc<RefCell<Env>>,
        span: Span,
    ) -> Result<(), EvalError> {
        match loc {
            Expr::Ident { name, span: nspan } => self.assign_name(env, name, value, *nspan),
            Expr::Index {
                object,
                index,
                span: ispan,
            } => {
                let idx = self.eval_expr(index, env)?;
                self.assign_indexed(object, &idx, value, *ispan, env)
            }
            Expr::Index2 {
                object,
                row,
                col,
                span: ispan,
            } => {
                let obj = self.eval_expr(object, env)?;
                let i = self.eval_expr(row, env)?;
                let j = self.eval_expr(col, env)?;
                self.assign_index2(&obj, &i, &j, value, *ispan)
            }
            Expr::MapField {
                object,
                field,
                span: fspan,
            } => self.assign_map_field(object, field, value, *fspan, env),
            Expr::String { .. } => Err(self.err("não é possível alterar um texto literal", span)),
            _ => Err(self.err("não é possível atribuir neste alvo", span)),
        }
    }

    fn assign_map_field(
        &mut self,
        object: &Expr,
        field: &str,
        value: Value,
        span: Span,
        env: &Rc<RefCell<Env>>,
    ) -> Result<(), EvalError> {
        let obj = self.eval_expr(object, env)?;
        match &obj {
            Value::Mapa(_) => {
                self.assign_index(&obj, &Value::Texto(field.to_string()), value, span)
            }
            Value::Par(_, _) => {
                Err(self.err("par não pode ser alterado; :chave e :valor só leem", span))
            }
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

    fn replace_texto_char(
        &self,
        s: &str,
        index: &Value,
        value: &Value,
        span: Span,
    ) -> Result<String, EvalError> {
        let mut chars: Vec<char> = s.chars().collect();
        let i = self.to_index(index, chars.len(), span)?;
        let Value::Texto(t) = value else {
            return Err(self.err(
                format!(
                    "atribuição em texto espera um caractere, encontrado {}",
                    value.type_name()
                ),
                span,
            ));
        };
        let mut repl = t.chars();
        let Some(ch) = repl.next() else {
            return Err(self.err("atribuição em texto espera um caractere (tamanho 1)", span));
        };
        if repl.next().is_some() {
            return Err(self.err(
                format!(
                    "atribuição em texto espera um caractere (tamanho 1), encontrado {}",
                    t.chars().count()
                ),
                span,
            ));
        }
        chars[i] = ch;
        Ok(chars.into_iter().collect())
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

    pub(crate) fn to_index(
        &self,
        index: &Value,
        len: usize,
        span: Span,
    ) -> Result<usize, EvalError> {
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

    pub(crate) fn to_slice_bounds(
        &self,
        start: &Value,
        end: Option<&Value>,
        len: usize,
        span: Span,
    ) -> Result<(usize, usize), EvalError> {
        let s = self.expect_int(start, span)?;
        if s < 1 {
            return Err(self.err("índices de fatia devem ser >= 1", span));
        }
        let e = match end {
            None => len as i64,
            Some(v) => {
                let e = self.expect_int(v, span)?;
                if e < 1 {
                    return Err(self.err("índices de fatia devem ser >= 1", span));
                }
                e
            }
        };
        if len == 0 || s > len as i64 {
            return Ok((0, 0));
        }
        let e = e.min(len as i64);
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
                format!("não é possível alterar `{name}` (nome nativo)"),
                span,
            ),
        })
    }

    pub(crate) fn check_deadline(&self, span: Span) -> Result<(), EvalError> {
        if let Some(deadline) = self.deadline {
            if Instant::now() >= deadline {
                return Err(self.err("tempo esgotado", span));
            }
        }
        Ok(())
    }

    pub(crate) fn sleep_secs(&mut self, secs: f64, span: Span) -> Result<(), EvalError> {
        if !secs.is_finite() || secs < 0.0 {
            return Err(self.err("durma() espera um número >= 0 (segundos)", span));
        }
        self.check_deadline(span)?;
        if secs == 0.0 {
            return Ok(());
        }
        let want = Duration::try_from_secs_f64(secs)
            .map_err(|_| self.err("durma() duração grande demais", span))?;
        let slice = match self.deadline {
            Some(deadline) => {
                let now = Instant::now();
                if now >= deadline {
                    return Err(self.err("tempo esgotado", span));
                }
                let remaining = deadline.saturating_duration_since(now);
                if want > remaining {
                    thread::sleep(remaining);
                    return Err(self.err("tempo esgotado", span));
                }
                want
            }
            None => want,
        };
        // In short slices, so Parar does not wait for a long `durma`.
        let end = Instant::now() + slice;
        loop {
            let left = end.saturating_duration_since(Instant::now());
            if left.is_zero() {
                break;
            }
            thread::sleep(left.min(Duration::from_millis(50)));
            if self.hook.interrupted() {
                return Err(EvalError::Quit(0));
            }
        }
        self.check_deadline(span)
    }

    fn pause(&mut self, span: Span, env: &Rc<RefCell<Env>>) -> Result<(), EvalError> {
        self.check_deadline(span)?;
        let file = self.file.clone();
        let source = Rc::clone(&self.source);
        let stack = self.stack.clone();
        match self.hook.before_stmt(&DebugCtx {
            file: &file,
            source: &source,
            span,
            env,
            stack: &stack,
        }) {
            DebugAction::Continue => Ok(()),
            DebugAction::Quit => Err(EvalError::Quit(0)),
            DebugAction::Timeout => Err(self.err("tempo esgotado", span)),
        }
    }

    fn retorne_so_em_funcao<T>(&self, result: Result<T, EvalError>) -> Result<T, EvalError> {
        match result {
            Err(EvalError::Return { span, .. }) => {
                Err(self.err("retorne só pode ser usado numa função", span))
            }
            Err(EvalError::Break { span }) => {
                Err(self.err("pare só pode ser usado num laço", span))
            }
            Err(EvalError::Continue { span }) => {
                Err(self.err("continue só pode ser usado num laço", span))
            }
            other => other,
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

fn is_bare_module_spec(spec: &str) -> bool {
    !spec.contains('/')
        && !spec.contains('\\')
        && !spec.starts_with('.')
        && Path::new(spec).extension().is_none()
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

impl Vm<'_> {
    pub(crate) fn format_value(&self, v: &Value) -> String {
        v.format_with(self.numero_locale)
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
        let mut err = Vec::new();
        run_with(
            src,
            "teste.lep",
            Box::new(NoopHook),
            &mut input,
            &mut out,
            &mut err,
            None,
            None,
            None,
            &[],
        )
        .unwrap_or_else(|e| panic!("run failed: {e}"));
        String::from_utf8_lossy(&out).into_owned()
    }

    fn run_args(src: &str, args: &[&str], input: &str) -> String {
        let args: Vec<String> = args.iter().map(|s| (*s).to_string()).collect();
        let mut input = io::Cursor::new(input.to_string());
        let mut out = Vec::new();
        let mut err = Vec::new();
        run_with(
            src,
            "teste.lep",
            Box::new(NoopHook),
            &mut input,
            &mut out,
            &mut err,
            None,
            None,
            None,
            &args,
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
        let src = include_str!("../../exemplos/media.lep");
        let out = run_to_string(src, "media.lep").unwrap();
        assert_eq!(out, "Média: 7,3\nResultado: Aprovado\n");
    }

    #[test]
    fn example_programs_run() {
        let examples = [
            (
                "exemplos/tabuada.lep",
                include_str!("../../exemplos/tabuada.lep"),
            ),
            (
                "exemplos/fatorial.lep",
                include_str!("../../exemplos/fatorial.lep"),
            ),
            (
                "exemplos/filtra.lep",
                include_str!("../../exemplos/filtra.lep"),
            ),
            (
                "exemplos/receita.lep",
                include_str!("../../exemplos/receita.lep"),
            ),
            (
                "exemplos/poema.lep",
                include_str!("../../exemplos/poema.lep"),
            ),
            (
                "exemplos/turma.lep",
                include_str!("../../exemplos/turma.lep"),
            ),
            (
                "exemplos/palindromo.lep",
                include_str!("../../exemplos/palindromo.lep"),
            ),
            (
                "exemplos/ascii.lep",
                include_str!("../../exemplos/ascii.lep"),
            ),
            (
                "exemplos/caixa.lep",
                include_str!("../../exemplos/caixa.lep"),
            ),
            (
                "exemplos/diario.lep",
                include_str!("../../exemplos/diario.lep"),
            ),
            (
                "exemplos/contatos.lep",
                include_str!("../../exemplos/contatos.lep"),
            ),
            (
                "exemplos/usa_matematica.lep",
                include_str!("../../exemplos/usa_matematica.lep"),
            ),
            (
                "exemplos/closures.lep",
                include_str!("../../exemplos/closures.lep"),
            ),
            (
                "exemplos/escola/bhaskara.lep",
                include_str!("../../exemplos/escola/bhaskara.lep"),
            ),
            (
                "exemplos/escola/juros.lep",
                include_str!("../../exemplos/escola/juros.lep"),
            ),
            (
                "exemplos/escola/regra_de_tres.lep",
                include_str!("../../exemplos/escola/regra_de_tres.lep"),
            ),
            (
                "exemplos/escola/progressoes.lep",
                include_str!("../../exemplos/escola/progressoes.lep"),
            ),
            (
                "exemplos/escola/estatistica.lep",
                include_str!("../../exemplos/escola/estatistica.lep"),
            ),
            (
                "exemplos/escola/pitagoras.lep",
                include_str!("../../exemplos/escola/pitagoras.lep"),
            ),
            (
                "exemplos/escola/geometria.lep",
                include_str!("../../exemplos/escola/geometria.lep"),
            ),
            (
                "exemplos/escola/mdc_mmc.lep",
                include_str!("../../exemplos/escola/mdc_mmc.lep"),
            ),
            (
                "exemplos/escola/media_ponderada.lep",
                include_str!("../../exemplos/escola/media_ponderada.lep"),
            ),
            (
                "exemplos/escola/cinematica.lep",
                include_str!("../../exemplos/escola/cinematica.lep"),
            ),
        ];
        for (file, src) in examples {
            run_to_string(src, file).unwrap_or_else(|e| panic!("{file} failed: {e}"));
        }
    }

    #[test]
    fn erros_example_does_not_crash() {
        let src = include_str!("../../exemplos/erros.lep");
        let out = run_to_string(src, "erros.lep").unwrap();
        assert_eq!(out, "");
    }

    #[test]
    fn sair_stops_the_program() {
        let src = r#"
escreva("antes")
sair(1)
escreva("depois")
"#;
        let mut input = io::Cursor::new("");
        let mut out = Vec::new();
        let mut err = Vec::new();
        let code = run_with(
            src,
            "t.lep",
            Box::new(NoopHook),
            &mut input,
            &mut out,
            &mut err,
            None,
            None,
            None,
            &[],
        )
        .unwrap();
        assert_eq!(code, 1);
        assert_eq!(String::from_utf8(out).unwrap(), "antes\n");
        assert_eq!(run(r#"sair()"#), "");
        let skipped = run(r#"
sair(0)
escreva("depois")
"#);
        assert_eq!(skipped, "");
        let not_caught = run(r#"
sair(1) se_falhar 0
escreva("depois")
"#);
        assert_eq!(not_caught, "");
    }

    #[test]
    fn para_em_iterates_text() {
        assert_eq!(
            run(r#"
para ch em "olá"
inicio
    escreva(ch)
fim
"#),
            "o\nl\ná\n"
        );
        assert_eq!(run(r#"para ch em "" { escreva(ch) }"#), "");
        let msg = run_err(
            r#"
para x em 10
inicio
    escreva(x)
fim
"#,
        );
        assert!(msg.contains("lista, texto ou conjunto"), "{msg}");
    }

    #[test]
    fn pare_and_continue_in_loops() {
        assert_eq!(
            run(r#"
para i de 1 ate 5
inicio
    se i == 3 { pare }
    escreva(i)
fim
"#),
            "1\n2\n"
        );
        assert_eq!(
            run(r#"
para i de 1 ate 5
inicio
    se i == 3 { continue }
    escreva(i)
fim
"#),
            "1\n2\n4\n5\n"
        );
        assert_eq!(
            run(r#"
para ch em "abcd"
inicio
    se ch == "c" { pare }
    escreva(ch)
fim
"#),
            "a\nb\n"
        );
        assert_eq!(
            run(r#"
i = 0
enquanto verdadeiro
inicio
    i = i + 1
    se i == 2 { continue }
    escreva(i)
    se i >= 3 { pare }
fim
"#),
            "1\n3\n"
        );
        let msg = run_err("pare");
        assert!(msg.contains("laço"), "{msg}");
        let msg = run_err("continue");
        assert!(msg.contains("laço"), "{msg}");
    }

    #[test]
    fn retorne_leaves_the_function() {
        let src = r#"
busca = funcao(xs, alvo)
inicio
    para x em xs
    inicio
        se x == alvo
        inicio
            retorne verdadeiro
        fim
    fim
    falso
fim
escreva(busca([1, 2, 3], 2))
escreva(busca([1, 2, 3], 9))
"#;
        assert_eq!(run(src), "verdadeiro\nfalso\n");
        assert_eq!(
            run(r#"
f = funcao()
inicio
    retorne
    escreva("nao")
fim
escreva(f())
"#),
            "nada\n"
        );
        assert_eq!(
            run(r#"
soma = funcao(x, y)
inicio
    x + y
fim
escreva(soma(2, 3))
"#),
            "5\n"
        );
        let msg = run_err("retorne 1");
        assert!(msg.contains("função"), "{msg}");
        let skipped = run(r#"
f = funcao()
inicio
    se verdadeiro
    inicio
        retorne 1
    fim
    senao
    inicio
        0
    fim
    se_falhar 9
fim
escreva(f())
"#);
        assert_eq!(skipped, "1\n");
    }

    #[test]
    fn shebang_is_ignored_at_runtime() {
        assert_eq!(run("#!/usr/bin/env expressa\nescreva(2 + 2)\n"), "4\n");
    }

    #[test]
    fn conjunto_unique_contem_union_remova() {
        assert_eq!(
            run(r#"
s = conjunto([:ana, :bia, :ana, 1])
escreva(s contem :ana)
escreva(s contem :carlos)
escreva(tamanho(s))
s += :carlos
escreva(s contem :carlos)
s += conjunto([:bia, :dani])
escreva(tamanho(s))
s = s.remova(:ana)
escreva(s contem :ana)
para x em conjunto([10, 20])
inicio
    escreva(x)
fim
"#),
            "verdadeiro\nfalso\n3\nverdadeiro\n5\nfalso\n10\n20\n"
        );
        assert_eq!(
            run("escreva(conjunto([1, 2]) == conjunto([2, 1]))"),
            "verdadeiro\n"
        );
        assert!(run_err("conjunto([[1]])").contains("inválido"));
    }

    #[test]
    fn compound_assign() {
        assert_eq!(
            run(r#"
i = 10
i += 3
i -= 1
escreva(i)
nome = "Ana"
nome += " Silva"
escreva(nome)
xs = [1, 2]
xs += [3]
escreva(xs)
xs[1] += 10
escreva(xs)
"#),
            "12\nAna Silva\n[1, 2, 3]\n[11, 2, 3]\n"
        );
        assert!(
            run_err(
                r#"xs = [1]
xs -= [1]"#
            )
            .contains("numero")
        );
        assert_eq!(
            run(r#"
tentativas = []
para i de 1 ate 6 {
    tentativas += [["", "", "", "", ""]]
}
escreva(tamanho(tentativas))
"#),
            "6\n"
        );
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
        let msg = run_err(
            r#"
s = "ABC"
f = funcao()
inicio
    s[1] = "x"
fim
f()
"#,
        );
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
        assert_eq!(run("m = mapa([\"a\" -> 1])\nescreva(m[\"a\"])"), "1\n");
        assert_eq!(
            run(r#"
p = mapa([:nome -> "Thiago", :idade -> 25])
escreva(p:nome)
escreva(p[:nome])
escreva(:nome)
p = p.remova(:idade)
escreva(p contem :idade)
"#),
            "Thiago\nThiago\nnome\nfalso\n"
        );
        assert_eq!(
            run(r#"
opções = mapa([:nome -> "Thiago"])
args_pos = ["a"]
res = mapa([
    :op -> opções,
    :args -> args_pos,
])
escreva(res:args[1])
"#),
            "a\n"
        );
    }

    #[test]
    fn matriz_ops() {
        let src = r#"
importe "matriz"
A = matriz([
    [1, 2],
    [3, 4],
])
escreva(A[1, 2])
escreva(tamanho(A))
escreva(tamanho(A[1]))
escreva(nlinhas(A))
escreva(ncolunas(A))
escreva(det(A))
B = transposta(A)
escreva(B[1, 2])
I = identidade(2)
C = A * I
escreva(C[2, 2])
"#;
        assert_eq!(run(src), "2\n2\n2\n2\n2\n-2\n3\n4\n");
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
    fn map_colon_and_ufcs() {
        let src = r#"
p = mapa(["nome" -> "Ana"])
escreva(p:nome)
p:nome = "Bia"
escreva(p:nome)
escreva([10, 20, 30].tamanho())
"#;
        assert_eq!(run(src), "Ana\nBia\n3\n");
    }

    #[test]
    fn maps_and_contem() {
        let src = r#"
p = mapa([
    "nome" -> "Ana",
    "idade" -> 25,
])
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
        assert_eq!(run(r#"escreva("Média: " + 7.5)"#), "Média: 7,5\n");
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
        assert_eq!(run(r#"escreva("b"[1..3])"#), "b\n");
        assert_eq!(run(r#"escreva("bacia"[2..])"#), "acia\n");
        assert_eq!(run(r#"escreva("abc"[5..])"#), "\n");
        assert_eq!(run(r#"escreva([10, 20][1..9])"#), "[10, 20]\n");
        assert!(run_err(r#"escreva("b"[2])"#).contains("fora do intervalo"));
    }

    #[test]
    fn text_index_assign_replaces_one_char() {
        assert_eq!(
            run(r#"
s = "ABCDE"
s[2] = "b"
escreva(s)
"#),
            "AbCDE\n"
        );
        assert_eq!(
            run(r#"
s = "ação"
s[2] = "Ç"
escreva(s)
"#),
            "aÇão\n"
        );
        assert_eq!(
            run(r#"
linhas = ["ABC", "DEF"]
linhas[1][2] = "x"
escreva(linhas)
"#),
            "[\"AxC\", \"DEF\"]\n"
        );
        assert_eq!(
            run(r#"
m = mapa([:s -> "ABC"])
m:s[2] = "x"
escreva(m:s)
"#),
            "AxC\n"
        );
        assert_eq!(
            run(r#"
s = "ABC"
a = s
s[1] = "x"
escreva(a)
escreva(s)
"#),
            "ABC\nxBC\n"
        );
        assert!(
            run_err(
                r#"
s = "ABC"
s[2] = "ab"
"#
            )
            .contains("tamanho 1")
        );
        assert!(
            run_err(
                r#"
s = "ABC"
s[2] = ""
"#
            )
            .contains("tamanho 1")
        );
        assert!(
            run_err(
                r#"
s = "ABC"
s[2] = 1
"#
            )
            .contains("caractere")
        );
        assert!(run_err(r#""ABC"[2] = "b""#).contains("literal"));
        assert!(
            run_err(
                r#"
s = "ABC"
s[0] = "x"
"#
            )
            .contains(">= 1")
        );
        assert!(
            run_err(
                r#"
s = "ABC"
s[4] = "x"
"#
            )
            .contains("fora do intervalo")
        );
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
            dir.join("aritmetica.lep"),
            "soma = funcao(a, b)\ninicio\n    a + b\nfim\n",
        )
        .unwrap();
        let main = dir.join("main.lep");
        let src = r#"
m = importe "aritmetica"
escreva(m::soma(2, 3))
importe "aritmetica"
escreva(soma(4, 5))
"#;
        let out = run_to_string(src, main.to_str().unwrap()).unwrap();
        assert_eq!(out, "5\n9\n");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn par_mapa_conjunto_and_native_modules() {
        assert_eq!(
            run(r#"
p = :nome -> "Ana"
escreva(p:chave)
escreva(p:valor)
m = mapa([p, :idade -> 25])
escreva(m:nome)
escreva(tamanho(m))
s = conjunto([1, 1, 2])
escreva(tamanho(s))
escreva(tamanho(mapa()))
escreva(tamanho(conjunto()))
"#),
            "nome\nAna\nAna\n2\n2\n0\n0\n"
        );
        assert!(run_err("1 -> 2 -> 3").contains("encadeados"));
        assert!(run_err("mapa([:a -> 1, :a -> 2])").contains("duplicada"));
        assert!(run_err("[] -> 1").contains("chave de par"));
        assert_eq!(
            run(r#"
importe "matriz"
A = matriz([[1, 2], [3, 4]])
escreva(A.transposta()[1, 2])
m = importe "matriz"
escreva(A.m::transposta()[1, 2])
escreva(m::nlinhas(A))
"#),
            "3\n3\n2\n"
        );
        assert_eq!(
            run(r#"
matriz = importe "matriz"
A = matriz::matriz([[1, 2], [3, 4]])
escreva(A.matriz::transposta()[1, 2])
"#),
            "3\n"
        );
        assert!(run_err("zeros(2)").contains("não definida"));
    }

    #[test]
    fn native_import_conflicts_with_file() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("expressa-nat-{nanos}"));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("matriz.lep"), "x = 1\n").unwrap();
        let main = dir.join("main.lep");
        let err = match run_to_string(r#"importe "matriz""#, main.to_str().unwrap()) {
            Err(e) => e.message,
            Ok(out) => panic!("expected conflict, got {out:?}"),
        };
        assert!(err.contains("módulo nativo"), "{err}");
        assert!(err.contains("./matriz"), "{err}");
        let out = run_to_string(
            r#"
importe "./matriz"
escreva(x)
"#,
            main.to_str().unwrap(),
        )
        .unwrap();
        assert_eq!(out, "1\n");
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
        assert_eq!(base_dir_of("exemplos/media.lep").as_os_str(), "exemplos");
        assert_eq!(base_dir_of("media.lep").as_os_str(), ".");
        assert_eq!(Value::Numero(3.0).format_with(NumeroLocale::PtBr), "3");
        assert_eq!(
            Value::Texto("a".into()).format_with(NumeroLocale::PtBr),
            "a"
        );
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
        let src = r#"
importe "arquivo"
leia_arquivo("../secret.txt")
"#;
        let file = dir.join("main.lep");
        let err = run_to_string_with(src, file.to_str().unwrap(), "", Some(dir.clone()), None)
            .expect_err("should block escape");
        assert!(err.message.contains("fora da pasta"), "{err}");
    }

    /// Runs `src` under a ChannelDebugger, answering every pause with
    /// `continuar`; returns the pauses seen and the result.
    fn debug_pauses(
        src: &str,
    ) -> (
        Vec<super::super::debug::DebugPaused>,
        Result<i32, RuntimeError>,
    ) {
        use super::super::debug::ChannelDebugger;
        let (pause_tx, pause_rx) = std::sync::mpsc::channel();
        let (cmd_tx, cmd_rx) = std::sync::mpsc::channel::<String>();
        let src = src.to_string();
        let runner = std::thread::spawn(move || {
            let mut input = io::Cursor::new("");
            let mut out = Vec::new();
            let mut err = Vec::new();
            run_with(
                &src,
                "t.lep",
                Box::new(ChannelDebugger::new(pause_tx, cmd_rx)),
                &mut input,
                &mut out,
                &mut err,
                None,
                None,
                None,
                &[],
            )
        });
        let mut pauses = Vec::new();
        while let Ok(p) = pause_rx.recv() {
            pauses.push(p);
            let _ = cmd_tx.send("continuar".into());
        }
        (pauses, runner.join().unwrap())
    }

    /// Starts `src` under a ChannelDebugger on a thread: (pauses, commands,
    /// finished) channels.
    fn debug_session(
        src: &str,
    ) -> (
        std::sync::mpsc::Receiver<super::super::debug::DebugPaused>,
        std::sync::mpsc::Sender<String>,
        std::sync::mpsc::Receiver<Result<i32, RuntimeError>>,
    ) {
        use super::super::debug::ChannelDebugger;
        let (pause_tx, pause_rx) = std::sync::mpsc::channel();
        let (cmd_tx, cmd_rx) = std::sync::mpsc::channel::<String>();
        let (done_tx, done_rx) = std::sync::mpsc::channel();
        let src = src.to_string();
        std::thread::spawn(move || {
            let mut input = io::Cursor::new("");
            let (mut out, mut err) = (Vec::new(), Vec::new());
            let r = run_with(
                &src,
                "t.lep",
                Box::new(ChannelDebugger::new(pause_tx, cmd_rx)),
                &mut input,
                &mut out,
                &mut err,
                None,
                None,
                None,
                &[],
            );
            let _ = done_tx.send(r);
        });
        (pause_rx, cmd_tx, done_rx)
    }

    const WAIT: std::time::Duration = std::time::Duration::from_secs(5);

    #[test]
    fn debugger_stops_when_asked_while_running() {
        let (pauses, cmds, done) = debug_session("enquanto verdadeiro {\n    x = 1\n}\n");
        pauses.recv_timeout(WAIT).expect("pause on start");
        cmds.send("continuar".into()).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(50));
        cmds.send("terminar".into()).unwrap();
        assert!(
            done.recv_timeout(WAIT).is_ok(),
            "Parar after Continuar must end the program"
        );
    }

    #[test]
    fn breakpoint_added_while_running_is_hit() {
        let (pauses, cmds, done) =
            debug_session("enquanto verdadeiro {\n    x = 1\n    y = 2\n}\n");
        pauses.recv_timeout(WAIT).expect("pause on start");
        cmds.send("continuar".into()).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(50));
        cmds.send("ponto t.lep:3".into()).unwrap();
        let p = pauses
            .recv_timeout(WAIT)
            .expect("stops at the new breakpoint");
        assert_eq!(p.line, 3);
        // Removed while running: Continuar runs freely again until Parar.
        cmds.send("remover t.lep:3".into()).unwrap();
        // While paused, a breakpoint change answers with a fresh snapshot.
        assert_eq!(pauses.recv_timeout(WAIT).unwrap().line, 3);
        cmds.send("continuar".into()).unwrap();
        assert!(
            pauses
                .recv_timeout(std::time::Duration::from_millis(300))
                .is_err(),
            "breakpoint was removed"
        );
        cmds.send("terminar".into()).unwrap();
        assert!(done.recv_timeout(WAIT).is_ok());
    }

    #[test]
    fn parar_interrupts_a_long_durma() {
        let started = std::time::Instant::now();
        let (pauses, cmds, done) = debug_session("durma(30)\n");
        pauses.recv_timeout(WAIT).expect("pause on start");
        cmds.send("continuar".into()).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(100));
        cmds.send("terminar".into()).unwrap();
        assert!(done.recv_timeout(WAIT).is_ok());
        assert!(started.elapsed() < std::time::Duration::from_secs(5));

        // Same with Rodar's hook.
        use super::super::debug::StopHook;
        let (tx, rx) = std::sync::mpsc::channel::<String>();
        let (done_tx, done_rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let mut input = io::Cursor::new("");
            let (mut out, mut err) = (Vec::new(), Vec::new());
            let r = run_with(
                "durma(30)\n",
                "t.lep",
                Box::new(StopHook::new(rx)),
                &mut input,
                &mut out,
                &mut err,
                None,
                None,
                None,
                &[],
            );
            let _ = done_tx.send(r);
        });
        std::thread::sleep(std::time::Duration::from_millis(100));
        tx.send("terminar".into()).unwrap();
        assert!(
            done_rx.recv_timeout(WAIT).is_ok(),
            "Rodar: Parar during durma"
        );
    }

    #[test]
    fn debugger_pauses_on_runtime_error_with_locals() {
        let (pauses, result) = debug_pauses("f = funcao(x) {\n    y = x * 2\n    y / 0\n}\nf(3)\n");
        assert!(result.is_err());
        // First the pause on start, then the error.
        assert_eq!(pauses.len(), 2, "{pauses:?}");
        let p = &pauses[1];
        assert!(p.error.as_deref().unwrap_or("").contains("zero"), "{p:?}");
        assert_eq!(p.line, 3);
        let var = |n: &str| p.vars.iter().find(|v| v.name == n).map(|v| v.value.clone());
        assert_eq!(var("x").as_deref(), Some("3"));
        assert_eq!(var("y").as_deref(), Some("6"));
        assert_eq!(p.stack[0].name, "f", "innermost frame first: {:?}", p.stack);
    }

    #[test]
    fn errors_and_pauses_in_imported_functions_name_their_file() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("expressa-mod-file-{nanos}"));
        std::fs::create_dir_all(dir.join("lib")).unwrap();
        std::fs::write(
            dir.join("lib/m.lep"),
            "divide = funcao(a, b) {\n    q = a / b\n    q\n}\n",
        )
        .unwrap();
        let main = dir.join("main.lep");
        let src = "importe \"lib/m\"\nescreva(divide(1, 0))\n";
        let err = run_to_string(src, main.to_str().unwrap()).unwrap_err();
        assert!(err.file.ends_with("lib/m.lep"), "{err}");
        assert_eq!(err.span.line, 2);
        // The call site stays in the caller's file.
        assert!(
            err.stack.last().unwrap().file.ends_with("main.lep"),
            "{:?}",
            err.stack
        );

        // The debugger stops in the module, showing the module's line.
        use super::super::debug::ChannelDebugger;
        let (pause_tx, pause_rx) = std::sync::mpsc::channel();
        let (cmd_tx, cmd_rx) = std::sync::mpsc::channel::<String>();
        let main_s = main.to_str().unwrap().to_string();
        let runner = std::thread::spawn(move || {
            let mut input = io::Cursor::new("");
            let (mut out, mut errb) = (Vec::new(), Vec::new());
            run_with(
                src,
                &main_s,
                Box::new(ChannelDebugger::new(pause_tx, cmd_rx)),
                &mut input,
                &mut out,
                &mut errb,
                None,
                None,
                None,
                &[],
            )
        });
        let mut last = None;
        while let Ok(p) = pause_rx.recv() {
            let _ = cmd_tx.send("continuar".into());
            last = Some(p);
        }
        let _ = runner.join();
        let p = last.unwrap();
        assert!(p.error.is_some());
        assert!(p.file.ends_with("lib/m.lep"), "{p:?}");
        assert_eq!(p.source_line.trim(), "q = a / b");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn debugger_ignores_errors_handled_by_se_falhar() {
        let (pauses, result) = debug_pauses("x = 1 / 0 se_falhar 7\nescreva(x)\n");
        assert_eq!(result.unwrap(), 0);
        assert!(pauses.iter().all(|p| p.error.is_none()), "{pauses:?}");
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
        let mut err = Vec::new();
        run_with(
            r#"escreva(leia("Seu nome:"))"#,
            "t.lep",
            Box::new(NoopHook),
            &mut input,
            &mut out,
            &mut err,
            None,
            None,
            Some(&mut host),
            &[],
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
        let mut err = Vec::new();
        let mut input = io::Cursor::new("");
        run_with(
            src,
            "t.lep",
            Box::new(Shared(Rc::clone(&n))),
            &mut input,
            &mut buf,
            &mut err,
            None,
            None,
            None,
            &[],
        )
        .unwrap();
        assert_eq!(n.get(), 3);
        assert_eq!(String::from_utf8(buf).unwrap(), "3\n");
    }

    #[test]
    fn enquanto_counts() {
        let src = r#"
i = 1
enquanto i <= 3
inicio
    escreva(i)
    i = i + 1
fim
"#;
        assert_eq!(run(src), "1\n2\n3\n");
        assert_eq!(
            run(r#"
i = 0
enquanto falso
inicio
    escreva(i)
fim
escreva("ok")
"#),
            "ok\n"
        );
        let msg = run_err(
            r#"
enquanto 1
inicio
    escreva(1)
fim
"#,
        );
        assert!(msg.contains("esperado bool"), "{msg}");
    }

    #[test]
    fn leia_linhas_reads_until_eof() {
        assert_eq!(
            run_in(r#"escreva(leia_linhas())"#, "a\nb\nc\n"),
            "[\"a\", \"b\", \"c\"]\n"
        );
        assert_eq!(run_in(r#"escreva(leia_linhas())"#, ""), "[]\n");
        assert_eq!(
            run_in(
                r#"
a = leia()
escreva(a)
escreva(leia_linhas())
"#,
                "um\ndois\ntres\n"
            ),
            "um\n[\"dois\", \"tres\"]\n"
        );
        assert_eq!(
            run_in(r#"escreva(leia_linhas())"#, "sozinha"),
            "[\"sozinha\"]\n"
        );
    }

    #[test]
    fn argumentos_are_one_based() {
        assert_eq!(run(r#"escreva(argumentos)"#), "[]\n");
        assert_eq!(
            run_args(r#"escreva(argumentos[1])"#, &["Thiago"], ""),
            "Thiago\n"
        );
        assert_eq!(
            run_args(r#"escreva(tamanho(argumentos))"#, &["a", "b"], ""),
            "2\n"
        );
        let msg = run_err(r#"argumentos = 1"#);
        assert!(msg.contains("nome nativo"), "{msg}");
    }

    #[test]
    fn grep_filter_with_args_and_leia_linhas() {
        let src = r#"
busca = argumentos[1]
para linha em leia_linhas()
inicio
    se linha contem busca
    inicio
        escreva(linha)
    fim
fim
"#;
        assert_eq!(
            run_args(src, &["Thiago"], "Ana\nThiago Silva\nBruno\nThiago\n"),
            "Thiago Silva\nThiago\n"
        );
    }
}
