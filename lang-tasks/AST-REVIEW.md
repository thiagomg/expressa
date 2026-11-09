# AST improvements for review before bytecode migration

Status: implemented AST changes awaiting user review. No commits, bytecode backend, JIT, or Pico firmware are included.

The user requested AST improvements first, followed by review and explicit agreement. Only after that agreement will we implement the shared bytecode interpreter and compare its CPU and memory consumption against the approved AST reference. The agreed architecture does not bypass this gate. See [the benchmark protocol](../benchmarks/README.md).

## Implemented changes

| Tasks | Reviewable change |
| --- | --- |
| 001 | Inclusive ranges stop after their endpoint without overflowing i64, including `continue` at the final iteration. |
| 002 | Remove zero-substituting `From<f64>`; use explicit fallible conversion. Audit native math, matrices, graphical metadata, plotting, and formatting. Integer metadata stays integer. |
| 004 | Decimal-native floor, ceiling, and rounding preserve large exact values. Halfway rounding remains away from zero. |
| 005–006 | Preserve compound operators in the AST and resolve assignment destinations once. Nested text writeback retains its owner rather than reevaluating receiver/index expressions. |
| 008 | Preflight all flattened import binding conflicts and assignment restrictions before publishing names. Module initialization effects are not rolled back. |
| 010 | Reject matrix arithmetic results and cell conversions outside the scalar representation instead of storing/printing misleading zeros. f64 matrix storage itself is retained pending 003. |
| 013 | Construct invalid loop-control errors while the defining function source and frame are active; report that context to debugger hooks. |
| 030 | Add typed lexical/parser diagnostic kinds. REPL completeness no longer depends on message wording or duplicate tokenization. Malformed input is reported without waiting merely because a delimiter is open. |

## Supporting improvements and partial tasks

| Tasks | Implemented portion and remaining work |
| --- | --- |
| 009 | Supplement closure cleanup with conservative trial deletion for list/map cycles, including pairs wrapping collections. Use weak tracking, amortized checkpoint scans, and REPL/VM cleanup. Preserve external roots and borrowed objects. Oversized graphs are conservatively deferred; this is not the final Pico heap collector. |
| 011 | Checked matrix dimensions/cell counts, fallible output reserves, and configurable per-matrix budget. Default: 1,048,576 cells. This is not a total heap budget; lists, text, parsing, other native buffers, and direct Rust value construction remain outside it. |
| 012 | Poll cancellation/deadlines in matrix allocation/arithmetic/transpose, stable sorting, selected collection construction, and line-reading checkpoints. Sleeps remain stoppable even when longer than the deadline. Parsing and blocking host I/O are not preempted; not every native operation is covered. |
| 014, 017, 018, 022, 023 | Document current semantics: outer rebinding versus shared-object mutation; trailing conditionals; actual expression/block results and nonempty blocks; integer limits; shallow copies and aliasing. No alternate rules are selected. Out-of-range integers now have a distinct explanatory message. |
| 026 | Extract assignment and matrix execution into cohesive evaluator modules and move existing evaluator tests into a separate module. Broader decomposition remains pending. |
| 027 | Add named `ExecutionOptions`, `ExecutionOutput`, and `run_with_options`; captured convenience functions delegate to them. Retain existing public wrappers for compatibility. Legacy host APIs still have positional parameters. |
| 031–032 | Correct contradictory specification wording, remove the placeholder/stale debt reference and token scaffolding checklist, and document the embedding facade/API changes. A complete normative grammar, library-reference reorganization, and public-module deprecation policy remain pending. |
| 033 | Add semantic regressions, cycle-lifecycle checks, native cancellation checks, and reusable correctness-checked benchmark workloads. Broader cross-backend conformance and fuzzing follow later work. |

## Behavior and compatibility details to review

- Compound assignment reads receiver/indices left to right, then evaluates the RHS, then writes the same destination. Ordinary assignment preserves the existing RHS-first behavior and index-before-receiver order for `[i]`. No RefCell borrow is held while running the RHS. Text writeback reads the current text from the retained owner slot so RHS changes to other characters survive.
- Native rounding always operates on the stored decimal value, including when binary-intermediate arithmetic mode is selected. Decimal underflow/rounding and float-mode representation are not redesigned.
- Matrix arithmetic remains approximate because storage is still f64. A valid decimal can round to an unrepresentable f64 cell near the numeric boundary; that conversion now errors instead of later becoming zero. Matrix precision and nested writes through copied rows remain decisions in 003 and 007.
- The new matrix limit is a host resource policy. Embedders can configure it through `ExecutionOptions`; existing CLI/host entry points use the default. Aggregate heap accounting remains pending.
- Rust API changes: `Numero::try_from(f64)` replaces `.into()`; `Stmt::Assign` gains `compound`; `LexError` and `ParseError` gain `kind`. Existing source-language syntax and Rust execution wrappers remain available. See [embedding notes](../docs/embedding.md).
- Collection tracking adds weak-reference and scan overhead. It is conservative and defers borrowed or large graphs rather than risking reachable values. This fixes collection-only cycles without claiming a complete bounded tracing collector for Pico.

## Work deliberately left for separate review

Tasks 003, 007, 015–016, 019–021, 024–025, and observable portions of 028 require semantic decisions. Native descriptor consolidation (029) is still pending and should coordinate with native identity (028). Remaining work in partial tasks above must not be treated as complete.

Portable-core extraction, compact value/frame design, decimal benchmarks on actual Pico hardware, the bytecode engine, full heap budgeting, board libraries/loading/scheduling, and firmware remain tasks 034–043. None is silently implemented as part of this AST review.

## Validation record

Final checks on the review changes:

- `cargo test --workspace --offline --locked --quiet`: passed. Top-level suite totals: 577 passed and 3 existing ignored tests, including 363 interpreter library tests and 21 new integration regressions. Subprocess helper invocations are not double-counted. GTK emitted a frame-presentation timing warning in one successful UI run.
- `cargo test -p expressa --release --offline --locked --lib --test ast_regressions --quiet`: 363 library tests and 21 regressions passed. This verifies the range fix with release arithmetic as well as debug checks.
- `cargo check --workspace --all-targets --offline --locked`: passed.
- `cargo clippy --workspace --all-targets --offline --locked -- -D warnings`: passed.
- `cargo fmt -p expressa --check` and `git diff --check`: passed.
- Release AST benchmark: seven workloads, seven measured samples per workload after two warmup rounds; every output and exit-status check passed. See [the provisional baseline](../benchmarks/AST-BASELINE.md) and [raw samples](../benchmarks/results/ast-review.json). Source, executable, runner, and native-helper fingerprints were verified against the recorded files.

The existing home-cache filesystem tests required execution outside the workspace sandbox; they passed. No full fuzzing campaign or Pico hardware validation was performed. No bytecode CPU/memory comparison is possible yet: the VM remains unimplemented pending explicit review agreement.

The baseline measures fresh-process startup, parsing, execution, and teardown. It is provisional until the user accepts or regenerates it. The later benchmark protocol requires preserving the approved AST binary/source and measuring the semantically equivalent bytecode interpreter on the same host/build settings, with CPU time and peak RSS reported separately from execution-only and hardware measurements.
