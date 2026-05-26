# Current Loop State and Experiment Plan

Date: 2026-05-26

Viewpoint: `loop-flow.md`.

## Snapshot

Read-only commands used:

```bash
./bin/monobench list
./bin/monobench summary
./bin/monobench status bun-30185-getheapsnapshot-race --detail
./bin/monobench status cpython-142851-json-reentrant --detail
./bin/monobench report bun-30185-getheapsnapshot-race --since 24h
./bin/monobench report ghostty-8208-split-flicker --since 24h
./bin/monobench report bun-1.3.10-toThreadSafe --since 24h
./bin/monobench report ksmbd-37899 --since 24h
./bin/monobench monogram-audit bun-30185-getheapsnapshot-race
./bin/monobench monogram-audit ghostty-8208-split-flicker
./bin/monobench monogram-audit bun-1.3.10-toThreadSafe
./bin/monobench monogram-audit ksmbd-37899
./bin/monobench adoption bun-30185-getheapsnapshot-race
```

Active processes were present, so no new experiment was launched.

## Current Signals

### Strong positive signal

`ghostty-8208-split-flicker`:

- Haiku baseline: `5/10 FULL`.
- Haiku monogram: `16/16 FULL`.
- The successful monogram rail is UI/render timing: region-first, grep/context
  around rebuild/idle/setChild/split-tree behavior, then compact proof.

This is a good holdout for UI/render NEXT and region scoring. Do not tune only
for it; use it to catch regressions.

`ksmbd-37899`:

- Haiku baseline: `2/5 FULL`.
- Haiku monogram: `18/22 FULL`.
- GPT-5.4-mini-low baseline: `3/3 FULL`.
- GPT-5.4-mini-low monogram: `2/2 FULL`.

This is a good holdout for free/lifecycle/member-field proof. It is less useful
as a new tuning target because several baselines already solve it.

### Main negative signal

`bun-30185-getheapsnapshot-race`:

- Haiku baseline: `2/9 FULL`.
- Haiku monogram: `12/15 FULL`.
- Spark monogram: `10/13 FULL`.
- GPT-5.4-mini-low baseline: `7/9 FULL`.
- GPT-5.4-mini-low monogram: `0/5 FULL`.

This is the most important current experiment target because monogram improves
weak Haiku but hurts GPT-5.4-mini-low. The question is not "does monogram have
the evidence?" but "does the monogram surface steer this model into a broader or
wrong cone?"

Trace comparison:

- GPT-5.4-mini-low monogram MISS r1/r2 drifted into `NapiRef.cpp::ref`.
- GPT-5.4-mini-low monogram MISS r3 reached worker/JSWorker-adjacent code but
  finalized `BunProcess.cpp::Process::emitOnNextTick`.
- A Haiku monogram FULL run used fewer monogram calls, mixed in file/path
  discovery, followed `JSWorker` / `postTaskToWorkerGlobalScope`, and finalized
  the correct `JSWorker.cpp` body.

Method classification:

- code-analysis method: symbol lookup + call graph + ownership proof + output
  shaping;
- monogram primitive affected: region ranking and NEXT around symptom API vs
  generic ownership/refcount recipes;
- likely scoring issue: generic ownership terms can overpower symptom-specific
  API/path evidence for GPT-5.4-mini-low;
- likely prompt/output issue: no-arg `monogram` help is a 50KB+ dump in several
  GPT-5.4-mini-low runs, and may consume the model's early planning budget.

### Mature but still noisy target

`bun-1.3.10-toThreadSafe`:

- Haiku monogram is strong: `23/25 FULL`.
- Spark monogram is useful but still has expensive failures: `26/34 FULL`.
- GPT-5.4-mini-low monogram is currently weak on the available runs.

The audit still shows old hazards:

- `context --code >=100` repeated heavily;
- `chain --depth >=3`;
- `symbols "String" --json`;
- `coupling --domain ffi --all`;
- default `monogram` help treated as an oversized output;
- `help_exit_nonzero` and `bad_workdir_path` still appear in older traces.

This should be a score/budget regression suite, not the only tuning target.

## Current Infrastructure Watchpoints

1. Active stale run:
   `bun-30185-getheapsnapshot-race` has an active
   `monogram-codex-gpt-5.4-mini-low-r4-t1779734004452` with
   `telemetry_without_answer`. Do not count it until it finishes or is rerun.

2. Active Grok run:
   `cpython-142851-json-reentrant` has a direct `grok-build` baseline running.
   Treat this as CLI-adapter evidence first, not monogram scoring evidence.

3. Review debt:
   `monobench summary` reported hundreds of auto-only runs. Current analysis is
   good for pattern discovery, but final claims need judged/reviewed samples.

4. `monogram` no-arg help:
   `monogram-audit` repeatedly treats no-arg help as oversized and sometimes as
   nonzero. This is a monogram CLI/product issue first. Do not add a prompt-thin
   adapter as the first diagnostic axis; first reproduce with the real monogram
   arm, then compare MCP only if the delivery surface itself is the suspected
   variable.

## Experiments To Run Next

### Experiment A: monogram-only reproduction on bun-30185

Goal: determine whether the GPT-5.4-mini-low failure is a stable monogram CLI
steering pattern or just variance from the current run set.

Suggested run after active jobs clear:

```bash
./bin/monobench matrix bun-30185-getheapsnapshot-race \
  --tools monogram \
  --cli codex --model gpt-5.4-mini --effort low \
  --runs 3 --jobs 2 --prepared \
  --tag heap-mini-monogram-only
```

Read:

```bash
./bin/monobench report bun-30185-getheapsnapshot-race --since 3h
./bin/monobench adoption bun-30185-getheapsnapshot-race
./bin/monobench monogram-audit bun-30185-getheapsnapshot-race
```

