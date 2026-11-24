use std::io::{self, BufRead, IsTerminal, Write};
use std::path::PathBuf;
use std::process;
use std::rc::Rc;

use rustyline::DefaultEditor;
use rustyline::error::ReadlineError;

use crate::parser::{needs_more_input, parse};

use super::debug::{DebugHook, NoopHook};
use super::env::{Env, FrameKind};
use super::error::{EvalError, RuntimeError};
use super::eval::{ExecutionOptions, LineInput, Vm};
use super::leia::CLEAR_SCREEN;
use super::value::Value;

pub struct ReplSession<'a> {
    vm: Vm<'a, 'static>,
    env: Rc<std::cell::RefCell<Env>>,
    exit_code: Option<i32>,
}

/// The session's frame lives until the session ends, so its functions are
/// freed here rather than when a line finishes.
impl Drop for ReplSession<'_> {
    fn drop(&mut self) {
        self.env.borrow_mut().vars.clear();
        self.vm.liberar_frames();
    }
}

impl<'a> ReplSession<'a> {
    pub fn new(
        input: &'a mut dyn LineInput,
        out: &'a mut dyn Write,
        err: &'a mut dyn Write,
    ) -> Self {
        Self::with_hook(input, out, err, Box::new(NoopHook))
    }

    fn with_hook(
        input: &'a mut dyn LineInput,
        out: &'a mut dyn Write,
        err: &'a mut dyn Write,
        hook: Box<dyn DebugHook>,
    ) -> Self {
        let mut vm = Vm::new(
            "<repl>",
            "",
            hook,
            input,
            out,
            err,
            None,
            None,
            None,
            None,
            true,
            None,
            &[],
        );
        let env = Env::child(&vm.builtins_env(), FrameKind::Module);
        vm.enter_repl();
        Self {
            vm,
            env,
            exit_code: None,
        }
    }

    fn aplicar_opcoes(&mut self, options: ExecutionOptions) {
        self.vm.backend = options.backend;
        self.vm.numero_locale = options.numero_locale;
        self.vm.numero_aritmetica = options.numero_aritmetica;
        self.vm.max_matrix_cells = options.max_matrix_cells;
        if options.workspace_root.is_some() {
            self.vm.workspace_root = options.workspace_root;
        }
    }

    pub fn eval(&mut self, source: &str) -> Result<Value, RuntimeError> {
        self.vm.set_source("<repl>", source);
        let program = parse(source).map_err(|e| RuntimeError {
            message: format!("sintaxe: {}", e.message),
            file: "<repl>".into(),
            span: e.span,
            stack: vec![],
        })?;
        let result = self.vm.execute_program_value(program, &self.env);
        // The result is held here, so escaped objects remain external roots.
        super::value::collect_cycles();
        match result {
            Ok(v) => Ok(v),
            Err(EvalError::Quit(code)) => {
                self.exit_code = Some(code);
                Ok(Value::Nada)
            }
            Err(EvalError::Return { span, .. }) => Err(RuntimeError {
                message: "retorne só pode ser usado numa função".into(),
                file: "<repl>".into(),
                span,
                stack: vec![],
            }),
            Err(EvalError::Break { span }) => Err(RuntimeError {
                message: "pare só pode ser usado num laço".into(),
                file: "<repl>".into(),
                span,
                stack: vec![],
            }),
            Err(EvalError::Continue { span }) => Err(RuntimeError {
                message: "continue só pode ser usado num laço".into(),
                file: "<repl>".into(),
                span,
                stack: vec![],
            }),
            Err(EvalError::Runtime(e)) => Err(e),
        }
    }
}

mod canal;
pub use canal::{
    CanalRepl, ConteudoRepl, EventoRepl, FimRepl, FluxoRepl, IdAvaliacao, IdSessao,
    REPL_MAX_CODIGO, TextoRepl,
};

struct ConsoleInput;

