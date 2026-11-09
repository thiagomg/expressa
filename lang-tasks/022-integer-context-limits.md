# 022 — Specify supported integer ranges in restricted contexts

Status: Partially addressed for AST review; awaiting user review.

Priority/type: Decision · code finding.

## Implementation record

Existing integer range documented and out-of-range diagnostics clarified; broader integer representation remains pending. See [AST-REVIEW.md](AST-REVIEW.md) for scope, compatibility, and verification. No commits or bytecode implementation are included.

## Problem

The decimal number type can represent whole numbers beyond i64, but map keys, set elements, indices, and loop bounds use signed 64-bit integers. Whole-number rejection can be confusing.

## Proposed direction

Document the limit or deliberately expand the representation. Distinguish fractional input from an out-of-range whole number.

## Acceptance criteria

Each integer-only API reports useful errors for fractional and out-of-range values. Platform usize width does not silently truncate an accepted integer.

## Pico 2 implications and dependencies

Pico 2: especially important on a 32-bit target; audit all integer-to-usize casts.

## Relevant code

- [src/runtime/value.rs:100](../src/runtime/value.rs#L100)
- [src/runtime/eval.rs:1780](../src/runtime/eval.rs#L1780)

## Scope

This task records a finding or proposal, not an approved implementation. Resolve open semantic choices during individual review. Keep implementation changes and commits separate from this planning document.
