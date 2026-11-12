# Expressa language tasks

The original interpreter review is preserved as tasks **001–033**, one finding per file and in the original order. Tasks **034–043** add the Raspberry Pi Pico 2 work. Numbers identify findings; they are not a mandatory implementation sequence.

The AST phase was committed by the user as `bd654b5`; its historical review is [AST-REVIEW.md](AST-REVIEW.md). The user then approved proceeding with bytecode. The desktop VM is now implemented for review in [BYTECODE-REVIEW.md](BYTECODE-REVIEW.md), with [measured CPU/RSS comparison](../benchmarks/AST-VS-BYTECODE.md). No JIT or Pico firmware is included, and no new commits are created automatically. Individual task statuses still distinguish completed desktop work, partial work, and pending semantic/embedded decisions.

## Review baseline

The review covered the lexer, parser, evaluator, values, environments, native dispatch, REPL, and specification. All 355 library tests passed with `cargo test -p expressa --lib --offline --locked`. Focused programs reproduced additional failures. Code findings and design proposals are labeled separately from reproduced failures. This was not an exhaustive audit of every native library or Aula.

## How the Pico 2 changes the plan

The official Raspberry Pi documentation lists **520 kB SRAM and 4 MB flash** for the Pico 2. It is a microcontroller target; the existing desktop runtime cannot simply assume an operating system, processes, windowing, files, threads, or dynamically growing host stacks.

