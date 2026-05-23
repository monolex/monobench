# Ground truth — SPOILER (the real fix PR — author the instance FROM this, never feed to the solver)

**python/cpython PR #147962** — gh-146613: Fix re-entrant use-after-free in itertools._grouper

**fix commit:** fc7a188fe70a · **base (merge^):** c1a4112c225e · merged 2026-04-02

## Changed source files (test/fixture files filtered out)
- Misc/NEWS.d/next/Library/2026-04-01-11-05-36.gh-issue-146613.GzjUFK.rst
- Modules/itertoolsmodule.c

## PR body

Closes gh-146613

The same pattern was fixed in `groupby.__next__` (gh-143543 / a91b5c3), but `_grouper_next` (the inner group iterator returned by `groupby`) was missed.

A user-defined `__eq__` can re-enter the grouper during `PyObject_RichCompareBool`, causing `Py_XSETREF` to free `currkey` while it is still being used.

Fixed by taking strong references (`Py_INCREF` / `Py_DECREF`) to `tgtkey` and `currkey` before the comparison, exactly as done in `groupby_next`.

Added regression test `test_grouper_reentrant_eq_does_not_crash`.
