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
