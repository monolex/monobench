# Monobench Research Archive

This directory stores benchmark research material that must survive cache cleanup and remain comparable across future runs.

## Layout

```text
research/
├── README.md
├── AGENTS.md
├── CLAUDE.md
├── indexes/                 # Cross-case indexes and score tables
├── templates/               # Reusable case/run analysis templates
└── cases/
    ├── AGENTS.md
    ├── CLAUDE.md
    └── <case-id>/
        ├── README.md        # Case-level summary
        ├── analysis/        # Human analysis documents
        └── runs/
            └── <date>_<model>_<effort>_<arm>_<reps>/
                ├── README.md
                ├── SHA256SUMS
                └── raw/     # Immutable copied originals
```

## Current Cases

| Case | Initial Run Archive | Notes |
|---|---|---|
| `bun-1.3.10-toThreadSafe` | `cases/bun-1.3.10-toThreadSafe/runs/2026-05-23_gpt-5.3-codex-spark_high_monogram_r1-r3` | Spark high monogram r1-r3, one FULL and two MISS in the current local report |
| `bun-1.3.10-toThreadSafe` | `cases/bun-1.3.10-toThreadSafe/runs/2026-05-23_gpt-5.4-mini_low_monogram_r1-preindexed-r1-r2` | gpt-5.4-mini low monogram/preindexed, two FULL and one MISS |
| `cpython-147962-grouper-reentrant` | `cases/cpython-147962-grouper-reentrant/runs/2026-05-23_gpt-5.4-mini_low_monogram-preindexed-r1` | Report grades MISS despite answer naming the ground-truth symbol; needs grading audit |
| `ksmbd-37899` | `cases/ksmbd-37899/runs/2026-05-23_gpt-5.5_low_monogram-r1` | Clean monogram FULL, but baseline also solved |

## Rules

- Raw logs are copied source material. Do not edit files under `raw/`.
- Put interpretation, scoring, and tool-improvement hypotheses under `analysis/`.
- Every conclusion needs evidence from a raw log, answer file, meter file, monobench report, source file, or ground truth document.
- If a remembered result differs from archived output, preserve the discrepancy and cite the archived output.
- Keep future cases in the same shape so run-level metrics can be compared mechanically.
