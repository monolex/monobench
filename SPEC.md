# monobench — specification

## 1. What it measures
Whether a code-intelligence tool improves an agent's **root-cause localization** on bugs where the
symptom and the cause are linked *structurally* (call graph / data flow / ownership), not *textually*.
Primary metric: **root-cause Hit-rate** and **tokens-per-correct-root-cause**. Secondary:
tool-adoption (calls), tool-call count, wall-time, forfeits.

monobench may be published as a general CLI, but its first priority is the Monolex recursive
development loop. The benchmark must preserve evidence that helps monogram maker, NIIA research, and
structural memory decide what to improve next: missing primitives, score terms, output budgets,
`[NEXT]` guidance, telemetry parsers, and proof surfaces. Public-facing aggregate scores are useful
only if this internal feedback remains trustworthy. The canonical loop is documented in
`research/indexes/loop-flow.md`.

## 2. Instance admission criteria
An instance is admissible only if ALL hold:

| # | Criterion |
|---|-----------|
| C1 | Symptom location ≠ root-cause location (ideally different file *and* language) |
| C2 | No text token in the symptom greps directly to the cause (the link is a call/ownership edge) |
| C3 | Objective ground truth (a merged fix commit / gold patch) |
| C4 | Contamination controlled (recent/obscure; record the risk) |
| C5 | **Discriminating** — see the admission gate |
| C6 | The tool's claimed capability is on the critical path (e.g. cross-language call chain) |

Bonus quality: a **decoy** (an adjacent function that looks plausible but is wrong) that punishes
shallow pattern-matching.

## 3. The admission gate (C5) — the core safeguard
**Run the baseline arm first.** Baseline gets the full depth directive (it is *told* to dig hard) but
no tool. If baseline reaches FULL at a rate above `admission.baseline_full_hit_rate_max` (default
0.5), the instance is **non-discriminating** → keep it for the record but down-weight it. Only
problems that defeat a hard-trying baseline measure tool value. This is what stops the benchmark
from being a grep-solvable toy.

## 4. Arms = pluggable tool adapters (fairness)
Each tool is a drop-in adapter `harness/tools/<name>/tool.json`:
`{ index_steps, skill, deliver: none|cli|mcp, mcp:{command,args}, forfeit_grep }`.
- **baseline** — `deliver:none`, no skill. Builtins only. The control + admission gate.
- **+tool** (monogram, codegraph, or your own) — its `index_steps` run in the repo; its `skill.md` (if any)
  is injected; `deliver:mcp` exposes it as first-class MCP tools.
- The shared depth directive (`prompts/depth.md`) is injected for **every** arm — the ONLY variable
  is the tool. A tool whose index output matches `forfeit_grep` is recorded **FORFEIT** (e.g.
  codegraph OOMs on Zig). Add a tool: `cp -r harness/tools/_TEMPLATE harness/tools/<name>`.

Model invocation is split into two axes:
- `--cli` — the CLI environment under test: `claude`, `codex`, `agy`, `gemini`, or `grok`.
- `--model` — the full model name/alias recorded for that CLI environment.
- `--via` — execution path: `direct` (default) or `niia` headless terminal.

This is intentionally not a single "runner" axis: `agy` may run a Claude model, and `claude` may run a
full model name rather than an alias. Result labels are `<tool>-<cli>-<model>-<effort>-rN-t<start_ms>`, so
`baseline-agy-claude-opus-4.1-low-r1-t1779581234567` and
`baseline-claude-claude-opus-4.1-low-r1-t1779581234568` remain distinct even if their run numbers match.
`rN` is an automatic repeat index, not the experiment's memory. Store human intent in
`<run>.meta.json` with `--tag`, `--note`, or `monobench note`; metadata must not alter artifact
identity or backward-compatible label parsing.
Analysis commands may use that metadata as a read-only filter; for example, `monogram-audit --tag`
isolates one repeated experiment batch without changing the underlying artifact identity.
`monogram-audit --json` must preserve the same text-surface facts as structured data, including
maker recommendations and the lib-niia-core maker-state bridge query provenance. Diagnostic pressure
layers must default to `affects_score:false` until a holdout loop proves they should influence the
maker SMPC score.
Run one CLI+model per `monobench matrix` command, then repeat the command for the next model. Run
**n ≥ 3** per arm for a median (these bugs have high variance).

For agy, direct `--print` still has no `--model` flag. Monobench keeps the model axis honest by
preflighting `~/.gemini/antigravity-cli/settings.json` and refusing a run when the configured model
does not match `--model`. The meter records `requested_model`, `requested_effort`, `observed_model`
when it can be parsed from agy logs, `model_enforced:true` only when the observed display label
normalizes back to the requested label, and `effort_enforced:false`. Agy cost/token fields are
unavailable rather than zero-valued measurements. Current agy labels include
`gemini-3.5-flash-low` (`Gemini 3.5 Flash (Low)`) and `gemini-3.5-flash-medium`
(`Gemini 3.5 Flash (Medium)`). For these agy Flash labels, Low/Medium is part of `--model`;
leave `--effort` empty unless a separate experiment intentionally needs another effort axis.

