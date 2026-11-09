# 027 — Replace long positional execution options with configuration

Status: Partially addressed for AST review; awaiting user review.

Priority/type: Organization · proposal.

## Implementation record

Named captured-execution configuration/output added while retaining existing wrappers; remaining host API migration pending. See [AST-REVIEW.md](AST-REVIEW.md) for scope, compatibility, and verification. No commits or bytecode implementation are included.

## Problem

Many run_* wrappers forward long argument lists with optional hosts, flags, and limits. This obscures defaults and makes embedding error-prone.

## Proposed direction

Introduce an explicit execution configuration and host bundle or builder. Preserve simple convenience entry points where useful.

## Acceptance criteria

Call sites name significant options. Defaults and lifecycle are documented. Desktop, tests, and embedded hosts configure capabilities without ambiguous positional arguments.

## Pico 2 implications and dependencies

Pico 2: options must avoid mandatory desktop host fields and OS-backed defaults.

## Relevant code

- [src/runtime/eval.rs:104](../src/runtime/eval.rs#L104)

## Scope

This task records a finding or proposal, not an approved implementation. Resolve open semantic choices during individual review. Keep implementation changes and commits separate from this planning document.
