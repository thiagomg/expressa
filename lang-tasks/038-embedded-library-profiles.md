# 038 — Define optional native libraries and board capabilities

Status: Proposed; awaiting individual review.

Priority/type: Pico 2 · foundational.

## Problem

Desktop natives include subprocesses, filesystem operations, windows, PNG handling, terminal queries, and wall-clock/calendar services. The board cannot inherit those capabilities from desktop APIs.

## Proposed direction

Define a minimal core profile and optional libraries. Exclude unavailable implementations at compile time. Add capability-aware registration and clear unavailable-module errors. Plan GPIO, ADC, PWM, I2C, SPI, and UART APIs as separate board-native capabilities with host-testable behavior.

## Acceptance criteria

The minimal image excludes desktop libraries and unused help. Supported modules are discoverable. Unsupported imports fail clearly. A small GPIO/LED example demonstrates the board extension without changing core syntax.

## Pico 2 implications and dependencies

Extends 028, 029, and 031. No Wi-Fi requirement is assumed for a plain Pico 2.

## Relevant code

- [Cargo.toml](../Cargo.toml)
- [src/runtime/nativas/mod.rs](../src/runtime/nativas/mod.rs)
- [src/runtime/nativas/referencia.rs](../src/runtime/nativas/referencia.rs)

## Scope

This task records a finding or proposal, not an approved implementation. Resolve open semantic choices during individual review. Keep implementation changes and commits separate from this planning document.
