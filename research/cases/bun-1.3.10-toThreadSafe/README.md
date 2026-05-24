# Case: bun-1.3.10-toThreadSafe

## Status

Initial spark-high monogram analysis archived.

## Source

Original monobench root:

`/Users/macbook/.monobench/0.1.6-1779528810`

Durable run archive:

`runs/2026-05-23_gpt-5.3-codex-spark_high_monogram_r1-r3/`

Primary analysis:

`analysis/2026-05-23_spark-high-r1-r3.md`

## Ground Truth

Root cause:

`src/bun.js/bindings/BunString.cpp::BunString__toThreadSafe`

Core bug:

`isolatedCopy()` creates a new impl and `str->impl.wtf = &impl.leakRef()` overwrites the old pointer without releasing the old ref.

## Current Archived Results

| Run | Grade | Answer Root Cause |
|---|---:|---|
| `monogram-gpt-5.3-codex-spark-high-r1` | MISS | `BunString__transferToJS` |
| `monogram-gpt-5.3-codex-spark-high-r2` | MISS | `src/string.zig::toThreadSafeSlice` |
| `monogram-gpt-5.3-codex-spark-high-r3` | FULL | `BunString__toThreadSafe` |

## Research Questions

- Why did r3 promote `BunString__toThreadSafe` while r1/r2 demoted it?
- Which monogram hints led r2 into `toThreadSafeSlice`?
- Should `coupling --pattern` support alternation or clearer empty-state output?
- Can ownership-risk ranking identify `isolatedCopy + leakRef + pointer overwrite` as a high-risk pattern?
