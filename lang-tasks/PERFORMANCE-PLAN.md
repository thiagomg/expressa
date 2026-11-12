# Performance implementation plan after stage 09

Status: planning only; none of the five tasks below is implemented by this plan. No language changes, commits, or staging operations are part of planning.

The [perf investigation](../benchmarks/PERF-PRIORITIES.md) identified costs in the current bytecode runtime. Its percentages identify work to investigate, not predicted speedups. The existing resource policy (`:_clonavel` plus the internal native-resource flag) stays intact; its memory overhead is accepted.

## Tasks and sequence

| Task | Scope | Suggested sequence |
| --- | --- | --- |
| [049](049-lexical-slot-access.md) | Extend positional access to parameters in nested scopes, then provable locals | Largest VM opportunity; start with the bounded parameter phase |
| [050](050-palette-and-opaque-pixel-paths.md) | Direct palette conversion, then opaque-pixel copying | Start here for the next small, isolated optimization |
| [051](051-borrowed-graphics-map-lookups.md) | Avoid allocating temporary graphics lookup keys | Independent small follow-up |
| [052](052-function-call-allocation-costs.md) | Reduce per-call metadata and temporary-vector allocations | After measuring the remaining call costs |
| [053](053-cycle-collector-scratch-storage.md) | Reduce collector scratch allocations | Conditional on representative allocation/cycle pressure |

Recommended delivery order: 050-A (palette), 051 (lookup keys), 050-B (opaque pixels), 049-A (nested parameter reads), then 049-B (nested writes). Reprofile before expanding 049 to locals or choosing between 052 and 053. The order favors small reviewable changes first; 049 remains the largest architectural opportunity. Tasks can be reprioritized if later profiles change the evidence.

Every lettered increment is a separate implementation and measurement checkpoint. Do not combine palette conversion, alpha blending, variable addressing, or allocator changes into one performance result.

## Common correctness requirements

- Preserve source syntax, numeric results, scoping, aliasing, native-resource protection, cancellation and diagnostic behavior. No JIT, numeric-mode change, new graphics API, or no_std work is included.
- Use existing conformance tests and add targeted tests for the boundaries affected by each change. Shared runtime/native code must behave identically through AST, bytecode source and loaded EXB where applicable.
- Slot/compiler changes must also update bytecode verification and function capture analysis as required. Existing EXB fixtures must remain usable; if an encoding change is unavoidable, explicitly specify compatibility/versioning rather than silently changing the v1 layout.
- Validate default and minimal profiles where the changed code exists. Graphics-only work need not execute unavailable graphics in the minimal build, but the minimal build must continue to compile.
- Run the appropriate existing tests, Clippy and formatting checks before measurements. Do not compile or run tests alongside timed samples.

## Benchmark protocol for every increment

1. Freeze the immediately preceding reviewed runtime as the reference. The initial candidate is stage 09, described in its [manifest](../benchmarks/clone-fastpath/manifest.json); use that artifact only if it is still the relevant reference. Do not overwrite old datasets or sum speedup percentages from different stages.
2. Preserve before/after source archives or an applicable patch, binary and library hashes, build profile/features, compiler version, program hashes and command lines. Use separate Cargo target directories for different checkouts; copy libraries before building a different source tree.
3. Compare identical workloads in interleaved randomized order, with the same CPU affinity, decimal/en settings and backend. Use two rounds with unused seeds: 31 samples plus three warmups per ordinary case; 11 plus three for expensive graphical cases. Record any justified protocol deviation.
4. Record user+system CPU, wall time and whole-process peak RSS. Use the existing [ordinary runner](../benchmarks/run.py) and [graphics runner](../benchmarks/clone_graphics.py) where their cases apply. Include AST and bytecode as regression coverage; do not compare AST source against VM EXB without separating input-format costs.
5. Collect user-space hardware counters (`cycles:u`, `instructions:u`, `branches:u`, `branch-misses:u`) on longer targeted cases. Record event coverage/multiplexing and perf permissions. Use flat cycle profiles or annotation to test the proposed mechanism; percentages assigned to a caller may increase when code is inlined.
6. Compile the same [allocation probe](../benchmarks/clone_allocations.rs) against each frozen library. Use at least three warmed runs per targeted case, checking output and cleanup. Count alloc/realloc calls, requested bytes, peak live-byte delta and retained delta. These bytes are not RSS or malloc usable sizes.
7. Preserve expected-output/checksum validation, raw samples, medians and a dispersion summary. Report regressions, memory/binary-size costs and unsupported counters. Raster results are not window FPS; desktop results are not Raspberry Pi measurements.

## Adoption criteria

Correctness is mandatory. A performance increment is ready to adopt when the targeted mechanism improves reproducibly, its effect is visible in the relevant end-to-end cases, and unrelated regressions or memory costs are understood and documented. No universal percentage threshold is invented from the current profiles.

If results overlap the observed noise, lengthen the targeted workload or collect more independent rounds before claiming a gain. If the mechanism improves only in isolation, document that limitation; do not present it as an end-to-end win. Any meaningful tradeoff must be explicit in the reviewable result. Retain the simpler implementation if no useful gain is demonstrated.

Each delivery should contain the narrow runtime change, semantic-boundary tests, a before/after report with raw data and provenance, and an updated task status. Leave commits and staging to the user.
