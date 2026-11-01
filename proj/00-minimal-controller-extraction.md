# 00 — Extract the minimum boundaries for reliability fixes

Priority: first enabling step. This is the small initial slice of
[item 10](10-architecture-and-ownership.md), not the full architectural reorganization.

## Goal

Make document lifecycle operations and execution availability easier to change and
test before implementing recovery, safe saves, and workspace fixes (01–03).
Preserve existing behavior during this extraction, including known bugs that will
receive explicit regression fixes in the following steps.

## Scope

### 1. Establish a verification baseline

Record the relevant IDE and server test results. Use temporary storage roots for
new tests and ensure GTK-dependent verification actually runs with a display.
Apply only the test isolation changes from [item 11](11-testing-and-verification.md)
needed for this work; completing all CI and platform work is not a prerequisite.

### 2. Extract document lifecycle coordination

Introduce a document controller module, for example `aula/ui/src/documentos.rs`.
Move the coordination currently spread across `open_path`, `save_doc`, `saved`,
`close_doc`, and `confirm_unsaved` in `main.rs` behind a small public interface.

- Keep `Doc` responsible for its GTK buffer, view, and marks.
- Give the controller narrow access to documents and storage. Pass explicit inputs
  rather than introducing another owner of the entire `UiRc`.
- Keep dialog construction and presentation in the UI layer; communicate choices
  and operation results through explicit values or callbacks.
- Keep session serialization, backup layout, and recovery policy in `sessao.rs` for
  now. Provide a document-opening boundary it can call in item 01.
- Preserve existing synchronous storage behavior in this step. Background I/O and
  document revision handling belong to items 02 and 06.
- Move operations incrementally. Existing UI functions may remain as thin delegates
  while callers migrate, avoiding a broad rewrite of event handlers.

Do not create a generic framework. Use the existing `Files` type initially; add a
small test substitution only where it enables a meaningful failure scenario.

### 3. Centralize execution availability

Introduce a small execution lifecycle model, for example in
`aula/ui/src/execution_state.rs`, covering the currently supported transitions:

`idle → preparing → running → uploading → idle`

Represent preparation failure as a return to idle. Preserve current behavior for
normal completion, stop, and execution failure, which may still need an upload.

- Centralize checks for whether a run or project switch may start.
- Replace duplicated decisions based on `run` and `ocupado` with lifecycle queries.
- Keep run handles and workspace data with their current owners during this first
  extraction; the lifecycle model records availability, not a second copy of resources.
- Represent non-execution busy operations explicitly if existing call sites require
  them. Inventory those uses before replacing the current busy flag.
- Keep debugger pause/step handling and execution event transport in place.

Full execution/debug controllers, richer stopping states, and structured project
identity remain in items 10 and 03. They are not prerequisites for recovery fixes.

## Suggested change boundaries

1. Extract document coordination and migrate its callers; verify existing workflows.
2. Extract execution availability and migrate its checks; verify state transitions.
3. Proceed directly to item 01, then items 02–03. Stop extracting once those fixes
   have the boundaries they need.

If the extraction expands into unrelated ownership changes, reduce its scope.
A straightforward critical recovery fix can proceed before extraction finishes.

## Acceptance criteria

- Document open/save/close coordination has one clear entry point per operation;
  corresponding `main.rs` handlers delegate rather than duplicate policy.
- Document outcomes and execution availability can be exercised without constructing
  the complete application window.
- Preparation failure, execution completion, stop, and upload completion have tested
  availability transitions; a second run cannot begin during preparation or upload.
- Unsaved-change prompts, shortcuts, tab selection, unsaved-tab execution, and
  imported-file debugging retain their existing behavior.
- No protocol, cache format, recovery format, language semantics, or user-facing
  feature change is bundled into the extraction.
- No broad `UiRc` dependency is introduced into the new lifecycle model.
- Items 01–03 can proceed without completing the remaining architecture plan.

## Verification

Run focused controller and transition tests, then the IDE and server suites.
With a working GTK display, check opening files, saving named and untitled tabs,
cancelling close, saving on close, switching projects, running, stopping, and
debugging an import. Include preparation failure and upload failure.
Record known recovery failures as follow-up cases for item 01 rather than making
the current faulty behavior a permanent test expectation.

## Implementation ready for review

Step 00 is implemented in commit `d96e442`. Items 01–03 were delivered separately;
see the [current status](README.md#current-status).

- `aula/ui/src/documentos.rs` owns open/reuse decisions, save outcomes and name
  validation, close decisions, and save/discard/cancel policy. It borrows explicit
  document and storage inputs; it does not own or borrow `UiRc`.
- `main.rs` delegates those decisions while retaining dialogs, tab attachment,
  file monitoring, and visible status updates.
- `aula/ui/src/execution_state.rs` owns availability and valid lifecycle transitions.
  Run/project-switch guards use it; run handles and workspace data keep their
  existing owners. The busy-flag inventory found only preparation and upload uses.
- New tests cover document outcomes without the complete window, invalid lifecycle
  transitions, actual worker success/error/stop, failed preparation/upload, and an
  unsaved imported module with a breakpoint.
- The GTK test harness requires successful display initialization and caches failure
  without retrying initialization on other threads. A negative check confirmed that
  an unavailable display fails explicitly rather than skipping tests or crashing.
- The PNG workspace test uses a temporary cache, and home-path tests receive their
  home directory explicitly rather than changing process-wide `HOME`.

Verification: the baseline was 73 IDE tests and 22 server tests passing. After the
extraction, 78 IDE tests and 22 server tests pass, with GTK assertions confirmed
under the desktop display. Clippy passes for IDE/server targets with warnings denied.
Changed Rust files pass formatting checks and the diff passes whitespace checks.
The initial workspace-wide formatting check also found
pre-existing differences in `src/runtime/graf.rs` and `src/runtime/nativas/graf.rs`;
those unrelated files were left unchanged.

Automated tests exercise document and worker workflows; a manual walkthrough of
the complete desktop application, including project switching and keyboard/focus
behavior, remains a review check. See `aula/README.md` for test commands and display
requirements.
