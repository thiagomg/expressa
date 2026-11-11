mod bindings;
mod cartesiano_host;
mod debug;
mod env;
mod error;
mod eval;
mod folha;
mod fonte8x8;
mod formate;
mod graf;
mod graf_host;
#[cfg(feature = "graphics")]
mod graf_proto;
mod imagem;
mod leia;
mod letterbox;
mod nativas;
mod numero;
mod pilha;
#[cfg(feature = "process")]
mod processo;
#[cfg(feature = "terminal")]
mod repl;
mod rng;
#[cfg(feature = "terminal")]
mod terminal;
mod value;
#[cfg(feature = "graphics")]
mod viewer;

#[cfg(feature = "graphics")]
pub use cartesiano_host::CliCartesianoHost;
pub use cartesiano_host::{CapturaCartesiano, CartesianoHost, NullCartesiano};
#[cfg(feature = "terminal")]
pub use debug::CliDebugger;
pub use debug::{
    ChannelDebugger, DebugAction, DebugBinding, DebugCtx, DebugFrameInfo, DebugHook, DebugPaused,
    DebugSession, NoopHook, StopHook, collect_stack, collect_vars, snapshot,
};
pub use error::{CallFrame, RuntimeError};
pub use eval::{
    CompiledProgram, DEFAULT_MAX_MATRIX_CELLS, EXB_MAX_BYTES, EXB_VERSION, ExbError, ExbRunMode,
    ExecutionBackend, ExecutionOptions, ExecutionOutput, compile_exb, debug_source,
    debug_source_args, load_exb, read_exb, run_compiled_with_options, run_exb_file_args,
    run_exb_with_options, run_source, run_source_args, run_source_marcador,
    run_source_marcador_args, run_to_string, run_to_string_with, run_to_strings_with,
    run_with_hook, run_with_hosts, run_with_leia_host, run_with_options,
};
pub use folha::{CurvaPronta, FolhaPronta, GraficoPronto, pintar};
pub use graf::{OpGraf, TelaGrafica, cor_tela_rgb32};
pub use graf_host::{CapturaGraf, EventoGraf, GrafHost, NullGraf};
pub use imagem::Imagem;
pub use leia::{CLEAR_SCREEN, CURSOR_HOME, LEIA_MARKER, LeiaHost};
pub use letterbox::{Letterbox, blit_letterbox, letterbox};
pub use nativas::{
    BuiltinDoc, NATIVAS, Nativa, builtin_docs, docs_agrupados, lookup_alvo, lookup_builtin_doc,
    lookup_ficha, tema_de, titulo_modulo,
};
pub use numero::{Numero, NumeroAritmetica, default_numero_aritmetica};
pub use pilha::PROFUNDIDADE_MAXIMA;
#[cfg(feature = "terminal")]
pub use repl::{
    CanalRepl, ConteudoRepl, EventoRepl, FimRepl, FluxoRepl, IdAvaliacao, IdSessao,
    REPL_MAX_CODIGO, TextoRepl, run_repl,
};
pub use value::{NumeroLocale, Value, default_numero_locale};
#[cfg(feature = "graphics")]
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
