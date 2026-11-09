# 008 — Validate flattened import bindings before installing names

Status: Implemented for AST review; awaiting user review.

Priority/type: High · reproduced.

## Implementation record

All flattened import names are validated before any are bound. See [AST-REVIEW.md](AST-REVIEW.md) for scope, compatibility, and verification. No commits or bytecode implementation are included.

## Problem

Flattened imports install sorted names one at a time. A later name conflict leaves earlier names installed even though the import fails.

## Reproduction / example

```text
fundo = 1
avalie("importe \"tela\"") se_falhar 0
escreva(tipo(casa) se_falhar "ausente")
```

## Proposed direction

Preflight all binding conflicts and assignment restrictions before installing any exported names. Treat module initialization side effects separately from binding atomicity.

## Acceptance criteria

A caught conflicting import adds no exported bindings. Existing bindings are preserved. Successful imports and aliases retain their behavior.

## Pico 2 implications and dependencies

Pico 2: module availability and bounded module tables must follow the embedded capability profile.

## Relevant code

- [src/runtime/eval.rs:657](../src/runtime/eval.rs#L657)

## Scope

This task records a finding or proposal, not an approved implementation. Resolve open semantic choices during individual review. Keep implementation changes and commits separate from this planning document.
