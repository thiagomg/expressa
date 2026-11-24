# Project-file storage

Shared by the editor and server. Requires Rust 1.89 or newer for standard-library
file locking. This crate has no GTK, protobuf, or interpreter dependency.

`read` returns bytes and a SHA-256 content revision calculated from those exact
bytes. `save` requires `Expected::Missing` (exclusive creation) or the previously
read `Expected::Revision`. `Unversioned` is rejected. Identical contents have the
same revision; this is a content comparison, not a monotonic edit counter.

Writes stage a complete file beside the destination, synchronize it, check the
baseline again, and publish it through an atomic filesystem operation. Updates
never remove the original as a prerequisite to replacement. Unix mode bits and
effective write access are checked, and mode changes observed during staging are
respected. Windows updates use [ReplaceFileW](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-replacefilew)
to preserve the destination's DACL and named streams, with a native backup retained
or restored on exceptional replacement errors.

A directory `.expressa-save.lock` serializes cooperating save processes across
replacement of the destination inode. A process-wide mutation guard also protects
local editor rename/delete operations against its upload workers. The server
serializes save, rename, and delete requests. Lock files persist intentionally;
deleting a live lock file would let writers lock different inodes. These names and
the `.expressa-save-` prefix are reserved and omitted from project listings.

Temporary files are removed on ordinary failed writes through RAII. Crash leftovers
are hidden from project listings and retained conservatively: blindly sweeping the
reserved namespace could delete a retained Windows original or interrupted output.
This is not a guarantee against arbitrary external writers that ignore the lock,
directory replacement, or filesystems without the required locking/rename semantics.

Publication is the commit point. A directory synchronization or backup-cleanup
failure after that point returns `Saved.warning`, not a false pre-commit error.
Callers must keep the new revision and report the warning. Conflict and other
pre-publication errors leave document state unsaved.

Run `cargo test -p expressa-aula-storage --locked`. Tests include injected failures,
exclusive creation, stale revisions, competing threads/processes, permission changes,
and preservation of binary conflict output. The ignored `writer_process` test is a
subprocess helper actually invoked by the competing-processes test. The Windows
test additionally checks explicit ACLs and NTFS named streams. The Linux/Windows
workflow is `.github/workflows/storage.yml`; native Windows execution is required
to validate its platform-specific behavior.
