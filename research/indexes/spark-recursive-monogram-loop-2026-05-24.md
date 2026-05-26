# Spark Recursive Monogram Loop 2026-05-24

## Decision

Use GPT-5.3-Codex-Spark as the cost-favorable canary for recursive monogram
development. Spark is useful because it exposes failure loops cheaply: broad
search expansion, chain fan-out, context over-reading, JSON dump hazards, and
wrong-region selection.

The goal is not to tune monogram to one benchmark answer. The goal is to extract
general tool failures from monobench traces and convert them into safer
monogram behavior.

## Loop

1. Run Spark on a valid hard instance:

   ```bash
   monobench matrix <id> \
     --tools monogram \
     --cli codex \
     --model gpt-5.3-codex-spark \
     --effort high \
     --runs N \
     --prepared
   ```

2. Read:

   ```bash
   monobench report <id>
   monobench adoption <id>
   monobench trace <id> <run>
   monobench monogram-audit <id>
   ```

3. Read the spoil only in the orchestrator context:

   ```bash
   monobench show <id> --spoil
   ```

   Never pass the spoil content to the solver.

4. Classify the failure:

   - output budget / full dump
   - chain fan-out
   - wrong-region ranking
   - missing primitive
   - missing invariant detector
   - stale index / index runtime
   - misleading or missing NEXT
   - invalid instance metadata
   - solver reached evidence but failed to promote it

5. Implement the generalized monogram fix.

   Do not encode the benchmark filename/function. Convert the observed failure
   into a reusable behavior: budget gate, compact output, better NEXT, region
   cluster, ownership invariant, language-aware ranking, or index/runtime fix.

6. Re-test:

   - the failed Spark case
   - one prior FULL case
   - one unrelated holdout
   - `monobench monogram-audit <id>`
   - `cargo check --bin monogram`
   - focused CLI smoke commands on the affected behavior

7. Document:

   Add a dated note under this directory with the failed run, trace evidence,
   generalized fix, and verification command output.

## Current Lessons

### Output Size

The first major class was output explosion, not lack of monogram adoption.

Observed in `bun-1.3.10-toThreadSafe` Spark runs:

- `search "PathLike" --explain -n 40` produced a large ecosystem tour.
- `chain "toThreadSafeSlice" --callers --depth 4` produced a 1.47MB fan-out.
- `context --code >=100` repeated across adjacent symbols kept the solver near
  the right subsystem but away from the proof.

Fix direction:

- preflight fan-out
- staged depth
- compact JSON with `next_hint`
- context caps
- safer NEXT toward `region`, `context --code 80`, and `chain --depth 1`

### Wrong Region

`bun-30185-getheapsnapshot-race` showed that monogram could be heavily used and
still rank the wrong cone. The solver pursued `Worker.cpp`/`MessagePort` decoys
instead of the `JSWorker.cpp` cross-thread `Strong<JSPromise>` capture.

Fix direction:

- region-level evidence clusters
- risk patterns that combine task handoff with VM/Strong/Promise evidence
- auxiliary/generated path penalties unless they carry the precise cluster

### Evidence Reached But Not Promoted

`bun-1.3.10-toThreadSafe` showed a subtler failure: the solver opened
`BunString__toThreadSafe`, but did not promote it to root cause. The missing
evidence was an invariant:

- new impl from `isolatedCopy`
- assignment replaces old impl
- `leakRef` retains the new impl
- no same-region old-impl `deref`
- a compensating `orig.deref()` exists on some Zig paths

Fix direction:

- ownership/refcount balance detectors
- missing inverse-operation evidence
- cross-language compensation warnings
- scoring that rewards invariant proof more than raw hit count

## Guardrail

Spark is the canary, not the target. A change is not accepted just because it
improves one Spark run. It must reduce a general failure class and survive
holdout checks.

## Loop Execution 2026-05-24 Afternoon

Ran `bun-1.3.10-toThreadSafe` with GPT-5.3-Codex-Spark high effort in three
batches while updating monogram/monobench between batches.

### Batch A: Dirty Prepared Snapshot

Run: `monogram-codex-gpt-5.3-codex-spark-high-r1-t1779596285947`

Result: FULL, but expensive.

- Time: 1615s
- Calls: 131 total, 82 monogram
- Cost: $3.14
- Failure class exposed: prepared DB looked stale, so Spark ran `monogram
  reindex .` and later `search ... -r`, burning ~7-8 minutes on full Bun index.

General fix:

- `monobench prepare` now clears the stable clone's monogram SQLite DB before
  snapshot indexing, so prepared snapshots do not accumulate duplicate absolute
  and relative paths.
- prepared install now refreshes `files.indexed_at` against the target worktree,
  including relative paths.

Verification:

- Prepared snapshot now has `9319|0|9319|0` for total/absolute/relative/other.
- Worker logs show `refreshed monogram mtimes updated=9319 missing=0`.

### Batch B: Clean Prepared Snapshot

Runs:

- `monogram-codex-gpt-5.3-codex-spark-high-r1-t1779600035356`: FULL
- `monogram-codex-gpt-5.3-codex-spark-high-r2-t1779600035356`: FULL

Effects:

- No index/reindex loop.
- Runtime dropped to 486s and 294s.
- Valid traces began using `region` and bounded context.

General fix added during this batch:

- `monogram` no-arg help now exits 0. The monobench skill intentionally asks
  the solver to run `monogram` first, so help-as-nonzero was teaching agents
  that the tool failed.
- `grep/refgrep --chain/--tree` now cap broad match expansion and emit
  `grep_chain_capped`, `budget_truncated`, and region-first NEXT.

Verification:

- `/tmp/monogram-check-target/debug/monogram` exits `code=0`.
- `monogram refgrep "toSlice" --chain` emits `grep_chain_capped` and a region
  NEXT instead of dumping every caller cone.
- `CARGO_TARGET_DIR=/tmp/monogram-check-target cargo check --bin monogram`
  passes with existing warnings only.

### Batch C: Updated Prompt Surface

Runs:

- `monogram-codex-gpt-5.3-codex-spark-high-r1-t1779601001922`: NAME_ONLY
- `monogram-codex-gpt-5.3-codex-spark-high-r2-t1779601001922`: FULL

Effects:

- The new monobench lead prompt was visible in trace: region-first flow before
  deep chains.
- r2 found the correct root cause in 187s, 52 calls, $1.18.
- r1 still found the right function name but missed full proof details.

Prompt fix:

- `harness/tools/monogram/lead.md` now recommends:
  `search -> region -> context --code 80 -> chain depth 1/2`.
- It explicitly says `chain --depth 3+` is only for a proven symbol after
  fan-out NEXT says it is safe.
- `harness/tools/monogram/initiate.md` was minimally updated to include
  `region`, `refgrep`, raw+structural `grep`, and ownership-region guidance.

### Remaining Failure Classes

- Static prompt still needs full synchronization from live monogram help. The
  top-level lead is fixed, but some lower examples still mention older chain
  habits.
- `context 1060` and other line-number-only seeds cause bad symbol resolution
  and can show misleading `Files: 0` status. `context` should detect numeric or
  `file:line` seeds and suggest `grep`, `region`, or `context <symbol>` rather
  than trying symbol lookup.
- `coupling --domain ffi --all` can still dump hundreds of KB. It needs the
  same summary-first budget treatment as chain/context/search.
- `chain --depth 2` can still be too large for generic symbols such as
  `fromUTF8`; fan-out preflight should not only guard depth >=3.
- Agents still typo long worktree paths. Shorter run aliases or trace-local
  `$REPO` guidance would reduce `bad_workdir_path`.

### Current Scoreboard Snapshot

After this loop, `bun-1.3.10-toThreadSafe` has Spark high:

- 6 valid Spark monogram runs
- 4 FULL, 1 NAME_ONLY, 1 MISS
- median time 486s
- calls min/median/max: 52/93/131

The important result is not only higher FULL rate. The loop turned three
failure classes into concrete fixes: prepared DB freshness, help exit semantics,
and grep/refgrep chain output caps.

## Loop Execution 2026-05-24 Evening

Second pass targeted the residual failure classes exposed by the afternoon
audit:

- `chain --depth 2` can still exceed 50KB for broad caller cones.
- `coupling --domain ffi --all` still has a 289KB dump path.
- `context 1060 --code 80` treats a line seed as a symbol query.
- Some `bad_workdir_path` issues come from solvers fabricating old worktree
  paths instead of staying in the prepared repo.

### Source Changes

Implemented source-level upgrades in `lib-monogram`, not a separate guarded
wrapper:

- `chain` now preflights unfiltered caller `--depth >= 2` when direct fanout is
  already broad. It blocks inline output and gives staged depth/file/lang/through
  NEXT commands.
- `coupling --domain ffi --all` now uses summary-first output when no narrowing
  pattern/framework/category is provided.
- `context` now detects numeric and `file:line` seeds, emits
  `context_needs_symbol` plus `line_seed_redirect`, and points to symbol/region
  recovery instead of pretending the number is a symbol.
- `symbols --json` now compacts large JSON into a summary envelope with
  `next_hint`.
- `context` and `symbols` now parse `--file` and `--lang`, so solver attempts to
  narrow homonyms actually change the result set.
- `region` now also parses `--file` and `--lang`, so region-first recovery can
  stay inside the intended file/language instead of silently ignoring filters.
- `grep` now parses `--file` and `--lang`, so raw/ref fallback commands emitted
  by recovery hints are real narrowing commands.
- `context` now prioritizes ownership/FFI callees in NEXT. For
  `toThreadSafe --file src/string.zig`, NEXT moves toward
  `BunString__toThreadSafe` instead of the wrapper's first incidental callee.
- `context` now emits an `ownership_imbalance_candidate` warning when the local
  code region contains ref/retain/leak-like evidence without a same-region
  release/deref inverse.
- MCP schema and initiate docs now expose the new `symbols/context --file/--lang`
  narrowing surface. Region docs/schema expose the same filter surface.
- monobench's monogram lead now explicitly says the prepared worktree is already
  the current directory and agents should not `cd /tmp/monobench-work/...`.
- `symbols` flag-only calls now return a compact `symbol_query_required`
  recovery response with NEXT instead of exiting nonzero.

### Smoke Verification

All checks used the debug binary from `/tmp/monogram-check-target/debug` against
the prepared Bun worktree.

- `monogram context 1060 --code 80` prints bounded line-seed recovery markers.
- `monogram coupling --domain ffi --all` shrank from the old 289KB dump path to
  a compact 3KB/55-line guard.
- `monogram chain fromUTF8 --callers --depth 2` is blocked by fanout preflight
  in about 4s instead of printing a 68KB graph after a long run.
- `monogram chain toThreadSafe --callers --depth 2` is also preflighted.
- `monogram symbols String --json` compacts from about 55KB to about 12KB with
  `next_hint`.
- `monogram context BunString__toThreadSafe --code 120` prints
  `ownership_imbalance_candidate`.
- `monogram context toThreadSafe --file src/string.zig --code 120` now stays in
  `src/string.zig` and points at the FFI callee.
- `monogram symbols -n 200 --file ./src/string.zig --json` now returns
  `symbol_query_required` with JSON NEXT instead of a hard nonzero error.
