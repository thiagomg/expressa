mod cartesiano_host;
mod debug;
mod env;
mod error;
mod eval;
mod folha;
mod formate;
mod graf;
mod graf_host;
mod graf_proto;
mod imagem;
mod leia;
mod nativas;
mod numero;
mod repl;
mod rng;
mod value;
mod viewer;

pub use cartesiano_host::{CapturaCartesiano, CartesianoHost, CliCartesianoHost, NullCartesiano};
pub use debug::{
    collect_stack, collect_vars, snapshot, ChannelDebugger, CliDebugger, DebugAction, DebugBinding,
    DebugCtx, DebugFrameInfo, DebugHook, DebugPaused, DebugSession, NoopHook, StopHook,
};
pub use error::{CallFrame, RuntimeError};
pub use eval::{
    debug_source, debug_source_args, run_source, run_source_args, run_source_marcador,
    run_source_marcador_args, run_to_string, run_to_string_with, run_to_strings_with,
    run_with_hook, run_with_hosts, run_with_leia_host,
};
pub use folha::{pintar, CurvaPronta, FolhaPronta, GraficoPronto};
pub use graf::{OpGraf, TelaGrafica};
pub use graf_host::{CapturaGraf, EventoGraf, GrafHost, NullGraf};
pub use imagem::Imagem;
pub use leia::{LeiaHost, CLEAR_SCREEN, CURSOR_HOME, LEIA_MARKER};
pub use nativas::{
    builtin_docs, lookup_alvo, lookup_builtin_doc, lookup_ficha, BuiltinDoc, Nativa, NATIVAS,
};
pub use numero::{default_numero_aritmetica, Numero, NumeroAritmetica};
pub use repl::run_repl;
pub use value::{default_numero_locale, NumeroLocale, Value};
pub use viewer::{run_live_viewer, run_viewer};

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
