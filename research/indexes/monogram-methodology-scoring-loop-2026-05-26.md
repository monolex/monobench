# Monogram Methodology-to-Scoring Loop

Date: 2026-05-26

## Thesis

The monogram recursive loop is not only a failure-pattern patch loop. Its target
is to absorb useful code-analysis methods, express them as monogram evidence,
then harden that evidence into explainable scoring, budget, NEXT, or proof
behavior.

The intended path is:

```text
existing code-analysis method
  -> monogram primitive
  -> trigram/fuzzy/query evidence where useful
  -> explainable score or proof term
  -> monobench success/failure validation
  -> holdout check
```

## What Counts as Source Material

Existing methods are treated as source material, not as final UX:

- grep/raw text search becomes raw code hits plus structural refs and region
  candidates;
- symbol search becomes definition pinning, line hints, and homonym handling;
- call graph search becomes caller/callee proximity and fan-out-aware graph
  proof;
- dependency analysis becomes import/export and cross-file boundary evidence;
- coupling analysis becomes HTTP, SQL, pubsub, Tauri IPC, FFI, event, CSS token,
  and export/import contract evidence;
- metrics become risk signals that help prioritize expensive proof;
- ownership/refcount analysis becomes inverse-operation balance and compact
  proof evidence.

## Where Monomento Helps

Monomento is useful as a scoring architecture reference:

- broad-term damping similar to IDF;
- coverage multipliers instead of raw hit-count ranking;
- length normalization so large files/regions do not win by volume;
- field/component separation, equivalent to monogram separating name, path,
  raw hits, structural refs, chain, coupling, boundary, and risk evidence;
- explain/debug output that makes recursive tuning possible;
- benchmark-driven score adjustment instead of intuition-only constants.

This does not mean copying monomento document search into monogram. Monogram has
graph, boundary, coupling, language, and ownership evidence that monomento does
not own. The shared lesson is formula discipline and explainability.

## Loop Rule

Before editing monogram from a benchmark result, classify the observed failure as
one of:

- missing primitive;
- primitive exists but is not exposed;
- primitive exists but is not ranked;
- broad term or large region is over-rewarded;
- proof evidence exists but is too verbose;
- NEXT sends the solver into a wider cone;
- the distinction requires a proof layer monogram does not currently own.

Only the first six are implementation candidates. The seventh is a boundary to
document unless a measurable indexed-code feature can separate the candidates.

## Implementation Standard

A change is valid only if it can be stated without benchmark answer literals.

Good feature language:

- trigram/query overlap;
- query-term coverage;
- broad-term damping;
- structural reference density;
- caller/callee proximity;
- coupling boundary match;
- inverse-operation balance;
- region size normalization;
- compact proof marker;
- output budget or frontier staging.

Invalid feature language:

- exact benchmark file path;
- exact answer function;
- exact answer field;
- answer-key-only symbol;
- one instance's known root cone.

## Validation Standard

After implementation, rerun:

1. the failed case that exposed the issue;
2. at least one prior FULL holdout;
3. one unrelated hard instance;
4. `monobench monogram-audit <id>` when command shape or output budget changed.

The expected improvement is not only a better grade. The trace should show a
better method: narrower region selection, fewer broad dumps, stronger proof
evidence, better NEXT adherence, or a score-debug explanation that separates the
root region from the decoy.
