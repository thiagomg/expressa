# 006 — Resolve nested text-writeback destinations once

Status: Implemented for AST review; awaiting user review.

Priority/type: High · code finding.

## Implementation record

Nested text writes use retained owner destinations; side-effect regressions are included. See [AST-REVIEW.md](AST-REVIEW.md) for scope, compatibility, and verification. No commits or bytecode implementation are included.

## Problem

Character assignment rebuilds a text value, then traverses its containing destination again to store it. Side effects in a receiver or index can redirect the write.

## Proposed direction

Reuse the resolved-destination abstraction from task 005, including nested writeback paths.

## Acceptance criteria

Side-effecting expressions in nested text targets are evaluated once. The rebuilt text returns to the same containing slot. Existing Unicode scalar indexing behavior remains documented and tested.

## Pico 2 implications and dependencies

Pico 2: bound temporary text allocations and avoid retaining redundant path objects.

## Relevant code

- [src/runtime/eval.rs:1619](../src/runtime/eval.rs#L1619)

## Scope

This task records a finding or proposal, not an approved implementation. Resolve open semantic choices during individual review. Keep implementation changes and commits separate from this planning document.
