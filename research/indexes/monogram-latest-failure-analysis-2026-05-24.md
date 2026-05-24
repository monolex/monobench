# Monogram Latest Failure Analysis 2026-05-24

## Scope

This note records the follow-up analysis after the output-budget upgrades. The
latest failures were not caused by low monogram adoption. The failing runs used
monogram heavily, but still entered the wrong candidate cone or spent too long
inside broad indexing/region work.

## Evidence

### Correction: bun-30185 Spark hit-rate after grader fix

The earlier `bun-30185-getheapsnapshot-race: 1/7 FULL` read was partly a
monobench grading artifact, not a monogram behavior fact. `ROOTCAUSE:` lines
were capped at 92 chars; this truncated long C++ paths such as
`src/bun.js/bindings/webcore/JSWorker.cpp::jsWorkerPrototypeFunction_getHeapSnapshotBody`
before the function name could match.

After raising the conclusion-line cap and adding a regression test, existing
results regrade as:

```text
CODEX/GPT-5.3-CODEX-SPARK@HIGH monogram: 4/6 FULL
latest holdout: FULL, 68 monogram calls
```

The remaining true failure pattern is wrong-cone drift into N-API/finalizer or
worker setup decoys. The latest holdout proves the region/raw-budget changes can
now steer Spark to the `JSWorker.cpp` root cause without oversized output.

### Latest scoreboard read

`monobench watch` after the new runs:

- `bun-30196-htmlrewriter-uaf`: 5/5 FULL.
- `ghostty-8208-split-flicker`: 8/9 FULL. The excluded failure is
  `codegraph-r1 FORFEIT` because it could not index the repo due to OOM.
- `ksmbd-37899`: 2/2 FULL.
- `bun-30185-getheapsnapshot-race`: 1/7 FULL.
- `bun-1.3.10-toThreadSafe`: 5/12 FULL.
- Several newer Bun instances currently report `INVALID` only because their
  instance metadata still has TODO/provisional grading markers:
  `bun-20093`, `bun-27838`, `bun-28907`, `bun-29829`, `bun-29951`.

The valid current signal is therefore concentrated in two solver failures:
`bun-30185-getheapsnapshot-race` and `bun-1.3.10-toThreadSafe`, plus one
operational/indexing failure mode.

### bun-30185-getheapsnapshot-race

Ground truth from `monobench show bun-30185-getheapsnapshot-race --spoil`:

- File: `src/bun.js/bindings/webcore/JSWorker.cpp`
- Function: `jsWorkerPrototypeFunction_getHeapSnapshotBody`
- Mechanism: parent VM `Strong<JSPromise>` is captured by value into
  `worker.postTaskToWorkerGlobalScope([strong, parentId] ...)`, causing
  cross-thread `JSC::Strong` copy/destruction against the parent VM HandleSet.

Failing monogram runs had high tool adoption but converged on the decoy:

- `Worker.cpp::createNodeWorkerThreadsBinding`
- `worker_threads`, `workerDataAndEnvironmentData`, `MessagePort`

Before the region update:

- `monogram grep "Strong<JSPromise>" --chain --depth 2` found the exact
  ground-truth line.
- `monogram grep "getHeapSnapshot" --chain --depth 2` found the exact function,
  but old NEXT could point at broad module context.
- Natural `region` queries were too broad and slow, and ranked minified
  benchmark JS or generic `StrongRef.cpp` HandleSet helpers above `JSWorker.cpp`.

## Root Causes

1. Region did not encode the cross-thread Strong/Promise pattern.
   It saw individual terms such as `worker`, `handle`, and `lambda`, but did
   not reward same-region co-occurrence of task handoff and VM handle evidence.

2. Region still had hidden fan-out.
   Default `region` called chain evidence at depth 2. Sampling showed:
   `attach_chain_evidence -> trace_chain -> get_callers`.

3. Region still had hidden global work.
   Sampling after chain was disabled showed:
   `attach_metrics_evidence` and then `attach_coupling_evidence`.

4. Region overused structural refs for natural words.
   `refgrep` ran on broad query terms such as `worker`, `thread`, `handle`, and
   `lambda`. These are useful raw code hints but poor structural-ref keys.

