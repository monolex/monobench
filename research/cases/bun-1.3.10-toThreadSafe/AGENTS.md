# Case Agent Guide: bun-1.3.10-toThreadSafe

This case is a cross-language ownership/refcount benchmark for Bun.

## Ground Truth Anchor

`src/bun.js/bindings/BunString.cpp::BunString__toThreadSafe`

The relevant pattern is `isolatedCopy + leakRef + overwrite of str->impl.wtf without releasing the old ref`.

## Known Archived Batch

`runs/2026-05-23_gpt-5.3-codex-spark_high_monogram_r1-r3`

Current archived grades:

- r1: MISS, selected `BunString__transferToJS`
- r2: MISS, selected `src/string.zig::toThreadSafeSlice`
- r3: FULL, selected `BunString__toThreadSafe`

## Analysis Focus

When adding new analysis for this case, compare against the archived spark-high r1-r3 batch and record:

- whether the agent reached `BunString__toThreadSafe`
- when it first saw `isolatedCopy` and `leakRef`
- whether it got trapped by `transferToJS`, `toThreadSafeSlice`, or `toCrossThreadShareable`
- whether `coupling` helped or returned a misleading empty result
- whether a new monogram hint would have changed the decision

Do not edit `runs/*/raw/`.
