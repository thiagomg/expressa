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
