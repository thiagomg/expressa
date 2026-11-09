# 032 — Clarify the supported embedding API and AST layout

Status: Partially addressed for AST review; awaiting user review.

Priority/type: Organization · proposal.

## Implementation record

Embedding facade and Rust API adaptations documented; compatibility modules retained pending API policy. See [AST-REVIEW.md](AST-REVIEW.md) for scope, compatibility, and verification. No commits or bytecode implementation are included.

## Problem

The empty parser::ast compatibility module and broadly public implementation modules obscure which interfaces are supported.

## Proposed direction

Define the embedding surface first. Consolidate, restrict, or deprecate compatibility scaffolding deliberately and organize AST types cohesively.

## Acceptance criteria

Supported APIs are documented. Internal modules have appropriate visibility. Downstream workspace users compile after any planned migration.

## Pico 2 implications and dependencies

Pico 2: portable AST/engine APIs must not expose desktop-only types or force std dependencies.

## Relevant code

- [src/parser/mod.rs:12](../src/parser/mod.rs#L12)
- [src/parser/ast.rs](../src/parser/ast.rs)
- [src/lib.rs](../src/lib.rs)

## Scope

This task records a finding or proposal, not an approved implementation. Resolve open semantic choices during individual review. Keep implementation changes and commits separate from this planning document.