- `monogram region "BunString ownership boundary" --file
  src/bun.js/bindings/BunString.cpp -n 3` keeps the top regions inside that
  file and ranks `BunString__toThreadSafe` first.
- `monogram grep "deinit" --raw -n 20 --file src/string.zig` now finds the
  file-local raw hits, including `SliceWithUnderlyingString.deinit`.
- `monogram region "deinit" --file src/string.zig -n 3` now materializes the
  file-local `deinit` region instead of returning an empty broad-query sample.
- `cargo check --bin monogram` and `cargo build --bin monogram` pass with only
  existing warnings.

### Spark Canary Results

Batch D, after the first guard set:

- `r1-t1779603774059`: FULL, 452s, 114 monogram calls.
- `r2-t1779603774059`: NAME_ONLY, 315s, 51 monogram calls.
- `r3-t1779603774059`: FULL, 389s, 67 monogram calls.

The NAME_ONLY run was useful: it reached `BunString__toThreadSafe` early, then
drifted back to `src/string.zig::toThreadSafe`. The trace showed that
`context/symbols --file` was being ignored and context NEXT was expanding the
wrapper cone instead of the FFI ownership callee.

Batch E, after file/lang filtering, FFI-callee priority, and ownership scent:

- `r1-t1779605071746`: FULL, 1485s, 62 monogram calls, oversized=0.
- `r2-t1779605071746`: FULL, 743s, 102 monogram calls, oversized=0.

This fixed the immediate output-budget regressions: new runs from the second
loop have no oversized outputs. The remaining weakness changed shape. One
graded-FULL answer still labels `ROOTCAUSE: src/string.zig::toThreadSafe` while
the prose correctly describes the Zig-to-FFI handoff through
`BunString__toThreadSafe`. That is no longer a dump/fanout problem; it is a
candidate-labeling and proof-compression problem.

### Current Scoreboard Snapshot

After the second loop, `bun-1.3.10-toThreadSafe` has Spark high:

- 11 Spark monogram runs.
- 8 FULL, 2 NAME_ONLY, 1 MISS.
- Median time 485s.
- Calls min/median/max: 52/87/131.
- Monogram share of tool calls: 86%.

The latest two-run canary after the evening source fixes was 2 FULL / 2 with
zero oversized outputs.

### Remaining Failure Classes

- `context deinit --file src/string.zig` still does not auto-open the file-local
  method, but it now points to working `symbols`/`region`/`grep --file`
  recovery commands. A future improvement can make `context` automatically
  promote the top filtered region when direct symbol resolution is empty.
- The aggregate audit still shows old `bad_workdir_path` and oversized rows
  because it includes historical runs. Newest-only audit would separate live
  regressions from already-fixed failures.
- Installed binary parity is still separate from source validation: old audit
  rows still show no-arg help as exit 1, while the debug source binary exits 0.
- The 1485s FULL run shows a new bottleneck: the agent spent time proving
  ref/deref/deinit ownership balance. A compact ownership-balance/proof recipe
  could turn that reasoning loop into one bounded monogram surface.

## Loop Execution 2026-05-24 Night

This pass changed the recursive loop itself before running more Spark. The
`monolex-monogram-maker` skill now requires explicit success-pattern extraction:
compare a `FULL` trace against the failed trace, identify the successful command
rail, then fix the first divergence rather than only blocking the failed command.

### Batch F: bun-30185-getheapsnapshot-race

Ran three Spark monogram canaries:

- `r1-t1779608650704`: FULL, 412s, 76 calls, 76 monogram.
- `r2-t1779608650704`: FULL, 204s, 32 calls, 32 monogram.
- `r3-t1779608650704`: MISS, 590s, 149 calls, 123 monogram.

Scoreboard after the batch:

- Spark high monogram on this instance: 2 FULL / 3.
- Median time 412s.
- Calls min/median/max: 32/76/149.
- Oversized outputs: 0.

### Success Rail

The shortest success was r2. Its rail:

1. Start from broad symptom terms (`node:worker_threads`, `worker threads`).
2. Use `region "node worker_threads strong handle parent vm" -n 8`.
3. Stay on `JSWorker.cpp::jsWorkerPrototypeFunction_getHeapSnapshotBody`.
4. Read bounded context and targeted raw hits around `Strong<`, `postTaskTo`,
   and parent/worker callback ownership.
5. Conclude that `Strong<JSPromise>` is copied into a worker-thread lambda and
   later destroyed off the parent VM thread.

r1 followed the same root-cause rail but used more grep/context calls.

### Failure Divergence

r3 initially explored the right worker/parent context area, but drifted from the
successful `JSWorker.cpp::jsWorkerPrototypeFunction_getHeapSnapshotBody` rail to
`Worker.cpp::createNodeWorkerThreadsBinding`. The repeated divergence pattern
was not a giant output dump; it was rg-style grep syntax that monogram only
treated literally:

- `monogram grep "worker->clientIdentifier\\(|clientIdentifier\\)" --n 50 bindings/webcore/Worker.cpp`
- `monogram grep "postTaskTo\\(ScriptExecutionContextIdentifier|postTaskTo\\(" --file ... --n 80`
- `monogram grep "void ScriptExecutionContext::postTaskConcurrently|postTaskConcurrently\\(" --n 60`

These returned empty or weak results, then NEXT still suggested deep chain
expansion. The solver kept proving the decoy worker bootstrap path.

### Source Changes

Implemented generalized fixes in `cmd_grep`:

- `grep` now accepts `--n` and `--limit` as aliases for `-n`.
- A trailing positional path after the pattern is accepted as `--file <path>`,
  matching the way agents naturally type rg-style commands.
- Regex-style `A|B` alternation is detected and split into literal searches with
  `regex_alternation_query` markers instead of being treated as one literal.
- Escaped regex parens like `foo\\(` are cleaned when deriving split literals.
- File/lang filtered grep scans a wider candidate set before truncating, so
  broad literals do not get truncated before the file filter is applied.
- `grep` and `refgrep` NEXT now route to `context --code 80`,
  `chain --callers --depth 1`, and `chain/tree --callees --depth 2` rather than
  directly suggesting caller depth 3.

### Smoke Verification

The failed r3 commands now recover:

- `grep "worker->clientIdentifier\\(|clientIdentifier\\)" --n 50
  bindings/webcore/Worker.cpp` expands to two literals and finds
  `createNodeWorkerThreadsBinding` hits instead of no-result.
- `grep "postTaskTo\\(ScriptExecutionContextIdentifier|postTaskTo\\(" --file
  src/bun.js/bindings/ScriptExecutionContext.cpp --n 80` expands to literals and
  finds `ScriptExecutionContext::postTaskTo`.
- `grep "void ScriptExecutionContext::postTaskConcurrently|postTaskConcurrently\\(" --n 60`
  expands to literals and finds both code hits and structural refs.
- The NEXT tail now uses staged depth 1/2 instead of caller depth 3.

### Re-run Note

Started a one-run Spark recheck after the source changes, but the runner stalled
before the first solver command after printing monogram help. It produced no
answer file and no monogram commands beyond help, so it is not a valid canary.
The stale `.running` marker and `.matrix-stop` marker were cleared after
confirming there were no active processes.

### New Failure Classes

- monobench `--prepared` spent about 7 minutes re-indexing the same Bun snapshot
  before each recheck. This was benchmark harness cost, not solver reasoning.
  It is now fixed: `prepare` reuses an existing monogram snapshot unless
  `MONOBENCH_REFRESH_PREPARED=1` or `MONOBENCH_PREPARE_REFRESH=1` is set.
  Smoke: repeated `monobench prepare bun-30185-getheapsnapshot-race --tools
  monogram` now logs `[prepared] reused existing monogram snapshot` instead of
  running `monogram index . --no-workspace`.
  Note: an interrupted pre-fix prepare briefly left an empty snapshot during this
  session; it was rebuilt with `MONOBENCH_REFRESH_PREPARED=1`, verified at 9752
  files / 226.72 MB, and then rechecked for reuse.
- `grep` examples in initiate still include old `--tree --depth 3` examples for
  human workflows. Agent NEXT is now staged, but static examples should be
  audited separately.
- Success r2 shows that the ideal rail is not just "find the file"; it is
  "stay on the parent-VM Strong handle owner function". Region scoring could add
  a stronger bonus for regions that combine `Strong<...>`, worker callback
  transfer, and parent-context postback in the same source slice.

## 2026-05-24 Loop: agy/Gemini Medium as Second Canary

### Experiment

Ran `bun-1.3.10-toThreadSafe` with:

```bash
monobench matrix bun-1.3.10-toThreadSafe \
  --tools monogram \
  --cli agy \
  --model gemini-3.5-flash-medium \
  --effort medium \
  --runs 2 \
  --jobs 2 \
  --prepared
```

Both agy runs graded FULL:

- r1: 82 total tool calls, 41 monogram calls, rootcause
  `src/bun.js/bindings/BunString.cpp::BunString__toThreadSafe`.
- r2: 79 total tool calls, 13 monogram calls, rootcause
  `src/bun.js/bindings/BunString.cpp::toCrossThreadShareable`.

r2 is accepted by the grader but is a weak success rail: it names the documented
decoy helper. It reached enough adjacent evidence to pass, but it did not stay
on the strongest root-cause function.

### Success/Failure Comparison

Strong Spark FULL rail:

1. `region "ownership boundary ref deref leakRef isolatedCopy"`.
2. `context BunString__toThreadSafe --code 80`.
3. Shallow caller/callee hops around `toThreadSafe`, `toThreadSafeEnsureRef`,
   `isolatedCopy`, `leakRef`, and `deref`.
4. Keep proof centered on C++ `BunString__toThreadSafe` and its Zig compensation
   path.

agy r1 mostly followed this rail, then widened to PathLike/PathString late.

agy r2 diverged earlier:

1. It started correctly with `region`, `BunString__toThreadSafe`,
   `toThreadSafe`, and `toThreadSafeEnsureRef`.
2. It then ran `context String`, `symbols String`, `context PathLike`, and
   `context PathString`.
3. Built-in agy `view_file`/`grep_search` recovered enough evidence, but
   monogram did not strongly discourage the generic-symbol drift.

This is not an output-size failure. The new failure class is "generic symbol as
decoy bridge": the output is small, but the NEXT rail still allowed `String` /
`PathLike` / `PathString` to become an investigation center.

### Source Changes

Implemented generalized generic-symbol steering:

- Added `PathString` to the broad ecosystem symbol set.
- `context <generic>` without `--file` or `--lang` now emits
  `generic_symbol_redirect` and `region_first_next`.
- Empty generic context no longer points first to `symbols "<generic>"`; it
  recommends `region`, raw `grep`, then file-filtered `symbols`/`context`.
- Resolved generic context still shows the entry point, but its NEXT is
  file-filtered and asks for a specific caller before expanding.
- Text `symbols <generic>` now warns before graph expansion and points to
  region/file-filtered `context` and `chain`.
- Generic text `symbols` suppresses broad language-aware audit until a concrete
  symbol/file is chosen.