For grok (single model `grok-build`, OAuth/subscription auth), direct mode runs
`grok -p <prompt> --cwd <clone> --model grok-build --output-format json`. grok exposes no per-turn
token split and no cost, so `tokens`/`cost_usd` are null (never zero-valued measurements);
`tokens_available` and `cost_available` are false. The meter instead records honest per-session metrics
read from `~/.grok/sessions/<urlenc-cwd>/<sessionId>/signals.json` — `turns`, `tool_calls`,
`context_tokens_used`, `session_duration_s`, `avg_ttft_ms` — located by the `sessionId` in grok's JSON
envelope (robust to grok canonicalizing the cwd, e.g. `/tmp`→`/private/tmp`). `model_enforced` is true
when `signals.primaryModelId` matches the requested model; `effort_enforced` is false (`grok-build` has
`supports_reasoning_effort:false`).

## 5. Output contract & grading
Agent must end with `ROOTCAUSE: <file>::<fn>` and `FIX: <one sentence>`. `monobench grade` gives an
automatic keyword score:
- **FULL** — names `grading.full_must_name` AND a `grading.mechanism_keywords` term.
- **NAME_ONLY** — names the function but not the mechanism.
- **DECOY** — names a `grading.decoy_markers` function instead.
- **MISS** — none.
Also extracts cost, tokens, tool-call count, and **tool-adoption** (monogram/codegraph calls). Cost
or token medians exclude runs whose meter marks `cost_available:false` or `tokens_available:false`.
For final benchmark truth, run `monobench judge <id> <run>` in a separate answer-key-aware context,
then record the checked result with `monobench review <id> <run> --final <GRADE> --reason <TEXT>`.
The final review is stored as `results/<id>/<run>.review.json`.

## 6. Procedure — staged adaptive sampling (don't fix n blindly)
1. `monobench run` clones the repo at the pinned tag and forces it pristine (no fix applied);
   indexes per the tool adapter's Rust-executed `index_steps` (records OOM forfeit).
2. **n=1 — validity gate.** Run once and INSPECT before scaling: did the run produce a result, and
   for a tool arm, did the agent actually *use* the tool (`adopt > 0`)? If the tool wasn't used or
   the harness misbehaved, fix the adapter/prompt and redo — a non-using run is not a tool test.
3. **n=3 — signal.** If n=1 is valid, run to 3. Read `monobench report`: the **wobble** column
   (max CV of calls/cost) tells you if it's settled.
4. **Escalate only if wobbly.** `stable ✓` → stop at 3. `wobbly → n=5`. `high → n=9`. Different arms
   converge at different n (in practice monogram settles fast; baseline is high-variance and needs
   more). Don't pay for 9 runs on an arm that's already stable at 3.
5. Run the **admission gate** (baseline) the same way; if baseline FULL-solves cheaply *and stably*,
   the instance is non-discriminating → down-weight.
6. `monobench grade` per run → `monobench report` aggregates medians + ranges + the wobble verdict.

## 7. Validity notes (hard-won)
- **No reading the fix from git history.** The on-demand clone carries full history — *including the
  fix commit*. Run every arm git-blocked (`--disallowedTools "Bash(git:*)"`); otherwise an agent runs
  `git log --all --grep=<symptom>` → `git show <fix>` and copies the answer. Observed: opus baselines
  posting 3/3 "FULL" that were pure git-cheat (8 and 4 un-denied `git` calls), exposed by `monobench
  trace`. The git column in `monobench adoption` must read "all denied" for the run to count. This is
  non-negotiable and applies to EVERY arm, baseline included — a contaminated control invalidates the
  whole comparison.
- **A tool that the agent didn't use was not tested.** Report tool-adoption per run; if ~0, the run
  is invalid for the tool. The fix that worked: inject the tool's FULL reference (`initiate.md` +
  `skill.md`) into the `-p` prompt, and tell a CLI tool's agent to run it FIRST (a `lead.md` line) —
  merely listing the tool in a skill yielded ~5–13% adoption; docs-in-prompt + run-first lifted both
  adoption and correctness sharply. MCP delivery (forced into the toolset) is the other lever.
- **Equal effort across arms.** If only the tool arm is told to "dig deep," you measure the prompt,
  not the tool. `prompts/depth.md` is shared.
- **Index freshness.** Pre-index in the adapter so the agent doesn't burn a turn reindexing.
  In prepared monogram runs this is a hard runtime contract: the solver gets a PATH wrapper plus
  `MONOGRAM_PREPARED_INDEX=1`, and `monogram index`, `monogram reindex`, or `-r` / `--reindex`
  must return a compact guard + `[NEXT]` instead of mutating the prepared DB. A tiny/wrong DB is
  `HARNESS_DB_MISMATCH`, not permission to reindex inside the solver.
