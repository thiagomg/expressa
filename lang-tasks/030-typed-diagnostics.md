# 030 — Use typed diagnostics and incomplete-input categories

Status: Implemented for AST review; awaiting user review.

Priority/type: Organization · proposal.

## Implementation record

Typed lexer/parser kinds drive completeness without inspecting messages. See [AST-REVIEW.md](AST-REVIEW.md) for scope, compatibility, and verification. No commits or bytecode implementation are included.

## Problem

REPL completeness partly depends on matching Portuguese error-message strings. Wording changes can therefore change behavior.

## Proposed direction

Introduce typed lexer/parser errors with incomplete-input classification. Keep human messages and source spans separate from programmatic kinds.

## Acceptance criteria

Changing a message does not alter completeness detection. Unterminated input, malformed complete input, and mismatched delimiters are classified consistently across REPL hosts.

## Pico 2 implications and dependencies

Pico 2: compact error codes and optional message rendering should share the same categories.

## Relevant code

- [src/parser/incomplete.rs:6](../src/parser/incomplete.rs#L6)
- [src/parser/error.rs](../src/parser/error.rs)
- [src/runtime/error.rs](../src/runtime/error.rs)

## Scope

This task records a finding or proposal, not an approved implementation. Resolve open semantic choices during individual review. Keep implementation changes and commits separate from this planning document.
