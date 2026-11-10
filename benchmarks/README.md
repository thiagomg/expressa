# AST and bytecode performance comparison

The desktop bytecode backend is implemented following the user's approval of the AST phase (commit `bd654b5`) and subsequent instruction to proceed. Bytecode is the default; `--backend ast|bytecode` or `ExecutionOptions::backend` selects the temporary reference backend. There is no JIT. See [the implementation review](../lang-tasks/BYTECODE-REVIEW.md) and [the measured comparison](AST-VS-BYTECODE.md). Pico firmware and its resource validation remain separate pending work.

Subsequent optimization experiments are recorded separately in [BYTECODE-OPTIMIZATIONS.md](BYTECODE-OPTIMIZATIONS.md), including rejected changes, per-step patches, binary/source manifests and repeated measurements. Historical result files are preserved. Each new candidate is compared with the preceding accepted VM binary in the same run; the AST remains an additional reference. CPU affinity is fixed for the later experiments and recorded in the raw results.

## Phase 1: reviewable AST changes

Run the regression suite, review the task status in `lang-tasks/AST-REVIEW.md`, and build the AST interpreter with release optimizations. The runner measures the complete executable: startup, parsing, execution, and teardown. It uses Python's standard library plus a small benchmark-only C helper compiled with the system `cc`. The runner currently supports Linux/macOS. The interpreter itself remains Rust; this helper is not part of Expressa or its firmware.

```sh
cargo build -p expressa --release --bin expressa --offline --locked
python3 benchmarks/run.py --engine ast=target/release/expressa --repeat 7 --warmup 2 --output benchmarks/results/ast-review.json
```

These workloads have deterministic correctness checks and cover startup, arithmetic, calls/closures, collections, dynamic evaluation, matrices, allocation/cycles, and deep recursion. Step 03 adds a workload combining zero-argument and eight-argument calls; it is measured on both reference and candidate binaries. Every sample must print exactly `verdadeiro` and exit successfully. Failed samples abort the run rather than entering the statistics.

The JSON includes raw samples, medians, workload hashes, binary hashes/sizes, the source fingerprint including uncommitted Rust files, Git revision, compiler version, platform/CPU, and CPU affinity. Run while the machine is otherwise idle. Repeat longer workloads if startup dominates; very short startup results are descriptive, not reliable speedup evidence. A small native helper records the interpreter child's CPU usage, monotonic wall time, and peak RSS. Python only drives scheduling and timeouts; its polling interval does not enter the reported wall time. Measuring RSS directly from a Python-launched child can inherit the large launcher's pre-exec RSS high-water mark, so the helper starts through an additional exec boundary before spawning the measured interpreter. Check the helper against `/usr/bin/time -v` when validating a new host.

## Phase 2: explicit review gate (satisfied for this migration)

1. The user reviews the AST fixes, documented semantic boundaries, tests, and any baseline results.
2. The user explicitly agrees to proceed with bytecode implementation.
3. Preserve the approved AST binary and its source snapshot/fingerprint before replacing execution internals. Do not silently compare against a rebuilt AST with different semantics or build settings.
4. Implement one shared bytecode interpreter, without a JIT, preserving `avalie`, diagnostics, scope, and the reviewed numeric model. Keep the AST backend available as a temporary reference until parity and comparison are complete.

The user committed the AST changes and then explicitly asked to proceed with bytecode implementation. The approved AST binary and complete source archive were preserved under `target/ast-reference-bd654b5/` before changing the engine. The tracked reference manifest is [references/ast-bd654b5.json](references/ast-bd654b5.json). No new commits are created automatically.

## Phase 3: measured comparison

Build the AST reference and bytecode candidate with the same compiler, release options, numeric configuration, library features, and target architecture. On the same machine, interleave both binaries in one run:

```sh
python3 benchmarks/run.py --engine ast=target/ast-reference-bd654b5/expressa-ast --engine bytecode=target/release/expressa --backend ast=ast --backend bytecode=bytecode --repeat 15 --warmup 3 --output benchmarks/results/ast-vs-bytecode.json
```

The runner randomizes workload/engine order per round with a recorded seed. Each binary must accept the same `binary file.lep` command. The runner records and explicitly sets `EXPRESSA_BACKEND` for each engine (`--backend NAME=ast|bytecode`). Without an override the label `ast` selects AST and other labels select bytecode. The preserved pre-migration binary only implements AST and ignores this new setting. Engine names alone are not proof of backend identity: record each binary's build command and source snapshot alongside the JSON. An optional precompiled-bytecode execution result must be reported separately from source-to-result execution.

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

`results/ast-review.json`, if present, is an unapproved working-tree AST snapshot only. It must be regenerated or explicitly accepted after review. The desktop comparison is recorded separately in [AST-VS-BYTECODE.md](AST-VS-BYTECODE.md); the older AST-only result is historical, not the comparison denominator. Execution-only, heap-allocation and Pico measurements remain unmeasured. Do not use the previous broken evaluator as the performance reference for a candidate that performs additional correctness checks without disclosing the difference.
