---
title: "Monogram Command Output Audit"
created: 2026-05-24
phase: complete
project: monobench
---

# Monogram Command Output Audit

## Research Questions

1. Do solver logs show command-format mistakes around monogram CLI usage?
2. Do monogram outputs or `[NEXT]` hints create recurring failure patterns?
3. Is a structured parser needed beyond `monobench adoption`?

## Conclusions

Structured parsing is needed. `adoption` counts calls and rough failures, but it does not classify
why a monogram call failed or whether `[NEXT]` is misleading. The new `monobench monogram-audit`
command parses solver telemetry and classifies `help_exit_nonzero`, `bad_workdir_path`,
`sqlite_locked`, `no_bindings_indexed`, `no_symbol`, `no_results`, and oversized outputs.

The observed issues are not primarily `$` prompt markers in final answers. They are execution-log
and tool-output interpretation issues:

- `monogram` with no args prints useful help but exits nonzero, so the mandated first action looks
  like a command failure in Codex stderr.
- Model-generated commands sometimes reconstruct `/tmp/monobench-work/wt/...` paths incorrectly,
  producing `bad_workdir_path` before monogram can run.
- `coupling --domain ffi` can say `No bindings indexed. Run \`monogram index\` first` even when the
  repo was freshly indexed, which is a misleading NEXT/remediation signal.
- Some symbol-level commands fail because the model queries an invented/nearby symbol name instead
  of backing off to `search`/`grep`/`context`.
- Large `context`/`grep --chain` outputs are common in failed runs and need measurement because they
  can drown the solver.

## Evidence

### Parser Added

| File | Lines | Finding |
|------|-------|---------|
| `src/monogram_audit.rs` | 22-30 | Extracts the actual unquoted `monogram` command token and subcommand. |
| `src/monogram_audit.rs` | 32-60 | Classifies issue kinds from command result text. |
| `src/monogram_audit.rs` | 82-188 | Aggregates per-run calls, issues, oversized outputs, help calls, NEXT line count, subcommands, examples. |
| `src/util.rs` | 250-296 | Quote-aware command word detection prevents regex/path text from being treated as command tokens. |
| `src/adoption.rs` | 42-52 | Subcommand extraction now uses the quote/path-aware token position instead of raw substring search. |

### Observed Patterns

| Source | Line | Evidence |
|--------|------|----------|
| `/Users/macbook/.monobench/0.1.2-1779431036/results/bun-1.3.10-toThreadSafe/monogram-preindexed-gpt-5.4-mini-low-r1.err` | 1056 | `Error: SQLite failure: database is locked`. |
| `/Users/macbook/.monobench/0.1.2-1779431036/results/bun-1.3.10-toThreadSafe/monogram-preindexed-gpt-5.4-mini-low-r1.err` | 1360 | Second SQLite lock in same run. |
| `/Users/macbook/.monobench/0.1.2-1779431036/results/ksmbd-37899/monogram-low-r1.err` | 1100 | `No results found` for a broad mixed query. |
| `/Users/macbook/.monobench/0.1.2-1779431036/results/ksmbd-37899/monogram-low-r1.err` | 3605 | `No bindings indexed. Run monogram index first` after the run already had a fresh index. |
| `/Users/macbook/.monobench/0.1.6-1779528810/results/bun-1.3.10-toThreadSafe/monogram-gpt-5.3-codex-spark-high-r2.err` | 11417 | Bad reconstructed worktree path. |
| `/Users/macbook/.monobench/0.1.6-1779528810/results/bun-1.3.10-toThreadSafe/monogram-gpt-5.3-codex-spark-high-r2.err` | 31036 | `No symbol matches "SliceWithUnderlyingString"`. |
| `/Users/macbook/.monobench/0.1.6-1779528810/results/bun-1.3.10-toThreadSafe/monogram-gpt-5.3-codex-spark-high-r2.err` | 198012 | `No bindings indexed. Run monogram index first`. |

## Audit Runs

Command:

```bash
MONOBENCH_ROOT=/Users/macbook/.monobench/0.1.2-1779431036 monobench monogram-audit bun-1.3.10-toThreadSafe
```

Result:

```text
runs=3 calls=68 issues=5 oversized=1 help=3 next-lines=165
issues: help_exit_nonzero×3, no_results×1, sqlite_locked×1
```

Command:

```bash
MONOBENCH_ROOT=/Users/macbook/.monobench/0.1.2-1779431036 monobench monogram-audit ksmbd-37899
```

Result:

