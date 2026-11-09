# 013 — Preserve source context for invalid control-flow errors

Status: Implemented for AST review; awaiting user review.

Priority/type: Medium · code finding.

## Implementation record

Invalid control-flow diagnostics preserve defining source/stack and debugger context. See [AST-REVIEW.md](AST-REVIEW.md) for scope, compatibility, and verification. No commits or bytecode implementation are included.

## Problem

Function calls restore the caller source and pop the frame before converting escaped pare/continue into errors. Imported-function spans can consequently be associated with the caller file.

## Proposed direction

Construct errors while the defining source and stack are still active, or attach source identity directly to control-flow signals.

## Acceptance criteria

Invalid pare/continue in imported functions identify the defining file and correct line with a useful stack. Valid loop control continues to work.

## Pico 2 implications and dependencies

Pico 2: compact diagnostics may omit source text but must retain accurate source identity.

## Relevant code

- [src/runtime/eval.rs:1344](../src/runtime/eval.rs#L1344)

## Scope

This task records a finding or proposal, not an approved implementation. Resolve open semantic choices during individual review. Keep implementation changes and commits separate from this planning document.
