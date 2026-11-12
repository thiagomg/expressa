# 049 — Extend lexical access by position

Status: planned; not implemented. Extends [045](045-indexed-function-parameters.md) without treating its existing parameter slots as unfinished work.

## Evidence and objective

[Current profiles](../benchmarks/PERF-PRIORITIES.md) attribute roughly 60% of arithmetic and 50–53% of Conway samples to hashing/binding lookup. Only a subset can be removed. `Op::Load` still resolves strings through `Env::get`, and stores resolve a destination then consult its bindings again. Parameter slots currently apply only to the function's main scope.

Eliminate provably unnecessary name lookup while preserving the language's dynamic behavior. The first target is repeated access to `m` and `n` inside `proxima(m,n)` in Conway; general locals come later.

## Implementation increments

### A. Parameters read inside nested scopes

1. Map how function frames, nested blocks, loop-variable frames, `ParentScope`, callbacks and `avalie` select environments. Specify which owner a positional read addresses.
2. Add conservative lexical resolution for parameter reads. Track shadowing explicitly; retain name lookup when the owner cannot be proved. Do not merely enable the compiler's `locals` flag in every block: current local instructions read the current environment, which may not contain parameters.
3. Choose an explicit owner-addressing representation. Prefer a compatible use of existing instructions only if their documented/verified meaning is preserved; otherwise update the instruction contract and EXB compatibility policy deliberately.
4. Read the same canonical parameter cell used by dynamic lookup. Do not maintain a second unsynchronized copy or change parameter lifetime.
5. Update compiler, executor, verifier and capture analysis together. Keep names/spans for debugging and errors. Measure this read-only increment before adding writes.

### B. Parameters written inside nested scopes

1. Extend proven addressing to assignment and compound assignment, preserving destination evaluation, function boundaries and builtin protections.
2. Ensure `avalie` reads and writes the exact same values, including evaluation through aliases/callbacks. Preserve the fallback for ambiguous cases.
3. Validate read/write coherence and measure separately from A.

### C. Provable locals and iteration variables

Design this only after A/B are measured. Model declaration availability, lexical owner, scope entry/exit and captured iteration bindings. Allocating slots must not make a name visible before its original declaration. Escaped closures must retain the correct iteration's cell. Unknown/dynamic names continue through the existing path.

Do not combine this with symbol interning, a new hasher, frame pooling or superinstructions.

## Correctness checks

- Parameter reads/writes across nested if/block/loop scopes, including a loop variable with the same name as a parameter.
- Compound assignments, parent-scope while conditions, break/continue, errors and `se_falhar` unwinding.
- `avalie` mutating parameters, conditional declarations and missing-name diagnostics; reads/writes through callbacks must remain coherent.
- Captured parameters, nested factories, escaped iteration closures, forbidden outer rebinding and object mutation through aliases.
- Debugger visibility and assignment, cycle reclamation through parameter values, and `clone` capture classification after any instruction changes.
- AST/bytecode/EXB parity, existing v1 fixtures and rejection of malformed owner/slot operands; default/minimal profiles.

Tests should exercise these semantic boundaries, not just assert an instruction spelling. Update the existing compiler fallback test only for scopes now proved safe.

## Measurements and completion

Follow the [shared protocol](PERFORMANCE-PLAN.md). First measure Conway simulation/full, long closures and parameters inside nested loops. Include a shadowing/dynamic-fallback workload. Ordinary arithmetic at module scope is regression coverage, not a promised beneficiary of phase A.

Check emitted addressing and reductions in name-resolution work, hardware instructions/cycles, end-to-end CPU and allocation/RSS costs. Keep source and EXB results separate if compiler/loader work changes. A/B are independently deliverable; C remains pending until designed and measured.
