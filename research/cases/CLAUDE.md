# Monobench Case Analysis Claude Guide

Analyze cases as repeatable evidence packages, not one-off notes.

## Rules

- Keep one case per directory.
- Keep one experiment batch per run directory.
- Keep one dated analysis file per analysis pass.
- Prefer tables for metrics and short prose for interpretation.
- Preserve uncertainty as `INCONCLUSIVE`, not guessed conclusions.

## Special Attention

For monogram-vs-baseline studies, look for:

- cases solved only by monogram
- cases where monogram increased cost without solving
- cases where monogram found the right file but selected the wrong symbol
- cases where hints or empty-state messages misled the agent
- cases where a command should be promoted, demoted, or rewritten
