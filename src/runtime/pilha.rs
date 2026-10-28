//! Limits that keep deep recursion a normal Expressa error instead of a
//! Rust stack overflow (which aborts the process — and the Aula with it).

/// Function calls deeper than this raise "recursão profunda demais".
pub const PROFUNDIDADE_MAXIMA: usize = 10_000;

/// When less than this much stack is left, the evaluator moves to a new
/// segment ([`stacker::maybe_grow`]). Must cover the deepest Rust path
/// between two `eval_expr` calls, which is large in debug builds.
pub(crate) const PILHA_RESERVA: usize = 256 * 1024;

/// Size of each extra stack segment allocated on the heap.
pub(crate) const PILHA_SEGMENTO: usize = 4 * 1024 * 1024;
