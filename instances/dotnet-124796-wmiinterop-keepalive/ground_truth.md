# Ground truth — ⚠️ SPOILER (never fed to the agent)

**Root cause:** `…/System.Management/…/InteropClasses/WMIInterop.cs :: CompareTo_` (and the identical
`PutMethod_`). PR #124796, fix `adc191279b42`, base `0f125741186d`.

These methods pass the **argument** wrapper's native handle (`pCompareTo` in `CompareTo_`;
`pInSignature` / `pOutSignature` in `PutMethod_`) into a native WMI COM call, but only call
`GC.KeepAlive(this)` — they never keep the *argument* objects alive. Under GC pressure the argument
wrapper can be collected (and its `IWbemClassObjectFreeThreaded` finalizer free the COM object) while
the native call is still using the handle → premature-GC use-after-free in unmanaged code.

**Decoy:** the present `GC.KeepAlive(this)` makes the method look like lifetime is already handled —
the missing keepalive is on a *different* object (the parameter), not the receiver. The finalizer is
the plausible-but-wrong place to look.

**Fix:** add `GC.KeepAlive(pCompareTo)` (CompareTo_) and `GC.KeepAlive(pInSignature)` /
`GC.KeepAlive(pOutSignature)` (PutMethod_) after the native call.

**Provenance note:** the PR states the issue was surfaced by an LLM ("Opus highlighted these"). Strong
viral framing, but it is a *defensive* fix (the author could not fully prove the race), so grade on
ownership/lifetime correctness, not a confirmed crash.

**Admission (C1–C6):** C1 ✓ crash in native COM ≠ cause in the C# shim. C2 ✓ symptom avoids
"WMIInterop" / "KeepAlive". C3 ✓ merged PR #124796. C4 — obscure interop file → low contamination,
**but single-file**: watch that a hard baseline can't just grep `WMIInterop.cs` → if it FULL-solves,
down-weight (C5). C6 ✓ managed-lifetime ↔ native-COM edge (cross-language ownership).
