# 005 — Evaluate compound-assignment destinations once

Status: Implemented for AST review; awaiting user review.

Priority/type: High · reproduced.

## Implementation record

Compound assignment keeps its operator and resolves a single destination. See [AST-REVIEW.md](AST-REVIEW.md) for scope, compatibility, and verification. No commits or bytecode implementation are included.

## Problem

The parser expands += and -= into a binary expression plus ordinary assignment. Side-effecting target expressions run once for the read and again for the write, potentially selecting different destinations.

## Reproduction / example

```text
xs = [10,20]
m = mapa([:n -> 0])
idx = funcao() { m:n += 1 m:n - 1 }
xs[idx()] += 1
escreva(xs)
escreva(m:n)
```

## Proposed direction

Keep compound assignment explicit in the AST and resolve its destination once before read-modify-write. Define receiver, index, and RHS evaluation order.

## Acceptance criteria

A side-effecting index runs once; the selected element supplies the old value and receives the result. Cover names, list/map indices, fields, and matrix cells.

## Pico 2 implications and dependencies

Pico 2: resolved destinations should avoid unnecessary AST cloning and allocations.

## Relevant code

- [src/parser/parser.rs:262](../src/parser/parser.rs#L262)
- [src/runtime/eval.rs:1582](../src/runtime/eval.rs#L1582)

## Scope

This task records a finding or proposal, not an approved implementation. Resolve open semantic choices during individual review. Keep implementation changes and commits separate from this planning document.
