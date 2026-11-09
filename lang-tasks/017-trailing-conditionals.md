# 017 — Resolve conflicting trailing-conditional rules

Status: Partially addressed for AST review; awaiting user review.

Priority/type: Decision · specification conflict.

## Implementation record

Specification aligned with the current trailing-conditional behavior; changing that rule remains a decision. See [AST-REVIEW.md](AST-REVIEW.md) for scope, compatibility, and verification. No commits or bytecode implementation are included.

## Problem

Specification section 6 allows trailing se without senao in a function, while section 8 says a final value requires senao. The implementation permits it and returns nada when no branch matches.

## Reproduction / example

```text
f = funcao() { se falso { 1 } }
escreva(f())
```

## Proposed direction

Choose a single rule for function tails and distinguish statement/effect contexts from value contexts consistently.

## Acceptance criteria

Specification, parser validation, tutorials, and examples agree. Cover function tails, explicit retorne, assigned blocks, nested conditionals, and missing branches.

## Pico 2 implications and dependencies

Pico 2: the rule belongs to the shared language definition.

## Relevant code

- [src/parser/se_value.rs:121](../src/parser/se_value.rs#L121)
- [especificacao-linguagem.md](../especificacao-linguagem.md)

## Scope

This task records a finding or proposal, not an approved implementation. Resolve open semantic choices during individual review. Keep implementation changes and commits separate from this planning document.
