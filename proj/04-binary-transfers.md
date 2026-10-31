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
