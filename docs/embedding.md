# Embedding Expressa

Execution defaults to bytecode after the reviewed AST phase. Use `ExecutionOptions::backend = ExecutionBackend::Ast` for the temporary reference interpreter, or select `--backend ast|bytecode` in the CLI (`EXPRESSA_BACKEND` sets the default for CLI/REPL/hosts). Both backends use the same native services and scope rules. See [the current review](../lang-tasks/BYTECODE-REVIEW.md) and [comparison protocol](../benchmarks/README.md).

## Captured execution with explicit options

Use the re-exported types and functions in `expressa::runtime` rather than depending on the internal evaluator layout:

```rust
use expressa::runtime::{ExecutionBackend, ExecutionOptions, run_with_options};
use std::time::Duration;

fn main() -> Result<(), expressa::runtime::RuntimeError> {
    let options = ExecutionOptions {
        backend: ExecutionBackend::Bytecode,
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

## Bytecode representation and current limits

The compiler emits a private in-memory instruction format with constant/name pools, interned source spans, and nested function prototypes. Bytecode closures retain compiled code rather than AST blocks. There is no JIT. Desktop programs can be compiled to the versioned [EXB v1 file format](exb.md), loaded with `load_exb`/`read_exb`, and executed with `run_compiled_with_options`. `ExecutionOptions` gains the `backend` field; use `..ExecutionOptions::default()` for forwards-compatible option construction. Internally a closure body is now tagged as AST or bytecode.

Stored instructions retain the logical `Op` enum and a span index. The compact instruction experiment was reverted after review because the measured CPU/RSS costs outweighed the small code-storage savings in the workloads tested. Its [historical report and artifacts](../benchmarks/BYTECODE-STEP-05.md) remain available. The `.exb` file format uses a separate encoding and a validating loader; stored runtime instructions remain 16 bytes. This desktop implementation does not yet provide Pico firmware or execution directly from flash.

Ordinary Expressa calls and direct `avalie` calls use explicit VM frames. Compiled function parameters now have indexed storage shared with name-based lookup: `avalie`, closures, native callbacks, debugger snapshots and cycle collection see the same values. Only reads/writes in the function's main scope use indexed instructions; nested blocks and loops retain dynamic lookup to preserve shadowing. Other names remain dynamic, including names introduced by evaluation. The existing call-depth limit remains enforced. Parser/compiler recursion and synchronous native callbacks can still use growing host stacks; portable host separation, a bounded heap, and Pico compilation/hardware tests remain pending. The new engine is not yet a flashable Pico runtime.

Rust code reaching `DebugCtx.env` indirectly sees a changed implementation type for `Env.vars`: it is now `Bindings`, rather than a concrete `HashMap<String, Value>`. The map-style operations used by the workspace (`get`, `get_mut`, `insert`, `contains_key`, `iter`, `values`, `len`, `is_empty`, `clear`) remain available and include parameter values. External code relying on the concrete HashMap type or other HashMap-specific methods needs adjustment. See [the measured parameter optimization](../benchmarks/BYTECODE-STEP-04.md).
