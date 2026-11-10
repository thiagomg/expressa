# 10 — Separate controllers and make state transitions explicit

Priority: incremental foundation. Complete the bounded first slice in
[step 00](00-minimal-controller-extraction.md) before continuing this broader plan.

## Problem and evidence

`aula/ui/src/main.rs` contains roughly 3,800 lines and a broad `Ui` structure shared
through `Rc<RefCell<Ui>>`. It coordinates project selection, documents, storage,
execution, debugging, menus, and platform behavior. Callbacks consequently depend
on broad mutable state, making lifecycle changes and regression tests harder.

## Start here, but keep the first change small

The initial document coordination and execution availability extraction is specified
in step 00. Its acceptance criteria deliberately stop short of the full ownership
model below. After that step, implement items 01–03 and continue this reorganization
alongside subsequent fixes. Structured project identity is addressed in item 03.

Do not mix broad file moves, new abstractions, and behavioral fixes in one patch.
If a critical recovery fix is straightforward, it can precede extraction.

## Intended responsibilities

| Component | Owns |
| --- | --- |
| Application shell | Window construction, actions, menus, controller wiring |
| Project controller | Full project identity, repository, file tree, synchronization |
| Document controller | Tabs, revisions, saves, dirty state, recovery, external changes |
| Execution controller | Preparation, run handles, source snapshots, stop, uploads |
| Debug controller | Breakpoints, pause state, stack, variables, source navigation |
| Language service | Versioned analysis, completion, definitions, diagnostics |

Prefer concrete types initially; introduce traits for meaningful substitution,
such as delayed/failing storage in tests. Avoid adding an event bus merely to move
existing coupling into message handlers.

## State and ownership changes

- Replace loosely related run/busy flags with explicit states such as idle,
  preparing, running, paused, stopping, and uploading, with defined transitions.
- Associate asynchronous operations with project and document revisions.
- Give workers, timers, monitors, and child processes explicit lifecycle owners.
- Use weak callbacks where appropriate to prevent ownership cycles.
- Reduce broad mutable borrows across callbacks and nested UI event processing.
- Keep platform-specific startup and window behavior in separate modules.

## Acceptance criteria

- `main.rs` primarily constructs and connects components.
- Recovery/save decisions and execution transitions can be tested without constructing
  the entire application window.
- Invalid state transitions are rejected in one place.
- Closing or switching projects cancels or safely detaches outstanding work.
- Existing shortcuts, unsaved-tab execution, imports, and debugger behavior are retained.
- Each extraction is independently reviewable and passes relevant regression checks.

## Implementation (first extraction after step 00)

The GTK handlers that lived in `aula/ui/src/main.rs` (~5500 lines) are split by
domain. Behavior is unchanged: callbacks still borrow `UiRc`; this slice does
not introduce a second copy of run/project state.

| Module | Owns |
| --- | --- |
| `app.rs` | `Ui`, `Run`, `BarraExec` |
| `shell.rs` | menubar, shortcuts, CSS, status, dialogs, Windows foreground |
| `editor.rs` | tabs, open/save/close, theme, in-file search |
| `depuracao.rs` | debug panel, run-bar, breakpoints, paused-source viewer |
| `execucao.rs` | F5/F6, output streaming, upload after run |
| `projeto.rs` | identity, file tree, local/remote open, tree actions |
| `main.rs` | `main`, `build_ui_with_prefs`, GTK tests |

Second slice (Aula-only, no interpreter changes):

- `Ui` now holds `depuracao: Depuracao` (watches, stacks, run bar) and
  `execucao: Execucao` (run handle, console, graphics, source snapshot).
- `gtk_test.rs` and `debug_ui_tests.rs` live beside `main.rs`.

Still open: nest project/editor/shell fields the same way; drop glob
`pub(crate) use`; `main.rs` still builds the window.
