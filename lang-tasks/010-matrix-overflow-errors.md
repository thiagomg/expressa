# 010 — Check matrix arithmetic results consistently

Status: Implemented for AST review; awaiting user review.

Priority/type: High · code finding.

## Implementation record

Matrix results are range-checked; precision/storage remains pending task 003. See [AST-REVIEW.md](AST-REVIEW.md) for scope, compatibility, and verification. No commits or bytecode implementation are included.

## Problem

Matrix arithmetic performs unchecked f64 calculations and can create nonfinite or decimal-unrepresentable cells. Later reads or printing can encounter silent zero substitution.

## Proposed direction

Apply the numeric contract selected in task 003 and explicit conversion failures from task 002 throughout matrix operations.

## Acceptance criteria

Addition, scaling, multiplication, and native matrix results either produce valid cells or a catchable documented error. No NaN, infinity, or out-of-range cell becomes a misleading zero.

## Pico 2 implications and dependencies

Pico 2: also bound operation cost and temporary storage.

## Relevant code

- [src/runtime/eval.rs:1494](../src/runtime/eval.rs#L1494)

## Scope

This task records a finding or proposal, not an approved implementation. Resolve open semantic choices during individual review. Keep implementation changes and commits separate from this planning document.
