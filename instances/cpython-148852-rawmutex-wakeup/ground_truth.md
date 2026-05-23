# Ground truth — SPOILER (the real fix PR — author the instance FROM this, never feed to the solver)

**python/cpython PR #148852** — gh-148820: Fix _PyRawMutex use-after-free on spurious semaphore wakeup

**fix commit:** ad3c5b7958b8 · **base (merge^):** 59b41c8c3ba3 · merged 2026-04-22

## Changed source files (test/fixture files filtered out)
- Misc/NEWS.d/next/Core_and_Builtins/2026-04-21-14-36-44.gh-issue-148820.XhOGhA.rst
- Python/lock.c
- Python/parking_lot.c

## PR body

_PyRawMutex_UnlockSlow CAS-removes the waiter from the list and then calls _PySemaphore_Wakeup, with no handshake. If _PySemaphore_Wait returns Py_PARK_INTR, the waiter can destroy its stack-allocated semaphore before the unlocker's Wakeup runs, causing a fatal error from ReleaseSemaphore / sem_post.

Loop in _PyRawMutex_LockSlow until _PySemaphore_Wait returns Py_PARK_OK, which is only signalled when a matching Wakeup has been observed.

Also include GetLastError() and the handle in the Windows fatal messages in _PySemaphore_Init, _PySemaphore_Wait, and _PySemaphore_Wakeup to make similar races easier to diagnose in the future.



<!-- gh-issue-number: gh-148820 -->
* Issue: gh-148820
<!-- /gh-issue-number -->
