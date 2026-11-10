# 018 — Describe expression forms and decide empty-block behavior

Status: Partially addressed for AST review; awaiting user review.

Priority/type: Decision · grammar mismatch.

## Implementation record

Actual expression forms, block results, and nonempty-block behavior documented; extensions remain pending. See [AST-REVIEW.md](AST-REVIEW.md) for scope, compatibility, and verification. No commits or bytecode implementation are included.

## Problem

The claim that everything is an expression overstates the grammar: assignments and loops are statements, imports have restricted positions, and empty blocks are rejected.

## Proposed direction

Describe actual expression forms. Decide whether empty blocks and functions should yield nada and whether a direct nada literal is needed.

## Acceptance criteria

The specification accurately classifies constructs. Empty functions, empty branches, statement-only blocks, and block results have explicit outcomes and examples.

## Pico 2 implications and dependencies

Pico 2: make these decisions before creating an alternative execution representation.

## Relevant code

- [src/parser/parser.rs:375](../src/parser/parser.rs#L375)
- [src/lexer/tokens.rs](../src/lexer/tokens.rs)
- [especificacao-linguagem.md](../especificacao-linguagem.md)

## Scope

This task records a finding or proposal, not an approved implementation. Resolve open semantic choices during individual review. Keep implementation changes and commits separate from this planning document.
