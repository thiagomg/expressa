# 040 — Design program loading and module storage

Status: Proposed; awaiting individual review.

Priority/type: Pico 2 · architecture decision.

## Problem

The existing loader expects OS paths, canonicalization, and filesystem reads. The Pico 2 has finite on-board flash shared with firmware and no implied OS filesystem.

## Proposed direction

Choose program delivery over USB/UART and a flash storage/module format, or host-prepared program images. If programs are persisted, specify partition sizes, versioning, corruption recovery, update atomicity, and flash-write endurance considerations. Preserve on-board `avalie` through the shared runtime compiler and bytecode interpreter. Source and any optional host-prepared bytecode must enter the same VM. If a serialized bytecode format is exposed, define compatibility/version checks and validation of untrusted input; an internal bytecode representation does not itself require a persistent file format.

## Acceptance criteria

A program can be loaded and run using a documented workflow. Imports resolve deterministically. Firmware/program storage boundaries are enforced. Interrupted persistence cannot destroy the recovery path.

## Pico 2 implications and dependencies

Extends 008, 024, and 031. Coordinate with 034 and 036: on-board source compilation is the initial target, with feasibility subject to measured budgets and review.

## Relevant code

- [src/runtime/eval.rs:585](../src/runtime/eval.rs#L585)
- [src/runtime/nativas/nucleo.rs:509](../src/runtime/nativas/nucleo.rs#L509)
- [src/runtime/nativas/arquivo.rs](../src/runtime/nativas/arquivo.rs)

## Scope

This task records a finding or proposal, not an approved implementation. Resolve open semantic choices during individual review. Keep implementation changes and commits separate from this planning document.

## Desktop precursor implemented for review

[Task 047](047-exb-file-format-and-loader.md) implements the separate [EXB v1 format and validating desktop loader](../docs/exb.md). Source programs and EXB enter the same bytecode VM. File compatibility, bounds and structural checks are implemented; imports remain explicit files rather than a bundled image. Firmware delivery, flash partitions, update/recovery protocol and execution from flash remain pending. Desktop RSS is not a Pico SRAM measurement.
