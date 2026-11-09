# AST baseline and later bytecode comparison

The bytecode backend is **not implemented in this phase**. The user must review and explicitly agree to the AST changes before VM implementation begins. CPU and memory comparisons happen after the bytecode backend implements the same reviewed semantics. No speedup or memory-saving claim is established by this benchmark harness.

## Phase 1: reviewable AST changes

Run the regression suite, review the task status in `lang-tasks/AST-REVIEW.md`, and build the AST interpreter with release optimizations. The runner measures the complete executable: startup, parsing, execution, and teardown. It uses Python's standard library plus a small benchmark-only C helper compiled with the system `cc`. The runner currently supports Linux/macOS. The interpreter itself remains Rust; this helper is not part of Expressa or its firmware.

```sh
cargo build -p expressa --release --bin expressa --offline --locked
python3 benchmarks/run.py --engine ast=target/release/expressa --repeat 7 --warmup 2 --output benchmarks/results/ast-review.json
```

These workloads have deterministic correctness checks and cover startup, arithmetic, calls/closures, collections, dynamic evaluation, matrices, and allocation/cycles. Every sample must print exactly `verdadeiro` and exit successfully. Failed samples abort the run rather than entering the statistics.

The JSON includes raw samples, medians, workload hashes, binary hashes/sizes, the source fingerprint including uncommitted Rust files, Git revision, compiler version, platform/CPU, and CPU affinity. Run while the machine is otherwise idle. Repeat longer workloads if startup dominates; very short startup results are descriptive, not reliable speedup evidence. A small native helper records the interpreter child's CPU usage, monotonic wall time, and peak RSS. Python only drives scheduling and timeouts; its polling interval does not enter the reported wall time. Measuring RSS directly from a Python-launched child can inherit the large launcher's pre-exec RSS high-water mark, so the helper starts through an additional exec boundary before spawning the measured interpreter. Check the helper against `/usr/bin/time -v` when validating a new host.

## Phase 2: explicit review gate

1. The user reviews the AST fixes, documented semantic boundaries, tests, and any baseline results.
2. The user explicitly agrees to proceed with bytecode implementation.
3. Preserve the approved AST binary and its source snapshot/fingerprint before replacing execution internals. Do not silently compare against a rebuilt AST with different semantics or build settings.
4. Implement one shared bytecode interpreter, without a JIT, preserving `avalie`, diagnostics, scope, and the reviewed numeric model. Keep the AST backend available as a temporary reference until parity and comparison are complete.

Creating this harness or accepting the architecture does not waive this review gate. No commits are created automatically.

## Phase 3: measured comparison

Build the AST reference and bytecode candidate with the same compiler, release options, numeric configuration, library features, and target architecture. On the same machine, interleave both binaries in one run:

```sh
python3 benchmarks/run.py --engine ast=/path/to/approved/expressa-ast --engine bytecode=/path/to/expressa-bytecode --repeat 15 --warmup 3 --output benchmarks/results/ast-vs-bytecode.json
```

The runner randomizes workload/engine order per round with a recorded seed. Each binary must accept the same `binary file.lep` command. Engine names are labels, not proof of backend identity: record each binary's build command and source snapshot alongside the JSON. An optional precompiled-bytecode execution result must be reported separately from source-to-result execution.

Report per workload:

| Metric | Meaning |
| --- | --- |
| User + system CPU seconds | Actual process CPU consumption; lower CPU seconds means less CPU work for the same result |
| User and system seconds separately | Distinguishes engine work from operating-system overhead |
| Wall seconds | End-to-end latency, including scheduling and startup |
| CPU time / wall time × 100 | Average utilization, an auxiliary metric; a higher percentage is not by itself better performance |
| Peak RSS bytes | High-water resident process memory, including native stacks, libraries, compiler, program, and values; measured separately for every child |
| Executable bytes | File size, not a substitute for firmware flash usage or live memory |

For each metric retain all samples and report median plus spread (for example min/max or percentiles). CPU speedup is AST median CPU / bytecode median CPU. Report RSS differences in bytes and percentages, including regressions. Do not average speedups across workloads without specifying weighting. The harness captures process-level data only; it does not claim to separate parsing from execution or measure heap allocations.

For deeper analysis add the same instrumented embedding benchmark to both backends: separately measure parse/compile time, repeated execution of a prepared program in equivalent fresh state, live/peak heap bytes, allocation counts, VM frame/value-stack high-water marks, collection time, and retained memory after repeated sessions. Report instrumentation overhead and ensure both backends include equivalent output and cleanup work. Do not present execution-only bytecode results against end-to-end AST results.

## Pico 2 validation

Desktop results do not prove Pico suitability. Once firmware exists, measure on the same board and clock configuration: elapsed execution time/cycles, total SRAM high-water usage including native and VM stacks, heap/live objects and fragmentation, firmware flash usage, compilation peak, cancellation latency, and recovery after repeated runs. Document unavailable counters rather than treating desktop RSS as board memory. Preserve identical workloads and semantics where supported; distinguish board library limitations.

## Results policy

The current provisional measurement is summarized in [AST-BASELINE.md](AST-BASELINE.md), with raw samples in [results/ast-review.json](results/ast-review.json).

`results/ast-review.json`, if present, is an unapproved working-tree AST snapshot only. It must be regenerated or explicitly accepted after review. Bytecode performance remains unmeasured until phase 3. Do not use the previous broken evaluator as the performance reference for a candidate that performs additional correctness checks without disclosing the difference.
