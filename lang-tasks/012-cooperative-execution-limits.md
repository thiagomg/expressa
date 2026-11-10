# 012 — Apply execution limits inside expensive native operations

Status: Partially addressed for AST review; awaiting user review.

Priority/type: High · code finding.

## Implementation record

Matrix/sort/selected collection loops and sleep polling implemented; blocking host I/O and full native coverage remain pending. See [AST-REVIEW.md](AST-REVIEW.md) for scope, compatibility, and verification. No commits or bytecode implementation are included.

## Problem

Statement-level deadline checks do not bound long native computations or blocking input. A configured deadline can expire while native code still runs.

## Proposed direction

Define cooperative cancellation boundaries and poll in substantial computations and host waits. Separate monotonic time from operating-system APIs.

## Acceptance criteria

Representative long matrix, collection, and input operations respond to cancellation within a documented bound. Limits cannot be defeated by error recovery. Blocking host contracts are explicit.

## Pico 2 implications and dependencies

Pico 2: use instruction/work budgets and a board monotonic clock; integrate watchdog servicing and USB polling in task 041.

## Relevant code

- [src/runtime/eval.rs:1935](../src/runtime/eval.rs#L1935)
- [src/runtime/nativas/nucleo.rs:80](../src/runtime/nativas/nucleo.rs#L80)

## Scope

This task records a finding or proposal, not an approved implementation. Resolve open semantic choices during individual review. Keep implementation changes and commits separate from this planning document.
