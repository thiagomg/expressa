# 11 — Isolate tests and make GUI verification meaningful

Priority: cross-cutting. Establish the minimum harness needed for each fix.

## Problem and evidence

The review run passed 73 IDE tests and 22 server tests, and the VS Code test command
passed. These results do not cover all lifecycle failures identified in this plan.

`gtk_test::run` in `aula/ui/src/main.rs` returns successfully when GTK initialization
fails, allowing GUI checks to appear green without executing their assertions.
`download_copies_a_png_without_failing` uses the real user cache. A preferences test
changes process-wide `HOME` without restoring it, affecting parallel tests and
potentially racing worker threads. No tracked repository CI configuration was found
in the review.

## Proposed changes

- Pass cache, configuration, and recovery roots explicitly to code under test.
  Use temporary directories and avoid process-global environment mutation.
- Separate pure model/storage tests from tests that require GTK.
- Make the GUI job require a display and fail clearly when initialization fails.
  If developer runs allow skipping, report that explicitly rather than silently
  treating an unexecuted test body as coverage.
- Add automated Linux GUI execution under a virtual display and platform-specific
  checks for Windows storage, paths, and packaging behavior.
- Run relevant formatting, lint, Rust tests, VS Code tests, and generated-data
  consistency checks in CI. Preserve a documented local equivalent.
- Add deterministic delayed/failing storage and controllable worker completion for
  recovery, conflict, stale-result, and shutdown tests.
- Fix broken references to absent `ajustes/` design documents in source comments,
  pointing to the applicable maintained plan instead.

## Acceptance criteria

- Tests do not write into real user cache/configuration/data directories.
- Parallel test ordering cannot change results through environment mutation.
- The GUI CI job proves that GTK-dependent assertions actually executed.
- New regressions cover the failures fixed by items 01–09, including lifecycle
  transitions, not just helper functions.
- Generated native metadata and editor grammar checks run automatically.
- Test commands and required system dependencies are documented for contributors.

## Verification strategy

Use focused regression checks per change, followed by the appropriate crate and
integration suites. Re-run broader checks when shared contracts change. Keep manual
checks for real window behavior, keyboard navigation, focus, and classroom workflows
as an explicit supplement to automated coverage.
