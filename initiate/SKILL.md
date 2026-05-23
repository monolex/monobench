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
- Comparing models on real bugs (the runner is pluggable → claude / codex / gemini × tool).
- Adding a new real bug as a reproducible benchmark instance.
- Checking that a candidate problem is even *fair* (the admission gate rejects grep-solvable toys).

## Workflow
1. **See what's there** — `monobench list`, then `monobench show <id>` (the task; never `--spoil` into a solver).
2. **Admission gate first** — `monobench run <id> baseline 1` (×3). If baseline solves it cheaply, the
   instance is non-discriminating; pick a harder one.
3. **Tool arm** — `monobench run <id> monogram 1` (×3). The monogram skill is injected and the agent
   is told to run `monogram` first; check `tool-adoption` in the grade (a tool not called = not tested).
4. **Report** — `monobench report <id>` → per-arm FULL Hit-rate · median $ · median tokens · adoption.
5. **Add a bug** — `monobench add <id>`, then fill `instance.json` (repo, tag, ground_truth, grading),
   `symptom.md` (no spoilers), `ground_truth.md` (gated). Confirm it meets C1–C6 in SPEC.md.

## Integrity (don't break the benchmark)
- The SOLVER (the model being tested) is a fresh subprocess that receives ONLY `symptom.md`.
- The GRADER (`grade.mjs`) holds the answer key (`instance.json.grading` + `ground_truth.md`).
- An AI may orchestrate runs/grading, but must not also be the solver in the same context that has
  seen the key — that contaminates the result.

## Integration with the mono-series
- **monogram / codegraph** — the tools under test (each is a `harness/tools/<tool>/tool.json` adapter;
  codegraph is recorded FORFEIT when it can't index a repo, e.g. Zig).
- **monometer** — independent token/cost meter to cross-check per-run `total_cost_usd`.
- **monoterm / monolex-headless** — the planned `pty` runner drives an interactive model CLI over a
  PTY (off metered `claude -p`), and generalizes the benchmark to any model CLI.

## Full reference
See [initiate.md](initiate.md) for all commands, arms/runners, metric, fairness rules, and env vars.
