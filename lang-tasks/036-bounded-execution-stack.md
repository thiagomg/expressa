# 036 — Migrate to a shared bytecode interpreter with bounded execution

Status: Bytecode interpreter direction agreed; implementation and detailed design pending individual review. No JIT is planned.

Priority/type: Pico 2 · foundational.

## Problem

The evaluator reserves 256 KiB of host stack and allocates 4 MiB stack segments; value traversal and parser validation also use stacker. These sizes exceed the board memory budget. The existing call-depth limit is 10,000.

## Review gate

Do not begin bytecode implementation until the user reviews the AST changes in [AST-REVIEW.md](AST-REVIEW.md) and explicitly agrees to proceed. Preserve the approved AST reference and follow [the CPU/memory comparison protocol](../benchmarks/README.md). Comparison measurements follow a working, semantically equivalent bytecode implementation; none are claimed in this phase.

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
