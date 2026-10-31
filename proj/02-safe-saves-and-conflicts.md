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
