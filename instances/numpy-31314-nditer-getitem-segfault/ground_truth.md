# Ground truth — ⚠️ SPOILER (never fed to the agent)

**Root cause:** `numpy/_core/src/multiarray/nditer_pywrap.c :: npyiter_multi_index_set` (PR #31314,
fix `fa6d67432de1`, base `a9c745972cd7`).

The setter called `PySequence_GetItem` on the supplied index object and used the result **without a
NULL check**. If the object's `__getitem__` raises, `PySequence_GetItem` returns `NULL` and the C
code dereferenced it → segfault.

**Decoy:** the user's Python `__getitem__` merely raises (not the bug); the iterator-advance path is
a plausible-but-wrong suspect.

**Fix:** add a `NULL` check after `PySequence_GetItem` and propagate the error.

**Admission (C1–C6):** C1 ✓ crash (C nditer setter) ≠ cause (a Python callback raising mid-call).
C2 ✓ symptom never names the function/`PySequence_GetItem`. C3 ✓ PR #31314. C4 — recent, single file
→ watch contamination (C5). C6 ✓ Python re-entrancy ↔ C error handling.
