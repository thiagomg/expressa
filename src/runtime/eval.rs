mod assignment;
mod exb;
pub use bytecode::format::{
    CompiledProgram, EXB_MAX_BYTES, EXB_VERSION, ExbError, compile_exb, load_exb, read_exb,
};
pub use exb::{ExbRunMode, run_compiled_with_options, run_exb_file_args, run_exb_with_options};
pub(crate) mod bytecode;
pub use bytecode::ExecutionBackend;
mod matrix;

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::io::{self, BufRead, IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::rc::{Rc, Weak};
use std::thread;
use std::time::{Duration, Instant};

use crate::lexer::Span;
use crate::parser::{
    AssignTarget, BinaryOp, Block, Expr, Import, Item, Param, Program, Stmt, UnaryOp, parse,
};

use super::cartesiano_host::{CartesianoHost, CliCartesianoHost};
use super::debug::{DebugAction, DebugCtx, DebugHook, NoopHook};
use super::env::{AssignError, Env, FrameKind};
use super::error::{CallFrame, EvalError, RuntimeError};
use super::graf_host::{CliGrafHost, GrafHost};
use super::leia::LeiaHost;
use super::nativas;
use super::numero::{Numero, NumeroAritmetica, default_numero_aritmetica, parse_numero};
use super::pilha::{PILHA_RESERVA, PILHA_SEGMENTO, PROFUNDIDADE_MAXIMA};
use super::rng::Rng;
use super::value::{
    Closure, FunctionBody, ListaRc, MapKey, NumeroLocale, Value, default_numero_locale,
    format_numero,
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

pub(crate) struct Vm<'a, 'c> {
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
    pub(crate) native_modules: HashMap<String, Rc<RefCell<Env>>>,
    /// If set, file I/O and `importe` must stay under this directory.
    pub(crate) workspace_root: Option<PathBuf>,
    deadline: Option<Instant>,
    pub(crate) max_matrix_cells: usize,
    backend: ExecutionBackend,
    pub(crate) leia_host: Option<&'a mut dyn LeiaHost>,
    pub(crate) cartesiano_host: Option<&'c mut dyn CartesianoHost>,
    pub(crate) cli_cart: CliCartesianoHost,
    pub(crate) graf_host: Option<&'c mut dyn GrafHost>,
    pub(crate) cli_graf: CliGrafHost,
    pub(crate) abrir_cartesiano: bool,
    pub(crate) numero_locale: NumeroLocale,
    pub(crate) numero_aritmetica: NumeroAritmetica,
    pub(crate) rng: Rng,
    /// Depth of `se_falhar` attempts being evaluated: errors there are
    /// handled, so the debugger must not stop on them.
    protected: u32,
    /// The debugger already saw the runtime error now unwinding.
    error_reported: bool,
    /// Map identities currently inside `:_como_texto` (recursion guard).
    como_texto_guard: HashSet<usize>,
    /// Frames where a function was defined ([`Env::marcar_funcao`]). Those
    /// the end-of-frame check could not free are emptied by
    /// [`Vm::liberar_frames`] when the run ends.
    frames_com_funcao: Vec<Weak<RefCell<Env>>>,
    /// Length of `frames_com_funcao` at which dead entries are dropped.
    podar_em: usize,
}

impl Drop for Vm<'_, '_> {
    fn drop(&mut self) {
        self.liberar_frames();
        // Modules can own collection cycles without ever defining a function.
        // Release their roots before the final collection pass.
        self.modules.clear();
        self.native_modules.clear();
        if Rc::strong_count(&self.builtins) == 1 {
            self.builtins.borrow_mut().vars.clear();
        }
        super::value::collect_cycles();
    }
}

/// Per-matrix desktop default, not a language-level dimension restriction.
pub const DEFAULT_MAX_MATRIX_CELLS: usize = 1_048_576;

/// Host policy for captured execution. Limits do not change Expressa's numeric types.
#[derive(Debug, Clone)]
pub struct ExecutionOptions {
    pub backend: ExecutionBackend,
    pub workspace_root: Option<PathBuf>,
    /// Cooperative evaluation deadline; parsing and blocking host I/O are not preempted.
    pub time_limit: Option<Duration>,
    /// Maximum number of cells in each matrix, including arithmetic temporaries.
    pub max_matrix_cells: usize,
}

impl Default for ExecutionOptions {
    fn default() -> Self {
        Self {
            backend: bytecode::default_backend(),
            workspace_root: None,
            time_limit: None,
            max_matrix_cells: DEFAULT_MAX_MATRIX_CELLS,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

/// Execute with explicit host limits and capture both output streams.
pub fn run_with_options(
    source: &str,
    file: &str,
    stdin: &str,
    options: ExecutionOptions,
) -> Result<ExecutionOutput, RuntimeError> {
    let mut input = io::Cursor::new(stdin);
    let mut out = Vec::new();
    let mut err = Vec::new();
    let exit_code = run_configured_with_io(
        source,
        file,
        Box::new(NoopHook),
        &mut input,
        &mut out,
        &mut err,
        options,
        None,
        None,
        false,
        None,
        &[],
    )?;
    Ok(ExecutionOutput {
        stdout: String::from_utf8_lossy(&out).into_owned(),
        stderr: String::from_utf8_lossy(&err).into_owned(),
        exit_code,
    })
}

/// First size at which `frames_com_funcao` drops dead entries.
const PODA_INICIAL: usize = 1024;

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
    run_with_io(
        source,
        file,
        Box::new(NoopHook),
        &mut input,
        &mut stdout,
        &mut stderr,
        None,
        None,
        None,
        None,
        true,
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
    run_with_io(
        source,
        file,
        Box::new(NoopHook),
        &mut dummy_in,
        &mut stdout,
        &mut stderr,
        None,
        None,
        Some(&mut host),
        None,
        true,
        None,
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
    run_with_io(
        source,
        file,
        Box::new(super::debug::CliDebugger::new()),
        &mut input,
        &mut stdout,
        &mut stderr,
        None,
        None,
        None,
        None,
        true,
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
    run_with_options(
        source,
        file,
        stdin,
        ExecutionOptions {
            workspace_root,
            time_limit,
            ..ExecutionOptions::default()
        },
    )
    .map(|output| output.stdout)
}

/// Like [`run_to_string_with`], also capturing stderr (`escreva_erro`).
pub fn run_to_strings_with(
    source: &str,
    file: &str,
    stdin: &str,
) -> Result<(String, String), RuntimeError> {
    run_with_options(source, file, stdin, ExecutionOptions::default())
        .map(|output| (output.stdout, output.stderr))
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
    run_with_hosts(
        source,
        file,
        hook,
        workspace_root,
        time_limit,
        out,
        err,
        leia_host,
        None,
        None,
        script_args,
    )
}

pub fn run_with_hosts<'c>(
    source: &str,
    file: &str,
    hook: Box<dyn DebugHook>,
    workspace_root: Option<PathBuf>,
    time_limit: Option<Duration>,
    out: &mut dyn Write,
    err: &mut dyn Write,
    leia_host: &mut dyn LeiaHost,
    cartesiano_host: Option<&'c mut dyn CartesianoHost>,
    graf_host: Option<&'c mut dyn GrafHost>,
    script_args: &[String],
) -> Result<i32, RuntimeError> {
    let mut dummy_in = io::Cursor::new("");
    run_with_io(
        source,
        file,
        hook,
        &mut dummy_in,
        out,
        err,
        workspace_root,
        time_limit,
        Some(leia_host),
        cartesiano_host,
        false,
        graf_host,
        script_args,
    )
}

#[cfg(test)]
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
    run_with_io(
        source,
        file,
        hook,
        input,
        out,
        err,
        workspace_root,
        time_limit,
        leia_host,
        None,
        false,
        None,
        script_args,
    )
}

pub(crate) fn run_with_io<'a, 'c>(
    source: &str,
    file: &str,
    hook: Box<dyn DebugHook>,
    input: &'a mut dyn LineInput,
    out: &'a mut dyn Write,
    err: &'a mut dyn Write,
    workspace_root: Option<PathBuf>,
    time_limit: Option<Duration>,
    leia_host: Option<&'a mut dyn LeiaHost>,
    cartesiano_host: Option<&'c mut dyn CartesianoHost>,
    abrir_cartesiano: bool,
    graf_host: Option<&'c mut dyn GrafHost>,
    script_args: &[String],
) -> Result<i32, RuntimeError> {
    run_configured_with_io(
        source,
        file,
        hook,
        input,
        out,
        err,
        ExecutionOptions {
            workspace_root,
            time_limit,
            ..ExecutionOptions::default()
        },
        leia_host,
        cartesiano_host,
        abrir_cartesiano,
        graf_host,
        script_args,
    )
}

fn run_configured_with_io<'a, 'c>(
    source: &str,
    file: &str,
    hook: Box<dyn DebugHook>,
    input: &'a mut dyn LineInput,
    out: &'a mut dyn Write,
    err: &'a mut dyn Write,
    options: ExecutionOptions,
    leia_host: Option<&'a mut dyn LeiaHost>,
    cartesiano_host: Option<&'c mut dyn CartesianoHost>,
    abrir_cartesiano: bool,
    graf_host: Option<&'c mut dyn GrafHost>,
    script_args: &[String],
) -> Result<i32, RuntimeError> {
    if options
        .time_limit
        .is_some_and(|limit| Instant::now().checked_add(limit).is_none())
    {
        return Err(RuntimeError {
            message: "limite de tempo fora do intervalo".into(),
            file: file.into(),
            span: Span::default(),
            stack: vec![],
        });
    }
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
        options.workspace_root,
        options.time_limit,
        leia_host,
        cartesiano_host,
        abrir_cartesiano,
        graf_host,
        script_args,
    );
    vm.max_matrix_cells = options.max_matrix_cells;
    vm.backend = options.backend;
    match vm.run(program) {
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

impl<'a, 'c> Vm<'a, 'c> {
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
        cartesiano_host: Option<&'c mut dyn CartesianoHost>,
        abrir_cartesiano: bool,
        graf_host: Option<&'c mut dyn GrafHost>,
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
            deadline: time_limit.and_then(|d| Instant::now().checked_add(d)),
            max_matrix_cells: DEFAULT_MAX_MATRIX_CELLS,
            backend: bytecode::default_backend(),
            leia_host,
            cartesiano_host,
            cli_cart: CliCartesianoHost::new(),
            graf_host,
            cli_graf: CliGrafHost::default(),
            abrir_cartesiano,
            numero_locale: default_numero_locale(),
            numero_aritmetica: default_numero_aritmetica(),
            rng: Rng::new(),
            protected: 0,
            error_reported: false,
            como_texto_guard: HashSet::new(),
            frames_com_funcao: Vec::new(),
            podar_em: PODA_INICIAL,
        }
    }

    fn run(&mut self, program: Program) -> Result<(), EvalError> {
        let env = Env::child(&self.builtins, FrameKind::Module);
        let result = self.eval_program(program, &env);
        drop(env);
        self.liberar_frames();
        result
    }

    /// A function value was just created in `env`.
    fn marcar_funcao(&mut self, env: &Rc<RefCell<Env>>) {
        let frames = &mut self.frames_com_funcao;
        Env::marcar_funcao(env, |e| frames.push(Rc::downgrade(e)));
        if frames.len() >= self.podar_em {
            // A frame left alone at its end because a function escaped is
            // garbage once that function is dropped: check it again. The
            // upgrade is the only handle, as the evaluator's is at frame
            // exit; one at a time, so a handle never counts against another
            // frame's check. Frames still running have a second handle and
            // are skipped.
            for frame in frames.iter() {
                if let Some(frame) = frame.upgrade() {
                    Env::liberar_se_lixo(&frame);
                }
            }
            // Freed frames are dead entries; drop them so a long loop does
            // not grow this list forever.
            frames.retain(|w| w.strong_count() > 0);
            self.podar_em = (frames.len() * 2).max(PODA_INICIAL);
        }
    }

    /// End of the run: empties every frame with a function that is still
    /// alive. Frames the end-of-frame check left alone (a function escaped
    /// and later became garbage) are freed here instead of leaking.
    pub(crate) fn liberar_frames(&mut self) {
        for frame in std::mem::take(&mut self.frames_com_funcao) {
            if let Some(frame) = frame.upgrade() {
                let vars = std::mem::take(&mut frame.borrow_mut().vars);
                drop(frame);
                drop(vars);
            }
        }
    }

    fn eval_program(&mut self, program: Program, env: &Rc<RefCell<Env>>) -> Result<(), EvalError> {
        self.stack.push(CallFrame {
            name: "<modulo>".into(),
            file: self.file.clone(),
            span: program.span,
        });
        let result = if self.backend == ExecutionBackend::Bytecode {
            self.execute_module(program, env).map(|_| ())
        } else {
            (|| {
                for item in &program.items {
                    self.eval_item(item, env)?;
                }
                Ok(())
            })()
        };
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
        if self.backend == ExecutionBackend::Bytecode {
            return self.execute_items(items, env);
        }
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
        if is_bare_module_spec(&import.path)
            && let Some(native) = self.native_modules.get(&import.path).cloned()
        {
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
            enum Input {
                Source(String, Program),
                Compiled(CompiledProgram),
            }
            let input = if exb::is_exb_path(&path) {
                if self.backend != ExecutionBackend::Bytecode {
                    return Err(self.err("importar .exb exige o backend bytecode", import.span));
                }
                Input::Compiled(read_exb(&path).map_err(|e| {
                    self.err(
                        format!("não foi possível importar '{}': {e}", path.display()),
                        import.span,
                    )
                })?)
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

                Input::Source(source, program)
            };

            self.loading.insert(key.clone());

            let saved_file = self.file.clone();
            let saved_source = std::mem::replace(&mut self.source, Rc::from(""));
            let saved_base = self.base_dir.clone();

            self.base_dir = base_dir_of(&path.display().to_string());
            let module_env = Env::child(&self.builtins, FrameKind::Module);
            let result = match input {
                Input::Source(source, program) => {
                    self.file = path.display().to_string();
                    self.source = Rc::from(source);
                    self.eval_program(program, &module_env)
                }
                Input::Compiled(program) => {
                    self.file = program.source_name.clone();
                    self.source = Rc::clone(&program.source);
                    self.eval_loaded_module(&program, &module_env)
                }
            };

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
            // Validate the complete export set before publishing any binding.
            for (name, _) in &bindings {
                if Env::definido_no_programa(env, name) {
                    return Err(self.err(
                        format!(
                            "já existe `{name}` neste escopo. Use um apelido: m = importe \"{}\"",
                            import.path
                        ),
                        import.span,
                    ));
                }
                Env::check_assign(env, name).map_err(|e| self.assignment_error(e, import.span))?;
            }
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
        match &self.workspace_root {
            None => Ok(path),
            Some(root) => self.confine_in(path, root, span),
        }
    }

    pub(crate) fn confine_in(
        &self,
        path: PathBuf,
        root: &Path,
        span: Span,
    ) -> Result<PathBuf, EvalError> {
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
        self.report_runtime_error(&result, env);
        result
    }

    fn report_runtime_error(&mut self, result: &Result<Value, EvalError>, env: &Rc<RefCell<Env>>) {
        if let Err(error) = result {
            self.report_eval_error(error, env);
        }
    }

    fn report_eval_error(&mut self, error: &EvalError, env: &Rc<RefCell<Env>>) {
        if let EvalError::Runtime(e) = error {
            // The innermost statement sees the error first, with its
            // variables still in scope: let the debugger show them.
            if self.protected == 0 && !self.error_reported {
                self.error_reported = true;
                self.hook.on_error(
                    &DebugCtx {
                        file: &self.file,
                        source: &self.source,
                        span: e.span,
                        env,
                        stack: &self.stack,
                    },
                    &e.message,
                );
            }
        }
    }

    fn eval_stmt_inner(&mut self, stmt: &Stmt, env: &Rc<RefCell<Env>>) -> Result<Value, EvalError> {
        match stmt {
            Stmt::Expr { expr, .. } => self.eval_expr(expr, env),
            Stmt::Assign {
                target,
                compound,
                value,
                span,
                ..
            } => {
                if let Some(op) = compound {
                    // Read the resolved destination before the RHS, just as a + b.
                    let destination = self.resolve_target(target, env, true)?;
                    let left = self.read_destination(&destination)?;
                    let right = self.eval_expr(value, env)?;
                    let result = self.apply_binary(&left, *op, &right, *span, env)?;
                    self.write_destination(destination, result)?;
                } else {
                    // Ordinary assignment retains its existing RHS-first order.
                    let value = self.eval_expr(value, env)?;
                    let destination = self.resolve_target(target, env, false)?;
                    self.write_destination(destination, value)?;
                }
                Ok(Value::Nada)
            }
            Stmt::Repita { count, body, .. } => {
                let count_v = self.eval_expr(count, env)?;
                let n = self.expect_int(&count_v, count.span())?;
                if n < 0 {
                    return Err(self.err("'repita' espera um número >= 0", count.span()));
                }
                self.eval_in_child(env, FrameKind::Block, |vm, loop_env| {
                    for _ in 0..n {
                        if vm.eval_loop_body(&body.stmts, loop_env)? {
                            break;
                        }
                    }
                    Ok(Value::Nada)
                })
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
                self.eval_in_child(env, FrameKind::Block, |vm, loop_env| {
                    let mut i = start;
                    while i <= end {
                        loop_env
                            .borrow_mut()
                            .define(var.clone(), Value::Numero(Numero::from_i64(i)));
                        if vm.eval_loop_body(&body.stmts, loop_env)? {
                            break;
                        }
                        if i == end {
                            break;
                        }
                        i += 1;
                    }
                    Ok(Value::Nada)
                })
            }
            Stmt::ParaIn {
                var, iter, body, ..
            } => {
                let seq = self.eval_expr(iter, env)?;
                if let Value::Processo(p) = &seq {
                    // Lines as they arrive; leaving the loop stops the command.
                    let p = Rc::clone(p);
                    let r = self.eval_in_child(env, FrameKind::Block, |vm, loop_env| {
                        while let Some(linha) = vm.processo_proxima(&p, iter.span(), loop_env)? {
                            loop_env
                                .borrow_mut()
                                .define(var.clone(), Value::Texto(linha));
                            if vm.eval_loop_body(&body.stmts, loop_env)? {
                                break;
                            }
                        }
                        Ok(Value::Nada)
                    });
                    p.borrow_mut().encerre();
                    return r;
                }
                let items = self.iter_items(&seq, iter.span())?;
                self.eval_in_child(env, FrameKind::Block, |vm, loop_env| {
                    for item in items {
                        loop_env.borrow_mut().define(var.clone(), item);
                        if vm.eval_loop_body(&body.stmts, loop_env)? {
                            break;
                        }
                    }
                    Ok(Value::Nada)
                })
            }
            Stmt::Enquanto { cond, body, span } => {
                self.eval_in_child(env, FrameKind::Block, |vm, loop_env| {
                    loop {
                        vm.check_deadline(*span)?;
                        let cond_v = vm.eval_expr(cond, env)?;
                        if !vm.expect_bool(&cond_v, cond.span())? {
                            break;
                        }
                        if vm.eval_loop_body(&body.stmts, loop_env)? {
                            break;
                        }
                    }
                    Ok(Value::Nada)
                })
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
        self.eval_in_child(env, FrameKind::Block, |vm, child| {
            vm.eval_stmts(&block.stmts, child)
        })
    }

    /// Runs `f` in a fresh frame and breaks `Rc` cycles before the frame
    /// is dropped (`g = funcao() { … }` inside a function or block).
    fn eval_in_child(
        &mut self,
        parent: &Rc<RefCell<Env>>,
        kind: FrameKind,
        f: impl FnOnce(&mut Self, &Rc<RefCell<Env>>) -> Result<Value, EvalError>,
    ) -> Result<Value, EvalError> {
        let child = Env::child(parent, kind);
        let result = f(self, &child);
        Env::liberar_se_lixo(&child);
        result
    }

    /// Every recursion in the evaluator (calls, nested expressions, blocks)
    /// passes through here, so this is where the Rust stack is grown before
    /// it can overflow — on any thread, in debug or release.
    fn eval_expr(&mut self, expr: &Expr, env: &Rc<RefCell<Env>>) -> Result<Value, EvalError> {
        #[cfg(test)]
        assert_eq!(
            self.backend,
            ExecutionBackend::Ast,
            "bytecode must not fall back to AST execution"
        );
        stacker::maybe_grow(PILHA_RESERVA, PILHA_SEGMENTO, || {
            self.eval_expr_inner(expr, env)
        })
    }

    fn eval_expr_inner(&mut self, expr: &Expr, env: &Rc<RefCell<Env>>) -> Result<Value, EvalError> {
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
            Expr::Function {
                params,
                body,
                span,
                doc,
            } => {
                check_unique_params(params)
                    .map_err(|name| self.err(format!("parâmetro duplicado `{name}`"), *span))?;
                self.marcar_funcao(env);
                Ok(Value::Funcao(Rc::new(Closure {
                    params: params.clone(),
                    body: FunctionBody::Ast(Rc::clone(body)),
                    env: Rc::clone(env),
                    span: *span,
                    file: self.file.clone(),
                    source: Rc::clone(&self.source),
                    doc: doc.clone(),
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
                        Value::Matriz(_) => self.matriz_scale(&v, &v, -1.0, *span),
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
            } => {
                self.protected += 1;
                let attempt = self.eval_expr(attempt, env);
                self.protected -= 1;
                match attempt {
                    Ok(v) => Ok(v),
                    Err(EvalError::Quit(code)) => Err(EvalError::Quit(code)),
                    Err(EvalError::Return { value, span }) => {
                        Err(EvalError::Return { value, span })
                    }
                    Err(EvalError::Break { span }) => Err(EvalError::Break { span }),
                    Err(EvalError::Continue { span }) => Err(EvalError::Continue { span }),
                    Err(EvalError::Runtime(_)) => self.eval_expr(fallback, env),
                }
            }
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
                self.apply_binary(&l, op, &r, span, env)
            }
        }
    }

    fn apply_binary(
        &mut self,
        l: &Value,
        op: BinaryOp,
        r: &Value,
        span: Span,
        env: &Rc<RefCell<Env>>,
    ) -> Result<Value, EvalError> {
        match op {
            BinaryOp::Add => match (l, r) {
                (Value::Numero(a), Value::Numero(b)) => Ok(Value::Numero(
                    a.add(*b, self.numero_aritmetica)
                        .map_err(|m| self.err(m, span))?,
                )),
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
                    self.como_texto_de(l, span, env)?,
                    self.como_texto_de(r, span, env)?
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
                    self.expect_numero(l, span)?
                        .sub(self.expect_numero(r, span)?, self.numero_aritmetica)
                        .map_err(|m| self.err(m, span))?,
                )),
            },
            BinaryOp::Mul => match (l, r) {
                (Value::Matriz(_), Value::Matriz(_)) => self.matriz_mul(l, r, span),
                (Value::Matriz(_), Value::Numero(k)) | (Value::Numero(k), Value::Matriz(_)) => {
                    self.matriz_scale(l, r, k.to_f64(), span)
                }
                _ => Ok(Value::Numero(
                    self.expect_numero(l, span)?
                        .mul(self.expect_numero(r, span)?, self.numero_aritmetica)
                        .map_err(|m| self.err(m, span))?,
                )),
            },
            BinaryOp::Div => Ok(Value::Numero(
                self.expect_numero(l, span)?
                    .div(self.expect_numero(r, span)?, self.numero_aritmetica)
                    .map_err(|m| self.err(m, span))?,
            )),
            BinaryOp::Rem => Ok(Value::Numero(
                self.expect_numero(l, span)?
                    .rem(self.expect_numero(r, span)?, self.numero_aritmetica)
                    .map_err(|m| self.err(m, span))?,
            )),
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

    pub(crate) fn call_value(
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
                self.check_profundidade(span)?;
                let call_env = Env::child(&closure.env, FrameKind::Function);
                if let FunctionBody::Bytecode(code) = &closure.body {
                    code.bind_parameters(&mut call_env.borrow_mut(), args.iter().cloned());
                } else {
                    for (param, arg) in closure.params.iter().zip(args) {
                        call_env
                            .borrow_mut()
                            .define(param.name.clone(), arg.clone());
                    }
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
                let result = match &closure.body {
                    FunctionBody::Ast(body) => self.eval_stmts(&body.stmts, &call_env),
                    FunctionBody::Bytecode(code) => {
                        self.execute_function(Rc::clone(code), &call_env)
                    }
                };
                // Build invalid-control errors before restoring the caller's source.
                let result = match result {
                    Err(EvalError::Break { span }) => {
                        Err(self.err("pare só pode ser usado num laço", span))
                    }
                    Err(EvalError::Continue { span }) => {
                        Err(self.err("continue só pode ser usado num laço", span))
                    }
                    other => other,
                };
                self.report_runtime_error(&result, &call_env);
                self.file = saved_file;
                self.source = saved_source;
                self.stack.pop();
                Env::liberar_se_lixo(&call_env);
                match result {
                    Ok(v) => Ok(v),
                    Err(EvalError::Return { value, .. }) => Ok(value),
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
                let row: Vec<Value> = rows[i]
                    .iter()
                    .copied()
                    .map(|x| self.number_from_f64(x, span))
                    .collect::<Result<_, _>>()?;
                Ok(Value::lista(row))
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
        if n < 0 {
            return Err(self.err(format!("índice deve ser >= 0, encontrado {n}"), span));
        }
        let i = usize::try_from(n).map_err(|_| {
            self.err(
                format!("índice {n} fora do intervalo (tamanho {len})"),
                span,
            )
        })?;
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
        if s < 0 {
            return Err(self.err("índices de fatia devem ser >= 0", span));
        }
        let e = match end {
            None => len as i64,
            Some(v) => {
                let e = self.expect_int(v, span)?;
                if e < 0 {
                    return Err(self.err("índices de fatia devem ser >= 0", span));
                }
                e.saturating_add(1)
            }
        };
        if len == 0 || s >= len as i64 {
            return Ok((0, 0));
        }
        let e = e.min(len as i64);
        if s > e {
            return Ok((0, 0));
        }
        Ok((s as usize, e as usize))
    }

    fn assign_name(
        &self,
        env: &Rc<RefCell<Env>>,
        name: &str,
        value: Value,
        span: Span,
    ) -> Result<(), EvalError> {
        Env::assign(env, name, value).map_err(|e| self.assignment_error(e, span))
    }

    fn assignment_error(&self, e: AssignError, span: Span) -> EvalError {
        match e {
            AssignError::FunctionBoundary { name } => {
                self.err(format!("função não pode alterar `{name}`"), span)
            }
            AssignError::Builtin { name } => self.err(
                format!("não é possível alterar `{name}` (nome nativo)"),
                span,
            ),
        }
    }

    pub(crate) fn interrupted(&mut self) -> bool {
        self.hook.interrupted()
    }

    /// Called before pushing a function frame: past the limit the call is
    /// a normal runtime error (`se_falhar` catches it), not a stack overflow.
    pub(crate) fn check_profundidade(&self, span: Span) -> Result<(), EvalError> {
        if self.stack.len() >= PROFUNDIDADE_MAXIMA {
            return Err(self.err(
                format!(
                    "recursão profunda demais (mais de {PROFUNDIDADE_MAXIMA} chamadas): \
                     a função chama a si mesma sem parar?"
                ),
                span,
            ));
        }
        Ok(())
    }

    /// Child processes (`importe "processo"`) would escape the folder a
    /// confined program (Aula, fuzz) must stay in, so they are off there.
    pub(crate) fn processos_permitidos(&self) -> bool {
        self.workspace_root.is_none()
    }

    /// Time limit and Parar, polled while waiting on a child process.
    pub(crate) fn check_espera(&mut self, span: Span) -> Result<(), EvalError> {
        self.check_deadline(span)?;
        if self.hook.interrupted() {
            return Err(EvalError::Quit(0));
        }
        Ok(())
    }

    pub(crate) fn check_deadline(&self, span: Span) -> Result<(), EvalError> {
        if let Some(deadline) = self.deadline
            && Instant::now() >= deadline
        {
            return Err(self.err("tempo esgotado", span));
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
        let end = Instant::now()
            .checked_add(want)
            .ok_or_else(|| self.err("durma() duração grande demais", span))?;
        // Poll stop and deadline even when the requested sleep exceeds the limit.
        loop {
            self.check_espera(span)?;
            let now = Instant::now();
            let left = end.saturating_duration_since(now);
            if left.is_zero() {
                break;
            }
            let until_deadline = self
                .deadline
                .map_or(left, |deadline| deadline.saturating_duration_since(now));
            thread::sleep(left.min(until_deadline).min(Duration::from_millis(50)));
        }
        self.check_deadline(span)
    }

    fn pause(&mut self, span: Span, env: &Rc<RefCell<Env>>) -> Result<(), EvalError> {
        super::value::collect_cycles_if_due();
        self.check_deadline(span)?;
        // Borrow the fields directly: cloning the stack here made every
        // statement cost O(call depth).
        match self.hook.before_stmt(&DebugCtx {
            file: &self.file,
            source: &self.source,
            span,
            env,
            stack: &self.stack,
        }) {
            DebugAction::Continue => Ok(()),
            DebugAction::Quit => Err(EvalError::Quit(0)),
            DebugAction::Timeout => Err(self.err("tempo esgotado", span)),
            DebugAction::Interrompido => Err(self.err("interrompido", span)),
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
        let file = self.file.clone();
        EvalError::Runtime(RuntimeError {
            message: message.into(),
            stack: CallFrame::executing(&self.stack, &file, span),
            file,
            span,
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

    pub(crate) fn expect_numero(&self, v: &Value, span: Span) -> Result<Numero, EvalError> {
        match v {
            Value::Numero(n) => Ok(*n),
            _ => Err(self.err(
                format!("esperado numero, encontrado {}", v.type_name()),
                span,
            )),
        }
    }

    pub(crate) fn number_from_f64(&self, n: f64, span: Span) -> Result<Value, EvalError> {
        Numero::try_from(n)
            .map(Value::Numero)
            .map_err(|message| self.err(message, span))
    }

    pub(crate) fn expect_f64(&self, v: &Value, span: Span) -> Result<f64, EvalError> {
        Ok(self.expect_numero(v, span)?.to_f64())
    }

    pub(crate) fn expect_int(&self, v: &Value, span: Span) -> Result<i64, EvalError> {
        let n = self.expect_numero(v, span)?;
        n.to_i64().ok_or_else(|| {
            self.err(
                if n.is_integer() {
                    format!(
                        "número inteiro fora do intervalo de 64 bits: {}",
                        format_numero(n)
                    )
                } else {
                    format!("esperado número inteiro, encontrado {}", format_numero(n))
                },
                span,
            )
        })
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

    pub(crate) fn expect_lista(&self, v: &Value, span: Span) -> Result<ListaRc, EvalError> {
        match v {
            Value::Lista(xs) => Ok(Rc::clone(xs)),
            _ => Err(self.err(
                format!("esperado lista, encontrado {}", v.type_name()),
                span,
            )),
        }
    }
}

pub(crate) fn lexical_normalize(path: &Path) -> PathBuf {
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

impl Vm<'_, '_> {
    pub(crate) fn format_value(&self, v: &Value) -> String {
        v.format_with(self.numero_locale)
    }

    /// Text for `escreva` / `formate` / `+`. A mapa with `:_como_texto`
    /// (a function of one argument) supplies its own text.
    pub(crate) fn como_texto_de(
        &mut self,
        v: &Value,
        span: Span,
        env: &Rc<RefCell<Env>>,
    ) -> Result<String, EvalError> {
        let Value::Mapa(xs) = v else {
            return Ok(self.format_value(v));
        };
        let hook = {
            let key = MapKey::Texto("_como_texto".into());
            xs.borrow()
                .iter()
                .find(|(k, _)| *k == key)
                .map(|(_, v)| v.clone())
        };
        let Some(hook) = hook else {
            return Ok(self.format_value(v));
        };
        if !matches!(hook, Value::Funcao(_) | Value::Builtin(_)) {
            return Err(self.err(
                format!(
                    ":_como_texto deve ser uma função, encontrado {}",
                    hook.type_name()
                ),
                span,
            ));
        }
        let ptr = Rc::as_ptr(xs) as usize;
        if !self.como_texto_guard.insert(ptr) {
            return Err(self.err(":_como_texto não pode formatar o mesmo mapa de novo", span));
        }
        let result = self.call_value(&hook, std::slice::from_ref(v), span, "como_texto", env);
        self.como_texto_guard.remove(&ptr);
        match result? {
            Value::Texto(s) => Ok(s),
            other => Err(self.err(
                format!(
                    ":_como_texto deve devolver texto, encontrado {}",
                    other.type_name()
                ),
                span,
            )),
        }
    }
}

#[cfg(test)]
mod native_limits_tests;
#[cfg(test)]
mod tests;
