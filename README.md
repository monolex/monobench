# monobench

**Does giving an AI coding agent a code-intelligence tool actually help it find the ROOT CAUSE of a
real bug — when the symptom and the cause are in different files/languages, linked by *structure*
(call graph / ownership / FFI boundary) rather than *text*?**

Most "tool helps" claims are tested on grep-solvable problems, where a strong model + plain grep wins
and the tool adds nothing. monobench only admits problems where a strong baseline — *told to dig just
as hard* — still struggles, then measures whether a tool (and which model) changes the outcome.

Think *"SWE-bench, but for cross-boundary fault localization, comparing code-intelligence tools."*
The axes are pluggable: the **tool** under test, the **CLI environment**, and the model full name.

monobench is public-shaped, but it is first an internal Monolex instrument. Its primary job is to
feed the monogram maker, NIIA research, and structural memory loops with clean evidence: which tool
behavior helped, where the solver drifted, which output shape caused loops, and which general
primitive or score term should be improved. Public reports matter, but they are downstream of that
recursive tool-development feedback. Canonical flow:
`research/indexes/loop-flow.md`.

## Layout
```
monobench/
├── bin/monobench            # the CLI — a single Rust binary (no-arg → help). Commands:
│                            #   list·tools·status·watch·stop·clean·show·run·matrix·grade·judge·review
│                            #   inspect·integrity·evidence
│                            #   trace·export·report·summary·column·adoption·monogram-audit·meter·add·version
├── Cargo.toml  src/         # Rust core: grade·report·adoption·trace·meter + util, and the native run.rs.
│                            #   One dependency (serde_json). `cargo build --release` → bin/monobench.
├── initiate/
│   ├── SKILL.md             # AI-discovery skill (mono-series / OpenCLIs initiate convention)
│   └── initiate.md          # full command reference (what `monobench` prints)
├── instances/<id>/          # the problem set (DATA, not code)
│   ├── instance.json        # repo + tag + ground_truth + grading + admission
│   ├── symptom.md           # the task given to the agent — NO root-cause spoilers
│   └── ground_truth.md      # the real fix (GATED — never fed to the solver)
├── harness/
│   ├── prompts/depth.md      # shared "dig deep" directive — EVERY arm gets it (fairness)
│   ├── tools/<name>/         # PLUGGABLE tool adapters: tool.json + injected docs (skill.md/initiate.md/lead.md)
│   │   ├── baseline/  monogram/  monogram-mcp/  codegraph/  _TEMPLATE/
├── results/<id>/             # run logs + grades (gitignored)
├── SPEC.md   CANDIDATES.md
```
The CLI is a **Rust binary** (analysis + the native runner) — its only build dependency is `serde_json`,
no Node. Repos-under-test are **cloned on demand** at the pinned tag (not vendored) → the bench stays small.

