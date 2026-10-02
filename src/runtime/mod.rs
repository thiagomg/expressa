mod nativas;
mod debug;
mod env;
mod error;
mod eval;
mod formate;
mod leia;
mod rng;
mod repl;
mod value;

pub use debug::{
    ChannelDebugger, CliDebugger, DebugAction, DebugBinding, DebugCtx, DebugFrameInfo, DebugHook,
    DebugPaused, DebugSession, NoopHook, StopHook, collect_stack, collect_vars, snapshot,
};
pub use error::{CallFrame, RuntimeError};
pub use eval::{
    debug_source, debug_source_args, run_source, run_source_args, run_source_marcador,
    run_source_marcador_args, run_to_string, run_to_string_with, run_to_strings_with,
    run_with_hook, run_with_leia_host,
};
pub use leia::{CLEAR_SCREEN, CURSOR_HOME, LEIA_MARKER, LeiaHost};
pub use nativas::{BUILTIN_DOCS, BuiltinDoc, NATIVAS, Nativa, lookup_builtin_doc};
pub use repl::run_repl;
pub use value::{NumeroLocale, Value, default_numero_locale};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_run_to_string_executes_a_program() {
        let out = run_to_string(r#"escreva("ok")"#, "t.lep").unwrap();
        assert_eq!(out, "ok\n");
    }

    #[test]
    fn public_run_to_string_surfaces_runtime_errors() {
        let err = run_to_string("1 / 0", "t.lep").unwrap_err();
        assert!(err.message.contains("divisão por zero"));
        assert_eq!(err.file, "t.lep");
    }
}
