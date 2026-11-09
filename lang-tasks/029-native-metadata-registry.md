# 029 — Centralize native function metadata

Status: Proposed; awaiting individual review.

Priority/type: Organization · proposal.

## Problem

Names, aliases, module membership, dispatch, signatures, and help are maintained in multiple places, increasing drift risk.

## Proposed direction

Build one descriptor source containing qualified identity, aliases, callable, signatures/arity, documentation reference, and capability requirements.

## Acceptance criteria

Registration, dispatch, introspection, and help agree. Automated validation detects missing or duplicated entries. Documentation can be excluded from a firmware image without breaking dispatch.

## Pico 2 implications and dependencies

Pico 2: descriptors should support feature-based omission of help, modules, and unavailable capabilities.

## Relevant code

- [src/runtime/nativas/mod.rs:30](../src/runtime/nativas/mod.rs#L30)
- [src/runtime/nativas/referencia.rs](../src/runtime/nativas/referencia.rs)

## Scope

This task records a finding or proposal, not an approved implementation. Resolve open semantic choices during individual review. Keep implementation changes and commits separate from this planning document.
