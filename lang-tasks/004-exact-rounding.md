# 004 — Preserve exact values in rounding functions

Status: Implemented for AST review; awaiting user review.

Priority/type: High · reproduced.

## Implementation record

Exact decimal rounding and halfway regressions are included. See [AST-REVIEW.md](AST-REVIEW.md) for scope, compatibility, and verification. No commits or bytecode implementation are included.

## Problem

piso, teto, and arredonde convert through f64. piso(9007199254740993) currently returns 9007199254740992.

## Reproduction / example

```text
importe "mat"
escreva(piso(9007199254740993))
```

## Proposed direction

Implement decimal rounding directly on Numero, with a documented halfway rule and explicit behavior for float mode.

## Acceptance criteria

Large exact integers remain unchanged by rounding. Positive and negative fractional and halfway cases follow the documented rule.

## Pico 2 implications and dependencies

Pico 2: direct decimal rounding also avoids unnecessary software floating-point conversions.

## Relevant code

- [src/runtime/nativas/mat.rs:121](../src/runtime/nativas/mat.rs#L121)

## Scope

This task records a finding or proposal, not an approved implementation. Resolve open semantic choices during individual review. Keep implementation changes and commits separate from this planning document.
