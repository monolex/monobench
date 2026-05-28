# Monogram 0.61.31 Loop Update - 2026-05-27

## Scope

This document records the two-hour monobench -> monogram loop segment that
produced and validated the current `lib-monogram` source version `0.61.31`.

The loop focused on:

- keeping monogram changes generalized, not benchmark-answer-coded
- checking version capture for source-built monogram arms
- comparing repeated Haiku behavior across KSMBD, Bun, and CPython instances
- deciding whether the next score change is justified by repeated evidence

## Current Version State

Source version:

- `tauri-apps/lib-monogram/Cargo.toml`: `0.61.31`
- `tauri-apps/lib-monogram/openclis.json`: `0.61.31`
- `tauri-apps/Cargo.lock`: `lib-monogram 0.61.31`

Runtime used for experiments:

```bash
PATH=/Users/macbook/Projects/monolex/monolex/tauri-apps/target/debug:$PATH
```

Important distinction:

- `target/debug/monogram --version` reports `monogram 0.61.31`.
- `~/.openclis/bin/monogram` was still the older installed tool during this loop.
- No OpenCLIs install/publish step was performed in this loop.

## Source Changes In Current 0.61.31

### 1. Version Capture

Added a real `version` / `--version` command path in `monogram.rs`.

Reason:

- monobench version labels should identify the monogram binary actually on
  `PATH`.
- source-built arms previously risked falling back to legacy labels when
  `--version` was not available.

Observed result:

- New runs label as `monogram-0.61.31-claude-haiku-...`.
- Reports aggregate them under `monogram @0.61.31`.

### 2. Guarded Anchor Preserve Hint

Added `guarded_anchor_preserve` / `bounded_contrast_only` output markers for
lifecycle and rootcause guard rails.

Purpose:

- contrast is allowed
- contrast must stay on the same symptomatic field/object/query anchor
- contrast should not open a fresh broad search cone unless local proof
  disproves the current boundary

This is a prompt/output-shaping change, not a benchmark answer rail.

### 3. Removed Bad Hidden-Contrast Experiment

Tried `0.61.30` with hidden contrast candidate names.

Result:

- KSMBD became worse: more calls, higher cost, slower median time.
- Hiding contrast names caused wider search instead of tighter reasoning.

Decision:

- Reverted the hidden contrast behavior.
- Current `0.61.31` keeps contrast candidates visible and only adds guard
  markers.

## Validation Commands

Build and surface checks:

```bash
cd /Users/macbook/Projects/monolex/monolex/tauri-apps
cargo build -p lib-monogram --bin monogram
PATH=/Users/macbook/Projects/monolex/monolex/tauri-apps/target/debug:$PATH monogram --version
monogrid tauri-apps/lib-monogram/src/bin/initiate/initiate.md --check
```

Observed:

- build succeeded
- `monogram --version` returned `monogram 0.61.31`
- `monogrid` reported `0 issue(s)`

Known build noise:

- workspace profile warnings
- pre-existing unreachable-pattern warning in symbol extraction
- pre-existing dead-code warnings

## Experiment Timeline

### KSMBD Baseline: 0.61.28

Command shape:

```bash
monobench matrix ksmbd-37899 \
  --tools monogram \
  --cli claude \
  --model haiku \
  --runs 2 \
  --jobs 2 \
  --prepared \
  --tag haiku-v06128-ksmbd-r2j2
```

Result:

- FULL: `2/2`
- median cost: `$0.27`
- median time: `155s`
- calls: `16-18-20`
- monogram share: `58%`

Pattern:

- correct, but still showed rootcause-label guard pivot pressure.

### KSMBD Guarded Anchor: 0.61.29 / 0.61.31-Equivalent

Command shape:

```bash
PATH=/Users/macbook/Projects/monolex/monolex/tauri-apps/target/debug:$PATH \
monobench matrix ksmbd-37899 \
  --tools monogram \
  --cli claude \
  --model haiku \
  --runs 2 \
  --jobs 2 \
  --prepared \
  --tag haiku-v06129-guarded-anchor-ksmbd-r2j2
```

