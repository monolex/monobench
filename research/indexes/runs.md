# Monobench Research Run Index

This index is manually maintained as durable run archives are added under `research/cases/`.

| Date | Case | Run Archive | Model | Effort | Arm | Reps | Grades | Key Finding |
|---|---|---|---|---|---|---:|---|---|
| 2026-05-23 | `bun-1.3.10-toThreadSafe` | `cases/bun-1.3.10-toThreadSafe/runs/2026-05-23_gpt-5.3-codex-spark_high_monogram_r1-r3` | `gpt-5.3-codex-spark` | `high` | `monogram` | 3 | `MISS, MISS, FULL` | r3 succeeded by following `isolatedCopy -> BunString__toThreadSafe -> leakRef`; r1/r2 were pulled into downstream or compensating ownership paths. |
| 2026-05-23 | `bun-1.3.10-toThreadSafe` | `cases/bun-1.3.10-toThreadSafe/runs/2026-05-23_gpt-5.4-mini_low_monogram_r1-preindexed-r1-r2` | `gpt-5.4-mini` | `low` | `monogram + monogram-preindexed` | 3 | `FULL, MISS, FULL` | Same Bun ownership case; preindexed r1 failed by selecting `VirtualMachine.zig::refCountedResolvedSource`. |
| 2026-05-23 | `cpython-147962-grouper-reentrant` | `cases/cpython-147962-grouper-reentrant/runs/2026-05-23_gpt-5.4-mini_low_monogram-preindexed-r1` | `gpt-5.4-mini` | `low` | `monogram-preindexed` | 1 | `INVALID` | Instance metadata was still a TODO scaffold; monobench 0.1.7 now excludes it from hit-rate summaries. |
| 2026-05-23 | `ksmbd-37899` | `cases/ksmbd-37899/runs/2026-05-23_gpt-5.5_low_monogram-r1` | `gpt-5.5` | `low` | `monogram` | 1 | `FULL` | Correctly selected `smb2_session_logoff`, but baseline also solved this case. |

## Pending Comparison Questions

- Which future cases are solved only by monogram?
- Which monogram hints increase cost without improving grade?
- Which command families correlate with FULL results?
- Which empty-state messages cause wrong tool interpretation?
- Which provisional TODO instances should be authored vs removed from the benchmark set?
