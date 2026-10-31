# 06 — Keep storage and analysis off the GTK thread

Priority: high. Use the ownership boundaries introduced incrementally in item 10.

## Problem and evidence

`open_path` and `save_doc` in `aula/ui/src/main.rs` call storage synchronously.
`aula/ui/src/rpc.rs` uses `block_on` and permits requests to take ten seconds.
Other file operations and session restoration can issue multiple blocking calls.

`Doc::check_now` parses the complete document on the GTK thread. Block matching
also reads and scans the full document on cursor movement. Debouncing syntax
checks reduces frequency but does not remove their blocking cost.

## Proposed changes

- Inventory storage entry points and move network/disk work to a bounded worker
  mechanism, including open/save, rename, imported-file reads, and restoration.
- Capture immutable input snapshots; apply results on the GTK thread only.
- Attach project identity, operation identity, and document revision to results.
  Ignore stale results and avoid marking newer edits as saved.
- Provide cancellation and operation-specific pending states without globally
  disabling unrelated editing.
- Run syntax analysis in workers. Cache lexical/block data by revision and avoid
  rescanning unchanged documents for every cursor move.
- Move blocking server filesystem work out of Tokio async request workers, using
  bounded concurrency. Bound traversal and transfer workloads.
- Measure typing latency and operation responsiveness on representative lesson
  projects and large inputs before setting performance budgets.

## Acceptance criteria

- A delayed server does not prevent typing, repainting, or cancelling an operation.
- Completion of an old save does not clear the dirty flag on newer edits.
- Switching projects cannot apply an earlier project's result to the new one.
- Stale syntax results never replace diagnostics for the current revision.
- Worker count and queued work stay bounded under repeated actions.

## Verification

Use delayed fake repositories and deterministic completion ordering. Add focused
GTK responsiveness checks and measured large-document scenarios; avoid brittle
microsecond timing assertions in ordinary unit tests.