## Quickstart
```bash
monobench list                                 # instances
monobench tools                                # adapters (baseline · monogram · monogram-mcp · codegraph · yours)
monobench show  ghostty-8208-split-flicker     # the task (no spoilers)
monobench status ghostty-8208-split-flicker    # done/running/forfeit + time + output growth/age
monobench watch --live --every 5                # keep open in another terminal for live overview
monobench status ghostty-8208-split-flicker --live --every 5 --detail  # live per-instance detail
monobench prepare ghostty-8208-split-flicker --tools monogram  # pre-index once and snapshot the DB
# watch works from another cwd by remembering or inferring the real result root.
# Explicit override: MONOBENCH_ROOT=/path/to/monobench monobench watch

monobench matrix ghostty-8208-split-flicker \  # one CLI+model per command; repeat per model
    --tools baseline,monogram --cli claude --model haiku --runs 5 --jobs 6 \
    --tag haiku-pass --note "cheap admission + real monogram adoption check"
# Add monogram-mcp in a separate run only when delivery/interaction is the suspected variable.
monobench matrix ghostty-8208-split-flicker \
    --tools baseline,monogram --cli codex --model gpt-5.3-codex-spark --effort high --runs 2 --prepared \
    --tag lockfix-spark --note "prepared index lock fix 이후 재검증"
monobench matrix ksmbd-37899 \
    --tools baseline,monogram --cli agy --model gemini-3.5-flash-low --runs 2 --jobs 1 --prepared \
    --tag flashlow-ksmbd --note "agy settings.json must show Gemini 3.5 Flash (Low)"
monobench report   ghostty-8208-split-flicker  # per-CLI/MODEL grid: FULL Hit-rate · review status · med $/tokens/time · mono%
monobench report   bun-1.3.10-toThreadSafe --since 9h  # window to one session (all-time totals conflate old arms)
monobench summary                              # cross-instance FULL Hit-rate + median wall time
monobench column  baseline-codex-gpt-5.4-mini-low  # ONE arm's per-instance FULL/MISS/DECOY/INVALID + judged vs unreviewed
monobench adoption ghostty-8208-split-flicker  # per-run tool-call breakdown + git-integrity
monobench inspect ghostty-8208-split-flicker monogram-agy-gemini-3.5-flash-medium-medium-r1-t1779614150693
monobench note ghostty-8208-split-flicker monogram-agy-gemini-3.5-flash-medium-medium-r1-t1779614150693 \
    --tag suspect --note "registry race 의심; 실패 분석용"
monobench integrity ghostty-8208-split-flicker monogram-agy-gemini-3.5-flash-medium-medium-r1-t1779614150693
monobench evidence ghostty-8208-split-flicker monogram-claude-haiku-r1 --pattern '^/bin/zsh -lc|ROOTCAUSE|StringImpl::isolatedCopy'
monobench trace    ghostty-8208-split-flicker monogram-claude-haiku-r1  # one run's tool-call timeline
monobench export   ghostty-8208-split-flicker monogram-claude-haiku-r1  # full evidence markdown
monomento index . --project
monomento search "rootcause decoy monogram grep" --project --read --h2
monobench judge    ghostty-8208-split-flicker monogram-claude-haiku-r1 --model gpt-5.5 --write
monobench review   ghostty-8208-split-flicker monogram-claude-haiku-r1 --final FULL --reason "root cause and mechanism match" --judge-model gpt-5.5
monobench run   ghostty-8208-split-flicker baseline "quick baseline sanity"  # single run + note
monobench clean ghostty-8208-split-flicker baseline              # wipe an arm before a fresh re-bench
```
Requires `git`, the tool under test, and a model **CLI environment**: `--cli claude`, `--cli codex`,
`--cli agy`, `--cli gemini`, or `--cli grok`. The CLI binary itself has no Node dependency.

## Command flow
Every command ends with a `[NEXT]` hint, so the CLI is self-guiding — no command dead-ends (the same
discovery-graph reachability monogram keeps):

```
list ──→ show ──→ run ──→ grade ──→ judge / review
  └──→ status ──→ report ──→ summary ──→ column <arm>
         └──→ watch --live
tools ──→ run / matrix
{trace · adoption · monogram-audit} ──→ evidence ──→ export / integrity / trace
```

| goal | flow |
|------|------|
| compare a tool vs baseline | `run <id> baseline` → `run <id> monogram` → `report <id>` |
| investigate a MISS | `report <id>` → `evidence <id> <run> --pattern ROOTCAUSE` → `trace <id> <run>` → `export <id> <run>` |
| diagnose a monogram loop failure | `monogram-audit <id> --tag <batch>` or `--run <run>` → read `MAKER RECOMMENDATIONS` → `evidence <id> --pattern 'region_first_next|score-debug|ROOTCAUSE'` → classify path-not-closed vs closed-but-uncalibrated |
| validate before counting a run | `integrity <id>` → `inspect <id> <run>` → rerun if contaminated |
| scan conclusions across runs | `evidence <id> --pattern ROOTCAUSE` (index) → `evidence <id> <run>` (drill in) |
| watch live runs | `matrix <id> …` → `watch --live` / `status <id> --live` |
| cross-instance leaderboard | `summary` → `column <arm>` → `report <id>` |
| verify one arm fully judged | `column <arm>` (per-instance FULL/MISS/DECOY/INVALID + judged vs unreviewed) |
| isolate one session's score | `report <id> --since 9h` (all-time totals conflate old arms/configs) |

