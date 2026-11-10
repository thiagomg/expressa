# 035 — Separate a portable language core from desktop hosts

Status: Proposed; awaiting individual review.

Priority/type: Pico 2 · foundational.

## Problem

The current interpreter uses std throughout and unconditionally includes desktop I/O, processes, windows, paths, threads, and terminal support. Merely cross-compiling the current binary is insufficient.

## Proposed direction

Extract a shared parser/compiler and bytecode VM for desktop and Pico 2 (task 036), without a JIT. Make the portable core no_std-compatible using alloc where needed, or validate an equivalent supported firmware runtime. Put desktop services behind optional features and host traits for input/output, clock, storage, cancellation, and native capabilities.

## Acceptance criteria

A minimal embedded core builds without desktop dependencies. Desktop CLI/Aula behavior remains covered. A dependency/feature matrix identifies what is included in each target.

## Pico 2 implications and dependencies

Extends 026, 027, 029, and 032. Do not assume that disabling window creation removes compile-time desktop dependencies.

## Relevant code

- [Cargo.toml](../Cargo.toml)
- [src/runtime/mod.rs](../src/runtime/mod.rs)
- [src/runtime/eval.rs](../src/runtime/eval.rs)

## Scope

This task records a finding or proposal, not an approved implementation. Resolve open semantic choices during individual review. Keep implementation changes and commits separate from this planning document.
