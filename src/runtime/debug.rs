use std::collections::HashSet;
use std::io::{self, BufRead, Write};
use std::rc::Rc;

use crate::lexer::Span;

use super::env::{Env, FrameKind};
use super::error::CallFrame;
use super::value::Value;

pub struct DebugCtx<'a> {
    pub file: &'a str,
    pub source: &'a str,
    pub span: Span,
    pub env: &'a Rc<RefCellEnv>,
    pub stack: &'a [CallFrame],
}

/// Alias so DebugCtx can mention the env without importing RefCell in every use.
pub type RefCellEnv = std::cell::RefCell<Env>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DebugAction {
    Continue,
    Quit,
    /// Stop the program as a runtime error (used for time limits).
    Timeout,
}

pub trait DebugHook {
    fn before_stmt(&mut self, ctx: &DebugCtx<'_>) -> DebugAction;
}

pub struct NoopHook;

impl DebugHook for NoopHook {
    fn before_stmt(&mut self, _ctx: &DebugCtx<'_>) -> DebugAction {
        DebugAction::Continue
    }
}

#[derive(Debug, Clone)]
enum StepMode {
    /// Run until a breakpoint (or the first statement, if `pause_on_start`).
    Continue,
    /// Pause on the next statement.
    StepIn,
    /// Pause on the next statement outside `span` at this call depth.
    StepOver {
        depth: usize,
        file: String,
        span: Span,
    },
    /// Pause when the call stack is at or below `depth`.
    StepOut { depth: usize },
}

pub struct CliDebugger {
    breakpoints: HashSet<(String, u32)>,
    mode: StepMode,
    last_line: String,
    pause_on_start: bool,
    out: Box<dyn Write>,
}

impl CliDebugger {
    pub fn new() -> Self {
        Self::with_out(Box::new(io::stderr()))
    }

    pub fn with_out(out: Box<dyn Write>) -> Self {
        Self {
            breakpoints: HashSet::new(),
            mode: StepMode::StepIn,
            last_line: String::new(),
            pause_on_start: true,
            out,
        }
    }

    fn should_pause(&self, ctx: &DebugCtx<'_>) -> bool {
        if self
            .breakpoints
            .contains(&(ctx.file.to_string(), ctx.span.line))
        {
            return true;
        }
        match &self.mode {
            StepMode::Continue => false,
            StepMode::StepIn => true,
            StepMode::StepOver { depth, file, span } => {
                if ctx.stack.len() < *depth {
                    true
                } else if ctx.stack.len() == *depth {
                    ctx.file != file.as_str() || !strictly_inside(*span, ctx.span)
                } else {
                    false
                }
            }
            StepMode::StepOut { depth } => ctx.stack.len() <= *depth,
        }
    }

    fn print_location(&mut self, ctx: &DebugCtx<'_>) {
        let _ = writeln!(self.out, "quebrado em {}", ctx.span.location_in(ctx.file));
        let line = source_line(ctx.source, ctx.span.line);
        let _ = writeln!(self.out, "{:>4} | {}", ctx.span.line, line.trim_end());
    }

    fn print_vars(&mut self, ctx: &DebugCtx<'_>) {
        let mut current = Some(Rc::clone(ctx.env));
        let mut any = false;
        while let Some(cell) = current {
            let env = cell.borrow();
            if env.kind != FrameKind::Builtins {
                let bindings: Vec<_> = env
                    .bindings_sorted()
                    .into_iter()
                    .filter(|(_, v)| !matches!(v, Value::Builtin(_)))
                    .collect();
                if !bindings.is_empty() {
                    any = true;
                    let label = match env.kind {
                        FrameKind::Module => "módulo",
                        FrameKind::Block => "bloco",
                        FrameKind::Function => "função",
                        FrameKind::Builtins => "",
                    };
                    let _ = writeln!(self.out, "[{label}]");
                    for (k, v) in bindings {
                        let _ = writeln!(self.out, "  {k} = {v}");
                    }
                }
            }
            let parent = env.parent.clone();
            drop(env);
            current = parent;
        }
        if !any {
            let _ = writeln!(self.out, "(nenhuma variável)");
        }
    }

