# 011 — Bound matrix and other user-controlled allocations

Status: Partially addressed for AST review; awaiting user review.

Priority/type: High · code finding.

## Implementation record

Matrix limits and fallible allocation implemented; whole-runtime allocation budgeting remains pending. See [AST-REVIEW.md](AST-REVIEW.md) for scope, compatibility, and verification. No commits or bytecode implementation are included.

## Problem

Matrix constructors accept enormous positive dimensions and allocate directly. Allocation failure and capacity overflow can escape the language error model.

## Proposed direction

Introduce checked dimension conversion, checked cell-count arithmetic, and configurable allocation limits. Audit other user-sized buffers and use fallible allocation where practical.

## Acceptance criteria

Oversized requests fail predictably before large allocations. Matrix limits cover cell count and byte cost. Desktop and embedded limits can differ without changing ordinary semantics.

## Pico 2 implications and dependencies

Pico 2: raise priority and broaden this to the whole runtime; see task 037.

## Relevant code

- [src/runtime/nativas/matriz.rs:60](../src/runtime/nativas/matriz.rs#L60)

## Scope

This task records a finding or proposal, not an approved implementation. Resolve open semantic choices during individual review. Keep implementation changes and commits separate from this planning document.
