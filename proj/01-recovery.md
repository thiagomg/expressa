# 01 — Preserve unsaved work during recovery

Priority: critical. Start after the minimum document-controller extraction in
[step 00](00-minimal-controller-extraction.md); do not wait for the full item 10 refactor.

## Problem and evidence

`aula/ui/src/sessao.rs` recovers an untitled backup through `open_doc(None, texto)`.
`Doc::new` in `docs.rs` initializes its buffer as unmodified. Because recovery
compares the already-loaded text with the backup, it does not mark that tab dirty.
Closing it can therefore bypass the unsaved-changes prompt.

The recovery callback removes the backup directory after attempting recovery.
`recuperar` can return early when opening the original file fails. A missing file
or unavailable server can consequently cause the only backup to be deleted.

## Proposed changes

- Make recovery return a per-backup outcome instead of returning no result.
- Explicitly mark recovered content as unsaved, including untitled documents.
- Recover unavailable originals into a new document that retains the original
  path as recovery metadata; do not require the original to exist.
- Retain failed backups. Remove a recovered backup only after a replacement
  recovery copy has been persisted, or the student saves or explicitly discards it.
- Treat dismissing the recovery dialog as cancellation, preserving backups.
- Report failures with retry and export options.
- Define recovery behavior for untitled documents created before choosing a project;
  current project-keyed backup storage does not cover that case.

## Acceptance criteria

- Recovering an untitled document produces a dirty tab and a save prompt on close.
- A missing original or disconnected server never deletes the backup.
- A crash immediately after recovery still leaves recoverable content.
- Recovering several files preserves failures independently of successful files.
- Cancel, explicit discard, successful save, and repeated startup have distinct,
  tested outcomes.

## Verification

Add document-lifecycle and filesystem regression tests using temporary directories.
Exercise recovery callbacks under a real or virtual GTK display, including a second
startup after a simulated interrupted recovery.
