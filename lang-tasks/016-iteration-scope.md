# 016 — Define loop scope and captured iteration bindings

Status: Proposed; awaiting individual review.

Priority/type: Decision · existing tests confirm behavior.

## Problem

Loop bodies reuse one scope across iterations. Locals can persist and closures share the changing loop variable; an existing test expects the final loop value.

## Proposed direction

Choose persistent loop scope or fresh per-iteration bindings. Fresh captured iteration bindings are generally easier for beginners, but compatibility and allocation costs must be assessed.

## Acceptance criteria

Document local lifetime and closure behavior for every loop form. Tests cover closures retained from multiple iterations and variables created conditionally in a loop.

## Pico 2 implications and dependencies

Pico 2: fresh bindings need not imply unbounded allocation; coordinate with slots/frames and heap design.

## Relevant code

- [src/runtime/eval.rs:756](../src/runtime/eval.rs#L756)
- [src/runtime/eval.rs:2847](../src/runtime/eval.rs#L2847)

## Scope

This task records a finding or proposal, not an approved implementation. Resolve open semantic choices during individual review. Keep implementation changes and commits separate from this planning document.
