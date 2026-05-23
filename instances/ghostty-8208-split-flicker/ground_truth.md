# Ground truth — ⚠️ SPOILER (never fed to the agent)

**Issue:** ghostty-org/ghostty #8208 — *"gtk-ng: fix split change (new, delete, resize, etc.) flicker"*
(closed 2026-03-04). **Fix commit:** `264dbf9e` (base/buggy parent: `b7913f09a`).

**Root cause:** `src/apprt/gtk/class/split_tree.zig` — the split-tree rebuild used a **clear-then-rebuild**
flow: `tree_bin` was cleared immediately, then the code waited *asynchronously* for the old surfaces
to become unparented before rebuilding the new widget tree. During that intermediate window the bin
had no child → an observable **blank frame** → the flicker on every split new/delete/resize.

**Fix (#8208 / `264dbf9e`):**
1. **Debounced rebuild, retained content** — `propTree` schedules a single idle callback while keeping
   the existing tree visible (no immediate clear).
2. **Atomic widget swap** — the rebuild happens in one pass and the bin child is swapped atomically to
   the new root widget tree.
3. **Leaf widget reuse** — reuse an existing `SurfaceScrolledWindow` ancestor per surface (detach +
   reparent) instead of recreating it on every change.
4. Empty-tree fast path for shutdown cleanup.

**Why it's a (maximally) fair instance**
- Open ~6 months; the author (Mitchell Hashimoto) + Opus 4.6 + lower Codex levels all FAILED — only
  Codex 5.3 (xhigh) solved it. The admission gate is guaranteed to pass.
- Symptom (a one-frame visual flicker) has **no error string to grep** — the link to the root cause
  (async rebuild ordering in split_tree.zig) is purely structural/behavioral.
- Zig + GTK → **codegraph forfeits** (OOMs on Zig). Pure monogram-vs-baseline.
- Category = UI async-ordering / render timing → diversifies the set vs the bun refcount UAF.

Trace: https://ampcode.com/threads/T-019cbadf-cb5a-742e-b0e3-2d7164de743f
Tweet: https://x.com/mitchellh/status/2029348087538565612
