# 014 — Specify outer binding versus object mutation

Status: Partially addressed for AST review; awaiting user review.

Priority/type: Decision · reproduced behavior.

## Implementation record

Existing outer-binding/object-mutation distinction documented; alternate semantics require separate review. See [AST-REVIEW.md](AST-REVIEW.md) for scope, compatibility, and verification. No commits or bytecode implementation are included.

## Problem

Functions cannot reassign outer bindings but can mutate outer lists, maps, and matrices. The specification says functions cannot modify outer variables without explaining this distinction.

## Reproduction / example

```text
xs = [1]
f = funcao() { xs[0] = 9 }
f()
escreva(xs)
```

## Proposed direction

Recommend defining the restriction as a ban on rebinding outer names, while permitting mutation of shared objects. Deep immutability would require a separate design.

## Acceptance criteria

Specification, tutorials, error wording, and examples distinguish binding assignment from object mutation. Tests cover both permitted mutation and rejected rebinding.

## Pico 2 implications and dependencies

Pico 2: preserve one semantic contract across targets.

## Relevant code

- [src/runtime/env.rs:127](../src/runtime/env.rs#L127)
- [especificacao-linguagem.md](../especificacao-linguagem.md)

## Scope

This task records a finding or proposal, not an approved implementation. Resolve open semantic choices during individual review. Keep implementation changes and commits separate from this planning document.
