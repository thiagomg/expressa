# 033 — Add regression and conformance coverage at semantic boundaries

Status: Partially addressed for AST review; awaiting user review.

Priority/type: Validation · proposal.

## Implementation record

AST regression coverage and benchmark workloads added; cross-backend conformance/fuzz expansion remains pending. See [AST-REVIEW.md](AST-REVIEW.md) for scope, compatibility, and verification. No commits or bytecode implementation are included.

## Problem

355 library tests passed during review, but focused programs found failures. Random syntactic fuzzing is unlikely to cover numeric extremes and side-effecting destinations adequately.

## Proposed direction

Add regressions as individual fixes land, then cross-context and property checks for numerical, assignment, scope, import, and lifecycle contracts. Seed fuzzing with boundary programs.

## Acceptance criteria

Coverage includes debug/release range endpoints, failed conversions, scalar/matrix values, compound destinations, nested writeback, import state, diagnostics, and script/REPL parity. Keep each test attached to a documented contract.

## Pico 2 implications and dependencies

Pico 2: add a shared conformance subset plus board resource/lifecycle tests; task 043.

## Relevant code

- [src/runtime/eval.rs](../src/runtime/eval.rs)
- [src/runtime/numero.rs](../src/runtime/numero.rs)
- [fuzz/README.md](../fuzz/README.md)

## Scope

This task records a finding or proposal, not an approved implementation. Resolve open semantic choices during individual review. Keep implementation changes and commits separate from this planning document.
