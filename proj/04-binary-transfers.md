# 04 — Transfer binary assets in remote projects

Priority: high for graphics lessons. Coordinate protocol changes with item 02.

## Problem and evidence

`aula/proto/proto/aula.proto` carries source text as protobuf strings.
`copy_project_file` in `aula/ui/src/exec.rs` skips remote files when the read error
mentions invalid UTF-8. `upload_changes` also skips remote binary output.
Consequently, image assets and generated images behave differently in local and
remote projects, without a clear transfer failure.

## Proposed changes

- Add byte-based read and write operations with revision metadata. Keep text
  decoding in the editor rather than imposing UTF-8 on repository contents.
- Preserve old text operations for compatibility or negotiate a protocol capability.
- Replace matching English error-message text with typed errors or capabilities.
- Establish transfer size limits, cancellation, and progress reporting. Use chunked
  or streaming transfer when required by the supported asset sizes.
- Copy and upload binary and text files through the same accounting model.
- Preserve unsupported or failed transfers for retry/export using item 03.
- Keep binary files out of text tabs while allowing them to exist in the project tree.

## Acceptance criteria

- A remote PNG is byte-identical in the execution workspace.
- A generated binary file returns to the remote project without corruption.
- Empty files and invalid UTF-8 transfer correctly.
- Unsupported older servers produce an actionable message instead of silent omission.
- Oversized and interrupted transfers fail predictably without replacing valid files.

## Verification

Add RPC round-trip tests with binary fixtures and end-to-end workspace synchronization
tests. Test compatibility with older protocol behavior and cancellation mid-transfer.

## Implementation ready for review

Implemented in the working tree; no commit created.

- `ReadFileBytes` and `WriteFileBytesChecked` carry raw contents plus the same
  SHA-256 revision as text saves. `ReadFile` / `WriteFileChecked` remain for
  older editors. Text decoding stays in the editor: `ReadFile` on a PNG returns
  the stable `arquivo não é texto UTF-8` and does not open a tab.
- The editor copies and uploads through `Files::current_bytes` /
  `write_bytes`. UTF-8 outputs still use `WriteFileChecked` so a mixed
  deployment keeps text execution. Non-UTF-8 outputs use
  `WriteFileBytesChecked`. Identical destination bytes acknowledge without a
  second write. Failed or unsupported transfers stay in the item 03 journal
  for retry/export.
- An older server answers `UNIMPLEMENTED`. The client then tries `ReadFile`
  for UTF-8 files and otherwise reports that the server must be updated,
  including the path. It never omits the file so the run can continue with a
  missing sprite. Writes map `UNIMPLEMENTED` to a Portuguese error rather
  than a `utf-8` substring.
- Transfer size is the same 3 MiB whole-file limit as text, below the default
  gRPC message size. Oversized writes are rejected before `storage::save`.
  Classroom sprites and `salve("casa.png")` frames fit. Chunked streaming is
  not required at this size. RPC cancellation from item 06 aborts the client
  future; a request that never completes does not publish.
- Binary files remain in the project tree. Opening them as source uses
  `ReadFile` and stays on the status bar.

Verification:

- Server: PNG and empty-file round trip, `ReadFile` rejection of the same PNG,
  oversized byte write leaving the original intact, competing byte writes with
  one revision.
- IDE: local PNG copy (existing), remote PNG copy into the execution workspace,
  generated PNG/`novo.bin` uploaded and read back byte-identical, tree click
  error `arquivo não é texto UTF-8`.

Native Windows and a manual classroom walkthrough remain review checks.