Interpretation:

- repeated `NapiRef.cpp::ref` / event-loop wrong cone: region/NEXT/scoring issue;
- repeated right-neighborhood but different wrong roots: missing proof marker or
  model reasoning boundary;
- no repeated wrong cone and mixed grades: current evidence is too noisy; collect
  one more paired monogram batch before editing.

### Experiment A2: optional CLI vs MCP delivery check

Run this only after Experiment A shows that monogram CLI has stable evidence but
the model appears to use the CLI surface poorly. This is not the first step.

```bash
./bin/monobench matrix bun-30185-getheapsnapshot-race \
  --tools monogram,monogram-mcp \
  --cli codex --model gpt-5.4-mini --effort low \
  --runs 3 --jobs 2 --prepared \
  --tag heap-mini-cli-vs-mcp
```

Interpretation:

- monogram-mcp improves while CLI fails: delivery/interaction surface issue;
- both fail with the same wrong cone: scoring/NEXT/proof issue in monogram;
- both find right candidates but name unstable roots: missing proof marker or
  model reasoning boundary.

`monogram-thin` is intentionally excluded from the current loop. It can remain a
historical prompt-load control, but it adds a third variable before monogram
itself is understood.

### Experiment B: Symptom-anchor vs generic-ownership region scoring

Goal: make region scoring prefer symptom/API/path evidence before generic
ownership recipes when both are present.

Loop cycle 2026-05-26 executed the first source-side version of this idea, then
rejected a literal-tainted variant: `region` must separate domain anchor terms
from generic proof terms by universal probe words plus DB-wide DF/IDF broad-term
detection. If a query mixes symptom/API terms with ownership/refcount terms,
regions that only satisfy genuinely broad proof terms are damped; corpus-specific
symbols remain anchors unless the current index proves they are broad. Pure
ownership queries keep the prior ranking.

The first smoke exposed a second scoring prerequisite: `-n` was also shrinking
the internal search/raw/ref candidate pool, so `-n 5` could miss an anchor that
`-n 8` found. The loop now separates display limit from internal evidence scan:
`-n` caps shown regions, while region still scans a wider bounded pool before
scoring and truncation.

Do not hardcode benchmark symbols. Candidate score terms:

- symptom-term coverage;
- API/body/path co-location;
- caller/callee proximity from symptom entry to async boundary;
- universal/low-IDF ownership term damping when not co-located with symptom terms;
- candidate-comparison NEXT when generic refcount helpers compete with the
  symptom API rail.

Validation:

- direct smoke on the active prepared worktree moved the cross-thread
  symptom/API region from buried below generic proof helpers to rank 1, but that
  contaminated run is not accepted as evidence because it used a hardcoded
  corpus-symbol generic list;
- next validation must rerun after the DF/IDF genericness replacement;
- rerun bun-30185 failed GPT-5.4-mini-low arm after active r4 clears;
- rerun one prior Haiku FULL on bun-30185;
- rerun ghostty UI holdout;
- rerun ksmbd lifecycle holdout.

### Experiment C: monogram help/no-arg output budget

Goal: prevent no-arg `monogram` from becoming a 50KB+ planning dump while
preserving discoverability in the real monogram CLI.

Loop cycle 2026-05-26 implemented the first mechanism:

- no-arg `monogram` prints a compact starter, DB state, and search/region NEXT;
- `monogram help` and `monogram --help` keep the full reference;
- this preserves discovery while keeping accidental first-turn help output under
  the output-budget danger zone.

Remaining possible mechanisms:

- monobench's monogram adapter keeps using real monogram, but its lead may tell
  low-context models to start from symptom `region/search` rather than reread the
  whole no-arg help every run.

Do not solve this by making `monogram-thin` the default experiment. That hides
the real CLI problem instead of improving monogram.

Validation:

- bun-30185 GPT-5.4-mini-low: current monogram is `0/5`, baseline `7/9`.
- ghostty GPT-5.4-mini-low: current monogram is `4/8`, baseline `9/11`.

### Experiment D: monogram-audit maker-proposal mode

Goal: turn audit output from counters into loop-flow proposals.

Add a read-only report section that classifies:

- first divergence;
- method involved;
- primitive affected;
- candidate score/proof/budget/NEXT change;
- validation set suggestion.

This belongs in monobench, not solver prompts.

### Experiment E: toThreadSafe budget regression

Goal: make sure prior output-budget improvements continue to reduce expensive
Spark loops without hiding raw evidence.

Run only after current active jobs clear:

```bash
./bin/monobench matrix bun-1.3.10-toThreadSafe \
  --tools monogram \
  --cli codex --model gpt-5.4-mini --effort low \
  --runs 3 --jobs 2 --prepared \
  --tag tothreadsafe-mini-budget
```

Then inspect:

```bash
./bin/monobench monogram-audit bun-1.3.10-toThreadSafe
```

Watch specifically for:

- `symbols String --json`;
- `coupling --domain ffi --all`;
- `chain fromUTF8 --callers`;
- `context --code >=100` loops;
- `help_exit_nonzero`.

## Priority Order

1. Do not launch more runs until the active stale `bun-30185` and active Grok run
   are understood.
2. Run Experiment A as monogram-only. Do not add `monogram-thin`.
3. If the wrong cone repeats, implement Experiment B scoring/NEXT.
4. If the evidence looks good but CLI interaction seems to steer poorly, run A2
   with `monogram` vs `monogram-mcp`.
5. Fix monogram help/no-arg output as a product issue when it appears in traces;
   do not route around it with thin as the primary path.
6. Use ghostty and ksmbd as holdouts after any monogram change.
7. Add monogram-audit maker-proposal mode so future loops need less manual trace
   reading.
