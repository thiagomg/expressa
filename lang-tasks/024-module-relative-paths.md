# 024 — Define resource and import paths inside imported functions

Status: Proposed; awaiting individual review.

Priority/type: Decision · code finding.

## Problem

Calling an imported function changes diagnostic source identity but retains the current execution base directory. Resource lookup can depend on the calling context rather than the defining module.

## Proposed direction

Define separate origins for module imports and user data paths. Consider definition-relative module/resource lookup and an explicit working-directory host capability.

## Acceptance criteria

The same imported function behaves predictably when called during module initialization and later by different callers. Tests use nested modules with same-named resources.

## Pico 2 implications and dependencies

Pico 2: replace implicit filesystem assumptions with module IDs and storage capabilities; see task 040.

## Relevant code

- [src/runtime/eval.rs:1339](../src/runtime/eval.rs#L1339)

## Scope

This task records a finding or proposal, not an approved implementation. Resolve open semantic choices during individual review. Keep implementation changes and commits separate from this planning document.
