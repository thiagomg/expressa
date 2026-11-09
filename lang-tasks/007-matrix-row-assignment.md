# 007 — Prevent silent writes to copied matrix rows

Status: Proposed; awaiting individual review.

Priority/type: High · reproduced.

## Problem

A[0] returns a copied list. A[0][0] = 9 succeeds but leaves A unchanged, making an apparent matrix update ineffective.

## Reproduction / example

```text
importe "matriz"
A = matriz([[1]])
A[0][0] = 9
escreva(A[0,0])
```

## Proposed direction

Choose between rejecting writes through copied rows, introducing row views, or implementing explicit matrix writeback. Rejection with guidance to use A[i,j] is the smallest change.

## Acceptance criteria

The nested assignment either updates the matrix according to a documented contract or raises a clear error. It never silently pretends to update the original matrix.

## Pico 2 implications and dependencies

Pico 2: row views may save copies but introduce lifetime and aliasing complexity; decide deliberately.

## Relevant code

- [src/runtime/eval.rs:1399](../src/runtime/eval.rs#L1399)

## Scope

This task records a finding or proposal, not an approved implementation. Resolve open semantic choices during individual review. Keep implementation changes and commits separate from this planning document.
