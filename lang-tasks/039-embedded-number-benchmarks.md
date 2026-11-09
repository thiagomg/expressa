# 039 — Validate numeric representation on the Pico 2

Status: Proposed; awaiting individual review.

Priority/type: Pico 2 · measurement and decision.

## Problem

Decimal storage and arithmetic, f64 intermediates, matrices, and parsing have different code-size, memory, and execution costs. The desktop model already has inconsistencies and should not be replaced by an unmeasured board-specific approximation.

## Proposed direction

Benchmark decimal operations, conversion, transcendental functions, formatting, and representative matrix workloads on the selected target. Confirm dependency portability. Decide whether to retain decimal semantics, restrict optional math libraries, or introduce an explicit documented alternative profile.

## Acceptance criteria

Report firmware size, per-value/cell memory, numeric latency, and semantic differences if any. Common programs agree across targets. No silent decimal-to-f32/f64 substitution is introduced.

## Pico 2 implications and dependencies

Informs 002, 003, 004, 010, 020, and 022 before large representation changes.

## Relevant code

- [src/runtime/numero.rs](../src/runtime/numero.rs)
- [src/runtime/nativas/mat.rs](../src/runtime/nativas/mat.rs)
- [src/runtime/nativas/matriz.rs](../src/runtime/nativas/matriz.rs)
- [Cargo.toml](../Cargo.toml)

## Scope

This task records a finding or proposal, not an approved implementation. Resolve open semantic choices during individual review. Keep implementation changes and commits separate from this planning document.