impl LineInput for ConsoleInput {
    fn read_bytes(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        io::Read::read(&mut io::stdin().lock(), buf)
    }
    fn read_line(&mut self, buf: &mut String) -> io::Result<usize> {
        io::stdin().read_line(buf)
    }

    fn is_terminal(&self) -> bool {
        io::stdin().is_terminal()
    }
}

/// Interactive prompt. `sair` or Ctrl+D leaves.
/// On a terminal, ↑/↓ walk the command history.
pub fn run_repl() -> io::Result<()> {
    run_repl_com(ExecutionOptions::default())
}

pub fn run_repl_com(options: ExecutionOptions) -> io::Result<()> {
    println!(
        "Expressa. Use modulos(), funcoes() ou ajuda(\"escreva\"). Digite 'sair' para encerrar."
    );
    if io::stdin().is_terminal()
        && let Ok(mut rl) = DefaultEditor::new()
    {
        return run_repl_editor(&mut rl, options);
    }
    run_repl_plain(options)
}

fn history_path() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    Some(PathBuf::from(home).join(".expressa_history"))
}

fn run_repl_editor(rl: &mut DefaultEditor, options: ExecutionOptions) -> io::Result<()> {
    if let Some(path) = history_path() {
        let _ = rl.load_history(&path);
    }
    let mut stdout = io::stdout();
    let mut stderr = io::stderr();
    let mut input = ConsoleInput;
    let mut session = ReplSession::new(&mut input, &mut stdout, &mut stderr);
    session.aplicar_opcoes(options);
    let mut buf = String::new();
    loop {
        let prompt = if buf.is_empty() {
            "expressa> "
        } else {
            "       … "
        };
        match rl.readline(prompt) {
            Ok(line) => match apply_line(&mut session, &mut buf, &line) {
                LineResult::Quit(code) => {
                    if let Some(path) = history_path() {
                        let _ = rl.save_history(&path);
                    }
                    if code != 0 {
                        process::exit(code);
                    }
                    break;
                }
                LineResult::Continue => {}
                LineResult::Ran(entry) => {
                    if !entry.is_empty() {
                        let _ = rl.add_history_entry(entry);
                    }
                }
            },
            Err(ReadlineError::Interrupted) => {
                buf.clear();
                println!("^C");
            }
            Err(ReadlineError::Eof) => {
                println!();
                break;
            }
            Err(e) => return Err(io::Error::other(e)),
        }
    }
    if let Some(path) = history_path() {
        let _ = rl.save_history(&path);
    }
    Ok(())
}

fn run_repl_plain(options: ExecutionOptions) -> io::Result<()> {
    let mut stdin_lines = io::stdin().lock();
    let mut stdout = io::stdout();
    let mut stderr = io::stderr();
    let mut input = ConsoleInput;
    let mut session = ReplSession::new(&mut input, &mut stdout, &mut stderr);
    session.aplicar_opcoes(options);
    let mut buf = String::new();
    loop {
        let prompt = if buf.is_empty() {
            "expressa> "
        } else {
            "       … "
        };
        print!("{prompt}");
        io::stdout().flush()?;
        let mut line = String::new();
        let n = BufRead::read_line(&mut stdin_lines, &mut line)?;
        if n == 0 {
            println!();
            break;
        }
        if let LineResult::Quit(code) =
            apply_line(&mut session, &mut buf, line.trim_end_matches(['\n', '\r']))
        {
            if code != 0 {
                process::exit(code);
            }
            break;
        }
    }
    Ok(())
}

enum LineResult {
    Continue,
    Quit(i32),
    Ran(String),
}

