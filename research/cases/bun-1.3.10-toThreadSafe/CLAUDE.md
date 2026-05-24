# Case Claude Guide: bun-1.3.10-toThreadSafe

Use this file when continuing analysis inside this case directory.

## Case Shape

- Symptom language/file: Zig `string.zig` corrupt value panic.
- Root cause language/file: C++ `BunString.cpp`.
- Correct symbol: `BunString__toThreadSafe`.
- Main decoys: `toThreadSafeSlice`, `BunString__transferToJS`, `toCrossThreadShareable`.

## Required Comparison

Every new run analysis should compare against:

`analysis/2026-05-23_spark-high-r1-r3.md`

Use the same categories:

- result state
- answer root cause
- successful path
- failure path
- coupling/hint issue
- monogram strengthening candidate