```text
runs=1 calls=45 issues=4 oversized=0 help=1 next-lines=93
issues: no_results×2, help_exit_nonzero×1, no_bindings_indexed×1
```

Command:

```bash
MONOBENCH_ROOT=/Users/macbook/.monobench/0.1.6-1779528810 monobench monogram-audit bun-1.3.10-toThreadSafe
```

Result:

```text
runs=3 calls=679 issues=20 oversized=14 help=4 next-lines=1689
issues: no_symbol×8, bad_workdir_path×6, help_exit_nonzero×4, no_bindings_indexed×2
```

### Oversized Output Detail

The large-output pattern is concentrated, not universal. In the `0.1.6` spark run set,
`monogram-gpt-5.3-codex-spark-high-r2` was a `MISS` and produced 11 of the 14 oversized
outputs. `r1` was also `MISS` and produced 1 oversized output. `r3` was `FULL` and produced
2 oversized outputs.

| Run | Grade | Oversized Count | Largest Pattern |
|-----|-------|-----------------|-----------------|
| `monogram-gpt-5.3-codex-spark-high-r2` | MISS | 11 | `search --explain`, `chain --depth 4`, `--json`, large `context --code` |
| `monogram-gpt-5.3-codex-spark-high-r1` | MISS | 1 | huge `symbols` listing after near-symbol query |
| `monogram-gpt-5.3-codex-spark-high-r3` | FULL | 2 | bounded enough to recover |

Largest observed outputs:

| Run | Grade | Command Pattern | Size | Lines | Evidence |
|-----|-------|-----------------|------|-------|----------|
| `spark-high-r2` | MISS | `monogram search "PathLike" --explain --cwd -n 40` | 1,979,020 B | 46,229 | stderr line 96778 |
| `spark-high-r2` | MISS | `monogram chain "toThreadSafeSlice" --callers --depth 4` | 1,472,794 B | 34,227 | stderr line 153218 |
| `spark-high-r2` | MISS | `monogram chain "toJSWithOptions" --callees --depth 2 --json` | 515,044 B | 13,273 | audit output |
| `spark-high-r2` | MISS | `monogram context toJSWithOptions --code 220` | 451,902 B | 10,623 | stderr line 31053 |
| `spark-high-r2` | MISS | `monogram context toThreadSafeEnsureRef --code 220` | 445,660 B | 10,519 | stderr line 29930 |
| `spark-high-r1` | MISS | `monogram symbols "SliceWithUnderlyingString" --cwd src/string.zig` | 147,386 B | 3,022 | stderr line 2500 |
| `spark-high-r3` | FULL | `monogram chain BunString__toThreadSafe --callers --depth 3` | 57,286 B | 1,444 | audit output |
| `gpt-5.5-low-r2` on `bun-27838` | INVALID | `monogram symbols "intern" --json` | 58,558 B | 503 | result line 2457 |

Detailed failure sequence observed in `spark-high-r2`:

1. The model moves from the original string ownership issue toward broad `PathLike`/async path
   transfer exploration.
2. `search "PathLike" --explain -n 40` produces a 1.9MB result and introduces many adjacent
   symbols unrelated to the exact root cause.
3. `chain toThreadSafeSlice --callers --depth 4` produces a 1.47MB caller graph and spreads the
   search across broad `PathLike` consumers.
4. The model then repeats broad JSON and context calls (`toJSWithOptions`, `fromJSMaybeAsync`,
   `toThreadSafeEnsureRef`, `PathLike`) instead of narrowing to the specific failing ownership
   contract.

This supports a stronger conclusion than "context is large": the harmful pattern is **high-fanout
symbol + high depth/explain/json + no compact budget**, especially on generic symbols such as
`PathLike`, `String`, and `toThreadSafeSlice`.

### Explain, JSON, and NEXT Semantics

`--explain` is currently a match-reason feature, not a summarizer or output guard. In `search`, it
prints the identifiers that drove the file score (`why: ...`) and caps that identifier list at six
per result. This is useful after the query is already narrow. It is harmful when attached to broad
generic terms such as `PathLike` because it adds adjacent names that the solver can chase instead of
narrowing.

There are clear pre-output budget points where monogram can predict that an answer will be too large:

| Command | Pre-output budget signal | Better staged NEXT |
|---------|--------------------------|--------------------|
| `search` | `limit`, `--cwd` internal `limit * 5`, result count, matched identifier count | Show top files plus why-summary; suggest `symbols`, `context <one symbol>`, or file-scoped rerun. |
| `chain` | `max_depth`, node count, depth histogram, fanout, filters | If depth is high or nodes exceed budget, show histogram/top fanout nodes first; suggest `--depth 1`, `--file`, `--lang`, or `--through`. |
| `context` | `entry_points * code_lines`, snippet byte count, calls/called_by counts | Show entry table first; suggest one-symbol `context --code 80` or a filtered `chain`. |
| `--json` | serialized byte length before printing | Return compact JSON plus `next_hint`, or require an explicit full-output flag for large raw dumps. |