- FFI language-aware NEXT changed from broad `coupling --domain ffi --all` to
  `coupling --domain ffi --summary` plus a pattern-scoped audit.
- The repo-aware footer now suggests `coupling --domain ffi --summary`, not the
  broad domain dump.

### Verification

The normal workspace compile is currently blocked by unrelated
`app-monolex-on-browser` workspace manifest drift:

```text
dependency.lib-monolex-core was not found in workspace.dependencies
```

To validate the monogram crate itself, copied only `tauri-apps/lib-monogram` to
a temporary standalone directory and ran:

```bash
CARGO_TARGET_DIR=/tmp/monogram-standalone-target cargo check --bin monogram
CARGO_TARGET_DIR=/tmp/monogram-standalone-target cargo build --bin monogram
```

Both passed. The only warnings were pre-existing `symbol_extractor.rs`
unreachable pattern and `important/specificity.rs` dead-field warnings.

Smoke checks against the prepared Bun worktree:

- `monogram context PathLike --code 80` now warns
  `generic_symbol_redirect` and suggests `region`, file-filtered `grep`, and
  file-filtered shallow `chain`.
- `monogram context PathString --code 80` now converts the unresolved generic
  context into `region`, raw hit, and file-filtered symbol/context NEXT.
- `monogram symbols String --n 20` now warns and does not suggest unfiltered
  `chain "String"` as the primary next step.
- `monogram search "worker_threads" --cwd -n 20` now prints FFI NEXT as
  `coupling --domain ffi --summary` plus pattern-scoped audit.
- `monogram coupling --domain ffi` remains compact at about 1 KB on the prepared
  Bun worktree.

### Next Loop Target

Re-run the same agy/Gemini canary using the standalone monogram binary path, then
compare whether r2-like generic drift disappears or merely moves to a different
ecosystem noun. If Spark is available, run one Spark holdout on
`bun-1.3.10-toThreadSafe` and one on `bun-30185-getheapsnapshot-race` to ensure
the generic guard does not damage the prior successful rails.

## 2026-05-24 Loop: grading correction and ownership proof rail

### Re-run Result

Re-ran the agy/Gemini canary with the standalone monogram binary that had the
generic-symbol guard. Auto report initially showed both new runs as FULL, but
manual trace showed both final `ROOTCAUSE:` lines were wrong:

- r1: `src/bun.js/bindings/BunString.cpp::WTFStringImpl__isThreadSafe`
- r2: `src/string.zig::String.toThreadSafe`

Both runs mentioned `BunString__toThreadSafe` in the explanation body, which
was enough for the old auto grader to count FULL. After fixing the grader to
prioritize the final `ROOTCAUSE:` line, current aggregate changed materially:

- Spark high monogram: 5/11 FULL instead of 8/11.
- agy/Gemini medium monogram: 1/4 FULL instead of 4/4.
- The old agy r2 is now DECOY because its final rootcause is
  `toCrossThreadShareable`.

### New Failure Pattern

The failure is not output volume. Both new agy runs reached
`context BunString__toThreadSafe --code 80`, which already showed:

```cpp
auto impl = str->impl.wtf->isolatedCopy();
if (impl.ptr() != str->impl.wtf) {
    str->impl.wtf = &impl.leakRef();
}
```

The existing warning said this was an ownership imbalance candidate, but the
next warning also said to inspect the ownership callee before blaming the
wrapper. That made the decoy space stronger than the actual "fresh owner
assigned over an old owner with no release" signal.

### Source Changes

Implemented generalized proof-rail changes:

- `ownership_scent_summary` now distinguishes `isolatedCopy + leakRef + assignment`
  as "fresh/isolated ownership is assigned or leaked without a same-region
  release of the previous owner."
- When a `context` region already has ownership imbalance scent and also calls
  an ownership/FFI callee, NEXT emits `ownership_boundary_priority` instead of
  only `ffi_callee_priority`.
- `ownership_boundary_priority` tells the agent to prove the current wrapper
  releases the previous owner before pivoting to a sibling/callee, and suggests
  same-file `deref`/`release`, `refgrep "leakRef"`, and shallow callers.
- The first "expand callee source" NEXT is suppressed for such regions; the
  callee is shown only after "current wrapper balance is checked."

### Monobench Grader Fix

Updated `grade_text_str` so the final `ROOTCAUSE:` line dominates body mentions:

- correct function on `ROOTCAUSE:` + mechanism => FULL
- correct function on `ROOTCAUSE:` only => NAME_ONLY
- decoy on `ROOTCAUSE:` => DECOY
- wrong `ROOTCAUSE:` even if body mentions the correct function => MISS

Added a regression test:

```bash
cargo test rootcause_line_dominates_body_mentions
```

and rebuilt monobench successfully.

### Verification

Normal monogram workspace compile is still blocked by unrelated
`app-monolex-on-browser` workspace manifest drift, so monogram was again built
from the standalone temp copy. The new context smoke now shows:

```text
[WARN] ownership_imbalance_candidate:
fresh/isolated ownership is assigned or leaked without a same-region release of the previous owner.

[WARN] ownership_boundary_priority:
this region already has a transfer/ref imbalance scent; prove the current wrapper releases the
previous owner before pivoting to a sibling or callee.
```

Next re-run should test whether this keeps agy/Spark on
`BunString__toThreadSafe` after the correct context is reached.

## 2026-05-24 Loop: Spark r2 false-positive ownership smell

### Spark Holdout Result

Re-ran `bun-1.3.10-toThreadSafe` with
`GPT-5.3-Codex-Spark` after the ownership boundary rail:

```bash
PATH=/tmp/monogram-standalone-target/debug:$PATH \
./target/debug/monobench matrix bun-1.3.10-toThreadSafe \
  --tools monogram --cli codex --model gpt-5.3-codex-spark \
  --effort high --runs 2 --jobs 2 --prepared
```

Result:

- r1: FULL, 49/49 monogram calls, final root
  `src/bun.js/bindings/BunString.cpp::BunString__toThreadSafe`.
- r2: MISS, 130/130 monogram calls, final root
  `src/string.zig::toThreadSafeSlice`.

This is useful because r2 did reach and re-read the correct function twice.
The failure was not tool adoption and not only fan-out; it was a false-positive
ownership smell outranking the in-place owner replacement.

### Failure Mechanism

r2 saw this `toThreadSafeSlice` branch:

```zig
// Once for the string
this.ref();

// Once for the utf8 slice
this.ref();

return .{
    .utf8 = ZigString.Slice.init(this.value.WTFStringImpl.refCountAllocator(), slice.slice()),
    .underlying = this.*,
};
```

and concluded "two refs, one deref." But the returned composite owner is
released by `SliceWithUnderlyingString.deinit()`:

```zig
this.utf8.deinit();
this.underlying.deref();
```

So the pattern is a balance probe, not a root-cause proof. The correct root
remains the in-place replacement in `BunString__toThreadSafe`, where a fresh
`isolatedCopy()` is assigned over `str->impl.wtf` without releasing the old
owner.

### Source Changes

- Added `ownership_composite_container_candidate`.
- Composite owner returns with multiple refs feeding distinct fields no longer
  produce `ownership_imbalance_candidate` by default.
- Text and JSON context output now point to `deinit`, `utf8.deinit`, and
  `underlying.deref`, and explicitly say to keep in-place owner replacement
  candidates higher until a returned field is proven unreleased.
- `chain <homonym> --file <path>` now prefers a root definition matching the
  file filter, strips `./` when matching absolute index paths, and hides
  off-file sibling definitions.
- File-filtered chain now suppresses ambiguous unresolved same-language caller
  refs outside the file, while preserving cross-language bridge names such as
  `BunString__toThreadSafe`.

### Smoke Verification

Against the prepared Bun worktree:

```bash
monogram context toThreadSafeSlice --code 140
```

now emits:

```text
[WARN] ownership_composite_container_candidate:
multiple ref-like operations feed distinct returned owner fields; verify the returned container
deinit before promoting this as a leak root cause.
```

and no longer emits `ownership_imbalance_candidate` for that function.

```bash
monogram chain deinit --callers --depth 1 --file ./src/string.zig --strict
```

now starts at `src/string.zig:1221` and stays in the file-local caller set
instead of reopening the global `deinit` ecosystem. A bridge sanity check:

```bash
monogram chain BunString__toThreadSafe --callers --depth 1 --file ./src/bun.js/bindings/BunString.cpp
```

still shows the cross-language Zig callers `toThreadSafe` and
`toThreadSafeEnsureRef`.

### Next Loop Target

Run another Spark holdout on this same task. If it still misses, compare whether
the final decoy is still `toThreadSafeSlice` or whether the failure moves to a
new proof gap. Then run agy/Gemini once to see whether the same marker is
understandable outside Codex.

## 2026-05-24 Loop: contaminated Spark r2 deletion and grep probe

### Deleted Contaminated Run

The Spark run
`monogram-codex-gpt-5.3-codex-spark-high-r2-t1779615890071` was deleted from
the result set. It was too contaminated to keep in the benchmark record:

- prepared snapshot copied a 217 MB DB correctly, but parallel registry writes
  merged two `.registry` lines into one invalid row;
- the solver started on a stale 2-file tmp DB, then ran `monogram reindex .`;
- after the reindex path stalled it issued `kill`, removed an `.indexlock`, and
  inspected DB files directly with `sqlite3`.

That run no longer represents monogram CLI behavior or solver use of the intended
prepared index. Removed:

- result files matching `monogram-codex-gpt-5.3-codex-spark-high-r2-t1779615890071*`;
- its `mcp-empty-*` file;
- its copied per-run monogram DB, WAL/SHM, and indexlock;
- the malformed merged registry row and the duplicate codex-r2 registry row.

### New Failure Pattern From agy

agy r1 after the composite context fix still failed with:

```text
ROOTCAUSE: src/string.zig::SliceWithUnderlyingString.toThreadSafe
```

The important observation is that agy did not use `context toThreadSafeSlice`
when it formed the decoy. It used `monogram grep "fn toThreadSafeSlice"` and
then plain file reads. That bypassed the `context`-only
`ownership_composite_container_candidate` marker.

### Source Change

Added a grep-level ownership probe:

- `grep` now reads each containing function range for displayed code/ref hits;
- if that range has `ownership_scent_summary`, grep prints
  `grep_ownership_probe` with `ownership_imbalance_candidate`;
- if that range has a composite returned-owner pattern, grep prints
  `grep_ownership_probe` with `ownership_composite_container_candidate` and
  `ownership_balance_probe`;
- file filtering in grep now uses the same normalized `./`/absolute suffix match
  used by chain filtering.

Smoke check:

```bash
monogram grep "fn toThreadSafeSlice" --file ./src/string.zig -n 20
```

now emits:

```text
[WARN] grep_ownership_probe: containing function `toThreadSafeSlice` has a composite owner pattern
[marker: ownership_composite_container_candidate]
[marker: ownership_balance_probe]
```

This is meant to catch runners that use monogram for raw hit discovery but then
switch to normal file reads before the `context` rail can correct the decoy.

### Clean Retest

Started a clean Spark retest with sequential jobs to avoid registry races:

