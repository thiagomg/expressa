# 042 — Build the Pico 2 firmware and minimal console integration

Status: Proposed; awaiting individual review.

Priority/type: Pico 2 · delivery.

## Problem

A portable engine still needs board startup, linker/memory layout, allocator policy, panic behavior, transport, flashing, and a console. rustyline and the threaded desktop REPL are not the embedded console.

## Proposed direction

After architecture decisions, add an isolated firmware target and reproducible build/flash workflow. Implement a bounded USB CDC or UART console with upload/run/stop/reset and useful diagnostics; use the portable parser completeness rules.

## Acceptance criteria

A clean build produces a flashable image. A documented hardware demo loads and executes Expressa, prints output, handles an error, stops a loop, and runs another program without rebooting.

## Pico 2 implications and dependencies

Depends on 034–041 as applicable. Keep firmware implementation out of the desktop CLI.

## Relevant code

- [Cargo.toml](../Cargo.toml)
- [src/runtime/repl.rs](../src/runtime/repl.rs)
- [src/runtime/repl/canal.rs](../src/runtime/repl/canal.rs)
- [src/parser/incomplete.rs](../src/parser/incomplete.rs)

## Scope

This task records a finding or proposal, not an approved implementation. Resolve open semantic choices during individual review. Keep implementation changes and commits separate from this planning document.
