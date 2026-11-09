# 015 — Define local declaration and shadowing rules

Status: Proposed; awaiting individual review.

Priority/type: Decision · observed behavior.

## Problem

Parameters can shadow outer and builtin names, but ordinary assignment to those names is rejected. Because = combines declaration and reassignment, a function cannot naturally declare a same-named local.

## Proposed direction

Choose an explicit local declaration form or make the existing restriction a deliberate and prominently documented rule. Do not alter shadowing incidentally during refactoring.

## Acceptance criteria

A written decision covers parameters, locals, builtins, blocks, and outer names. Diagnostics explain how to introduce a local under the selected design.

## Pico 2 implications and dependencies

Pico 2: name resolution and local slots can reduce lookup costs, but must implement the same rules.

## Relevant code

- [src/runtime/env.rs](../src/runtime/env.rs)
- [src/runtime/eval.rs:1306](../src/runtime/eval.rs#L1306)

## Scope

This task records a finding or proposal, not an approved implementation. Resolve open semantic choices during individual review. Keep implementation changes and commits separate from this planning document.
