# 07 — Share scope-aware language analysis across editors

Priority: medium; correctness foundation for future language tooling.

## Problem and evidence

`aula/ui/src/lang.rs` infers top-level definitions from zero indentation and keeps
the first definition for each name. This does not model brace-based scope,
shadowing, function parameters, or arbitrary multiline definitions accurately.
`editors/vscode/src/analise.js` implements a separate text-analysis system.

The GTK integration already derives native-function documentation from interpreter
metadata. Preserve that single source of truth.

## Proposed changes

- Define a UI-independent Rust analysis API for document snapshots, source spans,
  symbols, lexical scopes, imports, definitions, and diagnostics.
- Support incomplete code explicitly. Do not assume the execution parser's
  first-error result is sufficient for interactive editing.
- Reuse the language lexer and grammar where practical; introduce recovery or an
  editor syntax representation without changing accepted language semantics.
- Resolve names at a cursor position using actual scope and binding rules.
- Model project files as versioned snapshots, with unsaved open documents taking
  precedence over disk or remote content.
- Define conversions among byte offsets, GTK character offsets, and LSP positions.
- Move GTK consumers to the analysis library incrementally. Expose the same analysis
  through an LSP adapter for VS Code after the core behavior is stable.
- Continue generating native metadata and highlighting data from authoritative inputs.

## Acceptance criteria

- Shadowed names navigate to the binding visible at the cursor.
- Parameters and multiline definitions appear in the correct scope.
- Formatting changes do not change whether a definition is top-level.
- Incomplete code still supplies useful nearby symbols and diagnostics.
- Unicode spans agree across interpreter, GTK editor, and protocol adapters.
- Shared fixtures demonstrate consistent behavior across editor integrations.

## Verification

Build syntax and scope fixtures covering braces, `inicio`/`fim`, imports, accented
identifiers, malformed code, and unsaved imported modules. Benchmark before committing
to a particular incremental parsing strategy.
