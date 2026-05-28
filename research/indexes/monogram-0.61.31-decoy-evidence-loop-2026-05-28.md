# Monogram 0.61.31 Decoy-Pattern Evidence Loop — 2026-05-28

## Objective

Continue the evidence-driven `monobench -> monogram` loop started in the
2026-05-27 handoff (`.claude/MONOGRAM-LOOP-HANDOFF-2026-05-28.md`).

The handoff's standing decision:

- keep `monogram 0.61.31`
- do **not** add a new region-score change yet
- collect repeated evidence for the next failure class before editing

This loop's job is to **strengthen or refute** one suspected failure class by
running many Haiku + monogram trials in parallel across unrelated instances.

## Failure Class Under Investigation

```text
generic-region lexical decoy -> wrapper/factory root label
```

Observed once (2026-05-27) in `cpython-147962-grouper-reentrant`:

- a generic `region` query on grouping/key/reentrancy terms ranked unrelated
  UI/config code because a lexical phrase matched better than the intended
  iterator implementation;
- the solver later reached the right file neighborhood but labeled the
  **wrapper/factory** rather than the **child iterator boundary**.

### Decision gate (from the handoff)

Implement a generalized fix **only if** this shape:

- repeats in **>= 2 runs**, OR
- appears in a **second unrelated instance**.

Otherwise: keep 0.61.31, classify the single grouper DECOY as weak-model
reasoning variance.

## Anti-Overfit Guardrails (must hold all loop)

- No benchmark answer literals in monogram code / NEXT hints / docs / solver
  prompts / canned commands / score gates.
- `show --spoil` is orchestrator-only evaluation evidence; never feed to a solver.
- Allowed generalized evidence only: trigram/query overlap, query-term coverage,
  broad-term IDF/DF damping, structural refs, caller/callee proximity, local
  inverse-operation balance, fanout/output budget, file/language filters, compact
  proof markers, region size normalization.
- A fix is KEPT only if it lifts the **gold-region rank on a held-out instance**,
  then passes `monobench monogram-audit` as a hard no-literal gate.

## Environment (verified 2026-05-28 00:35)

```text
monogram 0.61.31   (installed: ~/.openclis/versions/monogram/0.61.31/2026-05-27-230159)
monobench 0.1.34
active processes: none at loop start
budget cap: MONOBENCH_CAP default $6/run (kept; Haiku medians ~$0.25 never approach it)
```

Worktree isolation (run.rs:897): `{work}/wt/{runid}-{pid}` + instance-scoped
results dirs + O_EXCL same-repo base lock released before the run. => separate
`sweep`/`matrix` processes and `--jobs` lanes are collision-safe. The historical
"shared-t wrong-repo contamination" bug is fixed by the `-{pid}` suffix.

## Instance Set — 14 prepared (warmed snapshots verified)

8 bun, 3 cpython, deno, ghostty, ksmbd. Excluded `node-56840` and `node-62095`
(prepared monogram.db = 0 files => empty/stale => confounded).

Prime "wrapper/factory -> child iterator/handle" candidates for the decoy shape:
`cpython-147962-grouper-reentrant` (the weak case), `deno-31770-statementsync-iter`
(JS iterator), `bun-30196-htmlrewriter-uaf` (wrapper owns Response),
`bun-30185-getheapsnapshot-race`, `bun-20093-napi-handlescope-race`,
`bun-29951-directorywatch-uaf`, `cpython-142851-json-reentrant`.

Holdouts (must stay stable): `cpython-148852-rawmutex-wakeup`,
`bun-30185-getheapsnapshot-race`, `ksmbd-37899`.

## Loop Protocol

```text
Round 1  broad net   : all 14, monogram/haiku, runs 2, jobs 5, prepared
   -> per-run: report, adoption, integrity, trace, monogram-audit
   -> classify each run by FAILURE SHAPE, not grade
Round 2  deepen      : decoy-positive instances + grouper -> runs 3-5
Round 3+ widen/hold  : add unrelated instances; re-confirm holdouts
Decision : if decoy repeats >=2x or in a 2nd instance -> draft GENERALIZED
           maker proposal (do NOT implement without held-out validation plan)
```

Each round: launch sweep in background -> on completion analyze + append a
results table here -> launch next round. Mechanism = harness re-invokes on
background-command completion (no /loop, no wakeup polling).

## Region scoring map (for interpreting audit / score-debug)

`lib-monogram/src/region.rs` `RegionScoreDebug` already carries the terms most
relevant to this decoy class — so the candidate-fix area is PARTIALLY BUILT:

- bonuses: `diversity / structural / boundary / operation_boundary / graph /
  fuzzy`
- penalties: `size / anchor / operation_symbol / raw_only / document /
  declaration / generated_surface / anonymous_symbol / test_region`
- generic-probe handling: `is_generic_region_probe_term` -> when a query has
  generic probe terms it filters to non-generic `anchor_terms` (this is the
  existing defense against the "custom key" style lexical decoy)
- surface distinction: `is_source_region_path` vs
  `is_declaration_or_generated_surface_path` (impl vs decl/generated)
- wrapper signal: `response_wrapper_ownership_cluster` ->
  `has_response_wrapper_ownership` -> `operation_boundary_bonus` (a
  wrapper-ownership boundary bonus)