`--json` changes both shape and size for the same semantic command. Text output contains headers,
truncated display paths, and visible `[NEXT]` lines. JSON output carries structured arrays and can be
much larger; in the observed runs, `chain ... --json` produced 515KB, 365KB, and 285KB dumps.

`next=0` in `monobench monogram-audit` does not always mean "no guidance exists." The audit originally
counted textual `[NEXT]` lines only. Some JSON paths already carry structured `next_hint` fields
instead of printing `[NEXT]`: `search --json`, `chain --json`, and `context --json` have source-level
support for this. The audit now adds a separate `json-next` / `jsonNext` detector so text NEXT and
structured NEXT are measured separately.

Re-running the spark `toThreadSafe` audit after this change shows:

```text
runs=3 calls=679 issues=20 oversized=14 help=4 next-lines=1689 json-next=7
```

The largest `chain --json` dumps still show `next=0`, but now show `jsonNext=true`. That means the
guidance exists as JSON, but the raw solver transcript and previous audit did not surface it as a
visible `[NEXT]` line.

The monomento model is the right control shape: `search -> tree -> peek -> read`, with max-child and
section filters, prevents immediate full dumps. Monogram needs the same staged behavior around
`chain/context/search --explain`: first show the structure and budget warning, then tell the agent
which narrowed command to run.

### Additional Pattern Sweep

`monobench monogram-audit` now classifies risky command shapes in addition to failures and oversized
outputs. Re-running the `0.1.6` spark `toThreadSafe` set shows that the MISS run is not only failing
on isolated huge outputs; it repeatedly chooses high-volume forms:

```text
patterns
  context_code_ge_100      239
  chain_depth_ge_3         69
  chain_callers_depth_ge_3 51
  json_without_next_hint   36
  search_explain           32
  search_explain_high_limit 24
  generic_symbol_or_query  19
  query_pipe_marker        11
  shell_post_filter_pipeline 5
  oversized_context_bundle 3
  oversized_search_explain 2
  oversized_json_without_next_hint 1
```

Per-run split:

| Run | Grade | Pattern Signal |
|-----|-------|----------------|
| `spark-high-r2` | MISS | `context_code_ge_100×160`, `chain_depth_ge_3×49`, `json_without_next_hint×28`, `search_explain_high_limit×18`, `generic_symbol_or_query×16` |
| `spark-high-r1` | MISS | `context_code_ge_100×59`, `chain_depth_ge_3×15`, `chain_callers_depth_ge_3×10` |
| `spark-high-r3` | FULL | `context_code_ge_100×20`, `json_without_next_hint×8`, `search_explain×8` |

This adds a stronger prioritization:

1. `context --code >=100` needs a compact/preview stage or default cap for agents.
2. `chain --callers --depth >=3` needs fanout prediction and depth histogram before printing.
3. `symbols --json` needs `next_hint` support; it is the main JSON path still showing
   `json_without_next_hint`.
4. `search --explain -n >=20` should warn or switch to summary because it broadens rather than
   narrows on generic terms.
5. Pipe usage is two separate patterns:
   - `query_pipe_marker`: model uses `A\|B` or `A|B` as an OR marker inside monogram queries.
   - `shell_post_filter_pipeline`: model uses `| head`, `| jq`, or `| sed` after monogram output.

The pipe finding means the original suspicion was partially correct: `|` is observed, but the common
damage is not final-answer corruption. The common damage is that agents attempt unsupported OR-style
queries or locally post-filter huge output instead of getting a native compact mode from monogram.

### Region Applicability Sweep

The recently added `region` command targets the exact loop shape seen above. Source evidence:

- `lib-monogram/src/bin/initiate/initiate.md:63` documents `region, locate, discover` as "rank
  functional regions" from fuzzy, raw, refs, graph, and coupling evidence.
- `lib-monogram/src/bin/monogram.rs:530` makes `search` point to `region "<intent>"` before
  `symbols` or `grep`; `:531` makes `region` point to bounded `context <top-symbol> --code 80`.
- `lib-monogram/src/region.rs:120` builds a report by fusing `search`, raw code hits, `refgrep`,
  coupling, metrics, and chain evidence; `:313` returns report-level `next_hint`.
