# 001 — Prevent overflow at inclusive range endpoints

Status: Implemented for AST review; awaiting user review.

Priority/type: High · reproduced.

## Implementation record

Range endpoint regression covers both integer limits, empty ranges, and continue. See [AST-REVIEW.md](AST-REVIEW.md) for scope, compatibility, and verification. No commits or bytecode implementation are included.

## Problem

A range ending at i64::MAX increments past the endpoint. The debug binary panics after printing the final value; unchecked release arithmetic can wrap.

## Reproduction / example

```text
para i de 9223372036854775807 ate 9223372036854775807 { escreva(i) }
```

## Proposed direction

Stop after processing the endpoint or use checked advancement without excluding the final iteration.

## Acceptance criteria

The one-iteration range at i64::MAX terminates normally. Ordinary, empty, and interrupted ranges retain their behavior. Verify debug and release behavior.

## Pico 2 implications and dependencies

Pico 2: a host-language panic is especially disruptive in firmware; keep failure inside the language error model.

## Relevant code

- [src/runtime/eval.rs:772](../src/runtime/eval.rs#L772)

## Scope

This task records a finding or proposal, not an approved implementation. Resolve open semantic choices during individual review. Keep implementation changes and commits separate from this planning document.