Result:

- FULL: `2/2`
- median cost: `$0.23`
- median time: `100s`
- calls: `21-22`
- monogram share: `63%`

Interpretation:

- faster and cheaper than the 0.61.28 baseline
- more tool calls, but better bounded path behavior
- residual issue: one run still used broad output / pivot pressure

### KSMBD Hidden Contrast: 0.61.30

Command shape:

```bash
PATH=/Users/macbook/Projects/monolex/monolex/tauri-apps/target/debug:$PATH \
monobench matrix ksmbd-37899 \
  --tools monogram \
  --cli claude \
  --model haiku \
  --runs 2 \
  --jobs 2 \
  --prepared \
  --tag haiku-v06130-hidden-contrast-ksmbd-r2j2
```

Result:

- FULL: `2/2`
- median cost: `$0.34`
- median time: `165s`
- calls: `26-31`
- monogram share: `56%`

Interpretation:

- rejected experiment
- hiding contrast candidate names created more search/fanout
- reverted before 0.61.31

### Bun Holdout: 0.61.31

Instance:

```text
bun-30185-getheapsnapshot-race
```

Command shape:

```bash
PATH=/Users/macbook/Projects/monolex/monolex/tauri-apps/target/debug:$PATH \
monobench matrix bun-30185-getheapsnapshot-race \
  --tools monogram \
  --cli claude \
  --model haiku \
  --runs 2 \
  --jobs 2 \
  --prepared \
  --tag haiku-v06131-bun-getheap-r2j2
```

Result:

- FULL: `2/2`
- median cost: `$0.24`
- median tokens: `1.11M`
- median time: `174s`
- calls: `15-19-23`
- monogram share: `63%`
- integrity: both runs `CLEAN`

Trace summary:

- r1: correct root, but used some shell `find/grep` post-filtering
- r2: correct root with no grep/find fallback

Audit:

- issues: `0`
- oversized: `0`
- maker recommendation: `broad_output_or_fanout_loop count=2`

Decision:

- no regression from guarded lifecycle marker changes
- still keep broad-output/fanout reduction as a future scoring/output task

### CPython Grouper Holdout: 0.61.31

Instance:

```text
cpython-147962-grouper-reentrant
```

Comparison arm:

- `0.61.28`: `2/2 FULL`, median cost `$0.25`, median time `125s`,
  calls `20-25-29`, monogram share `45%`

0.61.31 command shape:

```bash
PATH=/Users/macbook/Projects/monolex/monolex/tauri-apps/target/debug:$PATH \
monobench matrix cpython-147962-grouper-reentrant \
  --tools monogram \
  --cli claude \
  --model haiku \
  --runs 2 \
  --jobs 2 \
  --prepared \
  --tag haiku-v06131-cpython-grouper-r2j2
```

Follow-up third sample:

```bash
PATH=/Users/macbook/Projects/monolex/monolex/tauri-apps/target/debug:$PATH \
monobench matrix cpython-147962-grouper-reentrant \
  --tools monogram \
  --cli claude \
  --model haiku \
  --runs 1 \
  --jobs 1 \
  --prepared \
  --tag haiku-v06131-cpython-grouper-r3
```

0.61.31 result after 3 samples:

- FULL: `2/3`
- DECOY: `1/3`
- median cost: `$0.21`
- median tokens: `1.06M`
- median time: `135s`
- calls: `18-18-19`
- monogram share: `38%`

DECOY classification:

- The decoy run used a generic `region` query around grouping/key/reentrancy.
- Region top results jumped to unrelated UI/config code because lexical
  `custom key` matched better than the intended iterator implementation.
- The agent later inspected the right file neighborhood but labeled the wrapper
  rather than the child iterator boundary.

Decision:

- do not change scoring yet from one DECOY
- record as candidate class:
  `generic-region lexical decoy -> wrapper/factory root label`