```bash
PATH=/tmp/monogram-standalone-target/debug:$PATH \
./target/debug/monobench matrix bun-1.3.10-toThreadSafe \
  --tools monogram --cli codex --model gpt-5.3-codex-spark \
  --effort high --runs 2 --jobs 1 --prepared
```

Expected comparison points:

- Does Spark still spend 100+ calls proving `toThreadSafeSlice`, or does the
  grep-level composite marker shorten that detour?
- If a run misses, is the new decoy still `SliceWithUnderlyingString.toThreadSafe`
  or a different ownership-order hypothesis?
- If Spark stabilizes, rerun agy/Gemini because agy was the runner that exposed
  the `grep -> Read` bypass.

### Contaminated agy Retest Deletion

The agy/Gemini retest after the grep probe exposed a different harness/runner
failure, so the run files are deleted instead of being counted as behavioral
evidence.

Runs:

- `monogram-agy-gemini-3.5-flash-medium-medium-r1-t1779617394089`
- `monogram-agy-gemini-3.5-flash-medium-medium-r2-t1779617610748`

Why r1 is contaminated:

- the final answer file contains only two waiting sentences:
  `I will wait for the monogram region command...`;
- the internal trace did see the new `grep_ownership_probe` for
  `BunString__toThreadSafe`;
- after seeing that probe, the runner continued unrelated searches and never
  produced a root-cause answer.

Why r2 is contaminated:

- the answer file is empty;
- the run is recorded as `NO_RESULT` / incomplete, not as a meaningful MISS.

Useful signal kept from r1 before deletion: grep-level probes are visible in
agy output, but the signal is still too late/noisy when it appears after a broad
`isolatedCopy --raw -n 50` result. The next monogram-side improvement should make
ownership probes closer to each relevant raw hit and make the NEXT point to the
flagged containing function first, before generic result-set NEXT hints.

### Grep Probe Priority Fix

The first grep-probe implementation still had a failure mode: broad
`isolatedCopy --raw -n 50` could find `BunString__toThreadSafe`, but the generic
NEXT still preferred the first containing function in the raw list. That can send
agents into `BunDebugger`/WebCore helper regions before the true ownership
replacement candidate.

Source change:

- print `grep_ownership_probe` immediately under the matching code/ref hit;
- suppress repeated inline probes for the same function range;
- rank ownership NEXT by probe strength:
  - fresh/isolated assignment or `leakRef` without same-region release;
  - FFI-style names containing `__`;
  - thread/cross-thread naming;
  - binding/FFI path scent;
- when any ownership probe exists, suppress the generic containing-function NEXT
  and print `Follow ownership probe before generic trace`.

Smoke checks:

```bash
monogram grep "fn toThreadSafeSlice" --file ./src/string.zig -n 20
```

now keeps `toThreadSafeSlice` as a composite balance probe:

```text
[WARN] grep_ownership_probe: containing function `toThreadSafeSlice` has a composite owner pattern
[NEXT] Follow ownership probe before generic trace:
  monogram context "toThreadSafeSlice" --code 80 --file ./src/string.zig
```

```bash
monogram grep "isolatedCopy" --raw -n 50
```

now promotes the stronger candidate despite earlier generic hits:

```text
[NEXT] Follow ownership probe before generic trace:
  monogram context "BunString__toThreadSafe" --code 80 --file ./src/bun.js/bindings/BunString.cpp
```

### Spark Retest Result

Retest:

```bash
monobench matrix bun-1.3.10-toThreadSafe \
  --tools monogram --cli codex --model gpt-5.3-codex-spark \
  --effort high --runs 1 --jobs 1 --prepared
```

Run `monogram-codex-gpt-5.3-codex-spark-high-r1-t1779618196455` finished
FULL:

```text
ROOTCAUSE: src/bun.js/bindings/BunString.cpp::BunString__toThreadSafe
```

The good news: the run found `BunString__toThreadSafe` early and used the new
ownership probe. The remaining cost problem is not root-cause discovery; it is
post-discovery proof drift:

- broad `isolatedCopy` and `StringOrBuffer` greps still produced too many
  ownership-looking side paths;
- `context deinit --file ...`, `context "deinit at 204" --file ...`, and
  `context ref --file ...` repeatedly failed or bounced through generic region
  because the file/line clue was not resolved directly;
- the agent eventually returned to `BunString__toThreadSafe`, but only after
  127 monogram calls and 530 KB of stderr trace.

### Second Tightening

Source changes after the FULL run:

- `ownership_scent_summary` no longer treats plain `isolatedCopy` or plain
  `.ref()` as an imbalance by itself;
- `ownership_imbalance_candidate` now requires stronger transfer evidence such
  as `leakRef`/`retain`, with a special high-priority case for
  `isolatedCopy + leakRef + assignment`;
- `context "<name> at <line>" --file <path>` now resolves the containing indexed
  symbol at that file line directly instead of returning "No entry-point symbols
  resolved" and sending the agent back to generic region search;
- context file filtering now uses normalized path matching instead of raw
  substring containment.

Smoke checks:

```bash
monogram grep "isolatedCopy" --raw -n 50
```

still promotes:

```text
monogram context "BunString__toThreadSafe" --code 80 --file ./src/bun.js/bindings/BunString.cpp
```

while `JSBunInspectorConnection` and `StringOrBuffer` no longer receive
ownership imbalance warnings from plain `isolatedCopy`/`.ref()` alone.

```bash
monogram context "deinit at 204" --file ./src/bun.js/node/types.zig --code 80
```

now returns a bounded code context instead of an empty context failure.

### Spark Retest After Second Tightening

Retest:

```bash
monobench matrix bun-1.3.10-toThreadSafe \
  --tools monogram --cli codex --model gpt-5.3-codex-spark \
  --effort high --runs 1 --jobs 1 --prepared
```

Run `monogram-codex-gpt-5.3-codex-spark-high-r1-t1779619225131` finished
FULL:

```text
ROOTCAUSE: src/bun.js/bindings/BunString.cpp::BunString__toThreadSafe
```

The second tightening compressed the path substantially: the previous clean
Spark FULL used 127 monogram calls; this run used 57. The agent still explored
proof side paths, but the earlier false `StringOrBuffer` ownership cone did not
reappear.

Remaining useful failure pressure from the FULL trace:

- `refgrep "refCountAllocator\|ensureHash"` was treated as one literal query,
  so structural reference search missed an OR-shaped intent;
- `refgrep "Slice\..*deinit\|utf8.deinit" --raw -n 50 --file src/string/wtf.zig`
  did not apply `--file`, so an attempted narrow structural proof could turn
  back into a broad dump;
- regex-like `.*` inside refgrep is still not a supported structural primitive;
  this should redirect to `grep` or `region`, not silently widen.

### Refgrep OR And Filter Fix

Source changes:

- `refgrep` now parses `-n`/`--limit`, `--file`, and `--lang`;
- `refgrep` splits regex-style alternation markers such as `A|B` and `A\|B`
  into multiple literal structural searches and deduplicates results;
- filtered `refgrep` results now use normalized path matching, matching the
  grep/context file-filter behavior;
- no-match output for regex-like alternation now keeps the agent on a compact
  path: try raw-code `grep` for textual patterns or a natural-language
  `region` query for intent-level OR.

Smoke checks:

```bash
monogram refgrep 'refCountAllocator\|ensureHash' --chain --depth 2
```

now prints:

```text
regex_alternation_query
Expanded regex-style OR into literal searches: refCountAllocator, ensureHash
REFGREP: "refCountAllocator\|ensureHash" (4 matches)
```

```bash
monogram refgrep 'ensureHash' --raw -n 20 --file src/string.zig
```

now returns only the `./src/string.zig:68` structural reference, not the broader
RuntimeTranspilerStore references.

```bash
monogram refgrep 'Slice\..*deinit\|utf8.deinit' --raw -n 50 --file src/string/wtf.zig
```

now avoids a broad dump and says to switch to raw-code `grep` or
`region "Slice.deinit utf8.deinit implementation region" -n 5`.

### Spark Retest After Refgrep Fix

Run `monogram-codex-gpt-5.3-codex-spark-high-r1-t1779620006933` finished
FULL:

```text
ROOTCAUSE: `src/bun.js/bindings/BunString.cpp::BunString__toThreadSafe`
```

Compared to the previous clean Spark FULL:

- calls stayed compact: 57 -> 58 monogram calls;
- output shrank: 287,524 bytes -> 269,222 bytes in the stderr trace;
- audit issues stayed low: 2 no-result events, 0 oversized outputs;
- the root cause was reached early, but proof drift continued after discovery.

New failure pressure from the trace:

- `region "BunString__toThreadSafe cross thread shared"` contained an exact
  code-shaped symbol but originally ranked fuzzy `shared` regions too high;
- `grep "inline void ref\(\);"` came from rg/regex habit and no-matched as a
  literal containing backslashes;
- after the right C++ callee was known, the agent still wandered into generic
  `BunString::`/`toWTFString` proof regions before finalizing.

### Direct Symbol Anchor And Regex-Literal Recovery

Source changes:

- `region` now adds `direct_symbol_anchor` proof evidence for code-shaped query
  tokens that resolve to an indexed symbol definition;
- direct symbol anchors are excluded from the raw-only penalty, so an exact
  definition can outrank callsites and fuzzy theme matches;
- generic natural tokens such as `shared` are not direct anchors;
- `grep`/`refgrep` now also try plain literal forms for regex-escaped fragments
  such as `ref\(\)` -> `ref()`.

Smoke checks:

```bash
monogram region "BunString__toThreadSafe" -n 5 --score-debug
```

now ranks the C++ definition first:

```text
REGION 1  ./src/bun.js/bindings/BunString.cpp:211..224
  symbol: BunString__toThreadSafe
  [direct_symbol_anchor:proof] definition `BunString__toThreadSafe` matched direct query token
```

```bash
monogram region "BunString__toThreadSafe cross thread shared" -n 5 --score-debug
```

keeps the same C++ definition above `shared.zig` fuzzy matches.

```bash
monogram grep "inline void ref\(\);" --file ./src/bun.js/bindings/headers-handwritten.h -n 20
```

now emits `regex_literal_unescaped_query` and finds `inline void ref();`.

### Mixed Spark Run And Follow-Up Fixes

Run `monogram-codex-gpt-5.3-codex-spark-high-r1-t1779620951894` finished
FULL, but it is a mixed run: the monogram binary was rebuilt during the run while
the solver was still active. Do not use its 70-call count as a clean before/after
measurement.

Useful signals from the mixed run:

- the first version of `direct_symbol_anchor` was too broad and promoted every
  `toThreadSafe` homonym in `node_fs.zig`;
- grep's generic containing-function NEXT omitted `--file`, so a precise
  `grep "pub fn ref(this: String)" --file ./src/string.zig` still suggested
  broad `context "ref"`;
- `region "toThreadSafeEnsureRef|toThreadSafe|deref" --file ./src/string.zig`
  treated the pipe as a literal and fell back to file-level `<top>`;
- `grep "Ref<.*>::leakRef"` no-matched because grep was literal-only and did
  not compact regex wildcards to useful literals.