fn apply_line(session: &mut ReplSession<'_>, buf: &mut String, line: &str) -> LineResult {
    let trimmed = line.trim();
    if buf.is_empty() {
        match parse_repl_command(trimmed) {
            Some(ReplCommand::Sair) => return LineResult::Quit(0),
            Some(ReplCommand::Cls) => {
                print!("{CLEAR_SCREEN}");
                let _ = io::stdout().flush();
                return LineResult::Ran(trimmed.to_string());
            }
            Some(ReplCommand::Vazio) => return LineResult::Continue,
            None => {}
        }
    }
    buf.push_str(line);
    buf.push('\n');
    if needs_more_input(buf) {
        return LineResult::Continue;
    }
    let source = std::mem::take(buf);
    match session.eval(&source) {
        Ok(Value::Nada) => {}
        Ok(v) => println!("{}", v.repl_format(session.vm.numero_locale)),
        Err(e) => eprintln!("{e}"),
    }
    if let Some(code) = session.exit_code {
        return LineResult::Quit(code);
    }
    LineResult::Ran(source.trim().to_string())
}

enum ReplCommand {
    Sair,
    Cls,
    Vazio,
}

fn parse_repl_command(line: &str) -> Option<ReplCommand> {
    if line.is_empty() {
        return Some(ReplCommand::Vazio);
    }
    let line = line.strip_prefix('.').unwrap_or(line);
    let mut parts = line.split_whitespace();
    let cmd = parts.next()?;
    // `cls = 3` and `sair = 3` are Expressa assignments.
    if continua_como_codigo(line, cmd) {
        return None;
    }
    let rest = parts.next();
    match normalize_topic(cmd).as_str() {
        "sair" | "exit" | "quit" => Some(ReplCommand::Sair),
        "cls" | "limpe_tela" | "clear" if rest.is_none() => Some(ReplCommand::Cls),
        _ => None,
    }
}

/// The token after the command word starts an assignment or a postfix.
fn continua_como_codigo(line: &str, cmd: &str) -> bool {
    let Some(rest) = line.trim_start().get(cmd.len()..) else {
        return false;
    };
    let rest = rest.trim_start();
    let b = rest.as_bytes();
    if b.is_empty() {
        return false;
    }
    matches!(b[0], b'=' | b'(' | b'.' | b':' | b'[')
        || rest.starts_with("+=")
        || rest.starts_with("-=")
}

fn normalize_topic(s: &str) -> String {
    s.trim()
        .chars()
        .map(|c| match c {
            'á' | 'à' | 'ã' | 'â' | 'Á' | 'À' | 'Ã' | 'Â' => 'a',
            'é' | 'ê' | 'É' | 'Ê' => 'e',
            'í' | 'Í' => 'i',
            'ó' | 'ô' | 'õ' | 'Ó' | 'Ô' | 'Õ' => 'o',
            'ú' | 'Ú' => 'u',
            'ç' | 'Ç' => 'c',
            _ => c.to_ascii_lowercase(),
        })
        .collect()
}