## Two pluggable dimensions
**Tools** — each is a drop-in adapter `harness/tools/<name>/tool.json`
(`index_steps` argv commands · `skill` to inject · `deliver`: none|cli|mcp · `forfeit_grep`). Add one:
`cp -r harness/tools/_TEMPLATE harness/tools/<name>`. Core shipped arms: **baseline**
(control/admission gate), **monogram** (real CLI + skill), **monogram-mcp** (same monogram index
through forced MCP tools), and **codegraph** (MCP; FORFEITs on repos it can't index, e.g. Zig).
For Monolex tool-development loops, start with `monogram`; add `monogram-mcp` only when delivery
surface is the question. Legacy prompt-load controls such as `monogram-thin` are not part of the
main recursive loop and should only be used for an explicitly named prompt-load experiment.

**CLI environments** (`--cli`) — `claude`, `codex`, `agy`, `gemini`, or `grok`. The model is always the full
model label in `--model`, so `--cli agy --model claude-opus-4.1` and
`--cli claude --model claude-opus-4.1` are distinct runs. Direct `agy --print` does not expose a
stable model/effort flag, so monobench reads `~/.gemini/antigravity-cli/settings.json` before the
run and refuses a model mismatch. It records `requested_model`, any `observed_model` parsed from agy
logs, and `model_enforced:true` only when the observed display label normalizes back to the requested
label. Current agy labels include `gemini-3.5-flash-low` (`Gemini 3.5 Flash (Low)`) and
`gemini-3.5-flash-medium` (`Gemini 3.5 Flash (Medium)`). For these labels, Low/Medium belongs in
`--model`; leave `--effort` empty unless a separate experiment intentionally needs another effort
axis. Effort remains label-only.

**Execution path** (`--via`) — `direct` (default: direct `claude -p`, `codex exec`, `agy --print`, or
`grok -p --output-format json`) or `niia` (model CLI driven through the niia headless terminal). Via-niia
agy uses `agy --print` with `--dangerously-skip-permissions` to avoid permission prompts being captured
as answers. Agy cost/token metering is marked unavailable (`cost_available:false`,
`tokens_available:false`) until a stable agy telemetry source is implemented. **grok** (single model
`grok-build`, OAuth subscription) likewise has no per-turn token split or cost, so `tokens`/`cost_usd`
are null; its meter instead carries honest session metrics read from
`~/.grok/sessions/<cwd>/<id>/signals.json` (turns, tool calls, `contextTokensUsed`, duration, TTFT),
located via the `sessionId` in grok's JSON envelope. `--effort` is label-only for grok (`grok-build`
has `supports_reasoning_effort:false`).

**Prepared mode** — `monobench prepare <id> --tools monogram` indexes the stable shared clone once,
then snapshots the resulting monogram SQLite DB under `results/<id>/_prepared/<tool>/`. With the
default worktree isolation, `matrix --prepared` copies that snapshot into each run's expected
per-worktree monogram DB path, rewrites stored absolute path prefixes to the run worktree, and skips
per-run indexing. During prepared monogram solver runs, monobench also installs a small PATH wrapper
and `MONOGRAM_PREPARED_INDEX=1`; `monogram index`, `monogram reindex`, and `-r` / `--reindex` return
a compact guard + `[NEXT]` instead of mutating the prepared DB. If a solver sees a tiny or wrong DB,
that is `HARNESS_DB_MISMATCH`, not permission to reindex. `--isolate shared` still works, but is
single-lane and reuses the stable clone DB directly.

Model selection is intentionally **one CLI+model per `matrix` command**. Use `--cli <name>
--model <full-name>` and repeat the same matrix command for the next model. Result labels are
`<tool>-<cli>-<model>-<effort>-rN-t<start_ms>`, which keeps the CLI environment, model, run number,
and start time from drifting apart. Legacy untimestamped `-rN` results still read normally, and
`trace`/`grade`/`export` can resolve an untimestamped prefix when it matches exactly one timestamped run.

