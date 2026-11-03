# 09 — Stream bounded REPL output and own session lifecycles

Priority: high. Reuse the normal execution output design where practical.

## Problem and evidence

`Saida` in `src/runtime/repl.rs` accumulates bytes in a growing vector. The REPL
sends output to the editor only when evaluation finishes. An infinite printing
loop produces no live output and can exhaust memory.

`aula/ui/src/painel_repl.rs` keeps appending to its output buffer. Restart replaces
the session, while polling callbacks refer to the panel's current session rather
than a captured session identity. Pending old callbacks require lifecycle protection.

## Proposed changes

- Introduce REPL events for stdout, stderr, evaluation results, and termination.
- Send output while evaluation runs through a bounded channel with backpressure.
- Drain events with a bounded per-frame work budget and cap displayed scrollback.
- Preserve UTF-8 across write boundaries and distinguish output streams.
- Give sessions and evaluations identifiers. Cancel old polling sources and reject
  stale events after restart or panel disposal.
- Ensure stopping can interrupt evaluation even while output is backpressured.
- Define shutdown/disconnection behavior and surface worker failures.

## Acceptance criteria

- A printing loop displays output before it terminates.
- Memory use stays bounded during sustained output, and the editor remains responsive.
- Stop works under output saturation.
- Restart during a running evaluation cannot mix output or pending counts between sessions.
- Closing the panel/window releases its worker and event sources.
- Non-ASCII output remains intact when writes split an encoded character.

## Verification

Use bounded-channel runtime tests and GTK integration tests for restart, stop,
and output saturation. Include a long-running print loop and repeated restarts.

## Implementation ready for review

Implemented in the working tree; no commit created.

- `src/runtime/repl/canal.rs` replaces `RespostaRepl` / `repl_por_canal` with
  `CanalRepl`, which owns a worker, cancellation state and bounded command/event
  queues. Events identify the session and evaluation and distinguish stdout,
  stderr, results and termination. The command-line REPL retains direct I/O.
- Output is sent during evaluation in chunks of at most 4096 bytes. The event
  queue holds 64 events and applies backpressure. Cancellation is checked while
  waiting for capacity, independently of GTK draining. Stop targets an evaluation
  ID; idle Stop and unread older results cannot cancel subsequent input.
- Commands are limited to 64 KiB each, with at most 16 outstanding evaluations.
  Oversized or full-queue submissions are rejected visibly without increasing the
  outstanding count. The panel preserves rejected code for retry and bounds its
  command history to 100 entries. Result/error previews are shortened at 16 KiB,
  on a UTF-8 boundary, with an explicit abbreviation marker.
- One weak polling source belongs to each session. Every 30 ms it collects at most
  128 events, with a 64 KiB target byte budget (plus at most one final event) and
  a 4 ms collection budget, then renders adjacent chunks in batches. Rendering
  itself is bounded by these event/payload limits, rather than a timing guarantee.
- Scrollback uses the normal console's 5000-line trimming and an additional cap
  of 262144 characters, covering output with no newlines. Old text is discarded
  from the beginning. Stdout/stderr have independent incremental UTF-8 decoders;
  stderr remains styled as an error. Decoder tails end at evaluation boundaries.
- Restart cancels/removes the old polling source, drops the old session and clears
  unfinished code and decoder state. Session IDs reject stale callbacks/events;
  queued old commands are abandoned. Previous displayed output remains visible.
- Confirmed window close, native window teardown and panel disposal cancel the
  worker and remove polling sources. Shutdown abandons queued work and blocked
  sends without joining on the GTK thread. Cancellation remains cooperative at
  interpreter/native interruption points; it does not forcibly terminate an OS
  operation or constrain memory explicitly allocated by the student's program.
- `saia(code)` ends the panel session, reports its code and clears queued work.
  Stop during a native wait cancels only that evaluation. Worker panics and
  unexpected channel disconnection are surfaced; Reiniciar starts a usable session.
  Explicit shutdown may discard terminal events to avoid blocking on a full queue.

Regression coverage includes a live infinite printer, a saturated channel stopped
without any consumer, Stop followed by a successful command, native-wait interruption,
bounded admission/scrollback, UTF-8 at every byte split, stream styling, repeated
restart with immediate new input, stale callback rejection, panic/disconnection
recovery, panel disposal and destruction of a real IDE window while UI state remains
referenced. GTK tests exercise Entry activation and the Stop button, and check that
the main loop continues processing callbacks under sustained output.

Verification passed on Linux with desktop access:

- Runtime suite: 344 passed.
- IDE suite: 133 passed; 1 ignored subprocess helper (executed by its parent test).
- `cargo clippy --workspace --all-targets --offline --locked -- -D warnings`.
- Targeted Rust formatting and `git diff --check`.

Native Windows and manual desktop walkthroughs remain review/release checks.
