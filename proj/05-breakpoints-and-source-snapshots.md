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