`rN` is automatic repeat metadata, not the human memory surface. `matrix --runs N` assigns it for
each repeat; `run` defaults to `r1` unless you pass a legacy numeric index. Human intent belongs in
`results/<id>/<run>.meta.json`: pass `--tag` / `--note`, put non-numeric text after `run <id> <arm>`,
or annotate later with `monobench note <id> <run> --tag T --note "why this exists"`. `report` and
`inspect` surface that metadata without changing the artifact filename format.

## Run analysis memory
Use `monobench inspect <id> <run>` before tailing provider logs. It gives the current grade/review
status, artifact sizes and age, agy conversation/live transcript fallback, observed model, event
counts, active process hints, and a next action.

Use `monobench integrity <id> [run]` before counting a run in benchmark stats. It gives a heuristic
contamination score from observable signals: git history access, solver-side sqlite3/lock/registry
or index mutation, tool process kills, monogram re-indexing, stale prepared DB path/mtime anomalies,
and missing telemetry. A high score means "keep for failure analysis and rerun," not an automatic
final verdict.

Use `monobench monogram-audit <id> --tag <batch>` or `--run <run>` when tuning the recursive monogram
loop. The filter keeps fresh Haiku batches separate from older accumulated telemetry, so maker
recommendations reflect the current experiment rather than all historical runs.
For lifecycle/root-cause tuning, inspect proof-marker patterns such as `systems_lifecycle_next`,
`lifecycle_file_probe`, `region_contrast_lock`, and `rootcause_label_guard` before deciding whether
a failure is path-not-closed or closed-but-uncalibrated.

Use `monobench evidence <id> <run> --pattern 'A|B|^/bin/zsh -lc'` when you would otherwise run
`rg -n` against `results/<id>/<run>.err` or pipe `monogram-audit` into `tail`. It resolves the run
label, selects the right transcript/log, and prints both matching tool calls and raw line-numbered
matches from the source log, answer, and index artifacts. Raw source matches skip prompt/tool-doc
preamble by default; add `--include-prompt` when that preamble is the evidence you need.

Omit `<run>` (`monobench evidence <id> --pattern 'ROOTCAUSE'`) to scan **every** run at once. The
index lists each run with its conclusion line and four hit counts — `ans` (answer/conclusion hits),
`tool` (matching tool calls), `raw` (source+index log hits), `notable` (state/process calls) — ranked
so runs that concluded on the pattern come first. It is the read-only "which runs found X?" view;
drill into a single run once the index points you at one. Like `integrity`, it never mutates state.

Use `monobench trace <id> <run>` for a compact ordered timeline of tool calls. Use
`monobench export <id> <run>` when the run should become reusable evidence: it writes the full
transcript/log to `results/<id>/<run>.md`. Then run `monomento index . --project` and use
`monomento search` or `monomento peek` to compare success/failure cases without re-opening raw logs.
The exported markdown is intentionally verbose evidence; compact conclusions belong in `research/`
analysis notes that can be indexed separately.

For recursive monogram work, compare failure patterns inside a comparable cohort first: same
instance, CLI, model, effort, tool arm, monogram version or explicit experiment epoch, and
prepared-index policy. On a MISS, inspect the pre-failure prefix before the final wrong root: did
several runs reach the same region, follow the same NEXT, widen at the same command, or name the same
decoy? Repeated prefixes are maker-loop evidence; unrelated wrong roots are usually model variance
until a shared motif appears.

Read monogram failures through the same rule monogram uses: close the path first, then search within
it. If a run never reaches the right neighborhood, propose a structural primitive or staged frontier.
If it reaches the right neighborhood but a generic/name-decoy wins, propose trigram/facet calibration:
broad-term damping, query facet coverage, anchor coverage, graph propagation, coupling endpoint, or a
clear unknown-confidence marker. If the proof appears but is not promoted, improve compact proof
markers, output shaping, or NEXT.

## Tool-maker feedback

Use `trace`, `adoption`, `evidence`, `export`, and `monogram-audit` to produce maker proposals, not
solver hints. `monogram-audit` prints `MAKER RECOMMENDATIONS` when repeated command shapes suggest a
general monogram change. Treat those as implementation directions to investigate, not benchmark
answer hints. A good proposal says which general mechanism should change:

