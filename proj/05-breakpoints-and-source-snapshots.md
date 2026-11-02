# 05 — Track breakpoints and identify the source being debugged

Priority: high. Integrate with the document and execution controllers in item 10.

## Problem and evidence

`Doc` in `aula/ui/src/docs.rs` stores breakpoint line numbers in a `BTreeSet<u32>`
and separately paints GTK source marks. Buffer marks can move after edits, while
the stored numbers used by execution and session persistence are not updated.

Execution uses a source snapshot, but documents remain editable. Debugger positions
can therefore refer to an earlier version of the code visible in a tab.

## Proposed changes

- Store breakpoint identity using persistent buffer marks or equivalent anchors.
- Derive current line numbers from anchors when launching, persisting, or sending
  breakpoint updates. Choose explicit behavior for deleting a breakpoint's line.
- Associate each run with immutable source snapshots and document revisions.
- Show when the visible document differs from the running source, and provide a
  way to inspect that source. Define breakpoint behavior during such divergence.
- Preserve per-file identity across imports, renames, and platform path separators.
- Indicate breakpoints that cannot bind to an executable statement.

## Acceptance criteria

- Inserting or deleting lines before a breakpoint moves its execution location
  consistently with the visual marker.
- Undo/redo and multiline replacement have defined, tested breakpoint behavior.
- Session restoration uses the latest anchored positions.
- Debugging imported files uses the correct file and source revision.
- Editing during a run does not silently present stale debugger positions as current.

## Verification

Add GTK buffer tests for anchored edits and debugger integration tests that verify
the actual paused statement, rather than only checking the painted marker.

## Implementation ready for review

Implemented in the working tree; no commit created.

- Persistent GTK text marks are the authoritative breakpoint positions. Inserting
  lines at a breakpoint follows the original text. Deleting a breakpoint's text
  moves it to the next surviving position, or EOF. Undo/redo follows that surviving
  position; it does not restore the identity of deleted text. Coincident points
  appear and persist once; toggling removes all points at that position.
- Session saving derives line numbers from those anchors. Launch captures sources,
  entry identity and breakpoint positions before asynchronous preparation, so edits
  or renames during preparation cannot mismatch the running source and its points.
- Each run uses fixed sources and its initial breakpoint set. Changes to source or
  breakpoints affect the next run; there is no hot reload or live breakpoint update.
- Pauses and stack navigation open a separate read-only execution-source tab.
  Editable tabs show a notice and a button to inspect their running source. Source
  revisions and the original editor identity survive edits and renames. The source
  tab is excluded from saves, recovery and project session persistence.
- The runtime hook reports loaded source before a pause, including imported or
  generated modules. The view does not reread mutable workspace files to explain
  existing execution positions. Completed sources remain inspectable until the
  next successful preparation or project switch.
- Lines without a parsed executable statement use a hollow gray breakpoint marker
  and a tooltip. Launch reports inactive points and omits them from the debugger.
  A point on an executable statement is eligible to bind; this does not imply that
  control flow will reach it.

Regression coverage includes anchored insertion/deletion, multiline replacement,
undo/redo, session round trips, unavailable statement locations, read-only source
views, immutable captures, and an actual pause inside an imported function. That
integration test edits and renames the tab during the run, verifies the original
statement and variable value, then verifies that a subsequent run uses the edits.

Verification passed on Linux with desktop access:

- `cargo test -p expressa-aula --quiet`: 124 passed, 1 ignored subprocess helper.
- The REPL layout regression remains passing.
- `cargo clippy --workspace --all-targets -- -D warnings`.
- `cargo build --workspace`.
- `git diff --check`.

Native Windows verification and a manual debugger walkthrough remain review checks.