5. `monogram index .` was not incremental.
   A live monobench run was stuck in installed monogram 0.52.1:
   `monogram index .` on `/private/tmp/monobench-work/bun`. Sampling showed the
   hot path inside `Indexer::index_file -> sqlite3_step`, not agent reasoning.

## Source Changes

### Region correctness

- Added risk patterns for cross-thread Strong/Promise handoff:
  - `Strong<JSPromise>`
  - `strong(vm, promise)`
  - `[strong, parentId]`
  - `postTaskToWorkerGlobalScope`
  - `ScriptExecutionContext::postTaskTo`
- Added `cross_thread_handle_cluster` evidence when a region combines task
  handoff with `Strong`/`JSPromise`/`parentId`/HandleSet evidence.
- Added cheap pre-ranking before expensive composite scoring.
- Penalized auxiliary/generated paths such as `bench`, `misctools`,
  `node_modules`, `fixtures`, `vendor`, `.min.`, and `.workerd.js` unless they
  carry the specific cluster evidence.

### Region safety and performance

- Changed default region chain depth from 2 to 0.
- If `--depth` is explicitly requested, region estimates caller fan-out before
  tracing and emits `chain_budget_skipped` evidence instead of expanding unsafe
  chains.
- Metrics evidence is now opt-in by query intent:
  `metric`, `hotspot`, `fanout`, `fanin`, `complexity`, `cycle`, `depth`.
- Coupling evidence is now opt-in by query intent or `--domain`:
  `coupling`, `boundary`, `binding`, `orphan`, `dynamic`, `ffi`, `tauri`,
  `ipc`, `sql`, `http`, `pubsub`, `bridge`.
- Structural refs are now only run for code-shaped patterns, not every natural
  language token.

### Indexing

- Added unchanged-file skip in `Indexer::index_file` using stored `indexed_at`
  mtime.
- Reindex still does a full rebuild because it clears the DB first.
- Changed-file reindex now clears additional file-scoped tables before
  reinserting derived data.

## Verification

Build:

```bash
CARGO_TARGET_DIR=/tmp/monogram-check-target cargo check --bin monogram
CARGO_TARGET_DIR=/tmp/monogram-check-target cargo build --bin monogram
```

Both passed. Existing unrelated warnings remain in:

- `tree_sitter/symbol_extractor.rs`
- `important/specificity.rs`

Region smoke on Bun:

```bash
/usr/bin/time -p /tmp/monogram-check-target/debug/monogram \
  region "worker thread copies parent VM Strong handle across lambda causing HandleSet race" \
  -n 5
```

Result:

- Runtime: 7.63s with the active old `monogram index .` process still consuming
  CPU.
- Rank 1:
  `src/bun.js/bindings/webcore/JSWorker.cpp:656..706`
- Symbol:
  `jsWorkerPrototypeFunction_getHeapSnapshotBody`
- Evidence included:
  `Strong<JSPromise>`, `strong(vm, promise)`, `[strong, parentId]`,
  `postTaskToWorkerGlobalScope`, `ScriptExecutionContext::postTaskTo`, and
  `cross_thread_handle_cluster`.

Grep NEXT smoke:

```bash
/tmp/monogram-check-target/debug/monogram grep "getHeapSnapshot" --chain --depth 2 -n 10
```

Result:

- Exact JSWorker hits are shown.
- NEXT now points to `jsWorkerPrototypeFunction_getHeapSnapshotBody`, not broad
  module context.

Index incremental smoke:

```bash
HOME=/tmp/monogram-home... /tmp/monogram-check-target/debug/monogram index /tmp/monogram-fixture...
HOME=/tmp/monogram-home... /tmp/monogram-check-target/debug/monogram index /tmp/monogram-fixture...
```

Second run result:

- Files: 1
- Identifiers: 0
- Trigrams: 0
- Symbols: 0
- References: 0
- Time: 0 ms

## Remaining Operational Note

The active monobench process observed during this analysis is still using the
installed old binary:

- `monobench run bun-1.3.10-toThreadSafe monogram 7`
- child: `monogram index .`
- installed path: OpenCLIs monogram 0.52.1 package

The source fix will affect rebuilt or reinstalled monogram runs. That active
process cannot benefit from the source changes already made in the workspace.
