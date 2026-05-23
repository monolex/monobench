# monobench — specification

## 1. What it measures
Whether a code-intelligence tool improves an agent's **root-cause localization** on bugs where the
symptom and the cause are linked *structurally* (call graph / data flow / ownership), not *textually*.
Primary metric: **root-cause Hit-rate** and **tokens-per-correct-root-cause**. Secondary:
tool-adoption (calls), tool-call count, wall-time, forfeits.

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

The model invocation is a pluggable **runner** (`MONOBENCH_RUNNER`):
- `claude-p` (default) — headless `claude -p`, cost/tokens from `--output-format json`.
- `niia` — interactive model CLI (claude/codex/gemini) over the niia headless terminal, OFF metered
  `-p`, metered per-run by **monometer incl. cache** through the Rust niia runner + `src/meter.rs`.
Both run parent-stripped (`--setting-sources '' --disable-slash-commands --strict-mcp-config`) with a
`--max-budget-usd` cap. Run **n ≥ 3** per arm for a median (these bugs have high variance).

## 5. Output contract & grading
Agent must end with `ROOTCAUSE: <file>::<fn>` and `FIX: <one sentence>`. `grade.mjs` scores:
- **FULL** — names `grading.full_must_name` AND a `grading.mechanism_keywords` term.
- **NAME_ONLY** — names the function but not the mechanism.
- **DECOY** — names a `grading.decoy_markers` function instead.
- **MISS** — none.
Also extracts cost, tokens, tool-call count, and **tool-adoption** (monogram/codegraph calls).

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
6. `grade.mjs` per run → `monobench report` aggregates medians + ranges + the wobble verdict.

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
