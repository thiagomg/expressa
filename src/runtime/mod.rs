mod builtins;
mod debug;
mod env;
mod error;
mod eval;
mod value;

pub use debug::{CliDebugger, DebugAction, DebugCtx, DebugHook, NoopHook};
pub use error::{CallFrame, RuntimeError};
pub use eval::{debug_source, run_source, run_to_string};
pub use value::Value;

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