Source: [Raspberry Pi microcontroller board documentation](https://www.raspberrypi.com/documentation/microcontrollers/pico-series.html). Additional hardware reference: [Pico 2 product brief](https://datasheets.raspberrypi.com/pico/pico-2-product-brief.pdf).

The current runtime requests 4 MiB stack segments and reserves 256 KiB before growing the stack (`src/runtime/pilha.rs`), so stack architecture is a concrete blocker before accounting for program memory and values. Pico support raises memory reclamation, allocation budgets, portable host interfaces, and predictable execution limits from improvements to prerequisites.

The default planning assumption is that Expressa programs execute **on the board**, rather than a desktop program merely controlling it. The agreed execution architecture is one shared bytecode interpreter for desktop and Pico 2, without a JIT. Source is compiled to bytecode and executed by the VM; `avalie(texto)` uses runtime compilation in the current scope. The initial Pico target includes on-board source compilation to preserve `avalie`; feasibility and resource costs must be measured. Console details remain open in task 034. An f32 numeric model and every desktop native library are not assumed necessary. Begin with one core and a measured minimal profile.

## Agreed execution architecture

- Pipeline: Expressa source → shared parser/compiler → bytecode → shared VM.
- No JIT or runtime native machine-code generation is planned.
- Replace tree-walking execution while preserving the language contracts selected during review. The current interpreter can serve as a migration reference, not a permanent separate Pico implementation.
- Use explicit VM frames and value slots with bounded resource use. Bytecode instruction format and register-versus-stack instruction design remain open.
- Preserve `avalie`: visible binding lookup, permitted updates, new names visible to subsequent code, and escaping closures. Already compiled code must observe bindings according to the same scope rules; task 036 must validate this before fixing the frame layout.
- Desktop and board hosts provide different capabilities and resource budgets around the same engine. If full source compilation does not fit the Pico budget, return to review with measurements rather than silently removing `avalie`.

Task 036 owns the VM migration; 034–035, 037, and 039–043 define its target contract, boundaries, memory model, measurements, loading, and validation.

## Review and migration sequence

1. AST changes were reviewed and committed as `bd654b5`; see [AST-REVIEW.md](AST-REVIEW.md). Review and resolve remaining pre-migration prerequisites explicitly; partial tasks are not silently complete.
2. The user explicitly approved bytecode implementation after the AST commit. This gate is satisfied.
3. Preserve the approved AST source and binary as the comparison reference. The benchmark harness can record a baseline now, but a working-tree sample is not an approved release.
4. Implement one shared bytecode interpreter (036) with the portable boundary, measured numeric representation, and bounded memory/execution architecture (034–039). Preserve reviewed semantics, including `avalie`.
5. Establish semantic parity, then compare actual CPU consumption and memory use against the preserved AST reference using [the documented protocol](../benchmarks/README.md). Keep compilation/startup and execution-only results separate.
6. Add board loading, scheduling, firmware, and hardware validation (040–043). Desktop RSS does not establish Pico SRAM usage.

## Task index

| ID | Task | Priority/type |
| --- | --- | --- |
| 001 | [Prevent overflow at inclusive range endpoints](001-range-endpoint-overflow.md) | High · reproduced |
| 002 | [Remove silent zero substitution from numeric conversions](002-fallible-numeric-conversions.md) | High · reproduced |
| 003 | [Define consistent scalar and matrix numeric semantics](003-matrix-number-consistency.md) | High · reproduced |
| 004 | [Preserve exact values in rounding functions](004-exact-rounding.md) | High · reproduced |
| 005 | [Evaluate compound-assignment destinations once](005-compound-assignment-location.md) | High · reproduced |
| 006 | [Resolve nested text-writeback destinations once](006-nested-text-writeback.md) | High · code finding |
| 007 | [Prevent silent writes to copied matrix rows](007-matrix-row-assignment.md) | High · reproduced |
| 008 | [Validate flattened import bindings before installing names](008-atomic-import-bindings.md) | High · reproduced |
| 009 | [Reclaim cycles in lists and maps without closures](009-collection-cycle-reclamation.md) | High · code finding |
| 010 | [Check matrix arithmetic results consistently](010-matrix-overflow-errors.md) | High · code finding |
| 011 | [Bound matrix and other user-controlled allocations](011-allocation-budgets.md) | High · code finding |
| 012 | [Apply execution limits inside expensive native operations](012-cooperative-execution-limits.md) | High · code finding |
| 013 | [Preserve source context for invalid control-flow errors](013-control-flow-diagnostics.md) | Medium · code finding |
| 014 | [Specify outer binding versus object mutation](014-outer-mutation-contract.md) | Decision · reproduced behavior |
| 015 | [Define local declaration and shadowing rules](015-local-declaration-shadowing.md) | Decision · observed behavior |
| 016 | [Define loop scope and captured iteration bindings](016-iteration-scope.md) | Decision · existing tests confirm behavior |
| 017 | [Resolve conflicting trailing-conditional rules](017-trailing-conditionals.md) | Decision · specification conflict |
| 018 | [Describe expression forms and decide empty-block behavior](018-expression-and-empty-block-contract.md) | Decision · grammar mismatch |
| 019 | [Define consistent statement continuation across newlines](019-newline-continuation.md) | High · indexing fixed for review; broader continuation pending |
| 020 | [Define what float arithmetic mode promises](020-float-mode-contract.md) | Decision · reproduced limitation |
| 021 | [Remove ambiguity between locale input and source numbers](021-locale-number-parsing.md) | Decision · code finding |
| 022 | [Specify supported integer ranges in restricted contexts](022-integer-context-limits.md) | Decision · code finding |
| 023 | [Document copying, aliasing, and binding immutability together](023-copy-and-alias-contract.md) | Implementado para revisão · clone e referências |
| 024 | [Define resource and import paths inside imported functions](024-module-relative-paths.md) | Decision · code finding |
| 025 | [Consider structured error information for recovery](025-structured-error-recovery.md) | Proposal · language improvement |
| 026 | [Split evaluator responsibilities into cohesive modules](026-split-evaluator.md) | Organization · proposal |
| 027 | [Replace long positional execution options with configuration](027-execution-configuration.md) | Organization · proposal |
| 028 | [Give native functions module-qualified identities](028-qualified-native-identities.md) | Organization · semantic concern |
| 029 | [Centralize native function metadata](029-native-metadata-registry.md) | Organization · proposal |
| 030 | [Use typed diagnostics and incomplete-input categories](030-typed-diagnostics.md) | Organization · proposal |
| 031 | [Strengthen and organize the language specification](031-specification-conformance.md) | Documentation · proposal |
| 032 | [Clarify the supported embedding API and AST layout](032-public-api-and-ast-layout.md) | Organization · proposal |
| 033 | [Add regression and conformance coverage at semantic boundaries](033-semantic-boundary-tests.md) | Validation · proposal |
| 034 | [Define the Pico 2 execution and compatibility contract](034-pico-target-contract.md) | Pico 2 · architecture decision |
| 035 | [Separate a portable language core from desktop hosts](035-portable-core-and-hosts.md) | Pico 2 · foundational |
| 036 | [Migrate to a shared bytecode interpreter with bounded execution](036-bounded-execution-stack.md) | Pico 2 · foundational |
| 037 | [Design a bounded heap and runtime resource model](037-embedded-heap-and-resource-model.md) | Pico 2 · foundational |
| 038 | [Define optional native libraries and board capabilities](038-embedded-library-profiles.md) | Pico 2 · foundational |
| 039 | [Validate numeric representation on the Pico 2](039-embedded-number-benchmarks.md) | Pico 2 · measurement and decision |
| 040 | [Design program loading and module storage](040-embedded-program-loading.md) | Pico 2 · architecture decision |
| 041 | [Define cooperative scheduling, time, and cancellation on board](041-embedded-scheduling-and-time.md) | Pico 2 · runtime integration |
| 042 | [Build the Pico 2 firmware and minimal console integration](042-pico-firmware-and-console.md) | Pico 2 · delivery |
| 043 | [Add Pico 2 conformance, resource, and hardware validation](043-pico-validation-and-size-gates.md) | Pico 2 · validation |
| 044 | [Testes determinísticos de retenção de processos](044-deterministic-process-tests.md) | Implementado para revisão |
| 045 | [Parâmetros de funções por posição](045-indexed-function-parameters.md) | Implementado para revisão; CPU/RSS medidos |
| 046 | [Instruções compactas em memória](046-compact-bytecode-instructions.md) | Descartado e revertido; dados e patch preservados |
| 047 | [Formato EXB v1 e carregamento desktop](047-exb-file-format-and-loader.md) | Implementado para revisão; CPU/RSS medidos |
| 048 | [Primeiro perfil mínimo do runtime desktop](048-minimal-desktop-runtime-profile.md) | Implementado para revisão; tamanho/CPU/RSS medidos; núcleo portátil pendente |