#[cfg(test)]
fn eval_snippets(snippets: &[&str]) -> Result<Vec<String>, RuntimeError> {
    let mut input = io::Cursor::new("");
    let mut out = Vec::new();
    let mut err = Vec::new();
    let mut session = ReplSession::new(&mut input, &mut out, &mut err);
    let mut values = Vec::new();
    for src in snippets {
        let v = session.eval(src)?;
        values.push(v.repl_format(session.vm.numero_locale));
    }
    Ok(values)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repl_keeps_bindings_and_prints_exprs() {
        let vals = eval_snippets(&["x = 10", "x + 1", r#"maiuscula("oi")"#]).unwrap();
        assert_eq!(vals, ["nada", "11", "\"OI\""]);
    }

    #[test]
    fn repl_argumentos_is_empty() {
        let vals = eval_snippets(&["argumentos"]).unwrap();
        assert_eq!(vals, ["[]"]);
    }

    #[test]
    fn repl_escreva_is_nada() {
        let mut input = io::Cursor::new("");
        let mut out = Vec::new();
        let mut err = Vec::new();
        let mut session = ReplSession::new(&mut input, &mut out, &mut err);
        let v = session.eval(r#"escreva("oi")"#).unwrap();
        assert!(matches!(v, Value::Nada));
        drop(session);
        assert_eq!(String::from_utf8(out).unwrap(), "oi\n");
    }

    #[test]
    fn repl_function_and_call() {
        let vals =
            eval_snippets(&["dobra = funcao(n)\ninicio\n    n * 2\nfim\n", "dobra(21)"]).unwrap();
        assert_eq!(vals.last().unwrap(), "42");
    }

    #[test]
    fn removed_commands_are_ordinary_code() {
        for line in [
            "ls",
            "ls matriz",
            "ajuda",
            "help",
            "ajuda funções",
            "catalogo",
            "catálogo",
        ] {
            assert!(parse_repl_command(line).is_none(), "{line}");
        }
        assert!(matches!(parse_repl_command("cls"), Some(ReplCommand::Cls)));
        assert!(matches!(
            parse_repl_command("sair"),
            Some(ReplCommand::Sair)
        ));
        assert!(parse_repl_command("limpe_tela()").is_none());
        assert!(parse_repl_command("sair = 3").is_none());
    }

    #[test]
    fn discovery_keeps_imported_files_and_user_docs_between_repl_lines() {
        let path = std::env::temp_dir().join(format!(
            "expressa-repl-discovery-{}.lep",
            std::process::id()
        ));
        std::fs::write(
            &path,
            "/// Documento do arquivo.\ndobra = funcao(n) { n * 2 }\n",
        )
        .unwrap();
        for backend in [
            super::super::ExecutionBackend::Ast,
            super::super::ExecutionBackend::Bytecode,
        ] {
            if !backend.is_available() {
                continue;
            }
            let mut input = io::Cursor::new("");
            let (mut out, mut err) = (Vec::new(), Vec::new());
            let mut session = ReplSession::new(&mut input, &mut out, &mut err);
            session.vm.backend = backend;
            session
                .eval(&format!("m = importe {:?}", path.to_string_lossy()))
                .unwrap();
            session
                .eval(&format!("outro = importe {:?}", path.to_string_lossy()))
                .unwrap();
            session
                .eval("/// Documento da sessão.\nsoma = funcao(a, b) { a + b }")
                .unwrap();
            session
                .eval(
                    r#"
                count = 0
                para m em modulos(:dados) {
                    se m:funcoes == funcoes(outro, :dados) { count += 1 }
                }
                garanta(count == 1)
                garanta(funcoes(m, :dados)[0]:doc == "Documento do arquivo.")
                para f em funcoes("", :dados) {
                    se f:nome == "soma" { garanta(f:doc == "Documento da sessão.") }
                }
            "#,
                )
                .unwrap();
            session.eval("funcoes(m)").unwrap();
            drop(session);
            assert_eq!(
                String::from_utf8(out).unwrap(),
                "dobra(n)\n    Documento do arquivo.\n"
            );
            assert!(err.is_empty());
        }
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn repl_avisa_cada_erro_desprotegido() {
        struct Conta(std::rc::Rc<std::cell::Cell<u32>>);
        impl DebugHook for Conta {
            fn before_stmt(
                &mut self,
                _: &super::super::debug::DebugCtx<'_>,
            ) -> super::super::debug::DebugAction {
                super::super::debug::DebugAction::Continue
            }
            fn on_error(&mut self, _: &super::super::debug::DebugCtx<'_>, _: &str) {
                self.0.set(self.0.get() + 1);
            }
        }
        let n = std::rc::Rc::new(std::cell::Cell::new(0));
        let mut input = io::Cursor::new("");
        let mut out = Vec::new();
        let mut err = Vec::new();
        let mut session = ReplSession::with_hook(
            &mut input,
            &mut out,
            &mut err,
            Box::new(Conta(std::rc::Rc::clone(&n))),
        );
        assert!(session.eval("1 / 0").is_err());
        assert!(session.eval("2 / 0").is_err());
        assert_eq!(n.get(), 2);
    }
}
