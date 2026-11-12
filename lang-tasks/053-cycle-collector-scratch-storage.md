# 053 — Reduce temporary work in cycle collection

Status: conditional plan; not implemented. Supplements [009](009-collection-cycle-reclamation.md); it does not replace cycle-reclamation correctness work.

## Evidence and scope

[Current profiles](../benchmarks/PERF-PRIORITIES.md) show about 10% in explicit collector symbols and 21% in allocator symbols in the cycle-heavy case. Hashing and allocation called by the collector are reported under other symbols, so 10% is not its inclusive cost. Other profiled workloads show little time in explicit collector symbols.

Investigate reusable scratch storage and redundant temporary structures. Do not remove the collector, weaken protection of external roots or assume it caused the stage-09 CPU regression without a before/after collector-specific comparison.

## A. Characterize the cost and boundaries

1. Reproduce allocation/cycle pressure using the current reference. Separate acyclic churn, unreachable cycles, externally rooted cycles, closures capturing collections, and repeated program executions.
2. Inspect `value/cycles.rs` and its coordination with environment/closure collection. Attribute temporary maps, sets, queues, node/edge tables and reference clones before selecting a structure to reuse.
3. Record total CPU and collection pause distributions. Phase instrumentation, if needed, belongs in a separate diagnostic build; do not mix its timings with uninstrumented end-to-end results.
4. Preserve the existing collection period and work limits in the first optimization. Changing thresholds simultaneously would prevent attributing the result.

## B. Reuse one proven scratch structure

1. Select one queue/vector/table with clear per-pass ownership; introduce a scoped scratch owner with explicit entry, exit and failure cleanup.
2. Clear values, weak/strong references and identity mappings before reuse. Address reuse must not make a new object inherit a previous collection's mark or edge count.
3. Handle reentrancy and thread-local borrowing explicitly. Recursive cleanup, destructors and independent interpreter runs must not observe an already-borrowed or partially reset collector.
4. Set a retained-capacity bound and a release policy for exceptionally large runs. Maintain existing allocation/work error behavior; retained scratch is additional runtime memory and must be measured.
5. Preserve external-root accounting and the existing environment/closure coordination. Scratch references must not accidentally keep dead graphs alive or permit clearing reachable objects.
6. Measure this structure before reusing another. If scratch allocation is not important, retain the simpler implementation.

## Correctness checks

- Reachable/unreachable list-map cycles, shared aliases, self-cycles and cycles passing through closures/parameter cells.
- Roots held by globals, native hosts, debug hooks or evaluator temporaries; collector must never clear them prematurely.
- Repeated collection with object/address churn, independent runs on the same thread and separate threads.
- Clone failure/cancellation cleanup and errors during execution: scratch must reset without keeping partial graphs alive.
- A large collection followed by small programs: verify the stated retained-capacity policy and bounded pause/work behavior.

Use existing reclamation tests and reference/liveness observations. Do not rely only on process RSS, which can stay high after logically freed objects return to malloc.

## Measurements and completion

Follow the [shared protocol](PERFORMANCE-PLAN.md). Include cycles, acyclic map/list churn, long-lived rooted graphs, escaping closures and normal Conway/sprite workloads. Report allocation counts per collection, total CPU, requested/live/retained bytes, RSS and high-percentile/maximum collection pauses. State the pause instrumentation and its overhead.

Do not trade materially worse pauses, growing retained roots or normal-workload regressions for a better cycle-only throughput score. Reprioritize this task if representative games do not exhibit significant collector pressure. Existing cancellation/deadline semantics and work bounds remain requirements.