- `lib-monogram/src/region.rs:949` applies `region_size_penalty`, so very large regions are
  explicitly down-ranked instead of blindly expanded.
- `lib-monogram/src/mcp.rs:122` exposes `region` in the MCP schema, so Codex/agy runners should be
  able to use it once the deployed binary includes the command.

Concrete places where `region` should replace the failing command shapes:

| Observed Pattern | Count | Region Replacement | Expected Effect |
|------------------|-------|--------------------|-----------------|
| `search --explain -n >=20` | 24 | `monogram region "<intent>" -n 5 --score-debug` | Avoids broad file-result dumps such as `PathLike` 1.9MB output. |
| `context --code >=100` | 239 | `region` first, then only `context <top-symbol> --code 80` | Stops the 160-call MISS loop from expanding adjacent helpers before ranking. |
| `chain --callers --depth >=3` | 51 | `region` first with default chain evidence depth 2 | Uses shallow graph evidence for ranking before expensive fanout traversal. |
| `generic_symbol_or_query` | 19 | Natural-language `region` query | Better than exact `symbols "String"`/`symbols "PathLike"` on overloaded names. |
| `query_pipe_marker` | 11 | Natural-language `region` query without `A|B` syntax | Avoids unsupported OR-marker searches like `ZigString__fromUTF16\|...`. |
| JSON outputs with no visible `[NEXT]` | 36 | `region --json` where task is "find implementation area" | `RegionReport` carries `next_hint` and per-region `next` fields. |

The pre-update state is deployment/version mismatch, not a concept gap: current source and docs include
`region`, but the installed OpenCLIs binary at `/Users/macbook/.openclis/bin/monogram` still reports
`Unknown command: region` under `monogram 0.52.0`. That explains why the recorded solver logs never
used it. Before re-running the benchmark, rebuild/publish/install monogram with the source version
that contains `cmd_region`; otherwise prompts that mention `region` will fail and fall back into the
same broad `search/context/chain` loop.

### Haiku Cross-Session Sweep

Correction: Haiku replay telemetry is present under `results/*/*haiku*.jsonl`; the archived
`research/cases/*/raw` folders do not contain every Haiku run. Re-running `monobench report`,
`monobench adoption`, and `monobench trace` against those JSONL files separates five regimes:

| Instance | Baseline Haiku | Monogram Haiku | Interpretation |
|----------|----------------|----------------|----------------|
| `bun-1.3.10-toThreadSafe` | 2/5 FULL; median $0.25, 27 calls | 1/1 FULL; $0.20, 14 calls, 57% mono | Best current monogram-positive Haiku case. The successful run used `search "string corrupt value"` -> `grep ref/deref` -> `search "BunString C++"` -> `grep toThreadSafe`, then named `BunString__toThreadSafe`. |
| `bun-30185-getheapsnapshot-race` | 0/1 FULL; $0.77, 68 calls | `discover` = DECOY, `thin` = MISS | Hard hidden-subsystem case. Monogram reduced calls, but `discover` overfit the worker-thread cone and `thin` fell back to grep/edit around `fakeParentPort`. |
| `bun-30196-htmlrewriter-uaf` | 1/1 FULL; $0.17, 6 calls | 1/1 FULL; $0.42, 26 calls, 23% mono | Non-discriminating. The symptom names `HTMLRewriter`, so baseline finds the target faster; monogram adds cost. |
| `ksmbd-37899` | 1/1 FULL; $0.31, 37 calls | 1/1 FULL; $0.47, 47 calls, 34% mono | Non-discriminating. Both runs reach `smb2_session_logoff`; monogram is slower here. |
| `ghostty-8208-split-flicker` | 2/3 FULL baseline only | No monogram Haiku run | Useful for Haiku baseline calibration, not for tool comparison yet. |

Observed Haiku-specific monogram patterns:

1. The positive case is not high-volume output. `bun-1.3.10-toThreadSafe` monogram-haiku used only
   8 monogram calls, no oversized output, and a simple ownership vocabulary: `ref`, `deref`,
   `BunString`, `toThreadSafe`.
2. The hard failure/decoy case is wrong-region selection. `bun-30185` monogram-discover selected
   `Worker.cpp::createNodeWorkerThreadsBinding`; monogram-thin selected `worker_threads.ts::fakeParentPort`.
   Both stayed near worker setup because the queries were broad (`worker threads handle management`,
   `Worker`, `Strong`, `Handle`) and never forced a candidate comparison against the real heap snapshot
   ownership mechanism.
