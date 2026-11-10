# 009 — Reclaim cycles in lists and maps without closures

Status: Partially addressed for AST review; awaiting user review.

Priority/type: High · code finding.

## Implementation record

Conservative collection-cycle reclamation is implemented and lifecycle-tested; large/borrowed graphs are deferred. The final bounded heap remains 037. See [AST-REVIEW.md](AST-REVIEW.md) for scope, compatibility, and verification. No commits or bytecode implementation are included.

## Problem

Lists and maps can contain themselves. Rc-based storage cannot reclaim such cycles through ordinary Drop. The existing cycle collector tracks function-containing environments rather than arbitrary standalone collection cycles.

## Proposed direction

Evaluate a general heap collector, arena with traced handles, or explicit cycle restrictions. Account for escaped values and persistent REPL sessions.

## Acceptance criteria

Repeated creation and disposal of self-cyclic and mutually cyclic collections has bounded live memory. Reachable cycles and closures remain usable. Verify cleanup during execution and at session shutdown.

## Pico 2 implications and dependencies

Pico 2: foundational work. A long-running board cannot rely on process exit for cleanup; coordinate with task 037.

## Relevant code

- [src/runtime/value.rs:72](../src/runtime/value.rs#L72)
- [src/runtime/eval.rs:480](../src/runtime/eval.rs#L480)

## Scope

This task records a finding or proposal, not an approved implementation. Resolve open semantic choices during individual review. Keep implementation changes and commits separate from this planning document.
