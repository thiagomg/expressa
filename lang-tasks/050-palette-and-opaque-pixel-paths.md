# 050 — Direct palette conversion and opaque pixels

Status: planned; not implemented. Suggested first small performance task.

## Evidence and objective

[Sprite profiles and annotation](../benchmarks/PERF-PRIORITIES.md) place `TelaGrafica::como_imagem` at 17.60–18.53% of samples. Its inlined work includes index → color name → RGBA conversion and general alpha blending. That entire percentage is not removable conversion cost.

Reduce work per pixel without changing palette values, blending results, clipping, snapshots or the graphics API. Deliver the two changes below separately.

## A. Direct palette-to-RGBA conversion

1. Inventory palette indices, color values, `normal`, `transparente` and the fallback for unknown indices in `graf.rs`. Establish exact current output before modifying the loop.
2. Replace per-pixel name lookup/string interpretation in `TelaGrafica::como_imagem` with direct indexed RGBA data. Keep palette names used by the public API; avoid divergent definitions of the same colors.
3. Preserve the fallback for every out-of-range byte value and avoid adding unchecked indexing. Keep transparency alpha exactly as before.
4. Keep the rest of rasterization, image ownership and stamping unchanged. Do not add snapshot caching or reusable mutable output buffers in this increment.
5. Measure and document A before starting B. `Raster::rgb32` has a similar pattern, but optimize it in a separate increment only with its own evidence and RGB-output coverage.

## B. Opaque-pixel copy path

1. Add a fast path for source alpha 255 in `carimbar_rgba`: after the same coordinate/clipping checks, copy the source RGBA pixel directly. Prove equivalence to the existing formula for all destination alpha values.
2. Keep intermediate-alpha blending and integer rounding unchanged. Preserve the existing skip threshold for alpha below 16, not only alpha zero.
3. Preserve odd/even sprite centering, rotations and edge behavior. The angle-zero integer path already exists; this task is not adding it again.
4. Treat an equivalent RGB32/window stamping path as another increment if pursued. Preserve that output's channel representation rather than copying an RGBA word blindly.

## Correctness checks

- All palette entries, `normal`, transparent pixels and unknown indices; compare exact pixels with a frozen reference or independent expected colors.
- Source alpha 0, 15, 16, intermediate values, 254 and 255, over transparent, partially transparent and opaque destinations.
- Negative positions, clipping on every edge, odd/even dimensions, zero/right-angle/arbitrary rotation and overlapping sprites.
- Snapshot isolation and image clone/COW behavior: no change in which objects share identity or pixels.
- Source, bytecode and EXB graphics results where available; minimal-profile compilation still succeeds without graphics.

Use existing image tests and small pixel-level regression cases. Do not introduce an effect API or change the language.

## Measurements and completion

Follow the [shared protocol](PERFORMANCE-PLAN.md). Reuse existing sprites and Conway checksums, and add representative larger sprites, transparency and rotation cases before generalizing gains. Include an image with palette drawing but no sprite stamping to isolate A, and opaque versus mixed-alpha workloads to distinguish B's useful path from its extra branch cost.

Record whole-process CPU/RSS, instruction/cycle counts, allocation deltas and binary size for each increment. A microbenchmark may explain the loop but does not replace end-to-end measurements. Alpha-mixed cases must not silently regress to pay for the opaque-only benchmark. Window FPS remains unmeasured unless explicitly instrumented later.
