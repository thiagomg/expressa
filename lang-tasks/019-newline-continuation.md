# 019 — Define consistent statement continuation across newlines

Status: Partially implemented for review: same-line indexing opener; broader continuation rules remain proposed.

Priority/type: High · reproduced surprise / design decision.

## Problem

Postfix calls crossed newlines, as did indexing before this fix, while map fields had a same-line restriction. A separate-looking list expression could become indexing of the previous call result.

## Implemented correction

The reported `exemplos/tarefas/persiste.lep` failure reproduced this ambiguity: `dt = serialize_data(tarefa:data)` followed by `[c, dt, tarefa:desc]` was parsed as indexing of the call result. The indexing opener `[` now must be on the same line as the end of its receiver. The next-line list is a separate expression. Index/slice/matrix contents and call arguments may still span lines. The parser checks the source gap between tokens, preserving indexing immediately after a multiline string and recognizing line breaks inside comments.

Regression tests cover the parser structure, comments, multiline contents and the returned serialization list in both AST and bytecode. The language specification records the boundary. Existing source that deliberately put the indexing opener on the next line must move `[` to the receiver's line. Postfix calls, module access and UFCS continuation are not changed by this correction; their broader consistency remains open.

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

The user authorized fixing the reported parser bug. The narrow indexing correction is implemented for review without an automatic commit. Resolve remaining continuation choices during individual review.
