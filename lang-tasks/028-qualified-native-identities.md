# 028 — Give native functions module-qualified identities

Status: Proposed; awaiting individual review.

Priority/type: Organization · semantic concern.

## Problem

Builtin values carry unqualified string names. Shared names such as adicione dispatch by arity and argument shape rather than distinct module identities.

## Proposed direction

Introduce stable NativeId values distinguishing functions such as crono::adicione and cartesiano::adicione. Define equality and alias behavior deliberately.

## Acceptance criteria

Qualified natives dispatch to their own implementation and report their own signatures. Aliases retain intended identity. Tests cover equal names across modules.

## Pico 2 implications and dependencies

Pico 2: compact IDs can reduce string dispatch and allow unused native implementations to be excluded.

## Relevant code

- [src/runtime/value.rs:29](../src/runtime/value.rs#L29)
- [src/runtime/nativas/mod.rs:590](../src/runtime/nativas/mod.rs#L590)

## Scope

This task records a finding or proposal, not an approved implementation. Resolve open semantic choices during individual review. Keep implementation changes and commits separate from this planning document.
