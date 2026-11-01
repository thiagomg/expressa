# 02 — Atomic saves and conflicting edits

Priority: high. Coordinate the storage contract with items 03, 04, and 06.

## Problem and evidence

`aula/ui/src/store.rs` and `aula/server/src/service.rs` overwrite existing files
with `fs::write`. Interrupted writes can leave partial content. The protobuf save
request carries no expected revision, allowing stale editor content or execution
output to replace newer changes without detecting the conflict.

Local file monitoring warns about external edits but does not prevent a subsequent
save from overwriting them. Remote projects lack equivalent save-time protection.

## Proposed changes

- Introduce a shared save contract with a revision returned by reads and an
  expected revision supplied by updates.
- Write to a unique temporary file in the destination directory, flush as required,
  then atomically replace the destination using appropriate platform semantics.
- Preserve relevant permissions and clean up abandoned temporary files safely.
- Serialize revision checks and replacement on the server so two concurrent requests
  cannot both pass the same expected revision. Document limitations for external
  writers that bypass the server.
- Keep create-new operations exclusive and distinguish them from replacement.
- Offer compare, reload, save-as, and explicit overwrite on conflict; keep the tab dirty.
- Apply the same conflict policy to program-generated output.
- Define protocol compatibility: old clients must not silently bypass a deployment's
  required conflict protection.

## Acceptance criteria

- Injected failure before replacement leaves the original content intact.
- Two writes using the same revision cannot both replace the file successfully.
- A stale save preserves both the student's buffer and the newer stored content.
- A failed save never marks the tab as saved.
- Creation still fails when the destination already exists.
- Replacement behavior is verified on Linux and Windows.

## Verification

Use fault-injected storage tests, concurrent RPC tests, and document-controller
tests. Include permission errors, external modification, and conflict handling
when execution uploads a file that the user edited during the run.

## Implementation ready for review

Step 02 is implemented in commit `9289114`.

- The shared `expressa-aula-storage` crate provides content revisions, typed
  expectations/errors, same-directory staging, file synchronization, exclusive
  publication, atomic replacement, and a stable cross-process directory lock.
  Local mutations also coordinate with upload workers; server mutation requests
  are serialized. Ordinary failure paths clean temporary files; ambiguous crash
  leftovers/native original backups are retained rather than swept blindly.
- Reads return a revision derived from exactly the bytes read. Editor buffers
  retain that baseline through editing and failures. Successful reload/save updates
  it. Existing-file saves cannot silently recreate removed files/directories.
- Conflict dialogs use named response constants and preserve the buffer until an
  explicit resolution. Compare displays both versions; reload reads the latest;
  save-as creates a separate file exclusively; overwrite still compares the revision
  shown to the student. Late callbacks check document membership/name again.
- Saves clear recovery records only after publication. Failures preserve dirty state
  and recovery copies. Post-publication flush/cleanup warnings are reported separately
  from failed writes. Save-as restarts monitoring for the new path, and monitoring
  handles atomic replacement events.
- Replace-in-project uses read revisions for unopened files. Execution keeps repository
  revisions separate from editor overlays, including the editor's older revision if
  preparation read a newer destination. Text, binary, and new-output collisions are
  rejected without clobbering the project. Text conflicts become recovered dirty tabs;
  binary conflicts are retained in application data with their paths reported.
- Protocol reads/writes carry revisions. New clients use `WriteFileChecked`, so an
  older server cannot silently ignore an added request field. Legacy unconditional
  updates are rejected; legacy reads and exclusive creation remain supported. There
  is no unconditional force-overwrite RPC. Editor and server should be upgraded together.

## Verification and limits

Validated locally: 94 IDE tests, 22 server tests and 7 storage tests passed.
The ignored storage helper is invoked explicitly by the cross-process test.
Clippy passed for these crates and all targets with warnings denied; the offline
workspace check and `git diff --check` also passed before review.

Regression coverage includes failure injection before publication, competing threads
and processes, real concurrent RPC requests, legacy update rejection, unchanged-size
external edits, deletion, permission changes, save-as/reload, a second modification
after overwrite consent, and conflicts involving execution text/binary/new files.

Windows-specific tests check native replacement, explicit DACLs and named streams.
The added Linux/Windows CI workflow runs storage tests and lint without GTK/protobuf
system dependencies. Windows tests and CI cannot be executed on this Linux host;
their execution remains required before claiming native Windows verification.
Desktop conflict-dialog interaction also remains a manual review check.

The revisions are SHA-256 content identities: unchanged bytes retain the same revision.
Cooperating writers are serialized, but arbitrary external filesystem writers can
race the last check when they ignore the lock. Filesystem-level transactions, workspace
identity/pending-upload queues (03), binary RPC transfer (04), and background UI I/O
(06) are not introduced by this step. See `aula/storage/README.md` for storage semantics.
