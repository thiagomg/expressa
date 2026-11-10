# 003 — Define consistent scalar and matrix numeric semantics

Status: Proposed; awaiting individual review.

Priority/type: High · reproduced.

## Problem

Matrix cells use f64 while scalar numbers use decimal storage. Merely storing 0.1 or 9007199254740993 in a matrix changes equality with the original scalar.

## Reproduction / example

```text
importe "matriz"
A = matriz([[0.1]])
escreva(A[0,0] == 0.1)
B = matriz([[9007199254740993]])
escreva(B[0,0] == 9007199254740993)
```

## Proposed direction

Prefer Numero cells if matrices promise the same numeric semantics. Otherwise explicitly define approximate matrices and their conversions; measure representation costs first.

## Acceptance criteria

A documented decision defines storage, arithmetic mode, conversion, equality, and overflow. Regression examples verify the selected contract rather than silently losing precision.

## Pico 2 implications and dependencies

Pico 2: do not blindly enlarge every matrix cell. Coordinate with numeric benchmarks (039) and optional module profiles (038).

## Relevant code

- [src/runtime/value.rs:25](../src/runtime/value.rs#L25)
- [src/runtime/eval.rs:1459](../src/runtime/eval.rs#L1459)

## Scope

This task records a finding or proposal, not an approved implementation. Resolve open semantic choices during individual review. Keep implementation changes and commits separate from this planning document.
