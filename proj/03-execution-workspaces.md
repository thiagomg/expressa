# 03 — Unique workspaces and durable pending uploads

Priority: high. Depends on a stable project identity; integrate with item 02.

## Problem and evidence

`Files::cache_key` in `aula/ui/src/store.rs` uses folder names locally and
student/project names remotely. It omits the full local path or server address.
`cache_name` in `exec.rs` replaces punctuation with underscores, introducing
additional collisions. A fresh workspace may delete an existing cache directory.

`Workspace::upload_changes` reports failures but leaves the output in a disposable
execution copy. A later synchronization can replace modified files or remove new
files that were never successfully uploaded.

## Proposed changes

- Introduce structured project identity: canonical local directory, or normalized
  server endpoint plus student and project. Keep display names separate.
- Derive filesystem-safe cache keys from the complete identity using a stable hash.
- Give each run a separate directory, or establish explicit ownership and locking
  for any reusable copy. Never delete another active run's workspace.
- Persist failed uploads under durable application data with their project identity,
  original revision, destination, content, and failure reason.
- Provide retry, export, and explicit discard actions across application restarts.
- Use item 02's expected revisions to prevent delayed retries overwriting newer work.
- Clean up run copies only after their output is accounted for. Establish a bounded
  retention policy that does not silently discard pending student work.

## Acceptance criteria

- Identically named folders in different locations use different run directories.
- Matching student/project names on different servers do not share state.
- Punctuation differences do not collapse distinct project identities.
- An upload failure survives another run and an application restart.
- Retrying against a changed destination reports a conflict.
- Cleanup does not remove active workspaces or pending uploads.

## Verification

Use temporary roots and a fake file repository with injected upload failures.
Test concurrent workspace preparation and restart recovery. Avoid the real user cache.

## Implementation ready for review

Item 03 is implemented in the working tree, without a Git commit.

- `identidade.rs` uses a canonical local directory, or the normalized endpoint,
  student and project. Structured serialization plus SHA-256 supplies a safe key;
  punctuation, field boundaries, server ports and non-root URL paths remain distinct.
  Endpoint host case and explicit default ports normalize consistently. Credentials
  and queries are rejected in server URLs. Non-UTF-8 local paths fail explicitly.
- `exec.rs` creates a unique run directory under application data for every execution.
  It recopies the current project and then overlays editor buffers, preserving their
  original repository revisions. The old shared cache and incremental reuse are gone.
  A later execution cannot overwrite or delete another run's output.
- `pendencias.rs` stores a versioned journal outside the interpreter's filesystem
  root, alongside its `files/` directory. Baseline content hashes, expected revisions,
  destination paths, origin and upload failure reasons survive restart. Journal writes
  use the atomic storage layer. Prepared inputs are flushed before a durable ready
  marker permits execution; an interrupted preparation cannot become uploadable
  partial source. SHA-256 scanning and exports stream files rather than
  loading all pending output into memory; individual upload payloads still use the
  existing whole-file repository API.
- Each run holds an OS lock. The interpreter worker retains shared ownership even
  if the window drops its run handle. Other windows/processes skip live runs during
  discovery, export, retry and cleanup. Invalid paths and symbolic links are refused.
- Successful uploads are acknowledged individually. Failed files remain pending
  while unrelated files proceed. If a response/journal acknowledgement was lost,
  an identical destination accounts for the output without writing it again.
  Otherwise retry uses the original expectation; it never reads a newer revision
  to authorize overwriting a newer destination.
- **Arquivo → Sincronização… → Arquivos ainda não enviados ao servidor** shows
  origins, destinations and failure reasons for the currently open remote project.
  The synchronization interface is hidden for local projects; local write failures
  receive contextual retry/export dialogs instead.
  Retry requires opening the original project. Export creates an exclusive copy
  plus a journal sidecar, leaving the pending run intact. Export cannot target the
  retained run itself. Explicit discard confirms before deleting a retained run.
  Damaged/unknown journals are retained and can be exported as raw records.
  Startup reports pending/unreadable runs without connecting to their servers.
- Successful or empty inactive runs are removed. Pending work has no automatic
  expiry or eviction. Admission refuses new runs at 100 retained run records or
  512 MiB of retained data that can be safely inspected; it does not inspect live
  mutable trees. A running program can exceed that admission threshold. Users must
  retry/export and explicitly discard pending output to release retained storage.

## Verification and limits

Local verification passed after the interface follow-up: 107 IDE tests, 22 server tests and 7 storage tests.
The ignored IDE/storage helpers were explicitly run as child processes by their
parent tests. Clippy passed for those crates and all targets with warnings denied;
the offline workspace check, targeted formatting and whitespace checks passed.

Regression coverage includes identically named local folders, canonical aliases,
server/port and punctuation distinctions, concurrent preparations, leases checked
from another process, a worker lease outliving its window, failure injection,
partial success, crash/restart discovery, lost acknowledgements, stale-revision and
new-file conflicts on delayed retries, exact binary exports, malformed journal
exports, export destination isolation, and retention admission without eviction.
The ignored lease test is a subprocess helper invoked by the cross-process test.
Tests use temporary data roots and injected repository failures, not the user cache.

Native Windows execution and manual interaction with the new dialogs/folder chooser
remain review checks. File synchronization follows platform support; tests simulate
process interruption and publication failures, not an OS power cut. Arbitrary
external writers that ignore ownership locks retain the filesystem race limitations
from item 02. The lease journal is not a hostile-process security boundary.

Binary RPC download/upload is item 04. File deletions are not propagated to
the repository. Legacy cache folders have no persisted baseline
or reliable origin; they are left untouched and must be inspected/exported manually.
Session/unsaved-buffer recovery retains its existing format and locations. Performance
work on background storage/analysis remains item 06; fresh full copies are intentional
here to keep execution ownership and recovery explicit.


## Interface review follow-up

The file menu now has one remote-only **Sincronização…** window, combining generated
files not saved to the server and locally preserved editor text. Discovery is scoped
to the current project; old records from other projects/servers remain on disk but
are not mixed into its lists. Local projects keep automatic backup and contextual
failure recovery without a synchronization interface. Remote editor-copy recovery
is reviewed in that window rather than opening an automatic dialog on project load.
The REPL layout also protects the lower pane's minimum height so its input cannot
be allocated beneath the status bar; a real GTK allocation regression reproduces
the old overlap and verifies the corrected bounds.

Verification of the interface follow-up includes project-scoped discovery (another
server's or a local project's records are not exposed or cleaned) and a GTK layout
regression that first failed with the REPL input below the status bar, then passed
with minimum lower-pane sizing enforced. The full IDE/server/storage suite, Clippy
and workspace build checks passed. The remote synchronization dialog's interactive
workflow and native Windows presentation remain manual review checks.