Follow-up source changes:

- direct symbol anchors now apply only to high-confidence code-shaped tokens:
  `__`, `::`, or a small exact-symbol set; broad homonyms such as `toThreadSafe`
  are not mass-promoted;
- grep's generic containing-function NEXT now includes `--file <hit-file>` for
  `context` and shallow `chain`;
- `region` tokenization splits `|` into OR-like terms;
- `grep`/`refgrep` compact regex-shaped fragments, e.g.
  `Ref<.*>::leakRef` -> `Ref<>::leakRef`, `leakRef`.

Smoke checks:

```bash
monogram region "toThreadSafe ownership boundary" -n 5 --score-debug
```

now ranks `BunString__toThreadSafe` first through FFI/coupling evidence, while
the `node_fs.zig` homonyms stay below it.

```bash
monogram region "toThreadSafeEnsureRef|toThreadSafe|deref" -n 6 --file ./src/string.zig --score-debug
```

now materializes real `src/string.zig` function regions instead of a file-level
`<top>` fallback.

```bash
monogram grep "Ref<.*>::leakRef" --raw -n 20 --file ./src
```

now emits `regex_literal_unescaped_query` and finds `leakRef` code hits plus the
ownership-probe NEXT.

### Clean Spark Best Run After Anchor/Regex Fixes

Run `monogram-codex-gpt-5.3-codex-spark-high-r1-t1779621533879` finished
FULL:

```text
FULL  $1.04  213s  47 calls  ·47 monogram
ROOTCAUSE: ./src/bun.js/bindings/BunString.cpp::BunString__toThreadSafe
```

This became the clean best Spark run so far by call count, time, and cost:

| run | grade | calls | time | cost | stderr bytes |
|---|---:|---:|---:|---:|---:|
| t1779619225131 | FULL | 57 | 329s | $1.29 | 287,524 |
| t1779620006933 | FULL | 58 | 300s | $1.24 | 269,222 |
| t1779621533879 | FULL | 47 | 213s | $1.04 | 247,893 |

Remaining issues in the best clean run:

- `monogram search "switch on corrupt value" --cwd -n 20` no-matched; this is
  a symptom string from logs, not source text, and NEXT correctly points toward
  grep/region/ownership verbs.
- `monogram coupling --domain ffi --pattern toThreadSafeEnsureRef --all`
  printed `No bindings indexed`, even though `coupling --domain ffi --summary`
  showed 3,783 indexed FFI keys. This phrasing is misleading: the index exists;
  the pattern is a local Zig helper/wrapper name, not a boundary key.

### Coupling Filter No-Match Fix

Source changes:

- `coupling` now distinguishes true empty index from a user filter that matches
  zero keys inside a non-empty domain;
- text output emits `coupling_filter_no_match` and
  `coupling_pattern_no_match`;
- JSON output returns a compact object with `nearest_keys`, `budget.markers`,
  and `next_hint`;
- nearest-key scoring uses monogram's normalized trigram similarity over
  indexed coupling keys;
- NEXT preserves the user's coupling intent but redirects through bounded
  recovery: `region`, `refgrep`, `context`, `coupling --summary`, then one
  boundary-key pattern.

Smoke check:

```bash
monogram coupling --domain ffi --pattern toThreadSafeEnsureRef --all
```

now reports:

```text
coupling_filter_no_match
coupling_pattern_no_match
[WARN] 3783 indexed coupling key(s) exist in domain `ffi`, but the filter matched none.
NEAREST INDEXED KEYS
     52%  BunString__toThreadSafe  2 site(s)  cpp -> cpp
[NEXT]
  monogram region "toThreadSafeEnsureRef ownership boundary retain release" -n 5
  monogram refgrep toThreadSafeEnsureRef --chain --depth 2
  monogram coupling --domain ffi --pattern '<boundary-key>' --all
```

The JSON variant includes `json_next_hint_present`.

Docs updated:

- `src/bin/initiate/initiate.md`
- `src/bin/initiate/SKILL.md`
- `src/bin/initiate/flow-guide.md`
- `src/bin/initiate/audit-guide.md`
- `src/bin/initiate/ownership-ffi-guide.md`

### Spark Retest And Broad-Root Preflight Discovery

A parallel Spark attempt (`t1779622261420`, two jobs) stalled after tool output:

- both child Codex Spark processes stayed at CPU 0%;
- stderr stopped growing at ~282KB and ~278KB;
- no answer files were created after 12 minutes;
- the partial files were deleted to avoid audit contamination.

The follow-up single Spark run
`monogram-codex-gpt-5.3-codex-spark-high-r1-t1779623046856` finished FULL:

```text
FULL  $1.06  315s  59 calls  ·59 monogram
ROOTCAUSE: src/bun.js/bindings/BunString.cpp::BunString__toThreadSafe
```

This run did not hit the old `no_bindings_indexed` path. It used the precise
boundary key:

```bash
monogram coupling --domain ffi --pattern BunString__toThreadSafe --all --min-confidence 0.5
```

New bottleneck discovered:

```bash
monogram chain deref --callers --depth 3
```

The output dump was blocked, but the preflight itself took 33.9s:

```text
fanout_preflight
blocked_inline_output
staged_depth_next
[WARN] caller graph estimate exceeds inline budget:
nodes=121 edges=46089 max_direct_refs=2826 requested_depth=3
```

Conclusion: after output-budget fixes, a second-order problem appears: for
broad ownership roots (`deref`, `ref`, `deinit`) the estimator can be expensive
even when final inline output is blocked. The correct guard must fire before
estimation.

### Broad Chain Root Guard

Source changes:

- added `looks_like_broad_chain_root`;
- unfiltered `chain <root> --callers --depth >=2` now blocks before fan-out
  estimation when `<root>` is broad (`ref`, `deref`, `deinit`, `free`,
  `release`, `retain`, generic ecosystem roots, etc.);
- text emits `broad_chain_root_redirect`, `blocked_inline_output`,
  `region_first_next`, and `ownership_verb_redirect`;
- JSON returns the same markers plus `next_hint`;
- NEXT keeps the graph need but requires a bounded entry point:
  `region`, `refgrep`, `grep --file`, `chain --file`, `chain --lang`, or
  `chain --through`.

Smoke check:

```bash
/usr/bin/time -p monogram chain deref --callers --depth 3
```

now returns the guard in ~1.3s instead of the previous 33.9s preflight:

```text
CHAIN BROAD ROOT GUARD: deref (callers)
broad_chain_root_redirect
blocked_inline_output
region_first_next
ownership_verb_redirect
[WARN] "deref" is too broad for an unfiltered caller graph at depth 3.
[NEXT]
  monogram region "deref ownership boundary" -n 5
  monogram refgrep deref --chain --depth 2
  monogram chain "deref" --callers --depth 2 --file <path>
  monogram chain "deref" --callers --depth 2 --lang <ext>
  monogram chain "deref" --callers --depth 2 --through <boundary-verb>
```

Verification:

```text
cargo build --bin monogram                         PASS
cargo test --lib                                   PASS, 198 tests
monogrid initiate.md SKILL.md flow-guide.md
         audit-guide.md ownership-ffi-guide.md     PASS
```

### Spark Retest After Broad-Root Guard

Run `monogram-codex-gpt-5.3-codex-spark-high-r1-t1779623703569` finished
FULL:

```text
FULL  $1.37  342s  59 calls  ·59 monogram
ROOTCAUSE: ./src/bun.js/bindings/BunString.cpp::BunString__toThreadSafe
```

Comparison to the prior single run:

| run | grade | calls | time | adoption fails | stderr bytes |
|---|---:|---:|---:|---:|---:|
| t1779623046856 | FULL | 59 | 315s | 21 | 297,258 |
| t1779623703569 | FULL | 59 | 342s | 1 | 276,365 |

The broad-root guard did not need to fire in this run because the solver no
longer executed the unfiltered expensive command. It only ran filtered caller
expansion:

```bash
monogram chain deref --callers --depth 2 --file ./src/string.zig
```

Remaining no-match literals were proof-side probes, not root-cause drift:

```bash
monogram grep "struct BunString" --raw --file ./src/bun.js/bindings/BunString.cpp --n 20
monogram grep "assign" --raw --file ./src/bun.js/bindings/headers-handwritten.h --n 20
monogram grep "pub fn deinit" --raw --file ./src/string/wtf.zig -n 20
```

The solver recovered from those through `symbols BunString`, `region`, and
context reads, then finalized the right C++ boundary root. This suggests the
next compression target is not another fan-out block, but better no-match
recovery for file-filtered grep:

- if `grep "<type declaration>" --file <wrong-file>` no-matches but `symbols
  <name>` finds a typed symbol elsewhere, surface that symbol/file directly;
- if a generic verb (`assign`) no-matches under a file filter, suggest region
  ownership verbs and adjacent `release`/`deref` probes rather than plain
  `search`;
- keep treating rg-style habits as recovery opportunities: the option spelling
  already works, but regex/literal intent and wrong-file type declarations
  still need better no-match steering.

### Grep No-Match Recovery Fix

Source changes:

- `grep` now extracts declaration-shaped symbol candidates from no-match
  literals such as `struct BunString`;
- suggestions are sorted by exact symbol-name match before file match, so an
  actual `BunString` struct outranks same-file `BunString__*` helper functions;
- broad proof verbs embedded in literals (`assign`, `pub fn deinit`, `release`,
  etc.) emit `grep_proof_verb_no_match`;
- proof-verb NEXT keeps the agent on local ownership-region and paired
  `release`/`deref`/`leakRef` probes before plain file search.

Smoke checks:

```bash
monogram grep "struct BunString" --raw --file ./src/bun.js/bindings/BunString.cpp --n 20
```

now prints:

```text
grep_declaration_symbol_suggestion
Declaration-shaped literal no-matched; indexed symbol `BunString` exists:
  BunString [struct] bun.js/bindings/headers-handwritten.h:58
[NEXT]
  monogram context BunString --code 80 --file ./src/bun.js/bindings/headers-handwritten.h
```

```bash
monogram grep "pub fn deinit" --raw --file ./src/string/wtf.zig -n 20
```

now prints:

```text
grep_proof_verb_no_match
"pub fn deinit" is a broad proof verb; prefer ownership-region probes over file-level search.
[NEXT]
  monogram region "pub fn deinit ownership boundary" -n 5 --file ./src/string/wtf.zig
  monogram grep "release" --raw -n 20 --file ./src/string/wtf.zig
  monogram grep "deref" --raw -n 20 --file ./src/string/wtf.zig
  monogram grep "leakRef" --raw -n 20 --file ./src/string/wtf.zig
```

Verification:

```text
cargo build --bin monogram                         PASS
cargo test --lib                                   PASS, 198 tests
monogrid initiate.md SKILL.md flow-guide.md
         ownership-ffi-guide.md                    PASS
```

### Spark Retest After Grep No-Match Recovery

Run `monogram-codex-gpt-5.3-codex-spark-high-r1-t1779624407056` finished
FULL:

```text
FULL  $1.15  348s  55 calls  ·55 monogram
ROOTCAUSE: src/bun.js/bindings/BunString.cpp::BunString__toThreadSafe
```

