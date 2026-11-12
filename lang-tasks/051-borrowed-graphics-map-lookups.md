# 051 — Avoid temporary graphics lookup keys

Status: planned; not implemented. Independent follow-up to palette conversion.

## Evidence and objective

[Profiles](../benchmarks/PERF-PRIORITIES.md) attribute about 19% of sprite samples to allocator symbols, covering many allocation sources. Code inspection identifies a specific avoidable source: graphics getters create `MapKey::Texto(chave.into())` merely to search for a borrowed `&str`.

Remove these temporary key allocations. Do not claim all allocator cost will disappear, or change the map/command representation.

## Implementation steps

1. Inventory `tem_chave`, `get_lista`, `get_imagem`, `get_texto`, `get_u32`, `get_f64` and `get_i32` in `nativas/graf.rs` and their callers.
2. Match an existing text key against the borrowed `&str`, preserving the current first-match semantics. Introduce a small shared helper only if it makes borrowing and behavior clearer.
3. Preserve ownership of returned data: strings returned as snapshots still need the correct ownership; list references share identity; image extraction must preserve snapshot pixel semantics. Removing key allocation is not permission to change value copying.
4. Keep `RefCell` borrows scoped so callbacks, mutations and error creation cannot encounter a new outstanding borrow. No references may outlive the map borrow.
5. Leave map creation, insertion, arbitrary key types, visible `ops` maps and the resource noncloneable flag unchanged.
6. Measure this graphics-only change first. Similar getters in other native modules can be audited later and delivered independently with their own coverage.

## Correctness checks

- Missing fields, wrong value types and all existing defaults/conversions for size, angle, coordinates and text.
- Numeric/bool keys must not match their textual spellings. Preserve first-match behavior if host-created storage contains duplicate keys.
- Mutating `ops`, its command maps, colors or coordinates between draws must still affect the next conversion exactly as before.
- Lists/images retain their aliasing and snapshot behavior; windows/documents remain noncloneable after marker edits/removal.
- Existing graphics error messages and pixel-reference tests through AST, bytecode and EXB.

These checks should validate externally visible behavior, not copy the lookup helper into its tests.

## Measurements and completion

Follow the [shared protocol](PERFORMANCE-PLAN.md). Compare sprites, Conway raster and a retained command list converted more than once. Use the warmed allocation probe to demonstrate fewer allocation requests on the paths actually exercised; distinguish key savings from unchanged image and command allocations.

Measure CPU and instruction/cycle counts as well: fewer allocations alone does not guarantee faster rendering. Include mixed command types and deliberately mutated command maps to prevent an invalid cache-based shortcut. Ordinary nongraphical workloads and minimal-profile compilation guard against accidental scope expansion.
