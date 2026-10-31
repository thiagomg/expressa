# 08 — Enforce classroom identity and filesystem boundaries

Priority: high before shared classroom deployment that requires student isolation.

## Problem and evidence

The server accepts a student name from every request without authentication.
`aula/server/src/main.rs` documents this behavior. A connected client can therefore
select another student's name for reading, modifying, or deleting files.

`paths::student_file` and the local storage resolver check lexical containment.
The server's `walk_tree` follows directory symlinks. Existing links can lead outside
the project or create recursive traversal. Local listings skip links, but direct
file operations still need an explicit containment policy.

## Proposed changes

- Define deployment modes explicitly: single-user loopback and authenticated classroom.
- Bind student identity to an authenticated session; authorize requests on the server
  rather than trusting their student field. Define teacher privileges separately.
- Protect credentials and content in transit for classroom connections.
- Choose and enforce a symlink policy, initially rejecting links throughout project
  paths if that is sufficient for supported workflows.
- Validate existing ancestors for new-file writes and use filesystem operations
  that prevent link substitution races where required by the supported platforms.
- Prevent traversal cycles and bound tree depth, entry count, transfer sizes, and
  concurrent work. Return clear errors when limits are reached.
- Apply equivalent path rules to local storage and review execution confinement;
  lexical normalization alone is not a filesystem containment guarantee.

## Acceptance criteria

- An authenticated student cannot access another student's files by changing a field.
- Teacher access is explicit and covered by authorization tests.
- Links in student, project, parent-directory, and target-file positions cannot
  escape the configured storage boundary.
- A symlink cycle cannot hang or exhaust a listing operation.
- A non-loopback deployment cannot accidentally claim student isolation while
  running the unauthenticated mode.

## Verification

Use RPC authorization tests and temporary filesystem fixtures, including symlinks
on supported platforms. Exercise each read/write/create/rename/delete operation,
not just path parsing. Test local mode compatibility separately.