- require repeated evidence before changing region score formula

### CPython RawMutex Holdout: 0.61.31

Instance:

```text
cpython-148852-rawmutex-wakeup
```

Command shape:

```bash
PATH=/Users/macbook/Projects/monolex/monolex/tauri-apps/target/debug:$PATH \
monobench matrix cpython-148852-rawmutex-wakeup \
  --tools monogram \
  --cli claude \
  --model haiku \
  --runs 2 \
  --jobs 2 \
  --prepared \
  --tag haiku-v06131-cpython-rawmutex-r2j2
```

Result:

- FULL: `2/2`
- median cost: `$0.25`
- median tokens: `695k`
- median time: `200s`
- calls: `11-11-11`
- monogram share: `55%`
- integrity: both runs `CLEAN`

Trace summary:

- both runs converged to the same root
- no git attempts
- no grep/find fallback
- raw mutex synchronization/lifetime search path remained stable

Audit:

- issues: `0`
- oversized: `0`
- `free_site_triage`: `2`
- `region_first_next`: `2`
- `region_score_debug`: `2`

Decision:

- strong unrelated holdout pass
- argues against an immediate scoring rollback

## Current Evidence Table

| Instance | Version | Runs | Result | Median Cost | Median Time | Calls | Notes |
|---|---:|---:|---:|---:|---:|---:|---|
| KSMBD | 0.61.28 | 2 | 2/2 FULL | $0.27 | 155s | 16-18-20 | baseline for guarded anchor |
| KSMBD | 0.61.29/31 shape | 2 | 2/2 FULL | $0.23 | 100s | 21-22 | faster/cheaper, more calls |
| KSMBD | 0.61.30 hidden contrast | 2 | 2/2 FULL | $0.34 | 165s | 26-31 | rejected and reverted |
| Bun getHeapSnapshot | 0.61.31 | 2 | 2/2 FULL | $0.24 | 174s | 15-19-23 | holdout pass |
| CPython grouper | 0.61.28 | 2 | 2/2 FULL | $0.25 | 125s | 20-25-29 | comparison arm |
| CPython grouper | 0.61.31 | 3 | 2/3 FULL | $0.21 | 135s | 18-18-19 | one lexical-decoy run |
| CPython RawMutex | 0.61.31 | 2 | 2/2 FULL | $0.25 | 200s | 11-11-11 | unrelated holdout pass |

## Hardcoding Review

No answer-key hardcoding was added in this loop.

Allowed/general changes:

- `version` / `--version` command
- generic guarded anchor preserve markers
- generic documentation of bounded contrast behavior

Rejected/removed:

- hidden contrast candidate behavior from 0.61.30

Known residual literals:

- historical docs and benchmark logs still contain example names because they
  describe prior experiments
- active `monogram.rs` should not contain the old answer-cone benchmark literal
  comments that were cleaned during the loop

## Current Decision

Keep current `0.61.31` source as the best candidate from this segment.

Do not add a new region-score change yet.

Reason:

- the clearest new failure class appeared once in CPython grouper
- the third CPython grouper run recovered to FULL
- Bun and RawMutex holdouts passed
- changing scoring from one DECOY risks overfitting or damaging unrelated
  successful rails

## Next Candidate Improvement

If the `generic-region lexical decoy -> wrapper/factory root label` pattern
repeats, the next change should be a generalized region/context improvement, not
a benchmark literal.

Candidate directions:

- make runtime/reentrancy/mutation queries less vulnerable to UI/config lexical
  matches such as "custom key"
- improve region scoring when concrete implementation files are available but
  generic natural-language terms rank documentation/config surfaces
- add a bounded context hint for factory/wrapper functions that create child
  iterator/handle objects and then require the child boundary to be checked
  before naming the wrapper as `ROOTCAUSE`

Validation requirement before implementing:

- reproduce the same decoy shape in at least two runs or a second unrelated
  instance
- verify Bun getHeapSnapshot and CPython RawMutex remain stable afterward

