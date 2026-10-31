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

## Implementation ready for review

Step 01 is implemented in the working tree, without a Git commit. Step 02's
project-file save protocol and conflict detection remain separate work.

- `documentos.rs` returns per-copy recovery outcomes, explicitly marks restored
  documents dirty, and recovers missing originals or conflicts with dirty tabs
  into new untitled tabs. Their original names remain attached as metadata and
  appear in tab tooltips. Failed saves retain recovery ownership.
- `recuperacao.rs` publishes complete, synced snapshots under unique filenames
  before removing previous generations. Stable copy IDs distinguish different
  documents with the same original name. Restart scanning merges complete
  generations of one copy and retains malformed or unreadable records separately.
- The previous encoded-filename/plain-source backup format remains readable. New
  versioned records contain original names, copy IDs, and generations; older editor
  versions do not understand these new records. Legacy copies migrate only after
  a replacement snapshot is successfully written.
- `sessao.rs` no longer deletes a recovery directory or sweeps files without an
  owner. Cancellation, failed reads, and failed restores survive autosave and normal
  shutdown. Successful saves or explicit tab/discard decisions remove only owned
  snapshots; failed cleanup is reported separately from save success.
- Scratch tabs have a recovery scope before a project exists. Startup offers those
  copies before reopening the last project, and selecting the first project adopts
  the tabs. Their copies migrate only after a new snapshot is safely published.
- **Arquivo → Recuperar alterações…** provides retry and export. Export includes
  current unsaved buffers when autosave fails, preserves malformed records as raw
  files when readable, and never overwrites destination files or retires backups.
- The manual recovery menu discovers pending scopes without opening their projects
  or servers. Copies from another scope always open as new untitled documents,
  avoiding reads or writes to similarly named files in the active project.

Verification covers legacy/unreadable copies, interrupted writes and cleanup,
duplicate original names, non-overwriting export, cancellation, repeated recovery
after a simulated restart, failed saves, missing originals, conflicts with dirty
tabs, partial recovery, scratch migration, and autosave retry. GTK-backed tests
exercise document recovery with an actual initialized display. A manual walkthrough
of the recovery dialogs and native export folder chooser remains a desktop review
check; tests do not simulate an OS power failure.

Final verification: 88 IDE tests and 22 server tests pass, with GTK assertions
confirmed under the desktop display. Clippy passes for IDE/server targets with
warnings denied. Changed Rust files pass formatting checks and
the diff passes whitespace checks. Pending-scope discovery also covers interrupted
cross-project migration, so an open older snapshot does not hide a newer copy in
another project.
