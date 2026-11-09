# 020 — Define what float arithmetic mode promises

Status: Proposed; awaiting individual review.

Priority/type: Decision · reproduced limitation.

## Problem

Float mode uses f64 intermediates and immediately stores results as decimals. Decimal range and scale still apply, so it is not a full IEEE-754 value model. Tiny representable f64 results can become zero.

## Reproduction / example

```text
formato("float")
escreva(0.0000000000000000000000000001 / 10)
```

## Proposed direction

Either document binary intermediate arithmetic precisely or introduce a floating representation with explicit conversion and comparison rules.

## Acceptance criteria

Define range, underflow, overflow, rounding, equality, serialization, and mode-switch behavior. Tests distinguish the selected model from a full f64 model.

## Pico 2 implications and dependencies

Pico 2: decide alongside task 039; do not silently change the default numeric semantics for the board.

## Relevant code

- [src/runtime/numero.rs:120](../src/runtime/numero.rs#L120)

## Scope

This task records a finding or proposal, not an approved implementation. Resolve open semantic choices during individual review. Keep implementation changes and commits separate from this planning document.
