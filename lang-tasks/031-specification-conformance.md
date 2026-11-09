# 031 — Strengthen and organize the language specification

Status: Partially addressed for AST review; awaiting user review.

Priority/type: Documentation · proposal.

## Implementation record

Known contradictions/placeholders/stale references corrected; full conformance reference and library separation remain pending. See [AST-REVIEW.md](AST-REVIEW.md) for scope, compatibility, and verification. No commits or bytecode implementation are included.

## Problem

Core semantics and a large native catalog are mixed together. Precedence, evaluation order, copying, equality, numeric limits, and scope rules need a stronger normative reference. There are placeholders and stale references.

## Proposed direction

Separate normative language rules from library reference material. Add tables and executable examples; remove yyy1 and resolve the missing ajustes/03-estrutura-do-interpretador.md reference.

## Acceptance criteria

Specification covers precedence/associativity, evaluation order, scopes, mutation, errors, equality, numeric limits, and target profiles. Examples agree with tests and tutorials.

## Pico 2 implications and dependencies

Pico 2: explicitly distinguish common language semantics, optional libraries, and host resource limits.

## Relevant code

- [especificacao-linguagem.md](../especificacao-linguagem.md)
- [Cargo.toml](../Cargo.toml)

## Scope

This task records a finding or proposal, not an approved implementation. Resolve open semantic choices during individual review. Keep implementation changes and commits separate from this planning document.
