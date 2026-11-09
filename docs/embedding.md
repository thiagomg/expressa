# Embedding the AST interpreter

The current implementation executes the AST. A bytecode backend is planned but is gated on explicit user review and agreement; it is not included in these changes. See [the review record](../lang-tasks/AST-REVIEW.md) and [the later comparison protocol](../benchmarks/README.md).

## Captured execution with explicit options

Use the re-exported types and functions in `expressa::runtime` rather than depending on the internal evaluator layout:

```rust
use expressa::runtime::{ExecutionOptions, run_with_options};
use std::time::Duration;

fn main() -> Result<(), expressa::runtime::RuntimeError> {
    let options = ExecutionOptions {
        time_limit: Some(Duration::from_secs(2)),
        max_matrix_cells: 4096,
        ..ExecutionOptions::default()
    };
    let output = run_with_options("escreva(2 + 3)", "example.lep", "", options)?;
    assert_eq!(output.stdout, "5\n");
    assert_eq!(output.stderr, "");
    assert_eq!(output.exit_code, 0);
    Ok(())
}
```

`ExecutionOptions` names the existing workspace confinement and evaluation deadline settings and the new per-matrix cell budget. `ExecutionOutput` keeps stdout, stderr, and the language exit code distinct. Existing `run_source*`, `run_to_string*`, and host/debug entry points remain available for compatibility. The legacy host wrappers still have positional arguments; replacing that public API is not silently bundled into this change.

The default matrix budget is 1,048,576 cells per matrix. It applies to language constructors and arithmetic results, including temporaries, and is not an aggregate heap limit. Allocations use checked dimensions and fallible reserves. Other lists, text, parser data, and native buffers do not yet have a global allocator budget. Rust host code constructing `Value::matriz` directly remains responsible for its representation invariants and allocations.

The deadline is cooperative during execution. Matrix loops, sorting, selected collection constructors, and sleeps poll for cancellation. It does not preempt parsing, a blocking host input implementation, standard-library string operations, or every native operation. A nonblocking/cancellable host contract and a whole-runtime allocation budget remain pending tasks 012 and 037. `workspace_root` retains its existing behavior; it is not an operating-system sandbox.

## Values and diagnostics

`Numero` no longer implements `From<f64>`: an out-of-range or nonfinite conversion must be handled with `Numero::try_from(value)` or `Numero::from_f64(value)`. Decimal underflow/rounding and the current float-intermediate model are separate existing semantics. Integer conversions remain infallible for the provided integer types.

`LexError::kind` and `ParseError::kind` identify diagnostic categories independently of Portuguese messages. `ParseError::is_incomplete()` is the completeness contract used by the REPL. The AST `Stmt::Assign` now carries `compound: Option<BinaryOp>`; += and -= are no longer expanded into a duplicated target expression. Rust consumers constructing AST nodes or matching these structs exhaustively must adapt to the new fields. Expressa source syntax is unchanged.

AST types remain re-exported from `expressa::parser`, while compatibility modules, including the empty `parser::ast`, remain available. No compatibility module was removed solely for cosmetic cleanup.

## Memory and lifecycle

The existing environment/closure collector is supplemented by conservative collection-only cycle reclamation. Collections are tracked by thread-local weak references; external Rust handles, active evaluator temporaries, and references from environments count as roots. Registry scans are amortized over allocations, and collection runs at execution checkpoints, REPL boundaries, and VM cleanup. Borrowed or oversized graphs are deferred conservatively; this is not the final bounded heap/GC architecture for Pico.

Values containing cycles that outlive an execution remain the host's responsibility until another collection checkpoint runs. The AST runtime still uses dynamically growing host stacks and is not Pico firmware. Task 036 replaces that execution architecture only after review.
