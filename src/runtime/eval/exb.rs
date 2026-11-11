//! Execution entry points for already verified modules. No source parsing here.
use super::bytecode::format::{self, CompiledProgram};
use super::*;

fn load_error(error: format::ExbError, file: &str) -> RuntimeError {
    RuntimeError {
        message: error.to_string(),
        file: file.into(),
        span: Span::default(),
        stack: vec![],
    }
}

pub fn run_exb_with_options(
    bytes: &[u8],
    execution_file: &str,
    stdin: &str,
    options: ExecutionOptions,
) -> Result<ExecutionOutput, RuntimeError> {
    let program = format::load_exb(bytes).map_err(|e| load_error(e, execution_file))?;
    run_compiled_with_options(&program, execution_file, stdin, options)
}
pub fn run_compiled_with_options(
    program: &CompiledProgram,
    execution_file: &str,
    stdin: &str,
    options: ExecutionOptions,
) -> Result<ExecutionOutput, RuntimeError> {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let mut input = io::Cursor::new(stdin);
    let code = run(
        program,
        execution_file,
        Box::new(NoopHook),
        &mut input,
        &mut out,
        &mut err,
        options,
        None,
        false,
        &[],
    )?;
    Ok(ExecutionOutput {
        stdout: String::from_utf8_lossy(&out).into_owned(),
        stderr: String::from_utf8_lossy(&err).into_owned(),
        exit_code: code,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExbRunMode {
    Run,
    Debug,
    Marker,
}

pub(super) fn is_exb_path(path: &Path) -> bool {
    path.extension()
        .and_then(|s| s.to_str())
        .is_some_and(|s| s.eq_ignore_ascii_case("exb"))
}

pub fn run_exb_file_args(
    path: &str,
    args: &[String],
    mode: ExbRunMode,
) -> Result<i32, RuntimeError> {
    let program = format::read_exb(Path::new(path)).map_err(|e| load_error(e, path))?;
    let hook: Box<dyn DebugHook> = if mode == ExbRunMode::Debug {
        Box::new(super::super::debug::CliDebugger::new())
    } else {
        Box::new(NoopHook)
    };
    let mut out = io::stdout();
    let mut err = io::stderr();
    if mode == ExbRunMode::Marker {
        let mut host = super::super::leia::MarkerLeiaHost {
            stdin: io::stdin().lock(),
        };
        run(
            &program,
            path,
            hook,
            &mut io::Cursor::new(""),
            &mut out,
            &mut err,
            ExecutionOptions::default(),
            Some(&mut host),
            true,
            args,
        )
    } else {
        run(
            &program,
            path,
            hook,
            &mut ConsoleInput,
            &mut out,
            &mut err,
            ExecutionOptions::default(),
            None,
            true,
            args,
        )
    }
}

fn run<'a>(
    program: &CompiledProgram,
    file: &str,
    hook: Box<dyn DebugHook>,
    input: &'a mut dyn LineInput,
    out: &'a mut dyn Write,
    err: &'a mut dyn Write,
    options: ExecutionOptions,
    leia: Option<&'a mut dyn LeiaHost>,
    graphics: bool,
    args: &[String],
) -> Result<i32, RuntimeError> {
    if options.backend != ExecutionBackend::Bytecode {
        return Err(RuntimeError {
            message: "arquivos .exb exigem o backend bytecode".into(),
            file: file.into(),
            span: Span::default(),
            stack: vec![],
        });
    }
    if options
        .time_limit
        .is_some_and(|d| Instant::now().checked_add(d).is_none())
    {
        return Err(RuntimeError {
            message: "limite de tempo fora do intervalo".into(),
            file: file.into(),
            span: Span::default(),
            stack: vec![],
        });
    }
    let mut vm = Vm::new(
        &program.source_name,
        "",
        hook,
        input,
        out,
        err,
        options.workspace_root,
        options.time_limit,
        leia,
        None,
        graphics,
        None,
        args,
    );
    vm.source = Rc::clone(&program.source);
    vm.base_dir = base_dir_of(file);
    vm.backend = ExecutionBackend::Bytecode;
    vm.max_matrix_cells = options.max_matrix_cells;
    let env = Env::child(&vm.builtins, FrameKind::Module);
    let result = vm.eval_loaded_module(program, &env);
    drop(env);
    vm.liberar_frames();
    match result {
        Ok(()) => Ok(0),
        Err(EvalError::Quit(code)) => Ok(code),
        Err(EvalError::Runtime(e)) => Err(e),
        Err(EvalError::Return { span, .. }) => Err(RuntimeError {
            message: "retorne só pode ser usado numa função".into(),
            file: program.source_name.clone(),
            span,
            stack: vec![],
        }),
        Err(EvalError::Break { span }) => Err(RuntimeError {
            message: "pare só pode ser usado num laço".into(),
            file: program.source_name.clone(),
            span,
            stack: vec![],
        }),
        Err(EvalError::Continue { span }) => Err(RuntimeError {
            message: "continue só pode ser usado num laço".into(),
            file: program.source_name.clone(),
            span,
            stack: vec![],
        }),
    }
}
impl Vm<'_, '_> {
    pub(super) fn eval_loaded_module(
        &mut self,
        program: &CompiledProgram,
        env: &Rc<RefCell<Env>>,
    ) -> Result<(), EvalError> {
        self.stack.push(CallFrame {
            name: "<modulo>".into(),
            file: self.file.clone(),
            span: program.span,
        });
        let result = self
            .execute_loaded_module(Rc::clone(&program.code), env)
            .map(|_| ());
        self.stack.pop();
        self.retorne_so_em_funcao(result)
    }
}
