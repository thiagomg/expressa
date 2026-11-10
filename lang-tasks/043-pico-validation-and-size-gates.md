# 043 — Add Pico 2 conformance, resource, and hardware validation

Status: Desktop conformance and CPU/RSS comparison implemented for review; Pico hardware and firmware validation pending.

Priority/type: Pico 2 · validation.

## Problem

Desktop tests do not establish bare-metal compatibility, memory fit, latency, or recoverability. Successful cross-compilation alone does not prove the language works on the board.

## Review gate

The user approved the bytecode migration after committing the AST improvements as `bd654b5`. Desktop conformance and performance results are documented in [BYTECODE-REVIEW.md](BYTECODE-REVIEW.md) and [the comparison report](../benchmarks/AST-VS-BYTECODE.md). No Pico SRAM, flash, latency, or firmware result is inferred from desktop RSS.

## Proposed direction

Create shared semantic tests for host and embedded engine, cross-build checks, firmware RAM/flash reporting, and a hardware smoke suite. Include long-running allocation/cycle tests, malformed input, numeric limits, cancellation, and repeated sessions.

## Acceptance criteria

Validate shared bytecode behavior on desktop and Pico, including runtime compilation by `avalie`, updates and newly introduced bindings visible to already compiled code, escaping closures, and accurate source diagnostics. Use the existing evaluator as a temporary semantic reference where appropriate.

Record verified board/toolchain and actual memory/size results. Set evidence-based limits with headroom. Hardware-only checks are documented when unavailable in CI. Desktop regression coverage remains intact.

## Pico 2 implications and dependencies

Extends 033. Benchmarks and a small end-to-end prototype should precede a broad portability rewrite.

## Relevant code

- [fuzz/README.md](../fuzz/README.md)
- [src/runtime/eval.rs](../src/runtime/eval.rs)
- [Cargo.toml](../Cargo.toml)

## Scope

This task records a finding or proposal, not an approved implementation. Resolve open semantic choices during individual review. Keep implementation changes and commits separate from this planning document.
