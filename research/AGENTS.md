# Monobench Research Agent Guide

Use this directory for evidence-based monobench and monogram benchmark research.

## Required Workflow

1. Run `niia` first when starting work in this tree.
2. Use `niia-research` style: formulate a concrete question, gather evidence, record findings with citations, and mark hypotheses as VERIFIED, REFUTED, or INCONCLUSIVE.
3. Preserve original benchmark outputs under `cases/<case-id>/runs/<run-id>/raw/`.
4. Store checksums in the run directory as `SHA256SUMS`.
5. Store interpretation under `cases/<case-id>/analysis/`.

## Evidence Rules

- Cite local files with paths and line numbers when possible.
- Treat `raw/` as immutable source material.
- Do not rewrite agent logs, answer files, meter JSON, ground truth, or symptom files.
- If the monobench report and memory disagree, trust the archived report for that archive and document the disagreement.
- Separate observed facts from monogram-improvement hypotheses.

## Comparison Axes

Record these fields for every run:

- case id
- monobench root
- model and effort
- arm (`baseline`, `monogram`, or other)
- repetition id
- grade
- cost
- total tokens
- input/output/cache tokens when available
- wall time
- total tool calls
- monogram calls
- first monogram call position
- failure pattern
- successful discovery path, when present

## Monogram-Specific Review

When analyzing a monogram run, always check:

- Whether `monogram` was used early.
- Which command family dominated: `search`, `grep`, `symbols`, `context`, `chain`, `coupling`.
- Whether `context` hints directed the agent to a wrong key.
- Whether `coupling` empty output means no data, no match after filters, or a syntax mismatch.
- Whether the winning or failing path reveals a reusable ranking/hint pattern.

## Constraints

- Do not run git commands unless explicitly requested.
- Do not edit source code from this research tree.
- Do not modify files under `raw/`; copy a new run instead.
