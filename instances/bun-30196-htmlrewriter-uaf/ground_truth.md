# Ground truth — ⚠️ SPOILER (never fed to the agent)

**Repo:** oven-sh/bun · **PR #30196** "Fix HTMLRewriter use-after-free when handler rejects during end()"
(merged 2026-05-03). **Fix commit:** `0561f87d42ce` · **pre-fix base:** `d0a0bc4c9a5d`.
**Root cause:** `src/bun.js/api/html_rewriter.zig :: BufferOutputSink.runOutputSink`.

## Mechanism (double-free across the Zig ↔ JSC-wrapper ownership boundary)
For a buffered `transform()` (string / ArrayBuffer) the body is fed to lol-html via `write()` then
`end()`. The output `Response` is created in `init()` via `sink.response.toJS()` — which hands
**ownership to the JS wrapper cell** (`JSResponse`, `m_ctx`). The buggy `end()` catch branch did:

```zig
sink.rewriter.?.end() catch {
    if (!is_async) response.finalize();   // <-- destroys a Response the JS wrapper already owns
    sink.response = undefined;
    ...
};
```

When a handler returns a **rejected promise on the final `lastInTextNode` chunk** (emitted from
`end()`), this branch runs and finalizes the Response in place. The JS wrapper's `m_ctx` now dangles.
On a later GC sweep the wrapper's destructor runs `JSResponse::~JSResponse` → `Response.finalize` →
`JSRef.deinit` on the freed pointer = **use-after-poison / double-free**. The crash is at GC time,
arbitrarily far from the HTMLRewriter call.

**Fix:** drop the manual `response.finalize()` + `sink.response = undefined` in the `end()` catch and
let the JS wrapper own the lifetime — exactly what the sibling `write()` error path already did.

## Decoys (both live at base_commit)
1. **The crash site** — `Response.finalize` / `ResponseClass__finalize` / `JSResponse::~JSResponse`.
   The ASAN trace screams these; naming any of them as the root cause is **DECOY** (they correctly
   destroy a Response they believe they own).
2. **The sibling `write()` error path** (just above `end()` in the same sink): it returns the error
   and lets the JS wrapper keep ownership — i.e. it is the CORRECT template. An agent that "fixes"
   write(), or concludes both paths are fine, has missed it.

## Why it meets C1–C6 (SPEC.md) — and why it's HARDER than ksmbd-37899
- **C1** symptom (Response finalizer at GC sweep) ≠ cause (`runOutputSink` end() branch) — different
  file, different time (GC vs transform), and a Zig↔C++ boundary.
- **C2** no text bridge: the crash trace names Response/JSResponse/JSRef, never `runOutputSink` or the
  output sink; `finalize`/`Response` grep to the crash site + many call sites, not to the cause. The
  discriminator is *ownership* (who created the JS wrapper that also owns this Response) — a
  who-owns / who-frees question = monogram `chain --callers` / symbol-reference territory.
- **C3** objective truth: merged single-concern fix `0561f87d42ce` (html_rewriter.zig +1/-3).
- **C4** contamination: LOW. An obscure 2026 fuzzing fix in a 1M-line repo, not a famous CVE — the key
  contrast with ksmbd-37899 (CVE-2025-37899), where baseline-haiku scored FULL 1/1 because the bug is
  in training data as prose. This instance should actually defeat a tool-less baseline.
- **C5** discriminating (to verify): the crash trace actively misleads toward the finalize/destructor
  decoy; the correct answer requires reasoning that toJS() in init() already transferred ownership.
- **C6** tool capability on the path: localizing requires tracing Response ownership across the sink's
  init/write/end paths and the JS wrapper — call-graph / ownership tracing, monogram's claim. codegraph
  indexes Zig poorly (forfeit risk) → likely a monogram-vs-baseline contest with codegraph absent.

## Provenance
PR body (Jarred-style writeup) gives the full ASAN poison-history trace and a minimal repro:
`new HTMLRewriter().onDocument({text(c){ if (c.lastInTextNode) return Promise.reject(Error("boom")); }})`
then `rewriter.transform(new Uint8Array([97,98,99]).buffer)` then `Bun.gc(true)`. Regression tests added
in test/js/workerd/html-rewriter-end-error.test.ts.
