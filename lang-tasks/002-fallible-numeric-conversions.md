# 002 — Remove silent zero substitution from numeric conversions

Status: Implemented for AST review; awaiting user review.

Priority/type: High · reproduced.

## Implementation record

All former float conversions are explicit and checked; unrepresentable native results are catchable errors. See [AST-REVIEW.md](AST-REVIEW.md) for scope, compatibility, and verification. No commits or bytecode implementation are included.

## Problem

From<f64> for Numero converts unrepresentable values to zero. exp(100) is finite in f64 but outside decimal range, and currently returns 0 instead of an error.

## Reproduction / example

```text
importe "mat"
escreva(exp(100) se_falhar "erro")
```

## Proposed direction

Replace the infallible conversion with an explicit fallible API and audit native results, matrix reads, and formatting.

## Acceptance criteria

exp(100) reports a catchable range error under the existing decimal representation. No conversion failure silently produces zero. Valid conversions retain their values.

## Pico 2 implications and dependencies

Pico 2: choose the embedded numeric representation in task 039 before broad conversion refactoring.

## Relevant code

- [src/runtime/numero.rs:197](../src/runtime/numero.rs#L197)
- [src/runtime/nativas/mat.rs:161](../src/runtime/nativas/mat.rs#L161)

## Scope

This task records a finding or proposal, not an approved implementation. Resolve open semantic choices during individual review. Keep implementation changes and commits separate from this planning document.
