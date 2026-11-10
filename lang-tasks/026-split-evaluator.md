# 026 — Split evaluator responsibilities into cohesive modules

Status: Partially addressed for AST review; awaiting user review.

Priority/type: Organization · proposal.

## Implementation record

Assignment/matrix operations and evaluator tests extracted; broader organization remains pending. See [AST-REVIEW.md](AST-REVIEW.md) for scope, compatibility, and verification. No commits or bytecode implementation are included.

## Problem

eval.rs combines execution wrappers, VM construction, imports, statements, expressions, operators, indexing, assignment, diagnostics, and a large test suite.

## Proposed direction

Extract cohesive modules while retaining one shared VM and preserving behavior. Establish portable core boundaries before desktop-only organization work.

## Acceptance criteria

Modules have clear responsibilities and limited internal visibility. Existing behavior tests pass and public embedding APIs remain deliberate. No semantic changes are hidden in file moves.

## Pico 2 implications and dependencies

Pico 2: prioritize portable engine versus desktop host separation over cosmetic file-size reduction; task 035.

## Relevant code

- [src/runtime/eval.rs](../src/runtime/eval.rs)

## Scope

This task records a finding or proposal, not an approved implementation. Resolve open semantic choices during individual review. Keep implementation changes and commits separate from this planning document.
