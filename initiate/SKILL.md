---
name: monobench
description: >
  Benchmark whether a code-intelligence tool (monogram, codegraph, …) helps an AI agent find the
  ROOT CAUSE of a real bug, vs a baseline told to dig equally hard, on bugs where the symptom and
  the cause are linked structurally (call graph / ownership / FFI boundary) not textually. Runs an
  instance under each arm (clones the repo at a pinned buggy commit, indexes per tool, runs a model
  CLI, grades the ROOTCAUSE answer against a gated ground truth), with an ADMISSION GATE (only
  problems a hard-trying baseline fails are admitted) and a tool-ADOPTION metric. Use when asked to
  "benchmark a code tool", "does monogram/codegraph help", "add a benchmark bug", "run monobench",
  "evaluate a model on a real bug", or "fault-localization benchmark".
allowed-tools: Bash, Read
---

```bash
monobench
```

**The output above is your starting point — the full command reference. Read it before anything else.**

## When to Use
- Comparing whether a code-intelligence tool earns its keep (root-cause Hit-rate, tokens-per-correct).
- Comparing CLI environments and models on real bugs (`--cli agy --model claude-opus-4.1` is distinct
  from `--cli claude --model claude-opus-4.1`).
- Adding a new real bug as a reproducible benchmark instance.
- Checking that a candidate problem is even *fair* (the admission gate rejects grep-solvable toys).

## Command flow
Every command ends with a `[NEXT]` block, so the CLI is self-guiding and no command dead-ends
(the same discovery-graph reachability monogram keeps):

```
list ──→ show ──→ run ──→ grade ──→ judge / review
  └──→ status ──→ report ──→ summary
         └──→ watch --live
tools ──→ run / matrix
{trace · adoption · monogram-audit} ──→ evidence ──→ export / integrity / trace
```

Example flows (the detailed way to use it):
- Compare a tool vs baseline:   `run <id> baseline 1` → `run <id> monogram 1` → `report <id>`
- Investigate a MISS:           `report <id>` → `evidence <id> <run> --pattern ROOTCAUSE` → `trace <id> <run>` → `export <id> <run>`
- Validate before counting:     `integrity <id>` → `inspect <id> <run>` → rerun if contaminated
- Scan conclusions across runs: `evidence <id> --pattern ROOTCAUSE` (index) → `evidence <id> <run>` (drill in)
- Watch live runs:              `matrix <id> …` → `watch --live` / `status <id> --live`
- Cross-instance leaderboard:   `summary` → `report <id>`

## Workflow
1. **See what's there** — `monobench list`, then `monobench show <id>` (the task; never `--spoil` into a solver).
2. **One CLI+model per matrix** — run `monobench matrix <id> --tools baseline,monogram --cli <cli>
   --model <full-model> --runs 3 --jobs 2`. Repeat the command for the next model instead of passing
   multiple models at once; result labels are `<tool>-<cli>-<model>-<effort>-rN-t<start_ms>`.
   Add `--tag` / `--note` when a batch has a reason; `rN` is automatic repeat metadata, while human
   intent lives in `<run>.meta.json` and is shown by `report` / `inspect`.
3. **Admission gate first** — inspect the baseline rows. If baseline solves it cheaply, the instance is
   non-discriminating; pick a harder one.
4. **Tool arm** — compare against `monogram` / `monogram-mcp` rows. The monogram skill is injected and
   the agent is told to run `monogram` first; check `tool-adoption` in the grade (a tool not called =
   not tested).
5. **Run analysis** — use `monobench inspect <id> <run>` before tailing logs, `monobench integrity`
   before counting a run in benchmark stats, `monobench evidence <id> --pattern P` to scan every run
   for "which runs concluded on X?" (omit `<run>`), `monobench evidence <id> <run>` instead of ad hoc
   `rg`/`tail` for focused single-run evidence, `monobench trace` for a compact ordered tool-call
   timeline, and `monobench export` when the full transcript should become reusable markdown evidence.
   After export, run `monomento index . --project`, then search/peek the run later with monomento.
6. **Report** — `monobench report <id>` → per-arm FULL Hit-rate · median $ · median tokens · adoption.
7. **Add a bug** — `monobench add <id>`, then fill `instance.json` (repo, tag, ground_truth, grading),
   `symptom.md` (no spoilers), `ground_truth.md` (gated). Confirm it meets C1–C6 in SPEC.md.

## Integrity (don't break the benchmark)
- The SOLVER (the model being tested) is a fresh subprocess that receives ONLY `symptom.md`.
- The Rust grader (`monobench grade`) holds the answer key (`instance.json.grading` + `ground_truth.md`).
- An AI may orchestrate runs/grading, but must not also be the solver in the same context that has
  seen the key — that contaminates the result.
- `monobench integrity <id> [run]` is the first-pass contamination screen. High scores mean the run
  should be kept for failure analysis and rerun before inclusion in benchmark statistics.

## Integration with the mono-series
- **monogram / codegraph** — the tools under test (each is a `harness/tools/<tool>/tool.json` adapter;
  codegraph is recorded FORFEIT when it can't index a repo, e.g. Zig).
- **monomento** — indexes exported run markdown so success/failure transcripts can be searched and
  compared later without re-tailing raw provider logs.
- **monometer** — independent token/cost meter to cross-check per-run `total_cost_usd`.
- **niia headless terminal** — `--via niia` drives an interactive model CLI over a PTY and generalizes
  the benchmark to CLIs that do not have a direct runner.

## Full reference
See [initiate.md](initiate.md) for all commands, tool arms, CLI/model axes, metric, fairness rules,
and env vars.
