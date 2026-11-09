# 019 — Define consistent statement continuation across newlines

Status: Proposed; awaiting individual review.

Priority/type: High · reproduced surprise / design decision.

## Problem

Postfix calls and indexing cross newlines, while map fields have a same-line restriction. A separate-looking list expression can become indexing of the previous call result.

## Reproduction / example

```text
formato("pt")
[1.5]
```

## Proposed direction

Specify when newlines terminate statements and when continuation is allowed. Evaluate deliberate continuation syntax or consistent same-line postfix rules without breaking multiline expressions.

## Acceptance criteria

The example has an intentional documented interpretation with useful diagnostics. Tests cover calls, indexing, fields, operators, comments, and multiline argument/list forms.

## Pico 2 implications and dependencies

Pico 2: serial REPL completeness must use the same continuation rules as files.

## Relevant code

- [src/parser/parser.rs:563](../src/parser/parser.rs#L563)

## Scope

This task records a finding or proposal, not an approved implementation. Resolve open semantic choices during individual review. Keep implementation changes and commits separate from this planning document.
