# Monolex Recursive Tool-Development Loop

Date: 2026-05-26

## Position

monobench is public-shaped, but its first role is internal: it is the evidence
engine for improving monogram, NIIA research memory, and monokist-style
structural memory.

The loop is not "run a benchmark, patch the failing answer." The loop is:

```text
problem instance
  -> controlled solver run
  -> preserved trace/evidence
  -> success/failure comparison
  -> code-analysis method classification
  -> monogram primitive or score proposal
  -> generalized implementation
  -> holdout validation
  -> NIIA/monokist memory
```

## 24-Hour Workflow Loop Mode

When the user asks for a 24-hour loop, the loop becomes a direct operating
mode for the current agent session. It is not satisfied by starting an
unattended background job and reporting that the job exists.

```text
agent preflight
  -> run foreground breadth batch
  -> actively monitor process/log state
  -> report + adoption + monogram-audit
  -> classify success/failure rails
  -> append research note
  -> widen to more problems if no stable pattern
  -> deepen only after a repeated pattern appears
  -> implement generalized maker change only after the evidence gate
  -> validate with holdouts
  -> continue until deadline or explicit stop
```

This mode is continuous direct operation. Do not use `screen`, `nohup`, cron,
launchd, or any other detached process as the loop owner unless the user
explicitly asks for unattended execution.

A valid 24-hour loop must have:

- the current agent session as owner;
- foreground `monobench` batches that the agent waits on and inspects;
- research notes that record start time, tool versions, ignored stale PIDs,
  batch queue, per-batch verdicts, and current decision;
- one active clean sweep at a time unless intentional parallelism is documented;
- post-batch summaries from `report`, `adoption`, and `monogram-audit`;
- a decision gate before code changes.

If no pattern appears, widen first:

```text
same monogram version
same cli/model axis
more unrelated repos
more problem families
Haiku runs 2..5
jobs 2..3
```

Do not over-deepen one favorite instance unless the same failure shape appears
again. The target is a repeated tool-behavior pattern, not a solved benchmark
answer.

For the executable hotplate, load the monobench maker guide:

```text
.claude/skills/monolex-monobench-maker/Full-Guide.md
```

## Roles

| System | Role in the loop |
|---|---|
| monobench | Runs controlled experiments, preserves telemetry, exposes adoption, drift, output, and grading evidence. |
| monogram-language | Derives language/runtime lifecycle role profiles from corpus repositories; produces evidence priors only. |
| monogram | Provides code-search primitives: trigram search, region, grep/refs, context, chain/tree, coupling, metrics, audits, and NEXT guidance. |
| monogram maker | Converts repeated evidence patterns into generalized implementation changes: primitive, score, budget, proof marker, or NEXT. |
| monomento | Supplies scoring discipline and memory navigation: IDF-like damping, coverage, length normalization, explain fields, and searchable exported transcripts. |
| NIIA | Keeps durable research notes, handoff context, and cross-session loop continuity. |
| monokist | Keeps structural relationships between methods, evidence, command flows, and implementation boundaries. |

## Three-CLI Branch: 2026-05-28

The loop has now split into three CLI responsibilities:

```text
monogram-language
  -> analyzes language/core/runtime repositories
  -> emits .niia/monogram-language/<lang>.json profile evidence
  -> must not learn benchmark answers

monogram
  -> consumes profile priors later through query-aware score terms
  -> must keep score-debug explainable
  -> must not grow inline language string lists without corpus evidence

monobench
  -> validates behavior as a canary
  -> preserves traces and failure shapes
  -> must not feed root-cause literals back into tools
```

Current owner split:

```text
Another session may own active development of:
  tauri-apps/lib-monogram-language/

This loop session may own:
  monobench/monogram loop docs
  .claude skill handoff docs
  read-only monobench status/report/adoption/trace inspection
```

Return point:

```text
tauri-apps/lib-monogram/docs/MONOGRAM-LANGUAGE-THREE-CLI-ARCHITECTURE-2026-05-28.md
```

If another session is actively editing `lib-monogram-language`, do not make
conflicting edits there. Treat profile generation as an external evidence input
until that session hands back a stable profile format.

## Epistemic Scoring Principle

Monogram's working principle is:

```text
close the path first, then search within it
```

For the recursive loop, that means code-analysis primitives and trigram scoring
are not separate features:

- structural analysis closes the candidate space: chain frontiers, coupling
  endpoints, ownership boundaries, structural refs, containing regions;
- trigram/query evidence searches inside that closed space: query overlap,
  facet coverage, broad-term damping, length normalization, and explainable
  confidence;
- NEXT should move the solver from open search to closed candidate spaces, then
  ask for proof inside the narrowed space.

Two failure modes are especially important:

| Failure mode | Shape | Maker interpretation |
|---|---|---|
| Trigram without structure | name decoys win because the text looks close | path not closed; add graph/coupling/boundary reach |
| Structure without trigram calibration | fan-out returns connected nodes without ranking confidence | closed space not searched; add facet coverage and broad-term damping |

