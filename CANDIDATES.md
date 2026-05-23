# Candidate instances (backlog)

Add one at a time: copy `instances/_TEMPLATE/` → fill in → run the admission gate (baseline n≥3) →
keep if discriminating. Each must meet the C1–C6 criteria in SPEC.md. Aim for a spread of
language pairs and bug categories.

## Seeded (researched this session)

| candidate | repo @ ver | langs | category | why it fits / risk |
|-----------|-----------|-------|----------|--------------------|
| ✅ `bun-1.3.10-toThreadSafe` | bun v1.3.10 | zig↔c++ | refcount UAF | **shipped seed.** symptom(string.zig) ≠ cause(BunString.cpp), decoy, fix #30049 |
| `bun-1.3.10-napi-vtable` | bun ~v1.3.10 | c++↔js | N-API vtable corruption (#27471) | cross-boundary (N-API); risk: fix may be large/multi-file → grading harder |
| `bun-gzip-chunked-empty` | bun (pre-#22360) | zig | HTTP decompression logic | focused, verifiable (#22360); risk: single-language → weaker C2 (text bridge) |
| `bun-fetch-brotli-shortread` | bun (post-1.0.21, pre-fix) | zig | streaming decompression | focused "ShortRead" bug; verify the fix commit exists |
| `tauri-ipc-mismatch` | a public Tauri app OR monolex app | ts↔rust | cross-language IPC orphan | **purest cross-boundary** (grep can't bridge invoke("x") ↔ Rust fn if names differ) = monogram's home turf; codegraph indexes Rust so no forfeit |
| `zig-only-deep-uaf` | a Zig project @ buggy commit | zig | UAF / logic | tests monogram on Zig where **codegraph forfeits** (OOM) — isolates robustness value |

## Category coverage to aim for
- **cross-language** (zig↔c++, ts↔rust, c++↔js) — the core of monobench; tests cross-FFI tracing.
- **single-language deep** (rust UAF, large py/go) — tests impact/ownership tracing without an FFI hop.
- **navigation-only** (e.g. SWE-bench Verified localization) — a SEPARATE, lower-difficulty category;
  note these are mostly grep-solvable, so most will FAIL the admission gate (keep only the hard ones).

## Notes
- Prefer bugs with a **decoy** (an adjacent plausible-but-wrong function) — they punish shallow guessing.
- Prefer **recent/obscure** bugs to limit training-data contamination (record the risk per instance).
- Each new tool (codegraph, future tools) = a `harness/tools/<tool>/tool.json` adapter; record FORFEIT if it
  can't index the repo (this is itself a result, e.g. codegraph on any Zig-heavy instance).

## AI-found / fuzzer-found cross-boundary backlog (2026-05-22 research)

**Selection principle learned this session:** monobench admits ONLY structural cross-boundary bugs
(symptom≠cause, no text bridge) that DEFEAT a hard-trying baseline. Two corollaries:
- **Famous CVEs are contaminated.** `ksmbd-37899` (CVE-2025-37899, o3-found) is real + cross-connection
  structural, but **baseline-haiku solved it FULL 1/1** ($0.31, no tool) — the famous CVE is in the
  model's training data as prose → non-discriminating, down-weighted. Prefer **obscure fuzzer/ASAN/race
  finds** over headline CVEs.
- **The "AI cracked a famous bug" genre is rare** with clean public fixes (ghostty #8208 / ksmbd / Mythos's
  3 public ones). The scalable seam = **famous-repo merged UAF/race fix PRs** — `gh search` on bun alone
  yields dozens of cross-boundary, low-contamination, single-concern fixes.

### bun shortlist (famous repo · Zig↔C++↔JS · recent 2026 · low contamination)
| candidate | PR | boundary | category | note |
|-----------|----|---------|----------|------|
| ✅ `bun-30196-htmlrewriter-uaf` | #30196 | zig↔c++/jsc | double-free (ownership) | **BUILT.** crash@GC Response.finalize ≠ cause runOutputSink end() path; decoy = crash-site finalize + the CORRECT sibling write() path |
| `bun-29829-onhandshake-uaf` | #29829 | zig (deep chain) | UAF (callback frees client) | checkServerIdentity→closeAndFail→…→deinit frees client; onHandshake uses it after the call returns. ⚠ multi-concern PR → grade harder |
| `bun-30185-heapsnapshot-race` | #30185 | c++↔worker | cross-thread race | HandleSet race in getHeapSnapshot — ksmbd-like concurrency, but obscure |
| `bun-27838-sslconfig-race` | #27838 | zig/uSockets | race + deref | SSLConfig intern/deref race → segfault in proxy tunnel setup |
| `bun-20093-napi-handlescope-race` | #20093 | c++↔js | N-API race | NapiHandleScopeImpl race condition |
| `bun-29951-directorywatch-uaf` | #29951 | zig | UAF (lifetime) | DirectoryWatchStore when a client component boundary is demoted |
| `bun-28907-threadpool-wakeup` | #28907 | zig | lost-wakeup race | ThreadPool.notify() on aarch64 — pure concurrency |

Source: `gh api -X GET search/issues -f q='repo:oven-sh/bun is:pr is:merged use-after-free in:title'`
(and `…race in:title`). Author recipe: pin `base_commit` = the merge commit's FIRST parent; read the PR
body for the root-cause fn + the correct sibling path (= the decoy); de-keyword the symptom so the crash
trace doesn't grep to the cause.

### Live validation (haiku · baseline vs monogram) — calibration so far
| instance | baseline-haiku | monogram-haiku | verdict |
|----------|---------------|----------------|---------|
| ksmbd-37899 | FULL | FULL | ❌ too easy — famous CVE is in training data as prose |
| bun-30196-htmlrewriter-uaf | **FULL** (6 calls, $0.17) | NO_RESULT (full arm, incomplete) | ❌ too easy — symptom named "HTMLRewriter" → grep bridges to the file; heavy arm made haiku wander |
| bun-30185-getheapsnapshot-race | MISS (68c, $0.77) | MISS (thin, 42c, $0.56) | ✅ properly HARD — baseline can't grep in; but **haiku too weak for BOTH** → no correctness signal (monogram −38% calls = efficiency only). Discrimination likely needs sonnet. |

**Difficulty band finding (2026-05-22):** haiku has a narrow usable band — *easy* instances (named API / famous CVE) → baseline FULL (no signal); *hard* instances (hidden API + crash≠cause) → BOTH arms MISS (no correctness signal, only monogram efficiency). The baseline-MISS / tool-FULL discrimination likely lives at sonnet/opus for these cross-boundary bugs, OR at a *medium* difficulty tier for haiku.

**Generator note:** `harness/gen-instances.sh` + `instances/backlog.tsv` scaffold instances from merged fix-PRs (mechanical fields prefilled; judgment fields = TODO; full PR body dumped to ground_truth.md). 13 scaffolded 2026-05-22. ⚠ root_cause_file heuristic = first non-test changed file → WRONG for cpython (picks the Misc/NEWS.d entry) and some bun (picks boringssl.zig not HTTPContext.zig); fix root_cause_file + author symptom/grading from ground_truth.md before running.

**Authoring rules learned (a discriminating instance needs ALL THREE):**
1. **crash-fn ≠ cause-fn** — reject bugs where the crash is IN the buggy function (e.g. #29829 onHandshake
   crashes *in* onHandshake → weak C1). Want: crash deep in shared infra (GC marking, finalizer, allocator),
   cause in a feature binding far away.
2. **Symptom must NOT name the buggy subsystem/API** — writing "HTMLRewriter" / "getHeapSnapshot" lets a
   plain grep bridge straight to the cause file (this is why bun-30196's baseline got FULL in 6 calls).
   Give only the crash trace + observable behavior; make the agent infer the subsystem.
3. **Lean arm for haiku** — the full `monogram` arm's injected initiate.md makes haiku wander and DNF on
   1M-line repos; use `monogram-thin` / `monogram-discover`. Budget cap = $6 (MONOBENCH_CAP), so a DNF is
   incompleteness, not cost.

**Implication for the shortlist:** before authoring, confirm the PR's crash site is in different code than
the fix (rule 1). #30185 qualifies (crash in JSC HandleSet, fix in JSWorker getHeapSnapshot). Re-check
#29829 (likely crash==cause → drop or reframe).

## Multi-repo expansion (2026-05-22) — beyond bun, for diversity
`gh search` across famous runtimes surfaces the SAME crash-fn≠cause-fn shapes (crash in GC / parser /
mutex, cause in a feature binding far away). Strong recipe-fits flagged ⭐:

| candidate | repo | PR | boundary | shape | why it fits |
|-----------|------|----|----------|-------|-------------|
| ⭐ node-56840-statementsync-gc | nodejs/node | #56840 | c++↔js↔V8-GC | premature-GC UAF in sqlite StatementSync | crash in GC ≠ cause in the binding |
| ⭐ cpython-142831-json-reentrant | python/cpython | gh-142831 | c↔python | re-entrant-mutation UAF in the json encoder | crash in encoder/GC ≠ cause (re-entrancy via __repr__) |
| ⭐ node-62095-freeparser-llhttp | nodejs/node | #62095 | c++↔js | UAF: freeParser during llhttp_execute (re-entrancy) | crash in parser ≠ cause |
| cpython-146613-grouper-reentrant | python/cpython | gh-146613 | c↔python | re-entrant UAF in itertools._grouper | re-entrancy |
| cpython-148820-rawmutex-wakeup | python/cpython | gh-148820 | c (threads) | _PyRawMutex UAF on spurious semaphore wakeup | concurrency / lifetime |
| node-62325-zlib-reset-write | nodejs/node | #62325 | c++↔js | UAF: reset() during write (zlib) | conditional error path |
| node-59910-diagchannel-gc | nodejs/node | #59910 | c++↔js↔GC | race between diagnostics_channel and GC | crash in GC ≠ cause |
| deno-31770-statementsync-iter | denoland/deno | #31770 | rust↔js | UAF in StatementSync JS iterator | cross-runtime twin of node-56840 |

Diversity now spans **4 repos × {C, C++, Rust, Zig} × {Python, JS}** and categories: premature-GC UAF,
re-entrancy UAF, cross-thread handle race, mutex-wakeup UAF, double-free/ownership. Author next from the
⭐ rows (verify crash-fn≠cause-fn from the PR body, hide the API in the symptom, run with the thin arm).
