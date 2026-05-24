# Case: cpython-147962-grouper-reentrant

## Status

Initial monogram log archived.

## Source

Original monobench root:

`/Users/macbook/.monobench/0.1.2-1779431036`

Durable run archive:

`runs/2026-05-23_gpt-5.4-mini_low_monogram-preindexed-r1/`

## Ground Truth

Root cause:

`Modules/itertoolsmodule.c::_grouper_next`

Mechanism:

`_grouper_next()` performs equality comparison without strong references to `tgtkey` and `currkey`; user-defined `__eq__` can re-enter and free one of those objects during `PyObject_RichCompareBool`.

## Current Archived Results

| Run | Grade | Answer Root Cause |
|---|---:|---|
| `monogram-preindexed-gpt-5.4-mini-low-r1` | INVALID | `Modules/itertoolsmodule.c::_grouper_next` |

## Research Questions

- Should this provisional instance be authored fully or removed from the benchmark set?
- Should this case be used as an invalid-instance regression fixture?