The loop should classify a MISS as one of these before proposing a change:

- **path not closed** — the solver never reached the right neighborhood, or the
  right graph/coupling/boundary edge was not exposed as a staged candidate;
- **closed but uncalibrated** — the solver reached the right neighborhood, but
  generic symbols, proof words, or name-decoys outranked the causal node;
- **proof not promoted** — the right invariant appeared in evidence, but NEXT or
  compact proof markers did not push it into the final answer.

Useful proposal language in this layer includes `graph_propagation`,
`coupling_endpoint`, `query_facet_coverage`, `anchor_coverage`,
`generic_probe_damping`, and `unknown_confidence` markers. These are allowed
only when they are measurable from traces, index data, command output, or
score-debug fields.

## Region Score Boundary And Validation Gate

User-facing score explanations and maker scoring telemetry are different
surfaces.

Keep in user-facing `initiate.md`:

- the `SEARCH SCORE` formula, because it explains why file/search results are
  trustworthy;
- normalized rank percent, confidence, role/proof labels, concise evidence, and
  runnable `[NEXT]`;
- the practical rule that scores choose a bounded proof path and must be
  confirmed by `context`, `chain`, `tree`, `grep`, or `coupling`.

Keep in maker/research surfaces only:

- raw region scores such as `55.997`;
- term weights, evidence weights, bonuses, penalties, and ablation knobs;
- benchmark traces, answer-key-derived labels, run IDs, and solver transcripts;
- monobench tuning grids and holdout ledgers.

The public boundary is:

```text
default CLI output
  -> normalized rank percent + confidence + concise evidence + NEXT

--score-debug / --json
  -> raw score + term/evidence weights + bonuses/penalties

maker docs / monobench research
  -> ablation, holdout, rank-lift, no-literal audit, failure-shape comparison
```

Scoring work must be classified before testing:

| Change type | Minimum gate | Monobench gate |
|---|---|---|
| docs/output boundary only | no-args/help equality, `--mcp-schema` parse, monogrid, keyword leak scan | optional report/adoption/audit read |
| default output shape | default-output smoke: no raw score leak; `--score-debug` still exposes raw evidence | run `monogram-audit` for affected instance/tag |
| ranking formula or score weights | rank-lift smoke against the exposing case plus negative-control smoke | failed case + prior FULL holdout + unrelated hard instance |
| NEXT / output-budget steering | [NEXT] reachability check plus focused command smoke | affected run audit + one holdout |

Acceptance criteria for any scoring change:

- same monogram version or explicitly recorded experiment epoch;
- same prepared-index policy for compared runs;
- no benchmark answer literal in code, docs, NEXT, or query gates;
- rank lift is measured before FULL-rate claims;
- `--score-debug` explains why the root outranks the decoy;
- at least one prior FULL and one unrelated hard instance do not regress;
- `monobench integrity`, `adoption`, `trace`/`evidence`, and
  `monogram-audit` are read before the result is promoted.

## Canonical Flow

### 1. Select a discriminating instance

Use instances where the symptom and cause are structurally linked, not directly
grep-solvable. The baseline gets the same depth directive. If baseline solves it
cheaply and stably, the instance is not a strong tool-development signal.

### 2. Run paired evidence

Compare tool and baseline under the same CLI/model axis. For weak-model canaries,
prefer repeated runs over anecdotes:

```text
baseline vs monogram
same instance
same cli
same model
same effort label
same monogram version or explicit experiment epoch
same prepared-index policy
```

Experiment-axis discipline:

- Start with the real `monogram` arm. If that arm is confusing, reduce the
  analysis question, not the tool identity.
- Add `monogram-mcp` only when the delivery surface is the suspected variable:
  same index and core capability, different interaction path.
- Do not put `monogram-thin` in the main recursive loop. It may be a historical
  prompt-load control, but it adds a third variable before monogram itself is
  understood.

Version-axis discipline:

- Compare failure motifs first within the same model, same CLI, same tool arm,
  same monogram version, and same prepared-index policy. A different monogram
  binary or warmed index can change ranking/NEXT behavior enough that the trace
  is a different experiment.
- If the run metadata does not yet expose `monogram_version`, isolate by a short
  `--since` window plus `--tag`/`--note`, and record the observed binary/source
  state in the research note before treating two runs as comparable.
- For a MISS, compare the command prefix before the final wrong root against
  other runs in the same comparable cohort. Ask whether the solver reached the
  same region, followed the same NEXT, widened at the same command, or named the
  same decoy. A repeated pre-failure prefix is a tool-loop signal; unrelated
  wrong roots are usually model variance until a shared trace motif appears.

### 3. Preserve the trace before summarizing

Use monobench's own evidence tools before ad hoc tailing:

```text
inspect -> integrity -> adoption -> trace -> evidence -> export -> monomento index/search
```

