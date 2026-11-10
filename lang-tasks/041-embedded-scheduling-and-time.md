# 041 — Define cooperative scheduling, time, and cancellation on board

Status: Proposed; awaiting individual review.

Priority/type: Pico 2 · runtime integration.

## Problem

Desktop Instant, thread sleep, blocking input, and cancellation channels do not directly supply a board scheduler. Long interpreter operations must allow communication and cancellation to progress.

## Proposed direction

Provide monotonic time, yield/poll, nonblocking input, sleep, and stop hooks. Define instruction/work budgets and watchdog integration. Keep user callbacks out of interrupt handlers; queue hardware events for interpreter context.

## Acceptance criteria

An infinite language loop can be stopped over the selected transport. Long waits and computations keep communication responsive. Clock wraparound and deadline behavior are specified and tested.

## Pico 2 implications and dependencies

Extends 012, 025, and 030. Measure latency; do not promise hard real-time behavior from a general interpreter.

## Relevant code

- [src/runtime/eval.rs:1878](../src/runtime/eval.rs#L1878)
- [src/runtime/eval.rs:1894](../src/runtime/eval.rs#L1894)
- [src/runtime/nativas/nucleo.rs:80](../src/runtime/nativas/nucleo.rs#L80)
- [src/runtime/repl/canal.rs](../src/runtime/repl/canal.rs)

## Scope

This task records a finding or proposal, not an approved implementation. Resolve open semantic choices during individual review. Keep implementation changes and commits separate from this planning document.
