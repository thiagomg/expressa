# 037 — Design a bounded heap and runtime resource model

Status: Proposed; awaiting individual review.

Priority/type: Pico 2 · foundational.

## Problem

A 520 kB total SRAM budget must also hold firmware stacks, USB buffers, parser/program state, and runtime values. Unbounded strings, AST retention, environment maps, collection cycles, and allocation metadata compete for it.

## Proposed direction

Measure representative allocations and choose a heap/arena/GC strategy with explicit budgets for programs, values, frames, output, and temporary work. Include fragmentation, recoverable failures where feasible, and clean reset between runs.

## Acceptance criteria

Publish a memory budget backed by measurements. Repeated programs and cyclic data do not leak. Oversized input and allocations fail predictably. Document any fatal allocation boundary honestly.

## Pico 2 implications and dependencies

Extends 009, 011, 016, and 023. This is required before promising a useful embedded workload.

## Relevant code

- [src/runtime/value.rs](../src/runtime/value.rs)
- [src/runtime/env.rs](../src/runtime/env.rs)
- [src/runtime/eval.rs](../src/runtime/eval.rs)
- [src/runtime/nativas/matriz.rs](../src/runtime/nativas/matriz.rs)

## Scope

This task records a finding or proposal, not an approved implementation. Resolve open semantic choices during individual review. Keep implementation changes and commits separate from this planning document.