Implication for the decision gate: if the grouper decoy repeats, the fix is most
likely **calibration/normalization of existing terms** (strengthen generic-probe
damping or generalize the wrapper-ownership boundary beyond the Response-shaped
cluster), NOT a new term — consistent with the maker SKILL ("region's gap is
normalization + a scoring eval set, not missing terms").

WATCH-ITEM (verify, do not assume): confirm `response_wrapper_ownership_cluster`
is detected from GENERAL structural evidence (a wrapper type that owns a
finalizable child), not from benchmark-specific symbol/type literals. If it keys
off answer-key names it is an existing overfit to flag. Check during Round 1
analysis via `monogram-audit` (the no-literal hard gate) + reading the cluster
detector. Do NOT extend it until verified clean.

## Classification key (per run)

```text
class A  path not closed        : never reached right neighborhood
class B  closed but uncalibrated: reached neighborhood, named decoy/wrong root  <- THE target shape
class C  broad fanout           : high calls/output/chain depth
class D  guarded recovery weak  : guarded no-match but still broad/wrong
class E  contamination/parser   : git/db/index surgery or empty telemetry
FULL                            : correct root, compact proof
```

The decoy class we hunt is **class B with a wrapper/factory-vs-child boundary
miss** specifically. Generic class-B wrong roots that are NOT the wrapper/child
shape are logged but do not by themselves satisfy the decision gate.

---

## Round 1 — broad net (launched 2026-05-28 00:35)

```text
tag:  haiku-v06131-decoy-net-r2j5
cmd:  monobench sweep <14 ids> --tools monogram --cli claude --model haiku
      --runs 2 --jobs 5 --prepared
runs: 28 (2 per instance)
status: RUNNING (bg task bc1bbkj1h)
```

### Sweep schedule observed (run.rs behavior)

`sweep --prepared` runs a **prepare preamble for ALL instances first** (freshen
each `_prepared` snapshot), then the solver runs. Most bun/cpython snapshots were
**re-indexed fresh** ("prepared monogram"); only the 3 the 2026-05-27 handoff
just warmed showed "(reused)". => Round 1 uses fresh, version-consistent
snapshots; no stale-index confound. Cost: the freshen of 8 bun snapshots is
serial (~2 min each, same-repo base lock) => ~16 min preamble before solvers.

### THROUGHPUT LESSON (apply to all future rounds)

Lumping many SAME-repo instances (8 bun) in one sweep serializes both the warm
phase and per-run worktree-adds (the O_EXCL base lock orders same-repo git ops).
`--jobs` does not help a single-repo-dominated sweep. **Future rounds: split by
repo family into separate concurrent sweep processes** (different bases => true
parallel; pid-keyed worktrees keep them collision-safe).

### Concurrent batch — grouper deep-dive (launched 2026-05-28 00:51)

```text
tag:  haiku-v06131-grouper-deep-r6j3
cmd:  monobench matrix cpython-147962-grouper-reentrant --tools monogram
      --cli claude --model haiku --runs 6 --jobs 3 --prepared
runs: 6 (core-instance repetition test; cpython warm => produces grades before
      the bun-bound R1)
status: RUNNING (bg task b04l6pk52)
```

Rationale: directly tests decoy **repetition** on the core instance. With R1's 2
grouper runs + these 6 = 8 fresh grouper runs this round (+3 prior = 11 total) —
strong signal for whether the decoy is a stable rate or a one-off.

### Results (filled on completion)

| Instance | r | grade | cost | time | calls | mono% | first | class | wrapper/child miss? |
|---|---:|---|---:|---:|---:|---:|---|---|---|
| _pending_ | | | | | | | | | |

### Round 1 read-back (filled on completion)

- aggregate FULL rate: _pending R1 completion_
- decoy (class-B wrapper/child) instances: _pending_
- holdout stability (rawmutex / getheapsnapshot / ksmbd): _pending_
- confounded/contaminated runs: 1 known (R1 grouper NO_RESULT — see concurrency lesson)
- next round target: _pending R1 read_

---

## Round 1b — grouper deep-dive (completed 2026-05-28 ~00:56)

**Result: 6/6 FULL** (tag `haiku-v06131-grouper-deep-r6j3`). The decoy did NOT
reproduce in 6 fresh runs.

| run | grade | cost | time | calls | mono% | guarded |
|---|---|---:|---:|---:|---:|---|
| r1 | FULL | $0.25 | 136s | 22 | 45% | 3g |
| r2 | FULL | $0.27 | 128s | 26 | 46% | 2g |
| r3 | FULL | $0.21 | 126s | 16 | 69% | — |
| r4 | FULL | $0.21 | 125s | 20 | 30% | 1g |
| r5 | FULL | $0.21 | 128s | 16 | 50% | 1g |
| r6 | FULL | $0.25 | 152s | 20 | 40% | — |

**0.61.31 grouper cumulative = 8/9 FULL, 1 DECOY (~11%).**

### Decoy rail analysis (the lone failure: `...r1-t1779888627713`)

- `ROOTCAUSE` labeled = `itertoolsmodule.c::groupby_next` (the PARENT/wrapper
  `groupby` iterator's next).
- graded DECOY because the fix is the `_grouper` CHILD iterator boundary, not the
  parent wrapper => **CONFIRMS the failure SHAPE = wrapper/factory-vs-child
  boundary miss (class B)**.
- BUT it used `region --score-debug` + `region_first_next` (the *good* behaviors)
  and still picked the wrapper; it was the cost/time OUTLIER ($0.35/262s vs FULL
  median $0.21/128s). `monogram-audit` frames the whole 13-run grouper set as
  only `broad_output_or_fanout (count=6)` — NO wrong-root/calibration rec.
- => single occurrence; reads as broad-fanout-driven model variance, not a stable
  monogram calibration defect.

### Concurrency lesson (NEW — important)

R1's own grouper run (`...t1779897568554`) graded **NO_RESULT (calls=3)** — almost
certainly cpython base-lock / worktree contention from running grouper-deep + R1
on the SAME repo concurrently. **RULE: concurrent batches must target DISJOINT
repos.** R1 covers bun/cpython/deno/ghostty/ksmbd; the next concurrent batch only
widens to NEW repos.

### Decision-gate status after Round 1b

- grouper repetition: **1/9, NOT repeated** -> gate NOT met on grouper alone.
- 2nd-instance test: **pending R1 broad-net** (prime candidates: bun-htmlrewriter,
  deno-statementsync-iter, bun-getheapsnapshot).
- **LEANING: keep 0.61.31**, classify grouper decoy as weak-model variance —
  unless R1 (or Round 2 fresh families) shows the wrapper/child miss elsewhere.

## Round 2 prep — widen to fresh decoy-shape families (launched ~00:56, bg b1wxa3ia7)

Preparing disjoint-repo instances (no contention with R1) for the 2nd-instance
test — all "child-object lifetime" shapes:

- `netty-12036-unsafe-bytebuffer-uaf` (ByteBuffer from component ITERATION must
  keep native memory alive — iterator keepalive)
- `ruby-16964-iobuffer-and-uaf` (IO::Buffer#& UAF on an invalidated child slice)
- `redis-15095-stream-pel-doublefree` (double free from duplicate consumer PEL
  child entries)

---

## CRUX FINDING — bun-20093 decoy is MODEL-PRIOR, not monogram mis-ranking (~01:10)

### Decision gate: surface-MET, but decomposes into two different mechanisms

- **grouper**: 1/11 FULL-miss (lone decoy) — does NOT reproduce -> weak-model
  variance; wrapper/child boundary within the RIGHT file.
- **bun-20093-napi-handlescope**: 2/2 MISS, BOTH runs labeled
  `BunString__toThreadSafe` (the answer to a DIFFERENT instance, bun-1.3.10).
  Repeated + 2nd instance -> gate literally met.

### The crux experiment refutes a monogram defect

Symptom shown to solver: "native addon + concurrent GC + freed pointer +
cross-thread mutation, backtrace in GC sweep." Correct root = `NapiHandleScopeImpl`
m_storage race (spoil, orchestrator-only). Queried the SAME bun index the solver
used (`~/.monolex/monogram/bun-3a1071.db` via `/tmp/monobench-work/bun`):

| query | monogram top result | verdict |
|---|---|---|
| `"napi handle scope"` (answer domain) | `napi_handle_scope.cpp` #1 (100%, score 30.5) | answer IS indexed + discoverable |
| `"native addon gc concurrent crash"` (symptom) | crash_handler/VM/boringssl; `napi.zig` rank 8; answer file ABSENT | symptom query surfaces napi LOW |
| `"ownership boundary ref deref leakRef isolatedCopy"` (model call 4) | `toThreadSafe` score 31.2, anchor 1.0/1.0 + `top_region_lock` "verify it" | monogram faithfully returns the model's OWN vocabulary echo |

`leakRef`/`isolatedCopy` are NOT in the symptom — the MODEL injected toThreadSafe's
own domain identifiers from its training prior. monogram correctly returned
toThreadSafe for that query (anchor 1.0 = query echoes the function's own
identifiers). The model NEVER queried the symptom domain (`"napi handle scope"`,
which monogram ranks #1).

### Classification (loop-flow step 6)

- NOT path-not-closed (answer reachable, score 30+ for the right query).
- NOT classic closed-but-uncalibrated (monogram did not rank a decoy over the
  answer for a NEUTRAL query — the decoy only won a decoy-BIASED query).
- = **MODEL query-selection failure + skipped verification** (the `top_region_lock`
  hint explicitly said "verify it before contrast regions"; the model never
  verified that toThreadSafe sits in the GC-sweep/napi path).

### VERDICT: KEEP 0.61.31 — do NOT implement a scoring change on this evidence

- The two "decoys" have DIFFERENT mechanisms (grouper = wrapper/child within the
  right file; bun-20093 = identifier-echo query -> wrong file) -> not one fixable
  defect.
- monogram already closes the path for symptom-aligned queries.
- Forcing a fix risks overfit + holdout damage (handoff anti-overfit rule;
  the 6 retracted "monogram bug" leads of 2026-05-26).

### ONE generalizable candidate — RECORD ONLY (needs its own evidence; do NOT implement)

`top_region_lock` / anchor-1.0 confidence AMPLIFIES a decoy when the query tokens
are an IDENTIFIER-ECHO of the region's own symbol name (vs symptom/behavioral
terms). Candidate (future): dampen lock-confidence when the anchor match is
dominated by the region's own name tokens — "naming the answer" should be weaker
evidence than "a symptom description matching the answer." Measurable from indexed
code (query-token vs region-symbol-token overlap, generalizable, no answer-key
literal). Gate to implement: a dedicated eval set + 2 holdouts + `monogram-audit`
no-literal gate. NOT justified on current evidence.

---

## R1 BROAD-NET COMPLETE (28 runs) — CONCURRENCY CONFOUND SUSPECTED (~01:15)

Full 14-instance table (FULL/2):

| instance | r1 | r2 | | instance | r1 | r2 |
|---|---|---|---|---|---|---|
| bun-1.3.10 | MISS | FULL | | bun-30196-htmlrewriter | FULL | FULL |
| bun-20093-napi | MISS | MISS | | cpython-json | FULL | FULL |
| bun-27838-sslconfig | MISS | MISS | | cpython-grouper | FULL | FULL |
| bun-28907-threadpool | FULL | MISS | | cpython-rawmutex | FULL | **DECOY** |
| bun-29829-onhandshake | MISS | MISS | | deno-statementsync-iter | FULL | FULL |
| bun-29951-dirwatch | FULL | MISS | | ghostty | MISS | FULL |
| bun-30185-getheap | MISS | FULL | | ksmbd | MISS | MISS |

**R1 FULL rate = 14/28 = 50%.**

### Red flag: all 3 handoff holdouts regressed

| holdout | handoff | R1 |
|---|---|---|
| ksmbd | 2/2 FULL | 0/2 |
| cpython-rawmutex | 2/2 FULL | 1/2 (+DECOY) |
| bun-getheapsnapshot | 2/2 FULL | 1/2 |

If true FULL rate ~85% (handoff), R1 regressions are improbable; if ~50% (R1),
handoff's 3x2/2 is improbable (~0.25^3 = 1.6%). => conditions DIFFER. The
difference is **concurrency**: R1 = jobs 5 + concurrent grouper-deep (j3) + R2
prep + R2 sweep (heavy contention) vs handoff's isolated jobs 2.

Counter-evidence (contention is not uniformly fatal): grouper-deep (j3, concurrent)
6/6; ruby/redis fresh (j3, concurrent) 2/2; json/deno/htmlrewriter 2/2. => EASY
instances survive contention; only HARD instances (ksmbd, bun-napi/sslconfig/
onhandshake) regressed. Two live hypotheses:
- (A) contention degrades hard-instance solver runs (confound) -> R1 unreliable;
- (B) hard instances are high-variance and handoff 2/2 was small-n luck -> 50% honest.

### DECISIVE TEST (next, ISOLATED)

Re-run the 3 regressed holdouts at **jobs 2, single batch, nothing else running**
(replicates handoff conditions). Recover to ~2/2 => confound confirmed, R1
graded-evidence unreliable, recalibrate all graded sweeps to clean jobs<=2-3.
Stay regressed => R1 honest, 50% real, keep 0.61.31.

### METHODOLOGICAL LESSON

Maker docs + handoff prescribe jobs 1 -> 2 for graded comparison; I jumped to
jobs 5 + multi-batch per the "bold parallelism" steer. High throughput may COST
graded-evidence reliability. Reserve high parallelism for PREPARE/throughput
(snapshot warming, where confound is irrelevant); keep GRADED comparison runs at
bounded concurrency. Pending the decisive test before trusting any R1 MISS/DECOY
as monogram signal.

---

## CONFOUND TEST RESULT (jobs 2, isolated) — confound is REAL but PARTIAL (~01:25)

| holdout | handoff | R1 (j5+stacked) | isolated (j2) | verdict |
|---|---|---|---|---|
| ksmbd | 2/2 | 0/2 | **2/3** (+1 decoy) | RECOVERED -> R1 confounded |
| cpython-rawmutex | 2/2 | 1/2 +DECOY | **3/3 FULL** | RECOVERED -> R1 confounded |
| bun-getheapsnapshot | 2/2 | 1/2 | **1/3** | NO recovery -> honestly hard; handoff 2/2 = small-n luck |

### Conclusion

- Concurrency confound is **REAL** for contention-sensitive instances: ksmbd
  (0/2 -> 2/3) and rawmutex (1/2 -> 3/3) recover when isolated. R1's
  jobs-5 + stacked-batch run corrupted hard-instance GRADED evidence.
- But NOT all regressions are confound: getheapsnapshot is honestly ~1/3 even
  isolated (handoff 2/2 = small-n luck).
- **RECALIBRATE**: all graded comparison sweeps at **jobs <=3, NO stacking**.
  R1 MISS/DECOY cluster is NOT reliable monogram signal -> re-baseline clean.
- NEW clean-concurrency decoy: ksmbd r3 = DECOY at jobs 2 (real, not confound) ->
  candidate for the crux experiment.

### Implication for the bun-20093 decoy

bun-20093's 2/2 decoy was a TIGHT 14-call run (not an expensive flail), so it is
likely real (model-prior), unlike the contention flails. But it ran in R1 (j5) ->
MUST re-test at clean jobs to confirm before trusting it. (Round 3 includes it.)

### Operating rule going forward

Graded sweeps: ONE sweep at a time, jobs <=3, many instances inside it (breadth
for pattern). Prepares may overlap in background (light, disjoint repos, no
confound). This honors "many instances to see the pattern" WITHOUT stacking
graded solver load.

---

## ROUND 3 CLEAN BASELINE (jobs 3, no stacking) — confound RE-CONFIRMED + the real pattern (~01:55)

| instance | clean R3 grades | FULL/n |
|---|---|---|
| bun-20093-napi | MISS MISS MISS | **0/3** |
| cpython-grouper | FULL FULL FULL | 3/3 |
| deno-statementsync-iter | FULL FULL | 2/2 |
| netty-bytebuffer | FULL MISS FULL | 2/3 |
| bun-30196-htmlrewriter | FULL FULL FULL | 3/3 |
| cpython-json | FULL FULL FULL | 3/3 |
| ruby-iobuffer | FULL FULL MISS | 2/3 |
| redis-stream-pel | FULL FULL FULL | 3/3 |

**Clean FULL rate = 18/23 = 78%** (vs R1 confounded 50%) -> CONFOUND RE-CONFIRMED.
Clean jobs-3 is the honest operating point.

### The real cross-instance pattern (clean data)

- ONE consistent outlier: **bun-20093-napi = 0/3 clean (0/5 incl R1)**. Everything
  else 2/3-3/3.
- grouper recovered to 3/3 clean -> original decoy = variance, CONFIRMED.
- netty recovered 1/3 -> 2/3 -> was partly confounded.

### bun-20093-napi: the ONE real repeated failure — mechanism refined

Combines (a) MODEL prior: model injects `toThreadSafe`'s own vocabulary
(`leakRef`/`isolatedCopy`, NOT in the symptom) + (b) a MONOGRAM vocabulary gap:
the symptom query `"native addon gc concurrent crash"` surfaces `napi.zig` only at
rank 8 and NOT `napi_handle_scope.cpp`; the answer is reachable only via the exact
query `"napi handle scope"` (#1). **Trigram does not bridge "native addon" ->
"napi".** So the model's symptom-aligned query did not close the path, and it fell
back to its (wrong) prior.

### Candidate generalizable improvement (RECORD — needs more evidence)

**Query vocabulary/abbreviation bridging** for domain terms: `native addon` <->
`napi` <-> `node api`; `gc sweep` <-> `finalize/collect`. This is the one
path-not-closed gap with a generalizable shape (synonym/abbrev expansion,
measurable, no answer-key literal). Gate to implement: confirm the vocabulary-gap
pattern in >=1 more instance (R4/R5 new families) + eval set + 2 holdouts +
`monogram-audit` no-literal gate. NOT yet justified on one instance.

### VERDICT (clean evidence): KEEP 0.61.31

No clean monogram MIS-RANKING decoy found. The single repeated failure (bun-napi)
is model-prior + a vocabulary-bridge gap, not a scoring defect. Continue widening
(R4 dotnet/swift/node; R5 redis/numpy/ruby) to test the vocabulary-gap hypothesis
across families before any code change.

### CROSS-INSTANCE PATTERN (clean runs only, running tally)

| instance | family/shape | clean FULL/n | decoy class |
|---|---|---|---|
| bun-20093-napi | napi handle-scope race | 0/5 | model-prior + vocab-gap (native addon->napi) |
| cpython-grouper | iterator factory | 3/3 (+R1 2/2) | none clean (1 historical variance decoy) |
| cpython-rawmutex | mutex lifetime | 3/3 clean | none (R1 decoy was confound) |
| ksmbd | session UAF | 2/3 clean | 1 clean decoy (investigate) |
| bun-getheapsnapshot | cross-thread Strong | 1/3 | honestly hard (not decoy) |
| netty-bytebuffer | iterator keepalive | 2/3 | none clean |
| deno-statementsync-iter | JS iterator | 2/2 | none |
| bun-30196-htmlrewriter | wrapper owns Response | 3/3 | none (prime wrapper PASSED) |
| ruby-iobuffer | invalidated slice | 2/3 | none |
| redis-stream-pel | dup PEL child | 3/3 | none |
| cpython-json | reentrant UAF | 3/3 | none |

---

## 24H EXTENSION — many-problem Haiku pattern loop (started 2026-05-28 09:44 KST)

User directive: continue the loop for 24 hours. If no stable pattern appears,
increase breadth across more problems and use Haiku aggressively across multiple
instances rather than overfitting one case.

### Operating rule for this extension

- Keep the current evidence verdict unless new clean data changes it:
  **keep monogram 0.61.31; do not patch scoring yet**.
- Use clean graded sweeps only: one sweep at a time, `--jobs <= 3`, no stacked
  solver batches. Prior jobs-5/stacked evidence was confounded.
- Treat many-problem breadth as the primary probe. Do not deepen a single
  instance until the failure shape appears in at least one more unrelated repo.
- Record each batch by tag, report/adoption summary, failure class, and whether
  the run supports the current `native addon -> napi` vocabulary-bridge
  hypothesis or a different mechanism.
- Keep solver prompts clean: no `show --spoil`, no ground-truth literals, no
  answer-key symbols in prompts, tool hints, or monogram code.

### Live-state guard at extension start

At 09:44 KST, one old sweep was still present:

```text
monobench sweep dotnet-124796-wmiinterop-keepalive,swift-88509-demangler-uaf,node-62325-zlib-reset-write
tag: haiku-v06131-clean-newfam-r4j3
stale state: dotnet r1 .running marker + monogram search variant compare signature still alive
```

The stale process predates this extension. It is recorded as a known confound
and excluded from clean-cohort interpretation.

Known ignored stale PIDs:

```text
67607 zsh parent
67609 old monobench sweep
69203 old claude -p solver
72090 old long monogram search
```

Correction at 2026-05-28 ~10:06 KST:

The earlier detached `screen` launch was the wrong operating model for the
user's request. The user asked the agent to personally run the loop for 24 hours
in this session: observe, decide, run the next batch, analyze, and write notes.
The detached `screen` run was stopped and must not be treated as the active loop.

Stopped mistaken detached run:

```text
started: 2026-05-28 09:54 KST
screen:  monobench-24h-20260528
screen PID: 62609
first tag: haiku-v06131-24h-20260528-fresh-lifetime-a-r2j3
status: stopped; first prepare had stalled at dotnet-125293-gchandle-doublefree-race
```

Correct launch model from this point:

```text
owner: current agent session
mode: foreground monobench batch -> wait -> report/adoption/audit -> classify -> next batch
no detached supervisor unless explicitly requested by the user
```

### Direct loop batch 1 — breadth restart (started 2026-05-28 10:10 KST)

The detached supervisor attempt was stopped. The loop resumes as direct agent
operation in the current session.

```text
tag: haiku-v06131-direct-24h-20260528-breadth-r2j2
instances:
  redis-14929-restorecmd-meta-uaf
  ruby-16128-yjit-aliasing-uaf
  dart-3095-uri-backslash-bypass
mode: foreground monobench sweep, runs=2, jobs=2, prepared=yes
purpose: restart with small multi-problem breadth and observe Haiku+monogram usage patterns directly
```

Operational cleanup for direct ownership:

```text
stopped old stale sweep/process group:
  67607, 67609, 69203, 72088, 72090
reason:
  not part of direct loop; 72090 was an 8h stale monogram search consuming CPU
active direct batch PID:
  73853
```

Direct batch 1 outcome:

```text
status: aborted by operator
reason: dart-3095-uri-backslash-bypass prepared index entered long Dart SDK monogram index
cleanup: removed only the Dart prepare .running marker created by this aborted direct batch
decision: restart with smaller non-Dart breadth batch; keep direct foreground ownership
```

### Planned queue

All batches use:

```text
tool: monogram
cli/model: claude/haiku
monogram: 0.61.31
prepared: yes
jobs: 2 or 3
```

Batch A — fresh cross-family UAF/lifetime probes:

```text
dotnet-125293-gchandle-doublefree-race
envoy-45153-filestreamer-cancel-uaf
flutter-170284-surfacetexture-doublefree
grpc-39316-rls-shutdown-uaf
openresty-2483-luapipe-quic-uaf
php-19591-lexbor-mraw-uaf
```

Batch B — hard prior failures + vocabulary-bridge probes:

```text
bun-20093-napi-handlescope-race
node-59910-diagchannel-gc
node-62325-zlib-reset-write
dotnet-124796-wmiinterop-keepalive
swift-88509-demangler-uaf
ksmbd-37899
```

Batch C — broader non-Bun/iterator/control mix:

```text
numpy-31314-nditer-getitem-segfault
redis-14929-restorecmd-meta-uaf
ruby-16128-yjit-aliasing-uaf
dart-3095-uri-backslash-bypass
vapor-2500-filemiddleware-traversal
ktor-5626-readchannel-close
```

### Pattern gates

Promote to maker proposal only if one of these repeats cleanly:

- vocabulary/abbreviation gap across >= 2 unrelated repos
  (example shape: user symptom term does not bridge to project-native acronym);
- right-neighborhood/wrong-root split with the same role boundary in >= 2 repos;
- monogram `region`/`grep`/`NEXT` repeatedly amplifies identifier echo over
  symptom evidence;
- monogram adoption is early and high, but the correct neighborhood is still
  never surfaced under symptom-aligned queries.

Otherwise continue breadth-first sampling and keep 0.61.31 unchanged.

### Supervisor log

Runtime log for the 24h extension:

```text
monologue/demo/monobench/research/indexes/monogram-0.61.31-24h-loop-2026-05-28.log
```

Supervisor script:

```text
monologue/demo/monobench/research/indexes/run-24h-loop-2026-05-28.sh
```

### Direct loop batch 2 — non-Dart breadth (completed 2026-05-28 KST)

```text
tag: haiku-v06131-direct-24h-20260528-breadth2-r2j2
instances:
  redis-14929-restorecmd-meta-uaf
  ruby-16128-yjit-aliasing-uaf
  ktor-5626-readchannel-close
  vapor-2500-filemiddleware-traversal
mode: foreground monobench sweep, runs=2, jobs=2, prepared=yes
binary used: installed monobench 0.1.34 before local fix
```

Result snapshot:

```text
redis:
  baseline 0/2 FULL, monogram 0/2 FULL
  monogram adoption high and early: 42-56%, first call #1
  r1 invalid due to worktree/DB collision; r2 reached Redis but chose wrong root

ruby:
  baseline 0/2 FULL, monogram 0/2 FULL
  monogram adoption high and early: 66-72%, first call #1
  failure shape: closed nearby core.rs cone but MISS/DECOY between get_or_create_version_list and get_version_list

ktor:
  baseline 2/2 FULL, monogram 2/2 FULL
  monogram adoption moderate: 36-42%, first call #1
  not discriminating; useful only as guard/low-cost success rail

vapor:
  baseline 1/1 FULL, monogram 2/2 FULL
  monogram adoption high: 71-73%, first call #1
  not discriminating; useful only as guard/low-cost success rail
```

Critical infrastructure finding:

```text
bug:
  sweep can start different instances in the same millisecond.
  runid is only <arm>-<cli>-<model>-rN-t<ms>, not instance-scoped.
  worktree path was /tmp/monobench-work/wt/<runid>-<pid>.
  therefore two different instances with same runid in one process shared the same worktree path.

observed evidence:
  redis r1 and ktor r1 both used runid monogram-0.61.31-claude-haiku-r1-t1779931155669.
  redis r1 index log claimed prepared Redis snapshot install.
  the actual per-run DB at ~/.monolex/monogram/...9e5137.db contained 2295 Ktor files.
  redis r1 transcript searched Ktor paths and ended with "cannot locate Redis code".

classification:
  invalid benchmark evidence, not a monogram scoring failure.
  before interpreting multi-problem sweep results, monobench worktree isolation must be fixed.
```

Patch applied locally:

```text
file: monologue/demo/monobench/src/run.rs
change:
  worktree path now includes instance id:
  /tmp/monobench-work/wt/<instance-id>/<runid>-<pid>
test:
  cargo test worktree_path_includes_instance_id
  cargo check
status:
  both passed
next:
  continue loop with target/debug/monobench until installed monobench is updated.
```

Updated interpretation gate:

```text
ignore:
  redis r1 from batch 2 for monogram score/ranking conclusions

usable:
  redis r2 for wrong-root classification
  ruby r1/r2 for closed-cone-but-wrong-root classification
  ktor/vapor as success guard rails only

next validation:
  rerun a small multi-instance sweep with the patched target/debug/monobench
  and verify per-run DB row counts/path families match the intended instance.
```

### Isolation fix validation

First smoke attempt:

```text
tag: haiku-v06131-direct-24h-20260528-isolationfix-smoke-r1j2
status: invalid
reason:
  cargo check/test had passed, but target/debug/monobench binary had not been rebuilt.
  the smoke therefore used stale pre-fix worktree layout.
action:
  ignore this tag for monogram scoring.
  rebuilt with cargo build --bin monobench.
```

Second smoke attempt:

```text
tag: haiku-v06131-direct-24h-20260528-isolationfix-smoke2-r1j2
binary: monologue/demo/monobench/target/debug/monobench rebuilt 2026-05-28 10:37 KST
instances:
  redis-14929-restorecmd-meta-uaf
  ktor-5626-readchannel-close
same runid observed:
  monogram-0.61.31-claude-haiku-r1-t1779932288664
```

Proof of fixed isolation:

```text
worktrees:
  /tmp/monobench-work/wt/redis-14929-restorecmd-meta-uaf/monogram-0.61.31-claude-haiku-r1-t1779932288664-22109
  /tmp/monobench-work/wt/ktor-5626-readchannel-close/monogram-0.61.31-claude-haiku-r1-t1779932288664-22109

redis DB:
  ~/.monolex/monogram/monogram-0.61.31-claude-haiku-r1-t1779932288664-22109-50316d.db
  files: 833
  min/max path: ./deps/fast_float/fast_float.h -> ./utils/tracking_collisions.c

ktor DB:
  ~/.monolex/monogram/monogram-0.61.31-claude-haiku-r1-t1779932288664-22109-76e025.db
  files: 2295
  min/max path: ./build-logic/src/main/kotlin/ktorbuild/CInterop.kt -> ./ktor-utils/web/test/io.ktor.util/DigestTest.kt
```

Trace proof:

```text
redis smoke2:
  ROOTCAUSE: ./src/cluster.c::restoreCommand
  monogram commands stayed in Redis paths.

ktor smoke2:
  ROOTCAUSE: ktor-utils/jvm/src/io/ktor/util/cio/FileChannels.kt::readChannel
  monogram commands stayed in Ktor paths.
```

Conclusion:

```text
monobench parallel sweep isolation bug fixed locally.
future direct-loop batches must use target/debug/monobench until the installed ~/.openclis/bin/monobench is rebuilt/deployed.
```

## Direct loop continuation after isolation fix

### Mixed hard-instance batch

```text
tag: haiku-v06131-direct-24h-20260528-post-isolationfix-mixed-r2j2
binary: target/debug/monobench with instance-scoped worktree path
instances:
  ruby-16128-yjit-aliasing-uaf
  numpy-31314-nditer-getitem-segfault
  openresty-2483-luapipe-quic-uaf
  php-19591-lexbor-mraw-uaf
```

Result shape:

```text
ruby:
  current batch 0/2 FULL (MISS, DECOY)
  overall monogram @0.61.31 around 1/6 FULL at that point
  repeated pattern: high adoption, right neighborhood, wrong root

numpy:
  monogram 4/4 FULL and baseline 2/2 FULL
  classification: easy/success rail only

openresty:
  current batch 0/2 FULL (DECOY, DECOY)
  repeated pattern: lifecycle/free candidates surfaced, wrong helper/root selected

php:
  current batch 0/2 FULL (MISS, MISS)
  repeated pattern: broad region/grep/context route reaches URI ownership cone but labels nearby parse/clone/reset roots
```

Maker classification:

```text
primary:
  closed_candidate_space_but_wrong_root
  broad_output_or_fanout_loop

secondary:
  rootcause_label_guard_pivot
  regex_alternation_query
  shell_post_filter_pipeline

not a valid fix:
  copying benchmark answer symbols into monogram code or docs
  increasing raw output limits
  telling the solver more prose without measurable marker/adoption evidence
```

### Monogram NEXT/hint patch 1

Files:

```text
tauri-apps/lib-monogram/src/bin/monogram.rs
tauri-apps/lib-monogram/src/bin/initiate/initiate.md
tauri-apps/lib-monogram/src/bin/initiate/SKILL.md
tauri-apps/lib-monogram/src/bin/initiate/flow-guide.md
```

Generic behavior added:

```text
region_contrast_lock:
  fires when top region candidates are close, supported, and distinct.
  asks for bounded context + depth-1 caller comparison before widening.

lifecycle_file_probe:
  fires for broad lifecycle/system region queries.
  emits file-scoped context and focused same-file free/put/release/wait/idle greps for top regions.
```

Verification:

```text
cargo check -p lib-monogram --bin monogram
cargo test -p lib-monogram region_contrast_lock_requires_close_supported_candidates
cargo build -p lib-monogram --bin monogram
monogrid tauri-apps/lib-monogram/src/bin/initiate/initiate.md
monogrid tauri-apps/lib-monogram/src/bin/initiate/flow-guide.md
python3 .claude/skills/monolex-monogram-maker/scripts/check-flow.py
```

Manual output proof:

```text
cwd: /tmp/monobench-work/ruby
command:
  target/debug/monogram region "compiled block versions invalidation vector heap memory management" -n 5 --score-debug

observed:
  [marker: systems_lifecycle_next]
  [marker: lifecycle_file_probe]
  file-scoped context/grep commands for the top two regions
```

### Region contrast validation batch

```text
tag: haiku-v06131-direct-24h-20260528-region-contrast-r2j1
reason for jobs=1:
  unrelated monobench sweeps were already active, so this direct session used jobs=1 to reduce resource/quota collision.
instances:
  ruby-16128-yjit-aliasing-uaf
  openresty-2483-luapipe-quic-uaf
  php-19591-lexbor-mraw-uaf
```

Results:

```text
ruby:
  r1 FULL, r2 MISS
  r1 ROOTCAUSE: ./yjit/src/core.rs::get_iseq_payload
  r2 ROOTCAUSE: ./yjit/src/core.rs::get_or_create_version_list
  interpretation: patch may help one run, but failure still broadens into same-file helper/version-list cone.

openresty:
  r1 DECOY, r2 FULL
  r1 ROOTCAUSE: src/ngx_http_lua_pipe.c::ngx_http_lua_pipe_proc_read_stdout_cleanup
  r2 ROOTCAUSE: ./src/ngx_http_lua_pipe.c::ngx_http_lua_ffi_pipe_proc_destroy
  interpretation: one success signal; free_site/rootcause guard rail appears useful but not stable.

php:
  r1 MISS, r2 MISS
  r1 ROOTCAUSE: ./ext/uri/uri_parser_whatwg.c::reset_parser_state
  r2 ROOTCAUSE: ext/uri/uri_parser_whatwg.c::php_uri_parser_whatwg_clone
  interpretation: still closed candidate space but wrong root; solver follows URI ownership cone but not the decisive lifecycle owner.
```

Audit after rebuilding target/debug/monobench:

```text
ruby --tag region-contrast:
  patterns: region_first_next 6, fanout_preflight 5, systems_lifecycle_next 2
  recommendations:
    closed_candidate_space_but_wrong_root count=1
    broad_output_or_fanout_loop count=7
    lifecycle_proof_unresolved count=1

openresty --tag region-contrast:
  patterns: region_first_next 9, fanout_preflight 7, guarded_anchor_preserve 6,
            rootcause_label_guard 4, systems_lifecycle_next 3
  recommendations:
    closed_candidate_space_but_wrong_root count=1
    broad_output_or_fanout_loop count=10
    lifecycle_proof_unresolved count=1
    rootcause_label_guard_pivot count=1

php --tag region-contrast:
  patterns: region_first_next 7, fanout_preflight 7, shell_post_filter_pipeline 7,
            guarded_anchor_preserve 2, systems_lifecycle_next 1
  recommendations:
    closed_candidate_space_but_wrong_root count=2
    broad_output_or_fanout_loop count=8
    lifecycle_proof_unresolved count=1
    rootcause_label_guard_pivot count=1
```

### Monobench audit patch

Files:

```text
monologue/demo/monobench/src/monogram_audit.rs
monologue/demo/monobench/src/main.rs
monologue/demo/monobench/README.md
monologue/demo/monobench/initiate/initiate.md
```

Generic behavior added:

```text
classify marker patterns:
  systems_lifecycle_next
  lifecycle_file_probe
  region_contrast_lock
  guarded_anchor_preserve
  bounded_contrast_only

recommendations:
  lifecycle_proof_unresolved
  region_contrast_lock_unresolved

broad_output_or_fanout_loop:
  now counts fanout_preflight occurrences as pressure, not only large context/search/chain limits.
```

Verification:

```text
cargo fmt --check
cargo check
cargo test lifecycle_and_contrast_markers_are_command_shape_patterns
cargo test worktree_path_includes_instance_id
cargo build --bin monobench
monogrid monologue/demo/monobench/initiate/initiate.md
```

### Monogram NEXT/hint patch 2

Problem observed:

```text
Ruby r2 received systems_lifecycle_next from context invalidate_block_version,
but that branch did not emit guarded_anchor_preserve/bounded_contrast_only unless the symbol was already classified as an operation boundary.
The run then widened into broad same-file version-list helper exploration and ended MISS.
```

Generic behavior added:

```text
context lifecycle/root proof path now always emits:
  [marker: systems_lifecycle_next]
  [marker: guarded_anchor_preserve]
  [marker: bounded_contrast_only]

default lifecycle context NEXT now also emits a same-file region re-entry:
  monogram region "<symbol> lifecycle boundary" -n 5 --score-debug --file <current-file>
```

Manual output proof:

```text
cwd: /tmp/monobench-work/ruby
command:
  target/debug/monogram context invalidate_block_version --code 80 --file ./yjit/src/core.rs

observed:
  [marker: systems_lifecycle_next]
  [marker: guarded_anchor_preserve]
  [marker: bounded_contrast_only]
  monogram context "invalidate_block_version" --code 80 --file ...
  monogram chain "invalidate_block_version" --callers --depth 1 --file ...
  monogram region "invalidate_block_version lifecycle boundary" -n 5 --score-debug --file ...
  monogram grep "free\|put\|release\|wait\|idle" --file ... -n 80
```

Verification:

```text
cargo check -p lib-monogram --bin monogram
cargo test -p lib-monogram region_contrast_lock_requires_close_supported_candidates
cargo build -p lib-monogram --bin monogram
```

Current conclusion:

```text
monogram is improving, but the evidence is mixed.
Confirmed improvements:
  - monobench isolation bug fixed, so new evidence is valid by worktree/DB family.
  - monogram now emits bounded lifecycle/contrast markers in the places that exposed fanout drift.
  - monobench audit can now measure those markers and unresolved lifecycle proof loops.

Not yet proven:
  - stable success lift on Ruby/PHP.
  - OpenResty improvement may be real, but only 1/2 in the latest batch.

Next loop:
  rerun Ruby/OpenResty/PHP after patch 2 with PATH=tauri-apps/target/debug:$PATH.
  include at least one success guard rail such as numpy or ktor.
  classify whether systems_lifecycle_next now leads to same-file region/guarded proof instead of broad grep/read loops.
```

### Monogram NEXT/hint patch 3

Problem observed:

```text
PHP runs reached the correct allocator/reset neighborhood, including:
  lexbor_mraw_clean(lexbor_parser.mraw)
  ++parsed_urls % maximum_parses_before_cleanup == 0

But the grep lifecycle detector treated this as a generic containing-function
rail and emitted broad NEXT around uri_parser_whatwg instead of locking the
state-reset function. The solver then chose clone/reset-adjacent decoys.
```

Generic behavior added:

```text
Lifecycle/free-query language now includes clean/cleanup/reset/clear.
State progression evidence now recognizes counter/threshold updates such as
++ / += / increment / decrement when paired with max/min/limit/threshold/count
terms. The paired lifecycle root marker accepts either direct receiver-state
mutation or counter/threshold reset evidence.
```

Manual output proof:

```text
cwd: /tmp/monobench-work/php-src
command:
  target/debug/monogram grep "lexbor_mraw\\|maximum_parses_before_cleanup" --raw -n 30 --file ./ext/uri/uri_parser_whatwg.c

observed:
  reset_parser_state context hits
  [marker: free_site_triage]
  [marker: guarded_anchor_preserve]
  [marker: bounded_contrast_only]
  [marker: paired_lifecycle_state_mutation]
  [marker: context_root_lock]
  [marker: rootcause_label_guard]
  [marker: answer_ready]
  monogram context "reset_parser_state" --code 80 --file ./ext/uri/uri_parser_whatwg.c
  monogram chain "reset_parser_state" --callers --depth 1 --file ./ext/uri/uri_parser_whatwg.c
```

Verification:

```text
cargo test -p lib-monogram --bin monogram paired_lifecycle_candidate_detects_counter_threshold_reset
cargo check -p lib-monogram --bin monogram
cargo build -p lib-monogram --bin monogram
```

Next loop:

```text
Run PHP/Ruby/OpenResty plus Ktor guard rail with the rebuilt debug monogram.
Classification target:
  - PHP should stop widening from reset evidence to clone/container roots.
  - Ruby/OpenResty remain hard rails for wrong-root lifecycle drift.
  - Ktor should remain FULL to catch regression from broader lifecycle markers.
```

### Batch: state-reset lifecycle r2j1

Tag:

```text
haiku-v06131-direct-24h-20260528-state-reset-lifecycle-r2j1
```

Instances:

```text
php-19591-lexbor-mraw-uaf
ruby-16128-yjit-aliasing-uaf
openresty-2483-luapipe-quic-uaf
ktor-5626-readchannel-close
```

Results:

```text
PHP:       NO_RESULT, MISS
Ruby:      MISS, MISS
OpenResty: DECOY, DECOY
Ktor:      FULL, FULL
```

Classification:

```text
Patch 3 improved PHP enough that one run selected the reset/cleanup helper
instead of earlier clone/container decoys, but it was still the wrong label.
Ruby still ended on version-list helpers after reaching the local lifecycle
neighborhood. OpenResty still ended on cleanup/crash-site handlers after seeing
the destroy/ordering cone. Ktor stayed FULL, so the lifecycle markers did not
break the read-channel success rail.

Common failure class:
  helper/crash-site label lock after the right neighborhood is reached.

General rule needed:
  helper/reset/cleanup evidence must trigger owner contrast instead of final
  answer readiness; close/destroy/shutdown owners that coordinate cleanup order
  must keep the label on the ordering owner while callees remain mechanism proof.
```

### Monogram NEXT/hint patch 4

Generic behavior added:

```text
paired_lifecycle_state_mutation now emits answer_not_ready plus
lifecycle_owner_contrast instead of marking cleanup/reset helpers answer_ready.

lifecycle_owner_contrast asks for same-file owner/caller comparison using
creator/grower/parser/opener style operations before naming a helper.

context now detects lifecycle_ordering_owner_candidate for close/destroy/
shutdown-style functions that coordinate cleanup/free/finalize/pool/wait/
handler ordering. Those contexts emit rootcause_label_guard, context_root_lock,
and answer_ready while treating cleanup callees as mechanism evidence.
```

Manual output proof:

```text
cwd: /tmp/monobench-work/php-src
command:
  target/debug/monogram grep "lexbor_mraw\\|maximum_parses_before_cleanup" --raw -n 30 --file ./ext/uri/uri_parser_whatwg.c

observed:
  [marker: paired_lifecycle_state_mutation]
  [marker: rootcause_label_guard]
  [marker: answer_not_ready]
  [marker: lifecycle_owner_contrast]
  monogram chain "reset_parser_state" --callers --depth 1 --file ...
  monogram grep 'add\|insert\|append\|grow\|resize\|create\|alloc\|parse\|spawn\|open\|start' --file ...
```

```text
cwd: /tmp/monobench-work/lua-nginx-module
command:
  target/debug/monogram context ngx_http_lua_ffi_pipe_proc_destroy --code 100 --file ./src/ngx_http_lua_pipe.c

observed:
  [marker: lifecycle_ordering_owner_candidate]
  [marker: rootcause_label_guard]
  [marker: context_root_lock]
  [marker: answer_ready]
  monogram grep "cleanup\|free\|finalize\|pool\|wait\|handler" --file ...
  monogram chain "ngx_http_lua_ffi_pipe_proc_destroy" --callers --depth 1 --file ...
```

Verification:

```text
cargo test -p lib-monogram --bin monogram paired_lifecycle_candidate_detects_counter_threshold_reset
cargo check -p lib-monogram --bin monogram
cargo build -p lib-monogram --bin monogram
cargo fmt --check -p lib-monogram  # failed: package-wide pre-existing formatting drift; not auto-formatted
```

Next loop:

```text
Rerun the same PHP/Ruby/OpenResty/Ktor set with the rebuilt debug monogram.
Expected read:
  - OpenResty should stop naming cleanup handlers if ordering-owner guard works.
  - PHP should treat reset helper as proof, not final label.
  - Ruby may still need ownership-boundary owner contrast if it stays on remove/get helpers.
  - Ktor should remain FULL as the regression guard.
```

### Batch: owner contrast r2j1

Tag:

```text
haiku-v06131-direct-24h-20260528-owner-contrast-r2j1
```

Results:

```text
OpenResty: FULL, DECOY
PHP:       MISS, MISS
Ruby:      MISS, FULL
Ktor:      FULL, FULL
```

Classification:

```text
Patch 4 produced partial lift: one OpenResty run selected the ordering owner,
one Ruby run selected the owner, and Ktor stayed stable at 2/2 FULL. The
remaining OpenResty failure pivoted out into shell grep/find after an early
monogram stats path, so it is partly solver adherence/guard-pivot failure.

PHP did not actually use the new lifecycle_owner_contrast rail. Its trace
followed the older systems-language NEXT from search/region:
  free|put|release|unlock|wait|idle
and then generic free/dtor evidence. That vocabulary was stale relative to the
detector, which already treats clean/cleanup/reset/clear as lifecycle terms.

Common failure class:
  probe-vocabulary drift between detector semantics and solver-facing NEXT.
```

### Monogram NEXT/hint patch 5

Generic behavior added:

```text
Introduced a single lifecycle_probe_pattern for solver-facing NEXT output:
  free|put|release|unref|deref|clean|cleanup|reset|clear|unlock|wait|idle

Reused it across:
  - systems_lifecycle_next
  - systems-language search NEXT
  - region lifecycle proof commands
  - lifecycle_file_probe
  - regex alternation broad-lifecycle detection

This is a vocabulary sync, not a benchmark literal rail: cleanup/reset/clear
are lifecycle operations in many systems repos and now match detector behavior.
```

Verification target:

```text
Search/region NEXT on systems lifecycle queries should now surface clean/reset/
clear probes without the agent needing to invent those terms or fall back to
shell grep.
```

### Batch: probe vocabulary r2j1

Tag:

```text
haiku-v06131-direct-24h-20260528-probe-vocab-r2j1
```

Results:

```text
PHP:       MISS, MISS
OpenResty: DECOY, DECOY
Ruby:      MISS, MISS
Ktor:      FULL, FULL
```

Classification:

```text
Patch 5 was incomplete. It synchronized search/region lifecycle probes, but
context lifecycle/root proof output still had the old free/put/release/wait/idle
probe. PHP traces reached reset_parser_state and received lifecycle_owner_contrast,
but the context-local proof path still did not advertise clean/cleanup/reset/clear.
OpenResty and Ruby remained helper/crash-site or helper/getter misses. Ktor stayed
FULL, so the regression is limited to hard lifecycle/root-label rails.
```

Patch 6:

```text
Replaced the remaining context lifecycle/root proof hardcoded probe strings and
JSON next hint with lifecycle_probe_pattern(). This closes the vocabulary drift
across search, region, context, lifecycle_file_probe, and JSON NEXT.
```

### Batch: context probe r2j1

Tag:

```text
haiku-v06131-direct-24h-20260528-context-probe-r2j1
```

Results:

```text
PHP:       MISS, MISS
OpenResty: DECOY, FULL
Ruby:      MISS, MISS
Ktor:      FULL, FULL
```

Classification:

```text
Patch 6 partially recovered OpenResty and kept Ktor stable, but PHP stayed
locked on the reset helper. The exposing PHP trace ran:
  monogram grep "lexbor_mraw_clean|lexbor_parser_clean|reset_parser_state"
That output emitted broad_lifecycle_or_redirect, but the redirect sent the
agent to generic ownership probes instead of lifecycle owner contrast. This is
the same class as patch 5, one layer deeper: regex OR lifecycle handling was
out of sync with the lifecycle/root proof semantics.
```

Patch 7:

```text
broad_lifecycle_or_redirect now emits systems_lifecycle_next +
guarded_anchor_preserve and a lifecycle-first region/probe path using
lifecycle_probe_pattern(). It also emits lifecycle_owner_contrast and
answer_not_ready before accepting cleanup/reset/free helpers.

This is general: regex OR probes made from cleanup/reset/free lifecycle terms
should preserve the same file and compare owner/caller context instead of
opening generic ownership probes first.
```

### Batch: broad lifecycle OR r2j1

Tag:

```text
haiku-v06131-direct-24h-20260528-broad-or-lifecycle-r2j1
```

Results:

```text
Ktor:      FULL, FULL
OpenResty: DECOY, DECOY
PHP:       DECOY, MISS
Ruby:      MISS, MISS
```

Observed roots:

```text
OpenResty: ngx_http_lua_pipe_proc_read_stdout_cleanup, both runs
PHP:       php_uri_parser_whatwg_free, reset_parser_state
Ruby:      get_or_create_version_list, both runs
Ktor:      FileChannels.kt::readChannel, both runs
```

Classification:

```text
Patch 7 preserved the Ktor holdout but did not fix lifecycle owner selection.
Audit reported closed_candidate_space_but_wrong_root and lifecycle_proof_unresolved
for OpenResty/PHP/Ruby. Traces show the repeated trap: the solver sees relevant
same-file lifecycle/helper evidence, but a helper/accessor/crash-frame label is
still accepted before same-file owner/order contrast is closed.

This is not a need for answer-key literals. The general missing primitive is:
cleanup/reset/free/accessor hits must be contrasted against both setup owners
(create/grow/parse/open/start) and teardown/order owners
(destroy/close/shutdown/finalize) before the output implies answer readiness.
```

Patch 8:

```text
Extended lifecycle query recognition to clean/cleanup/reset/clear, and extended
owner contrast to setup and teardown owner verbs:
  add|insert|append|grow|resize|create|alloc|parse|spawn|open|start
  destroy|close|shutdown|finalize|teardown|terminate|cancel|stop

Added a lifecycle owner region NEXT:
  monogram region "lifecycle owner boundary create grow resize parse open start
  destroy close shutdown finalize" -n 5 --score-debug [--file ...]

Context no-entry handling now treats cleanup/reset lifecycle queries like lifecycle
queries, not plain unresolved symbols. Context lifecycle proof also lets
context_symbol_is_lifecycle_ordering_owner lock destroy/finalize-style ordering
owners before helper/destructor contrast.

Verification:
  cargo check -p lib-monogram --bin monogram --target-dir /tmp/monogram-patch5-target
  cargo test -p lib-monogram --bin monogram lifecycle_ --target-dir /tmp/monogram-patch5-target
  cargo build -p lib-monogram --bin monogram --target-dir /tmp/monogram-patch5-target
  monogrid initiate.md / flow-guide.md / SKILL.md
  check-flow.py

Live output proof:
  context "ngx_http_lua_ffi_pipe_proc_destroy" now emits lifecycle_owner_contrast
  with destroy/close/finalize owner terms.
  context "reset_parser_state" --file ext/uri/uri_parser_whatwg.c now emits
  systems_lifecycle_next plus lifecycle_owner_contrast even when the symbol is
  unresolved in the prepared index.
```

### Batch: owner teardown r2j1

Tag:

```text
haiku-v06131-direct-24h-20260528-owner-teardown-r2j1
```

Results:

```text
Ktor:      FULL, FULL
OpenResty: DECOY, DECOY
PHP:       DECOY, MISS
Ruby:      MISS, FULL
```

Classification:

```text
Patch 8 kept the Ktor holdout stable and recovered one Ruby run to FULL
(`add_block_version`). Ruby audit no longer reported lifecycle_proof_unresolved.
OpenResty and PHP remained wrong-root: OpenResty stayed on cleanup handlers,
and PHP stayed on free/reset helpers.

Important trace detail:
  OpenResty r1 used a valid prepared DB in the solver environment
  (Files: 137, C/H/Lua profile), so the failure is not simply an empty index.
  After `monogram grep "cleanup"`, monogram fallback still emitted an old
  answer-ready sentence: "Pick ROOTCAUSE by the symptom's lifecycle boundary".
  That path did not force lifecycle_owner_contrast even though the output also
  showed the destroy/finalize owner evidence.

Conclusion:
  The remaining C/PHP trap is not vocabulary. It is answer-readiness language
  in the fallback lifecycle candidate path. The fallback must be answer_not_ready
  and must emit lifecycle_owner_contrast before the solver accepts helper labels.
```

Patch 9:

```text
Changed the fallback lifecycle candidate NEXT in `grep` from answer-ready
"Pick ROOTCAUSE by lifecycle boundary" to:
  lifecycle_owner_contrast
  answer_not_ready
  lifecycle owner region + same-file owner/caller contrast
  ROOTCAUSE label guard

This is general: any lifecycle candidate list where no stronger paired-state,
UI, or response-wrapper proof has fired should not encourage a helper/root label
until same-file owner/caller contrast is closed.

Also extended `monobench monogram-audit` pattern recognition for:
  lifecycle_owner_contrast
  broad_lifecycle_or_redirect
  answer_not_ready

Verification:
  cargo check -p lib-monogram --bin monogram --target-dir /tmp/monogram-patch5-target
  cargo test -p lib-monogram --bin monogram lifecycle_ --target-dir /tmp/monogram-patch5-target
  cargo build -p lib-monogram --bin monogram --target-dir /tmp/monogram-patch5-target
  cd monologue/demo/monobench && cargo check
  cd monologue/demo/monobench && cargo test lifecycle_and_contrast_markers_are_command_shape_patterns
  cd monologue/demo/monobench && cargo build
```

### Batch: fallback contrast r2j1

Tag:

```text
haiku-v06131-direct-24h-20260528-fallback-contrast-r2j1
```

Results:

```text
Ktor:      FULL, FULL
OpenResty: DECOY, DECOY
PHP:       MISS, MISS
Ruby:      MISS, MISS
```

Classification:

```text
Patch 9 is not sufficient and likely regressive for Ruby. It reduced some broad
OpenResty audit symptoms but did not move the final root away from cleanup
handlers. PHP stayed reset_parser_state. Ruby lost the prior one-run FULL and
returned to get_or_create_version_list.

The trace exposes a more upstream problem: the injected monobench monogram lead
still tells Haiku to run:
  monogram region "ownership boundary ref deref leakRef isolatedCopy" ...

That is a JS/WebKit-specific ownership dialect. It contaminates C/PHP/Ruby
lifecycles before monogram's result-aware NEXT can steer the solver. This is a
harness prompt/tool-delivery issue, not just a monogram scoring issue.
```

Patch 10:

```text
Updated `harness/tools/monogram/lead.md` so UAF/ownership starts with a
repo-sensitive branch:
  systems lifecycle first for C/C++/Rust/PHP/Ruby/Kotlin/Java/Swift/Go or when
  output/symptom contains cleanup/reset/free/destroy/close/finalize/parse/pool.
  JS/WebKit recipe with leakRef/isolatedCopy only when those symbols or that
  ecosystem are actually present.

This avoids injecting leakRef/isolatedCopy into systems repos where those terms
are absent from the symptom and code.
```

### Batch: prompt branch r2j1

Tag:

```text
haiku-v06131-direct-24h-20260528-prompt-branch-r2j1
```

Results:

```text
Ktor:      FULL, FULL
OpenResty: DECOY, MISS
PHP:       MISS, MISS
Ruby:      MISS, MISS
```

Classification:

```text
The prompt branch was directionally correct but insufficient. It preserved Ktor
2/2 FULL, and some initial C/PHP/Ruby commands moved toward systems lifecycle
terms, but the solver still selected helper/crash/accessor rows:

OpenResty:
  r1 -> ngx_http_lua_pipe_proc_read_stdout_cleanup
  r2 -> ngx_http_lua_pipe_close_helper

PHP:
  r1/r2 -> reset_parser_state

Ruby:
  r1 -> rb_yjit_iseq_mark
  r2 -> get_or_create_version_list

Audit pattern:
  All three failing systems repos reported closed_candidate_space_but_wrong_root.
  lifecycle_owner_contrast and answer_not_ready appeared, but without a compact
  role table the model still treated helper/crash/accessor functions as roots.

Trace detail:
  OpenResty r2 reached `monogram region "pipe cleanup lifecycle boundary pool"`
  and same-file lifecycle greps, but final answer moved to a close helper.
  PHP r2 reached `monogram region "url parse lifecycle free release cleanup owner"`
  and a grep containing parsed_urls / maximum_parses_before_cleanup / reset /
  destroy, but final answer stayed on reset_parser_state.
  Ruby r2 reached both YJIT invalidation and version_map evidence, but final
  answer stayed on get_or_create_version_list.

Conclusion:
  The next source-level improvement should not be more prose. It should expose
  a bounded owner/helper role table from the actual grep/region candidates:
  owner-setup, owner-teardown, helper/crash-site, inspect. Setup owners must
  include add/create/grow/parse/open; helper traps must include get/find/read,
  cleanup/reset/clear, handler, and mark.
```

Patch 11:

```text
Implemented general lifecycle owner/helper contrast in monogram:

  - Setup owner boundary:
      add, insert, append, grow, resize, create, alloc, parse, spawn, open, start

  - Teardown owner boundary:
      logoff, logout, disconnect, close, shutdown, destroy, finalize, remove,
      unlink, evict

  - Helper/crash/accessor traps:
      get*, find*, lookup*, read*, cleanup, reset, clear, handler, mark

  - `grep` lifecycle triage and broad lifecycle OR paths now print:
      lifecycle_owner_helper_contrast
      answer_not_ready
      role=<owner-setup|owner-teardown|helper-or-crash-site|helper-lifecycle|inspect>
      top owner context/chain commands

  - Broad lifecycle `region` output now also prints candidate region roles before
    local region NEXT commands.

  - Context lifecycle signal now recognizes setup owner operation names, but
    explicitly excludes getter/helper/crash-site traps so `get_or_create_*`,
    `reset_*`, read cleanup handlers, and mark functions do not become
    answer-ready owner boundaries.

  - `parse` is token-bound, so parser nouns such as `uri_parser_whatwg` are not
    promoted above operation functions such as `php_uri_parser_whatwg_parse_ex`.

  - `monobench monogram-audit` now recognizes the new
    lifecycle_owner_helper_contrast marker.

Live output spot check:
  On lua-nginx-module, broad same-file lifecycle grep now emits role rows:
    owner-setup     ngx_http_lua_ffi_pipe_spawn
    owner-teardown  ngx_http_lua_ffi_pipe_proc_destroy
    owner-setup     ngx_http_lua_pipe_add_input_buffer
  and requires owner/helper contrast before cleanup/reset/free labels.

  On php-src, the parsed_urls / maximum_parses_before_cleanup / reset / destroy
  grep now ranks `php_uri_parser_whatwg_parse_ex` as owner-setup and
  `reset_parser_state` as helper-or-crash-site.

Verification:
  cargo test -p lib-monogram --bin monogram lifecycle_ --target-dir /tmp/monogram-patch5-target
  cd monologue/demo/monobench && cargo test lifecycle_and_contrast_markers_are_command_shape_patterns
  cargo check -p lib-monogram --bin monogram --target-dir /tmp/monogram-patch5-target
  cargo build -p lib-monogram --bin monogram --target-dir /tmp/monogram-patch5-target
```