```text
observed pattern: broad symbol drift after reaching the right neighborhood
analysis method: symbol search + call graph + output budget
candidate change: broad-term damping + candidate-comparison NEXT
validation: failed case + prior FULL + unrelated holdout
```

Keep answer-key literals out of these notes unless they stay in gated review/ground-truth material.
The reusable output should be phrased as monogram/NIIA/monokist evidence: primitives, score terms,
proof markers, budgets, command-flow relationships, and holdout checks. See
`research/indexes/loop-flow.md` for the full loop.

So the full grid is **instances × tools × CLI environments × model full names** — e.g. reproduce
*"Codex CLI + GPT solved it, Agy CLI + Claude Opus didn't"* tool-by-tool.

## What it measures
- **root-cause Hit-rate (FULL)** — named the true root-cause function *with* the correct mechanism.
  `NAME_ONLY` = right function, weak mechanism · `DECOY` = the adjacent trap · `MISS` = neither.
- **two-stage grading** — stage 1 is deterministic automatic grading from the solver's final answer
  against `instance.json.grading`. Stage 2 is a checked review: `monobench judge` builds an
  answer-key-aware final-grader prompt, and the orchestrating LLM or a human records the decision
  with `monobench review` in `results/<id>/<run>.review.json`. `judge` does not call a model by
  itself; this keeps the solver process separate from the reviewer that may see ground truth.
- **tokens-per-correct-root-cause** — the load-bearing efficiency metric.
- **cache breakdown** — fresh input vs cache_read vs cache_creation + hit-% (matters: tool arms inject
  more context, which caches differently; `monometer daily` `no_cache_usd` = the un-cached cost).
- **tool-adoption** — tool calls the agent actually made (a tool it never called wasn't tested).
- **forfeit** — the tool couldn't index the repo at all.

## Fairness rules (see SPEC.md)
1. **Every arm gets the same depth directive** — only the *tool* differs.
2. **Admission gate** — run baseline first; if it reliably solves the instance cheaply, the instance
   is non-discriminating and is down-weighted. Only hard problems count.
3. **Symptom ≠ cause, no text bridge** — reject instances where grep on the symptom finds the cause.
4. **The answer key is never shown to the solver** — it gets only `symptom.md`; the grader holds
   `instance.json.grading` + `ground_truth.md`. An AI may *orchestrate* runs but must not be the
   *solver* in a context that has seen the key.
5. **No reading the fix from git history (git-blocked)** — every arm runs with `git` denied
   (`--disallowedTools "Bash(git:*)"`). The repo is cloned with full history, which *includes the fix
   commit*; without this an agent simply runs `git log --all --grep=<symptom>` → `git show <fix>` and
   copies the answer. We caught baselines doing exactly this (3/3 "FULL" that were pure git-cheat — see
   `monobench trace`), so all comparisons are git-blocked and `monobench adoption` reports the git
   attempts and confirms each was denied.
6. **The tool is forced into the prompt, not merely offered** — its full reference (`initiate.md` +
   `skill.md`) is shoved into the `-p` prompt, and for a CLI tool the agent is told to run it FIRST.
   Models ignore a tool they're only told exists (observed adoption ~5–13%); with the docs in-prompt +
   a run-first directive, adoption *and* correctness both jump. A tool the agent never called wasn't
   tested — `monobench adoption` makes that visible (mono% share + first-use call#).

## Adding an instance
`monobench add <id>` → fill `instance.json` (repo, tag, `ground_truth`, `grading`), `symptom.md`
(no spoilers), `ground_truth.md` (gated). Confirm it meets C1–C6 in SPEC.md. No code changes.

## Instances
- **`bun-1.3.10-toThreadSafe`** — cross-thread string refcount UAF that crashed Claude Code (Zig↔C++;
  fixed upstream by PR #30049). Category: memory-safety.
- **`ghostty-8208-split-flicker`** — the GTK split-tree flicker open ~6 months that **Codex 5.3
  solved and Opus 4.6 failed** (Zig/GTK; #8208). Category: UI async-ordering. codegraph forfeits (Zig).

See `CANDIDATES.md` for the backlog.
