# Ground truth — ⚠️ SPOILER (never fed to the agent)

**Repo:** oven-sh/bun · **PR #30185** "worker: fix cross-thread HandleSet race in getHeapSnapshot"
(merged 2026-05-03). **Fix commit:** `0150c57051f2` · **pre-fix base:** `0561f87d42ce` (= merge of #30196).
**Root cause:** `src/bun.js/bindings/webcore/JSWorker.cpp :: jsWorkerPrototypeFunction_getHeapSnapshotBody`.

## Mechanism (JSC handle thread-affinity violation across the worker boundary)
`getHeapSnapshotBody` creates a promise handle in the **parent** VM and ships work to the worker:

```cpp
Strong<JSPromise> strong(vm, promise);                       // parent VM's HandleSet
worker.postTaskToWorkerGlobalScope([strong, parentId](auto& workerCtx) {   // runs on WORKER thread
    ...
    ScriptExecutionContext::postTaskTo(parentId, [strong, snapshot=...](auto& parentCtx){ ... });
});
```

`JSC::Strong<T>` has **no move constructor**, so capturing it *by value* copy-constructs it
(`HandleSet::allocate()` + `m_strongList.push()`) and later destroys it (`deallocate()` +
`NodeList::remove()`) — **on the worker thread**, against the **parent VM's** `HandleSet`, **without
the parent VM's lock**. `HandleSet::m_strongList` is a non-thread-safe `SentinelLinkedList`; push/remove
transiently null `m_next`/`m_prev`. The parent VM's "Sh" (Strong Handles) GC marking constraint walks
that list concurrently and follows a nulled link → `*(HandleNode*)nullptr->slot()` = `*(0x10)`.
`heapHelperPool()` is process-global, so the crashing helper belongs to the parent VM's collector even
though a worker-VM heap-snapshot full GC is running at the same time.

**Fix:** heap-allocate the `Strong<JSPromise>` once on the parent thread; pass only the **raw pointer**
through the cross-thread lambdas (the worker never touches the parent VM's HandleSet); the parent-side
completion lambda resolves the promise and frees the handle.

## Decoys (all live / plausible at base_commit)
1. **The JSC crash site** — `HandleSet::visitStrongHandles`, `SentinelLinkedList` push/remove,
   `MarkingConstraint::execute`. The trace screams these; they are **correct** (the invariant they rely
   on was broken by the caller). Naming any as root cause = DECOY.
2. **The recent worker-lifetime rewrites (#29957 / #29937)** — the natural "what changed recently?"
   suspect. The PR explicitly states they did NOT introduce this (it's been there since getHeapSnapshot
   was added). An agent that blames the recent rewrite is wrong.
3. **`BunV8HeapSnapshotBuilder`** — the worker VM's own snapshot GC, concurrent but not the cause.

## Why it meets C1–C6 (SPEC.md) — and why it's HARDER than #30196
- **C1** symptom (JSC GC marking the parent VM's HandleSet, on a GC helper thread) ≠ cause
  (`getHeapSnapshotBody` capturing a Strong by value), different file, different thread.
- **C2** no text bridge: the crash trace is pure JSC GC internals; the symptom never mentions
  "heap snapshot", so "snapshot"/"getHeapSnapshot" do not grep from the symptom to the cause. The
  discriminator is JSC's Strong<>/HandleSet thread-affinity semantics — a who-creates-this-handle /
  who-runs-on-which-thread question.
- **C3** objective truth: merged fix `0150c57051f2`.
- **C4** contamination: LOW (obscure 2026 CI flake in a 1M-line repo).
- **C5** discriminating (to verify): three live decoys + a crash that looks like a JSC/GC bug with no
  origin. A tool-less baseline is expected to blame JSC or the recent worker rewrite.
- **C6** tool capability on the path: requires finding the worker binding that mints a parent-VM
  Strong and crosses it to a worker thread — symbol search + call/usage tracing of Strong<> /
  postTaskToWorkerGlobalScope. codegraph indexes C++ (no forfeit) → a 3-way baseline/monogram/codegraph
  contest is possible.

## Provenance
Decoded + symbolized from a bun.report trace (buildkite build 50529). Fix also adds raw-pointer
plumbing in Worker.cpp/.h; the single root-cause function is the getHeapSnapshot binding in JSWorker.cpp.