Compared to the prior run:

| run | grade | calls | time | adoption fails | stderr bytes |
|---|---:|---:|---:|---:|---:|
| t1779623703569 | FULL | 59 | 342s | 1 | 276,365 |
| t1779624407056 | FULL | 55 | 348s | 5 | 265,408 |

The call count and output size improved, but adoption failures increased because
the run intentionally probed absent literals and then recovered through the new
markers:

- `grep_declaration_symbol_suggestion` fired for `struct BunString` and pointed
  to `BunString [struct] bun.js/bindings/headers-handwritten.h:58`;
- `grep_proof_verb_no_match` fired for `ref\(\) void` and kept NEXT on local
  ownership probes;
- `coupling_filter_no_match` fired for `toThreadSafeEnsureRef` and named
  `BunString__toThreadSafe` as the nearest boundary key.

Remaining open compression targets:

- `search "BunString__deinit"` no-matches because the agent invents a destructor
  name. A future guard could detect `__deinit`/destructor-shaped absent symbols
  and suggest nearby `BunString__*` lifecycle/ownership symbols instead of a
  plain file search.
- `grep "ref\(\) void"` recovers as a proof verb, but regex-literal fallback
  could also extract the leading identifier `ref` as a compact candidate when
  the full signature-shaped literal misses.
- Adoption fail counters currently count controlled no-match recoveries as
  fails. The audit should distinguish "dead no-match" from "guarded no-match
  with marker + NEXT".

## 2026-05-24 Loop: family-sibling, leading-ident fallback, guarded-vs-dead audit

Picked up the three open candidates above (separate Claude session; prior session
parked, its lib-monogram WIP committed as checkpoint 338e5642 before continuing).

### Source Changes

- `search` no-match on an FFI/namespaced `Prefix__member` query (e.g.
  `BunString__deinit`) now lists real `Prefix__*` sibling symbols, lifecycle/
  ownership members first (`invented_family_prefix` + `lifecycle_rank` +
  `search_family_sibling_suggestion` marker), so an invented destructor/member
  name pivots to a real one instead of a plain file search.
- `grep`/`refgrep` regex fallback now also extracts the leading identifier before
  `(` for signature-shaped literals (`identifier_before_paren`): `ref() void` ->
  `ref`, `inline void ref()` -> `ref`, `Foo::bar()` -> `bar`. The extractor stores
  bare identifiers, so this auto-corrects rg-style `name(args)` habits.
- monobench adoption splits the fail counter: `failed` counts only DEAD no-match
  (denied, or no marker/NEXT), and a new `guarded` counter (shown as `Ng guarded`)
  counts no-match calls that still emitted recovery steering (`[NEXT]`/`[marker:]`).
  monogram-audit classifies the same as a distinct `guarded_no_match` kind. Stops
  controlled no-match recoveries from reading as tool failures.

### Verification (existing runs, no canary)

- monogram `cargo test --lib` PASS (198); monobench `cargo test` PASS (46).
- `monobench adoption bun-1.3.10-toThreadSafe` reclassified prior runs:
  t1779624407056 ⚠5 -> 0 dead + 5g; t1779623703569 ⚠1 -> 0 + 1g;
  t1779623046856 ⚠21 -> ⚠14 + 7g. Audit now reports `guarded_no_match`.
- monogrid on initiate SKILL.md / flow-guide.md PASS.

### Spark Canary

Run `monogram-codex-gpt-5.3-codex-spark-high-r1-t1779627398986` finished FULL:

```text
FULL  $0.99  258s  48 calls  ·48 monogram
ROOTCAUSE: ./src/bun.js/bindings/BunString.cpp::BunString__toThreadSafe
```

Cheapest clean Spark FULL so far ($0.99 vs prior best $1.04; 48 calls vs 47). Both
new behaviors fired live:

- `search_family_sibling_suggestion` x2 — solver issued `search "BunString__deref"`
  (a guessed member) and was steered to real `BunString__*` siblings.
- `regex_literal_unescaped_query` x10 — including `grep "toThreadSafe()"`, where the
  leading-identifier fallback recovered `toThreadSafe`.
- adoption shows 0 dead fails + 5g guarded, confirming the split end to end.

### Next Candidate

- `grep "<broad>" --chain/--tree` (e.g. `grep "deref" --chain --depth 2`) still
  dumps large caller fans (~125KB observed): grep's chain expansion caps match
  COUNT (8) but not per-match caller fanout, and unlike `chain` it has no
  broad-root guard. Apply `looks_like_broad_chain_root` to grep's chain/tree
  expansion — keep raw hits, suppress the graph, redirect to region/filtered chain.
- Separately, `chain <high-fanout-non-broad> --callers --depth 2` (e.g. `fromUTF8`,
  `toThreadSafe`) can still take 90-119s / 50-68KB; the broad-root literal list does
  not cover these, so a fanout-estimate guard at depth 2 is the follow-on.

### Grep Broad-Root Chain Guard (`grep --chain`/`--tree`)

Implemented the first next-candidate above. `cmd_grep` now detects a broad root
(`looks_like_broad_chain_root`, the same predicate `chain` uses) when `--chain`/
`--tree` is set, suppresses the per-match caller expansion (raw hits stay), and
emits `grep_broad_chain_root_redirect` + `blocked_inline_output` with NEXT pointing
to `region` and filtered `chain`. grep previously capped only match COUNT (8), not
the per-match caller fanout, so a broad root could still dump a large graph.

Smoke (monolex repo index; the guard is pattern-keyed, so DB-independent):

- `monogram grep "deref" --chain --depth 2 -n 30` dropped from the ~125KB caller
  fan to ~14KB: raw hits shown, graph suppressed, broad-root redirect + NEXT.
- `monogram grep "deref" --raw -n 3` still returns code hits (raw path unaffected).
- `monogram grep "into_response" --chain --depth 1` is NOT guarded (non-broad name
  still expands — no over-guard).
- `cargo test --lib` PASS (198). flow-guide.md + SKILL.md synced, monogrid PASS.

Holdout canary on `bun-30185-getheapsnapshot-race` running to confirm a+b+d do not
regress the cross-thread race instance. `refgrep --chain` broad-root guard and the
depth-2 high-fanout (`fromUTF8`) estimate guard remain as follow-ons.

## 2026-05-24 Loop: genuine-failure audit + FFI wrapper-vs-owner labeling

### Grade integrity check (no bug)

Before trusting the scoreboard, re-checked a suspected false MISS. `report`/`adoption`
recompute the auto-grade through `grade_text_file` (current grader, worktree-prefix
strip) and only overlay a stored `*.review.json`. A fresh `report` grades
`t1779605071746` r1 as FULL (correct — the slow 1485s run), and the actual MISS is
r2. No stale-grade bug; the earlier "MISS" was an r1/r2 misread. Fresh Spark high
aggregate: **17/23 FULL**.

### Genuine failure mode: Zig wrapper decoy

The 6 genuine Spark MISS runs cluster on two decoys, both Zig wrappers:
`src/string.zig::toThreadSafe` (x4) and `src/string.zig::toThreadSafeSlice` (x2) —
never the C++ FFI owner `BunString.cpp::BunString__toThreadSafe`.

Traced `t1779601001922` (MISS, labels `string.zig::toThreadSafe`): it touched
`BunString__toThreadSafe` 97 times and its answer body literally says "`toThreadSafe`
in Zig delegates to `BunString__toThreadSafe`" — yet the final `ROOTCAUSE:` is the
Zig wrapper. So this is **not** a discovery or boundary-crossing failure; it is a
labeling failure: the solver inspects the real C++ owner but names the thin
delegating wrapper as root cause.

The existing `ffi_callee_priority` only says "inspect the callee before blaming the
wrapper" — it steers inspection but never the ROOTCAUSE label, and the solver did
inspect the callee.

### Source change (candidate g)

- `context <wrapper>`, when the symbol delegates to an ownership/FFI-boundary callee,
  now also prints `ffi_owner_rootcause_hint`: it names the owner and says to put the
  **owner** on the `ROOTCAUSE:` line, not the thin delegating wrapper (general FFI
  ownership guidance, not benchmark-specific). Fires under both
  `ownership_boundary_priority` and `ffi_callee_priority`.
