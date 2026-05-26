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

## Roles

| System | Role in the loop |
|---|---|
| monobench | Runs controlled experiments, preserves telemetry, exposes adoption, drift, output, and grading evidence. |
| monogram | Provides code-search primitives: trigram search, region, grep/refs, context, chain/tree, coupling, metrics, audits, and NEXT guidance. |
| monogram maker | Converts repeated evidence patterns into generalized implementation changes: primitive, score, budget, proof marker, or NEXT. |
| monomento | Supplies scoring discipline and memory navigation: IDF-like damping, coverage, length normalization, explain fields, and searchable exported transcripts. |
| NIIA | Keeps durable research notes, handoff context, and cross-session loop continuity. |
| monokist | Keeps structural relationships between methods, evidence, command flows, and implementation boundaries. |

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
