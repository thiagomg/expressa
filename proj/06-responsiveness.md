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

## Implementation ready for review

Implemented in the working tree; no commit created.

### Storage inventory and ownership

| Path | Execution and completion policy |
| --- | --- |
| Open, reload, conflict comparison, imported-file reads | Bounded I/O pool; immutable file/project inputs; project and operation identity checked before opening or replacing a buffer |
| Save, Save As, explicit overwrite | Capture text and expected storage revision on GTK; write on a worker; apply the receipt only to the same document identity; newer edits remain dirty |
| Create, rename, delete | Bounded I/O pool; project/path guards; edits made while deletion is pending remain in their tabs |
| Session restoration and recovery review/export | Read/scan on workers; create GTK tabs only on the main loop; restoration continues after each asynchronous read |
| Preferences, session snapshots, recovery copies and acknowledgement | Ordered, bounded persistence worker; durable maintenance can finish after its UI observer disappears |
| Tree listing, project search, execution preparation and output delivery | Existing background paths now use the shared bounded pool rather than creating a worker for every request |
| Syntax diagnostics, executable-line and highlight indexes | Separate analysis pool; results belong to a monotonic buffer revision; cursor movement queries cached character ranges |
| Server filesystem requests | Semaphore admission followed by `spawn_blocking`; permits stay with running calls after a client disconnects |

- The I/O pool has four workers and 32 queued jobs. Analysis has two workers and
  16 queued jobs; persistence has one worker and 32 queued jobs. Queue overflow
  returns an error instead of creating more threads. Worker panics become errors
  and do not destroy the pool's remaining capacity. Analysis retries bounded
  admission failures after its debounce interval.
- Project epochs reject results after navigation; path operation IDs reject
  superseded opens and invalidated rename/delete results. Save requests capture
  the document's edit and identity epochs. A receipt advances the saved storage
  revision, but clears the dirty flag only for the exact submitted edit revision.
  Save As also detects a destination opened in another tab before its receipt.
- Closing/switching continuations save documents asynchronously in sequence and
  stop on failure, cancellation or newly unsaved work. They cannot close the
  window merely because an earlier version finished saving.
- External-file reads are asynchronous and coalesced by document operation
  identity. Notifications arriving during a save are checked after its receipt;
  they cannot reload over newer edits. File monitors, analysis/highlight timers
  and recovery timers are stopped when their owner closes.
- Recovery ownership also lives outside GTK callbacks. The persistence worker
  records its latest snapshot, and atomic saved/closed acknowledgements retire
  only acknowledged revisions. Closing or saving before a callback arrives cannot
  resurrect obsolete recovery copies or erase a newer unsaved snapshot.
- The transient **Operações** control names active file operations and offers
  cancellation for reads, connections and execution preparation. Editing other
  documents remains available. Mutating operations are not offered as undoable
  cancellations. Dropping normal queued work prevents it from starting; durable
  maintenance is retained. RPC cancellation
  aborts the client's pending future. A running local OS call can finish normally.
- Preferences are loaded before entering GTK's event loop. Preference/session
  writes are ordered in the persistence queue. Tests inject non-persisting default
  preferences instead of reading/writing the real user's configuration. Project
  path labels use already validated identities without repeating filesystem I/O.

### Resource limits and verification

- Local text reads are limited to 8 MiB. Server text reads/writes are limited to
  3 MiB, below the default gRPC message limit. `storage::read_limited` checks both
  metadata and the actual bytes read, including a file growing during a read.
  Oversized writes are rejected before replacing a valid destination.
- Local/server tree listing limits entries to 20000 and depth to 64, and bounds
  directory collection before sorting. Server project listing and legacy layout
  preparation also bound directory collection. Server traversal skips symbolic
  links; complete classroom filesystem isolation remains item 08.
- The server admits at most four filesystem calls per instance and rejects excess
  requests as busy. Cancellation does not release an active call's permit early.
- GTK tests use gated workers to exercise typing during a delayed read, cancellation,
  project switches, overlapping opens, Save As identity collisions, and an older
  save followed by newer edits and another successful save. Tests cover stale
  syntax results, cached highlighting equivalence, recovery acknowledgement before
  GTK delivery, worker failures and shutdown timer cleanup.
- Menu keyboard tests now wait for layout frames, exclude unrelated desktop pointer
  targeting and sliding animation, and destroy their windows even after an assertion
  fails. Production menu navigation is unchanged by this item.
- [Recorded measurements](06-responsiveness-measurements.md) cover 500-, 5000- and
  50000-line GTK documents. They establish an initial component-level baseline;
  no machine-specific latency threshold was added to regression tests.

Validation: `cargo test --workspace --offline --locked`, the explicit GTK
measurement scenario, workspace Clippy with warnings denied, targeted Rust
formatting and `git diff --check`.

The Linux run passed 344 runtime, 145 IDE, 24 server, 8 storage and 4 tutorial
tests. Expected subprocess helpers are invoked by their parent tests; the ignored
measurement scenario was run explicitly. GTK assertions executed against a display.

Native Windows and manual desktop walkthroughs remain release checks. GTK buffer
snapshot/application work stays on GTK; completion and definition semantics remain
item 07. This change does not turn filesystem cancellation into transactional undo
or impose a memory quota on programs written by the student.
