# 036 — Migrate to a shared bytecode interpreter with bounded execution

Status: Desktop bytecode interpreter implemented for review; Pico portability, whole-runtime memory bounds, and remaining native/parser stack growth are still pending. No JIT is included.

Priority/type: Pico 2 · foundational.

## Problem

The evaluator reserves 256 KiB of host stack and allocates 4 MiB stack segments; value traversal and parser validation also use stacker. These sizes exceed the board memory budget. The existing call-depth limit is 10,000.

## Review gate and implementation record

The user committed the AST phase as `bd654b5` and then explicitly approved starting bytecode implementation. The AST reference was preserved before edits. See [BYTECODE-REVIEW.md](BYTECODE-REVIEW.md) and [the measured CPU/RSS comparison](../benchmarks/AST-VS-BYTECODE.md).

Implemented: source-to-bytecode compilation, explicit value/call/control stacks, compiled closures without retained AST bodies, runtime compilation for `avalie`, dynamic names and escaping closures, all statement/expression forms, imports/native callbacks, process cleanup, and debugger statement/source mapping. The default backend is bytecode; AST remains selectable for conformance and comparison. Checkpoint tests assert that bytecode execution does not fall back to AST evaluation.

The first implementation deliberately retains dynamic name-based environments, the existing numeric/value model, and desktop host services. The [parameter-slot optimization](045-indexed-function-parameters.md) now covers function-root parameters; broader local-slot optimization, no_std separation, global heap budgets, compiler/parser stack bounds, and synchronous native callback reentrancy remain embedded prerequisites. Ordinary Expressa calls use VM frames, but this does not mean every native/compiler path is ready for Pico.

## Proposed direction

Replace tree-walking execution with one shared bytecode interpreter for desktop and Pico 2. Compile source to bytecode and execute it with explicit VM call frames and value slots. Do not implement a JIT. Prototype the execution and scope model before broad migration; register-versus-stack bytecode design and instruction encoding remain open.

Preserve `avalie(texto)` by compiling text at runtime against the current scope and executing it in the same VM. Retain the name-to-binding information it needs, support dynamically introduced names, and give captured bindings stable lifetimes. Resolve how already compiled code and closures observe later bindings without changing Expressa scope rules.

Inventory every recursive path beyond evaluation. Make parsing, equality, formatting, and cleanup bounded too. Preserve source mapping for errors, stepping, breakpoints, and imported functions. Keep the old evaluator as a temporary conformance reference during migration, then retire it once parity is established.

## Acceptance criteria

Desktop and Pico execute the same bytecode instruction semantics without JIT compilation. No embedded path depends on stacker or dynamic OS stack growth. Measured worst-case stack use fits the budget. Excess depth returns a language error without firmware failure.

Conformance checks cover locals, scope boundaries, closures, loops, control flow, imports, errors, and debugger source locations. `avalie` can read visible bindings, perform permitted updates, introduce bindings visible to subsequent ordinary code, and return working closures after compilation temporaries are released. Verify later-binding visibility and forbidden outer rebinding explicitly. Compilation and execution both obey resource limits.

Measure flash, peak RAM, stack use, and recovered memory for a minimal end-to-end workload before expanding the migration. Desktop CLI, REPL, and Aula integrations retain their reviewed behavior.

## Pico 2 implications and dependencies

Extends 009, 012, 026, and 033. Numeric and language contracts must survive any new execution representation.

## Relevant code

- [src/runtime/pilha.rs](../src/runtime/pilha.rs)
- [src/runtime/eval.rs](../src/runtime/eval.rs)
- [src/runtime/value.rs](../src/runtime/value.rs)
- [src/parser/parser.rs](../src/parser/parser.rs)
- [src/parser/se_value.rs](../src/parser/se_value.rs)

## Scope

The shared bytecode interpreter is the agreed architecture. This document does not implement the migration or approve unrelated language changes. Detailed design and implementation remain subject to individual review. No JIT is in scope; keep implementation changes and commits separate from this planning document.
