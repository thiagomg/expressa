# 034 — Define the Pico 2 execution and compatibility contract

Status: Proposed; awaiting individual review.

Priority/type: Pico 2 · architecture decision.

## Problem

Running Expressa on the microcontroller requires an explicit scope. The Pico 2 is not a Linux Raspberry Pi; desktop processes, windowing, and OS files cannot be assumed.

## Proposed direction

Expressa executes on the board using the same bytecode interpreter as desktop, without a JIT (task 036). Initially include on-board source parsing and compilation so `avalie(texto)` retains its current-scope behavior. Measure feasibility rather than assuming it fits. Decide console interaction and program delivery, including whether host-prepared bytecode is also useful. Start with one core and one architecture, provisionally Arm Cortex-M33, subject to toolchain validation.

## Acceptance criteria

Record board variant, architecture, program delivery, startup behavior, common language semantics, required libraries, and unsupported capabilities. Establish a minimal demo and measurable RAM/flash targets.

## Pico 2 implications and dependencies

The bytecode-interpreter direction is agreed. Complete the remaining target contract before fixing VM resource budgets, heap design, or firmware integration. If on-board compilation is infeasible, bring measured alternatives back for review rather than silently changing `avalie`.

## Relevant code

- [Cargo.toml](../Cargo.toml)
- [src/runtime/mod.rs](../src/runtime/mod.rs)

## Scope

This task records a finding or proposal, not an approved implementation. Resolve open semantic choices during individual review. Keep implementation changes and commits separate from this planning document.
