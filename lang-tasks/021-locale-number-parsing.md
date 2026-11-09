# 021 — Remove ambiguity between locale input and source numbers

Status: Proposed; awaiting individual review.

Priority/type: Decision · code finding.

## Problem

Portuguese number parsing accepts a dot as a decimal separator in some shapes but treats grouping-shaped input such as 1.234 as thousands. Pasting a source literal can change its value.

## Proposed direction

Prefer strict locale parsing and a separate source-number parser, or define an equally explicit alternative with no shape-dependent surprises.

## Acceptance criteria

A parsing table covers 1.234, 1,234, mixed separators, signs, spaces, and invalid grouping in both locales. Documentation explains source literals separately from user input.

## Pico 2 implications and dependencies

Pico 2: keep locale presentation independent from internal representation and program storage.

## Relevant code

- [src/runtime/value.rs:499](../src/runtime/value.rs#L499)

## Scope

This task records a finding or proposal, not an approved implementation. Resolve open semantic choices during individual review. Keep implementation changes and commits separate from this planning document.