The goal is not only the final grade. The trace should show first tool use,
monogram share, grep/find fallback, stale-index symptoms, broad dumps, wrong
region drift, and whether the solver reached the right neighborhood.

### 4. Compare success and failure rails

Do not inspect a MISS alone. Compare it with at least one FULL or stronger run on
the same instance/model/tool family when available.

Record:

- first divergence;
- comparable cohort: same model, same CLI, same monogram version/epoch, same
  prepared-index policy;
- pre-failure prefix shared with other MISS runs;
- commands followed or ignored;
- region/symbol that stayed stable;
- region/symbol where drift started;
- output budget or JSON/context dump that changed reasoning;
- proof invariant that was reached but not promoted.

### 5. Classify by code-analysis method

Before proposing an implementation, classify the missing method:

| Method | monogram form |
|---|---|
| grep/raw search | raw code hits plus structural refs and containing regions |
| symbol lookup | definition pinning, line hints, homonym/file/lang filters |
| call graph | caller/callee proximity, fan-out budget, frontier staging |
| dependency graph | deps/rdeps, import/export relationships, path ownership |
| coupling | HTTP, SQL, pubsub, Tauri IPC, FFI, event, CSS token, export/import contracts |
| metrics/risk | long functions, fan-in/out, params, depth, dead/unused surfaces |
| ownership proof | inverse-operation balance, retain/release, ref/deref, free/use order |
| output shaping | compact proof markers, summary-first JSON, NEXT preservation |
| scoring | broad-term damping, coverage, length normalization, field/component separation |

If the method already exists but the solver did not use it, improve discovery,
NEXT, output budget, or ranking before adding another command.

### 6. Translate to monogram evidence

Valid proposal terms are measurable from query, index, code, and telemetry:

- trigram/query overlap;
- query-term coverage;
- broad-term damping;
- structural reference density;
- caller/callee proximity;
- coupling boundary match;
- inverse-operation balance;
- region size normalization;
- compact proof marker;
- output budget;
- staged frontier command.

Invalid proposal terms:

- benchmark answer file;
- exact answer function;
- exact answer field;
- answer-key-only symbol;
- known root cone copied from a solved run.

### 7. Borrow scoring discipline from monomento

Use monomento as the reference for formula discipline, not as a code-search
replacement.

Useful borrowed shapes:

- IDF-like broad-term damping for generic symbols;
- coverage multipliers over raw hit count;
- BM25-like length normalization for large files/regions;
- field/component separation;
- explain/debug output;
- benchmark-driven tuning.

monogram must keep its own evidence domains: graph, coupling, boundary, language,
ownership, and command-flow guidance.

### 8. Implement only a generalized mechanism

Allowed implementation targets:

- monogram scoring formula;
- region clustering or score-debug;
- chain/tree fan-out and frontier behavior;
- grep/refgrep structural refs;
- coupling no-match recovery;
- context proof markers;
- JSON compact envelope;
- monobench parser/report/audit visibility;
- NIIA research handoff.

Reject any change that needs benchmark literals to work.

### 9. Validate with holdouts

After a change, rerun:

1. the failed case that exposed the issue;
2. at least one prior FULL holdout;
3. one unrelated hard instance;
4. `monobench monogram-audit <id>` when command shape or output budget changed.

Success means the trace improved, not only the grade:

- narrower region selection;
- fewer broad dumps;
- stronger proof evidence;
- better NEXT adherence;
- score-debug separates root region from decoy;
- adoption stays real and git/index contamination stays controlled.

### 10. Store memory without contaminating solvers

Store reusable analysis under research docs or NIIA work memory. Keep answer keys
and benchmark literals out of solver prompts, tool skills, and generalized maker
instructions.

The final artifact should be a maker proposal:

```text
observed pattern:
source evidence:
analysis method involved:
monogram primitive affected:
candidate score/proof/budget/NEXT change:
contamination risk:
validation set:
```

## Document Audit From This View

Current alignment:

- `monolex-monogram-maker/SKILL.md` now states the methodology-to-scoring goal
  and anti-overfit rules.
- `monolex-monobench-maker/SKILL.md` now states public-surface/internal-first
  and has a tool-maker feedback section.
- `monobench/README.md` now explains that public reports are downstream of
  internal recursive tool-development feedback.
- `monobench/SPEC.md` now defines internal feedback trust as prior to aggregate
  public scores.
- `monogram-methodology-scoring-loop-2026-05-26.md` defines the scoring side of
  this loop.

Remaining watchpoints:

- Public `initiate` docs should stay short and operational; they can point to
  evidence/export/maker feedback, but should not turn into internal research
  essays.
- monogram user docs should remain tool-use guidance. The maker loop belongs in
  maker/research docs, not in solver-facing prompts.
- Any future benchmark result note must be audited for answer-key leakage before
  it becomes a maker rule or tool prompt.
