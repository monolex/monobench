# Ground truth — ⚠️ SPOILER (never fed to the agent)

**Root cause:** `src/rdb.c :: rdbLoadObject` (PR #15095, fix `fab099cdcffb`, base `0d9576435f83`).

When a loaded stream's consumer PEL lists the same pending ID twice, the duplicate-entry error
branch called `streamFreeNACK()` on a NACK **still referenced from the group's global PEL**
(`cgroup->pel`). Object teardown later freed that NACK again → double free / abort.

**Decoy:** `streamFreeNACK` performs the free and looks like the bug, but it is correct; the second
free crashes during `freeStreamObject` teardown.

**Fix:** on the `raxTryInsert(consumer->pel, ...)` failure branch, do **not** call `streamFreeNACK`;
keep reporting corruption and rely on `decrRefCount(o)` for cleanup (the NACK is owned by
`cgroup->pel`).

**Admission (C1–C6):** C1 ✓ crash (teardown) ≠ cause (load error branch). C2 ✓ symptom states the
input (duplicate ID), not the function. C3 ✓ PR #15095. C4 — crafted/corrupt RDB → low contamination.
C5 — run baseline. C6 ✓ C ownership graph (who owns the NACK).