3. Escaped regex alternation is visible in Haiku runs but is not always fatal. The audit tool now
   separates quoted grep regexes such as `HandleSet\|visitStrongHandles` from real shell pipelines.
   The remaining risk is when weak models treat `A|B` as a logical search language for non-regex
   commands and then continue with shell post-filters when monogram does not rank the intent.
4. The `monogram` no-arg help exit status is noisy in telemetry (`help_exit_nonzero`) but did not
   block Haiku successes. It should still be normalized because it makes audit output look like an
   actual command failure.

This changes the interpretation of `region` for Haiku:

1. `region` is most relevant to `bun-30185`-style wrong-region failures, not to easy named-subsystem
   cases like `HTMLRewriter` or famous CVE-like cases like `ksmbd`.
2. It should appear as a short runtime `[NEXT]` after broad `search`/`grep`, not as a long prompt
   recipe. Haiku succeeds when the command path is short and concrete.
3. The test should measure whether `region "<intent>"` can pull `bun-30185` away from the worker setup
   decoy and toward heap-snapshot / handle lifetime regions before `context` and `grep` expand.

Recommended Haiku re-test shape:

| Goal | Arm | Prompt Shape |
|------|-----|--------------|
| Measure region as navigation help | `monogram-thin` or default minimal arm | `Run monogram; follow [NEXT].` Let broad results advertise `region`. |
| Avoid heavy-skill regression | Do not use full static monogram skill | No long ownership recipe unless the instance explicitly requires it. |
| Focus on discrimination | `bun-30185` and similar hidden-subsystem cases | Exclude `HTMLRewriter`, `ksmbd`, and other cases where baseline already finds the named subsystem. |
| Preserve analyzability | Save raw `.jsonl` plus report/adoption/trace snapshots | JSONL exists now; future archival should copy it into `research/cases/*/runs/*/raw/results`. |

## Hypothesis Status

| Hypothesis | Status | Evidence |
|------------|--------|----------|
| `$` prompt markers break final grading | Refuted | `answer.txt` scan found no `|` lines and no `$` command-marker lines in final answers. |
| `|` can corrupt telemetry interpretation | Verified, fixed | `cmd_has_word` previously treated `|` as a separator without quote awareness; fixed in `src/util.rs`. `monogram-audit` now separates shell pipelines from `regex_alternation_query`. |
| A parser is needed for monogram output quality | Verified | `monogram-audit` reveals issue categories that `adoption` cannot show. |
| Some `[NEXT]` hints need improvement | Verified | `No bindings indexed. Run monogram index first` appears after fresh indexing, so the remediation is misleading. |
| `region` can reduce wrong-region and high-volume search/context/chain loops | Verified as test target | Existing logs show wrong-region selection (`bun-30185`) and high-volume loops (`context`, `grep --chain`); region should be evaluated as a benchmark variable, not assumed solved. |

## Analysis Tooling Fixes Applied

| Tool | Fix | Evidence |
|------|-----|----------|
| `monobench monogram-audit` | Treat `monogram 2>&1 | head` as a help probe instead of fake subcommand `2>&1`. | `bun-30185` audit now reports `help=3` and no `2>&1` subcommand. |
| `monobench monogram-audit` | Split actual shell pipelines from quoted grep regex alternation. | `query_pipe_marker` no longer appears for `HandleSet\|visitStrongHandles`; it is reported as `regex_alternation_query`. |
| `monobench monogram-audit` | Classify nonzero exit status from the first status line only. | `bun-1.3.10-toThreadSafe` issues dropped from 7 to 6; false `nonzero_other` on an interleaved Codex stderr block disappeared. |
| `monobench trace` | Widen command/root-cause rendering with middle truncation. | `trace bun-30185-getheapsnapshot-race monogram-thin-haiku-r1` now shows the full wrong ROOTCAUSE function and mechanism. |

## Recommended Monogram Improvements

1. Make `monogram` no-arg help exit `0`, or provide an explicit `monogram help` first-action instruction.
2. Change the `coupling --domain ffi` empty-state message to distinguish `index missing` from `no FFI bindings extracted for this repo/language`.
3. Add fallback NEXT after `No symbol matches`: suggest `monogram search <term> --explain` and `monogram grep <term> --chain`, not just another exact symbol query.
4. Add output-size controls to common high-volume commands: `context --max-callers`, `grep --max-callers`, or a compact mode for agents.
5. Gate any `region` benchmark on the runner-visible monogram help/schema showing the command; otherwise the run measures prompt/tool mismatch, not tool quality.
6. Keep `monobench monogram-audit` in the loop after every monogram experiment batch.
