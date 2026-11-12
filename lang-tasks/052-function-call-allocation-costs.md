# 052 — Reduce per-call metadata and temporary allocations

Status: planned investigation and implementation; not implemented. Reprofile after earlier tasks before choosing a concrete storage change.

## Evidence and objective

[Argument/closure profiles](../benchmarks/PERF-PRIORITIES.md) show approximately 6–8% in explicit allocator symbols, plus separate frame-entry/exit, string and movement costs. These are not all attributable to calls without further isolation.

`Machine::call_stack` already consumes arguments from the value stack and parameters already have indexed storage. Remaining work includes a new function `Env`, parameter materialization, the frame's environment vector and repeated owned diagnostic name/file metadata. Reduce avoidable allocations without changing observable frame/closure lifetime.

## A. Attribute allocations before choosing a change

1. Measure zero-argument, multi-argument, recursive and escaping-closure calls separately. Keep totals comparable with existing argument/closure workloads.
2. Inspect allocations and flat/annotated profiles for call entry, parameter binding, frame push/finish and name/file copies. Do not attribute all malloc/free samples to one structure.
3. Write down each per-call allocation, its owner/lifetime and whether it is observable through closures, `avalie`, errors, modules or debug hooks. Select one source for the first patch.

## B. Metadata sharing

1. Consider shared immutable internal name/file/source metadata instead of repeated owned strings. Retain correct call-site names, module-relative file context and source spans.
2. Prefer keeping public error/debug/embedding structures compatible by materializing owned metadata at boundaries if practical. If a Rust public type must change, document its compatibility impact and migration explicitly before adopting the design.
3. Preserve stack contents on normal return, nested errors, recovery, cancellation and callbacks. Never use a diagnostic call name as proof of which function actually executes.
4. Measure this increment before altering frame or parameter storage.

## C. Temporary frame/argument storage

1. After remeasurement, identify avoidable vector allocation/movement. Evaluate caller-stack parameter windows or reuse of cleared storage only with explicit ownership and cleanup rules.
2. Keep one canonical value for each parameter, visible to name lookup, indexed lookup, `avalie`, debug mutation and the collector.
3. Native callbacks, recursion, reentrant evaluation and nested calls must not overwrite live argument/parameter storage.
4. Escaped closures require their captured cells to outlive the call. Do not pool/rebind `Env` objects simply because a call returned: strong/weak references retained by closures or host hooks can observe their identity and lifetime.
5. Bound retained spare capacity and clear references through appropriate cleanup paths. Do not turn fewer allocation requests into unreported long-lived heap growth.

Frame storage may remain unchanged if a safe, useful win is not demonstrated. This task does not include a new calling convention for the language, tail-call semantics or changes to EXB encoding without a separate compatibility specification.

## Correctness checks

Cover zero/eight/many arguments, wrong arity, recursion limits, callbacks, returned closures and retained iteration captures. Include `avalie` reading/writing parameters, debug hooks retaining context, exact stack traces across modules, relative imports, recovery and cancellation. Verify references and cycles are released after both success and failure. Exercise AST/bytecode/EXB and applicable full/minimal profiles.

## Measurements and completion

Follow the [shared protocol](PERFORMANCE-PLAN.md). Use arguments, closures, recursion, long small-function loops, native callbacks and a workload retaining returned closures. Native graphics calls are regression coverage rather than assumed beneficiaries of an Expressa-only call change.

Report allocations per call, requested/live/retained bytes, RSS, CPU, counters and executable size. Measure metadata and frame-storage increments independently. Require the targeted allocation reduction and a reproducible useful end-to-end effect; document any retained-capacity or recursion-memory tradeoff.
