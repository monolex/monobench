# Monogram Success/Failure By Experiment

Generated from monogram-named result logs still present under `~/.monobench` on 2026-05-24.

## Scope

Included roots:

- `/Users/macbook/.monobench/0.1.2-1779431036`
- `/Users/macbook/.monobench/0.1.3-1779431036`
- `/Users/macbook/.monobench/0.1.4-1779431036`
- `/Users/macbook/.monobench/0.1.6-1779528810`

Deduplication note:

`0.1.2-1779431036`, `0.1.3-1779431036`, and `0.1.4-1779431036` contain identical monogram answer hashes for the discovered runs. The canonical archive uses `0.1.2-1779431036`; the duplicate roots are recorded here as duplicates, not copied three times.

## Case Summary

| Case | Canonical Root | Monogram Runs | FULL | MISS | Archived Run |
|---|---|---:|---:|---:|---|
| `bun-1.3.10-toThreadSafe` | `0.1.2-1779431036` | 3 | 2 | 1 | `cases/bun-1.3.10-toThreadSafe/runs/2026-05-23_gpt-5.4-mini_low_monogram_r1-preindexed-r1-r2` |
| `bun-1.3.10-toThreadSafe` | `0.1.6-1779528810` | 3 | 1 | 2 | `cases/bun-1.3.10-toThreadSafe/runs/2026-05-23_gpt-5.3-codex-spark_high_monogram_r1-r3` |
| `cpython-147962-grouper-reentrant` | `0.1.2-1779431036` | 1 | 0 | 0 valid; 1 invalid | `cases/cpython-147962-grouper-reentrant/runs/2026-05-23_gpt-5.4-mini_low_monogram-preindexed-r1` |
| `ksmbd-37899` | `0.1.2-1779431036` | 1 | 1 | 0 | `cases/ksmbd-37899/runs/2026-05-23_gpt-5.5_low_monogram-r1` |

Overall canonical monogram logs, excluding invalid/provisional instances:

| Total Runs | FULL | MISS |
|---:|---:|---:|
| 7 | 4 | 3 |

## Detailed Runs

### bun-1.3.10-toThreadSafe / gpt-5.4-mini low

Canonical root: `/Users/macbook/.monobench/0.1.2-1779431036`

Duplicate roots with identical monogram answers:

- `/Users/macbook/.monobench/0.1.3-1779431036`
- `/Users/macbook/.monobench/0.1.4-1779431036`

| Run | Grade | Cost | Tokens | Time | Calls | Monogram Calls | Answer Root Cause |
|---|---:|---:|---:|---:|---:|---:|---|
| `monogram-gpt-5.4-mini-low-r1` | FULL | $2.31 | 5.37M | 209s | 31 | 27 | `BunString.cpp::BunString__toThreadSafe` |
| `monogram-preindexed-gpt-5.4-mini-low-r1` | MISS | $2.03 | 4.74M | 295s | 37 | 11 | `VirtualMachine.zig::refCountedResolvedSource` |
| `monogram-preindexed-gpt-5.4-mini-low-r2` | FULL | $1.48 | 3.41M | 138s | 35 | 30 | `BunString.cpp::BunString__toThreadSafe` |

Pattern:

- Success cases used monogram heavily enough to inspect the ownership path.
- The failed preindexed r1 used only 11 monogram calls and locked onto a different refcount-looking symbol.

### bun-1.3.10-toThreadSafe / gpt-5.3-codex-spark high

Canonical root: `/Users/macbook/.monobench/0.1.6-1779528810`

| Run | Grade | Cost | Tokens | Time | Calls | Monogram Calls | Answer Root Cause |
|---|---:|---:|---:|---:|---:|---:|---|
| `monogram-gpt-5.3-codex-spark-high-r1` | MISS | $3.19 | 11.56M | 790s | 166 | 165 | `BunString.cpp::BunString__transferToJS` |
| `monogram-gpt-5.3-codex-spark-high-r2` | MISS | $8.38 | 31.26M | 2223s | 489 | 450 | `src/string.zig::toThreadSafeSlice` |
| `monogram-gpt-5.3-codex-spark-high-r3` | FULL | $1.75 | 5.85M | 552s | 67 | 64 | `BunString.cpp::BunString__toThreadSafe` |

Pattern:

- r3 solved fastest and cheapest because it reached `isolatedCopy -> BunString__toThreadSafe -> leakRef` early.
- r1 stayed in the correct file and ownership domain but selected the downstream transfer helper.
- r2 had the highest monogram usage and cost, but repeated the wrong Zig-side compensation cone.

### cpython-147962-grouper-reentrant / gpt-5.4-mini low

Canonical root: `/Users/macbook/.monobench/0.1.2-1779431036`

Duplicate roots with identical monogram answers:

- `/Users/macbook/.monobench/0.1.3-1779431036`
- `/Users/macbook/.monobench/0.1.4-1779431036`

| Run | Grade | Cost | Tokens | Time | Calls | Monogram Calls | Answer Root Cause |
|---|---:|---:|---:|---:|---:|---:|---|
| `monogram-preindexed-gpt-5.4-mini-low-r1` | INVALID | $0.82 | 1.92M | 114s | 25 | 23 | `Modules/itertoolsmodule.c::_grouper_next` |

Note:

The answer names the same symbol as ground truth, but the instance metadata was a TODO scaffold. `monobench 0.1.7` now grades this as `INVALID`, not `MISS`, and excludes it from medians and hit-rate summaries.

### ksmbd-37899 / gpt-5.5 low

Canonical root: `/Users/macbook/.monobench/0.1.2-1779431036`

Duplicate roots with identical monogram answers:

- `/Users/macbook/.monobench/0.1.3-1779431036`
- `/Users/macbook/.monobench/0.1.4-1779431036`

| Run | Grade | Cost | Tokens | Time | Calls | Monogram Calls | Answer Root Cause |
|---|---:|---:|---:|---:|---:|---:|---|
| `monogram-low-r1` | FULL | $1.04 | 1.41M | 88s | 45 | 45 | `smb2pdu.c::smb2_session_logoff` |

Pattern:

- This is a clean monogram success, but not monogram-only: the baseline Opus and GPT-5.4-mini baselines also solved it in the archived report.

## Immediate Follow-Up

1. Author or remove provisional TODO instances before any future benchmark matrix.
2. Compare the two Bun batches directly: gpt-5.4-mini has 2/3 FULL while spark high has 1/3 FULL.
3. Extract command-sequence signatures for all FULL vs MISS Bun runs.
4. Use this index when selecting future monogram regression fixtures.
