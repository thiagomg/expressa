# 025 — Consider structured error information for recovery

Status: Proposed; awaiting individual review.

Priority/type: Proposal · language improvement.

## Problem

se_falhar provides a fallback but cannot inspect the error category or message. Prior mutations are not rolled back.

## Proposed direction

Keep the simple form and consider an optional handler binding with stable error categories. Explicitly document that recovery does not imply a transaction.

## Acceptance criteria

A design decision defines error visibility, propagation, control-flow exclusions, mutation behavior, and compatibility. Any implemented extension has examples for branching on errors.

## Pico 2 implications and dependencies

Pico 2: distinguish resource exhaustion and cancellation from recoverable program errors; keep error objects bounded.

## Relevant code

- [src/runtime/error.rs](../src/runtime/error.rs)
- [src/runtime/eval.rs:1098](../src/runtime/eval.rs#L1098)

## Scope

This task records a finding or proposal, not an approved implementation. Resolve open semantic choices during individual review. Keep implementation changes and commits separate from this planning document.