    fn print_stack(&mut self, ctx: &DebugCtx<'_>) {
        if ctx.stack.is_empty() {
            let _ = writeln!(self.out, "(pilha vazia)");
            return;
        }
        for (i, frame) in ctx.stack.iter().rev().enumerate() {
            let _ = writeln!(
                self.out,
                "  {i}: {} ({})",
                frame.name,
                frame.span.location_in(&frame.file)
            );
        }
    }

    fn print_help(&mut self) {
        let _ = writeln!(
            self.out,
            "comandos:\n  \
             continuar, c          continua até o próximo ponto de parada\n  \
             proximo, n            executa o próximo comando (sem entrar em chamadas)\n  \
             entrar, s             entra na próxima chamada / bloco\n  \
             sair, o               sai da função atual\n  \
             vars, v               mostra variáveis visíveis\n  \
             pilha, k              mostra a pilha de chamadas\n  \
             ponto [arquivo:]N     coloca (ou lista) pontos de parada\n  \
             remover N             remove um ponto de parada na linha N\n  \
             ajuda, h              mostra esta ajuda\n  \
             terminar, q           encerra o programa"
        );
    }

    fn handle_command(&mut self, line: &str, ctx: &DebugCtx<'_>) -> Option<DebugAction> {
        let mut parts = line.split_whitespace();
        let cmd = parts.next().unwrap_or("");
        match cmd {
            "continuar" | "c" | "continue" => {
                self.mode = StepMode::Continue;
                Some(DebugAction::Continue)
            }
            "proximo" | "próximo" | "n" | "next" => {
                self.mode = StepMode::StepOver {
                    depth: ctx.stack.len(),
                    file: ctx.file.to_string(),
                    span: ctx.span,
                };
                Some(DebugAction::Continue)
            }
            "entrar" | "s" | "step" => {
                self.mode = StepMode::StepIn;
                Some(DebugAction::Continue)
            }
            "sair" | "o" | "out" | "fin" => {
                let depth = ctx.stack.len().saturating_sub(1);
                self.mode = StepMode::StepOut { depth };
                Some(DebugAction::Continue)
            }
            "vars" | "v" | "locais" => {
                self.print_vars(ctx);
                None
            }
            "pilha" | "k" | "stack" | "bt" => {
                self.print_stack(ctx);
                None
            }
            "ponto" | "break" | "b" => {
                match parts.next() {
                    None => {
                        if self.breakpoints.is_empty() {
                            let _ = writeln!(self.out, "(nenhum ponto de parada)");
                        } else {
                            let mut bps: Vec<_> = self.breakpoints.iter().collect();
                            bps.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
                            for (file, line) in bps {
                                let _ = writeln!(self.out, "  {file}:{line}");
                            }
                        }
                    }
                    Some(spec) => match parse_breakpoint(spec, ctx.file) {
                        Some((file, line)) => {
                            self.breakpoints.insert((file.clone(), line));
                            let _ = writeln!(self.out, "ponto de parada em {file}:{line}");
                        }
                        None => {
                            let _ = writeln!(self.out, "uso: ponto [arquivo:]linha");
                        }
                    },
                }
                None
            }
            "remover" | "delete" | "d" => match parts.next().and_then(|s| s.parse::<u32>().ok()) {
                Some(line) => {
                    let file = ctx.file.to_string();
                    if self.breakpoints.remove(&(file.clone(), line)) {
                        let _ = writeln!(self.out, "removido {file}:{line}");
                    } else {
                        let _ = writeln!(self.out, "nenhum ponto de parada em {file}:{line}");
                    }
                    None
                }
                None => {
                    let _ = writeln!(self.out, "uso: remover linha");
                    None
                }
            },
            "ajuda" | "h" | "help" | "?" => {
                self.print_help();
                None
            }
            "terminar" | "q" | "quit" => Some(DebugAction::Quit),
            "" => None,
            other => {
                let _ = writeln!(
                    self.out,
                    "comando desconhecido: {other} (ajuda para a lista)"
                );
                None
            }
        }
    }
}

impl Default for CliDebugger {
    fn default() -> Self {
        Self::new()
    }
}

impl DebugHook for CliDebugger {
    fn before_stmt(&mut self, ctx: &DebugCtx<'_>) -> DebugAction {
        if self.pause_on_start {
            self.pause_on_start = false;
        } else if !self.should_pause(ctx) {
            return DebugAction::Continue;
        }

        self.print_location(ctx);
        let stdin = io::stdin();
        let mut handle = stdin.lock();
        loop {
            let _ = write!(self.out, "(debug) ");
            let _ = self.out.flush();
            let mut line = String::new();
            match handle.read_line(&mut line) {
                Ok(0) => return DebugAction::Quit,
                Ok(_) => {}
                Err(_) => return DebugAction::Quit,
            }
            let trimmed = line.trim();
            let command = if trimmed.is_empty() {
                self.last_line.clone()
            } else {
                self.last_line = trimmed.to_string();
                self.last_line.clone()
            };
            if let Some(action) = self.handle_command(&command, ctx) {
                return action;
            }
        }
    }
}

fn strictly_inside(outer: Span, inner: Span) -> bool {
    inner.start >= outer.start
        && inner.end <= outer.end
        && (inner.start != outer.start || inner.end != outer.end)
}

fn parse_breakpoint(spec: &str, current_file: &str) -> Option<(String, u32)> {
    if let Some((file, line)) = spec.rsplit_once(':') {
        let looks_like_path = file
            .chars()
            .any(|c| c.is_ascii_alphabetic() || matches!(c, '/' | '\\' | '.'));
        if looks_like_path {
            let line = line.parse().ok()?;
            return Some((file.to_string(), line));
        }
    }
    let line = spec.parse().ok()?;
    Some((current_file.to_string(), line))
}

pub fn source_line(source: &str, line: u32) -> &str {
    source
        .lines()
        .nth((line as usize).saturating_sub(1))
        .unwrap_or("")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::env::Env;

    fn env() -> Rc<RefCellEnv> {
        Env::new(FrameKind::Module, None)
    }

    fn ctx<'a>(
        file: &'a str,
        source: &'a str,
        span: Span,
        env: &'a Rc<RefCellEnv>,
        stack: &'a [CallFrame],
    ) -> DebugCtx<'a> {
        DebugCtx {
            file,
            source,
            span,
            env,
            stack,
        }
    }

    fn debugger() -> CliDebugger {
        CliDebugger::with_out(Box::new(io::sink()))
    }

    #[test]
    fn breakpoint_spec() {
        assert_eq!(parse_breakpoint("12", "a.lep"), Some(("a.lep".into(), 12)));
        assert_eq!(
            parse_breakpoint("lib.lep:3", "a.lep"),
            Some(("lib.lep".into(), 3))
        );
        assert_eq!(parse_breakpoint("abc", "a.lep"), None);
    }

    #[test]
    fn span_nesting() {
        let outer = Span::new(1, 1, 0, 20);
        let inner = Span::new(2, 1, 5, 10);
        assert!(strictly_inside(outer, inner));
        assert!(!strictly_inside(outer, outer));
        let sibling = Span::new(3, 1, 20, 25);
        assert!(!strictly_inside(outer, sibling));
    }

    #[test]
    fn source_line_is_one_based() {
        let src = "a = 1\nb = 2\n";
        assert_eq!(source_line(src, 1), "a = 1");
        assert_eq!(source_line(src, 2), "b = 2");
        assert_eq!(source_line(src, 99), "");
    }

    #[test]
    fn commands_change_step_mode() {
        let env = env();
        let span = Span::new(1, 1, 0, 5);
        let stack = [CallFrame {
            name: "<modulo>".into(),
            file: "a.lep".into(),
            span,
        }];
        let ctx = ctx("a.lep", "x = 1", span, &env, &stack);
        let mut dbg = debugger();

        assert_eq!(
            dbg.handle_command("continuar", &ctx),
            Some(DebugAction::Continue)
        );
        assert!(matches!(dbg.mode, StepMode::Continue));

        assert_eq!(
            dbg.handle_command("entrar", &ctx),
            Some(DebugAction::Continue)
        );
        assert!(matches!(dbg.mode, StepMode::StepIn));

        assert_eq!(
            dbg.handle_command("proximo", &ctx),
            Some(DebugAction::Continue)
        );
        assert!(matches!(dbg.mode, StepMode::StepOver { depth: 1, .. }));

        assert_eq!(
            dbg.handle_command("sair", &ctx),
            Some(DebugAction::Continue)
        );
        assert!(matches!(dbg.mode, StepMode::StepOut { depth: 0 }));

        assert_eq!(
            dbg.handle_command("terminar", &ctx),
            Some(DebugAction::Quit)
        );
        assert_eq!(dbg.handle_command("vars", &ctx), None);
        assert_eq!(dbg.handle_command("xyzzy", &ctx), None);
    }

    #[test]
    fn ponto_and_remover_use_current_file() {
        let env = env();
        let span = Span::new(3, 1, 0, 1);
        let ctx = ctx("media.lep", "x = 1", span, &env, &[]);
        let mut dbg = debugger();

        assert_eq!(dbg.handle_command("ponto 7", &ctx), None);
        assert!(dbg.breakpoints.contains(&("media.lep".into(), 7)));

        assert_eq!(dbg.handle_command("ponto lib.lep:2", &ctx), None);
        assert!(dbg.breakpoints.contains(&("lib.lep".into(), 2)));

        assert_eq!(dbg.handle_command("remover 7", &ctx), None);
        assert!(!dbg.breakpoints.contains(&("media.lep".into(), 7)));
    }

    #[test]
    fn continue_only_pauses_on_breakpoints() {
        let env = env();
        let span3 = Span::new(3, 1, 0, 4);
        let span4 = Span::new(4, 1, 5, 8);
        let stack = [];
        let mut dbg = debugger();
        dbg.mode = StepMode::Continue;
        dbg.breakpoints.insert(("a.lep".into(), 3));

        assert!(dbg.should_pause(&ctx("a.lep", "", span3, &env, &stack)));
        assert!(!dbg.should_pause(&ctx("a.lep", "", span4, &env, &stack)));
    }

    #[test]
    fn step_in_pauses_on_every_statement() {
        let env = env();
        let span = Span::new(1, 1, 0, 1);
        let mut dbg = debugger();
        dbg.mode = StepMode::StepIn;
        assert!(dbg.should_pause(&ctx("a.lep", "", span, &env, &[])));
    }

    #[test]
    fn step_over_skips_statements_nested_in_the_current_span() {
        let env = env();
        let outer = Span::new(1, 1, 0, 20);
        let inner = Span::new(2, 1, 5, 10);
        let after = Span::new(3, 1, 21, 25);
        let stack = [CallFrame {
            name: "<modulo>".into(),
            file: "a.lep".into(),
            span: outer,
        }];
        let mut dbg = debugger();
        dbg.mode = StepMode::StepOver {
            depth: 1,
            file: "a.lep".into(),
            span: outer,
        };

        assert!(!dbg.should_pause(&ctx("a.lep", "", inner, &env, &stack)));
        assert!(dbg.should_pause(&ctx("a.lep", "", after, &env, &stack)));
        assert!(dbg.should_pause(&ctx("a.lep", "", inner, &env, &[])));
    }

    #[test]
    fn step_out_pauses_when_the_call_stack_shrinks() {
        let env = env();
        let span = Span::new(1, 1, 0, 1);
        let deep = vec![
            CallFrame {
                name: "<modulo>".into(),
                file: "a.lep".into(),
                span,
            },
            CallFrame {
                name: "f".into(),
                file: "a.lep".into(),
                span,
            },
        ];
        let shallow = vec![CallFrame {
            name: "<modulo>".into(),
            file: "a.lep".into(),
            span,
        }];
        let mut dbg = debugger();
        dbg.mode = StepMode::StepOut { depth: 1 };

        assert!(!dbg.should_pause(&ctx("a.lep", "", span, &env, &deep)));
        assert!(dbg.should_pause(&ctx("a.lep", "", span, &env, &shallow)));
    }

    #[test]
    fn noop_hook_never_stops() {
        let env = env();
        let span = Span::new(1, 1, 0, 1);
        let ctx = ctx("a.lep", "x = 1", span, &env, &[]);
        let mut hook = NoopHook;
        assert_eq!(hook.before_stmt(&ctx), DebugAction::Continue);
    }
}
