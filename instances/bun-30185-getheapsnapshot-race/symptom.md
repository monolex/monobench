You are working in the Bun codebase — a large JavaScript runtime (Zig core, C++/JSC bindings under
src/bun.js/bindings/, generated glue under codegen/). The bug involves `node:worker_threads`.

Symptom: `test/js/node/worker_threads/worker_threads.test.ts` occasionally segfaults in CI — a flaky,
timing-dependent crash:

    panic: Segmentation fault at address 0x10

The faulting thread is a GC helper thread of the MAIN (parent) VM, crashing while it marks strong
handles during garbage collection:

    wtfThreadEntryPoint → AutomaticThread::start → ParallelHelperPool::Thread::work
      → Heap::runBeginPhase(GCConductor) → SlotVisitor::drainFromShared
        → MarkingConstraintSolver::runExecutionThread
          → MarkingConstraint::execute            ("Sh" — Strong Handles)
            → HandleSet::visitStrongHandles
              → read of *(HandleNode*)nullptr -> m_value   = *(0x10)   ← crash

So the parent VM's GC is walking its strong-handle list (a SentinelLinkedList) and follows a nulled
link — the list was concurrently mutated. It reproduces only when worker threads are active and the
parent VM happens to collect at the wrong moment.

The crash site (JSC's GC marking of the parent VM's HandleSet) is NOT the bug — that code is correct,
and its invariant is explicit: a VM's handle list is only mutated under that VM's lock, on that VM's
own thread. Something violated that. Find the ROOT CAUSE: the exact function that causes the parent
VM's strong-handle list to be mutated from a WORKER thread (without the parent VM's lock), corrupting
it under concurrent GC. Explain the mechanism and the fix.

End your reply with exactly:
ROOTCAUSE: <file path>::<function>
FIX: <one sentence>
