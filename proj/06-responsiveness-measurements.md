# Item 06 — Document responsiveness measurements

Linux, GTK desktop session, development build. Run explicitly:

```sh
cargo test -p expressa-aula --offline --locked responsiveness_measurements -- --ignored --nocapture
```

The scenario creates a GTK document containing repeated valid Portuguese source,
inserts a comment while analysis is pending, requests analysis of the new revision,
and processes the GTK loop until that revision is ready. A 1 ms heartbeat records
main-loop progress. These are observations, not timing assertions or performance
budgets. Ordinary regression tests use controlled completion ordering instead.

| Lines | UTF-8 bytes | Buffer creation | Buffer insertion | Analysis wall time | Main-loop ticks |
| ---: | ---: | ---: | ---: | ---: | ---: |
| 500 | 6,750 | 131.09 ms | 0.25 ms | 19.36 ms | 12 |
| 5,000 | 67,500 | 27.82 ms | 1.12 ms | 154.65 ms | 120 |
| 50,000 | 675,000 | 64.31 ms | 9.84 ms | 727.32 ms | 660 |

Buffer creation in the first case includes cold GTK/font setup. Analysis wall time
includes worker scheduling, superseded analysis that may already be running and
delivery back to GTK. Its duration therefore differs from parser CPU time.

This measures the document component's programmatic editing and analysis pipeline,
not physical keyboard-to-screen latency or a full IDE completion popup. It shows
that analysis can take hundreds of milliseconds while the GTK loop continues
running; copying/inserting buffer contents still has a size-dependent GTK cost.
The delayed-I/O integration tests additionally exercise editing in a real IDE
document, cancellation and subsequent commands while a worker is held by a gate.

No fixed latency target is imposed from this single development-machine run.
Native Windows, rendering with real lesson projects, and full completion/definition
analysis remain useful measurements for the next performance review and item 07.
