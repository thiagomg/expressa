use std::io::{self, BufRead, IsTerminal, Write};
use std::path::PathBuf;
use std::process;
use std::rc::Rc;

use rustyline::DefaultEditor;
use rustyline::error::ReadlineError;

use crate::parser::{needs_more_input, parse};

use super::debug::NoopHook;
use super::env::{Env, FrameKind};
use super::error::{EvalError, RuntimeError};
use super::eval::{LineInput, Vm};
use super::value::Value;

pub struct ReplSession<'a> {
    vm: Vm<'a>,
    env: Rc<std::cell::RefCell<Env>>,
    exit_code: Option<i32>,
}

impl<'a> ReplSession<'a> {
    pub fn new(
        input: &'a mut dyn LineInput,
        out: &'a mut dyn Write,
        err: &'a mut dyn Write,
    ) -> Self {
        let mut vm = Vm::new(
            "<repl>",
            "",
            Box::new(NoopHook),
            input,
            out,
            err,
            None,
            None,
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

    pub fn eval(&mut self, source: &str) -> Result<Value, RuntimeError> {
        self.vm.set_source("<repl>", source);
        let program = parse(source).map_err(|e| RuntimeError {
            message: format!("sintaxe: {}", e.message),
            file: "<repl>".into(),
            span: e.span,
            stack: vec![],
        })?;
        match self.vm.eval_items_value(&program.items, &self.env) {
            Ok(v) => Ok(v),
            Err(EvalError::Quit(code)) => {
                self.exit_code = Some(code);
                Ok(Value::Nada)
            }
            Err(EvalError::Runtime(e)) => Err(e),
        }
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

/// Interactive prompt. `sair` or Ctrl+D leaves.
/// On a terminal, ↑/↓ walk the command history.
pub fn run_repl() -> io::Result<()> {
    println!("Expressa. Digite 'ajuda' ou 'sair'. Setas ↑ ↓ recorrem comandos.");
    if io::stdin().is_terminal() {
        if let Ok(mut rl) = DefaultEditor::new() {
            return run_repl_editor(&mut rl);
        }
    }
    run_repl_plain()
}

fn history_path() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    Some(PathBuf::from(home).join(".expressa_history"))
}

fn run_repl_editor(rl: &mut DefaultEditor) -> io::Result<()> {
    if let Some(path) = history_path() {
        let _ = rl.load_history(&path);
    }
    let mut stdout = io::stdout();
    let mut stderr = io::stderr();
    let mut input = ConsoleInput;
    let mut session = ReplSession::new(&mut input, &mut stdout, &mut stderr);
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

fn run_repl_plain() -> io::Result<()> {
    let mut stdin_lines = io::stdin().lock();
    let mut stdout = io::stdout();
    let mut stderr = io::stderr();
    let mut input = ConsoleInput;
    let mut session = ReplSession::new(&mut input, &mut stdout, &mut stderr);
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
            Some(ReplCommand::Ajuda(topic)) => {
                print_help(topic);
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

enum ReplCommand<'a> {
    Sair,
    Ajuda(Option<&'a str>),
    Vazio,
}

fn parse_repl_command(line: &str) -> Option<ReplCommand<'_>> {
    if line.is_empty() {
        return Some(ReplCommand::Vazio);
    }
    let line = line.strip_prefix('.').unwrap_or(line);
    let mut parts = line.split_whitespace();
    let cmd = parts.next()?;
    let rest = parts.next();
    match normalize_topic(cmd).as_str() {
        "sair" | "exit" | "quit" => Some(ReplCommand::Sair),
        "ajuda" | "help" => Some(ReplCommand::Ajuda(rest)),
        _ => None,
    }
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

fn print_help(topic: Option<&str>) {
    match topic {
        None => print_help_geral(),
        Some(t) => {
            let key = normalize_topic(t);
            match key.as_str() {
                "funcoes" | "funcao" | "nativas" | "builtins" => print_help_funcoes(),
                "linguagem" | "sintaxe" => print_help_linguagem(),
                "argumentos" => println!(
                    "  argumentos              lista dos valores após o .lep\n  \
                     argumentos[1]           primeiro (índices começam em 1)\n  \
                     exemplo: expressa grep.lep Thiago"
                ),
                name => match super::builtins::lookup_builtin_doc(name) {
                    Some(doc) => print_help_builtin(doc),
                    None => {
                        println!(
                            "Não conheço o tópico `{t}`.\n  \
                             Tente: ajuda, ajuda funcoes, ajuda linguagem, ajuda escreva"
                        );
                    }
                },
            }
        }
    }
}

fn print_help_geral() {
    println!(
        "  2 + 2                 avalia e mostra o resultado\n  \
         x = 10                 guarda o valor (não imprime)\n  \
         x                      mostra 10\n\n  \
         ajuda                  esta mensagem\n  \
         ajuda funcoes          lista as funções nativas\n  \
         ajuda linguagem        se, para, funcao, se_falhar…\n  \
         ajuda escreva          detalhe de uma função\n  \
         formato(\"pt\") / en     padrão de números em texto\n  \
         sair                   encerra o REPL (também Ctrl+D)\n  \
         sair() / sair(1)       nativa: encerra o processo\n  \
         ↑ ↓                    comandos anteriores"
    );
}

fn print_help_funcoes() {
    println!("Funções nativas:\n");
    for doc in super::builtins::BUILTIN_DOCS {
        println!("  {:<22} {}", doc.sig, doc.summary);
    }
    println!("\n  ajuda <nome>          detalhe, ex.: ajuda raiz");
}

fn print_help_builtin(doc: &super::builtins::BuiltinDoc) {
    println!(
        "  {}\n  {}\n  exemplo: {}",
        doc.sig, doc.summary, doc.example
    );
}

fn print_help_linguagem() {
    println!(
        "  se cond inicio … fim  [ou se …]  [senao …]\n  \
         se cond {{ … }} senao {{ … }}   '{{' e '}}' valem como inicio/fim\n  \
         para i de 1 ate 10 {{ … }}\n  \
         para x em lista {{ … }}\n  \
         enquanto cond {{ … }}\n  \
         repita n vezes {{ … }}\n  \
         argumentos[1]           primeiro argumento após o .lep\n  \
         funcao(a, b) {{ … }}            último valor é o resultado\n  \
         expr se_falhar outro            captura erro (divisão, arquivo, índice)\n  \
         mapa {{ \"k\" -> v }}           vazio: mapa {{}}\n  \
         matriz {{ [1, 2], [3, 4] }}     A[1, 2]  A+B  k*A  A*B\n  \
         lista[1]  texto[1..3]           índices começam em 1\n  \
         pessoa:nome                     chave de mapa (pessoa[\"nome\"])\n  \
         mat::soma(1, 2)                 nome em um módulo\n  \
         xs.tamanho()                    igual a tamanho(xs)\n  \
         e  ou  nao  contem\n  \
         #!/usr/bin/env expressa primeira linha (arquivo executável)"
    );
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
        assert_eq!(String::from_utf8(out).unwrap(), "oi\n");
    }

    #[test]
    fn repl_function_and_call() {
        let vals =
            eval_snippets(&["dobra = funcao(n)\ninicio\n    n * 2\nfim\n", "dobra(21)"]).unwrap();
        assert_eq!(vals.last().unwrap(), "42");
    }

    #[test]
    fn ajuda_parses_topics_with_accents() {
        assert!(matches!(
            parse_repl_command("ajuda funções"),
            Some(ReplCommand::Ajuda(Some("funções")))
        ));
        assert!(matches!(
            parse_repl_command("ajuda funcoes"),
            Some(ReplCommand::Ajuda(Some("funcoes")))
        ));
        assert!(matches!(
            parse_repl_command("ajuda raiz"),
            Some(ReplCommand::Ajuda(Some("raiz")))
        ));
        assert_eq!(normalize_topic("funções"), "funcoes");
        assert!(super::super::builtins::lookup_builtin_doc("escreva").is_some());
    }
}
