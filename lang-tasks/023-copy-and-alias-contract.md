# 023 — Document copying, aliasing, and binding immutability together

Status: Partially addressed for AST review; awaiting user review.

Priority/type: Decision · observed behavior.

## Implementation record

Existing copy/alias/binding behavior documented; no copy model redesign. See [AST-REVIEW.md](AST-REVIEW.md) for scope, compatibility, and verification. No commits or bytecode implementation are included.

## Problem

Texts behave as values; lists, maps, and matrices share storage; collection-producing operations often make shallow copies; matrix row reads copy. argumentos cannot be rebound but its list elements can change.

## Reproduction / example

```text
argumentos[0] = "changed"
escreva(argumentos)
```

## Proposed direction

Write a single value-semantics contract and concise examples distinguishing shared storage, shallow copies, independent values, and read-only bindings. Decide whether copy helpers are needed.

## Acceptance criteria

Assignment, parameter passing, concatenation, removal, matrix rows, nested objects, and argumentos are covered consistently in docs and tests.

## Pico 2 implications and dependencies

Pico 2: explicit alias rules are prerequisites for safe memory optimizations.

## Relevant code

- [src/runtime/value.rs](../src/runtime/value.rs)
- [src/runtime/eval.rs:1399](../src/runtime/eval.rs#L1399)

## Scope

This task records a finding or proposal, not an approved implementation. Resolve open semantic choices during individual review. Keep implementation changes and commits separate from this planning document.
