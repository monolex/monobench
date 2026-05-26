# Ground truth — ⚠️ SPOILER (never fed to the agent)

**Root cause:** `…/WinHttpHandler/…/WinHttpRequestState.cs :: Dispose` (PR #125293, fix
`28c5a4dcc838`, base `133f6a80839a`).

`Dispose` used `if (_disposed) return; _disposed = true;` with `_disposed` only **volatile**, not
atomic. Two threads (e.g. request completion + cancellation) can both pass the check and both `Free`
the `GCHandle` → double free → native callback handling faults.

**Decoy:** the `GCHandle.Free()` call looks like the bug; the finalizer is a plausible-but-wrong
suspect. The defect is the non-atomic check-then-set.

**Fix:** make the disposed transition atomic (`Interlocked`) so exactly one caller frees the handle.

**Admission (C1–C6):** C1 ✓ crash in native WinHTTP ≠ cause (the dispose race). C2 ✓ symptom avoids
"WinHttpRequestState"/"Interlocked". C3 ✓ PR #125293. C4 — obscure; single file → watch baseline can
grep it (C5, down-weight if FULL). C6 ✓ managed GCHandle lifetime ↔ native WinHTTP callbacks.