- ownership-ffi-guide.md gains a "Wrapper vs FFI owner — which to name as ROOTCAUSE"
  section, resolving the tension with the existing query-resolution note (query with
  the wrapper's indexed name, but label the owner). flow-guide.md gains the matching
  `ffi_owner_rootcause_hint` bullet.

### (f) verified already handled

`chain <high-fanout-non-broad> --callers --depth 2` (e.g. `fromUTF8`) is already
guarded: `needs_fanout_preflight` fires for depth>=2 callers, and `depth2_wide`
(`max_direct_refs > 80`, etc.) blocks the output. The 119s/68KB `fromUTF8` audit row
was a pre-guard run. No new fix needed.

### Verification status

`cargo check --bin monogram` + `cargo test --lib` PASS (198); docs monogrid PASS.
Behavior verification of (g) is pending: the bun-30185 holdout (testing a+b+d) is
still running and holds the shared `monogram` binary, so the (g) rebuild + a
bun-1.3.10 canary (where the wrapper decoy lives) come next.

## 2026-05-24 Loop: bun-30185 holdout failure + refgrep guard (e) + raw budget (h)

### Holdout result (a+b+d on bun-30185)

`monogram-codex-gpt-5.3-codex-spark-high-r1-t1779627994136`: **MISS, $4.10, 1215s,
237 calls (216 monogram)** — a runaway drift that finalized the decoy
`napi.zig::NapiFinalizerTask::schedule` instead of the correct
`JSWorker.cpp::jsWorkerPrototypeFunction_getHeapSnapshotBody`. (c) worked:
`guarded_no_match x14` kept guarded recoveries out of the dead-fail count. The race
instance remains a hard discovery/convergence problem (wrong-region drift), not an
ownership-labeling one — the a/b/g fixes do not apply here.

The one **oversized output (56KB / 913 lines)**:
`refgrep "\.ref()" --file ./src/bun.js/bindings -n 200` — a broad pattern with a huge
`-n` over a directory filter. Not caught by (e) (no `--chain`); plain grep/refgrep
honor an arbitrarily large `-n`, so the raw list itself becomes a context-flooding
dump.

### Source changes

- **(e) `refgrep --chain`/`--tree` broad-root guard** — symmetric with grep (d):
  `refgrep "deref" --chain` now keeps structural refs and suppresses the caller fan
  (`grep_broad_chain_root_redirect`). Needed because the `chain` broad-root guard's
  own NEXT can suggest `refgrep <root> --chain`. Smoke on the monolex index: 12KB,
  guard fired; non-broad `into_response --chain` still expands.
- **(h) raw-list output budget** — `OutputBudget.max_raw_items = 60`. grep and refgrep
  now cap the displayed raw hit list to `min(-n, 60)` and emit `budget_truncated` +
  a `region` / `--file` narrow NEXT when a large `-n` would have dumped. Targets the
  56KB refgrep case directly; reduces the context flooding that feeds drift.

### Verification

`cargo check --bin monogram` + `cargo test --lib` PASS (198). (e) smoke verified;
(h) smoke + the g-canary (bun-1.3.10, verifying `ffi_owner_rootcause_hint`) come on
the next rebuild. A bun-30185 recheck will confirm (h) removes the 56KB dump and
whether the tighter budgets reduce the 216-call drift.

## 2026-05-24 Loop: FFI owner hint over-correction

The `ffi_owner_rootcause_hint` canary
`monogram-codex-gpt-5.3-codex-spark-high-r1-t1779629351010` finished **FULL**, but
it was expensive:

```text
FULL  $2.36  1718s  103 calls  ·103 monogram
ROOTCAUSE: src/bun.js/bindings/BunString.cpp::BunString__toThreadSafe
```

This run exposed a subtle over-correction. The new wrapper-vs-owner hint did fix
the old label failure, but it also fired when `context` was already reading the
exported boundary/owner symbol:

```text
[WARN] ffi_owner_rootcause_hint: `BunString__toThreadSafe` crosses the C-ABI
boundary into `isolatedCopy`; name `isolatedCopy` as ROOTCAUSE...
```

That is wrong for this class of bug. `isolatedCopy`, `leakRef`, and `impl->deref`
are mechanism evidence when the current region is already the exported owner
where the ref transfer/replacement happens. The ROOTCAUSE label should move from
the Zig wrapper to `BunString__toThreadSafe`, but should not automatically move
one hop further to an incidental callee.

### Source change

- Added `looks_like_ffi_boundary_name`.
- `ffi_owner_rootcause_hint` now fires only for a non-boundary wrapper whose
  callee is a boundary/owner symbol (`toThreadSafe` -> `BunString__toThreadSafe`).
- If `context` is already on a boundary symbol, monogram prints
  `ffi_boundary_rootcause_hint` instead: keep the boundary symbol as the
  root-cause candidate and use the callee only as mechanism evidence.

Smoke:

```text
context toThreadSafe --file ./src/string.zig
  [marker: ffi_owner_rootcause_hint]
  name `BunString__toThreadSafe` as ROOTCAUSE

context BunString__toThreadSafe --file ./src/bun.js/bindings/BunString.cpp
  [marker: ffi_boundary_rootcause_hint]
  keep `BunString__toThreadSafe` as ROOTCAUSE and use `isolatedCopy` as evidence
```

Verification:

```text
cargo check --bin monogram  PASS
cargo build --bin monogram  PASS
cargo test --lib            PASS, 198 tests
monogrid flow/ffi docs       PASS
```

## 2026-05-24 Loop: diversified Spark canary, Ghostty split flicker

Ran the first non-Bun ownership canary with Spark:

```text
ghostty-8208-split-flicker
FULL  $1.77  304s  113 calls  ·113 monogram
ROOTCAUSE: src/apprt/gtk-ng/class/split_tree.zig::propTree
```

This was a useful success because the domain is different: GTK/Zig UI redraw
rather than FFI ownership. `region` immediately found the right `split_tree.zig`
areas (`resize`, `onIdle`, `buildTree`, `propTree`), and the solver eventually
proved that `propTree` clears the child with `tree_bin.setChild(null)` before
rebuild, producing the one-frame blank split.

The cost/call count exposed two new general patterns:

1. **Low-value callee NEXT loops.** `context` suggested callees such as `private`,
   `assert`, `idx`, `allocator`, `warn`, and `new`. The solver followed them and
   spent many calls in utility/helper noise. This is not a correctness failure,
   but it inflates runs and can derail weaker models.
2. **Line-range-as-file misuse.** The solver tried commands like
   `monogram grep "fn new(" src/apprt/gtk-ng/class/split_tree.zig --file 910-980`.
   That overwrote the real file filter with a fake path (`910-980`) and produced
   needless no-match recovery.

### Source changes prepared

- `preferred_context_callee` now suppresses low-information callee names before
  suggesting `monogram context <callee>`.
- `grep` detects `--file 910-980` style line/range mistakes. If a positional file
  path was already present, monogram keeps the real path, emits
  `bad_file_filter_line_range`, and redirects the range to `region`/`context`
  hints.

These changes are intentionally generic: they target the command-use pattern,
not Ghostty's answer.

## 2026-05-24 Loop: bun-30185 false MISS + utility-callee suppression

The latest `bun-30185-getheapsnapshot-race` Spark holdout finished with:

```text
monogram-codex-gpt-5.3-codex-spark-high-r1-t1779631340792
auto-before-fix: MISS
after grader fix: FULL
$1.35  517s  68 calls  ·68 monogram
ROOTCAUSE: src/bun.js/bindings/webcore/JSWorker.cpp::jsWorkerPrototypeFunction_getHeapSnapshotBody
```

This was not a monogram discovery failure. The answer named the ground-truth
function and mechanism (`Strong<JSPromise>` captured into the worker task, parent
VM `HandleSet` touched from the worker thread). The automatic grader cut the
`ROOTCAUSE:` line at 92 characters, truncating the long C++ function name before
`jsWorkerPrototypeFunction_getHeapSnapshotBody` could match.

### Monobench source change

- Raised the root-cause conclusion-line cap to 240 chars.
- Added a regression test for the long `JSWorker.cpp::jsWorkerPrototypeFunction_getHeapSnapshotBody`
  conclusion line.

After the fix, `bun-30185` Spark status is a different signal:

```text
CODEX/GPT-5.3-CODEX-SPARK@HIGH monogram: 4/6 FULL
latest holdout: FULL, 68 monogram calls, no oversized output
remaining true misses: wrong cone drift into N-API/finalizer or worker setup decoys
```

### Monogram source change

The Ghostty and Bun smokes showed the first low-value callee suppression list was
too narrow. It removed `private/assert/new`, but the next suggestions became
`remove`, `defaultGlobalObject`, and `DECLARE_THROW_SCOPE`. These are not useful
root-cause expansion targets; they are API/setup helpers around the already
visible source region.

`preferred_context_callee` now also suppresses:

```text
remove, argument, create, defaultGlobalObject, getVM, isUndefined,
RETURN_IF_EXCEPTION, validateObject, validateBoolean, wrapped,
idleAdd, isOnline, getTreeHasParents, notifyByPspec, promiseStructure,
scriptExecutionContext, setChild, uppercase macro-like callees
```

Smoke evidence:

```text
ghostty propTree context:
  no longer suggests context private/remove/getTreeHasParents/setChild

bun jsWorkerPrototypeFunction_getHeapSnapshotBody context:
  no longer suggests context defaultGlobalObject/DECLARE_THROW_SCOPE/wrapped

grep with --file 910-980:
  emits bad_file_filter_line_range and keeps the real file path

grep "Strong<" --file ./src/bun.js/bindings -n 200:
  emits budget_truncated and caps raw hits at 60
```

Verification:

```text
monogram cargo check --bin monogram  PASS
monogram cargo build --bin monogram  PASS
monogram cargo test --lib            PASS, 198 tests
monobench cargo test                 PASS, 48 tests
monobench cargo build                PASS
monogrid flow/ffi/research docs      PASS
```

## 2026-05-24 Loop: ksmbd helper-destructor decoy

Spark holdout:

```text
ksmbd-37899
MISS  $1.47  190s  67 calls  ·67 monogram
ROOTCAUSE: ./mgmt/user_session.c::ksmbd_sessions_deregister
```

This instance is not a clean discrimination target because baseline Haiku and
monogram Haiku both already solve it. The Spark miss is still useful as a
pattern: the solver found the correct `sess->user` free/read cone, but named a
helper/session-destroy path instead of the operation boundary
`smb2pdu.c::smb2_session_logoff`.

Success rail from `monogram-haiku-r1`:

```text
grep "ksmbd_free_user|user_free|sess->user.*NULL" --chain --depth 3
context ksmbd_session_destroy
grep "conn->sessions" --chain --depth 3
grep "sess->user" --chain --depth 2
context ksmbd_sessions_deregister
grep "ksmbd_free_user"
grep "ksmbd_conn_wait_idle_sess_id"
ROOTCAUSE: ./smb2pdu.c::smb2_session_logoff
```

Monogram behavior problem:

```text
monogram grep "ksmbd_free_user" -n 80
```

showed all direct free sites, including `smb2_session_logoff`, but the generic
NEXT traced the first definition/helper-ish hit. That makes helper destructors
and deregistration functions too easy to label as ROOTCAUSE in lifecycle races.

### Source change

`grep` now detects lifecycle/free/delete/deref/release style queries with
multiple direct call sites and emits:

```text
free_site_triage
[marker: free_site_triage]
```

The new NEXT lists operation/request call sites before helper destructors and
adds a proof warning: choose the ROOTCAUSE by the symptom's lifecycle boundary
(`auth/setup`, `logoff/close`, `teardown`, `finalizer`, etc.), not by the first
helper that contains the free.

This is intentionally generic. It applies to kernel C, C++ destructors, Zig
deinit/deref, N-API finalizers, and any ownership bug where the same release verb
has several plausible call sites.

### Re-test result

Run:

```text
monogram-codex-gpt-5.3-codex-spark-high-r1-t1779634203339
FULL  $0.91  110s  42 calls  ·42 monogram
ROOTCAUSE: ./smb2pdu.c::smb2_session_logoff
```

Before the change, Spark reached `ksmbd_free_user` late and still ended at the
helper/deregistration cone:

```text
MISS  $1.47  190s  67 calls
ROOTCAUSE: ./mgmt/user_session.c::ksmbd_sessions_deregister
```

After the change, `monogram grep "ksmbd_free_user" --cwd -n 80` emitted
`free_site_triage` and suggested:

```text
monogram context "smb2_session_logoff" --code 80 --file ./smb2pdu.c
monogram context "ntlm_authenticate" --code 80 --file ./smb2pdu.c
monogram context "krb5_authenticate" --code 80 --file ./smb2pdu.c
monogram context "ksmbd_session_destroy" --code 80 --file ./mgmt/user_session.c
monogram chain "smb2_session_logoff" --callers --depth 1 --file ./smb2pdu.c
```

The solver followed that first operation-boundary NEXT immediately, opened
`smb2_session_logoff`, and then verified the bound-connection teardown race. The
general lesson is that for lifecycle/free searches, monogram must not treat the
first helper that contains the release as the best root-cause expansion. It
should surface all direct free sites, rank operation/request boundaries ahead of
destructors/helpers, and ask the solver to match the site to the symptom
lifecycle.

Audit after the re-test:

```text
ksmbd Spark: 1/2 FULL
new FULL: calls 42, oversized 0, free_site_triage x2
old MISS: calls 67, oversized 0, free_site_triage absent
```

## Loop Note: Ghostty false FULL and UI/render rail

`ghostty-8208-split-flicker` exposed a different recursive failure mode. The
latest Spark run first graded as FULL, but the answer named
`src/apprt/gtk-ng/class/split_tree.zig::setTree` and argued boxed ownership/deep
copy semantics. The spoiled root cause is not the boxed tree setter itself; it
is the clear-then-async-rebuild path around `propTree` / `onRebuild`.

The grading was too loose:

```text
old full_must_name: ["split_tree"]
new full_must_name: ["propTree", "onRebuild"]
new decoys: setTree, boxedCopy, boxed ownership, deep clone
```

After tightening the instance, the same run becomes DECOY, while the older
`propTree` answer remains FULL. This matters because monogram was not merely
printing too much; it was steering a UI flicker symptom into an ownership cone.

Source changes:

```text
monogram: ownership probes no longer appear in generic region-first NEXT unless the query is ownership/lifetime-shaped.
monogram: UI/render timing NEXT is added for flicker/blank/rebuild/layout/split/tree style intent.
monogram: UI path ranking prefers gtk-ng split_tree/tree_view owners over generic datastruct split_tree files.
monogram: UI rail is query-aware, so Bun worker_threads/Strong handle ownership searches do not get false windows.zig redraw advice.
monobench: ghostty grading now rejects same-file boxed ownership decoys.
```

Smoke verification:

```text
ghostty region "split layout flicker resize"
  ui_render_timing_next -> ./src/apprt/gtk-ng/class/split_tree.zig
  NEXT grep rebuild / idle / setChild, then region "rebuild idle clear blank frame"

ghostty context setTree --file src/apprt/gtk-ng/class/split_tree.zig
  still emits ui_render_timing_next, so a solver caught in the setTree cone is redirected toward rebuild ordering.

bun region "worker_threads strong handle set"
  keeps jsWorkerPrototypeFunction_getHeapSnapshotBody as top region.
  UI/render timing NEXT is absent; FFI/ownership NEXT remains.
```

Next experiment: rerun Spark on `ghostty-8208-split-flicker` with the corrected
grader and new monogram binary. Success means the solver follows the UI timing
rail toward `propTree` / `onRebuild`; failure means the region scorer itself
still over-ranks keyboard/application split action regions before the rebuild
owner.

### Re-test result

Run:

```text
monogram-codex-gpt-5.3-codex-spark-high-r1-t1779635490801
FULL  $0.68  94s  44 calls  ·44 monogram
ROOTCAUSE: ./src/apprt/gtk-ng/class/split_tree.zig::propTree
```

The decisive divergence versus the DECOY run is early:

```text
new FULL:
  search "gtk split flicker blank frame"
  region "split flicker blank frame implementation"
  grep setChild/rebuild/idle --file gtk-ng/class/split_tree.zig
  region "rebuild idle clear blank frame" --file gtk-ng/class/split_tree.zig
  context onRebuild
  context propTree

old DECOY:
  search "split resize flicker"
  region "split layout flicker resize"
  context resize
  chain resize
  context setTree
  grep boxedCopy
  context boxedCopy
  ...
  ROOTCAUSE: setTree
```

This confirms that the useful fix is not only output budget. It is cone
selection: for UI flicker/blank-frame intent, monogram must provide a staged
redraw/rebuild rail before the generic systems-language/ownership rail.

The new FULL still performed an ownership detour after already finding
`propTree`, so the UI rail was tightened again: when UI/render intent produces a
specific UI path, the FFI/ownership NEXT block is suppressed unless the query is
also explicitly lifetime/ownership-shaped. Smoke check:

```text
context setTree --file gtk-ng/class/split_tree.zig
  emits ui_render_timing_next
  no longer emits FFI/ownership commands

region "worker_threads strong handle set"
  does not emit ui_render_timing_next
  still emits systems-language FFI/ownership commands
```

Next experiment: rerun Ghostty once more to see whether removing the FFI detour
reduces calls after `propTree`, then move the same success/failure comparison to
a non-UI hard case.

### Follow-up: ownership seed can reopen the wrong cone

The next Ghostty run did not produce a usable answer. It reached an ownership
loop around `boxedFree -> finalize -> isolatedCopy`, then `codex exec` stopped
writing output for several minutes and was terminated as a runner hang sample.

The useful trace fragment:

```text
grep "boxedFree"
free_site_triage
context finalize --file gtk-ng/class/split_tree.zig --code 160
refgrep "isolatedCopy" --chain --depth 2
```

This shows a second weakness: even if `setTree` no longer prints generic FFI
NEXT, the solver can invent an ownership seed (`boxedFree`) and re-open the same
cone. `grep boxedFree --file gtk-ng/class/split_tree.zig` now treats UI
lifecycle candidates specially:

```text
free_site_triage
ui_lifecycle_free_site_redirect
ui_render_timing_next
  grep rebuild --file gtk-ng/class/split_tree.zig
  grep idle --file gtk-ng/class/split_tree.zig
  grep setChild --file gtk-ng/class/split_tree.zig
  region "rebuild idle clear blank frame" --file gtk-ng/class/split_tree.zig

then only local lifecycle candidates:
  context setTree
  context finalize
  chain setTree --callers --depth 1
```

The generic ownership/FFI rail is suppressed for this UI-lifecycle case, and
monobench audit now counts `ui_lifecycle_free_site_redirect` separately. This
is more aggressive than the first UI rail fix, but it matches the observed
failure: when the original symptom is flicker/blank-frame, a free symbol inside
a UI path is more likely a decoy cone than a root-cause proof.

## Holdout: Bun ownership path still works

To check that the UI gating did not damage real ownership/lifetime cases, Spark
was rerun on `bun-30185-getheapsnapshot-race`.

```text
monogram-codex-gpt-5.3-codex-spark-high-r1-t1779637109441
FULL  $0.55  123s  26 calls  ·26 monogram
ROOTCAUSE: src/bun.js/bindings/webcore/JSWorker.cpp::jsWorkerPrototypeFunction_getHeapSnapshotBody
```

The successful path was compact:

```text
search "node worker_threads strong handles"
region "node worker_threads strong handles ownership boundary"
context jsWorkerPrototypeFunction_getHeapSnapshotBody --code 80
chain jsWorkerPrototypeFunction_getHeapSnapshotBody --callers --depth 1
refgrep "isolatedCopy" --chain --depth 2
grep "postTaskToWorkerGlobalScope" --lang cpp
context postTaskToWorkerGlobalScope
grep "Strong" --file JSWorker.cpp --lang cpp
...
ROOTCAUSE: jsWorkerPrototypeFunction_getHeapSnapshotBody
```

This run is the best Spark result on this instance so far: previous recent FULL
runs were 406s/77 calls and 517s/68 calls, while the bad MISS was 1215s/237
calls. The UI/render gating did not remove the ownership path; it kept Bun on
the strong-handle/cross-thread region and avoided the old broad search/oversized
loops.

## Holdout: KSMBD free-site triage still works

`ksmbd-37899` checks the other side of the UI-lifecycle change: kernel C
free-site triage must still rank operation/request boundaries before helper
destructors.

```text
monogram-codex-gpt-5.3-codex-spark-high-r1-t1779637294138
FULL  $0.57  79s  33 calls  ·33 monogram
ROOTCAUSE: ./smb2pdu.c::smb2_session_logoff
```

Trace:

```text
search "sess->user"
grep "sess->user"
grep "ksmbd_free_user"
context smb2_session_logoff --code 80 --file ./smb2pdu.c
chain smb2_session_logoff --callers --depth 2 --file ./smb2pdu.c
...
ROOTCAUSE: smb2_session_logoff
```

Audit for the latest run shows `free_site_triage x1`, zero oversized output, and
two guarded search no-matches that recovered through NEXT. Compared with the
earlier FULL at 110s/42 calls and the pre-triage MISS at 190s/67 calls, the
free-site ranking continues to improve rather than regress.

## toThreadSafe: signature-shaped context seeds

The latest `bun-1.3.10-toThreadSafe` Spark run was FULL:

```text
monogram-codex-gpt-5.3-codex-spark-high-r1-t1779637450884
FULL  $1.01  270s  48 calls  ·48 monogram
ROOTCAUSE: src/bun.js/bindings/BunString.cpp::BunString__toThreadSafe
```

The run was successful, but it exposed a small command-surface gap:

```text
monogram context "ref()(this: String)" --file ./src/string.zig --code 80
No entry-point symbols resolved from seeds: ["ref"]
```

Agents coming from raw grep output often paste signature-shaped strings into
`context`. `grep` already has identifier-before-paren fallback; `context` did
not. `context` now resolves filtered signature/call-shaped seeds to the
identifier before `(` and emits:

```text
context_signature_symbol_redirect
```

Smoke checks:

```text
context "ref()(this: String)" --file ./src/string.zig
  -> ref at ./src/string.zig:877

context "toWTFString()" --file ./src/bun.js/bindings/BunString.cpp
  -> toWTFString at ./src/bun.js/bindings/BunString.cpp:765
```

This is a narrow UX fix, but it removes a recurring no-entry hop in C/C++/Zig
debugging where the agent has a real function signature but not the exact
indexed symbol seed.

A follow-up Spark rerun could not be counted: `codex exec` hit
`invalid_grant: Invalid refresh token`, stopped writing answer output, and was
terminated as an external runner/auth failure. The smoke checks above verify the
CLI behavior; the next solver-level verification should use agy or a refreshed
Spark session.

## 2026-05-24 Loop: via-niia agy permission prompt

The first `--via niia --cli agy` rerun after the Spark auth failure was also not
valid evidence. It did not test monogram; the answer file was an agy permission
prompt:

```text
Bash(niia)
Requesting permission for: niia
```

Direct agy runs already use `--dangerously-skip-permissions`, but the niia
interactive runner spawned plain `agy`. The runner now adds
`--dangerously-skip-permissions` for `spawn_command("agy", ...)`, including
`MONOBENCH_CLI=agy ...` overrides that omit the flag.

The rebuilt canary then exposed a separate artifact bug. `niia_runner` used
`Path::with_extension("answer.txt")` and `with_extension("meter.json")`.
For dotted model labels such as `gemini-3.5-flash-medium`, that wrote files as
`monogram-agy-gemini-3.answer.txt` instead of preserving the full timestamped
run id. The fix appends artifact suffixes to the file name instead:
`<runid>.answer.txt` and `<runid>.meter.json`.

The invalid orphan artifacts from the pre-build and dotted-name runs were
deleted from the result set.

A third invalid canary showed that plain interactive agy is not stable enough in
the niia headless session: it produced a shell tail and no agy transcript. A
fourth invalid canary showed that pasting the entire prompt into one terminal
command is also too large. The runner now writes the prompt to a temp file and
handles `--via niia --cli agy` as a short one-shot terminal command:
`agy --print "$(cat <prompt-file>)" --dangerously-skip-permissions --log-file
<run>.agy.log`. The niia path still exercises `write`/`wait-idle`/`get-answer`,
but no longer depends on the agy interactive TUI or a huge pasted command line.

The runner also writes a per-run `MONOBENCH_CAPTURE_<runid>` marker before the
solver command and captures by that marker first, falling back to `ROOTCAUSE`
only if marker capture fails. This prevents old terminal history containing the
word `ROOTCAUSE` from being mistaken for the current answer.

Verification:

```text
cargo fmt --check  PASS
cargo test         PASS, 56 tests
cargo build        PASS
```

Next check: rerun the same via-niia agy canary. If it still fails, treat the new
trace as either a real model/monogram pattern or a separate niia session capture
issue, but not as the old permission-gate failure.
